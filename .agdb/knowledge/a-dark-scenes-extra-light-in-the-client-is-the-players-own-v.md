---
type: Decision
title: "A dark scene's extra light in the client is the player's own vision light"
description: "In a dim area the client adds one ambient-only light, the player's Low_Light_Vision (fx_light_clr), which Moonglow's area view (no player) lacks; the renderer is right, the readings give Moonglow's side the light."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T04:12:51Z }
touches: ["crates/mg-corpus-tests/tests/client_render/readings.rs:player_light", crates/mg-corpus-tests/tests/client_render/chrome_reflection.rs]
asks:
  - "Why are the client's pictures 4 to 8 times brighter than Moonglow's in a dark area?"
  - "Where is the player's darkvision or low-light vision light, and when is it on?"
  - "How do I read which point lights the client uploads (numLights, lightColor, position)?"
  - "Should Moonglow's area view light the scene as the player's light does?"
fingerprint: v2-93542435e0dee9a0
created: 2026-11-27
author: augustaugur
---
## Decided
- Read 2026-11-26 (89.8193.37) with patched inc_standard paints (chrome_reflection.rs MG_CHROME_PAINT=lights, light0, light0a, light0world, falloff): in a dim area numLights is 1 in the client, 0 in Moonglow. The light: fx_light_clr's Low_Light_Vision (the +TestNewModule player is an elf), ambient only (lightColor.a < 0, -484 = (11 * 2)^2), color lin(0.3 0.3 0.4), 0.64 m ahead of the player along its facing, 2.65 m up.
- On while the area's ambient + diffuse gray levels (bytes) add up to less than 128; out from 128. Invisible player (VFX_DUR_CUTSCENE_INVISIBILITY): no light, which is why the helmet reading's dark tops always agreed.
- Confirmed by prediction: client with the vision light black (override fx_light_clr, MG_CHROME_PLAYER_LIGHT=off) equals Moonglow without (4.0 vs 4.0); Moonglow's scene given readings::player_light equals the client (23.4 vs 23.7), Shaders off and on alike.
- So no renderer change: the editor has no player (settled). The readings give Moonglow's side the light where the client's player is seen. Lighting every area view with a player's light would be the owner's call. (The owner ruled it optional, off by default: since 2026-11-26 the area view's Player's light gives it to the start location; see the note players-light-the-players-vision-light-at-the-start-location.)
## Tried and ruled out
- frontLightModelProductSceneColor floor, lightAreaAmbient, dynamic contrast (disabled), gamma post pass (2.2: no-op), fog, emissive, falloff uniforms: all equal or off.
## Watch out
- In codemode, await each edit(): unawaited edits to one file race and the last write wins.
- Colored area lights: the on/off rule is not read; player_light refuses non-gray light (asserts).
