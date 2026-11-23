---
type: Decision
title: "The shared text right-click menu (5.2 W2): one global hook, how it keeps selection and focus"
description: "How widgets::text_menu gives every egui TextEdit Cut/Copy/Paste/Select All, and the egui and kittest traps it works around."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T13:43:40Z }
touches: [crates/mg-ui/src/widgets.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/script_view.rs]
asks:
  - how does a text field get its right-click menu
  - why does a right click in a text field lose the selection
  - how do I test the text field context menu with kittest
  - how is a read-only text field told apart from an editable one
fingerprint: v2-4d25d475661a5344
created: 2026-11-23
author: augustaugur
---
- One global hook, not per-field calls: `widgets::text_menus_begin` (first line of `Moonglow::ui`) and `text_menus_end` (before `dialogs_end`) give the focused egui TextEdit the menu: about 100 call sites, others' files and kit pages included. An owner-drawn field opts out with `own_text_menu(ctx, id)` and calls `text_menu(ctx, id, Some(TextUndo{..}), extra)` itself; only the script editor does (Undo/Redo through `script_tools.undo`, Convert Indentation, plugin commands). Plain fields get no Undo/Redo: the module's Undo owns Ctrl+Z there.
- egui moves the caret on ANY button's press and a field without focus collapses its selection, so `text_menus_begin` snapshots the range on the secondary press and `keep_selection` puts it back if the caret landed inside [min, max] (inclusive: the edge counts as inside).
- egui surrenders focus on a click elsewhere; while the menu is open `options.input_options... surrender_focus_on = Never` (restored on close), else a commit-on-blur field commits and the keys go nowhere. `set_focus_lock_filter` keeps Escape/Tab/arrows from moving focus; the begin hook takes Up/Down/Enter/Space/Escape/Tab as the menu's.
- A row's action is replayed into the field at the next frame's start as the event its key sends: `Event::Copy`/`Event::Cut`, Key A + COMMAND, `Event::Paste` after `ask_paste`/`asked_paste` (the existing clipboard path).
- Read-only: a focused mutable TextEdit sets `PlatformOutput.ime`, a `&mut &str` one does not; fields `typing_kept_out` (a viewed blueprint) register through `text_kept_out`.
- kittest: each queued event gets its own frame and `h.output()` is only the last frame's, so push press/release into `h.input_mut().events` and `step()`; kittest ignores `RequestPaste`, send `Event::Paste` yourself; "at rest" is `repaint_delay != 0` (the caret blink keeps a repaint requested). Helpers in tests/app/fields.rs: `right_click_at`, `text_menu_rows`, `choose_text_row`.
