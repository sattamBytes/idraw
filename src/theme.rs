//! Light / dark theme colors for UI chrome, and display mapping for document colors.

use eframe::egui::Color32;
use std::sync::atomic::{AtomicBool, Ordering};

static DARK: AtomicBool = AtomicBool::new(false);

pub fn dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

pub fn set_dark(d: bool) {
    DARK.store(d, Ordering::Relaxed);
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn pick(light: Color32, dark_c: Color32) -> Color32 {
    if dark() { dark_c } else { light }
}

pub const INDIGO: Color32 = rgb(0x63, 0x66, 0xf1);
pub const RED: Color32 = rgb(0xef, 0x44, 0x44);
pub const GUIDE: Color32 = rgb(0xec, 0x48, 0x99);

pub fn canvas_bg() -> Color32 {
    pick(rgb(0xfa, 0xfa, 0xfb), rgb(0x12, 0x14, 0x18))
}
pub fn grid() -> Color32 {
    pick(rgb(0xdd, 0xdf, 0xe4), rgb(0x2a, 0x2e, 0x36))
}
pub fn panel_bg() -> Color32 {
    pick(Color32::WHITE, rgb(0x1c, 0x1f, 0x26))
}
pub fn panel_border() -> Color32 {
    pick(rgb(0xe8, 0xe9, 0xee), rgb(0x2e, 0x33, 0x3d))
}
pub fn text() -> Color32 {
    pick(rgb(0x37, 0x41, 0x51), rgb(0xd1, 0xd5, 0xdb))
}
pub fn muted() -> Color32 {
    pick(rgb(0x6b, 0x72, 0x80), rgb(0x8b, 0x92, 0x9e))
}
pub fn hover() -> Color32 {
    pick(rgb(0xf3, 0xf4, 0xf6), rgb(0x2a, 0x2e, 0x37))
}
pub fn active_bg() -> Color32 {
    pick(rgb(0xee, 0xf2, 0xff), rgb(0x2b, 0x2d, 0x5c))
}
pub fn active_fg() -> Color32 {
    pick(rgb(0x4f, 0x46, 0xe5), rgb(0xa5, 0xb4, 0xfc))
}

/// Map a document color for display. Dark ink turns light, white/pastel fills turn dark.
pub fn map(c: Color32, dark: bool) -> Color32 {
    if !dark || c.a() == 0 {
        return c;
    }
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let l = (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;
    // only near-grey ink flips; saturated colors (reds, brand accents) keep their hue
    let grey = r.max(g).max(b) - r.min(g).min(b) < 60;
    // faint tints (region fills) keep their hue; only solid ink flips
    let ink = l < 0.4 && grey && a > 128;
    let f = |v: u8| -> u8 {
        if ink {
            (238.0 - v as f32 * 0.4) as u8
        } else {
            (30.0 + (v as f32 - 30.0) * 0.12) as u8
        }
    };
    if ink || l > 0.8 { Color32::from_rgba_unmultiplied(f(r), f(g), f(b), a) } else { c }
}
