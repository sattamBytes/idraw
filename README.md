# idraw

A fast, modern, open-source diagramming app for macOS, written in Rust.
It has Excalidraw's simplicity and draw.io's depth, and it's built first for **system design** diagrams.


## Features (v0.1)

- Infinite canvas: trackpad pan, pinch or ⌘-scroll to zoom, dot grid
- Shapes: rectangle, ellipse, diamond, text, freehand pen, dashed **group regions** (VPC, cluster, …)
- **System-design components**: a searchable library of consistent icons (clients, network, compute, data, messaging, security, ops, AI). Click or drag to add.
- **Smart connectors**: snap to shapes and stay attached, rounded elbow, straight or curved routes, label pills, arrowheads
- **Quick-connect**: hover a shape, drag from a blue anchor, and drop on empty canvas to spawn a connected twin
- **Flow animation** on connectors (packets or marching dashes)
- **Export** PNG (2×), SVG, and **animated GIF** that loops the flow animation seamlessly
- Smart alignment guides, undo/redo, copy/paste (JSON on the clipboard), duplicate, z-order
- Autosaves your last canvas; `.idraw` files are plain JSON

## Run

```bash
cargo run --release -- examples/system-design.idraw
```

## Shortcuts

| Key | Action | Key | Action |
|---|---|---|---|
| V / H | select / hand | ⌘Z / ⇧⌘Z | undo / redo |
| R / O / D | rect / ellipse / diamond | ⌘D | duplicate |
| A | connector | ⌘G | wrap selection in region |
| T / P / G | text / pen / region | ⌘S / ⌘O | save / open |
| L | toggle library | ⌘E / ⇧⌘E | export PNG / GIF |
| Space-drag | pan | ⇧1 | zoom to fit |
| double-click | edit label / new text | ⌘ while dragging | disable snapping |

## License

MIT
