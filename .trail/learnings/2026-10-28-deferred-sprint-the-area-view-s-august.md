---
title: "Deferred sprint: the area view's tile tools"
author: August
date: 2026-10-28
branch: worktree-agent-a666cb548e73ba8c9
cost: { minutes: 1019, tokens: 9409520 }
touches:
  - docs/deferred.md
  - docs/log.md
  - docs/manual/04-areas.md
  - crates/mg-module/src/tileset.rs
  - crates/mg-set/src/edit.rs
  - crates/mg-ui/src/side_doc.rs
  - crates/mg-ui/src/tileset_view.rs
  - crates/mg-ui/tests/app/tiles.rs
  - crates/mg-ui/src/tile_select.rs
  - crates/mg-ui/src/area_view.rs
---

## Decided
- Tile tools on a selection go through `terrain_mode::OnSelection` / `act_on_selection` (Step = the tile menu's steps, Refine = `refine_cells`, the drag's semantics over the selection in its order, Erase = `tile_select::delete`). The palette's `brush_row` records `Place::stepped` and the keys (`terrain_mode::tool_keys`, called in `area_view::input` before `terrain_mode::input`, not while `letters_taken`) call the same.
- Tile tool keys (Area group): X Eraser, T Refine Tile, N / Shift+N Next / Previous Tile Appearance, R / Shift+R Rotate Tile and clockwise. Keys match exactly, so Shift+R is its own command. Without a selection a key chooses the brush as a palette click does (again: put down); the Eraser's label is the tileset palette's own (read in `tool_label` if the palette was never shown).
- Tile Properties re-reads its tiles when they change under it (`tile_select::reread`, top of `ui`): `seen`/`seen_at` (structs and revision as last read or written), `wrote` (every value written, merged across re-reads, for Cancel), `own` (tiles after its own step: a redo of it makes `live` its own again). Undo detection: `peek_redo` label "Tile Properties".
- The tile selection outline is `AreaView::tile_outline`: 20 steps per edge on the ground, but only 4 corners when corners and middle are level (big selections of flat tiles cost what they did).
- Renaming a terrain/crosser: `mg_set::edit::rename_type` (tiles' corners or edges, GENERAL Border/Default/Floor and rule Placed/Adjacent/Changed for terrains, not TILEnDOORm). The ITP names terrains by lower-case RESREF; `TilesetDoc::save` compares names by index with the file on disk and applies `mg_module::tileset::rename_in_palette` (Terrain category only) to the palette beside.

## Tried and ruled out
- "A right drag back to where it began is a right click to the brush": not so. egui's `has_moved_too_much_for_a_click` is sticky for the press, and `Held::read` starts from `response.dragged()`, which egui sets only past the click distance; held by `a_right_drag_back_to_where_it_began_is_not_a_right_click`.

## Watch out
- A truncated egui Label's text is the accesskit node's `value()`, not `label()`; `get_by_label` still finds it.
- A SearchField (or any row) as wide as `available_width()` in a dock window grows the window's content: give it the visible width (`clip_rect().right() - cursor().left()`), as the galleries do, or pictures run past the edge.
- `tile_props.every` is every tile; the pictures shown are the search's `found`.
