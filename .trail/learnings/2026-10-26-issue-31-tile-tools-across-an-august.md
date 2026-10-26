---
title: "Issue 31: tile tools across an undo of Resize Area"
author: August
date: 2026-10-26
branch: issue-31-terrain-after-undo
cost: { minutes: 1027, tokens: 10079625 }
touches:
  - crates/mg-tiles/src/lib.rs
  - crates/mg-tiles/src/paint.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/terrain_mode.rs
  - crates/mg-ui/src/tile_select.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/log.md
  - bucket/moonglow.json
  - crates/mg-ui/tests/one_gate.rs
---

## Decided
- Issue 31's crash was not the click: in `area_view::viewport`, `terrain_mode::preview` (and `overlay`'s cursor) run before `input`, the only place `view.spot` is set. In the frame `refresh` reads a smaller ARE, the preview's cache misses on the revision and `click → stroke` gets last frame's spot (computed against the larger model) and the smaller `current_grid`. Hovering alone over the lost ground and undoing panics; no click is needed.
- The fix is on both sides. `refresh` calls `AreaView::ground_changed` when the model's size or the tileset changes: spot, terrain drag, crosser drag, preview and cursor caches, menu tile and refused flash go; the tile selection keeps what is inside; Tile Properties on that area closes (it holds `Tile_List` indices).
- mg-tiles kept `Lattice::corner`/`cell` and `Grid::tile` as indexing (they assert, with a message), and added `has_corner`, `has_cell`, `get_corner`, `get_cell`, `Grid::get_tile`. Every `Grid` brush (paint, raise, erase, erase_only, draw_crosser, repick, delete, place_tiles, set_exact) returns `None` for a coordinate outside, `locked` is false there, and `apply` ignores a stroke of another size. This keeps the 60-odd call sites and the plugin API (which validates its own coordinates) as they were.
- `terrain_mode::stroke` returns `(None, label)` for a spot outside the grid, as a second guard.

## Tried and ruled out
- Changing `corner`/`cell` to return `Option` everywhere: too much churn (reshape, corpus tests, the plugin, the UI tests) for coordinates that are already bounded by loops.
- Undo running mid-frame: actions (Undo included) run at the end of the frame (`run_actions`), and `refresh` comes first in `area_view::ui`, so the model and the workspace agree when `input` runs. The stale state is the previous frame's spot.

## Watch out
- `Grid::tile(x, y)` used to wrap silently for `x >= width` (it indexed `y * w + x`); it now panics with "cell (x, y) outside the grid". Use `get_tile` for anything kept across frames.
- `AreaView`'s `spot` and `terrain_drag` are `pub(crate)`: UI tests see the brush through `brush_cursor` (empty with no spot and no drag) and `tile_preview`.
- `resize_area_to` in `tests/app.rs` points the camera at the larger area (`Orbit::overview`): the harness's camera is set once from the 4 by 4 area, and the new rows may be off screen.
- Rotate Area of a square area keeps the size, so nothing is let go: the selection and Tile Properties then name other tiles (recorded in docs/deferred.md).
