---
type: Gotcha
title: "Right Dock tool sections (U29): Edit Areas Together, Custom Light and Bake as sections; keys, Enter and Escape, the dock's split"
description: "How M4's U29 turned Edit Areas Together, Custom Light and Bake Walkmesh into Right Dock sections and the Plugins window into a tab, the traps found doing it, and what its review corrected (Enter, Escape, presses outside, the split, the mixed mark)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T00:06:51Z }
touches: [crates/mg-ui/src/tool_sections.rs, crates/mg-ui/src/area_props.rs, "crates/mg-ui/src/shell.rs:tools_heights", "crates/mg-ui/src/report_table.rs:has_keys", "crates/mg-ui/src/blueprint/mod.rs:paint_mixed_mark", crates/mg-ui/tests/app/tool_sections.rs]
asks:
  - "Why did Enter in a shared page's field open a tab?"
  - "How do the Right Dock's tool sections take and give back the keys?"
  - "How is the Right Dock's height split when a tool section is open?"
  - How do I tell whether a field had the keyboard when Escape was pressed?
fingerprint: v2-4a0da5ea1cfcf951
created: 2026-11-20
author: augustaugur
---
- tool_sections.rs: `Panel::{AreasTogether, LightMaker, Bake}`. A command sets `app.tools.bring`; the shell shows the Right Dock (even hidden or folded) with the section in front and gives it the keys (`palette_view::hold_keys`). Any press outside the section takes the keys (`take_keys`), Properties included (it calls `give_keys(Properties)` on a press, as a pane does). `actions(.., why)` draws the top row (main action, Cancel/Close, detail) and, when the main action is disabled, why under it in sight. `keys_outline` is drawn at the end of `body()`, after the panel: drawn first, the list and rail covered it (U29 review).
- Enter: `enter_after(ui, keys, enabled, from_field)`. Edit Areas Together's Enter is Open in Tab on its Areas page alone, and never on the frame a field lets go (egui's one-line field gives up focus on Enter in the same frame). Escape: egui clears focus at Escape before widgets draw, so "was a field focused" is `widgets::field_had_keyboard`, recorded at the end of the frame (`dialogs_end`); Escape in a field gives up the field only. Popups: `widgets::popup_was_open` (= `escape_closes_a_popup`), set as the frame began.
- `report_table::has_keys` missed the check for a popup open as the frame began: Enter, Space or an arrow that closed a ComboBox list went to the row under it too. Fixed at the U29 review (it returns false while `popup_was_open`).
- Sections close when another module opens, judged by module identity (not by path or revision).
- Dock split: `shell::tools_heights` gives the tools section `tool_sections::share` (2/3 at first) of the column; a Properties with nothing selected (`properties_panel::nothing_selected`) is shut to its header (`DOCK_TABS`) and gives its share to the tools; dragging the seam above the tools sets `app.tools.share` (kept for the session, not in settings). No rule that can flip each frame.
- Area Properties' pages are shared with the section (`Shared.also: Arc`, cached `TogetherSet` keyed by module revision, ticks, `project_reads` and `settings.area_names`; the claims cached in it). `tools.walks` counts walks for the 3,000-area test. Never walk the areas in a frame.
- ≠ mark: `blueprint::paint_mixed_mark` (a badge after the field's label via `widgets::label_before`, else in the gutter; `mark_mixed_inline`, `mixed_mark(ui, tip)`); its AccessKit label "≠", value "mixed": tests count `by_value("mixed")`.
- egui's Grid sizes a column before a wrap, so a two-column form ran past the narrow dock. `area_props::fields(ui, id, two_columns, add)` stacks label over field below a width. Check boxes in a grid's field column need their own max width (`blueprint::room_right`) or they run past the dock at 420. Held by `every_page_fits_the_least_right_dock`.
- `AreaChooser::holding` must be asked after the Holding fields draw.
- Tab into sight: `tests/app/tool_sections.rs::tabbed_in_sight(h, room, most)` presses Tab round the whole window; judge by the clip (`interact_rect`), not the rect's middle.
- Kittest: a secondary click with press and release in one frame opens a tab's menu with every row disabled (a drag began): send them in separate frames.
- Pictures: `tool_sections::picture_of(h, name, what, shown)` asserts the thing named is on screen before saving (three U29 pictures' names lied).
- Plugins is `Tab::Plugins` (a document tab; `Action::OpenTab(Tab::Plugins)` in tests).
- Breaks: `target/u29-fix/breaker.py` applies one edit, runs a test filter, logs to `breaks.txt`, puts the file back (reading the file as the break starts: an edit made to that file meanwhile is lost); a break that does not compile reads DID NOT FAIL.
- `tests/one_gate.rs` scans mg-ui's source for `ws.module.set`, `.module = ` and the like: a new Module setter or a field named `module` trips it. The project file read again by a plugin is `Module::reread_project_file_as`; the tools' module identity is `tools.workspace`.
