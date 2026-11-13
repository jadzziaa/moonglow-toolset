---
type: Decision
title: "Interior shaders and the placeable's default reflection, read from the client"
tags: []
status: stable
generated: { by: trail, at: 2026-10-25T00:00:00Z }
touches: [crates/mg-preview/src/object.rs, crates/mg-corpus-tests/tests/client_render.rs, crates/mg-ui/tests/app.rs, crates/mg-corpus-tests/tests/common/mod.rs, docs/findings.md, docs/deferred.md]
created: 2026-10-25
author: August
description: "In the client, placeables.2da Reflection 'default' means chrome1, not the tileset EnvMap, and mg_preview maps it to PLACEABLE_DEFAULT_ENV_MAP; the 'shaders make interiors darker' report does not reproduce on stock content."
asks:
  - "Which environment map does a placeable with Reflection 'default' in placeables.2da use in the client, chrome1 or the tileset map?"
  - "Interiors look darker or grayer with shaders on; is that real on stock content, or caused by a hak or override inc_lighting?"
  - "How can I tell which env map a surface is using when flat cards facing the camera all look near-black?"
  - "My off-screen client render uses the wrong display or a cargo test is refused in a worktree; what should I run instead?"
---
## Decided
- placeables.2da Reflection 'default' = chrome1 in the client (tin01 and tic01), NOT the tileset EnvMap; TXI envmaptexture default and appearance ENVMAP default (bone golem row 24) = the tileset map. mg_preview::placeable maps default -> PLACEABLE_DEFAULT_ENV_MAP. Read with spheres painted by a patched inc_standard: FragmentColor = texture(texUnitEnv, vTexCoords) under ENVIRONMENT_MAP == 1 (area_env_map_look MG_ENV_PAINT MG_ENV_SHAPE=sphere MG_ENV_CREATURE=24); the override is shared by the client and Moonglow's game path, so both paint.
- Heed's 'Shaders on darker/grayer' is NOT reproduced on stock content: 7 interiors (2-111 lights) on == off to 130 px / max 4 of 255, both 2.5-15 of 255 from the client; owner settings (post FX on) only +0.02-0.07 saturation; Performance vs HQ a level or two. A hak/override inc_lighting (tdod_shaders, Tjured's diffuse wrap) makes on a tenth darker, as the client.
## Tried and ruled out
- Flat cards facing the camera to tell env maps apart: each samples one dark spot of the map; client cards near-black for any map. Use spheres.
- Candidates gamma/fog/uniforms/defines/intensity on stock content: excluded by on == off (WGSL is a near-exact port).
## Watch out
- The off-screen display was :2 here (the desktop's Xwayland held :0 and :1): use the display headless.sh prints, never the default :1 of run-client.sh.
- Large client differences in interiors are geometry: the client draws only tiles the player's tile sees (tileset visibility); Moonglow draws all, as Aurora.
- In a worktree session flock/loops are refused inline: scratchpad scripts (client.sh, areas.sh) wrap cargo test under the GPU lock.
