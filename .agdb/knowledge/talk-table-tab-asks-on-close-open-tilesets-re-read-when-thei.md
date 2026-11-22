---
type: Decision
title: Talk table tab asks on close; open tilesets re-read when their file changes (M5 W5)
description: "Rulings 35 and 44 built: closing the Talk Table tab with lines changed in its own editor asks Save/Don't Save/Cancel through side_doc::closing_window (also with dialog.tlk shown, by the fix round's ruling; not for lines typed in other editors alone, never on quit/close/open which the module's question covers); tilesets open in the tileset editor are watched (outside.rs reload_changed_tilesets) and re-read, with unsaved edits asked about in Changed Outside Moonglow."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T16:17:03Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-21T17:07:47Z }
  - { by: agdb/0.2.0, at: 2026-11-21T21:12:53Z }
  - { by: agdb/0.2.0, at: 2026-11-21T21:59:33Z }
  - { by: agdb/0.2.0, at: 2026-11-22T00:15:20Z }
touches: ["crates/mg-ui/src/side_doc.rs:Moonglow::talk_asks", "crates/mg-ui/src/side_doc.rs:closing_window", "crates/mg-ui/src/outside.rs:Moonglow::reload_changed_tilesets", "crates/mg-ui/src/tileset_view.rs:changed_on_disk", "crates/mg-ui/src/outside.rs:window"]
asks:
  - "Why does closing the talk table tab ask Save or Don't Save, even when dialog.tlk is shown?"
  - "Quitting with unsaved talk table lines asks twice: where is the talk table kept out of ask_side_docs?"
  - "A tileset edited in another program doesn't update in the tileset editor: what watches it?"
  - "Tileset Save refuses with 'changed outside Moonglow': how is that conflict answered?"
fingerprint: v2-5e64b9af63ae5c8a
created: 2026-11-21
author: augustaugur
---
## Decided (owner, 2026-11-21: "35: save or discard question on closing", "44: reread on change")

**Talk table close.** Every way the tab closes (×, Ctrl+W, tab menu Close/Close Others/Close All, the floating window's ×, the header's Cancel) reaches `may_close` → `may_close_side`, which now handles `Tab::TalkTable` by `Moonglow::talk_asks`: the table is editable and `Table::has_own_unsaved` (changes made in its own editor, not `typed_unsaved`). The question is the side editors' one (`closing_window`, "Unsaved Changes", Save / Don't Save / Cancel, Escape = Cancel); Save is `talk_view::save` (writes a file table; a module-kept table is put into the module and the question says so: `talk_view::CLOSE_SAVE_NOTE`), Don't Save is `forget_side` (drops the editor's own changes, not undoable, as for the other side editors; lines typed in other editors stay: `CLOSE_TYPED_NOTE`). Closing the module, opening another and quitting do NOT ask the tab's question: the talk table is counted in `has_unsaved_work` and the module's own question covers it (never add it to `ask_side_docs`). With `dialog.tlk` shown in front it asks too, naming the table behind (the first build closed unasked there; the fix round's ruling: ruling 35 has no exception). `side_doc::close` now notes the tab for Reopen Closed Tab.

**Tileset re-read.** `OutsideState.tilesets` is a `Watch` of the open tileset docs' file paths (by their folder, so a rename-over is heard; debounced by `Watch::QUIET`), started/restarted in `reload_changed_tilesets`, called every frame from `reload_timer` before its busy/game gate (the tileset tests have no game) and gated by `no_auto_reload` only (Tools › Options › Folders › Reload haks, override and development when they change; not Options › General). Heard files go to `tileset_view::changed_on_disk` after a typed field of that tileset's own editor has set its text (`tileset_view::typing_in`, by the layer and rect each editor was drawn in the last pass, then `widgets::leave_text`, one frame deferred); another editor's field keeps the keyboard. A look at every file (the watch's start or walk: `files() == None`) reads only the tilesets whose modified time or length differ from `TilesetDoc::seen`, not every open file on the frame (the tcm02 tileset is 884 KB). `TilesetDoc.disk` is the fingerprint last read or written (own saves are no change); clean docs take the file (`take_disk`: history cleared, saved, tile/group/renaming clamped); `keep_mine` sets `disk = theirs`, so the next look at the file asks nothing; dirty docs get `theirs` and join the Changed Outside Moonglow window (which now lists module conflicts then tilesets; `outside_conflicts()` includes them). While `theirs` is set, `TilesetDoc::save` refuses, and the refusal clears `later` so the window asks again. A file that doesn't parse, or is cut short (`cut_short`: no [TILES], or a list's Count naming a missing last section, which `Tileset::parse` accepts silently), keeps the last good state and warns once.

## Watch out
- Areas and the tiles palette draw tilesets from the game data, not from the editor's doc: a .set in the override, a development folder or a hak is re-read for them by the content watch (`reload_changed_content` → `game_data_changed` → `AreaView::reload`), tested by `a_tileset_in_the_override_changed_outside_is_redrawn`. A .set kept in the module itself is not refreshed in area views: `update_module_layer` changes the module layer in place without dropping the tilesets or the areas (filed in docs/deferred.md, Tileset authoring).
- Problems' staleness follows the editor's text (`follow_problems` each frame), so a deleted file does not mark findings Missing while the doc stays open.
