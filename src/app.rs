use crate::export;
use crate::icons::{self, CATEGORIES, ICONS, Icons};
use crate::model::*;
use crate::render::{self, View};
use eframe::egui::{
    self, Align, Align2, Color32, Context, CornerRadius, Event, FontId, Frame, Id, Key, LayerId, Margin, Modifiers, Order, PointerButton, Pos2,
    Rect, Response, RichText, Sense, Shadow, Stroke, StrokeKind, TextEdit, Ui, Vec2, pos2, vec2,
};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

const INDIGO_50: Color32 = Color32::from_rgb(0xee, 0xf2, 0xff);
const INDIGO_600: Color32 = Color32::from_rgb(0x4f, 0x46, 0xe5);
const GRAY_100: Color32 = Color32::from_rgb(0xf3, 0xf4, 0xf6);
const GRAY_700: Color32 = Color32::from_rgb(0x37, 0x41, 0x51);
const CANVAS_BG: Color32 = Color32::from_rgb(0xfa, 0xfa, 0xfb);
const GUIDE: Color32 = Color32::from_rgb(0xec, 0x48, 0x99);
const UV: Rect = Rect { min: pos2(0., 0.), max: pos2(1., 1.) };

const STROKES: [Color32; 8] = [
    INK,
    Color32::from_rgb(0xe0, 0x31, 0x31),
    Color32::from_rgb(0x2f, 0x9e, 0x44),
    Color32::from_rgb(0x19, 0x71, 0xc2),
    Color32::from_rgb(0xf0, 0x8c, 0x00),
    INDIGO,
    Color32::from_rgb(0xdb, 0x27, 0x77),
    Color32::from_rgb(0x0d, 0x94, 0x88),
];
const FILLS: [Color32; 8] = [
    Color32::TRANSPARENT,
    Color32::WHITE,
    Color32::from_rgb(0xff, 0xc9, 0xc9),
    Color32::from_rgb(0xb2, 0xf2, 0xbb),
    Color32::from_rgb(0xa5, 0xd8, 0xff),
    Color32::from_rgb(0xff, 0xec, 0x99),
    Color32::from_rgb(0xe0, 0xe7, 0xff),
    Color32::from_rgb(0xf1, 0xf5, 0xf9),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Hand,
    Rect,
    Ellipse,
    Diamond,
    Arrow,
    Text,
    Pen,
    Region,
}

const TOOLS: [(Tool, &str, &str, Key); 9] = [
    (Tool::Select, "t-select", "Select  V", Key::V),
    (Tool::Hand, "t-hand", "Hand  H", Key::H),
    (Tool::Rect, "t-rect", "Rectangle  R", Key::R),
    (Tool::Ellipse, "t-ellipse", "Ellipse  O", Key::O),
    (Tool::Diamond, "t-diamond", "Diamond  D", Key::D),
    (Tool::Arrow, "t-arrow", "Connector  A", Key::A),
    (Tool::Text, "t-text", "Text  T", Key::T),
    (Tool::Pen, "t-pen", "Draw  P", Key::P),
    (Tool::Region, "t-region", "Group region  G", Key::G),
];

enum Drag {
    None,
    Pan,
    Marquee(Pos2),
    Move { start: Pos2, orig: Vec<Element>, moved: bool },
    Resize { handle: usize, orig: Element },
    Create { id: u64, start: Pos2 },
    ArrowEnd { id: u64, to: bool, source: Option<u64> },
    Pen(u64),
}

pub struct App {
    doc: Doc,
    undo: Vec<Doc>,
    redo: Vec<Doc>,
    sel: Vec<u64>,
    tool: Tool,
    offset: Vec2,
    zoom: f32,
    canvas: Rect,
    drag: Drag,
    editing: Option<u64>,
    edit_focus: bool,
    icons: Icons,
    library: bool,
    replace_icon: bool,
    lib_query: String,
    lib_drag: Option<&'static str>,
    playing: bool,
    path: Option<PathBuf>,
    dirty: bool,
    title: String,
    toast: Option<(String, f64)>,
    export_rx: Option<Receiver<Result<String, String>>>,
    guides: Vec<[Pos2; 2]>,
    menu: bool,
    export_menu: bool,
    hover_shape: Option<u64>,
    drop_target: Option<u64>,
    fit_pending: bool,
    cascade: f32,
    nudge: Option<Vec2>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        crate::setup_fonts(&cc.egui_ctx);
        setup_style(&cc.egui_ctx);
        let mut path = None;
        let doc = match std::env::args().nth(1).map(PathBuf::from) {
            Some(p) => {
                let d = load(&p).unwrap_or_default();
                path = Some(p);
                d
            }
            None => cc.storage.and_then(|s| s.get_string("doc")).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
        };
        App {
            fit_pending: !doc.elements.is_empty(),
            doc,
            undo: vec![],
            redo: vec![],
            sel: vec![],
            tool: Tool::Select,
            offset: vec2(0.0, 0.0),
            zoom: 1.0,
            canvas: Rect::ZERO,
            drag: Drag::None,
            editing: None,
            edit_focus: false,
            icons: Icons::default(),
            library: true,
            replace_icon: false,
            lib_query: String::new(),
            lib_drag: None,
            playing: true,
            path,
            dirty: false,
            title: String::new(),
            toast: None,
            export_rx: None,
            guides: vec![],
            menu: false,
            export_menu: false,
            hover_shape: None,
            drop_target: None,
            cascade: 0.0,
            nudge: None,
        }
    }

    fn view(&self) -> View {
        View { origin: self.canvas.min + self.offset, zoom: self.zoom }
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.doc.clone());
        // ponytail: whole-doc snapshots; switch to diffs if docs get huge.
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    fn undo(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, d));
            self.sel.retain(|id| self.doc.get(*id).is_some());
        }
    }

    fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, d));
            self.sel.retain(|id| self.doc.get(*id).is_some());
        }
    }

    fn toast(&mut self, ctx: &Context, msg: impl Into<String>) {
        self.toast = Some((msg.into(), ctx.input(|i| i.time) + 2.5));
    }

    fn zoom_at(&mut self, screen: Pos2, factor: f32) {
        let w = self.view().s2w(screen);
        self.zoom = (self.zoom * factor).clamp(0.1, 8.0);
        self.offset = screen - self.canvas.min - w.to_vec2() * self.zoom;
    }

    fn fit(&mut self) {
        let Some(b) = self.doc.all_bounds() else {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
            return;
        };
        // keep clear of the floating panels
        let c = Rect::from_min_max(self.canvas.min + vec2(if self.sel.is_empty() { 40.0 } else { 250.0 }, 70.0), self.canvas.max - vec2(if self.library { 310.0 } else { 40.0 }, 70.0));
        self.zoom = ((c.width() - 40.0) / b.width()).min((c.height() - 40.0) / b.height()).clamp(0.2, 1.5);
        self.offset = c.center() - self.canvas.min - b.center().to_vec2() * self.zoom;
    }

    fn view_center(&self) -> Pos2 {
        self.view().s2w(self.canvas.center())
    }

    fn add_select(&mut self, e: Element) -> u64 {
        let id = self.doc.add(e);
        self.sel = vec![id];
        id
    }

    fn start_edit(&mut self, id: u64) {
        if self.doc.get(id).is_some_and(|e| e.kind != Kind::Pen) {
            self.checkpoint();
            self.editing = Some(id);
            self.edit_focus = true;
            self.sel = vec![id];
        }
    }

    fn finish_edit(&mut self) {
        if let Some(id) = self.editing.take() {
            if self.doc.get(id).is_some_and(|e| e.kind == Kind::Text && e.text.trim().is_empty()) {
                self.doc.remove(&[id]);
                self.sel.retain(|s| *s != id);
            }
        }
    }

    fn delete_selected(&mut self) {
        if !self.sel.is_empty() {
            self.checkpoint();
            let ids = std::mem::take(&mut self.sel);
            self.doc.remove(&ids);
        }
    }

    /// Clones elements (remapping arrow bindings inside the set). Returns new ids.
    fn insert_clones(&mut self, mut els: Vec<Element>, d: Vec2) -> Vec<u64> {
        let map: Vec<(u64, u64)> = els.iter().enumerate().map(|(i, e)| (e.id, self.doc.next_id + 1 + i as u64)).collect();
        let remap = |end: &mut End| {
            if let Some(id) = end.id {
                end.id = map.iter().find(|m| m.0 == id).map(|m| m.1);
            }
        };
        let mut ids = vec![];
        for e in &mut els {
            remap(&mut e.from);
            remap(&mut e.to);
            e.translate(d);
        }
        for e in els {
            ids.push(self.doc.add(e));
        }
        ids
    }

    /// Selected elements with arrow endpoints resolved to absolute positions.
    fn selected_snapshot(&self) -> Vec<Element> {
        self.doc
            .elements
            .iter()
            .filter(|e| self.sel.contains(&e.id))
            .map(|e| {
                let mut e = e.clone();
                if e.kind == Kind::Arrow {
                    let c = self.doc.arrow_corners(&e);
                    e.from.p = c[0];
                    e.to.p = *c.last().unwrap();
                }
                e
            })
            .collect()
    }

    fn duplicate(&mut self) {
        let els = self.selected_snapshot();
        if !els.is_empty() {
            self.checkpoint();
            self.sel = self.insert_clones(els, vec2(24.0, 24.0));
        }
    }

    fn paste(&mut self, ctx: &Context, s: &str) {
        self.checkpoint();
        if let Ok(els) = serde_json::from_str::<Vec<Element>>(s) {
            let b = render_bounds(&els);
            let d = self.view_center() - b.center();
            self.sel = self.insert_clones(els, d);
        } else if !s.trim().is_empty() {
            let mut e = Element::new(Kind::Text, Rect::from_min_size(self.view_center(), Vec2::ZERO));
            e.text = s.trim().to_string();
            e.rect.max = e.rect.min + render::measure(ctx, &e.text, e.style.font);
            self.add_select(e);
        }
    }

    fn group_region(&mut self) {
        let Some(b) = self.doc.bounds_of(&self.sel) else { return };
        self.checkpoint();
        let mut r = Element::new(Kind::Region, Rect::from_min_max(b.min - vec2(28.0, 48.0), b.max + vec2(28.0, 28.0)));
        r.text = "Group".into();
        let id = self.add_select(r);
        self.start_edit(id);
        self.undo.pop(); // start_edit's checkpoint duplicates ours
    }

    fn insert_node(&mut self, icon: &str, at: Option<Pos2>) {
        self.checkpoint();
        let at = at.unwrap_or_else(|| {
            self.cascade = (self.cascade + 24.0) % 168.0;
            self.view_center() + vec2(self.cascade, self.cascade)
        });
        self.add_select(Element::node(icon, at));
        self.tool = Tool::Select;
    }

    fn set_icon(&mut self, icon: &str) {
        self.checkpoint();
        let Some((_, label, cat, _)) = icons::find(icon) else { return };
        for id in self.sel.clone() {
            if let Some(e) = self.doc.get_mut(id).filter(|e| e.kind == Kind::Node) {
                let old_default = icons::find(&e.icon).map(|i| i.1);
                if old_default == Some(node_text(&e.text).0) {
                    e.text = e.text.replacen(node_text(&e.text).0, label, 1);
                }
                e.icon = icon.to_string();
                e.style.stroke = icons::accent(cat);
            }
        }
        self.replace_icon = false;
    }

    // ---------- files ----------

    fn save(&mut self, ctx: &Context, ask: bool) {
        let path = match (&self.path, ask) {
            (Some(p), false) => Some(p.clone()),
            _ => rfd::FileDialog::new().add_filter("idraw", &["idraw"]).set_file_name("diagram.idraw").save_file(),
        };
        let Some(p) = path else { return };
        match serde_json::to_string_pretty(&self.doc).map_err(|e| e.to_string()).and_then(|s| std::fs::write(&p, s).map_err(|e| e.to_string())) {
            Ok(()) => {
                self.dirty = false;
                self.toast(ctx, format!("Saved {}", p.file_name().unwrap_or_default().to_string_lossy()));
                self.path = Some(p);
            }
            Err(e) => self.toast(ctx, format!("Save failed: {e}")),
        }
    }

    fn open(&mut self, ctx: &Context) {
        let Some(p) = rfd::FileDialog::new().add_filter("idraw", &["idraw", "json"]).pick_file() else { return };
        match load(&p) {
            Ok(d) => {
                self.checkpoint();
                self.doc = d;
                self.sel.clear();
                self.path = Some(p);
                self.dirty = false;
                self.fit();
            }
            Err(e) => self.toast(ctx, format!("Open failed: {e}")),
        }
    }

    fn export(&mut self, ctx: &Context, fmt: &'static str) {
        self.export_menu = false;
        if self.doc.elements.is_empty() {
            return self.toast(ctx, "Nothing to export");
        }
        let stem = self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or("diagram".into());
        let Some(p) = rfd::FileDialog::new().add_filter(fmt, &[fmt]).set_file_name(format!("{stem}.{fmt}")).save_file() else { return };
        let (doc, only, ctx2) = (self.doc.clone(), self.sel.clone(), ctx.clone());
        let (tx, rx) = channel();
        self.export_rx = Some(rx);
        self.toast = Some((format!("Exporting {}…", fmt.to_uppercase()), f64::MAX));
        std::thread::spawn(move || {
            let r = match fmt {
                "png" => export::write_png(&doc, &only, &p),
                "svg" => export::write_svg(&doc, &only, &p),
                _ => export::write_gif(&doc, &only, &p),
            };
            let _ = tx.send(r.map(|_| p.display().to_string()));
            ctx2.request_repaint();
        });
    }

    // ---------- input ----------

    fn shortcuts(&mut self, ctx: &Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        let cmd = Modifiers::COMMAND;
        let shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
        let mut act: Vec<&str> = vec![];
        ctx.input_mut(|i| {
            for (k, m, name) in [
                (Key::Z, shift_cmd, "redo"),
                (Key::Z, cmd, "undo"),
                (Key::S, shift_cmd, "saveas"),
                (Key::S, cmd, "save"),
                (Key::O, cmd, "open"),
                (Key::A, cmd, "all"),
                (Key::D, cmd, "dup"),
                (Key::G, cmd, "group"),
                (Key::E, shift_cmd, "gif"),
                (Key::E, cmd, "png"),
                (Key::Equals, cmd, "zin"),
                (Key::Plus, cmd, "zin"),
                (Key::Minus, cmd, "zout"),
                (Key::Num0, cmd, "z100"),
                (Key::Num1, Modifiers::SHIFT, "fit"),
            ] {
                if i.consume_key(m, k) {
                    act.push(name);
                }
            }
            if i.modifiers.is_none() || i.modifiers == Modifiers::SHIFT {
                if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                    act.push("del");
                }
                if i.key_pressed(Key::Escape) {
                    act.push("esc");
                }
                for (k, d) in [(Key::ArrowLeft, vec2(-1., 0.)), (Key::ArrowRight, vec2(1., 0.)), (Key::ArrowUp, vec2(0., -1.)), (Key::ArrowDown, vec2(0., 1.))] {
                    if i.key_pressed(k) {
                        let d = d * if i.modifiers.shift { 10.0 } else { 1.0 };
                        self.nudge = Some(d);
                    }
                }
            }
            if i.modifiers.is_none() {
                for (t, _, _, k) in TOOLS {
                    if i.key_pressed(k) {
                        self.tool = t;
                    }
                }
                if i.key_pressed(Key::Num1) {
                    self.tool = Tool::Select;
                }
                if i.key_pressed(Key::L) || i.key_pressed(Key::N) {
                    act.push("lib");
                }
            }
        });
        if let Some(d) = self.nudge.take() {
            if !self.sel.is_empty() {
                self.checkpoint();
                for id in self.sel.clone() {
                    if let Some(e) = self.doc.get_mut(id) {
                        e.translate(d);
                    }
                }
            }
        }
        for a in act {
            match a {
                "redo" => self.redo(),
                "undo" => self.undo(),
                "saveas" => self.save(ctx, true),
                "save" => self.save(ctx, false),
                "open" => self.open(ctx),
                "all" => self.sel = self.doc.elements.iter().map(|e| e.id).collect(),
                "dup" => self.duplicate(),
                "group" => self.group_region(),
                "gif" => self.export(ctx, "gif"),
                "png" => self.export(ctx, "png"),
                "zin" => self.zoom_at(self.canvas.center(), 1.2),
                "zout" => self.zoom_at(self.canvas.center(), 1.0 / 1.2),
                "z100" => self.zoom_at(self.canvas.center(), 1.0 / self.zoom),
                "fit" => self.fit(),
                "del" => self.delete_selected(),
                "esc" => {
                    self.sel.clear();
                    self.tool = Tool::Select;
                    self.menu = false;
                    self.export_menu = false;
                    self.replace_icon = false;
                }
                "lib" => self.library = !self.library,
                _ => {}
            }
        }
        for ev in ctx.input(|i| i.events.clone()) {
            match ev {
                Event::Copy | Event::Cut if !self.sel.is_empty() => {
                    ctx.copy_text(serde_json::to_string(&self.selected_snapshot()).unwrap_or_default());
                    if ev == Event::Cut {
                        self.delete_selected();
                    }
                }
                Event::Paste(s) => self.paste(ctx, &s),
                _ => {}
            }
        }
    }

    fn handles(r: Rect) -> [Pos2; 8] {
        let c = r.center();
        [r.left_top(), pos2(c.x, r.min.y), r.right_top(), pos2(r.max.x, c.y), r.right_bottom(), pos2(c.x, r.max.y), r.left_bottom(), pos2(r.min.x, c.y)]
    }

    fn single_box(&self) -> Option<&Element> {
        match self.sel.as_slice() {
            [id] => self.doc.get(*id).filter(|e| e.kind != Kind::Arrow),
            _ => None,
        }
    }

    /// Connection anchors (screen) around a shape, offset outward.
    fn anchors(&self, id: u64) -> Vec<(Pos2, Pos2)> {
        let v = self.view();
        let Some(e) = self.doc.get(id).filter(|e| e.bindable()) else { return vec![] };
        let r = v.rect(e.rect);
        let c = r.center();
        let o = 16.0;
        vec![(pos2(c.x, r.min.y - o), pos2(c.x, r.min.y)), (pos2(r.max.x + o, c.y), pos2(r.max.x, c.y)), (pos2(c.x, r.max.y + o), pos2(c.x, r.max.y)), (pos2(r.min.x - o, c.y), pos2(r.min.x, c.y))]
    }

    fn begin_select_drag(&mut self, sp: Pos2, w: Pos2, shift: bool) {
        let v = self.view();
        // resize handles
        if let Some(e) = self.single_box() {
            if let Some(h) = Self::handles(v.rect(e.rect)).iter().position(|h| h.distance(sp) < 8.0) {
                let orig = e.clone();
                self.checkpoint();
                self.drag = Drag::Resize { handle: h, orig };
                return;
            }
        }
        // arrow endpoint handles
        if let [id] = self.sel.as_slice() {
            if let Some(e) = self.doc.get(*id).filter(|e| e.kind == Kind::Arrow) {
                let p = self.doc.arrow_path(e);
                let (a, b) = (v.w2s(p[0]), v.w2s(*p.last().unwrap()));
                let id = *id;
                if b.distance(sp) < 9.0 || a.distance(sp) < 9.0 {
                    self.checkpoint();
                    self.drag = Drag::ArrowEnd { id, to: b.distance(sp) < 9.0, source: None };
                    return;
                }
            }
        }
        // connection anchors -> new connector
        if let Some(src) = self.hover_shape {
            if self.anchors(src).iter().any(|(a, _)| a.distance(sp) < 9.0) {
                self.checkpoint();
                let mut a = Element::new(Kind::Arrow, Rect::ZERO);
                a.from = End { id: Some(src), p: w };
                a.to = End { id: None, p: w };
                let id = self.doc.add(a);
                self.sel = vec![id];
                self.drag = Drag::ArrowEnd { id, to: true, source: Some(src) };
                return;
            }
        }
        if let Some(id) = self.doc.hit(w, 6.0 / self.zoom) {
            if !self.sel.contains(&id) {
                if !shift {
                    self.sel.clear();
                }
                self.sel.push(id);
            }
            self.checkpoint();
            self.drag = Drag::Move { start: w, orig: self.doc.elements.iter().filter(|e| self.sel.contains(&e.id)).cloned().collect(), moved: false };
        } else {
            if !shift {
                self.sel.clear();
            }
            self.drag = Drag::Marquee(w);
        }
    }

    fn snap(&mut self, moving: Rect, delta: Vec2, exclude: &[u64]) -> Vec2 {
        let thr = 6.0 / self.zoom;
        let m = moving.translate(delta);
        let (mut bx, mut by): (Option<(f32, f32)>, Option<(f32, f32)>) = (None, None);
        for e in self.doc.elements.iter().filter(|e| !exclude.contains(&e.id) && e.kind != Kind::Arrow) {
            let b = self.doc.bounds(e);
            for cx in [b.min.x, b.center().x, b.max.x] {
                for mx in [m.min.x, m.center().x, m.max.x] {
                    let d = cx - mx;
                    if d.abs() < thr && bx.is_none_or(|(bd, _)| d.abs() < bd.abs()) {
                        bx = Some((d, cx));
                    }
                }
            }
            for cy in [b.min.y, b.center().y, b.max.y] {
                for my in [m.min.y, m.center().y, m.max.y] {
                    let d = cy - my;
                    if d.abs() < thr && by.is_none_or(|(bd, _)| d.abs() < bd.abs()) {
                        by = Some((d, cy));
                    }
                }
            }
        }
        let (a, b) = (self.view().s2w(self.canvas.min), self.view().s2w(self.canvas.max));
        if let Some((_, x)) = bx {
            self.guides.push([pos2(x, a.y), pos2(x, b.y)]);
        }
        if let Some((_, y)) = by {
            self.guides.push([pos2(a.x, y), pos2(b.x, y)]);
        }
        delta + vec2(bx.map_or(0.0, |b| b.0), by.map_or(0.0, |b| b.0))
    }

    fn canvas_input(&mut self, ctx: &Context, resp: &Response) {
        let v = self.view();
        let (mods, space, scroll, zd, hover) = ctx.input(|i| (i.modifiers, i.key_down(Key::Space), i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
        let space = space && !ctx.wants_keyboard_input();
        if resp.hovered() {
            self.offset += scroll;
            if zd != 1.0 {
                if let Some(p) = hover {
                    self.zoom_at(p, zd);
                }
            }
        }
        // hover shape for connection anchors (sticky while near it)
        if matches!(self.drag, Drag::None) && self.tool == Tool::Select {
            if let Some(p) = resp.hover_pos() {
                let w = v.s2w(p);
                let near = self.hover_shape.and_then(|id| self.doc.get(id)).is_some_and(|e| e.rect.expand(28.0 / self.zoom).contains(w));
                if !near || self.doc.hit_bindable(w, None).is_some() {
                    self.hover_shape = self.doc.hit_bindable(w, None);
                }
            } else {
                self.hover_shape = None;
            }
        } else if matches!(self.drag, Drag::None) {
            self.hover_shape = None;
        }

        if resp.drag_started() || resp.clicked() {
            self.menu = false;
            self.export_menu = false;
        }

        if resp.drag_started_by(PointerButton::Middle) || (resp.drag_started_by(PointerButton::Primary) && (space || self.tool == Tool::Hand)) {
            self.drag = Drag::Pan;
        } else if resp.drag_started_by(PointerButton::Primary) {
            let sp = ctx.input(|i| i.pointer.press_origin()).unwrap_or_default();
            let w = v.s2w(sp);
            self.finish_edit();
            match self.tool {
                Tool::Select => self.begin_select_drag(sp, w, mods.shift),
                Tool::Rect | Tool::Ellipse | Tool::Diamond | Tool::Region => {
                    self.checkpoint();
                    let kind = match self.tool {
                        Tool::Rect => Kind::Rect,
                        Tool::Ellipse => Kind::Ellipse,
                        Tool::Diamond => Kind::Diamond,
                        _ => Kind::Region,
                    };
                    let mut e = Element::new(kind, Rect::from_min_size(w, Vec2::ZERO));
                    if kind == Kind::Region {
                        e.text = "Region".into();
                    }
                    let id = self.add_select(e);
                    self.drag = Drag::Create { id, start: w };
                }
                Tool::Arrow => {
                    self.checkpoint();
                    let mut a = Element::new(Kind::Arrow, Rect::ZERO);
                    a.from = End { id: self.doc.hit_bindable(w, None), p: w };
                    a.to = End { id: None, p: w };
                    let id = self.add_select(a);
                    self.drag = Drag::ArrowEnd { id, to: true, source: None };
                }
                Tool::Pen => {
                    self.checkpoint();
                    let mut e = Element::new(Kind::Pen, Rect::ZERO);
                    e.points = vec![w];
                    let id = self.doc.add(e);
                    self.drag = Drag::Pen(id);
                }
                Tool::Text | Tool::Hand => {}
            }
        }

        if resp.dragged() {
            let p = resp.interact_pointer_pos().unwrap_or_default();
            let w = v.s2w(p);
            self.guides.clear();
            match &mut self.drag {
                Drag::Pan => self.offset += resp.drag_delta(),
                Drag::Move { start, orig, moved } => {
                    let (start, orig) = (*start, orig.clone());
                    *moved = true;
                    let mut d = w - start;
                    let ids: Vec<u64> = orig.iter().map(|e| e.id).collect();
                    if !mods.command {
                        let mb = render_bounds(&orig.iter().filter(|e| e.kind != Kind::Arrow).cloned().collect::<Vec<_>>());
                        if mb.is_positive() {
                            d = self.snap(mb, d, &ids);
                        }
                    }
                    for o in orig {
                        if let Some(e) = self.doc.get_mut(o.id) {
                            *e = o;
                            e.translate(d);
                        }
                    }
                }
                Drag::Resize { handle, orig } => {
                    let (h, orig) = (*handle, orig.clone());
                    let mut r = orig.rect;
                    match h {
                        0 => r.min = w,
                        1 => r.min.y = w.y,
                        2 => (r.max.x, r.min.y) = (w.x, w.y),
                        3 => r.max.x = w.x,
                        4 => r.max = w,
                        5 => r.max.y = w.y,
                        6 => (r.min.x, r.max.y) = (w.x, w.y),
                        _ => r.min.x = w.x,
                    }
                    let mut r = Rect::from_two_pos(r.min, r.max);
                    let min = if orig.kind == Kind::Node { vec2(120.0, 44.0) } else { vec2(8.0, 8.0) };
                    r.max = r.min + r.size().max(min);
                    if let Some(e) = self.doc.get_mut(orig.id) {
                        match e.kind {
                            Kind::Pen => {
                                let o = bbox(&orig.points);
                                let sx = r.width() / o.width().max(1.0);
                                let sy = r.height() / o.height().max(1.0);
                                e.points = orig.points.iter().map(|p| r.min + vec2((p.x - o.min.x) * sx, (p.y - o.min.y) * sy)).collect();
                            }
                            Kind::Text => e.style.font = (orig.style.font * r.height() / orig.rect.height().max(1.0)).clamp(6.0, 400.0),
                            _ => e.rect = r,
                        }
                    }
                }
                Drag::Create { id, start } => {
                    let (id, start) = (*id, *start);
                    let mut end = w;
                    if mods.shift {
                        let d = w - start;
                        let s = d.x.abs().max(d.y.abs());
                        end = start + vec2(s * d.x.signum(), s * d.y.signum());
                    }
                    if let Some(e) = self.doc.get_mut(id) {
                        e.rect = Rect::from_two_pos(start, end);
                    }
                }
                Drag::ArrowEnd { id, to, .. } => {
                    let (id, to) = (*id, *to);
                    let other = self.doc.get(id).and_then(|e| if to { e.from.id } else { e.to.id });
                    let t = self.doc.hit_bindable(w, other);
                    self.drop_target = t;
                    if let Some(e) = self.doc.get_mut(id) {
                        *(if to { &mut e.to } else { &mut e.from }) = End { id: t, p: w };
                    }
                }
                Drag::Pen(id) => {
                    let (id, z) = (*id, self.zoom);
                    if let Some(e) = self.doc.get_mut(id) {
                        if e.points.last().is_none_or(|l| l.distance(w) > 1.5 / z) {
                            e.points.push(w);
                        }
                    }
                }
                Drag::Marquee(_) | Drag::None => {}
            }
        }

        if resp.drag_stopped() {
            let w = v.s2w(resp.interact_pointer_pos().or(hover).unwrap_or_default());
            match std::mem::replace(&mut self.drag, Drag::None) {
                Drag::Marquee(start) => {
                    let r = Rect::from_two_pos(start, w);
                    for e in &self.doc.elements {
                        if r.contains_rect(self.doc.bounds(e)) && !self.sel.contains(&e.id) {
                            self.sel.push(e.id);
                        }
                    }
                }
                Drag::Move { moved: false, .. } => {
                    self.undo.pop();
                }
                Drag::Create { id, start } => {
                    if let Some(e) = self.doc.get_mut(id) {
                        if e.rect.width() < 6.0 && e.rect.height() < 6.0 {
                            e.rect = Rect::from_center_size(start, if e.kind == Kind::Region { vec2(360.0, 240.0) } else { vec2(140.0, 80.0) });
                        }
                    }
                    self.tool = Tool::Select;
                }
                Drag::ArrowEnd { id, to, source } => {
                    self.drop_target = None;
                    let len = self.doc.get(id).map(|e| path_len(&self.doc.arrow_path(e))).unwrap_or(0.0);
                    let free = self.doc.get(id).is_some_and(|e| if to { e.to.id.is_none() } else { e.from.id.is_none() });
                    if len < 8.0 && free {
                        self.doc.remove(&[id]);
                        self.undo.pop();
                        self.sel.clear();
                    } else if let (Some(src), true) = (source, free) {
                        // Quick-connect: dropping on empty canvas spawns a twin of the source.
                        if let Some(mut twin) = self.doc.get(src).cloned() {
                            twin.rect = Rect::from_center_size(w, twin.rect.size());
                            let t = self.doc.add(twin);
                            if let Some(e) = self.doc.get_mut(id) {
                                e.to.id = Some(t);
                            }
                            self.sel = vec![t];
                        }
                    }
                    self.tool = Tool::Select;
                }
                _ => {}
            }
            self.guides.clear();
        }

        if resp.clicked() {
            let w = v.s2w(resp.interact_pointer_pos().unwrap_or_default());
            if self.editing.is_some() {
                self.finish_edit();
                return;
            }
            match self.tool {
                Tool::Select | Tool::Hand | Tool::Arrow | Tool::Pen => {
                    match self.doc.hit(w, 6.0 / self.zoom) {
                        Some(id) if mods.shift => {
                            if let Some(i) = self.sel.iter().position(|s| *s == id) {
                                self.sel.remove(i);
                            } else {
                                self.sel.push(id);
                            }
                        }
                        Some(id) => self.sel = vec![id],
                        None if !mods.shift => self.sel.clear(),
                        None => {}
                    }
                }
                Tool::Rect | Tool::Ellipse | Tool::Diamond | Tool::Region => {
                    self.checkpoint();
                    let (kind, size) = match self.tool {
                        Tool::Rect => (Kind::Rect, vec2(140.0, 80.0)),
                        Tool::Ellipse => (Kind::Ellipse, vec2(120.0, 80.0)),
                        Tool::Diamond => (Kind::Diamond, vec2(120.0, 90.0)),
                        _ => (Kind::Region, vec2(360.0, 240.0)),
                    };
                    let mut e = Element::new(kind, Rect::from_center_size(w, size));
                    if kind == Kind::Region {
                        e.text = "Region".into();
                    }
                    self.add_select(e);
                    self.tool = Tool::Select;
                }
                Tool::Text => {
                    self.create_text(w);
                    self.tool = Tool::Select;
                }
            }
        }

        if resp.double_clicked() {
            let w = v.s2w(resp.interact_pointer_pos().unwrap_or_default());
            match self.doc.hit(w, 6.0 / self.zoom) {
                Some(id) => self.start_edit(id),
                None => self.create_text(w),
            }
        }

        // library drag & drop
        if let Some(icon) = self.lib_drag {
            if ctx.input(|i| i.pointer.any_released()) {
                self.lib_drag = None;
                if let Some(p) = hover.filter(|p| self.canvas.contains(*p) && !ctx.is_pointer_over_area()) {
                    self.insert_node(icon, Some(v.s2w(p)));
                }
            }
        }
    }

    fn create_text(&mut self, w: Pos2) {
        self.checkpoint();
        let e = Element::new(Kind::Text, Rect::from_min_size(w - vec2(0.0, 12.0), Vec2::ZERO));
        let id = self.add_select(e);
        self.editing = Some(id);
        self.edit_focus = true;
    }

    // ---------- painting overlays ----------

    fn paint_overlays(&self, p: &egui::Painter) {
        let v = self.view();
        let sel_stroke = Stroke::new(1.25, INDIGO);
        for g in &self.guides {
            p.line_segment([v.w2s(g[0]), v.w2s(g[1])], Stroke::new(1.0, GUIDE));
        }
        if let Some(t) = self.drop_target.and_then(|id| self.doc.get(id)) {
            let r = v.rect(t.rect).expand(4.0);
            p.rect(r, CornerRadius::same(10), INDIGO.gamma_multiply(0.08), Stroke::new(2.0, INDIGO), StrokeKind::Outside);
        }
        for id in &self.sel {
            let Some(e) = self.doc.get(*id) else { continue };
            if e.kind == Kind::Arrow {
                let path = self.doc.arrow_path(e);
                p.add(egui::Shape::line(path.iter().map(|q| v.w2s(*q)).collect(), Stroke::new(e.style.width * self.zoom + 4.0, INDIGO.gamma_multiply(0.18))));
                if self.sel.len() == 1 {
                    for q in [path[0], *path.last().unwrap()] {
                        p.circle(v.w2s(q), 5.0, Color32::WHITE, Stroke::new(1.5, INDIGO));
                    }
                }
            } else {
                p.rect_stroke(v.rect(self.doc.bounds(e)).expand(4.0), CornerRadius::same(4), sel_stroke, StrokeKind::Middle);
            }
        }
        if let Some(e) = self.single_box() {
            for h in Self::handles(v.rect(e.rect)) {
                p.rect(Rect::from_center_size(h, vec2(9.0, 9.0)), CornerRadius::same(3), Color32::WHITE, Stroke::new(1.5, INDIGO), StrokeKind::Middle);
            }
        } else if self.sel.len() > 1 {
            if let Some(b) = self.doc.bounds_of(&self.sel) {
                p.rect_stroke(v.rect(b).expand(8.0), CornerRadius::same(6), Stroke::new(1.0, INDIGO.gamma_multiply(0.6)), StrokeKind::Middle);
            }
        }
        if let Drag::Marquee(start) = self.drag {
            if let Some(h) = p.ctx().input(|i| i.pointer.hover_pos()) {
                let r = Rect::from_two_pos(v.w2s(start), h);
                p.rect(r, CornerRadius::same(2), INDIGO.gamma_multiply(0.08), Stroke::new(1.0, INDIGO), StrokeKind::Middle);
            }
        }
        if let (Some(id), Drag::None, Tool::Select) = (self.hover_shape, &self.drag, self.tool) {
            let hp = p.ctx().input(|i| i.pointer.hover_pos());
            for (a, edge) in self.anchors(id) {
                let hot = hp.is_some_and(|h| h.distance(a) < 9.0);
                p.line_segment([edge, a], Stroke::new(1.0, INDIGO.gamma_multiply(0.35)));
                p.circle(a, if hot { 6.5 } else { 5.0 }, if hot { INDIGO } else { Color32::WHITE }, Stroke::new(1.5, INDIGO));
            }
        }
    }

    // ---------- UI chrome ----------

    fn toolbar(&mut self, ctx: &Context) {
        egui::Area::new(Id::new("toolbar")).anchor(Align2::CENTER_TOP, vec2(0.0, 10.0)).show(ctx, |ui| {
            panel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (t, icon, tip, _) in TOOLS {
                        if icon_button(ui, &mut self.icons, icon, self.tool == t, tip).clicked() {
                            self.tool = t;
                        }
                    }
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    if icon_button(ui, &mut self.icons, "t-library", self.library, "Components library  L").clicked() {
                        self.library = !self.library;
                    }
                });
            });
        });
    }

    fn top_left(&mut self, ctx: &Context) {
        egui::Area::new(Id::new("menu")).anchor(Align2::LEFT_TOP, vec2(84.0, 10.0)).show(ctx, |ui| {
            panel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if icon_button(ui, &mut self.icons, "menu", self.menu, "Menu").clicked() {
                        self.menu = !self.menu;
                        self.export_menu = false;
                    }
                    let name = self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or("Untitled".into());
                    ui.label(RichText::new(format!("{name}{}", if self.dirty { " •" } else { "" })).color(GRAY_700).strong());
                    ui.add_space(6.0);
                });
            });
        });
        if !self.menu {
            return;
        }
        egui::Area::new(Id::new("menu-pop")).anchor(Align2::LEFT_TOP, vec2(84.0, 62.0)).order(Order::Foreground).show(ctx, |ui| {
            panel().show(ui, |ui| {
                ui.set_width(220.0);
                let item = |ui: &mut Ui, label: &str, key: &str| ui.add(egui::Button::new(label).shortcut_text(key).frame(false).min_size(vec2(208.0, 28.0))).clicked();
                if item(ui, "New", "") {
                    self.checkpoint();
                    self.doc = Doc::default();
                    self.sel.clear();
                    self.path = None;
                    self.menu = false;
                }
                if item(ui, "Open…", "⌘O") {
                    self.menu = false;
                    self.open(ctx);
                }
                if item(ui, "Save", "⌘S") {
                    self.menu = false;
                    self.save(ctx, false);
                }
                if item(ui, "Save as…", "⇧⌘S") {
                    self.menu = false;
                    self.save(ctx, true);
                }
                ui.separator();
                if item(ui, "Export PNG", "⌘E") {
                    self.menu = false;
                    self.export(ctx, "png");
                }
                if item(ui, "Export SVG", "") {
                    self.menu = false;
                    self.export(ctx, "svg");
                }
                if item(ui, "Export animated GIF", "⇧⌘E") {
                    self.menu = false;
                    self.export(ctx, "gif");
                }
                ui.separator();
                if item(ui, "Zoom to fit", "⇧1") {
                    self.menu = false;
                    self.fit();
                }
                if item(ui, "Wrap selection in region", "⌘G") {
                    self.menu = false;
                    self.group_region();
                }
            });
        });
    }

    fn top_right(&mut self, ctx: &Context) {
        egui::Area::new(Id::new("export")).anchor(Align2::RIGHT_TOP, vec2(-12.0, 10.0)).show(ctx, |ui| {
            panel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let b = egui::Button::new(RichText::new("  Export  ").color(Color32::WHITE).strong()).fill(INDIGO).corner_radius(8).min_size(vec2(0.0, 34.0));
                    if ui.add(b).clicked() {
                        self.export_menu = !self.export_menu;
                        self.menu = false;
                    }
                });
            });
        });
        if self.export_menu {
            egui::Area::new(Id::new("export-pop")).anchor(Align2::RIGHT_TOP, vec2(-12.0, 62.0)).order(Order::Foreground).show(ctx, |ui| {
                panel().show(ui, |ui| {
                    ui.set_width(220.0);
                    let scope = if self.sel.is_empty() { "whole canvas" } else { "selection only" };
                    ui.label(RichText::new(format!("Exports {scope}")).small().color(MUTED));
                    for (fmt, label, sub) in [("png", "PNG image", "2× resolution"), ("svg", "SVG vector", "editable, scalable"), ("gif", "Animated GIF", "loops flow animation")] {
                        let r = ui.add(egui::Button::new(RichText::new(label).strong()).shortcut_text(sub).frame(false).min_size(vec2(208.0, 30.0)));
                        if r.clicked() {
                            self.export(ctx, fmt);
                        }
                    }
                });
            });
        }
    }

    fn library_panel(&mut self, ctx: &Context) {
        if !self.library {
            return;
        }
        let h = (self.canvas.height() - 250.0).max(160.0);
        egui::Area::new(Id::new("library")).anchor(Align2::RIGHT_TOP, vec2(-12.0, 62.0)).show(ctx, |ui| {
            panel().inner_margin(Margin::same(10)).show(ui, |ui| {
                ui.set_width(272.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if self.replace_icon { "Pick a new icon" } else { "Components" }).strong().size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, &mut self.icons, "x", false, "Close  L").clicked() {
                            self.library = false;
                            self.replace_icon = false;
                        }
                    });
                });
                ui.add(TextEdit::singleline(&mut self.lib_query).hint_text("Search  e.g. postgres, queue, k8s").desired_width(f32::INFINITY).margin(Margin::symmetric(8, 6)));
                ui.add_space(4.0);
                let q = self.lib_query.to_lowercase();
                egui::ScrollArea::vertical().max_height(h).show(ui, |ui| {
                    for cat in CATEGORIES {
                        let items: Vec<_> = ICONS.iter().filter(|i| i.2 == *cat && (q.is_empty() || i.0.contains(&q) || i.1.to_lowercase().contains(&q) || cat.to_lowercase().contains(&q))).collect();
                        if items.is_empty() {
                            continue;
                        }
                        ui.add_space(4.0);
                        ui.label(RichText::new(cat.to_uppercase()).size(10.5).color(MUTED).strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                            for it in items {
                                let r = lib_tile(ui, &mut self.icons, it);
                                if r.clicked() {
                                    if self.replace_icon {
                                        self.set_icon(it.0);
                                    } else {
                                        self.insert_node(it.0, None);
                                    }
                                }
                                if r.drag_started() && !self.replace_icon {
                                    self.lib_drag = Some(it.0);
                                }
                            }
                        });
                    }
                });
                ui.add_space(2.0);
                ui.label(RichText::new("Click to add · drag onto canvas").small().color(MUTED));
            });
        });
        // drag ghost
        if let (Some(icon), Some(p)) = (self.lib_drag, ctx.input(|i| i.pointer.hover_pos())) {
            let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("ghost")));
            let r = Rect::from_center_size(p, vec2(168.0, 56.0) * self.zoom);
            painter.rect(r, CornerRadius::same(12), Color32::from_white_alpha(230), Stroke::new(1.5, INDIGO), StrokeKind::Inside);
            if let (Some(t), Some(it)) = (self.icons.get(ctx, icon), icons::find(icon)) {
                painter.image(t, Rect::from_center_size(r.center(), vec2(24.0, 24.0)), UV, icons::accent(it.2));
            }
        }
    }

    fn props_panel(&mut self, ctx: &Context) {
        let els: Vec<Element> = self.doc.elements.iter().filter(|e| self.sel.contains(&e.id)).cloned().collect();
        let Some(first) = els.first() else { return };
        let s = first.style;
        let has = |ks: &[Kind]| els.iter().any(|e| ks.contains(&e.kind));
        let mut change: Option<Box<dyn Fn(&mut Style)>> = None;
        let mut action = "";
        egui::Area::new(Id::new("props")).anchor(Align2::LEFT_TOP, vec2(12.0, 62.0)).show(ctx, |ui| {
            panel().inner_margin(Margin::same(12)).show(ui, |ui| {
                ui.set_width(212.0);
                ui.spacing_mut().item_spacing.y = 4.0;
                section(ui, if has(&[Kind::Node]) { "Accent" } else { "Stroke" });
                ui.horizontal_wrapped(|ui| {
                    for c in STROKES {
                        if swatch(ui, c, s.stroke == c).clicked() {
                            change = Some(Box::new(move |st| st.stroke = c));
                        }
                    }
                    let mut c = s.stroke;
                    if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque).changed() {
                        change = Some(Box::new(move |st| st.stroke = c));
                    }
                });
                if has(&[Kind::Rect, Kind::Ellipse, Kind::Diamond, Kind::Region, Kind::Node]) {
                    section(ui, "Background");
                    ui.horizontal_wrapped(|ui| {
                        for c in FILLS {
                            if swatch(ui, c, s.fill == c).clicked() {
                                change = Some(Box::new(move |st| st.fill = c));
                            }
                        }
                    });
                }
                if has(&[Kind::Rect, Kind::Ellipse, Kind::Diamond, Kind::Pen, Kind::Arrow, Kind::Region]) {
                    section(ui, "Stroke width");
                    if let Some(w) = seg(ui, s.width, &[(1.0, "Thin"), (2.0, "Regular"), (4.0, "Bold")]) {
                        change = Some(Box::new(move |st| st.width = w));
                    }
                }
                if !has(&[Kind::Text, Kind::Pen]) {
                    section(ui, "Line style");
                    if let Some(d) = seg(ui, s.dashed, &[(false, "Solid"), (true, "Dashed")]) {
                        change = Some(Box::new(move |st| st.dashed = d));
                    }
                }
                if has(&[Kind::Rect]) {
                    section(ui, "Corners");
                    if let Some(r) = seg(ui, s.round, &[(false, "Sharp"), (true, "Round")]) {
                        change = Some(Box::new(move |st| st.round = r));
                    }
                }
                if has(&[Kind::Rect, Kind::Ellipse, Kind::Diamond, Kind::Text, Kind::Arrow, Kind::Region]) {
                    section(ui, "Font size");
                    if let Some(f) = seg(ui, s.font, &[(13.0, "S"), (18.0, "M"), (24.0, "L"), (36.0, "XL")]) {
                        change = Some(Box::new(move |st| st.font = f));
                    }
                }
                if has(&[Kind::Arrow]) {
                    section(ui, "Route");
                    if let Some(r) = seg(ui, s.route, &[(Route::Elbow, "Elbow"), (Route::Straight, "Straight"), (Route::Curve, "Curve")]) {
                        change = Some(Box::new(move |st| st.route = r));
                    }
                    section(ui, "Arrowheads");
                    let cur = (s.head_start, s.head_end);
                    if let Some((a, b)) = seg(ui, cur, &[((false, false), "None"), ((false, true), "End"), ((true, true), "Both")]) {
                        change = Some(Box::new(move |st| (st.head_start, st.head_end) = (a, b)));
                    }
                    section(ui, "Flow animation");
                    if let Some(f) = seg(ui, s.flow, &[(Flow::None, "Off"), (Flow::Dots, "Packets"), (Flow::Dashes, "Dashes")]) {
                        change = Some(Box::new(move |st| st.flow = f));
                    }
                }
                if has(&[Kind::Node]) {
                    section(ui, "Icon");
                    if ui.button("Change icon…").clicked() {
                        action = "icon";
                    }
                }
                section(ui, "Arrange");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (icon, tip, a) in [("front", "Bring to front", "front"), ("back", "Send to back", "back"), ("copy", "Duplicate  ⌘D", "dup"), ("trash", "Delete  ⌫", "del")] {
                        if icon_button(ui, &mut self.icons, icon, false, tip).clicked() {
                            action = a;
                        }
                    }
                });
            });
        });
        if let Some(f) = change {
            self.checkpoint();
            for id in self.sel.clone() {
                if let Some(e) = self.doc.get_mut(id) {
                    f(&mut e.style);
                }
            }
        }
        match action {
            "icon" => {
                self.library = true;
                self.replace_icon = true;
            }
            "front" | "back" => {
                self.checkpoint();
                let (mut picked, rest): (Vec<Element>, Vec<Element>) = std::mem::take(&mut self.doc.elements).into_iter().partition(|e| self.sel.contains(&e.id));
                self.doc.elements = if action == "front" {
                    let mut v = rest;
                    v.append(&mut picked);
                    v
                } else {
                    picked.extend(rest);
                    picked
                };
            }
            "dup" => self.duplicate(),
            "del" => self.delete_selected(),
            _ => {}
        }
    }

    fn bottom_bars(&mut self, ctx: &Context) {
        egui::Area::new(Id::new("zoom")).anchor(Align2::LEFT_BOTTOM, vec2(12.0, -12.0)).show(ctx, |ui| {
            panel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if icon_button(ui, &mut self.icons, "minus", false, "Zoom out  ⌘−").clicked() {
                        self.zoom_at(self.canvas.center(), 1.0 / 1.2);
                    }
                    if ui.add(egui::Button::new(format!("{:.0}%", self.zoom * 100.0)).frame(false).min_size(vec2(52.0, 34.0))).on_hover_text("Reset zoom  ⌘0").clicked() {
                        self.zoom_at(self.canvas.center(), 1.0 / self.zoom);
                    }
                    if icon_button(ui, &mut self.icons, "plus", false, "Zoom in  ⌘+").clicked() {
                        self.zoom_at(self.canvas.center(), 1.2);
                    }
                    if icon_button(ui, &mut self.icons, "fit", false, "Zoom to fit  ⇧1").clicked() {
                        self.fit();
                    }
                    ui.separator();
                    if icon_button(ui, &mut self.icons, "undo", false, "Undo  ⌘Z").clicked() {
                        self.undo();
                    }
                    if icon_button(ui, &mut self.icons, "redo", false, "Redo  ⇧⌘Z").clicked() {
                        self.redo();
                    }
                });
            });
        });
        if self.doc.elements.iter().any(|e| e.kind == Kind::Arrow && e.style.flow != Flow::None) {
            egui::Area::new(Id::new("play")).anchor(Align2::RIGHT_BOTTOM, vec2(-12.0, -12.0)).show(ctx, |ui| {
                panel().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon, tip) = if self.playing { ("pause", "Pause flow animation") } else { ("play", "Play flow animation") };
                        if icon_button(ui, &mut self.icons, icon, self.playing, tip).clicked() {
                            self.playing = !self.playing;
                        }
                        ui.label(RichText::new("Flow").color(GRAY_700));
                        ui.add_space(6.0);
                    });
                });
            });
        }
    }

    fn text_editor(&mut self, ctx: &Context) {
        let Some(id) = self.editing else { return };
        let Some(e) = self.doc.get(id).cloned() else {
            self.editing = None;
            return;
        };
        let v = self.view();
        let z = self.zoom;
        let r = e.rect;
        let (pos, width, size, bold, color, align) = match e.kind {
            Kind::Text => (v.w2s(r.min), (r.width() + 40.0) * z, e.style.font, false, e.style.stroke, Align::Min),
            Kind::Node => {
                let l = node_layout(r);
                (v.w2s(pos2(l.text_x, r.min.y + 8.0)), (r.max.x - l.text_x - 8.0) * z, NODE_TITLE, false, INK, Align::Min)
            }
            Kind::Region => (v.w2s(r.min + vec2(14.0, 8.0)), 240.0 * z, e.style.font, true, e.style.stroke, Align::Min),
            Kind::Arrow => {
                let c = render::arrow_label_center(&self.doc.arrow_path(&e));
                (v.w2s(c - vec2(90.0, 10.0)), 180.0 * z, (e.style.font * 0.7).max(11.0), false, e.style.stroke, Align::Center)
            }
            _ => {
                let lines = e.text.split('\n').count().max(1) as f32;
                let h = lines * e.style.font * LINE_H;
                (v.w2s(pos2(r.min.x, r.center().y - h / 2.0)), r.width() * z, e.style.font, false, e.style.stroke, Align::Center)
            }
        };
        let mut done = false;
        let focus = std::mem::take(&mut self.edit_focus);
        let mut text = e.text.clone();
        egui::Area::new(Id::new("editor")).fixed_pos(pos).order(Order::Foreground).show(ctx, |ui| {
            let te = TextEdit::multiline(&mut text)
                .font(render::font(size * z, bold))
                .text_color(color)
                .frame(false)
                .desired_width(width.max(40.0))
                .desired_rows(1)
                .margin(Margin::ZERO)
                .horizontal_align(align);
            let r = ui.add(te);
            if focus {
                r.request_focus();
            }
            if r.lost_focus() {
                done = true;
            }
        });
        if let Some(el) = self.doc.get_mut(id) {
            el.text = text;
        }
        if done {
            self.finish_edit();
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(s) = serde_json::to_string(&self.doc) {
            storage.set_string("doc", s);
        }
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.export_rx {
            if let Ok(r) = rx.try_recv() {
                self.export_rx = None;
                match r {
                    Ok(p) => self.toast(ctx, format!("Exported → {p}")),
                    Err(e) => self.toast(ctx, format!("Export failed: {e}")),
                }
            }
        }
        let name = self.path.as_ref().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().to_string()).unwrap_or("Untitled".into());
        let title = format!("{name}{} — idraw", if self.dirty { " •" } else { "" });
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }

        self.shortcuts(ctx);

        // keep text boxes sized to their content
        for e in self.doc.elements.iter_mut().filter(|e| e.kind == Kind::Text) {
            e.rect.max = e.rect.min + render::measure(ctx, &e.text, e.style.font);
        }

        egui::CentralPanel::default().frame(Frame::new().fill(CANVAS_BG)).show(ctx, |ui| {
            let rect = ui.max_rect();
            self.canvas = rect;
            if self.fit_pending {
                self.fit_pending = false;
                self.fit();
            }
            let resp = ui.interact(rect, Id::new("canvas"), Sense::click_and_drag());
            self.canvas_input(ctx, &resp);
            let space = ctx.input(|i| i.key_down(Key::Space)) && !ctx.wants_keyboard_input();
            if matches!(self.drag, Drag::Pan) || space || self.tool == Tool::Hand {
                ctx.set_cursor_icon(if matches!(self.drag, Drag::Pan) { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
            } else if !matches!(self.tool, Tool::Select) {
                ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            }

            let p = ui.painter_at(rect);
            let v = self.view();
            render::draw_grid(&p, rect, &v);
            let t = ctx.input(|i| i.time) as f32;
            let phase = if self.playing { (t % LOOP_SECS) / LOOP_SECS } else { 0.0 };
            render::draw_doc(&p, ctx, &self.doc, &v, &mut self.icons, phase, self.editing);
            self.paint_overlays(&p);

            if self.doc.elements.is_empty() {
                let c = rect.center();
                p.text(c - vec2(0.0, 14.0), Align2::CENTER_CENTER, "Start your diagram", FontId::proportional(22.0), Color32::from_rgb(0x9c, 0xa3, 0xaf));
                p.text(
                    c + vec2(0.0, 16.0),
                    Align2::CENTER_CENTER,
                    "Drag a component from the library  ·  R rectangle  ·  A connector  ·  double-click for text",
                    FontId::proportional(13.0),
                    Color32::from_rgb(0xb4, 0xb9, 0xc2),
                );
            }
        });

        self.toolbar(ctx);
        self.top_left(ctx);
        self.top_right(ctx);
        self.library_panel(ctx);
        self.props_panel(ctx);
        self.bottom_bars(ctx);
        self.text_editor(ctx);

        if let Some((msg, until)) = self.toast.clone() {
            if ctx.input(|i| i.time) > until {
                self.toast = None;
            } else {
                egui::Area::new(Id::new("toast")).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -24.0)).order(Order::Tooltip).show(ctx, |ui| {
                    Frame::new().fill(Color32::from_rgb(0x11, 0x18, 0x27)).corner_radius(10).inner_margin(Margin::symmetric(14, 9)).show(ui, |ui| {
                        ui.label(RichText::new(msg).color(Color32::WHITE));
                    });
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }

        let animating = self.playing && self.doc.elements.iter().any(|e| e.kind == Kind::Arrow && e.style.flow != Flow::None);
        if animating {
            ctx.request_repaint();
        }
    }
}

// ---------- helpers ----------

fn load(p: &std::path::Path) -> Result<Doc, String> {
    serde_json::from_str(&std::fs::read_to_string(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn render_bounds(els: &[Element]) -> Rect {
    let pts: Vec<Pos2> = els
        .iter()
        .flat_map(|e| match e.kind {
            Kind::Pen => e.points.clone(),
            Kind::Arrow => vec![e.from.p, e.to.p],
            _ => vec![e.rect.min, e.rect.max],
        })
        .collect();
    bbox(&pts)
}

fn setup_style(ctx: &Context) {
    ctx.style_mut(|s| {
        s.visuals = egui::Visuals::light();
        s.visuals.selection.bg_fill = INDIGO;
        s.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        s.visuals.hyperlink_color = INDIGO;
        s.visuals.window_corner_radius = CornerRadius::same(12);
        s.visuals.menu_corner_radius = CornerRadius::same(10);
        s.visuals.panel_fill = CANVAS_BG;
        for w in [&mut s.visuals.widgets.inactive, &mut s.visuals.widgets.hovered, &mut s.visuals.widgets.active, &mut s.visuals.widgets.noninteractive, &mut s.visuals.widgets.open] {
            w.corner_radius = CornerRadius::same(7);
        }
        s.visuals.widgets.inactive.weak_bg_fill = GRAY_100;
        s.visuals.widgets.inactive.bg_fill = GRAY_100;
        s.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xe5, 0xe7, 0xeb);
        s.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        s.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        s.visuals.extreme_bg_color = GRAY_100;
        s.visuals.window_shadow = Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(28) };
        s.visuals.popup_shadow = s.visuals.window_shadow;
        s.spacing.item_spacing = vec2(6.0, 6.0);
        s.spacing.button_padding = vec2(8.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        use egui::TextStyle::*;
        for (st, size) in [(Body, 13.0), (Button, 13.0), (Small, 11.0), (Heading, 16.0), (Monospace, 12.5)] {
            if let Some(f) = s.text_styles.get_mut(&st) {
                f.size = size;
            }
        }
    });
}

fn panel() -> Frame {
    Frame::new()
        .fill(Color32::WHITE)
        .corner_radius(12)
        .stroke(Stroke::new(1.0, Color32::from_rgb(0xe8, 0xe9, 0xee)))
        .shadow(Shadow { offset: [0, 4], blur: 18, spread: 0, color: Color32::from_black_alpha(16) })
        .inner_margin(Margin::same(5))
}

fn icon_button(ui: &mut Ui, icons: &mut Icons, icon: &str, active: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
    let bg = if active {
        INDIGO_50
    } else if resp.hovered() {
        GRAY_100
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
    if let Some(t) = icons.get(ui.ctx(), icon) {
        ui.painter().image(t, Rect::from_center_size(rect.center(), vec2(18.0, 18.0)), UV, if active { INDIGO_600 } else { GRAY_700 });
    }
    if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
}

fn lib_tile(ui: &mut Ui, icons: &mut Icons, it: &(&str, &str, &str, &str)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(62.0, 64.0), Sense::click_and_drag());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), GRAY_100);
    }
    let accent = icons::accent(it.2);
    let tile = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 22.0), vec2(34.0, 34.0));
    ui.painter().rect_filled(tile, CornerRadius::same(9), tint(accent, 0.12));
    if let Some(t) = icons.get(ui.ctx(), it.0) {
        ui.painter().image(t, tile.shrink(8.0), UV, accent);
    }
    let label = if it.1.chars().count() > 10 { format!("{}…", it.1.chars().take(9).collect::<String>()) } else { it.1.to_string() };
    ui.painter().text(pos2(rect.center().x, rect.max.y - 9.0), Align2::CENTER_CENTER, label, FontId::proportional(10.5), GRAY_700);
    resp.on_hover_text(it.1).on_hover_cursor(egui::CursorIcon::Grab)
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).size(11.0).color(MUTED).strong());
}

fn swatch(ui: &mut Ui, c: Color32, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_stroke(rect.expand(2.0), CornerRadius::same(7), Stroke::new(2.0, INDIGO), StrokeKind::Outside);
    }
    if c.a() == 0 {
        p.rect(rect, CornerRadius::same(5), Color32::WHITE, Stroke::new(1.0, CARD_BORDER), StrokeKind::Inside);
        p.line_segment([rect.left_bottom() + vec2(4.0, -4.0), rect.right_top() + vec2(-4.0, 4.0)], Stroke::new(1.5, Color32::from_rgb(0xe0, 0x31, 0x31)));
    } else {
        p.rect(rect, CornerRadius::same(5), c, Stroke::new(1.0, Color32::from_black_alpha(20)), StrokeKind::Inside);
    }
    resp
}

fn seg<T: PartialEq + Copy>(ui: &mut Ui, cur: T, opts: &[(T, &str)]) -> Option<T> {
    let mut out = None;
    Frame::new().fill(GRAY_100).corner_radius(8).inner_margin(Margin::same(2)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let w = (204.0 - 2.0 * (opts.len() as f32 - 1.0)) / opts.len() as f32;
            for (v, label) in opts {
                let on = *v == cur;
                let b = egui::Button::new(RichText::new(*label).size(12.0).color(if on { INDIGO_600 } else { GRAY_700 }))
                    .fill(if on { Color32::WHITE } else { Color32::TRANSPARENT })
                    .corner_radius(6)
                    .min_size(vec2(w, 24.0));
                if ui.add(b).clicked() && !on {
                    out = Some(*v);
                }
            }
        });
    });
    out
}
