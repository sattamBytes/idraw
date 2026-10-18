//! Diagram as code, written for humans. One connection per line, using component names:
//!
//! ```text
//! Users -> API Gateway -> Orders service
//! Orders service -> Postgres : SQL          label after a colon
//! Orders service => Kafka                   => animated   --> dashed   <-> both ways
//! Cache                                     a component on its own
//! Payments [aws-lambda] (card processing)   optional [icon] and (subtitle)
//! ```
//! Icons are picked from names ("S3", "Lambda", "Postgres"…). Only components and the
//! arrows between them live in code; other shapes on the canvas are untouched.

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

fn clean_name(s: &str) -> String {
    s.chars().filter(|c| !matches!(c, ',' | ':' | '[' | ']' | '(' | ')' | '>' | '#')).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Name of every component as used in code: its title, made unique ("Worker 2").
pub fn names(doc: &Doc) -> Vec<(u64, String)> {
    let mut used: HashMap<String, usize> = HashMap::new();
    doc.elements
        .iter()
        .filter(|e| e.kind == Kind::Node)
        .map(|e| {
            let base = clean_name(node_text(&e.text).0);
            let base = if base.is_empty() { "Component".to_string() } else { base };
            let n = used.entry(base.to_lowercase()).or_insert(0);
            *n += 1;
            (e.id, if *n == 1 { base } else { format!("{base} {n}") })
        })
        .collect()
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
    let n = names(doc);
    let name = |id: u64| n.iter().find(|x| x.0 == id).map(|x| x.1.clone());
    let mut out = String::new();
    let mut linked: Vec<u64> = vec![];
    for e in doc.elements.iter().filter(|e| e.kind == Kind::Arrow) {
        if let (Some(a), Some(b)) = (e.from.id.and_then(name), e.to.id.and_then(name)) {
            out += &format!("{a} {} {b}", op_of(e));
            if !e.text.is_empty() {
                out += &format!(" : {}", e.text.replace('\n', " "));
            }
            out.push('\n');
            linked.extend([e.from.id.unwrap(), e.to.id.unwrap()]);
        }
    }
    for (id, nm) in &n {
        if !linked.contains(id) {
            out += &format!("{nm}\n");
        }
    }
    out
}

#[derive(Default, Clone)]
struct NodeRef {
    name: String,
    icon: Option<String>,
    sub: Option<String>,
}

fn parse_node(tok: &str) -> Option<NodeRef> {
    let mut t = tok.trim().to_string();
    let mut r = NodeRef::default();
    if t.ends_with(')') {
        if let Some(i) = t.rfind('(') {
            r.sub = Some(t[i + 1..t.len() - 1].trim().to_string());
            t.truncate(i);
        }
    }
    let t2 = t.trim_end().to_string();
    if t2.ends_with(']') {
        if let Some(i) = t2.rfind('[') {
            r.icon = Some(t2[i + 1..t2.len() - 1].trim().to_string());
            t = t2[..i].to_string();
        }
    }
    r.name = clean_name(&t);
    (!r.name.is_empty()).then_some(r)
}

/// Splits off a trailing ": label" (ignoring colons inside parentheses).
fn split_label(line: &str) -> (&str, String) {
    let mut depth = 0;
    for (i, c) in line.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ':' if depth == 0 => return (&line[..i], line[i + 1..].trim().to_string()),
            _ => {}
        }
    }
    (line, String::new())
}

/// Earliest arrow operator in `s` (longest wins at the same spot).
fn find_op(s: &str) -> Option<(usize, &'static str)> {
    OPS.iter().filter_map(|op| s.find(op).map(|i| (i, *op))).min_by_key(|(i, op)| (*i, usize::MAX - op.len()))
}

struct Edge {
    a: String,
    b: String,
    op: &'static str,
    label: String,
}

fn parse(src: &str) -> Result<(Vec<NodeRef>, Vec<Edge>), String> {
    let mut nodes: Vec<NodeRef> = vec![];
    let mut edges = vec![];
    let add = |r: NodeRef, nodes: &mut Vec<NodeRef>| -> String {
        let key = r.name.to_lowercase();
        match nodes.iter_mut().find(|n| n.name.to_lowercase() == key) {
            Some(n) => {
                n.icon = r.icon.or(n.icon.take());
                n.sub = r.sub.or(n.sub.take());
                n.name.clone()
            }
            None => {
                let nm = r.name.clone();
                nodes.push(r);
                nm
            }
        }
    };
    for (ln, raw) in src.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let (body, label) = split_label(line);
        let mut segs: Vec<(&str, &'static str)> = vec![];
        let mut rest = body;
        while let Some((i, op)) = find_op(rest) {
            segs.push((&rest[..i], op));
            rest = &rest[i + op.len()..];
        }
        segs.push((rest, ""));
        let groups: Vec<(Vec<String>, &'static str)> = segs
            .iter()
            .map(|(seg, op)| (seg.split(',').filter_map(parse_node).map(|r| add(r, &mut nodes)).collect(), *op))
            .collect();
        if groups.len() > 1 && groups.iter().any(|g| g.0.is_empty()) {
            return Err(format!("line {}: an arrow needs a name on both sides", ln + 1));
        }
        for w in groups.windows(2) {
            for a in &w[0].0 {
                for b in &w[1].0 {
                    edges.push(Edge { a: a.clone(), b: b.clone(), op: w[0].1, label: label.clone() });
                }
            }
        }
    }
    Ok((nodes, edges))
}

/// Picks a generic icon from a name, e.g. "orders db" -> database.
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
        (&["user", "client", "customer", "shopper"], "user"),
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
        (&["payment", "billing", "stripe"], "card"),
    ];
    for (keys, icon) in table {
        if keys.iter().any(|k| n.split(|c: char| !c.is_alphanumeric()).any(|w| w == *k) || (k.len() > 3 && n.contains(k))) {
            return icon;
        }
    }
    "service"
}

/// Best library icon for a component name: exact service name first ("S3", "Lambda"), then a guess.
pub fn best_icon(name: &str) -> String {
    let n = name.to_lowercase();
    let lib = crate::icons::all().iter().filter(|i| !matches!(i.cat, "ui" | "glyph" | "Boundaries") && !i.color);
    for i in lib {
        let id_tail = i.id.split_once('-').map(|x| x.1).unwrap_or(i.id);
        if i.label.to_lowercase() == n || id_tail == n {
            return i.id.to_string();
        }
    }
    guess_icon(name).to_string()
}

fn node_body(title: &str, sub: &str) -> String {
    if sub.is_empty() { title.to_string() } else { format!("{title}\n{sub}") }
}

fn free_spot(doc: &Doc, mut c: Pos2, size: Vec2) -> Pos2 {
    for _ in 0..40 {
        let r = Rect::from_center_size(c, size).expand(16.0);
        if !doc.elements.iter().any(|e| e.kind != Kind::Region && e.kind != Kind::Arrow && !e.rect.min.x.is_nan() && e.rect.intersects(r)) {
            return c;
        }
        c.y += size.y + 32.0;
    }
    c
}

/// Applies code to the doc: creates/renames/deletes components and the arrows between them.
pub fn apply(doc: &mut Doc, src: &str) -> Result<(), String> {
    let (nodes, edges) = parse(src)?;
    let was_empty = !doc.elements.iter().any(|e| e.kind == Kind::Node);
    let existing = names(doc);
    let low = |s: &str| s.to_lowercase();
    let mut id_of: HashMap<String, u64> = existing.iter().map(|(id, k)| (low(k), *id)).collect();
    let mut gone: Vec<(u64, String)> = existing.iter().filter(|(_, k)| !nodes.iter().any(|n| low(&n.name) == low(k))).cloned().collect();
    let fresh: Vec<&NodeRef> = nodes.iter().filter(|n| !id_of.contains_key(&low(&n.name))).collect();
    // one name changed while typing = rename (keep position, icon, notes)
    if gone.len() == 1 && fresh.len() == 1 {
        let (id, old) = gone.pop().unwrap();
        id_of.remove(&low(&old));
        id_of.insert(low(&fresh[0].name), id);
        if let Some(e) = doc.get_mut(id) {
            let (_, sub) = node_text(&e.text);
            e.text = node_body(&fresh[0].name, &sub.join("\n"));
        }
    }
    let gone_ids: Vec<u64> = gone.iter().map(|g| g.0).collect();
    let mut kill = gone_ids.clone();
    kill.extend(doc.elements.iter().filter(|e| e.kind == Kind::Arrow && [e.from.id, e.to.id].iter().flatten().any(|i| gone_ids.contains(i))).map(|e| e.id));
    doc.remove(&kill);
    let mut created = vec![];
    for n in &nodes {
        let icon = n.icon.clone().filter(|i| crate::icons::find(i).is_some());
        match id_of.get(&low(&n.name)).and_then(|id| doc.get_mut(*id)) {
            Some(e) => {
                let (title, sub) = node_text(&e.text);
                let sub = n.sub.clone().unwrap_or_else(|| sub.join("\n"));
                if title != n.name || n.sub.is_some() {
                    e.text = node_body(&n.name, &sub);
                }
                if let Some(i) = icon.filter(|i| *i != e.icon) {
                    e.style.stroke = crate::icons::find(&i).map(|x| crate::icons::accent(x.cat)).unwrap_or(e.style.stroke);
                    e.icon = i;
                }
            }
            None => {
                let icon = icon.unwrap_or_else(|| best_icon(&n.name));
                let mut e = Element::node(&icon, pos2(f32::NAN, f32::NAN));
                e.text = node_body(&n.name, n.sub.as_deref().unwrap_or(""));
                let id = doc.add(e);
                id_of.insert(low(&n.name), id);
                created.push((low(&n.name), id));
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
        let (a, b) = (id_of[&low(&ed.a)], id_of[&low(&ed.b)]);
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
    // place new components next to a connected neighbour
    let size = vec2(176.0, 56.0);
    for (key, id) in &created {
        let placed = |k: &str| id_of.get(k).and_then(|i| doc.get(*i)).filter(|e| !e.rect.min.x.is_nan()).map(|e| e.rect.center());
        let anchor = edges.iter().find_map(|e| {
            if &low(&e.b) == key {
                placed(&low(&e.a)).map(|c| c + vec2(280.0, 0.0))
            } else if &low(&e.a) == key {
                placed(&low(&e.b)).map(|c| c - vec2(280.0, 0.0))
            } else {
                None
            }
        });
        let c = anchor.unwrap_or_else(|| {
            let b = doc.elements.iter().filter(|e| !e.rect.min.x.is_nan() && e.kind != Kind::Arrow).map(|e| e.rect).reduce(|a, b| a.union(b));
            b.map_or(pos2(0.0, 0.0), |b| pos2(b.min.x + 88.0, b.max.y + 100.0))
        });
        let c = free_spot(doc, c, size);
        doc.get_mut(*id).unwrap().rect = Rect::from_center_size(c, size);
    }
    if was_empty && !created.is_empty() {
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
    let mut labels: HashMap<String, (String, &'static str)> = HashMap::new();
    let mut order: Vec<String> = vec![];
    let mut edges: Vec<(String, &'static str, String, String)> = vec![];
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
            let st = i;
            while i < ch.len() && (ch[i].is_alphanumeric() || ch[i] == '_') {
                i += 1;
            }
            if i == st {
                break;
            }
            let id: String = ch[st..i].iter().collect();
            let mut label = String::new();
            let mut icon = "";
            if i < ch.len() && "[({>".contains(ch[i]) {
                let open: String = ch[i..].iter().take_while(|c| "[({>".contains(**c)).collect();
                let close: String = open
                    .chars()
                    .rev()
                    .map(|c| match c {
                        '[' => ']',
                        '(' => ')',
                        '{' => '}',
                        _ => ']',
                    })
                    .collect();
                if open == "[(" {
                    icon = "database";
                }
                let rest: String = ch[i + open.len()..].iter().collect();
                match rest.find(&close) {
                    Some(end) => {
                        label = rest[..end].trim_matches('"').to_string();
                        i += open.len() + end + close.len();
                    }
                    None => i = ch.len(),
                }
            }
            let entry = labels.entry(id.clone()).or_insert_with(|| {
                order.push(id.clone());
                (id.clone(), "")
            });
            if !label.is_empty() {
                entry.0 = label;
            }
            if !icon.is_empty() {
                entry.1 = icon;
            }
            if let (Some(p), Some((op, lab))) = (&prev, pending.take()) {
                edges.push((p.clone(), op, id.clone(), lab));
            }
            prev = Some(id);
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
    let name = |id: &str| clean_name(&labels[id].0);
    let mut out = String::new();
    for id in &order {
        if labels[id].1 == "database" {
            out += &format!("{} [database]\n", name(id));
        }
    }
    let mut linked = vec![];
    for (a, op, b, lab) in &edges {
        out += &format!("{} {op} {}{}\n", name(a), name(b), if lab.is_empty() { String::new() } else { format!(" : {lab}") });
        linked.extend([a.clone(), b.clone()]);
    }
    for id in &order {
        if !linked.contains(id) && labels[id].1 != "database" {
            out += &format!("{}\n", name(id));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(d: &Doc) -> Vec<String> {
        d.elements.iter().filter(|e| e.kind == Kind::Node).map(|e| node_text(&e.text).0.to_string()).collect()
    }

    #[test]
    fn plain_sentences_build_a_diagram() {
        let mut d = Doc::default();
        apply(&mut d, "Users => API Gateway -> Orders service : REST\nOrders service -> S3\nCache").unwrap();
        assert_eq!(titles(&d).len(), 5);
        assert!(d.elements.iter().any(|e| e.icon == "aws-s3"), "S3 picks the AWS S3 icon");
        assert!(d.elements.iter().any(|e| e.icon == "user"));
        let code = to_code(&d);
        assert!(code.contains("Users => API Gateway"), "{code}");
        assert!(code.contains("API Gateway -> Orders service : REST"), "{code}");
        assert!(code.lines().any(|l| l == "Cache"), "{code}");
        // regenerating from own output is a no-op
        let n = d.elements.len();
        apply(&mut d, &code).unwrap();
        assert_eq!(d.elements.len(), n);
        assert!(d.elements.iter().all(|e| !e.rect.min.x.is_nan()));
    }

    #[test]
    fn renaming_keeps_the_component() {
        let mut d = Doc::default();
        apply(&mut d, "Web -> Orders\nOrders -> DB").unwrap();
        let id = d.elements.iter().find(|e| e.text == "Orders").unwrap().id;
        apply(&mut d, "Web -> Checkout\nCheckout -> DB").unwrap();
        assert_eq!(node_text(&d.get(id).unwrap().text).0, "Checkout");
        assert_eq!(titles(&d).len(), 3);
        // deleting a line's component removes it and its arrows
        apply(&mut d, "Web -> Checkout").unwrap();
        assert_eq!(d.elements.len(), 3);
    }

    #[test]
    fn optional_icon_and_subtitle() {
        let mut d = Doc::default();
        apply(&mut d, "Payments [aws-lambda] (card processing) -> Ledger").unwrap();
        let p = d.elements.iter().find(|e| e.text.starts_with("Payments")).unwrap();
        assert_eq!((p.icon.as_str(), p.text.as_str()), ("aws-lambda", "Payments\ncard processing"));
    }

    #[test]
    fn mermaid_import() {
        let m = "graph LR\n  A[Client] -->|HTTPS| B(API)\n  B --> C[(Orders DB)]\n  B -.-> D[Cache]";
        let code = from_mermaid(m).unwrap();
        let mut d = Doc::default();
        apply(&mut d, &code).unwrap();
        let c = to_code(&d);
        assert!(c.contains("Client -> API : HTTPS"), "{c}");
        assert!(c.contains("API --> Cache"), "{c}");
        assert!(d.elements.iter().any(|e| e.icon == "database"));
    }
}
