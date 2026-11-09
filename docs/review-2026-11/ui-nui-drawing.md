---
type: Report
title: 'Code review of November 2026: the NUI Creator''s drawing side, the Script Wizard''s window, the manual and the icons'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the NUI Creator''s drawing side, the Script Wizard''s window, the manual and the icons (mg-ui): 18 in all (2 high, 4 medium, 12 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-ui, nui, preview, script-wizard]
status: draft
generated: { by: claude-code/claude-fable-5-2, at: 2026-11-09T22:00:43Z }
stale_after: 2027-01-01T00:00:00Z
sources:
  - id: code
    resource: 'The source named in the title, at commit a46ba28a (develop, 2026-11-09), read by a reviewing agent under the brief the overview gives'
    title: The code as read
  - id: last-review
    resource: commit 2253fa78 (2026-10-27)
    title: The last whole-workspace review, which an entry's Since is told against
---

# Code review of November 2026: the NUI Creator's drawing side, the Script Wizard's window, the manual and the icons

One part of [the review of 9 November 2026](overview.md): the NUI Creator's drawing side, the Script Wizard's window, the manual and the icons (mg-ui). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it. Paths are under `crates/mg-ui/src/` unless they begin with `crates/` or `apps/`.
## High

### N1. A control's `aspect` makes it millions of points high, and its scrollbar's cursor is tiled along all of it: the Creator hangs and takes gigabytes

- **Kind:** bug
- **Severity:** high (hang or out of memory on opening a window, with unsaved work in other tabs; `mg nui render` and `layout` too)
- **Where:** nui_view/skin.rs:882-894; nui_view/preview/containers.rs:248-251 (also 232, 236, 257: sizes given past the clamp)
- **Since:** before 2253fa78 (1dac011e, 7d7a545e)
- **Scenario:** a JUI whose root column holds `{"type":"list","aspect":0.000001,"row_count":0,"row_template":[]}` (typed in JUI source, or a downloaded window), the game's skin loaded (its `scrollv.cursor_normal` is a picture). The list is 400 / 1e-6 = 4e8 pixels high, its thumb as long: 25 million image shapes in the first frame (several GB), and from 2.7e8 on the loop no longer advances. A group with `"scrollbars":3` or a text with `"scrollbars":2` and such an `aspect` does the same. With `aspect` 0.001 it is 25,000 shapes a frame: slow, alive.
- **Fix:** hold the size `children` gives a child to what `natural` holds it to (4,096 points a side, scaled); in `paint_tiled` start and stop at the painter's clip, and leave the loop when `end <= at`.
- **Test:** `hostile.rs`: add 1e-3, 1e-6 and 1e-9 to the sizes and give `"aspect": size` to a list, a group with `"scrollbars": 3` and a text with `"scrollbars": 2` (its skin already names a picture for `scrollv.cursor_normal`); the shape count and the time bound it has catch it.
- **Confidence:** confirmed by reading. `preview/containers.rs:248-251` (`size.y = size.x / aspect`, after `natural`'s 4,096 clamp at `preview/layout.rs:90` and not held to it), `preview/node.rs:198-204` (the rectangle is finite, positive and its top is in the clip, so it is drawn), `containers.rs:304, 367-373` (a list has a vertical bar by default), `node.rs:531-546` (the thumb is at least 5% of the track), `skin.rs:879-894` (`while at < rect.max[axis]`, one `p.image` a tile, nothing skipped out of the clip; egui 0.36.2 `Painter::add` keeps every shape). Past 2^28 pixels `at + 16.0 == at` in `f32` and the loop has no end. `preview/tests/hostile.rs:133-153` tries `aspect` on a label and an image only, with values that are 0 or huge as `f32` (1e-300 is 0): no small positive one.
- **Read again by the reviewer:** holds: `size.y = size.x / num(&child["aspect"], ..)` after the clamp (containers.rs:248-251), and `paint_tiled` adds one image a tile until `at` passes the rectangle's end, with nothing held to the clip (skin.rs:882-894).

### N2. The canvas's own walk for picture names recurses without end on a bind whose value names a bind: opening the window's tab overflows the stack

- **Kind:** bug (and duplication)
- **Severity:** high (a stack overflow aborts the program, unsaved work in every tab lost; each time the tab is opened once the module is saved so)
- **Where:** nui_view/skin.rs:504-510; crates/mg-nui/src/lib.rs:499-512 (the other copy)
- **Since:** before 2253fa78 (b2a9b17e)
- **Scenario:** Binding JSON (or an imported `NAME.txt`): the bind `icons` has the value `[{"bind":"icons"}]`, and an image's `value` (or a draw layer's `image`) is `{"bind":"icons"}`. As the field is left the Design page's next frame dies in `prepare`; `mg nui layout` and `render` die the same way. Two binds naming each other do too.
- **Fix:** one function (`mg_nui::image_names`, with a switch for the variants, which `skin::names` leaves out) that resolves a bind once: inside a bind's value a `{"bind":…}` object is data.
- **Test:** unit test in `skin.rs`: `names` of that window and settings returns; `hostile.rs`: add `{"bind": "rows"}` to the `rows` bind's array.
- **Confidence:** confirmed by reading `skin.rs:503-514` (`add`: `resolved`, then `add` again on each item of an array), `nui_view.rs:2278-2280` (`resolved` gives the bind's value for any object with a `bind`), `skin.rs:599-600` (`prepare` calls `names` every frame of the Design page: `nui_view.rs:1256`) and `headless.rs:559`. The hostile test's `rows` bind holds no bind. The same function is written a second time as `mg_nui::image_names` (`crates/mg-nui/src/lib.rs:498-542`), reported there by the `rules-leftovers` reader (its finding 1) for Export and `mg nui preview`: this copy runs each frame.
- **Read again by the reviewer:** holds: `add` resolves a bind and calls itself on each item of an array (skin.rs:503-510), and `resolved` gives a bind's value for any object with a `bind` (nui_view.rs:2278-2280).

## Medium

### N3. One `skin::Assets` serves every Creator tab: two tabs in view (or Split source while it is typed in) reload the skin, the pictures and the fonts twice a frame, without end

- **Kind:** performance (state shared that is one window's)
- **Severity:** medium
- **Where:** lib.rs:586; nui_view.rs:1256, 1292; nui_view/skin.rs:609, 641-646, 696, 779
- **Since:** before 2253fa78 (b2a9b17e) for two tabs; after (6201c463) for Split source
- **Scenario:** (a) two windows side by side in the dock (copying controls from one to the other, which `shortcuts.rs:81` provides for), one naming a picture the other doesn't: each frame both reload everything and ask for the next frame; the app repaints at full rate with disk reads, texture uploads and every text laid out again. Two windows naming the same pictures (or none) don't. (b) Split source, typing a picture's name: `design_view` prepares for the module's window and then for the typed one (`nui_view.rs:1292`), each frame, until the field is left; and `typed_source` (1441-1456) parses the whole typed text each frame, not each key.
- **Fix:** keep the skin (table, leaves, fonts, its pictures) by the load order alone, shared; keep each window's own pictures in its `State` (or in a map by name that is never trimmed to one window); parse a typed source when it changes.
- **Test:** a unit test in `skin.rs` with a counting `Module`: `prepare` for window A, B, A, B reads each picture once; `nui_perf`: two Creator tabs drawn in one frame, idle frame inside the budget.
- **Confidence:** confirmed by reading; the cost is not measured. `lib.rs:586` (one `nui_assets`), `nui_view.rs:1256-1263` (each tab's `design_view` prepares it for its own window), `skin.rs:599-609, 641-646` (the signature is the load order and the names this window wants: another window's names are another signature, so nothing is kept), `skin.rs:647-779` (then: the TML read and parsed, every picture of the skin and the window read and hashed, `pictures.retain` at 696 drops the other tab's pictures so they are decoded and uploaded again next call, every font read and hashed, the kept texts dropped at 654, and `request_repaint` at 779). The earlier reader's finding 20 has the reload for a changed name; this is the reload happening on every call. Searched `docs/deferred.md` and `docs/log.md` for `nui_assets` and two tabs: nothing.

### N4. The built-in manual follows only links that are a chapter's bare file name: 176 of its 315 links open the system's browser on a relative name

- **Kind:** bug
- **Severity:** medium (a builder clicks a link in Help › User Manual and nothing useful happens)
- **Where:** manual.rs:344-346, 285-287, 365, 315
- **Since:** before 2253fa78 (30d8cea3; the anchored links came with the chapters)
- **Scenario:** in Main Window, click "whose each area is" (`03-modules.md#claimed-areas`): the desktop is asked to open `03-modules.md#claimed-areas`; in Areas, click a link to `#tiles`: the same with `#tiles`. Neither changes the chapter or scrolls. The one picture (`03-modules.md:2035`, `../images/2da-table-described.png`) is not built in either (no image loader is installed: suspected to show as a failed image).
- **Fix:** hook every destination the chapters hold (gathered once from `CHAPTERS`), go to the chapter before the `#`, and scroll to the heading (split the blocks at headings and `scroll_to_me` the one whose slug matches); leave the picture out or build it in.
- **Test:** extend `links_between_chapters_resolve`: every `](x.md#y)` and `](#y)` names a chapter and a heading of it; a UI test that clicks an anchored link and finds `manual.chapter` changed and no `OpenUrl` command in the output.
- **Confidence:** confirmed by reading `manual.rs:344-346` (a hook for each of the 19 file names), `manual.rs:285-287`, and egui_commonmark_backend 0.25.0 `misc.rs:227-241` (a destination that is not exactly a hook's key, and is not a heading link with `enable_scroll_to_heading`, which is off by default and needs `{#id}` headings, goes to `ui.hyperlink_to`, that is `open_url`). Counted in `docs/manual/*.md`: 120 links `chapter.md#heading`, 56 links `#heading`, 139 bare ones. The test `links_between_chapters_resolve` (552-559) looks only at targets ending in `.md`. Nothing in `docs/deferred.md`.

### N5. The Script Wizard keeps its conversation line by position and outlives the module: Finish can set the script on another line, or write into the next module

- **Kind:** bug
- **Severity:** medium (a script silently on the wrong line; Undo takes it back)
- **Where:** script_wizard.rs:79, 902, 996-1001, 1016; lib.rs:3478 (beside `nui_views.clear()`)
- **Since:** before 2253fa78 (bb79ea90)
- **Scenario:** Actions Taken › Script Wizard… on the sixth reply; with the wizard up, delete the second reply (or Undo an added one) in the conversation behind it; Finish: `Script` is set on what is sixth now. With the module closed and another opened meanwhile, Finish writes the script there (and sets the field where that module has a conversation of the name); where the path is gone the command fails after the log has said "Script Wizard: wrote …" (1016 logs before the action is applied) and the wizard's pages are lost.
- **Fix:** drop the wizard in `close`; at Finish check that the line at `path` is the one it was opened for (keep the struct as it was and compare, refusing with the wizard left open), or follow the line as `dialog_view`'s `follow` does; log after the command applied.
- **Test:** `tests/app/conversations.rs`: open the wizard on a line, apply a command that removes an earlier line, Finish: no other line's `Script` changed; `close()` leaves `script_wizard` `None`.
- **Confidence:** confirmed by reading `script_wizard.rs:78-80` (`dialog`, `path`, `label` taken when it opens: `dialog_view.rs:1976`), `994-1001` (Finish sets the field at that path), `widgets.rs:2981-2997` (`widgets::modal` is a plain window: the editor behind it stays live), `crates/mg-gff/src/path.rs:25-30` (a path's list items are indices), `lib.rs:3440-3530` (`close` does not drop `script_wizard`; nothing else does). The same class as the `ui-editors` reader's finding 15 and the `ui-blueprints` reader's finding 7, neither of which names this window.

### N6. `mg nui layout` and `render` lay out 512 rows of every list at every level: lists nested three deep are 134 million controls, and the overlaps are found by comparing every pair

- **Kind:** performance (hostile input)
- **Severity:** medium (the command hangs or runs out of memory on a file of 300 bytes; the editor is not touched)
- **Where:** nui_view/preview/containers.rs:398-401; nui_view/headless.rs:641-653; nui_view/skin.rs:323
- **Since:** after (af178888, the rows out of view in `layout`)
- **Scenario:** `{"type":"list","row_count":1000,"row_template":[[{"type":"list","row_count":1000,"row_template":[[{"type":"list","row_count":1000,"row_template":[[{"type":"label"},0,true]]},0,true]]},0,true]]}` in a window: 512³ labels entered and drawn, up to six frames. Two levels (262,144) already take seconds a frame and 3.4e10 pair comparisons after.
- **Fix:** one budget of rows for a frame (512 in all; a nested list laid out for its first row only), said in the layout's `issues` when it cut; overlaps by grouping `placed` on (parent, row) first; `Placed` keeps the control's own fields, not its children.
- **Test:** `headless.rs` unit test: that window's `layout` returns inside a second with a bounded `elements` and an issue saying rows were left out.
- **Confidence:** confirmed by reading `preview/containers.rs:398-401` (recording: rows 0 to `ROWS_DRAWN` of each list, whatever shows), `preview/node.rs:171-183, 198-204` (recording, what is out of view is drawn all the same), `skin.rs:309-333` (`enter` copies the control's whole subtree for each), `headless.rs:641-653` (all pairs of `placed`). The nesting stops at 32 (`node.rs:198`), not the product. In the editor a nested list draws only the rows that fit its row, so it is bounded by the pixels.

## Low

### N7. Pressing and letting go of a handle without moving writes a size (or rounds a draw layer's place): an edit from a click

- **Kind:** bug
- **Severity:** low
- **Where:** nui_view/preview/stage.rs:225; nui_view/interaction.rs:305-306, 361-364; nui_view/draw.rs:398
- **Since:** before 2253fa78 (d91c815b, 1dac011e, 226d1317)
- **Scenario:** three labels with no width share a row 380 wide (126.67 each). Select one, click its right handle: `"width": 127.0` is written, one Undo step, and the label no longer shares the row. With Zoom Fit under 100% a size divided by the scale is never whole, so any click on a handle does it. A draw layer at x 10.3 (the number fields step by 0.1) becomes 10 at a click on its canvas handle, and `10` becomes `10.0`.
- **Fix:** apply an axis only when the pointer's move on it rounds to something other than 0 (keep `size` and the draft as they were until then).
- **Test:** unit tests: `Resize { initial: vec2(126.67, 30.0), .. }` after `move_to(origin)` leaves the release's condition false; `Gesture::drag_to(origin, 1.0)` leaves `draft == source` for `x` 10.3 and for the integer 10.
- **Confidence:** confirmed by reading `interaction.rs:283-310` (a handle senses drags only, so egui 0.36.2 starts the drag at the press: `interaction.rs:196-217` of egui; `initial` is the drawn size divided by the scale, not rounded), `interaction.rs:356-366` (`move_to` rounds and clamps), `preview/stage.rs:220-228` (`size != initial` decides); `draw.rs:297-301, 367-373, 390-400` (`drag_to` rounds what the layer had; the thumbnail's release at 533-537 writes with no comparison).

### N8. The draw layer chosen on the canvas is an index that Up, Down and Remove Drawing don't move; a canvas drag is not dropped when its layer changes

- **Kind:** bug (and duplication)
- **Severity:** low
- **Where:** nui_view/draw.rs:287-293, 367-373, 664-682; nui_view.rs:1208
- **Since:** before 2253fa78 (b8462f20, d604dbe1)
- **Scenario:** (a) choose the third layer in Layers (its handles show on the canvas), press Up under Draw layers: the handles are now on the layer that was second, and a drag there moves that one; Remove Drawing puts them on the next layer. (b) Undo with a layer's handle held: the release writes the layer as it was before the Undo over what is at that index now.
- **Fix:** `editor` says what it moved or removed and `draw_item` follows (cleared on Remove); drop `draw_drag` when `doc.pointer(path) != gesture.source`; one function for the handles, which the canvas (`canvas_handles`) and the properties' thumbnail (`draw.rs:482-537`) each write.
- **Test:** a UI test on `draw::editor`'s fixture: choose layer 2, Up, `draw_item` is `(path, 1)`; unit test: `canvas_handles` with a drag whose `source` is not the document's layer leaves the document alone at release.
- **Confidence:** (a) confirmed by reading `draw.rs:287-293` (the chosen layer is `state.draw_item`, a path and an index), `draw.rs:664-682` (the editor moves and removes items and has no `State`), `nui_view.rs:1208-1212` (only an index past the end is forgotten). (b) suspected: `draw.rs:297-301, 367-373` apply the draft at release with no look at whether the layer is still `gesture.source`, where the thumbnail's drag (450) and `Resize` (`stage.rs:177`) are dropped; what would settle it is whether Edit › Undo's key is taken while a pointer button is down.

### N9. A list keeps how far it is scrolled in pixels, a group and a text in points: a change of zoom or UI scale moves a scrolled list (the scroller is written four times)

- **Kind:** bug (duplication, drifted)
- **Severity:** low
- **Where:** nui_view/preview/containers.rs:84-159 (group), 326-377 (list); nui_view/preview/text_controls.rs:127-193 (text); nui_view/preview/combo.rs:84-101 (the popup)
- **Since:** before 2253fa78 (7d7a545e)
- **Scenario:** scroll a list of 40 rows to row 30, then UI scale 100% to 200% (or Zoom Fit to 100%, or with Fit the pane made narrower): the list shows row 15; a group and a text beside it stay where they were.
- **Fix:** one scroller (the strip, the wheel, the end buttons, the track's mapping, the offset kept in points) for the four; the copies also differ in what a click on an end button does (20 points, a font's height, nothing for a list).
- **Test:** unit test on the list: the first row shown is the same before and after `preview_scale` doubles.
- **Confidence:** confirmed by reading `preview/containers.rs:326-327, 377` (the list reads and stores `offset` as it is), against `containers.rs:92, 155-157` (group: stored divided by the scale, "Logical units keep the visible content stable when zoom changes") and `preview/text_controls.rs:119-122, 194-195` (text: the same).

### N10. A correction the simulation can't write is tried again every frame: Interact repaints without end and its event line shows nothing else

- **Kind:** bug
- **Severity:** low
- **Where:** nui_view/interaction.rs:126-135; nui_view/preview/combo.rs:30-36; nui_view/preview/sliders.rs:37-48
- **Since:** before 2253fa78 (b2a9b17e)
- **Scenario:** Import… a `.jui` with no `.txt` beside it (it gets a new window's settings: no binds) whose combo has `"value":{"bind":"choice"}`; Interact: every frame the combo asks for its first entry, `apply` answers "Missing preview bind: choice", and the next frame is asked for. The same for a slider bound so with `min` above 0, and for a bound combo in a list whose `row_count` is more than its bind's array holds.
- **Fix:** don't push a correction whose bind or row isn't there (or note the failure for the path and row until the session starts over).
- **Test:** a canvas test: such a window in Interact, three frames: the second and third have no change and ask for no repaint.
- **Confidence:** confirmed by reading `preview/combo.rs:27-36` and `preview/sliders.rs:36-48` (a "normalization" whenever the value is not an entry's or is out of range; a bind with no value resolves to the bind object, a row past the array to null), `preview/node.rs:395-404`, `interaction.rs:124-135` (no bind, or no such row: `last_event` set, nothing written), `preview/stage.rs:255-260` (any change asks for another frame).

### N11. `mg nui render --variant` on a JUI that is not an object panics

- **Kind:** bug
- **Severity:** low (hostile file, and the option)
- **Where:** nui_view/layouts.rs:167, 170; nui_view/headless.rs:452
- **Since:** after (af178888)
- **Scenario:** `w.jui` is `[]`, `w.txt` has `"views": {"v": {"type":"col","children":[]}}` and `"view_targets": {"v": "_window_"}`: `mg nui layout w --variant v` panics with "cannot access key "root" in JSON array".
- **Fix:** `document` returns `Error::Read` for a window that is not an object (as the editor says it), before anything indexes it.
- **Test:** `headless.rs` unit test: that module and `Options { variants: vec!["v".into()], .. }` give an `Err`.
- **Confidence:** confirmed by reading `headless.rs:426, 451-453` (`mg_nui::parse` gives any JSON; the size option checks `is_object` at 484, the variants don't), `layouts.rs:165-170` (`result["root"] = …` and `&mut result["root"]`: serde_json's `IndexMut` panics on an array, string or number), `apps/mg/src/nui/preview.rs:93-99, 246, 283` (no check before). The editor refuses a window that is not an object (`nui_view.rs:779-784`), so only the command line reaches it.

### N12. A row, column or group looks at every later child's width for each child, every frame: quadratic in its children

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view/preview/containers.rs:195-201
- **Since:** after (da8bd982)
- **Scenario:** a scrolling group holding a column of 300 rows (a shop's or a roster's entries built as rows, where a list's one row height doesn't do): 45,000 lookups a frame, about 2 ms of the 6.9; 1,000 rows: 500,000, the budget several times over, for rows that are then not drawn.
- **Fix:** one pass from the end keeping the largest width seen (a suffix maximum), and only for a column.
- **Test:** `nui_perf`: a group with a column of 1,000 rows of two controls, an idle frame inside the budget.
- **Confidence:** confirmed by reading `preview/containers.rs:195-201` (`widths_after`: for each child a fold over all after it, each a `num` with its map lookup and bind resolution), used only at 260-264 and only when the parent is a column and the child a row. Not measured; `nui_perf`'s window has 12 children in its root.

### N13. Each frame of the Design page copies the window and the settings twice and compares them twice; editing a variant copies them four more times and walks the result building a path for every value

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view.rs:792-793, 1191-1192, 1272-1278; nui_view/layouts.rs:115-121, 131-133
- **Since:** before 2253fa78
- **Scenario:** a data-driven window of 300 controls with a list bound to 1,000 rows of five columns, and a swap layout whose variant is being edited: about 10,000 JSON values copied six times a frame and compared twice, plus a few thousand path strings.
- **Fix:** compare against the window `state.source` already holds instead of a second copy (and hand the pages a copy only once something is set); keep the variant's context document for the bytes and the variant it was made from; build `group_path`'s path on the way back from the group found; keep Split source's text by the document it was written from.
- **Test:** `nui_perf`: that window, an idle frame of the Design page and one with the variant edited, inside the budget.
- **Confidence:** suspected (read, not measured: `nui_perf` has 100 controls, no binds and no variant). `nui_view.rs:792-793` (`check` hands out copies of the parsed window and settings), `1191-1192` (`before`, `before_settings`: copies of the same again), `1224-1229` (compared whole), `1254` and `preview/stage.rs:203` with `interaction.rs:82-84` (Interact: the session compared whole with both, twice a frame); `layouts.rs:111-123` (a variant: the settings cloned, the variant cloned twice, the main window cloned in `context_document_inner` at 165), `layouts.rs:126-139` (`group_path`: a `format!` and two `replace`s for every key it passes); `nui_view.rs:1272, 1278` (Split source: the window or the settings written out as text every frame). What would settle it: `nui_perf` with the window below.

### N14. In Interact every combo copies and measures all its entries every frame, its list closed

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view/preview/combo.rs:66-77
- **Since:** before 2253fa78 (7d7a545e)
- **Scenario:** a combo bound to the names of an `appearance.2da` of 7,000 rows (or of every item of a module): 7,000 strings, 7,000 vectors and about 100,000 glyph widths a frame while Interact is on, most of the frame's budget.
- **Fix:** make the entries and the width inside the popup's closure (or when `Popup::is_id_open`).
- **Test:** `nui_perf`: a combo of 10,000 entries in Interact, an idle frame inside the budget.
- **Confidence:** confirmed by reading `preview/combo.rs:54-56, 66-77` (`combo_popup` runs whenever the control is interactive: a `String` an entry, then `native_advances`, a lock on the fonts and a `Vec` an entry; only the closure of `Popup::menu` at 78-82 is for the open list). Not measured.

### N15. The list of problems opens by itself and draws every finding every frame, each with a walk of the document

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view.rs:978-1006
- **Since:** before 2253fa78 (aa9e5b42)
- **Scenario:** Import… a window written by a script with one mistake in each of 2,000 controls (an ID used twice, a size its row can't hold): the list opens and every frame lays out 2,000 wrapped rows of four widgets, tens of milliseconds, on every page of the tab until it is closed.
- **Fix:** `ScrollArea::show_rows` over the sorted findings (a row a finding, cut to one line with the rest in its tip), the places described once for the findings' `Rc`.
- **Test:** `nui_perf`: a window with 2,000 findings, the list open, an idle frame inside the budget.
- **Confidence:** confirmed by reading `nui_view.rs:963-965` (more errors than last frame: opened), `970-1007` (a `Vec` sorted and every finding laid out in a 160-point scroll area with no `show_rows` and no `is_rect_visible`), `1988-2009` (`describe_path` looks the path up once for each of its levels).

### N16. A chart makes a shape for every point and a polyline takes every point, in view or not

- **Kind:** performance (hostile or careless input)
- **Severity:** low
- **Where:** nui_view/preview/chart.rs:32-40, 51-84; nui_view/draw.rs:154-165
- **Since:** before 2253fa78 (7d7a545e)
- **Scenario:** a chart bound to a log of 1,000,000 samples (a bind's array a script fills, tried in Binding JSON): three million-item vectors and a million shapes a frame, seconds each; in a list's row template, once a row.
- **Fix:** at most one mark a pixel of the chart's width (the lowest and highest value of the samples that fall on it), and a polyline's points held to a count that is said.
- **Test:** `hostile.rs`: a chart of 1,000,000 values and a polyline of 1,000,000 points inside its time bound and shape count.
- **Confidence:** confirmed by reading `preview/chart.rs:22-40` (two vectors of all the values), `51-75` (a column: a rectangle and a lock on the input for each), `77-84` (a line: the points copied, then a rectangle each), `draw.rs:152-167` (a polyline of every pair), `preview/buttons.rs:117-161` and `preview/layout.rs:142-162` (Options and Tabs: every entry's rectangle made and painted, whatever the clip). `hostile.rs` has charts of five values.

### N17. A font's size is held to 4 to 128 but not its ratio of em to ascent minus descent: a font with a tiny `hhea` height is drawn thousands of pixels high

- **Kind:** bug
- **Severity:** low (a hostile or broken TTF in a hak the skin names)
- **Where:** nui_view/skin.rs:40, 757
- **Since:** before 2253fa78 (b2a9b17e)
- **Scenario:** such a font in a downloaded hak, units per em 2,048: the ratio is 2,048, a 17-point font 34,816 pixels; the first text of the preview allocates gigabytes for a glyph or stops at epaint's assertion.
- **Fix:** clamp the ratio (0.25 to 4) and say "Invalid font" outside it; hold the skin's numbers to a range as `preview::num` holds the window's.
- **Test:** unit test of `pixel_height_to_em` with made-up `hhea` and `head` tables; the hostile test with that font.
- **Confidence:** suspected. Read `skin.rs:32-41` (`em / height`, any positive height), `757, 772-774` (size and spacing clamped, the ratio not), `804` (`size * em_ratio * scale` is the `FontId`'s size), and epaint 0.36.2 `text/font.rs:267-283` with `texture_atlas.rs:226-231` (a glyph's bitmap is made at its size, up to 65,535 a side, then `assert!(w <= atlas width)`). What would settle it: a TTF whose `hhea` ascender is 1 and descender 0, named in `nui_skin.tml`, drawn in `hostile.rs`. (`toml_number` at 899-904 also lets 1e300 through as `f32` infinity; I traced the scrollbar sizes and paddings with it and found rectangles that are not drawn, no loop or panic.)

### N18. The font test leaves out ten of the glyphs `icons.rs` names

- **Kind:** hygiene
- **Severity:** low
- **Where:** icons.rs:384-425, 135-172
- **Since:** after (af9e8a48 and the constants added since)
- **Scenario:** a glyph of these that the fonts lack is drawn as a square in the tiles palette or the module tree, and no test says so.
- **Fix:** one table of the glyphs (a macro or a `const` slice the constants are read from) that the test walks.
- **Test:** the existing test over that table.
- **Confidence:** confirmed by reading `icons.rs:384-425` (`Tools::glyphs`) against the constants at 7-15 and 60-62: `TILES`, `ERASER`, `RAISE_LOWER`, `REFINE`, `NEXT_APPEARANCE`, `PREVIOUS_APPEARANCE`, `ROTATE`, `CLAIMED`, `CLAIMED_MINE` and `MERGED` (used by `terrain_mode.rs:1975-1988` and `claims.rs:454-527`) are in no list `the_fonts_have_every_toolbar_glyph` draws. Whether one of them is missing from the fonts I could not see without running it. `editor::ALL` is likewise kept by hand beside its constants.

## Not read

- Nothing of the part's code. Read whole: `nui_view.rs` 1-2280, `nui_view/preview.rs` and all twelve files of `preview/`, `layouts.rs`, `draw.rs`, `interaction.rs`, `shortcuts.rs`, `skin.rs` 1-956, `headless.rs`, `script_wizard.rs`, `manual.rs`, `icons.rs`.
- Tests, read only where they say what is meant: `preview/tests/hostile.rs` whole; the test modules inside the files above as far as they came with the code; `nui_view.rs` 2281-2636, `skin.rs` 957-1398, `preview/tests.rs`, `preview/tests/canvas.rs` and `native.rs` not read.
- Outside the part, read to settle findings: `mg_nui::parse` and `image_names`, `crates/mg-gff/src/path.rs` (head), `widgets::modal` and `Dropdown::show`, `Moonglow::close`, `references.rs` 430-548, `mg-module`'s `script_wizard.rs` (`quote`, `default_name_named`) and `project_file.rs` (`Naming::fit`), egui 0.36.2 (`Painter::add`, the drag's start), epaint 0.36.2 (`TextureAtlas::allocate`, `FontFace::new`, glyph allocation), egui_commonmark_backend 0.25.0 (links).
- Nothing was run: no finding is measured, and 1, 2, 6 and 17 are not reproduced.

## Looked at and found sound

- Hostile numbers on the canvas: `num` holds every number to ±1e18 and finite, `natural` to 4,096; every `f32::clamp` has ordered bounds that can't be NaN (the scroll offsets, the slider behind `max > min`, the image region, `Resize::move_to` after its `max`); a rectangle that is not finite or not positive stops at `draw_node`; float-to-integer casts saturate and are held by `count` and `ROWS_DRAWN`; a picture of no size is refused by the readers (`mg-image` `lib.rs:89`).
- Depth: `draw_node` and `natural` stop at 32 levels, `Session::dispatch` at 32 actions; `fills_height`, `group_path`, `replace`, `rename_binds` and `assign_ids` go only as deep as the JSON (serde_json's 128); `context_document_inner` keeps the variants it has been through.
- Windows that are not what they are taken for: `interaction::field` and `set` never index a non-object; every `value["key"] = …` in `nui_view.rs`, `layouts.rs` (in the editor), `draw.rs` and `shortcuts.rs` is behind a check that the value is an object; a resize and a window's move are dropped when the document changes under them and applied as one edit.
- Text: `cut` and `native_wrap` slice at characters and always advance for a positive width (the width given is finite: `text_block` takes it from a finite body, and `.max(1.0)` the second time); `NativeTextBuffer` cuts an insert at a character boundary inside its byte budget; kept galleys are dropped when egui's atlas is another (`TextCache::begin_pass`).
- The Script Wizard's text: every string typed reaches the script through `quote` (backslash, then quote), a single-line field holds no line end, the name is a ResRef, the lists hold 2DA rows not positions, and the selected row is cleared with each page.
- `manual.rs`: `body`, `cells` and `blocks` (`lines[i + 2..end]` is always in range: a table's first two lines are rows), `column_widths` fixes a column or ends each round. `icons.rs`: the bit tests' precedence, `tip` sliced with `get`.
