---
type: Decision
title: "Hak editor (U21): Preview follows the selection, caches keyed by Hak::changes"
description: "How the U21 hak editor keeps its table, facts and Preview fresh without per-frame walks (Hak::changes), and the traps found: Top Bar Undo off with typed text, the rename field re-grabbing focus, piled-up model views, Find's field width."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-18T10:02:16Z }
touches: [crates/mg-ui/src/hak_view.rs, crates/mg-ui/src/hak_view/panels.rs, "crates/mg-module/src/hak_edit.rs:Hak::changes", crates/mg-ui/src/side_doc.rs]
asks:
  - "The hak editor's table or Outliner counts are stale after a change made directly to doc.hak: where are they refreshed?"
  - "Why does the Top Bar's Undo do nothing after typing a hak's Description?"
  - "Rename… in the hak editor never ends on Enter: why?"
  - "Why does arrowing through a hak's models not keep a 3D view for each?"
fingerprint: v2-2fe009f5d8d23afe
created: 2026-11-18
---
## Decided
- Everything made of a whole hak (the table's rows for a Find and an order, `Facts`: count, size, the 2 GiB count, the Outliner's types) is cached by `Hak::changes()` (mg-module), which moves in `apply`, `revert` and `saved`. A test or a command that edits `doc.hak` directly is seen without calling `HakDoc::changed`; `hak_view::follow_all` (each frame's start) lets go of what was made (selection pruned, load order, description buffer) when the count moved. `HakDoc::walks()` counts every whole-hak walk for the tests.
- The Preview shows `preview_key()`: the anchor (clicked last) if selected, else the first selected; Close stores `closed`, cleared by any click or View. Its `Viewing` is made once per (key, changes). Only the shown model's `Source::Hak` view is kept (`retain` in `preview`), and views of closed haks go in `follow_all`.
- The Outliner's click filters the table (Find ".mdl"), the board's; again clears.
- Save stays the module's Ctrl+S (owner's ruling 2026-11-18): no Save hint.

## Watch out
- A Description typed and not set must count as unsaved (`HakDoc::is_dirty`) and as an Undo step (`SideDoc::step_name` says "Description" while typed): after a Save the hak's undo stack is empty, so the Top Bar's Undo was disabled and its click did nothing but set the text.
- The rename field asked for the keyboard every frame, so Enter never ended it (also at the tip): `rename_focus` asks once.
- `widgets::SearchField::width` is the text's width: a header item of 196 needs `.width(196 - 24)` or its clear button spills over the next item.
- `show_dock_panel(Preview)` set `palettes_asked`; a kind whose Preview is its own (`Brings::diagram`: tileset, hak) no longer brings the palettes with it.

## Review fix round (2026-11-18)
- A previewed sound plays only while its Preview is drawn: `HakDoc::preview_pass` (set as `preview` draws) checked in `follow_all`; `Moonglow::hak_sound` (hak id, module path) stops it with the hak closed or another module.
- Hak model views (`Source::Hak`) now go by `model_view::let_go_of_unseen`'s 2 s rule like any other (they were exempt, and outlived Close and a folded dock).
- The Outliner's type keeps what Find held (`type_find`, `find_before`): its second click puts it back; Find typed over is the builder's own (`follow_all` lets go of both). Equal counts by ResType number (the board's .are before .gic).
- With a Description pending there is nothing to redo (`SideDoc::step_name(redo)` None).
- `header_holds` reads AccessKit pixel rects: at a zoom it misplaces rows; test zoom with the header record and node rects (points) instead.
- `SearchField::clear_inside()` draws the clear × in the field's box (opt-in; the other editors still draw it after the field).

