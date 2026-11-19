---
type: Decision
title: "An area's model from values; the scene's objects followed without a rebuild"
description: "Why AreaModel::read goes through mg_area::values and from_values, where ranges are settled, how the reading was proved unchanged, and how a client moves, adds, changes and removes objects in AreaScene without rebuilding it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T12:55:26Z }
touches: [crates/mg-area/src/values.rs, "crates/mg-area/src/lib.rs:AreaModel::from_values", "crates/mg-area/src/scene.rs:AreaScene::update_objects", crates/mg-area/tests/scene_updates.rs]
asks:
  - how does a game client build an area without GFF fields
  - "does moving an object need AreaScene::update_objects"
  - what happens when update_objects sees a different number of objects
  - "how was AreaModel::read checked unchanged after the values refactor"
fingerprint: v2-7b36ce5174dc7257
created: 2026-11-19
author: augustaugur
---
- `AreaModel::read` = `values::Area::read` (fields to values; a missing field decides its default there) + `AreaModel::from_values` (out-of-range settled there: size at most 1024, orientation modulo 4, height within 1024 steps, directions normalized once in `Lighting::settled`, shadow opacity at most 100, the environment map from the tileset). Floats go from GFF to value exactly: an object's value is its rotation (the `Bearing` itself), not its facing, since a facing less 90 degrees and back changes bits; `Object::facing` makes one from a facing. Objects get their `index` by their order within their kind.
- Proof the refactor kept what `read` gives: `tests/area_values.rs` with `AREA_DUMP` hashes every area's Debug; with `MOONGLOW_FULL_CORPUS=1` before and after the change all 1462 areas were identical (2026-11-19). The test itself compares `read` and `from_values(Area::read)` as Debug strings (a NaN position compares equal so).
- Scene: `AreaScene::scene` reads `AreaObject::model_transform` each time it is made, so a pure move (`AreaObject::move_to`) needs no update. `update_objects(at)` shows `at` anew plus the objects past those shown (appended); fewer objects than shown still falls back to `update` (it cannot tell which went): use `remove_object(area, i)` after `area.objects.remove(i)`. The scene pairs objects with what it shows by place, so a removal from the model alone draws the neighbors' looks until the next `update_objects`. Adding fell back to `update` before (1.17 ms on Tyrants of the Moonsea's zhentkeepwar); now 0.015 ms. `update_objects` scans only the objects at hand for chunk emitters.
- mg-area's feature `edit` (default) gates mg-edit and mg-module; placement, `LOCKED`, grass and `lights::is_generated` moved to mg-rules, the door hooks to `mg_tiles::door_hooks`, each re-exported at its old path.
