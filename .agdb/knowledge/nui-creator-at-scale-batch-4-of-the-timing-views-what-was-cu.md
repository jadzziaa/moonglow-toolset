---
type: Gotcha
title: "NUI Creator at scale (batch 4 of the timing views): what was cut, what is left, and the traps"
description: "How the NUI Creator's and script editor's large-window costs were found and cut in November 2026 (natural-size memo, column clip break, widgets::in_sight, New Bind stamp, Choose Image fold) and why the rest (whole-window walks each frame) stays."
tags: [performance]
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T06:33:36Z }
touches: [crates/mg-ui/src/nui_view/preview/layout.rs, "crates/mg-ui/src/widgets.rs:in_sight", crates/mg-ui/tests/nui_perf.rs, crates/mg-ui/tests/timing/mod.rs]
asks:
  - "Why is the NUI preview slow with a column of 1,000 rows?"
  - How do I skip laying out egui parts out of sight without shifting ids?
  - "Can the natural-size memo key on a node's address?"
  - How do I profile a Rust test without perf on this machine?
fingerprint: v2-7e8ebd57152aaa96
created: 2026-11-26
author: augustaugur
---
- `preview::layout::natural` was asked again by each level (container asks children, each child asks its own), so a 1,000-row column was walked once per level, four passes a frame. It is memoized per canvas pass in `Renderer::naturals` keyed by (node address, depth). The address key is safe only because the window is not changed during a pass; a list row's cell (`row.is_some()`) is a temporary clone (`local_node`) whose address the next clone reuses, so rows are never kept. Don't cache Interact's double `Session::matches` across a pass: `track_gestures` may mutate the doc between the calls.
- `containers::children` breaks out of a col/row once the cursor is past the clip (not while the asset recorder walks for pictures): `draw_node` drew nothing there anyway (39 nui screens pictures identical by pixels).
- `widgets::in_sight(ui, salt, add)`: a part out of sight takes `allocate_space(remembered size)` with the size kept under a key from the PARENT id, never a skipped `push_id` scope (that cost ~1.3 µs a part). egui's `scope`/`push_id` advances the parent's auto-id exactly as one `allocate_space`, so later ids don't move (unit test `parts_out_of_sight_keep_their_room_and_ids`). A part that held the keyboard focus when last drawn is drawn even out of sight: egui drops focus from a widget not drawn in a frame, and a field that commits on lost_focus would lose its text. First frame draws everything to learn sizes; tests counting headers need two more frames.
- `IdTypeMap::remove_temp` moves a value out (no clone): New Bind's draft is taken that way, and its type stamp compared only in frames with input events.
- Choose Image: names already lowercase ASCII are searched with `finds_folded` directly (no per-name fold allocation over 130k names).
- Left (each needs a change signal for a window edited in place, not a local change): every frame still measures the whole window for the group's scroll extent, walks it for pictures (`image_names_in`) and strings (`prepare_strings`), and Interact deep-compares doc and settings twice (`Session::matches`).
- Profiling with no `perf`: sample the test binary with `eu-stack -p PID` in a loop from a Python launcher that calls `prctl(PR_SET_PTRACER, -1)` in preexec, demangle with c++filt, count inclusive frames.
- Loaded runs (ffmpeg at 1,200% CPU) misread costs near the 70 µs line by ±100 µs; give verdicts near the line only from quiet runs.
