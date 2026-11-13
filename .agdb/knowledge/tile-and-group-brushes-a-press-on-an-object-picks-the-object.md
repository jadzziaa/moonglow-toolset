---
type: Decision
title: "Tile and group brushes: a press on an object picks the object"
tags: []
status: stable
generated: { by: trail, at: 2026-11-12T00:00:00Z }
touches: [crates/mg-ui/src/area_view.rs, crates/mg-ui/src/terrain_mode.rs, crates/mg-ui/tests/app/hak_closing.rs, crates/mg-ui/tests/app/main.rs, crates/mg-ui/tests/app/tiles.rs, docs/log.md, docs/manual/04-areas.md, docs/manual/12-differences.md, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805/.mg-lock]
fingerprint: v2-dae18d92c296cfe1
created: 2026-11-12
author: August
description: "With a plain Gallery tile or a group brush (exact or not), a primary press on an object goes to the object, not tile placement; decided by `terrain_mode::objects_keep_their_presses`, extending the Exact Tile ruling."
asks:
  - "Why does clicking on a chest with a tile or group brush select it instead of placing a tile?"
  - "Which brushes still place on top of objects, and which now leave the press to the object?"
  - "What does the toolbar status line say when hovering an object with a group brush in hand?"
  - "How do I write brush tests that run the same case across all four brushes and report every failure?"
---
## Decided
- The owner's answers of 2026-11-12 widen the 2026-11-11 Exact Tile ruling: with a plain Gallery tile, a group, or a group with Exact Tile on in hand, a primary press on an object is the object's too. This supersedes the scope lines of `2026-11-12-exact-tile-a-press-on-an-august.md` (there: Exact Tile only, and its status text); that note's mechanism (decided at the press origin, held to the release frame, brush kept chosen) stands.
- One predicate, `terrain_mode::objects_keep_their_presses` (`Brush::Tile(_) | Brush::Group(_)`, exact or not), is asked by `object_press` (the press and the hover) and by the toolbar's status line in `area_view.rs`. Terrains, crossers, Raise/Lower, the Eraser, Refine Tile, the tile tools and Paint keep every press (pinned by `tiles::other_brushes_on_an_object_today`).
- Over an object the status line is "<brush>: [Exact Tile: ]a click or drag here goes to the object (select, move or turn it) and places nothing; Escape: stop." with no tile named after it.
- Ctrl + click on an object toggles it in the selection with any of these brushes (the owner: "Yes").
- Hover decision cost, release, 32 by 32 rural area, 3,000 chests: medians 0.13 ms, worst 0.43 ms over 1,000 places for each brush (`tile_edges::exact_tile_hover_decision_timing`).

## Tried and ruled out
- Choosing a second brush in the same harness by clicking its picture: when the picture is the one already chosen (the plain tile after Exact Tile's), the click lets the brush go. The timing test makes a harness per brush.
- Pinning the other brushes' changed tile indices: a variant's chance and where a drag's steps land decide them; the pin keeps whether the chest's tile is among them.

## Watch out
- The brush tests run each case for the four brushes through `each_hand`, which catches each brush's failure and fails once at the end naming them (pictures for all four even when one fails).
- A group's drag places nothing (its arm reads clicks only): a drag begun on ground with a group in hand changes nothing.
- An exact tile placed without a turn keeps a different orientation than a turned one; assert that a right click changed the orientation, not its value.
