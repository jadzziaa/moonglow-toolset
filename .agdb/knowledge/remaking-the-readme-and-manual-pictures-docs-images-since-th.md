---
type: How-to
title: "Remaking the README and manual pictures (docs/images), since the shell (M3)"
description: "How docs/images' README and manual pictures are made from screens.rs since M3's shell: the 1600 by 1022 harness, the cuts, the JPEG and PNG settings, and where to write them."
tags: []
status: deprecated
generated: { by: agdb/0.2.0, at: 2026-11-14T12:56:04Z }
touches: ["crates/mg-ui/tests/screens.rs:readme_screenshots", "crates/mg-ui/tests/screens.rs:described_table", docs/images/]
asks:
  - how were the README screenshots made
  - how do I remake the pictures in docs/images for a release
  - what crop and JPEG quality do the README pictures use
  - "the README picture shows the log's tab bar at its bottom: what changed?"
supersedes: [remaking-the-readme-and-manual-pictures-docs-images]
fingerprint: v2-f9a5174c908eb79d
created: 2026-11-14
---
The README's four pictures (area.jpg, conversation.png, creature.jpg, script.png) come from `screens.rs`'s `readme_screenshots`, run with the game installed: `MOONGLOW_TEST_OUTPUT=<absolute path under target/> cargo test -p mg-ui --test screens -- --ignored --exact readme_screenshots --test-threads=1`, writing `readme/*.png` at **1600 by 1022** (1022 since M3 T15: the shell's Bottom Dock begins 46 points higher than M2's log pane, under the editor area and over the Status Bar; 976 from 4.2.0, 960 before). The harness draws 8 points in from the window's edge. Each is cut from (8, 8) to (1592, 808) with Pillow, 1584 by 800, just above the Bottom Dock (its bar's top line at 809); area and creature saved as JPEG `quality=88, subsampling=2`, conversation and script as `quantize(colors=256)` PNG. The harness keeps the default settings (editors in floating windows, the log open, the Left Dock open at 1600) and sets `settings.no_last_area` before `open_module` (an area's tab behind the windows showed at their edges).

The manual's only picture, `2da-table-described.png` (embedded by `mg-ui/src/manual.rs`'s PICTURES), is `described_table`'s PNG as written (1280 by 800, RGBA). Its subtitle shows the folder it was written in: write it with `MOONGLOW_TEST_OUTPUT=/home/august/Projects/moonglow-toolset/target/manual-shots` (the short path it has carried), not a worktree's. Its wrong cells sit in rows 1 and 3 because the flags' list opens upward over rows 6 to 9.

Look at each before and after; dark theme unless `MG_LIGHT` is set.
