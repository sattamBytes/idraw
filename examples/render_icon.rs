//! Renders assets/icon.svg to a transparent 1024px PNG (used by scripts/bundle.sh).
fn main() {
    let out = std::env::args().nth(1).unwrap_or("icon.png".into());
    let svg = std::fs::read_to_string("assets/icon.svg").expect("assets/icon.svg");
    let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).expect("valid svg");
    let mut pm = resvg::tiny_skia::Pixmap::new(1024, 1024).unwrap();
    resvg::render(&tree, Default::default(), &mut pm.as_mut());
    pm.save_png(&out).expect("write png");
}
