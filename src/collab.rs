//! Serverless live collaboration: one peer hosts over TCP (LAN, Tailscale, …), others join with
//! address + code. Changes sync per element (last writer wins); cursors are shared.
//!
//! ponytail: plaintext JSON lines + a join code, no TLS. Fine on trusted networks; add
//! TLS/Noise before using across the open internet. Z-order changes are not synced.

use crate::model::{Doc, Element, Kind, Scenario};
use eframe::egui::Pos2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;

pub const PORT: u16 = 7878;
const SYNC_EVERY: f64 = 0.05;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "t")]
enum Msg {
    Hello { code: String, name: String },
    Welcome { peer: u32, elements: Vec<Element>, scenarios: Vec<Scenario> },
    Reject { reason: String },
    Ops { from: u32, upsert: Vec<Element>, delete: Vec<u64> },
    Cursor { from: u32, name: String, x: f32, y: f32 },
    Bye { from: u32 },
}

enum Incoming {
    Joined(u32, TcpStream, String),
    Msg(Msg),
    Left(u32),
}

pub struct Peer {
    pub name: String,
    pub pos: Pos2,
    pub seen: f64,
}

pub struct Collab {
    pub host: bool,
    pub me: u32,
    pub name: String,
    pub share: String,
    pub status: String,
    pub peers: HashMap<u32, Peer>,
    rx: Receiver<Incoming>,
    links: Vec<(u32, TcpStream)>,
    baseline: HashMap<u64, String>,
    last_sync: f64,
    last_cursor: Pos2,
    joined: bool,
}

fn send(s: &mut TcpStream, m: &Msg) -> bool {
    let mut line = serde_json::to_string(m).unwrap_or_default();
    line.push('\n');
    s.write_all(line.as_bytes()).is_ok()
}

fn reader(stream: TcpStream, tx: Sender<Incoming>, peer: u32) {
    let r = BufReader::new(stream);
    for line in r.lines() {
        let Ok(line) = line else { break };
        if let Ok(m) = serde_json::from_str::<Msg>(&line) {
            if tx.send(Incoming::Msg(m)).is_err() {
                return;
            }
        }
    }
    let _ = tx.send(Incoming::Left(peer));
}

pub fn local_ip() -> String {
    // UDP "connect" picks the outbound interface without sending anything.
    UdpSocket::bind("0.0.0.0:0").and_then(|s| s.connect("8.8.8.8:80").map(|_| s)).and_then(|s| s.local_addr()).map(|a| a.ip().to_string()).unwrap_or("127.0.0.1".into())
}

fn make_code() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
    let mut r = crate::model::Rng::new(t ^ std::process::id() as u64);
    const A: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    (0..6).map(|_| A[((r.f() + 1.0) * 15.99) as usize] as char).collect()
}

impl Collab {
    pub fn host(name: &str) -> Result<Collab, String> {
        let listener = TcpListener::bind(("0.0.0.0", PORT)).or_else(|_| TcpListener::bind(("0.0.0.0", 0))).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let code = make_code();
        let (tx, rx) = channel();
        let next = Arc::new(AtomicU32::new(2));
        let c2 = code.clone();
        std::thread::spawn(move || {
            for s in listener.incoming().flatten() {
                let (tx, next, code) = (tx.clone(), next.clone(), c2.clone());
                std::thread::spawn(move || {
                    let _ = s.set_nodelay(true);
                    let Ok(rs) = s.try_clone() else { return };
                    let mut first = String::new();
                    let mut br = BufReader::new(rs);
                    if br.read_line(&mut first).is_err() {
                        return;
                    }
                    let mut w = s;
                    match serde_json::from_str::<Msg>(&first) {
                        Ok(Msg::Hello { code: c, name }) if c.eq_ignore_ascii_case(&code) => {
                            let id = next.fetch_add(1, Ordering::Relaxed);
                            let Ok(wc) = w.try_clone() else { return };
                            let _ = tx.send(Incoming::Joined(id, wc, name));
                            // continue reading on the buffered reader
                            for line in br.lines() {
                                let Ok(line) = line else { break };
                                if let Ok(m) = serde_json::from_str::<Msg>(&line) {
                                    if tx.send(Incoming::Msg(m)).is_err() {
                                        return;
                                    }
                                }
                            }
                            let _ = tx.send(Incoming::Left(id));
                        }
                        _ => {
                            send(&mut w, &Msg::Reject { reason: "wrong session code".into() });
                        }
                    }
                });
            }
        });
        let share = format!("{}:{port}", local_ip());
        Ok(Collab {
            host: true,
            me: 1,
            name: name.into(),
            status: format!("Hosting · share {share} · code {code}"),
            share: format!("{share}  ·  code {code}"),
            peers: HashMap::new(),
            rx,
            links: vec![],
            baseline: HashMap::new(),
            last_sync: 0.0,
            last_cursor: Pos2::ZERO,
            joined: true,
        })
    }

    pub fn join(addr: &str, code: &str, name: &str) -> Result<Collab, String> {
        let addr = if addr.contains(':') { addr.to_string() } else { format!("{addr}:{PORT}") };
        let mut s = TcpStream::connect(&addr).map_err(|e| format!("connect {addr}: {e}"))?;
        let _ = s.set_nodelay(true);
        if !send(&mut s, &Msg::Hello { code: code.trim().into(), name: name.into() }) {
            return Err("handshake failed".into());
        }
        let (tx, rx) = channel();
        let rs = s.try_clone().map_err(|e| e.to_string())?;
        std::thread::spawn(move || reader(rs, tx, 1));
        Ok(Collab {
            host: false,
            me: 0,
            name: name.into(),
            share: addr.clone(),
            status: format!("Joining {addr}…"),
            peers: HashMap::new(),
            rx,
            links: vec![(1, s)],
            baseline: HashMap::new(),
            last_sync: 0.0,
            last_cursor: Pos2::ZERO,
            joined: false,
        })
    }

    fn broadcast(&mut self, m: &Msg, except: u32) {
        self.links.retain_mut(|(id, s)| *id == except || send(s, m));
    }

    fn snapshot(doc: &Doc) -> HashMap<u64, String> {
        doc.elements.iter().map(|e| (e.id, serde_json::to_string(e).unwrap_or_default())).collect()
    }

    /// Pump network events into the doc and publish local changes. Returns true if the doc changed.
    pub fn tick(&mut self, doc: &mut Doc, now: f64, cursor: Option<Pos2>) -> bool {
        let mut changed = false;
        while let Ok(inc) = self.rx.try_recv() {
            match inc {
                Incoming::Joined(id, mut s, name) => {
                    send(&mut s, &Msg::Welcome { peer: id, elements: doc.elements.clone(), scenarios: doc.scenarios.clone() });
                    self.links.push((id, s));
                    self.peers.insert(id, Peer { name: name.clone(), pos: Pos2::ZERO, seen: now });
                    self.status = format!("{name} joined");
                }
                Incoming::Left(id) => {
                    self.links.retain(|l| l.0 != id);
                    if let Some(p) = self.peers.remove(&id) {
                        self.status = format!("{} left", p.name);
                    }
                    if !self.host && id == 1 {
                        self.status = "Disconnected from host".into();
                    }
                }
                Incoming::Msg(m) => match m.clone() {
                    Msg::Welcome { peer, elements, scenarios } => {
                        self.me = peer;
                        doc.elements = elements;
                        doc.scenarios = scenarios;
                        // own id range so peers never collide
                        let base = peer as u64 * 1_000_000_000;
                        doc.next_id = doc.elements.iter().map(|e| e.id).filter(|i| *i >= base && *i < base + 1_000_000_000).max().unwrap_or(base);
                        self.baseline = Self::snapshot(doc);
                        self.joined = true;
                        self.status = format!("Live with {}", self.share);
                        changed = true;
                    }
                    Msg::Reject { reason } => self.status = format!("Rejected: {reason}"),
                    Msg::Ops { from, upsert, delete } => {
                        if from == self.me {
                            continue;
                        }
                        for e in upsert {
                            self.baseline.insert(e.id, serde_json::to_string(&e).unwrap_or_default());
                            match doc.get_mut(e.id) {
                                Some(x) => *x = e,
                                None if e.kind == Kind::Region => doc.elements.insert(0, e),
                                None => doc.elements.push(e),
                            }
                        }
                        for id in &delete {
                            self.baseline.remove(id);
                        }
                        doc.elements.retain(|e| !delete.contains(&e.id));
                        changed = true;
                        if self.host {
                            self.broadcast(&m, from);
                        }
                    }
                    Msg::Cursor { from, name, x, y } => {
                        if from != self.me {
                            self.peers.insert(from, Peer { name, pos: Pos2::new(x, y), seen: now });
                            if self.host {
                                self.broadcast(&m, from);
                            }
                        }
                    }
                    Msg::Bye { from } => {
                        self.peers.remove(&from);
                    }
                    Msg::Hello { .. } => {}
                },
            }
        }
        if !self.joined || now - self.last_sync < SYNC_EVERY {
            return changed;
        }
        self.last_sync = now;
        let snap = Self::snapshot(doc);
        let upsert: Vec<Element> = doc.elements.iter().filter(|e| self.baseline.get(&e.id) != snap.get(&e.id)).cloned().collect();
        let delete: Vec<u64> = self.baseline.keys().filter(|id| !snap.contains_key(id)).copied().collect();
        if !upsert.is_empty() || !delete.is_empty() {
            let m = Msg::Ops { from: self.me, upsert, delete };
            self.broadcast(&m, 0);
            self.baseline = snap;
        }
        if let Some(c) = cursor.filter(|c| c.distance(self.last_cursor) > 0.5) {
            self.last_cursor = c;
            let m = Msg::Cursor { from: self.me, name: self.name.clone(), x: c.x, y: c.y };
            self.broadcast(&m, 0);
        }
        self.peers.retain(|_, p| now - p.seen < 60.0);
        changed
    }

    pub fn leave(&mut self) {
        let m = Msg::Bye { from: self.me };
        self.broadcast(&m, 0);
        self.links.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Element;
    use eframe::egui::{Rect, pos2, vec2};

    #[test]
    fn host_and_guest_sync_both_ways() {
        let mut host = Collab::host("Host").unwrap();
        let code = host.share.rsplit("code ").next().unwrap().to_string();
        let port = host.share.split(':').nth(1).unwrap().split_whitespace().next().unwrap().to_string();
        let mut hd = Doc::default();
        hd.add(Element::new(Kind::Rect, Rect::from_min_size(pos2(0., 0.), vec2(10., 10.))));
        let mut guest = Collab::join(&format!("127.0.0.1:{port}"), &code, "Guest").unwrap();
        let mut gd = Doc::default();
        let mut t = 0.0;
        let mut pump = |host: &mut Collab, guest: &mut Collab, hd: &mut Doc, gd: &mut Doc| {
            for _ in 0..60 {
                t += 0.1;
                host.tick(hd, t, None);
                guest.tick(gd, t, None);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };
        pump(&mut host, &mut guest, &mut hd, &mut gd);
        assert_eq!(gd.elements.len(), 1, "guest got welcome snapshot");
        let gid = gd.add(Element::new(Kind::Ellipse, Rect::from_min_size(pos2(5., 5.), vec2(10., 10.))));
        assert!(gid > 1_000_000_000, "guest ids live in their own range");
        pump(&mut host, &mut guest, &mut hd, &mut gd);
        assert_eq!(hd.elements.len(), 2, "host received guest's shape");
        hd.elements[0].text = "hello".into();
        pump(&mut host, &mut guest, &mut hd, &mut gd);
        assert_eq!(gd.get(hd.elements[0].id).unwrap().text, "hello");
    }
}
