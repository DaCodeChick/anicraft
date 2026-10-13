# 06 — UI and UX

> **Scope.** The Craft design conventions FlashCraft must follow, the dark-mode palette, and the
> proposed FlashCraft interface organization mapped from the classic Flash/Animate workflow.
> Verified: VectorCraft `crates/ui-egui/src/theme.rs` (tokens/palette), `craft-libs/crates/ui`
> (docking/layout/tabs/buttons/icons), sibling `AGENTS.md`/`docs`. External: classic Flash/Animate
> workspace documentation. The four user-provided screenshots in `references/` are noted as
> reference material (their pixels were not analyzed in this research pass).

## 1. Craft design conventions (verified)

All six apps are **egui/eframe** frontends with a consistent structure:

- **A thin UI over the engine.** "The UI is thin: panels read engine state and act through
  `app.run(id, params)`. Colours come from `theme::Tokens`" (VectorCraft `AGENTS.md`). Panels hold
  `Arc` snapshots of the active document (no per-frame deep clones).
- **Docking via `craft-ui`.** The frontend uses `craft-ui::docking` for **atomic layout
  transactions** and pointer drop targets, and `craft-ui::layout` for **bounded split geometry**
  and resize interactions (EffectCraft `docs/architecture.md`). Panel ids are unique/stable; a
  small adapter preserves each app's saved dock tree, stacked entries, floating rectangles, and
  dynamic viewer identities.
- **Tabs, toolbars, icon buttons** via `craft-ui::tabs` / `layout::fit_toolbar` /
  `buttons::IconButton` (accessible, theme-driven).
- **Icons:** each app supplies its **own** catalog of embedded, attributed, monochrome SVGs to
  `craft-ui::icons::SvgIconSet`; the app installs the `egui_extras` SVG loader. No shared icon set.
- **Theme:** each app has a local `theme.rs` with a `Tokens` struct; the shared
  `craft-ui::theme::WidgetPalette` colors the five egui widget states. **Dark mode is the default
  primary theme in the creative apps** (VectorCraft default brightness resolves to a dark
  palette; LightCraft ships a "Dark photo workflow palette"; FilmCraft/EffectCraft have dark
  presets with custom accent + accessible contrast).
- **Automation ids.** Every interactive widget registers an automation id
  (`app.auto.add(id, rect, label)`) so agents can inspect/click the UI (FilmCraft/EffectCraft).
- **i18n, shortcuts, workspaces** are per-app (menus in `ui-egui/src/menus.rs`, shortcut presets in
  the engine).

## 2. Dark-mode palette (verified, VectorCraft `theme.rs`)

VectorCraft's dark `Tokens` (the concrete reference for a Craft dark UI; other apps vary their
accents/contrast but share the neutral-gray structure):

| Token | Hex | Token | Hex |
|-------|-----|-------|-----|
| `app_bar` | `#262626` | `text` | `#d8d8d8` |
| `panel` | `#323232` | `text_dim` | `#a8a8a8` |
| `panel_darker` | `#282828` | `text_disabled` | `#6e6e6e` |
| `tab_strip` | `#282828` | `text_strong` | `#f5f5f5` |
| `input` | `#1f1f1f` | `icon` | `#cfcfcf` |
| `input_border` | `#464646` | `hover` | `#444444` |
| `border` | `#1b1b1b` | `tool_active` | `#1d1d1d` |
| `divider` | `#3f3f3f` | `button` | `#444444` |
| `accent` | `#378ef0` | `accent_strong` | `#1473e6` |
| `row_selected` | `#484b50` | `selection` | `#4f80ff` |
| `pasteboard` | `#1e1e1e` | `anchor_selected` | `#3d82ff` |
| `ruler` | `#2f2f2f` | `flyout_active` | `#54b2f7` |
| `ruler_tick` | `#8a8a8a` | `guide` | `#4affff` |
| `smart_guide` | `#ff3dfc` | `error` | `#e34850` |
| `button_border` | `#6a6a6a` | `warning` | `#f0a330` |
| `radius` | 2 | `font_size` | 13.0 |

Design characteristics: **neutral, low-saturation grays** (no blue cast in surfaces), a **single
blue accent** (`#378ef0`) for selection/focus/active, **dimmed** secondary text, a **dark
pasteboard** (`#1e1e1e`) darker than the panels, and **high-visibility** guide/smart-guide colors
(cyan/magenta) that read as overlays, not UI. This satisfies the task's requirement to "avoid
unnecessarily bright backgrounds, eye-straining contrast, or visually noisy UI."

**Recommendation.** FlashCraft's `theme.rs` uses this dark palette as the **primary** theme (a
light theme can be added later). It should add a small set of **Flash-specific tokens**:
- `timeline_track` / `timeline_playhead` / `timeline_keyframe` (normal/blank) / `timeline_span`,
- `layer_*` (normal/guide/mask/masked row colors, layer-color swatches),
- `symbol_*` (graphic/movieclip/button type glyphs),
- `onion_prev` / `onion_next` (the two onion-skin hues),
- `tween_motion` / `tween_shape` (the colored tween spans in the timeline),
- `button_state_*` (up/over/down/hit indicators),
- `stage_bg` (the stage background, default near-white is *content*, not UI — it is the document's
  background color, user-settable, and shown against the dark pasteboard).

## 3. Typography and icons

- **Fonts:** from `craft-fonts` via `CRAFT_FONTS_DIR` (never committed). A UI font for chrome and
  any bundled document fonts. VectorCraft uses 13 pt body text in its dark theme; EffectCraft
  11 pt; FlashCraft should pick one (recommend 13 pt to match the vector/creative apps).
- **Icons:** an original (MIT/Apache) or open-licensed (OSI/CC0/CC) SVG catalog, monochrome,
  `currentColor`, ~1.75 stroke width (the `craft-ui` `SvgStyle` default in its README example),
  installed through `craft-ui::icons`. **No Adobe iconography** (clean-room rule, doc 01/AGENTS).
  The tool icons (selection, subselection, lasso, pen, line, rectangle, oval, polygon/star, text,
  bucket, ink bottle, eyedropper, eraser, free transform, gradient transform, hand, zoom) and
  panel glyphs are all **drawn in code / original SVG** (the apps "prefer art generated in code
  for defaults").

## 4. Classic Flash/Animate workflow (external, well-documented)

The "Essentials" workspace has these regions (consistent across Flash CS→Animate):

- **Stage** (center): the visible rectangle (default white), surrounded by the gray **pasteboard**
  (authoring space, invisible in output). Rulers, grid, guides. The **Document Settings**
  (size, frame rate, background) are set from the Properties panel or Modify ▸ Document.
- **Timeline** (below the stage): frame ruler, **layers** (name + color + eye/lock/outline
  controls), **frames** (keyframes, blank keyframes, spans, tween spans), **playhead** (a
  vertical line), **frame number + frame rate** readout, **transport** (play/pause/stop/step,
  loop), **Add Frame / Add Keyframe / Add Blank Keyframe / Tween** controls, **onion skin**
  controls, and a timeline zoom/resize. Auto-keyframe mode optional.
- **Tools panel** (left, or right in older layouts): selection, subselection, lasso, pen, line,
  rectangle/oval (+ primitives), polygon/star, text, paint bucket, ink bottle, eyedropper, eraser,
  free transform, gradient transform, hand, zoom; plus **Fill/Stroke color** swatches and the
  **Merge/Object** drawing-mode toggle. Single-letter shortcuts (V, A, L, pen, N, R, O, T, K, S,
  I, E, Q, F, H, Z).
- **Properties / Property Inspector** (right): context-dependent — **Tool**, **Object**, **Frame**,
  **Document** (Animate 2020+ has these four tabs). Shows transform (x/y/w/h, rotation, scale),
  color/alpha, blend mode, filters, and (for a frame) tween settings and the frame's actions.
- **Library** (right, tabbed with Properties): the symbol/asset manager — folders, symbols
  (graphic/movieclip/button), bitmaps, sounds, video; sort by type/use-count; search;
  **Add to Stage / Convert to Symbol / Export**.
- **Actions panel** (F9): edit script for the selected frame / button / movie clip
  (Frame Actions / Button Actions / Movie Clip Actions).
- **Movie Explorer**: a hierarchical tree of the display list (scenes → layers → instances →
  nested symbol contents) for navigating the document.
- **Color / Swatches / Align / Transform** panels: color editing, palettes, align/distribute,
  numeric transform.

The panel set docks, undocks, groups (tabs), and floats; **workspaces** are saved layouts.

## 5. Proposed FlashCraft interface organization (recommendation)

Map the Flash regions onto Craft conventions (docking via `craft-ui`, dark theme, thin panels over
the engine, everything a command):

```
┌───────────────────────────────────────────────────────────────────────────────┐
│  menu bar (File/Edit/Modify/Insert/Modify▸Timeline/Window/Help)   [workspace ▾] │
├──────────┬──────────────────────────────────────────────────────────┬──────────┤
│          │                                                          │          │
│  TOOLS   │                      STAGE (canvas)                       │  PROPERTIES│
│  (icon   │   ┌────────────────────────────────────────────────┐      │  /INSPECTOR│
│  column, │   │   pasteboard (dark)        stage (bg color)    │      │ (Tools/   │
│  V A L   │   │                       ┌──────────────┐         │      │  Object/  │
│  pen N R │   │                       │   Stage      │         │      │  Frame/   │
│  O T K S │   │                       └──────────────┘         │      │  Document)│
│  I E Q F │   └────────────────────────────────────────────────┘      ├──────────┤
│  H Z     │          (zoom/pan; onion-skin ghosts; selection)         │  LIBRARY  │
│  ─ fill/ │                                                          │ (symbols/ │
│  stroke  │                                                          │  bitmaps/ │
│  swatches│                                                          │  sounds)  │
│  + merge │                                                          ├──────────┤
│  /object │                                                          │  ACTIONS  │
│          │                                                          │ (script) │
├──────────┴──────────────────────────────────────────────────────────┴──────────┤
│  TIMELINE  [◀◀ ▶ ■ ▶▶] frame: 12  fps: 24   [Add Frame][Add Key][Add Blank]    │
│  [Tween ▾] [onion: ◀ 1 2 | ▶ 1 2]  | layers (name·color·eye·lock·outline)      │
│   Scene 1 │ Layer 3 (guide) │ ▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢▢ │
│           │ Layer 2 (mask)  │ ●──────────────────●  (motion tween span)        │
│           │ Layer 1         │ ○ ●▬▬▬▬▬● ○        (key/blank/span)               │
│           │                 │  ▲ (playhead)  ▸ frame cells (click to place)     │
└───────────────────────────────────────────────────────────────────────────────┘
```

Region → Craft mapping:

- **Menu bar** — from the engine command registry (`file.*`, `edit.*`, `timeline.*`, `symbol.*`,
  `tool.*`, `window.*`, `help.*`) + `UI_COMMANDS` (VectorCraft/FilmCraft pattern).
- **Tools panel** — a vertical icon column of `craft-ui::buttons::IconButton` (selected tool
  highlighted with `tool_active`); a tool's options (size, corner radius, gradient, drawing mode)
  in a strip above/below it (the "Options area" of the Flash toolbar). Fill/Stroke swatches and
  the Merge/Object toggle at the bottom.
- **Stage/canvas** — the `vello_cpu` render (doc 04) in an egui area; zoom/pan (Hand/Zoom),
  onion-skin overlay, selection handles (theme `selection`/`anchor_selected`), rulers/grid/guides.
  Driven by the tool state machines → commands.
- **Properties/Inspector** — a docked panel (right), context tabs **Tool / Object / Frame /
  Document** (matching Animate 2020+). Reads the current selection/frame/document from the
  engine and writes via commands. Draggable sections (Animate-style).
- **Library** — a docked panel (right, tabbed with Properties): `craft-ui` list/tree with symbol
  type glyphs (graphic/movieclip/button), folders, thumbnails, use-count, search; double-click or
  drag to place on the stage; right-click for Convert to Symbol / Export.
- **Actions** — a docked code panel (F9) bound to the selected frame/button/clip; the JS editor
  with a "run in editor" (authoring-time, undoable) and "attach to playback" distinction (doc 05).
- **Timeline** — the custom centerpiece panel (below the stage), built on `craft-ui` list/scroll
  primitives: layer header column (name, color swatch, eye/lock/outline, layer-type glyph), frame
  ruler, frame cells (keyframe ●, blank ○, span ▬, motion-tween span in `tween_motion` color,
  shape-tween span in `tween_shape` color), playhead, transport, and the Add Frame/Key/Blank +
  Tween + onion-skin controls. Scrubbing the playhead drives `evaluate()` + the renderer.
  Virtualized rows (EffectCraft's timeline draws only visible rows for thousands of layers).
- **Movie Explorer** — a docked tree panel of the display list (scenes → layers → instances →
  nested symbol contents), for navigation and "go to definition / select instances" (the Flash
  Movie Explorer).
- **Workspaces** — saved dock layouts via `craft-ui::docking` (persisted with `craft-settings`);
  an "Essentials" default matching the diagram, plus saved user workspaces.
- **Graph editor** (optional, later) — a panel to edit keyframe easing curves (EffectCraft has
  one; useful for motion tweens).

**Keyboard shortcuts** (Flash-familiar, Craft-dispatched): V selection, A subselection, L lasso,
pen, N line, R rectangle, O oval, T text, K bucket, S ink bottle, I eyedropper, E eraser, Q free
transform, F gradient transform, H hand, Z zoom, J drawing mode, F9 Actions, Ctrl/⌘+L Library,
Ctrl/⌘+J Document settings, Ctrl/⌘+G group, space-drag to pan, +/- zoom, play/space transport,
arrow keys step frames, Home/End to first/last frame. Final mapping is a Craft engine shortcut
preset (doc 01 §2.3).

## 6. Selection feedback and interaction conventions

- **Selection** uses the theme `selection` outline (`#4f80ff`) with `anchor_selected` handles for
  path vertices; marquee is a translucent accent fill. Matches VectorCraft's selection styling.
- **Sub-selection** (Flash's 4 modes): double-click a shape → whole object; single-click fill →
  fill only; double-click stroke → stroke; single-click stroke → that edge. Mode is a modifier +
  the Subselection tool.
- **Tool interactions** are `Begin`/`Preview`/`Commit` (doc 01 §2.5): a drag previews the
  transform live (re-applied on a snapshot), releasing commits one undo step; Esc cancels. This
  gives the familiar Flash "drag to move/transform, live preview" feel with clean undo.
- **Timeline interactions** (add keyframe, drag a keyframe, extend a span, scrub the playhead,
  drag a tween) are each commands or tool `Action`s, so they are undoable and agent-replayable.
- **Onion skinning** and **outlines** (layer "show as outline") are view options (not document
  edits), toggled in the timeline/layer controls.

## 7. What is reused vs. new

| Piece | Reused | New in FlashCraft |
|-------|--------|-------------------|
| Docking, tabs, toolbars, splits, resize | `craft-ui` | the panel set + workspaces |
| Icon buttons + SVG icon pipeline | `craft-ui` | the original/open Flash tool & panel icon set |
| Dark palette + widget states | VectorCraft `theme.rs` + `craft-ui::theme` | Flash-specific tokens (timeline, layers, symbols, onion, tween, button states) |
| Fonts | `craft-fonts` | — (chooses families) |
| Canvas rendering | doc 04 (`vello_cpu`) | the stage viewport (zoom/pan/pasteboard/onion) |
| Menu/commands/shortcuts | engine command registry pattern | the Flash command set + shortcut preset |
| Tools | VectorCraft tool→command pattern | the Flash tool set |
| Timeline | `craft-ui` list/scroll + EffectCraft timeline patterns | the **Flash timeline** (layers/frames/keyframes/tweens/playhead/transport/onion) |
| Library | — | the symbol/asset library panel |
| Actions | EffectCraft Script Console pattern | the JS actions panel (authoring + playback) |
| Movie Explorer | — | the display-list tree panel |
| Automation ids | the suite's `app.auto.add` pattern | per-widget ids |

## 8. Notes on the provided references

`references/` contains four small JPEGs (≈500–960 px wide) that appear to be UI screenshots
provided as visual reference. Their pixel content was **not** analyzed in this pass (text-based
research); they should be used during Phase 1+ UI work as side-by-side references. They are
**not** to be treated as licensed assets (provenance unverified) and are not to be committed as
app assets.

## 9. Risks / open questions (→ doc 10)

- Final **tool icon set** (original SVGs) is a real design deliverable; must be authored/
  licensed, not copied.
- The **timeline panel** is the most novel UI; its interaction model (keyframe/span/tween
  dragging) needs prototyping once implementation is authorized.
- Whether the **Properties panel** uses Animate's 4-tab model (Tool/Object/Frame/Document) vs.
  a single context panel — minor, decide in Phase 2.