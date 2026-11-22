---
type: Gotcha
title: "Reload frame over budget traced to the module tree's eager row words (M5 W7)"
description: "Why frame_perf's reload_frames_at_144 showed one 11.7-16.5 ms frame in M5's shared tree, and how it was traced and settled."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T18:26:39Z }
touches: [crates/mg-ui/src/tree.rs, crates/mg-ui/tests/frame_perf.rs]
asks:
  - why is the reload after a pull frame over budget
  - what made reload_frames_at_144 slow in M5
  - "should the module tree's row words be made for every row when the groups are rebuilt"
  - how to bisect a frame_perf spike between uncommitted tasks
fingerprint: v2-6198442d9f7fd081
created: 2026-11-21
author: augustaugur
---
`frame_perf::reload_frames_at_144` (a nasher project of 21,000 files reloaded after a pull) showed one frame of 11.7 to 16.5 ms at about 2.1 s in M5's shared worktree (W4's runs at 10:05), against 3.1 to 3.9 ms at `f1ff1fe6`.

Cause: W1's first version of `tree.rs` made every resource's `RowWords` (the words, the tip, the Filter text) for all files whenever the groups' lists were rebuilt. A pull rebuilds them, because `take_outside` resets `area_names`. That cost 10.7 ms inside `shell::left_dock`, measured with a scratch per-phase timer around the statements of `Moonglow::ui`. W1's later lazy rows (`Rows::get` makes a row's words when it is first drawn, and keeps them with the list) removed the cost.

Evidence, interleaved, at load 9 to 11:
- W1's earlier `tree.rs` (its version before the lazy rows) put back into a copy of the tree: 11.8, 15.3 and 12.0 ms.
- With the lazy rows: 3.3, 3.2 and 3.3 ms.
- `f1ff1fe6`: 3.2 to 4.2 ms, with two runs at 28 and 46 ms that were not explained (load is presumed).

Again at load 5.8 falling to 3.0, interleaved three times: `f1ff1fe6` 20.6 (the first run), 3.1 and 3.1 ms; the live tree 3.3, 3.2 and 3.2 ms; the eager `tree.rs` 12.2, 12.1 and 11.7 ms.

Method that worked: copy the worktree under `target/` (rsync, without `target` and `.git`) and build release `frame_perf` there, so no other session's build is disturbed. Swap one task's file into the copy, run the three binaries interleaved, and print a timer per phase for any frame over 5 ms. A binary another task kept (W4's `frame_perf-w4`) reproduces a past state when its source is gone.

Rule: anything that has to be made for every row of a module's resource list must not run when the lists are rebuilt. Make it when the row is first drawn, as `Rows` does.
