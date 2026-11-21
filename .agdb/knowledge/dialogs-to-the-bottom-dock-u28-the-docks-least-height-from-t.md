---
type: Decision
title: "Dialogs to the Bottom Dock (U28): the dock's least height from the first row, rows that take every click, keys under the area view"
description: "How M4's U28 moved Find Instance, Find and Replace Text, Build/Verify results and three reports into the Bottom Dock, the traps met (egui's ScrollArea least height, triple clicks, selectable labels, toggled vs selected, keys taken back by the area view) and the review round's focus and Tab fixes."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-20T16:14:19Z }
touches: [crates/mg-ui/src/report_table.rs, crates/mg-ui/src/search_results.rs, crates/mg-ui/src/dock_reports.rs, crates/mg-ui/src/problems.rs, "crates/mg-ui/src/shell.rs:first_row_ends", crates/mg-ui/src/jobs.rs]
asks:
  - Why does the Bottom Dock grow taller than I dragged it?
  - Why do the arrow keys not reach a dock table after Tab to its head?
  - Why does Shift+Tab from a dock table jump to the Top Bar?
  - Why is is_selected None on a row that is drawn chosen?
fingerprint: v2-e06d48c03c66d23c
created: 2026-11-20
author: augustaugur
---
## Decided
- The Bottom Dock's least height is the panel in front's: a body calls `shell::first_row_ends(ui, y)` with where its form or header and its first whole row end (the max of the frame is kept in temp data); `bottom_dock` turns it into `app.dock_need`, capped at half the window (a taller form scrolls in `search_results::form_frame`, which reports the WHOLE form's end, not the cut one, or the cap goes circular). `drawn_ends` (a body's min_rect bottom) is only for U28's own tabs (Search Results and the three reports) when they list nothing: given to every dock panel, it made Compiler/Bookmarks fill the dock and grow it to half the window (script_bottom test caught it).
- Every ScrollArea of rows in the dock takes `.min_scrolled_height(0.0)`: egui's default least (64) laid rows out past the dock's bottom.
- Rows activate on `double_clicked() || triple_clicked()`: a third click inside egui's 0.6 s makes `double_clicked()` false (tests that click then double-click hit it).
- A row's own `ui.interact` comes BEFORE what is drawn on it (Problems' heading), or its buttons (Counted as Used) lose their clicks to it.
- Text inside a clickable row is `Label::selectable(false)`: a selectable label takes the click (left AND right: Problems' message took the right click, so Mark as Used's menu never opened) and the row is never chosen.
- A chosen list row is `accent-fill` with its words in `on-fill`, as the tables (`select` is the 3D view's selection, orange, unreadable under text in Light). A plain `&str` label ignores `override_text_color`: give the words a RichText color.
- Search Results' form scrolls (`form_frame`) with `.min_scrolled_height(0.0)` too: with egui's 64 the least window's first hit was cut. The accessibility tree has no rect for a scroll's clip, so no test sees that cut: look at the 420x320 pictures (`dock_pictures`, run with --test-threads=1).
- Report tables are `report_table::show`: sort by head, rows in sight, keys only via `has_keys` (no menu/popover/dialog), Ctrl+C "name\tvalue"; narrower than its columns' least (fixed + 90 a shared column) it scrolls sideways.
## Watch out
- egui gives `WidgetInfo::selected` to AccessKit as `toggled`, not `selected`: test a row's chosen state with `toggled() == Some(Toggled::True)`.
- The body nodes of Bottom Dock panels are not children of the "Bottom Dock" node: find them by rect inside `bottom_dock_rect()` (`in_dock` in tests/app/common.rs).
- The area view took the arrow keys back when the pointer moved in (fixed in the review round: keys given by a command hold until a press elsewhere); a test opens the list by its command and hovers the view (`over_the_view`), never clicks the table first.
- A closed area tab keeps its view (camera kept): Resources Used goes only when the view goes (module closed).
- Settings::default opens editors floating: a window opened over the dock takes the dock's clicks in tests; use OpenEditors::Tabs.
## Review round (2026-11-20)
- Keys given by a command must hold over the area view: the view took them back as soon as the pointer moved in. `palette_view` keys hold now until a press elsewhere; tests open by the command and hover the view (`over_the_view`), never click the list first.
- `report_table::has_keys` holds back only for a focused TEXT field (`text_has_keys`: `TextEditState::load` under the focused id). `egui_wants_keyboard_input` is true for a focused button too, so a head reached by Tab kept the keys from the rows. A moving key (arrows, Page Up/Down, Home, End) surrenders a button's focus and remembers it; `tab_from_the_rows`, called at the top of `Moonglow::ui` BEFORE any widget, gives Tab/Shift+Tab back from that control: egui hands Tab on while widgets register interest, so doing it in the table's body (drawn late) is too late and focus lands on the Top Bar.
- `Context::get_response` (via `read_response`) surrenders focus when the pointer was pressed or clicked elsewhere: `form_frame`'s focus-scroll skips frames with a press, or a menu click's frame drops the field's focus and typing goes nowhere.
- A form line is as high as its tallest control AT THE ZOOM (`form_line_high`: body row height + field margins, interact size, CONTROL_HEIGHT, plus 2 px): at 125 % a field was 22.2 points on a 22-point line, cut.
- kittest keeps a harness 420 points wide under `ui_scale` and `LEAST_WINDOW_SIZE` is in points: the dock is never narrower than ~379 points, so the Claims Table's folded row menu is reachable only in the unit test (`claim_columns`).
- A job remembers its module (`Job.module_of`, `modules_opened`): one that ends after its module was closed or replaced hands nothing over (`MODULE_GONE`), or the old module's hits land on the new one.
- Mark as Used writes `[used]` at once, outside the module's undo: Ctrl+Z does not take it back; Counted as Used's Remove does.
- Find Instance's test waypoints need LocString words (the game's name is a talk-table line, which text search does not read); Verify needs scripts that are missing (OnEnter/OnExit) to find anything.
- Timing after the review round (`target/ui-mock/perf-m4/u28-fix/summary.txt`): the dock lists are no slower than the tip's dialogs (Find Instance 1.46 against 1.51 ms a frame; Build Module in Problems 1.46 against 1.93).
