---
type: Gotcha
title: "Recovery copy locks are unlocked, not only closed: a starting child kept them held"
description: "Why modules::unsaved_work_survives_a_crash once found 0 recoveries: a child process another thread was starting had a copy of the lock's open file; fixed in the product by unlocking before closing, as mg-update's staging lock."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T14:00:53Z }
touches: ["crates/mg-ui/src/recovery.rs:find", "crates/mg-ui/tests/app/common.rs:find_recoveries_let_go", "crates/mg-ui/tests/app/modules.rs:unsaved_work_survives_a_crash"]
asks:
  - Why did unsaved_work_survives_a_crash find no recovery copy?
  - Is the recovery flake real data loss?
  - How should a UI test look for a recovery copy a dropped window left?
  - "Why does recovery::find skip a copy right after its window is gone?"
fingerprint: v2-d9d14a5ac696568e
created: 2026-11-25
author: augustaugur
---
## Cause
A recovery copy's lock (`<name>-<id>.lock`, flock via `File::try_lock`, `recovery::hold`) belongs to the open file description. A program another thread starts (fork/clone, then exec) has a copy of every open file until it runs its program. If the window's `Lock` is only closed in that moment, the child's copy keeps the lock held, and `recovery::find` skips the copy as another window's. Same for `held()`'s own probe lock, which another look then saw as held. Seen once in a full test run (2026-11-25, 0 recoveries where 1 was expected); the running app starts programs too (the compiler, the external editor, `mg serve`).

## Not data loss
The copy stays on disk and is offered once the child execs.

## Fix (product)
`Lock::drop` and `held()` call `unlock()` (LOCK_UN acts on the description, so every copy lets go) before the file closes, as [[update-staging-lock-held-on-by-programs-starting-on-other-th]] did for mg-update. The tests' deadline waits (`once_let_go`, an app-test helper) were removed: they no longer earn their place.

## Tests
- `a_copy_let_go_of_is_offered_while_a_program_has_its_lock_file`: a `try_clone` of the lock's file stands in for the child's copy; without the unlock 0 offered.
- `a_lock_let_go_of_is_free_while_programs_start`: hold, let go and look 2000 times while a thread runs `true`; without the unlocks 359 refused.

## Ruled out
- A child started on the test's own thread: `Command::spawn` returns only after exec, so it can't reproduce it.
- Shared scratch folders: `ui-recovery` is the test's alone.
- A stand-in lock on a new open of the lock file: it is a separate holder, not the child's copy, so unlocking can't free it.
