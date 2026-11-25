---
type: Gotcha
title: "Floating window pane sizes kept per kind (every kind): DockLayout.window_width/window_bottom, seeded when drawn anew, saved a few passes past the drag"
description: "How a floating window's right pane width and bottom pane height are kept per editor kind across windows and restarts, why egui's panel memory is cleared for it, and why the save runs a few passes after the drag ends."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T08:44:34Z }
touches: ["crates/mg-ui/src/shell.rs:seed_window_pane", "crates/mg-ui/src/shell.rs:keep_window_size", "crates/mg-ui/src/shell.rs:window_kept", "crates/mg-ui/src/settings.rs:DockLayout", crates/mg-ui/tests/app/window_pane_sizes.rs]
asks:
  - Why does a new window not open at the pane width I dragged?
  - "Where is a floating window's pane size saved?"
  - "Why is egui's PanelState removed in window_panes?"
  - "The pane size I let go of is not the one kept, but one a frame earlier: why?"
fingerprint: v2-7f5d626a81784273
created: 2026-11-23
author: augustaugur
---
5.2.0 W5 made this for scripts alone (`KEPT_WINDOW_KINDS`); the owner's ruling of 2026-11-25 extends it to every window kind, and the constant is gone (`window_kept` takes any kind).

A floating editor's panes (`shell::window_panes`) are egui `Panel`s whose size egui keeps per id; `default_size` applies only to an id with no stored `PanelState`. The right pane's id is per tab, not per window, the bottom pane's `("window-bottom", owner)`. So an editor reopened, moved to another window or restored from egui's persisted memory takes its own old size, never the kept one.

- Kept in `settings.dock_layouts[kind].window_width / window_bottom` (u16, skipped when None), never when the window's own size clamps the pane (no choice of the builder's).
- `seed_window_pane` removes the egui `PanelState` when the pane was not drawn in the previous pass (opened, tab brought to front, first pass of a run) and a kept size exists, so `default_size(kept)` applies. An owner-based check was tried first and failed: the tab id does not change when a tab moves windows.
- **The pane lags the pointer a pass.** Saving only on passes where an edge `is_being_dragged` kept the size from one pass before the release when the drag was let go while moving. `keep_window_size` now stamps the pass number of the last dragged pass in egui temp data (`("window-size-dragged", edges)`) and keeps saving for up to 3 passes after it (working tree of `ui-ux-fixes`, 2026-11-25; `git diff 62e1c507 -- crates/mg-ui/src/shell.rs`).
- The "dragged" slot is per window (keyed by its edge ids), not per kind: keyed by kind, two windows of a kind overwrote each other's mark and the later kept its own untouched size (second review round; test in `window_pane_sizes.rs`).
- Tests: `script_bottom::a_script_window_s_pane_sizes_are_kept_for_the_next`; the other kinds in `tests/app/window_pane_sizes.rs`.
