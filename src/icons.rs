//! Built-in icon set. 24x24 stroke icons (lucide-style), drawn for idraw.
//! Each entry: (id, label, category, inner SVG). Category "ui" = app chrome only.

use eframe::egui::{Color32, ColorImage, Context, TextureHandle, TextureId, TextureOptions};
use std::collections::HashMap;

pub const CATEGORIES: &[&str] = &["Clients", "Network", "Compute", "Data", "Messaging", "Security", "Ops", "AI"];

pub fn accent(cat: &str) -> Color32 {
    match cat {
        "Clients" => Color32::from_rgb(0x4f, 0x46, 0xe5),
        "Network" => Color32::from_rgb(0x7c, 0x3a, 0xed),
        "Compute" => Color32::from_rgb(0x16, 0xa3, 0x4a),
        "Data" => Color32::from_rgb(0xdc, 0x26, 0x26),
        "Messaging" => Color32::from_rgb(0xd9, 0x77, 0x06),
        "Security" => Color32::from_rgb(0x0f, 0x76, 0x6e),
        "Ops" => Color32::from_rgb(0x02, 0x84, 0xc7),
        "AI" => Color32::from_rgb(0xdb, 0x27, 0x77),
        _ => Color32::from_rgb(0x47, 0x55, 0x69),
    }
}

pub const ICONS: &[(&str, &str, &str, &str)] = &[
    // Clients
    ("user", "User", "Clients", r#"<circle cx="12" cy="8" r="4"/><path d="M4 21v-1a6 6 0 0 1 6-6h4a6 6 0 0 1 6 6v1"/>"#),
    ("laptop", "Web client", "Clients", r#"<rect x="4" y="5" width="16" height="11" rx="2"/><path d="M2 20h20"/>"#),
    ("mobile", "Mobile app", "Clients", r#"<rect x="7" y="2" width="10" height="20" rx="2"/><path d="M11 18h2"/>"#),
    ("browser", "Browser", "Clients", r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18"/><path d="M6 6.5h.01M9 6.5h.01"/>"#),
    ("iot", "IoT device", "Clients", r#"<path d="M2 9a15 15 0 0 1 20 0M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0"/><path d="M12 19.5h.01"/>"#),
    ("terminal", "CLI / Admin", "Clients", r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M13 15h4"/>"#),
    // Network
    ("globe", "Internet", "Network", r#"<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18z"/>"#),
    ("dns", "DNS", "Network", r#"<path d="M12 3v18"/><path d="M5 6h11l3 3-3 3H5z"/><path d="M9 21h6"/>"#),
    ("cdn", "CDN", "Network", r#"<circle cx="12" cy="12" r="3"/><circle cx="4" cy="6" r="2"/><circle cx="20" cy="6" r="2"/><circle cx="4" cy="18" r="2"/><circle cx="20" cy="18" r="2"/><path d="M6 7l3.5 3M18 7l-3.5 3M6 17l3.5-3M18 17l-3.5-3"/>"#),
    ("lb", "Load balancer", "Network", r#"<circle cx="12" cy="5" r="2.5"/><path d="M12 7.5V14M5 18v-4h14v4M12 14v4"/><circle cx="5" cy="19.5" r="1.5"/><circle cx="12" cy="19.5" r="1.5"/><circle cx="19" cy="19.5" r="1.5"/>"#),
    ("gateway", "API gateway", "Network", r#"<path d="M3 12h5M16 12h5"/><rect x="8" y="6" width="8" height="12" rx="2"/><path d="M11 10l-1.5 2 1.5 2M13 10l1.5 2-1.5 2"/>"#),
    ("network", "VPC / Network", "Network", r#"<rect x="9" y="2" width="6" height="5" rx="1"/><rect x="2" y="17" width="6" height="5" rx="1"/><rect x="16" y="17" width="6" height="5" rx="1"/><path d="M12 7v5M5 17v-5h14v5"/>"#),
    ("api", "API", "Network", r#"<path d="M8 4c-2 0-3 1-3 3v2c0 1.5-1 3-2 3 1 0 2 1.5 2 3v2c0 2 1 3 3 3M16 4c2 0 3 1 3 3v2c0 1.5 1 3 2 3-1 0-2 1.5-2 3v2c0 2-1 3-3 3"/>"#),
    ("webhook", "Webhook", "Network", r#"<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>"#),
    // Compute
    ("server", "Server", "Compute", r#"<rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/>"#),
    ("service", "Service", "Compute", r#"<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M12 12l8-4.5M12 12v9M12 12L4 7.5"/>"#),
    ("container", "Container", "Compute", r#"<rect x="3" y="10" width="18" height="10" rx="2"/><path d="M7 10v10M12 10v10M17 10v10"/><rect x="8" y="4" width="8" height="6" rx="1"/>"#),
    ("k8s", "Kubernetes", "Compute", r#"<path d="M12 2.5l8.5 4.2 2 9.2-5.9 7.4H7.4L1.5 15.9l2-9.2z"/><circle cx="12" cy="12.5" r="3"/><path d="M12 6v3.5M12 15.5V19M6 10.5l3.2 1.2M18 10.5l-3.2 1.2M8 17.5l2.2-2.6M16 17.5l-2.2-2.6"/>"#),
    ("function", "Function", "Compute", r#"<rect x="3" y="3" width="18" height="18" rx="4"/><path d="M8 7h2l6 10M12 10.5L8 17"/>"#),
    ("worker", "Worker", "Compute", r#"<circle cx="12" cy="12" r="3"/><circle cx="12" cy="12" r="7"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1"/>"#),
    ("cpu", "Compute", "Compute", r#"<rect x="6" y="6" width="12" height="12" rx="2"/><rect x="9.5" y="9.5" width="5" height="5"/><path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4"/>"#),
    ("cloud", "Cloud", "Compute", r#"<path d="M7 18a5 5 0 1 1 1-9.9A6 6 0 0 1 19.5 10 4 4 0 0 1 18 18z"/>"#),
    // Data
    ("database", "Database", "Data", r#"<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5"/><path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3"/>"#),
    ("cache", "Cache", "Data", r#"<path d="M13 2L4 14h7l-1 8 9-12h-7z"/>"#),
    ("bucket", "Object storage", "Data", r#"<path d="M4 6h16l-2 14H6z"/><ellipse cx="12" cy="6" rx="8" ry="2.5"/>"#),
    ("file", "File / Blob", "Data", r#"<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>"#),
    ("search", "Search index", "Data", r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4"/>"#),
    ("warehouse", "Analytics", "Data", r#"<path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/>"#),
    // Messaging
    ("queue", "Queue", "Messaging", r#"<rect x="3" y="7" width="4" height="10" rx="1"/><rect x="10" y="7" width="4" height="10" rx="1"/><rect x="17" y="7" width="4" height="10" rx="1"/>"#),
    ("stream", "Stream / Topic", "Messaging", r#"<path d="M2 7c2.5-2 4.5-2 7 0s4.5 2 7 0 4.5-2 6 0M2 12c2.5-2 4.5-2 7 0s4.5 2 7 0 4.5-2 6 0M2 17c2.5-2 4.5-2 7 0s4.5 2 7 0 4.5-2 6 0"/>"#),
    ("pubsub", "Event bus", "Messaging", r#"<circle cx="12" cy="12" r="2"/><path d="M16.2 7.8a6 6 0 0 1 0 8.4M7.8 16.2a6 6 0 0 1 0-8.4M19.1 4.9a10 10 0 0 1 0 14.2M4.9 19.1a10 10 0 0 1 0-14.2"/>"#),
    ("email", "Email", "Messaging", r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 7l9 6 9-6"/>"#),
    ("bell", "Notifications", "Messaging", r#"<path d="M6 16V11a6 6 0 0 1 12 0v5l2 2H4z"/><path d="M10 21h4"/>"#),
    // Security
    ("shield", "Firewall / WAF", "Security", r#"<path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z"/>"#),
    ("lock", "Auth", "Security", r#"<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>"#),
    ("key", "Secrets", "Security", r#"<circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3M18 5l2 2"/>"#),
    ("card", "Payments", "Security", r#"<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M2 10h20M6 15h4"/>"#),
    // Ops
    ("chart", "Monitoring", "Ops", r#"<path d="M3 3v18h18"/><path d="M7 15l4-5 3 3 5-7"/>"#),
    ("logs", "Logs", "Ops", r#"<rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 8h8M8 12h8M8 16h5"/>"#),
    ("clock", "Scheduler", "Ops", r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>"#),
    ("git", "CI / CD", "Ops", r#"<circle cx="6" cy="6" r="2.5"/><circle cx="6" cy="18" r="2.5"/><circle cx="18" cy="8" r="2.5"/><path d="M6 8.5v7M18 10.5c0 4-6 3-11 5.5"/>"#),
    // AI
    ("sparkles", "AI model", "AI", r#"<path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9z"/><path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z"/>"#),
    // UI chrome
    ("t-select", "", "ui", r#"<path d="M5 3l14 8-6 2-3 6z"/>"#),
    ("t-hand", "", "ui", r#"<path d="M8 13V5.5a1.5 1.5 0 0 1 3 0V12M11 11V4.5a1.5 1.5 0 0 1 3 0V11M14 11V5.5a1.5 1.5 0 0 1 3 0V13a7 7 0 0 1-7 7h-.5a6 6 0 0 1-5-2.7L3 14.5a1.6 1.6 0 0 1 2.5-2L8 15"/>"#),
    ("t-rect", "", "ui", r#"<rect x="4" y="4" width="16" height="16" rx="3"/>"#),
    ("t-ellipse", "", "ui", r#"<circle cx="12" cy="12" r="8.5"/>"#),
    ("t-diamond", "", "ui", r#"<path d="M12 3l9 9-9 9-9-9z"/>"#),
    ("t-arrow", "", "ui", r#"<path d="M5 19L19 5M10 5h9v9"/>"#),
    ("t-text", "", "ui", r#"<path d="M5 6V4h14v2M12 4v16M9 20h6"/>"#),
    ("t-pen", "", "ui", r#"<path d="M4 20l1-4L16 5a2.1 2.1 0 0 1 3 3L8 19z"/>"#),
    ("t-region", "", "ui", r#"<rect x="3" y="3" width="18" height="18" rx="3" stroke-dasharray="3 3"/>"#),
    ("t-library", "", "ui", r#"<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>"#),
    ("menu", "", "ui", r#"<path d="M4 6h16M4 12h16M4 18h16"/>"#),
    ("undo", "", "ui", r#"<path d="M9 14L4 9l5-5"/><path d="M4 9h11a5 5 0 0 1 0 10h-3"/>"#),
    ("redo", "", "ui", r#"<path d="M15 14l5-5-5-5"/><path d="M20 9H9a5 5 0 0 0 0 10h3"/>"#),
    ("play", "", "ui", r#"<path d="M7 4l13 8-13 8z"/>"#),
    ("pause", "", "ui", r#"<path d="M8 4v16M16 4v16"/>"#),
    ("download", "", "ui", r#"<path d="M12 3v12M7 10l5 5 5-5M5 21h14"/>"#),
    ("trash", "", "ui", r#"<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>"#),
    ("copy", "", "ui", r#"<rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/>"#),
    ("front", "", "ui", r#"<path d="M12 19V5M5 12l7-7 7 7"/>"#),
    ("back", "", "ui", r#"<path d="M12 5v14M5 12l7 7 7-7"/>"#),
    ("plus", "", "ui", r#"<path d="M12 5v14M5 12h14"/>"#),
    ("minus", "", "ui", r#"<path d="M5 12h14"/>"#),
    ("x", "", "ui", r#"<path d="M6 6l12 12M18 6L6 18"/>"#),
    ("fit", "", "ui", r#"<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>"#),
];

pub fn find(id: &str) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    ICONS.iter().find(|i| i.0 == id)
}

/// Standalone SVG document for an icon, in a given stroke color.
pub fn svg_doc(inner: &str, size: u32, color: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="{size}" height="{size}" fill="none" stroke="{color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{inner}</svg>"#
    )
}

/// White icon textures, tinted at draw time.
#[derive(Default)]
pub struct Icons {
    tex: HashMap<&'static str, TextureHandle>,
}

impl Icons {
    pub fn get(&mut self, ctx: &Context, id: &str) -> Option<TextureId> {
        let (key, _, _, inner) = find(id)?;
        if let Some(t) = self.tex.get(key) {
            return Some(t.id());
        }
        const PX: u32 = 72;
        let tree = resvg::usvg::Tree::from_str(&svg_doc(inner, PX, "#ffffff"), &Default::default()).ok()?;
        let mut pm = resvg::tiny_skia::Pixmap::new(PX, PX)?;
        resvg::render(&tree, Default::default(), &mut pm.as_mut());
        let img = ColorImage::from_rgba_premultiplied([PX as usize, PX as usize], pm.data());
        let h = ctx.load_texture(*key, img, TextureOptions::LINEAR);
        let id = h.id();
        self.tex.insert(key, h);
        Some(id)
    }
}
