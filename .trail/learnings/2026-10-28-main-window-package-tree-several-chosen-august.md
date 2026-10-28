---
title: "Main window package: tree several chosen, Filter in every area, Escape as Cancel, log copy, tab MRU per window"
author: August
date: 2026-10-28
branch: worktree-agent-ad23cca8a38cff3a3
cost: { minutes: 1420, tokens: 12231800 }
touches:
  - crates/mg-ui/tests/app/area_view.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/03-modules.md
  - docs/manual/04-areas.md
  - docs/manual/10-options.md
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/tabs.rs
---

## Decided
- Module tree several chosen: `Moonglow::tree_selected`/`tree_anchor` (resources) and `tree_objects`/`tree_objects_anchor` (one area's placed objects, `tree::Placed`), through `widgets::select_in_list`; a click is resolved after every row is laid out (`picked`, by `key_rows(&shown, &outs)`), as `tree_to_match` is. Marked only when there are several. A right click on one of several: `several_menu` (resources) or `ContentKeys.together` (objects, which `go` turns into `targets`). The area view's selection of several is mirrored into `tree_objects` only when it changes (`tree_seen_selection`), so a tree-only choice in another area survives.
- A tab of several (`Tab::tree_keys`: Blueprints, AreasProperties) sets `tree_selected` to its keys; Module Properties marks `tree::module_row()` (module.ifo), which `listed()` would refuse.
- A tab chosen while the Filter hides its row: `tree_kept` = (its keys, the Filter's text), listed under the Filter until the text changes. Keeping `tree_marked` always under the Filter broke `areas_are_listed_by_name_when_asked` (the area opened with the module stayed in every filtered list).
- The Filter reads every area's contents (`area_contents` reads a GIT not open without loading it; `tree::Read` = the doc's revision if loaded, else the stored bytes' pointer and length), 12 ms of reading a frame.
- Escape in a Properties window's field is Cancel; search fields register themselves each frame (`search_field_shown`) and `dialogs_end` notes whether the focused id is one (`escape_left_search`). The hand-rolled Find fields (picker, creature lists, store, Options › Keyboard) are `SearchField`s now.
- Ctrl+Tab by recent: `note_recent_tabs` notes every leaf's front, skipping a leaf whose first tab in `tabs_recent` is already its front (no ping-pong).

## Tried and ruled out
- `h.event(PointerButton { modifiers })` alone for a Ctrl+click: the tree reads `i.modifiers`, which only `ModifiersChanged` sets. In UI tests use `Node::click_modifiers`; in screens.rs send `ModifiersChanged(m)` and reset it two steps later.

## Watch out
- Docked windows (a script's, Properties) open over the module tree's Filter in a 1100-1200 point harness: a click on the Filter lands in the window and moves `tree_marked` to its tab. Close them first, and find the Filter by its hint (`type_into_hint`) or by its value, not as the first TextInput.
- A kind's list opened by the Filter stays open (egui keeps the header's state): clicking "Waypoints (2)" afterwards closes it.
- `area_harness("tree-follows")` was used by two tests: `scratch_dir` empties the folder, so they failed now and then under load. Names must be unique per binary.
- Kittest's held Ctrl over several presses: queue `ModifiersChanged(COMMAND)`, raw `Event::Key` down/up with `modifiers: COMMAND`, then `ModifiersChanged(NONE)`; `key_press_modifiers` resets the modifiers in the same frame (the cycle ends).
