---
type: Decision
title: "The shadow plan's receiver box: why b4da2984 slowed the grass field's frame"
description: "The sun's shadow plan merged each receiving mesh's box through box_of, about 10,000 a frame, and the frame waited for its thread; fixed with plain min/max and a looped box_of."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T09:53:29Z }
touches: ["crates/mg-render/src/shadow.rs:plan", "crates/mg-render/src/reach.rs:box_of", crates/mg-render/src/renderer.rs]
asks:
  - "Why did the grass field's window frame get slower after the box_of change (R20)?"
  - "The area frame waits at the shadow plan's join: where is the time?"
  - Can box_of be used to merge boxes in a per-mesh loop?
  - How to time the shadow plan without perf?
fingerprint: v2-34fb9ce190d777a3
created: 2026-11-26
author: augustaugur
---
## Found
The grass field close up (frame_perf `grass_frames_at_144`, "field, close, no grass") lost 0.2 to 0.3 ms of window frame at b4da2984 (R20, `box_of` in one place). The hot site was `shadow::plan`: for each receiving mesh (about 10,000 a frame in a 32 by 32 area) it called `box_of` over the receivers' box so far chained with the mesh's box (an `Option` iterator chain, a filter and a fold). The plan runs on a thread beside the meshes' draws in `draw_scene` (`PLAN_APART` instances or more), so its 0.31 ms against the draws' 0.18 showed up as a wait at the join, not as work on the main thread.

## Fixed
The receivers' box (and `light_plan`'s box of a light's shadow parts) is taken with plain `min`/`max` as it goes, since each box is already checked `somewhere` before. `box_of` is a loop with an `is_finite` branch to each point. Plan's instance loop 0.29 -> 0.065 ms; join wait 0.18 -> 0.

## How it was found
`perf` is not installed. Temporary `AtomicU64` timers around `light_plan`, the spawned `plan`, the draws loop and the join, printed every 20 frames; then variant binaries copied out of the target dir and run alternated (`FRAME_VIEW="field, close, no grass"`).

## Watch out
- A per-frame cost on the shadow-plan thread is hidden until it exceeds the draws loop; time the join, not just the frame.
- Don't fold a running box through `box_of` per element: it is for a list of points, not for merging boxes in a hot loop.
