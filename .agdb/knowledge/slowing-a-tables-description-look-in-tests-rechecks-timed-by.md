---
type: Gotcha
title: "Slowing a table's description look in tests: rechecks timed by the simulated clock chase settle"
description: "Why injecting a delay into every 2DA description look makes settle-based tests say the window's work never ended, and where to inject instead."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T16:23:47Z }
touches: ["crates/mg-ui/src/table_view.rs:describe", "crates/mg-ui/tests/app/common.rs:settle"]
asks:
  - "Why do table tests fail with the window's work never ended when I add a sleep to look_for_description?"
  - "How do I sweep UI tests for waits on the 2DA editor's threads?"
  - Did frame_after in table_view.rs shorten run_steps waits?
fingerprint: v2-274b4bafa2b2e0bb
created: 2026-11-22
author: augustaugur
---
2026-11-22 (CI round 3): describe() restarts the look when ctx.input time passes described_at+1.0, and kittest's time advances 0.25 s a frame regardless of wall time, so a look that takes 150 ms restarts within the same try_run after it is taken, and settle never sees under_way None (24 table tests failed so). Real rechecks (Looked::Same) take microseconds. To sweep, delay only after walking.store(true) (the first read or a changed description) and in find_references' thread: the whole --test app then passed. run_steps(n) draws exactly n frames whatever repaints are asked, so frame_after changes only h.run() lengths; the tables.rs Find References test's run_steps(3) was a plain race (now settle).
