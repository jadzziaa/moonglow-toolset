---
title: "Soft particles: the client's particleSoftness and the fade (step 4)"
author: August
date: 2026-10-27
branch: framebuffer-water
cost: { minutes: 1241, tokens: 11181606 }
touches:
  - crates/mg-render/src/particles.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/game.rs
  - crates/mg-corpus-tests/tests/client_render/soft_particles.rs
---

## Decided
- Client particleSoftness = 10 / max(sizeStart, sizeEnd); 0 for Billboard_to_Local_Z and Billboard_to_World_Z; same at Performance (default) and High Quality; blend and Aligned_to_World_Z / Motion_Blur don't matter. Read by overriding fsparticle.shd itself (painting the uniform over each particle, emitter number in the particle colour's red /16).
- Moonglow: ParticleBatch::softness -> DrawValues.particle_softness -> Key::ParticleSoftness; a soft particle draw triggers the framebuffer copy. Renderer::soft_particles (default on) for comparing.
## Watch out
- Read runs must keep the post effects off (fbo.*): the client's default bloom/sharpen/vibrance change the colours read. 'Medium' is not a shader-quality the client knows (only Performance, High Quality): it compiled quality 0.
- Lighten particles add to what is behind: read them one at a time (lifeExp < 1/birthrate) over a black card.
- The fade is size/10 deep: a row or two at the reference camera; compare feet by each picture's own edge (the fitted camera is a few px off).
