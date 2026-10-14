# 08 — Proposed AniCraft Architecture

> **Status: PROPOSAL — not yet approved.** This is the research-backed architecture. It follows the
> verified Craft conventions (doc 01), reuses shared infrastructure (doc 02) and Ruffle `swf`
> (doc 07), and is organized to keep the **authoring document**, the **deterministic evaluator**,
> the **renderer**, and the **interactive runtime** cleanly separated (doc 03, 05).
>
> Components are classified: **[E]** exists in a Craft repo (reuse by dependency/pattern),
> **[A]** adapt/extend an existing piece, **[N]** new in AniCraft, **[D]** decision needed.

## 1. Application shell and workspace

Mirror the sibling shape exactly:

```
anicraft/
├── Cargo.toml                 # workspace: edition 2024, resolver 3, [workspace.dependencies], [lints]
├── crates/
│   ├── geom/                  # anicraft-geom      (L0) kurbo wrapper: Point/Vec2/Affine/Rect, twips<->px
│   ├── color/                 # anicraft-color     (L0) sRGB/linear, alpha, blend modes
│   ├── time/                  # anicraft-time      (L0) REUSE effectcraft-time (Tick, FrameRate) — [E]
│   ├── keyframe/              # anicraft-keyframe  (L1) REUSE effectcraft-keyframe + CreateJS easing — [A]
│   ├── doc/                   # anicraft-doc       (L1) THE document model (stage/scenes/timelines/symbols) — [N]
│   ├── shapes/                # anicraft-shapes    (L2) vector primitives: paths/fills/strokes/edges, morph — [N]
│   ├── text/                  # anicraft-text      (L2) skrifa+harfrust shaping, point/area text — [A]
│   ├── timeline/              # anicraft-timeline  (L3) deterministic evaluate() → DisplayList — [N]
│   ├── render/                # anicraft-render    (L3) DisplayList→pixels (vello_cpu CPU ref) — [A]
│   ├── gpu/                   # anicraft-gpu       (L3, later) wgpu compositor (parity-tested) — [A][D]
│   ├── tools/                 # anicraft-tools     (L4) tool state machines (Action) — [A]
│   ├── engine/                # anicraft-engine    (L5) Session, commands, undo, file I/O, guard — [A]
│   ├── script/                # anicraft-script    (L5) boa + Flash object model (__query/__exec) — [A]
│   ├── swf/                   # anicraft-swf       (L3) SWF I/O over Ruffle `swf` — [A]
│   ├── xfl/                   # anicraft-xfl       (L3) XFL I/O (new, from XSD) — [N]
│   ├── svg/                   # anicraft-svg       (L3) SVG export of a frame — [A]
│   ├── export/                # anicraft-export    (L3) image-seq/GIF/video/sprite-sheet/Lottie — [A]
│   ├── ui-egui/               # anicraft-ui-egui   (L6) egui frontend: stage, timeline, library, panels — [A]
│   ├── mcp/                   # anicraft-mcp       (L6) MCP server (JSON-RPC stdio) — [A]
│   └── testkit/               # anicraft-testkit   (dev) fixtures, generators, golden render — [A]
├── apps/
│   ├── anicraft/            # L7 desktop app (eframe/wgpu, cpal, rfd, craft-quit)
│   ├── anicraft-cli/        # L7 headless (run, convert, export, render, bench, play, mcp)
│   └── anicraft-web/        # L7 browser (wasm32, WebGPU/WebGL2)
└── xtask/                     # layers, wasm, ci, corpus
```

- **Reuse by git/crates.io pin** in `[workspace.dependencies]`: `craft-ui`, `craft-settings`,
  `craft-atomic`, `craft-print`, `craft-quit` (craft-libs); **`swf`** (Ruffle); **`effectcraft-time`**
  and **`effectcraft-keyframe`** (EffectCraft); `kurbo`, `peniko`, `vello_cpu`, `usvg`, `skrifa`,
  `harfrust`, `boa_engine` (crates.io); optionally **FilmCraft codec/export crates** (git) for
  video. A commented `[patch]` block for local co-development (EffectCraft's FilmCraft pattern).
- **`[lints]`** inherit the no-panic / no-unsafe workspace rules; every `lib.rs` starts with the
  deny-list. `cargo xtask layers` enforces the layer table; `cargo xtask wasm` checks
  `wasm32-unknown-unknown` for L0–L5 + ui-egui + web.

## 2. Layering (enforced by `cargo xtask layers`)

| Layer | Crates | Role | Depends on UI? |
|-------|--------|------|----------------|
| L0 | `geom`, `color`, `time` | foundations (kurbo; `time` = reused Tick) | no |
| L1 | `doc`, `keyframe` | document model (pure data+serde, `Arc` sharing); keyframe model | no |
| L2 | `shapes`, `text` | vector primitives/morph; text shaping | no |
| L3 | `timeline`, `render`, `gpu`, `swf`, `xfl`, `svg`, `export` | deterministic eval; pixels; all format I/O | no |
| L4 | `tools` | pointer events → `Action` + overlays | no |
| L5 | `engine`, `script` | Session, commands, undo, file I/O, guard; scripting | no |
| L6 | `ui-egui`, `mcp` | egui frontend; MCP server | **yes** (egui/eframe) |
| L7 | `apps/*`, `xtask` | desktop/CLI/web; build tooling | yes |

Allowed same-layer edges: `render → timeline`, `export → render`, `timeline → render` (if the
display list needs bounds), `script → engine`, `gpu → render`. **Nothing below L6 depends on
`egui`/`eframe`/`winit`/`rfd`/`cpal`/`muda`.** (Same shape as VectorCraft/EffectCraft/FilmCraft,
doc 01 §2.2.)

## 3. Component breakdown (by concern)

### 3.1 Document and scene model — `anicraft-doc` **[N]** (L1)
The core new work (doc 03 §4). Pure data + serde, `Arc` structural sharing (VectorCraft pattern):

```
Document { version, title, settings: DocSettings, scenes: Vec<Scene>, library: Library, next_id }
DocSettings { stage: SizePx, frame_rate: FrameRate, background: Color, … }
Scene { id, name, timeline: Timeline }
Timeline { layers: Vec<Layer>, … }                 // layer 0 = bottom
Layer { id, name, color, visible, locked, guide,
        kind: Normal | Folder | Mask | Masked, parent: Option<LayerId>,
        frames: Vec<Frame> }
Frame { index, duration, key: KeyKind (Key|Blank|Span),
        content: Option<FrameContent>, tween: Option<Tween>, script: Option<ScriptRef> }
FrameContent { items: Vec<Placed> }                // z-order bottom→top
Placed { kind: Shape | Text | SymbolInstance, transform, … }
SymbolInstance { def: SymbolId, matrix, color: ColorTransform, blend, name,
                 filters: Vec<Filter>, playback: Playback (loop, first_frame), … }
Library { symbols: Map<SymbolId, SymbolDef>, media: Map<MediaId, Media> }
SymbolDef { id, name, kind:
    Graphic { timeline: Timeline }            // plays with parent timeline
  | MovieClip { timeline: Timeline, default_loop }  // independent timeline
  | Button { up, over, down, hit: FrameContent, … }
  | Bitmap | Sound | Font | Shape | Video }
```

- **Symbols are shared definitions with independent instances** (doc 03 §4.2): a `SymbolDef` is
  `Arc`-shared in the `Library`; a `SymbolInstance` holds a `SymbolId` + its own
  `matrix`/`color`/`name`/`playback`. Editing a definition updates every instance; instance
  properties stay independent. (Extends VectorCraft's simple `Symbol { name, art }`.)
- **Undo:** `Arc<Document>` snapshots; edits copy only the path to the edited node (VectorCraft
  `doc` pattern). A frame-span/blank-keyframe/tween edit copies only the affected layer.
- **Invariants** (`Document::check()`): unique ids, no duplicate frame indices, mask layers have a
  masked layer below, folder parents valid, instance `def` ids exist. Engine runs it after every
  edit (FilmCraft `Sequence::check` pattern).

### 3.2 Vector shapes — `anicraft-shapes` **[N]** (L2)
The authoring vector primitive (doc 03 §2, doc 04 §1):
- `Shape { bounds, fills: Vec<FillStyle>, strokes: Vec<LineStyle>, edges: Vec<Edge> }` where
  `FillStyle = Color | LinearGradient | RadialGradient | FocalGradient | Bitmap { … }` and an
  `Edge` is a cubic-Bézier run with left/right fill indices + stroke — the **same edge/fill model
  as SWF** (so `swf::Shape` maps losslessly) and the **shared-edge** model XFL uses.
- Built on kurbo `BezPath`/`Affine`; boolean ops (union/subtract/intersect) via a boolean
  library (VectorCraft `pathops` is the reference; a new thin port is acceptable) — **[A][D]**
  (pick the boolean backend).
- **Morph** (`DefineMorphShape`/XFL `tweenType="shape"` model): two `Shape` forms with matched
  edges/fills; a `MorphDef { from, to }`. The **interpolation algorithm** (vertex/edge/fill
  matching + ratio `u`) is **[N]** (doc 10 risk).

### 3.3 Text — `anicraft-text` **[A]** (L2)
skrifa (font access) + harfrust (shaping: kerning, bidi, complex scripts) + `craft-fonts` (via
`CRAFT_FONTS_DIR`). Point and area text, static vs editable, render as vector glyph outlines or
outline to paths. (VectorCraft/EffectCraft/FilmCraft all do this; reuse the pattern.)

### 3.4 Timeline evaluation — `anicraft-timeline` **[N]** (L3) — *the determinism core*
`evaluate(doc: &Document, t: Tick, opts: EvalOpts) -> DisplayList`, a **pure function** (doc 03
§4.4):
- For the active scene at `t`: for each layer (respecting `kind` — guide layers are skipped for
  rendering, a `Mask` layer produces a clip for the `Masked` layer(s) below, `Folder` groups
  children), resolve the frame covering `t` (following **spans** to the last keyframe).
- For each placed item: a `Shape`/`Text` is emitted directly; a `SymbolInstance` **recurses**
  into its definition's timeline at the correct local time — **Graphic** follows the parent time,
  **MovieClip** its own time (`playback.first_frame`, loop mode) — and emits its content with the
  instance `matrix`/`color`/`blend`/`filters` composed in.
- **Tweens:** a motion tween interpolates the instance `matrix`/`color`/`filters` between
  keyframes via `anicraft-keyframe` (CreateJS easing); a shape tween interpolates the two
  `Shape` forms via the morph algorithm.
- Output: a flattened `DisplayList` of `DrawItem`s (each a shape/text/bitmap with an effective
  `Affine`, paint, blend, opacity, filter chain, and bounds), in draw order. **No stored
  per-frame state** — always derived. This is the Flash analogue of how Ruffle reconstructs the
  display list from tags each frame (doc 03 §2).

### 3.5 Rendering — `anicraft-render` **[A]** (L3)
`DisplayList → Pixmap` on **`vello_cpu`** (CPU reference), per doc 04:
- Build a peniko scene from the display list; **cull by the viewport** view transform; apply
  fills/strokes/gradients/text/images; **clip groups** from mask layers.
- **Caches:** per (symbol id, local frame) and per (layer, frame) content hash (the Flash analogue
  of EffectCraft's layer cache) — static content renders once; scrubbing reuses.
- **Filters** (blur, drop shadow, glow, gradient glow, bevel) as offscreen `vello_cpu` passes,
  cached.
- **Onion skinning** = `evaluate` at `t±k` rendered at low alpha under the current frame.
- **`anicraft-gpu`** (later, **[D]**): a wgpu compositor implementing the same steps,
  **parity-tested** against the CPU (≤ 1/255 at 8 bpc), `backend = Cpu | Gpu | Auto` (the
  EffectCraft/FilmCraft pattern).

### 3.6 Editing operations — `anicraft-tools` **[A]** (L4)
Tool state machines emit `Action` (`Begin`/`Preview`/`Commit`/`Cancel`/`Exec`/`Dialog`/
`SwitchTool`/`Notify`) that the engine executes (VectorCraft `tools` pattern, doc 01 §2.5). The
Flash tool set (doc 04 §5): selection/subselection/lasso, pen, line/rect/oval/poly-star, text,
paint bucket/ink bottle, eyedropper, eraser, free transform, gradient transform, hand/zoom.
Transformation is an `Affine` on the selection/instance; grouping creates a `Group`; hit-testing
is against the current frame's display list (uniform with rendering).

### 3.7 Engine — `anicraft-engine` **[A]** (L5)
`Session { doc: Arc<Document>, editor: EditorState, history: History, commands: Registry, … }`:
- **Command registry** (`CommandSpec { id, label, menu, shortcut, params, enabled, run, journal }`);
  `Session::execute(id, params)` and `Session::edit(label, |doc, state| …)`. The menu bar, command
  palette, control channel, CLI, and MCP all dispatch the same ids. Command ids follow the
  structure: `file.*`, `edit.*`, `timeline.*`, `symbol.*`, `layer.*`, `frame.*`, `tool.*`,
  `export.*`, `script.*`, `window.*`, `help.*`.
- **Undo/redo** = `Arc<Document>` snapshot swaps (constant time); modified-state follows the last
  saved snapshot (EffectCraft).
- **`guard`** catches a panic at every entry point and rolls the document back (VectorCraft).
- **File I/O:** the `FORMATS` table (`.anicraft`, XFL, SWF, SVG, GIF/APNG/PNG-seq, video,
  sprite-sheet, Lottie); atomic save via `craft-atomic`; backward-readable versioning.
- **Autosave/Data Recovery** in the background; **events** drained by frontends each frame.
- **Control channel** (`--control <port>`, JSON lines, pointer/keyboard injection + screenshots)
  and **MCP** (doc 01 §2.3).

### 3.8 Scripting / interactive runtime — `anicraft-script` **[A]** (L5)
Per doc 05: boa (`boa_engine`) with a `prelude.js` Flash object model (Stage/MovieClip/Sprite/
DisplayObject/Timeline/Library/Button/Text) and a `runtime.rs` `__query`/`__exec` bridge +
thread-local session + undo groups + sandbox (fresh context per run, file-scoped reads,
writes/network behind a pref, stack retry). The **runtime** (authoring-time = engine commands,
undoable; playback-time = runtime state over `evaluate()`'s display list with the frame phases
enterFrame→construct→frameScripts→exitFrame, mirroring Ruffle).

### 3.9 Import/export — `anicraft-swf`/`xfl`/`svg`/`export` **[A/N]** (L3)
- `anicraft-swf`: thin adapter over **Ruffle `swf`** — import (tags → `anicraft-doc`) and
  practical export (document → tags, documented set) (doc 07 §2).
- `anicraft-xfl`: new XFL reader/writer from the public XSD (doc 07 §3).
- `anicraft-svg`: export one frame as SVG 1.1 (VectorCraft pattern).
- `anicraft-export`: image sequences / GIF / APNG / video (FilmCraft codecs, git) / sprite
  sheets / Lottie (EffectCraft pattern) / WASM-JS player (doc 07 §5).

### 3.10 Persistence and recovery **[E/A]**
`craft-settings` (preferences: theme, workspaces, shortcut presets, script security),
`craft-atomic` (atomic `.anicraft` saves + recovery copies), `craft-quit` (app-shell quit),
`craft-print` (optional stage→PDF), `craft-fonts` (fonts via `CRAFT_FONTS_DIR`).

### 3.11 Testing and validation **[A]**
- Unit tests per crate; **property tests** (`proptest`) for parsers and `Document::check`
  invariants; **fuzz tests** for every importer (`.anicraft`, XFL, SWF) and the control/MCP
  protocol (VectorCraft `import_fuzz` pattern).
- **Golden-image render tests**: CPU renderer vs reference; (later) GPU vs CPU parity (≤ 1/255).
- **Determinism tests**: same (document, `t`) → identical pixels across runs.
- **Round-trip tests**: `.anicraft`→XFL→`.anicraft`; `.anicraft`→SWF→Ruffle-playback frame
  comparison (Ruffle as the SWF-conformance oracle).
- **Shared corpora** in `photocraft-corpus` (fetched pinned + SHA-256); no large binary fixtures
  committed.

## 4. Data-flow diagram

```mermaid
flowchart TD
  subgraph Authoring
    T[Tools: Action] --> E[anicraft-engine: Session + commands + undo]
    E --> D[(anicraft-doc: Arc<Document>)]
  end
  subgraph Deterministic
    D --> EV[anicraft-timeline: evaluate(doc,t) -> DisplayList]
  end
  subgraph Pixels
    EV --> R[anicraft-render: vello_cpu CPU ref]
    R --> G[anicraft-gpu: wgpu (later, parity-tested)]
  end
  subgraph Runtime
    EV --> RT[anicraft-script runtime: frame phases + events + boa]
    RT -->|authoring-time: commands| E
    RT -->|playback-time: runtime state| RT
  end
  subgraph Formats
    D --> SWF[anicraft-swf: Ruffle swf]
    D --> XFL[anicraft-xfl]
    D --> SVG[anicraft-svg]
    EV --> EXP[anicraft-export: seq/GIF/video/sheet/Lottie]
  end
  R --> UI[anicraft-ui-egui: stage + timeline + library + panels]
  E --> UI
  D --> UI
  UI -->|craft-ui: docking/tabs/theme| UI
```

## 5. Reuse classification summary

| Component | Class | Source |
|-----------|-------|--------|
| `craft-ui` (docking/tabs/toolbars/icons/theme op) | **E** | craft-libs (git) |
| `craft-settings` / `craft-atomic` / `craft-quit` / `craft-print` | **E** | craft-libs (git) |
| `craft-fonts` | **E** | craft-fonts (`CRAFT_FONTS_DIR`) |
| `swf` (SWF read/write) | **E** | Ruffle (git/crates.io) |
| `time` (Tick, FrameRate) | **E** | effectcraft-time (git) |
| `keyframe` (temporal/spatial/easing) | **A** | effectcraft-keyframe + CreateJS easing set |
| `vello_cpu` / kurbo / peniko / usvg / skrifa / harfrust / boa | **E** | crates.io |
| FilmCraft codec/encoder/mux + audio | **E** | filmcraft-* (git) — for video export |
| CPU-reference/GPU compositor + frame pool | **A** | EffectCraft/FilmCraft pattern |
| Tool→command state machines | **A** | VectorCraft `tools` pattern |
| `__query`/`__exec` scripting bridge + sandbox | **A** | EffectCraft `script` + PdfCraft `js` pattern |
| SVG / Lottie export | **A** | VectorCraft `svg` / EffectCraft `lottie` pattern |
| **Document/scene model (stage/scenes/timelines/layers/frames)** | **N** | — |
| **Symbol/instance model (graphic/movieclip/button)** | **N** | extends VectorCraft `Symbol` |
| **Display-list `evaluate()` (deterministic)** | **N** | — |
| **Vector shape authoring (edge/fill model, booleans)** | **N** | SWF/XFL model; boolean backend **[D]** |
| **Shape morph algorithm** | **N** | — (risk) |
| **Flash scripting object model** | **N** | Ruffle AVM2/display_object as reference |
| **XFL reader/writer** | **N** | from public XSD |
| **`.anicraft` native format** | **N** | suite idiom |
| **Flash UI: stage/timeline/library/actions/movie-explorer** | **N** | on `craft-ui` |
| GPU compositor | **D** | defer; EffectCraft/FilmCraft pattern |
| Boolean ops backend | **D** | kurbo-based port vs. a dependency |
| ActionScript legacy (AVM/ABC) | **D** | future via Ruffle, not initial |

## 6. Key architectural tradeoffs (recap, decisions in doc 10 §7)

1. **Native `.anicraft` + XFL/SWF interchange** (vs. XFL- or SWF-primary). Chosen for
   lossless, undo-friendly authoring; costs maintaining XFL + SWF on top.
2. **Reuse generic EffectCraft primitives (time, keyframe) but build Flash's document/renderer
   natively.** EffectCraft's `project`/`render` are bound to the After Effects model and would
   force the wrong abstraction; cross-repo git pins add coupling (accepted cost).
3. **boa for scripting** (vs. QuickJS/V8). Pure Rust + wasm + suite standard; lower raw JS
   throughput, mitigated by keeping heavy math in Rust.
4. **CPU-first rendering, deferred GPU** (vs. GPU-first). De-risks v1; GPU added behind the CPU
   oracle later (parity-tested).
5. **SWF read-all / practical-write; XFL round-trip; no FLA; no SWF 20–30.** Clean-room and
   spec-driven; a deliberate, documented limitation.
6. **Deterministic pure `evaluate()`** as the single source of display-list truth — the decision
   that makes rendering, scrubbing, onion skinning, scripting, and export all uniform, and that
   keeps the document separate from runtime state.

## 7. What this does NOT propose (boundaries)

- No changes to any existing Craft repo or to `craft-libs` (Flash-specific code stays in
  `anicraft-*`; any future shared-crate promotion is a separate craftrules workstream).
- No FLA support, no SWF 20–30, no ActionScript authoring (future interop only).
- No 3D, no particle systems, no advanced After-Effects-style effects — AniCraft is 2D vector
  animation + interactivity in the Flash/Animate scope.
- No web *authoring* target in v1 — the web build is a player/light authoring surface (wasm32),
  matching the siblings' web apps.