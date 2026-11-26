---
type: How-to
title: "Timing views at an old commit: build each side, copy the binary"
description: "How the V, C, M and P timing views of November 2026 were timed before and after their fix commits, and what the traps were."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T12:45:27Z }
touches: [crates/mg-module/tests/timing.rs, crates/mg-plugin/tests/timing.rs, apps/mg/tests/timing.rs]
asks:
  - How do I time a fix before and after its commit with a test that does not exist in the old tree?
  - Why does the apps/mg test run the wrong mg binary?
  - Why is a second Find with haks and game slow?
fingerprint: v2-e252866510c76c87
created: 2026-11-26
author: augustaugur
---
Second worktree under target/, check out the commit, copy the timing test in, build with --message-format=json to find the test binary, copy the binary out (CARGO_BIN_EXE_mg is baked in: apps/mg's view takes MG_TIMING_BIN and MG_TIMING_DIR, a copy of mg per side), then alternate runs. A fix's parent in a branch may lack another fix's API (rename_among): sed the view's marked lines. Old mg-plugin trees have no tests/groups: the view is its own binary there, with a local Host. Results: docs/log.md 2026-11-26. Left: Kept::search still re-lists the layers' keys at each Find (about 25 ms at 45,000); UsageIndex::links_into scans every read for LinkedTo (7 ms an area at 5,000).
