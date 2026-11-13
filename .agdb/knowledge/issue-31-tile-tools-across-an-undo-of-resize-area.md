---
type: Decision
title: "Issue 31: tile tools across an undo of Resize Area"
tags: []
status: stable
generated: { by: trail, at: 2026-10-26T00:00:00Z }
touches: [crates/mg-tiles/src/lib.rs, crates/mg-tiles/src/paint.rs, crates/mg-ui/src/area_view.rs, crates/mg-ui/src/terrain_mode.rs, crates/mg-ui/src/tile_select.rs, crates/mg-ui/tests/app.rs, docs/deferred.md, docs/log.md, bucket/moonglow.json, crates/mg-ui/tests/one_gate.rs]
created: 2026-10-26
author: August
description: "Issue 31's panic came from hovering after undoing Resize Area, when a stale view spot hit a smaller grid; the fix resets area-view state via ground_changed on workspace rewrites and makes Grid lookups bounds-safe."
asks:
  - "Why does the editor panic when I hover over tiles after undoing Resize Area, even without clicking?"
  - "When does AreaView::ground_changed fire, and which cached state like spot, drag, and tile selection does it clear?"
  - "Why did Grid::tile start panicking for out-of-range x, and what should I call for coordinates kept across frames?"
  - "How do I make Tile Properties not write to the wrong tile after Rotate Area or a Tile_List change?"
---
## Decided
- Issue 31's crash was not the click: in `area_view::viewport`, `terrain_mode::preview` (and `overlay`'s cursor) run before `input`, the only place `view.spot` is set. In the frame `refresh` reads a smaller ARE, the preview's cache misses on the revision and `click → stroke` gets last frame's spot (computed against the larger model) and the smaller `current_grid`. Hovering alone over the lost ground and undoing panics; no click is needed.
- The fix is on both sides. `refresh` calls `AreaView::ground_changed` when the tiles are laid out anew: the ARE's `mg_edit::Workspace::rewrites` moved (the resource replaced or read again, a root list set whole or an item put in or taken out: Resize Area, Rotate Area of any size, their Undo and Redo, a plugin setting `Tile_List`), the size changed, or another tileset (not a first read or Reload Resources). Spot, terrain drag, crosser drag, preview and cursor caches, menu tile, refused flash and the whole tile selection go.
- Tile Properties holds tiles by `Tile_List` index: it records `rewrites` as it opens and `tile_select::tiles_moved` closes it, writing nothing, at the top of its `ui` and of `closing_window` (Don't Save would write `base` back). It doesn't depend on the area view being drawn.
- Painting, Delete, paste and Tile Properties edit tiles by item path (`tile_edits`), so they never count as a rewrite and the selection survives them.
- mg-tiles kept `Lattice::corner`/`cell` and `Grid::tile` as indexing (they assert, with a message), and added `has_corner`, `has_cell`, `get_corner`, `get_cell`, `Grid::get_tile`. Every `Grid` brush (paint, raise, erase, erase_only, draw_crosser, repick, delete, place_tiles, set_exact) returns `None` for a coordinate outside, `locked` is false there, and `apply` ignores a stroke of another size. This keeps the 60-odd call sites and the plugin API (which validates its own coordinates) as they were.
- `terrain_mode::stroke` returns `(None, label)` for a spot outside the grid, as a second guard.

## Tried and ruled out
- Changing `corner`/`cell` to return `Option` everywhere: too much churn (reshape, corpus tests, the plugin, the UI tests) for coordinates that are already bounded by loops.
- Undo running mid-frame: actions (Undo included) run at the end of the frame (`run_actions`), and `refresh` comes first in `area_view::ui`, so the model and the workspace agree when `input` runs. The stale state is the previous frame's spot.

## Watch out
- `Grid::tile(x, y)` used to wrap silently for `x >= width` (it indexed `y * w + x`); it now panics with "cell (x, y) outside the grid". Use `get_tile` for anything kept across frames.
- `AreaView`'s `spot` and `terrain_drag` are `pub(crate)`: UI tests see the brush through `brush_cursor` (empty with no spot and no drag) and `tile_preview`.
- `resize_area_to` in `tests/app.rs` points the camera at the larger area (`Orbit::overview`): the harness's camera is set once from the 4 by 4 area, and the new rows may be off screen.
- A size check alone misses Rotate Area of a square area: the first version of the fix let a height set in Tile Properties after a turn land on the tile turned into its place. Watch the rewrite count, not the size.
- `Workspace::derive` is not counted (it makes palettes and compiled scripts); a new derive that touches AREs would need to be.
