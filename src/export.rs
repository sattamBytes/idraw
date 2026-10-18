//! SVG export; PNG and animated GIF are rasterized from the same SVG.

use crate::icons;
use crate::model::*;
use eframe::egui::{Color32, Pos2, Rect};
use std::fmt::Write;
use std::path::Path;

const FONT: &str = "Avenir Next, Helvetica Neue, Arial, sans-serif";
const PAD: f32 = 32.0;

fn col(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// fill/stroke attr with opacity (Color32 is premultiplied; unmultiply for SVG).
fn paint(attr: &str, c: Color32) -> String {
    if c.a() == 0 {
        return format!(r#"{attr}="none""#);
    }
    let u = c.to_srgba_unmultiplied();
    let hex = format!("#{:02x}{:02x}{:02x}", u[0], u[1], u[2]);
    if u[3] == 255 { format!(r#"{attr}="{hex}""#) } else { format!(r#"{attr}="{hex}" {attr}-opacity="{:.3}""#, u[3] as f32 / 255.0) }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn pts(p: &[Pos2]) -> String {
    p.iter().map(|p| format!("{:.1},{:.1}", p.x, p.y)).collect::<Vec<_>>().join(" ")
}

/// Text block; `top` is block top. Baseline placed ~ like egui's centered row.
fn text(out: &mut String, lines: &[&str], x: f32, top: f32, size: f32, bold: bool, c: Color32, anchor: &str) {
    for (i, l) in lines.iter().enumerate() {
        let y = top + i as f32 * size * LINE_H + size * LINE_H / 2.0 + size * 0.35;
        let w = if bold { r#" font-weight="600""# } else { "" };
        let _ = write!(out, r#"<text x="{x:.1}" y="{y:.1}" font-family="{FONT}" font-size="{size}"{w} text-anchor="{anchor}" {}>{}</text>"#, paint("fill", c), esc(l));
    }
}

pub fn svg(doc: &Doc, only: &[u64], phase: f32) -> Option<(String, Rect)> {
    let ids: Vec<u64> = if only.is_empty() { doc.elements.iter().map(|e| e.id).collect() } else { only.to_vec() };
    let b = doc.bounds_of(&ids)?.expand(PAD);
    let mut o = String::new();
    let _ = write!(
        o,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}" viewBox="{x:.1} {y:.1} {w:.1} {h:.1}"><defs><filter id="sh" x="-20%" y="-20%" width="140%" height="160%"><feDropShadow dx="0" dy="3" stdDeviation="5" flood-opacity="0.10"/></filter></defs><rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="#ffffff"/>"##,
        x = b.min.x,
        y = b.min.y,
        w = b.width(),
        h = b.height()
    );
    for e in doc.elements.iter().filter(|e| ids.contains(&e.id)) {
        element(&mut o, doc, e, phase);
    }
    o.push_str("</svg>");
    Some((o, b))
}

fn element(o: &mut String, doc: &Doc, e: &Element, phase: f32) {
    let s = &e.style;
    let r = e.rect;
    let dash = if s.dashed { r#" stroke-dasharray="8 6""# } else { "" };
    let stroke = format!(r#"{} stroke-width="{}"{dash} stroke-linejoin="round" stroke-linecap="round""#, paint("stroke", s.stroke), s.width);
    match e.kind {
        Kind::Rect | Kind::Region => {
            let _ = write!(o, r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" {} {stroke}/>"#, r.min.x, r.min.y, r.width(), r.height(), corner_radius(e), paint("fill", s.fill));
            if e.kind == Kind::Region {
                if let Some(l) = e.text.lines().next() {
                    text(o, &[l], r.min.x + 14.0, r.min.y + 8.0, s.font, true, s.stroke, "start");
                }
            }
        }
        Kind::Ellipse => {
            let c = r.center();
            let _ = write!(o, r#"<ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" {} {stroke}/>"#, c.x, c.y, r.width() / 2.0, r.height() / 2.0, paint("fill", s.fill));
        }
        Kind::Diamond => {
            let _ = write!(o, r#"<polygon points="{}" {} {stroke}/>"#, pts(&shape_outline(e)), paint("fill", s.fill));
        }
        Kind::Text => {
            let lines: Vec<&str> = e.text.split('\n').collect();
            text(o, &lines, r.min.x, r.min.y, s.font, false, s.stroke, "start");
        }
        Kind::Node => {
            let border = if s.dashed { format!(r#"{} stroke-width="1.5""#, paint("stroke", s.stroke)) } else { format!(r#"stroke="{}" stroke-width="1""#, col(CARD_BORDER)) };
            let _ = write!(o, r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="12" {} {border} filter="url(#sh)"/>"#, r.min.x, r.min.y, r.width(), r.height(), paint("fill", s.fill));
            let l = node_layout(r);
            let _ = write!(o, r#"<rect x="{:.1}" y="{:.1}" width="32" height="32" rx="8" {}/>"#, l.tile.min.x, l.tile.min.y, paint("fill", tint(s.stroke, 0.12)));
            if let Some((_, _, _, inner)) = icons::find(&e.icon) {
                let _ = write!(
                    o,
                    r#"<g transform="translate({:.1} {:.1}) scale({:.3})" fill="none" stroke="{}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{inner}</g>"#,
                    l.icon.min.x,
                    l.icon.min.y,
                    l.icon.width() / 24.0,
                    col(s.stroke)
                );
            }
            let (title, sub) = node_text(&e.text);
            let (tt, st) = node_text_tops(sub.len());
            text(o, &[title], l.text_x, r.center().y + tt, NODE_TITLE, true, INK, "start");
            text(o, &sub, l.text_x, r.center().y + st, NODE_SUB, false, MUTED, "start");
        }
        Kind::Pen => {
            let _ = write!(o, r#"<polyline points="{}" fill="none" {stroke}/>"#, pts(&e.points));
        }
        Kind::Arrow => {
            let path = doc.arrow_path(e);
            let (line, heads) = arrow_geometry(&path, s);
            let base = format!(r#"<polyline points="{}" fill="none" stroke-width="{}" stroke-linejoin="round" stroke-linecap="round""#, pts(&line), s.width);
            match s.flow {
                Flow::Dashes => {
                    let _ = write!(o, r#"{base} {} stroke-dasharray="{DASH} {DASH}" stroke-dashoffset="{:.2}"/>"#, paint("stroke", s.stroke), -dash_offset(phase));
                }
                Flow::Dots => {
                    let _ = write!(o, r#"{base} {}/>"#, paint("stroke", s.stroke.gamma_multiply(0.45)));
                }
                Flow::None => {
                    let _ = write!(o, r#"{base} {}{dash}/>"#, paint("stroke", s.stroke));
                }
            }
            for h in heads {
                let _ = write!(o, r#"<polygon points="{}" {}/>"#, pts(&h), paint("fill", s.stroke));
            }
            if s.flow == Flow::Dots {
                let rad = 3.0 + s.width * 0.75;
                for d in flow_dots(&path, phase) {
                    let _ = write!(o, r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" {}/>"#, d.x, d.y, rad * 2.0, paint("fill", s.stroke.gamma_multiply(0.18)));
                    let _ = write!(o, r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" {}/>"#, d.x, d.y, rad, paint("fill", s.stroke));
                }
            }
            if !e.text.is_empty() {
                let size = (s.font * 0.7).max(11.0);
                let lines: Vec<&str> = e.text.split('\n').collect();
                // ponytail: label width estimated (0.55em/char); exact metrics need font shaping here.
                let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * size * 0.55 + 14.0;
                let h = lines.len() as f32 * size * LINE_H + 4.0;
                let c = point_at(&path, path_len(&path) / 2.0);
                let _ = write!(
                    o,
                    r##"<rect x="{:.1}" y="{:.1}" width="{w:.1}" height="{h:.1}" rx="{:.1}" fill="#ffffff" {} stroke-width="1"/>"##,
                    c.x - w / 2.0,
                    c.y - h / 2.0,
                    h / 2.0,
                    paint("stroke", s.stroke.gamma_multiply(0.3))
                );
                text(o, &lines, c.x, c.y - (h - 4.0) / 2.0, size, false, s.stroke, "middle");
            }
        }
    }
    if matches!(e.kind, Kind::Rect | Kind::Ellipse | Kind::Diamond) && !e.text.is_empty() {
        let lines: Vec<&str> = e.text.split('\n').collect();
        let top = r.center().y - lines.len() as f32 * s.font * LINE_H / 2.0;
        text(o, &lines, r.center().x, top, s.font, false, s.stroke, "middle");
    }
}

fn options() -> resvg::usvg::Options<'static> {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    opt
}

fn rasterize(svg: &str, opt: &resvg::usvg::Options, scale: f32) -> Result<resvg::tiny_skia::Pixmap, String> {
    let tree = resvg::usvg::Tree::from_str(svg, opt).map_err(|e| e.to_string())?;
    let size = tree.size();
    let (w, h) = ((size.width() * scale).ceil() as u32, (size.height() * scale).ceil() as u32);
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h).ok_or("image too large")?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pm.as_mut());
    Ok(pm)
}

pub fn write_svg(doc: &Doc, only: &[u64], path: &Path) -> Result<(), String> {
    let (s, _) = svg(doc, only, 0.0).ok_or("nothing to export")?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn write_png(doc: &Doc, only: &[u64], path: &Path) -> Result<(), String> {
    let (s, _) = svg(doc, only, 0.0).ok_or("nothing to export")?;
    rasterize(&s, &options(), 2.0)?.save_png(path).map_err(|e| e.to_string())
}

/// Animated GIF of one seamless flow loop (single frame if nothing animates).
pub fn write_gif(doc: &Doc, only: &[u64], path: &Path) -> Result<(), String> {
    let animated = doc.elements.iter().any(|e| e.kind == Kind::Arrow && e.style.flow != Flow::None && (only.is_empty() || only.contains(&e.id)));
    const FPS: f32 = 25.0;
    let frames = if animated { (LOOP_SECS * FPS) as usize } else { 1 };
    let (_, b) = svg(doc, only, 0.0).ok_or("nothing to export")?;
    let scale = (1600.0 / b.width().max(b.height())).min(1.5);
    let opt = options();
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc: Option<gif::Encoder<std::fs::File>> = None;
    let mut file = Some(file);
    for i in 0..frames {
        let (s, _) = svg(doc, only, i as f32 / frames as f32).unwrap();
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
        let mut rgba = pm.take(); // opaque white background => premultiplied == straight
        let mut f = gif::Frame::from_rgba_speed(w, h, &mut rgba, 10);
        f.delay = (100.0 / FPS) as u16;
        enc.write_frame(&f).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{pos2, vec2};

    #[test]
    fn exports_png_and_gif() {
        let mut d = Doc::default();
        let a = d.add(Element::node("database", pos2(0., 0.)));
        let b = d.add(Element::node("server", pos2(300., 120.)));
        let mut arr = Element::new(Kind::Arrow, Rect::ZERO);
        arr.from.id = Some(a);
        arr.to.id = Some(b);
        arr.style.flow = Flow::Dots;
        arr.text = "SQL".into();
        d.add(arr);
        d.add(Element::new(Kind::Ellipse, Rect::from_min_size(pos2(0., 200.), vec2(80., 50.))));
        let dir = std::env::temp_dir();
        write_png(&d, &[], &dir.join("idraw-test.png")).unwrap();
        write_gif(&d, &[], &dir.join("idraw-test.gif")).unwrap();
        assert!(std::fs::metadata(dir.join("idraw-test.gif")).unwrap().len() > 1000);
    }
}
