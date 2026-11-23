---
type: Gotcha
title: Update staging lock held on by programs starting on other threads
description: "Why a released mg-update cache lock was refused as BUSY (macOS CI, a_package_that_doesn_t_verify_is_never_kept): a flock belongs to the open file, which a child spawned on another thread copies until exec, so the lock must be unlocked explicitly, not just closed."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T08:41:05Z }
touches: ["crates/mg-update/src/stage.rs:Locked", "crates/mg-update/src/stage.rs:hold", "crates/mg-update/src/stage.rs:clean"]
asks:
  - "Why did a mg-update stage test fail with 'another Moonglow that is running has this update'?"
  - Why is the update lock unlocked explicitly instead of just closed?
  - Can a lock file be held after the file is dropped when another thread spawns a process?
fingerprint: v2-3f6f18bdadf8f688
created: 2026-11-23
author: augustaugur
---
**Cause.** `stage::hold` takes the cache's lock with `File::try_lock` (flock on Unix). A flock belongs to the open file description, not the fd. When any thread of the process starts a program (`Command`; glibc's posix_spawn clones, XNU's posix_spawn forks in the kernel, the fallback forks), the child has a copy of every open fd, CLOEXEC ones included, until exec. If the lock's `File` is dropped (closed) in that window, the lock stays held by the child's copy, and a `hold` right after in the same process gets `WouldBlock` and returns `BUSY`. CI (macOS, once, 2026-11-23): `a_package_that_doesn_t_verify_is_never_kept` failed its second `fetch` (stage.rs:518) while `install::tests` ran `sleep`/`sh` on other threads.

**Fix.** The held lock is a `Locked(File)` whose `Drop` calls `unlock()` (flock LOCK_UN acts on the description, so the child's copy no longer holds it); `clean`'s own hold uses it too. Tests that simulate another run's lock (`another_run_s_staged_update_is_left_alone`) unlock before dropping.

**Repro.** `a_lock_let_go_of_is_free_while_programs_start`: a thread runs `true` in a loop while the lock is taken and let go 4000 times; without the unlock ~25% of takes are refused on Linux, with it none. Plain loops of the suite (300 runs, 32 threads) never showed it: the window is microseconds.

**Users.** Only a retry starting within microseconds of a failed download's release, while Moonglow starts a program on another thread; retries are user-driven (mg-ui updates.rs), so practically never met.
