---
type: Decision
title: "Soft particles: the client's particleSoftness and the fade (step 4)"
tags: []
status: stable
generated: { by: trail, at: 2026-10-27T00:00:00Z }
touches: [crates/mg-render/src/particles.rs, crates/mg-render/src/renderer.rs, crates/mg-render/src/game.rs, crates/mg-corpus-tests/tests/client_render/soft_particles.rs]
fingerprint: v2-e8349770b72c3e53
created: 2026-10-27
author: August
description: "Client particleSoftness equals 10 / max(sizeStart, sizeEnd), zero for Billboard_to_Local_Z and Billboard_to_World_Z, and maps to Moonglow's soft particle path; the fade is size/10 deep, and read runs need post effects off."
asks:
  - "How is the client's particleSoftness value calculated from particle size, and when is it zero?"
  - "Why do my soft particle colour readings look wrong or differ from the client, and what settings should be off?"
  - "How does Moonglow carry softness from ParticleBatch through to Key::ParticleSoftness, and what triggers the framebuffer copy?"
  - "How deep is the soft particle fade and how should I compare feet against the reference picture?"
---
## Decided
- Client particleSoftness = 10 / max(sizeStart, sizeEnd); 0 for Billboard_to_Local_Z and Billboard_to_World_Z; same at Performance (default) and High Quality; blend and Aligned_to_World_Z / Motion_Blur don't matter. Read by overriding fsparticle.shd itself (painting the uniform over each particle, emitter number in the particle colour's red /16).
- Moonglow: ParticleBatch::softness -> DrawValues.particle_softness -> Key::ParticleSoftness; a soft particle draw triggers the framebuffer copy. Renderer::soft_particles (default on) for comparing.
## Watch out
- Read runs must keep the post effects off (fbo.*): the client's default bloom/sharpen/vibrance change the colours read. 'Medium' is not a shader-quality the client knows (only Performance, High Quality): it compiled quality 0.
- Lighten particles add to what is behind: read them one at a time (lifeExp < 1/birthrate) over a black card.
- The fade is size/10 deep: a row or two at the reference camera; compare feet by each picture's own edge (the fitted camera is a few px off).
