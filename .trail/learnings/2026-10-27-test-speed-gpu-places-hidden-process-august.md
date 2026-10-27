---
title: "Test speed: GPU places, hidden process-wide state, measuring per test"
author: August
date: 2026-10-27
branch: develop
cost: { minutes: 1, tokens: 1483 }
touches:
  - crates/mg-testkit/src/gpu.rs
  - crates/mg-testkit/src/places.rs
  - crates/mg-testkit/src/engine.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-corpus-tests/tests/engine_tileset_holes.rs
---

## Decided
- `mg_testkit::gpu::hold()` is a semaphore of 4 places (`MG_TEST_GPUS`, 1 = one at a time), after the owner said the machine is stable for batched GPU tests (2026-10-27). 8 was stable too and 3 s faster; 4 kept. The one-device lock had made the `app` binary one 42 s queue (188 GPU tests x ~0.21 s held); now ~13 s. Everyday suite 69 s -> 27 s.
- `mg_testkit::places::Places` is the one counting semaphore (GPU, `engine` servers 8 at once, `engine::timed_turn()` 3 at once for tests that start nwserver through `mg serve`/`mg test`/`mg watch` and wait by the clock; was `one_at_a_time`, a mutex).
- A test that changes process-wide render state (`mg_render::guard::fail_on`) calls `gpu::alone()` (takes every place; new takers wait behind it; it gives back what it held before waiting, so two alone-callers don't deadlock).
- `engine_tileset_holes`' LF test waits 3x the CRLF run (min 20 s) instead of 60 s: the corpus `engine` group 66 s -> 25 s.

## Tried and ruled out
- `RUSTC_BOOTSTRAP=1 cargo test ...` to get `--report-time`: the env var is in cargo's fingerprint and rebuilds the whole tree (and back again after). Run the built test binaries directly instead (`cargo test --no-run --message-format=json` lists them; run each with cwd = its package dir, `RUSTC_BOOTSTRAP=1 <exe> -Z unstable-options --format json --report-time`).
- `exec_time` from libtest includes time queued on a lock: a 7.5 s UI test was 0.2 s of work behind the GPU lock. Instrument the lock (wait vs held) before believing per-test times.

## Watch out
- Serializing tests hides process-wide state. Going to 4 places exposed `mg_render::guard::take_failures()` (a process-global list) being drained by every `Moonglow` frame, including non-GPU test windows: a test's guarded failure landed in another test's log. Fixed by draining only when the window has a viewport (`lib.rs`, the frame's start). Grep for `static .*Mutex|Atomic` in mg-ui/mg-render/mg-area when a test fails only under concurrency.
- `mg_testkit::scratch_dir(name)` empties the folder on every call: a name used by two tests (or twice in one) wipes what the other is writing once they run side by side. Pictures several tests add to go in `output_dir`; `every_scratch_dir_is_named_once` (mg-testkit) fails on a repeated name.
- Merging UI tests: only where nothing has to be put back between parts (a read of all 655 found no test wholly covered by another; ~20 more merges need resets and are left in docs/deferred.md for the owner).
