---
type: Gotcha
title: "Script window pane sizes kept per kind: DockLayout.window_width/window_bottom, seeded when the pane is drawn anew"
description: "How a floating script window's Reference width and bottom pane height are kept across windows and restarts, and why egui's own panel memory has to be cleared for it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T14:52:51Z }
touches: ["crates/mg-ui/src/shell.rs:seed_window_pane", "crates/mg-ui/src/shell.rs:keep_window_size", "crates/mg-ui/src/settings.rs:DockLayout"]
asks:
  - Why does a new script window not open at the pane width I dragged?
  - "Where is a floating window's pane size saved?"
  - "Why is egui's PanelState removed in window_panes?"
fingerprint: v2-665388cdb1d08e14
created: 2026-11-23
author: augustaugur
---
5.2.0 W5. A floating editor's panes (`shell::window_panes`) are egui `Panel`s whose size egui keeps per id; `default_size` applies only to an id with no stored `PanelState`. The right pane's id is `ui.unique_id()` of the tab's ui (per tab, not per window), the bottom pane's `("window-bottom", owner)`. So a script reopened, moved to another window or restored from egui's persisted memory takes its own old size, never the kept one.

- Kept in `settings.dock_layouts[kind].window_width / window_bottom` (u16, skipped when None), written by `keep_window_size` only while one of the pane's edges is being dragged (egui's `__resize` id or the window seam `window_edge_id(pane).with("seam")`), never when the window's own size clamps the pane.
- `seed_window_pane` removes the egui `PanelState` when the pane was not drawn in the previous pass (opened, tab brought to front, first pass of a run) and a kept size exists, so `default_size(kept)` applies. Drawn pass after pass, the pane keeps its own size. An owner-based check was tried first and failed: the tab id does not change when a tab moves windows.
- Test: script_bottom::a_script_window_s_pane_sizes_are_kept_for_the_next (drag both edges, close and reopen the other script, compare).
