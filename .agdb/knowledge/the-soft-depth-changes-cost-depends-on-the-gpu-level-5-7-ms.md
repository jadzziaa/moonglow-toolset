---
type: Fact
title: "The soft-depth change's cost depends on the GPU clock: +5.7 ms median near 800 MHz, +0.4 at 1,100-1,400"
description: "What the held soft-depth change (62a1d436, soft-water-2) costs the woods turned and overview at each GPU clock level, how it was measured (fp-softab plus GpuWatch pacing), and that the decision waits on the owner."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T22:42:24Z }
touches: [crates/mg-ui/tests/frame_perf.rs, crates/mg-render/src/renderer.rs, docs/deferred.md]
asks:
  - What does the soft-depth change cost in the woods turned view?
  - Why did the soft-depth A/B give +2.5 ms one morning and +0.4 the afternoon?
  - "Is the soft-depth slowdown an artifact of alternating frames or of the test's pacing?"
  - "How was fp-softab run with the app's pacing?"
fingerprint: v2-4861e677e8c3fa37
created: 2026-11-26
author: augustaugur
---
In-process A/B, alternate frames (base = soft texels write depth, variant MG_VAR=softold), 160 frames a run, read-only clock sampler beside each run, 26 Nov 2026:
- Near 800 MHz the change's frames often take 17.5 ms against 11.7 before it. Each frame waited for, alone (934-1091 MHz): turned tenth +0.25 to +2.02 (median +1.17 of 9), median 11.6-11.9 vs 17.4-17.7 in 7 of 9. Paced as the app but held at 766-819 MHz by FRAME_GAP_MS=30: 17.3 and 17.6 vs 11.7 in 2 of 3; A/A (change both sides) 17.5-17.7 in 2 of 3; FRAME_ALT_ALL (old code both sides) 11.7-11.8 at 771-787 MHz. So the clock, not the pacing nor the alternation. This is the morning's "+2.5 ms" (median 11.5 to 17).
- At 1110-1405 MHz (paced as the app, nothing else drawing): turned +0.37 to +0.41 tenth, +0.3 to +0.5 median (5 runs, A/A within 0.05); overview +0.21 to +0.32 (6).
- The 72-frame runs are too noisy for medians (A/A sides 4.6 ms apart); use FRAME_COUNT=160.
- Binary: target/levels-wt/target/softab-paced-wt (detached 4ef2d2c9 + target/retime/softab-variants.patch + GpuWatch pacing via FRAME_PACED=1 + FRAME_ALT_ALL + FRAME_GAP_MS; its diff in target/levels-wt/target/levels/softab-paced.patch), copied to target/levels-wt/target/levels/bin/fp-softab-paced3; never commit.
- The orchestrator's rule (slow level reproduces about +2.5 ms: stop and bring both levels' figures) fell on "reproduces"; whether to take the change is the owner's call. Do not merge soft-water-2 without it.
