---
title: "Closing with side editors' unsaved work carries on"
author: August
date: 2026-11-11
branch: develop
cost: { minutes: 86, tokens: 376665 }
touches:
  - crates/mg-ui/src/side_doc.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/hak_view.rs
  - crates/mg-ui/src/open_tabs.rs
  - crates/mg-ui/tests/app/hak_closing.rs
---

## Decided
- What closing the window, Exit, File › Close or another module asked for waits in `Moonglow::side_then` while the side editors' questions are asked (`ask_side_docs(then)`); `side_doc::closing_window` pushes it onto `actions` once the queue is empty (`carry_on`), never under a job (`run` refuses Quit and discarding actions under one). Cancel clears the queue and `side_then`. The window's X with only side work asks the side questions, not the module's.
- The module question's Don't Save pushes `Proceed(next)`; with a side document changed, `Proceed` asks of it first (`ask_side_docs(Proceed(next))`) and runs `next` only once none is unsaved.
- `side_failed` (one `(Tab, reason)`) is why a side save failed: set by `hak_view::save`/`save_then` with the bare error, by `save_side` from a 2DA's `problem` or else the newest error logged during the save; a canceled Save dialog sets nothing. Shown in the question and the editor's foot (`ok_cancel`; a 2DA's own problem line already says it). A foot OK whose save fails queues the tab's question.
- `side_docs_follow_tabs` runs after each dock reset (`close`, `use_module`, `restore_tabs`): a tabless clean side doc is forgotten, a changed one gets its tab back (the NewModule / OpenModuleAsIs second step is not asked again for side docs: `unsaved_mark` hashes only the module and scripts).

## Watch out
- `scratch_dir` empties its folder: the matrix test's cells use `hak-closing-cells`, and its `matrix.md` and the screens pictures go to `output_dir("hak-closing")`.
- `NoDialogs::save` answers once: a second Save of a new hak is a canceled dialog (no failure, reason cleared).
