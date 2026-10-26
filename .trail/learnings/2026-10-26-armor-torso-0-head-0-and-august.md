---
title: "Armor torso 0, head 0 and upright particles measured"
author: August
date: 2026-10-26
branch: engine-client-measurements
cost: { minutes: 28, tokens: 43884 }
touches:
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/05-blueprints.md
  - crates/mg-corpus-tests/tests/client_render.rs
  - crates/mg-corpus-tests/tests/client_render/upright.rs
  - crates/mg-render/src/particles.rs
  - crates/mg-corpus-tests/tests/client_render/heads.rs
  - crates/mg-preview/src/creature.rs
  - crates/mg-preview/tests/previews.rs
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
