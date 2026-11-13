---
type: Decision
title: "Gallery pictures read apart: what made a picture's frame slow"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [crates/mg-ui/src/model_view.rs, crates/mg-ui/tests/frame_perf.rs]
fingerprint: v2-9bad3a6708936ba3
created: 2026-10-29
author: August
description: "Gallery pictures (make_thumbnail) are built off-thread when Moonglow::background_jobs is on, and the slowest ones stayed slow because game-shader pipelines compiled in the frame, not because of model or texture reads."
asks:
  - "How does make_thumbnail produce gallery pictures in the background, and when is a picture kept versus retried with Err(Waiting)?"
  - "Why are some gallery pictures like tnp_balc_a02 still taking 13-27 ms in the frame after moving model and texture loading to threads?"
  - "How do I get reliable frame_perf numbers when timings jump around between runs?"
  - "Does turning on Moonglow::background_jobs by itself add frame cost if pictures are still drawn in the frame?"
---
## Decided
A gallery picture (model_view.rs make_thumbnail) is made apart only with Moonglow::background_jobs on (the app; frame_perf sets it around the galleries; tests that make pictures with it off draw in the frame, as before). Models: PictureReader (2 threads) runs Composed::new with a Gpu clone from a snapshot of the module's MDLs (Arc per revision) and the game's resman; textures: Renderer::textures_apart + read_textures_apart(textures_from(game)); game programs: game_background. A picture is kept only when drawn.textures_pending == 0 and drawn.game_pending == 0 (else Err(Waiting), retried next frame). Jobs hold Arc<GameData>: Thumbnails::let_go is called from Moonglow::release_game_data.
## Watch out
After models and textures moved off the thread, the slowest pictures (tnp_balc_a02 13-27 ms) were still slow: the time was game-shader pipelines compiled in the frame (game_background false for pictures), not reads. Instrumenting draw_scene's sections (instance loop vs surfaces()) found it; surfaces() itself was microseconds. frame_perf numbers swing with machine load (other worktrees' builds): compare modes in turn, several rounds.
## Tried and ruled out
Counting frames with background_jobs on but pictures in the frame showed background_jobs alone costs nothing; a 2 ms base frame seen once was load.
