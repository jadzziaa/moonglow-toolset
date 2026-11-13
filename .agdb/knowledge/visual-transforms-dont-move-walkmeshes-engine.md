---
type: Decision
title: "Visual transforms don't move walkmeshes (engine)"
tags: []
status: stable
generated: { by: trail, at: 2026-11-03T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/engine_placeable_walkmesh.rs, crates/mg-area/src/walk.rs, docs/manual/04-areas.md, docs/findings.md]
fingerprint: v2-5d3493b7d55c3fff
created: 2026-11-03
author: August
description: "In the engine, visual transforms (scale, rotation, translation, including SetObjectVisualTransform) never move or resize .pwk/.dwk walkmeshes, which stay at the object's position and Bearing; also notes test pitfalls with random tiles and locked doors."
asks:
  - "Does scaling or rotating a placeable with a visual transform also change where its walkmesh (.pwk or .dwk) sits?"
  - "My walkmesh doesn't line up with a placeable I scaled or translated visually; is that a bug in ObjectWalkmeshes::faces?"
  - "How do I test whether a door or placeable walkmesh blocks a walker without random area tiles skewing results?"
  - "Why did walkers turn aside by metres in my walkmesh test on add_area ttr01 tiles, and how do I get clean results?"
---
## Decided
- Engine (walkmeshes_by_visual_transforms in engine_placeable_walkmesh.rs): no part of a visual transform moves a .pwk or .dwk: Scale 3 (dynamic, static, or set by SetObjectVisualTransform), X Rotation 90, Y Rotation 90, TranslateX 4 all leave the walkmesh at the object's position and Bearing, at its size; GetObjectVisualTransform reads each. ObjectWalkmeshes::faces was right (o.transform(), no VT); no code changed.
## Tried and ruled out
- Judging lanes by max distance aside on add_area's random ttr01 tiles: the tiles' rocks and trees turned walkers 1.7 to 4.3 m aside where nothing of ours was, and moved with the area's width. open_ground() sets every tile to one walkable all over (ttr01_o02_01); then straight lanes read 0.00 m and turned ones 2.15 to 2.17.
## Watch out
- A path goes through a locked door (doors are opened): the walker stops at its face, it doesn't go round. Test a door by whether a walker passes where it would be.
- Prefer 'walks over the would-be footprint' (segment crosses the rectangle) to 'aside > x': crossing proves absence, avoiding can be chance.
