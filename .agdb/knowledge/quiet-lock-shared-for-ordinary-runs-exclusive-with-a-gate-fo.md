---
type: Decision
title: "Quiet lock: shared for ordinary runs, exclusive with a gate for timing runs"
description: "Why the quiet lock has a gate file, why test binaries share it from their first GPU/display hold or nwn_root, why refusals are remembered, and why children must be told MOONGLOW_LOCKS_HELD=quiet."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T07:43:10Z }
touches: [crates/mg-testkit-gpu/src/quiet.rs, tools/locks.sh, docs/locks.md, crates/mg-testkit/src/quiet_tests.rs]
asks:
  - why does the quiet lock need a gate file
  - how does cargo test honor the quiet lock
  - "why can't a process upgrade from sharing the quiet lock to holding it"
  - why does a GPU test binary fail at once after one quiet refusal
fingerprint: v2-b4f0e2a9cd4b3cce
created: 2026-11-27
author: augustaugur
---
Quiet lock (docs/locks.md, 2026-11-27): quiet.lock shared by ordinary heavy runs, exclusive for a timing run; files in XDG state moonglow/locks beside gpu/.

Gate: flock has no writer preference, so a waiting exclusive taker does not stop new shared takers; test binaries starting one after another would starve a quiet run. A quiet run takes quiet.gate exclusive first and keeps it; ordinary runs pass the gate shared only while taking quiet.lock (then unlock it). Test a_waiting_quiet_run_comes_before_new_ordinary_runs fails without the gate (seen).

Test binaries share it for their run at the first gpu::take (before places), display::try_hold (before the display) or mg_testkit::nwn_root (corpus tests). cargo test itself takes nothing; plain unit tests take nothing. Timing runs wrap the command: tools/locks.sh hold quiet -- cargo test ...; the binary sees MOONGLOW_LOCKS_HELD=quiet and shares nothing.

Gotchas: (1) children of any holder must get MOONGLOW_LOCKS_HELD=quiet: a child sharing again would wait at the gate behind a quiet run that waits for the child's parent (deadlock). display::held_by_us passes quiet and display in one env value (two .env calls overwrite). (2) Nobody upgrades: quiet::hold() panics after a share in the same process. (3) share() remembers a refusal (Held::Refused) so a binary refuses every later test at once instead of waiting MOONGLOW_LOCK_WAIT per test. (4) Windows locks are mandatory: the holder line can't be read while held; tests gate those reads #[cfg(unix)]. (5) The doc's two shell examples are run by quiet_tests from docs/locks.md (<!-- example: quiet-alone / quiet-shared --> markers): change them only with the test.
