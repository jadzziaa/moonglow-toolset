---
title: "Gallery pictures read apart: what made a picture's frame slow"
author: August
date: 2026-10-29
branch: worktree-agent-a1fcbc27377a85d15
cost: { minutes: 2022, tokens: 13913029 }
touches:
  - crates/mg-ui/src/model_view.rs
  - crates/mg-ui/tests/frame_perf.rs
---

## Decided
A gallery picture (model_view.rs make_thumbnail) is made apart only with Moonglow::background_jobs on (the app; frame_perf sets it around the galleries; tests that make pictures with it off draw in the frame, as before). Models: PictureReader (2 threads) runs Composed::new with a Gpu clone from a snapshot of the module's MDLs (Arc per revision) and the game's resman; textures: Renderer::textures_apart + read_textures_apart(textures_from(game)); game programs: game_background. A picture is kept only when drawn.textures_pending == 0 and drawn.game_pending == 0 (else Err(Waiting), retried next frame). Jobs hold Arc<GameData>: Thumbnails::let_go is called from Moonglow::release_game_data.
## Watch out
After models and textures moved off the thread, the slowest pictures (tnp_balc_a02 13-27 ms) were still slow: the time was game-shader pipelines compiled in the frame (game_background false for pictures), not reads. Instrumenting draw_scene's sections (instance loop vs surfaces()) found it; surfaces() itself was microseconds. frame_perf numbers swing with machine load (other worktrees' builds): compare modes in turn, several rounds.
## Tried and ruled out
Counting frames with background_jobs on but pictures in the frame showed background_jobs alone costs nothing; a 2 ms base frame seen once was load.
