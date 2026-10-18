//! SVG export; PNG and animated GIF are rasterized from the same SVG.

use crate::icons;
use crate::model::*;
use crate::render::{arrow_state, label_size, scenario_visited, step_title};
use crate::theme;
use eframe::egui::{Color32, Pos2, Rect, pos2, vec2};
use std::fmt::Write;
use std::path::Path;

const FONT: &str = "Avenir Next, Helvetica Neue, Arial, sans-serif";
const HAND: &str = "Noteworthy, Chalkboard SE, Comic Sans MS, cursive";
const PAD: f32 = 32.0;
const DARK_BG: Color32 = Color32::from_rgb(0x12, 0x14, 0x18);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Quality {
    Standard,
    Retina,
    UltraHd,
}

impl Quality {
    /// Pixel scale for a drawing of world-size `b`. 4K = 3840px on the long edge.
    pub fn scale(self, b: Rect) -> f32 {
        match self {
            Quality::Standard => 1.0,
            Quality::Retina => 2.0,
            Quality::UltraHd => 3840.0 / b.width().max(b.height()),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Look {
    pub dark: bool,
    pub failures: bool,
    /// flow animation speed (1.0 = one loop per LOOP_SECS)
    pub speed: f32,
}

struct Ctx<'a> {
    doc: &'a Doc,
    look: Look,
    phase: f32,
}

impl Ctx<'_> {
    fn m(&self, c: Color32) -> Color32 {
        theme::map(c, self.look.dark)
    }
    fn font(&self) -> &'static str {
        if self.doc.sketch { HAND } else { FONT }
    }
}

fn hex(c: Color32) -> String {
    let u = c.to_srgba_unmultiplied();
    format!("#{:02x}{:02x}{:02x}", u[0], u[1], u[2])
}

/// fill/stroke attr with opacity (Color32 is premultiplied; unmultiply for SVG).
fn paint(attr: &str, c: Color32) -> String {
    if c.a() == 0 {
        return format!(r#"{attr}="none""#);
    }
    let a = c.to_srgba_unmultiplied()[3];
    if a == 255 { format!(r#"{attr}="{}""#, hex(c)) } else { format!(r#"{attr}="{}" {attr}-opacity="{:.3}""#, hex(c), a as f32 / 255.0) }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn pts(p: &[Pos2]) -> String {
    p.iter().map(|p| format!("{:.1},{:.1}", p.x, p.y)).collect::<Vec<_>>().join(" ")
}

/// Rough estimate of text width (no font shaping here).
fn est_w(text: &str, size: f32) -> f32 {
    text.split('\n').map(|l| l.chars().count()).max().unwrap_or(0) as f32 * size * 0.55
}

#[allow(clippy::too_many_arguments)]
fn text(o: &mut String, font: &str, lines: &[&str], x: f32, top: f32, size: f32, bold: bool, c: Color32, anchor: f32) {
    let anchor = if anchor == 0.0 {
        "start"
    } else if anchor == 1.0 {
        "end"
    } else {
        "middle"
    };
    for (i, l) in lines.iter().enumerate() {
        let y = top + i as f32 * size * LINE_H + size * LINE_H / 2.0 + size * 0.35;
        let w = if bold { r#" font-weight="600""# } else { "" };
        let _ = write!(o, r#"<text x="{x:.1}" y="{y:.1}" font-family="{font}" font-size="{size}"{w} text-anchor="{anchor}" {}>{}</text>"#, paint("fill", c), esc(l));
    }
}

fn icon_svg(o: &mut String, id: &str, r: Rect, color: Color32) {
    let Some(i) = icons::find(id) else { return };
    if i.color {
        let _ = write!(
            o,
            r#"<image x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" href="data:image/svg+xml;base64,{}"/>"#,
            r.min.x,
            r.min.y,
            r.width(),
            r.height(),
            icons::base64(i.svg.as_bytes())
        );
    } else {
        let _ = write!(
            o,
            r#"<g transform="translate({:.1} {:.1}) scale({:.3})" fill="none" stroke="{}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{}</g>"#,
            r.min.x,
            r.min.y,
            r.width() / 24.0,
            hex(color),
            i.svg
        );
    }
}

fn poly_path(o: &mut String, p: &[Pos2], closed: bool, attrs: &str) {
    let tag = if closed { "polygon" } else { "polyline" };
    let _ = write!(o, r#"<{tag} points="{}" {attrs}/>"#, pts(p));
}

/// Outline as either crisp or two wobbly hand-drawn strokes.
fn outline(o: &mut String, c: &Ctx, p: &[Pos2], closed: bool, stroke: Color32, width: f32, dashed: bool, seed: u64) {
    let dash = if dashed { r#" stroke-dasharray="8 6""# } else { "" };
    let base = format!(r#"fill="none" {} stroke-width="{width}" stroke-linejoin="round" stroke-linecap="round"{dash}"#, paint("stroke", stroke));
    if c.doc.sketch {
        for (i, q) in rough(p, closed, seed, 1.6).iter().enumerate() {
            let a = if i == 1 { base.replace(&format!(r#"stroke-width="{width}""#), &format!(r#"stroke-width="{}""#, width * 0.7)) } else { base.clone() };
            poly_path(o, q, false, &a);
        }
    } else {
        poly_path(o, p, closed, &base);
    }
}

fn fill(o: &mut String, c: &Ctx, p: &[Pos2], color: Color32, seed: u64) {
    if color.a() == 0 {
        return;
    }
    let [r, g, b, _] = color.to_srgba_unmultiplied();
    if c.doc.sketch && (r as u32 + g as u32 + b as u32) <= 3 * 245 {
        for [a, q] in hachure(p, 9.0, seed) {
            let _ = write!(o, r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" {} stroke-width="1.3" stroke-linecap="round"/>"#, a.x, a.y, q.x, q.y, paint("stroke", color));
        }
    } else {
        poly_path(o, p, true, &paint("fill", color));
    }
}

fn header(o: &mut String, b: Rect, bg: Color32) {
    let _ = write!(
        o,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}" viewBox="{x:.1} {y:.1} {w:.1} {h:.1}"><defs><filter id="sh" x="-20%" y="-20%" width="140%" height="160%"><feDropShadow dx="0" dy="3" stdDeviation="5" flood-opacity="0.10"/></filter></defs><rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" {}/>"##,
        paint("fill", bg),
        x = b.min.x,
        y = b.min.y,
        w = b.width(),
        h = b.height()
    );
}

fn export_ids(doc: &Doc, only: &[u64]) -> Vec<u64> {
    doc.elements.iter().filter(|e| e.kind != Kind::Comment && (only.is_empty() || only.contains(&e.id))).map(|e| e.id).collect()
}

pub fn svg(doc: &Doc, only: &[u64], phase: f32, look: Look) -> Option<(String, Rect)> {
    let ids = export_ids(doc, only);
    let b = doc.bounds_of(&ids)?.expand(PAD + 20.0);
    let c = Ctx { doc, look, phase };
    let mut o = String::new();
    header(&mut o, b, if look.dark { DARK_BG } else { Color32::WHITE });
    for e in doc.elements.iter().filter(|e| ids.contains(&e.id)) {
        element(&mut o, &c, e);
    }
    o.push_str("</svg>");
    Some((o, b))
}

fn element(o: &mut String, c: &Ctx, e: &Element) {
    let s = &e.style;
    let r = e.rect;
    let down = c.look.failures && e.down;
    let stroke = if down { DOWN_RED } else { c.m(s.stroke) };
    match e.kind {
        Kind::Rect | Kind::Ellipse | Kind::Diamond | Kind::Region | Kind::Sticky => {
            let p = shape_outline(e);
            if e.kind == Kind::Sticky {
                poly_path(o, &p, true, &format!(r#"{} filter="url(#sh)""#, paint("fill", c.m(s.fill))));
                let f = 18.0;
                poly_path(o, &[pos2(r.max.x - f, r.max.y), pos2(r.max.x, r.max.y - f), pos2(r.max.x - f, r.max.y - f)], true, r##"fill="#000000" fill-opacity="0.11""##);
            } else {
                fill(o, c, &p, if down { tint(DOWN_RED, 0.08) } else { c.m(s.fill) }, e.id);
                outline(o, c, &p, true, stroke, s.width, s.dashed, e.id);
            }
            if !e.text.is_empty() {
                let lines: Vec<&str> = e.text.split('\n').collect();
                let has_icon = e.kind == Kind::Region && !e.icon.is_empty();
                let mut size = vec2(est_w(&e.text, s.font), lines.len() as f32 * s.font * LINE_H);
                if has_icon {
                    size.x += 22.0;
                }
                let (mut x, top, ax) = text_place(r, e.tpos, e.outside, size, LABEL_PAD);
                if has_icon {
                    let left = x - size.x * ax;
                    icon_svg(o, &e.icon, Rect::from_min_size(pos2(left, top + (s.font * LINE_H - 16.0) / 2.0), vec2(16.0, 16.0)), stroke);
                    x += 22.0 * (1.0 - ax);
                }
                let color = if e.kind == Kind::Sticky { c.m(INK) } else { stroke };
                text(o, c.font(), &lines, x, top, s.font, e.kind == Kind::Region, color, ax);
            }
        }
        Kind::Text => {
            let lines: Vec<&str> = e.text.split('\n').collect();
            text(o, c.font(), &lines, r.min.x, r.min.y, s.font, false, c.m(s.stroke), 0.0);
        }
        Kind::Node => node(o, c, e),
        Kind::Pen => {
            let _ = write!(o, r#"<polyline points="{}" fill="none" {} stroke-width="{}" stroke-linejoin="round" stroke-linecap="round"/>"#, pts(&e.points), paint("stroke", stroke), s.width);
        }
        Kind::Arrow => arrow(o, c, e),
        Kind::Comment => {}
    }
}

fn node(o: &mut String, c: &Ctx, e: &Element) {
    let s = &e.style;
    let r = e.rect;
    let down = c.look.failures && e.down;
    let accent = if down { DOWN_RED } else { s.stroke };
    let card = if down { c.m(tint(DOWN_RED, 0.06)) } else { c.m(s.fill) };
    let border = if down {
        format!(r#"stroke="{}" stroke-width="2""#, hex(DOWN_RED))
    } else if s.dashed {
        format!(r#"{} stroke-width="1.5""#, paint("stroke", c.m(s.stroke)))
    } else {
        format!(r#"stroke="{}" stroke-width="1""#, hex(c.m(CARD_BORDER)))
    };
    let rect = |o: &mut String, rr: Rect, extra: &str| {
        let _ = write!(o, r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="12" {} {border} {extra}/>"#, rr.min.x, rr.min.y, rr.width(), rr.height(), paint("fill", card));
    };
    for k in (1..e.replicas.min(3)).rev() {
        rect(o, r.translate(vec2(6.0, 6.0) * k as f32), "");
    }
    if c.doc.sketch {
        poly_path(o, &shape_outline(e), true, &paint("fill", card));
        outline(o, c, &shape_outline(e), true, if down { DOWN_RED } else { c.m(INK) }, 1.6, false, e.id);
    } else {
        rect(o, r, r#"filter="url(#sh)""#);
    }
    let l = node_layout(r);
    let color_icon = icons::find(&e.icon).is_some_and(|i| i.color);
    if !color_icon {
        let _ = write!(o, r#"<rect x="{:.1}" y="{:.1}" width="32" height="32" rx="8" {}/>"#, l.tile.min.x, l.tile.min.y, paint("fill", c.m(tint(accent, 0.12))));
    }
    icon_svg(o, &e.icon, if color_icon { l.tile } else { l.icon }, accent);
    let (title, sub) = node_text(&e.text);
    let (tt, st) = node_text_tops(sub.len());
    text(o, c.font(), &[title], l.text_x, r.center().y + tt, NODE_TITLE, true, c.m(INK), 0.0);
    text(o, c.font(), &sub, l.text_x, r.center().y + st, NODE_SUB, false, c.m(MUTED), 0.0);
    let pill = |o: &mut String, center: Pos2, label: &str, bg: Color32| {
        let w = est_w(label, 10.5) + 12.0;
        let _ = write!(o, r#"<rect x="{:.1}" y="{:.1}" width="{w:.1}" height="17" rx="8.5" {}/>"#, center.x - w / 2.0, center.y - 8.5, paint("fill", bg));
        text(o, FONT, &[label], center.x, center.y - 10.5 * LINE_H / 2.0, 10.5, true, Color32::WHITE, 0.5);
    };
    if e.replicas > 1 {
        pill(o, pos2(r.max.x - 6.0, r.min.y), &format!("×{}", e.replicas), INDIGO);
    }
    if down {
        pill(o, pos2(r.min.x + 22.0, r.min.y), "DOWN", DOWN_RED);
    }
    if let (true, Some(cap)) = (e.show_cap, e.cap) {
        let b = cap.badge();
        let w = est_w(&b, 10.5) + 12.0;
        let top = r.max.y + node_extras(e).0;
        let _ = write!(
            o,
            r#"<rect x="{:.1}" y="{top:.1}" width="{w:.1}" height="18" rx="4" {} stroke="{}" stroke-width="1"/>"#,
            r.center().x - w / 2.0,
            paint("fill", c.m(Color32::from_rgb(0xf8, 0xfa, 0xfc))),
            hex(c.m(CARD_BORDER))
        );
        text(o, "Menlo, monospace", &[&b], r.center().x, top + 1.0, 10.5, false, c.m(MUTED), 0.5);
    }
    if e.show_notes && !e.notes.trim().is_empty() {
        let lines = node_notes(e);
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        text(o, c.font(), &refs, r.center().x, r.max.y + node_extras(e).1, NOTE_SIZE, false, c.m(MUTED), 0.5);
    }
}

fn arrow(o: &mut String, c: &Ctx, e: &Element) {
    let s = &e.style;
    let (color, dashed, flow) = arrow_state(c.doc, e, c.look.failures);
    let color = c.m(color);
    let (width, speed) = edge_weight(e);
    let phase = c.phase * speed;
    let path = c.doc.arrow_path(e);
    let (line, heads) = arrow_geometry(&path, s, width);
    let base = format!(r#"<polyline points="{}" fill="none" stroke-width="{width:.2}" stroke-linejoin="round" stroke-linecap="round""#, pts(&line));
    match flow {
        Flow::Dashes => {
            let _ = write!(o, r#"{base} {} stroke-dasharray="{DASH} {DASH}" stroke-dashoffset="{:.2}"/>"#, paint("stroke", color), -dash_offset(phase));
        }
        _ if dashed => {
            let _ = write!(o, r#"{base} {} stroke-dasharray="8 6"/>"#, paint("stroke", color));
        }
        Flow::Dots => {
            let _ = write!(o, r#"{base} {}/>"#, paint("stroke", color.gamma_multiply(0.45)));
        }
        Flow::None if c.doc.sketch => outline(o, c, &line, false, color, width, false, e.id),
        Flow::None => {
            let _ = write!(o, r#"{base} {}/>"#, paint("stroke", color));
        }
    }
    for h in heads {
        if c.doc.sketch {
            poly_path(o, &[h[1], h[0], h[2]], false, &format!(r#"fill="none" {} stroke-width="{width}" stroke-linecap="round""#, paint("stroke", color)));
        } else {
            poly_path(o, &h, true, &paint("fill", color));
        }
    }
    if flow == Flow::Dots {
        let rad = 3.0 + width * 0.75;
        for d in flow_dots(&path, phase) {
            let _ = write!(o, r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" {}/>"#, d.x, d.y, rad * 2.0, paint("fill", color.gamma_multiply(0.18)));
            let _ = write!(o, r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" {}/>"#, d.x, d.y, rad, paint("fill", color));
        }
    }
    if !e.text.is_empty() {
        let size = label_size(e);
        let lines: Vec<&str> = e.text.split('\n').collect();
        let w = est_w(&e.text, size) + 14.0;
        let h = lines.len() as f32 * size * LINE_H + 4.0;
        let m = point_at(&path, path_len(&path) / 2.0);
        let _ = write!(
            o,
            r#"<rect x="{:.1}" y="{:.1}" width="{w:.1}" height="{h:.1}" rx="{:.1}" {} {} stroke-width="1"/>"#,
            m.x - w / 2.0,
            m.y - h / 2.0,
            h / 2.0,
            paint("fill", c.m(Color32::WHITE)),
            paint("stroke", color.gamma_multiply(0.3))
        );
        text(o, c.font(), &lines, m.x, m.y - (h - 4.0) / 2.0, size, false, color, 0.5);
    }
}

/// One frame of a scenario at time `t`, framed with room for the caption card.
pub fn scenario_svg(doc: &Doc, scn: &Scenario, t: f32, look: Look) -> Option<(String, Rect)> {
    let ids = export_ids(doc, &[]);
    let content = doc.bounds_of(&ids)?.expand(PAD + 20.0);
    let b = Rect::from_min_max(content.min, content.max + vec2(0.0, 130.0)).union(Rect::from_center_size(content.center(), vec2(640.0, 0.0)));
    let c = Ctx { doc, look, phase: 0.0 };
    let bg = if look.dark { DARK_BG } else { Color32::WHITE };
    let mut o = String::new();
    header(&mut o, b, bg);
    for e in doc.elements.iter().filter(|e| ids.contains(&e.id)) {
        element(&mut o, &c, e);
    }
    if scn.steps.is_empty() {
        o.push_str("</svg>");
        return Some((o, b));
    }
    let st = scenario_state(scn.steps.len(), t);
    let _ = write!(o, r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" {} fill-opacity="0.78"/>"#, b.min.x, b.min.y, b.width(), b.height(), paint("fill", bg));
    let hl = INDIGO;
    for s in &scn.steps[..st.step] {
        poly_path(&mut o, &doc.step_path(s), false, &format!(r#"fill="none" stroke="{}" stroke-opacity="0.55" stroke-width="3" stroke-linecap="round" stroke-linejoin="round""#, hex(hl)));
    }
    let visited = scenario_visited(scn, &st);
    for e in doc.elements.iter().filter(|e| visited.contains(&e.id)) {
        element(&mut o, &c, e);
    }
    let cur = &scn.steps[st.step];
    let path = doc.step_path(cur);
    if path.len() >= 2 {
        let l = path_len(&path);
        let tt = st.travel.clamp(0.0, 1.0);
        let d = l * tt * tt * (3.0 - 2.0 * tt);
        let mut part = vec![path[0]];
        let mut acc = 0.0;
        for w in path.windows(2) {
            acc += w[0].distance(w[1]);
            if acc < d {
                part.push(w[1]);
            }
        }
        part.push(point_at(&path, d));
        poly_path(&mut o, &part, false, &format!(r#"fill="none" stroke="{}" stroke-width="3" stroke-linecap="round" stroke-linejoin="round""#, hex(hl)));
        if st.since_arrive < 0.0 {
            for k in 0..4 {
                let q = point_at(&path, (d - k as f32 * 9.0).max(0.0));
                let a = 1.0 - k as f32 * 0.25;
                let _ = write!(o, r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" fill-opacity="{:.2}"/>"#, q.x, q.y, 7.0 - k as f32, hex(hl), 0.25 * a);
                let _ = write!(o, r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" fill-opacity="{a:.2}"/>"#, q.x, q.y, 4.5 - k as f32 * 0.8, hex(hl));
            }
        }
    }
    if st.since_arrive >= 0.0 {
        if let Some(tg) = doc.get(cur.to) {
            let k = ease(st.since_arrive / 0.7);
            let rr = tg.rect.expand(4.0 + k * 18.0);
            let _ = write!(
                o,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" fill="none" stroke="{}" stroke-opacity="{:.2}" stroke-width="2.5"/>"#,
                rr.min.x,
                rr.min.y,
                rr.width(),
                rr.height(),
                14.0 + k * 10.0,
                hex(hl),
                1.0 - k
            );
        }
    }
    let caption = if cur.caption.is_empty() { step_title(doc, cur) } else { cur.caption.clone() };
    let w = (est_w(&caption, 15.0) + 40.0).max(260.0);
    let card = Rect::from_center_size(pos2(b.center().x, b.max.y - 62.0), vec2(w, 72.0));
    let (pb, pbr) = if look.dark { (Color32::from_rgb(0x1c, 0x1f, 0x26), Color32::from_rgb(0x2e, 0x33, 0x3d)) } else { (Color32::WHITE, Color32::from_rgb(0xe8, 0xe9, 0xee)) };
    let _ = write!(
        o,
        r#"<rect x="{:.1}" y="{:.1}" width="{w:.1}" height="72" rx="14" {} stroke="{}" filter="url(#sh)"/>"#,
        card.min.x,
        card.min.y,
        paint("fill", pb),
        hex(pbr)
    );
    let head = format!("{}  ·  STEP {} / {}", scn.name.to_uppercase(), st.step + 1, scn.steps.len());
    text(&mut o, FONT, &[&head], card.min.x + 16.0, card.min.y + 8.0, 10.5, true, hl, 0.0);
    text(&mut o, FONT, &[&caption], card.min.x + 16.0, card.min.y + 32.0, 15.0, false, c.m(INK), 0.0);
    o.push_str("</svg>");
    Some((o, b))
}

fn options() -> resvg::usvg::Options<'static> {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    opt
}

fn rasterize(svg: &str, opt: &resvg::usvg::Options, scale: f32) -> Result<resvg::tiny_skia::Pixmap, String> {
    let tree = resvg::usvg::Tree::from_str(svg, opt).map_err(|e| e.to_string())?;
    let size = tree.size();
    let (w, h) = ((size.width() * scale).round().max(1.0) as u32, (size.height() * scale).round().max(1.0) as u32);
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h).ok_or("image too large")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
    Ok(pm)
}

pub fn write_svg(doc: &Doc, only: &[u64], look: Look, path: &Path) -> Result<(), String> {
    let (s, _) = svg(doc, only, 0.0, look).ok_or("nothing to export")?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn write_png(doc: &Doc, only: &[u64], q: Quality, look: Look, path: &Path) -> Result<(), String> {
    let (s, b) = svg(doc, only, 0.0, look).ok_or("nothing to export")?;
    rasterize(&s, &options(), q.scale(b))?.save_png(path).map_err(|e| e.to_string())
}

fn gif_frames(frames: Vec<String>, b: Rect, q: Quality, fps: f32, path: &Path) -> Result<(), String> {
    let scale = q.scale(b);
    let opt = options();
    let mut file = Some(std::fs::File::create(path).map_err(|e| e.to_string())?);
    let mut enc: Option<gif::Encoder<std::fs::File>> = None;
    for s in frames {
        let pm = rasterize(&s, &opt, scale)?;
        let (w, h) = (pm.width() as u16, pm.height() as u16);
        let enc = match &mut enc {
            Some(e) => e,
            None => {
                let mut e = gif::Encoder::new(file.take().unwrap(), w, h, &[]).map_err(|e| e.to_string())?;
                e.set_repeat(gif::Repeat::Infinite).map_err(|e| e.to_string())?;
                enc.insert(e)
            }
        };
        let mut rgba = pm.take(); // opaque background => premultiplied == straight
        let mut f = gif::Frame::from_rgba_speed(w, h, &mut rgba, 10);
        f.delay = (100.0 / fps).round() as u16;
        enc.write_frame(&f).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Animated GIF of one seamless flow loop (single frame if nothing animates).
pub fn write_gif(doc: &Doc, only: &[u64], q: Quality, look: Look, path: &Path) -> Result<(), String> {
    let animated = doc.elements.iter().any(|e| e.kind == Kind::Arrow && (e.style.flow != Flow::None || e.failover) && (only.is_empty() || only.contains(&e.id)));
    const FPS: f32 = 25.0;
    let speed = if look.speed > 0.0 { look.speed } else { 1.0 };
    let n = if animated { (LOOP_SECS / speed * FPS).round().max(2.0) as usize } else { 1 };
    let (_, b) = svg(doc, only, 0.0, look).ok_or("nothing to export")?;
    let frames = (0..n).map(|i| svg(doc, only, i as f32 / n as f32, look).unwrap().0).collect();
    gif_frames(frames, b, q, FPS, path)
}

/// Animated GIF that plays a scenario once through (then loops).
pub fn write_scenario_gif(doc: &Doc, scn: &Scenario, q: Quality, look: Look, path: &Path) -> Result<(), String> {
    const FPS: f32 = 20.0;
    let len = scenario_len(scn.steps.len());
    let n = (len * FPS) as usize;
    let (_, b) = scenario_svg(doc, scn, 0.0, look).ok_or("nothing to export")?;
    let frames = (0..n).map(|i| scenario_svg(doc, scn, i as f32 / FPS, look).unwrap().0).collect();
    gif_frames(frames, b, q, FPS, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Doc {
        let mut d = Doc::default();
        let a = d.add(Element::node("aws-rds", pos2(0., 0.)));
        let b = d.add(Element::node("server", pos2(300., 120.)));
        let mut arr = Element::new(Kind::Arrow, Rect::ZERO);
        arr.from.id = Some(a);
        arr.to.id = Some(b);
        arr.style.flow = Flow::Dots;
        arr.text = "SQL · 2k rps".into();
        d.add(arr);
        let mut r = Element::new(Kind::Rect, Rect::from_min_size(pos2(0., 200.), vec2(120., 60.)));
        r.text = "below".into();
        r.tpos = TextPos::Bottom;
        r.outside = true;
        d.add(r);
        d.scenarios.push(Scenario { name: "read".into(), steps: vec![Step { from: b, to: a, caption: "query".into() }] });
        d
    }

    #[test]
    fn exports_png_gif_and_scenario() {
        let d = sample();
        let dir = std::env::temp_dir();
        write_png(&d, &[], Quality::UltraHd, Look::default(), &dir.join("idraw-test.png")).unwrap();
        let img = std::fs::read(dir.join("idraw-test.png")).unwrap();
        let w = u32::from_be_bytes(img[16..20].try_into().unwrap());
        assert_eq!(w, 3840, "4K export should be 3840px wide");
        write_gif(&d, &[], Quality::Standard, Look::default(), &dir.join("idraw-test.gif")).unwrap();
        write_scenario_gif(&d, &d.scenarios[0], Quality::Standard, Look { dark: true, failures: false, speed: 1.0 }, &dir.join("idraw-scn.gif")).unwrap();
        assert!(std::fs::metadata(dir.join("idraw-scn.gif")).unwrap().len() > 1000);
    }

    #[test]
    fn sketch_and_dark_svg_render() {
        let mut d = sample();
        d.sketch = true;
        let (s, _) = svg(&d, &[], 0.0, Look { dark: true, failures: true, speed: 1.0 }).unwrap();
        resvg::usvg::Tree::from_str(&s, &Default::default()).unwrap();
    }
}

