---
title: Framebuffer copies and fswater water by default (steps 1-3)
author: August
date: 2026-10-27
branch: framebuffer-water
cost: { minutes: 1245, tokens: 11158055 }
touches:
  - crates/mg-render/src/framebuffer.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/game.rs
  - crates/mg-shader/src/link.rs
  - crates/mg-render/src/particles.rs
  - crates/mg-corpus-tests/tests/common/mod.rs
  - docs/framebuffer-proposal.md
---

## Decided
- Copies (colour + R32Float depth) live in Targets, made with them; Renderer::render_targets uses them (area view, model viewer, render_image); Renderer::render has none -> shiny water Moonglow's there.
- Copy only when a frame has a reader: solid game draws whose pipeline wants_fb are 'late' (after the copy); particles never ask (particleSoftness 0). Pass cut after the game's solid block, copy draw, second pass loads.
- texFB* moved to group 2 (FB_BINDING 7..10) by place_named on texFBColor_image/_sampler; per-pipeline fb bind group keyed by the copies' id, white in the pipeline's own group 2.
- gl_FragCoord.y flipped in IR in the fragment entry wrapper (Store of the Position argument) using textureDimensions(texFB*_image).y: no uniform needed. Reading screenParams there fails: split_uniforms wants the spare blocks' and draw-place expressions in every function that touches the block.
- inc_framebuffer's 45 deg fov kept (the client's camera is ~48 by the fitted camera).
- GAME_WATER true: client far water 10.9 vs own 12.6, fountain 10.6 vs 11.7, card 14.6 vs 57.1.
- Still particles (particleRot 0) drawn upright: the client draws them unturned.
## Watch out
- Client pictures kept in captures/client/*.png; MG_CLIENT_CAPTURED=1 reuses them (client_capture in tests/common).
- The off-screen display is :2 here (X1 belongs to something else); start it with tools/aurora/headless.sh start.
- The water area's Shaders-off area view is 6.6-8.5 ms GPU on develop too.
