---
type: Gotcha
title: "Script editor tests and code_edit: kittest Ctrl, one-second merges, scroll lag, focus on hidden editors"
description: "Traps met fixing reviews D and K of the script editor (code_edit): what kittest's keys and frames do to it, why lines and history check the text they were made for, and focus on an editor not drawn."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T02:02:25Z }
touches: ["crates/mg-ui/src/script_layout.rs:code_edit", crates/mg-ui/src/script_view.rs, crates/mg-ui/tests/app/script_keys.rs]
asks:
  - why does Ctrl+K or Ctrl+Left do nothing in a kittest script editor test
  - why does undo take back only one Delete in a UI test
  - Focused ID is not in the node list panic in a UI test
  - why is the cursor not in sight right after a paste in the script editor test
fingerprint: v2-bcb1b90449bdac08
created: 2026-11-24
author: augustaugur
---
* `egui::Modifiers::COMMAND` alone is not Ctrl for egui's editing keys (they read `ctrl`: Ctrl+H/K/U/W, Ctrl+arrows). On Linux and Windows winit sends ctrl and command both; tests use `CTRL` = ctrl|command (`tests/app/script_keys.rs`). Shortcuts through `matches_logically(COMMAND)` (Ctrl+Z) work with COMMAND alone.
* kittest frames are 0.25 s apart and `Harness::run` runs several, so a `press()` per key spends over the 1 s merge window of `ScriptUndo::record_edit` (egui's too): queue the keys, then one `run()`.
* egui 0.36's ScrollArea is `animated` by default: `scroll_to_rect` becomes an offset target applied the next frame even with `ScrollAnimation::none()`. The script editor's area is `.animated(is_smooth)`, so with Options › Smooth scrolling off the scroll is taken in the frame that asks, against that frame's content size: hence keys are handled before the editor is sized (else a paste at the end clamps short of the caret, review K's N6). Rows show a scroll the frame after it.
* Ctrl+W on Windows and Linux is Close Tab: the keymap consumes it before the editor, so only a Mac's Ctrl+W deletes a word.
* `LaidOut::apply_edit` takes the pre-edit hash: a tab behind another keeps lines of the text before an outside rewrite (`follow`), so byte places are read from them only when `in_sync`; `ScriptUndo::step` checks its step fits the text and lets the history go otherwise. Any `buf.text = ...` must clear or record history (`set_text`).
* `follow()` with the stored text equal to the editor's (Save, Build, Compile) keeps the history: clearing it took Ctrl+Z away after a save.
* `request_focus` on a widget not drawn this pass names a node AccessKit's tree lacks; accesskit_consumer panics ("Focused ID ... is not in the node list"), in kittest and for screen-reader users. `script_view::set_cursor` leaves the focus in temp data (`focus_when_drawn`) for an editor not drawn and the editor takes it, with its cursor revealed, when drawn.
* The live check and symbol jobs can finish between two frames a test samples: count jobs started (`script_workers_for_test`'s counters), don't sample busy flags. Without the game the check reports "FILE NOT FOUND" (nwscript.nss) for every script.
* Word moves: `word_left/right` read only back to a line with non-space text and equal egui's whole-text result (property test), except between a CR and its LF.
