---
type: Gotcha
title: "NUI Creator: keeping what is worked out of the window across frames (generation, proof, traps)"
description: "How the NUI canvas keeps its layout, walks and Interact's check between frames safely, the bugs met building it, and how to profile a UI frame without perf."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T10:33:58Z }
touches: ["crates/mg-ui/src/nui_view.rs:settle", "crates/mg-ui/src/nui_view/preview/layout.rs:Measured", "crates/mg-ui/src/nui_view/interaction.rs:Session", "crates/mg-ui/src/nui_view/skin.rs:Assets"]
asks:
  - why does the NUI canvas keep its layout between frames and when is it dropped
  - what is window_gen and settle in nui_view
  - how do I profile an egui frame without perf
  - "why did Design eight deep and the shell's frame stay over the line"
fingerprint: v2-5908c023b6365c2a
created: 2026-11-26
author: augustaugur
---
- The window (`Work.window`) is edited in place by many paths, so there is no change signal. Instead a positive proof: `end()` arms `state.settled = (copies, edit_view)` only in its `can_skip_diff` branch (a quiet frame, window verified equal to the module's source); `settle()` at the start of `design_view` takes it and also requires this frame quiet up to the canvas (no input events but pointer moves, no focus, popup, popover, drag, dnd payload, `mutated_id` flag). Anything else bumps `state.window_gen`. Consumers trust the generation only through `trusted(ctx, state)` (= proven in this pass): a canvas drawn without `settle` (tests' direct `preview::canvas`) keeps nothing.
- Trap met: `end()` first called `unsettle()` (which bumps the generation) before arming, so every frame was a new generation. It must clear `settled` and bump only if it did not arm. A frame where `design_view` doesn't run but `end()` does must still bump (the window may change on another page).
- Identity must include `edit_view`: a variant is swapped into `window["root"]` every frame, so the same address holds other content on other pages. `store_typed` edits the window outside a frame: it unsettles. Never carry when `shown` is Some (typed source is a fresh Value each frame; Interact's doc changes).
- Kept: `preview::Measured` (natural sizes keyed by (address, depth) and each container's `Plan`: sizes, fills, sums, widths_after), carried in `State::measured` under (generation, scale bits, ui scale bits); `Walked` (picture names and strrefs) under (generation, session stamp); `Session::proven` (compared once a generation) and `Session::revision` (a global stamp bumped by apply/collapse/dispatch/constrain_geometry when it changes a value). A debug build recomputes each carried value once a pass and asserts equal, so the whole NUI suite is the differential.
- Choose Image: `Needle::narrows` (each wider word inside a narrower word) lets a key typed search the last result; the stack pops on keys taken back. The rest of its cost was a picture dropped after one pass unasked and decoded again every other frame: pictures now linger `LINGER` = 30 passes.
- Left for reason: Design 511 controls eight deep (+157..164 µs): per extra container a third JSON lookups, a sixth `ui.interact`, the rest drawing; lookups at zero still ~+105. M3 shell +0.11 ms (grass field): diffed against abe3e024, three fifths new chrome widgets, two fifths tessellation; nothing redundant.
- Profiling without perf: sample with `eu-stack -q -p PID` in a loop (raw addresses, fast: ~10k samples a minute), save /proc/PID/maps, resolve offline via `readelf -lW` (file offset → vaddr) and `nm -n`, demangle with c++filt; filter stacks by a frame regex (`Views>::against`) and diff against `Views>::rest` (rest runs twice the frames). For a regression across commits, build both in a scratch worktree with a separate CARGO_TARGET_DIR and diff per-function counts normalized by the test function's samples. Inlined code is attributed to the preceding symbol (a `get_mut::<str>` may be `get`).
