---
title: Renderer fixes from the sixth client batch
author: August
date: 2026-11-02
branch: worktree-agent-a4a3c0aa1c6635d90
cost: { minutes: 2939, tokens: 20111468 }
touches:
  - crates/mg-render/src/shadow.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/shader.wgsl
  - crates/mg-render/src/anim.rs
  - crates/mg-mdl/src/lib.rs
  - crates/mg-shader/src/assemble.rs
  - crates/mg-corpus-tests/tests/common/mod.rs
---

## Decided
- A client reading's Moonglow side is drawn again without the client: `MG_CLIENT_PICTURES=<an earlier run's target/test-output>` makes `common::client_screenshot` copy that run's `client_reading_<name>/client.png` and its log (the player's place) instead of running the client. The sixth batch's pictures were in the reading agent's own worktree (`agent-a60b1d06317702ef5/target/test-output`), not the main checkout's.
- Light shadows: darkness = min(the light's share, opacity × fade), the share computed in the darkening pass as Moonglow's shader lights (area ambient + softened N·L diffuse + the 32 lights nearest the target, the casting ones first; `shadow::Share` in the darken uniform), the receiver's normal from `cross(dpdx(world), dpdy(world))` taken before any discard, turned to the camera; gamma terms: 1 − (1 − lamp/total)^(1/2.2). The result tracks the client's measured *shares* closely (its darkening runs a little above them at 3–7 m). Cost +0.26 ms at 16 casters, 3440×1440 4×.
- Ripple (`proceduretype arturo`): push 0.58 × amplitude / texture width per unit of the shader's wave (whose vector RMS is √2), clock 0.18 × speed^0.83 rad/s; both fitted by rendering a uv-coded card over 20 s and reading it as `ripple2.py` read the client (quantization adds to the change rate: fit on the render, not on the formula). Shiny water keeps the old far × 0.12, speed × 0.06.
- Chain scale: `mg_mdl::animations_scaled` carries each animation's product of `setanimationscale` along the chain (the owner's not counted); `Binding::all_scaled`, `pose_scaled`; compose's worn parts take the base's chain scale for the base's animation.
- Fade Geometry: `MeshOverride::hidden` (not drawn, still casts, not a receiver) instead of alpha 0.
## Tried and ruled out
- Porting the grass's distance fade ((32 − d)/16): the area view's camera stands farther than the game's, so it would remove the grass the owner asked to see across the whole view. Left for the owner.
- Previews' shadows: no ground to receive them; a shadow catcher changes every thumbnail. Left for the owner.
- Pipelines at module open: keys need mesh traits incl. NO_DISCARD (texture alpha), i.e. every area's models and textures read. Left for the owner (remembered keys per module is the cheap way).
## Watch out
- The worktree guard refuses heredocs, `sed` programs containing `||`, `for` loops over computed paths and relative script paths after `cd`; write helpers into `target/` with Write and run them by absolute path, or use Edit.
- The High (stencil) shadows end a tile's sun volume at the tile model's box bottom: a test "roof" model alone must not be `tile: true`.
