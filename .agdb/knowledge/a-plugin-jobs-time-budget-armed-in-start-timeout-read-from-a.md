---
type: Gotcha
title: "A plugin job's time budget: armed in start(), timeout read from a latch"
description: "Why a plugin panel or event reported 'ran out of time' for an error it raised in time, and how the budget is now measured."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T05:00:24Z }
touches: ["crates/mg-plugin/src/runtime.rs:start", "crates/mg-plugin/src/runtime.rs:failed", "crates/mg-plugin/src/runtime.rs:Shared"]
asks:
  - "Why did a plugin panel fail with 'ran out of time' on Windows CI?"
  - "When does a plugin job's time budget start counting?"
  - How does failed() decide a job ran out of time?
  - Is the panel test a_panel_is_held_to_its_limits flaky?
fingerprint: v2-f68ade6a56bf584b
created: 2026-11-26
author: augustaugur
---
Windows CI (run 45754933913) failed kinds::a_panel_is_held_to_its_limits: the set panel, which raises 'a panel only reads' in under 1 ms, was reported 'it ran out of time' (250 ms PANEL_BUDGET). Two flaws, both fixed:
1. The deadline was set when Shared was made, so the host's setup (Shared, Lua::new, API tables, prelude, sandbox, reading main.luau) counted. Locally that is at most 2.3 ms, so it was not the cause alone. Now start() arms the deadline just before lua.load of the entry (or at its end with no entry).
2. failed() and the manifest check re-read the clock when the error was reported, so any error reported after the deadline (the thread stalled between raise and report) was relabeled a timeout. Now out_of_time() latches Shared::ran_out when it finds the job past its deadline (the interrupt, the pcall pass-through, folder listings all go through it), and the reports read ran_out().
Reproduced on Linux: the plugins binary under taskset -c 0 --test-threads=16 failed 1 of 30 on the old code (a different panel test, same relabel); 0 of 130 after. Tests: runtime::tests::the_hosts_setup_is_not_the_jobs_time and an_error_reported_after_the_deadline_is_the_codes_own (both fail on the old code). The panel test itself was not changed or widened.
