---
title: "Custom Light: way B written into a chosen hak"
author: August
date: 2026-10-30
branch: worktree-agent-a02be1f76ba28c11f
cost: { minutes: 120, tokens: 9530372 }
touches:
  - crates/mg-module/src/lights.rs
  - crates/mg-ui/src/light_maker.rs
  - crates/mg-area/src/scene.rs
---

## Decided
- Way B behind an option off by default (owner, 2026-10-29): settings.generated_lights, project [lights] generated (wins), hak, first_row. mg_module::lights::plan works out everything before writing; the placeables.2da written is the copy the game reads with the row appended, so the chosen hak must hold that copy or be above it (else refused); same settings reuse their row/model (mgl_ + 48-bit FNV of the settings).
- Light model: dummy + unrendered 0.2 m square + light node (affectdynamic, shadow, lightpriority); classification character; compiled by mg_mdl. Row cells copied from the game's Light, White: Static 1, ShadowSize 1, BodyBag 0, LightColor empty.
- Engine (engine_custom_light.rs): the server reads the new hak, Get2DAString gives the model, ResManGetAliasFor finds it in the hak, the placeable is placed.
- AreaScene::bare_light treats model-lamp objects as bare lights only for generated models (a builder's small lamp model stays as drawn: the_area_view_is_lit_by_objects_lights_and_illumination).
## Watch out
- area_app's scratch_dir wipes ui-area-<name>: keep before/after pictures in another output_dir.
- light_maker::listed uses app.haks_layered, not module.haks() (stale after an unsaved hak-list edit).
- Tests that write a new hak must give the harness a scratch user folder (area_harness_with user), never the real one.
