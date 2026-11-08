---
title: macOS file dialog crash and live tint picking
author: August
date: 2026-11-08
branch: develop
cost: { minutes: 1, tokens: 1300 }
touches:
  - Cargo.lock
  - Cargo.toml
  - apps/mg/src/plugins.rs
  - crates/mg-area/src/lib.rs
  - crates/mg-plugin/src/kinds.rs
  - crates/mg-plugin/src/lib.rs
  - crates/mg-plugin/src/manifest.rs
  - crates/mg-plugin/src/runtime.rs
  - crates/mg-plugin/tests/examples.rs
  - crates/mg-ui/src/area_view.rs
---

## Decided
- The macOS abort on resizing a file dialog (or a Finder drag over the window while one is open) is winit#3992: rfd's blocking `runModal` runs a nested run loop inside the egui frame, a queued event re-enters winit's handler, winit panics in a callback that can't unwind. Fixed by vendoring winit 0.30.13 in `vendor/winit` (`[patch.crates-io]`, `exclude` from the workspace so clippy/fmt skip it) with one change in `src/platform_impl/macos/event_handler.rs`: the event is queued and handled after the handler returns. Drop it when winit fixes #3992 in a release eframe uses (see `vendor/winit/MOONGLOW.md`). Type-check it with `CARGO_TARGET_DIR=target/mac-check cargo check -p winit --target aarch64-apple-darwin`.
- A color being picked is previewed, not committed: windows push `(key, path, VarTable)` to `Moonglow::var_preview_next`; it becomes `var_preview` at the next frame's start; `area_view::preview_variables` re-reads just those objects with `AreaModel::read_object` and `scene.update_objects`, and restores them when the preview ends. The Variables page holds its commit while a swatch's picker is open (`widgets::picking()` temp flag set by `color_swatch`); the Tint section and a plugin form's color field (`variable`, API 0.8) use the same path.
## Tried and ruled out
- Converting the 63 `app.dialogs.*` call sites to request-plus-continuation (what other apps did): equally unverifiable here and far bigger; kept as the lasting option in `docs/deferred.md` if winit never fixes it.
- Timing a tint commit (frame_perf `tint_edits`): 1-7 ms in the campaign and TDoD; the lag was a command (and Undo step) per drag frame, not a module walk.
## Watch out
- A UI test with `background_jobs = true` and an area view must `settle` with background jobs off before it ends, or the process can segfault at exit (threads touching the GPU).
- The Variables list needed 690 points; in narrower Properties windows the swatch and Delete were clipped off (click by node then misses). Fields now shrink to two fifths.
