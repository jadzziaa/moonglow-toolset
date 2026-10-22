---
title: "Tile groups protected and Tile Properties from the model (Jasperre's feedback)"
author: August
date: 2026-10-22
branch: tile-groups-and-properties
cost: { minutes: 13, tokens: 28912 }
touches:
  - crates/mg-area/src/lib.rs
  - crates/mg-corpus-tests/tests/aurora_terrain.rs
  - crates/mg-plugin/src/terrain.rs
  - crates/mg-tiles/src/paint.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/options.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/src/tabs.rs
  - crates/mg-ui/src/terrain_mode.rs
---

## Decided
- Tile Properties' lights/loops come from the tile MODEL, not the SET: every tile of the game's sets has MainLight1..AnimLoop3=1. `mg_area::TileFeatures::of(model, resref)`: main light = Light node named `<model>ml1/ml2` (model's own name or the resref, case aside), source = any node `<model>sl1/sl2`, loop = animation `animloop01..03`. Model not found → SET flags. Aurora (seen on the oracle, tin01 tiles 18/0/134/45): always 7 controls, missing ones greyed, a greyed swatch blank and a greyed loop unchecked even if the file has 1; several tiles = union. `apply` writes each tile only its own `has_each` (lossless).
- Close prompt: `TileProps::changed()` (only enabled controls vs `opened`), `may_close` sets `closing` and returns false; `tile_select::closing_window` (called beside `side_doc::closing_window`) asks Save/Don't Save/Cancel. Escape in a floating tab goes through `widgets::escape_here` in the window itself (its Cancel button no longer takes Escape: `cancel_discard` would have discarded); tabs.rs's Escape pushes `CloseTabs` (through may_close) for TileProperties, `CloseTab` (unconditional) for the rest. Aurora's X discards silently.
- Group membership is derived, never stored: `TileIndex::is_grouped` (tile id in any .set group) + `group_at` (whole where every slot matches, same turn and height). `Grid::free_groups` (Options › Area "Allow editing tiles of groups and features", `settings.edit_group_tiles`, set in `terrain_mode::current_grid`, part of PreviewKey) gates everything: `Grid::locked(x,y)`; place_tiles refuses over any group tile (blocked = the covered groups' cells) unless free or `replace_groups`; stroke_unlocking re-picks group tiles that no longer fit only when free; delete expands to whole groups unless free; turn_fit turns a group tile where its turned cell still matches. `Grid::groups_broken` drives the ⚠ warning in `tile_command_keeping` (now `&mut AreaView`).
- The plugin API keeps its documented place_group replacing: its grid sets `replace_groups = true` (mg-plugin outside.rs `an_area_is_made_and_painted` asserts it).
- Select Tiles had no status line for notices; added a tile_mode branch to the area status (refusals were silent there before).

## Tried and ruled out
- First tests clicked palette tool rows while the Groups branch was expanded: the rows were scrolled out of view, `get_by_label().click()` hit nothing and the Barn brush stayed (looked like the tools did nothing). Collapse Groups first, or scroll_to_me.

## Watch out
- `AreaView::notice` is private to tests: assert the status line text with `query_by_label_contains` (it is shown only with a brush or in Select Tiles).
- Disabled swatches need their WidgetInfo `enabled` passed explicitly (`named(r, name, enabled)`): `allocate_exact_size` with Sense::hover still reports enabled.
