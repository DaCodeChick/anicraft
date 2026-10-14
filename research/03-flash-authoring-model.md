# 03 — Flash Authoring Model

> **Scope.** The document, timeline, symbol, and animation model AniCraft must author. Distinguishes
> **verified** (Ruffle `swf`/`core` source), **external** (SWF spec, XFL schema, public Flash
> documentation), and **recommendations**. See doc 07 for format/persistence details and doc 08 for
> how this maps to crates.
>
> Verified sources: `ruffle/swf/src/{types.rs,tag_code.rs}`, `ruffle/core/src/{frame_lifecycle.rs,
> display_object/*, library.rs}`. External: SWF File Format Specification v19 (Adobe, 2012), the
> public XFL XSD (friction2d/xfl-schema), and classic Flash/Animate workspace documentation.

## 1. The core Flash model (external, well-documented)

A Flash/Animate document is a **stage** (a fixed-size rectangle, in pixels) with a **timeline**
organized into **scenes → layers → frames**, a **library** of reusable **symbols**, and optional
**actions** (scripts). The document "measures time in frames"; the **playhead** advances through
frames at the document's **frame rate**.

Key concepts (classic Flash/Animate; consistent with the SWF and XFL structures in §2–§4):

- **Stage**: the visible rectangle (e.g. 800×600). Surrounding gray area is the **pasteboard**
  (authoring space, not shown in output). Document settings: size, frame rate, background color.
- **Scenes**: a document has one or more scenes; each scene has its own timeline (layer set).
- **Layers**: transparent sheets stacked; each has name, a color (shown in the timeline header),
  and visibility/lock/outline (guide) controls. Layer **types**: normal, **guide**, **folder**,
  **mask**, **masked** (a mask layer clips the layer(s) below it).
- **Frames**: discrete slices of time. Kinds:
  - **Keyframe** (solid dot): holds new content or a new value.
  - **Blank keyframe** (hollow dot): holds nothing (clears the layer at that frame).
  - **Frame span / hold** (filled cells): no change; the layer keeps showing the last keyframe's
    content ("frame spans").
- **Keyframe values**: what changes on a keyframe — a placed symbol instance's transform, a
  shape, opacity, color, a filter, a tween, a script, etc.
- **Symbols**: reusable library definitions:
  - **Graphic**: a timeline that plays **with** the main timeline (can't contain script, no
    independent playback).
  - **Movie Clip**: an **independent** timeline (plays on its own, can contain script, can
    instantiate other clips).
  - **Button**: four (or five) frames — **Up / Over / Down / Hit** (Hit is an invisible click
    region) — with optional per-state sound and actions.
  - Non-timeline assets: **bitmap**, **sound**, **font**, **shape**, **video**.
- **Symbol instances**: placed on a layer; reference a definition by identity and carry
  **independent** properties: transform (matrix), color transform (tint/alpha), blend mode,
  **instance name**, filters, and (movie clip) loop mode / first frame / playback state. Instances
  can be **nested** (a movie clip's timeline contains instances of other symbols).
- **Motion tween**: animate an instance's **transform** (position, scale, rotation, skew),
  **color**, and **filters** between two keyframes.
- **Shape tween**: **morph** between two **shapes** (edit vertices/fills so one shape becomes
  another). (XFL: `tweenType="shape"`; SWF: `DefineMorphShape`.)
- **Easing**: an acceleration value (classic Flash: −100…100; negative = ease-in, positive =
  ease-out). Modern Animate uses **CreateJS** easing functions (quad/cubic/quart/quint/sine/
  circ/expo/back/elastic/bounce × In/Out/InOut) plus custom piecewise curves (XFL `<Ease>` /
  `<CustomEase>`).
- **Onion skinning**: draw faint ghosts of adjacent frames to aid hand animation.
- **Drawing modes**: **Merge** (painting combines shapes) vs **Object** (painting makes
  discrete objects).
- **Interactivity**: buttons respond to mouse states; movie clips run scripts; the **Actions**
  panel edits per-frame / per-button / per-clip code.

## 2. How the model appears in SWF (verified in Ruffle)

Ruffle's `swf` crate models the SWF 19 file (see `ruffle/swf/src/types.rs`). The authoring model
maps onto **characters** (library definitions) and a **display list** driven by tags:

- **Header** (`Header`): `compression`, `version: u8`, `stage_size: Rectangle<Twips>`,
  `frame_rate: Fixed8`, `num_frames: u16`. Coordinates are **twips** (1 twip = 1/20 px); the
  matrix is 16.8 fixed-point (`a,b,c,d`) with 11.5 fixed-point translation (`tx,ty`).
  `FileAttributes` (tag 69) is required since v8 and must be first.
- **Tags** (`tag_code.rs` `TagCode`): the file is a sequence of tags. The ones that carry the
  authoring model:
  - **Definitions (library):** `DefineShape2/3/4` (vector shapes), `DefineSprite` (a movie
    clip: `Sprite { id, num_frames, tags }` — a nested timeline), `DefineButton2`,
    `DefineMorphShape2` (shape morph), `DefineText2`/`DefineEditText` (text),
    `DefineFont2/3/4` + `DefineFontInfo/Name`, `DefineBitsJpeg2/3/4` /
    `DefineBitsLossless2` (bitmaps), `DefineSound`, `DefineScalingGrid`,
    `DefineBinaryData`, `DefineVideoStream`.
  - **Display list:** `PlaceObject2/3/4` (place a character at a `depth` with `matrix`,
    `color_transform`, `ratio` (morph position), `name` (instance name), `clip_depth`,
    `class_name`, `filters`, `blend_mode`, `is_visible`, `clip_actions`), `RemoveObject2`,
    `ShowFrame` (frame boundary), `FrameLabel`, `SetBackgroundColor`.
  - **Actions/AS3:** `DoAction` (AS1/2), `DoInitAction`, `DoAbc`/`DoAbc2` (AS3 ABC),
    `SymbolClass` (link a symbol id to an AS3 class), `Metadata`.
  - **Assets:** `ExportAssets`/`ImportAssets` (linkage names), `NameCharacter`.
- **Shape** (`Shape`): `id`, `shape_bounds`, `edge_bounds`, `styles: ShapeStyles
  { fill_styles: Vec<FillStyle>, line_styles: Vec<LineStyle> }`, and `shape: Vec<ShapeRecord>`
  where `ShapeRecord = StyleChange | StraightEdge | CurvedEdge`. `FillStyle = Color |
  LinearGradient | RadialGradient | FocalGradient | Bitmap`. `LineStyle` = width, fill, and flags
  (caps, joins, miter limit, pixel hinting, scale-lock). This is the vector primitive AniCraft
  must read/write and (for authoring) edit.
- **PlaceObject** (`PlaceObject`): `version, action, depth, matrix, color_transform, ratio,
  name, clip_depth, class_name, filters, background_color, blend_mode, clip_actions, is_visible`.
  This is the **instance** in file form: a `depth` (z-order) + character id + independent
  transform/color/name/filters. `ratio: u16` is the morph-interpolation position for
  `DefineMorphShape`.
- **Sprite** = a movie clip: its `tags` are a self-contained mini-timeline (nested
  `PlaceObject`/`ShowFrame`), giving Flash's **nested animation** for free.
- **Button** (`Button`): records for the Up/Over/Down/Hit frames (each a set of placed
  characters), matching the four-state button model.

**Runtime (verified, `ruffle/core`)** — how the file becomes a live display list
(`frame_lifecycle.rs`): AVM2 frames run phases **Enter → Construct → FrameScripts → Exit → Idle**,
firing `enterFrame` / `frameConstructed` / `exitFrame`; `PlaceObject` tags execute at
**Construct**; a `MovieClip` owns its display list by **depth**; the `library` holds characters.
This is the *playback* side — AniCraft's authoring document is the *source* of these tags, and
its renderer reproduces the same display-list construction deterministically (doc 04).

## 3. How the model appears in XFL (external, open)

XFL is an **open XML** authoring format — a ZIP of `DOMDocument.xml`, `LIBRARY/*.xml` (one file
per symbol), `bin/*.dat` (binary media), and a `mimetype`. Namespace
`http://ns.adobe.com/xfl/2008/`; a public XSD exists (friction2d/xfl-schema). Modern FLA (CS5+) is
the same structure compressed. Structure (from the XSD and public write-ups):

```
DOMDocument (width, height, frameRate, backgroundColor, currentTimeline)
 ├─ media            DOMBitmapItem / DOMSoundItem / DOMVideoItem (hrefs into bin/)
 ├─ symbols          Include href="LIBRARY/Name.xml" (library index)
 └─ timelines        DOMTimeline (name = scene)
      └─ layers       DOMLayer (name, color, visible, locked,
                            layerType = normal|guide|folder|mask|masked, parentLayerIndex)
           └─ frames    DOMFrame (index, duration, keyMode,
                                  tweenType = shape|motion, acceleration, <tweens>)
                └─ elements  DOMSymbolInstance | DOMShape | DOMGroup | DOMText | ...
```

- **DOMSymbolInstance**: `libraryItemName`, `symbolType = graphic|movieclip|button`, `loop`,
  `firstFrame`, `<matrix>` (a,b,c,d,tx,ty), `<transformationPoint>`, optional `<color>`.
  This is the **instance** in XFL form (references a library symbol; independent matrix/
  transform-point/color).
- **DOMShape**: `<fills>` (`FillStyle index=…` → `SolidColor` / `LinearGradient` /
  `RadialGradient` with `GradientEntry`), `<strokes>` (`StrokeStyle` → `SolidStroke
  weight/caps/joints`), `<edges>` (`Edge fillStyle0/fillStyle1/strokeStyle` with a path encoding).
  Edges are **shared between two fill regions** (stored once) — the same edge-based model as
  SWF shapes.
- **DOMGroup**: `<members>` (nested `DOMShape`/`DOMSymbolInstance`/`DOMGroup`) — Flash's "Group".
- **Tweening**: `DOMFrame` carries `tweenType="motion"|"shape"`, `acceleration`, and `<tweens>`
  with `<Ease target method|intensity>` or `<CustomEase>` (piecewise cubic, 3n+1 points).
  **Easing is CreateJS** (see §1): `back` overshoot 1.7, `backInOut` ×1.525,
  `elasticInOut` period 0.45. Per-target eases exist
  (`target="all"|"position"|"rotation"|"scale"|"color"|"filters"`).
- **Symbol definition files** (`LIBRARY/*.xml`): root `DOMSymbolItem` with `symbolType`,
  `name`, and a `<timeline>` (graphic/movie clip) or the button states — i.e. each symbol is its
  own mini-document, exactly like an SWF `DefineSprite`.

XFL is the natural **authoring interchange**: it round-trips the full model (layers, frames,
tweens, symbols, instances, media) in a readable, schema-described, open format — and is the
basis for SWF publishing (Animate publishes XFL→SWF).

## 4. The model AniCraft must author (recommendation)

AniCraft's document model should be a **modern, editable, undoable** representation of the §1
model that can **emit SWF tags** and **round-trip XFL**. Design points (detail in doc 08):

1. **Separate the library from the timelines.** A `Library` maps `SymbolId → SymbolDefinition`.
   `SymbolDefinition` is one of:
   - `Graphic { timeline: Timeline }` (plays with the parent timeline),
   - `MovieClip { timeline: Timeline, default_loop: bool }` (independent timeline),
   - `Button { up/over/down/hit: FrameContent, … }`,
   - `Bitmap { … } | Sound { … } | Font { … } | Shape { … } | Video { … }`.
   A **Timeline** is `{ layers: Vec<Layer>, frame_rate, … }`; a **Layer** is
   `{ id, name, color, visible, locked, guide, kind: Normal|Folder|Mask|Masked, parent,
   frames: Vec<Frame> }`; a **Frame** is `{ index, duration, content: Option<FrameContent>,
   keyframe: KeyframeKind, tween: Option<Tween> }`.
2. **Instances reference definitions by id.** `FrameContent` is a list of placed items in
   z-order; each is either a **placed shape** (inline vector art, the authoring equivalent of
   SWF `DefineShape`), a **text block**, or a **SymbolInstance { def: SymbolId, matrix,
   color, blend, name, filters, playback: LoopMode/first_frame, … }**. This is exactly the
   "shared definition + independent instance" requirement: editing a `SymbolDefinition`
   updates every instance (shared via `Arc`), while each instance keeps its own
   `matrix`/`color`/`name`/`playback`.
3. **Coordinates: pixels in the document, twips only at the SWF boundary.** Author in clean
   floating-point pixels (kurbo `Point`); convert to twips (×20) and 16.8 fixed matrices when
   writing SWF, and back on read. Stage size, frame rate, and background are document settings.
4. **Deterministic evaluation.** `evaluate(doc, t, opts) → DisplayList` is a **pure function**:
   for the top timeline at frame/time `t`, for each layer (respecting mask/guide/folder and
   layer type), resolve each frame's content (following frame spans and tweens), resolve each
   symbol instance by recursing into its definition's timeline at the appropriate local time
   (graphic: follow parent time; movie clip: its own time), and emit a flattened display list of
   placed shapes/instances with effective transforms and color/blend. No stored per-frame state —
   the display list is always derived. This mirrors how Ruffle reconstructs the display list
   from tags at each frame (§2), and it is what makes rendering, scrubbing, onion skinning, and
   export uniform (doc 04).
5. **Motion tween** = keyframes on a `SymbolInstance`'s `matrix` (and color/filters) evaluated by
   the keyframe engine (doc 05/08) with CreateJS easing. **Shape tween** = keyframes on two
   `Shape` forms, morphed by matching edges/fills (the `DefineMorphShape`/XFL `tweenType="shape"`
   model) — a genuinely new piece of work (see doc 10 risk).
6. **Scenes** = top-level timelines; the document is `{ settings, scenes: Vec<Scene>,
   library: Library, … }`.
7. **Interactivity** (buttons, movie-clip scripts) lives in the **runtime**, not the document:
   the document stores *what* the button states and *where* scripts attach (a `script` reference
   per frame/instance/button), and the runtime (doc 05) interprets them. This keeps the document
   serializable and undoable and the runtime stateless.

## 5. Mapping to the Craft infrastructure

| Flash concept | Reuse / source | New in AniCraft |
|---------------|----------------|-------------------|
| Time base (frame rate, exact ticks) | `effectcraft-time` / `filmcraft-time` (`Tick`, rational `FrameRate`) | none |
| Keyframe model + easing (motion tween) | `effectcraft-keyframe` (temporal linear/bezier/hold, spatial bezier, arc-length) + CreateJS easing functions | CreateJS easing set + per-target ease mapping |
| Vector shapes (fills/strokes/edges) | kurbo `BezPath`, peniko paint; SWF `Shape` read/write via Ruffle `swf` | edge/fill style authoring + shared-edge editing |
| Timeline/layer/scene structure | pattern from `filmcraft-project` (tracks) / `effectcraft-project` (layers) | Flash-specific `Timeline`/`Layer`/`Frame` + mask/guide/folder kinds |
| Symbol definition + instance | pattern from VectorCraft `Symbol { name, art: Arc<Node> }` (simple) | **timeline symbols** (graphic/movie clip/button) + nested instances |
| Display list construction | Ruffle `core` `MovieClip`/depth model (reference) | **deterministic `evaluate()`** producing the display list |
| Nested animation | SWF `Sprite` (nested tags) / XFL symbol files | recursion in `evaluate()` (graphic follows parent time, movie clip its own) |
| Button states | SWF `Button` / XFL button symbol | button model + Up/Over/Down/Hit + runtime hit-testing |
| Onion skinning | — (render feature, doc 04) | frame-ghost rendering option |
| Shapes morph (shape tween) | SWF `DefineMorphShape` (read) | **morph algorithm** (edge/fill matching) — new |
| Actions/scripts | boa (doc 05); SWF `DoAction`/`DoAbc` (read via Ruffle) | Flash scripting object model + authoring-time scripts |

## 6. What is clearly new (not in the suite)

- The **symbol/instance model with timeline symbols** (graphic/movie clip/button) — VectorCraft's
  symbol is shared art only; EffectCraft/FilmCraft have precomps/clips but not Flash's symbol
  types or the shared-definition/independent-instance semantics tied to a library.
- **Stage/timeline authoring** (scenes, layers with mask/guide/folder kinds, frame spans,
  blank keyframes, onion skinning) — no sibling app authors a Flash-style timeline.
- **Shape tweening / morphing** — not present in the suite.
- The **Flash scripting object model** (Stage/MovieClip/Sprite/DisplayObject/Timeline/Library)
  for interactive content.

Everything else (time, keyframes, vector primitives, display-list reconstruction, rendering,
formats) is either directly reusable or a well-understood pattern.