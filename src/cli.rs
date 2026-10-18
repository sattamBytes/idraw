//! Command line: `idraw render | check | icons`, for scripts and AI agents.
//! Anything else (no args, or a file path) opens the app.

use crate::export::{self, Look, Quality};
use crate::icons;
use crate::lenient;
use crate::model::*;
use eframe::egui::Rect;
use std::path::PathBuf;

const HELP: &str = "idraw — diagrams for system design

  idraw [file.idraw]                     open the app
  idraw render <file> [-o out] [opts]    draw to .png (default), .svg or .gif
        --dark   dark theme    --4k   3840px wide    --1x   screen size (default 2x)
        --failures   show the outage simulation
  idraw check <file>                     report problems: bad ids, overlaps, overflowing text…
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
        "skill" => report(skill(rest)),
        "icons" => {
            icons_list(rest);
            0
        }
        "help" | "-h" | "--help" => {
            print!("{HELP}");
            0
        }
        _ => return None,
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
    let name = |e: &Element| {
        let t = e.text.lines().next().unwrap_or("").trim();
        let k = if e.key.is_empty() { format!("#{}", e.id) } else { format!("\"{}\"", e.key) };
        if t.is_empty() { format!("{:?} {k}", e.kind) } else { format!("{:?} {k} \"{t}\"", e.kind) }
    };
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
    for e in &errors {
        println!("error: {e}");
    }
    for t in &tips {
        println!("warning: {t}");
    }
    let arrows = doc.elements.iter().filter(|e| e.kind == Kind::Arrow).count();
    let size = doc.all_bounds().map_or("empty".into(), |b| format!("{:.0}×{:.0}", b.width(), b.height()));
    println!("{} elements ({} connectors), {size}: {} errors, {} warnings", doc.elements.len(), arrows, errors.len(), tips.len());
    Ok(errors.is_empty())
}

/// Partly inside, partly outside.
fn straddles(e: Rect, r: Rect) -> bool {
    e.intersects(r) && !r.expand(1.0).contains_rect(e) && !e.contains_rect(r)
}
