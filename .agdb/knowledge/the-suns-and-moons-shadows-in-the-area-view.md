---
type: Decision
title: "The sun's and moon's shadows in the area view"
tags: []
status: stable
generated: { by: trail, at: 2026-10-28T00:00:00Z }
touches: [docs/PLAN.md, docs/deferred.md, docs/log.md, docs/manual/04-areas.md, docs/research/notes_shaders.md]
fingerprint: v2-cdf7ec3a788d8a8a
created: 2026-10-28
author: August
description: "Documents how the game client renders sun/moon shadows (a faded black multiply over tiles and static placeables) and how Moonglow reproduces them with a depth map, plus caster rules, camera clamps and caching pitfalls."
asks:
  - "Why do ShadowOpacity 60 and 100 look identical in the client's area view?"
  - "How does Moonglow render shadows, and why are receivers drawn before the scene pass is cut?"
  - "My far-away editor camera makes shadows fade out completely; what causes that and how is it fixed?"
  - "Why don't creatures or dynamic placeables like an armoire show shadows, and which objects cast or receive them?"
---
## Decided
- The client's sun shadows (measured in `client_render.rs` `shadows_look`, captures kept as `captures/client/shadows_<tileset>_<lighting>.png`): a flat black multiply of the finished gamma-space picture, alpha = ShadowOpacity/100, capped by `fs_shadowplane`'s fade up the screen (`fFade = 1.1·(P − y/H)/P − 0.1`, P from the camera pitch, its height over its focus and FogClipDist). At the reference camera the fade is 0.47 at mid-screen, so opacity 60 and 100 look the same there. Receivers: tiles and static placeables only (a dynamic armoire isn't darkened, even by its own shadow). Casters: tiles and placeables, static or not. Same direction by night. SunShadows gates day, MoonShadows night.
- Moonglow (`mg_render::shadow`): a 2048² depth map from the area light, its box fitted to what the camera sees of the receivers; receivers are drawn first (sort-key bit 60, the batcher's pass split on (pass, receives), game draws sorted (late, !receives, order)), then the scene pass is cut and a full-screen pass (per sample with MSAA, `@builtin(sample_index)`) lays black over them, then the rest. One path for the game's programs and Moonglow's own shader (the game's shaders are on by default, so forward sampling in shader.wgsl alone would have shown nothing to most builders).
- An editor camera's height over its focus is clamped to the game camera's 25 m reach in the fade (SetCameraFacing's range), else a far camera fades its shadows away.
- `render 0`, `shadow 1` meshes go to `GpuModel::shadow_only`, not `meshes` (everything iterating `meshes`, and Moonglow Viewer, would see invisible geometry).
- Creatures cast none for now (skinned meshes left out; their rigid parts alone would cast floating heads).
## Tried and ruled out
- Reading the "cap" at opacity 60/100 as an engine clamp of the opacity: it is the plane's fade.
- tic01 for the numbers: its walls and pillars throw long shadows over the floor; ttr01 gives clean ones (tic01 still matches by eye).
## Watch out
- Every `Draw` and `GameDraw` has `receives`; it is set only when the frame has a shadow plan, so frames without shadows keep their old order.
- The shadow map isn't drawn again while the light box and the casters' transforms are unchanged (`ShadowMaps::holds`).
- egui's default fonts have "◑" but not "◐" (nor 🌙: the Night switch's glyph comes from somewhere else or shows as a box).
