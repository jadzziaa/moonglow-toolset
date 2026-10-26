---
title: "Conversation tree type-ahead, keys and bookmarks after outside changes"
author: August
date: 2026-10-26
branch: conversation-tree-rest
cost: { minutes: 920, tokens: 9431572 }
touches:
  - crates/mg-module/src/dialog.rs
  - crates/mg-ui/src/dialog_view.rs
  - crates/mg-ui/src/keys.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/log.md
  - docs/manual/06-conversations.md
---

## Decided
- Aurora (under Wine, driven 2026-10-26) matches letters typed in the conversation tree against the row's text as drawn: an NPC row begins with `[OWNER] - `, so `o` from Root skips `[OWNER] - Over here.` for `Off I go.`, and `[` goes to NPC rows. Moonglow's `row_text` now uses `row_label` (the same text `tree()` draws; with speakers hidden, the line's own). Prefix typing ("gol") and a reset after a pause were seen too; Escape does nothing in Aurora's tree.
- Type-ahead: `DialogView::{typed, typed_at}`, `TYPE_AHEAD = 1.0` s read from `i.time`; `type_ahead()`: the same letter repeated cycles (`seek` from `at+1`), else prefix from `at` inclusive. A space continues a buffer but never starts one. Any tree key (arrows, Home...) clears the buffer.
- A conversation command wins over a tree key: `Keymap::bound_in(Group::Conversation, e)`; the tree skips such Key events and Text whose char a bound key types (`typed_by`: letters, digits, +, -). General commands already win: `shortcuts()` runs before the dock is drawn. Delete Line's default has Backspace too on macOS (`cfg!`).
- Gates in `editor`: tree keys run when no *text edit* has focus (`ctx.text_edit_focused()`), not "no focus at all"; when the tree takes a key it calls `move_focus(FocusDirection::None)` (egui set the direction in `Focus::begin_pass`, before we run) and surrenders the non-text focus. Commands' `here` = (pointer over editor || tree has keys) && !typing.
- Escape: consumed and `give_arrows(false)` when `escape_past_dialogs` and the editor had no text focus last frame (`was_typing`): egui drops a field's focus on Escape in begin_pass, so the same frame looks unfocused.
- Unknown changes: `DialogView::last` keeps the Gff as last seen; when `steps()` finds no path, `Renumbering::by_lines(before, after)` (mg-module) is recorded as a `found` edge and applied. Signature = hashes of (Text, Speaker) Debug strings up the owner chain to Root. Groups alike: zip when counts equal, else only same-index. Unmatched keep their number when the kind's count is unchanged and nobody took it (protects the editor's own text edits, which are also "unknown" changes).
- `remember` drops an older edge with the same (before, after): fixes the two-identical-deletes ambiguity (undo/redo follow the one done last).

## Tried and ruled out
- Matching the line's own text (the earlier choice): Aurora doesn't.
- egui can't tell keypad − from main −, nor give keypad * as a Key: egui-winit 0.36.2 `key_from_key_code` maps `Minus | NumpadSubtract => Key::Minus`, has no `NumpadMultiply`, and ignores `location`. Moved to Settled.
- Recording identity edges for unknown changes (the old way) left bookmarks on stale numbers after a plugin's or outside edit.

## Watch out
- Wine's treeview is Wine's own: repeated same letters did NOT cycle there (`gg` stayed on Goodbye), and its pause is Wine's (450 ms in its source). Don't take Wine's type-ahead as Windows'.
- kittest advances `i.time` 0.25 s a step; `h.run()` may take several steps, so letters meant as separate searches need `h.run_steps(5)` between; queue several `Event::Text` before one `run()` for "quick" typing.
- A Tab-focused closed ComboBox in the editor loses arrows to the tree now (the tree has the keys); a later "arrows on a focused dropdown" change must gate on that.
- `okf stamp` takes one file per call.
