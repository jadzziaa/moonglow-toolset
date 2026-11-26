---
type: Gotcha
title: "NUI Creator: the variant's window and a long chart kept under the window's generation (N13, N16 closed)"
description: "Why the variant's context document and a chart's series are keyed by window_gen (a global stamp) plus Interact's session stamp instead of a writer-bumped revision, how the canvas borrows the kept context, and the test traps met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T20:49:32Z }
touches: ["crates/mg-ui/src/nui_view/layouts.rs:kept_context", crates/mg-ui/src/nui_view/preview/chart.rs, "crates/mg-ui/src/nui_view/preview/stage.rs:canvas", "crates/mg-ui/src/nui_view.rs:unsettle"]
asks:
  - how is the NUI variant context document cached
  - why does the NUI chart not walk a million samples each frame
  - which writers bump the NUI window revision
  - why is window_gen a global counter
fingerprint: v2-e664b1cfe6951c3b
created: 2026-11-26
author: augustaugur
---
- The deferred entry asked for a revision that every writer of the window bumps. None was needed: `window_gen` (`settle`/`trusted`, built 2026-11-26) is a positive proof, so no writer can forget it. Any frame not proven quiet (end()'s `can_skip_diff` on the frame before, no input up to the canvas) is a new generation. Module bytes changed between frames (Undo/Redo, plugins, a reload, Apply SetResource, Split source and Binding JSON set) change `copies` and so the settled identity; `store_typed` unsettles; Interact's `Session` touches its global stamp in apply/collapse/dispatch (handlers' Set/Toggle/Variant, the stand-in for script-driven values)/constrain_geometry. The key is (window_gen, the session's revision in Interact).
- `window_gen` is now drawn from a global atomic (`GENERATIONS`): the chart's cache lives in egui temp data under (source_path, row, slot), shared by every designer tab, and per-State counters from 0 could collide. Tests only compare generations with == and >.
- The canvas borrows the context: `kept_context` takes `State::context` out (a move), the Stage gets `shown = &context.doc`, and it is put back after the Stage is destructured (the Stage holds `&mut State`, so it cannot borrow from it). Not stored while a gesture (resize, draw, window move) replaced the shown window. The layout (`State::measured`) is now kept for a kept context too (not in Interact: measured's key has no session).
- A trusted hit is debug_assert-compared with a fresh build (layouts, chart), as measured's are: the UI suite is the differential.
- Perf (three runs each): a variant of 304 controls edited went from +421..452 µs over the same controls inline (load 4) to -25..-34 (load 9 to 13, its layout kept too); a chart of 1M samples, line +1.4..1.6 ms and columns +1.5..1.6 ms, to -5..+17 µs.
- Test traps: the Binding JSON text is taller than the window, so `click()` lands outside it; `focus()` the node first. Typed source is set when the source switch moves (JUI Source), not by Escape. A chart line's points move with the canvas zoom, so compare lines in the same Layout. Properties' Width is a SpinButton, not a TextInput. Removing only the revision compare does not fail the tests (`trusted` alone guards in the app); removing both does.
