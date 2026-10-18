//! Floating UI chrome: toolbar, menus, library, properties, scenarios, palette, picker, code pane.

use super::*;
use eframe::egui::{Align, Layout, Margin, Order, RichText, Shadow, TextEdit, Ui};

const PANEL_W: f32 = 228.0;

pub(super) fn panel() -> Frame {
    Frame::new()
        .fill(theme::panel_bg())
        .corner_radius(12)
        .stroke(Stroke::new(1.0, theme::panel_border()))
        .shadow(Shadow { offset: [0, 4], blur: 18, spread: 0, color: Color32::from_black_alpha(if theme::dark() { 70 } else { 16 }) })
        .inner_margin(Margin::same(5))
}

pub(super) fn icon_button(ui: &mut Ui, icons: &mut Icons, icon: &str, active: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
    let bg = if active {
        theme::active_bg()
    } else if resp.hovered() {
        theme::hover()
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
    if let Some(t) = icons.get(ui.ctx(), icon) {
        ui.painter().image(t, Rect::from_center_size(rect.center(), vec2(18.0, 18.0)), render::UV, if active { theme::active_fg() } else { theme::text() });
    }
    if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
}

fn lib_tile(ui: &mut Ui, icons: &mut Icons, it: &icons::Icon) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(62.0, 66.0), Sense::click_and_drag());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), theme::hover());
    }
    let accent = if it.cat == "Boundaries" {
        icons::BOUNDARIES.iter().find(|b| b.0 == it.id).map(|b| icons::boundary_color(b.3)).unwrap_or(theme::INDIGO)
    } else {
        icons::accent(it.cat)
    };
    let tile = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 22.0), vec2(34.0, 34.0));
    if it.cat == "Boundaries" {
        ui.painter().rect(tile, CornerRadius::same(9), accent.gamma_multiply(0.08), Stroke::new(1.2, accent), StrokeKind::Inside);
    } else if !it.color {
        ui.painter().rect_filled(tile, CornerRadius::same(9), theme::map(tint(accent, 0.12), theme::dark()));
    }
    if let Some(t) = icons.get(ui.ctx(), it.id) {
        ui.painter().image(t, if it.color { tile.shrink(2.0) } else { tile.shrink(8.0) }, render::UV, if it.color { Color32::WHITE } else { accent });
    }
    let label = if it.label.chars().count() > 11 { format!("{}…", it.label.chars().take(10).collect::<String>()) } else { it.label.to_string() };
    ui.painter().text(pos2(rect.center().x, rect.max.y - 9.0), Align2::CENTER_CENTER, label, FontId::proportional(10.5), theme::text());
    let tip = if it.sub.is_empty() { it.label.to_string() } else { format!("{}\n{}", it.label, it.sub) };
    resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::Grab)
}

fn custom_tile(ui: &mut Ui, icons: &mut Icons, b: &CustomBoundary) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(62.0, 66.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), theme::hover());
    }
    let tile = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 22.0), vec2(34.0, 34.0));
    ui.painter().rect(tile, CornerRadius::same(9), b.color.gamma_multiply(0.08), Stroke::new(1.2, b.color), StrokeKind::Inside);
    if let Some(t) = icons.get(ui.ctx(), &b.icon) {
        ui.painter().image(t, tile.shrink(8.0), render::UV, b.color);
    }
    let label = if b.name.chars().count() > 11 { format!("{}…", b.name.chars().take(10).collect::<String>()) } else { b.name.clone() };
    ui.painter().text(pos2(rect.center().x, rect.max.y - 9.0), Align2::CENTER_CENTER, label, FontId::proportional(10.5), theme::text());
    resp.on_hover_text(format!("{}\nYour boundary · right-click to edit", b.name))
}

fn new_tile(ui: &mut Ui, icons: &mut Icons) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(62.0, 66.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), theme::hover());
    }
    let tile = Rect::from_center_size(pos2(rect.center().x, rect.min.y + 22.0), vec2(34.0, 34.0));
    let mut pts = rounded_rect(tile, 9.0);
    pts.push(pts[0]);
    ui.painter().extend(egui::Shape::dashed_line(&pts, Stroke::new(1.2, theme::muted()), 4.0, 3.0));
    if let Some(t) = icons.get(ui.ctx(), "plus") {
        ui.painter().image(t, tile.shrink(9.0), render::UV, theme::muted());
    }
    ui.painter().text(pos2(rect.center().x, rect.max.y - 9.0), Align2::CENTER_CENTER, "New…", FontId::proportional(10.5), theme::text());
    resp.on_hover_text("Create your own boundary style")
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).size(11.0).color(theme::muted()).strong());
}

fn swatch(ui: &mut Ui, c: Color32, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_stroke(rect.expand(2.0), CornerRadius::same(7), Stroke::new(2.0, theme::INDIGO), StrokeKind::Outside);
    }
    if c.a() == 0 {
        p.rect(rect, CornerRadius::same(5), theme::panel_bg(), Stroke::new(1.0, theme::panel_border()), StrokeKind::Inside);
        p.line_segment([rect.left_bottom() + vec2(4.0, -4.0), rect.right_top() + vec2(-4.0, 4.0)], Stroke::new(1.5, theme::RED));
    } else {
        p.rect(rect, CornerRadius::same(5), c, Stroke::new(1.0, Color32::from_black_alpha(20)), StrokeKind::Inside);
    }
    resp
}

fn seg<T: PartialEq + Copy>(ui: &mut Ui, cur: T, opts: &[(T, &str)]) -> Option<T> {
    let mut out = None;
    Frame::new().fill(theme::hover()).corner_radius(8).inner_margin(Margin::same(2)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let w = (PANEL_W - 8.0 - 2.0 * (opts.len() as f32 - 1.0)) / opts.len() as f32;
            for (v, label) in opts {
                let on = *v == cur;
                let b = egui::Button::new(RichText::new(*label).size(12.0).color(if on { theme::active_fg() } else { theme::text() }))
                    .fill(if on { theme::panel_bg() } else { Color32::TRANSPARENT })
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

/// 3×3 label-position picker drawn as a tiny shape with hot spots.
fn pos_grid(ui: &mut Ui, cur: TextPos, outside: bool) -> Option<TextPos> {
    let (area, _) = ui.allocate_exact_size(vec2(PANEL_W, 92.0), Sense::hover());
    let p = ui.painter();
    let shape = Rect::from_center_size(area.center(), vec2(104.0, 56.0));
    p.rect(shape, CornerRadius::same(10), theme::hover(), Stroke::new(1.5, theme::panel_border()), StrokeKind::Inside);
    let mut out = None;
    for tp in TextPos::ALL {
        let (fx, fy) = (tp.fx(), tp.fy());
        let inner = shape.shrink(13.0);
        let mut c = pos2(inner.min.x + inner.width() * fx, inner.min.y + inner.height() * fy);
        if outside && !(fx == 0.5 && fy == 0.5) {
            if fx != 0.5 {
                c.x = if fx == 0.0 { shape.min.x - 16.0 } else { shape.max.x + 16.0 };
            }
            if fy != 0.5 {
                c.y = if fy == 0.0 { shape.min.y - 12.0 } else { shape.max.y + 12.0 };
            }
        }
        let id = ui.id().with(("tpos", fx.to_bits(), fy.to_bits()));
        let r = ui.interact(Rect::from_center_size(c, vec2(18.0, 16.0)), id, Sense::click());
        let on = tp == cur;
        let grow = ui.ctx().animate_bool_with_time(id, r.hovered() || on, 0.12);
        let p = ui.painter();
        if on {
            // little "text lines" glyph showing where the label lands
            let bars = Rect::from_center_size(c, vec2(16.0, 10.0));
            p.rect_filled(bars.expand(3.0), CornerRadius::same(4), theme::INDIGO);
            for k in 0..2 {
                let y = bars.min.y + 2.0 + k as f32 * 5.0;
                let w = if k == 0 { 12.0 } else { 8.0 };
                let x0 = bars.center().x - w / 2.0 + (fx - 0.5) * (16.0 - w);
                p.line_segment([pos2(x0, y), pos2(x0 + w, y)], Stroke::new(2.0, Color32::WHITE));
            }
        } else {
            p.circle_filled(c, 2.5 + grow * 2.5, theme::muted().gamma_multiply(0.5 + grow * 0.5));
        }
        if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !on {
            out = Some(tp);
        }
    }
    out
}

fn matches(q: &str, text: &str) -> bool {
    let t = text.to_lowercase();
    q.split_whitespace().all(|w| t.contains(&w.to_lowercase()))
}

fn suggest(cat: &str) -> &'static [&'static str] {
    match cat {
        "Clients" => &["cdn", "lb", "gateway", "dns", "lock"],
        "Network" => &["service", "server", "function", "container", "k8s"],
        "Compute" => &["database", "cache", "queue", "bucket", "stream"],
        "Data" => &["service", "warehouse", "search", "pipeline"],
        "Messaging" => &["worker", "function", "service", "email"],
        "AWS" => &["aws-lambda", "aws-dynamodb", "aws-sqs", "aws-s3", "aws-rds", "aws-elasticache"],
        "Google Cloud" => &["gcp-run", "gcp-sql", "gcp-pubsub", "gcp-gcs", "gcp-firestore", "gcp-memorystore"],
        "Azure" => &["az-functions", "az-cosmos", "az-servicebus", "az-blob", "az-sql", "az-redis"],
        "Cloudflare" => &["cf-workers", "cf-kv", "cf-r2", "cf-d1", "cf-queues"],
        _ => &["service", "database", "cache", "queue", "bucket"],
    }
}

impl App {
    fn right_x(&self) -> f32 {
        -12.0 - self.code_w
    }

    pub(super) fn toolbar(&mut self, ctx: &Context) {
        egui::Area::new(Id::new("toolbar")).anchor(Align2::CENTER_TOP, vec2(-self.code_w / 2.0, 10.0)).show(ctx, |ui| {
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
                    if icon_button(ui, &mut self.icons, "t-code", self.code_open, "Diagram as code  ⌘/").clicked() {
                        self.code_open = !self.code_open;
                    }
                });
            });
        });
    }

    pub(super) fn top_left(&mut self, ctx: &Context) {
        let mut save_now = false;
        egui::Area::new(Id::new("menu")).anchor(Align2::LEFT_TOP, vec2(84.0, 10.0)).show(ctx, |ui| {
            panel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if icon_button(ui, &mut self.icons, "menu", self.menu, "Menu").clicked() {
                        self.menu = !self.menu;
                        self.export_menu = false;
                    }
                    let name = self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or("Untitled".into());
                    ui.label(RichText::new(&name).color(theme::text()).strong());
                    // save status
                    if self.path.is_none() {
                        if ui.add(egui::Button::new(RichText::new("Save…").size(12.0))).on_hover_text("Choose a file; auto-save keeps it up to date").clicked() {
                            save_now = true;
                        }
                    } else if self.autosave {
                        ui.label(RichText::new("✓ Auto-saved").size(11.5).color(theme::muted())).on_hover_text("Changes are written to the file automatically. Toggle in ☰ menu.");
                    } else if self.dirty {
                        ui.label(RichText::new("• Unsaved").size(11.5).color(theme::RED)).on_hover_text("⌘S to save, or turn on Auto-save in the ☰ menu");
                    }
                    if let Some(c) = &self.collab {
                        let n = c.peers.len() + 1;
                        ui.label(RichText::new(format!("● Live · {n}")).color(Color32::from_rgb(0x10, 0xb9, 0x81)).strong()).on_hover_text(&c.status);
                    }
                    ui.add_space(6.0);
                });
            });
        });
        if save_now {
            self.save(ctx, true);
        }
        if !self.menu {
            return;
        }
        let mut act = None;
        egui::Area::new(Id::new("menu-pop")).anchor(Align2::LEFT_TOP, vec2(84.0, 62.0)).order(Order::Foreground).show(ctx, |ui| {
            panel().show(ui, |ui| {
                ui.set_width(250.0);
                let groups: &[&[(&str, &str, &str)]] = &[
                    &[("New", "", "new"), ("Open…", "⌘O", "open"), ("Save", "⌘S", "save"), ("Save as…", "⇧⌘S", "saveas"), (if self.autosave { "Auto-save: On ✓" } else { "Auto-save: Off" }, "", "autosave")],
                    &[("Import compose / k8s / Terraform / Mermaid…", "⌘I", "import"), ("Open icon packs folder", "", "icons-folder")],
                    &[("Command palette", "⌘K", "palette"), ("Diagram as code", "⌘/", "code"), ("Auto-layout", "⇧⌘L", "layout"), ("Request flows", "", "scenarios"), ("Simulate outage", "F", "failures"), ("Collaborate live…", "", "collab")],
                    &[(if self.dark { "Light mode" } else { "Dark mode" }, "⇧⌘D", "dark"), (if self.doc.sketch { "Clean style" } else { "Sketch style" }, "⇧⌘K", "sketch"), ("Zoom to fit", "⇧1", "fit")],
                ];
                for (gi, g) in groups.iter().enumerate() {
                    if gi > 0 {
                        ui.separator();
                    }
                    for (label, key, a) in g.iter() {
                        if ui.add(egui::Button::new(*label).shortcut_text(*key).frame(false).min_size(vec2(240.0, 28.0))).clicked() {
                            act = Some(*a);
                        }
                    }
                }
            });
        });
        if let Some(a) = act {
            self.menu = false;
            self.run(ctx, a);
        }
    }

    pub(super) fn top_right(&mut self, ctx: &Context) {
        if !self.code_open {
            egui::Area::new(Id::new("export")).anchor(Align2::RIGHT_TOP, vec2(self.right_x(), 10.0)).show(ctx, |ui| {
                panel().show(ui, |ui| {
                    let b = egui::Button::new(RichText::new("  Export  ").color(Color32::WHITE).strong()).fill(theme::INDIGO).corner_radius(8).min_size(vec2(0.0, 34.0));
                    if ui.add(b).clicked() {
                        self.export_menu = !self.export_menu;
                        self.menu = false;
                    }
                });
            });
        }
        if !self.export_menu {
            return;
        }
        let mut act: Option<(&'static str, Option<usize>)> = None;
        egui::Area::new(Id::new("export-pop")).anchor(Align2::RIGHT_TOP, vec2(self.right_x(), 62.0)).order(Order::Foreground).show(ctx, |ui| {
            panel().inner_margin(Margin::same(10)).show(ui, |ui| {
                ui.set_width(PANEL_W + 20.0);
                ui.label(RichText::new("Quality").size(11.0).color(theme::muted()).strong());
                if let Some(q) = seg(ui, self.quality, &[(Quality::Standard, "1×"), (Quality::Retina, "2× Retina"), (Quality::UltraHd, "4K")]) {
                    self.quality = q;
                }
                let scope = if self.sel.is_empty() { "whole canvas" } else { "selection only" };
                ui.label(RichText::new(format!("Exports {scope} · {} theme{}", if self.dark { "dark" } else { "light" }, if self.failures { " · outage view" } else { "" })).small().color(theme::muted()));
                ui.add_space(4.0);
                for (fmt, label, sub) in [("png", "PNG image", "best for docs"), ("svg", "SVG vector", "infinitely sharp"), ("gif", "Animated GIF", "loops flow animation")] {
                    if ui.add(egui::Button::new(RichText::new(label).strong()).shortcut_text(sub).frame(false).min_size(vec2(PANEL_W + 10.0, 30.0))).clicked() {
                        act = Some((fmt, None));
                    }
                }
                if !self.doc.scenarios.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("Flow GIFs").size(11.0).color(theme::muted()).strong());
                    for (i, s) in self.doc.scenarios.iter().enumerate() {
                        if ui.add(egui::Button::new(format!("▶  {}", s.name)).shortcut_text(format!("{} steps", s.steps.len())).frame(false).min_size(vec2(PANEL_W + 10.0, 28.0))).clicked() {
                            act = Some(("gif", Some(i)));
                        }
                    }
                }
                if self.quality == Quality::UltraHd {
                    ui.label(RichText::new("4K GIFs are large and take a few seconds").small().color(theme::muted()));
                }
            });
        });
        if let Some((fmt, s)) = act {
            self.export(ctx, fmt, s);
        }
    }

    pub(super) fn library_panel(&mut self, ctx: &Context) {
        if !self.library {
            return;
        }
        let h = (self.canvas.height() - 300.0).max(160.0);
        let mut pick: Option<&'static str> = None;
        egui::Area::new(Id::new("library")).anchor(Align2::RIGHT_TOP, vec2(self.right_x(), if self.code_open { 10.0 } else { 62.0 })).show(ctx, |ui| {
            panel().inner_margin(Margin::same(10)).show(ui, |ui| {
                ui.set_width(284.0);
                ui.set_max_height(h + 160.0); // before any widget: it resets the cursor
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if self.replace_icon { "Pick a new icon" } else { "Components" }).strong().size(14.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, &mut self.icons, "x", false, "Close  L").clicked() {
                            self.library = false;
                            self.replace_icon = false;
                        }
                    });
                });
                ui.add(TextEdit::singleline(&mut self.lib_query).hint_text("Search 250+  e.g. lambda, bigquery, kafka").desired_width(f32::INFINITY).margin(Margin::symmetric(8, 6)));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                    for (f, label) in [(None, "All"), (Some("Boundaries"), "Boundaries"), (Some("AWS"), "AWS"), (Some("Google Cloud"), "GCP"), (Some("Azure"), "Azure"), (Some("Open source"), "OSS")] {
                        if ui.selectable_label(self.lib_filter == f, RichText::new(label).size(11.5)).clicked() {
                            self.lib_filter = f;
                        }
                    }
                });
                ui.add_space(2.0);
                let q = self.lib_query.clone();
                let (mut custom_pick, mut custom_edit, mut custom_del, mut custom_new) = (None, None, None, false);
                egui::ScrollArea::vertical().max_height(h).show(ui, |ui| {
                    for cat in icons::categories() {
                        if self.lib_filter.is_some_and(|f| f != cat) || (self.replace_icon && cat == "Boundaries") {
                            continue;
                        }
                        let items: Vec<&icons::Icon> = icons::all().iter().filter(|i| i.cat == cat && (q.is_empty() || matches(&q, &format!("{} {} {} {}", i.id, i.label, i.sub, i.cat)))).collect();
                        if items.is_empty() {
                            continue;
                        }
                        ui.add_space(4.0);
                        ui.label(RichText::new(cat.to_uppercase()).size(10.5).color(theme::muted()).strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                            for it in items {
                                let r = lib_tile(ui, &mut self.icons, it);
                                if r.clicked() {
                                    pick = Some(it.id);
                                }
                                if r.drag_started() && !self.replace_icon {
                                    self.lib_drag = Some(it.id);
                                }
                            }
                            if cat == "Boundaries" && !self.replace_icon {
                                for (i, b) in self.custom_bounds.clone().iter().enumerate() {
                                    if !q.is_empty() && !matches(&q, &b.name) {
                                        continue;
                                    }
                                    let r = custom_tile(ui, &mut self.icons, b);
                                    if r.clicked() {
                                        custom_pick = Some(i);
                                    }
                                    r.context_menu(|ui| {
                                        if ui.button("Edit…").clicked() {
                                            custom_edit = Some(i);
                                            ui.close_menu();
                                        }
                                        if ui.button("Delete").clicked() {
                                            custom_del = Some(i);
                                            ui.close_menu();
                                        }
                                    });
                                }
                                if q.is_empty() && new_tile(ui, &mut self.icons).clicked() {
                                    custom_new = true;
                                }
                            }
                        });
                    }
                });
                if let Some(i) = custom_pick {
                    pick = Some("::custom");
                    self.lib_custom = i;
                }
                if let Some(i) = custom_edit {
                    let b = self.custom_bounds[i].clone();
                    self.bound_editor = Some(BoundEditor { idx: Some(i), b, query: String::new() });
                }
                if let Some(i) = custom_del {
                    self.custom_bounds.remove(i);
                }
                if custom_new {
                    self.bound_editor = Some(BoundEditor { idx: None, b: CustomBoundary { name: "My boundary".into(), icon: "folder".into(), color: theme::INDIGO, dashed: true }, query: String::new() });
                }
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Click or drag to add ·").small().color(theme::muted()));
                    if ui.link(RichText::new("add official icon packs").small()).clicked() {
                        pick = Some("::icons-folder");
                    }
                });
            });
        });
        match pick {
            Some("::icons-folder") => self.run(ctx, "icons-folder"),
            Some("::custom") => self.insert_custom_boundary(self.lib_custom),
            Some(id) if self.replace_icon => self.set_icon(id),
            Some(id) => {
                self.insert_icon(id, None);
            }
            None => {}
        }
        if let (Some(icon), Some(p)) = (self.lib_drag, ctx.input(|i| i.pointer.hover_pos())) {
            let painter = ctx.layer_painter(egui::LayerId::new(Order::Tooltip, Id::new("ghost")));
            let r = Rect::from_center_size(p, vec2(176.0, 56.0) * self.zoom);
            painter.rect(r, CornerRadius::same(12), theme::panel_bg().gamma_multiply(0.9), Stroke::new(1.5, theme::INDIGO), StrokeKind::Inside);
            if let (Some(t), Some(it)) = (self.icons.get(ctx, icon), icons::find(icon)) {
                painter.image(t, Rect::from_center_size(r.center(), vec2(24.0, 24.0)), render::UV, if it.color { Color32::WHITE } else { icons::accent(it.cat) });
            }
        }
    }

    pub(super) fn props_panel(&mut self, ctx: &Context) {
        let els: Vec<Element> = self.doc.elements.iter().filter(|e| self.sel.contains(&e.id)).cloned().collect();
        let Some(first) = els.first().cloned() else { return };
        let s = first.style;
        let has = |ks: &[Kind]| els.iter().any(|e| ks.contains(&e.kind));
        type Edit = Box<dyn Fn(&mut Element)>;
        let mut change: Option<Edit> = None;
        let mut action = "";
        let max_h = (self.canvas.height() - 150.0).max(200.0);
        egui::Area::new(Id::new("props")).anchor(Align2::LEFT_TOP, vec2(12.0, 62.0)).show(ctx, |ui| {
            panel().inner_margin(Margin::same(12)).show(ui, |ui| {
                ui.set_width(PANEL_W);
                // Areas cap content at last frame's size; let the panel grow when the selection changes.
                ui.set_max_height(max_h);
                egui::ScrollArea::vertical().max_height(max_h).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    if !has(&[Kind::Comment]) {
                        section(ui, if has(&[Kind::Node]) { "Accent" } else { "Stroke" });
                        ui.horizontal_wrapped(|ui| {
                            for c in super::STROKES {
                                if swatch(ui, c, s.stroke == c).clicked() {
                                    change = Some(Box::new(move |e| e.style.stroke = c));
                                }
                            }
                            let mut c = s.stroke;
                            if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque).changed() {
                                change = Some(Box::new(move |e| e.style.stroke = c));
                            }
                        });
                    }
                    if has(&[Kind::Rect, Kind::Ellipse, Kind::Diamond, Kind::Region, Kind::Node, Kind::Sticky]) {
                        section(ui, "Background");
                        ui.horizontal_wrapped(|ui| {
                            for c in super::FILLS {
                                if swatch(ui, c, s.fill == c).clicked() {
                                    change = Some(Box::new(move |e| e.style.fill = if e.kind == Kind::Region && c.a() > 0 { c.gamma_multiply(0.35) } else { c }));
                                }
                            }
                        });
                    }
                    if els.iter().any(|e| e.has_label_pos()) {
                        section(ui, "Label position");
                        if let Some(tp) = pos_grid(ui, first.tpos, first.outside) {
                            change = Some(Box::new(move |e| e.tpos = tp));
                        }
                        if let Some(o) = seg(ui, first.outside, &[(false, "Inside"), (true, "Outside")]) {
                            change = Some(Box::new(move |e| e.outside = o));
                        }
                    }
                    if has(&[Kind::Rect, Kind::Ellipse, Kind::Diamond, Kind::Pen, Kind::Arrow, Kind::Region]) {
                        section(ui, "Stroke width");
                        if let Some(w) = seg(ui, s.width, &[(1.0, "Thin"), (2.0, "Regular"), (4.0, "Bold")]) {
                            change = Some(Box::new(move |e| e.style.width = w));
                        }
                    }
                    if !has(&[Kind::Text, Kind::Pen, Kind::Comment, Kind::Sticky]) {
                        section(ui, "Line style");
                        if let Some(d) = seg(ui, s.dashed, &[(false, "Solid"), (true, "Dashed")]) {
                            change = Some(Box::new(move |e| e.style.dashed = d));
                        }
                    }
                    if has(&[Kind::Rect]) {
                        section(ui, "Corners");
                        if let Some(r) = seg(ui, s.round, &[(false, "Sharp"), (true, "Round")]) {
                            change = Some(Box::new(move |e| e.style.round = r));
                        }
                    }
                    if has(&[Kind::Rect, Kind::Ellipse, Kind::Diamond, Kind::Text, Kind::Arrow, Kind::Region, Kind::Sticky]) {
                        section(ui, "Font size");
                        if let Some(f) = seg(ui, s.font, &[(13.0, "S"), (18.0, "M"), (24.0, "L"), (36.0, "XL")]) {
                            change = Some(Box::new(move |e| e.style.font = f));
                        }
                    }
                    if has(&[Kind::Arrow]) {
                        section(ui, "Route");
                        if let Some(r) = seg(ui, s.route, &[(Route::Elbow, "Elbow"), (Route::Straight, "Straight"), (Route::Curve, "Curve")]) {
                            change = Some(Box::new(move |e| e.style.route = r));
                        }
                        if first.bend.is_some() {
                            if ui.button("Reset route (automatic)").clicked() {
                                change = Some(Box::new(|e| e.bend = None));
                            }
                        } else {
                            ui.label(RichText::new("Drag the line to reshape it · drag an end to reconnect").small().color(theme::muted()));
                        }
                        if !first.text.is_empty() {
                            section(ui, "Label");
                            let cur = if first.label_off != 0.0 { -1.0 } else { first.label_t };
                            if let Some(t) = seg(ui, cur, &[(0.15, "Start"), (0.5, "Middle"), (0.85, "End")]) {
                                change = Some(Box::new(move |e| (e.label_t, e.label_off) = (t, 0.0)));
                            }
                            ui.label(RichText::new("or drag the label anywhere along the line").small().color(theme::muted()));
                        }
                        section(ui, "Arrowheads");
                        if let Some((a, b)) = seg(ui, (s.head_start, s.head_end), &[((false, false), "None"), ((false, true), "End"), ((true, true), "Both")]) {
                            change = Some(Box::new(move |e| (e.style.head_start, e.style.head_end) = (a, b)));
                        }
                        section(ui, "Flow animation");
                        if let Some(f) = seg(ui, s.flow, &[(Flow::None, "Off"), (Flow::Dots, "Packets"), (Flow::Dashes, "Dashes")]) {
                            change = Some(Box::new(move |e| e.style.flow = f));
                        }
                        if s.flow != Flow::None || first.failover {
                            section(ui, "Speed");
                            if let Some(sp) = seg(ui, first.speed, &[(Speed::Slow, "Slow"), (Speed::Normal, "Normal"), (Speed::Fast, "Fast")]) {
                                change = Some(Box::new(move |e| e.speed = sp));
                            }
                        }
                        let mut fo = first.failover;
                        if ui.checkbox(&mut fo, "Backup path").changed() {
                            change = Some(Box::new(move |e| e.failover = fo));
                        }
                        ui.label(RichText::new("Stays faint normally. Takes over when you simulate an outage (F).").small().color(theme::muted()));
                        let hint = match traffic(&first.text) {
                            Some(r) => format!("Traffic {} rps → thicker, faster", short_num(r)),
                            None => "Tip: label it \"1.2k rps\" to make it thicker and faster".into(),
                        };
                        ui.label(RichText::new(hint).small().color(theme::muted()));
                    }
                    if has(&[Kind::Node, Kind::Rect, Kind::Ellipse, Kind::Diamond]) && self.sel.len() == 1 {
                        section(ui, "Request flow");
                        if ui.button("▶  Start a flow from here").on_hover_text("Then click each next component the request visits").clicked() {
                            action = "flow-here";
                        }
                    }
                    if has(&[Kind::Node, Kind::Rect, Kind::Ellipse, Kind::Diamond]) {
                        section(ui, "What if it fails?");
                        if ui.button("⚠  Simulate outage…").on_hover_text("Then click components to take them down (F)").clicked() {
                            action = "outage";
                        }
                        if first.down {
                            ui.label(RichText::new("Marked down in the outage view").small().color(theme::RED));
                        }
                    }
                    if has(&[Kind::Node]) && self.sel.len() == 1 {
                        section(ui, "Notes");
                        let mut notes = first.notes.clone();
                        let r = ui.add(TextEdit::multiline(&mut notes).hint_text("Explain what this does, owners, SLAs…").desired_rows(3).desired_width(f32::INFINITY));
                        if r.gained_focus() {
                            self.checkpoint();
                        }
                        if r.changed() {
                            let id = first.id;
                            if let Some(e) = self.doc.get_mut(id) {
                                e.notes = notes;
                            }
                        }
                        if let Some(show) = seg(ui, first.show_notes, &[(false, "On hover"), (true, "Always visible")]) {
                            change = Some(Box::new(move |e| e.show_notes = show));
                        }
                    }
                    if has(&[Kind::Node]) {
                        section(ui, "Replicas");
                        ui.horizontal(|ui| {
                            let n = first.replicas.max(1);
                            if ui.button(" − ").clicked() && n > 1 {
                                change = Some(Box::new(move |e| e.replicas = n - 1));
                            }
                            ui.label(RichText::new(format!("×{n}")).strong());
                            if ui.button(" + ").clicked() && n < 99 {
                                change = Some(Box::new(move |e| e.replicas = n + 1));
                            }
                        });
                        section(ui, "Capacity math");
                        let mut on = first.cap.is_some();
                        if ui.checkbox(&mut on, "Estimate load & storage").changed() {
                            change = Some(Box::new(move |e| e.cap = if on { Some(Capacity::default()) } else { None }));
                        }
                        if let Some(mut c) = first.cap {
                            let before = c;
                            egui::Grid::new("cap").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                                ui.label("Peak QPS");
                                ui.add(egui::DragValue::new(&mut c.qps).speed(10.0).range(0.0..=1e9));
                                ui.end_row();
                                ui.label("Payload KB");
                                ui.add(egui::DragValue::new(&mut c.kb).speed(0.1).range(0.0..=1e6));
                                ui.end_row();
                                ui.label("Writes %");
                                ui.add(egui::DragValue::new(&mut c.write_pct).speed(1.0).range(0.0..=100.0));
                                ui.end_row();
                                ui.label("Retention days");
                                ui.add(egui::DragValue::new(&mut c.days).speed(1.0).range(1.0..=36500.0));
                                ui.end_row();
                                ui.label("Replication");
                                ui.add(egui::DragValue::new(&mut c.rf).speed(0.1).range(1.0..=10.0));
                                ui.end_row();
                            });
                            let o = c.compute();
                            ui.label(RichText::new(format!("Bandwidth  {:.1} MB/s\nNew data   {}/day\nStored     {}\nStorage ≈  ${:.0}/mo", o.mb_s, bytes_gb(o.gb_day), bytes_gb(o.tb_total * 1024.0), o.usd_month)).monospace().size(11.5));
                            ui.label(RichText::new(format!("at ${USD_PER_GB_MONTH}/GB-month object storage")).small().color(theme::muted()));
                            let mut show = first.show_cap;
                            if ui.checkbox(&mut show, "Show badge on diagram").changed() {
                                change = Some(Box::new(move |e| e.show_cap = show));
                            }
                            if c != before {
                                change = Some(Box::new(move |e| e.cap = Some(c)));
                            }
                        }
                        section(ui, "Icon");
                        if ui.button("Change icon…").clicked() {
                            action = "icon";
                        }
                    }
                    if has(&[Kind::Region]) && self.sel.len() == 1 {
                        section(ui, "Header icon");
                        ui.horizontal(|ui| {
                            if ui.button("Change icon…").clicked() {
                                action = "icon";
                            }
                            if !first.icon.is_empty() && ui.button("Remove").clicked() {
                                change = Some(Box::new(|e| e.icon.clear()));
                            }
                        });
                        if ui.button("Save as my boundary…").on_hover_text("Reuse this style from the library").clicked() {
                            action = "save-boundary";
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
        });
        if let Some(f) = change {
            self.checkpoint();
            for id in self.sel.clone() {
                if let Some(e) = self.doc.get_mut(id) {
                    f(e);
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
            "layout" => self.auto_layout(false),
            "outage" => self.run(ctx, "failures"),
            "flow-here" => {
                self.start_record();
                if let Some((_, nodes)) = &mut self.rec {
                    nodes.push(first.id);
                }
                self.scn_panel = true;
            }
            "save-boundary" => {
                let b = CustomBoundary { name: node_text(&first.text).0.to_string(), icon: first.icon.clone(), color: first.style.stroke, dashed: first.style.dashed };
                self.bound_editor = Some(BoundEditor { idx: None, b, query: String::new() });
            }
            _ => {}
        }
    }

    pub(super) fn bottom_bars(&mut self, ctx: &Context) {
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
        let flows = self.doc.elements.iter().any(|e| e.kind == Kind::Arrow && e.style.flow != Flow::None);
        let mut act = None;
        egui::Area::new(Id::new("play")).anchor(Align2::RIGHT_BOTTOM, vec2(self.right_x(), -12.0)).show(ctx, |ui| {
            panel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if icon_button(ui, &mut self.icons, "alert", self.failures, "Simulate an outage  F").clicked() {
                        act = Some("failures");
                    }
                    if icon_button(ui, &mut self.icons, "film", self.scn_panel, "Request flows: record how a request travels, then play it").clicked() {
                        act = Some("scenarios");
                    }
                    if flows {
                        let (icon, tip) = if self.playing { ("pause", "Pause flow animation") } else { ("play", "Play flow animation") };
                        if icon_button(ui, &mut self.icons, icon, self.playing, tip).clicked() {
                            self.playing = !self.playing;
                        }
                    }
                    if icon_button(ui, &mut self.icons, "users", self.collab.is_some(), "Collaborate live").clicked() {
                        act = Some("collab");
                    }
                    if icon_button(ui, &mut self.icons, if self.dark { "sun" } else { "moon" }, false, "Toggle dark mode  ⇧⌘D").clicked() {
                        act = Some("dark");
                    }
                });
            });
        });
        if let Some(a) = act {
            self.run(ctx, a);
        }
    }

    pub(super) fn scenario_panel(&mut self, ctx: &Context) {
        if !self.scn_panel {
            return;
        }
        let now = ctx.input(|i| i.time);
        let mut act: Option<(&str, usize, usize)> = None;
        egui::Area::new(Id::new("scenarios")).anchor(Align2::RIGHT_BOTTOM, vec2(self.right_x(), -64.0)).show(ctx, |ui| {
            panel().inner_margin(Margin::same(12)).show(ui, |ui| {
                ui.set_width(330.0);
                ui.set_max_height(560.0); // before any widget: it resets the cursor
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Request flows").strong().size(14.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, &mut self.icons, "x", false, "Close").clicked() {
                            self.scn_panel = false;
                        }
                    });
                });
                ui.label(RichText::new("Animate how one request travels through your system.").color(theme::muted()));
                if self.doc.scenarios.is_empty() {
                    ui.add_space(6.0);
                    Frame::new().fill(theme::hover()).corner_radius(8).inner_margin(Margin::same(10)).show(ui, |ui| {
                        ui.set_width(310.0);
                        for (n, t) in [
                            ("1", "Press Record below"),
                            ("2", "Click the components (or connectors) in the order the request travels, e.g. Users → API → Orders → DB. Only connectors you already drew are used."),
                            ("3", "Press Done (or Enter)"),
                            ("4", "Add a caption to each step, then press ▶ to play or ⤓ to export a GIF"),
                        ] {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(n).strong().color(theme::INDIGO));
                                ui.label(t);
                            });
                        }
                    });
                }
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    let n = self.doc.scenarios.len();
                    for i in 0..n {
                        let playing = self.scn_play.is_some_and(|p| p.0 == i);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            if icon_button(ui, &mut self.icons, if playing { "stop" } else { "play" }, playing, if playing { "Stop" } else { "Play" }).clicked() {
                                act = Some(("play", i, 0));
                            }
                            let s = &mut self.doc.scenarios[i];
                            if ui.add(TextEdit::singleline(&mut s.name).desired_width(180.0).frame(false).font(egui::TextStyle::Body)).on_hover_text("Click to rename").changed() {
                                self.dirty = true;
                            }
                            if icon_button(ui, &mut self.icons, "download", false, "Export as animated GIF").clicked() {
                                act = Some(("gif", i, 0));
                            }
                            if icon_button(ui, &mut self.icons, "trash", false, "Delete flow").clicked() {
                                act = Some(("del", i, 0));
                            }
                        });
                        let steps = self.doc.scenarios[i].steps.clone();
                        egui::CollapsingHeader::new(RichText::new(format!("{} steps · captions", steps.len())).small().color(theme::muted()))
                            .id_salt(("steps", i))
                            .default_open(playing || n == 1)
                            .show(ui, |ui| {
                                for (k, st) in steps.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("{}. {}", k + 1, render::step_title(&self.doc, st))).small().strong());
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if ui.add(egui::Button::new(RichText::new("✕").small()).frame(false)).on_hover_text("Remove this step").clicked() {
                                                act = Some(("delstep", i, k));
                                            }
                                        });
                                    });
                                    let cap = &mut self.doc.scenarios[i].steps[k].caption;
                                    if ui.add(TextEdit::singleline(cap).hint_text("What happens here? (shown while playing)").desired_width(f32::INFINITY)).changed() {
                                        self.dirty = true;
                                    }
                                }
                                if ui.button("+ Add more steps").on_hover_text("Click more components to extend this flow").clicked() {
                                    act = Some(("extend", i, 0));
                                }
                            });
                        ui.add_space(4.0);
                    }
                });
                ui.add_space(4.0);
                let b = egui::Button::new(RichText::new("●  Record").color(Color32::WHITE).strong()).fill(theme::RED).corner_radius(8).min_size(vec2(330.0, 32.0));
                if ui.add(b).on_hover_text("Then click components in order").clicked() {
                    act = Some(("record", 0, 0));
                }
            });
        });
        match act {
            Some(("play", i, _)) => self.scn_play = if self.scn_play.is_some_and(|p| p.0 == i) { None } else { Some((i, now)) },
            Some(("gif", i, _)) => self.export(ctx, "gif", Some(i)),
            Some(("del", i, _)) => {
                self.checkpoint();
                self.doc.scenarios.remove(i);
                self.scn_play = None;
            }
            Some(("delstep", i, k)) => {
                self.checkpoint();
                self.doc.scenarios[i].steps.remove(k);
                self.scn_play = None;
            }
            Some(("extend", i, _)) => {
                // continue recording from the last component of this scenario
                let last = self.doc.scenarios[i].steps.last().map(|s| s.to);
                self.scn_play = None;
                self.rec = Some((self.doc.scenarios[i].name.clone(), last.into_iter().collect()));
                self.rec_into = Some(i);
            }
            Some(("record", _, _)) => self.start_record(),
            _ => {}
        }
    }

    pub(super) fn recording_banner(&mut self, ctx: &Context) {
        let mut act = "";
        if let Some((name, nodes)) = &self.rec {
            let (name, n) = (name.clone(), nodes.len());
            egui::Area::new(Id::new("rec")).anchor(Align2::CENTER_TOP, vec2(-self.code_w / 2.0, 64.0)).order(Order::Foreground).show(ctx, |ui| {
                panel().inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("●").color(theme::RED).size(16.0));
                        ui.label(RichText::new(format!("Recording “{name}”")).strong());
                        ui.label(RichText::new(if n == 0 { "click the component where the request starts".to_string() } else { format!("click the next component or connector · {n} picked · Enter or Done to finish") }).color(theme::muted()));
                        if ui.button("Cancel").clicked() {
                            act = "cancel";
                        }
                        if ui.add(egui::Button::new(RichText::new("Done").color(Color32::WHITE)).fill(theme::INDIGO)).clicked() {
                            act = "done";
                        }
                    });
                });
            });
        } else if let Some((i, _)) = self.scn_play {
            let name = self.doc.scenarios.get(i).map(|s| s.name.clone()).unwrap_or_default();
            egui::Area::new(Id::new("playing")).anchor(Align2::CENTER_TOP, vec2(-self.code_w / 2.0, 64.0)).order(Order::Foreground).show(ctx, |ui| {
                panel().inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("▶  {name}")).strong());
                        ui.label(RichText::new("Esc to stop").color(theme::muted()));
                        if ui.button("Export GIF").clicked() {
                            act = "gif";
                        }
                        if ui.button("Stop").clicked() {
                            act = "stop";
                        }
                    });
                });
            });
        }
        match act {
            "cancel" => self.rec = None,
            "done" => self.finish_record(ctx),
            "stop" => self.scn_play = None,
            "gif" => {
                if let Some((i, _)) = self.scn_play {
                    self.export(ctx, "gif", Some(i));
                }
            }
            _ => {}
        }
    }

    /// Create / edit a custom boundary: name, header icon, color, line style, live preview.
    pub(super) fn boundary_editor(&mut self, ctx: &Context) {
        let Some(ed) = &mut self.bound_editor else { return };
        let mut act = "";
        egui::Area::new(Id::new("bound-editor")).anchor(Align2::CENTER_CENTER, vec2(-self.code_w / 2.0, 0.0)).order(Order::Foreground).show(ctx, |ui| {
            panel().inner_margin(Margin::same(16)).show(ui, |ui| {
                ui.set_width(380.0);
                ui.label(RichText::new(if ed.idx.is_some() { "Edit boundary" } else { "New boundary" }).strong().size(15.0));
                ui.add_space(6.0);
                // live preview
                let (pr, _) = ui.allocate_exact_size(vec2(380.0, 86.0), Sense::hover());
                let box_r = pr.shrink2(vec2(30.0, 8.0));
                let p = ui.painter();
                p.rect_filled(box_r, CornerRadius::same(14), ed.b.color.gamma_multiply(if ed.b.dashed { 0.05 } else { 0.08 }));
                let mut pts = rounded_rect(box_r, 14.0);
                pts.push(pts[0]);
                if ed.b.dashed {
                    p.extend(egui::Shape::dashed_line(&pts, Stroke::new(1.5, ed.b.color), 8.0, 6.0));
                } else {
                    p.add(egui::Shape::line(pts, Stroke::new(1.5, ed.b.color)));
                }
                if let Some(t) = self.icons.get(ctx, &ed.b.icon) {
                    p.image(t, Rect::from_min_size(box_r.min + vec2(14.0, 10.0), vec2(16.0, 16.0)), render::UV, ed.b.color);
                }
                p.text(box_r.min + vec2(36.0, 10.0), Align2::LEFT_TOP, &ed.b.name, render::font(13.0, true, false), ed.b.color);
                ui.add_space(6.0);
                ui.label(RichText::new("Name").size(11.0).color(theme::muted()).strong());
                ui.add(TextEdit::singleline(&mut ed.b.name).desired_width(f32::INFINITY));
                ui.label(RichText::new("Color").size(11.0).color(theme::muted()).strong());
                ui.horizontal_wrapped(|ui| {
                    for c in super::STROKES.iter().chain(&[Color32::from_rgb(0x02, 0x84, 0xc7), Color32::from_rgb(0x47, 0x55, 0x69), Color32::from_rgb(0x7c, 0x3a, 0xed)]) {
                        if swatch(ui, *c, ed.b.color == *c).clicked() {
                            ed.b.color = *c;
                        }
                    }
                    egui::color_picker::color_edit_button_srgba(ui, &mut ed.b.color, egui::color_picker::Alpha::Opaque);
                });
                ui.label(RichText::new("Border").size(11.0).color(theme::muted()).strong());
                if let Some(d) = seg(ui, ed.b.dashed, &[(true, "Dashed"), (false, "Solid")]) {
                    ed.b.dashed = d;
                }
                ui.label(RichText::new("Header icon").size(11.0).color(theme::muted()).strong());
                ui.add(TextEdit::singleline(&mut ed.query).hint_text("Search icons…").desired_width(f32::INFINITY));
                let q = ed.query.clone();
                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                        let list = icons::all().iter().filter(|i| !matches!(i.cat, "ui" | "Boundaries") && !i.color && (q.is_empty() || matches(&q, &format!("{} {} {}", i.id, i.label, i.cat))));
                        let mut seen: Vec<&str> = vec![];
                        for i in list {
                            if seen.contains(&i.svg) {
                                continue; // service icons reuse glyphs; show each look once
                            }
                            seen.push(i.svg);
                            let (r, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                            let on = ed.b.icon == i.id;
                            if on || resp.hovered() {
                                ui.painter().rect_filled(r, CornerRadius::same(7), if on { theme::active_bg() } else { theme::hover() });
                            }
                            if let Some(t) = self.icons.get(ctx, i.id) {
                                ui.painter().image(t, r.shrink(8.0), render::UV, if on { ed.b.color } else { theme::text() });
                            }
                            if resp.on_hover_text(i.label).clicked() {
                                ed.b.icon = i.id.to_string();
                            }
                        }
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Save").color(Color32::WHITE).strong()).fill(theme::INDIGO).min_size(vec2(90.0, 30.0))).clicked() {
                        act = "save";
                    }
                    if ui.button("Cancel").clicked() {
                        act = "cancel";
                    }
                    ui.label(RichText::new("Saved on this Mac · appears under Boundaries").small().color(theme::muted()));
                });
            });
        });
        match act {
            "save" => {
                let ed = self.bound_editor.take().unwrap();
                match ed.idx {
                    Some(i) => self.custom_bounds[i] = ed.b,
                    None => self.custom_bounds.push(ed.b),
                }
                self.library = true;
                self.lib_filter = Some("Boundaries");
                self.toast(ctx, "Boundary saved to your library");
            }
            "cancel" => self.bound_editor = None,
            _ => {}
        }
    }

    /// Context bars for the pen tool and outage simulation.
    pub(super) fn mode_bars(&mut self, ctx: &Context) {
        let mut act = "";
        if self.failures {
            let down = self.doc.elements.iter().filter(|e| e.down).count();
            egui::Area::new(Id::new("outage")).anchor(Align2::CENTER_TOP, vec2(-self.code_w / 2.0, 64.0)).order(Order::Foreground).show(ctx, |ui| {
                panel().inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠  Outage simulation").color(theme::RED).strong());
                        ui.label(RichText::new(if down == 0 { "Click any component to take it down".to_string() } else { format!("{down} down · their connections stop, backup paths take over · click again to restore") }).color(theme::muted()));
                        if down > 0 && ui.button("Restore all").clicked() {
                            act = "restore";
                        }
                        if ui.add(egui::Button::new(RichText::new("Done").color(Color32::WHITE)).fill(theme::INDIGO)).clicked() {
                            act = "done";
                        }
                    });
                });
            });
        } else if self.tool == Tool::Pen {
            let n = self.doc.elements.iter().filter(|e| e.kind == Kind::Pen).count();
            egui::Area::new(Id::new("penbar")).anchor(Align2::CENTER_TOP, vec2(-self.code_w / 2.0, 64.0)).order(Order::Foreground).show(ctx, |ui| {
                panel().inner_margin(Margin::symmetric(14, 6)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Drawing").strong());
                        ui.label(RichText::new("Esc to stop").color(theme::muted()));
                        if ui.add_enabled(n > 0, egui::Button::new(format!("Clear all drawings ({n})"))).clicked() {
                            act = "clear";
                        }
                    });
                });
            });
        }
        match act {
            "restore" => {
                self.checkpoint();
                for e in &mut self.doc.elements {
                    e.down = false;
                }
            }
            "done" => self.failures = false,
            "clear" => self.run(ctx, "clear-ink"),
            _ => {}
        }
    }

    pub(super) fn collab_window(&mut self, ctx: &Context) {
        if !self.collab_panel {
            return;
        }
        let mut act = "";
        egui::Area::new(Id::new("collab")).anchor(Align2::CENTER_CENTER, vec2(-self.code_w / 2.0, 0.0)).order(Order::Foreground).show(ctx, |ui| {
            panel().inner_margin(Margin::same(16)).show(ui, |ui| {
                ui.set_width(380.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Collaborate live").strong().size(15.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, &mut self.icons, "x", false, "Close").clicked() {
                            self.collab_panel = false;
                        }
                    });
                });
                match &self.collab {
                    Some(c) => {
                        ui.label(RichText::new(&c.status).color(theme::muted()));
                        if c.host {
                            ui.add_space(4.0);
                            ui.label("Share with teammates:");
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&c.share).monospace().strong());
                                if ui.button("Copy").clicked() {
                                    ctx.copy_text(c.share.clone());
                                }
                            });
                        }
                        ui.add_space(6.0);
                        ui.label(RichText::new(format!("●  {} (you)", self.my_name)).color(peer_color(c.me)));
                        for (id, p) in &c.peers {
                            ui.label(RichText::new(format!("●  {}", p.name)).color(peer_color(*id)));
                        }
                        ui.add_space(6.0);
                        if ui.button("Leave session").clicked() {
                            act = "leave";
                        }
                    }
                    None => {
                        ui.horizontal(|ui| {
                            ui.label("Your name");
                            ui.add(TextEdit::singleline(&mut self.my_name).desired_width(200.0));
                        });
                        ui.add_space(6.0);
                        if ui.add(egui::Button::new(RichText::new("Host this canvas").color(Color32::WHITE).strong()).fill(theme::INDIGO).min_size(vec2(380.0, 32.0))).clicked() {
                            act = "host";
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new("or join a teammate").color(theme::muted()));
                        ui.add(TextEdit::singleline(&mut self.join_addr).hint_text("host address  e.g. 192.168.1.20:7878").desired_width(f32::INFINITY));
                        ui.add(TextEdit::singleline(&mut self.join_code).hint_text("session code").desired_width(f32::INFINITY));
                        if ui.add_enabled(!self.join_addr.trim().is_empty(), egui::Button::new("Join").min_size(vec2(380.0, 30.0))).clicked() {
                            act = "join";
                        }
                        ui.add_space(6.0);
                        ui.label(RichText::new("Peer-to-peer, no server. Works on the same network or over Tailscale/VPN. Traffic is not encrypted, so use trusted networks only.").small().color(theme::muted()));
                    }
                }
            });
        });
        match act {
            "host" => match Collab::host(&self.my_name) {
                Ok(c) => {
                    self.toast(ctx, c.status.clone());
                    self.collab = Some(c);
                }
                Err(e) => self.toast(ctx, format!("Could not host: {e}")),
            },
            "join" => match Collab::join(self.join_addr.trim(), &self.join_code, &self.my_name) {
                Ok(c) => {
                    self.checkpoint();
                    self.collab = Some(c);
                    self.collab_panel = false;
                }
                Err(e) => self.toast(ctx, e),
            },
            "leave" => {
                if let Some(mut c) = self.collab.take() {
                    c.leave();
                }
            }
            _ => {}
        }
    }

    fn picker_items(&self, query: &str, connect: Option<(u64, u64)>) -> Vec<(String, String, String)> {
        let mut out = vec![];
        if connect.is_some() && query.is_empty() {
            out.push(("::twin".to_string(), "Same as source".to_string(), "duplicate and connect".to_string()));
        }
        if query.is_empty() {
            let cat = connect.and_then(|(_, src)| self.doc.get(src)).and_then(|e| icons::find(&e.icon)).map(|i| i.cat).unwrap_or("");
            for id in suggest(cat) {
                if let Some(i) = icons::find(id) {
                    out.push((i.id.to_string(), i.label.to_string(), if i.sub.is_empty() { i.cat.to_string() } else { i.sub.to_string() }));
                }
            }
        } else {
            for i in icons::all().iter().filter(|i| i.cat != "ui" && i.cat != "glyph" && matches(query, &format!("{} {} {} {}", i.id, i.label, i.sub, i.cat))).take(8) {
                out.push((i.id.to_string(), i.label.to_string(), if i.sub.is_empty() { i.cat.to_string() } else { i.sub.to_string() }));
            }
        }
        out
    }

    pub(super) fn picker_popup(&mut self, ctx: &Context) {
        let Some(pk) = &self.picker else { return };
        let items = self.picker_items(&pk.query, pk.connect);
        let (mut pick, mut close) = (None, false);
        let (screen, focus) = (pk.screen, pk.focus);
        let (up, down, enter, esc) = ctx.input(|i| (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown), i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        egui::Area::new(Id::new("picker")).fixed_pos(screen + vec2(8.0, 8.0)).order(Order::Foreground).show(ctx, |ui| {
            panel().inner_margin(Margin::same(8)).show(ui, |ui| {
                ui.set_width(280.0);
                let pk = self.picker.as_mut().unwrap();
                let r = ui.add(TextEdit::singleline(&mut pk.query).hint_text(if pk.connect.is_some() { "Connect to…" } else { "Insert component…" }).desired_width(f32::INFINITY).margin(Margin::symmetric(8, 6)));
                if focus {
                    r.request_focus();
                    pk.focus = false;
                }
                if r.changed() {
                    pk.sel = 0;
                }
                if down {
                    pk.sel = (pk.sel + 1).min(items.len().saturating_sub(1));
                }
                if up {
                    pk.sel = pk.sel.saturating_sub(1);
                }
                if enter && !items.is_empty() {
                    pick = Some(items[pk.sel.min(items.len() - 1)].0.clone());
                }
                if esc {
                    close = true;
                }
                let sel = pk.sel;
                for (k, (id, label, sub)) in items.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(vec2(280.0, 38.0), Sense::click());
                    if k == sel || resp.hovered() {
                        ui.painter().rect_filled(rect, CornerRadius::same(7), theme::active_bg());
                    }
                    let tile = Rect::from_min_size(rect.min + vec2(6.0, 5.0), vec2(28.0, 28.0));
                    if let Some(i) = icons::find(id) {
                        let accent = icons::accent(i.cat);
                        if !i.color {
                            ui.painter().rect_filled(tile, CornerRadius::same(7), theme::map(tint(accent, 0.12), theme::dark()));
                        }
                        if let Some(t) = self.icons.get(ctx, id) {
                            ui.painter().image(t, tile.shrink(if i.color { 1.0 } else { 6.0 }), render::UV, if i.color { Color32::WHITE } else { accent });
                        }
                    } else if let Some(t) = self.icons.get(ctx, "copy") {
                        ui.painter().image(t, tile.shrink(6.0), render::UV, theme::text());
                    }
                    ui.painter().text(rect.min + vec2(42.0, 7.0), Align2::LEFT_TOP, label, FontId::proportional(13.0), theme::text());
                    ui.painter().text(rect.min + vec2(42.0, 23.0), Align2::LEFT_TOP, sub, FontId::proportional(10.5), theme::muted());
                    if resp.clicked() {
                        pick = Some(id.clone());
                    }
                }
                if items.is_empty() {
                    ui.label(RichText::new("No matches").color(theme::muted()));
                }
            });
        });
        if close {
            self.picker = None;
            return;
        }
        let Some(id) = pick else { return };
        let pk = self.picker.take().unwrap();
        let new_id = if id == "::twin" {
            pk.connect.and_then(|(_, src)| self.doc.get(src).cloned()).map(|mut t| {
                t.rect = Rect::from_center_size(pk.world, t.rect.size());
                t.key.clear();
                self.doc.add(t)
            })
        } else {
            Some(self.insert_icon(&id, Some(pk.world)))
        };
        if let (Some((arrow, _)), Some(n)) = (pk.connect, new_id) {
            if let Some(a) = self.doc.get_mut(arrow) {
                a.to.id = Some(n);
            }
        }
        if let Some(n) = new_id {
            self.sel = vec![n];
        }
    }

    pub(super) fn palette_popup(&mut self, ctx: &Context) {
        let Some(pal) = &self.palette else { return };
        let q = pal.query.clone();
        let mut rows: Vec<(String, String, String)> = COMMANDS.iter().filter(|c| q.is_empty() || matches(&q, c.0)).map(|c| (c.0.to_string(), c.1.to_string(), c.2.to_string())).collect();
        if !q.is_empty() {
            for i in icons::all().iter().filter(|i| i.cat != "ui" && i.cat != "glyph" && matches(&q, &format!("{} {} {}", i.label, i.sub, i.cat))).take(5) {
                rows.push((format!("Add {}", i.label), i.cat.to_string(), format!("insert:{}", i.id)));
            }
            for e in self.doc.elements.iter().filter(|e| !e.text.is_empty() && matches(&q, &e.text)).take(5) {
                rows.push((format!("Go to “{}”", node_text(&e.text).0), "on canvas".into(), format!("goto:{}", e.id)));
            }
        }
        let (up, down, enter, esc) = ctx.input(|i| (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown), i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        let (mut run, mut close) = (None, esc);
        egui::Area::new(Id::new("palette")).anchor(Align2::CENTER_TOP, vec2(-self.code_w / 2.0, 90.0)).order(Order::Foreground).show(ctx, |ui| {
            panel().inner_margin(Margin::same(10)).show(ui, |ui| {
                ui.set_width(560.0);
                ui.set_max_height(440.0); // before any widget: it resets the cursor
                let pal = self.palette.as_mut().unwrap();
                let r = ui.add(TextEdit::singleline(&mut pal.query).hint_text("Type a command, component or element…").desired_width(f32::INFINITY).font(egui::TextStyle::Heading).margin(Margin::symmetric(10, 8)));
                if pal.focus {
                    r.request_focus();
                    pal.focus = false;
                }
                if r.changed() {
                    pal.sel = 0;
                }
                if down {
                    pal.sel = (pal.sel + 1).min(rows.len().saturating_sub(1));
                }
                if up {
                    pal.sel = pal.sel.saturating_sub(1);
                }
                if enter && !rows.is_empty() {
                    run = Some(rows[pal.sel.min(rows.len() - 1)].2.clone());
                }
                let sel = pal.sel;
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for (k, (label, hint, a)) in rows.iter().enumerate() {
                        let (rect, resp) = ui.allocate_exact_size(vec2(560.0, 32.0), Sense::click());
                        if k == sel || resp.hovered() {
                            ui.painter().rect_filled(rect, CornerRadius::same(7), theme::active_bg());
                        }
                        ui.painter().text(rect.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER, label, FontId::proportional(13.5), theme::text());
                        ui.painter().text(rect.right_center() - vec2(10.0, 0.0), Align2::RIGHT_CENTER, hint, FontId::proportional(11.5), theme::muted());
                        if resp.clicked() {
                            run = Some(a.clone());
                        }
                    }
                });
            });
        });
        if run.is_some() || close {
            self.palette = None;
            close = false;
        }
        let _ = close;
        if let Some(a) = run {
            self.run(ctx, &a);
        }
    }

    pub(super) fn code_panel(&mut self, ctx: &Context) {
        if !self.code_open {
            self.code_w = 0.0;
            return;
        }
        let r = egui::SidePanel::right("code")
            .resizable(true)
            .default_width(400.0)
            .frame(Frame::new().fill(theme::panel_bg()).inner_margin(Margin::same(14)).stroke(Stroke::new(1.0, theme::panel_border())))
            .show(ctx, |ui| {
                ui.add_space(26.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Diagram as code").strong().size(15.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, &mut self.icons, "x", false, "Close  ⌘/").clicked() {
                            self.code_open = false;
                        }
                    });
                });
                ui.label(RichText::new("Write one connection per line. The canvas updates as you type, and drawing updates the text.").color(theme::muted()));
                ui.add_space(4.0);
                Frame::new().fill(theme::hover()).corner_radius(8).inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.label(RichText::new("Users -> API Gateway -> Orders\nOrders -> Postgres : SQL\nOrders => Kafka").monospace().size(12.0));
                    ui.add_space(2.0);
                    ui.label(RichText::new("->  arrow     =>  animated     -->  dashed\n:   adds a label    Postgres, S3, Lambda… get their icon").small().color(theme::muted()));
                });
                ui.add_space(6.0);
                let h = ui.available_height() - 60.0;
                let mut resp = None;
                egui::ScrollArea::vertical().max_height(h).show(ui, |ui| {
                    resp = Some(ui.add(TextEdit::multiline(&mut self.code_text).code_editor().desired_width(f32::INFINITY).desired_rows(30).margin(Margin::same(8))));
                });
                let resp = resp.unwrap();
                if resp.gained_focus() {
                    self.checkpoint();
                }
                if resp.changed() {
                    if let Some(m) = code::from_mermaid(&self.code_text) {
                        self.code_text = m;
                    }
                    match code::apply(&mut self.doc, &self.code_text) {
                        Ok(()) => self.code_err = None,
                        Err(e) => self.code_err = Some(e),
                    }
                    self.dirty = true;
                }
                if !resp.has_focus() {
                    self.code_text = code::to_code(&self.doc);
                }
                match &self.code_err {
                    Some(e) => ui.label(RichText::new(format!("⚠ {e}")).color(theme::RED).small()),
                    None => ui.label(RichText::new("Tip: paste a Mermaid flowchart to convert it").small().color(theme::muted())),
                };
            });
        self.code_w = r.response.rect.width();
    }

    pub(super) fn text_editor(&mut self, ctx: &Context) {
        let Some(id) = self.editing else { return };
        let Some(e) = self.doc.get(id).cloned() else {
            self.editing = None;
            return;
        };
        let v = self.view();
        let z = self.zoom;
        let r = e.rect;
        let hand = self.doc.sketch;
        let (pos, width, size, bold, color, align) = match e.kind {
            Kind::Text => (v.w2s(r.min), (r.width() + 40.0) * z, e.style.font, false, e.style.stroke, Align::Min),
            Kind::Node => {
                let l = node_layout(r);
                (v.w2s(pos2(l.text_x, r.min.y + 8.0)), (r.max.x - l.text_x - 8.0) * z, NODE_TITLE, false, INK, Align::Min)
            }
            Kind::Comment => (v.w2s(r.min) + vec2(24.0, -8.0), 240.0, 13.0 / z, false, INK, Align::Min),
            Kind::Arrow => {
                // a pill sized to the text, centred on the label spot
                let c = render::arrow_label_center(&self.doc.arrow_path(&e), &e);
                let size = render::label_size(&e);
                let m = render::measure(ctx, if e.text.is_empty() { "Label" } else { &e.text }, size, hand);
                let w = m.x + 16.0;
                (v.w2s(c - vec2(w / 2.0 + 8.0, m.y / 2.0 + 4.0)), w * z, size, false, e.style.stroke, Align::Center)
            }
            _ => {
                let m = render::measure(ctx, &e.text, e.style.font, hand);
                let (x, top, ax) = text_place(r, e.tpos, e.outside, m, LABEL_PAD);
                let w = if e.outside && e.tpos.fx() != 0.5 { m.x + 60.0 } else { (r.width() - 2.0 * LABEL_PAD).max(60.0) };
                let left = x - w * ax + if e.kind == Kind::Region && !e.icon.is_empty() { 22.0 } else { 0.0 };
                let align = if ax == 0.0 {
                    Align::Min
                } else if ax == 1.0 {
                    Align::Max
                } else {
                    Align::Center
                };
                (v.w2s(pos2(left, top)), w * z, e.style.font, e.kind == Kind::Region, if e.kind == Kind::Sticky { INK } else { e.style.stroke }, align)
            }
        };
        let color = theme::map(color, self.dark);
        let mut done = false;
        let focus = std::mem::take(&mut self.edit_focus);
        let mut text = e.text.clone();
        let pill = e.kind == Kind::Arrow;
        egui::Area::new(Id::new("editor")).fixed_pos(pos).order(Order::Foreground).show(ctx, |ui| {
            let te = TextEdit::multiline(&mut text)
                .font(render::font(size * z, bold, hand))
                .text_color(color)
                .frame(e.kind == Kind::Comment)
                .desired_width(width.max(40.0))
                .desired_rows(1)
                .hint_text(if pill { "Label" } else { "" })
                .margin(if e.kind == Kind::Comment { Margin::same(6) } else { Margin::ZERO })
                .horizontal_align(align);
            let r = if pill {
                // keep the label readable over lines and borders while typing
                Frame::new()
                    .fill(theme::panel_bg())
                    .stroke(Stroke::new(1.5, theme::INDIGO))
                    .corner_radius(CornerRadius::same(((size * z + 8.0) / 2.0).min(255.0) as u8))
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| ui.add(te))
                    .inner
            } else {
                ui.add(te)
            };
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

    pub(super) fn toast_view(&mut self, ctx: &Context) {
        let Some((msg, until)) = self.toast.clone() else { return };
        if ctx.input(|i| i.time) > until {
            self.toast = None;
            return;
        }
        egui::Area::new(Id::new("toast")).anchor(Align2::CENTER_BOTTOM, vec2(-self.code_w / 2.0, -24.0)).order(Order::Tooltip).show(ctx, |ui| {
            Frame::new().fill(Color32::from_rgb(0x11, 0x18, 0x27)).corner_radius(10).inner_margin(Margin::symmetric(14, 9)).show(ui, |ui| {
                ui.label(RichText::new(msg).color(Color32::WHITE));
            });
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}
