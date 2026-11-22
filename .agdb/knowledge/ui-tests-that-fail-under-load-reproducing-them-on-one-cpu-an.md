---
type: How-to
title: "UI tests that fail under load: reproducing them on one CPU, and what they had in common"
description: "How to make one UI test's load failure happen on demand (one or two CPUs beside busy loops), and which waits in tests/app failed under load in M5 S1"
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T10:56:10Z }
touches: [crates/mg-ui/tests/app/blueprints.rs, "crates/mg-ui/tests/app/common.rs:settle", crates/mg-ui/tests/app/renderer_check.rs, crates/mg-ui/tests/app/drawing.rs, crates/mg-ui/tests/app/scripts.rs, crates/mg-ui/tests/app/read_only.rs]
asks:
  - How do I make a UI test that fails only under load fail on demand?
  - "Why does Harness::run exceed max_steps after an armor color or part change?"
  - Which bare h.run() calls in tests/app can fail on a busy machine?
  - "Why did W6's picture decode tests fail on a busy machine?"
fingerprint: v2-1c5f2709e29d91a1
created: 2026-11-22
author: augustaugur
---
The older note [[ui-tests-under-load-what-makes-harness-run-exceed-max-steps]] says why `Harness::run` overruns under load (two runs sharing `target/test-output`, repaint delays under 250 ms becoming frames at once, threads' repaints mid-run), the rule that tests wait with `settle`, and its CI-like whole-suite load (two runs pinned to 4 or 2 CPUs with RUST_TEST_THREADS=32). This one adds a recipe that makes one test's failure come on demand, and the cases M5 S1 found.

Reproduce on demand (M5 S1, 2026-11-22): pin the test binary to one or two CPUs and start two busy loops on each of them (`taskset -c 5 bash -c 'while :; do :; done' &` twice, then `taskset -c 5 <test binary> NAME --exact`; kill the loops by their PIDs). `blueprints::an_armor_s_parts_take_colors_of_their_own_and_keep_them` then failed 3 runs in 4 (it had failed 1 in 3 at load average 30). The script used: `target/m5/S1/contend.sh`.

What failed and why: a bare `h.run()` (4 steps at most) right after something that has pictures made again a few milliseconds a frame (an armor part or color changed, Reset, a page switched to by `blueprint_pages`, the Creature Wizard's Next with its portraits): blueprints' part colors, part none, showing editors, creature wizard, projects' wizard builder code. Fixed by `settle`. Counts of frames standing for work on threads: renderer_check's never-ready watch counted its PATIENCE+20 simulated seconds from the start while the area's first look came 18 s of frames in (now counted from the first look); drawing's reload test ran six frames for textures read on threads (now a deadline). Races: jobs' to-its-end test slept 50 ms for the thread to look at Cancel (now a channel); the external-editor test shared `own_scratch_dir()/edit/hello.nss` with two other tests in the same process (the copies are per process, by script name; now a script of its own). W6's decode-pool tests (read_only.rs): the picture opened last is in sight and goes to the front of the queue (`DecodesShared::hurry`), so "four held ahead of it" holds only once the threads have taken the four (wait on `Moonglow::picture_decodes_untaken`); and a decoding thread is still counted a moment after its last decode's pending count drops, so `(0, 0)` is waited for with a deadline (`decodes_ended`), not asserted after `settle`.
