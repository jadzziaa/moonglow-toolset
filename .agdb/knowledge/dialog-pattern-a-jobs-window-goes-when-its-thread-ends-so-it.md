---
type: Gotcha
title: "dialog_pattern: a job's window goes when its thread ends, so its Escape and Enter are checked by a deadline (comes_to)"
description: "Why Plugin Question was still shown after Escape on Windows CI, and how dialog_pattern now waits for a job dialog to close or be answered."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T09:39:17Z }
touches: ["crates/mg-ui/tests/app/dialog_pattern.rs:comes_to", "crates/mg-ui/tests/app/dialog_pattern.rs:escape", "crates/mg-ui/src/plugins.rs:ask_for_tests_late", crates/mg-ui/tests/app/dialog_pattern/openers_shared.rs]
asks:
  - "Why does the_shared_dialogs_hold_the_pattern fail with Plugin Question: closed by Escape on Windows?"
  - "Why is a job's window still shown after Cancel or Escape in a UI test?"
  - How do I test a job thread that ends late after it is answered?
  - How should a test wait for a job dialog to close?
fingerprint: v2-6c7a14abdff5cb5b
created: 2026-11-23
author: augustaugur
---
2026-11-23, Windows CI (1 run in 3): dialog_pattern's escape asserted Plugin Question (jobs.rs:window#1 Folder) closed right after `pressed`, whose `rested(.., job=true)` is only "window still for 4 frames, 10 ms apart". Escape answers the Ask (plugins.rs question_answer) at once, but the job window goes only when the job's thread wakes from the condvar, ends, and poll_job sees `is_finished` (jobs.rs poll_job; no frame is asked for at the thread's end, the window's 50 ms repaint finds it). A starved thread made the still window look closed-for-good. Test fault, not program: a user sees at most ~50 ms linger, and a real plugin's job goes on after its answer anyway.

Fix: `comes_to(h, op, done)` in dialog_pattern.rs: for a `.job()` opener, steps frames until `done` or SETTLE_TIME; used by escape, enter (effect, close, enter_when_off) and enter_on_the_focused_main; a_plugin_message_closes_by_escape_and_by_enter waits the same way, keys_reach_the_job_window settles after Escape. Non-job openers rest by settle, which waits out `busy()`, so they never had it.

Reproduction: not on Linux by load (0 of 12 rounds, one CPU beside two busy loops; also two copies at nice 5). By injection: `ask_for_tests_late(q, late, ends_late)` sleeps `ends_late` after the answer; test a_question_ending_late_is_waited_for (300 ms) fails at escape's assertion without the wait, passes with it. Negative checks ("Enter leaves it open") stay immediate.
