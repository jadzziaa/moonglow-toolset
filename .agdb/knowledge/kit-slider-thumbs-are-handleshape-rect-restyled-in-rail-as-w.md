---
type: Gotcha
title: "Kit: slider thumbs are HandleShape::Rect restyled in rail_as_well; a focused button is already Pressed"
description: "Why restyling the slider thumb by matching circles fails, why an extra focus ring doubles a button's ring, and how the kit sheet shows focus."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T08:44:15Z }
touches: ["crates/mg-uikit/src/controls.rs:rail_as_well", crates/mg-uikit/src/icons.rs, crates/mg-uikit/src/theme.rs, crates/mg-uikit/tests/kit/sheet.rs]
asks:
  - Why is the slider thumb ringed white in the dark theme?
  - Why does a focused toggle that is on show two rings?
  - "Where is the slider thumb's ring painted?"
  - Why does the kit sheet show only one focused control per frame?
fingerprint: v2-30f56fa9f4b2ed7f
created: 2026-11-25
author: augustaugur
---
- The theme sets `handle_shape = HandleShape::Rect`, so egui paints a slider's thumb as a `Shape::Rect` (not a circle) stroked with the widget's `fg_stroke`, the value text's color: near white in dark, near black in light. Restyle it in `rail_as_well` (after the slider paints, by its fill `slider_thumb`): a match on `Shape::Circle` alone changed nothing. Dark has no ring (`slider_thumb_ring` transparent), light 1 px `#8d949e`.
- `ButtonState::of` returns `Pressed` on focus as well as on press, and `Pressed` already draws the 1 px accent ring on a button off the accent fill. Adding a focus ring there doubles it; on the accent fill the focus ring is the 1.5 px `on-fill` ring inside the edge (`paint_on_fill_focus`) and no accent ring. One ring per control.
- Tests: the kit sheet can show one real keyboard focus per frame (egui holds one), so its focus strip renders one frame per control.
