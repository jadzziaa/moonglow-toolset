---
title: Creature and light shadows behind the Shadows button
author: August
date: 2026-10-28
branch: worktree-agent-ab2abef6e375df47a
touches:
  - crates/mg-area/src/scene.rs
  - crates/mg-corpus-tests/tests/client_render/shadows.rs
  - crates/mg-preview/src/compose.rs
  - crates/mg-render/src/anim.rs
  - crates/mg-render/src/reach.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/scene.rs
  - crates/mg-render/src/shadow.rs
  - crates/mg-render/tests/render.rs
  - crates/mg-ui/src/area_view.rs
---

## Decided
- The client's Creature Shadow Detail (graphics.shadows.creatures.mode: 2 Best, 1 Fast, 0 Off) governs every dynamic object: at Best an ogre and the dynamic armoires cast the sun's shadow; at Fast/Off none of them do, while tiles and static placeables still cast (environment shadows). Moonglow's menu maps Sun and Moon Shadows to tiles + static placeables and Creature Shadows to creatures + dynamic placeables + doors.
- Light shadows in the client (measured on tic01 with lamp posts, placeables.2da row 73): each creature casts ONE shadow, from its nearest light with `shadow 1` and radius >= 10 (fx_placeable01's; fx_flame01's radius 7 casts none), lifted by the light's `verticaldisplacement` (3 m for placeable lights; without it the shadow was twice as long), only when the area has no sun/moon shadows at that time; dynamic armoires cast none from lamps. Darkness = opacity x plane fade (0.14 at 30, 0.49 at 100), not min() as the sun's. max-casting-lights 0/1/3 changed nothing with one creature.
- Moonglow: per creature (max 16, nearest the camera target) a 512² perspective map from its light into a texture array layer; one darkening pass loops the layers. Skinned casters use a skinned depth pipeline reading the caster's bones from a storage buffer (bones computed like the main draw loop: to_skin * pose[b] * inverse_bind).
- Light Shadows default on: 16 creatures cost +0.6 ms at 3440x1440 4x MSAA; creatures in the sun map < 0.1 ms.
## Tried and ruled out
- Cube shadow maps for the lights: not needed, a creature's shadow from one light fits a perspective cone around it.
- Measuring the lamp shadow against the day_off capture: day_off has the lamp shadow; the baseline is the same lighting with Creature Shadow Detail Off.
## Watch out
- egui's Popup with CloseOnClickOutside closed a held-open menu at the hold's release (a click on the button, outside the popup): `held_menu` ignores clicks on the release frame. Walkmesh… had the same bug.
- The sun map and the light maps need separate caster/param buffers: queue.write_buffer runs before the submitted commands, so a shared buffer gives both passes the last write.
