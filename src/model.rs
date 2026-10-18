//! Document model + all geometry shared by the live canvas and exporters.

use eframe::egui::{Color32, Pos2, Rect, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Kind {
    Rect,
    Ellipse,
    Diamond,
    Text,
    Node,
    Region,
    Pen,
    Arrow,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Route {
    Elbow,
    Straight,
    Curve,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Flow {
    None,
    Dots,
    Dashes,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Style {
    pub stroke: Color32,
    pub fill: Color32,
    pub width: f32,
    pub dashed: bool,
    pub round: bool,
    pub font: f32,
    pub route: Route,
    pub head_start: bool,
    pub head_end: bool,
    pub flow: Flow,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            stroke: INK,
            fill: Color32::TRANSPARENT,
            width: 2.0,
            dashed: false,
            round: true,
            font: 18.0,
            route: Route::Elbow,
            head_start: false,
            head_end: true,
            flow: Flow::None,
        }
    }
}

pub const INK: Color32 = Color32::from_rgb(0x1f, 0x29, 0x37);
pub const INDIGO: Color32 = Color32::from_rgb(0x63, 0x66, 0xf1);
pub const MUTED: Color32 = Color32::from_rgb(0x6b, 0x72, 0x80);
pub const CARD_BORDER: Color32 = Color32::from_rgb(0xe5, 0xe7, 0xeb);

#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct End {
    pub id: Option<u64>,
    pub p: Pos2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Element {
    pub id: u64,
    pub kind: Kind,
    pub rect: Rect,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub points: Vec<Pos2>,
    #[serde(default)]
    pub from: End,
    #[serde(default)]
    pub to: End,
    pub style: Style,
}

impl Element {
    pub fn new(kind: Kind, rect: Rect) -> Self {
        let mut style = Style::default();
        match kind {
            Kind::Arrow => style.stroke = INDIGO,
            Kind::Text => style.font = 20.0,
            Kind::Node => style.fill = Color32::WHITE,
            Kind::Region => {
                style.stroke = INDIGO;
                style.fill = INDIGO.gamma_multiply(0.06);
                style.dashed = true;
                style.width = 1.5;
                style.font = 13.0;
            }
            _ => {}
        }
        Element { id: 0, kind, rect, text: String::new(), icon: String::new(), points: vec![], from: End::default(), to: End::default(), style }
    }

    pub fn node(icon: &str, center: Pos2) -> Self {
        let (_, label, cat, _) = crate::icons::find(icon).copied().unwrap_or(("", "Component", "", ""));
        let mut e = Element::new(Kind::Node, Rect::from_center_size(center, vec2(168.0, 56.0)));
        e.icon = icon.to_string();
        e.text = label.to_string();
        e.style.stroke = crate::icons::accent(cat);
        e
    }

    /// Can arrows attach to this element?
    pub fn bindable(&self) -> bool {
        !matches!(self.kind, Kind::Pen | Kind::Arrow | Kind::Region)
    }

    pub fn translate(&mut self, d: Vec2) {
        self.rect = self.rect.translate(d);
        for p in &mut self.points {
            *p += d;
        }
        if self.from.id.is_none() {
            self.from.p += d;
        }
        if self.to.id.is_none() {
            self.to.p += d;
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Doc {
    pub elements: Vec<Element>,
    pub next_id: u64,
}

impl Doc {
    pub fn add(&mut self, mut e: Element) -> u64 {
        self.next_id += 1;
        e.id = self.next_id;
        if e.kind == Kind::Region {
            self.elements.insert(0, e); // regions live behind everything
        } else {
            self.elements.push(e);
        }
        self.next_id
    }

    pub fn get(&self, id: u64) -> Option<&Element> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Element> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    pub fn remove(&mut self, ids: &[u64]) {
        // Detach arrows from removed shapes, keeping their current endpoints.
        let fixes: Vec<(usize, Pos2, Pos2)> = self
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == Kind::Arrow)
            .map(|(i, e)| {
                let p = self.arrow_corners(e);
                (i, p[0], *p.last().unwrap())
            })
            .collect();
        for (i, a, b) in fixes {
            let e = &mut self.elements[i];
            if e.from.id.is_some_and(|id| ids.contains(&id)) {
                e.from = End { id: None, p: a };
            }
            if e.to.id.is_some_and(|id| ids.contains(&id)) {
                e.to = End { id: None, p: b };
            }
        }
        self.elements.retain(|e| !ids.contains(&e.id));
    }

    pub fn bounds(&self, e: &Element) -> Rect {
        match e.kind {
            Kind::Pen => bbox(&e.points),
            Kind::Arrow => bbox(&self.arrow_path(e)),
            _ => e.rect,
        }
    }

    pub fn bounds_of(&self, ids: &[u64]) -> Option<Rect> {
        let mut it = self.elements.iter().filter(|e| ids.contains(&e.id)).map(|e| self.bounds(e));
        let first = it.next()?;
        Some(it.fold(first, |a, b| a.union(b)))
    }

    pub fn all_bounds(&self) -> Option<Rect> {
        let ids: Vec<u64> = self.elements.iter().map(|e| e.id).collect();
        self.bounds_of(&ids)
    }

    /// Topmost element under `p`. `tol` in world units.
    pub fn hit(&self, p: Pos2, tol: f32) -> Option<u64> {
        self.elements.iter().rev().find(|e| self.hits(e, p, tol)).map(|e| e.id)
    }

    /// Topmost bindable shape under `p`, excluding `skip`.
    pub fn hit_bindable(&self, p: Pos2, skip: Option<u64>) -> Option<u64> {
        self.elements
            .iter()
            .rev()
            .find(|e| e.bindable() && Some(e.id) != skip && e.rect.expand(4.0).contains(p))
            .map(|e| e.id)
    }

    fn hits(&self, e: &Element, p: Pos2, tol: f32) -> bool {
        match e.kind {
            Kind::Pen => dist_to_path(&e.points, p) <= tol + e.style.width,
            Kind::Arrow => dist_to_path(&self.arrow_path(e), p) <= tol + e.style.width,
            Kind::Region => {
                let r = e.rect;
                r.expand(tol).contains(p) && (!r.shrink(tol).contains(p) || p.y < r.min.y + 28.0)
            }
            _ => e.rect.expand(tol).contains(p),
        }
    }

    fn end_rect(&self, end: &End) -> (Rect, Option<Kind>) {
        match end.id.and_then(|id| self.get(id)) {
            Some(e) => (e.rect, Some(e.kind)),
            None => (Rect::from_center_size(end.p, Vec2::ZERO), None),
        }
    }

    /// Polyline corners of an arrow (before rounding / curving).
    pub fn arrow_corners(&self, e: &Element) -> Vec<Pos2> {
        let (a, ka) = self.end_rect(&e.from);
        let (b, kb) = self.end_rect(&e.to);
        match e.style.route {
            Route::Straight => vec![boundary(a, ka, b.center()), boundary(b, kb, a.center())],
            _ => elbow(a, b),
        }
    }

    /// Final flattened path of an arrow.
    pub fn arrow_path(&self, e: &Element) -> Vec<Pos2> {
        let c = self.arrow_corners(e);
        match e.style.route {
            Route::Curve if c.len() == 4 => (0..=32).map(|i| cubic(c[0], c[1], c[2], c[3], i as f32 / 32.0)).collect(),
            Route::Elbow => round_corners(&c, 14.0),
            _ => c,
        }
    }
}

// ---------- geometry ----------

pub fn bbox(pts: &[Pos2]) -> Rect {
    let mut r = Rect::NOTHING;
    for p in pts {
        r.extend_with(*p);
    }
    if pts.is_empty() { Rect::ZERO } else { r }
}

/// Right-angle route between two boxes (zero-size box = free point).
fn elbow(a: Rect, b: Rect) -> Vec<Pos2> {
    let gx = (b.min.x - a.max.x).max(a.min.x - b.max.x);
    let gy = (b.min.y - a.max.y).max(a.min.y - b.max.y);
    let (ca, cb) = (a.center(), b.center());
    if gx >= gy {
        let right = cb.x >= ca.x;
        let s = pos2(if right { a.max.x } else { a.min.x }, ca.y);
        let t = pos2(if right { b.min.x } else { b.max.x }, cb.y);
        if (s.y - t.y).abs() < 0.5 {
            return vec![s, t];
        }
        let mx = (s.x + t.x) / 2.0;
        vec![s, pos2(mx, s.y), pos2(mx, t.y), t]
    } else {
        let down = cb.y >= ca.y;
        let s = pos2(ca.x, if down { a.max.y } else { a.min.y });
        let t = pos2(cb.x, if down { b.min.y } else { b.max.y });
        if (s.x - t.x).abs() < 0.5 {
            return vec![s, t];
        }
        let my = (s.y + t.y) / 2.0;
        vec![s, pos2(s.x, my), pos2(t.x, my), t]
    }
}

/// Point on a shape's outline in the direction of `target`.
fn boundary(r: Rect, kind: Option<Kind>, target: Pos2) -> Pos2 {
    let c = r.center();
    let d = target - c;
    let (hw, hh) = (r.width() / 2.0, r.height() / 2.0);
    if kind.is_none() || d.length() < 1e-3 || hw < 1e-3 || hh < 1e-3 {
        return c;
    }
    let t = match kind {
        Some(Kind::Ellipse) => 1.0 / ((d.x / hw).powi(2) + (d.y / hh).powi(2)).sqrt(),
        Some(Kind::Diamond) => 1.0 / (d.x.abs() / hw + d.y.abs() / hh),
        _ => 1.0 / (d.x.abs() / hw).max(d.y.abs() / hh),
    };
    c + d * t
}

fn quad(a: Pos2, c: Pos2, b: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    (a.to_vec2() * u * u + c.to_vec2() * 2.0 * u * t + b.to_vec2() * t * t).to_pos2()
}

fn cubic(a: Pos2, b: Pos2, c: Pos2, d: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    (a.to_vec2() * u * u * u + b.to_vec2() * 3.0 * u * u * t + c.to_vec2() * 3.0 * u * t * t + d.to_vec2() * t * t * t).to_pos2()
}

fn round_corners(c: &[Pos2], r: f32) -> Vec<Pos2> {
    if c.len() < 3 {
        return c.to_vec();
    }
    let mut out = vec![c[0]];
    for i in 1..c.len() - 1 {
        let (p0, p1, p2) = (c[i - 1], c[i], c[i + 1]);
        let r = r.min((p1 - p0).length() / 2.0).min((p2 - p1).length() / 2.0);
        if r < 0.5 {
            out.push(p1);
            continue;
        }
        let a = p1 + (p0 - p1).normalized() * r;
        let b = p1 + (p2 - p1).normalized() * r;
        out.extend((0..=6).map(|k| quad(a, p1, b, k as f32 / 6.0)));
    }
    out.push(*c.last().unwrap());
    out
}

fn dist_seg(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() < 1e-6 { 0.0 } else { ((p - a).dot(ab) / ab.length_sq()).clamp(0.0, 1.0) };
    (a + ab * t).distance(p)
}

pub fn dist_to_path(path: &[Pos2], p: Pos2) -> f32 {
    match path.len() {
        0 => f32::MAX,
        1 => path[0].distance(p),
        _ => path.windows(2).map(|w| dist_seg(p, w[0], w[1])).fold(f32::MAX, f32::min),
    }
}

pub fn path_len(path: &[Pos2]) -> f32 {
    path.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Point at distance `d` along path.
pub fn point_at(path: &[Pos2], mut d: f32) -> Pos2 {
    for w in path.windows(2) {
        let l = w[0].distance(w[1]);
        if d <= l && l > 0.0 {
            return w[0] + (w[1] - w[0]) * (d / l);
        }
        d -= l;
    }
    *path.last().unwrap_or(&Pos2::ZERO)
}

/// Shorten path at both ends (to leave room for arrowheads).
fn trim(path: &[Pos2], start: f32, end: f32) -> Vec<Pos2> {
    let l = path_len(path);
    if path.len() < 2 || l <= start + end + 1.0 {
        return path.to_vec();
    }
    let mut out = vec![point_at(path, start)];
    let mut acc = 0.0;
    for w in path.windows(2) {
        acc += w[0].distance(w[1]);
        if acc > start && acc < l - end {
            out.push(w[1]);
        }
    }
    out.push(point_at(path, l - end));
    out
}

/// Arrow stroke path + filled arrowhead triangles.
pub fn arrow_geometry(path: &[Pos2], s: &Style) -> (Vec<Pos2>, Vec<[Pos2; 3]>) {
    let size = 9.0 + s.width * 2.0;
    let mut heads = vec![];
    if path.len() >= 2 {
        let head = |tip: Pos2, from: Pos2| {
            let dir = (tip - from).normalized();
            let perp = vec2(-dir.y, dir.x) * size * 0.5;
            let base = tip - dir * size;
            [tip, base + perp, base - perp]
        };
        let n = path.len();
        if s.head_end {
            heads.push(head(path[n - 1], point_at(path, path_len(path) - size)));
        }
        if s.head_start {
            heads.push(head(path[0], point_at(path, size)));
        }
    }
    let k = size * 0.8;
    let line = trim(path, if s.head_start { k } else { 0.0 }, if s.head_end { k } else { 0.0 });
    (line, heads)
}

/// Loop length for flow animation (GIF loops seamlessly at this period).
pub const LOOP_SECS: f32 = 1.2;
pub const DOT_SPACING: f32 = 60.0;
pub const DASH: f32 = 8.0;

/// Positions of flow dots along a path at animation phase [0,1).
pub fn flow_dots(path: &[Pos2], phase: f32) -> Vec<Pos2> {
    let l = path_len(path);
    let mut d = phase * DOT_SPACING;
    let mut out = vec![];
    while d < l {
        out.push(point_at(path, d));
        d += DOT_SPACING;
    }
    out
}

/// Marching-ants offset for dashed flow.
pub fn dash_offset(phase: f32) -> f32 {
    phase * DASH * 2.0 * 3.0
}

pub fn shape_outline(e: &Element) -> Vec<Pos2> {
    let r = e.rect;
    match e.kind {
        Kind::Ellipse => (0..72)
            .map(|i| {
                let a = i as f32 / 72.0 * std::f32::consts::TAU;
                r.center() + vec2(a.cos() * r.width() / 2.0, a.sin() * r.height() / 2.0)
            })
            .collect(),
        Kind::Diamond => vec![pos2(r.center().x, r.min.y), pos2(r.max.x, r.center().y), pos2(r.center().x, r.max.y), pos2(r.min.x, r.center().y)],
        _ => rounded_rect(r, corner_radius(e)),
    }
}

pub fn corner_radius(e: &Element) -> f32 {
    let r = e.rect;
    match e.kind {
        Kind::Node => 12.0,
        Kind::Region => 16.0,
        Kind::Rect if e.style.round => 12.0f32.min(r.width() / 4.0).min(r.height() / 4.0),
        _ => 0.0,
    }
}

fn rounded_rect(r: Rect, rad: f32) -> Vec<Pos2> {
    if rad < 0.5 {
        return vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
    }
    let corners = [
        (pos2(r.max.x - rad, r.min.y + rad), -90.0f32),
        (pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (pos2(r.min.x + rad, r.max.y - rad), 90.0),
        (pos2(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    let mut out = vec![];
    for (c, start) in corners {
        for k in 0..=8 {
            let a = (start + k as f32 * 90.0 / 8.0).to_radians();
            out.push(c + vec2(a.cos(), a.sin()) * rad);
        }
    }
    out
}

/// Opaque light tint of a color (like Tailwind's 50/100 shades).
pub fn tint(c: Color32, t: f32) -> Color32 {
    let m = |v: u8| (255.0 + (v as f32 - 255.0) * t) as u8;
    Color32::from_rgb(m(c.r()), m(c.g()), m(c.b()))
}

pub struct NodeLayout {
    pub tile: Rect,
    pub icon: Rect,
    pub text_x: f32,
}

pub fn node_layout(r: Rect) -> NodeLayout {
    let tile = Rect::from_min_size(pos2(r.min.x + 12.0, r.center().y - 16.0), vec2(32.0, 32.0));
    NodeLayout { tile, icon: tile.shrink(6.0), text_x: tile.max.x + 10.0 }
}

pub const NODE_TITLE: f32 = 14.0;
pub const NODE_SUB: f32 = 11.5;
pub const LINE_H: f32 = 1.3;

/// Splits node text into (title, subtitle lines).
pub fn node_text(t: &str) -> (&str, Vec<&str>) {
    let mut it = t.lines();
    (it.next().unwrap_or(""), it.collect())
}

/// Vertical offset (from node center) of each text line's top: title first.
pub fn node_text_tops(sub_lines: usize) -> (f32, f32) {
    let th = NODE_TITLE * LINE_H;
    let sh = NODE_SUB * LINE_H;
    let total = th + sh * sub_lines as f32;
    (-total / 2.0, -total / 2.0 + th)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_binds_and_follows() {
        let mut d = Doc::default();
        let a = d.add(Element::new(Kind::Rect, Rect::from_min_size(pos2(0., 0.), vec2(100., 50.))));
        let b = d.add(Element::new(Kind::Rect, Rect::from_min_size(pos2(300., 200.), vec2(100., 50.))));
        let mut arr = Element::new(Kind::Arrow, Rect::ZERO);
        arr.from = End { id: Some(a), p: Pos2::ZERO };
        arr.to = End { id: Some(b), p: Pos2::ZERO };
        let ar = d.add(arr);
        let c = d.arrow_corners(d.get(ar).unwrap());
        assert_eq!(c.first(), Some(&pos2(100., 25.)));
        assert_eq!(c.last(), Some(&pos2(300., 225.)));
        d.get_mut(b).unwrap().translate(vec2(0., -200.));
        let c = d.arrow_corners(d.get(ar).unwrap());
        assert_eq!(c, vec![pos2(100., 25.), pos2(300., 25.)]);
        // deleting a target leaves a free end in place
        d.remove(&[b]);
        let e = d.get(ar).unwrap();
        assert_eq!(e.to, End { id: None, p: pos2(300., 25.) });
    }

    #[test]
    fn hit_prefers_topmost_and_region_border() {
        let mut d = Doc::default();
        let reg = d.add(Element::new(Kind::Region, Rect::from_min_size(pos2(0., 0.), vec2(400., 400.))));
        let n = d.add(Element::new(Kind::Rect, Rect::from_min_size(pos2(100., 100.), vec2(50., 50.))));
        assert_eq!(d.hit(pos2(120., 120.), 4.), Some(n));
        assert_eq!(d.hit(pos2(200., 300.), 4.), None); // empty interior of region
        assert_eq!(d.hit(pos2(200., 10.), 4.), Some(reg)); // title strip
    }
}
