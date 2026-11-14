---
type: Decision
title: "M3 T14 second review: what a UI test of the docks must do, keys leave panels not drawn, sizes at first per dock"
description: "Why a UI test under 1500 points must not write hide_tree or hide_palettes, how the docks' helpers behave, which panels the arrow keys leave, and which older dock sizes count as untouched."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T11:27:06Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-14T11:52:53Z }
touches: ["crates/mg-ui/src/shell.rs:keys_leave_panels_not_drawn", "crates/mg-ui/src/shell.rs:take_dock_sizes", "crates/mg-ui/src/shell.rs:folds_by_choice", "crates/mg-ui/tests/app/common.rs:unfold_left_dock", "crates/mg-ui/tests/app/common.rs:wide_window", "crates/mg-ui/src/palette_view.rs:HOVER_REST"]
asks:
  - "My UI test sets settings.hide_tree in a 1280 window and nothing changes: why?"
  - How do I fold or unfold the Left Dock in a narrow-window UI test?
  - "The arrow keys moved to the area when the Palette or Outliner went away: is that intended?"
  - Which old dock widths are dropped as never chosen when settings from an older version are read?
fingerprint: v2-8ad0d4ad4bc2177f
created: 2026-11-14
---
## Decided
- Below 1500 points `shell::folds_by_choice` reads `left_unfolded_here`, never `settings.hide_tree` (the Right Dock likewise below 796 with `right_unfolded_here`/`hide_palettes`). A test that writes `hide_tree` or `hide_palettes` in a narrow window changes nothing and passes on nothing: T14's review left several tests so (their "folded" passes ran with the dock open). A test about the builder's own fold runs in `common::wide_window` (1600, settled); a test about a narrow window folds through the dock's fold button and unfolds through the strip's chevron (`common::unfold_left_dock`, `unfold_right_dock`, which now fail when there is neither a fold button nor a chevron and assert a panel shown after). Under 962 unfolding one side dock folds the other (`BOTH_DOCKS_UNDER`).
- An action that shows a panel (`Action::ShowPanel`, Tools › ...) unfolds its dock: set a fold after it, not before.
- `keys_leave_panels_not_drawn`: the arrow keys leave any side-dock panel not drawn this frame (dock folded, Right Dock holding no panel beside a non-area tab, Outliner gone with the area's tab) for the editor's (`Moonglow::editor_keys`). Exception: a dock drawn on its other tab hands them to that tab's panel: the Left Dock's Tree/Browser keys are left to `left_dock`, which hands them over as it draws (resources_panel's F6 test holds that); the Right Dock's Palette/Preview keys are handed over in `keys_leave_panels_not_drawn` itself.
- `SIZES_AT_FIRST` is per dock, (tree 260 or 240, Right Dock 320, log 180 or 120), any mix or one alone counts as untouched (each was stored apart where its dock was drawn on the first frame of a version from M3 T5 to T13; 240/120 are every release's egui defaults). Any other size keeps the whole file's sizes.
- The palette's hovered row picture waits `palette_view::HOVER_REST` (0.12 s), not the tooltip delay: the wheel moves a row under the pointer each frame so it still asks for none while scrolling.
## Watch out
- `common::click_tab_close`/`tab_rects` pick tabs in the bar's own layer (Background for docked, the window's Middle layer for floating); a floating window over the bar no longer shifts the index.
- The Bottom Dock's narrowest stages (icon with fewer than two tools, nothing) are reached only below the 420 least window since the yields; status_bar's sweep starts at 300 (the harness does not hold the least size).
