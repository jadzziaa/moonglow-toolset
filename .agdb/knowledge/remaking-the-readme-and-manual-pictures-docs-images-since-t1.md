---
type: How-to
title: "Remaking the README and manual pictures (docs/images), since M5"
description: "How docs/images' README, manual and 2DA-proposal pictures are made from screens.rs since M5: the 1600 by 1022 harness, the creature's windows put twice, the cuts, the JPEG and PNG settings, the tables with the log closed, and what to look for."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T07:44:03Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-22T07:49:43Z }
  - { by: agdb/0.2.0, at: 2026-11-22T08:18:20Z }
touches: ["crates/mg-ui/tests/screens.rs:readme_screenshots", "crates/mg-ui/tests/screens.rs:described_table", "crates/mg-ui/tests/screens.rs:table_editor", docs/images/]
asks:
  - how were the README screenshots made
  - how do I remake the pictures in docs/images for a release
  - what crop and JPEG quality do the README pictures use
  - "the model window in the README's creature picture covers the editor: why?"
supersedes: [remaking-the-readme-and-manual-pictures-docs-images-since-th]
fingerprint: v2-2a3c7d0c8652596f
created: 2026-11-14
---
The README's four pictures (area.jpg, conversation.png, creature.jpg, script.png) come from `screens.rs`'s `readme_screenshots`, run with the game installed: `MOONGLOW_TEST_OUTPUT=<absolute path under the worktree's target/> MOONGLOW_REQUIRE_CORPUS=1 cargo test -p mg-ui --test screens -- --ignored --exact readme_screenshots described_table table_editor --test-threads=1` (a few seconds once built), writing `readme/*.png` at **1600 by 1022**. The harness draws 8 points in from the window's edge. Each is cut from (8, 8) to (1592, 808) with Pillow, 1584 by 800: still just above the Bottom Dock after M4 and M5 (the area view ends at y 802, the dock's line is at 809). area and creature saved as JPEG `quality=88, subsampling=2`, conversation and script as `quantize(colors=256)` PNG.

The harness keeps the default settings (editors in floating windows, the Bottom Dock's log open, the Left Dock open at 1600), sets `settings.no_last_area` before `open_module` (an area's tab behind the windows showed at their edges) and closes Module Properties after the module opens (its OK and Cancel showed under the floating windows). The editor area behind the windows shows its "No Area Open" tab. The area's picture has the Palette and Properties in the Right Dock.

The creature's two windows (blueprint 250..1010, model 1010..1590) are each put twice, 5 steps apart (since M5 P1): egui constrains a window's new position to the screen by the size it had the frame before (egui 0.36 `containers/area.rs`, `Area::begin`: `constrain_window_rect_to_area(state.rect(), ..)` with the stored size; egui_dock passes position and size together), so the model's window, opened wider than it is put, was pushed back to x 820 over the editor's form (StrRef, Gallery, Edit Factions and Portraits cut, its Maximize and Close hidden) in every picture before. A 2 px gap between the windows showed marks of what is behind: they touch. Moonglow re-puts a window for the same reason in its own two cases (`crates/mg-ui/src/lib.rs` `remember_window_sizes`'s `track.aim`, and a NUI design's window put back narrow, the `Action` that closes its source). An assertion checks the model's tab is where it was put.

The manual's picture, `2da-table-described.png` (embedded by `mg-ui/src/manual.rs`'s PICTURES), is `described_table`'s PNG (1280 by 800, RGBA, written to `screens-described/described-table.png` under the output folder), with the log closed (`settings.hide_log`); its flags are the Cell panel's rows (since M4 U16), not a list over the grid. The editor shows no folder line any more, so where it was written no longer shows (the old rule to write it under `target/manual-shots` is void). The 2DA proposal's six `2da-table-editor*.png` come from `table_editor` (1280 by 800, RGBA, written to `screens-table/table-editor*.png`, copied with the `2da-` prefix), also with `hide_log` since M5 P1 (the log's lines showed at the window's sides), and Find References waits with `settle` (it was shot half-searched, "Looking through the game's tables"). These PNGs are saved through Pillow with `optimize=True` (lossless, smaller than egui_kittest's own file). The framebuffer proposal's three pictures are renders, not screens: never remade.

Look at each before and after (a row's fill or a window's edge over another pane is a fault to fix, not to ship: M5 P1 found the conversation tree's selected-row fill running under the window's right pane, fixed in `dialog_view::fill_span`); dark theme unless `MG_LIGHT` is set. Never the owner's module or haks in a committed picture.

Remade again for 5.3.0 on 2026-11-25: all eleven changed (the header's OK as the main button, title-cased tree groups, the well's handle, script Help in the Bottom Dock). Run them to one fixed output folder (`target/manual-shots`): screens.rs pictures differ run to run in the Log's times and the output path (note `screens-rs-pictures-differ-run-to-run-716-of-1259-on-one-com`), and the README cuts stop above the Log. Compare each with the committed one (old above new, a pixel count over a threshold of 24) before copying.
