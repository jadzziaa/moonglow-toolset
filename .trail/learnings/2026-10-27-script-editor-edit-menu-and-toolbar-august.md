---
title: "Script editor: Edit menu and toolbar Undo are the script's"
author: August
date: 2026-10-27
branch: script-undo-menus
cost: { minutes: 1197, tokens: 11183070 }
touches:
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/script_tools.rs
  - crates/mg-ui/src/script_view.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/screens.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/02-main-window.md
  - docs/manual/07-scripts.md
---

## Decided
- Edit › Undo / Redo and the toolbar's go to a script's own egui history (not the module's) while its editor has the keys: `script_view::undo_target` sets `Moonglow::undo_script` once a frame at the top of `ui()`, before `shortcuts` and the menu bar (not in `shortcuts`, which `busy()` skips). `Id::run`, `label` and `enabled` read it first; the step is a pending `script_tools.undo`, taken by the editor where `convert_indent` is (`step_history`: `TextEditState::undoer().undo/redo(&(cursor, text))`, as egui's own Ctrl+Z), and the editor gets the keys back (`request_focus` after the TextEdit is shown).
- Not (b), a script's typing as module steps: typing is not a module edit until the script's Save, and the module's Undo from another tab would then take back typing in a script out of sight.
- Undo/Redo are enabled whenever a script is the target (no graying): `TextEditState::undoer()` clones the whole history (up to 100 copies of the text), too dear for every frame on a big script.

## Watch out
- egui gives up a widget's focus on a click elsewhere (`surrender_focus_on: Clicks`), inside the focused widget's own interact, so when the editor is drawn. The menu bar and toolbar are drawn before the dock: on the toolbar's click frame `with_keys` still sees the editor; once a menu is open it does not. Hold the target while `app.menu_open` is Some (set at the end of `menu_bar`, so still None on the frame the menu opens).
- `request_focus` before the TextEdit is shown, on the click frame, is undone by that same click's surrender; ask for it after.
- In kittest a menu row's accesskit label carries its key: "Undo Ctrl+Z", "Undo Earlier Ctrl+Z" (a `query_by_label("Undo Earlier")` finds nothing either way).
- Side editors: the toolbar's Undo after the pointer was over a hak's editor is the module's (the pointer is over the toolbar then); recorded in docs/deferred.md.
