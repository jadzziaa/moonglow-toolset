---
type: Decision
title: "Heed's dark gray area lights and a custom pair's lighting"
tags: []
status: stable
generated: { by: trail, at: 2026-10-27T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/client_render.rs, crates/mg-render/src/renderer.rs, crates/mg-render/tests/render.rs, docs/deferred.md, docs/findings.md, docs/log.md, crates/mg-corpus-tests/tests/client_render/heed.rs, Cargo.lock, Cargo.toml, packaging/linux/io.github.moonglow_toolset.Moonglow.metainfo.xml]
fingerprint: v2-5c96e1be91b24b46
created: 2026-10-27
author: August
description: "Moonglow's area lighting matches the client for Heed's dark gray interior (Shaders on equals off); the only real gap was custom pairs with NORMAL_MAP 1 in the FS, now fixed via Slots::lit_normal_mapped."
asks:
  - "Why does my custom shader pair look slightly brighter in dark corners with Shaders off than the game client shows?"
  - "Does the game use the _nm lighting for a custom fragment shader that defines NORMAL_MAP 1 regardless of render hint?"
  - "How do I reproduce the client camera for a Heed interior capture with MG_CLIENT_CAPTURED and compare region mean colours?"
  - "Heed says his area looks dark and muddy; did lights, fog, colour space or sun/moon explain it?"
---
## Decided
- Area ambient/diffuse are LIGHT added to tile lights, (b/255)^2.2 per channel (gray 60 = 4% white ambient, 170 = 41% diffuse from the default sun dir); white ambient drowns tile lights in the client too. Stock tin01 interior set as Heed's (map_m1q1k002, Always Bright, Interior Bright = scheme 12 whose DARK colors are exactly 60/170, black fog 5): Shaders on == off (0.00 apart), mean colour within 0.7-1.5 of 255 of the client (heed_interior_look, client_render/heed.rs; captures heed_<case>.png + heed_<case>_pc.txt kept so MG_CLIENT_CAPTURED=1 reproduces the camera).
- The one real on/off gap: a custom pair whose FS defines NORMAL_MAP 1 (Heed's fhss_skintile == fslit_nm with KEYHOLING 0) is lit by the game with the _nm lighting (no softening past the terminator, surface-normal clamp) regardless of render hint; Moonglow's WGSL keyed maps2.w on the render hint (normal_variant). Now Slots::lit_normal_mapped reads the FS's #define NORMAL_MAP; normal_variant stays as is because it also decides tangents on the game path. Gap was 0.2-0.8/255, dark corners ~10% brighter with Shaders off.
## Tried and ruled out
- Uniforms, colour space, sun vs moon, fog: all match the client (light_uniforms_match_the_client covers the uniforms); his pair alone on stock textures (MG_HEED_SHADERS) gives only the small _nm gap, not his 'dark and muddy'. Needs his module (wall/floor placeables, MTRs with maps, haks).
- Whole-picture pixel difference (11-19/255) is texture grain a sub-pixel apart, growing with brightness: assert on region mean colours, not on it.
## Watch out
- Desktop Xwayland holds :0 AND :1 here; run-client.sh defaults to :1. Use your own headless display (MOONGLOW_ORACLE=<scratch> headless.sh start gave :2) and pass DISPLAY explicitly.
- MG_CLIENT_CAPTURED alone loses the player's stood position (no client log): keep it beside the capture.
- Client defaults (post FX) make this interior more saturated/warmer (S 0.50 vs 0.43), not darker.
