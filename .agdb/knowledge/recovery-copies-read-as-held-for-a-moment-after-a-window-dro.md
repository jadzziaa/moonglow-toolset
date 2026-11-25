---
type: Gotcha
title: "Recovery copies read as held for a moment after a window drops: tests wait (find_recoveries_let_go)"
description: "Why modules::unsaved_work_survives_a_crash once found 0 recoveries: a child process another test's thread was starting had inherited the copy's lock file; a test race, not data loss."
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
A recovery copy's lock (`<name>-<id>.lock`, flock, `recovery::hold`) belongs to the open file description. A child process that another test's thread is starting (fork, then exec) has a copy of every open file until it runs its program: the lock stays held a moment after the window that took it is dropped, longer on a loaded machine. `recovery::find` skips a held copy (another window's work), so a session that looks straight after the first drops can find none. Seen once in a full run (2026-11-25, `recoveries.len()` 0, expected 1).

## Not data loss
The copy stays on disk and is offered once the child execs (O_CLOEXEC). In the product a crashed process is gone long before the next start looks.

## Fix (tests)
App tests look with `common::find_recoveries_let_go` (retries up to 20 s); `recovery.rs`'s unit tests already waited the same way (`once_let_go`). `unsaved_work_survives_a_crash` holds the lock itself and lets go 300 ms later, so it fails at once without the wait.

## Reproduced and ruled out
- On demand: a thread spawning `true` with a `pre_exec` sleep while the lock is held, then the drop: left 0, right 1.
- A child started on the test's own thread does not reproduce it: `Command::spawn` returns only after exec.
- Shared scratch folders: `ui-recovery` is used by that test alone.
