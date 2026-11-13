---
type: Decision
title: See-through ghosts that seem missing from pictures
tags: []
status: stable
generated: { by: trail, at: 2026-10-16T00:00:00Z }
touches: [crates/mg-ui/src/area_view.rs, crates/mg-ui/tests/app.rs, crates/mg-ui/tests/screens.rs, docs/deferred.md, docs/log.md, tools/aurora/headless.sh]
created: 2026-10-16
author: August
description: "Ghosts that looked missing from egui_kittest wgpu screenshots were actually blended correctly; low opacity and a dark statue in a wall made them faint, so verify by pixel-diffing renders with and without the ghost."
asks:
  - "Why does the blueprint ghost look invisible in my egui_kittest wgpu screenshots even though it is being drawn?"
  - "How do I verify a see-through ghost is actually rendered by comparing images with and without it?"
  - "What are GHOST_OPACITY and the tile preview opacity set to, and does drawing the ghost unlit improve contrast?"
  - "How do I run the real GUI headless with tools/aurora/headless.sh and get reliable screenshots and clicks?"
---
## Decided
- A ghost "missing" from the egui_kittest wgpu pictures was drawn and blended correctly: at 0.6 it added ~0.55-0.64 of the opaque model's difference per pixel. The cause was the frame (a dark statue half inside a wall, before a banner) and low opacity. GHOST_OPACITY is 0.8 (the tile preview is 0.85).
- Hold a ghost to the picture, not only to counts: render with and without it and count changed pixels around it, skipping the cyan outline (blue minus red > 60). See a_blueprint_about_to_be_placed_shows_in_the_views_picture.

## Tried and ruled out
- Drawing the ghost unlit (Instance::unlit): less luminance contrast than lit, not more.
- Suspecting the Blend/Fringe split, the batcher or PREDICTABLE renderer options: all fine.

## Watch out
- Before calling a ghost invisible, compare against opacity 0 and 1 pixel by pixel. Check that the spot under the pointer isn't inside a wall (depth hides it even when opaque).
- Real GUI on tools/aurora/headless.sh: run with WGPU_BACKEND=gl, because Vulkan windows give stale xdrive screenshots. Approach a click in several small pointer moves, or egui misses it. Use scratch XDG dirs and reset them between runs.
