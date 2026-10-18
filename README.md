<p align="center"><img src="docs/icon.png" width="128" alt="idraw icon"></p>

<h1 align="center">idraw</h1>

<p align="center"><b>Diagrams that move.</b> A fast, open-source diagramming app for macOS, built for system design.<br>
Excalidraw's simplicity, draw.io's depth, plus animated request flows you can export as GIFs.</p>

<p align="center">
  <a href="https://github.com/sattamBytes/idraw/releases/latest"><img src="https://img.shields.io/github/v/release/sattamBytes/idraw?label=download&color=6366f1" alt="Download"></a>
  <img src="https://img.shields.io/badge/macOS-11%2B-black?logo=apple" alt="macOS 11+">
  <img src="https://img.shields.io/badge/Apple%20Silicon%20%2B%20Intel-universal-555" alt="Universal">
  <img src="https://img.shields.io/badge/built%20with-Rust-dea584?logo=rust" alt="Rust">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-22c55e" alt="MIT"></a>
</p>

<p align="center"><img src="docs/flow.gif" alt="A request flow playing through a hybrid-cloud diagram" width="820"></p>

## Install

**Homebrew**

```bash
brew install --cask sattamBytes/tap/idraw
```

**Or download** the latest `idraw-…-macos-universal.zip` from [Releases](https://github.com/sattamBytes/idraw/releases/latest), unzip it, and drag **idraw.app** into Applications.

> **First launch:** idraw isn't notarized by Apple yet, so macOS may say it "can't be opened". Right-click idraw.app and choose **Open**, then **Open** again. You only have to do this once. Homebrew installs skip this step.

**Or build from source** (Rust 1.85+):

```bash
git clone https://github.com/sattamBytes/idraw && cd idraw
scripts/bundle.sh --install   # builds idraw.app into ~/Applications
```

<p align="center"><img src="docs/hero.png" alt="Hybrid-cloud system design diagram made in idraw" width="820"></p>

## Features

**Draw**
- Infinite canvas: trackpad pan, pinch or ⌘-scroll to zoom, dot grid, alignment guides
- Shapes, text, freehand pen, **sticky notes**, **comments**, dashed group regions
- Labels go anywhere: a 3×3 position picker, inside or outside the shape, and they glide into place when moved
- **Clean** or **sketch** (hand-drawn, hachure fills) style, in **light** or **dark** theme

**System design**
- 250+ components: generic infra, plus **AWS, Google Cloud, Azure, Cloudflare**, other clouds and open-source services, all drawn as consistent glyphs
- **Boundary presets**: Cloud, On-prem data center, Region, AZ, VPC, public/private subnet, K8s cluster, DMZ, corporate network, edge, account
- **Bring your own icons**: drop SVG packs (e.g. the official AWS/Azure/GCP sets, under their own licences) into `~/Library/Application Support/idraw/icons/<Category>/`
- **Smart connectors** route around other shapes, snap to sides, and use elbow, straight or curved lines with label pills
- **Quick-connect**: drag from a shape's blue anchor onto empty canvas and pick what to create
- **Notes** on components (shown on hover, or under the card), **replicas** (×N stacked cards) and **capacity math** (QPS, payload, retention → bandwidth, storage, $/month)

**Animate & tell the story**
- **Flow animation** on connectors (packets or marching dashes), at Slow / Normal / Fast speed
- **Traffic-weighted edges**: label a connector "1.2k rps" and it gets thicker and faster
- **Scenarios**: record a request path by clicking components, add captions, then play it back with a travelling packet and pulses, or export it as a GIF
- **Outage simulation** (F): click components to take them down. Their connections stop, and **backup paths** take over

**Speed**
- **⌘K command palette**: commands, components, and jump to any element
- **/** inserts a component at the cursor
- **Auto-layout** (⇧⌘L or menu): animated layered layout that refits regions
- **Pen** with a one-click **Clear all drawings**
- **Diagram as code** (⌘/): a text pane with two-way sync. Paste **Mermaid** to convert it
- **Import** (⌘I): docker-compose, Kubernetes YAML, Terraform, Mermaid

**Share**
- Export **PNG / SVG / animated GIF** at 1×, 2× or **4K** (3840px)
- **Live collaboration**, peer-to-peer with no server: host on your network (or Tailscale) and share the address and code. Live cursors included
- Autosaves your last canvas. `.idraw` files are plain JSON

### Diagram as code

Write one connection per line, using plain names:

```text
Users -> API Gateway -> Orders service
Orders service -> Postgres : SQL        # text after ":" is the label
Orders service => Kafka                 # => animated, --> dashed, <-> both ways
Cache                                   # a component on its own
Payments [aws-lambda] (card processing) # optional [icon] and (subtitle)
```

Names like S3, Lambda, BigQuery or Postgres get their icon automatically. Renaming a line renames the component.

## Development

```bash
cargo run --release -- examples/hybrid-cloud.idraw   # run with a sample diagram
cargo test                                            # unit tests
scripts/bundle.sh --universal                         # universal idraw.app + release zip in dist/
```

Releases are built by GitHub Actions: push a tag like `v0.2.0` and the universal zip is attached to the GitHub release.

## Shortcuts

| Key | Action | Key | Action |
|---|---|---|---|
| V / H | select / hand | ⌘Z / ⇧⌘Z | undo / redo |
| R / O / D | rect / ellipse / diamond | ⌘D | duplicate |
| S / C | sticky note / comment | ⌘K | command palette |
| F | outage simulation | ⌘/ | diagram as code |
| / | insert component at cursor | ⇧⌘L | auto-layout |
| ⇧⌘D | dark mode | ⇧⌘K | sketch style |
| A | connector | ⌘G | wrap selection in region |
| T / P / G | text / pen / region | ⌘S / ⌘O | save / open |
| L | toggle library | ⌘E / ⇧⌘E | export PNG / GIF |
| Space-drag | pan | ⇧1 | zoom to fit |
| double-click | edit label / new text | ⌘ while dragging | disable snapping |

## Contributing

Issues and pull requests are welcome. Ideas, bugs and icon packs especially.

## License

MIT
