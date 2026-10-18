mod app;
mod export;
mod icons;
mod model;
mod render;

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
    for (family, key, index) in [("canvas", "avenir", 7), ("canvas-bold", "avenir-demi", 2)] {
        let mut list = fallback.clone();
        if load(&mut f, key, "/System/Library/Fonts/Avenir Next.ttc", index) {
            list.insert(0, key.into());
        }
        f.families.insert(FontFamily::Name(family.into()), list);
    }
    ctx.set_fonts(f);
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("idraw")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([820.0, 520.0])
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        ..Default::default()
    };
    eframe::run_native("idraw", opts, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
