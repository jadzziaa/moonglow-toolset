---
type: Gotcha
title: "dialog_pattern job openers: rested takes a still job window for the plugin question before its thread asks"
description: "Why the Plugin Question openers failed on Windows CI only, and how they wait for the job's thread to ask (question_waiting, ask_for_tests_late)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T15:02:24Z }
touches: [crates/mg-ui/tests/app/dialog_pattern/openers_shared.rs, "crates/mg-ui/src/plugins.rs:ask_for_tests_late", "crates/mg-ui/tests/app/dialog_pattern.rs:rested"]
asks:
  - Why does the_shared_dialogs_hold_the_pattern fail on Windows with no main action or no field in Plugin Question?
  - Do plugins run on Windows?
  - How do I reproduce a job thread that answers late in UI tests?
fingerprint: v2-5c4732eff40872c6
created: 2026-11-22
author: augustaugur
---
2026-11-22, Windows CI round 2: Confirm, Form and Page Plugin Question openers saw no main action or field. ask_for_tests starts a job titled Plugin Question whose thread posts the question; the job window (title, Cancel) is drawn from frame one, and rested(job) waits only for 4 still frames (~40-50 ms), so a late thread start left the bare window. Not a plugin fault: 113 plugin tests passed on Windows in that run. Reproduced on Linux with ask_for_tests_late (70 ms: all six fail; 40 ms: pass). Fix: openers' ask waits for Moonglow::question_waiting (deadline), then rested; test a_question_asked_late_is_waited_for. Any job opener whose content comes from its thread needs a wait on that content, not stillness.
