---
type: Gotcha
title: "Journal and faction editor (U22): matrix keys by the active editor, Tab kept at frame start, talk-table names, AccessKit boxes in pixels"
description: "What U22 learned building the journal's and faction editor's panels, matrix and Outliner, and the traps its tests fell into."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-18T17:30:11Z }
touches: [crates/mg-ui/src/faction_view.rs, crates/mg-ui/src/journal_view.rs, crates/mg-ui/src/outliner.rs, crates/mg-ui/tests/app/journal_factions.rs]
asks:
  - Why does Tab move the focus out of the faction matrix?
  - Why did the journal tree show empty category names?
  - Why are the faction tests failing to click a faction in a floating window?
  - "How do I measure a panel's height at a zoom in a kittest test?"
fingerprint: v2-957cb3b5bf31d6a3
created: 2026-11-18
---
## Decided
- The faction matrix's cells are `Sense::CLICK | Sense::DRAG`, not focusable: the matrix holds the keys by `matrix_has_keys` (a click or F6 gave them, the editor is the active one, no modal, popover or Add/Change Name window). Tab is consumed at frame start by `faction_view::keep_tab_for_the_matrix` (called in lib.rs after `table_view::keep_tab_for_the_grid`): egui moves the focus on Tab before any widget is drawn, so consuming it in the matrix's own code is too late.
- Typing in a cell is a TextEdit with `egui::Frame::NONE` (`.frame(false)` is gone in egui 0.36); a drag commits once as it is let go (one Undo step).
- The journal tree's text goes through `journal_view::tree_text`, `widgets::loc_shown`'s order (own text, the talk table, elsewhere): the official modules' journals are StrRefs. The Seen cache is keyed by revision, edit language and `app.load_order`.
- Panel forms in a narrow pane use `blueprint::narrow_form` so their fields fill the room instead of overflowing.
- Basic keeps the chart (the moves called it Not Drawn): the switch's Basic has nothing else to show. Basic brings Faction and Reaction, Advanced adds Reactions.
- The Faction Editor now brings its own Right Dock panels: tests that took it as "a tool that brings no Right Dock" (right_dock.rs, windows.rs) were changed.

## Watch out
- (Corrected in the U22 fix round.) A floating editor opened at a window 1100 to 1280 wide put its top-left resize handle (egui's corner grab, 10 points past the window's edge) over the folded Left Dock's "Show the Left Dock" chevron: a click on the chevron took hold of the window instead. Outliner rows were never the problem (the review could not make a row click miss): the strip's Outliner button opens it and rows click. Fixed in `Moonglow::open_window`: a floating window's first position is placed clear of a folded dock's strip (`Moonglow::strips`, recorded by `shell::left_dock` and the right dock), its size still fitted to the screen. The fields.rs faction tests choose through the Outliner again.
- AccessKit `bounding_box()` is in pixels; kittest `Node::rect()`, `right_dock_rect` and `tab_rect` are in points. At a zoom, divide the box by `pixels_per_point()` or every comparison is off (the sweep test read 0 heights).
- `get_all_by_role` panics when nothing matches; use `query_all_by_role` where none is a valid answer.
- The faction matrix's cells are AccessKit Sliders: a query for the Reaction panel's slider must be held to the Right Dock's rect.
- A pasted journal entry keeps its ID: two rows "[0002] …" after a paste.
- `blueprint::follow` finds a journal struct by its place and content: set the whole Categories list with one category's entries changed and an entry inside it is not followed (its typed text is dropped). The editor's own edits set a category's EntryList, which follows; a test must use the same edit.
- frame_perf: a case that unfolds the Right Dock on its own panel (ShowPanel(Entry/Reaction)) must fold it again if it found it folded, or the blueprint views after it ask for the Preview and find the Palette in front.
- In tests/screens.rs the harness keeps no AccessKit boxes (`Node::rect` panics "Every egui node should have a rect"); place the pointer from `dock.iter_leaves()`'s leaf rect, not `tab_rect` (the tab's button).
- chrome_colors fails on any `Color32::from_*` or named color in a view: use theme tokens (`t.on_fill.gamma_multiply(..)`, `ui.visuals().text_color()`).

