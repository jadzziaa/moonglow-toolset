---
type: Fact
title: "The client's fog start follows its camera's distance (night fog, upstream ask A8)"
description: "Why Moonglow's fog fell short of the client's with a zoomed-in camera (Moonglow Client's night Shard picture): the client's fogStart is 30 - FogAmount - (20 - camera distance), which every earlier reading missed because they all used the entry camera at 20 m."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T13:49:54Z }
touches: ["crates/mg-area/src/scene.rs:fog_from", crates/mg-corpus-tests/tests/client_render/night_fog.rs, "crates/mg-ui/src/area_view.rs:view_fog"]
asks:
  - "Why is Moonglow's fog weaker than the client's at night with a short fog clip distance?"
  - "How does the client's camera distance change the fog start?"
  - "Why are distant things darker and crisper than the game's in a fogged area?"
  - "What does View::camera do in mg-area?"
  - "Does View::camera take the script's camera distance or the fitted eye distance?"
fingerprint: v2-1b94690a256f1e64
created: 2026-11-19
author: augustaugur
---
## Fact
The client sets fogStart = min(30 - FogAmount - (20 - d), FogClipDist - 1), fogEnd = FogClipDist, where d is its camera's distance from the player as SetCameraFacing gives it (20 as a player enters). Pitch, day/night and interior don't matter. Read back 2026-11-19 (night_fog.rs fog_start_follows_the_camera): amount 12 clip 22 -> 3.5 at 5.5 m, 6 at 8, 10 at 12, 18 at 20; amount 0 clip 45 -> 15.5, 22, 30.

## Why it hid
fog_uniforms_match_the_client and every day scene used SetCameraFacing(90, 12, 45) given at entry, which the client ignores: its own camera at 20 m, where the formula reduces to the old 30 - amount. The Shard (camera 5.5 m, amount 12, clip 22) then had fog 18..22 in Moonglow against 3.5..22 in the client: near things clear, then a line where the ground met the fog.

## Tried and ruled out
Fog uniforms at night vs day, moon vs sun amount and colour, interior flag: all identical (18..22 with the entry camera). The shader's fog (inc_fog: planar view depth, linear, gamma space) is the same as Moonglow's: painting fFogFragCoord via a patched inc_fog in the override showed the client's start at ~4 m, then the debug inc_standard read 3.50.

## Watch out
mg_area::fog(l, night) is the fog for the entry camera (20 m); scenes for another camera set View::camera (mg-ui sets it to the orbit distance capped at 20, view_fog still pushes it back beyond 20). Moonglow's camera fit for SetCameraFacing(90, 5.5, 84): 5.0 m, 84 deg, focus 1.25.
- Which distance View::camera takes (settled 2026-11-19, docs on View::camera, fog_from, GAME_CAMERA, Camera::game): the SCRIPT's (SetCameraFacing's d, the client's kept distance), never the fitted eye: 5.5 for the Shard though its eye is 5.0 (5.25 in the client's own fit), 12 for armourer.rs's eye of 11, 20 at entry (eye 19). night_fog.rs and armourer.rs pass the script's. mg-ui has no script distance and passes its orbit's (an eye's) as is: no rule turns an eye into a script distance, so up to about a metre nearer fog, documented on fog_camera. Moonglow Client's mc-scene keeps it on Stage::camera_distance from World::camera, so `mgc picture --distance` moves the eye and not the fog.
