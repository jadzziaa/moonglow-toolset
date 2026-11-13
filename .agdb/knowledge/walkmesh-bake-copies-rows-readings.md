---
type: Decision
title: "Walkmesh bake: copies, rows, readings"
tags: []
status: stable
generated: { by: trail, at: 2026-11-03T00:00:00Z }
touches: [crates/mg-module/src/bake.rs, crates/mg-ui/src/bake_view.rs, crates/mg-mdl/src/walkmesh.rs, crates/mg-corpus-tests/tests/engine_placeable_walkmesh.rs, crates/mg-corpus-tests/tests/client_render/readings.rs]
fingerprint: v2-c207ec68caf0ae13
created: 2026-11-03
author: August
description: "Baked walkmesh copies are renamed models keeping the original's part numbers so the client draws them identically; placeables.2da rows must come from a hak or override, not the module, and rows past 65535 wrap."
asks:
  - "How do I make a baked walkmesh copy render the same as the original placeable in the client?"
  - "Why does a placeables.2da that exists only in my module get ignored when the game loads baked rows?"
  - "My baked object doesn't appear in the view before the build runs; how do pending rows get shown?"
  - "Why does Rows::parse reject placeables.2da rows above 65535, and how do I find those placeables anyway?"
---
## Decided
- A baked copy is Model::renamed (name, root node, animroot and anim nodes of the old name) written in file_order of the original's PartNumbers, compiled against its supermodel: part numbers match the original (asserted in reading_baked_copy_drawn_as_the_original); the client draws it as the original and a supermodel's open plays on the copy.
- Engine (baked_walkmeshes_block_as_drawn): a placeables.2da only in the module is not read; rows reach the game via a hak or override. mg_module::bake::table_under skips the MODULE layer and the bake:rows layer.
- Rows file keeps the settled four keys; reuse is found by comparing the baked .pwk's corners (1 mm + reach x 0.1 deg) with the hak's copy.
- Pending rows are a resman layer (bake:rows, priority HAK_USER+1) laid in Module::stack_onto, sync_haks and sync_bake_rows, so the view draws a baked object before the build runs.
## Tried and ruled out
- plc_kelp33 (the game's only placeable with a supermodel, plc_kelp31 default) does not sway in the client, original or copy: use an armoire without own animations whose supermodel is plc_a01 and play open.
## Watch out
- placeables.2da rows past 65535 wrap (16-bit): Rows::parse refuses them; appearance-range finds placeables past it.
- cargo test stops at the first failing binary: use --no-fail-fast for the whole suite.
