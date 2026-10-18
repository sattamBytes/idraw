//! Live canvas painting with egui.

use crate::icons::Icons;
use crate::model::*;
use eframe::egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Painter, Pos2, Rect, Shadow, Shape, Stroke, StrokeKind, Vec2, pos2, vec2,
};

#[derive(Clone, Copy)]
pub struct View {
    pub origin: Pos2,
    pub zoom: f32,
}

impl View {
    pub fn w2s(&self, p: Pos2) -> Pos2 {
        self.origin + p.to_vec2() * self.zoom
    }
    pub fn s2w(&self, p: Pos2) -> Pos2 {
        ((p - self.origin) / self.zoom).to_pos2()
    }
    pub fn rect(&self, r: Rect) -> Rect {
        Rect::from_min_max(self.w2s(r.min), self.w2s(r.max))
    }
    fn pts(&self, p: &[Pos2]) -> Vec<Pos2> {
        p.iter().map(|p| self.w2s(*p)).collect()
    }
}

pub fn font(size: f32, bold: bool) -> FontId {
    FontId::new(size, FontFamily::Name(if bold { "canvas-bold" } else { "canvas" }.into()))
}

/// Size of a text block in world units.
pub fn measure(ctx: &Context, text: &str, size: f32) -> Vec2 {
    let lines: Vec<&str> = if text.is_empty() { vec![""] } else { text.split('\n').collect() };
    let w = ctx.fonts(|f| lines.iter().map(|l| f.layout_no_wrap(l.to_string(), font(size, false), Color32::BLACK).size().x).fold(0.0, f32::max));
    vec2(w.max(size * 0.5), lines.len() as f32 * size * LINE_H)
}

/// Draws lines of text. `anchor_x`: 0 = left, 0.5 = centered. `top` is the block's top.
fn text_lines(p: &Painter, v: &View, lines: &[&str], x: f32, top: f32, size: f32, bold: bool, color: Color32, anchor_x: f32) {
    let f = font(size * v.zoom, bold);
    for (i, l) in lines.iter().enumerate() {
        let g = p.layout_no_wrap(l.to_string(), f.clone(), color);
        let line_top = top + i as f32 * size * LINE_H;
        let pad = (size * LINE_H * v.zoom - g.size().y) / 2.0;
        let s = v.w2s(pos2(x, line_top));
        p.galley(pos2(s.x - g.size().x * anchor_x, s.y + pad), g, color);
    }
}

fn stroke_outline(p: &Painter, pts: Vec<Pos2>, stroke: Stroke, dashed: bool, zoom: f32) {
    if dashed {
        let mut closed = pts;
        closed.push(closed[0]);
        p.extend(Shape::dashed_line(&closed, stroke, 8.0 * zoom, 6.0 * zoom));
    } else {
        p.add(Shape::closed_line(pts, stroke));
    }
}

pub fn draw_doc(p: &Painter, ctx: &Context, doc: &Doc, v: &View, icons: &mut Icons, phase: f32, hide_text: Option<u64>) {
    for e in &doc.elements {
        draw_element(p, ctx, doc, e, v, icons, phase, hide_text == Some(e.id));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_element(p: &Painter, ctx: &Context, doc: &Doc, e: &Element, v: &View, icons: &mut Icons, phase: f32, hide_text: bool) {
    let z = v.zoom;
    let s = &e.style;
    let stroke = Stroke::new(s.width * z, s.stroke);
    let r = e.rect;
    match e.kind {
        Kind::Rect | Kind::Ellipse | Kind::Diamond => {
            let pts = v.pts(&shape_outline(e));
            if s.fill.a() > 0 {
                p.add(Shape::convex_polygon(pts.clone(), s.fill, Stroke::NONE));
            }
            stroke_outline(p, pts, stroke, s.dashed, z);
            if !hide_text && !e.text.is_empty() {
                let lines: Vec<&str> = e.text.split('\n').collect();
                let top = r.center().y - lines.len() as f32 * s.font * LINE_H / 2.0;
                text_lines(p, v, &lines, r.center().x, top, s.font, false, s.stroke, 0.5);
            }
        }
        Kind::Text => {
            if !hide_text {
                let lines: Vec<&str> = e.text.split('\n').collect();
                text_lines(p, v, &lines, r.min.x, r.min.y, s.font, false, s.stroke, 0.0);
            }
        }
        Kind::Region => {
            let pts = v.pts(&shape_outline(e));
            p.add(Shape::convex_polygon(pts.clone(), s.fill, Stroke::NONE));
            stroke_outline(p, pts, stroke, s.dashed, z);
            if !hide_text && !e.text.is_empty() {
                text_lines(p, v, &[e.text.lines().next().unwrap_or("")], r.min.x + 14.0, r.min.y + 8.0, s.font, true, s.stroke, 0.0);
            }
        }
        Kind::Node => {
            let sr = v.rect(r);
            let cr = CornerRadius::same((12.0 * z).clamp(0.0, 255.0) as u8);
            let shadow = Shadow { offset: [0, (3.0 * z).clamp(0.0, 127.0) as i8], blur: (14.0 * z).clamp(0.0, 255.0) as u8, spread: 0, color: Color32::from_black_alpha(22) };
            p.add(shadow.as_shape(sr, cr));
            let border = if s.dashed { Stroke::new(1.5 * z, s.stroke) } else { Stroke::new(1.0 * z, CARD_BORDER) };
            p.rect(sr, cr, s.fill, border, StrokeKind::Inside);
            let l = node_layout(r);
            p.rect_filled(v.rect(l.tile), CornerRadius::same((8.0 * z) as u8), tint(s.stroke, 0.12));
            if let Some(tex) = icons.get(ctx, &e.icon) {
                p.image(tex, v.rect(l.icon), Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)), s.stroke);
            }
            if !hide_text {
                let (title, sub) = node_text(&e.text);
                let (tt, st) = node_text_tops(sub.len());
                let cy = r.center().y;
                text_lines(p, v, &[title], l.text_x, cy + tt, NODE_TITLE, true, INK, 0.0);
                text_lines(p, v, &sub, l.text_x, cy + st, NODE_SUB, false, MUTED, 0.0);
            }
        }
        Kind::Pen => {
            if e.points.len() > 1 {
                p.add(Shape::line(v.pts(&e.points), stroke));
            }
        }
        Kind::Arrow => {
            let path = doc.arrow_path(e);
            let (line, heads) = arrow_geometry(&path, s);
            let sl = v.pts(&line);
            match s.flow {
                Flow::Dashes => p.extend(Shape::dashed_line_with_offset(&sl, stroke, &[DASH * z], &[DASH * z], (DASH * 2.0 - dash_offset(phase) % (DASH * 2.0)) * z)),
                _ if s.dashed => p.extend(Shape::dashed_line(&sl, stroke, 8.0 * z, 6.0 * z)),
                Flow::Dots => {
                    p.add(Shape::line(sl, Stroke::new(s.width * z, s.stroke.gamma_multiply(0.45))));
                }
                Flow::None => {
                    p.add(Shape::line(sl, stroke));
                }
            }
            for h in heads {
                p.add(Shape::convex_polygon(v.pts(&h), s.stroke, Stroke::NONE));
            }
            if s.flow == Flow::Dots {
                let rad = (3.0 + s.width * 0.75) * z;
                for d in flow_dots(&path, phase) {
                    let c = v.w2s(d);
                    p.circle_filled(c, rad * 2.0, s.stroke.gamma_multiply(0.18));
                    p.circle_filled(c, rad, s.stroke);
                }
            }
            if !hide_text && !e.text.is_empty() {
                draw_arrow_label(p, ctx, v, &path, e);
            }
        }
    }
}

pub fn arrow_label_center(path: &[Pos2]) -> Pos2 {
    point_at(path, path_len(path) / 2.0)
}

fn draw_arrow_label(p: &Painter, ctx: &Context, v: &View, path: &[Pos2], e: &Element) {
    let size = (e.style.font * 0.7).max(11.0);
    let lines: Vec<&str> = e.text.split('\n').collect();
    let m = measure(ctx, &e.text, size);
    let c = arrow_label_center(path);
    let pill = Rect::from_center_size(c, m + vec2(14.0, 4.0));
    p.rect(v.rect(pill), CornerRadius::same((pill.height() / 2.0 * v.zoom).min(255.0) as u8), Color32::WHITE, Stroke::new(1.0, e.style.stroke.gamma_multiply(0.3)), StrokeKind::Inside);
    text_lines(p, v, &lines, c.x, c.y - m.y / 2.0, size, false, e.style.stroke, 0.5);
}

pub fn draw_grid(p: &Painter, rect: Rect, v: &View) {
    let mut step = 24.0;
    while step * v.zoom < 14.0 {
        step *= 2.0;
    }
    let a = v.s2w(rect.min);
    let b = v.s2w(rect.max);
    let col = Color32::from_rgb(0xdd, 0xdf, 0xe4);
    let mut x = (a.x / step).floor() * step;
    while x <= b.x {
        let mut y = (a.y / step).floor() * step;
        while y <= b.y {
            p.circle_filled(v.w2s(pos2(x, y)), 1.1, col);
            y += step;
        }
        x += step;
    }
}
