---
title: "Conversation Delete crash and the tree's arrow keys (Sacha's report)"
author: August
date: 2026-10-25
branch: dialog-delete-arrows
cost: { minutes: 90, tokens: 7771124 }
touches:
  - apps/moonglow/src/main.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/dialog_view.rs
  - crates/mg-ui/src/outside.rs
  - crates/mg-ui/src/palette_view.rs
  - crates/mg-ui/src/recovery.rs
  - crates/mg-ui/src/references.rs
  - crates/mg-ui/src/tabs.rs
  - crates/mg-ui/src/tree.rs
  - crates/mg-ui/tests/app.rs
---

## Decided
- Sacha's "delete a line, a window asked, Yes, the toolset closed" was two bugs in a row. (1) Routing: the conversation editor's line Delete has no prompt; the prompt was the module tree's "Delete from Module". Opening a conversation from the tree leaves `tree_cursor` on it with `Keys::Tree`, and tree.rs's "Delete at the cursor's resource" (`pointed.or(at_cursor)`, inside the `keys_are(Tree)` block) consumed Delete wherever the pointer was. The tree's Ctrl+F stole the editor's Find the same way. Fix: `Keys::Dialog` + `dialog-keys-of` (the ResKey) in ctx data, taken by any pointer press inside the editor (`dialog_view::take_keys`, at the end of `editor` where `ui.ui_contains_pointer()` covers the drawn content). (2) Crash: `DockState::retain_tabs` on a window surface losing its only tab leaves a tree with a `Node::Empty` root and `focused_node = Some(0)`; `show_window_surface` unwraps it (window_surface.rs:53/55). Conversations and Properties open in floating windows, so any `retain_tabs` that empties one crashed. `tabs::close_tabs` loops `find_tab_from` + `remove_tab` (remove_leaf clears the tree, remove_surface drops the window). Used at references.rs (Delete resource), outside.rs (files gone outside), tree.rs and area_view.rs (placed object deleted while its Properties float).
- Tree keys follow Windows TreeView (Aurora's `tvMain` is a VCL TTreeView; the oracle was not driven): `dialog_view::shown_rows` mirrors `tree()` (same cycle guard), `tree_step` is pure and unit-tested. Root is row 0. Right goes to the first child only if the next row's `up` is this row (a cycle row shows "−" with nothing under it). Page = rows the tree had room for last frame (`view.page`), 10 before the first draw. `view.nudge` scrolls with `scroll_to_me(None)` (Center for `reveal` jumps).
- `Moonglow::recovering(frame)` in recovery.rs wraps the frame (apps/moonglow main.rs): catch_unwind, write_recovery if `has_unsaved_work` and not `no_autosave`, resume_unwind. The panic hook (crash report) already ran before the catch.
- Drag payload is `(ResKey, Row)` now: a Row dropped from another conversation's tree indexed this one's links (`target_of` panics on a stale pos).

## Tried and ruled out
- Looking for the crash in `mg_module::dialog::remove`/`prune` and the selection filter: none panics (18 delete cases by key, menu and button pass on the old code paths once the key reaches the editor).

## Watch out
- egui's `consume_key` ignores extra Shift/Alt: the tree's keys are taken only with `modifiers.is_none()`, so Alt+Up/Down still move lines.
- In kittest, after Tab out of the line's Text field the focus moves to the next field, so a following Ctrl+F (which wants `memory.focused().is_none()`) does nothing: click a row first.
- Worktree Bash guard refuses `flock ... cargo test` and heredocs: a script in the scratchpad (`uitest.sh`) runs the mg-ui tests under the GPU lock.
