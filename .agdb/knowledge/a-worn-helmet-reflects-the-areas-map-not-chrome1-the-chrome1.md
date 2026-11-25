---
type: Decision
title: "A worn helmet reflects the area's map, not chrome1 (the chrome1 reading, 2026-11-25)"
description: "Read in the client: chrome1 on wholly reflecting placeable boxes matches Moonglow (upload, coordinates, normals, texel); a worn helmet reflects the area's EnvMap (tic01__ref01) in the client but Moonglow binds chrome1 (creature.rs p.env_map), the cause of the black helmet tops."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T18:24:44Z }
touches: [crates/mg-corpus-tests/tests/client_render/chrome_reflection.rs, crates/mg-preview/src/creature.rs]
asks:
  - Why are the helmet tops black in Moonglow where the client draws them gray?
  - Which environment map does the client bind to a worn helmet?
  - "How do I tell which env map a client mesh samples (gradient texture, paint probe)?"
  - "Is chrome1's orientation or sampler wrong in Moonglow?"
fingerprint: v2-d7ab18e7406bfd75
created: 2026-11-25
author: augustaugur
---
## Decided
- Reading: `chrome_reflection::chrome1_reflection_look` (boxes over placeables 428 and 447, Reflection default) and `chrome1_helmet_reflection_look` (worn helm_241 box). Placeable boxes: client = Moonglow within a few of 255 under sun and nosun; paint probes (uv, normal, flags) agree; a gradient override of chrome1.tga shows the same texel. So chrome1's upload, flip, sampler, coordinates are NOT the difference.
- A worn helmet: client's raw env sample is warm gray (tic01__ref01, the tileset EnvMap), Moonglow's is blue gray chrome1, at identical coordinates and normals; overriding tic01__ref01.tga with the gradient makes only the client's helmets show it. Cause: crates/mg-preview/src/creature.rs:291 `p.env_map = Some("chrome1".into())` for the worn helmet; the client uses the creature's default = the area's map. Fixed 2026-11-25: `Some("default".into())` there and in item.rs's `part`; after, the helmet's gradient reading matches the client on the sides to 1 or 2 of 255.
- Held items (MG_CHROME_HELD=shield|weapon): the client reflects the area's map there too (gradient on tic01__ref01 shows), Moonglow had chrome1; fixed the same way. A placeable's `default` also keeps chrome1 over a TXI-named map (renderer.rs, tested in render.rs).
## Tried and ruled out
- The deferred entry's guess (chrome1 reflected differently by a wholly reflecting surface): boxes with chrome1 match.
## Watch out
- Dark lighting (ambient 0x080808, no diffuse): client boxes 4 to 8 times brighter than Moonglow (cause not found; frontLightModelProductSceneColor and lightAreaAmbient equal). Placeable TXI envmaptexture named map is ignored by the client where Reflection is default; Moonglow takes it.
- Face windows from Moonglow's camera only work for the fitted cameras (entry, 12/45 turned); the part textures' 5 m level camera is unfitted and put the windows off the helmet in the client.
- The display lock: other sessions restart the headless display; hold the lock around the whole batch (tools/locks.sh hold display -- ...) and start the display inside it.
