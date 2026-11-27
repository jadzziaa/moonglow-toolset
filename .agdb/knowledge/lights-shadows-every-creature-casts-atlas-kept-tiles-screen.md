---
type: Fact
title: "Lights' shadows: every creature casts (atlas, kept tiles, screen squares); the client casts from one light"
description: "How the lights' shadow cap of 16 was lifted (Nov 2026), what the client showed (one light, the one nearest the player), how it was timed, and what is left (the one-light ruling, High's volume fill)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T01:56:05Z }
touches: [crates/mg-render/src/shadow.rs, crates/mg-corpus-tests/tests/client_render/batch6.rs, crates/mg-ui/tests/frame_perf.rs]
asks:
  - Why do only some lamps cast creature shadows in the client?
  - "How many creatures can cast a light's shadow in Moonglow, and how is the atlas laid out?"
  - Why does frame_perf draw every light map again each frame?
  - "How was the light shadows' change timed against the old 16-creature path?"
fingerprint: v2-6112feb8cbac84bf
created: 2026-11-27
author: augustaugur
---
## Decided (26 Nov 2026)
- Every creature in a casting light's reach casts (crates/mg-render/src/shadow.rs): one Depth32Float atlas of square tiles, 512 texels up to 64 maps (4096 squared at 8 across), then smaller tiles (256 / 128 / 64), MAX_LIGHT_SHADOWS = 4096 (64 MiB at most). Grown only when it must (LightMaps::hold, a new id that rebuilds the darken bind group).
- Tiles are kept (HeldTile: light matrix, caster keys, bones relative to the creature's first) and drawn again only when that changes. One pass, a viewport and scissor per tile, a stale tile cleared by a depth-only triangle; the whole atlas with LoadOp::Clear when every tile is stale, and `held` reset then (an advisor-found bug: tiles past the plan's count still counted as held).
- Creatures stay in scene order so tiles keep their place; sorted by distance to the camera's target only past 4096.
- The darkening reads per 32-pixel screen square the maps whose light-view box reaches it (CPU counting sort, storage buffer), the box cut at the receivers' lowest height (receivers_floor); a box partly behind the eye goes into every square. Early exits: a >= cap, or with one casting light, shade >= 1. These took 500 creatures close up from 6.5 to 2.8 ms.
## Client findings (batch6_light_shadow_ring, MG_B6_CASES ring|lamps)
- All 24 commoners in a ring round a lamp cast. Only ONE light casts in the whole view: the lamp nearest the player (not the camera's eye; checked by moving the lamps with MG_LAMPS_DY=-3). Every creature in its reach casts from it, ogres by other lamps too, and lamp posts. max-casting-lights 1 and 3 change nothing. Moonglow still casts each creature from its own nearest light: waits on the owner's ruling (deferred "Shadows").
- The client's shadow by a lamp is darker (0.39-0.51 vs 0.14-0.19 in the six-lamp room), not chased.
## Watch out
- frame_perf's creatures animate, so their bones change and every map is drawn again each frame (its line prints "N of their maps drawn again"); kept tiles help only at rest.
- Timing the 16-cap path in process (hooks MG_VAR=cap16 via target/ab/hooks.py, never committed) on alternate frames thrashes tiles and GPU clocks in big scenes; trust the A/A round for absolute figures there.
- Sixteen lamps of 500: close-up time grows as frame_perf's camera turns (2.6 to 4.9 ms), so its median is far above its tenth. That is the view, not thrash.
- High (stencil volumes) costs about 0.1 ms per casting creature close up (100: 11.6 ms): each volume reaches the light's radius. Left waiting on the same ruling.
- Plain FRAME_PARTS takes one part's name; '|' lists only work with the hooks' FRAME_PARTS_ONLY.
