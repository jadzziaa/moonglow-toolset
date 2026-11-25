---
type: Decision
title: "Area view: clicks_place and to_place decide whether a click places; Paint swallows only the placing click"
description: "The one rule for whether a click on the area view places what is in hand, which every placing path goes through, and where Paint's swallowing check lives."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T08:44:14Z }
touches: [crates/mg-ui/src/area_view.rs, crates/mg-ui/src/paint_mode.rs, "crates/mg-ui/src/blueprint/mod.rs:mixed_cue"]
asks:
  - Why does a paste click do nothing with Paint on and a door chosen?
  - "With Paint on and a door chosen, why does clicking the ground place nothing?"
  - Where is the check that stops a click placing while Paint is on?
  - Why can I still select and drag objects with Paint on and nothing chosen?
  - "Why does the mixed-markers note say more than 65, not 64?"
fingerprint: v2-94d49b674b539440
created: 2026-11-25
author: augustaugur
---
## Decided (owner's ruling 5, 2026-11-25)
Paint on with nothing paintable chosen swallows only the click that would place what is in hand (a palette door or waypoint, a gallery appearance or light, the start location), not all input: selecting, box select, dragging, Delete, copy and paste work as in Select Objects; Escape or a right click turns Paint off; a double click on the ground goes to Select Tiles and turns Paint off.

- `clicks_place(app)` (`!app.paint`) and `to_place(app)` (something is in hand) in `area_view.rs` are the single rule. `PressTo::of`, `brush()`, `ghost()`, the door hooks and the start-location preview all go through `clicks_place`, so no ghost shows where a click would place nothing.
- The swallow (`press == PressTo::Objects && !clicks_place && to_place && clicked/double/triple`: return, on a writable view only) lives in `area_view::input`, after the Pasting/Redraw/Select Tiles dispatch, not in `paint_mode::input`, which only takes Escape or a right click to put Paint away (not while a paste, redraw or tile paste is in hand: that drops first, Paint stays on).
- `alt_pans` uses `paint_mode::active` (Paint with placeables chosen); Paint with none is Select Objects, so Alt + drag raises the selection.

## Tried and ruled out
- Wave 2 sent every press to `PressTo::Paint` whenever `app.paint`: with nothing chosen the view went dead.

- The swallow placed before the paste/redraw dispatch ate those clicks too (found in the second review round).

## Watch out
- `MIXED_LIMIT` (blueprint/mod.rs) counts the objects edited beside the first, so the cue says `MIXED_LIMIT + 1` ("more than 65 selected"); it is derived from the constant, not written.
