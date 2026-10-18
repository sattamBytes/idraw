//! Diagram-as-code: a tiny text format kept in two-way sync with the canvas, plus Mermaid import.
//!
//! ```text
//! users   = user "Users" "web + mobile"     # key = icon "Title" "subtitle"
//! gateway = aws-apigw "API Gateway"
//! users => gateway : HTTPS                   # => packets  ~> dashes  --> dashed  <-> both  -> plain
//! gateway -> orders, payments
//! ```
//! Only component nodes and the arrows between them live in code; other shapes are untouched.

use crate::model::*;
use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};
use std::collections::HashMap;

const OPS: [&str; 5] = ["<->", "-->", "=>", "~>", "->"];

pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_string();
    match out.chars().next() {
        None => "node".into(),
        Some(c) if c.is_ascii_digit() => format!("n{out}"),
        _ => out,
    }
}

/// Stable code key for every component node (derived from its title).
pub fn keys(doc: &Doc) -> Vec<(u64, String)> {
    let mut used: HashMap<String, usize> = HashMap::new();
    doc.elements
        .iter()
        .filter(|e| e.kind == Kind::Node)
        .map(|e| {
            let base = if e.key.is_empty() { slug(node_text(&e.text).0) } else { e.key.clone() };
            let n = used.entry(base.clone()).or_insert(0);
            *n += 1;
            (e.id, if *n == 1 { base } else { format!("{base}{n}") })
        })
        .collect()
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "'"))
}

fn op_of(e: &Element) -> &'static str {
    match (e.style.flow, e.style.dashed, e.style.head_start && e.style.head_end) {
        (Flow::Dots, _, _) => "=>",
        (Flow::Dashes, _, _) => "~>",
        (_, _, true) => "<->",
        (_, true, _) => "-->",
        _ => "->",
    }
}

pub fn to_code(doc: &Doc) -> String {
    let k = keys(doc);
    let key = |id: u64| k.iter().find(|x| x.0 == id).map(|x| x.1.clone());
    let mut out = String::from("# key = icon \"Title\" \"subtitle\"   ·   edges: -> => ~> --> <->   ·   a -> b : label\n");
    for (id, kk) in &k {
        let e = doc.get(*id).unwrap();
        let (title, sub) = node_text(&e.text);
        let mut line = format!("{kk} = {} {}", if e.icon.is_empty() { "service" } else { &e.icon }, quote(title));
        if !sub.is_empty() {
            line += &format!(" {}", quote(&sub.join(" | ")));
        }
        out += &line;
        out.push('\n');
    }
    let mut first = true;
    for e in doc.elements.iter().filter(|e| e.kind == Kind::Arrow) {
        if let (Some(a), Some(b)) = (e.from.id.and_then(key), e.to.id.and_then(key)) {
            if first {
                out.push('\n');
                first = false;
            }
            out += &format!("{a} {} {b}", op_of(e));
            if !e.text.is_empty() {
                out += &format!(" : {}", e.text.replace('\n', " "));
            }
            out.push('\n');
        }
    }
    out
}

struct Def {
    key: String,
    icon: String,
    title: String,
    sub: String,
}

struct Edge {
    a: String,
    b: String,
    op: &'static str,
    label: String,
}

fn strings(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur: Option<String> = None;
    for c in s.chars() {
        match (&mut cur, c) {
            (None, '"') => cur = Some(String::new()),
            (Some(t), '"') => {
                out.push(std::mem::take(t));
                cur = None;
            }
            (Some(t), c) => t.push(c),
            _ => {}
        }
    }
    out
}

fn parse(src: &str) -> Result<(Vec<Def>, Vec<Edge>), String> {
    let (mut defs, mut edges) = (vec![], vec![]);
    for (n, raw) in src.lines().enumerate() {
        let line = match raw.find('#') {
            Some(i) if !raw[..i].contains('"') || raw[..i].matches('"').count() % 2 == 0 => &raw[..i],
            _ => raw,
        }
        .trim();
        if line.is_empty() {
            continue;
        }
        let err = |m: &str| format!("line {}: {m}", n + 1);
        if let Some(op) = OPS.iter().find(|o| line.contains(**o)).copied() {
            let (body, label) = match line.split_once(" : ").or_else(|| line.split_once(':')) {
                Some((b, l)) => (b, l.trim().to_string()),
                None => (line, String::new()),
            };
            let parts: Vec<Vec<String>> = body.split(op).map(|p| p.split(',').map(|k| slug(k.trim())).filter(|k| k != "node").collect()).collect();
            if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
                return Err(err("edge needs a source and a target"));
            }
            for w in parts.windows(2) {
                for a in &w[0] {
                    for b in &w[1] {
                        edges.push(Edge { a: a.clone(), b: b.clone(), op, label: label.clone() });
                    }
                }
            }
        } else if let Some((k, rest)) = line.split_once('=') {
            let key = slug(k.trim());
            let rest = rest.trim();
            let icon = if rest.starts_with('"') { String::new() } else { rest.split_whitespace().next().unwrap_or("").to_string() };
            let s = strings(rest);
            defs.push(Def { title: s.first().cloned().unwrap_or_else(|| key.replace('_', " ")), sub: s.get(1).cloned().unwrap_or_default(), key, icon });
        } else {
            return Err(err("expected `key = icon \"Title\"` or `a -> b`"));
        }
    }
    Ok((defs, edges))
}

/// Picks a generic icon from a name, e.g. "orders_db" -> database.
pub fn guess_icon(name: &str) -> &'static str {
    let n = name.to_lowercase();
    let table: &[(&[&str], &str)] = &[
        (&["postgres", "mysql", "sql", "db", "database", "mongo", "dynamo", "aurora"], "database"),
        (&["redis", "cache", "memcache"], "cache"),
        (&["kafka", "stream", "kinesis", "event"], "stream"),
        (&["queue", "sqs", "rabbit", "nats"], "queue"),
        (&["s3", "bucket", "storage", "blob", "minio"], "bucket"),
        (&["lb", "balancer", "alb", "elb", "haproxy"], "lb"),
        (&["nginx", "envoy", "proxy", "traefik"], "proxy"),
        (&["gateway", "gw", "apigw"], "gateway"),
        (&["cdn", "cloudfront", "edge"], "cdn"),
        (&["dns"], "dns"),
        (&["user", "client", "customer"], "user"),
        (&["web", "frontend", "ui", "browser", "spa"], "laptop"),
        (&["mobile", "ios", "android"], "mobile"),
        (&["auth", "login", "keycloak", "oauth"], "lock"),
        (&["search", "elastic", "opensearch"], "search"),
        (&["worker", "job", "consumer"], "worker"),
        (&["cron", "scheduler", "timer"], "clock"),
        (&["mail", "smtp", "email"], "email"),
        (&["llm", "model", "ai", "ml", "openai"], "sparkles"),
        (&["grafana", "prometheus", "monitor", "metrics"], "chart"),
        (&["log", "loki", "elk"], "logs"),
        (&["vault", "secret"], "key"),
        (&["k8s", "kube"], "k8s"),
        (&["lambda", "function", "fn"], "function"),
    ];
    for (keys, icon) in table {
        if keys.iter().any(|k| n.contains(k)) {
            return icon;
        }
    }
    "service"
}

fn node_body(title: &str, sub: &str) -> String {
    if sub.is_empty() { title.to_string() } else { format!("{title}\n{}", sub.replace(" | ", "\n")) }
}

fn free_spot(doc: &Doc, mut c: Pos2, size: Vec2) -> Pos2 {
    for _ in 0..40 {
        let r = Rect::from_center_size(c, size).expand(16.0);
        if !doc.elements.iter().any(|e| e.kind != Kind::Region && e.kind != Kind::Arrow && e.rect.intersects(r)) {
            return c;
        }
        c.y += size.y + 32.0;
    }
    c
}

/// Applies code to the doc: creates/updates/deletes component nodes and the arrows between them.
pub fn apply(doc: &mut Doc, src: &str) -> Result<(), String> {
    let (mut defs, edges) = parse(src)?;
    for e in &edges {
        for k in [&e.a, &e.b] {
            if !defs.iter().any(|d| &d.key == k) {
                defs.push(Def { key: k.clone(), icon: guess_icon(k).into(), title: k.replace('_', " "), sub: String::new() });
            }
        }
    }
    let was_empty = !doc.elements.iter().any(|e| e.kind == Kind::Node);
    let existing = keys(doc);
    let mut id_of: HashMap<String, u64> = existing.iter().map(|(id, k)| (k.clone(), *id)).collect();
    // remove nodes no longer in code
    let mut gone: Vec<u64> = existing.iter().filter(|(_, k)| !defs.iter().any(|d| &d.key == k)).map(|(id, _)| *id).collect();
    let dangling: Vec<u64> = doc.elements.iter().filter(|e| e.kind == Kind::Arrow && [e.from.id, e.to.id].iter().flatten().any(|i| gone.contains(i))).map(|e| e.id).collect();
    gone.extend(dangling);
    doc.remove(&gone);
    // create / update nodes
    let mut fresh = vec![];
    for d in &defs {
        let icon = if d.icon.is_empty() || crate::icons::find(&d.icon).is_none() { guess_icon(&d.key).to_string() } else { d.icon.clone() };
        let body = node_body(&d.title, &d.sub);
        match id_of.get(&d.key).and_then(|id| doc.get_mut(*id)) {
            Some(e) => {
                if e.icon != icon {
                    e.icon = icon.clone();
                    if let Some(i) = crate::icons::find(&icon) {
                        e.style.stroke = crate::icons::accent(i.cat);
                    }
                }
                e.text = body;
                e.key = d.key.clone();
            }
            None => {
                let mut e = Element::node(&icon, pos2(f32::NAN, f32::NAN));
                e.text = body;
                e.key = d.key.clone();
                let id = doc.add(e);
                id_of.insert(d.key.clone(), id);
                fresh.push((d.key.clone(), id));
            }
        }
    }
    // arrows between components
    let node_arrows: Vec<(u64, u64, u64)> = doc
        .elements
        .iter()
        .filter(|e| e.kind == Kind::Arrow)
        .filter_map(|e| Some((e.id, e.from.id?, e.to.id?)))
        .filter(|(_, a, b)| doc.get(*a).is_some_and(|e| e.kind == Kind::Node) && doc.get(*b).is_some_and(|e| e.kind == Kind::Node))
        .collect();
    let mut keep = vec![];
    for ed in &edges {
        let (a, b) = (id_of[&ed.a], id_of[&ed.b]);
        let id = match node_arrows.iter().find(|x| x.1 == a && x.2 == b && !keep.contains(&x.0)) {
            Some(x) => x.0,
            None => {
                let mut e = Element::new(Kind::Arrow, Rect::ZERO);
                e.from.id = Some(a);
                e.to.id = Some(b);
                doc.add(e)
            }
        };
        keep.push(id);
        let e = doc.get_mut(id).unwrap();
        e.text = ed.label.clone();
        let s = &mut e.style;
        s.flow = match ed.op {
            "=>" => Flow::Dots,
            "~>" => Flow::Dashes,
            _ => Flow::None,
        };
        s.dashed = ed.op == "-->";
        s.head_start = ed.op == "<->";
        s.head_end = true;
    }
    let stale: Vec<u64> = node_arrows.iter().map(|x| x.0).filter(|id| !keep.contains(id)).collect();
    doc.remove(&stale);
    // place new nodes next to a connected neighbour
    for (key, id) in &fresh {
        let size = vec2(176.0, 56.0);
        let placed = |k: &String| id_of.get(k).and_then(|i| doc.get(*i)).filter(|e| !e.rect.min.x.is_nan()).map(|e| e.rect.center());
        let anchor = edges.iter().find_map(|e| if &e.b == key { placed(&e.a).map(|c| c + vec2(280.0, 0.0)) } else if &e.a == key { placed(&e.b).map(|c| c - vec2(280.0, 0.0)) } else { None });
        let c = anchor.unwrap_or_else(|| {
            let b = doc.elements.iter().filter(|e| !e.rect.min.x.is_nan() && e.kind != Kind::Arrow).map(|e| e.rect).reduce(|a, b| a.union(b));
            b.map_or(pos2(0.0, 0.0), |b| pos2(b.min.x + 88.0, b.max.y + 100.0))
        });
        let c = free_spot(doc, c, size);
        doc.get_mut(*id).unwrap().rect = Rect::from_center_size(c, size);
    }
    if was_empty && !fresh.is_empty() {
        for (id, r) in crate::layout::layered(doc, &[], false) {
            if let Some(e) = doc.get_mut(id) {
                e.rect = r;
            }
        }
    }
    Ok(())
}

/// Converts a Mermaid `graph`/`flowchart` into idraw code.
pub fn from_mermaid(src: &str) -> Option<String> {
    let mut lines = src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("%%"));
    let head = lines.next()?;
    if !(head.starts_with("graph") || head.starts_with("flowchart")) {
        return None;
    }
    let mut defs: Vec<(String, String, &'static str)> = vec![];
    let mut edges = vec![];
    for l in lines {
        if ["subgraph", "end", "classDef", "class ", "style ", "linkStyle", "click", "direction"].iter().any(|k| l.starts_with(k)) {
            continue;
        }
        let ch: Vec<char> = l.trim_end_matches(';').chars().collect();
        let mut i = 0;
        let mut prev: Option<String> = None;
        let mut pending: Option<(&'static str, String)> = None;
        while i < ch.len() {
            while i < ch.len() && ch[i] == ' ' {
                i += 1;
            }
            // node ref
            let st = i;
            while i < ch.len() && (ch[i].is_alphanumeric() || ch[i] == '_') {
                i += 1;
            }
            if i == st {
                break;
            }
            let id: String = ch[st..i].iter().collect();
            let mut label = id.clone();
            let mut icon = "";
            if i < ch.len() && "[({>".contains(ch[i]) {
                let open: String = ch[i..].iter().take_while(|c| "[({>".contains(**c)).collect();
                let close: String = open.chars().rev().map(|c| match c {
                    '[' => ']',
                    '(' => ')',
                    '{' => '}',
                    _ => ']',
                }).collect();
                if open == "[(" {
                    icon = "database";
                }
                let rest: String = ch[i + open.len()..].iter().collect();
                if let Some(end) = rest.find(&close) {
                    label = rest[..end].trim_matches('"').to_string();
                    i += open.len() + end + close.len();
                } else {
                    i = ch.len();
                }
            }
            if let Some(d) = defs.iter_mut().find(|d| d.0 == id) {
                if label != id {
                    d.1 = label;
                }
                if !icon.is_empty() {
                    d.2 = icon;
                }
            } else {
                defs.push((id.clone(), label.clone(), if icon.is_empty() { guess_icon(&label) } else { icon }));
            }
            if let (Some(p), Some((op, lab))) = (&prev, pending.take()) {
                edges.push(format!("{p} {op} {id}{}", if lab.is_empty() { String::new() } else { format!(" : {lab}") }));
            }
            prev = Some(id);
            // edge operator
            while i < ch.len() && ch[i] == ' ' {
                i += 1;
            }
            let st = i;
            while i < ch.len() && "-=.<>ox".contains(ch[i]) {
                i += 1;
            }
            if i == st {
                break;
            }
            let opstr: String = ch[st..i].iter().collect();
            let mut lab = String::new();
            if opstr == "--" || opstr == "==" {
                // `A -- text --> B`
                let rest: String = ch[i..].iter().collect();
                if let Some(end) = rest.find("--").or_else(|| rest.find("==")) {
                    lab = rest[..end].trim().to_string();
                    i += end;
                    while i < ch.len() && "-=.<>".contains(ch[i]) {
                        i += 1;
                    }
                }
            }
            while i < ch.len() && ch[i] == ' ' {
                i += 1;
            }
            if i < ch.len() && ch[i] == '|' {
                let rest: String = ch[i + 1..].iter().collect();
                if let Some(end) = rest.find('|') {
                    lab = rest[..end].to_string();
                    i += end + 2;
                }
            }
            let op = if opstr.contains('.') {
                "-->"
            } else if opstr.contains('=') {
                "=>"
            } else if opstr.starts_with('<') {
                "<->"
            } else {
                "->"
            };
            pending = Some((op, lab));
        }
    }
    // readable keys from labels
    let mut keymap: HashMap<String, String> = HashMap::new();
    let mut used: Vec<String> = vec![];
    for (id, label, _) in &defs {
        let mut k = slug(label);
        while used.contains(&k) {
            k.push('2');
        }
        used.push(k.clone());
        keymap.insert(id.clone(), k);
    }
    let mut out = String::new();
    for (id, label, icon) in &defs {
        out += &format!("{} = {icon} {}\n", keymap[id], quote(label));
    }
    out.push('\n');
    for e in edges {
        let mut it = e.splitn(3, ' ');
        let (a, op, rest) = (it.next().unwrap(), it.next().unwrap(), it.next().unwrap_or(""));
        let (b, lab) = rest.split_once(' ').unwrap_or((rest, ""));
        out += &format!("{} {op} {} {lab}\n", keymap[a], keymap[b]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_roundtrip_and_edit() {
        let mut d = Doc::default();
        apply(&mut d, "users = user \"Users\" \"web\"\nusers => api : HTTPS\napi -> db").unwrap();
        assert_eq!(d.elements.iter().filter(|e| e.kind == Kind::Node).count(), 3);
        let code = to_code(&d);
        assert!(code.contains("users => api : HTTPS"), "{code}");
        assert!(code.contains("db = database"), "{code}");
        // regenerate from own output is a no-op
        let n = d.elements.len();
        apply(&mut d, &code).unwrap();
        assert_eq!(d.elements.len(), n);
        // removing a node removes its edges
        apply(&mut d, "users = user \"Users\" \"web\"\napi = gateway \"api\"\nusers -> api").unwrap();
        assert_eq!(d.elements.len(), 3);
        assert!(d.elements.iter().all(|e| !e.rect.min.x.is_nan()));
    }

    #[test]
    fn mermaid_import() {
        let m = "graph LR\n  A[Client] -->|HTTPS| B(API)\n  B --> C[(Orders DB)]\n  B -.-> D[Cache]";
        let code = from_mermaid(m).unwrap();
        let mut d = Doc::default();
        apply(&mut d, &code).unwrap();
        let c = to_code(&d);
        assert!(c.contains("orders_db = database"), "{c}");
        assert!(c.contains("client -> api : HTTPS"), "{c}");
        assert!(c.contains("api --> cache"), "{c}");
    }
}
