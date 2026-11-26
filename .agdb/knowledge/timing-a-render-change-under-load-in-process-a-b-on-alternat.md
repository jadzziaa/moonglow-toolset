---
type: How-to
title: "Timing a render change under load: in-process A/B on alternate frames"
description: "How to resolve a 0.3-1 ms render change when other sessions load the machine, where frame_perf's noise is about 1 ms."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T03:24:18Z }
touches: [crates/mg-corpus-tests/examples/render_area.rs, crates/mg-ui/tests/frame_perf.rs]
asks:
  - How do I measure a sub-millisecond render change when the machine is loaded?
  - Why do frame_perf A/B runs disagree with each other?
  - How was the renderer profiled without symbol resolution being too slow?
fingerprint: v2-8401b01c1808a1b5
created: 2026-11-26
author: augustaugur
---
Under load 4-12 frame_perf moves ~1 ms run to run on 20-30 ms views (even the unchanged Shaders-off path), and its 'on the processor' is tick-quantized (~1 ms).
- What worked: both code paths built into one render_area binary, toggled on alternate frames (a OnceLock flag, never env::var per draw), medians of encode time compared within one run (MG_TIME_RENDER prints encoded ms and the processor's ms from /proc/thread-self/schedstat).
- Caveat: render_area's camera turns cumulatively, so odd and even frames are not the same work and batching counts grow over frames; swap parity or run A/A before trusting a small difference.
- Profiling: eu-stack -q (no symbols) with /proc/PID/maps saved, resolved offline with nm and readelf; symbol resolution in the sampler is far too slow.
