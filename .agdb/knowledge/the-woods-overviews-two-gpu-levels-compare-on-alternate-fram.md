---
type: Gotcha
title: "The woods overview's two GPU levels: the test's waiting for each frame, alone on the GPU"
description: "Why game_shaders_frames_at_144's woods overview GPU time jumped between about 6 and 11.2 ms between runs of the same code, what decides it, and how frame_perf's runs compare since 26 Nov 2026."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T22:27:01Z }
touches: [crates/mg-ui/tests/frame_perf.rs, crates/mg-ui/tests/render_views/mod.rs, crates/mg-testkit/src/clocks.rs, docs/research/notes_frames.md]
asks:
  - "Why does frame_perf's woods overview GPU time jump between 6 and 11 ms between runs?"
  - "Did the late see-through draws' order (94db6613) slow the woods overview?"
  - "How do I compare two render builds' GPU time when runs differ by a level?"
  - "Is the woods test's slow GPU level caused by VRAM, GTT or the clocks?"
fingerprint: v2-05535b94ea83262e
created: 2026-11-26
author: augustaugur
---
game_shaders_frames_at_144, woods overview (4x MSAA, Shaders on): a frame is drawn either at about 800 MHz (11.2 ms) or about 1,300 (6 ms) on the RX 9070 XT; a run's median pins at 11.2 when over half its frames are slow. Not memory (GTT flat, mclk always 1258).
- Cause (59 runs, 26 Nov, read-only sampler of card1 pp_dpm_sclk plus every process's /proc/*/fdinfo drm-engine-gfx): the test waited for the GPU after each frame and encoded the next (17-21 ms) with the GPU idle. Alone on the GPU, 18 of 18 such runs slow; with another program drawing (Aurora on the off-screen display :2, a test client, another session's UI tests; >10 ms GPU/s) mostly quick. That is why the same binary flipped at any CPU load.
- Paced as the app (eframe never waits: one frame on the GPU while the next is encoded), alone, 11 of 12 quick. The one slow had 45 ms encodes. Adding 20/60 ms sleeps did not make it monotonic: don't claim a governor model; never set a power profile or clock to probe it.
- Since then area_parts and render_views pace as the app (GpuWatch thread times each submission: done - max(submit, previous done)); FRAME_BLOCKING=1 restores the old wait; every view prints "GPU clock N MHz (median; ...)" (mg_testkit::clocks, "n/a" off amdgpu). Compare runs at the same clock; small differences still on alternate frames (the renderer variant flag toggled from the loop).
- 94db6613 was first read as +1.8 ms: that was the level; in-process A/B put the order within 0.3 ms. Its debug patch is untracked at target/retime/lateord-variants.patch (main checkout); never commit it.
