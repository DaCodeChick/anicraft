# 07 — Formats and Compatibility

> **Scope.** FlashCraft's native format and its import/export/interop surface: XFL, SWF, FLA,
> SVG, animated images, video, sprite sheets, and standalone interactive output — including the
> technical and **legal/clean-room** limits. Verified: Ruffle `swf` crate (source + `Cargo.toml` +
> license), sibling format crates (VectorCraft `svg`/`pdf`/`format`, EffectCraft `lottie`/
> `export`). External: SWF File Format Specification v19, XFL schema, public format write-ups.

## 1. Format strategy (recommendation)

FlashCraft has **one native authoring format** and several **interchange** formats:

| Role | Format | Direction | Notes |
|------|--------|-----------|-------|
| **Native (primary)** | `.flashcraft` | read/write | Versioned, compressed, structurally-shared JSON (the suite's native-format idiom). Lossless authoring + editor state. |
| **Authoring interchange** | **XFL** | read/write | Open XML (the uncompressed FLA). Round-trips the full authoring model. |
| **Playback / legacy** | **SWF** | read + practical write | The Flash runtime format. Via Ruffle `swf`. |
| **Static vector** | SVG | read/write | One frame (or a selected frame) as SVG 1.1; animation via SMIL is out of scope (use Lottie/video). |
| **Animated images** | GIF / APNG / WebP (animated) / PNG sequence | write | Rasterized from `evaluate()` at the doc frame rate. |
| **Video** | MP4 (H.264/HEVC/AV1), WebM (VP9/AV1), MOV (ProRes), image sequences (PNG/TIFF/EXR) | write | Via FilmCraft's pure-Rust codecs/encoders (git dep) — the suite already does this. |
| **Sprite sheets** | PNG/WEBP atlas + JSON (frame rects, timing) | write | For game engines; frames from `evaluate()`. |
| **Web / interactive** | Lottie (bonus), standalone HTML/JS/WASM player | write | EffectCraft already exports Lottie; a WASM player reuses the wasm32 stack (doc 05). |
| **Print** | PDF | write (optional) | Stage→PDF via `craft-print` (doc 02). |

**Why a native format rather than XFL-or-SWF-primary** (the key format decision):
- **SWF is a playback format, not an authoring store.** It has no editor state (selection,
  workspaces, unsaved work), encodes shapes in a compact tag stream that is awkward to edit
  incrementally, and some authoring-only concepts don't map cleanly. It is the *published
  output*, the same role SVG/PDF play for VectorCraft.
- **XFL is open and round-trips authoring data**, but it is verbose (one XML file per symbol + a
  DOM document), uses an edge-based shape encoding and shared-edge model, and has known
  quirks (rounding, `bin/*.dat` media, `SymDepend.cache`). Making it the *primary* store would
  make structural-sharing undo and incremental editing awkward and couple FlashCraft to Adobe's
  XML evolution.
- **The native `.flashcraft`** (the suite idiom: VectorCraft `.vectorcraft`, EffectCraft
  `.ecproj`, FilmCraft `.fcproj` — "the document as readable JSON, versioned, saved atomically,
  backward-readable") gives lossless, fast, undo-friendly authoring. XFL and SWF are then robust
  import/export targets. This mirrors how every sibling app has a native format + open
  interchange.

## 2. SWF (verified + external)

**Format facts (external).** The **SWF File Format Specification, version 19 (Adobe, 2012)** is
the **last publicly documented version**. Versions 20–30 exist (PRONOM lists them) but are **not
publicly documented** (loc.gov FDD: "no documentation found for … after version 19"). Adobe
published the spec as **open** — "available as an open specification to create products and
technology that implement the specification" (Adobe devnet SWF page). Milestones: SWF 9 introduced
**ActionScript 3 + AVM2**; SWF 10 added bidirectional text (`DefineFont4`), `DefineBitsJPEG4`
(JPEG with alpha), and Speex. `FileAttributes` (tag 69) is required since v8 and must be first.
Coordinates are **twips** (1/20 px); matrices are 16.8 fixed-point.

**Reusable implementation (verified).** Ruffle's **`swf` crate** (v0.3.0, published on crates.io;
part of Ruffle, **MIT OR Apache-2.0**) **reads and writes** SWF 19 — `swf/src/read.rs` and
`swf/src/write.rs` both exist, with `Swf`/`Tag`/all the character and display-list types
(`ruffle/swf/src/types.rs`, `tag_code.rs`). Its dependencies (`bitflags`, `bitstream-io`,
`encoding_rs`, `num-*`, `log`, `flate2`, `lzma-rs`, `enum-map`, `simple_asn1`) are ordinary and
wasm-friendly. This is the **single highest-value reuse** for FlashCraft: full SWF interop with
**zero format code**.

**Recommendation.**
- **SWF import:** parse with `swf::read` into a `Swf` (tags), then convert characters (
  `DefineShape*`, `DefineSprite`, `DefineButton*`, `DefineMorphShape*`, `DefineText*`,
  `DefineFont*`, `DefineBits*`, `DefineSound`) into `flashcraft-doc` symbols and the
  `PlaceObject`/`ShowFrame`/`FrameLabel` tag stream into scenes/layers/frames/instances. Fuzz the
  importer (untrusted input).
- **SWF export ("Publish"):** serialize the document to tags — emit `FileAttributes` (version
  13–19 range, the documented window), `SetBackgroundColor`, character definitions for each
  library symbol, and per-scene display-list tags (`PlaceObject`/`RemoveObject`/`ShowFrame`/
  `FrameLabel`). Target the **documented tag set** (SWF ≤ 19). A "practical writer" means:
  vector shapes, movie-clip symbols, buttons, bitmaps, (static + editable) text, sounds, morphs,
  and frame labels; **not** every exotic tag (video, advanced font4 CSM, etc.) in v1.
- **Version target:** read any version `swf` supports; write a mid-range version (e.g. 13–19)
  that covers the documented feature set FlashCraft authors.
- **Do not** attempt to read/write undocumented SWF 20–30 (clean-room + no public spec). If a
  future need arises, it is a separate, spec-driven effort.

**Legal/clean-room.** Implementing SWF *from the public spec* is exactly the clean-room pattern
the siblings use for codecs (FilmCraft: "codecs and containers are written from public
specifications"). Depending on Ruffle's `swf` (MIT/Apache) is license-compatible and avoids
re-implementation. **No Adobe software is used** at any point.

## 3. XFL (external, open)

**Format facts.** XFL is an **open XML** authoring format — a ZIP of `DOMDocument.xml`,
`LIBRARY/*.xml` (one file per symbol), `bin/*.dat` (binary media), and a `mimetype`
(`application/vnd.adobe.xfl`). Namespace `http://ns.adobe.com/xfl/2008/`. A **public XSD exists**
(friction2d/xfl-schema). **Modern FLA (CS5+) is the same structure compressed**; Flash supports
saving directly to XFL. Structure and the symbol/instance/tween model are detailed in doc 03 §3.
Easing is **CreateJS** (`quad/cubic/quart/quint/sine/circ/expo/back/elastic/bounce × In/Out/
InOut`; `back` overshoot 1.7, `backInOut` ×1.525, `elasticInOut` period 0.45; `<CustomEase>` =
piecewise cubic, 3n+1 points).

**Recommendation.**
- **XFL import:** unzip → parse `DOMDocument.xml` (width/height/frameRate/backgroundColor/
  currentTimeline) + each `LIBRARY/*.xml` (symbols) + `bin/*` (media) → build `flashcraft-doc`.
  Map `DOMSymbolInstance` → instance (matrix/transform-point/color), `DOMShape` (fills/strokes/
  edges) → shapes, `DOMFrame` (duration/keyMode/tweenType/acceleration/tweens) → frames/tweens,
  layer `layerType` (normal/guide/folder/mask/masked) → layer kinds.
- **XFL export:** serialize `flashcraft-doc` to the same structure (one XML per symbol + a DOM
  document + `bin/` for embedded media), matching the XSD. This is a **new crate**
  (`flashcraft-xfl`) — there is no Rust XFL writer in the suite or (to our knowledge) a
  maintained one to depend on. It is medium effort (XML in/out + the edge-based shape encoding).
- **XFL is the clean-room "open authoring" interchange** — ideal for the pipeline use case
  (programmatic scene authoring, external tooling) that motivated XFL's existence, and a safer
  interchange than SWF for authoring round-trips.

**Legal/clean-room.** XFL is open (XML + public schema); implementing a reader/writer from the
schema is clean-room. No Adobe software involved.

## 4. FLA (proprietary — out of scope)

**FLA** (pre-CS5) is a **proprietary binary** (Microsoft Compound File / OLE structured storage
container; CS5+ FLA is a ZIP of XFL). There is **no public specification** for the classic binary
FLA, and deriving one would require **reverse engineering**, which is **forbidden** by the
sibling apps' absolute clean-room rule (no Adobe software, no reverse engineering).
**Recommendation:** **no FLA support.** Interop with modern files goes through **XFL** (which
*is* the uncompressed FLA) and **SWF** (the published output). This is a deliberate, documented
limitation, not an oversight.

## 5. Other outputs (verified capabilities in the suite)

- **SVG** — VectorCraft `crates/svg` (usvg import + own writer) shows the pattern: write one
  frame of the display list as SVG 1.1 (paths, fills/strokes, gradients in `<defs>`, text as
  `<text>`/`<tspan>` or outlines, images as `data:` URIs). FlashCraft reuses this for "Export
  frame as SVG" (static). *Animated* SVG (SMIL) is **out of scope** — use Lottie or video for
  animation.
- **Animated images** — rasterize `evaluate()` per frame → GIF (the `gif` crate, already a
  workspace dep in VectorCraft/EffectCraft), APNG, animated WebP, or a PNG sequence. Deterministic
  (same frames every run).
- **Video** — FilmCraft's pure-Rust **codec/encoder/mux** crates (`filmcraft-h264enc`,
  `filmcraft-hevc`, `filmcraft-av1`, `filmcraft-vp9`, `filmcraft-prores`, `filmcraft-aac`,
  `filmcraft-isobmff`, …) are already reused by **EffectCraft** (git-pinned) for H.264/HEVC/AV1
  MP4, ProRes MOV, WebM, image sequences, and audio (WAV/AIFF). FlashCraft can reuse the **same
  git-pinned crates** for "Export as video" (a Flash export is a fixed-size, fixed-frame-rate
  sequence — a natural fit for the render→encode→mux pipeline, doc 01 §3.3). Audio (event sounds,
  streaming) would use FilmCraft's audio decode + cpal (desktop) / WebAudio (web).
- **Sprite sheets** — render frames to a PNG/WEBP atlas with a JSON manifest (frame rects,
  per-frame duration) — straightforward from `evaluate()`.
- **Standalone interactive** — (a) **Lottie** bonus (EffectCraft `crates/lottie` exports Lottie;
  a subset of Flash motion-tween vector animation maps cleanly), and (b) a **WASM/JS player**
  (the wasm32 build of `flashcraft-render` + a runtime, doc 05 §6). Both are Phase 8.

## 6. Persistence and recovery (reuse)

- **Atomic saves** via `craft-atomic` (doc 02) for `.flashcraft`; **backward-readable** versioning
  (VectorCraft reads "down to v1"); a `savedBy`/`schema` header (EffectCraft) so newer files warn
  and older fields are dropped safely.
- **Autosave / Data Recovery** copies in the background (VectorCraft/EffectCraft pattern) using
  `craft-settings`/`craft-atomic`.
- **Media embedding vs. linking:** the native format embeds images (base64/blob) by default with
  an optional linked-file mode (VectorCraft `links`); XFL uses `bin/*.dat`; SWF embeds
  `DefineBits*`.

## 7. Determinism and reproducibility (a first-class requirement)

- `evaluate(document, t)` is pure → the same frame renders the same pixels every run (doc 03/04).
- **Exports are reproducible:** same document + settings → same file (FilmCraft convention;
  frames rendered in fixed groups, encoders deterministic). This is tested, not assumed.
- Round-trip tests: `.flashcraft` → XFL → `.flashcraft` and `.flashcraft` → SWF → Ruffle-playback
  frame-comparison (using Ruffle as an oracle for SWF conformance) are the key validation gates.

## 8. Licensing summary

| Asset/format | Licence / status | Reuse path |
|-------------|------------------|------------|
| Ruffle `swf` crate | MIT OR Apache-2.0 | git/crates.io dependency (not copied in) |
| SWF spec v19 | Adobe-published open spec | implement from spec / use `swf` |
| XFL | Open XML + public XSD | new `flashcraft-xfl` crate (clean-room from schema) |
| FLA (classic binary) | **Proprietary, no public spec** | **not supported** (clean-room) |
| kurbo/peniko/vello_cpu/usvg/skrifa/harfrust/boa | various open (see doc 01/05) | crates.io dependencies |
| FilmCraft codecs/encoders | MIT OR Apache-2.0 (same family) | git-pinned dependency (EffectCraft pattern) |
| App icons/fonts | original (MIT/Apache) / OSI / CC0 / CC | authored + attributed (ASSETS.md); fonts in `craft-fonts` |

## 9. Interop scope recommendation (decision for the user)

- **In:** `.flashcraft` (native), **XFL** (round-trip), **SWF** (import all; export the documented
  tag set), SVG (static frame), GIF/APNG/PNG-sequence, **video** (H.264/HEVC/AV1/ProRes/WebM via
  FilmCraft codecs), sprite sheets, Lottie (bonus), WASM/JS player, PDF (optional print).
- **Out:** **FLA** (proprietary), **SWF 20–30** (undocumented), **ActionScript 1/2/3 authoring**
  (future interop via Ruffle for *playing* legacy SWF, doc 05 §8).