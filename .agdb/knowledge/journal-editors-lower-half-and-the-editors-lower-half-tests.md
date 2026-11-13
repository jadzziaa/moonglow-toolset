---
type: Decision
title: "Journal editor's lower half and the editors' lower-half tests"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [crates/mg-ui/src/journal_view.rs, crates/mg-ui/tests/app/windows.rs, crates/mg-ui/tests/screens.rs]
fingerprint: v2-eb431e9e331e430f
created: 2026-10-29
author: August
description: "A resizable egui bottom Panel in a tab shrinks to its content and ignores drags; fix with ui.set_min_height(ui.max_rect().height()), and test height via PanelState::load outer_rect, dragging 2 points below rect.top()."
asks:
  - "Why does my resizable egui bottom panel snap back to its content height after I drag it taller inside a tab?"
  - "How do I make the journal editor's fields pane keep its height when a selection like Root has little to show?"
  - "How can I read a panel's height and max_size in a kittest egui test?"
  - "Why does a drag on an egui panel's resize handle do nothing in kittest when starting at rect.top()?"
---
An egui Panel::bottom(...).resizable(true) shown inside a tab is as tall as its content: dragged taller it snaps back, its default_size is not kept, and a selection with less to show shrinks it (the journal editor's fields pane was 20 points high with Root chosen). The fix is the conversation editor's: ui.set_min_height(ui.max_rect().height()) first thing in the panel's show. To test a panel's height and its max_size, read egui::containers::panel::PanelState::load(&h.ctx, id).outer_rect with the panel's own Id, and drag from (rect.center().x, rect.top() + 2.0): at rect.top() exactly the press missed the resize handle in kittest; the 2 points come off the distance dragged.
