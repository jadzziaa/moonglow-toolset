---
type: Decision
title: "The sky under a camera higher than the game's: khaki bands at the sides of a wide area view"
description: "Why the skybox is raised with a view's camera above 25 m (sky_place), what the khaki bands at the sides of a 2559 by 800 area view were, and what of them is the game's own look."
tags: []
status: deprecated
generated: { by: agdb/0.2.0, at: 2026-11-14T12:55:43Z }
touches: ["crates/mg-render/src/renderer.rs:sky_place", "crates/mg-ui/tests/app/drawing.rs:the_sky_under_a_camera_higher_than_the_game_s"]
asks:
  - Why are there khaki or fog-colored bands at the left and right of the area view in a very wide window?
  - "Where is the skybox placed relative to the camera, and why not at ground level for every camera?"
  - "The sides of a wide top-down area view still show a faint dark yellow gradient: is that a bug?"
  - What is GAME_CAMERA_HIGHEST and where does 25 m come from?
fingerprint: v2-4220e29f33996adf
created: 2026-11-14
---
## Cause (M3 T15, seen in T14's shell_width_2559 pictures)
- Not the shell's: the skybox was drawn around the camera at ground level since 2026-10-01 (commit 25135a28, checked against the client at the game's camera heights). An area seen whole puts the area view's camera far higher than the game's (Chapter1 map_m1q1a: 151 m; a 32 by 32 area over the skybox's top, about 310 m over its ground).
- sky_001 is a box about 460 m to each side, from 458 m under its ground to 310 m over it; skyfade1 is a bowl 390 m deep under the horizon, drawn in the area's fog color (map_m1q1a's sun fog 0x988f67 is the khaki). Under the fade the sky's lower faces show a forest seen from above.
- A 2559 by 800 window's area view is about 4.2 times as wide as high (fov_y 40°): its sides look 57° off straight down. From 151 m those rays met the fade's upper wall, all fog color.

## Decided
- `sky_place(eye)`: the sky follows the camera's x and y, at ground level up to the game's highest camera (`GAME_CAMERA_HIGHEST`, 25 m: CameraMaxDist, 1 to 25 since 1.69), and above that raised to stay 25 m under the camera. Identical to before for every game camera; from high up the sky looks as from the game's highest.
- What remains at the very sides (about (70, 66, 47), the fade's lower band over the sky's forest) is what the game's camera sees 57° off straight down. Checked with the fade switched off (the forest then fills the sides). Not removed: removing it would draw a sky the game doesn't.

## Tests
- Unit: `renderer::tests::the_sky_stands_under_the_camera_as_the_game_s`.
- Picture: `drawing::the_sky_under_a_camera_higher_than_the_game_s` reads the side strips of the 2559 by 800 view, both shader paths: under 3/5 of the fog color's sum (was the whole fog color), over 6 (the sky drawn).
