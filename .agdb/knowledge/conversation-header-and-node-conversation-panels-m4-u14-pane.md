---
type: Decision
title: "Conversation header and Node/Conversation panels (M4 U14): panel actions through the editor, ids follow lines, form rows"
description: "How the conversation editor's header and its Right Dock panels (Node section with Data/Search, Conversation panel) work since U14, and the traps found: panel edits must go through the editor for Cancel, typed buffers follow renumbering, one too-wide row widens every later row."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-16T22:30:42Z }
touches: [crates/mg-ui/src/dialog_view.rs, "crates/mg-ui/src/shell.rs:section_bar", crates/mg-ui/tests/app/conversation_panels.rs, "crates/mg-uikit/src/shell.rs:DockTabs"]
asks:
  - "why do the conversation's panel edits go through DialogView::from_panels"
  - "why is Node a title and not a tab in the conversation's Right Dock"
  - "why did a conversation panel's fields run past the dock's edge"
  - "how do tests reach the conversation header's buttons and a section's Add Set"
fingerprint: v2-df5dffe6c9560ca2
created: 2026-11-16
---
- Header: `dialog_view::header`, kit EditorHeader `CONVERSATION_HEADER` ("Conversation header"); OK/Cancel in the end slot (Close when `looked_at`). Tests: `common::dialog_tool`/`dialog_click` (through More: the floating window's editor is narrow), `common::in_section(h, heading, name)` for controls duplicated in Text Appears When… and Actions Taken (both on one scroll now).
- Panels: `Panel::NodeData` ("Data"), `NodeSearch` ("Search"), `Conversation`; `brings_of(Tab::Dialog)` = DIALOG_RIGHT + floats; `pane_halves` includes Dialog (the window pane never drops). "Node" is the Conversation board's `.docktab.title` (kit `DockTabs::title`), NOT a third tab; `Panel::section_title`. Bookmark/Test on the bar via `section_bar` -> `dialog_view::node_tools`/`node_tool`.
- Bodies run BEFORE the editor (docks first). They must: run `begin()` (follow + selection bounds, else they resolve an old row), apply `larger_text` themselves, and NOT push to app.actions: `DialogView::from_panels`, pushed by `dialog_view::ui` after `pending` so `blueprint::window_drawn`'s Cancel notes them.
- Redraw: `data_drawn`/`bar_drawn` stamped with the pass; `follow_panels` after the editor (U12 rule). Search forward: `Moonglow::dialog_search_forward` -> `search_forward` at right_dock start (right_fronts + focus_right / show_dock_panel when folded).
- Field ids are by line/row number; on a renumbering `begin` calls `follow_typed`, which moves the chosen row's field buffers (`row_fields`: app.buffers + the params temp data), TextEditState and focus from the old (row, line) to the followed selection's (else typed text was lost or shown on the line that took the number). A press commits before the selection moves, so a test needs an outside renumber to catch it.
- Find per window: `search_forward` -> `Moonglow::window_rights` for a floating conversation (NOT `right_fronts`, which the dock and every window share). Read-only must also gate tree drops: the editor's commands use `Edit::SetResource`, which `Workspace` does not refuse for viewed docs.
- Layout trap: egui widens max_rect to a too-wide row, so ONE overflowing row (a non-wrapping note, heading + button, a ComboBox `.width(available)`) pushes every later row and the scroll bar past the dock. Use `form_row` (label column 96, label over the field when < FIELD_LEAST scaled by text size), `fitted` (reserve the trailing button + slack), wrapping labels; `every_conversation_panel_fits_the_right_dock` checks at 280..600 and 20 pt. `ui.horizontal_wrapped` broke widths (available_width is the whole line there).
- Hex field: `color_button::hex_width` (mg-ui; kit untouched).
- Review fixes (2026-11-17): resref fields via `resref_row` + `widgets::with_resref_width` (16 chars, `name_least`; label over the row via `form_row_least`; trailing Edit/Play wraps when even the full row is short). One copy of the conversation per workspace revision (`DialogView::copy`, `begin` -> Arc<Gff>; edits copy with `Gff::clone(&g)`). Renumberings followed only in `follow_view` (begin and `follow_unseen`). `flush_panels` sets panel edits the editor did not take, each frame. Find over an unfocused group: `search_forward` calls `show_tab`. Conversation section least height: `shell::section_least` / `section_heights_with`, measured from the last draw (`Moonglow::dialog_conversation_height`). Both Tests named "Test"; tests tell them apart by group (`RIGHT_DOCK` / "Window's Right Pane"). Data headings scale with the editor's text (`panel_heading`).
- Test gotchas: F3 unfolds a folded dock (fold again to test the bar's stamp); a Paste check needs a target the clip fits (Root for an Entry); a seam restart test compares shares as a fraction of the room, not points.
