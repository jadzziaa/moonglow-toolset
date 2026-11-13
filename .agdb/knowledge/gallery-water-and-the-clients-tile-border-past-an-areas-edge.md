---
type: Decision
title: "Gallery water and the client's tile border past an area's edge"
tags: []
status: stable
generated: { by: trail, at: 2026-11-01T00:00:00Z }
touches: [crates/mg-ui/src/model_view.rs, crates/mg-render/src/renderer.rs, crates/mg-area/src/border.rs, crates/mg-area/src/scene.rs, crates/mg-corpus-tests/tests/client_render/readings.rs]
fingerprint: v2-daace10f9ac7193a
created: 2026-11-01
author: August
description: "Gallery water now renders like the area view via Renderer::render_with_copies (fswater, not WGSL fallback), and the client's tile border past an area edge is reproduced with edge tables, rotated models, and corner squares; it is off by default."
asks:
  - "Why does shiny water in the gallery look different from the area view, and how was it fixed?"
  - "How do I make Moonglow draw the tile border (the client's void) beyond an area's edge?"
  - "My readings past an area edge show no border or missing tiles; which client settings and ARE fields do I need to change?"
  - "How should the north-side border model be rotated relative to the east-side model, and what went wrong with other rotations?"
---
## Decided
- Gallery water: every gallery picture goes through model_view.rs render_picture, which called Renderer::render (no framebuffer copies) so shiny water fell back to Moonglow's WGSL water while the area view (render_targets) drew fswater. Fixed with Renderer::render_with_copies(…, &targets.copies). The M5 "water stays Moonglow's" in shaders-proposal.md was superseded 2026-10-27 (GAME_WATER true, framebuffer-proposal.md): check renderer.rs GAME_WATER before trusting M5. A placeable fountain (plc_j02) has no shinywater: fslit in both.
- Tile border (the client's "void"): edge table row by (left corner, crosser, right corner) looking out, either order (reversed: model turned 180°), models made for the EAST side (north side = 1 quarter turn CCW), 5 deep, 5×5 corner squares from `Corner2=****` rows, raised corner = `Terrain+`. Material ambient = diffuse = (0.5·sun diffuse gamma)^2.2, emissive 0; area light unchanged. Moonglow within 0.2/255 of the client (top3 reading).
- Default off (Aurora draws nothing; builder asked for an option).
## Watch out
- The client test SETTINGS turn `graphics.tile-borders` OFF: readings that look past an edge must turn it on.
- Tiles past 45 m are not drawn by the client: raise the ARE's FogClipDist (and SetCameraLimits for a far camera) to see the whole border.
- To read a uniform the client gives a mesh, patch inc_standard's end in the scratch override to paint it by screen third (MG_MATERIAL in reading_tile_border); colors come out raw (no gamma after).
- ARE colors are BGR: a diffuse of 0xFF4020 is R 0x20, B 0xFF.
- The worktree guard refuses `$var` loops and python heredocs containing "git" (e.g. `read(git)`): use a scratchpad sh script with literal paths, or Edit.
## Tried and ruled out
- North side unturned and three quarter turns: streams came out as east-west bands / water channels on the wrong side.
