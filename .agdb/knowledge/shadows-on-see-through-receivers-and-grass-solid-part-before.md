---
type: Decision
title: "Shadows on see-through receivers and grass: solid part before the lay, soft part times a screen mask"
description: "How grass and see-through tile meshes take the sun's and lights' shadows in both qualities and both shader paths, and the traps met building it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T22:19:50Z }
touches: [crates/mg-render/src/renderer.rs, "crates/mg-render/src/shadow.rs:Mask", crates/mg-render/src/game.rs, "crates/mg-shader/src/link.rs:darken_by_mask", crates/mg-render/src/shader.wgsl]
asks:
  - "why doesn't grass or a tree's leaves get darkened by shadows"
  - what is the shadows mask and who reads it
  - why does a soft part read the mask instead of being drawn before the lay
  - why does the textureLoad of the mask clamp its coordinates
fingerprint: v2-e71804918b756ba5
created: 2026-11-25
author: augustaugur
---
Decided (2026-11-25, branch shadows-first-frame):
- The client darkens grass and a tile's see-through meshes (solid and soft texels alike: ttr01 road 0.39 = its ground, crown 0.07, grass 0.42); deep water not (0.002); a shallow pool shows its darkened floor through refraction (Moonglow's water hides it: the fountain entry).
- Textures are mostly soft (grass 3% solid / 10% soft texels, splotch 3/33, road 69/16), so moving solid parts before the lay was not enough.
- Approach: receives = receives_shadows || grass (grass NOT given receives_shadows: that flag also picks the mesh bucket lead and the map's receiver box). A see-through receiver's solid part (Pass::Blend, receives=true, sort rank 2 with bit 59) is drawn before the lay; the game's opaque receivers are drawn before the first of them (draw_received!). Every darkening pass also writes an R8 mask (MSAA + resolve, ViewState::mask, cleared white per lay) with DARKEN_BLEND, so it holds what the picture was multiplied by. The soft part multiplies rgb by it: own shader by DrawUniform.water.w, game programs by Program::darken_by_mask in every GamePass::Fringe pipeline that reads no fb copy (group 1 binding 11; own group 1 binds white, GameDraw::shaded picks the frame group; batch_key's material top bit keeps shaded and unshaded draws apart).
- Excluded: water (surface.water), ordered (mesh alpha < 1), instance opacity < 1.
Watch out:
- The white texture is 1x1: textureLoad at the pixel reads out of bounds (black). Both shaders clamp to textureDimensions - 1.
- The lay-trigger, the game's non-receivers and the framebuffer copy all fired at 'the first non-solid draw': with receiving Blend draws before the lay they must skip d.receives (past_solid), or the copy and the non-receivers go before the shadows.
- The mask under a soft texel is the mask of what is behind it (the ground), so it needs a receiver under it: a soft strip over a non-receiver gets no shadow.
Tried: the loose end of area084's High grass differing after volumes went apart was not the sway (pose and time identical); it was the game's programs made apart becoming ready at different frames.
