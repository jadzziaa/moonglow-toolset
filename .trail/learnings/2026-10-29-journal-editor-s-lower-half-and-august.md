---
title: "Journal editor's lower half and the editors' lower-half tests"
author: August
date: 2026-10-29
branch: worktree-agent-ae0036152cca4bf67
cost: { minutes: 1710, tokens: 13408751 }
touches:
  - crates/mg-ui/src/journal_view.rs
  - crates/mg-ui/tests/app/windows.rs
  - crates/mg-ui/tests/screens.rs
---

An egui Panel::bottom(...).resizable(true) shown inside a tab is as tall as its content: dragged taller it snaps back, its default_size is not kept, and a selection with less to show shrinks it (the journal editor's fields pane was 20 points high with Root chosen). The fix is the conversation editor's: ui.set_min_height(ui.max_rect().height()) first thing in the panel's show. To test a panel's height and its max_size, read egui::containers::panel::PanelState::load(&h.ctx, id).outer_rect with the panel's own Id, and drag from (rect.center().x, rect.top() + 2.0): at rect.top() exactly the press missed the resize handle in kittest; the 2 points come off the distance dragged.
