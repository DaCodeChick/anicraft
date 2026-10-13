# 10 — Risks, Uncertainties, and Open Questions

> **Scope.** Technical risks, missing information, legal/clean-room constraints, and the
> decisions that require **user approval** before implementation. Each open question is marked
> **[DECISION]** (needs the user) or **[INVESTIGATE]** (needs more research, not blocking).
> Cross-references: architecture in doc 08, roadmap gates in doc 09.

## 1. Technical risks (ranked by impact)

### R1. Shape tweening / morphing — **HIGH**
The **morph algorithm** (matching edges/fills between two shapes and interpolating by ratio `u`)
is the hardest genuinely-new piece of work (doc 08 §3.2, Phase 6.3). SWF `DefineMorphShape` and
XFL `tweenType="shape"` define the *storage* (a pair of shapes + a ratio) but not an authoring
matcher. A robust general morph (arbitrary topology changes, holes, multiple fills) is a known
hard problem (vertex correspondence).
- **Mitigation:** ship **motion tweens first** (well-understood, via `effectcraft-keyframe` +
  CreateJS easing); gate **shape tweens** behind a documented "matched-vertex / same-structure"
  fast path initially, and improve the matcher iteratively. Validate against SWF morph read (the
  ratio interpolates a stored pair) so at least *playing* morphs is exact even if *authoring* a
  new morph is limited.
- **Residual risk:** some authoring-time shape morphs may be limited in v1.

### R2. SWF conformance and coverage — **MEDIUM-HIGH**
SWF is a large format. Writing a "practical" subset risks producing files that don't play in
Ruffle or that omit features users expect; reading all versions risks hitting undocumented edge
cases.
- **Mitigation:** depend on **Ruffle's `swf` crate** (the reference implementation, MIT/Apache)
  rather than writing our own; use **Ruffle as the conformance oracle** (export → Ruffle-plays →
  frame-compare against our `evaluate`+`render`). Fuzz all importers. Target the **documented
  (≤ 19) tag set** and a mid-range write version.
- **Residual risk:** exotic tags (advanced fonts, video, some filters) are out of the v1 write
  scope; some legacy SWF may import with lossy mapping (documented per-tag).

### R3. Cross-repo dependency coupling — **MEDIUM**
Reusing `effectcraft-time`/`effectcraft-keyframe` (git-pinned) and Ruffle `swf` and (optionally)
FilmCraft codecs introduces **version-pinning and upstream-tracking** burden, and means FlashCraft
tracks another team's API.
- **Mitigation:** the git-pin + `[patch]` pattern is already the ecosystem idiom (EffectCraft→
  FilmCraft). Pin exact revisions; keep the reuse surface **narrow and stable** (time, keyframe,
  swf — all low-churn). If a pinned crate's API churns, vendor a thin local copy (a last resort,
  documented).
- **Residual risk:** an upstream breaking change in a pinned crate could require a FlashCraft
  update.

### R4. boa scripting throughput — **MEDIUM**
boa is ~8× slower than QuickJS and ~400× slower than V8 (doc 05). Very tight per-frame JS loops
or large interactive scenes could lag.
- **Mitigation:** keep the **render/animation math in Rust** (boa is the authoring/interactive
  layer, not the render hot path); expose heavy operations as engine commands; sandbox and bound
  scripts. Benchmark scripted playback in Phase 7 and document the envelope.
- **Residual risk:** compute-heavy interactive content is not the intended path; documented.

### R5. The deterministic `evaluate()` + caching — **MEDIUM**
Correctness of the pure evaluator (mask/guide/folder kinds, frame spans, nested symbol time,
tween interpolation, morph) and the soundness of the (symbol,frame)/(layer,frame) cache are the
core correctness surface.
- **Mitigation:** exhaustive golden tests per model feature; **determinism tests** (same doc+t →
  identical pixels); property tests for `Document::check`; the SWF round-trip (via Ruffle) as an
  independent oracle. Keep the display list `Arc`-shared (no per-frame deep clones — the failure
  EffectCraft fixed, doc 01 §3.2).
- **Residual risk:** subtle time/masking bugs in complex nested documents; caught by the oracle
  tests but require a good test corpus.

### R6. Timeline UI interaction complexity — **MEDIUM**
The Flash timeline (keyframe/span/tween dragging, retiming, onion skinning, layer types) is the
most novel UI (doc 06). Interaction correctness and performance (virtualized rows) are a
substantial effort.
- **Mitigation:** build on `craft-ui` list/scroll + EffectCraft's virtualized-timeline pattern;
  prototype the drag interactions early (Phase 4); every interaction is a command/`Action`
  (undoable, agent-replayable, testable without a UI).
- **Residual risk:** some timeline interactions may be simplified in v1.

### R7. Asset/icon licensing — **LOW-MEDIUM**
The tool/panel **icon set** must be original or open-licensed (no Adobe iconography — clean-room).
Authoring a consistent, professional icon set is a real deliverable.
- **Mitigation:** author original SVGs (code-generated preferred, the apps' convention) with an
  `ASSETS.md` attribution row each; use `craft-fonts` for any font. Gate on `cargo xtask assets`-
  style checks (sibling pattern).
- **Residual risk:** icon quality is a design task, not a technical blocker.

### R8. GPU investment (optional) — **LOW (if deferred)**
A wgpu compositor is significant work. **Recommendation: defer** (Phase 9.1, decision-gated).
- **Mitigation:** CPU (`vello_cpu`) reference first; GPU added behind the CPU oracle with
  parity tests (≤ 1/255). If deferred indefinitely, CPU-only is a valid v1 (the siblings ship CPU
  reference + GPU fast path; CPU-only is a smaller but complete product).
- **Residual risk:** large documents may be slower without GPU; mitigated by aggressive caching.

## 2. Uncertainties and missing information

- **U1. SWF versions 20–30.** Not publicly documented (loc.gov FDD). FlashCraft targets ≤ 19.
  **[INVESTIGATE]** whether the specific files a user cares about use post-19 features; if so,
  that is a separate spec-driven effort (or a Ruffle dependency for playback only).
- **U2. XFL quirks.** XFL is open but has known rough edges (shape value rounding, `bin/*.dat`
  media layout, `SymDepend.cache`, exact `keyMode` bit semantics). **[INVESTIGATE]** the exact
  `keyMode`/`keyFlags` bit layout and the `bin/` media indexing against the public XSD + a small
  corpus of user-owned XFL files (locally, never committed) before Phase 8.3.
- **U3. EffectCraft `keyframe` API fit.** We assume `effectcraft-keyframe`'s temporal/spatial
  model maps cleanly to Flash motion tweens + CreateJS easing. **[INVESTIGATE]** the exact API
  (ease representation, per-target easing) to confirm the CreateJS set slots in without forking
  the crate.
- **U4. Boolean-ops backend.** The suite uses kurbo + a boolean library (VectorCraft `pathops`).
  **[INVESTIGATE/DECISION]** whether to port VectorCraft's `pathops` or depend on a maintained
  boolean crate (e.g. a kurbo-based boolean) — affects Phase 3.5.
- **U5. Web authoring scope.** The web build (wasm32) — player only, or light authoring?
  **[DECISION-light]** recommend **player/light** for v1 (matching the siblings' web apps as a
  secondary surface); full web authoring is out of scope.
- **U6. Audio scope in v1.** Event sounds / streaming audio and audio export. **[DECISION-light]**
  recommend deferring audio to a Phase 8 sub-milestone (reusing FilmCraft codecs/cpal); a
  silent/vector-only v1 is acceptable.
- **U7. The provided reference screenshots.** `references/` holds four JPEGs (UI screenshots).
  Their provenance/license is unverified and their pixels were not analyzed. **[INVESTIGATE]**
  use them as local side-by-side references in Phase 1+ only; do not commit as assets.
- **U8. `craftrules` not local.** The shared standards repo (`craftrules`) referenced by the
  siblings is **not checked out locally**, so its `standards/*.md` (never-crash, engineering,
  licensing, fonts, test-corpora, progress-docs) were inferred from the sibling `AGENTS.md`/docs
  rather than read directly. The conventions used here are consistent with what the siblings
  state; **[INVESTIGATE]** confirm against `craftrules` before Phase 1 if strict compliance is
  required.

## 3. Legal / clean-room constraints (hard limits)

- **C1. No Adobe software.** Never run, script, screenshot, or reverse-engineer Macromedia Flash /
  Adobe Animate (or any Adobe app) to derive behavior; no output comparison against them.
  Behavior comes only from **public specifications** (SWF spec, XFL schema, CreateJS easing,
  AVM2/ABC overview) and **user-owned files used locally** (never committed). (Inherited from
  sibling `AGENTS.md`.)
- **C2. No FLA (classic binary).** Proprietary, no public spec → **not supported** (doc 07 §4).
  Interop is via XFL (the open FLA) and SWF.
- **C3. No SWF 20–30.** Undocumented → not targeted (doc 07 §2).
- **C4. No GPL/AGPL code** (Inkscape, lib2geom, etc.).
- **C5. Assets must be original (MIT/Apache) / OSI / CC0 / CC**, each with an attribution row;
  **fonts live in `craft-fonts`**, never in this repo; **no Adobe iconography/artwork**.
- **C6. Reuse Ruffle by dependency, not by copying its source** into this repo (git-pin idiom).
- **C7. ActionScript compatibility is future interop only** (via Ruffle for *playing* legacy SWF);
  FlashCraft-authored content is JavaScript (boa), a clean-room modern language.

## 4. What is solidly grounded (low risk)

- The **workspace/layering/commands/undo/never-crash/wasm/MCP/control-channel** conventions are
  proven across six apps (doc 01) — following them is low-risk.
- **`craft-ui`** docking/theme/icons and `craft-settings`/`craft-atomic`/`craft-quit` are mature
  shared crates (doc 02) — direct reuse is low-risk.
- **`vello_cpu`** CPU raster + kurbo/peniko + skrifa/harfrust text is the suite's proven vector
  stack (doc 04) — low-risk.
- **Ruffle `swf`** for SWF I/O is a published, MIT/Apache crate that reads and writes SWF 19
  (doc 07) — low-risk to depend on.
- **`effectcraft-time`/`keyframe`** for time/tweens are stable, low-churn primitives (doc 01/03) —
  low-risk (subject to U3 confirmation).
- **boa** scripting with the `__query`/`__exec` bridge is already used by EffectCraft and
  PdfCraft (doc 05) — low-risk pattern (subject to R4 throughput).

## 5. Summary of the highest-leverage decisions

The decisions that most shape the project (all in §7): the **native format** (D1), the **reuse
scope** (D2), the **scripting engine** (D3), the **interop scope** (D4), and the **GPU plan**
(D5). Getting these right early prevents rework; the technical risks (R1 morph, R2 SWF, R5
evaluator) are best de-risked by the roadmap's early `evaluate`+render (Phase 2) and the
Ruffle-as-oracle strategy.

## 6. Recommendations (recap)

1. Proceed with the doc 08 architecture (new `flashcraft-*` app; reuse craft-libs + Ruffle `swf`
   + `effectcraft-time`/`keyframe` + boa + vello_cpu; build the Flash document/evaluator/renderer/
   scripting-model natively).
2. De-risk in order: **`evaluate`+CPU render (Phase 2)** → timeline (4) → symbols (5) → tweens
   (6, motion first, morph behind a fallback) → scripting (7) → **SWF/XFL interop (8, with
   Ruffle as oracle)**.
3. Keep the product **2D vector animation + interactivity** (no 3D/particles), **CPU-first
   rendering**, **dark-mode-primary UI**, and **no Adobe-derived assets or reverse engineering**.
4. Hold the **approval gates** (doc 09 §12) — do not start any phase before its decision is made.

## 7. Decisions requiring user approval

> These are the items to resolve before (or at the start of) the gated phases. None is
> blocking for the *research* (this is complete); all are required before *implementation*.

| # | Decision | Options | Recommendation | Gate |
|---|----------|---------|----------------|------|
| **D1** | **Native document format** | (a) `.flashcraft` (versioned, compressed, structurally-shared JSON) primary + XFL/SWF interchange — **(recommended)**; (b) XFL-primary; (c) SWF-primary | (a) | Before Phase 1 / 2.2, 2.6 |
| **D2** | **Reuse scope** | (a) Depend on Ruffle `swf` + `effectcraft-time`/`effectcraft-keyframe` (git pins) + `boa` + `vello_cpu` + `kurbo`/`peniko`/`skrifa`/`harfrust` + `craft-libs` — **(recommended)**; (b) port/vendor time+keyframe locally; (c) also pin FilmCraft codecs for video | (a) (add FilmCraft pins in Phase 8) | Before Phase 1.1 |
| **D3** | **Scripting engine** | (a) **boa** (`boa_engine` 0.22) — **(recommended)**; (b) QuickJS (C, breaks pure-Rust); (c) V8 (huge, not wasm) | (a) | Before Phase 7 |
| **D4** | **Interop scope** | (a) SWF read-all + practical write (≤ 19) + XFL round-trip; **no FLA, no SWF 20–30** — **(recommended)**; (b) SWF-only; (c) broader (incl. post-19, future) | (a) | Before Phase 8 |
| **D5** | **GPU investment** | (a) CPU-first, defer wgpu to Phase 9 (decision-gated) — **(recommended)**; (b) CPU-only v1; (c) GPU in an earlier phase | (a) | Before Phase 9 |
| **D6** | **Audio & video scope in v1** | (a) Vector-only + image/video *export*; audio deferred to a Phase 8 sub-milestone — **(recommended)**; (b) include event/streaming audio in v1 | (a) | Before Phase 8 |
| **D7** | **ActionScript legacy** | (a) **Out of scope** for v1 (boa/JS only); future AVM1/AVM2/ABC interop via Ruffle — **(recommended)**; (b) attempt legacy playback in v1 | (a) | Confirm (non-gating) |
| **D8** | **Boolean-ops backend** | (a) port VectorCraft `pathops`; (b) depend on a maintained kurbo-based boolean crate — **[INVESTIGATE U4]** | (a) or (b) per U4 | Before Phase 3.5 |
| **D9** | **Morph-algorithm approach** | (a) motion-first, shape-tween behind a "matched-vertex" fast path, improve matcher later — **(recommended)**; (b) attempt full general morph first | (a) | Before Phase 6.3 |
| **D10** | **Web target scope** | (a) player/light-authoring wasm v1 — **(recommended)**; (b) full web authoring | (a) | Confirm (non-gating) |

**Until D1–D5 (and the phase-specific D6–D10) are approved, and until the user explicitly
authorizes a phase, no implementation begins.** The deliverable remains this Markdown research
package.