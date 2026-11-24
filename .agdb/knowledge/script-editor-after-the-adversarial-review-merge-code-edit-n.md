---
type: Gotcha
title: "Script editor after the adversarial-review merge: code_edit, not TextEdit, carries the text menu, tips and right press"
description: "How 5.2.0's script editor features (shared text menu, Undo/Redo rows, caret and pointer name tips) were fitted onto the adversarial-review branch's virtualized script_layout::code_edit, and the traps."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T00:09:36Z }
touches: ["crates/mg-ui/src/script_layout.rs:code_edit", "crates/mg-ui/src/script_view.rs:name_at", "crates/mg-ui/src/script_view.rs:ScriptUndo", "crates/mg-ui/tests/app/common.rs:timed_harness"]
asks:
  - why does a right click in the script editor not open the text menu
  - "how does the script editor's menu know whether Undo is possible"
  - how does name_at find the name under the pointer without a whole-text galley
  - "why can't a UI test find a header's records in its own Harness::builder"
fingerprint: v2-ac28830a83619ac6
created: 2026-11-24
author: augustaugur
---
- The script editor is no egui TextEdit since the merge of adversarial-review (56f51075): script_layout::code_edit draws visible lines only (LaidOut, a galley per line), keeps its own history (ScriptUndo, byte ranges) and stores TextEditState only for the cursor. There is no out.galley / galley_pos: use LaidOut::cursor_local_rect, row_y, row_height, ensure_line and CodeEditOutput::text_origin.
- widgets::text_menu needs the field focused (has_keys) on the right release. TextEdit took focus and moved the caret on any button's press; code_edit only on the primary one, so code_edit handles the secondary press itself: request focus, and move the caret only where the press is outside the selection (inside, it is kept for Cut/Copy/Convert Indentation). widgets::keep_selection is not called for the script editor any more.
- Undo/Redo rows: ScriptUndo::can_step(redo). Every step in it changes the text, so the old has_step walk over egui's Undoer (passing over cursor-only steps) and its per-menu cache are gone.
- name_at lays out only the line under the point (LaidOut::ensure_line), caches (text hash, line, column, symbols ptr_id) -> (name, x range) and looks the symbol up with SymbolSet::find_by_name (binary search; symbols are own + spec, not one slice).
- Header records (mg_uikit::header::records) are opt-in since 9821715f: common::build_harness turns them on (record_headers). A test that builds its own Harness::builder finds no Options header; use common::timed_harness for 60-fps frames.
- Help is verbatim (5.2.0): the branch's HelpLayout cache for the old Block layout was dropped with that layout; RefCache (the Reference list's filter cache) kept.
