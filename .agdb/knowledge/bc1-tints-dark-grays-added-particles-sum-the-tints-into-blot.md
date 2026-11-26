---
type: Gotcha
title: "BC1 tints dark grays; added particles sum the tints into blotches: particle and additive pictures stay RGBA8"
description: "Why particles' pictures and TXI 'blending additive' textures are not compressed as read: fxpa_lensflare's gray rim, tinted by BC1's 565, drew green and magenta blotches over area026's floor."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T01:21:11Z }
touches: ["crates/mg-render/src/renderer.rs:Role"]
asks:
  - Why are particle textures not compressed to BC?
  - Where do green or magenta blotches around a glow come from?
  - Does the best effort or BC3 fix BC1 tint on dark grays?
fingerprint: v2-50731573d5a05586
created: 2026-11-26
author: augustaugur
---
Found 2026-11-25 (branch texture-memory, after the rebase onto 03340a65).
- Seen: area026 close shot, green/magenta blotches on the floor around a placeable's glow with compression on; deterministic (runs of one build match bit for bit in the crop).
- The glow: a Lighten particle emitter, picture fxpa_lensflare (64x64 TGA, pure grays on black, BC1). Found by skipping compression by name prefix (fx, then single names) and diffing crops.
- Best effort and BC3 do NOT help (BC3's color block is BC1's; 565 can't hold dark grays: 5 bits R/B, 6 G). Additive blending of many particles sums the per-texel tint.
- Fix: Role::Particle (particle reads, read before material_group so the cache holds it as stored) and Role::compressed(blending) refusing Blending::Additive. Cost: +0.2-0.3 MiB a stock area; opening unchanged.
- Caveat: a texture cached first as a mesh's Picture stays compressed for a particle sharing its name (rare).
- PLT colored copies (pmh0_chest*: max errors 82-167, mean up to 7.5 in BC3) were then shown blurred at layer edges (look_plt_creatures, a BC device: up to 72 of 255, trim and rivets dulled at 4x) and are kept as RGBA8 too: Role::compressed(blending, plt), plt = a name in colors. Cost: moonseabattle 156.7 MiB textures vs 45.2 with PLTs compressed. The UI tests' own GPU has no BC: a look test must make its own device (frame_perf's render_state pattern) or nothing is compressed.
