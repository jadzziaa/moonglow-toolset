---
type: Decision
title: "Flickering glow: overlays across the two shaders"
tags: []
status: stable
generated: { by: trail, at: 2026-10-23T00:00:00Z }
touches: [crates/mg-render/src/renderer.rs, crates/mg-render/src/game.rs, crates/mg-render/tests/render.rs, docs/deferred.md]
fingerprint: v2-a8ae60cecc6616f9
created: 2026-10-23
author: August
description: "Flickering patches on the zombieash creature's additive glow come from two shaders computing slightly different depths; fixed by OVERLAY_DEPTH_BIAS (constant -8, slope -1) on additive and fringe passes in both the WGSL and game shaders."
asks:
  - "Why does the additive glow on a skinned creature flicker or show hatching patches from frame to frame, especially at a distance?"
  - "How do I stop z-fighting between a creature's body and its glow overlay rendered by different shaders?"
  - "What depth bias values were tested for overlay passes, and which were the minimum that looked clean?"
  - "Why not make vs_skinned match the game's vslit_sk programs exactly, or route the arturo glow through the game's programs?"
---
## Decided
- dd_ce3_zombieash: two skins of the same 1857 vertices; the body by the game's programs (vslit_sk), the additive glow (TXI proceduretype arturo -> surface.ripple -> game_path false) by WGSL vs_skinned. Depths reckoned apart (GL projection + (z+w)/2 remap and the bone sum vs wgpu's projection) differ by the last bits: patches and hatching that change from frame to frame, worst seen from afar. The client draws both skins with one program: clean (creatures_look, MG_APPEARANCES=28518).
- Fix: OVERLAY_DEPTH_BIAS (constant -8, slope -1) on the passes drawn over the scene without laying depth, in both shaders: WGSL Pass::Additive/Fringe and GamePass::Additive/Fringe. Measured with a scratch harness at 1.6/20/60/150 m: -4,-1 was the least that was clean; -2,0 left 54 pixels at 20 m.
## Tried and ruled out
- Making vs_skinned bit-identical to the game's programs: different compilers and projections; not provable.
- Routing the arturo glow to the game's programs: drops Moonglow's ripple, and any fallback (failed program, known effect) mixes paths anyway.
## Watch out
- A close camera with a wide fov showed almost nothing; the fight shows at an area view's distance with a narrow fov (depth precision).
- Not covered: a see-through overlay's solid part (writes depth). docs/deferred.md.
- A creature's emitters hide its body in client pictures: strip 'node emitter' blocks from an ASCII copy in the scratch override.
