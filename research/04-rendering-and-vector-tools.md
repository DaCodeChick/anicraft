# 04 — Rendering Architecture and Vector Tools

> **Scope.** How AniCraft renders vector + animated content to pixels, the stage viewport, and
> vector editing. Verified from VectorCraft (`crates/render`, `crates/svg`, `crates/doc`,
> `crates/tools`), PhotoCraft (`crates/vector`), EffectCraft (`crates/render`, `crates/gpu`);
> recommendations for AniCraft are marked.

## 1. The vector primitive stack (reused, verified)

The suite builds vector art on a small, proven stack:

- **Geometry:** `kurbo` (0.13) — `BezPath` (cubic Béziers), `Affine`, `Rect`, `Point`, `Vec2`,
  `FillRule`. Used by VectorCraft (`vectorcraft-geom` on kurbo), PdfCraft, and EffectCraft
  (`effectcraft-path`). This is the natural geometry type for Flash shapes (SWF shapes are cubic
  Béziers; XFL edges are cubic curves).
- **Paint:** `peniko` (0.6) — `Fill`/`Stroke`/`Color`/`Gradient`/`BlendMode`, the paint model
  `vello_cpu` consumes. VectorCraft's renderer uses `vello_cpu::peniko`.
- **Raster:** `vello_cpu` (0.2) — a SIMD, multithreaded, sparse-strip CPU rasterizer.
  VectorCraft `crates/render/src/lib.rs`: "The backend is `vello_cpu` (SIMD, sparse strips).
  Callers give a *view transform* mapping document points to output pixels; the renderer culls by
  bounds, evaluates appearance stacks (multiple fills/strokes, opacity, blend modes, stroke
  alignment, dashes), clip groups, gradients, images and text."
- **Text:** `skrifa` (font access) + `harfrust` (shaping) — used by VectorCraft
  (`vectorcraft-text`), EffectCraft, and FilmCraft (`filmcraft-text`). Glyph outlines are
  available for vector text; system fonts are scanned lazily.

**Recommendation.** AniCraft reuses `kurbo` + `peniko` + `vello_cpu` + `skrifa`/`harfrust`
directly (they are already the suite's choice and build for wasm). No new geometry/paint/raster
engine is needed. A thin `anicraft-geom`/`anicraft-color` (or direct kurbo use) sits at L0,
consistent with the per-app foundation pattern (doc 01 §4).

## 2. VectorCraft's renderer as the reference pattern (verified)

VectorCraft's canvas is rasterized **on the CPU by `vello_cpu` on worker threads, off the UI
thread, with caches per object and per effect; the GPU (wgpu) only composites the result**
(`vectorcraft/docs/architecture.md` "Rendering"). The render crate:

- Takes a **view transform** (document → output pixels) and a `Pixmap`/`RenderContext`/
  `Resources` from `vello_cpu`.
- Walks the document `Node` tree, **culls by bounds**, and emits peniko paint for each object's
  appearance stack (fills, strokes, opacity, blend, dashes, stroke alignment), clip groups
  (first child clips), gradients, placed images, and text (via glyph outlines).
- Raster **effects** (shadows, glows, blurs) render offscreen per object and are cached while
  panning.

This "CPU raster + culling + appearance stacks + offscreen effect caches" is exactly the pattern
AniCraft's renderer should follow for the vector stage.

## 3. EffectCraft's CPU-reference / GPU-compositor pattern (verified)

EffectCraft and FilmCraft both use a stronger split:

- A **CPU reference compositor** is the oracle (EffectCraft `crates/render`: source → masks →
  effects → styles → transform → matte → blend, with a **layer cache** keyed by a content hash;
  motion-blur sub-sampling; an `Accelerator` trait).
- A **GPU compositor** (`crates/gpu`, wgpu compute) implements the same steps and is **tested
  against the CPU pixel-for-pixel** (≤ 1/255 at 8 bpc). `RenderOpts::backend = Cpu | Gpu | Auto`.
- A **frame-worker pool** (`ui-egui/src/frames.rs`) renders the on-screen frame first, then
  playback prefetch, then thumbnails; the UI never blocks on a frame.

**Recommendation.** AniCraft starts **CPU-first** (`vello_cpu` reference, with a per-layer/per-
symbol cache keyed by content hash + time), and defers a wgpu fast path to a later phase behind
the same oracle (the CPU/GPU parity test discipline). This matches the suite and de-risks the
initial release (doc 08, doc 10).

## 4. AniCraft's rendering architecture (recommendation)

Rendering is a **compositor over the deterministic display list** produced by timeline evaluation
(doc 03 §4.4). Pipeline:

```
Document + time t  ──evaluate()──►  DisplayList (stage → layers → placed shapes/instances,
                                      each with effective matrix, color, blend, filters, text)
                                     │
                                     ▼
                 anicraft-render (vello_cpu, CPU reference)
                   • build a peniko scene from the display list
                   • cull by the viewport bounds
                   • cache per (symbol id, local frame) and per (layer, frame) content hash
                   • rasterize gradients/fills/strokes/text/images
                   • apply filters (blur, drop shadow, glow, bevel) offscreen + cached
                   • blend modes + opacity
                                     │
                                     ▼
                 Pixmap (premultiplied RGBA)  ──►  egui canvas (composited by wgpu/egui)
```

Key points:

- **`evaluate()` is pure and cached.** Because the display list is a function of (document, `t`),
  a symbol rendered at a local frame can be cached and reused across every instance and every
  scrub position that reaches the same (symbol, local frame) — the Flash analogue of EffectCraft's
  layer cache. Static content (a graphic symbol, an un-tweened instance) renders once.
- **The viewport is a view transform.** Zoom/pan is a document→pixel `Affine`; the renderer culls
  by the visible rectangle. The **pasteboard** (area outside the stage) is drawn dimmed; the
  **stage rectangle** is the output bounds. "Clip to stage" just changes the cull rect.
- **Onion skinning** = evaluate the document at `t ± k` frames, render those display lists at low
  alpha (previous onions in one hue, next in another), and composite *under* the current frame.
  Because `evaluate()` is pure, this is trivially uniform with rendering/scrubbing/export.
- **Playback/scrub** reuses the frame-worker pool pattern (on-screen frame first, prefetch
  around the playhead). Determinism tests assert the same (document, `t`) renders identical
  pixels across runs and (later) across CPU/GPU.

## 5. Vector editing operations (recommendation, patterns from the suite)

VectorCraft's **tools** model is the template: tool state machines emit `Action`
(`Begin`/`Preview`/`Commit`/`Cancel`/`Exec`/`Dialog`/`SwitchTool`/`Notify`) that the engine
executes, so every gesture is one replayable, testable, undoable unit (VectorCraft
`crates/tools/src/lib.rs`). AniCraft's tools map to Flash's:

| Flash tool | AniCraft tool (Action-based) | Notes |
|------------|-------------------------------|-------|
| Selection (V) | select | marquee + object selection; fill/stroke/whole-object sub-selection (Flash's 4 selection modes) |
| Subselection (A) | direct | edit path vertices/handles (kurbo) |
| Lasso (L) | lasso | freehand/polygon/magic-wand selection |
| Pen | pen | draw Bézier paths (VectorCraft's pen pattern) |
| Line (N) / Rectangle (R) / Oval (O) / PolyStar | shape | live shapes (PhotoCraft `vector::shapes` pattern: rect/ellipse/polygon/star/line) |
| Text (T) | text | point + area text (skrifa/harfrust shaping; vector or outlined glyphs) |
| Paint Bucket (K) / Ink Bottle (S) | fill / stroke | apply fill/stroke (Flash's fill+stroke "paint proxy") |
| Eyedropper (I) | eyedropper | sample color/attributes |
| Eraser (E) | erase | erase a region of the current frame's shapes |
| Free Transform (Q) | transform | move/rotate/scale/skew/reflect (kurbo `Affine`) |
| Gradient Transform (F) | gradient | edit gradient matrix |
| Hand (H) / Zoom (Z) | pan / zoom | viewport view transform |

- **Transformation** is an `Affine` on the selected item(s)/instance (matrix), editable via the
  Free Transform tool or numeric fields (Properties panel). For a **symbol instance**, the
  transform is the instance's `matrix` (independent of the definition); for **shapes**, it is
  baked into the path or stored as a shape transform.
- **Grouping** creates a `DOMGroup`/`Group` node (members keep their relative transforms).
  Flash's **Merge vs Object** drawing mode controls whether new paint merges into existing shapes
  (Merge) or creates discrete objects (Object).
- **Selection feedback** uses theme tokens (selection outline, anchor handles) — doc 06.
- **Hit testing** is against the *current frame's* display list (shapes by fill/stroke coverage;
  instances by their bounds/shape), so it is uniform with rendering.

## 6. Text rendering (reused)

- Shape with `harfrust` (kerning, bidi, complex scripts), access glyphs with `skrifa`.
- Render as **vector glyph outlines** (scalable, matches Flash's "text as vector") or outline to
  paths for editing; embed/subset fonts via `craft-fonts` for export.
- Static text (a `Text` character, SWF `DefineText`) vs editable text (SWF `DefineEditText`)
  map to Flash's static/dynamic text.

## 7. What is reused vs. new

| Piece | Reused | New in AniCraft |
|-------|--------|-------------------|
| Geometry / paint types | kurbo, peniko | none (thin wrapper) |
| CPU rasterizer | `vello_cpu` | the display-list→scene walk (Flash-specific) |
| Culling + appearance stacks + offscreen filter caches | VectorCraft render pattern | Flash filters (blur/drop-shadow/glow/bevel/gradient glow) as `vello_cpu`-compatible passes |
| Text shaping + glyph outlines | skrifa, harfrust, `craft-fonts` | Flash text objects (point/area, static/dynamic) |
| Live shapes | PhotoCraft `vector::shapes` pattern | Flash primitive shapes (rect/oval/polygon/star/line) |
| Tool→command state machines | VectorCraft `tools` pattern | Flash tool set (V/A/L/pen/shapes/text/bucket/ink/eyedropper/eraser/transform/pan/zoom) |
| CPU-reference/GPU + frame-worker pool | EffectCraft/FilmCraft pattern | deferred wgpu compositor + playback pool |
| Per-(symbol,frame) / per-(layer,frame) render cache | EffectCraft layer-cache pattern | the **deterministic `evaluate()` + display-list cache** |
| Onion skinning | — | frame-ghost rendering option |
| Shape morph rendering | SWF `DefineMorphShape` (read) | morph display (doc 10) |

## 8. Performance considerations

- **Caching is everything.** The display list is derived, so the renderer must cache aggressively:
  per (symbol id, local frame) for symbols, per (layer, frame) for layers, per (object, effect).
  EffectCraft's numbers (a 5,000-layer comp steady frame 0.5–6.9 ms after fixing O(n²) menu
  builds and per-frame deep clones) show the discipline pays off. AniCraft should keep
  `Arc`-shared display-list nodes (no per-frame deep clones) and virtualized timeline rendering.
- **The render hot path stays Rust.** Scripting (boa) and event dispatch stay out of the per-frame
  render (doc 05) so the CPU rasterizer's throughput is not throttled by JS.
- **Deterministic + reproducible.** Same (document, `t`, view) → identical pixels; exports are
  reproducible (FilmCraft convention). This is a first-class test, not a side effect.