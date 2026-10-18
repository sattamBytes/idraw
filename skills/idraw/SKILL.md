---
name: idraw
description: Draw diagrams as idraw files (.idraw) — system design, architecture, request flows, data pipelines, org/process maps — and render them to PNG/SVG/GIF to look at the result. Use when the user asks to draw, diagram, sketch, visualise or "make an idraw" of something, or to edit an existing .idraw file.
---

# idraw

idraw is a diagram app. A `.idraw` file is JSON. You write it by hand, render it to an image, **look at the image**, and fix what looks wrong. Repeat until it looks like something a thoughtful designer made.

## The command

```bash
idraw render diagram.idraw            # → diagram.png (2x). Also: -o out.svg | out.gif, --dark, --4k
idraw check diagram.idraw             # broken ids, unknown icons, overlaps, text that won't fit
idraw icons kafka                     # find icon ids: idraw icons aws queue / idraw icons database
```

If `idraw` isn't on PATH, use `/Applications/idraw.app/Contents/MacOS/idraw` or `~/Applications/idraw.app/Contents/MacOS/idraw`. If neither exists, tell the user to install it: `brew install --cask sattamBytes/tap/idraw`.

## The loop (always do this)

1. **Plan the story before coordinates.** What should someone understand in 5 seconds? Pick a layout that tells it (see below). Sketch the grid in your head: columns, rows, which boundary holds what.
2. **Write the file.** Use string ids (`"id": "api"`) and connect by id. Start from `examples/` in this skill if one is close (each has a rendered `.png` next to it).
3. **`idraw check`** and fix every error. Treat overlap / cut-off warnings as real.
4. **`idraw render`, then open the PNG with your image-reading tool and actually look.** Ask:
   - Can I follow the main path without hunting? Do lines cross or tangle?
   - Is anything cramped, cut off, or floating with no reason?
   - Is spacing even? Do columns and rows line up?
   - Does colour mean something, or is it noise?
5. **Fix and re-render.** At least two passes. Stop when it reads instantly. Common fixes:
   - A line loops over the top or detours → the two shapes are slightly off-row/off-column. Give them the *same* `y` (or `x`), and pin sides: `"from": {"id": "api", "side": "right"}`.
   - Two lines leave the same shape and tangle → pin them to different sides (`right` for one, `bottom` for the other).
   - A label sits on an arrowhead or another label → move it along the line with `"label_t": 0.3` / `0.75`.
   - Big empty area → shrink the boundary or move a callout (sticky) into it.
6. Tell the user the file path and show (or attach) the PNG. Mention they can open the file in idraw to tweak it and play the animations.

Never hand over a diagram you have not rendered and looked at.

## Writing the file — essentials

Full field list: [reference.md](reference.md). The minimum:

```json
{
  "elements": [
    { "kind": "region", "preset": "vpc", "title": "Production VPC", "x": 280, "y": 40, "w": 760, "h": 420 },
    { "kind": "node", "id": "users", "icon": "user", "title": "Users", "subtitle": "web + mobile", "x": 40, "y": 220 },
    { "kind": "node", "id": "api", "icon": "gateway", "title": "API Gateway", "subtitle": "REST · auth", "x": 340, "y": 220 },
    { "kind": "node", "id": "db", "icon": "oss-postgres", "title": "Postgres", "subtitle": "primary", "x": 760, "y": 220 },
    { "kind": "edge", "from": "users", "to": "api", "text": "HTTPS", "flow": "request" },
    { "kind": "edge", "from": "api", "to": "db", "text": "SQL" }
  ]
}
```

- `x, y` = top-left. Components (`node`) are 176×56 unless you give `w, h`.
- Kinds: `node` (icon card), `rect`, `ellipse`, `diamond`, `text`, `sticky`, `region` (boundary), `edge`.
- Edges route themselves around shapes (elbow by default). Give `"route": "straight"` or `"curve"` when that reads better.
- Colours are `"#rrggbb"`. Style fields go straight on the element: `stroke`, `fill`, `text_color`, `width`, `dashed`, `pattern`, `face`, `font`, `flow`, `route`.

## Layouts that work

Pick one per diagram and commit to it:

- **Left → right journey** (request flows, pipelines): one row per tier or lane, columns ~300 apart.
- **Top → down layers** (client / edge / services / data): rows ~140 apart, boundaries per layer.
- **Hub and spoke** (event bus, shared DB): hub in the middle, consumers on a circle or a grid around it.
- **Zones** (trust boundaries, cloud vs on-prem, regions): nested boundaries; the boundary *is* the message.
- **Before / after** or **A vs B**: two mirrored halves with a divider and titles.

Spacing that looks designed: snap to a 20px grid; 100–140px gaps between components side by side, 60–80px between rows; 40px padding inside boundaries (60 at the top for the title). Keep columns aligned on the same x.

## Making it creative (not just correct)

- **Title + one-line takeaway** as `text` at the top (`"font": "xl"` then `"font": "m"`, muted colour).
- **Animate by meaning, not decoration** (`"flow"` on an edge). Each one tells the viewer how the systems talk:
  - `request` — synchronous call: the request goes out, the caller waits, a hollow reply comes back (UML call + reply). User → API, service → database.
  - `async` — fire-and-forget messages (little envelopes), several in flight, no reply. Service → queue, webhooks.
  - `stream` — steady data (events, logs, CDC, replication).
  - `batch` — bursts with pauses: nightly ETL, bulk sync, backups.
  - `fanout` — pub/sub: put it on *every* edge leaving a topic/bus; they all get a copy at once.
  - `balanced` — round robin: put it on *every* edge leaving a load balancer; they take turns.
  - `heartbeat` — health checks / keep-alives: a small ping and an "ok" ripple.
  - `errors` — some requests fail part-way (a red ✕ drops off): flaky dependency, rate limits, timeouts.
  Animate the paths that matter for the story, leave the rest still. Make the main path pop with a stronger colour or `"width": 3`; make side paths `"pattern": "dotted"` or grey.
- **Meaningful colour**: one accent per concern (e.g. blue = sync, orange = async, red = failure path, green = cache hit). Add a small legend (sticky or text) when you use more than two meanings.
- **Explain, don't just box**: `sticky` notes for callouts ("p99 < 50ms here"), `notes` on a component for the details, edge labels for protocols and rates (`"2.4k rps"` thickens and speeds the line).
- **Show behaviour**: `replicas` for scaled services, `"failover": true` on backup paths, a `scenarios` walkthrough for "what happens when a user does X".
- **Typeface as voice**: `"face": "hand"` for an informal whiteboard feel, `"mono"` for code-ish things, `"serif"` for a polished title. Or set `"sketch": true` at the top level for an all hand-drawn look.
- Vary shapes on purpose: `diamond` for decisions, `ellipse` for start/end, `rect` with `fill` for plain steps, `node` for real infrastructure.

## Editing an existing file

Read it, keep ids and positions of things you don't change, edit, then check → render → look as above. Native files (written by the app) use numeric ids and `rect: {min, max}` — both forms can be mixed.
