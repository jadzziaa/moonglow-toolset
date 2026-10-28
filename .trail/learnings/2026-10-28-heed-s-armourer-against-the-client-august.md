---
title: "Heed's armourer against the client, tiles' emitters run ahead"
author: August
date: 2026-10-28
branch: worktree-agent-a322801de0732fb1b
cost: { minutes: 30, tokens: 201777 }
touches:
  - crates/mg-area/src/scene.rs
  - crates/mg-corpus-tests/tests/client_render.rs
  - crates/mg-corpus-tests/tests/client_render/heed.rs
  - crates/mg-ui/tests/app/drawing.rs
  - crates/mg-ui/tests/app/tiles.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/04-areas.md
  - crates/mg-corpus-tests/tests/client_render/armourer.rs
---

## Decided
- Heed's hss_armourer (tni01 smithy group 296-299 in tile-94 black filler, 60/170 day, fog 5, PaleRed row 26 main lights) matches the client within 0.1-1.6/255 whole, <=3.1 per region, Shaders on == off: heed_armourer_look (client_render/armourer.rs) builds it from stock content (TILES table); MG_HEED_AREA=folder reads a builder's .are/.git instead and prints field differences.
- Tile main lights: the lightcolor row replaces the node's colour even when the node is black (r23_02/03/04 nodes are 0 0 0), radius forced 10/5 (nodes say 14/9). Filler tiles without ml/sl nodes light nothing, whatever their Tile_MainLight/SrcLight. LightingScheme is never read by the renderer or the client.
- Fixed: tiles' own emitters were not run ahead when new (objects' and source flames' were), so a forge fire (lifeExp 5) was dark for seconds after opening or editing. Now PARTICLES_AHEAD like objects; 128 smoking tiles cost +4 ms once.
- a_builders_area_in_the_view_draws_as_the_comparison (mg-ui drawing.rs, ignored, MG_HEED_AREA): the window's own picture vs mg_area scene + render_image at the orbit camera: within 0.9/255 (the gap is the start-location marker).
## Tried and ruled out
- At-once SetCameraFacing(f, ...) in OnClientEnter is ignored by the client (camera looks north whatever f and Mod_Entry_Dir). Delayed 1 s it applies: SetCameraFacing(f, 12, 45) fits camera_facing(..., fov 48, focus 1.25, distance 11, pitch 44) = TURNED_DISTANCE/TURNED_PITCH; scripted_camera() picks. fit_turned_camera under MG_FIT refits.
## Watch out
- Client comparisons that don't call AreaScene::particles draw no particles at all: a forge/brazier looks dark in Moonglow's picture only because of the test.
- The client culls by the player's tile visibility: a player on tni01's doorway tile (r23_02) draws nothing but creatures. Keep cameras' players inside rooms.
- Use your own headless display: a copy of headless.sh with another --socket and MOONGLOW_ORACLE=scratch (got :4); pass DISPLAY explicitly, never the desktop's :0/:1.
