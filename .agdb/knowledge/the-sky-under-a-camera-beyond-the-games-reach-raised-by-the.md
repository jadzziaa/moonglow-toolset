---
type: Decision
title: "The sky under a camera beyond the game's reach: raised by the height past the game's farthest camera, checked in the client"
description: "How sky_place places the skybox for a camera (at ground level for every game camera, 25 m from its target over any tile; raised for cameras farther off), the khaki bands it cured, and what the client comparison at the game's farthest camera showed, including a misleading 32 m fit."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T14:15:32Z }
touches: ["crates/mg-render/src/renderer.rs:sky_place", "crates/mg-render/src/shadow.rs:camera_fade_point", "crates/mg-ui/tests/app/drawing.rs:the_sky_under_a_camera_higher_than_the_game_s", "crates/mg-corpus-tests/tests/client_render.rs:sky_under_the_game_s_farthest_camera_look"]
asks:
  - Why are there khaki or fog-colored bands at the sides of a wide area view seen from high up?
  - "Where is the skybox placed relative to the camera, and why not at ground level for every camera?"
  - Does the sky move for a game camera over a raised tile or pitched 30 degrees?
  - "How far is the game client's camera really from the player at distance 25: a fit says 32 m, is that right?"
supersedes: [the-sky-under-a-camera-higher-than-the-games-khaki-bands-at]
fingerprint: v2-e733fb78ca3ab223
created: 2026-11-14
---
## Cause (M3 T15, seen in T14's shell_width_2559 pictures)
- The skybox was drawn around the camera at ground level (z 0) since 2026-10-01 (commit 25135a28). An area seen whole puts the view's camera far higher than the game's (Chapter1 map_m1q1a: 151 m).
- sky_001 is a box about 460 m to each side, from 458 m under its ground to 310 m over it; skyfade1 is a bowl 390 m deep under the horizon, drawn in the area's fog color (map_m1q1a's sun fog is the khaki). A 2559 by 800 view's sides look 57 degrees off straight down: from 151 m those rays met the fade's upper wall, all fog color.

## Decided (T15's review, superseding T15's rule)
- The game's 25 m (CameraMaxDist, `shadow::GAME_CAMERA_REACH`) is the camera's reach from its focus, not a height. T15 had raised the sky by `eye.z - 25` from world z 0: a game camera over a level-2 tile (10 m) got its sky moved.
- `sky_place(camera)`: x and y follow the eye; z = max(0, (distance - REACH) * up), `up` the sine of the eye's elevation over its target (the same height rule as `camera_fade_point`). Exactly z 0 for every camera within the reach, whatever the target's height or the pitch; beyond it the eye stands as high over the sky's ground as the game's farthest camera at that pitch would. The area view's cameras are orbits round a target on the ground, so the target distance is meaningful there.
- What remains at the very sides of a wide top-down view (44% of the fog color's sum, the fade's lower band over the sky's forest) is what the game's farthest camera sees at that angle. Pitched about 30 degrees down from high up, the view's top shows the fade's wall in fog color, not clouds: the client shows exactly that at its farthest camera (sky_under_the_game_s_farthest_camera_look, `low`: its top band is the fog color).

## The client comparison (2026-11-14)
- `sky_under_the_game_s_farthest_camera_look`: ttr01 4x4, Grass_Clear sky, magenta fog, SetCameraFacing(90, 25, 1) and (90, 25, 60). Moonglow's camera fitted on the ground pixels alone: 23 m pitch 2, and 23.5 m pitch 60 (pitch from straight down, fov 48, focus 1.25). Both within the reach: Moonglow's pictures there are pixel-identical before and after the fix; the sky beside the field is 7.87 of 255 from the client's at the fitted 23.5/60 (the fade's rows up to about 20 off), 6.05 at 23.5/59 (rows within about 5), the ground fitting about as well: the fit resolves only about a degree and a meter or two (fov 48 and focus 1.25 assumed from the reference scenes, a field of grass).
- Trap: a first fit over the whole picture (sky included) said 32 m at pitch 58, which would have put the game's camera beyond the reach. It was the sky being fitted: the same picture fitted 23/60 once the sky had moved. Fit on the ground only, half-degree steps.
- Seen, not about the sky's place: the client tints the last meters of ground at the field's far edge toward the fog color (fog amount 0): its own fog, always on from 30 m before the clip distance (docs/research/notes_tile_border.md); the test draws Moonglow with Fog off.
- The client's window on the off-screen display is 1024 by 768; no wide client picture is possible there.

## Tests
- Unit: `renderer::tests::the_sky_stands_under_the_camera_as_the_game_s` (game cameras over a 10 m tile at 30 and 89 degrees: z 0; far cameras: the game's height over the sky).
- Picture: `drawing::the_sky_under_a_camera_higher_than_the_game_s`: the side strips between 39% and 50% of the fog color's sum (the sky at the camera gave 27%, 12 m under it 35%, 100 m under it 99%).
