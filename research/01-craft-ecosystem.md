# 01 — The Craft Ecosystem

> **Scope.** This document compares the six local Craft applications and the shared
> repositories, documenting **verified** architectural conventions (cited to files) and the
> parts most relevant to AniCraft. Sibling paths are relative to `~/Documents/GitHub/`.
>
> Verified sources: each repo's `Cargo.toml`, `AGENTS.md`, `docs/architecture.md`, and the
> listed crate `lib.rs` headers. Repos inspected read-only; nothing was modified.

## 1. Family overview

All six apps are **clean-room, open-source, Rust-native** reimplementations of a Creative
Cloud product, in the **ArtCraft** family (getartcraft.com). They are dual-licensed **MIT OR
Apache-2.0**, target macOS/Windows/Linux (and the web via WebAssembly), and are "drivable end
to end by AI agents over MCP."

| App | Version | Reimplements | Local crates | Rendering | Notable tech |
|-----|---------|--------------|--------------|-----------|--------------|
| **VectorCraft** | 0.10.0 | Adobe Illustrator | 23 | `vello_cpu` (CPU) + wgpu composite | kurbo, peniko, wasmi plugins, MCP, SVG/PDF/EPS/DXF/EMF |
| **EffectCraft** | 0.9.2 | Adobe After Effects | 33 | CPU reference + wgpu compositor | boa (JS expr+script), Lottie I/O, keyframes, FilmCraft codecs (git) |
| **FilmCraft** | 0.7.2 | Adobe Premiere Pro | 50 | CPU reference + wgpu compositor | ~25 pure-Rust codecs/containers, hardware decode/encode FFI (`platform`) |
| **PhotoCraft** | 0.8.2 | Adobe Photoshop | 25 | CPU (`photocraft-raster`/`compose`) + wgpu | vector, paint (brushes), wasmi plugins, tablet (pen) input, PSD |
| **PdfCraft** | 0.7.1 | Adobe Acrobat | 33 | PDF→pixels (hayro/lopdf, krilla) | boa (Acrobat JS), craft-print, OCR/sign/forms/redact |
| **LightCraft** | 0.7.1 | Adobe Lightroom | 31 | `vello`-style pipeline + wgpu | RAW develop, denoise (candle), SAM3 segmentation, geocode |

The Craft family is **larger** than these six: `craft-libs` docs reference **DesignCraft,
WordCraft, GridCraft, DeckCraft, CADCraft, SoundCraft** (theme/docking/theme adapters "now also
serve" them). Those repos are **not present locally**, so they are out of scope for verified
findings but confirm the conventions documented here are suite-wide.

Shared repos present locally:
- **`craft-libs/`** — shared Rust libraries (see doc 02). Crates: `craft-atomic`, `craft-print`,
  `craft-quit`, `craft-settings`, `craft-ui`.
- **`craft-fonts/`** — font assets + `craft-fonts` crate (manifest, `CRAFT_FONTS_DIR`, `embed`).
- **`ruffle/`** — the Flash Player emulator (MIT OR Apache-2.0); its `swf` crate is the basis for
  AniCraft's SWF interop (see doc 07).
- **`photocraft-corpus/`** — shared real-file test oracles (fetched pinned, SHA-256 verified).

## 2. Verified common architecture (the conventions AniCraft must follow)

These are consistent across all six apps and are the strongest evidence of the house style.

### 2.1 Workspace shape
- Cargo workspace, **edition 2024, resolver 3**, `members = ["crates/*", "apps/*", "xtask"]`.
- `[workspace.package]` sets `version`, `edition`, `license = "MIT OR Apache-2.0"`,
  `rust-version` (1.95; LightCraft 1.90), `repository`.
- Internal crates are listed in `[workspace.dependencies]` as
  `<app>-<dir> = { path = "crates/<dir>" }`.
- Apps: `apps/<name>` (desktop), `apps/<name>-cli` (headless), `apps/<name>-web` (wasm32).
  (FilmCraft/EffectCraft/VectorCraft all have the web app; PdfCraft/LightCraft/PhotoCraft too.)
- `xtask/` holds build tooling; `cargo xtask ci` runs the gates; `cargo xtask layers` enforces
  layering; `cargo xtask wasm` checks the wasm build.

### 2.2 Layering (enforced by `cargo xtask layers`)
Every app defines a layer table and enforces that **dependencies only point downward** and that
**nothing below the UI layer touches `egui`/`eframe`/`winit`/`rfd`** (and `cpal`/`muda` where
present). The canonical shape (VectorCraft `docs/architecture.md`):

| Layer | Role |
|-------|------|
| L0 | `geom`, `color` (foundation, on kurbo) |
| L1 | document model (pure data + serde, structural sharing) |
| L2 | geometry ops, brushes, trace, text, effects, plugins |
| L3 | render (pixels), SVG/PDF/native format I/O |
| L4 | tools (pointer events → commands + overlays) |
| L5 | engine (Session, history, selection, command registry, file I/O, panic guard) |
| L6 | `ui-egui`, `mcp` |
| L7 | `apps/*`, `xtask` |

EffectCraft and FilmCraft use the same idea with their own tables (FilmCraft reserves names for
planned crates; `platform` is the single crate allowed `unsafe`, per its ADR 0001).

### 2.3 Engine-first; everything is a command
- A `Session` owns the document, editor state, history, and the command registry.
  `Session::execute(id, params)` (or `Session::edit(label, |doc, state| …)`) is the single
  mutation path.
- Every user-visible action is a `CommandSpec { id, label, menu, shortcut, params, enabled(),
  run(), journal }`. The menu bar is a tree of command ids. UI-only commands live in
  `ui-egui` (`UI_COMMANDS`).
- Menus, the command palette, the JSON **control channel** (`--control <port>`, default 7979),
  the **CLI**, and the **MCP** server all dispatch the same ids → "anything a person can do, an
  agent can do." (VectorCraft `AGENTS.md`; FilmCraft `docs/architecture.md` §4.)

### 2.4 Copy-on-write undo
- The document is `Arc<Document>` (or `Arc<Project>`); children are `Arc<Node>`; edits copy only
  the path to the edited node (VectorCraft `crates/doc/src/lib.rs`: "Keeping the previous
  `Document` value around is therefore a cheap undo snapshot").
- Undo/redo are pointer swaps (constant time). FilmCraft keeps a 200-step stack; EffectCraft a
  "branching undo history." Modified-state follows the last saved snapshot.

### 2.5 Tools never mutate the document
- VectorCraft `crates/tools/src/lib.rs`: tools emit `Action` (`Begin`, `Preview`, `Commit`,
  `Cancel`, `Exec`, `Dialog`, `SwitchTool`, `Notify`). `Begin` snapshots, `Preview` re-applies one
  command on that snapshot, `Commit` records the last preview as a single undo step. This makes
  every gesture replayable by the control channel/MCP and testable without a UI.

### 2.6 Never crash; pure Rust; wasm
- Workspace lints: `unsafe_code = "forbid"`/`"deny"`; clippy `unwrap_used`/`expect_used`/`panic`/
  `todo`/`unimplemented`/`unreachable` denied (VectorCraft denies `unwrap_used`/`expect_used` at
  workspace level; others deny in each `lib.rs`).
- A `guard` (e.g. `vectorcraft_engine::guard`) catches a panic at every entry point and rolls the
  document back. Data Recovery/autosave copies run in the background.
- "Pure Rust (no handwritten JS/TS)." Everything up to the engine builds for
  `wasm32-unknown-unknown` (`cargo xtask wasm`).
- Determinism where it matters: FilmCraft "Exact time — integer ticks, rational frame rates. No
  f64 seconds in edit math"; reproducible exports (same project+settings → same bytes).

### 2.7 Shared UI and platform helpers
- **`craft-ui`** (git-pinned) provides docking, tabs, toolbars, split/resize geometry, icon SVG
  catalogs, and the shared widget-palette theme operation. Lower layers must not depend on it.
- **`craft-settings`** (`config.toml` storage, portable mode, migrations), **`craft-atomic`**
  (crash-safe atomic file writes), **`craft-print`** (PDF spooler), **`craft-quit`** (macOS quit).
- Fonts come from `craft-fonts` via an optional `CRAFT_FONTS_DIR` build input, never committed to
  the app repo.

### 2.8 Cross-app crate reuse by git pin
EffectCraft depends on **FilmCraft's** pure-Rust codec/media crates by **git revision**
(`filmcraft-codecs`, `filmcraft-media`, `filmcraft-frame`, … pinned to one rev), with commented
`[patch]` blocks for local co-development. This is the established mechanism for sharing
substantive crates across repos — and it is the mechanism AniCraft would use for Ruffle `swf`
and any reused `effectcraft-*` primitives (doc 02, 08).

## 3. App-by-app notes (relevance to AniCraft)

### 3.1 VectorCraft (closest for vector drawing) — `vectorcraft/`
- **Document model** (`crates/doc`): `Document { version, title, units, color_mode, artboards,
  layers }`; `Node` with `NodeKind` = `Layer`, `Group`, `Path { path, rule, live, clipping,
  guide }`, `Text`, image, etc. (`crates/doc/src/node.rs`). `Symbol { name, art: Arc<Node> }`
  (`crates/doc/src/lib.rs:527`) — a simple shared-art symbol (Illustrator model), **not** a
  timeline symbol.
- **Rendering** (`crates/render`): `vello_cpu` (SIMD, sparse strips); callers pass a view
  transform; culls by bounds; evaluates appearance stacks (fills/strokes, opacity, blend, dashes),
  clip groups, gradients, images, text.
- **Export** (`crates/svg` on `usvg` + own writer; `crates/pdf` on `krilla`/`hayro-interpret`;
  `crates/eps`, `crates/cad` DXF, `crates/metafile` EMF/WMF). `crates/format` = native
  `.vectorcraft` (versioned, compressed JSON, atomic save, backward-readable).
- **Tools** (`crates/tools`): Pen, Direct Selection, shape, text, transform, smart guides, etc.
- **Engine** (`crates/engine`): ~710 commands in `src/cmd/*`, `guard`, `journal`, autosave,
  recovery.
- **MCP** (`crates/mcp`): JSON-RPC over stdio, `Remote` (control channel) or `Headless`.
- **Reuse for AniCraft:** vector document patterns, `vello_cpu` renderer, SVG/PDF export,
  tool→command pattern, `kurbo`/`peniko`/`skrifa`/`harfrust` usage, native-format design.

### 3.2 EffectCraft (closest for timeline/tweening/scripting) — `effectcraft/`
- **Time** (`crates/time`): `Tick` = 254 016 000 000 / s (LCM of broadcast rates and sample
  rates) → exact frames and samples.
- **Keyframes** (`crates/keyframe`): temporal interpolation `Linear | Bezier | Hold`; ease as
  `speed` (units/s) + `influence` (0..1); **spatial** Bezier motion path with an arc-length
  table; auto/continuous Bezier derivation. This is directly the machinery for Flash **motion
  tweens** (and the easing model maps to CreateJS, doc 03).
- **Document** (`crates/project`): `Project { settings, items, render_queue }`; `Item` (Folder /
  Comp / Footage / Solid); `Comp { size, pixel aspect, frame rate, duration, start timecode,
  background, work area, layers, markers, renderer }`; `Layer { id, name, source (LayerSource),
  in/out/start, stretch, switches, blend mode, track matte, parent, props }`; everything
  animatable is a `Property` in a `PropGroup` tree. `LayerSource` = Footage | Comp | Solid | Text |
  Shape | Null | Camera | Light | Model | Primitive.
- **Render** (`crates/render`): CPU reference compositor (source → masks → effects → styles →
  transform → matte → blend), a **layer cache** keyed by content hash, motion-blur sub-sampling,
  `Accelerator` trait; **`crates/gpu`** is the wgpu implementation tested against the CPU.
- **Scripting** (`crates/expr`, `crates/script`): **boa** JavaScript. `expr` = per-frame property
  expressions (thisComp/thisLayer/wiggle/loopOut). `script` = full app/project object model
  (`app`, `Project`, `CompItem`, `Layer`, `Property`, `RenderQueue`, `File`), `ScriptUI`
  (dialogs/palettes/panels), **edits run engine commands** via a `__query`/`__exec` JSON bridge,
  `beginUndoGroup`/`endUndoGroup`, match names, and file/network security behind a preference.
- **Lottie** (`crates/lottie`): Lottie/dotLottie **import and export** (layers, precomps, eased
  and spatial keyframes, shapes, masks, mattes) with a warnings list for what Lottie can't
  express.
- **Export** (`crates/export`): H.264/HEVC/AV1 MP4, ProRes, WebM (VP9+alpha / AV1, Opus),
  PNG/JPEG/TIFF/EXR sequences, GIF, WAV/AIFF.
- **UI** (`crates/ui-egui`): docking (uses `craft-ui::docking` + `layout`), panels, viewer,
  **timeline**, **graph editor**, dialogs, control channel.
- **Reuse for AniCraft:** `time` + `keyframe` (generic animation), the CPU-reference/GPU
  compositor pattern, the boa `__query`/`__exec` scripting bridge, Lottie export, the
  timeline/graph-editor UI patterns. **Not** directly reusable: `project`/`render` (bound to the
  After Effects model).

### 3.3 FilmCraft (time base, playback, export, audio) — `filmcraft/`
- **Time** (`crates/time`): `Tick(i64)` at the same 254 016 000 000 rate; `FrameRate { num, den }`
  with `frame_duration()`, `tick_of(frame)`, `frame_at(tick)`, `snap(tick)`; timecode is display
  only.
- **Document** (`crates/project`): `Project { root: Bin, items: map ItemId→ProjectItem, next_id }`;
  `Sequence { settings, video_tracks/audio_tracks, markers }`; `Track { items: Vec<TrackItem>
  (sorted, non-overlapping), transitions, audio: MixerStrip }`; `EffectInstance` (params are
  constant values or **keyframe tracks**); effect **definitions are data**
  (`project::effect::effect_defs()`).
- **Render/playback** (`crates/render`, `crates/ui-egui/src/frames.rs`): CPU compositor is the
  oracle; a prioritized frame-worker pool (on-screen first, then prefetch, then thumbnails);
  audio clock drives the playhead; `gpu` is the wgpu compositor.
- **Export** (`crates/export`): render → encode → mux, progress/cancel, reproducible output,
  presets; ~25 pure-Rust codec/container crates (h264/hevc/vp8/vp9/av1/prores/dnx/apv/dng,
  aac/ac3/mpa/flac/opus, isobmff/matroska/mxf/cfb/mpegts/ogg).
- **`platform`**: the single `unsafe` crate — hardware video decode/encode FFI (VideoToolbox,
  Media Foundation, VA-API, NVENC) behind `codecs::VideoDecoder` / `export::VideoEncoder`.
- **Reuse for AniCraft:** the exact-tick time base (or EffectCraft's equivalent), the
  export/encode/mux pipeline and codec crates for **video output**, the audio pipeline (cpal,
  mixer) for event/streaming sounds, the frame-worker playback pattern.

### 3.4 PhotoCraft (vector/paint/plugins/tablet) — `photocraft/`
- **Vector** (`crates/vector`): point-level path editing (Direct Selection, Convert Point), hit
  testing, cubic flattening, non-affine maps (Distort/Perspective/Warp), exact area-coverage
  scanline rasterizer (non-zero/even-odd, boolean combination), polygon stroking (miter/round/
  bevel, caps, dashes), live shapes (rect/ellipse/polygon/star/line), contour tracing.
- **Paint** (`crates/paint`): Photoshop-grade brushes (dynamics, scattering, texture, dual,
  color/transfer, symmetry); strokes are **data** (replayable/deterministic by seed hash).
- **Plugins** (`crates/plugins`): sandboxed **wasmi** WebAssembly modules, ABI v1 — **no host
  imports**, per-call instruction budget (fuel) + wall-clock deadline, linear-memory cap, fresh
  instance per call; every failure is an `Error`, never a panic.
- **Tablet** (`crates/tablet`): pen pressure/tilt/rotation on macOS (AppKit local monitor) and
  Linux X11 (XInput2 raw events); the only `unsafe` in the crate.
- **Reuse for AniCraft:** the wasmi **plugin sandboxing model** (fuel, no imports, fresh
  instance) as the template for any AniCraft extension mechanism; tablet input for
  pressure-sensitive drawing; vector stroke/raster helpers as reference.

### 3.5 PdfCraft (JS engine, forms, printing) — `pdfcraft/`
- **JS** (`crates/js`): **boa** (feature `annex-b`) with a sandboxed Acrobat-JS object model
  subset (`event`, `getField`, `app`, `util`, `console`). Confirms boa as the suite's JS engine
  and the pattern of a *bounded* object model over the engine.
- **Print** (`crates/print` + `craft-print`): sheet composition in the engine; spooling via
  `craft-print`.
- **Vendored/patched** crates under `vendor/` (`[patch.crates-io]`) with an `ATTRIBUTION.toml` —
  a model for documenting non-crate.io dependencies.
- **Reuse for AniCraft:** the bounded-boa-object-model pattern; `craft-print` for any
  print-to-PDF of a stage; the vendoring/attribution documentation pattern.

### 3.6 LightCraft (RAW pipeline, GPU, AI) — `lightcraft/`
- RAW develop pipeline (`develop`, `pipeline`, `raster`, `color`), denoise (candle), SAM3
  segmentation (`segment`), catalog/preview, `gpu` (wgpu, no GL), `mcp`. Has Nix (`flake.nix`),
  a web profile, and a `dist` profile.
- **Reuse for AniCraft:** minimal — included for completeness; confirms the wgpu + `mcp` +
  web-profile conventions.

## 4. Duplication and constraints introduced by the current design

- **Per-app foundation crates.** `geom`, `color`, `raster` exist **independently** in each app
  (e.g. `vectorcraft-geom`, `effectcraft-geom`, `filmcraft-geom`, `photocraft-geom`,
  `lightcraft-geom`). They have not been promoted to `craft-libs`. Consequence: AniCraft will
  either add its own `anicraft-geom`/`anicraft-color` (consistent, but duplicative) or depend
  on kurbo/moxcms directly. **Recommendation (doc 08):** build a thin `anicraft-geom` over
  kurbo for now; a future craftrules workstream could promote a shared geometry/color crate.
- **The same engine pattern is re-derived per app.** `Session`, `CommandSpec`, `guard`,
  control channel, and MCP server are structurally identical across apps but not shared.
  Consequence: AniCraft re-implements these (thinly) per the convention; they are small and
  well-understood.
- **`craft-ui` is the only shared *frontend* piece**, and it deliberately stays low (no assets,
  no complete themes, no app engines). So each app keeps its own `theme.rs`, icons, fonts, and
  localization. AniCraft will do the same.
- **Cross-repo dependencies are git-pinned**, which adds version-pinning and co-development
  burden (`[patch]`). This is an accepted cost in the ecosystem.

## 5. What AniCraft can and cannot take

| Need | Source | Take directly? |
|------|--------|----------------|
| Vector doc patterns, tool→command, `vello_cpu`, SVG/PDF export, native format design | VectorCraft | Patterns + crates (`vello_cpu`, `usvg`, `krilla`); **not** its `doc`/`render` (Illustrator model) |
| Time base, keyframes/tweens, easing | EffectCraft / FilmCraft | **Yes** (generic primitives), via git pin |
| Timeline evaluation/compositing, scripting, Lottie, GPU pattern | EffectCraft | **Patterns**; `project`/`render` **not** (AE model) |
| Export/encode/mux, audio, codecs, playback frame pool | FilmCraft | **Yes** (codec/export crates), via git pin |
| UI shell (docking/tabs/toolbars/theme op), settings, atomic save, print, quit | craft-libs | **Yes**, via git pin |
| Font assets + loader | craft-fonts | **Yes**, via `CRAFT_FONTS_DIR` |
| Plugin sandboxing (wasmi), tablet input, stroke/raster helpers | PhotoCraft | **Patterns** |
| Bounded-boa object-model pattern | PdfCraft / EffectCraft | **Pattern** |
| SWF read/write, Flash runtime reference, AVM2 | Ruffle | **`swf` crate (dep)**; runtime as reference only |
| Flash symbol/instance, stage/timeline authoring, XFL, Flash scripting model, `.anicraft` | — | **New in AniCraft** |

**Conclusion.** AniCraft is a genuinely new app (the symbol/instance + stage/timeline authoring
model and the Flash scripting object model do not exist in the suite), but it stands on a deep
base of shared infrastructure, a reusable format reader/writer (Ruffle `swf`), and proven
patterns (layering, commands, copy-on-write undo, CPU-reference/GPU, boa scripting). The bulk of
*mechanical* work is already solved; the *Flash-specific* model is the core contribution.