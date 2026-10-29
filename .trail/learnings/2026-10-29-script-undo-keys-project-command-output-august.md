---
title: "Script Undo keys, project command output and input, field meanings, manual tables, palette StrRef"
author: August
date: 2026-10-29
branch: worktree-agent-a767531318a2c9bda
cost: { minutes: 1884, tokens: 13750142 }
touches:
  - crates/mg-ui/src/project_commands.rs
  - "crates/mg-ui/src/script_view.rs#give_keys_back"
  - "crates/mg-rules/src/rows.rs#meaning"
  - "crates/mg-ui/src/manual.rs#blocks"
  - crates/mg-ui/src/palette_categories.rs
---

## Decided
- A project's command reads what is typed only with `input = true` in moonglow.toml (stdin piped, a line under its output in the job's window); every other command keeps stdin closed, so a team's script with `read` that works today (answered empty at once) is not left waiting. A program that asks on the terminal itself (ssh, sudo) can't be answered from Moonglow; git gets `GIT_TERMINAL_PROMPT=0` unless the builder set it.
- What a field's number names is one rule, `mg_rules::rows::meaning` (and `list_item_name`), which the fields view and `mg gff --names` both call: fixed tables (`row_table`), an item property's subtype/cost/param tables found from its sibling fields, factions (names passed in by the caller: the module's repute.fac), dialog animations, plain-int StrRefs (`is_strref_field`). `STORE_PAGES` and `DIALOG_ANIMATIONS` live there now, one copy each.
- The manual viewer draws tables itself (`manual::blocks`, `table_ui`): egui_commonmark 0.25 draws a cell in `ui.horizontal` inside a Grid, which never wraps. Column widths by the square root of each column's measured text width; a list only below 90 points a column.
- A palette category is named by a talk-table line through `STRREF` (as the game's skeletons are); no LocString is needed.

## Ruled out
- Requesting the script editor's focus from `commands::menu_bar` the frame a menu closes: a click on the menu's name is a click outside the TextEdit, which surrenders the focus later that frame. Ask for it after the editor is drawn (`script_tools.keys_back`, taken where `stepped` refocuses).
- `setsid` for a project's command (to make /dev/tty readers fail fast): needs `pre_exec`, which is unsafe, and the workspace denies `unsafe_code`; `process_group(0)` makes the child a group leader, so a later setsid would fail anyway.

## Watch out
- `egui_commonmark`'s `CommonMarkViewer::show` calls `prepare_show`, which deactivates every link hook: with several shows a frame (a chapter's blocks, each table cell), read `get_link_hook` after each one, or a click on a chapter link is lost.
- A shell prompt without a newline (`printf 'Name: '`) stays in the reader's partial line; when the builder sends an answer, end the partial lines (`Incoming::end_lines`), or the next prompt is appended to it.
- In kittest, a job run with `background_jobs = true` keeps the UI repainting: use `h.step()`, not `h.run()`, while it runs.
