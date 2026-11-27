---
type: Decision
title: "Player's Light: the player's vision light at the start location, via View::player"
description: "How the area view's Player's Light is built (mg_area::player_light, View::player, scene_hiding), the unread colored-light rule, and why it casts no shadow (the client's vision light casts none)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T06:42:41Z }
touches: [crates/mg-area/src/player_light.rs, "crates/mg-area/src/scene.rs:scene_hiding", "crates/mg-ui/src/area_view.rs:start_shown", crates/mg-ui/tests/app/player_light.rs, "crates/mg-corpus-tests/tests/client_render/batch6.rs:batch6_player_light_shadows"]
asks:
  - "where does the area view add the player's light"
  - why is a dim area brighter in the client than in moonglow
  - "does the player's vision light cast creature shadows"
  - "should fx_light_clr's black shadow light cast in moonglow"
fingerprint: v2-0a9ec9adce026856
created: 2026-11-27
author: augustaugur
---
The client adds the player's Low_Light_Vision light (ambient only, 0.3 0.3 0.4, radius 11 so cutoff 22 m, 0.64 m ahead along facing, 2.65 m up) while ambient+diffuse gray levels sum < 128 (read gray only; docs/findings.md 'The player's own light in a dim area'). Moonglow: mg_area::player_light (dim, light, of) holds the rule; the corpus readings' player_light wraps it and still asserts gray input. mg_area::View::player: Option<(Vec3, f32 radians)> carries it; View::of leaves it None, so only the area view's viewport sets it, from Settings::player_light (written only when true) and area_view::start_shown (Drag::Start, then Adjust Location's previewed_start, else start_on_ground). Because Drawn keys on View, toggling or moving it redraws the kept picture by itself. scene_hiding adds it after lights.clear() for !lit, by l.sky(view.night), so a dark night has it where a bright day doesn't. Colored light (unread): the mean of the three channels' sums < 128, i.e. all six bytes sum < 384; the brightest-channel alternative would put it out on a deep blue night. Another race's vision light isn't read. Tests: mg-area unit (six read pairs), --test app player_light (pixel levels at the start and beyond 22 m; facing compare via cos/sin since atan2 gives -pi for west).
Shadows (2026-11-27, owner had ruled 'player light shadows: yes'): the client's vision light casts NO creature shadow and takes no lamp's place. fx_light_clr's second light AuroraLight01 (black, shadow 1, radius 50) looks like a caster but batch6_player_light_shadows showed nothing at Best or Fast, and with a lamp beside the player the lamp cast every creature's shadow as in a bright area. The old 'player's own light casts' finding (batch6_player_light_at_night) was VFX_DUR_LIGHT_WHITE_20, a light effect, not the vision light. Don't port AuroraLight01 as a casting light: under per-creature-nearest it would steal lamp shadows near the start. Held by the_player_s_light_takes_no_lamp_s_place (mg-render shadow.rs).
