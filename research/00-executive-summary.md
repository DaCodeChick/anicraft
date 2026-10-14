# 00 — Executive Summary

> **Status:** Research complete. **AniCraft is in research-only mode — no implementation is
> authorized.** This document summarizes the research in `research/` and the recommended
> direction. All findings distinguish *verified in source* (sibling Craft repos, Ruffle) from
> *external* (public specifications and web research) and from *recommendations*.

## 1. What AniCraft is

A native, open-source, Rust 2D vector animation and interactive-content authoring application
in the Craft (ArtCraft) family — a modern successor to **Macromedia Flash / Adobe Animate**. It
must deliver the classic Flash authoring model (stage, timeline, layers, keyframes, symbols,
buttons, motion/shape tweening, scripting, publish) using the **architecture, shared
infrastructure, and dark visual design** of the six existing Craft applications, and it must
interoperate with the open Flash formats (SWF, XFL) rather than depend on proprietary ones.

## 2. Headline findings

1. **The Craft ecosystem is a strong, mature, and consistent base to build on.** Six
   production-quality Rust apps share a tightly enforced architecture: a Cargo workspace with
   layered library crates, an engine-first "everything is a command" core, copy-on-write undo,
   a `vello_cpu`/wgpu rendering split, a `craft-ui` frontend, `craft-settings`/`craft-atomic`
   persistence, an MCP server, a JSON control channel, and strict no-panic / no-unsafe lints.
   Verified across `vectorcraft/`, `effectcraft/`, `filmcraft/`, `photocraft/`, `pdfcraft/`,
   `lightcraft/` and `craft-libs/`.

2. **The closest analogues are VectorCraft (vector drawing) and EffectCraft (timeline,
   keyframes, tweening, scripting), with FilmCraft (time base, playback, export, audio) as
   support.** VectorCraft supplies the vector document model, `vello_cpu` renderer, SVG/PDF
   export, and the tool→command pattern. EffectCraft supplies the keyframe/tween engine, the
   expression and scripting runtimes (boa), Lottie export, and the CPU-reference/GPU compositor
   pattern. FilmCraft supplies the exact-tick time base, the export pipeline, and playback.
   None, however, has Flash's **symbol/instance** or **stage/timeline authoring** model — that
   is AniCraft's core new work. (01, 03, 04, 08)

3. **The Flash format layer is reusable and legally clean.** Ruffle (MIT OR Apache-2.0) ships a
   standalone `swf` crate (v0.3.0, on crates.io) that **reads and writes SWF 19**. SWF 19 is the
   last publicly documented version and is an *open specification*; XFL (the open XML authoring
   format, the uncompressed form of modern FLA) has a public XSD. This means SWF/XFL import and
   export can be built **clean-room from public specs** without any Adobe software, consistent
   with the sibling apps' absolute clean-room rule. (07)

4. **Scripting should use boa (boa_engine 0.22), the ecosystem standard.** EffectCraft
   (expressions + full scripting + ScriptUI) and PdfCraft (Acrobat JS) both already use boa. It
   is pure Rust, builds for `wasm32`, needs no C toolchain, and its `__query`/`__exec` JSON
   bridge pattern (scripts read via JSON snapshots and mutate via engine commands, so edits are
   undoable) is exactly the separation AniCraft wants between scripting and the document model.
   boa is slower than QuickJS/V8 by roughly an order of magnitude or more, but that is
   acceptable for authoring-time scripting and keeps the workspace pure-Rust. (05)

5. **A native `.anicraft` document format is recommended, with XFL/SWF as interchange.**
   This mirrors the sibling pattern (VectorCraft `.vectorcraft` + SVG/PDF; EffectCraft
   `.ecproj` + Lottie): a versioned, compressed, structurally-shared native format for lossless
   authoring, plus robust import/export. SWF is a *playback* format and is a poor primary
   authoring store (it has no editor state and some authoring-only data); XFL is open and
   round-trips authoring data but is verbose and has quirks. (07, 08)

6. **The UI can belong to the Craft family with little new design work.** `craft-ui` provides
   docking, tabs, toolbars, split/resize geometry, icon SVG catalogs, and the shared
   widget-palette theme operation; the sibling apps define a consistent dark palette (e.g.
   VectorCraft dark: panel `#323232`, pasteboard `#1e1e1e`, accent `#378ef0`, text `#d8d8d8`).
   Flash's well-documented workflow (Stage + pasteboard, Timeline below the stage, Tools panel,
   Properties inspector, Library, Actions, Movie Explorer) maps cleanly onto Craft's
   docking/panel/inspector conventions with **dark mode as the primary theme**. (06)

## 3. Recommended direction (summary of 08)

Build AniCraft as a new sibling app with `apps/anicraft`, `-cli`, `-web` and
`anicraft-*` crates, following the standard layering. Reuse:

- **Directly (git/crates.io deps):** `craft-libs` (`craft-ui`, `craft-settings`, `craft-atomic`,
  `craft-print`, `craft-quit`); Ruffle's **`swf`** crate for SWF I/O; `kurbo`/`peniko`
  (geometry/paint), `vello_cpu` (CPU raster), `skrifa`/`harfrust` (text); **`effectcraft-time`
  and `effectcraft-keyframe`** (generic animation primitives); **`boa_engine`** (scripting);
  optionally FilmCraft's export/codec crates for video output.
- **Adapt/extend:** the CPU-reference/GPU compositor pattern (EffectCraft), the tool→command
  pattern (VectorCraft), the `__query`/`__exec` scripting bridge (EffectCraft), the
  symbol definition/instance model (VectorCraft's simple symbol, extended for Flash timelines).
- **Build new in AniCraft:** the Flash **document/scene model** (stage, scenes, timelines,
  layers, frames, symbols/instances, buttons), the **display-list compositor** for Flash
  (stage → layers → placed content at time `t`), **motion & shape tweening/evaluation**, the
  **XFL reader/writer**, the **Flash scripting object model**, and the **`.anicraft`
  native format**.

The centerpiece architectural decision — a **pure, deterministic timeline evaluation** that
produces a display list, with rendering, interaction, and scripting layered on top and the
document kept separate from runtime state — is what lets AniCraft reuse the Craft
infrastructure while keeping the authoring model clean.

## 4. Strongest reuse opportunities (ranked)

| # | Reuse | From | Why it matters |
|---|-------|------|----------------|
| 1 | `swf` crate (SWF 19 read/write) | Ruffle | The single highest-value reuse: full SWF interop, legally clean, zero format work. (07) |
| 2 | Keyframe + tween primitives (`effectcraft-keyframe`, `effectcraft-time`) | EffectCraft | Motion/shape tweening, easing, and exact time — hard to get right, already solved. (03, 05) |
| 3 | `craft-ui` (docking, tabs, toolbars, theme) | craft-libs | The entire panel/inspector/toolbar shell with Craft look-and-feel. (06) |
| 4 | `vello_cpu` renderer + SVG/PDF export pattern | VectorCraft | Vector rasterization and static export without reimplementation. (04) |
| 5 | boa scripting + `__query`/`__exec` bridge | EffectCraft | Interactive runtime with undoable, sandboxed scripting. (05) |
| 6 | `craft-settings`/`craft-atomic`/`craft-quit` | craft-libs | Preferences, crash-safe atomic saves, quit handling. (08) |

## 5. Major tradeoffs

- **Native format: `.anicraft` (JSON, structurally shared) vs. XFL vs. SWF.** Recommended:
  native `.anicraft` primary, XFL + SWF interchange. Tradeoff: maintain a second rich format
  (XFL) and a playback format (SWF) on top of the native one; the alternative (XFL-native)
  simplifies storage but inherits XFL's verbosity/quirks and makes undo/structural sharing
  awkward. (07, 10)
- **Reuse EffectCraft's animation crates vs. build Flash-native.** Recommended: reuse the
  *generic* primitives (time, keyframes) but build Flash's *document/renderer* natively,
  because EffectCraft's `project`/`render` are bound to the After Effects model (footage,
  comps, camera, lights, 3D). Tradeoff: cross-repo git dependencies increase coupling and
  version-pinning burden. (02, 08, 10)
- **Scripting engine: boa vs. QuickJS/V8.** Recommended: boa (pure Rust, wasm, ecosystem
  standard). Tradeoff: markedly lower raw JS throughput than QuickJS/V8; mitigated by keeping
  heavy per-frame work in Rust and scripts in the authoring/interactive layer, not the render
  hot path. (05, 10)
- **CPU-only vs. CPU+GPU rendering.** Recommended: CPU (`vello_cpu`) reference first, add a wgpu
  fast path later behind the same oracle (the EffectCraft/FilmCraft pattern). Tradeoff: defer
  GPU investment to a later phase. (04, 08)
- **SWF/FLA interop scope.** Recommended: full SWF read + practical SWF write (the documented
  tags), XFL round-trip; **no** FLA (proprietary, clean-room-incompatible), no reverse
  engineering of undocumented SWF 20–30. (07, 10)

## 6. Decisions requiring user approval (see 10 §7)

1. **Native document format** — confirm `.anicraft` (versioned, compressed, structurally
   shared JSON) as primary, with XFL and SWF as interchange.
2. **Reuse scope** — confirm depending on Ruffle `swf` and selected `effectcraft-*`
   (`time`, `keyframe`) crates by git pin, versus vendoring/porting them.
3. **Scripting engine** — confirm boa (boa_engine 0.22) and the `__query`/`__exec` bridge.
4. **Interop scope** — confirm SWF (read + practical write) and XFL (round-trip); confirm **no**
   FLA support and no reverse engineering of SWF 20–30.
5. **GPU investment** — confirm CPU-first rendering with a deferred wgpu fast path.
6. **Audio & video scope** — confirm whether Phase 1 targets are silent/vector-only and audio
   (event sounds, streaming) and video export are later phases (FilmCraft codecs available).
7. **ActionScript legacy** — confirm that ActionScript 1/2/3 *compatibility* is out of scope for
   initial release (boa/JS only), with legacy AVM2/ABC interop (via Ruffle) a documented future
   option.
8. **Phase-specific decisions** — the boolean-ops backend (D8), the morph-algorithm approach
   (D9), and the web target scope (D10) are also listed in 10 §7, each with its own phase gate.

## 7. Repository integrity confirmation

- **No application code was written.** The repo still contains only `.gitignore`, `references/`
  (four user-provided JPEG screenshots), and this `research/` package plus `AGENTS.md`.
- **No existing Craft repository was modified.** All sibling repos and `ruffle/` were inspected
  read-only.
- **No Rust project was initialized, no build was run, no commit was made.**

**Next:** the user should review `research/`, make the decisions in §6, and explicitly authorize
Phase 1 (application bootstrap and Craft integration). **Nothing proceeds until then.**