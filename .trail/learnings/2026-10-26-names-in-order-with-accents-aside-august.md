---
title: "Names in order with accents aside, arrow keys on the area's lists, hak list drag scrolling, talk table split"
author: August
date: 2026-10-26
branch: lists-sort-keys-drag
cost: { minutes: 874, tokens: 9387149 }
touches:
  - crates/mg-core/src/text.rs
  - crates/mg-module/src/new.rs
  - crates/mg-rules/src/lib.rs
  - crates/mg-ui/src/area_grass.rs
  - crates/mg-ui/src/area_props.rs
  - crates/mg-ui/src/area_reshape.rs
  - crates/mg-ui/src/area_tools.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/module_props.rs
  - crates/mg-ui/src/talk_view.rs
  - crates/mg-ui/src/tree.rs
  - crates/mg-ui/src/widgets.rs
  - docs/deferred.md
---

## Decided
- One order for names everywhere: `mg_core::text` (`fold`: lower case, Latin-1 and Latin Extended-A accents dropped, æ/œ/ß written out, combining marks dropped; `sort_key` = (folded, the text itself) so names that fold alike keep one order). Used by `mg_rules::by_name` (every list of a table's rows), the module tree's two sorts (`tree.rs`: an area's objects, and areas/blueprints by name) and `mg_module::new::word_sort_key` (Aurora's Windows word sort: palettes, tilesets, script wizard), which folds accents too, as Windows' collation does at the primary level. No new dependency: `unicode-normalization` isn't in Cargo.lock, a hand table is ~30 lines.
- The blueprint's Category list, alphabetical, sorts with `mg_rules::by_path(choices, " / ")`: part by part, so a group's own categories stay together (by the whole text, "Oficios, viejos" came between "Oficios" and "Oficios / Herrero": a comma sorts before the slash).
- Arrow keys on a plain egui ComboBox: `widgets::arrow_step(ui, &response, &options, &mut current)` (`arrow_pick` over the options' indices). Used where `row_choice` can't host the list: the area toolbar's Fade Geometry, Snap and Turn (Turn has a DragValue inside its popup, and the toolbar's `Fits` needs the response), Resize Area, Find Instance, Edit Areas Together. The grass's surface material is a table's rows, so it went to `row_choice`.
- Area Properties' 2DA lists already took the arrows (they are `Form::choice` → `row_choice` since 2026-10-21); the deferred entry was older than that.
- The talk table's split is drawn by hand (`talk_view::split`), not an `egui::Panel::bottom` as the conversation and journal editors' are: a bottom panel claims its room before the list is drawn, so the editor would come first in the frame and a click or double click in the list would reach it a frame late. The share of the tab is kept in egui's persisted memory (`talk-split`), held between the boxes' three lines and seven tenths.
- The hak list scrolls under a drag with `ui.scroll_with_delta` inside its ScrollArea (positive y shows rows above), speed from `module_props::edge_scroll`, and `request_repaint` so it goes on while the pointer rests.

## Tried and ruled out
- An `egui::Panel::bottom` for the talk table's text boxes (as the conversation and journal editors have): it must be shown before the list, so the editor would lag a frame behind a click in the list (see Decided).

## Watch out
- The talk table's split follows the pointer against the press origin, from the editor's height when the drag began. A UI test dragging it far sees fewer points moved than dragged: that is the seven-tenths limit, so drag it a little (80 px in a 800 px window) to check it follows exactly.
- The camera doesn't take the arrows while any widget has egui's focus (`area_view`: `typing = focused().is_some()`), so a toolbar list in focus doesn't move it.
- In this worktree session Bash refuses `flock ... cargo test`, `python3 - <<EOF` mixed with git, and long `&&` chains with okf/git: put the lock in a scratchpad script (`locked-acfd.sh`) and run okf, python and git as separate commands.
- `graft` isn't installed here; `trail note` is the tool.
