# 02 — Shared Libraries (`craft-libs`, `craft-fonts`)

> **Scope.** Analysis of the shared Craft infrastructure and its reuse opportunities for
> AniCraft. Verified from `craft-libs/` and `craft-fonts/` source. Nothing was modified.
>
> Key source files: `craft-libs/Cargo.toml`, `craft-libs/AGENTS.md`, `craft-libs/README.md`,
> `craft-libs/crates/*/src/lib.rs`, `craft-libs/crates/ui/README.md`,
> `craft-libs/docs/{theme-infrastructure,ui-docking,ui-components-audit,settings-storage}.md`,
> `craft-fonts/{Cargo.toml,README.md}`.

## 1. What `craft-libs` is

A Rust monorepo workspace (edition 2024, resolver 3, `MIT OR Apache-2.0`) of small libraries
"used by the various Craft Apps." Each library is a crate under `crates/<name>` named
`craft-<name>`, inheriting workspace fields and lints, registered in `[workspace.dependencies]`.
Apps depend on a crate **by git**:

```toml
craft-<name> = { git = "https://github.com/storytold/craft-libs", rev = "<commit>" }
```

`craft-libs/AGENTS.md` states the rules that shape what belongs here:
- **Never crash** — every `lib.rs` starts with the no-panic lint deny-list; no `unsafe`.
- **New crates** go in `crates/<name>`, named `craft-<name>`, inherit workspace fields/lints,
  listed in `[workspace.dependencies]`.
- **No app code** — "A crate here must be useful to more than one app and must not depend on
  any app."
- **Before merging:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace`.

The crates (from `craft-libs/Cargo.toml`): `craft-atomic`, `craft-print`, `craft-quit`,
`craft-settings`, `craft-ui`.

## 2. Crate-by-crate

### 2.1 `craft-atomic` — crash-safe atomic file writing
`crates/atomic/src/lib.rs`: `atomic_write` never truncates the destination in place. It writes a
temporary file **in the same directory**, flushes to non-volatile storage (`F_FULLFSYNC` on macOS,
`sync_all` on Windows/Linux), renames over the destination atomically, and (Unix) best-effort
flushes the directory. Guarantees **atomicity** (all-or-nothing; a failed/crashed write leaves the
original bytes and removes the staging file) and documents the **durability** boundary. Orphaned
staging-file sweeping is the app's responsibility.

**AniCraft use:** the atomic save path for the native `.anicraft` document (and any
`config.toml`), plus Data Recovery copies. Directly reusable.

### 2.2 `craft-settings` — `config.toml` preferences store
`crates/settings/src/lib.rs`: decides **where** `config.toml` lives (platform folders, portable
mode, overrides), loads it **without ever failing**, saves it **atomically while the app runs**,
keeps a backup, moves damaged files aside, and runs **schema migrations**. The file is kept in
memory as a `toml_edit::DocumentMut` so hand edits/comments and unknown keys survive saves. Apps
provide a `Settings` impl (`APP_ID`, `SCHEMA_VERSION`) and their own defaults. Policy in
`docs/settings-storage.md`.

**AniCraft use:** preferences (theme, last open, shortcut presets, workspace layouts, script
security toggle). Directly reusable.

### 2.3 `craft-print` — system PDF spooler
`crates/print/src/lib.rs`: on macOS/Linux, printers come from `lpstat` and jobs are piped to
`lp` (copies, collation, duplex, colour); on Windows via `Windows.Data.Pdf` + the driver through
`powershell` (no direct Win32). "PDF generation stays in each app." Exposes `Printer`, `Job`,
discovery and submission; `Error::Spool(String)`.

**AniCraft use:** *optional* "Print stage" to PDF. Low priority (Flash's core is screen
animation); include if a print path is wanted. Reusable.

### 2.4 `craft-quit` — platform-correct quit
`crates/quit/src/lib.rs`: on macOS, eframe must quit via `-[NSApplication terminate:]` (⌘Q), not
by closing the window (avoids an AppKit Touch Bar KVO crash). `quit()` and `on_close_request(ctx)`
(hook placed last in `eframe::App::logic`) route window close through it. Unsaved-document
decisions stay in each app.

**AniCraft use:** the app shell's quit/close handling. Directly reusable.

### 2.5 `craft-ui` — shared egui components (the big one)
`crates/ui/README.md` (egui 0.36, Rust ≥ 1.95, `serde` feature for persisting the shared
layout). "Assets, complete themes, localization and command handling remain in each application.
No icons, fonts or application engines are bundled." Modules:

- **`icons`** — `SvgIconSet`/`SvgStyle`: prepares an app-supplied catalog of embedded,
  attributed, monochrome SVGs once (held in `Arc<[u8]>`), does `currentColor`/`stroke-width`
  substitution, and returns a source URI for `egui_extras`' SVG loader. Immutable; missing names
  degrade to empty bytes (no panic). The **app installs the loader and supplies its own assets**.
- **`buttons`** — `IconButton` with `ButtonVisuals`/`IconButtonStyle` for normal/hovered/pressed/
  selected states, selection wins, AccessKit role + label + selection, focus outline. Returns an
  `egui::Response`; the app dispatches the command and toggles selection.
- **`tabs`** — `tabs::fit(…)` (fit/shrink/hide tabs, linear, ≤ 32 passes) and `tabs::Tab`
  (accessible Tab role, keyboard activation, focus outline, caller paints).
- **`layout`** — `split_rect` (Horizontal/Vertical, Ratio/fixed first/fixed second),
  `ResizeHandle` (pointer + arrow-key + AccessKit delta), `fit_toolbar` (overflow groups). Bounded,
  non-panicking geometry.
- **`docking`** — `Layout<P>`: a tree of split/tab/accordion nodes plus floating tab groups;
  `apply(Action)` **validates the complete layout, applies to a copy, commits only a valid
  result** (atomic layout transactions). Moves, tab insert/reorder, side splits, detach, activate,
  close, reveal, resize all use one transaction path. `Layout::location`/`restore` remember a
  panel's placement. Optional `DockArea` renderer. Panel ids must be unique/stable.
- **`theme`** — `WidgetPalette { noninteractive, inactive, hovered, active, open }` with
  `apply_to(&mut Widgets, radius, foreground)`: colors egui's five widget states; borders,
  expansion, selection, shadows stay local. (VectorCraft, DesignCraft, LightCraft use it.)
- **`printing`** — `State` for transient printer/copies/duplex/grayscale + discovery/submission
  receivers and `Feedback` (used by WordCraft/GridCraft).

`docs/ui-docking.md` details validated panel moves, floating groups, workspace adapters, and the
optional egui docking surface; `docs/ui-components-audit.md` is the suite-wide source inventory
and extraction priorities; `docs/theme-infrastructure.md` is the eight-app theme comparison.

**AniCraft use:** the entire panel/inspector/toolbar/tabs/docking shell with the Craft
look-and-feel. AniCraft supplies its own icon SVG catalog (original/open-licensed), its own
`theme.rs` tokens (dark primary), its own fonts (via `craft-fonts`), and dispatches its own
commands. **Directly reusable — the single largest UI reuse.**

## 3. `craft-fonts`

A separate repo for **font assets** (large files) plus the `craft-fonts` Rust crate.
`fonts/manifest.txt` is a machine-readable list (family, style, file, scripts, licence, SHA-256,
pinned source). The crate exposes the manifest as a static table, finds a checkout via
`CRAFT_FONTS_DIR`, loads it, and has an `embed` feature that compiles fonts in. Families present
locally: biz-ud-mincho, biz-ud-pgothic, noto-sans-arabic, noto-sans-cjk-sc, noto-sans-thai,
shippori-mincho. `craft-fonts-tests` asserts checksums/licences/coverage/vertical-text support.

**Convention (from VectorCraft `AGENTS.md`):** "Fonts live in `storytold/craft-fonts`, **never in
this repo**… The app uses it only through the optional `CRAFT_FONTS_DIR` build option… read by
`crates/text/build.rs` into `<app>_text::CRAFT_FONTS`; never add it to a `Cargo.toml`. Everything
must build, test and run with `CRAFT_FONTS` empty."

**AniCraft use:** UI fonts and any bundled document fonts via `CRAFT_FONTS_DIR`. Reusable; no
font files are committed to AniCraft.

## 4. Reuse summary for AniCraft

| Crate | Reuse | Notes |
|-------|-------|-------|
| `craft-ui` | **Yes (core)** | Docking, tabs, toolbars, split/resize, icon catalogs, widget palette. AniCraft supplies assets, theme tokens, fonts, command dispatch. |
| `craft-settings` | **Yes** | Preferences store (theme, layouts, shortcut presets, script security). |
| `craft-atomic` | **Yes** | Atomic saves for `.anicraft` + recovery copies. |
| `craft-quit` | **Yes** | App-shell quit/close. |
| `craft-print` | Optional | Stage→PDF print, if wanted. |
| `craft-fonts` | **Yes** | UI/document fonts via `CRAFT_FONTS_DIR`; never commit fonts. |

**Not in `craft-libs` (must come from elsewhere or be built):** any document model, rendering,
geometry, color, animation/timeline, text shaping, format I/O, scripting, or app-specific
themes/icons/localization. These are per-app or third-party (kurbo, peniko, vello_cpu, usvg,
skrifa/harfrust, boa, Ruffle `swf`), or AniCraft-specific (doc 08).

## 5. Constraints the shared design imposes

1. **`craft-ui` is deliberately low-level.** It ships no assets, no complete theme, no engine.
   AniCraft must build its own `ui-egui` crate, `theme.rs` tokens, icon catalog, menus, and
   command dispatch — the same as every sibling app. There is no "AniCraft-ready" panel kit.
2. **Git-pin coupling.** Depending on `craft-libs` (and Ruffle `swf`, and any `effectcraft-*`)
   pins revisions; co-development needs `[patch]`. This is the accepted ecosystem cost but it
   means AniCraft tracks upstream `craft-libs` releases.
3. **`craft-libs` accepts only broadly useful, app-free crates.** AniCraft-specific helpers
   (Flash display list, symbol model, XFL) **cannot** go into `craft-libs`; they live in
   `anicraft-*` crates. (A candidate future promotion — e.g. a shared geometry/color crate —
   is a craftrules workstream, out of scope now.)
4. **Layering must keep `craft-ui` above the engine.** "Lower layers cannot depend on `craft-ui`"
   (EffectCraft `docs/architecture.md`). So AniCraft's `ui-egui` is the only crate that may use
   `craft-ui` (and `egui`/`eframe`), consistent with the layer tables.

## 6. Licensing

`craft-libs` and `craft-fonts` are **MIT OR Apache-2.0** (dual, at your option), matching the
apps. Bundled assets keep their own open licences, listed in `ATTRIBUTION.md` / `NOTICE`.
Depending on them introduces no licence conflict with AniCraft's MIT OR Apache-2.0.

## 7. Recommendations

1. Depend on `craft-libs` (all five crates; `craft-print` optional) and `craft-fonts`
   (`CRAFT_FONTS_DIR`) by git pin, exactly as the siblings do.
2. Build AniCraft's `ui-egui` on `craft-ui::docking` + `layout` + `tabs` + `buttons` +
   `icons`, with a AniCraft `theme.rs` (dark primary, doc 06) and an original/open-licensed SVG
   icon catalog.
3. Do **not** attempt to put Flash-specific code in `craft-libs`; keep it in `anicraft-*`.
4. Record the pinned `craft-libs` revision in the workspace once implementation begins, and add a
   `[patch]` block (commented) for local co-development, mirroring EffectCraft's FilmCraft pattern.