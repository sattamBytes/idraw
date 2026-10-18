//! Live canvas painting with egui.

use crate::icons::{self, Icons};
use crate::model::*;
use crate::theme;
use eframe::egui::{
    Align2, Color32, Context, CornerRadius, FontFamily, FontId, Id, Painter, Pos2, Rect, Shadow, Shape, Stroke, StrokeKind, Vec2, pos2, vec2,
};

pub const UV: Rect = Rect { min: pos2(0., 0.), max: pos2(1., 1.) };

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
    pub fn pts(&self, p: &[Pos2]) -> Vec<Pos2> {
        p.iter().map(|p| self.w2s(*p)).collect()
    }
}

/// Everything the renderer needs besides the doc.
pub struct Opts<'a> {
    pub phase: f32,
    pub dark: bool,
    pub sketch: bool,
    pub failures: bool,
    pub hide_text: Option<u64>,
    /// comments to show expanded (hovered / selected)
    pub open_comments: &'a [u64],
}

pub fn font(size: f32, bold: bool, hand: bool) -> FontId {
    let fam = match (hand, bold) {
        (true, false) => "hand",
        (true, true) => "hand-bold",
        (false, false) => "canvas",
        (false, true) => "canvas-bold",
    };
    FontId::new(size, FontFamily::Name(fam.into()))
}

/// Size of a text block in world units.
pub fn measure(ctx: &Context, text: &str, size: f32, hand: bool) -> Vec2 {
    let lines: Vec<&str> = if text.is_empty() { vec![""] } else { text.split('\n').collect() };
    let w = ctx.fonts(|f| lines.iter().map(|l| f.layout_no_wrap(l.to_string(), font(size, false, hand), Color32::BLACK).size().x).fold(0.0, f32::max));
    vec2(w.max(size * 0.5), lines.len() as f32 * size * LINE_H)
}

/// Draws lines of text. `anchor_x`: 0 left, 0.5 centered, 1 right. `top` is the block top (world).
#[allow(clippy::too_many_arguments)]
pub fn text_lines(p: &Painter, v: &View, lines: &[&str], x: f32, top: f32, size: f32, bold: bool, color: Color32, anchor_x: f32, hand: bool) {
    let f = font(size * v.zoom, bold, hand);
    for (i, l) in lines.iter().enumerate() {
        let g = p.layout_no_wrap(l.to_string(), f.clone(), color);
        let line_top = top + i as f32 * size * LINE_H;
        let pad = (size * LINE_H * v.zoom - g.size().y) / 2.0;
        let s = v.w2s(pos2(x, line_top));
        p.galley(pos2(s.x - g.size().x * anchor_x, s.y + pad), g, color);
    }
}

fn outline(p: &Painter, v: &View, pts: &[Pos2], closed: bool, stroke: Stroke, dashed: bool, sketch: bool, seed: u64) {
    let z = v.zoom;
    let paths: Vec<Vec<Pos2>> = if sketch {
        rough(pts, closed, seed, 1.6).into_iter().map(|q| v.pts(&q)).collect()
    } else {
        let mut q = v.pts(pts);
        if closed && dashed {
            q.push(q[0]);
        }
        vec![q]
    };
    for (i, q) in paths.into_iter().enumerate() {
        let st = if sketch && i == 1 { Stroke::new(stroke.width * 0.7, stroke.color) } else { stroke };
        if dashed {
            p.extend(Shape::dashed_line(&q, st, 8.0 * z, 6.0 * z));
        } else if closed && !sketch {
            p.add(Shape::closed_line(q, st));
        } else {
            p.add(Shape::line(q, st));
        }
    }
}

fn fill(p: &Painter, v: &View, pts: &[Pos2], color: Color32, sketch: bool, seed: u64) {
    if color.a() == 0 {
        return;
    }
    let [r, g, b, _] = color.to_srgba_unmultiplied();
    let light = (r as u32 + g as u32 + b as u32) > 3 * 245;
    if sketch && !light {
        for [a, c] in hachure(pts, 9.0, seed) {
            p.line_segment([v.w2s(a), v.w2s(c)], Stroke::new(1.3 * v.zoom, color));
        }
    } else {
        p.add(Shape::convex_polygon(v.pts(pts), color, Stroke::NONE));
    }
}

fn cr(px: f32) -> CornerRadius {
    CornerRadius::same(px.clamp(0.0, 255.0) as u8)
}

pub fn draw_doc(p: &Painter, ctx: &Context, doc: &Doc, v: &View, icons: &mut Icons, o: &Opts) {
    for e in &doc.elements {
        draw_element(p, ctx, doc, e, v, icons, o);
    }
    // popovers last so nothing covers them
    for e in doc.elements.iter().filter(|e| o.open_comments.contains(&e.id)) {
        let (anchor, txt) = match e.kind {
            Kind::Comment => (v.w2s(e.rect.min) + vec2(20.0, -14.0), if e.text.is_empty() { "Empty comment — double-click to write" } else { e.text.as_str() }),
            Kind::Node if !e.notes.trim().is_empty() && !e.show_notes => (v.w2s(pos2(e.rect.max.x, e.rect.min.y)) + vec2(12.0, 0.0), e.notes.as_str()),
            _ => continue,
        };
        bubble(p, anchor, txt);
    }
}

fn bubble(p: &Painter, at: Pos2, txt: &str) {
    let g = p.layout(txt.to_string(), FontId::proportional(13.0), theme::text(), 280.0);
    let b = Rect::from_min_size(at, g.size() + vec2(20.0, 16.0));
    let sh = Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(36) };
    p.add(sh.as_shape(b, cr(10.0)));
    p.rect(b, cr(10.0), theme::panel_bg(), Stroke::new(1.0, theme::panel_border()), StrokeKind::Inside);
    p.galley(b.min + vec2(10.0, 8.0), g, theme::text());
}

pub fn draw_element(p: &Painter, ctx: &Context, doc: &Doc, e: &Element, v: &View, icons: &mut Icons, o: &Opts) {
    let z = v.zoom;
    let s = &e.style;
    let m = |c: Color32| theme::map(c, o.dark);
    let down = o.failures && e.down;
    let stroke_c = if down { DOWN_RED } else { m(s.stroke) };
    let stroke = Stroke::new(s.width * z, stroke_c);
    let hide = o.hide_text == Some(e.id);
    let hand = o.sketch;
    let r = e.rect;
    match e.kind {
        Kind::Rect | Kind::Ellipse | Kind::Diamond | Kind::Region | Kind::Sticky => {
            let pts = shape_outline(e);
            if e.kind == Kind::Sticky {
                let sh = Shadow { offset: [0, (4.0 * z) as i8], blur: (12.0 * z).min(255.0) as u8, spread: 0, color: Color32::from_black_alpha(30) };
                p.add(sh.as_shape(v.rect(r), cr(4.0 * z)));
                p.add(Shape::convex_polygon(v.pts(&pts), m(s.fill), Stroke::NONE));
                let f = 18.0;
                let fold = [pos2(r.max.x - f, r.max.y), pos2(r.max.x, r.max.y - f), pos2(r.max.x - f, r.max.y - f)];
                p.add(Shape::convex_polygon(v.pts(&fold), Color32::from_black_alpha(28), Stroke::NONE));
            } else {
                fill(p, v, &pts, if down { tint(DOWN_RED, 0.08) } else { m(s.fill) }, o.sketch, e.id);
                outline(p, v, &pts, true, stroke, s.dashed, o.sketch, e.id);
            }
            if !hide && !e.text.is_empty() {
                let bold = e.kind == Kind::Region;
                let lines: Vec<&str> = e.text.split('\n').collect();
                let mut size = measure(ctx, &e.text, s.font, hand);
                let has_icon = e.kind == Kind::Region && !e.icon.is_empty();
                if has_icon {
                    size.x += 22.0;
                }
                let (x, top, ax) = text_place(r, e.tpos, e.outside, size, LABEL_PAD);
                // glide labels between positions
                let dx = ctx.animate_value_with_time(Id::new(("tx", e.id)), x - r.min.x, 0.18);
                let dy = ctx.animate_value_with_time(Id::new(("ty", e.id)), top - r.min.y, 0.18);
                let (mut x, top) = (r.min.x + dx, r.min.y + dy);
                let color = if e.kind == Kind::Sticky { m(INK) } else { stroke_c };
                if has_icon {
                    let left = x - size.x * ax;
                    let ir = Rect::from_min_size(pos2(left, top + (s.font * LINE_H - 16.0) / 2.0), vec2(16.0, 16.0));
                    if let Some(t) = icons.get(ctx, &e.icon) {
                        p.image(t, v.rect(ir), UV, stroke_c);
                    }
                    x += 22.0 * (1.0 - ax);
                }
                text_lines(p, v, &lines, x, top, s.font, bold, color, ax, hand);
            }
        }
        Kind::Text => {
            if !hide {
                let lines: Vec<&str> = e.text.split('\n').collect();
                text_lines(p, v, &lines, r.min.x, r.min.y, s.font, false, m(s.stroke), 0.0, hand);
            }
        }
        Kind::Node => draw_node(p, ctx, e, v, icons, o, hide),
        Kind::Pen => {
            if e.points.len() > 1 {
                p.add(Shape::line(v.pts(&e.points), stroke));
            }
        }
        Kind::Arrow => draw_arrow(p, ctx, doc, e, v, o, hide),
        Kind::Comment => {
            let c = v.w2s(r.min);
            p.circle_filled(c + vec2(0.0, 2.0), 14.0, Color32::from_black_alpha(30));
            p.circle_filled(c, 13.0, m(s.stroke));
            if let Some(t) = icons.get(ctx, "t-comment") {
                p.image(t, Rect::from_center_size(c, vec2(14.0, 14.0)), UV, Color32::WHITE);
            }
        }
    }
}

fn draw_node(p: &Painter, ctx: &Context, e: &Element, v: &View, icons: &mut Icons, o: &Opts, hide: bool) {
    let z = v.zoom;
    let s = &e.style;
    let m = |c: Color32| theme::map(c, o.dark);
    let down = o.failures && e.down;
    let r = e.rect;
    let accent = if down { DOWN_RED } else { s.stroke };
    let card = if down { m(tint(DOWN_RED, 0.06)) } else { m(s.fill) };
    let border = if down {
        Stroke::new(2.0 * z, DOWN_RED)
    } else if s.dashed {
        Stroke::new(1.5 * z, m(s.stroke))
    } else {
        Stroke::new(1.0 * z, m(CARD_BORDER))
    };
    let shadow = Shadow { offset: [0, (3.0 * z).clamp(0.0, 127.0) as i8], blur: (14.0 * z).clamp(0.0, 255.0) as u8, spread: 0, color: Color32::from_black_alpha(if o.dark { 60 } else { 22 }) };
    if let Some(nr) = node_extras(e).notes {
        let sr = v.rect(nr);
        p.add(Shadow { offset: [0, (3.0 * z) as i8], blur: (10.0 * z).min(255.0) as u8, spread: 0, color: Color32::from_black_alpha(if o.dark { 50 } else { 16 }) }.as_shape(sr, cr(10.0 * z)));
        p.rect(sr, cr(10.0 * z), m(tint(accent, 0.05)), Stroke::new(1.0 * z, m(CARD_BORDER)), StrokeKind::Inside);
        let lines = node_notes(e);
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        text_lines(p, v, &refs, nr.min.x + NOTE_PAD, nr.min.y + NOTE_TUCK + NOTE_PAD, NOTE_SIZE, false, m(Color32::from_rgb(0x4b, 0x55, 0x63)), 0.0, o.sketch);
    }
    for k in (1..e.replicas.min(3)).rev() {
        let rr = v.rect(r.translate(Vec2::splat(6.0 * k as f32)));
        p.rect(rr, cr(12.0 * z), card, border, StrokeKind::Inside);
    }
    let sr = v.rect(r);
    if o.sketch {
        let pts = shape_outline(e);
        p.add(Shape::convex_polygon(v.pts(&pts), card, Stroke::NONE));
        outline(p, v, &pts, true, Stroke::new(1.6 * z, if down { DOWN_RED } else { m(INK) }), false, true, e.id);
    } else {
        p.add(shadow.as_shape(sr, cr(12.0 * z)));
        p.rect(sr, cr(12.0 * z), card, border, StrokeKind::Inside);
    }
    let l = node_layout(r);
    let color_icon = icons::find(&e.icon).is_some_and(|i| i.color);
    if !color_icon {
        p.rect_filled(v.rect(l.tile), cr(8.0 * z), m(tint(accent, 0.12)));
    }
    if let Some(tex) = icons.get(ctx, &e.icon) {
        let ir = if color_icon { l.tile } else { l.icon };
        p.image(tex, v.rect(ir), UV, if color_icon { Color32::WHITE } else { accent });
    }
    if !hide {
        let (title, sub) = node_text(&e.text);
        let (tt, st) = node_text_tops(sub.len());
        let cy = r.center().y;
        text_lines(p, v, &[title], l.text_x, cy + tt, NODE_TITLE, true, m(INK), 0.0, o.sketch);
        text_lines(p, v, &sub, l.text_x, cy + st, NODE_SUB, false, m(MUTED), 0.0, o.sketch);
    }
    let pill = |p: &Painter, center: Pos2, label: &str, bg: Color32| {
        let g = p.layout_no_wrap(label.to_string(), FontId::proportional(10.5 * z), Color32::WHITE);
        let b = Rect::from_center_size(center, g.size() + vec2(10.0 * z, 4.0 * z));
        p.rect_filled(b, cr(b.height() / 2.0), bg);
        p.galley(b.center() - g.size() / 2.0, g, Color32::WHITE);
    };
    if e.replicas > 1 {
        pill(p, v.w2s(pos2(r.max.x - 6.0, r.min.y)), &format!("×{}", e.replicas), INDIGO);
    }
    if down {
        pill(p, v.w2s(pos2(r.min.x + 22.0, r.min.y)), "DOWN", DOWN_RED);
    }
    if e.show_cap {
        if let Some(c) = e.cap {
            let g = p.layout_no_wrap(c.badge(), FontId::monospace(10.5 * z), m(MUTED));
            let top = v.w2s(pos2(r.center().x, node_extras(e).badge_top));
            let b = Rect::from_min_size(pos2(top.x - g.size().x / 2.0 - 6.0 * z, top.y), g.size() + vec2(12.0 * z, 4.0 * z));
            p.rect(b, cr(4.0 * z), theme::map(Color32::from_rgb(0xf8, 0xfa, 0xfc), o.dark), Stroke::new(1.0, m(CARD_BORDER)), StrokeKind::Inside);
            p.galley(b.min + vec2(6.0 * z, 2.0 * z), g, m(MUTED));
        }
    }
    if !e.notes.trim().is_empty() {
        if !e.show_notes {
            // small "has notes" marker; full text on hover
            let c = v.w2s(pos2(r.max.x - 12.0, r.max.y - 12.0));
            p.circle_filled(c, 7.0 * z, m(tint(accent, 0.18)));
            p.text(c, Align2::CENTER_CENTER, "i", FontId::proportional(10.0 * z), accent);
        }
    }
}

/// How an arrow looks right now given failure simulation: (color, dashed, flow).
pub fn arrow_state(doc: &Doc, e: &Element, failures: bool) -> (Color32, bool, Flow) {
    let s = &e.style;
    if failures && doc.arrow_dead(e) {
        return (DOWN_RED.gamma_multiply(0.55), true, Flow::None);
    }
    if e.failover {
        // backup paths take over only when something is actually down
        if failures && doc.any_down() {
            return (s.stroke, s.dashed, if s.flow == Flow::None { Flow::Dots } else { s.flow });
        }
        return (s.stroke.gamma_multiply(0.35), true, Flow::None);
    }
    (s.stroke, s.dashed, s.flow)
}

fn draw_arrow(p: &Painter, ctx: &Context, doc: &Doc, e: &Element, v: &View, o: &Opts, hide: bool) {
    let z = v.zoom;
    let s = &e.style;
    let (color, dashed, flow) = arrow_state(doc, e, o.failures);
    let color = theme::map(color, o.dark);
    let (width, speed) = edge_weight(e);
    let stroke = Stroke::new(width * z, color);
    let path = doc.arrow_path(e);
    let (line, heads) = arrow_geometry(&path, s, width);
    let sl = v.pts(&line);
    let phase = o.phase * speed;
    match flow {
        Flow::Dashes => {
            // egui's offset = where the first dash starts, so growing it marches dashes forward.
            let off = dash_offset(phase) % (DASH * 2.0);
            p.extend(Shape::dashed_line_with_offset(&sl, stroke, &[DASH * z], &[DASH * z], off * z));
            if off > DASH {
                p.add(Shape::line_segment([sl[0], v.w2s(point_at(&line, off - DASH))], stroke));
            }
        }
        _ if dashed => p.extend(Shape::dashed_line(&sl, stroke, 8.0 * z, 6.0 * z)),
        Flow::Dots => {
            p.add(Shape::line(sl, Stroke::new(width * z, color.gamma_multiply(0.45))));
        }
        Flow::None if o.sketch => outline(p, v, &line, false, stroke, false, true, e.id),
        Flow::None => {
            p.add(Shape::line(sl, stroke));
        }
    }
    for h in heads {
        if o.sketch {
            p.add(Shape::line(v.pts(&[h[1], h[0], h[2]]), stroke));
        } else {
            p.add(Shape::convex_polygon(v.pts(&h), color, Stroke::NONE));
        }
    }
    if flow == Flow::Dots {
        let rad = (3.0 + width * 0.75) * z;
        for d in flow_dots(&path, phase) {
            let c = v.w2s(d);
            p.circle_filled(c, rad * 2.0, color.gamma_multiply(0.18));
            p.circle_filled(c, rad, color);
        }
    }
    if !hide && !e.text.is_empty() {
        let size = label_size(e);
        let lines: Vec<&str> = e.text.split('\n').collect();
        let mt = measure(ctx, &e.text, size, o.sketch);
        let c = arrow_label_center(&path, e);
        let pill = Rect::from_center_size(c, mt + vec2(14.0, 4.0));
        p.rect(v.rect(pill), cr(pill.height() / 2.0 * z), theme::panel_bg(), Stroke::new(1.0, color.gamma_multiply(0.3)), StrokeKind::Inside);
        text_lines(p, v, &lines, c.x, c.y - mt.y / 2.0, size, false, color, 0.5, o.sketch);
    }
}

pub fn label_size(e: &Element) -> f32 {
    (e.style.font * 0.7).max(11.0)
}

pub fn arrow_label_center(path: &[Pos2], e: &Element) -> Pos2 {
    label_anchor(path, e.label_t, e.label_off)
}

pub fn draw_grid(p: &Painter, rect: Rect, v: &View) {
    let mut step = 24.0;
    while step * v.zoom < 14.0 {
        step *= 2.0;
    }
    let a = v.s2w(rect.min);
    let b = v.s2w(rect.max);
    let col = theme::grid();
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

/// Elements a scenario has "lit up" by time state `st`.
pub fn scenario_visited(scn: &Scenario, st: &ScnState) -> Vec<u64> {
    let mut ids = vec![];
    for (i, s) in scn.steps.iter().enumerate() {
        if i == 0 {
            ids.push(s.from);
        }
        if i < st.step || (i == st.step && st.since_arrive >= 0.0) {
            ids.push(s.to);
        }
        if i == st.step {
            ids.push(s.from);
        }
    }
    ids
}

/// Dims the canvas and plays a scenario on top: lit nodes, travelling packet, arrival pulse.
pub fn draw_scenario(p: &Painter, ctx: &Context, doc: &Doc, v: &View, icons: &mut Icons, o: &Opts, scn: &Scenario, t: f32, canvas: Rect) {
    if scn.steps.is_empty() {
        return;
    }
    let st = scenario_state(scn.steps.len(), t);
    let bg = theme::canvas_bg();
    p.rect_filled(canvas, 0.0, Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), 200));
    let z = v.zoom;
    let hl = theme::INDIGO;
    // trail of completed hops
    for s in &scn.steps[..st.step] {
        p.add(Shape::line(v.pts(&doc.step_path(s)), Stroke::new(3.0 * z, hl.gamma_multiply(0.55))));
    }
    let visited = scenario_visited(scn, &st);
    for e in doc.elements.iter().filter(|e| visited.contains(&e.id)) {
        draw_element(p, ctx, doc, e, v, icons, o);
    }
    let cur = &scn.steps[st.step];
    let path = doc.step_path(cur);
    if path.len() >= 2 {
        let l = path_len(&path);
        let d = l * smooth(st.travel);
        // drawn portion of the current hop
        let mut part = vec![path[0]];
        let mut acc = 0.0;
        for w in path.windows(2) {
            acc += w[0].distance(w[1]);
            if acc < d {
                part.push(w[1]);
            }
        }
        part.push(point_at(&path, d));
        p.add(Shape::line(v.pts(&part), Stroke::new(3.0 * z, hl)));
        if st.since_arrive < 0.0 {
            for k in 0..4 {
                let q = v.w2s(point_at(&path, (d - k as f32 * 9.0).max(0.0)));
                let a = 1.0 - k as f32 * 0.25;
                p.circle_filled(q, (7.0 - k as f32) * z, hl.gamma_multiply(0.25 * a));
                p.circle_filled(q, (4.5 - k as f32 * 0.8) * z, hl.gamma_multiply(a));
            }
        }
    }
    if st.since_arrive >= 0.0 {
        if let Some(target) = doc.get(cur.to) {
            let k = ease(st.since_arrive / 0.7);
            let rr = v.rect(target.rect).expand(4.0 + k * 18.0 * z);
            p.rect_stroke(rr, cr(14.0 * z + k * 10.0), Stroke::new(2.5, hl.gamma_multiply(1.0 - k)), StrokeKind::Outside);
        }
    }
    // caption card
    let caption = if cur.caption.is_empty() { step_title(doc, cur) } else { cur.caption.clone() };
    let head = format!("{} / {}", st.step + 1, scn.steps.len());
    let g = p.layout(caption, FontId::proportional(15.0), theme::text(), 520.0);
    let w = g.size().x.max(220.0) + 32.0;
    let card = Rect::from_center_size(pos2(canvas.center().x, canvas.max.y - 110.0), vec2(w, g.size().y + 50.0));
    let sh = Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(40) };
    p.add(sh.as_shape(card, cr(14.0)));
    p.rect(card, cr(14.0), theme::panel_bg(), Stroke::new(1.0, theme::panel_border()), StrokeKind::Inside);
    p.text(card.min + vec2(16.0, 12.0), Align2::LEFT_TOP, format!("{}  ·  STEP {head}", scn.name.to_uppercase()), FontId::proportional(10.5), hl);
    p.galley(card.min + vec2(16.0, 32.0), g, theme::text());
    // progress dots
    let n = scn.steps.len();
    for i in 0..n {
        let c = pos2(card.max.x - 16.0 - (n - 1 - i) as f32 * 12.0, card.min.y + 18.0);
        p.circle_filled(c, if i == st.step { 4.0 } else { 3.0 }, if i <= st.step { hl } else { theme::panel_border() });
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn step_title(doc: &Doc, s: &Step) -> String {
    let name = |id| doc.get(id).map(|e| node_text(&e.text).0.to_string()).filter(|s| !s.is_empty()).unwrap_or("?".into());
    format!("{} → {}", name(s.from), name(s.to))
}
