---
title: "Line ends by field kind, NUI page keys, linear diff"
author: August
date: 2026-10-26
branch: text-fields-rest
cost: { minutes: 372, tokens: 6239131 }
touches:
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/dialog_view.rs
  - crates/mg-ui/src/gff_view.rs
  - crates/mg-ui/src/module_props.rs
  - crates/mg-ui/src/nui_view/preview/text_controls.rs
  - crates/mg-ui/src/store_wizard.rs
  - crates/mg-ui/src/text.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
---

## Decided
- Aurora (EE, on the oracle) writes a line typed into a localized string (module description, placeable Description, conversation line Text) with LF, into a CExoString (conversation line Comment, blueprint Comment) with CRLF; saving a blueprint turns its LF Comment into CRLF untouched. All those fields are TMemo (forms.json in ~/.local/share/moonglow-oracle/uiinv), so the LF is Aurora's own code, not Wine. Capture `~/.local/share/moonglow-oracle/captures/line-ends/typed.mod`; test `text::tests::a_new_line_takes_the_line_end_aurora_writes`.
- `text::with_loc_line_ends` (LF default) for localized text; `with_line_ends` (CRLF default) for files and CExoStrings. Widgets: `commit_loc_text`/`commit_loc_text_in` beside `commit_text`/`commit_text_in` (both over `commit_typed`, which returns the editor text); `loc_text`, `with_token`, `LocStringEdit::value`, module_props `edit_field(.., loc)`, gff_view `Ctx::loc_text`, blueprint `hold_text(.., loc, ..)`, dialog line text, Store Wizard and the conversation Input Text use the loc one. Moonglow keeps an untouched LF comment LF (lossless) where Aurora would rewrite it.
- `blueprint::Form::text` (one-line CExoString) stored `buf.clone()`: now `with_line_ends(buf, &current)`, so a Tag with CRLF keeps it.
- The game's NUI multi-line text edit ignores Page Down/Up (client, 2026-10-26): caret stays, no scroll; Down moves a line. So the NUI preview keeps no page keys (comment in text_controls.rs).
- `kept_lines` → `common_lines`: LCS table when the run fits DIFF_CELLS (identical tie-breaks to before), else patience anchors (`unique_common`: lines once on each side, LIS by patience sorting), gaps trimmed of equal ends and recursed; a work budget DIFF_WORK (16M) bounds the time; leftovers go by position as before.

## Tried and ruled out
- Reading the wiki for NUI keys: nothing on it. The client check was cheap enough: a module from placement-probe.mod (creatures removed, PC immortal: bandits killed the PC the first time), OnClientEnter opening a NuiTextEdit of 60 lines with a watch; type a letter after each key and log the lines whose length changed.

## Watch out
- The client on the off-screen display: the first click only focuses its window; click again in the field before typing. xdrive `key Next`/`Page_Down` both reach it.
- Aurora's welcome list includes bundled modules after the user's; type the first letter to jump. A "did not close properly" prompt comes from an earlier session's crash; No discards only the oracle's own backup.
- Worktree Bash refuses `sh SCRIPT` and variables in commands; make scratch scripts executable and call them by absolute path.
