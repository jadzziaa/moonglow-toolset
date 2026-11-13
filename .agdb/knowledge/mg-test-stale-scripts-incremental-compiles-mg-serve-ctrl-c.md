---
type: Decision
title: "mg test stale scripts, incremental compiles, mg serve Ctrl+C"
tags: []
status: stable
generated: { by: trail, at: 2026-10-28T00:00:00Z }
touches: [apps/mg/src/main.rs, apps/mg/src/serve.rs, apps/mg/src/test.rs, crates/mg-corpus-tests/tests/aurora_lighting_scheme.rs, docs/deferred.md, docs/log.md, docs/manual/07-scripts.md, docs/manual/10-options.md, docs/manual/11-command-line.md, docs/manual/13-troubleshooting.md]
fingerprint: v2-8ecfa1bb533377bd
created: 2026-10-28
author: August
description: "mg test and mg serve now recompile every script build::changed finds stale, not just uncompiled ones; also records mg serve Ctrl+C handling via its own process group, a nasher .ncs caveat, a testkit scan pitfall, and an NWNX driver-none recipe."
asks:
  - "Why did mg test run an old .ncs for a script I changed, and passing on outdated code?"
  - "How does mg serve let nwserver exit cleanly with 'Server: Exiting...' when I press Ctrl+C?"
  - "Why does mg build's remembered-compiled-scripts cache do nothing for my nasher project across runs?"
  - "How do I run a module's own tests with NWNX using mg test --driver none, preload and an env file?"
---
decided: `mg test` (and `mg serve` on a folder or project) compiles, for its copy, every script `build::changed` finds not as `mg compile`/the window remember it, not only the uncompiled ones: a test ran a changed script's older .ncs and passed on old code. It reads what is remembered and notes only the builder's compiled scripts found byte-equal to a fresh compile (what goes into the copy is not the builder's module).
decided: the stale-script note says "not what their source compiles to now (changed since, or made by another compiler)": BioWare's Neverwinter Chess, compiled by the 2002 compiler, has 140 scripts whose bytes differ from today's compiler with unchanged sources.
watch out: a nasher project's tree keeps no .ncs (`project.rs` never writes them), so `Remembered` passes over nothing for a project across runs: `mg build` reading and writing it is correct but changes nothing unless the project keeps .ncs among its sources. No compiled-scripts cache was made (a new mechanism; the owner's call).
watch out: nwserver handles SIGINT itself and exits cleanly ("Server: Exiting..."); what lost the clean end was `mg` dying first. mg serve now catches Ctrl+C (ctrlc) and gives the server a process group of its own (`mg_serve::Options::own_group`, only where the caller catches Ctrl+C, else cargo test's Ctrl+C would orphan servers). The test signals the whole group (`kill -INT -- -PGID`, mg spawned with `process_group(0)`), as a terminal does.
watch out: `mg_testkit`'s `every_scratch_dir_is_named_once` scans the source for `scratch_dir("…")`: a function of another crate with that name and a literal argument trips it (hence `serve::run_dir`).
tried: `mg test --driver none` with NWNX: the local build in ~/.local/opt/nwnx/current (Binaries/NWNX_Core.so for build 8193.37.17) with `--preload` and an `--env` file (NWNX_CORE_LOAD_PATH, NWNX_CORE_SKIP_ALL=y, NWNX_UTIL_SKIP=n, NWNX_UTIL_PRE_MODULE_START_SCRIPT=own_tests) runs a module's own tests and NWNX_Util functions answer; results go under "(outside a test)".
