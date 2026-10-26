---
title: "Page keys in text fields, line ends by a line diff"
author: August
date: 2026-10-26
branch: text-fields-keys-line-ends
cost: { minutes: 871, tokens: 9355883 }
touches:
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/dialog_view.rs
  - crates/mg-ui/src/hak_view.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/nui_view.rs
  - crates/mg-ui/src/nwsync_view.rs
  - crates/mg-ui/src/plugins.rs
  - crates/mg-ui/src/store_wizard.rs
  - crates/mg-ui/src/talk_view.rs
  - crates/mg-ui/src/text.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/tests/app.rs
---

## Decided
- Page keys for every multi-line field live in `widgets::page_keys(ui, &TextEditOutput)`, called after `show` (by `widgets::text_area(ui, id, edit)` and `commit_text_in`): it steps `galley.cursor_down_one_row`/`up_one_row` with `h_pos` a page of times (page = rows of `text_clip_rect ∩ ui.clip_rect()` less one), stores the state and calls `ui.scroll_to_rect` itself. A new multi-line field should use `text_area`, not `ui.add(TextEdit::multiline)`. The script editor keeps its own.
- `text::with_line_ends(text, original)` diffs the lines (prefix/suffix trim, then an LCS table capped at 4M cells): unchanged lines keep their own end, a line changed in place (pairs in a run between kept lines) keeps its old end, a line added takes the majority. `with_line_ends_or(.., crlf)` sets the end of a text that had none: CRLF for GFF text, LF for talk tables. `from_editor` is gone.
- Game data, counted: every install TLK (all six languages) is LF only, no `\r`; GFF text with line ends in the BIFs and campaign `.nwm`s is 13,925 CRLF to 1,191 LF, none mixed (docs/findings.md). New GFF text with a line end (commit fields, the conversation Input Text, the Store Wizard) gets CRLF.
- Pasted text's `\r\n` / lone `\r` become `\n` once a frame for every field (`widgets::plain_pastes`, after `clipboard_keys`).

## Tried and ruled out
- Injecting N ArrowDown key events before `show` for the page keys: needs the id and last frame's rect before the field draws; the galley-after-show way is exact and needs no synthetic events.
- Storing the cursor after `show` and relying on egui to scroll: egui scrolls only when the selection changed within its own `show`, so the next frame sees no change; call `scroll_to_rect` yourself.

## Watch out
- egui's `\r` is not invisible in practice: in a TextEdit it shows a space's width, and End stops after it, so "line\rX\n" breaks the CRLF. Everything editable must hold `to_editor` text.
- kittest: `Modifiers::COMMAND` + Home/End doesn't move a TextEdit's caret (egui checks `modifiers.ctrl`); use `Modifiers::CTRL`. A field taller than the window is clicked at its center, off screen: click its top with PointerButton events.
- `str::lines()` strips `\r\n` whole; to check line ends, split on `'\n'`.
- `matches_logically(NONE)` takes a Shift press: consume SHIFT first, then NONE. Ctrl isn't matched, so Ctrl+Page Down still switches tabs.
