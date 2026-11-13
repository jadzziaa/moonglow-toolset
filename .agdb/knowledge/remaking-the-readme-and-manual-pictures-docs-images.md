---
type: How-to
title: Remaking the README and manual pictures (docs/images)
description: "How docs/images' README and manual pictures are made from screens.rs, and how they are cut and encoded, so a release can make them again the same way."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-13T21:52:57Z }
touches: ["crates/mg-ui/tests/screens.rs:readme_screenshots", "crates/mg-ui/tests/screens.rs:described_table", docs/images/]
asks:
  - how were the README screenshots made
  - how do I remake the pictures in docs/images for a release
  - what crop and JPEG quality do the README pictures use
  - which docs/images pictures does the manual embed
fingerprint: v2-a00898e3f6b21fd2
created: 2026-11-13
---
The README's four pictures (area.jpg, conversation.png, creature.jpg, script.png) come from `screens.rs`'s `readme_screenshots`, run with the game installed: `MOONGLOW_TEST_OUTPUT=$PWD/target/x cargo test -p mg-ui --test screens -- --ignored --exact readme_screenshots --test-threads=1`, writing `readme/*.png` at 1600 by 976 (976 since 4.2.0: with M2's look the log pane began 16 points higher, and the old 960 left its splitter and chevron in the cut). Each is cut from (8, 8) to (1592, 808) with Pillow, 1584 by 800, just above the log; area and creature saved as JPEG `quality=88, subsampling=2` (4:2:0, the originals' tables), conversation and script as `quantize(colors=256)` PNG. The harness sets `settings.no_last_area` before `open_module`: since the module opens on its last or first area, that area's tab showed at the floating editors' edges (a black 3D view). The manual's only picture, `2da-table-described.png` (embedded in the app by `mg-ui/src/manual.rs`'s PICTURES), is `described_table`'s PNG as written (1280 by 800, RGBA): its wrong cells sit in rows 1 and 3 because the flags' list opens upward over rows 6 to 9 with 22-point rows. The 2DA editor's six and the framebuffer's three pictures are only in proposals and are not remade with releases. Dark theme unless `MG_LIGHT` is set.
