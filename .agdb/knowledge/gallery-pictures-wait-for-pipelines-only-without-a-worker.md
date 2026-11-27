---
type: Fact
title: Gallery pictures wait for pipelines only without a worker
description: "Timed 2026-11-27: with background_jobs on (the app) Gallery pictures are read apart and wait for the worker's pipelines, so Shaders on costs the same as off; only a window without a worker draws them in the frame (one 16-30 ms frame)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T09:05:58Z }
touches: [crates/mg-ui/src/model_view.rs]
asks:
  - "Do the game's shaders make the Gallery's thumbnails slow?"
  - "Why does a test's Gallery show a slow first frame that the app doesn't?"
  - "Does the Shaders switch change the Gallery's scrolling frames?"
fingerprint: v2-b26f7a41f0a36bdd
created: 2026-11-27
author: augustaugur
---
apps/moonglow sets background_jobs = true; make_thumbnail passes apart = true and render_picture sets game_background, so pictures wait through Err(Waiting) while the worker makes pipelines (model_view.rs ~2599, ~3229). draw_thumbnail and draw_group_thumbnail run in the frame only with background_jobs off (tests): there the first pictures take one 16-30 ms frame with Shaders on. frames_at_144, five rounds alternated, fresh MOONGLOW_TEST_OUTPUT per run (kept-pipelines list and shader cache), plus a warm pair: Gallery wheel p99 1-4 ms on and off alike; the deferred entry's 10 and 49 ms not reproduced. Knobs used, uncommitted: FRAME_SHADERS=off, FRAME_GALLERY_FIRST=sync|apart. Supersedes the older note that the slowest Gallery pictures were pipelines compiled in the frame (before pictures were read apart).
