---
title: "Parts' emitters: tails, wings, held items, chunk size"
author: August
date: 2026-11-11
branch: develop
cost: { minutes: 29, tokens: 144377 }
touches:
  - crates/mg-area/src/scene.rs
  - crates/mg-corpus-tests/tests/client_render.rs
  - crates/mg-corpus-tests/tests/client_render/batch4.rs
  - crates/mg-corpus-tests/tests/client_render/programs.rs
  - crates/mg-corpus-tests/tests/client_render/readings.rs
  - crates/mg-corpus-tests/tests/models.rs
  - crates/mg-corpus-tests/tests/textures.rs
  - crates/mg-image/src/tga.rs
  - crates/mg-preview/src/compose.rs
  - crates/mg-preview/src/creature.rs
---

## Decided
- `Composed::emitting(animation, t)` gives every model of a preview that emits (`source` 0 the base, parts from 1), found once at `Composed::new` (`emits()`): area view (`AreaScene::particles_and_chunks`, sims keyed `place + (source << 40)`), the model view (`part_particles`), pictures (`draw_composed`) all loop over it.
- Keys, read in the client (`client_render/part_emitters.rs`, `part_emitters_look`): tail/wings own anim of the asked name, else own `default` (posed by it), else none (geometry); helmet none; held item (attach rhand/lhand/lforearm, item) its `equip` at its end (`held_at`).
- Particle size × the emitter's world scale (`grown_by`), speed not (WING_TAIL_SCALE, scope 253, placeable scope 0 all read). Chunks at the model's own size × that, never the emitter's size (`reading_chunk_size`).
## Tried and ruled out
- "No animation of the name → geometry" alone: `fx_lavaspurt` (geometry 0, keyed in `default`) spurts in the client.
- `vco_mehanfire01` showing nothing is not keys: its emitters are trails (spawntype 1).
## Watch out
- `Reading::new` empties its scratch folder: copy client.png and the log elsewhere before `MG_CLIENT_PICTURES` redraws, or they are gone.
- A still picture at view.time 0 of an emitter keyed from 0 (lavaspurt) throws nothing: run-ahead keeps the time fixed.
- The commoner placed with no item closure has no clothes, so no body: give `instance()` an item lookup.
