# 05 — Scripting and Interactive Runtime

> **Scope.** Research on the scripting system for AniCraft's interactive content: JavaScript
> runtimes, the script-to-native bridge, the Flash object model, sandboxing, and legacy
> ActionScript interop. Verified from EffectCraft (`crates/expr`, `crates/script`), PdfCraft
> (`crates/js`), and Ruffle (`core/src/avm1`, `core/src/avm2`); runtime comparisons are external
> (benchmarks/conformance). Recommendations are marked.

## 1. The authoring/runtime split (principle)

Flash distinguishes the **authoring document** (what is stored: stage, timelines, symbols,
keyframes, and *where* scripts attach) from the **interactive runtime** (what happens at
playback: the live display list, per-frame state, event dispatch, and running scripts).

**Recommendation (carried through doc 08):** AniCraft keeps that split. Scripts are *referenced*
by the document (a frame, button, or movie-clip instance carries a `script` id or inline source)
but **never executed during document editing**. The runtime builds the display list from
`evaluate()` (doc 03), installs the scripting object model over it, and runs scripts in a
sandboxed engine. This keeps the document serializable/undoable and the renderer unaware of
scripting — the task's requirement that "scripting can remain separate from the document model and
rendering backend."

## 2. JavaScript runtime comparison (external)

Embeddable options, with the tradeoffs that matter for a pure-Rust, cross-platform (incl.
wasm32), no-`unsafe` Craft app:

| Engine | Language | wasm32 | Conformance | Relative speed* | Notes |
|--------|----------|--------|-------------|-----------------|-------|
| **Boa** (`boa_engine`) | Rust | **Yes** (`js` feature) | >90% ES (published dashboard) | ~105–107 (≈1/8 of QuickJS, ≈1/400 of V8) | **Pure Rust**, no C toolchain, no `unsafe`. **Already the Craft standard** (EffectCraft, PdfCraft use `boa_engine 0.22`). |
| QuickJS / rquickjs | C (Rust binding) | Yes (wasm build) | High (ES2023) | ~829–835 (fast interpreter) | Small, fast, mature — but a **C dependency + build step**, breaks the pure-Rust/`forbid unsafe` convention unless isolated. |
| V8 (`rusty_v8`) | C++ | No (host-only) | Effectively complete | ~44,000+ (fastest) | Peak throughput + npm ecosystem, but **huge** (prebuilt binaries, ~50 MB+), C++, JIT, not pure Rust, no wasm32 target. |
| Deno (`deno_core`) | Rust (over V8) | No | Complete + Web APIs | V8-class | Full Web APIs but **heavy** (V8 + Deno extensions); a runtime, not an embeddable library. |
| Wasmer (JS/Wasm runtimes) | Rust | — | varies | — | Oriented to running Wasm/JS in cloud; not the suite's idiom. |
| Zipp | Rust | Yes | ES2025 (JIT tiers) | competitive on short jobs | Newer (2026), not yet the ecosystem idiom; not on crates.io as a drop-in. |

\* Speedometer-class figures from public engine benchmarks (boa ~105–107; QuickJS ~829–835;
V8 ~44,000+), plus a head-to-head worker benchmark (QuickJS exec ~12 µs, V8 ~32 µs, Boa ~12 µs on
simple workloads, V8/Deno pass more conformance tests). Treat as orders of magnitude.

**Decision.** **Use boa (`boa_engine` 0.22).** Rationale:
1. **It is the suite standard.** EffectCraft (expressions + full scripting + ScriptUI) and
   PdfCraft (Acrobat JS) both already depend on `boa_engine 0.22` (verified in their
   `Cargo.toml`). Adopting it keeps AniCraft idiomatic and means the object-model and
   sandboxing patterns are already proven in this codebase family.
2. **Pure Rust + wasm32.** Boa builds for `wasm32-unknown-unknown` (with its `js` feature for the
   browser clock/random), needs no C/C++ toolchain, and has no `unsafe` — matching the
   workspace's `unsafe_code = "forbid"` and the "pure Rust (no handwritten JS/TS)" and
   `cargo xtask wasm` conventions. QuickJS/V8/Deno all violate one of these.
3. **Performance is adequate for the role.** AniCraft scripts run in the **authoring and
   interactive layers**, not the per-frame render hot path (which stays Rust, doc 04). Authoring
   scripts (build a scene, keyframe properties) and interactive handlers (button clicks, simple
   per-frame logic) are short-lived; boa's interpreter speed is acceptable. Heavy computation
   belongs in Rust, exposed to scripts as commands.

**Tradeoff accepted:** boa is markedly slower than QuickJS/V8 for compute-heavy JS. Mitigation:
keep the render/animation math in Rust, expose it to scripts via the command/query bridge (§3),
and document that very tight per-frame JS loops are not the intended path.

## 3. The script-to-native bridge (verified pattern → recommendation)

EffectCraft's `effectcraft-script` (`crates/script/src/{lib.rs,runtime.rs,prelude.js}`) is the
model. Verified facts:

- **Object model in JS, engine in Rust.** A `prelude.js` (included via `include_str!`) defines the
  whole object model (`app`, `Project`, `ItemCollection`, `CompItem`/`FootageItem`/`FolderItem`,
  `LayerCollection`, `AVLayer`/`TextLayer`/`ShapeLayer`/…, `PropertyGroup`/`Property`,
  `KeyframeEase`, `MarkerValue`, `RenderQueue`, `File`/`Folder`, `ScriptUI`).
- **Two native bridges:** `__query` returns **JSON snapshots** of the model (items, layers,
  property nodes, values, keyframes); `__exec` **runs an engine command**. "Edits are engine
  commands: every mutating call runs a command through `Session::execute`, so it is undoable,
  journaled and identical to the UI's action."
- **Undo groups:** `app.beginUndoGroup`/`endUndoGroup` fold the steps in between into one named
  undo step.
- **Session hand-off:** while a script runs, the `Session` is moved into a **thread-local**
  (`ACTIVE`) the natives borrow — "nothing borrowed enters the JS heap." The Script Console keeps
  one persistent context; a run that hits the stack limit gets a fresh context and one retry.
- **Security:** `File` reads limited to the project folder; writes and network refused unless a
  Preferences toggle is on.
- **Entry points:** the `script.run` command, File ▸ Scripts ▸ Run Script File (`.jsx`/`.js`),
  Window ▸ Script Console, the CLI, and the MCP `run_script` tool.
- **Match names:** a table maps properties/effects to After Effects' documented match names.

PdfCraft's `pdfcraft-js` (verified `Cargo.toml`) is the same idea at smaller scale: boa
(feature `annex-b`) with a **bounded** Acrobat-JS object model subset (`event`, `getField`, `app`,
`util`, `console`).

**Recommendation for AniCraft.** Replicate this architecture (it is exactly the
"script-to-native application interface" the task asks about):
- `anicraft-script` crate: a `prelude.js` defining the **Flash object model** (§4) + a
  `runtime.rs` with `__query`/`__exec` natives, a thread-local active session, undo groups, and
  error reporting.
- **Scripts read via JSON queries and mutate via engine commands** → scripted edits are undoable,
  journaled, and identical to UI edits; the document model and renderer never see the JS heap.
- **Sandboxing:** a fresh boa `Context` per run (persistent console context for the interactive
  console); `File` reads confined to the document folder; writes/network behind a Preferences
  toggle; no clock/random leaks; a bounded recursion/stack with a fresh-context retry.
- **Separation:** the runtime builds over `evaluate()`'s display list (doc 03/04); rendering is
  not touched by scripts except through commands (e.g. `gotoFrame`, `setPlayback`).

## 4. The Flash scripting object model (recommendation)

Modeled on the Flash/Animate (AS3) display-list API that Ruffle implements (verified present in
`ruffle/core/src/avm2` and `core/src/display_object`), but exposed to **JavaScript** (not AS3):

- **Root / stage:** `stage` (width, height, frameRate, backgroundColor, `currentTime`,
  `currentFrame`, `play()`, `pause()`, `gotoAndPlay/gotoAndStop(frame|label)`), `library`.
- **Display objects:** `DisplayObject` (x, y, scaleX/scaleY, rotation, alpha, visible,
  blendMode, name, parent, addChild/removeChild, contains, hitTest), `MovieClip` (a `DisplayObject`
  that is also a `Timeline`), `Sprite` (movie clip), `Graphic`, `Shape`, `Bitmap`, `Text`,
  `Button` (up/over/down/hit states, `useHandCursor`), `Stage`.
- **Timeline:** `Timeline` / `MovieClip` (`totalFrames`, `currentFrame`, `currentLabel`,
  `addFrame`, `addFrameLabel`, `gotoAndPlay/Stop`, `play/pause`, `enterFrame`/`frameLabel`
  events, `frameRate`).
- **Library:** `Library` (`getDefinition(id)`, `addDefinition`, `removeDefinition`, iteration),
  `SymbolDefinition` types (Graphic/MovieClip/Button/Bitmap/Sound/Font/Shape/Video).
- **Properties as keyframeable values:** each animatable property exposes a keyframe array
  (`setKeyframes`, `valueAtTime`, `addKeyframe`) reusing `anicraft-keyframe` (doc 03/08), so
  scripts can author motion tweens the same way the UI does.
- **Events:** `addEventListener(type, handler)` / `removeEventListener`, event objects
  (`Event.ENTER_FRAME`, `Event.FRAME_CONSTRUCTED`, mouse events, button states). Dispatch is
  driven by the runtime's frame loop (Ruffle's Enter/Construct/FrameScripts/Exit phases, verified
  in `frame_lifecycle.rs`, are the reference ordering).

The **AS3 object model is the reference for names/semantics** (it is what Ruffle implements and
what classic Flash content expects); **JavaScript is the syntax** that AniCraft executes (boa).
This gives a modern scripting language with the familiar Flash object graph.

## 5. Event dispatch, timeline control, and playback

- **Frame loop (runtime):** each display tick, the runtime (a) advances the stage timeline by one
  frame (or to the playhead), (b) runs the frame phases (enterFrame → construct → frame scripts →
  exitFrame, mirroring Ruffle's `FramePhase`), (c) dispatches events, (d) renders via
  `anicraft-render`. Deterministic and decoupled from the UI refresh.
- **Timeline control** from scripts: `gotoAndPlay/Stop`, `play/pause`, set `currentTime`,
  add/replace frames — all as **engine commands** (undoable when authoring) or **runtime state**
  (during playback, not undoable). The distinction: *authoring-time* scripts (run from the editor,
  edit the document) vs *playback-time* scripts (run in the runtime, change runtime state).
- **Movie-clip scripting:** each movie clip instance has its own timeline and its own event
  scope (its scripts see `this` = the clip). Nested clips run their own frame loops, exactly the
  SWF `Sprite`/XFL nested-timeline model (doc 03).
- **Runtime state** (playhead, per-clip current frame, button hover state, loaded assets) lives in
  the **runtime**, never in the document.

## 6. Export and standalone playback targets (recommendation)

- **Web-compatible export:** export a standalone **HTML/JS/WASM** package: the document serialized
  to JSON + a small WASM (or JS) runtime (boa or a JS port) + the display-list renderer, or simply
  **compile the animation to a self-contained JS bundle** (Lottie is a natural bonus — EffectCraft
  already exports Lottie, doc 01/03). Because the whole stack builds for wasm32 (doc 01), a
  browser player is achievable from the same `anicraft-render` + runtime.
- **Native standalone playback:** a `anicraft-cli play` / a headless player that loads a
  `.anicraft` (or SWF) and runs the runtime + renderer with audio (cpal), for "publish" output
  that is not the authoring app.
- **Deterministic export:** the same document exports the same frames (FilmCraft reproducible-
  export convention), enabling byte-stable sprite sheets / video / image-sequence output.

## 7. TypeScript authoring (optional)

boa runs **JavaScript**, not TypeScript. For a TypeScript authoring experience, the standard
approach is to **compile TS → JS at authoring time** (a build step, e.g. `tsc`/esbuild producing
JS that boa executes). This keeps the runtime JS-only (matching the "no handwritten JS/TS in the
app" rule — the *app* is Rust; user scripts are an input). **Recommendation:** accept JavaScript
natively; document an optional TS→JS build path for users who prefer TS. Do **not** embed a TS
compiler in the app.

## 8. Legacy ActionScript interop (future, out of initial scope)

- **ActionScript 1/2** (AVM1) and **ActionScript 3** (AVM2, ECMAScript-based, executed as **ABC
  bytecode**) are the legacy Flash scripting languages. The **ABC format is fully documented**
  (Adobe `avmplus` `abcFormat-46-16.txt`; major 46 / minor 16; ~164 opcodes), and **Ruffle
  implements both AVM1 and AVM2** (verified `ruffle/core/src/avm1`, `ruffle/core/src/avm2`).
- **Recommendation:** for the **initial release**, AniCraft scripts are **JavaScript (boa)**
  with the Flash object model (§4) — a modern, clean-room scripting system. **ActionScript
  compatibility is a documented *future interoperability* option**, achievable by running legacy
  ABC through Ruffle's AVM2 (or a dependency on it) when *importing/playing SWF*, not by
  requiring AniCraft-authored content to be AS. This satisfies the task: "Legacy ActionScript
  compatibility may be investigated as a future interoperability feature, but it is not a
  mandatory initial implementation requirement."
- **Clean-room note:** implementing a *new* JS object model and a JS runtime is clean-room.
  *Playing* legacy SWF/ABC is best done by **depending on Ruffle** (MIT/Apache) rather than
  re-implementing the AVM, consistent with the git-pin reuse convention (doc 07).

## 9. Risks / open questions (→ doc 10)

- boa throughput for large interactive scenes (mitigated by keeping heavy math in Rust).
- Deciding the **exact object-model surface** for v1 (a bounded subset like PdfCraft, vs. a
  broad AS3-like surface) — affects scripting API stability.
- Whether to **embed Ruffle** for legacy SWF/ABC playback (full F1–F30 playback) or keep
  AniCraft's player to its own content + SWF import only.
- Web export target: WASM runtime vs. compiled Lottie/JS — a Phase 8 decision.

## 10. Summary of decisions

| Decision | Recommendation | Basis |
|----------|----------------|-------|
| Scripting language | **JavaScript** (boa), optional TS→JS build | suite standard, modern, clean-room |
| JS engine | **`boa_engine` 0.22** | pure Rust, wasm32, no unsafe, already used by EffectCraft/PdfCraft |
| Bridge | **`__query` (JSON) / `__exec` (commands)** + thread-local session + undo groups | EffectCraft pattern; keeps scripts off the doc/renderer |
| Object model | Flash/AS3-style display list (Stage/MovieClip/Sprite/DisplayObject/Timeline/Library/Button/Text) in JS | Ruffle's AVM2/display_object as the reference; modern syntax |
| Sandbox | fresh boa context per run; file-scoped reads; writes/network behind a pref; stack retry | EffectCraft + PdfCraft |
| Authoring vs playback scripts | authoring-time = engine commands (undoable); playback-time = runtime state | doc/runtime split |
| Web / native playback | WASM/JS or Lottie web export; headless native player | wasm32 stack + EffectCraft Lottie |
| ActionScript | **future interop** via Ruffle (AVM1/AVM2/ABC), not an initial requirement | task scope + clean-room |