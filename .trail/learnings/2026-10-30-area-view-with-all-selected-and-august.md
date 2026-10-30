---
title: Area view with all selected and the walkmesh within budget
author: August
date: 2026-10-30
branch: worktree-agent-a7368c202790ee045
cost: { minutes: 75 }
touches:
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/reach.rs
  - crates/mg-render/src/scene.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/tests/frame_perf.rs
---

## Decided
- Sound Ranges draws copies of a shape per dot count (`mg_render::LineShape`, `sound_shapes()` in area_view.rs, built once): the renderer keeps each shape's lines in a buffer keyed by the `Arc`'s pointer (held in the cache so the pointer stays theirs) and sends only the copies. A whole-frame cache of `Vec<Line>` would not have worked: 2-5 of 299 sounds change dot count most frames as the camera turns.
- `ReachCache` (reach.rs): LightGrid::reaching results keyed by the ball's bits, kept while every light's position, cutoff and priority are bit-equal to the last frame's; two generations (now/before) so only the last frame's meshes are kept. Colour changes (flicker) don't invalidate.
- Walkmesh overlay: per-tile box culling (all 8 corners in front of the near plane and beside one side of the rect) plus a no-cut path for faces wholly in front; same arithmetic as `screen_polygon`/`clip_polygon` (the `ahead` vector computed once is bit-equal), so pixels are identical.
## Watch out
- area084's base frame at 120 m is ~4-5 ms of renderer encode (3,900 meshes, 2,500 batches, ~200 pipelines with shadows on); the overlays have ~1.5 ms between them.
- frame_perf's views differ pointer/idle/wheel because the camera keeps turning across them: "idle" being slower is a different part of the area, not noise.
- `FRAME_SHOTS=1` pictures are deterministic but for the log panel's clock (bottom left); compare with a pixel diff and look at the bbox.
- The overview/turning *wheel* views of area084 draw the whole area (16k meshes, ~9 ms encode, as area_parts' overview) and were over before this too.
