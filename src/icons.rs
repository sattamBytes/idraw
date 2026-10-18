//! Icon catalog: idraw's own 24x24 stroke glyphs, cloud/OSS services drawn with them,
//! boundary presets, and user-imported full-color SVGs (e.g. official vendor packs).
//!
//! Vendor logos are trademarks with restrictive licenses, so built-ins use our own glyphs
//! labelled with the service name; official packs can be dropped into `custom_dir()`.

use eframe::egui::{Color32, ColorImage, Context, TextureHandle, TextureId, TextureOptions};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Icon {
    pub id: &'static str,
    pub label: &'static str,
    pub cat: &'static str,
    /// Inner SVG (stroke glyph) or, when `color`, a full SVG document.
    pub svg: &'static str,
    pub sub: &'static str,
    pub color: bool,
}

pub const CATEGORIES: &[&str] = &[
    "Shapes",
    "Boundaries",
    "Clients",
    "Network",
    "Compute",
    "Data",
    "Messaging",
    "Security",
    "Ops",
    "AI",
    "AWS",
    "Google Cloud",
    "Azure",
    "Cloudflare",
    "Other clouds",
    "Open source",
];

pub fn accent(cat: &str) -> Color32 {
    let h = |v: u32| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    match cat {
        "Clients" => h(0x4f46e5),
        "Network" => h(0x7c3aed),
        "Compute" => h(0x16a34a),
        "Data" => h(0xdc2626),
        "Messaging" => h(0xd97706),
        "Security" => h(0x0f766e),
        "Ops" => h(0x0284c7),
        "AI" => h(0xdb2777),
        "AWS" => h(0xe7730b),
        "Google Cloud" => h(0x1a73e8),
        "Azure" => h(0x0078d4),
        "Cloudflare" => h(0xe8710f),
        "Other clouds" => h(0x0891b2),
        "Open source" => h(0x475569),
        "Shapes" => h(0x1f2937),
        _ => h(0x475569),
    }
}

/// Generic glyphs: (id, label, category, inner svg). Category "ui"/"glyph" = not shown as components.
const GLYPHS: &[(&str, &str, &str, &str)] = &[
    // Clients
    ("user", "User", "Clients", r#"<circle cx="12" cy="8" r="4"/><path d="M4 21v-1a6 6 0 0 1 6-6h4a6 6 0 0 1 6 6v1"/>"#),
    ("laptop", "Web client", "Clients", r#"<rect x="4" y="5" width="16" height="11" rx="2"/><path d="M2 20h20"/>"#),
    ("mobile", "Mobile app", "Clients", r#"<rect x="7" y="2" width="10" height="20" rx="2"/><path d="M11 18h2"/>"#),
    ("browser", "Browser", "Clients", r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18"/><path d="M6 6.5h.01M9 6.5h.01"/>"#),
    ("iot", "IoT device", "Clients", r#"<path d="M2 9a15 15 0 0 1 20 0M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0"/><path d="M12 19.5h.01"/>"#),
    ("terminal", "CLI / Admin", "Clients", r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M13 15h4"/>"#),
    ("identity", "Identity", "Clients", r#"<circle cx="9" cy="8" r="4"/><path d="M2 21v-1a6 6 0 0 1 10-4.5M16 18l2 2 4-4"/>"#),
    // Network
    ("globe", "Internet", "Network", r#"<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18z"/>"#),
    ("dns", "DNS", "Network", r#"<path d="M12 3v18"/><path d="M5 6h11l3 3-3 3H5z"/><path d="M9 21h6"/>"#),
    ("cdn", "CDN", "Network", r#"<circle cx="12" cy="12" r="3"/><circle cx="4" cy="6" r="2"/><circle cx="20" cy="6" r="2"/><circle cx="4" cy="18" r="2"/><circle cx="20" cy="18" r="2"/><path d="M6 7l3.5 3M18 7l-3.5 3M6 17l3.5-3M18 17l-3.5-3"/>"#),
    ("lb", "Load balancer", "Network", r#"<circle cx="12" cy="5" r="2.5"/><path d="M12 7.5V14M5 18v-4h14v4M12 14v4"/><circle cx="5" cy="19.5" r="1.5"/><circle cx="12" cy="19.5" r="1.5"/><circle cx="19" cy="19.5" r="1.5"/>"#),
    ("gateway", "API gateway", "Network", r#"<path d="M3 12h5M16 12h5"/><rect x="8" y="6" width="8" height="12" rx="2"/><path d="M11 10l-1.5 2 1.5 2M13 10l1.5 2-1.5 2"/>"#),
    ("network", "VPC / Network", "Network", r#"<rect x="9" y="2" width="6" height="5" rx="1"/><rect x="2" y="17" width="6" height="5" rx="1"/><rect x="16" y="17" width="6" height="5" rx="1"/><path d="M12 7v5M5 17v-5h14v5"/>"#),
    ("api", "API", "Network", r#"<path d="M8 4c-2 0-3 1-3 3v2c0 1.5-1 3-2 3 1 0 2 1.5 2 3v2c0 2 1 3 3 3M16 4c2 0 3 1 3 3v2c0 1.5 1 3 2 3-1 0-2 1.5-2 3v2c0 2-1 3-3 3"/>"#),
    ("webhook", "Webhook", "Network", r#"<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>"#),
    ("nat", "NAT gateway", "Network", r#"<path d="M4 8h12l-3-3M20 16H8l3 3"/>"#),
    ("vpn", "VPN / Tunnel", "Network", r#"<path d="M3 20V11a9 9 0 0 1 18 0v9"/><path d="M8 20v-8a4 4 0 0 1 8 0v8"/>"#),
    ("edge", "Edge function", "Network", r#"<circle cx="12" cy="12" r="9"/><path d="M13 6l-4 7h4l-2 5"/>"#),
    ("proxy", "Reverse proxy", "Network", r#"<rect x="9" y="8" width="6" height="8" rx="1.5"/><path d="M2 12h7M15 12h7M5 9l-3 3 3 3M19 9l3 3-3 3"/>"#),
    // Compute
    ("server", "Server", "Compute", r#"<rect x="3" y="4" width="18" height="7" rx="2"/><rect x="3" y="13" width="18" height="7" rx="2"/><path d="M7 7.5h.01M7 16.5h.01"/>"#),
    ("service", "Service", "Compute", r#"<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M12 12l8-4.5M12 12v9M12 12L4 7.5"/>"#),
    ("container", "Container", "Compute", r#"<rect x="3" y="10" width="18" height="10" rx="2"/><path d="M7 10v10M12 10v10M17 10v10"/><rect x="8" y="4" width="8" height="6" rx="1"/>"#),
    ("k8s", "Kubernetes", "Compute", r#"<path d="M12 2.5l8.5 4.2 2 9.2-5.9 7.4H7.4L1.5 15.9l2-9.2z"/><circle cx="12" cy="12.5" r="3"/><path d="M12 6v3.5M12 15.5V19M6 10.5l3.2 1.2M18 10.5l-3.2 1.2M8 17.5l2.2-2.6M16 17.5l-2.2-2.6"/>"#),
    ("function", "Function", "Compute", r#"<rect x="3" y="3" width="18" height="18" rx="4"/><path d="M8 7h2l6 10M12 10.5L8 17"/>"#),
    ("worker", "Worker", "Compute", r#"<circle cx="12" cy="12" r="3"/><circle cx="12" cy="12" r="7"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1"/>"#),
    ("cpu", "Compute", "Compute", r#"<rect x="6" y="6" width="12" height="12" rx="2"/><rect x="9.5" y="9.5" width="5" height="5"/><path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4"/>"#),
    ("gpu", "GPU", "Compute", r#"<rect x="2" y="6" width="20" height="12" rx="2"/><circle cx="9" cy="12" r="3"/><circle cx="16" cy="12" r="2"/><path d="M6 18v2M10 18v2M14 18v2"/>"#),
    ("autoscale", "Auto scaling", "Compute", r#"<rect x="8" y="8" width="8" height="8" rx="1"/><path d="M3 3l4 4M21 3l-4 4M3 21l4-4M21 21l-4-4M3 6V3h3M21 6V3h-3M3 18v3h3M21 18v3h-3"/>"#),
    ("cloud", "Cloud", "Compute", r#"<path d="M7 18a5 5 0 1 1 1-9.9A6 6 0 0 1 19.5 10 4 4 0 0 1 18 18z"/>"#),
    ("rack", "Bare metal", "Compute", r#"<rect x="5" y="2" width="14" height="20" rx="2"/><path d="M5 8h14M5 14h14M8 5h.01M8 11h.01M8 17h.01"/>"#),
    ("building", "Data center", "Compute", r#"<rect x="4" y="3" width="16" height="18" rx="1"/><path d="M8 7h2M14 7h2M8 11h2M14 11h2M8 15h2M14 15h2M10 21v-3h4v3"/>"#),
    // Data
    ("database", "Database", "Data", r#"<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5"/><path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3"/>"#),
    ("cache", "Cache", "Data", r#"<path d="M13 2L4 14h7l-1 8 9-12h-7z"/>"#),
    ("bucket", "Object storage", "Data", r#"<path d="M4 6h16l-2 14H6z"/><ellipse cx="12" cy="6" rx="8" ry="2.5"/>"#),
    ("file", "File / Blob", "Data", r#"<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>"#),
    ("disk", "Block storage", "Data", r#"<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M7 14h.01M11 14h6"/>"#),
    ("archive", "Archive", "Data", r#"<rect x="3" y="4" width="18" height="5" rx="1"/><path d="M5 9v10a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V9M10 13h4"/>"#),
    ("table", "Key-value / NoSQL", "Data", r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 10h18M3 15h18M10 10v10"/>"#),
    ("graph", "Graph DB", "Data", r#"<circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="8" r="2.5"/><circle cx="9" cy="18" r="2.5"/><path d="M8.5 6.2l7 1.5M6.6 8.4l1.8 7.2M16.6 10.1l-6 6.1"/>"#),
    ("search", "Search index", "Data", r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4"/>"#),
    ("warehouse", "Analytics", "Data", r#"<path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/>"#),
    ("lake", "Data lake", "Data", r#"<rect x="2" y="4" width="20" height="16" rx="3"/><path d="M5 11c2.3-1.5 4.7-1.5 7 0s4.7 1.5 7 0M5 15.5c2.3-1.5 4.7-1.5 7 0s4.7 1.5 7 0"/>"#),
    ("pipeline", "Data pipeline", "Data", r#"<path d="M3 12h4l2-5 3 10 2-5h7"/>"#),
    ("notebook", "Notebook", "Data", r#"<rect x="5" y="3" width="14" height="18" rx="2"/><path d="M9 3v18M12 8h4M12 12h4"/>"#),
    // Messaging
    ("queue", "Queue", "Messaging", r#"<rect x="3" y="7" width="4" height="10" rx="1"/><rect x="10" y="7" width="4" height="10" rx="1"/><rect x="17" y="7" width="4" height="10" rx="1"/>"#),
    ("stream", "Stream / Topic", "Messaging", r#"<path d="M2 7c2.5-2 4.5-2 7 0s4.5 2 7 0 4.5-2 6 0M2 12c2.5-2 4.5-2 7 0s4.5 2 7 0 4.5-2 6 0M2 17c2.5-2 4.5-2 7 0s4.5 2 7 0 4.5-2 6 0"/>"#),
    ("pubsub", "Event bus", "Messaging", r#"<circle cx="12" cy="12" r="2"/><path d="M16.2 7.8a6 6 0 0 1 0 8.4M7.8 16.2a6 6 0 0 1 0-8.4M19.1 4.9a10 10 0 0 1 0 14.2M4.9 19.1a10 10 0 0 1 0-14.2"/>"#),
    ("workflow", "Workflow", "Messaging", r#"<rect x="3" y="3" width="6" height="6" rx="1.5"/><rect x="15" y="3" width="6" height="6" rx="1.5"/><rect x="15" y="15" width="6" height="6" rx="1.5"/><path d="M9 6h6M18 9v6M6 9v3a3 3 0 0 0 3 3h6"/>"#),
    ("email", "Email", "Messaging", r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 7l9 6 9-6"/>"#),
    ("bell", "Notifications", "Messaging", r#"<path d="M6 16V11a6 6 0 0 1 12 0v5l2 2H4z"/><path d="M10 21h4"/>"#),
    // Security
    ("shield", "Firewall / WAF", "Security", r#"<path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z"/>"#),
    ("firewall", "Firewall", "Security", r#"<rect x="3" y="4" width="18" height="16" rx="1"/><path d="M3 9.3h18M3 14.6h18M9 4v5.3M15 9.3v5.3M9 14.6V20"/>"#),
    ("lock", "Auth", "Security", r#"<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>"#),
    ("key", "Secrets", "Security", r#"<circle cx="8" cy="15" r="4"/><path d="M11 12l9-9M16 7l3 3M18 5l2 2"/>"#),
    ("cert", "Certificates", "Security", r#"<circle cx="12" cy="9" r="5"/><path d="M9 13.5L8 21l4-2 4 2-1-7.5"/>"#),
    ("card", "Payments", "Security", r#"<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M2 10h20M6 15h4"/>"#),
    // Ops
    ("chart", "Monitoring", "Ops", r#"<path d="M3 3v18h18"/><path d="M7 15l4-5 3 3 5-7"/>"#),
    ("dashboard", "Dashboard", "Ops", r#"<rect x="3" y="3" width="7" height="9" rx="1"/><rect x="14" y="3" width="7" height="5" rx="1"/><rect x="14" y="12" width="7" height="9" rx="1"/><rect x="3" y="16" width="7" height="5" rx="1"/>"#),
    ("logs", "Logs", "Ops", r#"<rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 8h8M8 12h8M8 16h5"/>"#),
    ("clock", "Scheduler", "Ops", r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>"#),
    ("git", "CI / CD", "Ops", r#"<circle cx="6" cy="6" r="2.5"/><circle cx="6" cy="18" r="2.5"/><circle cx="18" cy="8" r="2.5"/><path d="M6 8.5v7M18 10.5c0 4-6 3-11 5.5"/>"#),
    ("registry", "Registry", "Ops", r#"<rect x="4" y="4" width="16" height="16" rx="2"/><rect x="7" y="7" width="4" height="4"/><rect x="13" y="7" width="4" height="4"/><rect x="7" y="13" width="4" height="4"/><rect x="13" y="13" width="4" height="4"/>"#),
    // AI
    ("sparkles", "AI model", "AI", r#"<path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9z"/><path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z"/>"#),
    ("vector", "Vector DB", "AI", r#"<circle cx="5" cy="7" r="1.5"/><circle cx="12" cy="4" r="1.5"/><circle cx="19" cy="9" r="1.5"/><circle cx="8" cy="15" r="1.5"/><circle cx="16" cy="17" r="1.5"/><path d="M3 21h18"/>"#),
    ("agent", "AI agent", "AI", r#"<rect x="4" y="7" width="16" height="12" rx="3"/><path d="M12 3v4M9 12h.01M15 12h.01M9 16h6"/>"#),
    // glyph-only
    ("pin", "Region", "glyph", r#"<path d="M12 21s-7-6.2-7-12a7 7 0 0 1 14 0c0 5.8-7 12-7 12z"/><circle cx="12" cy="9" r="2.5"/>"#),
    ("layers", "Zone", "glyph", r#"<path d="M12 3l9 5-9 5-9-5z"/><path d="M3 13l9 5 9-5"/>"#),
    ("subnet", "Subnet", "glyph", r#"<rect x="3" y="3" width="18" height="18" rx="2" stroke-dasharray="3 2.5"/><rect x="8" y="8" width="8" height="8" rx="1"/>"#),
    ("stack", "Layers", "glyph", r#"<rect x="3" y="3.5" width="18" height="5" rx="1.5"/><rect x="3" y="9.5" width="18" height="5" rx="1.5"/><rect x="3" y="15.5" width="18" height="5" rx="1.5"/>"#),
    ("folder", "Account", "glyph", r#"<path d="M3 6a2 2 0 0 1 2-2h4l2 3h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#),
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
    ("t-sticky", "", "ui", r#"<path d="M4 4h16v11l-5 5H4z"/><path d="M15 20v-5h5"/>"#),
    ("t-comment", "", "ui", r#"<path d="M4 5h16v11H9l-5 4z"/>"#),
    ("t-library", "", "ui", r#"<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>"#),
    ("t-code", "", "ui", r#"<path d="M8 7l-5 5 5 5M16 7l5 5-5 5M14 4l-4 16"/>"#),
    ("menu", "", "ui", r#"<path d="M4 6h16M4 12h16M4 18h16"/>"#),
    ("undo", "", "ui", r#"<path d="M9 14L4 9l5-5"/><path d="M4 9h11a5 5 0 0 1 0 10h-3"/>"#),
    ("redo", "", "ui", r#"<path d="M15 14l5-5-5-5"/><path d="M20 9H9a5 5 0 0 0 0 10h3"/>"#),
    ("play", "", "ui", r#"<path d="M7 4l13 8-13 8z"/>"#),
    ("pause", "", "ui", r#"<path d="M8 4v16M16 4v16"/>"#),
    ("stop", "", "ui", r#"<rect x="6" y="6" width="12" height="12" rx="2"/>"#),
    ("record", "", "ui", r#"<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="3"/>"#),
    ("download", "", "ui", r#"<path d="M12 3v12M7 10l5 5 5-5M5 21h14"/>"#),
    ("trash", "", "ui", r#"<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>"#),
    ("copy", "", "ui", r#"<rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3"/>"#),
    ("front", "", "ui", r#"<path d="M12 19V5M5 12l7-7 7 7"/>"#),
    ("back", "", "ui", r#"<path d="M12 5v14M5 12l7 7 7-7"/>"#),
    ("plus", "", "ui", r#"<path d="M12 5v14M5 12h14"/>"#),
    ("minus", "", "ui", r#"<path d="M5 12h14"/>"#),
    ("x", "", "ui", r#"<path d="M6 6l12 12M18 6L6 18"/>"#),
    ("fit", "", "ui", r#"<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>"#),
    ("wand", "", "ui", r#"<path d="M4 20L16 8M14 4v3M19 9h3M18 5l2-2M11 4l1 1M19 13l1 1"/>"#),
    ("bolt", "", "ui", r#"<path d="M13 2L4 14h7l-1 8 9-12h-7z"/>"#),
    ("sun", "", "ui", r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>"#),
    ("moon", "", "ui", r#"<path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5z"/>"#),
    ("users", "", "ui", r#"<circle cx="9" cy="8" r="3.5"/><path d="M2 20v-1a5 5 0 0 1 5-5h4a5 5 0 0 1 5 5v1M16 4.5a3.5 3.5 0 0 1 0 7M22 20v-1a5 5 0 0 0-3.5-4.8"/>"#),
    ("film", "", "ui", r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 4v16M17 4v16M3 9h4M3 15h4M17 9h4M17 15h4"/>"#),
    ("alert", "", "ui", r#"<path d="M12 3l10 18H2z"/><path d="M12 10v5M12 18h.01"/>"#),
];

/// Cloud & OSS services: (id, label, category, glyph, subtitle).
const SERVICES: &[(&str, &str, &str, &str, &str)] = &[
    // AWS
    ("aws-ec2", "EC2", "AWS", "server", "AWS · Virtual machines"),
    ("aws-lambda", "Lambda", "AWS", "function", "AWS · Serverless functions"),
    ("aws-ecs", "ECS", "AWS", "container", "AWS · Containers"),
    ("aws-eks", "EKS", "AWS", "k8s", "AWS · Kubernetes"),
    ("aws-fargate", "Fargate", "AWS", "container", "AWS · Serverless containers"),
    ("aws-apprunner", "App Runner", "AWS", "service", "AWS · Managed apps"),
    ("aws-beanstalk", "Elastic Beanstalk", "AWS", "cloud", "AWS · PaaS"),
    ("aws-batch", "Batch", "AWS", "worker", "AWS · Batch jobs"),
    ("aws-asg", "Auto Scaling", "AWS", "autoscale", "AWS · Scaling group"),
    ("aws-outposts", "Outposts", "AWS", "rack", "AWS · On-prem racks"),
    ("aws-s3", "S3", "AWS", "bucket", "AWS · Object storage"),
    ("aws-ebs", "EBS", "AWS", "disk", "AWS · Block storage"),
    ("aws-efs", "EFS", "AWS", "file", "AWS · File storage"),
    ("aws-glacier", "S3 Glacier", "AWS", "archive", "AWS · Archive"),
    ("aws-rds", "RDS", "AWS", "database", "AWS · Relational DB"),
    ("aws-aurora", "Aurora", "AWS", "database", "AWS · Distributed SQL"),
    ("aws-dynamodb", "DynamoDB", "AWS", "table", "AWS · Key-value"),
    ("aws-documentdb", "DocumentDB", "AWS", "database", "AWS · Document DB"),
    ("aws-neptune", "Neptune", "AWS", "graph", "AWS · Graph DB"),
    ("aws-elasticache", "ElastiCache", "AWS", "cache", "AWS · Redis / Memcached"),
    ("aws-memorydb", "MemoryDB", "AWS", "cache", "AWS · Durable Redis"),
    ("aws-redshift", "Redshift", "AWS", "warehouse", "AWS · Data warehouse"),
    ("aws-opensearch", "OpenSearch", "AWS", "search", "AWS · Search"),
    ("aws-athena", "Athena", "AWS", "search", "AWS · SQL on S3"),
    ("aws-glue", "Glue", "AWS", "pipeline", "AWS · ETL"),
    ("aws-emr", "EMR", "AWS", "cpu", "AWS · Big data"),
    ("aws-lakeformation", "Lake Formation", "AWS", "lake", "AWS · Data lake"),
    ("aws-quicksight", "QuickSight", "AWS", "dashboard", "AWS · BI"),
    ("aws-kinesis", "Kinesis", "AWS", "stream", "AWS · Streaming"),
    ("aws-msk", "MSK", "AWS", "stream", "AWS · Managed Kafka"),
    ("aws-sqs", "SQS", "AWS", "queue", "AWS · Queue"),
    ("aws-sns", "SNS", "AWS", "bell", "AWS · Pub/sub"),
    ("aws-eventbridge", "EventBridge", "AWS", "pubsub", "AWS · Event bus"),
    ("aws-stepfunctions", "Step Functions", "AWS", "workflow", "AWS · Workflows"),
    ("aws-ses", "SES", "AWS", "email", "AWS · Email"),
    ("aws-apigw", "API Gateway", "AWS", "gateway", "AWS · APIs"),
    ("aws-appsync", "AppSync", "AWS", "api", "AWS · GraphQL"),
    ("aws-cloudfront", "CloudFront", "AWS", "cdn", "AWS · CDN"),
    ("aws-route53", "Route 53", "AWS", "dns", "AWS · DNS"),
    ("aws-elb", "Elastic Load Balancing", "AWS", "lb", "AWS · ALB / NLB"),
    ("aws-vpc", "VPC", "AWS", "network", "AWS · Network"),
    ("aws-natgw", "NAT Gateway", "AWS", "nat", "AWS · Egress"),
    ("aws-tgw", "Transit Gateway", "AWS", "network", "AWS · Network hub"),
    ("aws-dx", "Direct Connect", "AWS", "vpn", "AWS · Dedicated link"),
    ("aws-cognito", "Cognito", "AWS", "identity", "AWS · User auth"),
    ("aws-iam", "IAM", "AWS", "lock", "AWS · Access"),
    ("aws-kms", "KMS", "AWS", "key", "AWS · Keys"),
    ("aws-secrets", "Secrets Manager", "AWS", "key", "AWS · Secrets"),
    ("aws-acm", "Certificate Manager", "AWS", "cert", "AWS · TLS certs"),
    ("aws-waf", "WAF", "AWS", "shield", "AWS · Web firewall"),
    ("aws-shield", "Shield", "AWS", "shield", "AWS · DDoS"),
    ("aws-cloudwatch", "CloudWatch", "AWS", "chart", "AWS · Monitoring"),
    ("aws-cloudtrail", "CloudTrail", "AWS", "logs", "AWS · Audit logs"),
    ("aws-xray", "X-Ray", "AWS", "search", "AWS · Tracing"),
    ("aws-codepipeline", "CodePipeline", "AWS", "git", "AWS · CI/CD"),
    ("aws-ecr", "ECR", "AWS", "registry", "AWS · Container registry"),
    ("aws-sagemaker", "SageMaker", "AWS", "notebook", "AWS · ML platform"),
    ("aws-bedrock", "Bedrock", "AWS", "sparkles", "AWS · Foundation models"),
    ("aws-amplify", "Amplify", "AWS", "laptop", "AWS · Web & mobile"),
    ("aws-iot", "IoT Core", "AWS", "iot", "AWS · Devices"),
    // Google Cloud
    ("gcp-gce", "Compute Engine", "Google Cloud", "server", "GCP · Virtual machines"),
    ("gcp-run", "Cloud Run", "Google Cloud", "container", "GCP · Serverless containers"),
    ("gcp-functions", "Cloud Functions", "Google Cloud", "function", "GCP · Functions"),
    ("gcp-gke", "GKE", "Google Cloud", "k8s", "GCP · Kubernetes"),
    ("gcp-appengine", "App Engine", "Google Cloud", "cloud", "GCP · PaaS"),
    ("gcp-gcs", "Cloud Storage", "Google Cloud", "bucket", "GCP · Object storage"),
    ("gcp-pd", "Persistent Disk", "Google Cloud", "disk", "GCP · Block storage"),
    ("gcp-filestore", "Filestore", "Google Cloud", "file", "GCP · File storage"),
    ("gcp-sql", "Cloud SQL", "Google Cloud", "database", "GCP · Postgres / MySQL"),
    ("gcp-alloydb", "AlloyDB", "Google Cloud", "database", "GCP · Postgres-compatible"),
    ("gcp-spanner", "Spanner", "Google Cloud", "database", "GCP · Global SQL"),
    ("gcp-firestore", "Firestore", "Google Cloud", "table", "GCP · Document DB"),
    ("gcp-bigtable", "Bigtable", "Google Cloud", "table", "GCP · Wide-column"),
    ("gcp-memorystore", "Memorystore", "Google Cloud", "cache", "GCP · Redis"),
    ("gcp-bigquery", "BigQuery", "Google Cloud", "warehouse", "GCP · Data warehouse"),
    ("gcp-dataflow", "Dataflow", "Google Cloud", "pipeline", "GCP · Stream/batch"),
    ("gcp-dataproc", "Dataproc", "Google Cloud", "cpu", "GCP · Spark / Hadoop"),
    ("gcp-looker", "Looker", "Google Cloud", "dashboard", "GCP · BI"),
    ("gcp-pubsub", "Pub/Sub", "Google Cloud", "pubsub", "GCP · Messaging"),
    ("gcp-eventarc", "Eventarc", "Google Cloud", "pubsub", "GCP · Events"),
    ("gcp-tasks", "Cloud Tasks", "Google Cloud", "queue", "GCP · Task queue"),
    ("gcp-workflows", "Workflows", "Google Cloud", "workflow", "GCP · Orchestration"),
    ("gcp-scheduler", "Cloud Scheduler", "Google Cloud", "clock", "GCP · Cron"),
    ("gcp-apigee", "Apigee", "Google Cloud", "gateway", "GCP · API management"),
    ("gcp-cdn", "Cloud CDN", "Google Cloud", "cdn", "GCP · CDN"),
    ("gcp-dns", "Cloud DNS", "Google Cloud", "dns", "GCP · DNS"),
    ("gcp-lb", "Cloud Load Balancing", "Google Cloud", "lb", "GCP · Load balancer"),
    ("gcp-vpc", "VPC", "Google Cloud", "network", "GCP · Network"),
    ("gcp-nat", "Cloud NAT", "Google Cloud", "nat", "GCP · Egress"),
    ("gcp-interconnect", "Interconnect", "Google Cloud", "vpn", "GCP · Dedicated link"),
    ("gcp-armor", "Cloud Armor", "Google Cloud", "shield", "GCP · WAF / DDoS"),
    ("gcp-iam", "IAM", "Google Cloud", "lock", "GCP · Access"),
    ("gcp-identity", "Identity Platform", "Google Cloud", "identity", "GCP · User auth"),
    ("gcp-secret", "Secret Manager", "Google Cloud", "key", "GCP · Secrets"),
    ("gcp-kms", "Cloud KMS", "Google Cloud", "key", "GCP · Keys"),
    ("gcp-monitoring", "Cloud Monitoring", "Google Cloud", "chart", "GCP · Metrics"),
    ("gcp-logging", "Cloud Logging", "Google Cloud", "logs", "GCP · Logs"),
    ("gcp-build", "Cloud Build", "Google Cloud", "git", "GCP · CI/CD"),
    ("gcp-artifact", "Artifact Registry", "Google Cloud", "registry", "GCP · Registry"),
    ("gcp-vertex", "Vertex AI", "Google Cloud", "sparkles", "GCP · ML platform"),
    ("gcp-gemini", "Gemini API", "Google Cloud", "agent", "GCP · Foundation models"),
    ("gcp-firebase", "Firebase", "Google Cloud", "mobile", "GCP · App backend"),
    // Azure
    ("az-vm", "Virtual Machines", "Azure", "server", "Azure · VMs"),
    ("az-appservice", "App Service", "Azure", "cloud", "Azure · Web apps"),
    ("az-functions", "Functions", "Azure", "function", "Azure · Serverless"),
    ("az-aks", "AKS", "Azure", "k8s", "Azure · Kubernetes"),
    ("az-containerapps", "Container Apps", "Azure", "container", "Azure · Serverless containers"),
    ("az-aci", "Container Instances", "Azure", "container", "Azure · Containers"),
    ("az-vmss", "VM Scale Sets", "Azure", "autoscale", "Azure · Scaling"),
    ("az-blob", "Blob Storage", "Azure", "bucket", "Azure · Object storage"),
    ("az-files", "Azure Files", "Azure", "file", "Azure · File shares"),
    ("az-disk", "Managed Disks", "Azure", "disk", "Azure · Block storage"),
    ("az-sql", "SQL Database", "Azure", "database", "Azure · SQL"),
    ("az-postgres", "Database for PostgreSQL", "Azure", "database", "Azure · Postgres"),
    ("az-cosmos", "Cosmos DB", "Azure", "table", "Azure · Multi-model DB"),
    ("az-redis", "Cache for Redis", "Azure", "cache", "Azure · Redis"),
    ("az-synapse", "Synapse", "Azure", "warehouse", "Azure · Analytics"),
    ("az-fabric", "Fabric", "Azure", "lake", "Azure · Data platform"),
    ("az-datafactory", "Data Factory", "Azure", "pipeline", "Azure · ETL"),
    ("az-databricks", "Databricks", "Azure", "notebook", "Azure · Spark"),
    ("az-eventhubs", "Event Hubs", "Azure", "stream", "Azure · Streaming"),
    ("az-servicebus", "Service Bus", "Azure", "queue", "Azure · Messaging"),
    ("az-eventgrid", "Event Grid", "Azure", "pubsub", "Azure · Events"),
    ("az-logicapps", "Logic Apps", "Azure", "workflow", "Azure · Workflows"),
    ("az-apim", "API Management", "Azure", "gateway", "Azure · APIs"),
    ("az-frontdoor", "Front Door", "Azure", "cdn", "Azure · Global edge"),
    ("az-dns", "Azure DNS", "Azure", "dns", "Azure · DNS"),
    ("az-lb", "Load Balancer", "Azure", "lb", "Azure · L4"),
    ("az-appgw", "Application Gateway", "Azure", "lb", "Azure · L7 + WAF"),
    ("az-vnet", "Virtual Network", "Azure", "network", "Azure · Network"),
    ("az-expressroute", "ExpressRoute", "Azure", "vpn", "Azure · Dedicated link"),
    ("az-vpngw", "VPN Gateway", "Azure", "vpn", "Azure · VPN"),
    ("az-firewall", "Azure Firewall", "Azure", "firewall", "Azure · Firewall"),
    ("az-entra", "Entra ID", "Azure", "identity", "Azure · Identity"),
    ("az-keyvault", "Key Vault", "Azure", "key", "Azure · Secrets & keys"),
    ("az-monitor", "Monitor", "Azure", "chart", "Azure · Monitoring"),
    ("az-loganalytics", "Log Analytics", "Azure", "logs", "Azure · Logs"),
    ("az-appinsights", "App Insights", "Azure", "dashboard", "Azure · APM"),
    ("az-devops", "Azure DevOps", "Azure", "git", "Azure · CI/CD"),
    ("az-acr", "Container Registry", "Azure", "registry", "Azure · Registry"),
    ("az-openai", "Azure OpenAI", "Azure", "sparkles", "Azure · Foundation models"),
    ("az-aisearch", "AI Search", "Azure", "search", "Azure · Vector + text search"),
    ("az-staticweb", "Static Web Apps", "Azure", "browser", "Azure · Static hosting"),
    // Cloudflare
    ("cf-cdn", "Cloudflare CDN", "Cloudflare", "cdn", "Cloudflare · Edge cache"),
    ("cf-workers", "Workers", "Cloudflare", "edge", "Cloudflare · Edge functions"),
    ("cf-pages", "Pages", "Cloudflare", "browser", "Cloudflare · Static sites"),
    ("cf-r2", "R2", "Cloudflare", "bucket", "Cloudflare · Object storage"),
    ("cf-d1", "D1", "Cloudflare", "database", "Cloudflare · SQLite"),
    ("cf-kv", "Workers KV", "Cloudflare", "table", "Cloudflare · Key-value"),
    ("cf-do", "Durable Objects", "Cloudflare", "service", "Cloudflare · Stateful edge"),
    ("cf-queues", "Queues", "Cloudflare", "queue", "Cloudflare · Messaging"),
    ("cf-waf", "WAF", "Cloudflare", "shield", "Cloudflare · Web firewall"),
    ("cf-zerotrust", "Zero Trust", "Cloudflare", "lock", "Cloudflare · Access"),
    ("cf-tunnel", "Tunnel", "Cloudflare", "vpn", "Cloudflare · Private origin"),
    ("cf-dns", "DNS", "Cloudflare", "dns", "Cloudflare · DNS"),
    ("cf-ai", "Workers AI", "Cloudflare", "sparkles", "Cloudflare · Inference"),
    // Other clouds
    ("oci-compute", "OCI Compute", "Other clouds", "server", "Oracle Cloud · VMs"),
    ("oci-adb", "Autonomous DB", "Other clouds", "database", "Oracle Cloud · Database"),
    ("ali-ecs", "Alibaba ECS", "Other clouds", "server", "Alibaba Cloud · VMs"),
    ("ali-oss", "Alibaba OSS", "Other clouds", "bucket", "Alibaba Cloud · Object storage"),
    ("ibm-code", "IBM Code Engine", "Other clouds", "container", "IBM Cloud · Serverless"),
    ("do-droplet", "Droplet", "Other clouds", "server", "DigitalOcean · VM"),
    ("do-app", "App Platform", "Other clouds", "cloud", "DigitalOcean · PaaS"),
    ("do-spaces", "Spaces", "Other clouds", "bucket", "DigitalOcean · Object storage"),
    ("vercel-fn", "Vercel Functions", "Other clouds", "edge", "Vercel · Serverless"),
    ("netlify", "Netlify", "Other clouds", "browser", "Netlify · Web hosting"),
    ("fly-machine", "Fly Machines", "Other clouds", "container", "Fly.io · Micro-VMs"),
    ("render", "Render", "Other clouds", "cloud", "Render · PaaS"),
    ("heroku", "Heroku Dyno", "Other clouds", "container", "Heroku · PaaS"),
    ("supabase", "Supabase", "Other clouds", "database", "Supabase · Postgres backend"),
    ("snowflake", "Snowflake", "Other clouds", "warehouse", "Snowflake · Data cloud"),
    ("hetzner", "Hetzner", "Other clouds", "rack", "Hetzner · Servers"),
    // Open source
    ("oss-postgres", "PostgreSQL", "Open source", "database", "Relational DB"),
    ("oss-mysql", "MySQL", "Open source", "database", "Relational DB"),
    ("oss-mongodb", "MongoDB", "Open source", "table", "Document DB"),
    ("oss-redis", "Redis", "Open source", "cache", "In-memory store"),
    ("oss-cassandra", "Cassandra", "Open source", "table", "Wide-column"),
    ("oss-clickhouse", "ClickHouse", "Open source", "warehouse", "OLAP"),
    ("oss-elastic", "Elasticsearch", "Open source", "search", "Search"),
    ("oss-kafka", "Kafka", "Open source", "stream", "Event streaming"),
    ("oss-rabbitmq", "RabbitMQ", "Open source", "queue", "Message broker"),
    ("oss-nats", "NATS", "Open source", "pubsub", "Messaging"),
    ("oss-nginx", "Nginx", "Open source", "proxy", "Reverse proxy"),
    ("oss-envoy", "Envoy", "Open source", "proxy", "Service proxy"),
    ("oss-istio", "Istio", "Open source", "network", "Service mesh"),
    ("oss-docker", "Docker", "Open source", "container", "Containers"),
    ("oss-k8s", "Kubernetes", "Open source", "k8s", "Orchestration"),
    ("oss-prometheus", "Prometheus", "Open source", "chart", "Metrics"),
    ("oss-grafana", "Grafana", "Open source", "dashboard", "Dashboards"),
    ("oss-otel", "OpenTelemetry", "Open source", "logs", "Telemetry"),
    ("oss-vault", "Vault", "Open source", "key", "Secrets"),
    ("oss-keycloak", "Keycloak", "Open source", "identity", "Identity"),
    ("oss-airflow", "Airflow", "Open source", "workflow", "Orchestration"),
    ("oss-temporal", "Temporal", "Open source", "workflow", "Durable workflows"),
    ("oss-spark", "Spark", "Open source", "cpu", "Big data"),
    ("oss-minio", "MinIO", "Open source", "bucket", "Object storage"),
    ("oss-pgvector", "pgvector", "Open source", "vector", "Vector search"),
    ("oss-graphql", "GraphQL", "Open source", "api", "API"),
    ("oss-grpc", "gRPC", "Open source", "api", "RPC"),
];

/// Plain drawing shapes offered next to components: (id, label, glyph).
pub const SHAPES: &[(&str, &str, &str)] = &[
    ("shape-rect", "Rectangle", "t-rect"),
    ("shape-ellipse", "Ellipse", "t-ellipse"),
    ("shape-diamond", "Diamond", "t-diamond"),
    ("shape-sticky", "Sticky note", "t-sticky"),
    ("shape-text", "Text", "t-text"),
];

/// Boundary presets: (id, label, header glyph, color, dashed).
pub const BOUNDARIES: &[(&str, &str, &str, u32, bool)] = &[
    ("b-cloud", "Cloud", "cloud", 0x0284c7, true),
    ("b-onprem", "On-prem data center", "building", 0x475569, false),
    ("b-region", "Region", "pin", 0x6366f1, true),
    ("b-az", "Availability zone", "layers", 0x0d9488, true),
    ("b-vpc", "VPC / VNet", "network", 0x7c3aed, true),
    ("b-public", "Public subnet", "subnet", 0x16a34a, true),
    ("b-private", "Private subnet", "subnet", 0x2563eb, true),
    ("b-k8s", "Kubernetes cluster", "k8s", 0x326ce5, true),
    ("b-system", "System", "service", 0x9333ea, false),
    ("b-app", "Application layer", "stack", 0xea580c, true),
    ("b-dmz", "DMZ / Security zone", "shield", 0xdc2626, true),
    ("b-corp", "Corporate network", "building", 0xd97706, false),
    ("b-edge", "Edge / PoP", "globe", 0xdb2777, true),
    ("b-account", "Account / Project", "folder", 0x64748b, true),
];

pub fn boundary_color(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn glyph(id: &str) -> &'static str {
    GLYPHS.iter().find(|g| g.0 == id).map(|g| g.3).unwrap_or("")
}

pub fn all() -> &'static [Icon] {
    static ALL: OnceLock<Vec<Icon>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v: Vec<Icon> = GLYPHS.iter().map(|g| Icon { id: g.0, label: g.1, cat: g.2, svg: g.3, sub: "", color: false }).collect();
        v.extend(SHAPES.iter().map(|s| Icon { id: s.0, label: s.1, cat: "Shapes", svg: glyph(s.2), sub: "", color: false }));
        v.extend(BOUNDARIES.iter().map(|b| Icon { id: b.0, label: b.1, cat: "Boundaries", svg: glyph(b.2), sub: "", color: false }));
        v.extend(SERVICES.iter().map(|s| Icon { id: s.0, label: s.1, cat: s.2, svg: glyph(s.3), sub: s.4, color: false }));
        v.extend(load_custom());
        v
    })
}

/// Library categories in display order (built-ins, then imported folders).
pub fn categories() -> Vec<&'static str> {
    let mut c: Vec<&'static str> = CATEGORIES.to_vec();
    for i in all() {
        if !c.contains(&i.cat) && i.cat != "ui" && i.cat != "glyph" {
            c.push(i.cat);
        }
    }
    c
}

pub fn find(id: &str) -> Option<&'static Icon> {
    all().iter().find(|i| i.id == id)
}

/// Where users drop their own SVG icon packs (one sub-folder per category).
pub fn custom_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("Library/Application Support/idraw/icons")
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn load_custom() -> Vec<Icon> {
    // ponytail: loaded once at startup (leaked); restart idraw to pick up new packs.
    let mut out = vec![];
    let mut stack = vec![custom_dir()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")) {
                let Ok(svg) = std::fs::read_to_string(&p) else { continue };
                let cat = p.parent().and_then(|d| d.file_name()).map(|s| s.to_string_lossy().to_string()).unwrap_or("Imported".into());
                let cat = if cat == "icons" { "Imported".to_string() } else { cat };
                let label = p.file_stem().unwrap_or_default().to_string_lossy().replace(['_', '-'], " ");
                out.push(Icon { id: leak(format!("custom:{}", p.display())), label: leak(label), cat: leak(cat), svg: leak(svg), sub: "", color: true });
            }
        }
    }
    out.sort_by(|a, b| (a.cat, a.label).cmp(&(b.cat, b.label)));
    out
}

/// Standalone SVG document for a stroke glyph, in a given stroke color.
pub fn svg_doc(inner: &str, size: u32, color: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="{size}" height="{size}" fill="none" stroke="{color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{inner}</svg>"#
    )
}

pub fn base64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            s.push(if i <= c.len() { T[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    s
}

/// Icon textures. Stroke glyphs are white (tint at draw time); color icons draw with WHITE tint.
#[derive(Default)]
pub struct Icons {
    tex: HashMap<&'static str, Option<TextureHandle>>,
}

impl Icons {
    pub fn get(&mut self, ctx: &Context, id: &str) -> Option<TextureId> {
        let icon = find(id)?;
        if let Some(t) = self.tex.get(icon.id) {
            return t.as_ref().map(|t| t.id());
        }
        const PX: u32 = 72;
        let h = (|| {
            let doc = if icon.color { icon.svg.to_string() } else { svg_doc(icon.svg, PX, "#ffffff") };
            let tree = resvg::usvg::Tree::from_str(&doc, &Default::default()).ok()?;
            let s = tree.size();
            let k = PX as f32 / s.width().max(s.height());
            let mut pm = resvg::tiny_skia::Pixmap::new(PX, PX)?;
            let off = ((PX as f32 - s.width() * k) / 2.0, (PX as f32 - s.height() * k) / 2.0);
            resvg::render(&tree, resvg::tiny_skia::Transform::from_row(k, 0.0, 0.0, k, off.0, off.1), &mut pm.as_mut());
            let img = ColorImage::from_rgba_premultiplied([PX as usize, PX as usize], pm.data());
            Some(ctx.load_texture(icon.id, img, TextureOptions::LINEAR))
        })();
        let id = h.as_ref().map(|t| t.id());
        self.tex.insert(icon.id, h);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_has_a_glyph_and_unique_id() {
        for s in SERVICES {
            assert!(!glyph(s.3).is_empty(), "missing glyph {} for {}", s.3, s.0);
        }
        for b in BOUNDARIES {
            assert!(!glyph(b.2).is_empty(), "missing glyph for {}", b.0);
        }
        let mut ids: Vec<&str> = all().iter().map(|i| i.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(n, ids.len(), "duplicate icon ids");
    }

    #[test]
    fn base64_matches_reference() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }
}
