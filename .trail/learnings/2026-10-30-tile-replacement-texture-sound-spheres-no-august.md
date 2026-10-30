---
title: "Tile replacement texture, sound spheres, no-model marks, c_wings2's pose"
author: August
date: 2026-10-30
branch: worktree-agent-aefb3598f87cf0e12
cost: { minutes: 150, tokens: 4854243 }
touches:
  - crates/mg-ui/src/tile_select.rs
  - crates/mg-ui/tests/app/tiles.rs
  - crates/mg-rules/src/appearance.rs
  - crates/mg-rules/src/lib.rs
  - crates/mg-ui/src/appearance_gallery.rs
  - crates/mg-ui/src/blueprint/creature.rs
  - crates/mg-ui/src/blueprint/door.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/blueprint/placeable.rs
  - crates/mg-ui/src/blueprint/waypoint.rs
---

## Decided
- Tile Properties' Replacement Texture is the 8th slot of the draft (`tile_select::FIELDS`, `REPLACE`): value = row + 1 (`mg_rules::rows::replacement_value`), an absent field shows row 0; Defaults leaves it (`touched[..REPLACE]`). Enabled by `TileFeatures::replace` (`mg_mdl::Model::draws_texture("replace_tex")`). Verify's `tile-replace` (Objects group, an error) reads only tiles that have the field.
- Every dropdown in mg-ui must be `widgets::row_choice`/`dropdown`: `tests/one_gate.rs` fails on any `ComboBox` outside widgets.rs (only the full `--tests` run catches it, not `--test app`).
- Appearance lists: `GameData::appearance_choices` marks rows "(no model)", its own cache `marked` cleared with `invalidate`. Palette Gallery: `Made::no_model`, set only when the picture's look is None.

## Tried and ruled out
- c_wings2 on a human: dropping `compose.rs`'s `["creadyl","cpause1","pause1"]` fallback for animated parts (the wing at its bind pose, spread) did not plainly match the client from another camera; reverted, left to the client batch with `moonglow_beside`. c_wingsdm has none of pause1, creadyl, cpause1.
- tilefade 4 in the game's tiles: 1,877 meshes, 1,744 of them black caps (as the tilefade 2 caps Aurora keeps).

## Watch out
- A "before" picture after the code changed: `git diff -- src > x.diff; git checkout -- src`, build, render, `git apply x.diff` (never git stash: it is shared).
- Palette gallery tests: big tiles hide most blueprints; set `settings.gallery_tile = Some(64)` and filter by resref.
