//! Forgiving `.idraw` reader, so people and agents can write diagrams by hand.
//!
//! On top of the native format it accepts:
//! - missing `id` / `next_id` / `style` / `rect` fields (kind defaults fill them in)
//! - string ids (`"id": "api"`) and connector ends by id (`"from": "api"`, `"to": 7`, `"to": [x, y]`)
//! - `x, y, w, h` on the element (or `"rect": {x, y, w, h}` / `[x, y, w, h]`); nodes may give just x, y
//! - style fields directly on the element (`"stroke": "#2563eb"`, `"flow": "comet"`)
//! - `#rrggbb` / `#rrggbbaa` colours, and enum names in any case (`"dash-dot"`, `"top_left"`)
//! - `title` + `subtitle` on components, `side: "right"` on connector ends

use crate::model::*;
use eframe::egui::{Color32, Rect, pos2, vec2};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

const STYLE_KEYS: [&str; 14] = ["stroke", "fill", "width", "dashed", "round", "font", "route", "head_start", "head_end", "flow", "pattern", "face", "text_color", "color"];
const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

/// Parses a native or hand-written diagram. Returns the doc and warnings about things it had to guess or ignore.
pub fn parse(text: &str) -> Result<(Doc, Vec<String>), String> {
    let mut root: Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;
    // a bare list of elements is fine too
    if root.is_array() {
        root = json!({ "elements": root });
    }
    let obj = root.as_object_mut().ok_or("expected an object with \"elements\"")?;
    let native = obj.contains_key("next_id");
    let mut warn = vec![];
    let els = obj.remove("elements").unwrap_or(json!([]));
    let els = els.as_array().ok_or("\"elements\" must be a list")?.clone();
    let pages = obj.remove("pages").unwrap_or(json!([]));
    let pages = pages.as_array().ok_or("\"pages\" must be a list of { \"name\", \"elements\" }")?.clone();
    let page_els = |p: &Value| p.get("elements").and_then(Value::as_array).cloned().unwrap_or_default();

    // numeric ids are kept, so fresh ones start above the biggest anywhere in the file
    let in_pages = pages.iter().filter_map(|p| p.get("elements").and_then(Value::as_array)).flatten();
    let mut next = els.iter().chain(in_pages).filter_map(|e| e.get("id").and_then(Value::as_u64)).max().unwrap_or(0);
    let mut keys: HashMap<String, u64> = HashMap::new();
    let mut out = convert(&els, &mut next, &mut keys, &mut warn, native, "");
    let mut conv_pages = vec![];
    for (pi, p) in pages.iter().enumerate() {
        let name = p.get("name").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("Page {}", pi + 1));
        if conv_pages.iter().any(|c: &Value| c["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(&name))) {
            warn.push(format!("two pages are called \"{name}\"; give each a different name"));
        }
        let els = convert(&page_els(p), &mut next, &mut keys, &mut warn, native, &format!("page \"{name}\", "));
        conv_pages.push(json!({ "name": name, "elements": els }));
    }
    if !conv_pages.is_empty() {
        // the open page's elements live at the top level
        let open = obj.get("page").and_then(Value::as_u64).unwrap_or(0).min(conv_pages.len() as u64 - 1) as usize;
        if out.is_empty() {
            out = std::mem::take(conv_pages[open]["elements"].as_array_mut().unwrap());
        }
        obj.insert("page".into(), json!(open));
        obj.insert("pages".into(), Value::Array(conv_pages));
    }
    obj.insert("elements".into(), Value::Array(out));
    // scenario steps may name elements by their string id too
    for scn in obj.get_mut("scenarios").and_then(Value::as_array_mut).into_iter().flatten() {
        for step in scn.get_mut("steps").and_then(Value::as_array_mut).into_iter().flatten() {
            for k in ["from", "to", "edge"] {
                if let Some(Value::String(s)) = step.get(k).cloned() {
                    match keys.get(&s) {
                        Some(id) => step[k] = json!(id),
                        None => return Err(format!("scenario step: no element with id \"{s}\"")),
                    }
                }
            }
        }
    }
    obj.entry("next_id").or_insert(json!(next));
    if let Some(j) = obj.get_mut("jumps") {
        *j = variant(j, &["Hop", "Gap", "Off"]);
    }
    let doc: Doc = serde_json::from_value(root).map_err(|e| e.to_string())?;
    Ok((doc, warn))
}

/// Turns one list of hand-written elements into native ones. String ids are local to the list
/// (each page may reuse "api"); `all_keys` collects them for scenario steps (first one wins).
fn convert(els: &[Value], next: &mut u64, all_keys: &mut HashMap<String, u64>, warn: &mut Vec<String>, native: bool, at: &str) -> Vec<Value> {
    let mut keys: HashMap<String, u64> = HashMap::new();
    let mut ids = vec![];
    for (i, e) in els.iter().enumerate() {
        let id = match e.get("id") {
            Some(Value::Number(n)) if n.as_u64().is_some() => n.as_u64().unwrap(),
            other => {
                *next += 1;
                if let Some(Value::String(s)) = other {
                    if keys.insert(s.clone(), *next).is_some() {
                        warn.push(format!("{at}element {i}: id \"{s}\" is used twice"));
                    }
                }
                *next
            }
        };
        ids.push(id);
    }
    let mut out = vec![];
    for (i, (raw, id)) in els.iter().zip(&ids).enumerate() {
        let Some(raw) = raw.as_object() else {
            warn.push(format!("{at}element {i}: not an object, skipped"));
            continue;
        };
        match element(raw, *id, &keys, warn) {
            Ok(e) => out.push(e),
            Err(m) => warn.push(format!("{at}element {i} ({}): {m}, skipped", label(raw))),
        }
    }
    if !native {
        // boundaries sit behind everything else
        out.sort_by_key(|e| e["kind"] != "Region");
    }
    for (k, v) in keys {
        all_keys.entry(k).or_insert(v);
    }
    out
}

fn label(raw: &Map<String, Value>) -> String {
    let s = |k: &str| raw.get(k).and_then(Value::as_str).unwrap_or("");
    let t = [s("title"), s("text"), s("kind")].into_iter().find(|t| !t.is_empty()).unwrap_or("?");
    t.lines().next().unwrap_or("?").to_string()
}

fn element(raw: &Map<String, Value>, id: u64, keys: &HashMap<String, u64>, warn: &mut Vec<String>) -> Result<Value, String> {
    let kind = raw.get("kind").and_then(Value::as_str).ok_or("missing \"kind\"")?;
    let kind = parse_kind(kind).ok_or_else(|| format!("unknown kind \"{kind}\" (use node, rect, ellipse, diamond, text, region, arrow, sticky, comment)"))?;
    let num = |k: &str| raw.get(k).and_then(Value::as_f64).map(|v| v as f32);
    let icon = raw.get("icon").and_then(Value::as_str).unwrap_or("");

    // start from what the app would create, then lay the given fields over it
    let (dw, dh) = match kind {
        Kind::Node | Kind::Rect | Kind::Ellipse | Kind::Diamond => (NODE_SIZE.x, NODE_SIZE.y),
        Kind::Region => (480.0, 320.0),
        Kind::Sticky => (180.0, 140.0),
        Kind::Text => (120.0, 24.0),
        _ => (160.0, 80.0),
    };
    let rect = match raw.get("rect") {
        Some(Value::Array(a)) if a.len() == 4 => {
            let f = |i: usize| a[i].as_f64().unwrap_or(0.0) as f32;
            Some(Rect::from_min_size(pos2(f(0), f(1)), vec2(f(2), f(3))))
        }
        Some(Value::Object(r)) if r.contains_key("min") => serde_json::from_value(Value::Object(r.clone())).ok(),
        Some(Value::Object(r)) if r.contains_key("x") => {
            let f = |k: &str, d: f32| r.get(k).and_then(Value::as_f64).map_or(d, |v| v as f32);
            Some(Rect::from_min_size(pos2(f("x", 0.0), f("y", 0.0)), vec2(f("w", dw), f("h", dh))))
        }
        _ => None,
    };
    let rect = rect.or_else(|| {
        let (x, y) = (num("x")?, num("y")?);
        Some(Rect::from_min_size(pos2(x, y), vec2(num("w").unwrap_or(dw), num("h").unwrap_or(dh))))
    });
    let rect = rect.unwrap_or_else(|| Rect::from_min_size(pos2(0.0, 0.0), vec2(dw, dh)));
    if !["rect", "x", "y"].iter().any(|k| raw.contains_key(*k)) && !matches!(kind, Kind::Arrow | Kind::Pen) {
        warn.push(format!("\"{}\": no position given, placed at 0,0", label(raw)));
    }
    let mut base = if kind == Kind::Node {
        let mut e = Element::node(icon, rect.center());
        e.rect = rect;
        e
    } else {
        Element::new(kind, rect)
    };
    // boundary presets from the library: "preset": "vpc" / "private subnet" / "b-k8s"
    if let Some(p) = raw.get("preset").and_then(Value::as_str) {
        let q = squash(p.trim_start_matches("b-"));
        match crate::icons::BOUNDARIES.iter().find(|b| squash(&b.0[2..]) == q || squash(b.1) == q || squash(b.1).starts_with(&q)) {
            Some(&(_, name, glyph, c, dashed)) => {
                let c = crate::icons::boundary_color(c);
                base.text = name.into();
                base.icon = glyph.into();
                base.style.stroke = c;
                base.style.fill = c.gamma_multiply(if dashed { 0.05 } else { 0.08 });
                base.style.dashed = dashed;
            }
            None => warn.push(format!("\"{}\": unknown boundary preset \"{p}\"", label(raw))),
        }
    }
    let mut v = serde_json::to_value(&base).map_err(|e| e.to_string())?;
    let known: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
    let o = v.as_object_mut().unwrap();

    let style = o.get_mut("style").unwrap().as_object_mut().unwrap();
    if let Some(Value::Object(s)) = raw.get("style") {
        for (k, val) in s {
            style.insert(k.clone(), val.clone());
        }
    }
    for k in STYLE_KEYS {
        if let Some(val) = raw.get(k) {
            style.insert(if k == "color" { "stroke".into() } else { k.into() }, val.clone());
        }
    }
    for k in ["stroke", "fill", "text_color"] {
        if let Some(c) = style.get(k).cloned() {
            match color(&c) {
                Some(c) => style.insert(k.into(), c),
                None if c.is_null() => None,
                None => {
                    warn.push(format!("\"{}\": can't read colour {c}", label(raw)));
                    style.remove(k)
                }
            };
        }
    }
    for (k, names) in [
        ("route", &["Elbow", "Straight", "Curve"][..]),
        ("flow", &["None", "Dots", "Dashes", "Request", "Async", "Batch", "Heartbeat", "Errors", "Balanced", "Fanout"][..]),
        ("pattern", &["Solid", "Dotted", "DashDot", "Double"][..]),
        ("face", &["Auto", "Sans", "Hand", "Mono", "Serif"][..]),
    ] {
        if let Some(val) = style.get_mut(k) {
            *val = variant(val, names);
        }
    }
    if let Some(Value::String(f)) = style.get("flow") {
        // friendly names for the animations
        let f = match squash(f).as_str() {
            "packets" | "dot" | "stream" | "comet" => "Dots",
            "dash" | "marching" => "Dashes",
            "off" | "none" => "None",
            "requestreply" | "sync" | "rpc" | "call" | "reply" | "signal" => "Request",
            "message" | "event" | "fireandforget" => "Async",
            "pubsub" | "broadcast" => "Fanout",
            "roundrobin" | "lb" | "loadbalanced" => "Balanced",
            "health" | "healthcheck" | "ping" | "keepalive" => "Heartbeat",
            "fail" | "failing" | "dropped" | "throttled" | "rejected" | "somefail" => "Errors",
            _ => "",
        };
        if !f.is_empty() {
            style.insert("flow".into(), json!(f));
        }
    }
    if let Some(n) = style.get("font").and_then(Value::as_str) {
        let px = match n.to_lowercase().as_str() {
            "s" | "small" => 13.0,
            "l" | "large" => 24.0,
            "xl" => 36.0,
            _ => 18.0,
        };
        style.insert("font".into(), json!(px));
    }

    // a coloured boundary without its own fill gets a faint tint of its colour
    if kind == Kind::Region && raw.contains_key("stroke") && !raw.contains_key("fill") {
        if let Ok(c) = serde_json::from_value::<Color32>(style["stroke"].clone()) {
            style.insert("fill".into(), serde_json::to_value(c.gamma_multiply(0.06)).unwrap());
        }
    }
    let skip = ["id", "kind", "style", "rect", "x", "y", "w", "h", "title", "subtitle", "from", "to", "preset"];
    for (k, val) in raw {
        if skip.contains(&k.as_str()) || STYLE_KEYS.contains(&k.as_str()) {
            continue;
        }
        if !known.contains(k) {
            warn.push(format!("\"{}\": unknown field \"{k}\" ignored", label(raw)));
            continue;
        }
        o.insert(k.clone(), val.clone());
    }
    if let Some(t) = raw.get("title").and_then(Value::as_str) {
        let sub = raw.get("subtitle").and_then(Value::as_str).unwrap_or("");
        o.insert("text".into(), json!(if sub.is_empty() { t.to_string() } else { format!("{t}\n{sub}") }));
    }
    for k in ["tpos", "icon_pos"] {
        if let Some(val) = o.get_mut(k).filter(|v| !v.is_null()) {
            *val = variant(val, &["TopLeft", "Top", "TopRight", "Left", "Center", "Right", "BottomLeft", "Bottom", "BottomRight"]);
        }
    }
    if let Some(val) = o.get_mut("speed") {
        *val = variant(val, &["Slow", "Normal", "Fast"]);
    }
    if let Some(Value::String(s)) = raw.get("id") {
        o.insert("key".into(), json!(s));
    }
    o.insert("id".into(), json!(id));
    for k in ["from", "to"] {
        if let Some(val) = raw.get(k) {
            o.insert(k.into(), end(val, keys).map_err(|m| format!("\"{k}\": {m}"))?);
        }
    }
    Ok(v)
}

fn end(v: &Value, keys: &HashMap<String, u64>) -> Result<Value, String> {
    let point = |a: &[Value]| json!({ "id": null, "p": { "x": a[0].as_f64().unwrap_or(0.0), "y": a[1].as_f64().unwrap_or(0.0) } });
    let by_key = |s: &str| keys.get(s).copied().ok_or_else(|| format!("no element with id \"{s}\""));
    Ok(match v {
        Value::Number(n) => json!({ "id": n, "p": { "x": 0.0, "y": 0.0 } }),
        Value::String(s) => json!({ "id": by_key(s)?, "p": { "x": 0.0, "y": 0.0 } }),
        Value::Array(a) if a.len() == 2 => point(a),
        Value::Object(o) => {
            let id = match o.get("id") {
                Some(Value::String(s)) => json!(by_key(s)?),
                Some(x) => x.clone(),
                None => Value::Null,
            };
            let p = o.get("p").cloned().unwrap_or(json!({ "x": o.get("x").and_then(Value::as_f64).unwrap_or(0.0), "y": o.get("y").and_then(Value::as_f64).unwrap_or(0.0) }));
            let side = match o.get("side") {
                Some(Value::String(s)) => json!(SIDES.iter().position(|x| x.eq_ignore_ascii_case(s)).ok_or_else(|| format!("side \"{s}\" should be top, right, bottom or left"))?),
                Some(x) => x.clone(),
                None => Value::Null,
            };
            json!({ "id": id, "p": p, "side": side })
        }
        _ => return Err("expected an element id, [x, y] or {id, side}".into()),
    })
}

fn parse_kind(s: &str) -> Option<Kind> {
    Some(match squash(s).as_str() {
        "rect" | "rectangle" | "box" => Kind::Rect,
        "ellipse" | "circle" | "oval" => Kind::Ellipse,
        "diamond" | "decision" => Kind::Diamond,
        "text" | "label" => Kind::Text,
        "node" | "component" | "service" | "icon" => Kind::Node,
        "region" | "boundary" | "group" | "zone" => Kind::Region,
        "pen" | "freehand" => Kind::Pen,
        "arrow" | "edge" | "connector" | "line" => Kind::Arrow,
        "sticky" | "note" | "stickynote" => Kind::Sticky,
        "comment" => Kind::Comment,
        _ => return None,
    })
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase()
}

/// `"dash-dot"` → `"DashDot"` for a list of enum variants (left alone when nothing matches).
fn variant(v: &Value, names: &[&str]) -> Value {
    match v.as_str().and_then(|s| names.iter().find(|n| squash(n) == squash(s))) {
        Some(n) => json!(n),
        None => v.clone(),
    }
}

fn color(v: &Value) -> Option<Value> {
    let c = match v {
        Value::String(s) => {
            let h = s.trim_start_matches('#');
            let b = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
            match (h.len(), s.as_str()) {
                (_, "none" | "transparent") => Color32::TRANSPARENT,
                (6, _) => Color32::from_rgb(b(0)?, b(2)?, b(4)?),
                (8, _) => Color32::from_rgba_unmultiplied(b(0)?, b(2)?, b(4)?, b(6)?),
                (3, _) => {
                    let n = |i: usize| u8::from_str_radix(h.get(i..i + 1)?, 16).ok().map(|x| x * 17);
                    Color32::from_rgb(n(0)?, n(1)?, n(2)?)
                }
                _ => return None,
            }
        }
        Value::Array(a) if a.len() >= 3 => return Some(v.clone()),
        _ => return None,
    };
    serde_json::to_value(c).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_written_diagram_loads() {
        let src = r##"{ "elements": [
            { "kind": "boundary", "id": "vpc", "title": "VPC", "x": 0, "y": 0, "w": 600, "h": 300, "stroke": "#2563eb", "pattern": "dash-dot" },
            { "kind": "node", "id": "api", "icon": "api-gateway", "x": 40, "y": 60, "title": "API", "subtitle": "REST" },
            { "kind": "box", "id": "db", "x": 300, "y": 60, "text": "DB", "fill": "#fef3c7", "face": "serif", "tpos": "top_left" },
            { "kind": "edge", "id": "q", "from": "api", "to": {"id": "db", "side": "left"}, "flow": "request-reply", "text": "SQL" },
            { "kind": "region", "preset": "private subnet", "x": 0, "y": 400 }
        ], "scenarios": [{ "name": "read", "steps": [{ "from": "api", "to": "db", "edge": "q", "caption": "query" }] }] }"##;
        let (d, warn) = parse(src).unwrap();
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(d.elements.len(), 5);
        let sub = d.elements.iter().find(|e| e.text == "Private subnet").unwrap();
        assert!(sub.style.dashed && sub.icon == "subnet");
        assert_eq!(d.scenarios[0].steps[0].edge, d.elements.iter().find(|e| e.key == "q").map(|e| e.id));
        assert_eq!(d.elements[0].kind, Kind::Region, "boundaries go behind");
        let api = d.elements.iter().find(|e| e.key == "api").unwrap();
        assert_eq!(api.text, "API\nREST");
        assert_eq!(api.rect.size(), vec2(176.0, 56.0));
        let db = d.elements.iter().find(|e| e.key == "db").unwrap();
        assert_eq!(db.style.fill, Color32::from_rgb(0xfe, 0xf3, 0xc7));
        assert_eq!((db.style.face, db.tpos), (Face::Serif, TextPos::TopLeft));
        let e = d.elements.iter().find(|e| e.kind == Kind::Arrow).unwrap();
        assert_eq!((e.from.id, e.to.id, e.to.side), (Some(api.id), Some(db.id), Some(3)));
        assert_eq!(e.style.flow, Flow::Request);
        assert!(d.next_id >= 4);
    }

    #[test]
    fn pages_load_switch_and_survive_a_save() {
        let src = r#"{ "pages": [
            { "name": "Checkout", "elements": [
                { "kind": "node", "id": "api", "title": "Orders API", "x": 0, "y": 0 },
                { "kind": "node", "id": "db", "title": "Orders DB", "x": 300, "y": 0 },
                { "kind": "edge", "from": "api", "to": "db" } ] },
            { "name": "Login", "elements": [
                { "kind": "node", "id": "api", "title": "Auth API", "x": 0, "y": 0 },
                { "kind": "node", "id": "users", "title": "Users", "x": 300, "y": 0 },
                { "kind": "edge", "from": "api", "to": "users" } ] }
        ] }"#;
        let (mut d, warn) = parse(src).unwrap();
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(d.pages.len(), 2);
        assert_eq!(d.elements.len(), 3, "the first page is open");
        assert!(d.pages[0].elements.is_empty());
        // "api" means a different box on each page
        d.switch_page(1);
        let edge = d.elements.iter().find(|e| e.kind == Kind::Arrow).unwrap();
        assert_eq!(d.get(edge.from.id.unwrap()).unwrap().text, "Auth API");
        // a saved file opens on the same page with everything intact
        let back: Doc = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        assert_eq!((back.page, back.elements.len(), back.pages[0].elements.len()), (1, 3, 3));
        // duplicates get fresh ids and keep their connectors attached
        let j = d.duplicate_page(1);
        d.switch_page(j);
        let edge = d.elements.iter().find(|e| e.kind == Kind::Arrow).unwrap();
        assert!(d.get(edge.to.id.unwrap()).is_some_and(|e| e.text == "Users"));
        assert!(d.page_elements(1).iter().all(|e| d.get(e.id).is_none()), "no shared ids");
        // reorder keeps the open page open; deleting down to one page leaves a plain diagram
        d.move_page(2, 0);
        assert_eq!((d.page, d.pages[0].name.as_str()), (0, "Login copy"));
        d.remove_page(0);
        d.remove_page(1);
        assert!(d.pages.is_empty() && d.page == 0 && d.elements.len() == 3);
        // page names stay unique
        let mut d = Doc::default();
        d.add_page("Page 2");
        d.add_page("Page 3");
        d.remove_page(1); // Page 1, Page 3 left
        assert_eq!(d.next_page_name(), "Page 4", "not a second \"Page 3\"");
        assert_eq!(d.duplicate_page(0), 1);
        assert_eq!(d.unique_page_name("Page 1 copy"), "Page 1 copy 2");
        assert!(d.page_name_taken("page 3", None) && !d.page_name_taken("Page 3", Some(2)));
    }

    #[test]
    fn native_files_round_trip_and_mistakes_are_reported() {
        let mut d = Doc::default();
        d.add(Element::node("aws-s3", pos2(0.0, 0.0)));
        let (back, warn) = parse(&serde_json::to_string(&d).unwrap()).unwrap();
        assert!(warn.is_empty());
        assert_eq!(serde_json::to_string(&back).unwrap(), serde_json::to_string(&d).unwrap());
        let (_, warn) = parse(r#"[{ "kind": "node", "x": 1, "y": 1, "colour": "red" }, { "kind": "blob" }]"#).unwrap();
        assert!(warn.iter().any(|w| w.contains("colour")) && warn.iter().any(|w| w.contains("blob")), "{warn:?}");
        assert!(parse(r#"[{ "kind": "edge", "from": "nope" }]"#).unwrap().1.iter().any(|w| w.contains("nope")));
    }
}
