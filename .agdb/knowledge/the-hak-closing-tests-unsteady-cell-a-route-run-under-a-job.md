---
type: Decision
title: "The hak-closing test's unsteady cell: a route run under a job without a race"
tags: []
status: stable
generated: { by: trail, at: 2026-11-12T00:00:00Z }
touches: [crates/mg-ui/tests/app/hak_closing.rs]
fingerprint: v2-035bbab246ed2507
created: 2026-11-12
author: August
description: "The flaky WhileSaving/Module/*/WindowX/Cancel cell in every_way_of_closing_a_hak_with_a_file_added was a test race, not a product bug; the fix sends the click-push-release as one input event and uses settle/wait_for instead of frame counts."
asks:
  - "Why does every_way_of_closing_a_hak_with_a_file_added fail about 1 run in 15 to 40, and is the product at fault?"
  - "How do I make a kittest action run reliably while a Save Hak job is still in progress?"
  - "My test pushes a route under a background job but the job sometimes finishes first and the route isn't refused; what's wrong?"
  - "Should hak_closing.rs wait a fixed number of frames or use settle and wait_for with SETTLE_TIME?"
---
## Decided
- The unsteady cell of `every_way_of_closing_a_hak_with_a_file_added` was the test's, not the product's: `WhileSaving/Module/*/WindowX/Cancel` (the window's X pressed during the Save Hak job, "Close Moonglow?" answered Cancel). The old block clicked Save, stepped, pushed the route and stepped: sometimes the job's thread ended before the route's frame, `poll_job` finished it at that frame's start, and the route was not refused. The verdict took "not refused" only with the goal reached, which Cancel rightly does not reach. The product was right in every failure seen (about 1 run in 15 to 40, by load).
- To run an action under a job every time: press the button and step, push the action, release the button as ONE input event (`h.event(PointerButton)`) and step once. The Save is taken while the editor is drawn, so the job starts before that frame's `run_actions`, and a job only ends at a later frame's `poll_job`. The verdict now requires `busy true, refused true, quit false` and a retry in every WhileSaving cell.
- Every wait on a count of frames in `hak_closing.rs` became `settle` or `wait_for(h, what, condition)` (settles until the condition holds or `SETTLE_TIME` runs out).

## Tried and ruled out
- Accepting "not refused and done" as a legal outcome (the 2026-11-11 learning's advice): that allowance is what failed, and it is gone. The save ending before the route is covered by the `EditorSaveFirst` cells.
- A product race from a hak's question answered Save under its own Save job: `closing_window` does not draw a hak's question while Save Hak runs, the clean tab closes once the job ends, and `carry_on` is reached every frame.

## Watch out
- A kittest step is one frame per queued input event: a node's `click()` queues three (move, press, release), and `common.rs`'s `press` three too (the modifiers, then the button). A job started by a click has frames to finish in before the next thing the test does.
- `trail check` flags the `pointer` helper as a Ctrl+click sent without `ModifiersChanged`: it has no modifiers on purpose.
