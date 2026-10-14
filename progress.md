# AniCraft — Implementation Progress

> Tracking document for phase/milestone execution (see `AGENTS.md` §5/§7). Updated in the same
> change that closes a milestone.

**Status line (2026-10-14):** Phase 1 in progress — **milestones 1.1–1.2 complete and verified**;
awaiting user authorization for milestone 1.3.

## Approved decisions (user, 2026-10-13)

- Rust edition 2024, idiomatic Rust, existing Craft conventions.
- Reuse `craft-libs`; reuse Ruffle's `swf` crate and compatible `effectcraft-time`/
  `effectcraft-keyframe` primitives; `vello_cpu` as initial renderer; GPU deferred (Phase 9.1
  decision-gated); Boa for future JavaScript scripting; versioned native `.anicraft` format;
  dark mode primary.
- Clean-room and licensing requirements preserved; sibling Craft repositories unmodified.
- **New:** `references/` screenshots may be used for implementation (design reference; not
  shipped assets).
- **New:** post-CS5 (CS5+) `.fla` format supported (ZIP-of-XFL; same machinery as XFL, Phase 8.3
  scope). Pre-CS5 binary `.fla` remains out of scope (proprietary, no public spec).

## Completed

### Phase 0 — Research (research/00–10)

Complete. The research package is the architectural baseline; doc 07 §4 and doc 10 (U7, D4) carry
dated status notes for the 2026-10-13 amendments.

### Milestone 1.1 — Cargo workspace + xtask (2026-10-13)

**Built** (all verified by running the gates below):

- `Cargo.toml` — workspace (edition 2024, resolver 3); `[workspace.package]` (v0.1.0,
  MIT OR Apache-2.0, rust-version 1.95); `[workspace.dependencies]` with all researched pins;
  `[workspace.lints]` (`unsafe_code = "forbid"`; clippy `unwrap_used`/`expect_used`/`panic`/
  `todo`/`unimplemented`/`unreachable` = deny); commented `[patch]` blocks for local
  co-development (EffectCraft→FilmCraft pattern); sibling profile settings.
- Pins recorded: `craft-atomic`/`craft-print`/`craft-quit`/`craft-settings`/`craft-ui`
  (craft-libs rev `6006abb757df7030ffdd78c4fc952904a9d36491`), `effectcraft-time`/
  `effectcraft-keyframe` (effectcraft rev `470ea7a3af82ee1dd1d2f86c777fe6891bd0c37b`, release
  0.9.2), `swf` 0.3 (crates.io), `kurbo` 0.13, `peniko` 0.6, `vello_cpu` 0.2, `boa_engine` 0.22,
  `usvg` 0.48, `skrifa` 0.47, `harfrust` 0.13, serde/serde_json/thiserror/log/proptest, and the
  UI set (egui/eframe 0.36, rfd 0.17, cpal 0.16, muda 0.21).
- `xtask/` — `cargo xtask layers` (layer rule engine per VectorCraft pattern, with the AniCraft
  table from research/08 §2: L0 `geom`/`color`/`time`, L1 `doc`/`keyframe`, L2 `shapes`/`text`,
  L3 `timeline`/`render`/`gpu`/`swf`/`xfl`/`svg`/`export`, L4 `tools`, L5 `engine`/`script`,
  L6 `ui-egui`/`mcp`, testkit, exempt apps/xtask; allowed same-layer edges; UI-crate floor L6
  incl. `cpal`/`muda`), `cargo xtask wasm` (L0–L5 + ui-egui → wasm32-unknown-unknown),
  `cargo xtask ci` (fmt, clippy, test, layers, wasm). 12 unit tests for the rule engine.
- Build/test infrastructure: `.cargo/config.toml` (xtask alias), `clippy.toml` (tests may
  unwrap), `rustfmt.toml` (width 150, sibling settings), `.github/workflows/ci.yml` (gates on
  macOS/Ubuntu/Windows), `.gitignore` (sibling style + `/corpus` + `/craft-fonts`).
- `AGENTS.md` — research-only restriction replaced with phase-gated implementation authorization;
  all prior constraints preserved; §8 records the decision/gate status.
- `references/` usage and post-CS5 `.fla` support reflected in `AGENTS.md` §1/§8 (with dated
  status notes in `research/07` §4 and `research/10` U7/D4).

**Verification (2026-10-13, local, cargo/rustc 1.99.0, wasm32-unknown-unknown target installed):**

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 12/12 pass (layers rule engine) |
| `cargo xtask layers` | pass (1 member: xtask, exempt; no violations) |
| `cargo xtask wasm` | pass (no wasm-target crates yet — see notes) |
| `cargo xtask ci` (all five) | pass |
| Pin resolvability | all 27 `[workspace.dependencies]` pins resolved via a scratch `cargo generate-lockfile` (456 packages); both git revs verified fetchable from the public remotes and containing the pinned crates |

**Notes / limitations:**

1. `members` is staged: currently `["xtask"]`. Cargo rejects a member glob that matches nothing
   (verified empirically), so `apps/*` is added when `apps/anicraft` lands (milestone 1.2) and
   `crates/*` when the first library crate lands (1.3); final shape `["crates/*", "apps/*",
   "xtask"]` (research/08 §1).
2. `swf` is pinned on crates.io (v0.3) rather than by git rev: the `storytold/ruffle` fork is
   not publicly reachable, so the published crate from the public `ruffle-rs/ruffle` repo is the
   canonical dependency (the local fork remains a reference checkout, per C6).
3. Pins are recorded at the research-verified revisions (not upstream HEADs: craft-libs remote
   main is 24 commits ahead; effectcraft remote main 284 ahead). Bump deliberately, with an API
   review, when needed.
4. Git deps enter `Cargo.lock` only once a member consumes them (first: `craft-ui`/`craft-quit`
   in milestone 1.2). The lock currently holds only xtask's tree.
5. `cargo xtask wasm` passes trivially until Phase 2.1 (L0 crates); the command and target are
   verified working, the loop is unexercised by real crates yet.
6. Research uncertainty U3 (exact `effectcraft-keyframe` API fit for CreateJS easing) is deferred
   to Phase 6.1 as planned.

### Milestone 1.2 — `apps/anicraft` shell (2026-10-14)

**Built** (all verified by running the gates below):

- Workspace: `members` now `["apps/*", "xtask"]` (`crates/*` joins when the first library crate
  lands, 1.3; final shape per research/08 §1); `default-members = ["apps/anicraft"]` so a bare
  `cargo run` at the root launches the desktop app (the xtask binary stays reachable through the
  `cargo xtask` alias).
- `apps/anicraft` — a launchable eframe shell (the `glow` renderer; no wgpu — GPU deferred per
  decision D5). Deliberately feature-free: no panels, tools or documents are drawn, so nothing
  here is a placeholder for later milestones.
  - `src/theme.rs` — `Tokens`, the full doc-06-§2 dark palette as `Tokens::DARK` (neutral grays,
    single `#378ef0` accent, `#1e1e1e` pasteboard, radius 2, 13 pt base); `apply(ctx, tokens)`
    installs the egui `Visuals` (five widget states through `craft_ui::theme::WidgetPalette`,
    selection/hyperlink/shadows) plus spacing and text styles — the VectorCraft `theme.rs`
    pattern, dark-only for now. Tests pin the palette to the documented reference values and
    check `apply` colors the five widget states.
  - `src/main.rs` — window titled **AniCraft** (1440×900 default, 800×500 minimum, app id
    `ai.storyteller.anicraft`); the central area is the dark pasteboard with the app name and
    version; `--version` prints `anicraft <version>`; any other argument is rejected.
  - `craft-quit` wiring: `on_close_request` at the end of `eframe::App::logic` (the app's own
    close decision would precede it; with no document yet there is nothing to save); on macOS
    `install_app_menu` replaces winit's default app menu so Quit (⌘Q) asks the app first. The
    `menu` feature (muda) is target-gated to macOS so no menu backend builds elsewhere.
- First consumer of the `craft-libs` pins: `craft-ui` and `craft-quit` now materialize in
  `Cargo.lock` (git rev `6006abb…`, per the 1.1 pins).
- Repository metadata corrected: `.gitignore` no longer ignores `Cargo.lock` (sibling apps commit
  it; this is a binary workspace), and `[workspace.package].repository` now matches the actual
  origin remote (`github.com/DaCodeChick/anicraft`).

**Verification (2026-10-14, local, cargo/rustc 1.99.0, macOS):**

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 14/14 pass (theme 2, layers rule engine 12) |
| `cargo xtask layers` | pass (2 members: `anicraft`, `xtask` — both exempt; no violations) |
| `cargo xtask wasm` | pass (no wasm-target crates yet) |
| `cargo xtask ci` (all five) | pass |
| `anicraft --version` | prints `anicraft 0.1.0`, exit 0; unknown argument → stderr message, exit 1 |
| Launch | `cargo run -p anicraft` opens the window and the process stays alive |
| Runtime (macOS) | launched and observed: the window appears in the on-screen window list owned by `anicraft`, titled **AniCraft** (1440×932 logical = 1440×900 content + 32 pt title bar, centered); a by-window-id capture shows the content area as a uniform `#1e1e1e` pasteboard (1760 of 1799 content rows, full width) with the `#f5f5f5` “AniCraft” label and the `#a8a8a8` version line; the process stops cleanly on SIGTERM |

**Notes / limitations:**

1. Runtime verification was performed on macOS only (the only platform available here). The CI
   workflow runs the full gate matrix on macOS/Ubuntu/Windows; launching on the other two OSes is
   untested locally.
2. The UI chrome is composited by eframe's `glow` (CPU) renderer. The stage compositor
   (`vello_cpu` CPU reference, later wgpu) is Phase 2/9.1 scope (decision D5), not this shell.
3. No custom window/Dock icon (eframe's default) and eframe's default fonts (via `default_fonts`):
   an original icon set and the `craft-fonts` UI font arrive with the `ui-egui` work (Phase 2/3)
   and packaging (9.5) — no assets are bundled in this milestone.
4. The dark theme is applied unconditionally (the approved primary theme); a user theme
   preference persisted via `craft-settings` is not part of the shell.
5. Interactive close (close button / ⌘Q) was not exercised in this environment: the agent
   session has no accessibility permission, so UI scripting is blocked. The wiring follows the
   `craft-quit` documented pattern (`on_close_request` last in `logic`; on macOS the app menu's
   Quit goes through the app first), whose close-takeover behaviour is unit-tested in
   `craft-libs`; a clean process stop on SIGTERM was observed locally.

## Current milestone

None — milestone 1.2 is closed; awaiting user authorization for the next milestone.

## Outstanding issues

- (none blocking) — see the "Notes / limitations" above.
- Phase 8 gate D6 (audio/video scope) and Phase 3/6 gates D8/D9 remain open for the user; they
  do not block any Phase 1 work.

## Next authorized task (pending user)

**Milestone 1.3 — `anicraft-engine` skeleton** (research/09 §1): a `Session` with an (empty)
`CommandSpec` registry, `Session::execute`, `Arc<Document>` undo, and a `guard`; a "New Document"
command; the JSON control channel (`--control <port>`) and `ui.screenshot` working. Adds
`crates/*` to workspace `members` with the first library crates (`anicraft-geom`/`color`/`time`
per 2.1 may come with or ahead of it).