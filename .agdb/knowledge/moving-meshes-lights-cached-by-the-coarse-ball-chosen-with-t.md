---
type: Decision
title: "Moving meshes' lights: cached by the coarse ball, chosen with the exact ball"
description: "Why ReachCache::reaching takes a key ball and an exact ball, and what a moving mesh may miss."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T08:23:13Z }
touches: [crates/mg-render/src/reach.rs, crates/mg-render/src/renderer.rs]
asks:
  - "why does ReachCache::reaching take two balls?"
  - does a skinned mesh pick different lights than a still one with more than 32 reaching?
  - what is coarse_ball for in the renderer?
  - can a swaying creature miss a light at its cutoff edge?
fingerprint: v2-49fec94273730609
created: 2026-11-24
author: augustaugur
---
The renderer keys a moving (skinned, swaying) mesh's light choice by coarse_ball (centre snapped, radius grown) so the cache holds while it sways. With more than 32 lights reaching, ranking by the coarse ball could pick a different 32 than the mesh's own ball, so ReachCache::reaching(key, exact, ..) chooses anew with grid.reaching on the exact ball and only caches under the coarse key. Trade-off: the kept choice is the first exact ball's; while the mesh drifts inside its coarse ball (about 0.2 m) a light reaching only the drifted ball is not added (it is at its cutoff, adding almost nothing), and two moving meshes sharing a coarse ball share the first one's choice. Unit test reach::tests::a_mesh_known_by_a_coarse_ball_takes_its_own_ball_s_lights (33 lights, no GPU); the one-line renderer wiring has no pixel test.
