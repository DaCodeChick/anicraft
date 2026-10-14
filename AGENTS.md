# AniCraft — instructions for agents

AniCraft is an open-source, cross-platform, Rust-native 2D vector animation and
interactive-content authoring application in the **Craft (ArtCraft) family** — a modern
spiritual successor to **Macromedia Flash / Adobe Animate**. It combines classic Flash
authoring (stage, timeline, layers, keyframes, symbols, buttons, motion/shape tweening,
ActionScript-style interactivity) with the architectural conventions, shared infrastructure,
and visual design of the existing Craft ecosystem (VectorCraft, EffectCraft, FilmCraft,
PhotoCraft, PdfCraft, LightCraft).

> ## ✅ IMPLEMENTATION MODE — PHASE-GATED AUTHORIZATION (since 2026-10-13)
>
> Research Phase 0 is complete (`research/00`–`10`). **Phase 1 (Application Bootstrap and
> Craft Integration) is authorized** and is executed milestone by milestone per
> `research/09-implementation-roadmap.md`.
>
> Execution rules (binding until changed by the user):
> - **One milestone at a time.** Begin a milestone only after the previous one is complete,
>   verified, and recorded in `progress.md`. After finishing a milestone, **stop and return
>   control to the user** — do not advance to the next milestone automatically.
> - **Inspect the sibling Craft implementations first** (read-only) and reuse their established
>   conventions rather than inventing parallel infrastructure.
> - **Real functionality only.** No placeholders or filler. Do not claim anything is complete
>   without verifying it (the §4 quality gates green, tests run, output observed).
> - **Scope.** No Phase 2 or later work (no timeline, drawing, scripting, or format-import
>   features) while Phase 1 is incomplete. No speculative architectural changes contrary to the
>   approved research. No unnecessary documentation.
> - **Sibling repositories are read-only.** Do not modify `../vectorcraft`, `../effectcraft`,
>   `../filmcraft`, `../lightcraft`, `../pdfcraft`, `../photocraft`, `../craft-libs`,
>   `../craft-fonts`, or `../ruffle` without explicit permission. Depending on them by pinned
>   revision is how reuse works; `[patch]` blocks for local co-development stay commented out
>   unless the user asks.
> - **No autonomous subagents.**
>
> **Approved architectural decisions (2026-10-13**, recorded in `progress.md`): Rust edition
> 2024 with idiomatic Rust and the existing Craft conventions; reuse `craft-libs`
> (`craft-ui`, `craft-settings`, `craft-atomic`, `craft-quit`, optional `craft-print`);
> reuse Ruffle's `swf` crate and the compatible `effectcraft-time`/`effectcraft-keyframe`
> primitives; `vello_cpu` as the initial (CPU reference) renderer with **GPU acceleration
> deferred** (decision-gated at Phase 9.1); **Boa** for future JavaScript scripting; a
> **versioned native `.anicraft` project format** (primary) with XFL/SWF interchange;
> **dark mode is the primary interface**; the clean-room and licensing rules below are
> unchanged; **post-CS5 (CS5+) `.fla` files are supported** (see §1); the user-provided
> screenshots in `references/` **may be used for implementation** (see §1).

The research package lives in [`research/`](research/). Read `research/00-executive-summary.md`
first for conclusions, and `research/08-proposed-architecture.md` for the approved
architecture. `progress.md` tracks executed milestones and the current state.

---

## 1. Project objectives

- Provide a native (macOS, Windows, Linux; and web via WebAssembly) 2D vector animation and
  interactive-content authoring tool, in the Craft family's style and conventions.
- Preserve familiar Flash/Animate authoring concepts (stage, timeline, library, symbols,
  properties, drawing tools) while adopting modern Craft architecture.
- **Dark mode is the primary interface theme.**
- Support modern, open output and interchange: a versioned native document format (`.anicraft`),
  XFL, **post-CS5 (CS5+) `.fla`** (the ZIP-compressed XFL container — import/export through the
  same XFL machinery), SWF, SVG, animated images, video, sprite sheets, and standalone
  interactive (JS/WASM) export. The pre-CS5 **binary** `.fla` (proprietary OLE compound
  container, no public specification) remains unsupported by clean-room constraint (§2).
- The user-provided screenshots in `references/` **may be used as design and implementation
  reference** (layout, panel arrangement, palette, interaction). They are not application
  assets: shipped assets must still satisfy the asset rules in §2 (original/open-licensed,
  attributed), and the screenshots themselves are not bundled or embedded.
- Prefer reuse of established Craft infrastructure and the public Flash format specifications
  (SWF, XFL, Ruffle) over parallel reimplementations — but never force inappropriate reuse.

## 2. Craft ecosystem conventions (follow these)

These are verified conventions from the sibling repos. AniCraft must follow them.

- **Workspace shape.** A Cargo workspace (edition 2024, resolver 3) with `crates/*` (library
  crates named `anicraft-<dir>`), `apps/anicraft`, `apps/anicraft-cli`,
  `apps/anicraft-web`, and `xtask`. Workspace package fields and `[lints]` are inherited.
- **Layering, enforced.** `cargo xtask layers` must enforce that a crate depends only on lower
  layers and that nothing below the UI layer depends on `egui`/`eframe`/`winit`/`rfd`/`cpal`/
  `muda`. The same engine runs the desktop app, the CLI, the web build, the control channel,
  and the MCP server.
- **Engine-first.** Project-changing actions go through a `Session` (`anicraft_engine`).
- **Everything is a command.** Every user-visible action is a `CommandSpec` (id, label, menu
  path, shortcut, params doc, `enabled()`, `run()`, journal flag). Menus, the command palette,
  the JSON control channel, the CLI, and MCP all dispatch the same ids.
- **Tools never mutate the document directly.** Tool state machines emit `Action`s
  (`Begin` / `Preview` / `Commit` / `Cancel` / `Exec` / `Dialog` / `SwitchTool` / `Notify`)
  that the engine executes, so every gesture is replayable and testable without a UI (see
  VectorCraft `crates/tools/src/lib.rs`).
- **Copy-on-write snapshots.** The document is `Arc<Document>` (structural sharing); undo is a
  stack of snapshots, so undo/redo is constant time. Edits copy only the affected nodes.
- **Deterministic time.** Exact integer ticks and rational frame rates (as in
  `effectcraft-time` / `filmcraft-time`); no floating-point seconds in timeline math.
- **CPU reference, GPU fast path.** A CPU renderer is the oracle; any GPU path is tested
  against it pixel-for-pixel.
- **Pure Rust, clean-room.** No handwritten JS/TS in the app. Code and formats are written from
  public specifications (SWF File Format Spec, XFL schema, CreateJS easing, ABC format). See
  the clean-room rules below.
- **Shared infrastructure via git pins.** Depend on `craft-libs` and (where appropriate)
  Ruffle's `swf` crate and selected `effectcraft-*` crates by git revision, pinned in
  `[workspace.dependencies]`, with `[patch]` blocks for local co-development — the same pattern
  EffectCraft uses for FilmCraft's codecs.

### Clean-room and asset rules (inherited from the sibling apps)

- **Never use Adobe software** (Macromedia Flash, Adobe Animate, or any Adobe application) to
  derive behavior: do not run, script, screenshot, or reverse-engineer it, and do not compare
  output against it. Behavior comes only from public specifications (SWF spec, XFL schema,
  CreateJS easing, public forum posts cited by URL) and from user-owned files used locally
  (never committed).
- **Never copy GPL/AGPL code** (Inkscape, lib2geom, etc.).
- **Assets** (icons, fonts, images) must be original (MIT OR Apache-2.0), OSI open source,
  public domain/CC0, or Creative Commons with redistribution. Every asset needs an attribution
  row; fonts live in `storytold/craft-fonts`, never in this repo.
- Ruffle is MIT OR Apache-2.0; its `swf` crate is reusable. Reuse it by dependency (not by
  copying its code into this repo), consistent with the git-pin convention.

## 3. Architectural principles

1. **Separate the authoring document from the interactive runtime.** The document model
   (stage, timelines, symbols, keyframes) is pure, serializable, undoable data. The interactive
   playback/runtime environment (display list, events, scripting, per-frame state) is derived
   from the document and is never stored in the document.
2. **Deterministic, pure timeline evaluation.** Evaluating the document at a time `t` yields the
   same display list every time; it is a pure function of (document, `t`, eval options).
   Rendering and interaction are layered on top.
3. **Symbols are shared definitions with independent instances.** A symbol definition (graphic /
   movie clip / button / non-timeline asset) lives in the library; instances reference it by id
   and carry independent transform / color / name / playback state. Editing a definition updates
   all instances; instance properties stay independent and are stored on the instance.
4. **Reuse at the right layer.** Reuse shared infrastructure (`craft-libs`), the Flash format
   reader/writer (Ruffle `swf`), and generic animation primitives (time, keyframes). Build
   Flash-specific models (symbol/instance, stage/timeline authoring, Flash object model for
   scripting) in AniCraft — do not force EffectCraft's After Effects model into Flash.
5. **The renderer is a compositor over a display list.** The timeline evaluation produces a
   display list (stage → layers → placed instances/shapes at a time `t`); the renderer flattens
   it to pixels with `vello_cpu` (CPU reference) and, later, a wgpu fast path.
6. **Scripting talks to the engine through commands, not memory.** Scripts (JavaScript via boa)
   read the model through JSON queries and mutate it through engine commands, so scripted edits
   are undoable and identical to UI edits (the EffectCraft `__query`/`__exec` pattern).

## 4. Rust engineering standards (inherited)

- **Never crash in shipped code.** Every crate's `lib.rs` starts with
  `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented,
  clippy::todo, clippy::unreachable)]`. No `unsafe` (workspace `unsafe_code = "forbid"`/`"deny"`).
  Fallible work returns `Result`/`Option`; untrusted input (files, pasted data, MCP/control
  messages, command params) is handled, and importers/parsers are fuzzed.
- **Quality gates before every commit:** `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `cargo xtask layers`, `cargo xtask wasm` (everything up to the engine must build for
  `wasm32-unknown-unknown`). `cargo xtask ci` runs the whole suite.
- **Tests may unwrap** (`#[cfg(test)]`, `testkit`); integration-test files start with the
  matching `#![allow(…)]`.
- Use `.get(i)` rather than indexing when the index comes from data; cap sizes/counts read from
  input; check floats are finite before casting/looping.

## 5. Documentation expectations

- Keep `research/` current and internally consistent; distinguish **verified findings**
  (cited to a file or public spec) from **recommendations** and **open questions**. Research
  positions that a user decision later amends get a brief, dated status note — the original
  finding is preserved, not deleted.
- `progress.md` tracks completed work, the current milestone, outstanding issues, and the next
  authorized task; update it in the same change that closes a milestone.
- Follow the sibling apps' progress-doc standard as the project grows: a `ROADMAP.md`,
  `docs/architecture.md`, parity/gaps documents, and a revision history with a status line
  (Phase 9.6).
- Cite external specifications (SWF File Format Specification, XFL schema, CreateJS easing,
  AVM2/ABC overview) by URL where used.
- Do not produce placeholder or filler documentation.

## 6. Testing and verification standards (inherited)

- Unit tests per crate; property tests (`proptest`) for parsers and invariants; fuzz tests for
  every importer/parser/protocol method; golden-image render tests comparing the CPU renderer
  (and any GPU path) against reference output.
- Determinism tests: the same document + time must render identical pixels across runs and
  across CPU/GPU backends (within a documented tolerance).
- Shared real-file test oracles live in `storytold/photocraft-corpus` (fetch pinned by commit +
  SHA-256); never commit large binary fixtures to this repo.

## 7. Phased development practices

Work proceeds in the phases defined in `research/09-implementation-roadmap.md`
(Phase 0 research → Phase 9 production readiness). Rules:

- **One phase at a time, one milestone at a time.** A phase's milestones must be complete and
  verified before the next phase begins; within a phase, milestones are executed in dependency
  order, one at a time, with control returned to the user after each (see the implementation
  mode block above).
- **Measurable completion criteria.** Every milestone has observable, testable criteria.
- **Update `progress.md` (and the roadmap docs) in the same change** that closes a milestone.
- **User approval gates.** Beginning an implementation phase requires explicit user
  authorization. Phases with major tradeoffs (native format choice, scripting engine,
  SWF/XFL/FLA scope, GPU investment) require an explicit decision before they start. Status of
  each gate: §8.

## 8. Authorization status and remaining gates

Phase 0 (research) is **complete** (`research/00`–`10`). **Phase 1 is authorized** as of
2026-10-13, with the architectural decisions approved by the user (full record in
`progress.md`):

| # | Decision | Status |
|---|----------|--------|
| D1 | Native document format: versioned `.anicraft` (JSON, structurally shared) primary + XFL/SWF interchange | **Approved** |
| D2 | Reuse scope: `craft-libs` + Ruffle `swf` + `effectcraft-time`/`effectcraft-keyframe` + Boa + `vello_cpu` + kurbo/peniko (FilmCraft codec pins added in Phase 8) | **Approved** |
| D3 | Scripting engine: Boa (`boa_engine`) | **Approved** (Phase 7) |
| D4 | Interop scope: SWF read-all + practical write (≤ 19), XFL round-trip, **post-CS5 `.fla` (ZIP-of-XFL) supported**; pre-CS5 binary `.fla`, SWF 20–30, ActionScript authoring excluded | **Approved (amended 2026-10-13)** |
| D5 | GPU: CPU-first (`vello_cpu`), wgpu deferred and decision-gated at Phase 9.1 | **Approved** |
| D6 | Audio/video scope in v1 | **Gate pending** (before Phase 8; recommendation: vector + image/video export first, audio deferred) |
| D7 | ActionScript legacy: out of scope for v1 (boa/JS only) | Confirmed by default (non-gating) |
| D8 | Boolean-ops backend (port VectorCraft `pathops` vs. maintained kurbo-based crate) | **Gate pending** (before milestone 3.5) |
| D9 | Morph-algorithm approach (motion-first, shape-tween behind a matched-vertex fast path) | **Gate pending** (before milestone 6.3) |
| D10 | Web target scope (player/light-authoring wasm v1) | Confirmed by default (non-gating) |

Until a pending gate is decided by the user, the gated milestone waits; everything else in
Phase 1 proceeds. Sibling repositories remain read-only (see the implementation mode block).