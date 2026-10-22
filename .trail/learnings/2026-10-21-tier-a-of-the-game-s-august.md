---
title: "Tier A of the game's shaders (M2): MTR pairs, the Shaders switch, TDoD moving"
author: August
date: 2026-10-21
branch: shaders-2
cost: { minutes: 31, tokens: 155753 }
touches:
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/model.rs
  - crates/mg-shader/src/link.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/model_view.rs
  - crates/mg-corpus-tests/tests/game_shaders.rs
  - crates/mg-corpus-tests/tests/client_render.rs
  - docs/shaders-proposal.md
---

## Decided
- An MTR's custom pair is used only when it names BOTH customshaderVS and customshaderFS (game.rs custom_pair); one alone = the stock pair. Evidence: 250 TDoD MTRs name `customshadervs fslit_nm` alone (tdod_fbr armor parts) and aren't magenta in game. Verify: shader-compile compiles pairs only; new `shader-pair` warning, one per hak.
- Shaders switch = Settings::game_shaders: Option<bool>, default GAME_SHADERS_BY_DEFAULT=false (flip after M3). One shared Renderer for all 3D views; set renderer.game_shaders right before the area render (from the picture key), not only at frame start, or the cached picture never redraws.
- See-through game draws reuse Moonglow's two-part split: two pipelines (GamePass::Solid/Fringe), Program::keep_alpha adds a Kill on the output alpha in IR, same uniform blocks (GameDraw::with_pipeline); they sit in `draws` (Draw.game) sorted with WGSL see-through draws, batcher key made unique.
- NO_DISCARD stays 0; fAlphaDiscardValue 0.2 (client-measured). Ghost instances (opacity<1), additive, sky, markers stay WGSL.
- Tangents made for meshes without (model::made_tangents); WGSL ignores them via light_count.w = tangents_made so its look is unchanged.
- areaGlobalWind from ARE WindPower: (0.707,0.707,0)*power (TDoD shader comments); areaFlags = ARE Flags (Scene::environment).
## Tried and ruled out
- "timed" = block has worldtimerTimeOfDay: wrong, glslang keeps every DECLARED loose uniform and every stock program declares it -> use Program::reads_uniform (AccessIndex on the block global).
- Asserting a see-through solid part shows no under-layer at all: the >=0.95 part still blends ~3% of whatever was drawn before it (WGSL too); tolerate ~13/255.
## Watch out
- TDoD grass drew BLACK on the game path: binary model, no tangents, MTR renderhint -> stock SetupTSB normalize(0) = NaN. Fixed by made tangents.
- Wind sway phase is world Y * time: a test mesh at y=0 never moves. Place it off the axis.
- TDoD's inc_lighting (tdod_shaders.hak) darkens everything on the game path; compare with the stock include before calling a colour difference a bug. Against the client it MATCHES (floor 73.4 vs 73.5; WGSL 87.9).
- Roof shader FS discards fragments whose ORIGINAL world z > 0: roof meshes live under the walkmesh and are raised by fRoofOffsetZ.
- placeables.2da for TDoD is in tdod_top.hak, not tdod_rules. mg_testkit::scratch_dir empties the dir on each call: use output_dir for several pictures.
- Client: symlinks into SCRATCH/user/hak work (bwrap binds / read-only).
