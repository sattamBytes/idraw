# .idraw format reference

A file is `{ "elements": [...], "scenarios": [...], "sketch": false, "jumps": "hop" }`. Only `elements` is required. A bare `[...]` list of elements also works. Unknown fields are ignored with a warning from `idraw check`.

Elements are drawn in list order (later = on top), except boundaries (`region`) which always go behind.

## Every element

| field | meaning |
|---|---|
| `kind` | `node`, `rect`, `ellipse`, `diamond`, `text`, `sticky`, `region`, `edge` (aliases: box, circle, boundary/group, arrow/connector, note) |
| `id` | any string (`"api"`) or number. Needed if something connects to it. |
| `x`, `y`, `w`, `h` | top-left and size. Or `"rect": {"x","y","w","h"}` / `[x, y, w, h]`. Defaults: node, rect, ellipse and diamond 176×56 (so they line up), region 480×320, sticky 180×140 |
| `text` | label; `\n` for a new line |
| `title`, `subtitle` | for nodes: card title and grey second line (same as `text: "Title\nSub"`) |
| `stroke` (or `color`) | outline / accent colour `#rrggbb` |
| `fill` | background `#rrggbb`, `#rrggbbaa` or `"none"` |
| `text_color` | label colour (default: same as stroke; dark ink for nodes and stickies) |
| `width` | line thickness, 1–4 (default 2) |
| `dashed` | `true` for a dashed outline / line |
| `font` | `"s"`, `"m"`, `"l"`, `"xl"` or a pixel size (13, 18, 24, 36) |
| `face` | `"sans"` (clean), `"hand"`, `"mono"`, `"serif"`; default follows sketch mode |
| `tpos` | label position: `top_left`, `top`, `top_right`, `left`, `center`, `right`, `bottom_left`, `bottom`, `bottom_right` |
| `outside` | `true` puts the label outside the shape (at `tpos`) |
| `icon` | icon id (`idraw icons <word>`); on rect/ellipse/diamond/sticky it sits inside, on a region it's the header icon |
| `icon_pos` | where the icon sits inside a shape (same values as `tpos`) |
| `round` | `false` for sharp rect corners |

## Components (`node`)

An icon tile + title + subtitle card. Set `icon`. Ids are prefixed by provider: `aws-`, `gcp-`, `az-`, `oss-` (open source), plus generic ones (e.g. `aws-lambda`, `gcp-bigquery`, `az-cosmos`, `oss-kafka`, `oss-postgres`, `oss-redis`, `user`, `mobile`, `browser`, `gateway`, `lb`, `cdn`, `queue`). If you leave out `title`, the icon's own name is used.

Beyond infrastructure there are icons for logic, AI and ideas, handy for flowcharts, agent designs and product diagrams:
- **Logic**: `decision`, `decision-tree`, `router`, `if-else`, `switch`, `merge`, `split`, `filter`, `loop`, `rules`, `gate`, `toggle` (feature flag), `ab-test`, `timeout`, `gauge` (threshold), `state` (state machine), `dice` (random)
- **AI**: `sparkles` (model), `brain` (reasoning), `agent`, `prompt`, `tokens`, `temperature`, `sliders` (parameters), `embedding`, `vector`, `rag`, `neural`, `guardrail`, `eval`, `memory`, `tool`, `chain`, `classifier`, `chat`, `vision`, `voice`, `image-gen`, `fine-tune`
- **Ideas**: `idea`, `palette` (design), `magic` (automation), `target` (goal), `rocket` (launch), `puzzle` (integration), `team`, `milestone`, `book` (docs) Long titles and subtitles wrap, and the card grows taller to fit; give `w` (e.g. 220) to keep it on fewer lines.

Extra fields:

| field | meaning |
|---|---|
| `replicas` | number ≥ 2 shows a ×N badge and stacked cards |
| `notes` | longer explanation (hover in the app) |
| `show_notes` | `true` prints the notes under the card |
| `down` | `true` marks it failed (shown when rendered with `--failures`) |

## Boundaries (`region`)

Big labelled zones: cloud, VPC, subnet, cluster, team, layer. Title goes top-left by default.

- `"preset"`: `cloud`, `onprem`, `region`, `az`, `vpc`, `public`, `private`, `k8s`, `system`, `app`, `dmz`, `corp`, `edge`, `account`. Each sets colour, icon, dash and a default title (override with `title`).
- Or style it yourself: `stroke` (fill becomes a faint tint automatically), `dashed`, `icon`.
- Nest boundaries freely (inner ones later in the list). Keep children fully inside; `check` warns when something sticks out.

## Connectors (`edge`)

| field | meaning |
|---|---|
| `stroke` | line colour; connectors are Sky (`#0369a1`) unless you set one |
| `from`, `to` | an element id, `{"id": "api", "side": "right"}` to pin the side (`top`/`right`/`bottom`/`left`), or `[x, y]` for a loose end |
| `text` | label on the line. A rate like `"1.2k rps"` makes the line thicker and faster |
| `route` | `elbow` (default, avoids shapes), `straight`, `curve` |
| `pattern` | `solid`, `dotted`, `dash_dot`, `double` (or `dashed: true`) |
| `flow` | animation by meaning: `request` (sync call + hollow reply), `async` (one-way envelopes), `stream` (steady packets), `batch` (bursts), `fanout` (pub/sub: all sibling edges at once), `balanced` (round robin: sibling edges take turns), `heartbeat` (ping + ok ripple), `errors` (some fail and drop off), `dashes` (marching), `none` |
| `speed` | `slow`, `normal`, `fast` |
| `head_start`, `head_end` | arrowheads (default: end only). Both `true` = two-way |
| `failover` | `true` = backup path: faint until something is `down` |
| `label_t` | where the label sits along the line, 0–1 (default 0.5) |

Animations show in the app and in `.gif` renders; a `.png` shows one frame.

## Text, stickies

- `text`: free text; size it with `font`. Good for titles, captions, legends.
- `sticky`: a yellow-ish note card (`fill` to recolour). Good for callouts.

## Scenarios (step-by-step walkthroughs)

```json
"scenarios": [
  { "name": "User places an order", "steps": [
    { "from": "users", "to": "api", "edge": "e1", "caption": "POST /orders" },
    { "from": "api", "to": "db", "edge": "e2", "caption": "write order row" }
  ] }
]
```

Each step travels along an existing connector (`edge` = that connector's id). Played in the app with the ▶ button; render one with idraw's Export → scenario GIF.

## Top level

| field | meaning |
|---|---|
| `sketch` | `true` = hand-drawn strokes and handwriting font everywhere |
| `jumps` | where lines cross: `hop` (default), `gap`, `off` |

## Colour palette that matches the app

Strokes: `#1f2937` ink · `#64748b` slate · `#dc2626` red · `#c2410c` orange · `#b45309` amber · `#15803d` green · `#0f766e` teal · `#0369a1` sky · `#2563eb` blue · `#6366f1` indigo · `#7c3aed` violet · `#db2777` pink.
Soft fills: `#ffffff` white · `#f1f5f9` mist · `#ffe4e6` rose · `#ffedd5` peach · `#fef9c3` butter · `#dcfce7` mint · `#ccfbf1` aqua · `#e0f2fe` sky · `#e0e7ff` periwinkle · `#ede9fe` lavender · `#fce7f3` blush.
