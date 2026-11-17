---
type: Decision
title: "Conversation bookmarks in the Outliner, indent guides and hints (M4 U15)"
description: "How U15 put a conversation's bookmarks in the Outliner (docked and in a floating window's left pane), drew the tree's indent guides and gave the Status Bar its hints, with the keys and redraw rules and a U14 redraw fault found."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-17T02:14:03Z }
touches: [crates/mg-ui/src/outliner.rs, "crates/mg-ui/src/dialog_view.rs:bookmark_rows", "crates/mg-ui/src/dialog_view.rs:guides", "crates/mg-ui/src/shell.rs:dialog_shown", "crates/mg-ui/src/shell.rs:window_panes", "crates/mg-ui/src/lib.rs:Moonglow::step_pane"]
asks:
  - Where does a floating conversation show its bookmarks?
  - Why do the conversation tests fold the Right Dock beside Farewell or Who?
  - "The Outliner's arrow keys act on the wrong list (area rows vs bookmarks): how are they told apart?"
  - "Why does the conversation's Status Bar drop Ctrl + A or * hints while typing?"
fingerprint: v2-811395b5cea29b72
created: 2026-11-17
---
## Decided
- `outliner::contents` chooses what the Outliner lists by the active docked editor: `Contents::Area` (T13's panel) or `Contents::Bookmarks(key)` (`outliner::bookmarks`). Later editors add arms there. `Brings::outliner` is true for `Tab::Dialog`.
- A floating conversation's bookmarks are in its window, a LEFT pane drawn by `shell::window_panes` after the right pane: T13 says the main Outliner never counts a floating window, U2 says a kind that floats takes what it brings into its window. Review ruling (coordinator): the pane gives way before the header does. The pane is drawn only where the tree keeps its old 638 (`WINDOW_TREE_AT_FIRST`), 260 or narrower down to 150; otherwise `dialog_view::outliner_strip` folds it to a strip (kit `FoldStrip`, named `OUTLINER_STRIP` "Window's Outliner") at the tree's left UNDER the header (so the header keeps its width), and its icon unfolds `outliner_over` over the tree, drawn in `dialog_view::ui` AFTER `editor` (the view must be back in `dialog_views`; drawing it inside `editor` would `begin` with the view taken out and get a default view). New windows open 261 wider (`Moonglow::open_window`); remembered sizes are left as they are.
- Keys: the bookmarks have their own `Keys::Marks`, with `Outliner::marks_keys` naming WHICH conversation's list (docked and a window's can both be drawn). `step_pane` carries a third column (the conversation) per stop; window lists come from `Outliner::marks_last` (the last frame's drawn lists, swapped in `frame_yields`). `keys_leave_panels_not_drawn` hands Marks back to the editor where no list of that conversation is drawn, and treats the area's `Keys::Outliner` as not drawn where the Outliner lists bookmarks.
- Rows: `dialog_view::bookmark_rows` runs `begin` (so they follow their lines: delete, undo, outside change via `Renumbering::by_lines`). A pass stamp (`marks_drawn`/`marks_changed`) was tried and removed at the review: no UI case needs it (the Node bar's Bookmark is a click, egui draws a frame after it). `marks_moved` is keyed by conversation (two lists drawn at once each scroll their own row). While a list holds the keys, the conversation editor's `here` is false with the pointer alone (`a_list_has_the_keys`). Type-ahead via generic `tree::typed_row`. The seam beside a conversation is its own setting (`Settings::bookmarks_panel_height`).
- Guides: `dialog_view::guides` paints a 1 px `tokens.guide` vline per ancestor level at `left + level*16 + 11` (INDENT 16, arrow 22), row height plus half the item spacing, only where the row is visible (the tree lays out every open row). Kit tokens `guide` (#2c3036 / #cdd2d8 derived), `guide-hover` (#3a3f47 / #bcc2ca: guide on hover_row was 1.0:1), `guide-on-fill` (board's 18% white). The selected row is filled the tree's whole width (`dialog_view::fill_row`, a Noop shape reserved before the row). Levels stay 16 points with the 22-point arrow (coordinator's ruling); Root has no arrow (it doesn't fold).
- Hints (`shell::dialog_shown`): pointer gestures always (not Drag/Ctrl+Drag where only looked at); Add, Alt+Up, Find only while the editor's commands take keys (`DialogView::keys_took.0`, set in `editor`: pointer over it or tree keys, not typing); `*` only with tree keys (`keys_took.1`); none while `keys_blocked`; count from `NumWords` (`file_words`). Alt+Up and `*` are fixed keys (no Cmd); Add and Find come from the keymap.

## Watch out
- U14's Data never-resting fault (taller where wider) is fixed in U15's review: see the note "A scrolled panel taller where wider never comes to rest".
- A dim `TreeRow` (the empty "Bookmarks (0)") is a Label: its words are in accesskit `value`, not `label`.
- `get_all_by_label` panics on none; use `query_all_by_label` to collect Outliner groups (several can exist: main and a window's).
- "[OWNER] - Hello" contains-queries also match the arrow "Collapse [OWNER] - Hello…": filter by label prefix.
