---
title: CI's time: the caches, and the tests' build
author: August
date: 2026-10-29
branch: worktree-agent-a0eec6c3bc42437d8
touches:
  - ".github/workflows/ci.yml"
  - ".github/workflows/release.yml"
  - docs/PLAN.md
---

## Decided
- rust-cache saves on develop alone (`save-if`), in ci.yml and release.yml. GitHub keeps 10 GB of caches per repository; every push that changes a manifest is a new key (about 3.8 GB for the three systems), so a busy branch (renderer-ci) evicted all of develop's. Read them with `gh cache list --json key,ref,sizeInBytes`.
- The tests are built in CI with `CARGO_PROFILE_TEST_DEBUG=0`. Natively (no `--target`) cargo shares host and target units when their profiles agree; with Cargo.toml's line tables, `cargo test --doc` and `cargo test --lib --bins --tests` resolved different host units (proc-macro2, quote, syn, serde, toml), so on Linux the doc tests rebuilt all 26 workspace libraries (two minutes). With `--target` given the graphs agree, so test natively.
- Ruled out: `rust-lld` for the Windows tests (run 40971587639). Windows builds every unit about 1.8 times as long as Linux, compile-heavy and link-heavy alike, so the gap is the runner, not link.exe; the build's last stretch (mg's links) took no less.
- fmt and clippy are a Lint job per system, beside the Test job; each job has its own cache key (the job id is in it).

## How to look
- A job's log with timestamps: `gh run view --job ID --log`; the Test step's `Compiling mg-ui` and `Compiling mg` lines and `Finished` show the long pole (Windows 418 s from mg-ui to mg, Linux 197).
- Unit graphs without building: `__CARGO_TEST_CHANNEL_OVERRIDE_DO_NOT_USE_THIS=nightly cargo test ... --unit-graph -Zunstable-options`; compare units with their dependency trees (features, profile) to see why a second command rebuilds.
- Each Test job uploads `timings-<os>`, the build's `--timings` report.
