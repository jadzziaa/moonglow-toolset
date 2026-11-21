---
type: Decision
title: "M4 closing pass (U30): what counts, how timing against U0 was read, what stays"
description: "How U30 closed M4: the deferred count's method, the moves file's Not Drawn bullets, the floating tab menu's Eject, why U0's logs alone could not judge the frame times, and the review's correction: area084's wheel rows were the test's camera, not U6."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T06:23:24Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-21T02:40:18Z }
touches: [docs/deferred.md, docs/ui-redesign-moves.md, crates/mg-ui/tests/app/open_editors.rs, crates/mg-ui/src/tabs.rs, crates/mg-ui/tests/frame_perf.rs, crates/mg-ui/tests/editor_perf.rs]
asks:
  - "How is the deferred count counted, and why did M4 close only one entry?"
  - "Every editor view is slower than U0's frame_perf log: is the machine busy or did M4 cost it?"
  - "Why does a floating window's tab menu have no Eject?"
  - "frame_perf fails at 1280 by 800 with 'Faction not shown' or 'no S29 row': why?"
  - "Why did area084's wheel rows beside a dock panel get slower at U6 when U6's code costs nothing there?"
fingerprint: v2-b7a1453fddf598aa
created: 2026-11-21
author: augustaugur
---
## Decided
- The deferred count is `grep -c '^- \*\*' docs/deferred.md`: every bold top-level entry, the open sections, "Questions asked, unanswered" and "Settled" alike (nested `  - **` sub-entries not counted). 230 at 11c618df (79 open, 7 questions, 144 settled), 229 after M4: the one closed was "The palette's header, not as drawn in the issue" (a Settled entry, overturned by U6's row; wholly closed: its one fact still true, "More" a word, is what the boards draw; evidence `properties_panel::the_narrow_dock_has_three_tabs_and_the_narrow_row`, board_main, board_narrow). The plan's 1922 (Tile Properties' lights and loops) and 1428 (Creature Properties' overlapping elements) were restated, not closed, because their facts still hold; 362 (a preview shown twice) is the owner's call.
- A moves file "Not Drawn" bullet names a decided state, not a control to build: tick it where the code keeps what it names (with the task), else leave it unticked with "(Not drawn, as decided: ...)" in the line itself.
- The floating tab menu's "Unchanged" row: `a_floating_editor_s_tab_menu_keeps_today_s_entries` checks the entries' order and that a window's lone tab has no Eject (`leaf.rs`: `path.surface.is_main() || !is_lonely_tab`); Eject where the tab is not alone is untested.

## Timing against U0
- U0's logs alone cannot judge M4: build U0's tree (ad6b28c8) from a `git archive` under the worktree's `target/u30/u0-tree` with its own CARGO_TARGET_DIR and run it interleaved with the tip (target/ui-mock/perf-m4/u30/seq.sh, summary.py's "U0now" column). On 2026-11-20 U0's tree ran 3 to 7 percent slower than its own logs (machine), while the editor views were far slower in M4's tree: that part is M4's own drawing (headers, the docks' panels for every editor, a blueprint's own Preview), each task having timed only its own cases against its parent.
- Which task: git archives of task commits built in turn into one shared target dir (extract with `tar -x -m`: the archive's old mtimes make cargo think nothing changed and copy the stale binary), each binary run alone and interleaved, two rounds (target/ui-mock/perf-m4/u30/bisect/). Found: forms +0.1 to 0.3 at U9, the creature +0.3 at U11; the huge script's scrolling 1.3 to 5.6 ms first at U12 and U13 (mostly the test: since U12's review editor_perf opens the script docked, U0 in a harness-sized window; U12's own code about +0.4 to 0.8 ms, U13's untested); the 15,100-row table 0.46 to 0.80 at U16; the Faction Editor at U22; Select Texture at U11, U25, U26.
- A view that is slower only in one round's mean (least near U0) is a disturbed round: rerun its test alone before blaming a task.
- frame_perf at FRAME_SCREEN=1280x800 must hold at 1280: under 1400 an editor's Right Dock sections fold into one tab bar (only the front panel is drawn) and short windows build only rows in sight. U20 to U23 had added asserts true only at the default width; U30 made them hold at both.

## Watch out
- A copied run.sh whose wait loop gives up after 30 minutes starts a run busy: make the wait loop wait on. Killing a parent script can leave an orphan child script running its own loop: check `ps` after.

## Corrected at U30's review (2026-11-21)
- A step a bisect finds may be the test's, not the task's: area084's wheel rows beside a dock panel (+1.5 to 2 ms, "U6") were frame_perf's camera. U6 added two views whose wheel frames zoom the orbit camera 18 percent farther out, and the later views (the Outliner, the tiles palette, the Right Dock) never set their own camera. Proof: U6's parent with the camera scaled 1.18 timed as U6. Before blaming a task for a view's wheel row, log `area_views[..].orbit` at each view's start in both trees.
- frame_perf now takes the camera with `resting_camera` (steps until the wheel's zoom momentum stops) before U6's views and gives it back with `rest_camera` after them. Taking it without waiting caught it mid-zoom (17.9 where it rests near 13) and made the rows 9 to 10 ms. Re-timed: 5.2 to 6.3 ms (the five once over) (target/ui-mock/perf-m4/u30/fix-round/).
- Under 1400, `tabs_folded(h, tabs, front)` asserts the folded tabs' names in order on the dock's top row and a label of the front panel's own under them. A label the panel shares with the editor is no proof: the faction matrix's cells (" toward ") have accesskit rects past the editor's edge, inside the dock's rect, drawn or not; Reaction's own "Reputation value" is used. Kittest `Node::rect()` panics on nodes without bounds: filter on `bounding_box().is_some()` in `query_all_by`.
