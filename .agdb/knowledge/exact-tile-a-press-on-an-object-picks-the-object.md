---
type: Decision
title: "Exact Tile: a press on an object picks the object"
tags: []
status: stable
generated: { by: trail, at: 2026-11-12T00:00:00Z }
touches: [crates/mg-ui/src/area_view.rs, crates/mg-ui/src/terrain_mode.rs, crates/mg-ui/tests/app/hak_closing.rs, crates/mg-ui/tests/app/main.rs, crates/mg-ui/tests/app/tiles.rs, docs/log.md, docs/manual/04-areas.md, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1098591-f068852f/.mg-lock]
fingerprint: v2-95d8a86b6649cf10
created: 2026-11-12
author: August
description: "With the Exact Tile brush chosen, a press on an object now selects/moves the object instead of painting, decided by terrain_mode::object_press and object_takes_press; the gap-drop ground_at behavior is kept."
asks:
  - "Why does clicking a chest with the Exact Tile brush select it instead of painting a tile underneath?"
  - "How does terrain_mode::object_press decide whether a press goes to the object or to the brush?"
  - "I can't paint a tile where an object stands in Exact Tile mode; is that intended, and how do I paint there?"
  - "Does the plain tile brush or an exact group also pick objects on press, or only the Exact Tile brush?"
---
## Decided
- The owner's rulings of 2026-11-12 on the tile-edge report: (1) beside a raised tile with no cliff face, an object let go over the open gap goes to what is seen through the gap (`AreaView::ground_at`): kept as it is; (2) with the Exact Tile brush chosen, a press on an object picks the object. This supersedes two lines of `2026-11-12-tile-edge-drag-crash-report-raised-august.md`: its "whether that is wanted is undecided" (now ruled: kept) and its "Exact Tile brush still chosen: a press-drag on a placeable paints the tile" (now the press picks the placeable).
- The decision is `terrain_mode::object_press`, called from `terrain_mode::input` after Escape and before the `camera_press` check: on the primary press frame it sets `AreaView::object_press` from `area_view::object_takes_press` at the press origin, holds it through the release frame (clicked and the drag's commit are read then) and clears it when the button is up. While it holds, `input` returns false and the press goes down the object path (selection, Drag::Move, handles). The brush stays chosen and `terrain_mode::active` unchanged, so `view.tool` doesn't change and `let_go` doesn't drop the drag.
- `object_takes_press`: a selection handle (turn ring without Shift; tilt ring, axis arrow, scale handle with Shift; a spawn point's arrow or foot), an object `AreaView::pick` picks (shown, not locked) that `seen_before_ground` says is nearer the eye than the walkmesh the ray meets (as `put_down_at` weighs them), or the start location's marker.
- Only `brush.exact && Brush::Tile(_)`: the plain tile brush, an exact group and every other brush still take every press (pinned by `tiles::other_brushes_on_an_object_today`, a question for the owner).
- Hover: `AreaView::over_object` (pub) clears `view.spot` so neither preview nor cursor promise a tile, and the status line reads "Exact Tile: click to select the object under the pointer, drag to move it".
- Ctrl or Alt + press on an object goes to the object path too: a drag is the camera's there (the object path makes no drag of a camera press), a Ctrl + click toggles the object in the selection, as in object mode.

## Tried and ruled out
- Gating on `pick` alone: it weighs objects only, so a chest behind a raised tile was picked through the tile; the brush's decision adds the ground comparison. Object mode's `pick` was left as it is.
- Raising the tiles in front six steps for the "behind a tile" test: the ray passed under their ground (a raised tile has no cliff face, its walkmesh is only its top). One step, seen low, blocks.

## Watch out
- In the tile-edge tests a chest painted over by the wall end stands inside the wall's raised ground: a press there paints (the tile is seen in front). Use another chest for a later drag.
- `tiles::exact_tile_*` and `tile_edges::edge_drag_with_the_exact_tile_brush_left` hold the ruling; the status text is read by `status_line` in tiles.rs.
