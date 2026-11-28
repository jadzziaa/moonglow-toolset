---
type: Decision
title: "One tile set anew: set_tiles, update_tiles and the ground's"
description: "How a client follows a server's SetTile without rebuilding the area (AreaModel::set_tiles, AreaScene::update_tiles, Ground::update_tiles), what each keeps, and why the toolset's area view still uses update."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-28T07:35:56Z }
touches: ["crates/mg-area/src/lib.rs:AreaModel::set_tiles", "crates/mg-area/src/scene.rs:AreaScene::update_tiles", "crates/mg-area/src/walk.rs:Ground::update_tiles", crates/mg-corpus-tests/tests/tile_updates.rs]
asks:
  - how does a client change one tile without rebuilding AreaScene
  - does update_tiles need the tileset or reload lights
  - "why doesn't the area view use update_tiles after a tile edit"
  - "how was update_tiles proved equal to a fresh AreaScene::new"
fingerprint: v2-f6fc487bb215440a
created: 2026-11-28
author: augustaugur
---
- A client changes tiles with `AreaModel::set_tiles(game, Some(&tileset), &[(i, values::Tile)])`. It needs the `Tileset` the area was built with, so a client has to keep it (mc-scene dropped it after `Stage::new`). The call returns the indexes that really changed, and the caller passes those to `AreaScene::update_tiles` and `Ground::update_tiles`. It shares `settle_tile`, `tile_problems` and `tiles_border` with `from_values`, so the result equals `from_values` on the changed values, problems included. The tiles' problems are spliced in where `from_values` puts them: after the "Tile_List has" message and before the tileset's.
- `update_tiles` loads only a model no tile had (`tile_model`, keyed by name in `tile_cache`). It grows grass for `grass_for` (what is grown now, which may be settings being tried out), not for `area.grass`. It re-zips the border from the cache and calls `settle_tiles`, the tile half of `settle`: it lets go of models no tile uses and keeps the objects' look clock. Far grass chunks need nothing, because `ChunkGrass::is_of` compares each tile's grass and transform. Lights need no update: `scene_hiding` makes them per frame. If the tile count differs, it falls back to `update`.
- Tests: `tile_updates.rs` (corpus, areas group) builds the fresh model, scene and ground from the values along an independent path, never from the `set_tiles` model. It compares instances, lights, grass, Ground faces, levels, heights and hits, and a 480x360 picture to the pixel.
- Cost on a 32x32 ttr01 area with no objects (release): `update_tiles` 0.64 ms, `update` 1.02 ms, a full rebuild 16.8 ms. The area view reads its model whole after an ARE edit, so switching it to `update_tiles` would save about 0.4 ms. It was left on `update`, which also follows the sky, grass and objects.
