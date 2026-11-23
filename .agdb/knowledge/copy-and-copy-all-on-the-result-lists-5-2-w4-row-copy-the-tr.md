---
type: Decision
title: "Copy and Copy All on the result lists (5.2 W4): row_copy, the traps"
description: "How the result lists got Copy, Copy All, exact Ctrl+C and Shift+F10 on the chosen or focused row through row_copy, Problems' per-heading copy and save, and the test traps met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T17:33:45Z }
touches: [crates/mg-ui/src/row_copy.rs, crates/mg-ui/src/problems.rs, crates/mg-ui/src/report_table.rs, crates/mg-ui/src/references.rs, crates/mg-ui/src/nui_view/checks.rs]
asks:
  - "how do I add Copy to a list's row menu"
  - why does Ctrl+Alt+C also trigger Ctrl+C handlers
  - how do keys reach a row in a list with no chosen row
  - where does Verify write its findings
fingerprint: v2-b5882eba2e9db798
created: 2026-11-23
author: augustaugur
---
- `row_copy::row_menu(ui, &row, keyed, own)` is the one row menu: `own` draws the row's own commands first (Mark as Used, a plugin's), then a separator, Copy, Copy All (the module tree's order: the row's commands before the clipboard). Opened by a right click or `keyed` (Shift+F10 on the chosen row; `row_copy::menu_key`), at the pointer or under the row. Text is built only when asked, synchronously as Copy Log does (50,000 rows is milliseconds). `row_menu_saying` gives Copy All a list's own tip.
- `row_copy::copy_pressed` is exact (`copy_in`): egui-winit sends `Event::Copy` for Ctrl+Alt+C (the Creature Wizard's key) and Ctrl+Shift+C too, so the held modifiers must be none or exactly COMMAND. `report_table` uses it.
- A right click chooses the row (Problems, `report_table`, Search Results' lines and strings), so Ctrl+C after its menu copies that row.
- Lists that choose no row (Compiler, Bookmarks, NUI Checks, Advanced Search, References) take Shift+F10 and Ctrl+C through `row_copy::focused_keys(ui, &row)` on their focusable link or label. A row-wide `ui.interact` under the contents is `Sense::CLICK` (not a Tab stop), and the menu gets `link | row`: the union carries the link's id, so focus and the popup follow the link. A menu opened by keys gives the keyboard back to the row on Escape (temp data `row-menu-keyed`).
- In kittest a `widget_info` on a `ui.interact` row never showed in the AccessKit tree: make the place a `Label::selectable(false).sense(Sense::click())` Tab stop instead (NUI Checks rows with no link).
- `report_table::show` carries the menu for every report table (Find Instance, Hak Conflicts, Resources Used, Claims): Copy is what its Ctrl+C copies (`name\tvalue`), Copy All follows `order` (the sort).
- A clickable row's words must be `Label::selectable(false)`: a selectable label takes the right click (the builder's report: Verify's findings could not be right-clicked).
- Problems' line (`problems::copy_line`): kind, place (left out where the message already says it), whole message, "(source[, stale words])". A finding's Copy All copies every finding under every heading (`Problems::text`); a heading's menu (right click or the `…` Problems Actions button) has Copy These Findings, Save These Findings As… (`Problems::text_of`, `FileKind::Problems`) and Remove from Problems, all on that heading's source (the coordinator's ruling). Verify writes no file: its log summary ends with `problems::LISTED`; `tests/app/plugins.rs` strips it before `ends_with`.
- Tests: egui's `Label::truncate()` already shows the whole text as a tip: break a tip test with `.show_tooltip_when_elided(false)`. Use `common::tip_at` (a tip is suppressed after a click). Ties in a sort keep the results' order: test Copy All's order on a column whose values differ. To simulate Ctrl+Alt+C push `ModifiersChanged(mods)` and `Event::Copy` in one step and release in the next (`event_modifiers` releases in the same frame, so egui sees no modifiers). Ctrl+Alt+C in a test may open a menu of the app's own: close it with Escape.
- Break runs: never pass `--body` text starting with `-` to agdb (it is read as a flag; use stdin).
