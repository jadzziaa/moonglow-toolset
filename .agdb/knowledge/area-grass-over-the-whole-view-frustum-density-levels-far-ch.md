---
type: Decision
title: "Area grass over the whole view: frustum, density levels, far chunks"
tags: []
status: stable
generated: { by: trail, at: 2026-10-23T00:00:00Z }
touches: [crates/mg-area/src/grass_view.rs, crates/mg-area/src/grass.rs, crates/mg-area/src/scene.rs, crates/mg-render/src/model.rs, crates/mg-ui/src/area_view.rs, crates/mg-ui/tests/frame_perf.rs, crates/mg-ui/tests/app.rs]
created: 2026-10-23
author: August
description: "Grass is now selected by camera frustum (`mg_area::Sight`) with distance-based density levels (`grass::LEVELS`) and far chunks merged into one mesh built on a worker thread, replacing the old 30 m radius around the orbit target."
asks:
  - "Why was grass only showing around the orbit target, and how was that fixed?"
  - "How does grass density drop off with distance from the camera, and why not use a fixed fraction?"
  - "Why are far grass chunks built on a worker thread instead of on the UI thread per frame?"
  - "How should I judge grass coverage or benchmark it in frame_perf without misleading numbers from high views or the Shaders switch?"
---
## Decided
- Grass is chosen by the camera's frustum (`mg_area::Sight`, `grass_chosen` in `grass_view.rs`), not a radius: the old 30 m reach around the orbit target is what the owner saw as grass only around the target.
- Levels are tufts per square meter at most by distance from the eye (`grass::LEVELS`: all within 45 m, 10 / 5 / 2.5 past 45 / 90 / 180 m), so the tilesets' density 5 is whole out to 180 m. A fixed share (1/2, 1/4, 1/8) made a field seen from 150 m look bare next to the old full-density patch.
- A tile model's levels are prefixes of its tufts (shuffled by a sequence of their own, so the tufts themselves don't move) drawn from the same buffers: `GpuModel::first_indices`. Nothing extra uploaded.
- Far chunks (4 by 4 tiles, all at the last level) merged into one mesh: 1,000 grass instances cost ~1.6 ms of CPU encoding on a 32x32 field from high up, 64 chunks almost nothing. Built on a worker thread (`ChunkWorker`, wgpu Device/Queue are Send); until ready, the chunk's tiles are drawn singly with identical tufts, so nothing visibly fills in.
- A chunk keeps the `Arc<TileGrass>` it was made of and compares with `Arc::ptr_eq` (comparing raw addresses risks a freed address reused).

## Tried and ruled out
- Chunks built a few per frame on the UI thread with a time budget: one chunk is 0.6 to 1 ms (mostly `GpuModel::new`), and the high view (5.9 ms without grass) spiked to 8 ms while they were made.
- Starting the levels at 30 m: saves GPU only with the Shaders switch off at density 20, which is over budget near the camera anyway, and visibly thins the grass at the target in a low view.

## Watch out
- From high up (420 m) grass changes next to nothing in the picture, before or after: upright 1 m quads in the ground's colors. Judge coverage in low or halfway views.
- The Shaders switch is on by default: frame_perf's grass views must wait for `game_pending == 0`, and its WGSL-only numbers (area_parts with `false`) are much heavier for grass than the game's fsgrass (density 20 across: 11.9 vs 4.6 ms GPU).
- FRAME_SHOTS skipped a view's `each` closure, so a shot showed the previous camera; the grass test now sets the view before shooting.
