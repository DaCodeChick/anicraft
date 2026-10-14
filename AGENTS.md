# AniCraft — instructions for agents

AniCraft is a proposed open-source, cross-platform, Rust-native 2D vector animation and
interactive-content authoring application in the **Craft (ArtCraft) family** — a modern
spiritual successor to **Macromedia Flash / Adobe Animate**. It combines classic Flash
authoring (stage, timeline, layers, keyframes, symbols, buttons, motion/shape tweening,
ActionScript-style interactivity) with the architectural conventions, shared infrastructure,
and visual design of the existing Craft ecosystem (VectorCraft, EffectCraft, FilmCraft,
PhotoCraft, PdfCraft, LightCraft).

> ## ⛔ RESEARCH-ONLY MODE — NO IMPLEMENTATION IS AUTHORIZED
>
> **AniCraft is currently in research-only mode.** As of this writing, the repository
> contains **no application source code**: only a `.gitignore` and a `references/` folder of
> user-provided screenshots.
>
> You are **not** authorized to:
> - Write application source code, or create Rust crates, workspaces, or scaffolding.
> - Create `Cargo.toml`, `xtask`, `apps/`, `crates/`, or any build configuration.
> - Modify any existing Craft repository (`../vectorcraft`, `../effectcraft`, `../filmcraft`,
>   `../lightcraft`, `../pdfcraft`, `../photocraft`, `../craft-libs`, `../craft-fonts`, `../ruffle`).
> - Install dependencies or development tools, run builds, or execute applications.
> - Build prototypes or proof-of-concept applications.
> - Make Git commits or change Git configuration.
> - Begin any implementation phase (Phase 1 or later).
>
> The **only** permitted file changes are Markdown research and planning documents inside
> `research/` (and this file). After completing research, **stop and wait for explicit user
> authorization** before any implementation begins. The immediate deliverable is the complete
> Markdown research package in `research/` — **not** a working application.

The research package lives in [`research/`](research/). Read `research/00-executive-summary.md`
first for conclusions and the recommended direction, and `research/08-proposed-architecture.md`
for the proposed (not yet approved) architecture.

---

## 1. Project objectives

- Provide a native (macOS, Windows, Linux; and web via WebAssembly) 2D vector animation and
  interactive-content authoring tool, in the Craft family's style and conventions.
- Preserve familiar Flash/Animate authoring concepts (stage, timeline, library, symbols,
  properties, drawing tools) while adopting modern Craft architecture.
- **Dark mode is the primary interface theme.**
- Support modern, open output and interchange: a native document format, XFL, SWF, SVG,
  animated images, video, sprite sheets, and standalone interactive (JS/WASM) export.
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
- **Quality gates before every commit** (once implementation begins): `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `cargo xtask layers`, `cargo xtask wasm` (everything up to the engine must build for
  `wasm32-unknown-unknown`).
- **Tests may unwrap** (`#[cfg(test)]`, `testkit`); integration-test files start with the
  matching `#![allow(…)]`.
- Use `.get(i)` rather than indexing when the index comes from data; cap sizes/counts read from
  input; check floats are finite before casting/looping.

## 5. Documentation expectations

- Keep `research/` current and internally consistent; distinguish **verified findings**
  (cited to a file or public spec) from **recommendations** and **open questions**.
- Once implementation begins, follow the sibling apps' progress-doc standard: a `ROADMAP.md`,
  `docs/architecture.md`, parity/gaps documents, and a revision history with a status line.
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

- **One phase at a time.** A phase's milestones must be complete and verified before the next
  phase begins.
- **Measurable completion criteria.** Every milestone has observable, testable criteria.
- **Update the roadmap and progress docs in the same change** that closes a milestone.
- **User approval gates.** Beginning any implementation phase (Phase 1 onward) requires explicit
  user authorization. Phases with major tradeoffs (native format choice, scripting engine,
  SWF/FLA scope, GPU investment) require an explicit decision before they start.

## 8. Requirements for obtaining user approval

Do **not** begin implementation, initialize a Rust project, or start Phase 1 until the user
explicitly instructs you to. Before proposing to start, present:

1. The approved research package (`research/`).
2. The decisions requiring approval (see `research/10-risks-and-open-questions.md` §7): the
   native document format, the scope of reuse from EffectCraft/Ruffle, the scripting engine
   (boa), the SWF/XFL interop scope, and the GPU investment plan.
3. Confirmation that no application code or existing Craft repositories will be modified until
   authorized.

**Until that authorization is given, the deliverable remains the Markdown research package only.**