//! Command line: `idraw render | check | icons`, for scripts and AI agents.
//! Anything else (no args, or a file path) opens the app.

use crate::export::{self, Look, Quality};
use crate::icons;
use crate::lenient;
use crate::model::*;
use crate::render;
use eframe::egui::{Pos2, Rect, vec2};
use std::path::PathBuf;

const HELP: &str = "idraw — diagrams for system design

  idraw [file.idraw]                     open the app
  idraw render <file> [-o out] [opts]    draw to .png (default), .svg or .gif
        --dark   dark theme    --4k   3840px wide    --1x   screen size (default 2x)
        --failures   show the outage simulation
  idraw check <file>                     problems with exact fixes: bad ids, unknown icons, overlaps,
                                         misaligned rows, detours, crossings, hidden labels…
  idraw icons [search…]                  list icon ids for components (\"icon\": \"aws-lambda\")
  idraw skill install [--project]        add the idraw skill to Claude Code, Cursor, Codex,
                                         Gemini CLI, Antigravity and VS Code Copilot
";

/// The agent skill, built into the binary so `idraw skill install` works from Homebrew too.
const SKILL_FILES: [(&str, &[u8]); 6] = [
    ("SKILL.md", include_bytes!("../skills/idraw/SKILL.md")),
    ("reference.md", include_bytes!("../skills/idraw/reference.md")),
    ("examples/url-shortener.idraw", include_bytes!("../skills/idraw/examples/url-shortener.idraw")),
    ("examples/url-shortener.png", include_bytes!("../skills/idraw/examples/url-shortener.png")),
    ("examples/flow-meanings.idraw", include_bytes!("../skills/idraw/examples/flow-meanings.idraw")),
    ("examples/flow-meanings.png", include_bytes!("../skills/idraw/examples/flow-meanings.png")),
];

/// Runs a CLI command if one was given; returns the exit code, or None to open the app.
pub fn run() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first()?.as_str();
    let rest = &args[1..];
    Some(match cmd {
        "render" => report(render(rest)),
        "check" => report(check(rest)),
        "skill" | "skills" => report(skill(rest)),
        "icons" => {
            icons_list(rest);
            0
        }
        "help" | "-h" | "--help" => {
            print!("{HELP}");
            0
        }
        // a file to open in the app (or something the app can import)
        _ if std::path::Path::new(cmd).exists() || cmd.starts_with('-') => return None,
        // a typo shouldn't silently launch the app
        _ => {
            eprintln!("idraw: unknown command or missing file \"{cmd}\". Commands: render, check, icons, skill. Run idraw --help");
            2
        }
    })
}

fn report(r: Result<bool, String>) -> i32 {
    match r {
        Ok(ok) => i32::from(!ok),
        Err(e) => {
            eprintln!("error: {e}");
            2
        }
    }
}

fn load(rest: &[String]) -> Result<(PathBuf, Doc, Vec<String>), String> {
    let file = rest.iter().find(|a| !a.starts_with('-')).ok_or("which file? e.g. idraw check diagram.idraw")?;
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let (mut doc, warn) = lenient::parse(&text).map_err(|e| format!("{file}: {e}"))?;
    doc.fit_nodes();
    Ok((PathBuf::from(file), doc, warn))
}

fn render(rest: &[String]) -> Result<bool, String> {
    let (src, doc, warn) = load(rest)?;
    if doc.elements.is_empty() {
        return Err("nothing to draw".into());
    }
    let flag = |f: &str| rest.iter().any(|a| a == f);
    let out = match rest.iter().position(|a| a == "-o") {
        Some(i) => PathBuf::from(rest.get(i + 1).ok_or("-o needs a path")?),
        None => src.with_extension("png"),
    };
    let q = if flag("--4k") {
        Quality::UltraHd
    } else if flag("--1x") {
        Quality::Standard
    } else {
        Quality::Retina
    };
    let look = Look { dark: flag("--dark"), failures: flag("--failures") };
    match out.extension().and_then(|e| e.to_str()).unwrap_or("png") {
        "svg" => export::write_svg(&doc, &[], look, &out)?,
        "gif" => export::write_gif(&doc, &[], q, look, &out)?,
        _ => export::write_png(&doc, &[], q, look, &out)?,
    }
    for w in &warn {
        eprintln!("warning: {w}");
    }
    println!("{}", out.display());
    Ok(true)
}

/// Copies the skill into each agent tool's skills folder (only tools that are installed).
fn skill(rest: &[String]) -> Result<bool, String> {
    if rest.first().map(String::as_str) != Some("install") {
        return Err("usage: idraw skill install [--project]".into());
    }
    let home = PathBuf::from(std::env::var("HOME").map_err(|_| "HOME is not set")?);
    // (folder that must exist, skills folder, tools that read it)
    let targets: Vec<(PathBuf, PathBuf, &str)> = if rest.iter().any(|a| a == "--project") {
        vec![
            (".".into(), ".agents/skills".into(), "Cursor, Codex, Gemini CLI, Antigravity, VS Code (this project)"),
            (".".into(), ".claude/skills".into(), "Claude Code (this project)"),
        ]
    } else {
        let h = |p: &str| home.join(p);
        vec![
            (h(".claude"), h(".claude/skills"), "Claude Code"),
            (h(".cursor"), h(".cursor/skills"), "Cursor"),
            (home.clone(), h(".agents/skills"), "Codex, Gemini CLI and other Agent Skills tools"),
            (h(".gemini"), h(".gemini/skills"), "Gemini CLI"),
            (h(".gemini/config"), h(".gemini/config/skills"), "Antigravity"),
            (h(".gemini/antigravity-cli"), h(".gemini/antigravity-cli/skills"), "Antigravity CLI"),
            (h(".copilot"), h(".copilot/skills"), "VS Code / GitHub Copilot"),
        ]
    };
    for (needs, dir, who) in targets {
        if !needs.is_dir() {
            continue;
        }
        let dest = dir.join("idraw");
        for (name, bytes) in SKILL_FILES {
            let p = dest.join(name);
            std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| format!("{}: {e}", p.display()))?;
            std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
        }
        println!("✓ {:<48} {who}", dest.display());
    }
    println!("Restart your agent, then ask it to \"draw an idraw diagram of …\".");
    Ok(true)
}

fn icons_list(rest: &[String]) {
    let words: Vec<String> = rest.iter().map(|w| w.to_lowercase()).collect();
    let mut n = 0;
    for i in icons::all().iter().filter(|i| i.cat != "ui") {
        let hay = format!("{} {} {} {}", i.id, i.label, i.cat, i.sub).to_lowercase();
        if words.iter().all(|w| hay.contains(w.as_str())) {
            let sub = if i.sub.is_empty() { String::new() } else { format!(" · {}", i.sub) };
            println!("{:<26} {}{sub}  [{}]", i.id, i.label, i.cat);
            n += 1;
        }
    }
    if n == 0 {
        eprintln!("no icons match; try a shorter word");
    }
}

/// Problems a person would spot at a glance. Returns false when something is broken (not just untidy).
fn check(rest: &[String]) -> Result<bool, String> {
    let (_, doc, warn) = load(rest)?;
    let mut errors: Vec<String> = vec![];
    let mut tips: Vec<String> = warn;
    let name = name_of;
    let boxes: Vec<&Element> = doc.elements.iter().filter(|e| e.bindable()).collect();
    let regions: Vec<&Element> = doc.elements.iter().filter(|e| e.kind == Kind::Region).collect();

    for e in &doc.elements {
        if e.kind == Kind::Node && !e.icon.is_empty() && icons::find(&e.icon).is_none() {
            errors.push(format!("{}: unknown icon \"{}\" (see `idraw icons {}`)", name(e), e.icon, e.icon.split('-').next_back().unwrap_or("")));
        }
        if e.kind == Kind::Arrow {
            for (end, which) in [(&e.from, "from"), (&e.to, "to")] {
                match end.id {
                    Some(id) if doc.get(id).is_none() => errors.push(format!("{}: \"{which}\" points at missing element #{id}", name(e))),
                    Some(id) if !doc.get(id).unwrap().bindable() => errors.push(format!("{}: \"{which}\" can't attach to {}", name(e), name(doc.get(id).unwrap()))),
                    None => tips.push(format!("{}: \"{which}\" isn't attached to anything", name(e))),
                    _ => {}
                }
            }
        }
        // rough text fit (labels inside their shape)
        if matches!(e.kind, Kind::Rect | Kind::Ellipse | Kind::Diamond | Kind::Sticky) && !e.outside {
            let long = e.text.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f32;
            let room = e.rect.width() * if e.kind == Kind::Rect || e.kind == Kind::Sticky { 0.9 } else { 0.7 };
            if long * e.style.font * 0.55 > room {
                tips.push(format!("{}: text is probably wider than the shape; widen it to ~{:.0} or break the line", name(e), long * e.style.font * 0.55 / 0.85));
            }
        }
    }
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            if doc.bounds(a).shrink(2.0).intersects(doc.bounds(b).shrink(2.0)) {
                tips.push(format!("{} overlaps {}", name(a), name(b)));
            }
        }
    }
    for r in &regions {
        for e in doc.elements.iter().filter(|e| e.id != r.id && !matches!(e.kind, Kind::Arrow | Kind::Pen | Kind::Comment)) {
            if straddles(e.rect, r.rect) {
                tips.push(format!("{} sticks out of boundary {}", name(e), name(r)));
            }
        }
    }
    let layout = layout_issues(&doc);
    for e in &errors {
        println!("error: {e}");
    }
    for t in &tips {
        println!("warning: {t}");
    }
    for l in &layout {
        println!("layout: {l}");
    }
    let arrows = doc.elements.iter().filter(|e| e.kind == Kind::Arrow).count();
    let size = doc.all_bounds().map_or("empty".into(), |b| format!("{:.0}×{:.0}", b.width(), b.height()));
    println!("{} elements ({} connectors), {size}: {} errors, {} warnings, {} layout issues", doc.elements.len(), arrows, errors.len(), tips.len(), layout.len());
    Ok(errors.is_empty())
}

fn name_of(e: &Element) -> String {
    let t = e.text.lines().next().unwrap_or("").trim();
    let k = if e.key.is_empty() { format!("#{}", e.id) } else { format!("\"{}\"", e.key) };
    if t.is_empty() { format!("{:?} {k}", e.kind) } else { format!("{:?} {k} \"{t}\"", e.kind) }
}

/// Short name for a box: its id, else its title.
fn short(e: &Element) -> String {
    if !e.key.is_empty() {
        return e.key.clone();
    }
    let t = e.text.lines().next().unwrap_or("").trim();
    if t.is_empty() { format!("#{}", e.id) } else { t.to_string() }
}

/// Things that make a diagram look messy, each with the exact change that fixes it.
/// The agent (or person) applies the fix; nothing is moved automatically.
fn layout_issues(doc: &Doc) -> Vec<String> {
    let mut out = vec![];
    let boxes: Vec<&Element> = doc.elements.iter().filter(|e| e.bindable() && !matches!(e.kind, Kind::Text)).collect();
    let solid: Vec<&Element> = boxes.iter().copied().filter(|e| e.kind != Kind::Sticky).collect();
    let arrows: Vec<&Element> = doc.elements.iter().filter(|e| e.kind == Kind::Arrow).collect();
    let end = |id: Option<u64>| id.and_then(|i| doc.get(i)).filter(|e| e.bindable());
    let arrow_name = |a: &Element| {
        let n = |id: Option<u64>| end(id).map_or("(loose end)".to_string(), short);
        let label = if a.text.is_empty() { String::new() } else { format!(" \"{}\"", a.text.lines().next().unwrap_or("")) };
        format!("line {} → {}{label}", n(a.from.id), n(a.to.id))
    };
    let paths: Vec<Vec<Pos2>> = arrows.iter().map(|a| doc.arrow_path(a)).collect();

    // 1. connected shapes that are almost, but not quite, on the same row / column
    for a in &arrows {
        let (Some(f), Some(t)) = (end(a.from.id), end(a.to.id)) else { continue };
        let d = t.rect.center() - f.rect.center();
        if d.x.abs() >= d.y.abs() && d.y.abs() > 0.5 && d.y.abs() <= 24.0 {
            out.push(format!("\"{}\" and \"{}\" are {:.0}px off the same row, which bends the line between them: set y of \"{}\" to {:.0}", short(f), short(t), d.y.abs(), short(t), f.rect.center().y - t.rect.height() / 2.0));
        } else if d.y.abs() > d.x.abs() && d.x.abs() > 0.5 && d.x.abs() <= 24.0 {
            out.push(format!("\"{}\" and \"{}\" are {:.0}px off the same column, which bends the line between them: set x of \"{}\" to {:.0}", short(f), short(t), d.x.abs(), short(t), f.rect.center().x - t.rect.width() / 2.0));
        }
    }

    for (a, p) in arrows.iter().zip(&paths) {
        if p.len() < 2 {
            continue;
        }
        let (s, e) = (p[0], *p.last().unwrap());
        let len = path_len(p);
        // 2. detours: the line loops around something instead of going straight
        let direct = (e.x - s.x).abs() + (e.y - s.y).abs();
        if len > direct * 1.5 + 60.0 {
            let d = e - s;
            let (fs, ts) = if d.x.abs() >= d.y.abs() { if d.x > 0.0 { ("right", "left") } else { ("left", "right") } } else if d.y > 0.0 { ("bottom", "top") } else { ("top", "bottom") };
            let pin = |id: Option<u64>, side: &str| end(id).map_or(String::new(), |b| format!("{{\"id\": \"{}\", \"side\": \"{side}\"}}", short(b)));
            out.push(format!("{} detours ({:.0}px long for a {:.0}px gap): line the two shapes up, or pin sides with \"from\": {} and \"to\": {}", arrow_name(a), len, direct, pin(a.from.id, fs), pin(a.to.id, ts)));
        }
        // 2b. a line leaving (or entering) from a side that faces away from where it's going
        // (a side pinned by hand is deliberate, e.g. to route around something)
        if let (Some(f), Some(t), None, None) = (end(a.from.id), end(a.to.id), a.from.side, a.to.side) {
            let d = t.rect.center() - f.rect.center();
            let horizontal = d.x.abs() > d.y.abs() * 2.0;
            let vertical = d.y.abs() > d.x.abs() * 2.0;
            let seg_dir = |a: Pos2, b: Pos2| if (b.x - a.x).abs() > (b.y - a.y).abs() { "h" } else { "v" };
            let first = p.windows(2).find(|w| w[0].distance(w[1]) > 1.0).map(|w| seg_dir(w[0], w[1]));
            let last = p.windows(2).rev().find(|w| w[0].distance(w[1]) > 1.0).map(|w| seg_dir(w[0], w[1]));
            let (want, fs, ts) = if horizontal {
                ("h", if d.x > 0.0 { "right" } else { "left" }, if d.x > 0.0 { "left" } else { "right" })
            } else if vertical {
                ("v", if d.y > 0.0 { "bottom" } else { "top" }, if d.y > 0.0 { "top" } else { "bottom" })
            } else {
                ("", "", "")
            };
            if !want.is_empty() && (first != Some(want) || last != Some(want)) && len <= direct * 1.5 + 60.0 {
                out.push(format!("{} leaves or enters from the wrong side (\"{}\" is to the {fs} of \"{}\"): pin it with \"from\": {{\"id\": \"{}\", \"side\": \"{fs}\"}} and \"to\": {{\"id\": \"{}\", \"side\": \"{ts}\"}}", arrow_name(a), short(t), short(f), short(f), short(t)));
            }
        }
        // 3. a line running across a box it doesn't connect to
        let hit = solid.iter().find(|b| {
            Some(b.id) != a.from.id && Some(b.id) != a.to.id && {
                let r = doc.bounds(b).shrink(6.0);
                (0..=(len / 6.0) as usize).any(|i| r.contains(point_at(p, i as f32 * 6.0)))
            }
        });
        if let Some(b) = hit {
            out.push(format!("{} runs over \"{}\": move \"{}\" out of the way, or pin the line to another side", arrow_name(a), short(b), short(b)));
        }
        // 4. labels: on boxes, on other labels, on the arrowhead, or too long for the line
        if !a.text.is_empty() {
            let size = render::label_size(a);
            let lines: Vec<&str> = a.text.lines().collect();
            let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * size * 0.55 + 14.0;
            let lr = Rect::from_center_size(render::arrow_label_center(p, a), vec2(w, lines.len() as f32 * size * LINE_H + 4.0));
            let t = a.label_t;
            let other_t = if t > 0.5 { 0.3 } else { 0.7 };
            if len < w + 4.0 {
                out.push(format!("{} is shorter ({:.0}px) than its label ({:.0}px), so the label hides it: move the shapes about {:.0}px further apart, or shorten the label", arrow_name(a), len, w, w + 40.0 - len));
            } else if let Some(b) = boxes.iter().find(|b| lr.intersects(doc.bounds(b).shrink(2.0))) {
                out.push(format!("label of {} sits on \"{}\": set \"label_t\": {other_t} (now {t})", arrow_name(a), short(b)));
            } else if lr.expand(4.0).contains(e) && a.style.head_end {
                out.push(format!("label of {} sits on its arrowhead: set \"label_t\": {other_t} (now {t})", arrow_name(a)));
            }
        }
    }
    // label on label
    let labels: Vec<(usize, Rect)> = arrows
        .iter()
        .zip(&paths)
        .enumerate()
        .filter(|(_, (a, p))| !a.text.is_empty() && p.len() >= 2)
        .map(|(i, (a, p))| {
            let size = render::label_size(a);
            let w = a.text.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * size * 0.55 + 14.0;
            (i, Rect::from_center_size(render::arrow_label_center(p, a), vec2(w, a.text.lines().count() as f32 * size * LINE_H + 4.0)))
        })
        .collect();
    for (k, (i, r)) in labels.iter().enumerate() {
        for (j, q) in &labels[k + 1..] {
            if r.intersects(*q) {
                out.push(format!("labels of {} and {} overlap: give one \"label_t\": 0.3 and the other 0.7", arrow_name(arrows[*i]), arrow_name(arrows[*j])));
            }
        }
    }

    // 5. lines that cross each other
    for i in 0..arrows.len() {
        for j in i + 1..arrows.len() {
            // lines fanning out of (or into) the same shape bundle together; that's fine
            let shares = |x: Option<u64>| x.is_some() && (x == arrows[j].from.id || x == arrows[j].to.id);
            if shares(arrows[i].from.id) || shares(arrows[i].to.id) {
                continue;
            }
            if let Some(c) = path_crossings(&paths[i], &paths[j]).first() {
                out.push(format!("{} crosses {} at ({:.0}, {:.0}): move a shape or pin a different side so they don't cross", arrow_name(arrows[i]), arrow_name(arrows[j]), c.x, c.y));
            }
        }
    }

    // 6. shapes squeezed together
    for (i, a) in solid.iter().enumerate() {
        for b in &solid[i + 1..] {
            let (ra, rb) = (doc.bounds(a), doc.bounds(b));
            let gx = (rb.min.x - ra.max.x).max(ra.min.x - rb.max.x);
            let gy = (rb.min.y - ra.max.y).max(ra.min.y - rb.max.y);
            let gap = if gx > 0.0 && gy <= 0.0 { gx } else if gy > 0.0 && gx <= 0.0 { gy } else { continue };
            if gap < 16.0 {
                out.push(format!("\"{}\" and \"{}\" are only {gap:.0}px apart: leave at least 40px", short(a), short(b)));
            }
        }
    }
    out
}

/// Partly inside, partly outside.
fn straddles(e: Rect, r: Rect) -> bool {
    e.intersects(r) && !r.expand(1.0).contains_rect(e) && !e.contains_rect(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issues(src: &str) -> Vec<String> {
        layout_issues(&lenient::parse(src).unwrap().0)
    }

    #[test]
    fn layout_check_names_problems_and_fixes() {
        // B sits 12px below A's row; C is below A but the line has no pinned sides
        let messy = r#"[
            {"kind": "node", "id": "a", "x": 0, "y": 0},
            {"kind": "node", "id": "b", "x": 300, "y": 12},
            {"kind": "edge", "from": "a", "to": "b"}
        ]"#;
        let got = issues(messy);
        assert!(got.iter().any(|m| m.contains("12px off the same row") && m.contains("set y of \"b\" to 0")), "{got:?}");
        let clean = r#"[
            {"kind": "node", "id": "a", "x": 0, "y": 0},
            {"kind": "node", "id": "b", "x": 300, "y": 0},
            {"kind": "node", "id": "c", "x": 0, "y": 200},
            {"kind": "edge", "from": "a", "to": "b", "text": "call"},
            {"kind": "edge", "from": "a", "to": "c"}
        ]"#;
        assert!(issues(clean).is_empty(), "{:?}", issues(clean));
    }
}
