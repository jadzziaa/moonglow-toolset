---
type: Decision
title: "Shadows quality High: the game's stencil volumes"
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: [crates/mg-render/src/volume.rs, crates/mg-render/src/shadow.rs, crates/mg-render/src/renderer.rs, crates/mg-ui/src/area_view.rs, crates/mg-ui/src/settings.rs, crates/mg-corpus-tests/tests/client_render/shadows.rs]
fingerprint: v2-93a6379118ba2a3b
created: 2026-10-30
author: August
description: "Shadows quality High uses GPU-extruded stencil volumes (z-fail) in a separate depth+stencil target, leaving scene Depth32Float and Fast unchanged; skins cast no shadows in High, and the note lists pitfalls."
asks:
  - "How does the High shadow quality option render shadows, and why doesn't it change the scene's depth format?"
  - "Why do skinned creatures not cast shadows when shadow_quality is High but do in Fast?"
  - "Why was a global Depth24PlusStencil8 rejected for stencil shadows, and which depth/stencil format is used instead?"
  - "My frame_perf GPU timings look wrong or inflated when running tests; how do I get accurate costs?"
---
## Decided
- Shadows quality High = the game's stencil volumes (`mg_render::volume`, `Renderer::shadow_volumes`, Options › Area `shadow_quality`). The stencil lives in a depth+stencil target of its own (Depth32FloatStencil8 if the device has DEPTH32FLOAT_STENCIL8, else Depth24PlusStencil8), the scene's depth copied in per sample (`@builtin(sample_index)` + `frag_depth`): the scene's DEPTH_FORMAT stays Depth32Float, so Fast is unchanged and no pipeline doubles. (A global Depth24PlusStencil8 was ruled out: the owner's GPU is AMD, where D24S8 is D32S8, +80 MB always; and three Fast bindings read the depth as texture_depth, which needs DepthOnly views.)
- Volumes are extruded on the GPU from a per-mesh volume mesh made once (`GpuMesh::volume`, OnceLock): per vertex a role and its face's plane; the light in the caster's node space (world.inverse() * light) decides which faces are lit. Each lit face is a prism (near cap 0.1 m along the light, far cap extruded, a side only where the neighbour across the edge isn't lit), and a mesh that isn't closed casts from both sides of its faces: closed whatever the topology, so z-fail counts are right and an edge of three faces (the wiki's shadow tearing) casts no spike.
- vs_shadowvol (the game's, `mg cat vs_shadowvol.shd`) is copied: directional extrusion to projectionWorldZClip (a tile's box's bottom; for non-tiles Moonglow uses the lowest receiver seen, 0.1 m lower so the far cap doesn't sit on the ground's depth), never above max(camera.z, focus.z), extruded vertices past the far plane slid back along the light to it. Its near-plane clamp is left out (z-fail needs nothing near).
- Skins cast none in High (wiki: "Only Trimesh nodes in a model can cast a shadow"; stock creatures are trimeshes: c_ogre 19 trimeshes, c_horse1 and c_drggreen have 2 skins). Fast still casts skins.
## Tried and ruled out
- Reading the owner's blockiness in a 4×4 reading area: its map is small (2 cm texels), Fast looks fine; it shows over a 16×16 area seen to its far side (`high_shadows_beside_the_client`, case `wide`).
## Watch out
- The two sets (sun, lights) need their own caster/frame/param buffers: write_buffer lands before the submitted passes.
- frame_perf's tests run in parallel by default (their GPU times then are each other's): `--test-threads=1` for costs.
- The worktree guard refuses `python3 $VAR/...` and heredocs piped to python: put scripts in the scratchpad and call them with literal paths.
