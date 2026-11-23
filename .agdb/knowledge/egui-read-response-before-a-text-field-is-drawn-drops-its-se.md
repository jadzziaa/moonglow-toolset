---
type: Gotcha
title: egui read_response before a text field is drawn drops its selection
description: "Why Context::read_response must not be called for a TextEdit before it is drawn in a frame with a click, and the side-effect-free way to tell a right click."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T14:23:44Z }
touches: ["crates/mg-ui/src/widgets.rs:secondary_clicked_on", "crates/mg-ui/src/widgets.rs:focus_in_sight"]
asks:
  - why does the script editor lose its selection when I click Replace
  - is it safe to call read_response every frame
  - how do I know a widget was right-clicked without its response
fingerprint: v2-d216fd9368a98ddd
created: 2026-11-23
author: augustaugur
---
- egui 0.36 `Context::read_response` goes through `get_response`, which surrenders the widget's focus when a click lands outside it (context.rs ~1553). Read before a TextEdit is drawn, the field loads unfocused and collapses its selection to the caret (builder.rs ~554, `owns_ime_events` is `has_focus`), so a button beside it (the script editor's Replace) sees no selection. W2's `text_menu_wanted` did this every frame; W3 found it from a failing Replace test.
- Tell a right click with no side effect: `ctx.interaction_snapshot(|s| s.clicked == Some(id) || s.long_touched == Some(id))` plus `i.pointer.button_clicked(Secondary)` (`widgets::secondary_clicked_on`), as `get_response` sets `secondary_clicked`.
- Where a response is still needed, read it only where no click can be elsewhere (a right click on the field itself, which is hovered) or after the field is drawn, or while focus surrender is off. `focus_in_sight` guards the same trap by skipping frames with a press or click.
- Test: `widgets::text_menu_tests::a_click_beside_a_text_field_keeps_its_selection`.
