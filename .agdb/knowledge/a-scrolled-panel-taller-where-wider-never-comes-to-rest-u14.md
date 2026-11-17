---
type: Gotcha
title: "A scrolled panel taller where wider never comes to rest (U14 Data, found in U15)"
description: "Why a conversation's Data panel redrew without end beside its scroll bar: content taller when wider flips the bar every frame; the form row tested room without the row's gap."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-17T04:28:16Z }
touches: ["crates/mg-ui/src/dialog_view.rs:form_row_least", "crates/mg-ui/src/dialog_view.rs:resref_row"]
asks:
  - "The window keeps repainting with a scroll_area.rs repaint cause beside a panel: why?"
  - Why did the Data panel never come to rest at 319 to 322 points of Right Dock?
  - How should a form row decide to put its label over the field?
  - How do I test that a scrolled panel always comes to rest?
fingerprint: v2-743491bca6e9c2f1
created: 2026-11-17
---
## The shape
A vertical ScrollArea whose content is TALLER when the area is WIDER never rests where the content height lands between the two layouts: with the scroll bar (narrower) it fits without one, without the bar (wider) it needs one, so egui toggles `show_scroll` every frame (repaint cause `egui scroll_area.rs:1524`, "if show_scroll_this_frame != state.show_scroll"). Narrower-is-taller is fine; wider-is-taller is the bug.

## The cause here (M4 U14, found in U15's review)
`dialog_view::form_row_least` put the label beside the field when `available - label_width >= least`, but `ui.horizontal` then takes `item_spacing.x` (4) between label and field, so the field got 4 less than tested; `resref_row` then found field + picker + Edit did not fit and wrapped Edit onto its own line (`horizontal_wrapped`). At 311 (bar shown) the label stood over the field: 654.8 high; at 319 (no bar) beside it with Edit wrapped: 675.8; viewport 667. Fixed by testing the room actually given (`available - width - item_spacing.x`).

## Watch out
- U14's rest test passed because its chosen line's Data was shorter than the viewport in both layouts; rest tests must cover lines whose content lands near the viewport, or better, assert monotonic height.
- Test: `conversation_panels::data_is_never_taller_where_wider_and_comes_to_rest` sweeps the dock 270..480 by 1 at three text sizes on three kinds of line, asserting the height never grows with the width, and settles around each layout change at several window heights.
- Any row that decides layout from `available_width()` must subtract the spacing the row will insert before the part it is sizing.
