---
type: Decision
title: "M3 T14 review: the yields on in every UI test, fold controls inside the docks, keys leave folded docks, one width per layout"
description: "What T14's review changed: no test hook for the docks (tests unfold by hand or use a 1600 window), fold button at the tab bar's end and chevron at the strip's top, F6 only among what is drawn, keys leave a folded dock, Right Dock width kept per layout, sizes taken only as whole sets."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T09:01:15Z }
touches: ["crates/mg-ui/src/shell.rs:keys_leave_folded_docks", "crates/mg-ui/src/shell.rs:take_dock_sizes", "crates/mg-ui/src/shell.rs:right_dock_chosen", "crates/mg-ui/tests/app/common.rs:unfold_left_dock", "crates/mg-ui/tests/app/common.rs:wide_window", crates/mg-ui/src/open_tabs.rs]
asks:
  - How should a UI test get the module tree in a window under 1500 wide?
  - "Where is the side docks' fold button now?"
  - Does F6 unfold a folded dock?
  - Why are there two Right Dock widths in the settings?
fingerprint: v2-8d8e0e208a68a601
created: 2026-11-14
---
## Decided
- No hook: `docks_fold_by_width` is gone. A test that needs a dock a narrow window folds unfolds it as a builder would (`common::unfold_left_dock`, `unfold_right_dock`: the strip's chevron clicked), or runs in a window from 1500 (`common::wide_window`, 1600) where it reads the builder's own fold (`hide_tree`). In a narrow window folding by hand writes no setting (it ends the unfolding by hand): a test about `hide_tree` needs the wide window. screens.rs has its own `unfold_left_dock`.
- Fold controls (the orchestrator, by the boards): open dock: `kit::fold_button_tip` at its tab bar's inner end (the Left Dock's tabs show icons at 260 because of it); folded: `FoldStrip`'s chevron at the strip's top. The edge buttons over the editor area are gone; `left_fold`/`right_fold` too. `Moonglow::left_dock_rect` gives the dock's edge to tests.
- Orchestrator's decisions, reversible: F6 goes only among what is drawn (no unfolding); the arrow keys of a folded dock's panels go to the editor area's active editor (`shell::keys_leave_folded_docks`, `Moonglow::editor_keys`) and are not handed back.
- Asked by name (Show in Module Tree, Tools › Resource Browser, Palettes, Preview) goes through `show_dock_panel` → `set_dock_open`, unfolding a dock folded by width. Restoring open tabs writes no fold (only `left_panel`/`right_panel`).
- `Settings::right_dock_width_wide`: the Right Dock's width dragged in the wide layout (2560+), apart from the one below; old files map to the narrow one. The Left Dock and the seam keep one value (no threshold makes them a trap).
- `take_dock_sizes`: only whole sets count as "at first" (260/320/180, 240/320/120, 4.2.0's 240+120), the tree's among them.
## Watch out
- The Status Bar sweep's long texts hide the 1400 rule (room alone gives icons): its short-text pass after the job is the check.
- `common::click_tab_close` takes the rightmost close square in the tab: another tab's under a floating window can lie inside its rect.
- A test that sets the Right Dock's width in a window from 2560 sets `right_dock_width_wide`: `right_dock_width` is not read there (frame_perf's one-column view waited forever until it did).
- Full frame_perf runs on a busy machine put area views 0.3 to 0.5 ms over an older full run; T14's review timed against T13's tree interleaved (a `git archive` copy built apart, each binary run from its own `crates/mg-ui`): mean difference 0.00 ms over 237 views. Interleave before blaming a change.
