---
title: "The special surfaces through the game's programs (M5)"
author: August
date: 2026-10-21
branch: shaders-2
cost: { minutes: 420, tokens: 820623 }
touches:
  - crates/mg-corpus-tests/tests/client_render/programs.rs
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/shader.wgsl
  - crates/mg-render/src/batch.rs
  - crates/mg-shader/src/link.rs
  - crates/mg-area/src/scene.rs
  - crates/mg-ui/tests/frame_perf.rs
  - docs/shaders-proposal.md
---

## Decided
- Read in the client (surfaces_in_the_client, runs marked/black/sky/waterband/unmarked): water vswater/fswater ND1 (tile or placeable with TXI shinywater) with the TILESET'S 2D EnvMap bound (envMapCube 0), not the bumpyshiny cube; grass vsgrass/fsgrass ND0 FL0; skybox vsvt/fst ND1; additive meshes chosen as any mesh (vslit/fslit, _sm, decal -> vsvt/fst) ND1; decal+envmap vsvt_sm/fst_sm; particles vsparticle/fsparticle blendMode 0/1/2 (Normal/Lighten/Punch), ND 1/1/0, particleSoftness > 0; items on the ground _sm.
- Drawn by the game path now: sky (Program::at_far_plane: z = w*0.99999 in IR, no depth write, cull none), grass (Instance::grass), additive (GamePass::Additive, One/One like WGSL), particles (Geometry::Particles, vertex inputs by name from ParticleVertex, vColor = particle color, Lighten SrcAlpha/One). Sky fade stays WGSL.
- Water stays WGSL (GAME_WATER false; Renderer::game_water to compare): fswater without texFBColor/texFBDepth (white = far plane) draws opaque pale water; client water is the refraction of what's under it. Fountain 16.4 vs 11.7, lake 19.4 vs 12.6.
- See-through/additive/sky game draws batched (batch Key gained the pipeline; game_blended re-laid in draw order so a batch's instances are consecutive): area092 6.3 -> 5.8 ms back to back.
- M4's macOS CI failure: textureSampleGrad on a texture_cube -> naga MSL gradient2d(float3) -> Metal refuses the 'lit' pipeline. Fixed with cube_level + textureSampleLevel; tests: no grad/bias on cube in WGSL, naga msl-out has no cube .sample with gradient2d.
## Tried and ruled out
- A quick (FxHash-like, byte-at-a-time) hasher for the light sets: measured SLOWER than std's SipHash on slices; kept std.
- Expecting the particle game path or grass to be what breaks the water view's budget: no, M4 was already 7.9 ms there (2,400 game meshes, 160 lights; the lights block is 1 KB per set and pipeline).
## Watch out
- Additive marks read only over black: make the board's fst paint black (marked_as black). Several Lighten particles in one place add to white: one at a time (lifeExp < 1/birthrate).
- SetCameraFacing from OnClientEnter doesn't move the camera; DelayCommand(1.0, AssignCommand(pc, SetCameraFacing(...))) does.
- A water texture without arturo has ripple None: gate water on the game path by surface.water, not by ripple.
- Cube maps are uploaded with ONE mip level: a cube LOD mutation changes nothing; don't read the cube scene as testing the level.
- frame_perf on this machine is noisy when another session runs nwserver/rustc (load 3-6): A/B back to back, twice each; env-var toggles must be read once per frame, not per draw.
- pkill -f in zsh killed my own command (memory warns): never pkill by pattern.
