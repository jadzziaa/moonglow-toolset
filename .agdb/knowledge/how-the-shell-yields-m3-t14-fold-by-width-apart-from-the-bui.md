---
type: Decision
title: "How the shell yields (M3 T14): fold by width apart from the builder's fold, sizes not stored until dragged, a test hook for the old tests"
description: "Why narrow windows fold the side docks through shell::left_folded/right_folded and never hide_tree, why chosen dock sizes are None until dragged with a one-time take (dock_sizes_taken), and why old UI tests run with docks_fold_by_width off."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T05:48:56Z }
touches: ["crates/mg-ui/src/shell.rs:left_folded", "crates/mg-ui/src/shell.rs:set_dock_open", "crates/mg-ui/src/shell.rs:take_dock_sizes", "crates/mg-uikit/src/shell.rs:Yields", "crates/mg-ui/tests/app/common.rs:docks_as_set", crates/mg-ui/tests/app/yields.rs]
asks:
  - Why does the Left Dock fold by itself in a narrow window without changing hide_tree?
  - "Why is Settings::left_dock_width None until the edge is dragged, and what is dock_sizes_taken?"
  - Why do old UI tests at 1200 wide still see both docks open?
  - "Where do the editor area's rects in kittest tests start, 8 points in?"
fingerprint: v2-706a6172f3651946
created: 2026-11-14
---
## Decided
- One `Yields` a frame (`shell::frame_yields`, `Moonglow::yields`), from the window's width; the Top Bar, Quick Open, the docks and the Status Bar read it. Thresholds live in `mg_uikit::shell` (1300, 1400, 1500, 2560, and T14's own `RIGHT_DOCK_STRIP_UNDER` 796 = strip + editor least 490 + 280 + lines; `shell::BOTH_DOCKS_UNDER` 962).
- Every read of a side dock's fold goes through `shell::left_folded` / `right_folded`, every write through `shell::set_dock_open`. In a window that folds a dock, unfolding it sets `left_unfolded_here` (runtime), never `hide_tree`; it is cleared when the window is wide enough not to fold it, so narrowing again folds it again. Under 962 the dock not unfolded last folds (`unfolded_last`). F6 stops at a dock folded by width and unfolds it (`folded_by_width`).
- Chosen sizes are `None` until an edge is dragged (`chosen_size(kept, default)` stores nothing): only so can a dock "not chosen" take the wide layout's 320/741 at 2560+. `take_dock_sizes` runs once for good (`Settings::dock_sizes_taken`, true for a new installation): it takes 4.2.0's egui PanelState as chosen, and drops sizes that are only sizes at first (240/260 tree, 320 right, 120/180 log) that this branch stored. Never take PanelState more than once: draw_at writes it every frame (a narrow window's drawn width would become "chosen").
- Under 1400 the Right Dock is drawn exactly 280 and its edge is not draggable.
- The editor area is flush (CentralPanel frame without egui's 8 px margin): the boards' 974 at 1280.
- The Outliner rule (T13 review ruling) reads `left_folded`, not `hide_tree` (tree.rs `follow_selection`).
## Watch out
- `Moonglow::docks_fold_by_width` (hidden, default true) is turned off by `common::docks_as_set` in every app-test harness builder: about 160 tests written before T14 look at dock contents at 1000-1400 wide. Tests of the yields turn it back on (`yields.rs`, the area toolbar sweep, the Outliner's narrow test). screens.rs and frame_perf run with it on.
- kittest's harness keeps an 8 point outer margin round the app: editor/dock rects in app tests are 8 in, and the window's thresholds see 16 points more than the room.
- The Outliner's strip button is named "Outliner" like the Outliner group (`mg_ui::OUTLINER`): a folded dock still answers `query_by_label(OUTLINER)`.
- An area's view takes the arrow keys back while the pointer is over it: to test F6, hover the Top Bar.
