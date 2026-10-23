---
title: "Exact tiles, tile height and all tiles (issue #29)"
author: August
date: 2026-10-23
branch: exact-tiles
cost: { minutes: 213, tokens: 1624604 }
touches:
  - crates/mg-area/src/edit.rs
  - crates/mg-corpus-tests/tests/engine_terrain.rs
  - crates/mg-tiles/src/paint.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/src/terrain_mode.rs
  - crates/mg-ui/src/tile_select.rs
  - crates/mg-ui/src/tile_view.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/screens.rs
---

## Decided
- Issue #29 (archvillainette's "tile magic"), three explicit controls; the fit-only brushes and steps are unchanged with them off.
- Exact tile: `Settings::exact_tiles` (remembered, off by default), a `toggle_value` "📌 Exact tile" beside the tiles palette's ☰/⊞, shown with the Gallery or a `Brush::Tile` chosen. `TileBrush::exact` is filled in by `terrain_mode::active` (and `drop_tile`) from the setting, so it is part of `PreviewKey` and the preview/cursor recompute when toggled. `stroke()`'s `Brush::Tile(t) if brush.exact` arm calls `Grid::set_exact` (mg-tiles): a fixed-only Stroke at the replaced tile's own height; refused (blocked = the group's cells) on a locked group tile. Click, drag (`paint_corners`), preview, cursor and drop all go through `stroke()`.
- Tile Properties Height: `TileProps::height: Option<i32>` (None = mixed), `opened_height` in `changed()` (Escape asks), `-`/`+` buttons named "Lower Height"/"Raise Height" (widget_info) plus a DragValue (custom_formatter "Mixed"). Range ±1024 = mg-area's reading clamp. Locked when a selected tile is a group's and its whole group isn't selected. Applied on OK via `mg_area::edit::tile_height_edits`: only Tile_Height, and objects whose XY is over the tile moved by delta × step with `move_edits`.
- All tiles (ignore fit): `TileProps::all_tiles` (not remembered: off each opening), `turn`, `every`; pictures via `ScrollArea::show_rows` (only rows in sight, so only those thumbnails are asked for). A click: `set_exact` per selected cell at the Height, `terrain_mode::tile_command_with` with the GIT moves as `extra` edits placed FIRST in the command (door_edits' RemoveItem indices are computed against the GIT as it is).
- Engine: a tile's own Tile_Height from -3 to 1000 stands where it says, walkmesh included (`engine_terrain.rs a_tile_s_own_height_stands_where_the_engine_puts_it`). nwn.wiki has nothing on it.

## Watch out
- `▼`/`▲` aren't in the UI fonts (drawn as boxes): plain `-`/`+`.
- In kittest, a DragValue `labelled_by` a field label makes `get_by_label` find two nodes; name buttons with `WidgetInfo` instead.
- `area_shots` in app.rs (orbit set to south-west, put back) gives the angled pictures; the top-down view hides a raised tile's gaps.
- The worktree guard refuses Bash commands whose text contains "GIT"/"git" in odd places (even in a perl replacement string): use Edit.
