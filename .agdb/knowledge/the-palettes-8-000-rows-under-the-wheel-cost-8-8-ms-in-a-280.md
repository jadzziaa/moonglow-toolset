---
type: Gotcha
title: "The palette's 8,000 rows under the wheel cost 8.8 ms in a 280 dock, 2.6 in a 320 one (found in M3 T14, cause open)"
description: "frame_perf at FRAME_SCREEN=1280x800 fails on the custom item palette with all 8,000 rows under the wheel because the Right Dock is 280 there; what was ruled out and how to measure it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T07:18:28Z }
touches: [crates/mg-ui/src/palette_view.rs, "crates/mg-ui/tests/frame_perf.rs:palette_rect"]
asks:
  - Why does frame_perf fail at 1280 on the custom item palette under the wheel?
  - Is the palette slower in a narrow Right Dock?
  - How do I time frame_perf at another window size?
fingerprint: v2-870fc37d7570055e
created: 2026-11-14
---
## Found
- `FRAME_SCREEN=1280x800 cargo test -p mg-ui --release --test frame_perf -- --ignored --nocapture --test-threads=1 frames_at_144_in_a_persistent_world` (FRAME_VIEW="custom item palette, all"): the wheel over the palette with all 8,000 custom items shown takes 8.8 to 9.4 ms with the Right Dock 280 (any window under 1400 since T14), 2.6 at FRAME_SCREEN=1400x800 (dock 320), 2.7 in T13's tree at 1280 (its dock was 320 there). Pointer and idle are 1.8 ms at both widths, so it is the scrolling.
- Ruled out: the types' dropdown (TYPE_COMBO_UNDER at 0, the icon row at 280: 8.7 ms). Not tried: the rows' truncation (`wrap_mode = Truncate` in `palette_view`'s list, which lays out every row, not only those in sight).
- No profiler here: samply needs kernel.perf_event_paranoid at 1 (a system setting, not to be changed by an agent); perf is not installed.
- frame_perf's palette rectangle took the dock's center, which in the wide layout's two columns (741) is in the Preview's column: `palette_rect` now takes the first column. Runs before that timed the wheel over the Preview there.
