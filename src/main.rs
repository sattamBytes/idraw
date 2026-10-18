mod app;
mod cli;
mod code;
mod collab;
mod export;
mod finder;
mod icons;
mod import;
mod layout;
mod lenient;
mod model;
mod render;
mod theme;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// System fonts: SF for UI, Avenir Next for canvas text. Falls back to egui defaults.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut f = FontDefinitions::default();
    let fallback = f.families[&FontFamily::Proportional].clone();
    let load = |f: &mut FontDefinitions, key: &str, path: &str, index: u32| -> bool {
        let Ok(bytes) = std::fs::read(path) else { return false };
        let mut d = FontData::from_owned(bytes);
        d.index = index;
        f.font_data.insert(key.into(), Arc::new(d));
        true
    };
    if load(&mut f, "sf", "/System/Library/Fonts/SFNS.ttf", 0) {
        f.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "sf".into());
    }
    for (family, key, path, index) in [
        ("canvas", "avenir", "/System/Library/Fonts/Avenir Next.ttc", 7),
        ("canvas-bold", "avenir-demi", "/System/Library/Fonts/Avenir Next.ttc", 2),
        ("hand", "noteworthy", "/System/Library/Fonts/Noteworthy.ttc", 0),
        ("hand-bold", "noteworthy-bold", "/System/Library/Fonts/Noteworthy.ttc", 1),
        ("mono", "menlo", "/System/Library/Fonts/Menlo.ttc", 0),
        ("mono-bold", "menlo-bold", "/System/Library/Fonts/Menlo.ttc", 1),
        ("serif", "georgia", "/System/Library/Fonts/Supplemental/Georgia.ttf", 0),
        ("serif-bold", "georgia-bold", "/System/Library/Fonts/Supplemental/Georgia Bold.ttf", 0),
    ] {
        let mut list = fallback.clone();
        if load(&mut f, key, path, index) {
            list.insert(0, key.into());
        }
        f.families.insert(FontFamily::Name(family.into()), list);
    }
    ctx.set_fonts(f);
}

fn main() -> eframe::Result {
    if let Some(code) = cli::run() {
        std::process::exit(code);
    }
    finder::install();
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("idraw")
            // an empty icon keeps the app bundle's icon in the Dock (otherwise eframe shows its "e" logo)
            .with_icon(std::sync::Arc::new(egui::IconData::default()))
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([820.0, 520.0])
            // standard title bar: drags that start on the canvas or toolbar never move the window
            .with_fullsize_content_view(false),
        ..Default::default()
    };
    eframe::run_native("idraw", opts, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
