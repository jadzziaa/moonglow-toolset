---
type: Decision
title: "2DA editor's Cell, References and Outliner columns (M4 U16 and its review): grid keys by active editor, one list for choices, Outliner alone without a module"
description: "How U16 moved the 2DA editor's inspector into Cell and References and its columns into the Outliner, and what its review corrected: the grid's keys gated on the active editor and kit popovers, Cell's lists one Tab stop, F6 letting go of the focus, the Left Dock holding the Outliner alone with no module, the board's grid look, Outliner rows laid out in sight only."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-17T10:26:04Z }
touches: [crates/mg-ui/src/table_view/panels.rs, "crates/mg-ui/src/table_view.rs:grid_has_keys", "crates/mg-ui/src/table_view.rs:keep_tab_for_the_grid", "crates/mg-ui/src/shell.rs:outliner_alone", "crates/mg-ui/src/outliner.rs:columns_seen", "crates/mg-ui/src/lib.rs:step_pane", "crates/mg-module/src/table_edit.rs:Editing::follow"]
asks:
  - "Tab or the arrows in the 2DA grid act after I pressed another editor: what decides the grid has the keys?"
  - Why does the Left Dock show only the Outliner when a 2DA is open with no module?
  - "Enter in the Cell panel's Value is slow on a big table: why would it copy the table?"
  - "After F6 the Outliner's arrow keys do nothing: who kept the focus?"
fingerprint: v2-04d7b1f9425c1b5f
created: 2026-11-17
---
## Decided
- The header is the kit's `EditorHeader` "2DA table header" (`table_view::header`): Save primary; Add Row + count a `custom` item (KitButton named "Add Row" whatever its words, a DragValue named "How many rows Add Row adds"); Empty Row, Delete Row, Add Column… icons; Find and Go to `custom` and `primary` (they leave the row with Save, last); "N cells are wrong" a custom flat warn button; Values as names and Strings as text a `joined` group "How cells show"; More (`more_words`) holds Save As File…, Take Rows…, Compact, Show Hidden Columns (shown_words with the count) and Reset All Widths (the board's More names five, the plan four). Tests reach them with `common::table_tool` / `table_click` (opens More), `table_headers_with` (records).
- Panels: `Panel::Cell`, `Panel::References`; `brings_of(Tab::Table)` = TABLE_RIGHT [[Cell],[References]], floats, outliner. `pane_halves` includes tables (the choosers must stay reachable). Floating window opens 1040 + 281 + 261 wide; the Outliner pane only where the grid keeps `WINDOW_GRID_AT_FIRST` (1038), else the strip.
- The Outliner's keyed lists are generalized: `outliner::Owner { Dialog(ResKey), Table(u32) }` replaces ResKey in `marks_*`, `strips_*`; `Contents::Columns(id)`; `outliner::strip`/`over` moved from dialog_view and shared. Columns keep `Keys::Marks`.
- Cell's Value is kept in `TableDoc::cell_typed` (cell, `Editing::changes` at typing, text); `Editing::follow(cell, since)` (mg-module) walks a log of row/column renumberings (RowOut/RowIn/RowsTo/Column*), so a row taken away above moves the text, focus and selection, and its own row going lets it go. `commit()` commits it too (save, grid clicks).
- References: `panels::find_references` spawns a thread over `ws.module.clone()` (Arc'd resources) and the game Arc; `references_until` / `mg_module::row_uses::row_uses_until` check a stop flag between tables/resources; `Looking` stops on Drop, on another row, on `changed()`. `Moonglow::hold_reference_looks` holds it for tests; `under_way` counts it.
- Redraw: the pass stamps and `panels::follow_panels` were removed at the review (no test needed them, as U15's stamp). `panels::columns_mark` now keys the Outliner's cache of a table's columns (`Outliner::columns_seen`: names, icons, kinds' words).
- The grid's keys (review): `table_view::grid_has_keys(app, ctx, id, tab)` = keys are Table's, of this table, `active_tab(app) == tab`, nothing blocking. `Moonglow::active_tab_now` is computed at the start of `ui()` (`front_of_focused`: focused leaf's front, else active editor) because `app.dock` is taken out while DockArea draws. Read-only views pass their own tab (`browser::view_tab`, replacing `view_in_front`). `take_keys` asks the dock focus too (`focus_asked` -> `focus_asked_tab`, applied after the dock is put back): kittest's accesskit clicks do not set egui_dock focus.
- Cell's lists (review): `panels::choice_list`: one focusable list (`Sense::FOCUSABLE`, named "Values"), rows `Sense::CLICK` (not focusable), cursor in temp data, arrows/Page/Home/End/letters/Space/Enter, `ScrollArea` max 8.5 rows.
- No module open (coordinator's ruling at the review): `shell::outliner_alone` / `left_holds`: the Left Dock draws the Outliner alone (whole height, `outliner::alone` puts the dock's fold button at the end of its header via temp data); folded, the strip shows the Outliner's icon. Only tables qualify (`outliner::contents` needs a module for areas and conversations).
- Grid look (review): `table_view::look` (header 24 on `bar`, gutter `bar` mono right-aligned, cell lines, `cell_range` token for chosen cells, cursor `accent-fill` + inset ring while the grid has the keys, column name accent). `GridOf::has_keys` drives the ring; read-only views share `grid_in`.
- The Outliner's columns lay out only the rows in sight (`show_viewport` with a 2-point top), the keys' row brought into sight by `scroll_to_rect` of its computed place (`scroll_to_me` needs the row drawn).
## Tried and found
- egui gives Tab to the FIRST interested widget when nothing is focused (`Focus::interested_in_focus`), so the grid's Tab never worked (the Top Bar's buttons took it, before U16 too). `table_view::keep_tab_for_the_grid` at the start of `Moonglow::ui` calls `memory.move_focus(FocusDirection::None)` while the grid has the keys and nothing is focused.
- `undo_under_pointer`/`undo_keys`/`pressed_side_tab` were reset AFTER the docks were drawn, so a docked panel's `side_doc::note_pointer` was wiped: the reset moved before `left_dock` (after `undo_before_bars` is taken).
- Enter or Escape leaving Cell's Value reached the grid drawn after (began typing, or left the keys): the panel consumes them on lost focus. Go to's Enter did the same since before U16 (the header gives the grid the keys in the frame the grid then reads Enter): Go to consumes it too.
- Tab is kept for the grid only where `grid_has_keys` holds (its table the active editor) and nothing is focused or modal: else egui gives it to the first control. Asking only "in front of its pane" (first round) let a floating table's grid keep Tab, arrows and Delete after a press in the main window.
- A press in another editor already takes the arrow keys from the grid (`keys_are` changes), so the active-editor gate shows only for a floating table's main-window press. A read-only view does float (As Floating Windows: its own window), yet breaking its gate alone fails nothing for the same reason.
- `grid_has_keys` also asks `mg_uikit::popover::Popover::any_open`: kit popovers (the header's More) are not egui popups, so `shell::keys_blocked` misses them.
- F6 (`Moonglow::step_pane`) surrenders egui focus at every step: References' button kept it and the next list's keys (`tree::keys_free`) did nothing. With the Outliner alone, `left_dock_shows` returns `Panel::Outliner`, which must not be pushed as a stop of its own (it doubled the stop).
- Cell's `choice_list` keeps its cursor with the table row it was for; another row's cell resets it to the first value on and scrolls it into sight.
- The cursor cell's text is `on_fill` whatever the cell (a wrong cell's red on `accent-fill` was unreadable in light); the fault keeps its red outline.
- `spells.2da`'s `SpellDesc` is no StrRef column the game's kinds know (`doctor::STRREF_COLUMNS` has `Name`): a test wanting its text in Cell describes it `kind = "strref"`.
- Enter in Cell's Value copied the whole table: `commit_typed` ran while the panel still held `edit.shared()`; it now runs after `drop(table)` (test by `Arc::as_ptr`).
- A Cell section heading labeled with the column's name collided with the grid's column label in tests: it is painted and named "Mask column: any of 3 flags".
## Watch out
- Holding `edit.shared()` across `edit.set` (or `commit_typed`) copies the whole table: drop it first.
- `codemode` `edit()` is async: without `await` nothing is written and no error shows.
- Replacing a slice of `common.rs` between two doc-comment anchors once deleted helpers inserted between them (`cell_value`, `column_name`, `grid_cell`, `choice`): replace exact text, not a range.
- The Outliner rows' accessible names are "Kind, one of 2 values" (with words); tests use `column_row` / `common::column_name` (grid header, not the Outliner) / `grid_cell` / `choice`.
