---
type: Decision
title: "Armor torso 0, head 0 and upright particles measured"
tags: []
status: stable
generated: { by: trail, at: 2026-10-26T00:00:00Z }
touches: [docs/deferred.md, docs/findings.md, docs/log.md, docs/manual/05-blueprints.md, crates/mg-corpus-tests/tests/client_render.rs, crates/mg-corpus-tests/tests/client_render/upright.rs, crates/mg-render/src/particles.rs, crates/mg-corpus-tests/tests/client_render/heads.rs, crates/mg-preview/src/creature.rs, crates/mg-preview/tests/previews.rs]
fingerprint: v2-04d4633e6253f316
created: 2026-10-26
author: August
description: "Measured engine/client findings: blank or out-of-range torso ACBONUS means AC 0 (GameData::armor_class returns u32), head 0 equals no head, Aligned_to_World_Z particles face the eye; plus camera and test pitfalls."
asks:
  - "What armor class, weight and cost does the engine give a torso whose parts_chest.2da ACBONUS is blank or row 0?"
  - "Why does my character show no head with Appearance_Head 0 or missing, and how does mg-preview treat it?"
  - "My SetCameraFacing in OnClientEnter has no effect in the client test; how do I make the camera turn?"
  - "Do Aligned_to_World_Z particles face the viewer from every side, or do they need a renderer change?"
---
## Decided
- An armor's torso with a blank parts_chest.2da ACBONUS (row 0, row 2) or a row past the table is armor class 0 in the engine: GetItemACValue 0, GetWeight 10, GetGoldPieceValue 1, Cost 1 (armor.2da row 0). `GameData::armor_class` now returns u32 (0 for blank/missing), not Option (engine_armor_torso.rs).
- A head of 0, and no Appearance_Head at all, is no head in the client (neck alone, as head 254 with no model); the engine holds 0 for both. mg-preview draws no head for n <= 0 (heads_look, a_head_of_0_is_no_head). The old finding "a head showed without the field" came from MG_NO_BODY, which never removed Appearance_Head.
- Aligned_to_World_Z particles turn about Z to the eye from every side, whatever the emitter's orientation (upright_particles_look); Moonglow already did, no renderer change.

## Tried and ruled out
- SetCameraFacing straight in OnClientEnter (with or without CAMERA_MODE_CHASE_CAMERA): the client keeps its own camera. Wrap it in DelayCommand(1.0, ...). Other client tests that set it at once likely never got their camera (noted in deferred.md).
- Whole-picture differences for small changes (a head): too diluted; compare a region around the subject.

## Watch out
- tic01's 4x4 room looks alike from every side: put something fixed in the world (a card mesh in the probe model) to prove the client's view turned.
- scratch_dir clears its folder: an MG_NO_CLIENT run wipes the client pictures of the last run.
- Client runs share the off-screen display with Aurora: hold both aurora.lock and gpu.lock.
- The engine's at-least-1 gold value hides a computed 0: compare item_cost before that rule (or the UTI Cost via ObjectToJson).
