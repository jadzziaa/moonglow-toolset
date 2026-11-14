---
type: Decision
title: "M3 T15: no icon mesh cache (measured), the dock tabs' tight words, and how to time two trees on a busy machine"
description: "Why the drawn icons' meshes are not cached (the shell's interface views cost +0.025 ms, tessellation +1.6% over the T0 tree), why dock tabs close their words up before turning to icons, and the interleaved two-tree timing method."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T12:56:21Z }
touches: [crates/mg-uikit/src/icons.rs, "crates/mg-uikit/src/shell.rs:DockTabs", crates/mg-ui/tests/frame_perf.rs]
asks:
  - Should the drawn icons be cached as meshes instead of tessellated every frame?
  - "The Left Dock's tabs show icons instead of Module and Resources at its default width: why?"
  - How do I compare frame times with an older commit when the machine is busy?
  - What is DOCK_TAB_TIGHT_PADDING for?
fingerprint: v2-1766102a015c29a0
created: 2026-11-14
---
## The icon mesh cache: not built (T15, by the plan's rule)
- Rule: build it if the shell's median view cost rose more than about 0.1 ms or tessellation more than 25% over the T0 baseline.
- Measured 2026-11-14, the T0 tree (d7f03d88, before the shell) against T15, interleaved (t0, m3, m3, t0; ab2/) at a load of 1.4 to 2.5 sampled at each run's start (ab2/loads.txt ends at 9.61; the first set, ab1/, at 2.3 to 10.4, gave all views' tessellation +4.4%): the 156 views that name no 3D view, median 0.778 -> 0.783 ms (+0.025), their tessellation 10.54 -> 10.71 ms summed (+1.6%); all 306 common views median -0.057 ms, tessellation +2.3%. T9's crowded bar: about 20 µs a frame for 30 tabs. Far under either line.
- If it is ever built: key it by icon, size, color AND the font atlas's size (egui's round caps and dots come from discs in the font atlas, whose UVs move when the atlas grows), or draw the dots as circles.

## Dock tabs' tight words
- At the Left Dock's first 260 px (bar 259 inside its line) "Module" and "Resources" with 10 px padding (145.4) plus the Module panel's four buttons and the fold button (5 x 22) and TOOLS_PADDING (4 + 2) needed 261.4: 2.4 px over since T14 put the fold button on the bar, so they showed icons. The Main board fits because it draws no fold button.
- `DockTabs` now tries words at DOCK_TAB_PADDING, then at DOCK_TAB_TIGHT_PADDING (8, an icon tab's), then icons (`TabForm`, `tabs_width_in`). Tests: kit `a_dock_s_tabs_close_their_words_up_before_they_turn_to_icons`, app `the_left_dock_s_tabs_show_their_words_at_its_first_width`.

## Timing two trees on a busy machine
- `git archive <old> | tar -x -C target/t15-ab/base`, build its frame_perf with its own CARGO_TARGET_DIR, run the binaries directly from each tree's crates/mg-ui (cargo's cwd), alternating A B B A (target/ui-mock/perf-m3/t15/ab.sh), and count a view as moved only where both pairs agree in sign beyond 0.3 ms. A single run during a load spike gave views at 2x their time (Area Properties 3.3 ms against 1.6; area084's one-column idle 8.8 against 2.3) that the interleaved runs did not repeat.
- The T0 tree has no FRAME_SCREEN: at 1280 by 800 compare only with T14's logs.

## The crowded document tab bar (for the owner)
- Pictures in target/ui-mock/impl/m3-t15/tabcut/: today's end cut ("at_m1q4…"), a middle cut keeping ".nss" ("at…01.nss"), and one dropping it ("at_…001"). Chapter1's at_m1q4c_m1q_001 and at_m1q4e_m1q_001 still read alike cut in the middle: what tells them apart is in the middle of the name.
