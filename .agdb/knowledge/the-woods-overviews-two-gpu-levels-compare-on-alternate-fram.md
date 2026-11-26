---
type: Gotcha
title: "The woods overview's two GPU levels: compare on alternate frames, not run against run"
description: "Why game_shaders_frames_at_144's woods overview GPU time jumps 1-2 ms between runs of the same code, and how to compare builds under it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T11:44:12Z }
touches: [crates/mg-ui/tests/frame_perf.rs, crates/mg-render/src/renderer.rs, docs/research/notes_frames.md]
asks:
  - "Why does frame_perf's woods overview GPU time jump between 6 and 8 ms between runs?"
  - "Did the late see-through draws' order (94db6613) slow the woods overview?"
  - "How do I compare two render builds' GPU time when runs differ by a level?"
  - "Is the woods test's slow GPU level caused by VRAM or GTT pressure?"
fingerprint: v2-f3b69453e7441f3b
created: 2026-11-26
author: augustaugur
---
game_shaders_frames_at_144, woods overview (4x MSAA, Shaders on), lands per run in a slow level (GPU median pinned about 11.2 ms, tenth 6.8-8.0) or a quick one (median 6-8.5, tenth 5.3-6.9); same binary, at load 2 as at 50.
- Six runs sampled read-only (card1/device pp_dpm_sclk, gpu_busy_percent, mem_info_vram_used, mem_info_gtt_used every 0.2 s): the slow runs had the lowest clocks (288, 815 MHz vs 948-1514); GTT flat 0.3-0.4 GiB, VRAM 6.9-7.7 of 16 GiB. The RX 9070 XT's automatic clocks, not memory. Never set a power profile or force a clock to test it.
- 94db6613 (late see-through draws after the others) was first read as +1.8 ms: that was the level. In-process A/B (a renderer flag toggled from frame_perf's loop, even frames base, odd frames variant, tenths printed apart) put every order variant within 0.3 ms; a control (soft parts not drawn, 7.93 vs 4.78) showed it resolves real costs. The order is kept.
- The debug patch for that A/B is untracked at target/retime/lateord-variants.patch (main checkout); never commit it.
- Otherwise: many runs per build, sorted by level, compared within a level.
