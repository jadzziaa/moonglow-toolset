---
type: Decision
title: "A review for duplicated code, and windows of tabs moved by their tab bar"
tags: []
status: stable
generated: { by: trail, at: 2026-10-21T00:00:00Z }
touches: [crates/mg-ui/src/tabs.rs, crates/mg-ui/src/widgets.rs, crates/mg-gff/src/value.rs, crates/mg-module/src/vars.rs, crates/mg-module/src/areas.rs, crates/mg-ui/src/script_view.rs, docs/deferred.md]
fingerprint: v2-0da15c7bead37936
created: 2026-10-21
author: August
description: "Records a duplicated-code review: floating egui_dock tab windows drag by tab bar via a drag-catcher in TabViewer::ui, only drifted copies get fixed through shared helpers, the rest is deferred, plus Enter, Delete and test pitfalls."
asks:
  - "Why can I drag a floating tab window by clicking anywhere in its body, and how do I make it move only by the tab bar?"
  - "Which duplicated code did the review fix, and which copies were left in docs/deferred.md?"
  - "What shared helpers exist now for things like resref cutting, TGA reading, or committing text fields, so I don't write another copy?"
  - "Why did a dialog with Next and Finish both reading Enter finish immediately, and how is that prevented?"
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
