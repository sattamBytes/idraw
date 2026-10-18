//! Document model + all geometry shared by the live canvas and exporters.

use eframe::egui::{Color32, Pos2, Rect, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

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
    Sticky,
    Comment,
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

/// Where a label sits relative to its shape (3x3 grid).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum TextPos {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl TextPos {
    pub const ALL: [TextPos; 9] = [
        TextPos::TopLeft,
        TextPos::Top,
        TextPos::TopRight,
        TextPos::Left,
        TextPos::Center,
        TextPos::Right,
        TextPos::BottomLeft,
        TextPos::Bottom,
        TextPos::BottomRight,
    ];
    fn idx(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap()
    }
    pub fn fx(self) -> f32 {
        (self.idx() % 3) as f32 / 2.0
    }
    pub fn fy(self) -> f32 {
        (self.idx() / 3) as f32 / 2.0
    }
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
pub const DOWN_RED: Color32 = Color32::from_rgb(0xdc, 0x26, 0x26);
pub const STICKY_YELLOW: Color32 = Color32::from_rgb(0xfe, 0xf0, 0x8a);

/// Back-of-envelope capacity inputs for a component.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Capacity {
    pub qps: f32,
    pub kb: f32,
    pub write_pct: f32,
    pub days: f32,
    pub rf: f32,
}

impl Default for Capacity {
    fn default() -> Self {
        Capacity { qps: 1000.0, kb: 2.0, write_pct: 20.0, days: 365.0, rf: 3.0 }
    }
}

pub struct CapOut {
    pub mb_s: f32,
    pub gb_day: f32,
    pub tb_total: f32,
    pub usd_month: f32,
}

/// Storage priced at a typical object-storage rate.
pub const USD_PER_GB_MONTH: f32 = 0.023;

impl Capacity {
    pub fn compute(&self) -> CapOut {
        let mb_s = self.qps * self.kb / 1024.0;
        let gb_day = self.qps * self.write_pct / 100.0 * self.kb * 86400.0 / (1024.0 * 1024.0);
        let gb_total = gb_day * self.days * self.rf;
        CapOut { mb_s, gb_day, tb_total: gb_total / 1024.0, usd_month: gb_total * USD_PER_GB_MONTH }
    }
    pub fn badge(&self) -> String {
        let o = self.compute();
        format!("{} qps · {}", short_num(self.qps), bytes_gb(o.tb_total * 1024.0))
    }
}

pub fn short_num(v: f32) -> String {
    if v >= 1e6 {
        format!("{:.1}M", v / 1e6)
    } else if v >= 1e3 {
        format!("{:.1}k", v / 1e3)
    } else {
        format!("{v:.0}")
    }
}

pub fn bytes_gb(gb: f32) -> String {
    if gb >= 1024.0 * 1024.0 {
        format!("{:.1} PB", gb / 1024.0 / 1024.0)
    } else if gb >= 1024.0 {
        format!("{:.1} TB", gb / 1024.0)
    } else {
        format!("{gb:.1} GB")
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct End {
    pub id: Option<u64>,
    pub p: Pos2,
}

fn one() -> u8 {
    1
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
    #[serde(default)]
    pub tpos: TextPos,
    #[serde(default)]
    pub outside: bool,
    #[serde(default = "one")]
    pub replicas: u8,
    #[serde(default)]
    pub down: bool,
    #[serde(default)]
    pub failover: bool,
    #[serde(default)]
    pub cap: Option<Capacity>,
    #[serde(default)]
    pub show_cap: bool,
    /// Diagram-as-code key (empty = derived from title).
    #[serde(default)]
    pub key: String,
}

impl Element {
    pub fn new(kind: Kind, rect: Rect) -> Self {
        let mut style = Style::default();
        let mut tpos = TextPos::Center;
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
                tpos = TextPos::TopLeft;
            }
            Kind::Sticky => {
                style.fill = STICKY_YELLOW;
                style.font = 16.0;
                tpos = TextPos::TopLeft;
            }
            Kind::Comment => style.stroke = INDIGO,
            _ => {}
        }
        Element {
            id: 0,
            kind,
            rect,
            text: String::new(),
            icon: String::new(),
            points: vec![],
            from: End::default(),
            to: End::default(),
            style,
            tpos,
            outside: false,
            replicas: 1,
            down: false,
            failover: false,
            cap: None,
            show_cap: false,
            key: String::new(),
        }
    }

    pub fn node(icon: &str, center: Pos2) -> Self {
        let mut e = Element::new(Kind::Node, Rect::from_center_size(center, vec2(176.0, 56.0)));
        e.icon = icon.to_string();
        match crate::icons::find(icon) {
            Some(i) => {
                e.text = if i.sub.is_empty() { i.label.to_string() } else { format!("{}\n{}", i.label, i.sub) };
                e.style.stroke = crate::icons::accent(i.cat);
            }
            None => e.text = "Component".into(),
        }
        e
    }

    /// Can arrows attach to this element?
    pub fn bindable(&self) -> bool {
        !matches!(self.kind, Kind::Pen | Kind::Arrow | Kind::Region | Kind::Comment)
    }

    pub fn has_label_pos(&self) -> bool {
        matches!(self.kind, Kind::Rect | Kind::Ellipse | Kind::Diamond | Kind::Region | Kind::Sticky)
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

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Step {
    pub from: u64,
    pub to: u64,
    #[serde(default)]
    pub caption: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct Scenario {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Doc {
    pub elements: Vec<Element>,
    pub next_id: u64,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
    #[serde(default)]
    pub sketch: bool,
    #[serde(skip)]
    route_cache: RefCell<HashMap<u64, (u64, Vec<Pos2>)>>,
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
        for s in &mut self.scenarios {
            s.steps.retain(|st| !ids.contains(&st.from) && !ids.contains(&st.to));
        }
    }

    pub fn bounds(&self, e: &Element) -> Rect {
        match e.kind {
            Kind::Pen => bbox(&e.points),
            Kind::Arrow => bbox(&self.arrow_path(e)),
            Kind::Node if e.replicas > 1 => e.rect.union(e.rect.translate(Vec2::splat(6.0 * (e.replicas.min(3) - 1) as f32))),
            _ => e.rect,
        }
    }

    pub fn bounds_of(&self, ids: &[u64]) -> Option<Rect> {
        let mut it = self.elements.iter().filter(|e| ids.contains(&e.id) && e.kind != Kind::Comment).map(|e| self.bounds(e));
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
        self.elements.iter().rev().find(|e| e.bindable() && Some(e.id) != skip && e.rect.expand(4.0).contains(p)).map(|e| e.id)
    }

    fn hits(&self, e: &Element, p: Pos2, tol: f32) -> bool {
        match e.kind {
            Kind::Pen => dist_to_path(&e.points, p) <= tol + e.style.width,
            Kind::Arrow => dist_to_path(&self.arrow_path(e), p) <= tol + e.style.width,
            Kind::Region => {
                let r = e.rect;
                r.expand(tol).contains(p) && (!r.shrink(tol).contains(p) || p.y < r.min.y + 28.0)
            }
            Kind::Comment => e.rect.min.distance(p) < 14.0 + tol,
            _ => e.rect.expand(tol).contains(p),
        }
    }

    fn end_rect(&self, end: &End) -> (Rect, Option<Kind>) {
        match end.id.and_then(|id| self.get(id)) {
            Some(e) => (e.rect, Some(e.kind)),
            None => (Rect::from_center_size(end.p, Vec2::ZERO), None),
        }
    }

    /// Polyline corners of an arrow (before rounding / curving). Elbows avoid other shapes.
    pub fn arrow_corners(&self, e: &Element) -> Vec<Pos2> {
        let (a, ka) = self.end_rect(&e.from);
        let (b, kb) = self.end_rect(&e.to);
        if e.style.route == Route::Straight {
            return vec![boundary(a, ka, b.center()), boundary(b, kb, a.center())];
        }
        let area = a.union(b).expand(240.0);
        let obstacles: Vec<Rect> = self
            .elements
            .iter()
            .filter(|o| o.bindable() && Some(o.id) != e.from.id && Some(o.id) != e.to.id && area.intersects(o.rect))
            .map(|o| o.rect)
            .collect();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for r in [a, b].iter().chain(&obstacles) {
            for f in [r.min.x, r.min.y, r.max.x, r.max.y] {
                f.to_bits().hash(&mut h);
            }
        }
        let key = h.finish();
        if let Some((k, p)) = self.route_cache.borrow().get(&e.id) {
            if *k == key {
                return p.clone();
            }
        }
        let p = route(a, ka.is_some(), b, kb.is_some(), &obstacles);
        self.route_cache.borrow_mut().insert(e.id, (key, p.clone()));
        p
    }

    /// Final flattened path of an arrow.
    pub fn arrow_path(&self, e: &Element) -> Vec<Pos2> {
        let c = self.arrow_corners(e);
        let n = c.len();
        match e.style.route {
            Route::Curve if n >= 3 => {
                let (c1, c2) = if n == 3 { (c[1], c[1]) } else { (c[1], c[n - 2]) };
                (0..=32).map(|i| cubic(c[0], c1, c2, c[n - 1], i as f32 / 32.0)).collect()
            }
            Route::Elbow => round_corners(&c, 14.0),
            _ => c,
        }
    }

    /// Path a scenario packet travels for one step (follows the connecting arrow if any).
    pub fn step_path(&self, st: &Step) -> Vec<Pos2> {
        for e in self.elements.iter().filter(|e| e.kind == Kind::Arrow) {
            if e.from.id == Some(st.from) && e.to.id == Some(st.to) {
                return self.arrow_path(e);
            }
            if e.from.id == Some(st.to) && e.to.id == Some(st.from) {
                let mut p = self.arrow_path(e);
                p.reverse();
                return p;
            }
        }
        match (self.get(st.from), self.get(st.to)) {
            (Some(a), Some(b)) => vec![boundary(a.rect, Some(a.kind), b.rect.center()), boundary(b.rect, Some(b.kind), a.rect.center())],
            _ => vec![],
        }
    }

    /// Is this arrow attached to a failed component?
    pub fn arrow_dead(&self, e: &Element) -> bool {
        [e.from.id, e.to.id].iter().flatten().any(|id| self.get(*id).is_some_and(|n| n.down))
    }
}

// ---------- routing ----------

const STUB: f32 = 18.0;

fn sides(r: Rect, bound: bool) -> Vec<(Pos2, Vec2)> {
    if !bound {
        return vec![(r.center(), Vec2::ZERO)];
    }
    let c = r.center();
    vec![
        (pos2(c.x, r.min.y), vec2(0.0, -1.0)),
        (pos2(r.max.x, c.y), vec2(1.0, 0.0)),
        (pos2(c.x, r.max.y), vec2(0.0, 1.0)),
        (pos2(r.min.x, c.y), vec2(-1.0, 0.0)),
    ]
}

/// Picks the cheapest right-angle route over all side pairs and L/Z shapes:
/// length + bends, heavily penalising paths through other shapes or back through the endpoints.
// ponytail: candidate search, not a full visibility-graph A*; add that if dense diagrams need it.
fn route(a: Rect, ab: bool, b: Rect, bb: bool, obstacles: &[Rect]) -> Vec<Pos2> {
    let mut best = (f32::MAX, vec![a.center(), b.center()]);
    for (s, ds) in sides(a, ab) {
        for (t, dt) in sides(b, bb) {
            let (s1, t1) = (s + ds * if ab { STUB } else { 0.0 }, t + dt * if bb { STUB } else { 0.0 });
            let (mx, my) = ((s1.x + t1.x) / 2.0, (s1.y + t1.y) / 2.0);
            for mid in [vec![pos2(t1.x, s1.y)], vec![pos2(s1.x, t1.y)], vec![pos2(mx, s1.y), pos2(mx, t1.y)], vec![pos2(s1.x, my), pos2(t1.x, my)]] {
                let mut p = vec![s, s1];
                p.extend(mid);
                p.push(t1);
                p.push(t);
                let p = simplify(p);
                let c = route_cost(&p, a, ab, b, bb, obstacles);
                if c < best.0 - 0.01 {
                    best = (c, p);
                }
            }
        }
    }
    best.1
}

fn simplify(p: Vec<Pos2>) -> Vec<Pos2> {
    let mut out: Vec<Pos2> = vec![];
    for q in p {
        if out.last().is_some_and(|l| l.distance(q) < 0.1) {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            if ((b - a).x * (q - b).y - (b - a).y * (q - b).x).abs() < 0.01 {
                out.pop();
            }
        }
        out.push(q);
    }
    out
}

fn route_cost(p: &[Pos2], a: Rect, ab: bool, b: Rect, bb: bool, obstacles: &[Rect]) -> f32 {
    let mut c = path_len(p) + (p.len().saturating_sub(2)) as f32 * 30.0;
    for w in p.windows(2) {
        for o in obstacles {
            if seg_hits_rect(w[0], w[1], o.expand(6.0)) {
                c += 1500.0;
            }
        }
        if ab && seg_hits_rect(w[0], w[1], a.shrink(2.0)) {
            c += 3000.0;
        }
        if bb && seg_hits_rect(w[0], w[1], b.shrink(2.0)) {
            c += 3000.0;
        }
    }
    c
}

/// Liang–Barsky segment/rect intersection test.
fn seg_hits_rect(p: Pos2, q: Pos2, r: Rect) -> bool {
    if !r.is_positive() {
        return false;
    }
    let d = q - p;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (pk, qk) in [(-d.x, p.x - r.min.x), (d.x, r.max.x - p.x), (-d.y, p.y - r.min.y), (d.y, r.max.y - p.y)] {
        if pk.abs() < 1e-6 {
            if qk < 0.0 {
                return false;
            }
        } else {
            let t = qk / pk;
            if pk < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

// ---------- geometry ----------

pub fn bbox(pts: &[Pos2]) -> Rect {
    let mut r = Rect::NOTHING;
    for p in pts {
        r.extend_with(*p);
    }
    if pts.is_empty() { Rect::ZERO } else { r }
}

/// Point on a shape's outline in the direction of `target`.
pub fn boundary(r: Rect, kind: Option<Kind>, target: Pos2) -> Pos2 {
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

/// Arrow stroke path + arrowhead triangles.
pub fn arrow_geometry(path: &[Pos2], s: &Style, width: f32) -> (Vec<Pos2>, Vec<[Pos2; 3]>) {
    let size = 9.0 + width * 2.0;
    let mut heads = vec![];
    if path.len() >= 2 {
        let head = |tip: Pos2, from: Pos2| {
            let dir = (tip - from).normalized();
            let perp = vec2(-dir.y, dir.x) * size * 0.5;
            let base = tip - dir * size;
            [tip, base + perp, base - perp]
        };
        if s.head_end {
            heads.push(head(path[path.len() - 1], point_at(path, path_len(path) - size)));
        }
        if s.head_start {
            heads.push(head(path[0], point_at(path, size)));
        }
    }
    let k = size * 0.8;
    let line = trim(path, if s.head_start { k } else { 0.0 }, if s.head_end { k } else { 0.0 });
    (line, heads)
}

/// Requests/sec parsed from an edge label like "1.2k rps", "300 qps", "6000 rpm".
pub fn traffic(label: &str) -> Option<f32> {
    let t = label.to_lowercase();
    let toks: Vec<&str> = t.split(|c: char| c.is_whitespace() || c == ',' || c == '·').filter(|s| !s.is_empty()).collect();
    for (i, tok) in toks.iter().enumerate() {
        let num: String = tok.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        let Ok(mut v) = num.parse::<f32>() else { continue };
        let mut rest = &tok[num.len()..];
        if let Some(r) = rest.strip_prefix('k') {
            v *= 1e3;
            rest = r;
        } else if let Some(r) = rest.strip_prefix('m').filter(|r| !r.starts_with("in")) {
            v *= 1e6;
            rest = r;
        }
        let unit = if rest.is_empty() { toks.get(i + 1).copied().unwrap_or("") } else { rest };
        match unit {
            "rps" | "qps" | "req/s" | "tps" | "msg/s" | "/s" | "ops" => return Some(v),
            "rpm" | "req/min" => return Some(v / 60.0),
            _ => {}
        }
    }
    None
}

/// (stroke width, packet speed multiplier) for an arrow, scaled by labelled traffic.
pub fn edge_weight(e: &Element) -> (f32, f32) {
    match traffic(&e.text) {
        Some(rps) => {
            let l = rps.max(1.0).log10();
            (e.style.width * (1.0 + (l - 1.0).clamp(0.0, 4.0) * 0.45), (1.0 + (l / 1.5).floor()).clamp(1.0, 3.0))
        }
        None => (e.style.width, 1.0),
    }
}

/// Loop length for flow animation (GIF loops seamlessly at this period).
pub const LOOP_SECS: f32 = 1.2;
pub const DOT_SPACING: f32 = 60.0;
pub const DASH: f32 = 8.0;

/// Positions of flow dots along a path at animation phase [0,1).
pub fn flow_dots(path: &[Pos2], phase: f32) -> Vec<Pos2> {
    let l = path_len(path);
    let mut d = phase.fract() * DOT_SPACING;
    let mut out = vec![];
    while d < l {
        out.push(point_at(path, d));
        d += DOT_SPACING;
    }
    out
}

/// Marching-ants offset for dashed flow.
pub fn dash_offset(phase: f32) -> f32 {
    phase.fract() * DASH * 2.0 * 3.0
}

// ---------- scenarios ----------

pub const STEP_TRAVEL: f32 = 1.0;
pub const STEP_HOLD: f32 = 0.6;
pub const SCENARIO_TAIL: f32 = 1.2;

pub fn scenario_len(steps: usize) -> f32 {
    steps as f32 * (STEP_TRAVEL + STEP_HOLD) + SCENARIO_TAIL
}

pub struct ScnState {
    pub step: usize,
    /// 0..1 progress of the packet along the current step.
    pub travel: f32,
    /// seconds since the packet arrived (negative while travelling)
    pub since_arrive: f32,
}

pub fn scenario_state(steps: usize, t: f32) -> ScnState {
    let t = t % scenario_len(steps);
    let per = STEP_TRAVEL + STEP_HOLD;
    let step = ((t / per) as usize).min(steps.saturating_sub(1));
    let local = t - step as f32 * per;
    ScnState { step, travel: (local / STEP_TRAVEL).min(1.0), since_arrive: local - STEP_TRAVEL }
}

pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

// ---------- shapes & text ----------

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
        Kind::Sticky => 4.0,
        Kind::Rect if e.style.round => 12.0f32.min(r.width() / 4.0).min(r.height() / 4.0),
        _ => 0.0,
    }
}

pub fn rounded_rect(r: Rect, rad: f32) -> Vec<Pos2> {
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

pub const LINE_H: f32 = 1.3;
pub const LABEL_PAD: f32 = 12.0;

/// Placement of a text block of `size` around/inside `r`: (x, top, horizontal anchor 0/0.5/1).
pub fn text_place(r: Rect, pos: TextPos, outside: bool, size: Vec2, pad: f32) -> (f32, f32, f32) {
    let (fx, fy) = (pos.fx(), pos.fy());
    let center_both = fx == 0.5 && fy == 0.5;
    let (x, ax) = if outside && !center_both && fx != 0.5 {
        if fx == 0.0 { (r.min.x - pad, 1.0) } else { (r.max.x + pad, 0.0) }
    } else {
        (r.min.x + pad + (r.width() - 2.0 * pad) * fx, fx)
    };
    let top = if outside && !center_both && fy != 0.5 {
        if fy == 0.0 { r.min.y - pad * 0.6 - size.y } else { r.max.y + pad * 0.6 }
    } else {
        r.min.y + pad * 0.8 + (r.height() - 1.6 * pad - size.y) * fy
    };
    (x, top, ax)
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

// ---------- sketch (hand-drawn) geometry ----------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % 10_000) as f32 / 10_000.0 * 2.0 - 1.0
    }
}

/// Two wobbly strokes tracing `pts`, deterministic per seed.
pub fn rough(pts: &[Pos2], closed: bool, seed: u64, amp: f32) -> Vec<Vec<Pos2>> {
    let mut src = pts.to_vec();
    if closed && !src.is_empty() {
        src.push(src[0]);
    }
    (0..2)
        .map(|pass| {
            let mut rng = Rng::new(seed * 31 + pass);
            let mut out = vec![];
            for w in src.windows(2) {
                let l = w[0].distance(w[1]);
                let n = (l / 18.0).ceil().max(1.0) as usize;
                let perp = if l > 0.0 { vec2(-(w[1] - w[0]).y, (w[1] - w[0]).x) / l } else { Vec2::ZERO };
                let bow = rng.f() * amp * (l / 120.0).min(1.5);
                for k in 0..n {
                    let t = k as f32 / n as f32;
                    let arc = (t * std::f32::consts::PI).sin() * bow;
                    out.push(w[0] + (w[1] - w[0]) * t + perp * (arc + rng.f() * amp * 0.35));
                }
            }
            if let Some(l) = src.last() {
                out.push(*l + vec2(rng.f(), rng.f()) * amp * 0.5);
            }
            out
        })
        .collect()
}

/// Diagonal hachure lines filling a convex polygon.
pub fn hachure(poly: &[Pos2], gap: f32, seed: u64) -> Vec<[Pos2; 2]> {
    let rot = |p: Pos2, s: f32| pos2((p.x + p.y * s) * std::f32::consts::FRAC_1_SQRT_2, (p.y - p.x * s) * std::f32::consts::FRAC_1_SQRT_2);
    let r: Vec<Pos2> = poly.iter().map(|p| rot(*p, 1.0)).collect();
    let b = bbox(&r);
    let mut rng = Rng::new(seed);
    let mut out = vec![];
    let mut y = b.min.y + gap / 2.0;
    while y < b.max.y {
        let mut xs = vec![];
        for i in 0..r.len() {
            let (p, q) = (r[i], r[(i + 1) % r.len()]);
            if (p.y <= y && q.y > y) || (q.y <= y && p.y > y) {
                xs.push(p.x + (y - p.y) / (q.y - p.y) * (q.x - p.x));
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        if xs.len() >= 2 {
            let (a, c) = (pos2(xs[0] + 2.0, y + rng.f()), pos2(xs[xs.len() - 1] - 2.0, y + rng.f()));
            out.push([rot(a, -1.0), rot(c, -1.0)]);
        }
        y += gap;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_el(x: f32, y: f32) -> Element {
        Element::new(Kind::Rect, Rect::from_min_size(pos2(x, y), vec2(100., 50.)))
    }

    #[test]
    fn arrow_binds_and_follows() {
        let mut d = Doc::default();
        let a = d.add(rect_el(0., 0.));
        let b = d.add(rect_el(300., 200.));
        let mut arr = Element::new(Kind::Arrow, Rect::ZERO);
        arr.from = End { id: Some(a), p: Pos2::ZERO };
        arr.to = End { id: Some(b), p: Pos2::ZERO };
        let ar = d.add(arr);
        let c = d.arrow_corners(d.get(ar).unwrap());
        assert_eq!(c.first(), Some(&pos2(100., 25.)), "leaves a's facing side");
        let (l, bb) = (*c.last().unwrap(), Rect::from_min_size(pos2(300., 200.), vec2(100., 50.)));
        assert!(bb.expand(0.5).contains(l) && !bb.shrink(0.5).contains(l), "ends on b's outline: {c:?}");
        d.get_mut(b).unwrap().translate(vec2(0., -200.));
        let c = d.arrow_corners(d.get(ar).unwrap());
        assert_eq!(c, vec![pos2(100., 25.), pos2(300., 25.)]);
        d.remove(&[b]);
        assert_eq!(d.get(ar).unwrap().to, End { id: None, p: pos2(300., 25.) });
    }

    #[test]
    fn elbow_avoids_obstacle() {
        let mut d = Doc::default();
        let a = d.add(rect_el(0., 0.));
        let b = d.add(rect_el(400., 0.));
        d.add(rect_el(200., 0.)); // directly between
        let mut arr = Element::new(Kind::Arrow, Rect::ZERO);
        arr.from.id = Some(a);
        arr.to.id = Some(b);
        let ar = d.add(arr);
        let p = d.arrow_corners(d.get(ar).unwrap());
        let blocker = Rect::from_min_size(pos2(200., 0.), vec2(100., 50.));
        assert!(p.windows(2).all(|w| !seg_hits_rect(w[0], w[1], blocker)), "{p:?}");
    }

    #[test]
    fn hit_prefers_topmost_and_region_border() {
        let mut d = Doc::default();
        let reg = d.add(Element::new(Kind::Region, Rect::from_min_size(pos2(0., 0.), vec2(400., 400.))));
        let n = d.add(Element::new(Kind::Rect, Rect::from_min_size(pos2(100., 100.), vec2(50., 50.))));
        assert_eq!(d.hit(pos2(120., 120.), 4.), Some(n));
        assert_eq!(d.hit(pos2(200., 300.), 4.), None);
        assert_eq!(d.hit(pos2(200., 10.), 4.), Some(reg));
    }

    #[test]
    fn parses_traffic() {
        assert_eq!(traffic("HTTPS · 1.2k rps"), Some(1200.0));
        assert_eq!(traffic("300 qps"), Some(300.0));
        assert_eq!(traffic("6000 rpm"), Some(100.0));
        assert_eq!(traffic("2m req/s"), Some(2e6));
        assert_eq!(traffic("SQL"), None);
    }

    #[test]
    fn text_positions() {
        let r = Rect::from_min_size(pos2(0., 0.), vec2(100., 60.));
        let s = vec2(20., 10.);
        assert_eq!(text_place(r, TextPos::Center, false, s, 10.).2, 0.5);
        let (x, _, a) = text_place(r, TextPos::Right, true, s, 10.);
        assert_eq!((x, a), (110., 0.0));
        let (_, top, _) = text_place(r, TextPos::Bottom, true, s, 10.);
        assert!(top > 60.);
    }

    #[test]
    fn scenario_timing() {
        let s = scenario_state(3, STEP_TRAVEL + STEP_HOLD + 0.5);
        assert_eq!(s.step, 1);
        assert!((s.travel - 0.5).abs() < 1e-4);
    }
}
