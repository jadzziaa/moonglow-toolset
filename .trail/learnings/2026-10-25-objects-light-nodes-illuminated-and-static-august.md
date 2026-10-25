---
title: "Objects' light nodes, Illuminated and static reflection, read from the client"
author: August
date: 2026-10-25
branch: area-lighting
cost: { minutes: 726, tokens: 5669132 }
touches:
  - crates/mg-corpus-tests/tests/client_render.rs
  - crates/mg-ui/tests/frame_perf.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/04-areas.md
  - docs/manual/05-blueprints.md
  - docs/manual/12-differences.md
  - docs/research/notes_shaders.md
  - crates/mg-area/src/scene.rs
---

## Decided
- Read in the client (`model_light_uniforms`, an authored ASCII `plc_a01` in the scratch override, debug `inc_standard` blocks 32..): a placeable model's `light` nodes are lights (static or not). MDL `multiplier` multiplies the REACH only: cutoff = radius x 2 x max(1, max channel) x multiplier, colour = lin(c / max(1, max channel)). `Lamp::placed` now does that (was colour x multiplier). Tile/fx_flame01 lights unchanged (multiplier 1): `light_uniforms_match_the_client`, `reference_scenes...`, `placeable_light_uniforms` still pass.
- `LightState` 0 removes only the placeables.2da `LightColor` light (fx_placeable01); model light nodes stay. Moonglow keeps the row light in `Preview.lights` with `on: false` (the invisible light's sun marker stays, gray) and `Composed::row_lights` filters it.
- Client keeps 32 lights per draw by priority then nearness (crowd of 36, the 2 priority-1 far ones kept) = reach.rs already.
- A STATIC placeable ignores its placeables.2da `Reflection` (default or named): its texture alpha is opacity (alpha-0 card invisible). TXI `envmaptexture default` still reflects the tileset map. `mg_preview::placeable` drops `env_map` when Static != 0 (`area_env_map_look`). Heed's copper armor stand = tin01__ref01 (mean 85,63,48) via Reflection default.
- Object lamps gathered once per Composed (idle anim t=0; parts at rest, hung on the attach node). Model viewer and tile pictures use `row_lights` (base lamps come from the viewer's own Lights / tiles' main lights are area-coloured).
- The stock smithy (Chapter1 pensmithy1, tin01, tile lights only) matched the client 2.4/255 before and after; Heed's "tile lights missing" was the placeables' light nodes.
## Tried and ruled out
- Static placeable cards in `area_env_map_look` first: looked like "Reflection ignored everywhere"; dynamic cards do reflect. Always place both.
## Watch out
- `pkill -x nwmain-linux` can hit the user's OWN game client (no -userdirectory): kill only your bwrap child's PID.
- `area_lighting_look` logs the player's position (MG_PC): the engine snaps the entry point to the walkmesh, so fit the camera to the logged one.
- Readback of cutoff clips at 256 m (sqrt(|a|)/256): a 300 m light reads 256.
- 100 Txpple light-node placeables (reach up to 300 m) in the TDoD woods: close view 7.0 ms CPU / 8.0 GPU (2 lights: 4.7 / 2.3; 20: 4.3 / 3.0). `FRAME_TDOD_LAMPS`.
