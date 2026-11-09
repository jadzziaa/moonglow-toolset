---
type: Report
title: 'Code review of November 2026: plugins in the window, the NUI Creator''s editor side and the script editor'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in plugins in the window, the NUI Creator''s editor side and the script editor (mg-ui): 25 in all (1 high, 7 medium, 17 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-ui, plugins, nui, script-editor]
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

# Code review of November 2026: plugins in the window, the NUI Creator's editor side and the script editor

One part of [the review of 9 November 2026](overview.md): plugins in the window, the NUI Creator's editor side and the script editor (mg-ui). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it. Paths are under `crates/mg-ui/src/` unless they begin with `crates/` or `apps/`.
## High

### X1. Rename Symbol, References and Go to Definition work on script text that is not the module's: Rename replaces the wrong bytes, References can panic

- **Kind:** bug
- **Severity:** high (wrong bytes written into scripts, or a crash with unsaved work; the rename is one undoable command)
- **Where:** script_nav.rs:82-88, 140, 177-191; side_doc.rs:277; script_view.rs:78-80
- **Since:** (a) after (d74eb396, the script editor's Cancel); (b) before 2253fa78
- **Scenario:** (a) Open include `inc_a`, type 200 characters at its top, do not save. In `main` (which includes it) press F12 or Shift+F12 once: the index now holds `inc_a`'s unsaved text. Cancel `inc_a`'s editor (Don't Save): the buffer goes, the revision does not move, the index keeps the dropped text. Shift+F12 on a name used near the end of `inc_a`: `text[..r.span.start]` is past the module text's end, a panic (or, with `é` in a comment, a slice off a character boundary). Rename Symbol instead: each span is 200 bytes off, `text[at..s.start]` replaces other bytes of `inc_a` with the new name, and the log says "Renamed". (b) `win_e.nss` is open in a tab behind the NUI Creator, unchanged; the Creator (or a plugin's command, or Undo) rewrites it in the module. The buffer is clean but older, `sync_nav` indexes it, and a Rename Symbol from another script writes spans of the old text into the new one.
- **Fix:** Rename and References read each file's text from one place and apply to that same text: take the text from the index entry the spans came from (or re-index from the module after `store_script_text`, never from a clean buffer whose `source` is not the module's bytes), and `Index::forget` a script whose buffer is dropped. Slice with `get(..)` and refuse the rename when a span does not hold the old name.
- **Test:** `tests/app` script test: two scripts, type into the include, F12 in the other, Cancel the include with Don't Save, Rename Symbol; assert the include's module text differs from before only by the name. A second one for (b): `SetResource` on a script open in a hidden tab, then rename.
- **Confidence:** confirmed by reading. `script_nav.rs:75-89` (`sync_nav` gives the index every open buffer's text, and resets it only when the workspace revision moves), `crates/mg-script/src/analysis.rs:364-380` (`Index::set`/`file` keep a text until `forget`, which nothing in mg-ui calls), `side_doc.rs:277-279` (a script editor's Cancel drops the buffer and nothing else), `script_view.rs:560-585` (a buffer follows the module only while its tab is drawn; `nui_view/events.rs:30-32` says the same), `script_nav.rs:169-192` (the rename slices the *module's* text with the index's spans), `script_nav.rs:138-141` and `script_tools.rs:448-450` (`text[..r.span.start]`, `char_index`).
- **Read again by the reviewer:** holds for the mechanism: `sync_nav` gives the index every open buffer's text and empties it only when the revision moves, and `rename_symbol` slices the module's text by the index's spans (script_nav.rs:75-89, :169-191).

## Medium

### X2. What a plugin's events were allowed (a folder, a yes to write) carries over to changed code within a run

- **Kind:** bug
- **Severity:** medium (security: a grant given to one version of a plugin's code answers another's question unasked)
- **Where:** plugins.rs:1373-1389, 756-791, 586-605
- **Since:** before 2253fa78 for a plugin replaced by hand (the map by id is 5d90cb4f's); after for a project's plugin (82817304)
- **Scenario:** a project's plugin handles `module-saved`, asks for a folder and "Write 3 files?"; both are granted. A pull changes the plugin; Moonglow asks, the builder allows the new code. At the next save the new code's `ctx.ask_folder`/confirm of the same wording is answered from the old Arc without a question, and `keep_grants` then stores the grants under the new digest, so the next run does not ask either. The same for a plugin copied by hand with a new version and Reload, and for another project's plugin of the same id opened in the same run (the kept name has the folder in it for that case; the in-memory map does not).
- **Fix:** key `plugins.granted` by `grants_name(p)` (or drop the entry whenever the name computed for the id is not the one it was made for), and clear it in `read_project_plugins`.
- **Test:** extend `tests/app/plugins.rs`: a project plugin's event granted a folder, its `main.luau` changed and allowed again, the event queued: the question is asked again (`Ask::pending`), and `settings.plugin_grants` has nothing under the new digest.
- **Confidence:** confirmed by reading. `plugins.rs:1387-1389` (`granted` is kept by plugin id, made once), `1373-1381` (each frame it is written into the settings under the *current* `grants_name`, digest or version), `756-791` and `622-680` (allowing a changed project's plugin, and reading the project's plugins again, leave `granted` alone), `1065-1069` (only enable, disable, install, remove and Forget drop it), `1074-1077` (`forget_plugin_session` is called by a test only). `grants_name`'s doc (165-175) says another version asks again.

### X3. Replace All misses matches when the needle overlaps itself, and takes time by matches times text

- **Kind:** bug (and performance)
- **Severity:** medium (a builder sees fewer replaced than there are; a large script freezes the window)
- **Where:** script_tools.rs:755-770 (and 700-732)
- **Since:** before 2253fa78
- **Scenario:** Replace All of two spaces by one, or of three line ends by two, in any script: about half of each run is left. And in the game's `nwscript.nss` (13,869 lines) or a builder's 14,000-line include, Replace All of a name used 1,000 times runs `find` 1,000 times over the whole text (a `Vec<char>` of it and a letter-by-letter compare with `same_letter` each time): seconds with the window frozen.
- **Fix:** collect the matches once on the text as it is (the loop `find` already has), and write the result in one pass over them.
- **Test:** unit tests of `replace_all`: `("    ", "  ", " ")` gives two spaces and 2; `("a\n\n\n\n\n\nb", "\n\n\n", "\n\n")` gives four line ends; a 1 MB text with 5,000 matches inside a time bound.
- **Confidence:** confirmed by reading `script_tools.rs:755-770` and `700-732`: each round calls `find`, which copies the whole text into a `Vec<char>` and collects every non-overlapping match *from the start of the text as it now is*, then takes the first at or after `from`. Traced by hand: four spaces, `"  "` to `" "`: round 1 replaces at 0, leaving three spaces, `from` = 1; round 2 finds matches `[0]` only, none at or after 1, wraps to 0 < `from` and stops: three spaces, "Replaced 1", where one pass gives two. Six line ends with `"\n\n\n"` to `"\n\n"` leave five.

### X4. A panel's number field whose value lies outside its `min`/`max` runs its command as soon as it is drawn

- **Kind:** bug
- **Severity:** medium (a plugin's command edits the module with nobody having touched the field)
- **Where:** plugins.rs:2073; plugin_kinds.rs:672-678; crates/mg-plugin/src/runtime.rs:790-794
- **Since:** after (428a9dbe, API 0.9)
- **Scenario:** a panel returns `{ type = "field", kind = "number", id = "n", min = 1, command = "apply" }`. Opening the panel draws the field, egui makes 0 into 1, and `apply` runs as a job with `ctx.values.n == 1`. A panel whose code gives another out-of-range value after each job runs job after job.
- **Fix:** clamp the given value where the field is read (`form_field`, or refuse it there), and in `field_widget` count a number as set only by the user's act (`drag_stopped`, `lost_focus`, or `changed` with the pointer or keys on it).
- **Test:** `tests/app/plugins.rs`: a panel with such a field and a command that logs; draw two frames; assert no job started. A unit test of `form_field` for a value outside the range.
- **Confidence:** confirmed by reading. `plugins.rs:2068-2075`: `drawn.set = … || (r.changed() && !drawn.editing)`; egui 0.36.2 `drag_value.rs:534-543, 703-704` clamps an existing value to the range and marks the response changed; `plugin_kinds.rs:672-678` then runs the field's command because `v.value != v.set`. `crates/mg-plugin/src/runtime.rs:790-794` takes `value`, `min` and `max` with no check of one against the others (a number field with `min = 1` and no value starts at 0).

### X5. A choice field's options have no limit: every one is copied each frame and drawn while the list is open

- **Kind:** performance (hostile or careless plugin)
- **Severity:** medium
- **Where:** plugins.rs:2124, 2142, 2148; plugin_kinds.rs:601; crates/mg-plugin/src/runtime.rs:833
- **Since:** after (607e4937 panels, 428a9dbe swatches); a form's choice before
- **Scenario:** a panel (docked, so drawn every frame) with a choice of every resource of a module, 20,000 names, or the 100,000 a careless loop returns: 100,000 `String` clones twice a frame while it is only shown, and 100,000 `selectable_value` rows a frame once it is opened: the window stops answering, and the job's modal form does the same for `ctx.ask_form`.
- **Fix:** hold `choices_of` to a count (as `MAX_ELEMENTS`) and each text to `MAX_TEXT`, saying what was cut; in `choice_box` borrow the texts (the options are made only while the box has the keys: `show_with` takes a closure) and keep the panel's `Shown` behind an `Arc`.
- **Test:** `crates/mg-plugin` unit test: a choice of 100,000 options is cut or refused; a `frame_perf` case with a panel holding a choice of the limit.
- **Confidence:** confirmed by reading. `crates/mg-plugin/src/runtime.rs:833-862` (`choices_of`: no count, no text length; `MAX_ELEMENTS` and `MAX_TEXT` in `kinds.rs` cover a panel's elements, list items and texts only), `plugins.rs:2123-2159` (`choice_box` clones every option's text into a `Vec` each frame, open or not, and `color()` looks the chosen one up through them all), `plugin_kinds.rs:601` (the panel's whole `Shown`, options included, is cloned each frame), `widgets.rs:3819-3850` (`Dropdown::show` draws a row an option, none skipped, and looks `current` up through `options` each frame).

### X6. Save Anyway on a Save As leaves the claims' "not asked again" set: the next Save is not asked about other builders' areas

- **Kind:** bug
- **Severity:** medium (a team's project: a save that should have asked writes)
- **Where:** plugin_events.rs:430; lib.rs:3806-3810
- **Since:** after (a17fc726, Save As asked)
- **Scenario:** a project with claims; a plugin warns on File › Save As…, the builder chooses Save Anyway. `claims.cleared` stays true. Later they change an area claimed by someone else and press Save: `claims::hold_save` takes the stale flag and returns false, and the save goes on without the question.
- **Fix:** set `claims.cleared` only when `held.then` is one of the actions that go through `claims::hold_save` (`Save`, `SaveThen`), or have `SaveAt` take the flag.
- **Test:** `tests/app` (plugins or claims): hold a `SaveAt` with a `resource-saving` warning, Save Anyway, then `Action::Save` with another builder's claimed area changed: the claims' question shows.
- **Confidence:** confirmed by reading. `plugin_events.rs:425-431` sets `app.claims.cleared = true` whatever `held.then` is; `lib.rs:3806-3810` (`Action::SaveAt`) calls only `plugin_events::hold_save`, never `claims::hold_save`, which is the one place that takes the flag (`claims.rs:287-288`).

### X7. A panel's buttons and items act on the places its last render named, whatever the module has become since

- **Kind:** bug
- **Severity:** medium (a plugin's command runs on another object than the one the panel shows)
- **Where:** plugin_kinds.rs:648-650, 696-701, 731-734
- **Since:** after (607e4937)
- **Scenario:** a panel lists an area's creatures, each item `command = "retag", on = "tavern.git/Creature List[5]"`. The builder deletes creature 2 in the area view and clicks the item for the goblin: the command runs on what is now index 5, the next creature. Undo takes it back, but nothing said it was another object.
- **Fix:** on a click, when `state.revision` is not the workspace's revision, mark the panel stale and say so (or draw it again and drop the click) instead of running the command; the same for `open`.
- **Test:** `tests/app/plugins.rs`: render a panel with an item on `Creature List[1]`, remove item 0 with a command, click the item before the panel is drawn again: no job starts on index 1.
- **Confidence:** confirmed by reading `plugin_kinds.rs:643-651, 684-703, 731-738`: `on` and `open` are `Selected` values (an object is area, list and index) kept from the render; the click uses them without looking at `state.revision` (known at 441 and 468) against the workspace's. Without `refresh = "edits"` a panel is never drawn again for an edit; with it there is `PANEL_QUIET` plus the render's time.

### X8. Search Results is neither bounded nor drawn by rows in sight: Find In Files for a common word stops the window

- **Kind:** performance
- **Severity:** medium
- **Where:** script_view.rs:1118-1130, 2710-2718
- **Since:** before 2253fa78
- **Scenario:** Find In Files for `GetLocalInt` (or `int`) in a persistent world's 5,000 scripts: tens of thousands of lines (the game's own 4,144 scripts have 213,606 lines). From then on every frame with the Search Results tab in front builds and lays out every row; at 50,000 rows the editor is unusable until another tab of the pane is chosen.
- **Fix:** draw with `ScrollArea::show_rows` (rows are one line high), and cap what Find All keeps (say 10,000, with "and N more" in the log).
- **Test:** `editor_perf`: 50,000 results in `script_tools.search.results`, a frame of the script editor inside the budget.
- **Confidence:** confirmed by reading. `script_view.rs:2702-2719` (every matching line of every script pushed, no limit), `script_nav.rs:136-146` (every reference), `script_view.rs:1118-1130` (each result a `format!` and a `selectable_label` every frame, no `show_rows`, no `is_rect_visible`). Searched `docs/deferred.md` and `docs/log.md` for "search results": nothing.

## Low

### X9. A script's laid-out lines are kept until the module closes, for every script ever opened

- **Kind:** performance (memory; state kept past its time)
- **Severity:** low
- **Where:** lib.rs:603, 3481; script_view.rs:1293
- **Since:** after (42a07a28)
- **Scenario:** a scripter goes through a few hundred scripts in a session (Find References, compile errors), each a few hundred lines: about 2 MB kept for each after its tab is closed, hundreds of MB by the end of the day; a 14,000-line include opened once keeps tens of MB.
- **Fix:** drop `laid_out[key]` when the script's tab is no longer in the dock (where `forget_side` drops the buffer, and in the frame's sweep of closed tabs), as `nui_views` are.
- **Test:** `tests/app` script test: open a script, draw, close its tab, draw: `laid_out` has no entry for it.
- **Confidence:** confirmed by reading: `laid_out` is filled at `script_view.rs:1293` and cleared only at `lib.rs:3481` (`close`); closing a script's tab, its Cancel (`side_doc.rs:277`) and a rename or delete (`references.rs:445, 521`) leave the entry. An entry holds a galley per line, the line numbers' galleys and the whole text's (glyphs and their meshes: on the order of 100 bytes a character).

### X10. Open Script with everything selected compares every selected script with every listed one each frame

- **Kind:** performance
- **Severity:** low
- **Where:** script_view.rs:2455, 2449, 2347
- **Since:** after (1dc581fb: Ctrl+A and the `retain`); the list made each frame before
- **Scenario:** All Resources lists the game's 4,144 scripts plus the module's (8,000 in a large persistent world); Ctrl+A ("selects every one listed"): 12,000 × 6,000 key compares a frame, far past 6.9 ms, until the window is closed.
- **Fix:** make `names` a set for the `retain` (or retain only when the filter or what is shown changed), and build the list through a `HashSet` in `open_script_window`.
- **Test:** `editor_perf`: the Open Script window with 12,000 scripts all selected, a frame inside the budget.
- **Confidence:** confirmed by reading `script_view.rs:2441-2455`: `names` is a `Vec`, `w.selected.retain(|k| names.contains(k))` is selected × listed, every frame the window is open; `listed` also makes a `String` of every script's name each frame. `open_script_window` (2346-2347) is quadratic once too (`scripts.iter().any` for each of the game's scripts).

### X11. Choose Image looks through every picture name the game has, every frame it is open

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view/workflow.rs:285-286
- **Since:** before 2253fa78
- **Scenario:** a module with CEP and a persistent world's haks: on the order of 100,000 picture names. With the name of the wanted picture typed (the usual use: fewer than 24 match), every frame runs `Needle::finds` over all of them.
- **Fix:** keep the 24 names found for (catalog's identity, filter text) and search again only when one changes.
- **Test:** `nui_perf`: a catalog of 100,000 names, a filter matching three, the Properties pane's frame inside the budget.
- **Confidence:** confirmed by reading `nui_view/workflow.rs:284-287` (`assets.catalog.iter().filter(|n| query.finds(n)).take(24)`, per frame while the section is open) and `nui_view/skin.rs:620-639` (the catalog is every JPG, TGA, PNG, BMP and DDS name of the load order and the module). `take(24)` stops early only while the filter matches many.

### X12. The project's plugins folder is walked on the window's thread every three seconds, and the plugin code makes file-system calls every frame

- **Kind:** performance
- **Severity:** low
- **Where:** plugins.rs:132-158, 1373-1381, 2361
- **Since:** after (82817304 the folder watched, af291f40 Run Tests); the grants kept every frame before
- **Scenario:** a project's `plugins/` with a plugin whose `tests/fixtures` holds an unpacked module (5,000 files), or a nested checkout's `.git`: 5,000 to 20,000 `stat`s on the UI thread every three seconds, a dropped frame or several on Windows. Each alone is small; together they are the only disk access the idle window makes per frame.
- **Fix:** stamp only what a plugin's digest reads (`package::files_of`'s rule: no hidden entries, no `tests/`), on the thread that already looks at the module's files; in `next_plugin_event` return before the names when no event is queued and keep the grants when a job ends; read `test_files` when the plugins are loaded.
- **Test:** unit test of `folder_stamp`: a change under `tests/` or `.git` leaves the stamp as it was; a unit test of `next_plugin_event` with an empty queue leaving `settings.plugin_grants` untouched.
- **Confidence:** confirmed by reading. `plugins.rs:132-158` (`folder_stamp`: `read_dir` and `metadata` of up to 20,000 entries, eight levels, `tests/` fixtures and hidden folders included, though neither is in a plugin's digest), called from `project_plugins_changed` (703-709) by `lib.rs:5343-5361` every 3 s; `plugins.rs:1373-1381` (`next_plugin_event`, every frame with a module open and no job: `enabled_plugins()` and, for each project plugin, `grants_name` → `canonicalize`, before it looks whether an event is queued); `plugins.rs:2360-2361` (`mg_plugin::test_files`: a `read_dir` of each plugin's `tests/` and a stat of each entry, every frame the Plugins window is open).

### X13. Decide Later is undone by any change in the plugins folder: the question comes back within seconds

- **Kind:** bug
- **Severity:** low
- **Where:** plugins.rs:656-662, 715
- **Since:** after (82817304)
- **Scenario:** the builder puts the question off. An editor writes a swap file in `plugins/`, or a pull touches a test fixture: three seconds later the modal is back, over whatever they were doing, for plugins that did not change.
- **Fix:** after a re-read, ask only about plugins whose trust changed since the read before (new, or digest other than before); keep those put off out until the project is opened again.
- **Test:** `tests/app/plugins.rs`: Decide Later, touch a file in the folder's `tests/`, `project_plugins_changed()`: `project_asking` stays empty.
- **Confidence:** confirmed by reading `plugins.rs:622-662` (`read_project_plugins` clears `project_asking` and fills it with every project plugin still to ask about) and 703-715 (run whenever the folder's stamp moves, which any file under it does, digested or not). The button says "you are asked when the project is next opened" (2634-2639).

### X14. A DDS picture of a window is held to 16 megapixels but not to a side: 16,384 × 1,024 goes to the GPU

- **Kind:** bug
- **Severity:** low
- **Where:** nui_view/skin.rs:436-446
- **Since:** before 2253fa78
- **Scenario:** a hak a builder downloaded holds such a DDS and a window (or the skin) names it: the Creator's preview, the gallery of Choose Image, or `mg nui render` in CI crashes instead of saying "image exceeds 4096 pixels a side".
- **Fix:** check `t.width` and `t.height` against `MAX_SIDE` for DDS too, before `to_rgba`.
- **Test:** `preview/tests/hostile.rs`: a window naming a 16,384 × 2 DDS is drawn with the issue said, in a debug build.
- **Confidence:** suspected. Read `nui_view/skin.rs:431-453` (TGA, PNG, JPEG and BMP are held to `MAX_SIDE` 4,096 a side; DDS only to width × height ≤ 16,777,216, and `mg_image::MAX_SIDE` is 16,384) and 717-725 (`ctx.load_texture`). egui only `debug_assert`s the side against `max_texture_side` (8,192 in `headless.rs:549`); a texture wider than the device's `max_texture_dimension_2d` is a wgpu validation error, which is fatal by default. What would settle it: a 16,384 × 4 DDS named by a window, on a device whose limit is 8,192 (or `mg nui render` in a debug build, where the `debug_assert` fires).

### X15. A hak's name from the module's list is joined to the hak folder as it is, then written into

- **Kind:** bug
- **Severity:** low (needs a module made for it and a `.hak` file already at the place named)
- **Where:** nui_view/images.rs:210, 225
- **Since:** before 2253fa78
- **Scenario:** a downloaded module lists the hak `../../Documents/other/pack`. Images › Into shows it; Add images to it writes into `Documents/other/pack.hak` and leaves a `.bak` there, outside the hak folder.
- **Fix:** refuse a listed name that is not a plain file name (no separator, no `..`, not absolute) before joining, as `ResKey::path_in` does for resources; compare canonical paths for the game's folder.
- **Test:** unit test of `chosen_hak` with `Some("../x")`: `None`, and the log says why.
- **Confidence:** confirmed by reading `nui_view/images.rs:209-218` (`folder.join(mg_resman::named_file(hak, "hak"))`, `hak` one of `crate::listed_haks(ws)`, the IFO's `Mod_HakList`) and `crates/mg-resman/src/file_name.rs:44-57` (`named_file` only adds the extension: `..`, `/` and an absolute path pass). `write_hak` (241-293) then adds pictures to that file and writes a `.hak.bak` beside it; the "never into the game's folder" check at 225 compares the unresolved path.

### X16. Replace All and Replace are no step of the editor's Undo while the keys are in the Replace window

- **Kind:** bug
- **Severity:** low
- **Where:** script_view.rs:2696-2701, 2686-2693
- **Since:** before 2253fa78
- **Scenario:** Replace All, then Edit › Undo with the window still in front: the module's last command is taken back (an area edit, an earlier script save), not the replacement; and for an editor never clicked in, the editor's own Ctrl+Z has nothing to go back to either.
- **Fix:** give Replace and Replace All the same undo point Convert Indentation makes (`undoer.add_undo(&(range, old_text))`), and have `undo_target` take the Find window's script while that window has the keys.
- **Test:** the UI test above, asserting the script's text is back and the workspace's undo depth unchanged.
- **Confidence:** suspected. Read `script_view.rs:2686-2701` (`buf.text` is set directly; Convert Indentation at 521-523 adds its own undo point, these do not), `script_view.rs:396-403` (`undo_script` is the script whose editor has the keys: none while the Replace window's field has them) and egui's `Undoer::feed_state` (a state is fed only while the editor has the keys; an editor that never had them has no earlier state). What would settle it: a UI test that opens a script, opens Replace from the toolbar without clicking in the text, Replace All, then Edit › Undo.

### X17. `run_plugin` lets go of the running job's question before it knows its own job starts

- **Kind:** hygiene (latent bug)
- **Severity:** low
- **Where:** plugins.rs:1537, 1541-1542
- **Since:** before 2253fa78
- **Scenario:** an event's job starts at the top of a frame in which a click on a panel's button was already under way; the event's handler asks for a folder: its window shows a spinner for good.
- **Fix:** return at the top of `run_plugin` when `self.job.is_some()` (saying so, as `start_job` does), before touching `ask`, `form` and `answers`.
- **Test:** unit test in `plugins.rs`: with a job running and an `Ask` pending, `run_plugin` leaves `plugins.ask` pointing at the same `Ask`.
- **Confidence:** confirmed by reading `plugins.rs:1536-1544`: `plugins.answers` is taken, `plugins.form` cleared and `plugins.ask` replaced, then `start_job` refuses when a job runs (`jobs.rs:219-222`); `!self.busy()` at 1577 is then false, so the new, unused `Ask` stays. A plugin job waiting in `Ask::ask` on the old one is never shown its question again and waits until Cancel. Reaching it needs a plugin command's action in the frame another plugin's job starts (`next_plugin_event` at `lib.rs:1571` starts one before the frame's clicks are run at 1850); the modal keeps later frames out.

### X18. With the keys in a script's editor, each frame still walks the text up to the cursor

- **Kind:** performance
- **Severity:** low
- **Where:** script_view.rs:1246, 1188, 1099-1109; script_tools.rs:479-483
- **Since:** before 2253fa78 (1246 and `word_before`)
- **Scenario:** the cursor near the end of `nwscript.nss` (13,869 lines) or a 30,000-line include: the walk is the length of the file every frame, and with a completion list open a copy of it; ten bookmarks near the end are ten more passes.
- **Fix:** for 1246 take the byte range once (`byte_index` of `a` and `b`, or ask the galley's cursor rows) and look for `\n` in it only when Tab was pressed; `word_before` from the cursor's byte backwards, as `word_at` now does; bookmarks' texts through the line starts `script_layout` already keeps.
- **Test:** `editor_perf`'s `editing_a_huge_script`: an idle frame with the cursor on the last line and a completion list open, inside the budget.
- **Confidence:** confirmed by reading; not measured (no builds allowed). `script_view.rs:1246` (`buf.text.chars().skip(a).take(b - a)`, every frame the editor has the keys and a cursor: decodes every character before the cursor), `script_view.rs:1188` with `script_tools.rs:479-483` (`word_before` copies every character before the cursor into a `Vec<char>` each frame a completion list is open), `script_view.rs:1099, 1109` (`lines().nth(line)` from the top for each bookmark, each frame the Bookmarks tab shows). The log's entry for the large-script fix lists what it measured; these are not in it.

### X19. `convert_indentation_in` indexes `chars[to - 1]` after clamping `to` to the text's length

- **Kind:** hygiene
- **Severity:** low
- **Where:** script_tools.rs:675
- **Since:** after (130aea68)
- **Scenario:** the external editor empties the file in the frame a Convert Indentation of the selection is pending: a panic.
- **Fix:** `if from == to` again after clamping; `chars.get(to.wrapping_sub(1))`.
- **Test:** unit test: `convert_indentation_in("", true, 4, 3, 9)` returns `""`.
- **Confidence:** confirmed by reading `script_tools.rs:668-675`: the early return is for `from == to` as given; clamped to an empty text both are 0 and `to - 1` underflows. `indent_lines` (608) guards the same line with `to > from`. The selection comes from the editor's stored state (`script_view.rs:497-503`), which `reload_external` or `follow` can leave longer than a text they replaced in the same frame.

### X20. Every change of the pictures a window names reads the whole skin again

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view/skin.rs:609, 641-646, 704-716
- **Since:** before 2253fa78
- **Scenario:** each key typed into a picture's name in Properties, or into Find a picture… (each key another 24 names, up to 24 textures of a hak decoded): a full reload a key.
- **Fix:** keep the skin (table, leaves, fonts, its pictures) under the load order's signature alone, and for a changed set of names read only the names that came.
- **Test:** a unit test counting reads through a `ResMan` wrapper: one name added to the window reads one picture.
- **Confidence:** suspected (read, not measured). `nui_view/skin.rs:599-609, 641-780`: the signature is the load order and the set of picture names wanted (the Choose Image list's 24 included); when it changes, `nui_skin.tml` is read and parsed, every skin picture and wanted picture is read from the load order and hashed with `DefaultHasher` to see that it is the one held (713), every font is read and hashed (755), and new pictures are decoded, all on the window's thread. What would settle it: `Assets::prepare` timed in `nui_perf` after one name changes, the game loaded.

### X21. Install Now's question is set after a failed in-place install has cleared it

- **Kind:** hygiene
- **Severity:** low
- **Where:** updates.rs:417-418
- **Since:** after (the update install)
- **Scenario:** Install Now fails to replace the image (the folder is read-only); Try Again from the notice as Later succeeds: the window asks to restart.
- **Fix:** set `ask_restart = true` before `install_image_now`, or let that function's result decide it.
- **Test:** unit test of `got` with a `Staged` image whose target can't be written: `ask_restart` is false after.
- **Confidence:** confirmed by reading `updates.rs:416-419`: `install_image_now` clears `ask_restart` when the AppImage could not be replaced (449), and `got` sets it to true on the next line. Shown only once a later download of the same release (Later, or the automatic one) is ready: the window then asks "Restart Moonglow now?" for a download that was asked for as Later.

### X22. The Bindings page copies the whole window for each bind, and walks it again for each bind in use, every frame

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view/bindings.rs:246; nui_view/design.rs:772, 886
- **Since:** before 2253fa78
- **Scenario:** a data-driven window of 300 controls with 60 binds (every label and `enabled` bound): on the Bindings page each frame makes 61 deep copies of the window's JSON and 60 walks of it allocating a path per value: tens of milliseconds a frame.
- **Fix:** `events::editor` takes `&Value` where its target is a bind (only `element` for a control needs to write), so no copy; gather every bind's uses in one walk a frame (or keep them by the document's revision).
- **Test:** `nui_perf`: the Bindings page of such a window, a frame inside the budget.
- **Confidence:** confirmed by reading; not measured (`nui_perf` measures the Design page of a window of about 100 controls with no binds). `nui_view/bindings.rs:244-248` (`watch`: `doc.clone()` so that `events::editor` can have a `&mut Value` it never changes for a bind), called for every bind by `nui_view/design.rs:773-778, 830` (each bind's header is open by default); `design.rs:772` copies the window once more; `design.rs:885-886` (`bind_uses` walks the whole window with a `format!` for every value in it, for each bind in use).

### X23. Layers writes every control's whole subtree out as JSON text each frame, with nothing typed in its filter

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view/design.rs:418
- **Since:** before 2253fa78 (3f4ddcf5)
- **Scenario:** a window of 500 controls nested eight deep (about 125 KB of JUI): about 1 MB of JSON written a frame, a third of the frame's budget, growing with size times depth; at 100 controls three deep (what `nui_perf` measures) it does not show.
- **Fix:** `if !search.is_empty() && !search.finds(…)`; with a filter typed, match on the node's own name, ID and type and let a parent show when a child matched.
- **Test:** `nui_perf`: a window of 500 controls nested eight deep, an idle frame inside the budget.
- **Confidence:** confirmed by reading `nui_view/design.rs:417-420`: `search.finds(&node.to_string())` makes the text before `Needle::finds` (`crates/mg-core/src/text.rs:90-92`) sees that the filter is empty, and does so for every node, each time for everything under it, before the fold check at 422: the document is serialized once for each level of nesting, every frame the Design page's editor shows.

### X24. Row arrays are drawn whole in the forms: a list of thousands of rows, which New Bind for a Property allows up to 10,000, stops the pane

- **Kind:** performance
- **Severity:** low
- **Where:** nui_view/fields.rs:291, 462; nui_view/bindings.rs:144, 182
- **Since:** before 2253fa78
- **Scenario:** a list with `row_count` 5,000 (a log, an item browser tried at its real size): + New Bind for Property on a cell's label opens a dialog of 5,000 open items, each with a field and a button, and validates the window with it each frame; afterwards selecting the cell draws them again in Properties, and Row data 5,000 headers.
- **Fix:** draw arrays through `ScrollArea::show_rows` or the first few hundred items with "and N more: Binding JSON"; check `compatible` when the draft's value changes, not each frame.
- **Test:** a UI test in `nui_view/design.rs`'s fixture: a list of 5,000 rows, its cell selected, a frame inside a bound.
- **Confidence:** confirmed by reading `nui_view/fields.rs:289-302` (`value`: a `CollapsingHeader` for every item of an array, open by default for plain values), `fields.rs:462-491` (Row data: one for every row), `nui_view/bindings.rs:124-147, 176, 182` (the new bind's draft is `vec![source; count]` for up to 10,000 rows, drawn by `value`, and `compatible` copies the window and the settings and runs `mg_nui::validate` on them every frame the dialog is open), `bindings.rs:225-231` (a bound property's bind drawn whole in Properties).

### X25. The two windows that make a bind hold its name to different rules

- **Kind:** duplication (drifted)
- **Severity:** low
- **Where:** nui_view/bindings.rs:178; nui_view/workflow.rs:64-81
- **Since:** before 2253fa78
- **Scenario:** a name pasted with a tab in it into New Bind for a Property is made; the same name is refused by New Bind and by Rename.
- **Fix:** one function for a new bind's name (`BindDraft::error`'s rules, taking the name and the binds), used by both windows and by Rename.
- **Test:** unit test of that function with `"a\tb"`, `""`, a taken name, `" x "`.
- **Confidence:** confirmed by reading `nui_view/workflow.rs:64-81` (New Bind: empty, `mg_nui::plain_name`, taken) and `nui_view/bindings.rs:177-186` (New Bind for a Property: empty or holding `\0`, taken): the second lets through what `plain_name` refuses (a tab or another control character pasted in, spaces at either end are trimmed there but checked here), which the first says "the event script couldn't name". Rename (`design.rs:839-840`) uses `plain_name` too.

## Not read

- `nui_view.rs`: read lines 1235-1424 and the whole diff since 2253fa78 (`git diff`, changed lines only); the rest (about 2,300 lines) not read.
- `nui_view/interaction.rs`, `layouts.rs`, `draw.rs`, `preview.rs`: not read. `nui_view/shortcuts.rs`: lines 171-206 not read.
- `nui_view/headless.rs`: lines 340-420 and 540-556 only.
- `nui_view/skin.rs`: lines 196-824 only.
- `nui_view/preview/`: `containers.rs` 292-312 and 372-422 only; the other files not read (the casts in them were looked at through `rg`; `preview/tests/hostile.rs` read to see what is defended).
- `nui_view/structure.rs`: lines 300-365 (tests) not read.
- `script_wizard.rs`, `manual.rs`, `icons.rs`: not read (nor their diffs).
- Read whole: `plugins.rs`, `plugin_kinds.rs`, `plugin_events.rs`, `script_view.rs`, `script_tools.rs`, `script_layout.rs` (to its tests), `script_nav.rs`, `updates.rs`, `side_doc.rs`, `nui_view/events.rs`, `workflow.rs`, `images.rs`, `design.rs`, `fields.rs`, `bindings.rs`. The NUI Creator's drawing side (`preview/`, `layouts.rs`, `draw.rs`, most of `skin.rs` and `headless.rs`) is the largest gap: the next review should start there.
- `script_view.rs` 2281-2334 (`mod perf`, a test) and the test modules of the files read: not read.
- No finding was measured or run: builds were out of bounds. Costs given are estimates from the code.

## Looked at and found sound

- A project's plugin runs from the copy read when it was found (`Frozen`): what is allowed by digest is what runs, whatever the folder holds later; forms and wizard pages refuse a field id used twice, and a page asked again keeps what was typed (`mg-plugin` `form_fields`, `kinds.rs:330-340`).
- A plugin's job against a module that changes: the job's window is modal, keys are off while it runs (`lib.rs:1577`), outside changes wait (`outside.rs:315`), a panel that follows edits is drawn again once they stop, its code never in a frame; haks a command wrote are taken back when its edits fail or are undone.
- `script_layout.rs`: the prefix/suffix arithmetic, the splice of lines and the shift of the kept lines' starts hold for insertions, deletions, a change of the error's line alone and a first layout; line starts are always after a `\n` of the common prefix, so no slice lands inside a character; `Galley::concat` shares rows and copies no glyphs.
- `nui_view/structure.rs` `relocate` and `design.rs`'s drops: no move into itself or a descendant, the target's index is corrected for the removal, a list cell keeps its width and flag, a stale drag (`doc.pointer(path) != node`) is dropped, the trees stop at 64 levels (and serde_json reads no more than 128); the Creator's keys are off while any field has the keyboard.
- `updates.rs`: a second check joins the one under way; a download not ready at close installs nothing and is cleaned at the next start; each failure is said with the release page and Try Again.
