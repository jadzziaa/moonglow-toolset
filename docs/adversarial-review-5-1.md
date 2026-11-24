---
type: Report
title: 'Adversarial review: 5.1.0 performance, bugs, complexity and test speed'
description: 'Dated record (2026-11-23, branch adversarial-review, 7a65de0a..01263549) of the full-workspace adversarial review across moonglow-toolset: 144 fps performance at 3440x1400, bug fixes, deduplication and Clippy cleanup, and test-suite speedup.'
tags: [review, performance, bugs, refactoring, testing, clippy]
status: stable
generated: { by: antigravity/gemini-3-pro, at: 2026-11-23T23:34:49Z }
verified:
  - by: antigravity/gemini-3-pro
    at: 2026-11-23T23:34:49Z
sources:
  - id: owner
    resource: human:august
    title: 'Adversarial code review ask: 144 fps at 3440x1400 on host CPU/GPU (Radeon RX 9070 XT), deduplication, comment accuracy, bug fixes, and faster tests (2026-11-23)'
  - id: commits
    resource: git log 7a65de0a..01263549 on branch adversarial-review
    title: 22 implementation and test commits (398 files changed, 27421 insertions(+), 12529 deletions(-))
  - id: frame-perf
    resource: crates/mg-ui/tests/frame_perf.rs
    title: Release frame-budget suite at 3440x1400 (budget 6.94 ms) across Tyrants of the Moonsea, persistent-world module, grass field, Sound Ranges (area084), and Shaders switch
  - id: editor-perf
    resource: crates/mg-ui/tests/editor_perf.rs
    title: Release editor frame-budget suite (wide creature, 13,869-line script, 15,100-row 2DA table, texture picker, Creature Wizard)
  - id: nui-perf
    resource: crates/mg-ui/tests/nui_perf.rs
    title: Release NUI Creator frame-budget suite (1M-sample chart, large window, variants, text-heavy layout)
  - id: world-perf
    resource: crates/mg-ui/tests/world_perf.rs
    title: Release persistent-world throughput suite (149 MB module, 25,028 resources, 300 areas)
  - id: tdod-haks
    resource: /home/august/Projects/tdod-module/build/hak
    title: The Domains of Dread haks (MOONGLOW_HAKS) for game_shaders_frames_at_144 (area woods)
  - id: ci
    resource: .github/workflows/ci.yml
    title: Workspace fmt, Clippy, check, feature-isolation, and everyday cargo test suite gates
---

# Adversarial review: 5.1.0 performance, bugs, complexity and test speed

Requested across the whole `moonglow-toolset` workspace on branch `adversarial-review` (base `7a65de0a`, code HEAD `01263549`; 22 code commits touching `398` files with `27421 insertions(+), 12529 deletions(-)`).[^owner][^commits] The targets were a `6.94 ms` (`144 fps`) native ultrawide (`3440×1400`) budget on the host machine (AMD Ryzen 9 9950X + Radeon RX 9070 XT, Vulkan RADV) including dense persistent-world (PW) scenes, resolution of discovered bugs, structural deduplication and comment truthfulness, and a faster everyday test loop.

All six release performance binaries (`editor_perf`, `nui_perf`, `world_perf`, `frame_perf --skip game_shaders_frames_at_144`, and `game_shaders_frames_at_144` with `MOONGLOW_HAKS` set, both with shadows on and `FRAME_SHADOWS=off`) now exit `0` against four failing baseline suites, and the everyday workspace test run (`--workspace --exclude mg-corpus-tests`) dropped from `8m 27.902s` (with `1` failure) to `2m 28.869s` (`5,019 passed; 0 failed; 59 ignored` across `75` binary/doctest runs).[^commits][^ci]

## 144 fps release performance (`3440×1400`, budget `6.94 ms`)

Measured cleanly on HEAD `01263549` (no `nwmain-linux`, no background `ffmpeg`, starting load average `12.35`).

| Suite | Baseline (`7a65de0a`) | Final (`01263549`) | `OVER` / `spikes` |
|---|---:|---:|---:|
| `editor_perf`[^editor-perf] | `FAILED 4/5` (`25.39s`) | **`ok 5/5`** (`8.05s`) | `0` / `0` |
| `nui_perf`[^nui-perf] | `ok 6/6` (`8.74s`) | **`ok 6/6`** (`1.14s`) | `0` / `0` |
| `world_perf`[^world-perf] | `ok 1/1` (`10.78s`) | **`ok 1/1`** (`4.81s`) | `0` / `0` |
| `frame_perf` (`--skip game_shaders`)[^frame-perf] | `FAILED 6/10` (`321.36s`) | **`ok 10/10`** (`110.42s`) | `149` / `101` -> **`28` / `10`** (`28` all in `KNOWN_OVER` `area084`) |
| `game_shaders` (`MOONGLOW_HAKS`)[^frame-perf][^tdod-haks] | `FAILED 0/1` (`306.74s`) | **`ok 1/1`** (`130.64s`) | `126` / `83` -> **`23` / `7`** (`23` all in `KNOWN_OVER` `woods`) |
| `game_shaders` (`FRAME_SHADOWS=off`)[^frame-perf][^tdod-haks] | `FAILED 0/1` (`321.94s`) | **`ok 1/1`** (`133.33s`) | `127` / `82` -> **`23` / `7`** (`23` all in `KNOWN_OVER` `woods`) |

### Selected before-and-after measurements

- **Script and UI editors (`editor_perf`)**:[^editor-perf] `editing_a_huge_script` (`13,869` lines, viewport-virtualized `code_edit`, incremental line layout, delta undo, and off-thread symbols/live-check) dropped on `open` (`59.67 -> 26.20 ms`), `first frame` (`388.09 -> 24.36 ms`), `idle` (`2.91 -> 0.35 ms`), `typing a character` (`7.24 -> 1.05 ms`), `typing at a typist's pace` (`9.58 / 18.26 -> 0.46 / 5.30 ms`), `deleting` (`12.94 -> 0.44 ms`), `new line` (`10.55 -> 0.42 ms`), `scrolling` (`16.58 -> 0.35 ms`), `selecting a page longer` (`9.43 -> 0.40 ms`), `composing (IME)` (`12.18 -> 0.81 ms`), and accessibility-tree typing (`93.57 -> 25.81 ms`). `a_wide_creature_every_page_drawn` idle fell from `6.65` to `1.48 ms` (first frame `124.45 -> 85.29 ms`), and `editing_a_large_table` (`15,100` rows) idle dropped from `2.22 ms` to `747.34µs`.
- **NUI Creator (`nui_perf`)**:[^nui-perf] single-pass chart downsampling, `Work::verified_clean` quiet-frame diff skipping, and bounded preview formatting reduced `a_chart_of_a_million_samples` idle from `105.83 ms` (`worst 175.90 ms`) to `458.22µs` (`worst 674.37µs`), the bound chart from `51.22 ms` to `487.96µs`, and `a_large_window` edit frames from `6.41 ms` (`worst 17.36 ms`) to `1.97 ms` (`worst 4.32 ms`), with idle at `815.80µs`.
- **PW scale (`world_perf`, 149 MB module, 300 areas)**:[^world-perf] first-frame costs dropped on `Module Properties` (`15.89 -> 4.97 ms`), `module tree with 8,000 items` (`51.36 -> 11.93 ms`), `custom item palette (8,000)` (`27.99 -> 11.82 ms`), `store inventory with 1,000 items` (`64.43 -> 22.38 ms`), `placeable's 20,500 appearances` (`69.10 -> 13.98 ms`), `creature's 17,100 appearances` (`56.59 -> 16.25 ms`), `Area Properties (300 areas)` (`37.87 -> 21.11 ms`), and `resource browser` (`386.52 -> 172.85 ms`); `rebuild custom palettes` finished in `116.49 ms`, `verify` in `161.41 ms`, and `content doctor` in `735.87 ms`.
- **3D renderer and PW scene interactions (`frame_perf`)**:[^frame-perf] bind-group caching, selective `gone_mats` eviction, 2D DDA `Ground::hit`, static preview target reuse, and virtualized lists/pickers brought all non-`KNOWN_OVER` views inside `6.94 ms`:
  - `area026` turning pointer: `8.86 -> 1.66 ms` (`ok`).
  - `area092` (784 tiles, 61 objects): overview wheel `11.09 -> 2.73 ms`, turning pointer `9.37 -> 1.92 ms`, walkmesh pointer `14.65 -> 3.98 ms`, turning ring dragged `9.28 -> 2.05 ms`.
  - `creature in the Preview tab` pointer: `11.76 -> 2.41 ms`; `armor nw_aarcl001` wheel: `9.25 -> 1.80 ms`.
  - PW `module tree` filtered pointer: `7.95 -> 2.00 ms`; `Advanced Search, 8805 hits ticked`: `10.94 -> 1.06 ms`.
  - `field, across, grass` pointer: `11.06 -> 3.21 ms`; `reload after a pull` (`21,000`-file nasher project): longest `7.84 -> 2.9 ms` (`0` over budget, median `0.31 ms`).
  - `area084` (676 tiles, 549 objects): full-view zoom wheel and walkmesh dropped from `21.86–31.50 ms` down to `8.25–10.80 ms` (all 28 remaining `OVER` entries are tracked in `KNOWN_OVER`).
- **TDoD shaders (`game_shaders_frames_at_144` with `MOONGLOW_HAKS`)**:[^frame-perf][^tdod-haks] `area092` and `lake` with `Shaders on` dropped to `0` over budget (`area092` walkmesh pointer `13.67 -> 3.28 ms`; `lake` turning pointer `11.04 -> 2.24 ms`, Paint pointer `8.83 -> 2.06 ms`, walkmesh pointer `11.21 -> 3.57 ms`). In TDoD `area woods` (`576` tiles, `400` objects, `Shaders on`, shadows on), close and interactive editing dropped inside the `6.94 ms` budget: `close` pointer/idle/wheel `12.65/11.58/12.87 -> 4.17/3.98/4.40 ms`, `turning` pointer/idle `14.22/14.06 -> 6.22/6.71 ms`, `Paint` pointer/idle `9.77/9.14 -> 2.62/0.61 ms`, `32 selected, Shift held` pointer `8.57 -> 3.70 ms`, and selection drag/turn/arrow `10.58–11.61 -> 3.84–4.27 ms`. The 23 remaining `KNOWN_OVER` wheel and walkmesh rows dropped from `20.21–69.76 ms` (`Shaders off`) / `19.49–69.27 ms` (`Shaders on`) — and up to `95.06 ms` in the `FRAME_SHADOWS=off` baseline run — down to `8.98–32.03 ms` (`13.05–23.90 ms` `Shaders off`, `8.98–32.03 ms` `Shaders on`).

## Key bug fixes across waves

- **Core, format, and renderer (`592ff73d`)**:[^commits] fixed bounds/validation, resource loading, tile/shader defaults, and rendering correctness bugs across `mg-render`, `mg-tiles`, `mg-preview`, `mg-shader`, `mg-area`, `mg-mdl`, `mg-audio`, `mg-gff`, `mg-image`, `mg-resman`, and `mg-2da`.
- **Module, script, edit, plugin, CLI (`885106ab`, `ccfb8cfe`)**:[^commits] resolved `mg-module` cache/glob/safety bugs; fixed `mg-script` parameter scopes and `Index::text`; replaced whole-resource invalidation in `mg-edit::fix` with per-`ListAt` tracking and added `Workspace::run` rollback snapshots; clamped UTF-8 offsets in `mg-lsp`; added atomic `.mg-bake-bak` rollback in `mg bake`; fixed `.`/bare `nasher.cfg` handling in `mg serve` and `mg watch`; resolved `StoreList` selection resref in `mg-plugin`.
- **UI, UIKit, NUI, blueprints (`0e8606b4`, `36f34682`, `3f116b9d`, `01263549`)**:[^commits] fixed protected-area plugin/import hold confirmation and rollback, `Color` validation in plugin settings, `write_export` append error preservation, module-scoped state resets on close/load-order change, `held_text` commit/Cancel preservation (`Action::PopInProperties`), `faction_view` rollback on failed apply, `store_wizard` `StoreListKey` invalidation, `mg-nui` token-based `rename_calls` / `script_calls_function`, `blueprint_wizard` avoiding `&Arc<GameData>` retention (`kept_for_key`, preventing `10s` `RELEASE_WAIT` stalls), `nui_view::mark_mutated` quiet-frame invalidation, `background_jobs == false` inline execution in `script_nav::live_check` / `script_tools::symbols_for_ui`, and the `mg-plugin` duplicate scratch-dir collision (`mg-plugin-package-limits`).

## Deduplication, complexity, comments, and Clippy sweep (`e6c9162c`, `9821715f`)

- **Non-UI crates (`e6c9162c`)**:[^commits] shared `placed_at`/`place` between `mg-rules::levelup::new_class_gear` (removing `0..=255` clamping) and `mg-edit::lists`; extracted `cloak_textures` (`mg-preview`), `Grid::lay_crosser` simplification (`mg-tiles`), `resource_differs`, `line_ending_len`, `check_table!`, and Ink parser helpers (`mg-module`); decomposed eight `cmd_*` handlers out of `apps/mg/src/main.rs` and shared helpers in `diff.rs`, `edits.rs`, and `plugins.rs`.
- **UI and UIKit (`9821715f`)**:[^commits] extracted `move_focus_with_arrows`, `section_heading_rule`, and `menu_mark_row` in `mg-uikit`; introduced `widgets::header_with_ok_cancel` / `buttons_end_width` across nine `mg-ui` editors and `blueprint/mod.rs`; shared `saved::describe_lines`, `open_loc_edit_on_click` (with `dialog_pattern::SHARED_MAKERS` in sync), `store::restrict_fields`, and `encounter::creature_grid`; removed dead `"tile-properties"` handling in `tabs.rs`.
- **Comments and Clippy**:[^commits] updated stale module/function doc comments (nine blueprint modules, `ok_cancel`, `mg-uikit/src/lib.rs`, `options.rs`, `screens.rs`), stripped noisy milestone tags in edited scopes, and cleared all workspace default Clippy warnings plus strict `--lib --bins` (`redundant_clone`, `needless_collect`) warnings.

## Test suite speedup and reliability (`cc909a1e`, `48fadb2d`, `af774c77`)

- **Measured improvement (`--workspace --exclude mg-corpus-tests`)**:[^commits][^ci] wall time fell **3.4×** from `8m 27.902s` (`489.32s` binary sum, `1` baseline failure in `plugins::the_plugins_folder_s_path_takes_the_room_there_is`) to **`2m 28.869s`** (`144.64s` binary sum, `5,019 passed; 0 failed; 59 ignored` across `75` binary/doctest runs on `01263549`).
  - `mg-ui::app`: `397.63s` (`FAIL(1)`) -> **`96.25s`** (`2012 passed; 0 failed; 48 ignored`).
  - `mg-ui` (`lib`): `30.32s` -> **`10.13s`** (`605 passed; 0 failed; 3 ignored`).
  - `mg-render::render`: `27.93s` -> **`11.18s`** (`97 passed; 0 failed; 1 ignored`).
  - `mg-plugin`: consolidated 7 integration binaries into 1 (`plugins`, `27 + 96` passed) with one unified `TestHost`.
- **Reliability mechanisms**: converted non-pixel `mg-ui` area tests to CPU-only `area_ui_harness`, gated unasserted debug `.render().save()` calls behind `shot_dir` / `MG_SHOT`, removed `Box::leak`/`String::leak` leaks, scaled pixel scans by `ppp`, replaced shared game data in `talk_panels::talk_window_with` with a fresh `Arc<GameData>`, and added `corpus_required()` guards in `apps/mg/tests/json.rs` and `mg-corpus-tests` line-diff fixes (`diff_lines`).

## Final verification on HEAD `01263549`

| Check | Outcome | Notes |
|---|---|---|
| `cargo test --workspace --exclude mg-corpus-tests` | `5,019 passed; 0 failed; 59 ignored` | `75` runs, `2m 28.869s` real |
| `cargo fmt --all -- --check` | Exit `0` | — |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit `0` | — |
| `cargo clippy` (`screens`/perf tests, `client_*` tests, `--no-default-features` for `mg`/`mg-preview`/`mg-area`) | Exit `0` | Matches CI[^ci] |
| `cargo clippy --workspace --lib --bins -- -D warnings -W clippy::redundant_clone -W clippy::needless_collect` | Exit `0` | Strict pass |
| `RUSTFLAGS="-D warnings" cargo check --workspace --all-targets` | Exit `0` | — |
| `mg-area --no-default-features` feature isolation | `OK` | No `mg-edit`/`mg-module`/`mg-script` |
| All 6 release perf runs (`3440×1400`) | All `6` exit `0` | `area084` (`28`) & `woods` (`23`) remain in `KNOWN_OVER`; `frame_perf` (`10`) & `game_shaders` (`7`) have isolated single-frame worst-case `spikes` |
| `mg-corpus-tests` full execution & `client_*` game-client tests | Not run here | `client_*` Clippy & `--all-targets` check passed; `aurora` sample run (`5 passed`) earlier in Wave 3B-4a |

What remains tracked in `KNOWN_OVER` for future GPU/scene work: full-area zoomed-out wheel frames (`8.25–10.52 ms` in `area084`, `8.98–32.03 ms` in TDoD `woods`) and full-area selected walkmesh overlays on those two densest maps (`10.06–10.80 ms` in `area084`, `16.13–23.56 ms` in `woods` with shadows on).

[^owner]: Owner adversarial review request, 2026-11-23.
[^commits]: `git log 7a65de0a..01263549` on branch `adversarial-review` (`398` files changed, `27421 insertions(+)`, `12529 deletions(-)`).
[^frame-perf]: `crates/mg-ui/tests/frame_perf.rs`, run in `--release` with `-- --ignored --nocapture --test-threads=1` at `3440×1400` (`6.94 ms` budget).
[^editor-perf]: `crates/mg-ui/tests/editor_perf.rs`, run in `--release` with `-- --ignored --nocapture --test-threads=1`.
[^nui-perf]: `crates/mg-ui/tests/nui_perf.rs`, run in `--release` with `-- --ignored --nocapture --test-threads=1`.
[^world-perf]: `crates/mg-ui/tests/world_perf.rs`, run in `--release` with `-- --ignored --nocapture --test-threads=1`.
[^tdod-haks]: `MOONGLOW_HAKS=/home/august/Projects/tdod-module/build/hak` passed to `game_shaders_frames_at_144`.
[^ci]: `.github/workflows/ci.yml` (`cargo fmt`, Clippy targets, `--no-default-features` isolation check, `cargo check --workspace --all-targets`, and everyday `cargo test --workspace --exclude mg-corpus-tests`).
