---
type: Gotcha
title: A worktree nested in target/ can leave cargo running a stale test binary
description: "Why cargo test reported Fresh and ran a binary without the new tests while another agent's worktree under target/ built, and the fix (own CARGO_TARGET_DIR)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T20:41:01Z }
touches: [crates/mg-ui/tests/app/main.rs]
asks:
  - why does cargo test run 0 tests after I added a test file
  - why is mg-ui Fresh after I touched its sources
  - why did clippy miss a warning in my new test
fingerprint: v2-08e69858353b2ffd
created: 2026-11-26
author: augustaugur
---
On 26 Nov 2026 another agent's worktree at target/r9-wt built into this checkout's target/ (the build-dir layout target/debug/build/<crate>/<hash>/out hashes workspace-relative paths, so both trees' units share hashes and fingerprints). cargo then said mg-ui was Fresh even after touch, ran app-<hash> built from the other tree (no new test module: 'running 0 tests'), and clippy --all-targets passed while the real tree had a type_complexity warning. Fix: give your runs their own CARGO_TARGET_DIR (e.g. target/sb-target; sccache made the rebuild about a minute) and re-run tests and clippy there before trusting a green result. Check: the Running line's binary should list your new file in its .d file.
