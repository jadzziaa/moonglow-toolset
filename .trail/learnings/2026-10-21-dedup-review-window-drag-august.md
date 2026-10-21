---
title: "A review for duplicated code, and windows of tabs moved by their tab bar"
author: August
date: 2026-10-21
branch: develop
touches:
  - crates/mg-ui/src/tabs.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-gff/src/value.rs
  - crates/mg-module/src/vars.rs
  - crates/mg-module/src/areas.rs
  - crates/mg-ui/src/script_view.rs
  - docs/deferred.md
---

## Decided
- A window of tabs (egui_dock's floating windows) moves by its tab bar alone: egui_dock builds them with `title_bar(false)`, and egui then moves a window by any part (`WindowDrag` falls back to Anywhere). One drag-catcher behind every tab's body in a floating window (`TabViewer::ui` in `tabs.rs`, layer order not Background) stops it; Options › General › Move windows by dragging anywhere in them brings it back. egui's own dialogs already moved by their title bar alone.
- The rule for a duplication review: fix only the copies that have drifted into a wrong result, through the one helper the copies should share, with a test that fails without the fix. Everything else goes into `docs/deferred.md` (Written more than once) with every copy's file:line, for the owner to choose. Unifying where the right behavior is unknown (resref rules, palette name order, Copy's resref, the portrait filters) waits for an engine test or an Aurora capture.
- The shared helpers made: `Value::from_text` (mg-gff; typed text in the codepage), `mg_module::vars::VarType` (with `value`, `of_text`), `mg_rules::rows::generic_door_field`, `ResRef::cut` (a model's names cut to 16), `mg_image::tga::read_picture` (by the declared origin), `GameInstall::find_root` (the tests find the game there too), `mg_module::rename::recompile`, `script_view::script_source`, `script_tools::is_match_at`, `TableDoc::has_unsaved`, `AreaView::lift_of`, and `commit_text_in` with a character limit (every set-on-leave text field goes through it, `resref_field` too).

## Watch out
- `widgets::enter` takes the Enter press (`consume_key`): one press does one thing. A dialog with Next and Finish both asking it in one frame went on and finished at once. A field that handles its own Enter (Save Set's name) must take the press too, or the dialog's OK sees it.
- A Python `open(p,'w').write(f(p))` truncates the file before `f` reads it: read first, then write.
- Delete in a text field reaches the area view's input too: the area view checks `typing` (as the tile selection did).
- To see a test catch a bug, take the fix's one line out, run it, put it back (done for the resref field and Delete).
