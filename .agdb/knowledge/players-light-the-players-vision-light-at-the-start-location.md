---
type: Decision
title: "Player's Light: the player's vision light at the start location, via View::player"
description: "How the area view's Player's Light (off by default, the owner's ruling) is built: mg_area::player_light holds the read rule, View::player carries the start as shown, scene_hiding adds it only with Lighting on, and the colored-light rule is an unread choice (mean of channels)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T05:24:06Z }
touches: [crates/mg-area/src/player_light.rs, "crates/mg-area/src/scene.rs:scene_hiding", "crates/mg-ui/src/area_view.rs:start_shown", crates/mg-ui/tests/app/player_light.rs]
asks:
  - "where does the area view add the player's light"
  - why is a dim area brighter in the client than in moonglow
  - "how does Player's Light decide colored ambient light is dim"
  - does the player light follow the start location while dragging
fingerprint: v2-f7d8db6398a40bac
created: 2026-11-27
author: augustaugur
---
The client adds the player's Low_Light_Vision light (ambient only, 0.3 0.3 0.4, radius 11 so cutoff 22 m, 0.64 m ahead along facing, 2.65 m up) while ambient+diffuse gray levels sum < 128 (read gray only; docs/findings.md 'The player's own light in a dim area'). Moonglow: mg_area::player_light (dim, light, of) holds the rule; the corpus readings' player_light wraps it and still asserts gray input. mg_area::View::player: Option<(Vec3, f32 radians)> carries it; View::of leaves it None, so only the area view's viewport sets it, from Settings::player_light (written only when true) and area_view::start_shown (Drag::Start, then Adjust Location's previewed_start, else start_on_ground). Because Drawn keys on View, toggling or moving it redraws the kept picture by itself. scene_hiding adds it after lights.clear() for !lit, by l.sky(view.night), so a dark night has it where a bright day doesn't. Colored light (unread): the mean of the three channels' sums < 128, i.e. all six bytes sum < 384; the brightest-channel alternative would put it out on a deep blue night. Another race's vision light isn't read. Tests: mg-area unit (six read pairs), --test app player_light (pixel levels at the start and beyond 22 m; facing compare via cos/sin since atan2 gives -pi for west).
