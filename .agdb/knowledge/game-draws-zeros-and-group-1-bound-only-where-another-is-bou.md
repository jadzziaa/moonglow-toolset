---
type: Fact
title: "Game draws' zeros and group 1 bound only where another is bound; what is left of the woods' switch-on cost"
description: "Why the TDoD woods cost more with the game's shaders, what was cut on 2026-11-25 (shared group 1, zero buffer bound once) and what was tried and dropped."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T03:24:12Z }
touches: [crates/mg-render/src/game.rs, crates/mg-render/src/renderer.rs, crates/mg-render/src/batch.rs]
asks:
  - "Why does the woods view with the game's shaders cost more than with Moonglow's own?"
  - Was putting the chunk in the game see-through batch state tried?
  - How are group 1 bind groups shared between game draws?
  - Why must a skin bind clear the zero-buffer bit?
fingerprint: v2-9b16702751e2aa96
created: 2026-11-26
author: augustaugur
---
Frame cost work of 2026-11-25 (branch frame-cost).
- Seen: TDoD woods seen whole, Shaders on: every game draw set group 1 (bones, framebuffer copies, mask) and the zero vertex buffer again, 5,514 binds a frame each.
- Fix: one group 1 bind group per Group1Key (each stage's bones binding and size, reads_fb, shaded, frame) kept in GameDraws::group1s (cleared on clear and bones.fit, pruned with masks and framebuffers); Bound3 skips a bind of the same group and offsets; BoundMesh::zeros is a slot bitmask, and a skin bound at a slot clears its bit (test the_zeros_are_bound_again_after_a_skin_took_their_slot). Cut ~11k commands and ~1.0 ms encoding (in-process A/B). m_texture and skinmesh written as usual values (Own::Texture, Own::Skinmesh): small.
- Left, inherent: per-material programs give 5.5k-7.5k pipeline changes against ~3.2k off; the vertex stage samples texUnit0, so group 2 is per material (two groups a change); the game's see-through draws keep runs by material and change chunk ~21.8k times against 5.9k off.
- Tried and dropped: the chunk in the game see-through batch state (slower, more pipeline changes); runs split into chunk parts (21.8k->16.5k chunk changes, ~0.45 ms median in A/B, lowest mixed: inside the spread, not kept).
- wgpu 30's cost is in CommandEncoder::finish (encode_render_pass), per command; set_vertex_buffer/set_index_buffer each take a RwLock read.
