---
type: Decision
title: "The area view's grid fades in the fog to a least; the opening view's target on the ground; the watch at ground level"
description: "Why the grid's lines fade with the fog down to GRID_LEAST_IN_FOG (0.3), why Orbit::overview_over puts the opening target on the ground, why the watch expects tiles at their walkmesh level, why frame_perf's area084 rows moved and what the walkmesh overlay cache does and does not fix (M5 V3 and its fix round)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T23:30:00Z }
touches: [crates/mg-render/src/line.wgsl, "crates/mg-render/src/renderer.rs:GRID_LEAST_IN_FOG", "crates/mg-ui/src/area_view.rs:overview_over", "crates/mg-ui/src/area_view.rs:after_drawing", "crates/mg-area/src/walk.rs:level_at", "crates/mg-ui/src/area_view.rs:walkmesh_overlay", "crates/mg-ui/src/area_view.rs:sound_ranges_of", crates/mg-ui/tests/frame_perf.rs]
asks:
  - why are the grid lines faint in the fog
  - why does the area open with its camera target above zero
  - "why did frame_perf's area084 rows get slower after V3"
  - why are the all selected walkmesh frame_perf rows over budget
  - "does the game's fog use radial distance or depth"
fingerprint: v2-d6fb0247c2392bff
created: 2026-11-21
author: augustaugur
---
M5 V3 (2026-11-21), settled by pictures (Medieval City 2, 16 by 16, Fog on) and the owner's rulings ("For fog: B. For camera: 1.").

- The grid is red only because of the pitch: `grid_lines` blends faint white (pitch >= 0.6 rad) to red (<= 0.2).
- Before V3, `line.wgsl` never fogged, and that predates develop's fog change (6b6f5d51, 8f329f3b only moved the fog's start nearer for views nearer than 20 m). Far grid lines stood bright past the fog's end, as if hung in the sky.
- Ruling B, built: the grid's fragment (`fs_main`) multiplies its alpha by mix(1, grid_least, f), where f is the lit shader's fog factor (view depth passed from the vertex shader).
  - `grid_least` is a WGSL `const`, equal to `mg_render::GRID_LEAST_IN_FOG` (0.3), held by `the_grid_s_least_in_the_fog_is_the_renderer_s`.
  - It is not a pipeline `override`: the HLSL and GLSL translation tests over `wgsl_sources()` don't process overrides.
  - Candidates 0.15, 0.3 and 0.5 were pictured. At 0.15 the faint white grid (high pitch) vanishes in the fog; at 0.5 the red reads strong again.
- Sound Ranges (line copies) take `fs_plain`, unfogged: they are an audio aid. The egui overlays (selection, walkmesh, markers, outlines, start ring) are unchanged, as ruled. Fog off is pixel-identical. Test: render.rs `the_grid_fades_in_the_fog_down_to_its_least`.
- Medieval City 2's ground is 5 m above its tiles. `Orbit::overview` put the target at z 0, and tilt and zoom never change target.z, so the eye went under the ground.
  - Ruling 1: `Orbit::overview_over(model, ground)` takes the ground's level at the area's middle (`Ground::level_at`, else the nearest tile with a level, else 0). It is used when a view opens (`show`) and by View the Whole Area (5).
  - Go to Start Location, Find Instance and the tree's go-to (via `area_focus`, the object's own height), the minimap (ground height) and Alt+middle focus (walkmesh) already used the ground. Restored cameras stay as saved. Panning and zooming are unchanged (options 2 and 3 not built).
- The watch expected tiles at their origins (z 0) and, with the eye under the ground, raised a false recorded "stopped drawing" notice. `Watching::after_drawing` now lifts each tile to `Ground::level_at`.
- The game's fog is view depth, not radial (`inc_fog.shd:47`: `GetFog(-vPosView.z)`), as `line.wgsl` and `shader.wgsl` have it: no difference at the picture's sides (review C's V3-2, settled).
- frame_perf: the camera fix moved `sound_ranges_frames_at_144`'s area084 rows (ground about 34 m up; before V3 the close, turning and wheel views looked from under it and drew little). Of 28 rows over: 4 known, 18 wheel rows of the views after "close" (zooming out over all of area084: deferred "The largest areas in full view", now named one by one in `KNOWN_OVER`), 6 "all selected, walkmesh". The owner ruled the new rows the baseline (`target/m5/V3/frame_perf-baseline.log`, taken with the owner's game client running: to be retaken at the release gates).
- The walkmesh overlays keep their mesh (`OverlayKey`: camera matrix and near, rect, `overlay_generation` counted up in `show`/`reload`; the objects' also selection, show flags, baked faces). Test: bake.rs `the_walkmesh_overlays_are_kept_until_what_they_show_changes`.
  - Gotcha: this does not help frame_perf's "all selected, walkmesh" rows. Their closure turns the yaw by 0.01 every frame, idle included. With the overlays and the selection both off they still take about 10 ms (the whole area from 120 m); the overlays are about 2 ms. Review C's attribution to `walkmesh_overlay` was wrong; the cache helps only a camera at rest. By the owner's ruling the 6 rows are named in `KNOWN_OVER` under the same deferred entry.
- `sound_ranges_of` keeps its list by (workspace revision, selection); `sound_range_shapes` binary-searches it (GIT order) instead of building a map each frame.
