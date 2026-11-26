---
type: Fact
title: "The soft-depth change's cost depends on the GPU level: +5.7 ms median waiting per frame, +0.4 paced as the app"
description: "What the held soft-depth change (62a1d436, soft-water-2) costs the woods turned and overview at each GPU clock level, how it was measured (fp-softab plus GpuWatch pacing), and that the decision waits on the owner."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T22:27:01Z }
touches: [crates/mg-ui/tests/frame_perf.rs, crates/mg-render/src/renderer.rs, docs/deferred.md]
asks:
  - What does the soft-depth change cost in the woods turned view?
  - Why did the soft-depth A/B give +2.5 ms one morning and +0.4 the afternoon?
  - Is the soft-depth slowdown an artifact of alternating frames?
  - "How was fp-softab run with the app's pacing?"
fingerprint: v2-8482aa0727433ade
created: 2026-11-26
author: augustaugur
---
In-process A/B, alternate frames (base = soft texels write depth, variant MG_VAR=softold), 160 frames a run, read-only clock sampler beside each run, 26 Nov 2026:
- Each frame waited for (fp-softab's old pacing), alone at 934-1091 MHz: woods turned tenth +0.25 to +2.02 (median +1.17 of 9 runs), median 11.6-11.9 before vs 17.4-17.7 after in 7 of 9. A/A with the change on both sides also 17.2-17.7; FRAME_ALT_ALL (old code on both sides) 11.0-11.8: a real cost at that clock, not an alternation artifact. This is the morning's "+2.5 ms" (median 11.5 to 17).
- Paced as the app (1110-1405 MHz): turned +0.37 to +0.41 tenth, +0.3 to +0.5 median (5 runs, A/A within 0.05); overview +0.21 to +0.32 (6).
- The 72-frame runs are too noisy for medians (A/A sides 4.6 ms apart); use FRAME_COUNT=160.
- Binary: target/levels-wt/target/softab-paced-wt (detached 4ef2d2c9 + target/retime/softab-variants.patch + GpuWatch pacing via FRAME_PACED=1 + FRAME_ALT_ALL), copied to target/levels-wt/target/levels/bin/fp-softab-paced2; never commit.
- The orchestrator's rule (slow level reproduces about +2.5 ms: stop and bring both levels' figures) fell on "reproduces"; whether to take the change is the owner's call. Do not merge soft-water-2 without it.
