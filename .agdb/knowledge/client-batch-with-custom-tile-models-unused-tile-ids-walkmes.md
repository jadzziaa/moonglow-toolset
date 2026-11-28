---
type: How-to
title: "Client batch with custom tile models: unused tile ids, walkmesh copied"
description: "How batch9_static_tile_alpha_mean puts meshes of its own in tiles the client draws (Tile_ID of an unused model, the model and the old walkmesh in override), and what it found about tiles' see-through order."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-28T07:39:03Z }
touches: [crates/mg-corpus-tests/tests/client_render/batch9.rs, see_through_order]
asks:
  - How do I put my own meshes in a tile for a client render test?
  - "Do tiles' see-through meshes follow the static alpha-mean order in the client?"
  - "Why copy the old tile's walkmesh when replacing a tile model in a client batch?"
  - Which base-game tiles have meshes of alpha under 1?
fingerprint: v2-dc1af56d4c430267
created: 2026-11-28
author: augustaugur
---
- Replace a tile in a Reading: set Tile_ID (orientation 0, height 0) to a tile of the tileset whose model no tile of the area has (batch7_bucket_order's search), write ITS model name .mdl in user/override (programs::model of Quads; a 10 m floor quad at -5,-5 keeps the ground) and also copy the original tile's .wok under the new name, so the player at (20,20) stands as before (tile 10 touches that corner). Moonglow's beside view reads the same override.
- Found 2026-11-28: tiles' meshes of alpha<1 go in the client's static texture-mean order with static placeables (21/21 predicted, incl. across tiles against tile order and distance, and before dynamic cards). Moonglow: see_through_order takes (receives_shadows || tile).
- A dynamic placeable whose texture is also a static card's still draws after the statics (pairs 15/16), so the order is per object kind, not per texture alone.
- Base game sweep (mg cat --text --to over all .set Model= names): 117 drawn meshes of alpha in (0, 1) in 113 of 12,182 tile models, 80 of them ttu01_water at 0.7 (one texture; ttu01_crstllight's are alpha 0, not drawn); no tile with two drawn ones of different textures. Ordered draws never take the `receives` sort branch (soft_receives needs !ordered), and late (framebuffer) draws are their own rank, so in_client_order can't move shiny water across the copy.
