---
title: "2DA column widths and the read-only view's scroll bars"
author: August
date: 2026-10-21
branch: worktree-agent-a8101311a14da69fe
cost: { minutes: 164, tokens: 1198207 }
touches:
  - crates/mg-ui/src/table_view.rs
  - crates/mg-ui/src/browser.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/screens.rs
---

## Decided
- Column widths live in `Settings::table_widths` (table name -> column name -> points, last 200 tables), not on `TableDoc`: the 2DA editor and the resource browser's read-only view (`browser::two_da`) read the same widths, and they survive restarts. Helpers in table_view.rs: `column_widths`, `set_column_width`, `column_edge` (the resize handle), `fit_width`, `cell_shows`.
- The resize handle is a separate `ui.interact` zone over the right 4 px of a column's name and the gap after it, registered after the name so it is on top: a drag begun there is the edge's, one on the rest of the name still moves the column (`moving`). It follows the pointer from where it was pressed (`press_origin` minus the edge, kept in temp data), not `drag_delta` sums, which drifted a few points.
- The read-only view was rebuilt on the editor's layout: a header ScrollArea (hidden bar, `ScrollSource::NONE`) and a numbers gutter that follow the rows' `ScrollArea::both` offset a frame behind (offset kept in ctx temp data under `("2da-view", key)`).

## Watch out
- The old view put a `ScrollArea::vertical` inside a `ScrollArea::both`: the inner one grows as wide as its rows, so its vertical bar is painted off at the table's far right, out of sight. Yet in kittest a press at the pane's right edge still scrolled it, so a press test alone passed on the broken view: `a_game_table_read_only_scrolls` renders (wgpu, `gpu::hold`) and looks for the bar's track (`extreme_bg_color`) with the handle (`widgets.inactive.bg_fill`) at its top. The window's border stroke is the handle's gray too: require the track color as well.
- kittest: a Label's text is its accesskit `value()`, not `label()`. A double click needs its four pointer events a frame each with `with_step_dt(1.0/60.0)`; all four in one frame was not taken as a double click on a `ui.interact` zone.
