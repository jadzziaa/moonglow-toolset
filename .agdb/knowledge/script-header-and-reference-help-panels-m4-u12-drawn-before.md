---
type: Decision
title: "Script header and Reference/Help panels (M4 U12 and its review): drawn before the editor, pass stamps, help_forward, OK/Cancel at the end"
description: "How the script's header, Reference and Help work since U12 and its review: bodies drawn before the editor read its cached symbols and stamp what they drew with the pass, Help is brought forward by the dock for the script that asked, OK/Cancel sit in the header's end slot, and Help's doc parsing keeps lists, code and columns."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-16T17:20:34Z }
touches: [crates/mg-ui/src/script_reference.rs, "crates/mg-ui/src/script_view.rs:header", "crates/mg-ui/src/script_view.rs:follow_panels", "crates/mg-ui/src/shell.rs:help_forward", "crates/mg-ui/src/shell.rs:pane_halves", "crates/mg-ui/src/side_doc.rs:pressed_ok_cancel", crates/mg-ui/tests/app/script_header.rs]
asks:
  - why does active_editor return None inside the script editor
  - why did the script window keep repainting forever
  - how do tests click a script header button that went into More
  - "why does Help keep nwscript's lists, code and column tables as written"
fingerprint: v2-c239340a3b3bf995
created: 2026-11-16
---
- Reference and Help (`Panel::Reference`, `Panel::Help`, `script_reference.rs`) are brought by `Tab::Script` (dock shares 56/44, `Brings::window_shares` 44/56 in its window, floats). Their bodies run BEFORE the script editor, read `ScriptTools::symbols` as the editor last made them and hand work back via `ScriptTools::insert` (keyed by script). Don't hash the script text twice a frame.
- Repaint rule (review A1): Reference stamps `reference_drawn` and Help `help_drawn` with `ctx.cumulative_pass_nr()`; the editor (`follow_panels`) asks for a frame only if one was drawn THIS pass and differs. An unstamped "what I last drew" record loops forever when the panel stops being drawn (Help's tab in front, dock folded). The editor also asks for a frame after a key (`typing` keeps last frame's symbols).
- Gotcha: inside `TabViewer::ui` (any editor's drawing) `app.active_editor()` is None: egui_dock has `app.dock` taken. Help forward is `ScriptTools::help_forward: Option<ResKey>` (the asking script), acted on by `shell::help_forward` in the Right Dock: dropped if that script floats (its pane shows Help), kept until a script is in front (Go to Definition of nwscript.nss), then `focus_right` (nothing written to settings).
- OK/Cancel: in the header's end slot (proposal 15.2 ruling 10); `side_doc::pressed_ok_cancel` does what they do; `failed_foot` stays at the foot.
- A floating script's right pane never drops (`shell::pane_halves`): under 560 it is half the window. The editor is clipped to its room so its bottom strip does not draw over the pane.
- Help's prose (`doc_blocks_with(doc, param_names(sig))`): `- name:` / `name - text` rows, other list lines (`- …`, `-> …`, `1) …`) as `Block::Item`, `*` notes, code (`code_like`) and columns (3+ inner spaces) as `Block::Lines`, indented lines continue the block before. Corpus tests in `script_reference::corpus` sweep nwscript.nss. Labels in a horizontal row need `.wrap()` even with a LayoutJob width, or they run past the panel.
- Tests: `common::script_tool`/`script_click` reach header controls through More; `reference_row` finds list rows inside "Script reference". Kittest Label nodes: text in value(); `has_requested_repaint` counts `request_repaint_after` timers too (set `dialog_no_backup` to test a repaint request).
- editor_perf's "scrolling" case scrolls no script (its window is 338x418, the pointer over Module Properties).
