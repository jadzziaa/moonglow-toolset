---
title: "Closing with unsaved side editors: what the review added (reasons, the module's question, tabs kept)"
author: August
date: 2026-11-11
branch: develop
cost: { minutes: 47, tokens: 371327 }
touches:
  - crates/mg-ui/src/hak_view.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/open_tabs.rs
  - crates/mg-ui/src/side_doc.rs
  - crates/mg-ui/src/tileset_view.rs
  - crates/mg-ui/src/wizards.rs
  - crates/mg-ui/tests/app/hak_closing.rs
---

## Decided
- The report (Theus, 4.0: "added a file to a hak and now the editor refuses to close") reproduced as five faults in the close path, not one stuck state: Don't Save always got out. The window's X asked the module's question for a hak (`Moonglow::run`), the Exit or Close asked for was dropped after a side editor's question (`ask_side_docs`, `SaveThen`), a failed save said why only in the log and its question came back for ever, the module's Don't Save dropped a changed hak unasked (`Proceed`), and a hak document outlived its tab (`close`).
- `side_failed` is answered (`failed_saving`) only while its document has unsaved changes, and `let_go_of_failed` drops it once the document is seen clean (Undo after a failed save), so it does not come back with a later change. The question's Cancel keeps it under the editor. The talk table shares it (`save_side` records a reason for `Tab::TalkTable` too).
- `module_failed`: the module question's Save that fails (an error logged during `save` and module work still unsaved) asks the module's question again with the reason, instead of nothing and the Exit dropped. Not when nothing failed (Save As canceled, a `hold_save` window).
- Opening another module keeps the side editors' tabs that are open, with their documents (`use_module`, `restore_tabs`, `close_keeping`); only File › Close lets the clean ones go with their tabs.
- `run_actions` keeps the not-yet-run rest of its batch in `batch_rest`: a check for a pending `OpenTab` must read it as well as `actions`.

## Tried and ruled out
- Escape closing a hak's tab: by design it cancels Properties windows only (`tabs.rs` `front`), left as it is.
- Carrying a refused X or Exit on after the Save Hak job ends: left refused with the log line, the job's window is on screen.
- Clearing the failed reason on the question's Cancel: the editor stays open and should still say why.

## Watch out
- A job can finish inside the frame an action was pushed in: a test of "refused under a job" must accept "not refused and done".
- `trail check` flags `side_docs_follow_tabs` and the screens case as bringing back "push OpenTab for every kept tab": they only read pending `OpenTab` actions or push tabs straight into the dock. Say so in the commit message.
- Which of the five the reporter hit is not known: his log's "Could not save the hak" line, his OS and how he added the file would say. Windows was not run.
