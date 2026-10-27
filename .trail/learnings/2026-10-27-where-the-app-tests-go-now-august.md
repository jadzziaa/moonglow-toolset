---
title: Where the app tests go now
author: August
date: 2026-10-27
branch: split-app-tests
cost: { minutes: 1287, tokens: 11336306 }
touches:
  - crates/mg-ui/tests/app/main.rs
  - crates/mg-ui/tests/app/common.rs
---

The application's UI tests are no longer one file: `crates/mg-ui/tests/app.rs` became `crates/mg-ui/tests/app/`, one binary (`--test app`, cargo finds `tests/app/main.rs`) with twenty subject modules named in `main.rs` (area_view, areas, blueprints, conversations, drawing, fields, inventories, looks, modules, options_and_keys, palettes, plugins, projects, resources, scripts, search, tables, tiles, tree, windows) and `common.rs`.

Where a new test goes: the module of what it exercises. A helper used by one module stays in it (private); one used by two or more goes in `common.rs` as `pub(crate)`; every module starts with `use crate::common::*;`, which also brings the old file's imports (Harness, NodeT, Queryable, ResKey...). A new file must be named in `main.rs`: nothing checks it (deferred.md says so), and its tests would silently not run.

Names: tests keep their names under their module (`tiles::f10_over_an_area_selects_tiles`). `cargo test -p mg-ui --test app NAME` still finds a test by substring; `--exact` needs `module::NAME`. To compare lists, strip everything up to the last `::`.

Set-up helpers in common.rs: `game_app(root, user)` / `game_app_with(root, user, dialogs)` (the app on the install, English), `harness(app, w, h)` (the window at a size; its `render()` opens a GPU device of kittest's own, so a test that renders takes `mg_testkit::gpu::hold()` first), `gpu()` (takes the lock, skips without an adapter, returns the render state to give `app.set_render_state` and) `gpu_harness` / `area_gpu_harness(app, rs, w, h)` (the latter with frames a 60th of a second apart, for the area view's double clicks). `area_harness*`, `blueprint_harness`, `game_harness`, `plugin_harness` are in common.rs too.

The GPU lock, checked whole in October 2026: every test that took `hold()` opens a device, either a render state or `h.render()` (egui_kittest 0.36's default `LazyRenderer` makes a wgpu renderer on the first render, with the crate's `wgpu` feature), so the old deferred note that a dozen took it for nothing no longer held; one test rendered without it (`every_area_is_given_the_scheme_s_tile_lights`) and takes it now.

How it was split: a script parsed the file into top-level items (a column-0 item line with its comments and attributes before it), placed each test by a name-and-helper classification and each helper by which modules reach it, and checked every item came through byte for byte (helpers moved to common only gained `pub(crate) `). Replacements of the set-ups were regexes on exact shapes, counted, with the changed lines checked to be set-up lines alone (no assert line touched).
