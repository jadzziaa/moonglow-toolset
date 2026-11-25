---
type: Decision
title: "Custom Light flicker: colorkey in default, a dynamic placeable"
description: "How Custom Light's Flicker is written (light node colorkey in the model's default animation, row Static 0, instance dynamic), why multiplier is not keyed, how steady lights stay byte-identical, and where Static is set or kept."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T20:39:08Z }
touches: [crates/mg-module/src/lights.rs, crates/mg-ui/src/light_maker.rs, "crates/mg-ui/src/area_view.rs:change_light", "crates/mg-module/src/areas.rs:kept"]
asks:
  - How does Custom Light make a light flicker and why must the placeable be dynamic?
  - "Which light node keys does mg_mdl compile into a binary model's animation?"
  - "Why does the area view's flickering light dim to about 0.45 when the key says 0.7?"
  - Why does Make Placeables Static leave some generated lights dynamic?
fingerprint: v2-250e792dabaaf3c9
created: 2026-11-25
author: augustaugur
---
## Decided (2026-11-25)
- Flicker presets (None, Candle, Torch, Fire, Pulse) write only a light-node `colorkey` (color scaled by a brightness table, last key = first at `length`) in `newanim default` with `transtime 0` and `animroot` = model root, the same shape client_render batch4's `reading_static_placeable_default_animation` measured the client playing on a dynamic placeable (classification character). `multiplierkey` is NOT keyed: the multiplier lengthens reach, not brightness, and keyed multiplier was never measured in the client.
- mg_mdl's ASCII reader takes any `<controller>key N` row list; light controllers it knows (ctrl.rs LIGHT): color 76, radius 88, shadowradius 96, verticaldisplacement 100, multiplier 140. compile() carries them into the binary animation (lights::tests reads them back).
- A flickering light's placeables.2da row is `Static` 0; `lights::row_flickers` (generated model + Static 0) tells one. Every placing goes through `area_view::appearance_item` -> `lights::dynamic_if_flickering` (palette Lights, Add Light Here, gallery drop); Custom Light's own placing also passes `flickers` to `add_light_row` (`lights::make_flickering`), not relying on the table read again. A flickering instance is also Plot 1: a dynamic placeable can be destroyed (fireball's shape takes OBJECT_TYPE_PLACEABLE), a static one can't. Change Light (`area_view::change_light`) sets Appearance and Static in one command: dynamic when flickering, static again only where the old row flickered.
- Make Placeables Static keeps them: `StaticRules::flickering_rows` (non-file field, `with_flickering_rows(table)`), Kept::Flickers counted in `active`; the UI and `mg areas --static-placeables` fill it from the game's placeables.2da.
- Byte identity: the hash text and model text gain the flicker only when it is not None; `a_steady_light_writes_what_it_wrote_before_flicker` pins names and FNV of text and binary taken from the code before.
## Watch out
- The area view takes a light node's color as gamma and linearizes it: a 0.7 key shows as about 0.45 of the linear light (the UI test checks a uniform ratio in 0.4..0.5).
- The Flicker dropdown is found in kittest by value (`get_by_value("None")`), not by label.
- Watched in the client (2026-11-25, `reading_custom_light_flicker`): the generated Torch model flickers placed dynamic (floor red spread 7.5 of 255) and stays steady placed static (0.08).
