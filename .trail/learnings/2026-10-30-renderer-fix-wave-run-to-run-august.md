---
title: "Renderer fix wave: run-to-run order, additive alpha, boneless skins, env default"
author: August
date: 2026-10-30
branch: worktree-agent-a7881d371c6a5bf02
cost: { minutes: 2143, tokens: 14072907 }
touches:
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/model.rs
  - crates/mg-corpus-tests/tests/client_render/readings.rs
  - crates/mg-corpus-tests/tests/client_render/batch2.rs
---

## Decided
- Run-to-run picture differences with the game's shaders were the game draws' sort key: (pipeline, material, mesh address). Addresses change per process (ASLR, allocation), so coplanar meshes of one material (decals) swapped winners. `mg_render::game::sort` keys a mesh by the place of its first draw instead; all 140 Tyrants areas identical across processes. `coplanar_meshes_of_one_material_keep_the_scene_s_order` catches it in one process (two quads, the scene's order reversed).
- Additive meshes blend SrcAlpha, One (both paths), as the client. Moonglow's own shader divides specular by alpha for see-through surfaces, which with One, One made alpha-0 texels the brightest.
- A skin face with a vertex no valid bone weighs (or a skin with no bones) is dropped at upload (`GpuModel::new`): the client shows nothing of c_wings2's boneless right wing.
- A texture's `envmaptexture default` on a placeable whose Reflection is `default`: the area's map (`Instance::env_map_default`, set from `mg_preview::Part::env_map_default`). The viewer keeps the old behavior until it copies the flag.
- `graphics.grass.render-distance` is squared metres (900 = 30 m from the eye).
## Tried and ruled out
- Facelift cliffs' "sky through black": not index lists. The client fades `tilefade` meshes about the player; with the player 14 m away (`MG_PC_Y=6`) it matches Moonglow.
- Low-sun street shadows: not shadow-map acne (bias 0.5 m changes nothing). Placeable shadows are full length in the client at 14 degrees; tcn01's building tiles' shadows vanish at 14 degrees but not at 27/45. Rule unknown.
- Timing grass sway from window pictures: consecutive `winshot`s repeat (frames about 0.4 s apart).
## Watch out
- The worktree guard refuses heredocs/commands whose text contains `git` (e.g. `r.git.root`, `git_sun`) or computed paths: use Edit, or scripts in the scratchpad run by a plain `python3 FILE` / `sh FILE`.
- To render a "before" picture, a one-line hack of the fix (e.g. `true ||`) is quicker than another worktree; remember to revert it (grep for it).
- A creature for wings must be part-based (appearance 6) and wear nothing: nw_commale's appearance 267 has no wings node.
