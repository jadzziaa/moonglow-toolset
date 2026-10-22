---
title: "The engine's program choice read from the client (M4)"
author: August
date: 2026-10-21
branch: shaders-2
cost: { minutes: 85, tokens: 439678 }
touches:
  - crates/mg-corpus-tests/tests/client_render/programs.rs
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/shader.wgsl
  - crates/mg-image/src/dds.rs
  - crates/mg-image/src/txi.rs
  - crates/mg-ui/tests/app.rs
  - docs/research/notes_shaders.md
---

## Decided
- Program choice was READ from the client, not inferred: client_render/programs.rs puts every stock vs*/fs* (not vsfb*/fsfb*/pltgen) in SCRATCH/user/override with `#define main mg_main` on top and a wrapper main appended: VS writes `out float mgVs`, FS paints (fs id*8, vs id*8, flags) where flags = 4 + 8*m_texture turn + 32*KEYHOLING_ENABLED + 64*FRAGMENT_LIGHTING + 128*NO_DISCARD. A band (gl_FragCoord rows 6-7) shows uniforms. Moonglow's choice is compared card by card via Renderer::game_programs.
- Rules (notes §A.4): decal -> vsvt/fst (globalColor = (1,1,1,mesh alpha)); no tverts -> vslitnotex / vslitnt_sm; _sm for TXI envmaptexture (default too), placeables Reflection, appearance ENVMAP; _nm ONLY with a render hint (maps without one are not read: both paths); bumpyshinytexture ignored; tileset EnvMap = the texUnitEnv of every lit mesh, and what `default` means; vslitc never.
- NO_DISCARD 0 iff no texture, texture alpha (TGA by texels, DDS by format: DXT1 never, DXT5 always; TXI alphamean<1), mesh alpha<1, MTR transparency; env-mapped alpha = reflectivity -> 1. PipelineKey.no_discard; Worker programs keyed (name, stage, no_discard).
- m_texture = tile's turn for rotatetexture meshes of TILES only (Instance::tile); WGSL gets uv_turn.
- WGSL env map: derivatives of env coords at fs_main start, textureSampleGrad with grads * 2^bias (GLSL bias semantics; no uniformity issue). _sm 0.73/15 -> 0.00/0.
## Tried and ruled out
- SetCameraFacing in OnClientEnter doesn't move the client's camera: set Mod_Entry_Dir instead (cameraYaw = anticlockwise from north).
- A single-level standard DDS: the client reads past it (noise at a distance). Write all levels.
- NO_DISCARD 1 didn't speed TDoD's dense view (GPU 16.6 vs 17.5 ms; only 826/16605 meshes, DXT5-heavy).
## Watch out
- fAlphaDiscardValue is undeclared when NO_DISCARD 1: guard marker code by `#if NO_DISCARD != 1`.
- Additive meshes blend the marker: unreadable. Sky color can decode as a mark: tolerance <= 2.
- Sample cards on a decal board so a missing card shows; keep them clear of tcn01's tree.
- frame_perf: run one area per run (FRAME_AREA); the second area of a run measured up to 40% slower. Rebuild M3 sources (git checkout <rev> -- crates/...) for a same-session baseline.
- Reading TDoD hak files: no MTR names texture8-15; no TDoD shader reads gl_FragCoord.
