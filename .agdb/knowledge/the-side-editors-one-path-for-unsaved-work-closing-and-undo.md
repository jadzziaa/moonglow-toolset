---
type: Decision
title: "The side editors' one path for unsaved work, closing and undo"
tags: []
status: stable
generated: { by: trail, at: 2026-10-21T00:00:00Z }
touches: [crates/mg-ui/src/side_doc.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/commands.rs, crates/mg-ui/src/tabs.rs, crates/mg-ui/src/table_view.rs, crates/mg-ui/src/talk_view.rs]
fingerprint: v2-b0d3019e7ddd3cee
created: 2026-10-21
author: August
description: "The hak, 2DA and tileset editors share one SideDoc trait for unsaved-close prompts and undo; closing queues in side_closing, and Undo routes via local_undo and Id::enabled, so Ctrl+Z over a side editor never undoes the module."
asks:
  - "How do the hak, 2DA and tileset editors handle closing a tab with unsaved changes?"
  - "Why does Ctrl+Z do nothing when the pointer is over a hak editor with nothing to undo?"
  - "Where do I add a new side editor so it gets save prompts and undo support?"
  - "Why is Ctrl+Z ignored by the keymap while I'm typing in a 2DA cell, and where does it go?"
---
## Decided
- The hak, 2DA and tileset editors are one kind: `side_doc::SideDoc` (tab, title, has_unsaved, saved_with_module, can_undo, undo, and an associated `save(app, id)`, which needs the app: dialogs, log, install). The Tab is the id; `Moonglow::side_doc(tab)` / `side_docs()` dispatch over the three Vecs, in the order the module's Save needs (tables first: a hak's table is saved into the hak before the hak is written).
- A tab closed with unsaved changes is queued in `Moonglow::side_closing` (a VecDeque); one window asks of the front, as the module's prompt does (Save, Don't Save, Cancel). Cancel clears the queue. A queued doc saved meanwhile just closes.
- Undo and Redo go only through the keymap and `Action::Undo`/`Redo`; `Moonglow::local_undo` takes them first for `undo_under_pointer` (set by `tabs.rs` for the side tabs and by talk_view for the talk table) else `undo_keys` (the 2DA grid with the keys). Both are reset each frame where the dock is drawn. With the pointer over such an editor and nothing of its own to undo, nothing is undone (not the module's), the talk table included.

## Watch out
- `commands.rs` runs a key's command only if `Id::enabled`: Undo's enabled asks `local_can_undo` first (Some = a local editor owns it), else the module's. Without that, Ctrl+Z over a hak with no module step is swallowed.
- Frame order: `shortcuts()` runs before the dock draws and actions run at the end of the frame, so the action reads the pointer/keys state written by this frame's dock draw. Don't move the reset.
- The 2DA's `can_undo` is false while a cell is typed: a typed cell's Ctrl+Z is skipped by the keymap (`in_text`) and reaches the text field.
