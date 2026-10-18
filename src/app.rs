use crate::collab::Collab;
use crate::export::{self, Look, Quality};
use crate::icons::{self, Icons};
use crate::model::*;
use crate::render::{self, Opts, View};
use crate::{code, import, layout, theme};
use eframe::egui::{self, Align2, Color32, Context, CornerRadius, Event, FontId, Frame, Id, Key, Modifiers, PointerButton, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

mod ui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Select,
    Hand,
    Rect,
    Ellipse,
    Diamond,
    Arrow,
    Text,
    Pen,
    Region,
    Sticky,
    Comment,
}

pub(crate) const TOOLS: [(Tool, &str, &str, Key); 11] = [
    (Tool::Select, "t-select", "Select  V", Key::V),
    (Tool::Hand, "t-hand", "Hand  H", Key::H),
    (Tool::Rect, "t-rect", "Rectangle  R", Key::R),
    (Tool::Ellipse, "t-ellipse", "Ellipse  O", Key::O),
    (Tool::Diamond, "t-diamond", "Diamond  D", Key::D),
    (Tool::Arrow, "t-arrow", "Connector  A", Key::A),
    (Tool::Text, "t-text", "Text  T", Key::T),
    (Tool::Pen, "t-pen", "Draw  P", Key::P),
    (Tool::Region, "t-region", "Group region  G", Key::G),
    (Tool::Sticky, "t-sticky", "Sticky note  S", Key::S),
    (Tool::Comment, "t-comment", "Comment  C", Key::C),
];

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
/// Hit radius (px) of a connection dot.
const ANCHOR_HIT: f32 = 11.0;

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

/// (label, shortcut, action) for the command palette.
pub(crate) const COMMANDS: &[(&str, &str, &str)] = &[
    ("Auto-layout: left → right", "⇧⌘L", "layout"),
    ("Auto-layout: top → down", "", "layout-v"),
    ("Diagram as code", "⌘/", "code"),
    ("Record a request flow", "", "record"),
    ("Request flows…", "", "scenarios"),
    ("Simulate an outage", "F", "failures"),
    ("Clear all pen drawings", "", "clear-ink"),
    ("Toggle auto-save", "", "autosave"),
    ("Toggle dark mode", "⇧⌘D", "dark"),
    ("Toggle sketch (hand-drawn) style", "⇧⌘K", "sketch"),
    ("Insert component at cursor", "/", "picker"),
    ("Import compose / Kubernetes / Terraform / Mermaid…", "⌘I", "import"),
    ("Export PNG", "⌘E", "png"),
    ("Export SVG", "", "svg"),
    ("Export animated GIF", "⇧⌘E", "gif"),
    ("Collaborate live…", "", "collab"),
    ("Open icon packs folder", "", "icons-folder"),
    ("Save", "⌘S", "save"),
    ("Save as…", "⇧⌘S", "saveas"),
    ("Open…", "⌘O", "open"),
    ("New canvas", "", "new"),
    ("Undo", "⌘Z", "undo"),
    ("Redo", "⇧⌘Z", "redo"),
    ("Zoom to fit", "⇧1", "fit"),
    ("Select all", "⌘A", "all"),
    ("Duplicate", "⌘D", "dup"),
    ("Wrap selection in region", "⌘G", "group"),
    ("Toggle components library", "L", "lib"),
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
    Label(u64),
    Bend(u64),
}

/// A boundary style the user saved locally (shown in the library's Boundaries section).
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct CustomBoundary {
    pub name: String,
    pub icon: String,
    pub color: Color32,
    pub dashed: bool,
}

pub(crate) struct BoundEditor {
    idx: Option<usize>,
    b: CustomBoundary,
    query: String,
}

fn region_element(name: &str, icon: &str, color: Color32, dashed: bool, at: Pos2) -> Element {
    let mut e = Element::new(Kind::Region, Rect::from_center_size(at, vec2(440.0, 300.0)));
    e.text = name.into();
    e.icon = icon.into();
    e.style.stroke = color;
    e.style.fill = color.gamma_multiply(if dashed { 0.05 } else { 0.08 });
    e.style.dashed = dashed;
    e
}

/// Quick component chooser: `/` at the cursor, or after dropping a connector on empty canvas.
pub(crate) struct Picker {
    world: Pos2,
    screen: Pos2,
    query: String,
    sel: usize,
    connect: Option<(u64, u64)>,
    focus: bool,
}

pub(crate) struct Palette {
    query: String,
    sel: usize,
    focus: bool,
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
    lib_filter: Option<&'static str>,
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
    drop_side: Option<u8>,
    fit_pending: bool,
    cascade: f32,
    nudge: Option<Vec2>,
    // v0.2
    dark: bool,
    failures: bool,
    quality: Quality,
    palette: Option<Palette>,
    picker: Option<Picker>,
    scn_panel: bool,
    scn_play: Option<(usize, f64)>,
    rec: Option<(String, Vec<u64>)>,
    code_open: bool,
    code_text: String,
    code_err: Option<String>,
    code_w: f32,
    anim: Option<(Vec<(u64, Rect, Rect)>, f64)>,
    collab: Option<Collab>,
    collab_panel: bool,
    join_addr: String,
    join_code: String,
    my_name: String,
    custom_bounds: Vec<CustomBoundary>,
    bound_editor: Option<BoundEditor>,
    lib_custom: usize,
    /// recording appends to this scenario instead of creating a new one
    rec_into: Option<usize>,
    /// write the .idraw file automatically shortly after changes
    autosave: bool,
    last_saved: String,
    autosave_check: f64,
    /// connector started by clicking a connection dot; follows the mouse until the next click
    pending_arrow: Option<(u64, u64)>,
    /// window focus last frame, to re-send the cursor when idraw becomes active
    was_focused: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        crate::setup_fonts(&cc.egui_ctx);
        let st = cc.storage;
        let dark = st.and_then(|s| s.get_string("dark")).map(|v| v == "1").unwrap_or_else(|| cc.egui_ctx.system_theme() == Some(egui::Theme::Dark));
        setup_style(&cc.egui_ctx, dark);
        let mut path = None;
        let doc = match std::env::args().nth(1).map(PathBuf::from) {
            Some(p) => {
                let d = load(&p).unwrap_or_default();
                path = Some(p);
                d
            }
            None => st.and_then(|s| s.get_string("doc")).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
        };
        let my_name = st.and_then(|s| s.get_string("name")).unwrap_or_else(|| std::env::var("USER").unwrap_or("me".into()));
        App {
            fit_pending: !doc.elements.is_empty(),
            doc,
            undo: vec![],
            redo: vec![],
            sel: vec![],
            tool: Tool::Select,
            offset: Vec2::ZERO,
            zoom: 1.0,
            canvas: Rect::ZERO,
            drag: Drag::None,
            editing: None,
            edit_focus: false,
            icons: Icons::default(),
            library: true,
            replace_icon: false,
            lib_query: String::new(),
            lib_filter: None,
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
            drop_side: None,
            cascade: 0.0,
            nudge: None,
            dark,
            failures: false,
            quality: Quality::UltraHd,
            palette: None,
            picker: None,
            scn_panel: false,
            scn_play: None,
            rec: None,
            code_open: false,
            code_text: String::new(),
            code_err: None,
            code_w: 0.0,
            anim: None,
            collab: None,
            collab_panel: false,
            join_addr: String::new(),
            join_code: String::new(),
            my_name,
            custom_bounds: st.and_then(|s| s.get_string("boundaries")).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            bound_editor: None,
            lib_custom: 0,
            rec_into: None,
            autosave: st.and_then(|s| s.get_string("autosave")).map(|v| v != "0").unwrap_or(true),
            last_saved: String::new(),
            autosave_check: 0.0,
            pending_arrow: None,
            was_focused: true,
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
        self.toast = Some((msg.into(), ctx.input(|i| i.time) + 2.8));
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
        let c = Rect::from_min_max(self.canvas.min + vec2(if self.sel.is_empty() { 40.0 } else { 260.0 }, 70.0), self.canvas.max - vec2(if self.library { 330.0 } else { 40.0 }, 70.0));
        self.zoom = ((c.width() - 40.0) / b.width()).min((c.height() - 40.0) / b.height()).clamp(0.2, 1.5);
        self.offset = c.center() - self.canvas.min - b.center().to_vec2() * self.zoom;
    }

    fn center_on(&mut self, w: Pos2) {
        self.offset = self.canvas.size() / 2.0 - w.to_vec2() * self.zoom;
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
            if self.doc.get(id).is_some_and(|e| matches!(e.kind, Kind::Text | Kind::Comment) && e.text.trim().is_empty()) {
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
        for e in &mut els {
            remap(&mut e.from);
            remap(&mut e.to);
            e.translate(d);
        }
        els.into_iter().map(|e| self.doc.add(e)).collect()
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
        } else if let Some(c) = code::from_mermaid(s) {
            self.merge_doc(ctx, import::to_doc(&c, "paste.mmd"));
        } else if !s.trim().is_empty() {
            let mut e = Element::new(Kind::Text, Rect::from_min_size(self.view_center(), Vec2::ZERO));
            e.text = s.trim().to_string();
            e.rect.max = e.rect.min + render::measure(ctx, &e.text, e.style.font, self.doc.sketch);
            self.add_select(e);
        }
    }

    /// Drops an imported doc next to the existing content and selects it.
    fn merge_doc(&mut self, ctx: &Context, d: Result<Doc, String>) {
        match d {
            Ok(d) if !d.elements.is_empty() => {
                let b = d.all_bounds().unwrap_or(Rect::ZERO);
                let target = match self.doc.all_bounds() {
                    Some(cur) => pos2(cur.max.x + 160.0 + b.width() / 2.0, cur.center().y),
                    None => pos2(0.0, 0.0),
                };
                self.sel = self.insert_clones(d.elements, target - b.center());
                self.fit();
                self.toast(ctx, format!("Imported {} elements", self.sel.len()));
            }
            Ok(_) => self.toast(ctx, "Nothing to import"),
            Err(e) => self.toast(ctx, format!("Import failed: {e}")),
        }
    }

    fn group_region(&mut self) {
        let Some(b) = self.doc.bounds_of(&self.sel) else { return };
        self.checkpoint();
        let mut r = Element::new(Kind::Region, Rect::from_min_max(b.min - vec2(32.0, 52.0), b.max + vec2(32.0, 32.0)));
        r.text = "Group".into();
        let id = self.add_select(r);
        self.start_edit(id);
        self.undo.pop(); // start_edit's checkpoint duplicates ours
    }

    /// Adds a component (or boundary preset) at `at` or the view center.
    fn insert_icon(&mut self, icon: &str, at: Option<Pos2>) -> u64 {
        self.checkpoint();
        let at = at.unwrap_or_else(|| {
            self.cascade = (self.cascade + 24.0) % 168.0;
            self.view_center() + vec2(self.cascade, self.cascade)
        });
        self.tool = Tool::Select;
        if let Some(b) = icons::BOUNDARIES.iter().find(|b| b.0 == icon) {
            return self.add_select(region_element(b.1, b.2, icons::boundary_color(b.3), b.4, at));
        }
        if let Some(tool) = shape_tool(icon) {
            if tool == Tool::Text {
                let e = Element::new(Kind::Text, Rect::from_min_size(at - vec2(20.0, 12.0), Vec2::ZERO));
                let id = self.add_select(e);
                self.editing = Some(id);
                self.edit_focus = true;
                return id;
            }
            let e = self.new_shape(tool, Rect::from_center_size(at, Self::default_size(tool)));
            return self.add_select(e);
        }
        self.add_select(Element::node(icon, at))
    }

    fn insert_custom_boundary(&mut self, i: usize) {
        let Some(b) = self.custom_bounds.get(i).cloned() else { return };
        self.checkpoint();
        self.cascade = (self.cascade + 24.0) % 168.0;
        let at = self.view_center() + vec2(self.cascade, self.cascade);
        self.tool = Tool::Select;
        self.add_select(region_element(&b.name, &b.icon, b.color, b.dashed, at));
    }

    fn set_icon(&mut self, icon: &str) {
        let Some(i) = icons::find(icon) else { return };
        if icon.starts_with("b-") {
            return;
        }
        self.checkpoint();
        // regions: just swap the header icon
        for id in self.sel.clone() {
            if let Some(e) = self.doc.get_mut(id).filter(|e| e.kind == Kind::Region) {
                e.icon = icon.to_string();
            }
        }
        for id in self.sel.clone() {
            if let Some(e) = self.doc.get_mut(id).filter(|e| e.kind == Kind::Node) {
                let old = icons::find(&e.icon).map(|o| o.label);
                if old == Some(node_text(&e.text).0) {
                    e.text = if i.sub.is_empty() { i.label.to_string() } else { format!("{}\n{}", i.label, i.sub) };
                }
                e.icon = icon.to_string();
                e.style.stroke = icons::accent(i.cat);
            }
        }
        self.replace_icon = false;
    }

    fn set_dark(&mut self, ctx: &Context, d: bool) {
        self.dark = d;
        setup_style(ctx, d);
    }

    fn auto_layout(&mut self, vertical: bool) {
        let scope = if self.sel.len() > 1 { self.sel.clone() } else { vec![] };
        let targets = layout::layered(&self.doc, &scope, vertical);
        if targets.is_empty() {
            return;
        }
        self.checkpoint();
        let items = targets.into_iter().filter_map(|(id, r)| self.doc.get(id).map(|e| (id, e.rect, r))).collect();
        self.anim = Some((items, 0.0));
    }

    fn start_record(&mut self) {
        self.rec_into = None;
        let n = self.doc.scenarios.len() + 1;
        self.rec = Some((format!("Flow {n}"), vec![]));
        self.scn_play = None;
        self.sel.clear();
        self.tool = Tool::Select;
    }

    fn finish_record(&mut self, ctx: &Context) {
        let Some((name, picks)) = self.rec.take() else { return };
        let into = self.rec_into.take().filter(|i| *i < self.doc.scenarios.len());
        let steps = self.doc.steps_from_picks(&picks, None);
        if steps.is_empty() {
            return self.toast(ctx, "Click components or connectors in the order the request travels");
        }
        self.checkpoint();
        let idx = match into {
            Some(i) => {
                self.doc.scenarios[i].steps.extend(steps);
                i
            }
            None => {
                self.doc.scenarios.push(Scenario { name, steps });
                self.doc.scenarios.len() - 1
            }
        };
        self.scn_panel = true;
        self.scn_play = Some((idx, ctx.input(|i| i.time)));
    }

    fn open_picker(&mut self, screen: Pos2, connect: Option<(u64, u64)>) {
        self.picker = Some(Picker { world: self.view().s2w(screen), screen, query: String::new(), sel: 0, connect, focus: true });
    }

    // ---------- actions ----------

    fn run(&mut self, ctx: &Context, a: &str) {
        match a {
            "undo" => self.undo(),
            "redo" => self.redo(),
            "save" => self.save(ctx, false),
            "saveas" => self.save(ctx, true),
            "open" => self.open(ctx),
            "import" => self.import(ctx),
            "png" => self.export(ctx, "png", None),
            "svg" => self.export(ctx, "svg", None),
            "gif" => self.export(ctx, "gif", None),
            "layout" => self.auto_layout(false),
            "layout-v" => self.auto_layout(true),
            "dark" => self.set_dark(ctx, !self.dark),
            "sketch" => {
                self.checkpoint();
                self.doc.sketch = !self.doc.sketch;
            }
            "failures" => {
                self.failures = !self.failures;
                self.sel.clear();
            }
            "autosave" => {
                self.autosave = !self.autosave;
                if self.autosave && self.path.is_none() {
                    self.save(ctx, true); // auto-save needs a file to write to
                }
                self.toast(ctx, if self.autosave { "Auto-save on: changes are written to the file" } else { "Auto-save off: save with ⌘S" });
            }
            "clear-ink" => {
                let ink: Vec<u64> = self.doc.elements.iter().filter(|e| e.kind == Kind::Pen).map(|e| e.id).collect();
                if !ink.is_empty() {
                    self.checkpoint();
                    self.doc.remove(&ink);
                    self.toast(ctx, format!("Cleared {} drawings · ⌘Z to undo", ink.len()));
                }
            }
            "code" => self.code_open = !self.code_open,
            "record" => self.start_record(),
            "scenarios" => self.scn_panel = !self.scn_panel,
            "collab" => self.collab_panel = !self.collab_panel,
            "picker" => {
                let p = ctx.input(|i| i.pointer.hover_pos()).filter(|p| self.canvas.contains(*p)).unwrap_or(self.canvas.center());
                self.open_picker(p, None);
            }
            "icons-folder" => {
                let d = icons::custom_dir();
                let _ = std::fs::create_dir_all(&d);
                let _ = std::process::Command::new("open").arg(&d).spawn();
                self.toast(ctx, "Drop SVG packs here (one folder per category), then restart idraw");
            }
            "new" => {
                self.checkpoint();
                self.doc = Doc::default();
                self.sel.clear();
                self.path = None;
            }
            "fit" => self.fit(),
            "all" => self.sel = self.doc.elements.iter().map(|e| e.id).collect(),
            "dup" => self.duplicate(),
            "group" => self.group_region(),
            "lib" => self.library = !self.library,
            "zin" => self.zoom_at(self.canvas.center(), 1.2),
            "zout" => self.zoom_at(self.canvas.center(), 1.0 / 1.2),
            "z100" => self.zoom_at(self.canvas.center(), 1.0 / self.zoom),
            "del" => self.delete_selected(),
            "palette" => self.palette = Some(Palette { query: String::new(), sel: 0, focus: true }),
            "esc" => {
                if let Some((id, _)) = self.pending_arrow.take() {
                    self.doc.remove(&[id]);
                    self.undo.pop();
                    self.sel.clear();
                    self.drop_target = None;
                    return;
                }
                if std::mem::take(&mut self.failures) {
                    return;
                }
                if self.rec.take().is_some() || self.scn_play.take().is_some() || self.picker.take().is_some() {
                    return;
                }
                self.sel.clear();
                self.tool = Tool::Select;
                self.menu = false;
                self.export_menu = false;
                self.replace_icon = false;
            }
            _ => {
                if let Some(id) = a.strip_prefix("insert:") {
                    self.insert_icon(id, None);
                } else if let Some(id) = a.strip_prefix("goto:").and_then(|s| s.parse::<u64>().ok()) {
                    if let Some(c) = self.doc.get(id).map(|e| self.doc.bounds(e).center()) {
                        self.sel = vec![id];
                        self.center_on(c);
                    }
                }
            }
        }
    }

    // ---------- files ----------

    fn save(&mut self, ctx: &Context, ask: bool) {
        let path = match (&self.path, ask) {
            (Some(p), false) => Some(p.clone()),
            _ => rfd::FileDialog::new().add_filter("idraw", &["idraw"]).set_file_name("diagram.idraw").save_file(),
        };
        let Some(p) = path else { return };
        let json = serde_json::to_string_pretty(&self.doc).unwrap_or_default();
        match std::fs::write(&p, &json).map_err(|e| e.to_string()) {
            Ok(()) => {
                self.dirty = false;
                self.last_saved = json;
                self.toast(ctx, format!("Saved {}", p.file_name().unwrap_or_default().to_string_lossy()));
                self.path = Some(p);
            }
            Err(e) => self.toast(ctx, format!("Save failed: {e}")),
        }
    }

    /// Writes the file about a second after the last change (when auto-save is on and it has a path).
    fn autosave_tick(&mut self, ctx: &Context, now: f64) {
        if !self.autosave || now - self.autosave_check < 1.0 || !matches!(self.drag, Drag::None) || self.anim.is_some() {
            return;
        }
        self.autosave_check = now;
        let Some(p) = self.path.clone() else { return };
        let json = serde_json::to_string_pretty(&self.doc).unwrap_or_default();
        if self.last_saved.is_empty() {
            // baseline = the doc as opened; only real edits trigger a write
            self.last_saved = json;
            return;
        }
        if json != self.last_saved {
            match std::fs::write(&p, &json) {
                Ok(()) => {
                    self.last_saved = json;
                    self.dirty = false;
                }
                Err(e) => {
                    self.autosave = false;
                    self.toast(ctx, format!("Auto-save stopped: {e}"));
                }
            }
        }
    }

    fn open(&mut self, ctx: &Context) {
        let Some(p) = rfd::FileDialog::new().add_filter("idraw", &["idraw", "json"]).pick_file() else { return };
        match load(&p) {
            Ok(d) => {
                self.checkpoint();
                self.doc = d;
                self.sel.clear();
                self.last_saved = serde_json::to_string_pretty(&self.doc).unwrap_or_default();
                self.path = Some(p);
                self.dirty = false;
                self.fit();
            }
            Err(e) => self.toast(ctx, format!("Open failed: {e}")),
        }
    }

    fn import(&mut self, ctx: &Context) {
        let Some(p) = rfd::FileDialog::new()
            .add_filter("Infrastructure & diagrams", &["yml", "yaml", "tf", "mmd", "mermaid", "md", "txt"])
            .set_title("Import docker-compose, Kubernetes, Terraform or Mermaid")
            .pick_file()
        else {
            return;
        };
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        let res = std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| import::to_doc(&s, &name));
        if res.is_ok() {
            self.checkpoint();
        }
        self.merge_doc(ctx, res);
    }

    /// `scenario`: export that scenario as an animated GIF instead.
    fn export(&mut self, ctx: &Context, fmt: &'static str, scenario: Option<usize>) {
        self.export_menu = false;
        if self.doc.elements.is_empty() {
            return self.toast(ctx, "Nothing to export");
        }
        let stem = self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or("diagram".into());
        let stem = match scenario.and_then(|i| self.doc.scenarios.get(i)) {
            Some(s) => format!("{stem}-{}", code::slug(&s.name)),
            None => stem,
        };
        let Some(p) = rfd::FileDialog::new().add_filter(fmt, &[fmt]).set_file_name(format!("{stem}.{fmt}")).save_file() else { return };
        let (doc, only, q, look) = (self.doc.clone(), self.sel.clone(), self.quality, Look { dark: self.dark, failures: self.failures });
        let ctx2 = ctx.clone();
        let (tx, rx) = channel();
        self.export_rx = Some(rx);
        self.toast = Some((format!("Exporting {}…", fmt.to_uppercase()), f64::MAX));
        std::thread::spawn(move || {
            let r = match (fmt, scenario) {
                (_, Some(i)) => export::write_scenario_gif(&doc, &doc.scenarios[i], q, look, &p),
                ("png", _) => export::write_png(&doc, &only, q, look, &p),
                ("svg", _) => export::write_svg(&doc, &only, look, &p),
                _ => export::write_gif(&doc, &only, q, look, &p),
            };
            let _ = tx.send(r.map(|_| p.display().to_string()));
            ctx2.request_repaint();
        });
    }

    // ---------- input ----------

    fn shortcuts(&mut self, ctx: &Context) {
        let mut act: Vec<&str> = vec![];
        let typing = ctx.wants_keyboard_input();
        let cmd = Modifiers::COMMAND;
        let shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
        ctx.input_mut(|i| {
            if i.consume_key(cmd, Key::K) {
                act.push("palette");
            }
            if i.consume_key(cmd, Key::Slash) {
                act.push("code");
            }
            if typing {
                return;
            }
            for (k, m, name) in [
                (Key::Z, shift_cmd, "redo"),
                (Key::Z, cmd, "undo"),
                (Key::S, shift_cmd, "saveas"),
                (Key::S, cmd, "save"),
                (Key::O, cmd, "open"),
                (Key::I, cmd, "import"),
                (Key::A, cmd, "all"),
                (Key::D, shift_cmd, "dark"),
                (Key::D, cmd, "dup"),
                (Key::G, cmd, "group"),
                (Key::L, shift_cmd, "layout"),
                (Key::K, shift_cmd, "sketch"),
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
                        self.nudge = Some(d * if i.modifiers.shift { 10.0 } else { 1.0 });
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
                if i.key_pressed(Key::F) {
                    act.push("failures");
                }
                if i.key_pressed(Key::Slash) {
                    act.push("picker");
                }
                if i.key_pressed(Key::Enter) && self.rec.is_some() {
                    act.push("rec-done");
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
            if a == "rec-done" {
                self.finish_record(ctx);
            } else {
                self.run(ctx, a);
            }
        }
        if typing {
            return;
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
            [id] => self.doc.get(*id).filter(|e| !matches!(e.kind, Kind::Arrow | Kind::Comment)),
            _ => None,
        }
    }

    /// Cursor for the select tool: what would happen if you pressed here.
    fn select_cursor(&self, hover: Option<Pos2>) -> egui::CursorIcon {
        use egui::CursorIcon as C;
        let resize = |h: usize| [C::ResizeNwSe, C::ResizeVertical, C::ResizeNeSw, C::ResizeHorizontal][h % 4];
        match &self.drag {
            Drag::Move { .. } | Drag::Bend(_) => return C::Move,
            Drag::Resize { handle, .. } => return resize(*handle),
            Drag::ArrowEnd { .. } => return C::Crosshair,
            Drag::Marquee(_) => return C::Default,
            _ => {}
        }
        if self.pending_arrow.is_some() {
            return C::Crosshair;
        }
        let Some(h) = hover else { return C::Default };
        if self.anchor_at(h).is_some() {
            return C::Crosshair;
        }
        if let Some(e) = self.single_box() {
            if let Some(i) = Self::handles(self.view().rect(e.rect)).iter().position(|p| p.distance(h) < 8.0) {
                return resize(i);
            }
        }
        let w = self.view().s2w(h);
        let inside_selected_region = self.sel.iter().any(|id| self.doc.get(*id).is_some_and(|e| e.kind == Kind::Region && e.rect.contains(w)));
        if self.doc.hit(w, 6.0 / self.zoom).is_some() || inside_selected_region { C::Move } else { C::Default }
    }

    /// Shape (and side, if a side dot is under the pointer) a connector end would attach to.
    fn drop_at(&self, w: Pos2, skip: Option<u64>) -> (Option<u64>, Option<u8>) {
        let snap = 14.0 / self.zoom;
        for e in self.doc.elements.iter().rev().filter(|e| e.bindable() && Some(e.id) != skip) {
            if !e.rect.expand(snap).contains(w) {
                continue;
            }
            let (i, d) = (0..SLOTS).map(|i| (i, side_point(e.rect, i).distance(w))).min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
            if d < snap {
                return (Some(e.id), Some(i));
            }
            if e.rect.expand(4.0).contains(w) {
                return (Some(e.id), None);
            }
        }
        (None, None)
    }

    /// Connection anchors (screen) around a shape, offset outward.
    /// Connection dots (screen): (dot, point on the outline, slot). 3 per side, just outside the shape.
    fn anchors(&self, id: u64) -> Vec<(Pos2, Pos2, u8)> {
        let v = self.view();
        let Some(e) = self.doc.get(id).filter(|e| e.bindable()) else { return vec![] };
        let normals = [vec2(0.0, -1.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(-1.0, 0.0)];
        (0..SLOTS)
            .map(|i| {
                let edge = v.w2s(side_point(e.rect, i));
                (edge + normals[slot_side(i).0 as usize] * 13.0, edge, i)
            })
            .collect()
    }

    /// Connection dot under screen point `sp` on the hovered shape: (shape, slot).
    fn anchor_at(&self, sp: Pos2) -> Option<(u64, u8)> {
        let src = self.hover_shape?;
        self.anchors(src).into_iter().filter(|(a, _, _)| a.distance(sp) < ANCHOR_HIT).min_by(|x, y| x.0.distance(sp).total_cmp(&y.0.distance(sp))).map(|(_, _, i)| (src, i))
    }

    fn start_connector(&mut self, src: u64, slot: u8, w: Pos2) -> u64 {
        self.checkpoint();
        let mut a = Element::new(Kind::Arrow, Rect::ZERO);
        // the dot you grabbed is where the connector leaves from
        a.from = End { id: Some(src), p: w, side: Some(slot) };
        a.to = End { id: None, p: w, side: None };
        let id = self.doc.add(a);
        self.sel = vec![id];
        id
    }

    /// Connector whose label pill is under `w`.
    fn label_hit(&self, ctx: &Context, w: Pos2) -> Option<u64> {
        self.doc.elements.iter().rev().filter(|e| e.kind == Kind::Arrow && !e.text.is_empty()).find_map(|e| {
            let c = render::arrow_label_center(&self.doc.arrow_path(e), e);
            let m = render::measure(ctx, &e.text, render::label_size(e), self.doc.sketch);
            Rect::from_center_size(c, m + vec2(18.0, 8.0)).contains(w).then_some(e.id)
        })
    }

    fn begin_select_drag(&mut self, ctx: &Context, sp: Pos2, w: Pos2, shift: bool) {
        let v = self.view();
        if let Some(id) = self.label_hit(ctx, w) {
            self.checkpoint();
            self.sel = vec![id];
            self.drag = Drag::Label(id);
            return;
        }
        if let Some(e) = self.single_box() {
            if let Some(h) = Self::handles(v.rect(e.rect)).iter().position(|h| h.distance(sp) < 8.0) {
                let orig = e.clone();
                self.checkpoint();
                self.drag = Drag::Resize { handle: h, orig };
                return;
            }
        }
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
        if let Some((src, slot)) = self.anchor_at(sp) {
            let id = self.start_connector(src, slot, w);
            self.drag = Drag::ArrowEnd { id, to: true, source: Some(src) };
            return;
        }
        let hit = self.doc.hit(w, 6.0 / self.zoom);
        // dragging a connected line reshapes it instead of moving it
        if let Some(id) = hit.filter(|id| self.doc.get(*id).is_some_and(|e| e.kind == Kind::Arrow && (e.from.id.is_some() || e.to.id.is_some())) && !shift) {
            self.checkpoint();
            self.sel = vec![id];
            self.drag = Drag::Bend(id);
            return;
        }
        // a selected boundary can be dragged from anywhere inside it
        let hit = hit.or_else(|| self.sel.iter().rev().copied().find(|id| self.doc.get(*id).is_some_and(|e| e.kind == Kind::Region && e.rect.contains(w))));
        if let Some(id) = hit {
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
        for e in self.doc.elements.iter().filter(|e| !exclude.contains(&e.id) && !matches!(e.kind, Kind::Arrow | Kind::Comment)) {
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

    fn new_shape(&self, tool: Tool, r: Rect) -> Element {
        let kind = match tool {
            Tool::Rect => Kind::Rect,
            Tool::Ellipse => Kind::Ellipse,
            Tool::Diamond => Kind::Diamond,
            Tool::Sticky => Kind::Sticky,
            _ => Kind::Region,
        };
        let mut e = Element::new(kind, r);
        if kind == Kind::Region {
            e.text = "Region".into();
        }
        e
    }

    fn default_size(tool: Tool) -> Vec2 {
        match tool {
            Tool::Rect => vec2(140.0, 80.0),
            Tool::Ellipse => vec2(120.0, 80.0),
            Tool::Diamond => vec2(120.0, 90.0),
            Tool::Sticky => vec2(200.0, 160.0),
            _ => vec2(420.0, 280.0),
        }
    }

    fn canvas_input(&mut self, ctx: &Context, resp: &Response) {
        let v = self.view();
        let (mods, space, scroll, zd, hover) = ctx.input(|i| (i.modifiers, i.key_down(Key::Space), i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
        let space = space && !ctx.wants_keyboard_input();
        // panning by drag only with Space held or the middle button (two-finger scroll always pans)
        let pan_drag = resp.dragged_by(PointerButton::Middle) || (resp.dragged() && space);
        if resp.hovered() {
            self.offset += scroll;
            if zd != 1.0 {
                if let Some(p) = hover {
                    self.zoom_at(p, zd);
                }
            }
        }
        let presenting = self.scn_play.is_some();
        if matches!(self.drag, Drag::None) && self.tool == Tool::Select && !presenting && self.rec.is_none() {
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
            self.picker = None;
        }

        // recording a scenario: clicks pick components in order
        if self.rec.is_some() {
            if resp.clicked() {
                let w = v.s2w(resp.interact_pointer_pos().unwrap_or_default());
                // components or existing connectors (incl. their labels)
                let id = self.label_hit(ctx, w).or_else(|| self.doc.hit(w, 8.0 / self.zoom)).filter(|id| self.doc.get(*id).is_some_and(|e| e.kind == Kind::Arrow || e.bindable()));
                if let (Some(id), Some((_, picks))) = (id, &mut self.rec) {
                    if picks.last() != Some(&id) {
                        picks.push(id);
                    }
                }
            }
            if pan_drag {
                self.offset += resp.drag_delta();
            }
            return;
        }
        if presenting {
            if pan_drag {
                self.offset += resp.drag_delta();
            }
            return;
        }
        // connector started by a click: it follows the mouse; the next click places it
        if let Some((id, src)) = self.pending_arrow {
            let sp = ctx.input(|i| i.pointer.latest_pos()).or(hover).unwrap_or_default();
            let w = v.s2w(sp);
            let (t, side) = self.drop_at(w, Some(src));
            self.drop_target = t;
            self.drop_side = side;
            if let Some(e) = self.doc.get_mut(id) {
                e.to = End { id: t, p: w, side };
            }
            // place on click, or on release if you pressed and dragged to the target
            if resp.clicked() || resp.drag_stopped() {
                self.pending_arrow = None;
                self.drop_target = None;
                self.drop_side = None;
                if t.is_none() {
                    self.open_picker(sp, Some((id, src)));
                }
            }
            return;
        }
        // outage simulation: click a component to take it down / bring it back
        if self.failures {
            if resp.clicked() {
                let w = v.s2w(resp.interact_pointer_pos().unwrap_or_default());
                if let Some(id) = self.doc.hit_bindable(w, None) {
                    self.checkpoint();
                    if let Some(e) = self.doc.get_mut(id) {
                        e.down = !e.down;
                    }
                }
            }
            if pan_drag {
                self.offset += resp.drag_delta();
            }
            return;
        }

        if resp.drag_started_by(PointerButton::Middle) || (resp.drag_started_by(PointerButton::Primary) && (space || self.tool == Tool::Hand)) {
            self.drag = Drag::Pan;
        } else if resp.drag_started_by(PointerButton::Primary) {
            let sp = ctx.input(|i| i.pointer.press_origin()).unwrap_or_default();
            let w = v.s2w(sp);
            self.finish_edit();
            match self.tool {
                Tool::Select => self.begin_select_drag(ctx, sp, w, mods.shift),
                Tool::Rect | Tool::Ellipse | Tool::Diamond | Tool::Region | Tool::Sticky => {
                    self.checkpoint();
                    let e = self.new_shape(self.tool, Rect::from_min_size(w, Vec2::ZERO));
                    let id = self.add_select(e);
                    self.drag = Drag::Create { id, start: w };
                }
                Tool::Arrow => {
                    self.checkpoint();
                    let mut a = Element::new(Kind::Arrow, Rect::ZERO);
                    let (id, side) = self.drop_at(w, None);
                    a.from = End { id, p: w, side };
                    a.to = End { id: None, p: w, side: None };
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
                Tool::Text | Tool::Hand | Tool::Comment => {}
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
                    let (t, side) = self.drop_at(w, other);
                    self.drop_target = t;
                    self.drop_side = side;
                    if let Some(e) = self.doc.get_mut(id) {
                        *(if to { &mut e.to } else { &mut e.from }) = End { id: t, p: w, side };
                    }
                }
                Drag::Bend(id) => {
                    let id = *id;
                    if let Some(e) = self.doc.get_mut(id) {
                        e.bend = Some(w);
                    }
                }
                Drag::Label(id) => {
                    let id = *id;
                    if let Some(path) = self.doc.get(id).map(|e| self.doc.arrow_path(e)) {
                        let (t, off) = project(&path, w);
                        // snap back onto the line when close
                        let off = if off.abs() < 10.0 / self.zoom { 0.0 } else { off.clamp(-120.0, 120.0) };
                        if let Some(e) = self.doc.get_mut(id) {
                            (e.label_t, e.label_off) = (t.clamp(0.03, 0.97), off);
                        }
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
            let sp = resp.interact_pointer_pos().or(hover).unwrap_or_default();
            let w = v.s2w(sp);
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
                    let tool = self.tool;
                    if let Some(e) = self.doc.get_mut(id) {
                        if e.rect.width() < 6.0 && e.rect.height() < 6.0 {
                            e.rect = Rect::from_center_size(start, Self::default_size(tool));
                        }
                    }
                    self.tool = Tool::Select;
                    if tool == Tool::Sticky {
                        self.editing = Some(id);
                        self.edit_focus = true;
                    }
                }
                Drag::ArrowEnd { id, to, source } => {
                    self.drop_target = None;
                    self.drop_side = None;
                    let len = self.doc.get(id).map(|e| path_len(&self.doc.arrow_path(e))).unwrap_or(0.0);
                    let free = self.doc.get(id).is_some_and(|e| if to { e.to.id.is_none() } else { e.from.id.is_none() });
                    if len < 8.0 && free {
                        self.doc.remove(&[id]);
                        self.undo.pop();
                        self.sel.clear();
                    } else if let (Some(src), true) = (source, free) {
                        // Dropped on empty canvas: ask what to connect to.
                        self.open_picker(sp, Some((id, src)));
                    }
                    self.tool = Tool::Select;
                }
                _ => {}
            }
            self.guides.clear();
        }

        if resp.clicked() {
            let sp = resp.interact_pointer_pos().unwrap_or_default();
            let w = v.s2w(sp);
            if self.editing.is_some() {
                self.finish_edit();
                return;
            }
            if self.tool == Tool::Select {
                if let Some((src, slot)) = self.anchor_at(sp) {
                    let id = self.start_connector(src, slot, w);
                    self.pending_arrow = Some((id, src));
                    return;
                }
            }
            match self.tool {
                Tool::Select | Tool::Hand | Tool::Arrow | Tool::Pen => match self.doc.hit(w, 6.0 / self.zoom) {
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
                },
                Tool::Rect | Tool::Ellipse | Tool::Diamond | Tool::Region | Tool::Sticky => {
                    self.checkpoint();
                    let tool = self.tool;
                    let e = self.new_shape(tool, Rect::from_center_size(w, Self::default_size(tool)));
                    let id = self.add_select(e);
                    self.tool = Tool::Select;
                    if tool == Tool::Sticky {
                        self.editing = Some(id);
                        self.edit_focus = true;
                    }
                }
                Tool::Text => {
                    self.create_text(w);
                    self.tool = Tool::Select;
                }
                Tool::Comment => {
                    self.checkpoint();
                    let id = self.add_select(Element::new(Kind::Comment, Rect::from_min_size(w, Vec2::ZERO)));
                    self.editing = Some(id);
                    self.edit_focus = true;
                    self.tool = Tool::Select;
                }
            }
        }

        if resp.double_clicked() {
            let w = v.s2w(resp.interact_pointer_pos().unwrap_or_default());
            match self.label_hit(ctx, w).or_else(|| self.doc.hit(w, 6.0 / self.zoom)) {
                Some(id) => self.start_edit(id),
                None => self.create_text(w),
            }
        }

        if let Some(icon) = self.lib_drag {
            if ctx.input(|i| i.pointer.any_released()) {
                self.lib_drag = None;
                if let Some(p) = hover.filter(|p| self.canvas.contains(*p) && !ctx.is_pointer_over_area()) {
                    self.insert_icon(icon, Some(v.s2w(p)));
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

    fn step_anim(&mut self, now: f64) {
        let Some((items, start)) = &mut self.anim else { return };
        if *start == 0.0 {
            *start = now;
        }
        let t = ((now - *start) / 0.5) as f32;
        let k = ease(t);
        let items = items.clone();
        for (id, a, b) in &items {
            if let Some(e) = self.doc.get_mut(*id) {
                e.rect = Rect::from_min_max(a.min.lerp(b.min, k), a.max.lerp(b.max, k));
            }
        }
        if t >= 1.0 {
            self.anim = None;
        }
    }

    // ---------- painting overlays ----------

    fn paint_overlays(&self, p: &egui::Painter) {
        let v = self.view();
        let sel_stroke = Stroke::new(1.25, theme::INDIGO);
        for g in &self.guides {
            p.line_segment([v.w2s(g[0]), v.w2s(g[1])], Stroke::new(1.0, theme::GUIDE));
        }
        // boundaries are grabbed by their edge / title: show it when the pointer is there
        if let (Drag::None, Tool::Select, Some(h)) = (&self.drag, self.tool, p.ctx().input(|i| i.pointer.hover_pos())) {
            if let Some(e) = self.doc.hit(v.s2w(h), 6.0 / self.zoom).and_then(|id| self.doc.get(id)).filter(|e| e.kind == Kind::Region && !self.sel.contains(&e.id)) {
                let r = v.rect(e.rect);
                p.rect_stroke(r, CornerRadius::same((16.0 * self.zoom).min(255.0) as u8), Stroke::new(2.5, e.style.stroke.gamma_multiply(0.8)), StrokeKind::Middle);
            }
        }
        if let Some(t) = self.drop_target.and_then(|id| self.doc.get(id)) {
            let r = v.rect(t.rect).expand(4.0);
            p.rect(r, CornerRadius::same(10), theme::INDIGO.gamma_multiply(0.06), Stroke::new(1.5, theme::INDIGO.gamma_multiply(0.7)), StrokeKind::Outside);
            // side dots: drop on one to pin that side, or on the body to let idraw choose
            for i in 0..SLOTS {
                let c = v.w2s(side_point(t.rect, i));
                let hot = self.drop_side == Some(i);
                p.circle(c, if hot { 7.0 } else { 4.5 }, if hot { theme::INDIGO } else { Color32::WHITE }, Stroke::new(1.8, theme::INDIGO));
            }
        }
        for id in &self.sel {
            let Some(e) = self.doc.get(*id) else { continue };
            match e.kind {
                Kind::Arrow => {
                    let path = self.doc.arrow_path(e);
                    p.add(egui::Shape::line(path.iter().map(|q| v.w2s(*q)).collect(), Stroke::new(e.style.width * self.zoom + 4.0, theme::INDIGO.gamma_multiply(0.18))));
                    if self.sel.len() == 1 {
                        for q in [path[0], *path.last().unwrap()] {
                            p.circle(v.w2s(q), 5.0, Color32::WHITE, Stroke::new(1.5, theme::INDIGO));
                        }
                        if let Some(b) = e.bend {
                            p.circle(v.w2s(b), 5.5, theme::INDIGO, Stroke::new(2.0, Color32::WHITE));
                        }
                    }
                }
                Kind::Comment => {
                    p.circle_stroke(v.w2s(e.rect.min), 16.0, Stroke::new(2.0, theme::INDIGO));
                }
                _ => {
                    p.rect_stroke(v.rect(self.doc.bounds(e)).expand(4.0), CornerRadius::same(4), sel_stroke, StrokeKind::Middle);
                }
            }
        }
        if let Some(e) = self.single_box() {
            for h in Self::handles(v.rect(e.rect)) {
                p.rect(Rect::from_center_size(h, vec2(9.0, 9.0)), CornerRadius::same(3), Color32::WHITE, Stroke::new(1.5, theme::INDIGO), StrokeKind::Middle);
            }
        } else if self.sel.len() > 1 {
            if let Some(b) = self.doc.bounds_of(&self.sel) {
                p.rect_stroke(v.rect(b).expand(8.0), CornerRadius::same(6), Stroke::new(1.0, theme::INDIGO.gamma_multiply(0.6)), StrokeKind::Middle);
            }
        }
        if let Drag::Marquee(start) = self.drag {
            if let Some(h) = p.ctx().input(|i| i.pointer.hover_pos()) {
                let r = Rect::from_two_pos(v.w2s(start), h);
                p.rect(r, CornerRadius::same(2), theme::INDIGO.gamma_multiply(0.08), Stroke::new(1.0, theme::INDIGO), StrokeKind::Middle);
            }
        }
        if let (Some(id), Drag::None, Tool::Select, None) = (self.hover_shape, &self.drag, self.tool, self.pending_arrow) {
            let hot = p.ctx().input(|i| i.pointer.hover_pos()).and_then(|h| self.anchor_at(h)).map(|x| x.1);
            for (a, edge, i) in self.anchors(id) {
                let on = hot == Some(i);
                p.line_segment([edge, a], Stroke::new(1.0, theme::INDIGO.gamma_multiply(0.3)));
                p.circle(a, if on { 6.5 } else { 4.2 }, if on { theme::INDIGO } else { Color32::WHITE }, Stroke::new(1.5, theme::INDIGO));
            }
        }
        // scenario being recorded: numbered badges
        if let Some((_, picks)) = &self.rec {
            // highlight what was picked; never draw new lines
            for (i, id) in picks.iter().enumerate() {
                let Some(e) = self.doc.get(*id) else { continue };
                let badge = if e.kind == Kind::Arrow {
                    let path = self.doc.arrow_path(e);
                    p.add(egui::Shape::line(v.pts(&path), Stroke::new(e.style.width * self.zoom + 6.0, theme::RED.gamma_multiply(0.45))));
                    v.w2s(point_at(&path, path_len(&path) * 0.35))
                } else {
                    let r = v.rect(e.rect);
                    p.rect_stroke(r.expand(4.0), CornerRadius::same(12), Stroke::new(2.0, theme::RED), StrokeKind::Outside);
                    r.left_top()
                };
                p.circle_filled(badge, 11.0, theme::RED);
                p.text(badge, Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(12.0), Color32::WHITE);
            }
        }
        // collaborators' cursors
        if let Some(c) = &self.collab {
            for (id, peer) in &c.peers {
                let s = v.w2s(peer.pos);
                let col = peer_color(*id);
                p.add(egui::Shape::convex_polygon(vec![s, s + vec2(0.0, 16.0), s + vec2(4.5, 12.0), s + vec2(11.0, 11.0)], col, Stroke::new(1.0, Color32::WHITE)));
                let g = p.layout_no_wrap(peer.name.clone(), FontId::proportional(11.5), Color32::WHITE);
                let b = Rect::from_min_size(s + vec2(12.0, 14.0), g.size() + vec2(10.0, 4.0));
                p.rect_filled(b, CornerRadius::same(6), col);
                p.galley(b.min + vec2(5.0, 2.0), g, Color32::WHITE);
            }
        }
    }
}

/// Tool behind a plain-shape catalog id ("shape-rect" …).
pub(crate) fn shape_tool(id: &str) -> Option<Tool> {
    Some(match id {
        "shape-rect" => Tool::Rect,
        "shape-ellipse" => Tool::Ellipse,
        "shape-diamond" => Tool::Diamond,
        "shape-sticky" => Tool::Sticky,
        "shape-text" => Tool::Text,
        _ => return None,
    })
}

/// Catalog id for a toolbar shape tool, so the button can be dragged onto the canvas.
pub(crate) fn tool_shape_id(t: Tool) -> Option<&'static str> {
    Some(match t {
        Tool::Rect => "shape-rect",
        Tool::Ellipse => "shape-ellipse",
        Tool::Diamond => "shape-diamond",
        Tool::Sticky => "shape-sticky",
        Tool::Text => "shape-text",
        _ => return None,
    })
}

pub(crate) fn peer_color(id: u32) -> Color32 {
    const C: [Color32; 6] = [
        Color32::from_rgb(0xe1, 0x1d, 0x48),
        Color32::from_rgb(0x05, 0x96, 0x69),
        Color32::from_rgb(0xd9, 0x77, 0x06),
        Color32::from_rgb(0x25, 0x63, 0xeb),
        Color32::from_rgb(0x93, 0x33, 0xea),
        Color32::from_rgb(0x08, 0x91, 0xb2),
    ];
    C[id as usize % C.len()]
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(s) = serde_json::to_string(&self.doc) {
            storage.set_string("doc", s);
        }
        storage.set_string("dark", if self.dark { "1" } else { "0" }.into());
        storage.set_string("name", self.my_name.clone());
        storage.set_string("autosave", if self.autosave { "1" } else { "0" }.into());
        storage.set_string("boundaries", serde_json::to_string(&self.custom_bounds).unwrap_or_default());
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(c) = &mut self.collab {
            c.leave();
        }
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        theme::set_dark(self.dark);
        let now = ctx.input(|i| i.time);
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
        self.step_anim(now);
        self.autosave_tick(ctx, now);

        if let Some(c) = &mut self.collab {
            let cursor = ctx.input(|i| i.pointer.hover_pos()).filter(|p| self.canvas.contains(*p)).map(|p| View { origin: self.canvas.min + self.offset, zoom: self.zoom }.s2w(p));
            if c.tick(&mut self.doc, now, cursor) {
                self.sel.retain(|id| self.doc.get(*id).is_some());
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        // keep text boxes sized to their content
        let sketch = self.doc.sketch;
        for e in self.doc.elements.iter_mut().filter(|e| e.kind == Kind::Text) {
            e.rect.max = e.rect.min + render::measure(ctx, &e.text, e.style.font, sketch);
        }

        self.code_panel(ctx);

        egui::CentralPanel::default().frame(Frame::new().fill(theme::canvas_bg())).show(ctx, |ui| {
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
            } else if self.rec.is_some() || self.failures {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            } else if !matches!(self.tool, Tool::Select) {
                ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            } else if matches!(self.drag, Drag::Label(_)) || resp.hover_pos().is_some_and(|h| self.label_hit(ctx, self.view().s2w(h)).is_some()) {
                ctx.set_cursor_icon(if matches!(self.drag, Drag::Label(_)) { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
            } else {
                let want = self.select_cursor(resp.hover_pos());
                // macOS ignores cursor changes while the window is inactive, and egui won't
                // re-send an unchanged cursor; nudge it for one frame after focus returns.
                let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
                if focused && !self.was_focused && want != egui::CursorIcon::Default {
                    ctx.set_cursor_icon(egui::CursorIcon::Default);
                    ctx.request_repaint();
                } else {
                    ctx.set_cursor_icon(want);
                }
                self.was_focused = focused;
            }

            let p = ui.painter_at(rect);
            let v = self.view();
            render::draw_grid(&p, rect, &v);
            let phase = if self.playing { (now as f32 % LOOP_SECS) / LOOP_SECS } else { 0.0 };
            let hover_comment = resp
                .hover_pos()
                .and_then(|h| self.doc.hit(v.s2w(h), 4.0 / self.zoom))
                .filter(|id| self.doc.get(*id).is_some_and(|e| e.kind == Kind::Comment || (e.kind == Kind::Node && !e.notes.is_empty())));
            let mut open: Vec<u64> = self.sel.iter().copied().chain(hover_comment).chain(self.editing).collect();
            open.dedup();
            let opts = Opts { phase, dark: self.dark, sketch: self.doc.sketch, failures: self.failures, hide_text: self.editing, open_comments: &open, skip_arrows: self.scn_play.is_some() };
            render::draw_doc(&p, ctx, &self.doc, &v, &mut self.icons, &opts);
            if let Some((i, start)) = self.scn_play {
                if let Some(scn) = self.doc.scenarios.get(i).cloned() {
                    render::draw_scenario(&p, ctx, &self.doc, &v, &mut self.icons, &opts, &scn, (now - start) as f32, rect);
                    ctx.request_repaint();
                } else {
                    self.scn_play = None;
                }
            } else {
                self.paint_overlays(&p);
            }

            if self.doc.elements.is_empty() {
                let c = rect.center();
                p.text(c - vec2(0.0, 14.0), Align2::CENTER_CENTER, "Start your diagram", FontId::proportional(22.0), theme::muted());
                p.text(
                    c + vec2(0.0, 16.0),
                    Align2::CENTER_CENTER,
                    "Drag from the library  ·  /  insert component  ·  ⌘K commands  ·  ⌘/ diagram as code  ·  ⌘I import",
                    FontId::proportional(13.0),
                    theme::muted().gamma_multiply(0.8),
                );
            }
        });

        self.toolbar(ctx);
        self.top_left(ctx);
        self.top_right(ctx);
        self.library_panel(ctx);
        self.drag_ghost(ctx);
        if self.scn_play.is_none() && self.rec.is_none() && !self.failures {
            self.props_panel(ctx);
        }
        self.bottom_bars(ctx);
        self.scenario_panel(ctx);
        self.recording_banner(ctx);
        self.collab_window(ctx);
        self.picker_popup(ctx);
        self.palette_popup(ctx);
        self.text_editor(ctx);
        self.toast_view(ctx);

        let animating = self.anim.is_some() || (self.playing && self.doc.elements.iter().any(|e| e.kind == Kind::Arrow && (e.style.flow != Flow::None || (self.failures && e.failover))));
        self.mode_bars(ctx);
        self.boundary_editor(ctx);
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

pub(crate) fn setup_style(ctx: &Context, dark: bool) {
    theme::set_dark(dark);
    let mut s = (*ctx.style()).clone();
    s.visuals = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    // hand cursor over every clickable button / menu item
    s.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    s.visuals.selection.bg_fill = theme::INDIGO;
    s.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    s.visuals.hyperlink_color = theme::INDIGO;
    s.visuals.window_corner_radius = CornerRadius::same(12);
    s.visuals.menu_corner_radius = CornerRadius::same(10);
    s.visuals.panel_fill = theme::panel_bg();
    s.visuals.window_fill = theme::panel_bg();
    s.visuals.window_stroke = Stroke::new(1.0, theme::panel_border());
    for w in [&mut s.visuals.widgets.inactive, &mut s.visuals.widgets.hovered, &mut s.visuals.widgets.active, &mut s.visuals.widgets.noninteractive, &mut s.visuals.widgets.open] {
        w.corner_radius = CornerRadius::same(7);
    }
    s.visuals.widgets.inactive.weak_bg_fill = theme::hover();
    s.visuals.widgets.inactive.bg_fill = theme::hover();
    s.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    s.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    s.visuals.extreme_bg_color = theme::hover();
    s.visuals.window_shadow = egui::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(if dark { 90 } else { 28 }) };
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
    let t = if dark { egui::Theme::Dark } else { egui::Theme::Light };
    ctx.set_theme(t);
    ctx.set_style_of(t, s);
}
