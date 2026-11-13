---
type: Decision
title: "UI tests under load: what makes Harness::run exceed max_steps"
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: [CLAUDE.md, crates/mg-module/src/watch.rs, crates/mg-testkit/src/lib.rs, crates/mg-ui/src/area_view.rs, crates/mg-ui/src/jobs.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/outside.rs, crates/mg-ui/src/references.rs, crates/mg-ui/src/renderer_check.rs, crates/mg-ui/src/script_nav.rs]
fingerprint: v2-bc6f616d8ca77bdc
created: 2026-10-30
author: August
description: "UI test Harness::run exceeds max_steps because parallel runs share target/test-output and because sub-250 ms repaint delays become immediate frames under kittest; fix is gating thread and clock wakeups on background_jobs and waiting with settle."
asks:
  - "Why does Harness::run hit max_steps when two cargo test --test app runs execute at once in the same checkout?"
  - "How do I stop parallel UI test runs from deleting each other's files in target/test-output?"
  - "Why do tests with a 50 ms poll or wall-clock pause spin through frames under kittest, and what should replace them?"
  - "My UI tests fail only on CI with few CPUs and many test threads; how do I reproduce that locally?"
---
Two full --test app runs in one checkout collide: both use target/test-output and scratch_dir empties folders under the other (most of the '1 to 37 failures'). Run the second with MOONGLOW_TEST_OUTPUT=<other folder>. Plain pairs on 32 threads barely fail; CI-like load is both runs pinned to 4 or 2 CPUs (systemd-run --scope -p AllowedCPUs=0-3) with RUST_TEST_THREADS=32. kittest's step_dt is 1/4 s and egui subtracts predicted_dt from request_repaint_after, so any delay under 250 ms is a frame at once in tests: a 50 ms poll spins until a thread is done, a wall-clock pause (the live check) spins until it ends. A thread's ctx.request_repaint() landing mid-run adds 2-3 frames (outstanding=1 plus the pass it interrupts); repaint_causes() is the previous pass's. Rule now: threads and clock pauses ask for frames at once only with background_jobs (jobs::wake, Moonglow::frame_after); tests wait with settle (Moonglow::under_way), never frame counts. Ruled out: raising max_steps, wrapping Harness (730 direct constructions), synchronous looks when !background_jobs (two tests assert the look is off the window's thread).
