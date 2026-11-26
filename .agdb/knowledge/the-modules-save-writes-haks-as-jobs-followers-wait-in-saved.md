---
type: Decision
title: "The module's Save writes haks as jobs; followers wait in saved_waits"
description: "How the module's Save, Test Module, Exit and the plugins' saved event wait for open haks written as jobs (Moonglow::when_saved, saved_waits, hak_view::save_and), and the traps: one job at a time, a lost finish, a FIFO to hold a job in tests."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T21:20:08Z }
touches: ["crates/mg-ui/src/lib.rs:Moonglow::when_saved", "crates/mg-ui/src/hak_view.rs:save_and", "crates/mg-module/src/attach.rs:copy_with", crates/mg-ui/tests/app/hak_closing.rs]
asks:
  - why does Test Module or Exit run only after the hak save job
  - "how do I run something after the module's save finishes writing its haks"
  - how can a UI test hold a hak save job under way deterministically
  - how does Add Haks report byte progress and cancel
fingerprint: v2-4cb5dc74c62b1c5f
created: 2026-11-26
author: augustaugur
---
D19 (2026-11-26). The module's Save (`Moonglow::save`) writes the module in the frame, then each changed hak that is saved with it as a job, one after another (`save_haks` -> `save_next_hak` -> `hak_view::save_and`). Anything that must read the written files goes through `Moonglow::when_saved(f)`: while a chain runs, `saved_waits` is `Some(vec)` and `f` is queued; else it runs at once. `SaveThen`'s whole decision block (has_unsaved_work, module_failed, ask_side_docs) moved into `saved_then`, queued this way: left inline, it saw the hak still dirty and closed the hak's tab through ask_side_docs once clean. The plugins' saved event and Test Module's inner save (stale NUI windows) also wait. The state lives on the app, not in an argument, because a never-saved module reaches the save through SaveAsDialog -> SaveAt -> a nested `save`.

Traps: `start_job_on` only warns and returns when a job already runs, and `finish` is skipped when a thread panics or fails to spawn: any of these would leave `saved_waits` Some forever and drop Exit silently. `save_job` therefore checks `app.job` first (a failure with a reason), wraps the write in `catch_unwind`, and calls `after(false)` itself if no job exists after the start. With `background_jobs` off (tests) the finish runs inside the start call, so `saved_waits` must be set before the first job starts. `refuse_save_under_hak` refuses Save/SaveThen/SaveAt while SAVE_JOB runs or a chain is pending (the modal job window does not stop actions pushed round it).

Add Haks: `attach::copy_with(placements, replace, step: impl FnMut(done,total)->bool + 'static)`; the step must be 'static because rars' `ExtractionDecision::Extract(Box<dyn Write>)` is 'static, so the metering writer holds an Rc<RefCell<Meter>>. Unwanted entries of solid 7z/RAR go through `archive::extract_with`'s `skipped` writer so Cancel is polled there too. Stopped is checked before the error (the archive readers wrap a stop as their own error). `copy` was left untouched (std::fs::copy reflinks).

Tests hold a hak job under way with a FIFO (`held_hak` in tests/app/hak_closing.rs): a file added to the hak, then replaced by `mkfifo`; the job blocks opening it until `Held::release` opens it for writing; the 0 size matches the empty read. Clicking the job window's Cancel needs `click_pos` on its rect (Escape and `.click()` did not reach it).
