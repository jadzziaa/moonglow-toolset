---
type: Report
title: 'Code review of November 2026: the window'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the window (mg-ui: lib, widgets, commands, keys, tabs, tree, settings, options, recovery, jobs): 28 in all (5 high, 14 medium, 9 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-ui, window, recovery, settings]
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

# Code review of November 2026: the window

One part of [the review of 9 November 2026](overview.md): the window (mg-ui: lib, widgets, commands, keys, tabs, tree, settings, options, recovery, jobs). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it. Paths are under `crates/mg-ui/src/` unless they say otherwise.
## High

### S1. The module's Undo and Redo throw away every script editor's unsaved text

- **Kind:** bug
- **Severity:** high (data loss)
- **Where:** lib.rs:3850 (`self.scripts.clear()`), against script_view.rs:561 and 575-583
- **Since:** before 2253fa78 (cd074a54)
- **Scenario:** type in a script's editor without its Save (tab shows `*`); click in an area's view, move a placeable, Ctrl+Z over the area (or the toolbar's Undo with the keys outside the editor). `ws.undo()` succeeds, every `ScriptBuffer` is dropped, and the editor makes a new one from the module's stored bytes: what was typed is gone, with no word. Redo the same.
- **Fix:** don't clear; the editor already follows the module where a buffer isn't dirty (`buf.source != source`). At most drop the buffers that aren't dirty.
- **Test:** app test in `tests/app/scripts.rs`: type in a script, apply and undo a module command, assert `script_text` still has the typed text and the buffer is dirty.
- **Confidence:** confirmed by reading (`lib.rs` Undo/Redo arm; `script_view.rs:548-585`, where the editor re-reads a missing buffer from the module and itself keeps a dirty one when the bytes change underneath; `ScriptBuffer::is_dirty`; I found no app test that types in a script and then undoes a module step)
- **Read again by the reviewer:** holds: `self.scripts.clear()` after a successful `ws.undo()` or `ws.redo()` (lib.rs:3850).

### S2. Recover, New Module's Create and "Open Directory as a Module" replace the open module without asking, and delete its recovery copy

- **Kind:** bug
- **Severity:** high (data loss)
- **Where:** recovery.rs:464-467 and 383-417 (`recover` → `use_module`); lib.rs:3593-3609 (`NewModule`, `OpenModuleAsIs` not among `discards`), 3654, 3690, 3207-3208, 3449; wizards.rs:90-106; module_folder.rs:36-56
- **Since:** before 2253fa78
- **Scenario:** (a) after a crash the Recover window lists two copies; Recover the first (opened, marked modified), then Recover the second: the first is closed unasked and `close()` removes its copy from disk (same stem), so that work is gone for good. Same with any module opened and edited while the window still shows. (b) File › New Module with nothing unsaved (no question), edit the open module while the small window waits, then Create: the edits and the recovery copy go. (c) the same between the unsaved question and the answer to "Open Directory as a Module".
- **Fix:** make the check one place: `use_module` (or `run` for every action that ends in it, and `recover`) asks through `confirm_discard` when `has_unsaved_work()`.
- **Test:** app tests: recover a copy with a modified module open → the question shows and the module is still there; New Module's Create after an edit → asked.
- **Confidence:** confirmed by reading (`Moonglow::run`'s `discards` list; `use_module` → `close` → `forget_recovery`; `widgets::modal`/`dialog` are plain `egui::Window`s, the rest of the window stays live, as `tests/app/windows.rs:2303` relies on)
- **Read again by the reviewer:** holds for New Module's Create and Open Directory as a Module: neither is among `discards` (lib.rs:3593-3609); Recover's path was not traced again.

### S3. The Variables and String Edit windows keep their object by its place in a list, and write to whatever is there at OK

- **Kind:** bug
- **Severity:** high (another object's data overwritten, silently)
- **Where:** widgets.rs:1054-1255 (drawn every frame from `app.loc_edit`/`app.var_edit`), 35-46, 1758-1766; tree.rs:1944; blueprint/mod.rs:1121-1133, 1665; dialog_view.rs:1839; lib.rs:3444-3524
- **Since:** before 2253fa78 (the tree's Variables… route after, dcdc0b81)
- **Scenario:** module tree › an area's placeable #5 › Variables…; with the window open, Delete placeable #2 in the tree (or Undo, or delete in the area view); OK. `SetField` goes to `Placeable List[5]`, now another placeable, whose variables are replaced by the first one's whole list. Likewise String Edit on a placed object's name or a conversation line after a line is added or removed. And: Module Properties › Variables…, open another module, OK → the first module's variables written into the second.
- **Fix:** keep with each window what `follow` keeps for a Properties window (retarget on every command, close when the object goes), and drop `loc_edit`, `var_edit`, `picker`, `picked`, `var_pages` in `close()` and when the tab they were opened from closes.
- **Test:** app test: open Variables on object 5, delete object 2 through `apply`, press OK: object 4 (the same one) has the change and no other object changed.
- **Confidence:** confirmed by reading (`FieldTarget` is key + `GffPath` by index; `blueprint::follow::retarget` (blueprint/follow.rs:394-464) moves tabs, pages, window ids and `properties_changed` but not `var_edit`/`loc_edit`; `close()` clears neither, nor `picker`/`picked`; nothing closes them with the tab they were opened from)
- **Read again by the reviewer:** holds: a `FieldTarget` is a key and a path by index, written to with `SetField` at OK (widgets.rs:30-46), and `blueprint/follow.rs` names neither `var_edit` nor `loc_edit`.

### S4. A recovery copy is made only when the workspace changes: text typed in script editors alone is never kept

- **Kind:** bug
- **Severity:** high (the work a crash loses; the module's header promises "unsaved script text included")
- **Where:** recovery.rs:330-352
- **Since:** before 2253fa78
- **Scenario:** open a module, write scripts for an hour without Save: `is_modified()` is false, no copy is ever written. Or one edit, a copy at five minutes, then an hour of typing: the revision is the same, no further copy. A crash that is no frame panic (driver abort, OOM kill, power) loses it all.
- **Fix:** tick on `has_unsaved_work()`, and tell copies apart by the revision plus a hash (or a typed-since flag) of the dirty buffers.
- **Test:** unit test beside `a_copy_is_made_when_one_is_due`: only a dirty `ScriptBuffer`, tick past the interval, the copy holds the typed text; type more, tick again, the copy is newer.
- **Confidence:** confirmed by reading (`autosave_tick` returns unless `ws.is_modified()` and skips while `saved == revision`; script text enters the workspace only at a save, `store_script_text`'s callers; `has_unsaved_work` does count dirty scripts, so only the frame-panic path keeps them)
- **Read again by the reviewer:** holds: `autosave_tick` returns unless `ws.is_modified()` (recovery.rs:334-336).

### S5. Only the command keys are stopped while a job runs: the menus' own keys and files dropped on the window still run, and a job's result is then applied to another module

- **Kind:** bug
- **Severity:** high (one module's compiled scripts or edits written into another)
- **Where:** lib.rs:1577-1579, 3574-3630; commands.rs:1188-1392; transfer.rs:316-318, 953-966; jobs.rs:274-296; e.g. lib.rs:4815-4837 (Compile's `finish` applies its edits)
- **Since:** before 2253fa78 (the drop route after, 0b7c8c57)
- **Scenario:** Compile All Scripts on `castle_v1.mod`; while its window shows, drop `castle_v2.mod` on Moonglow (or F10, Down, Enter on Open Module…). v2 opens (the log says its haks couldn't be layered, the game data being in use); the job ends and "Compile scripts" sets v1's `.ncs` on v2's scripts of the same names. Also: the window's × with "Ask before the window is closed" off quits under a running job (a project command's child process left running).
- **Fix:** one guard in `run` (hold or refuse every action while `busy()`, at least those in `discards`, `CloseWindow`, `Undo`/`Redo`, `Apply*`), skip `menu_bar`'s keys and `drop_ui` while busy, and have a job remember the workspace it began on (`first_revision`'s range) and hand nothing to another.
- **Test:** `jobs.rs`: start a job in the background, push `Action::OpenModule(other)`, run frames, end the job: the other module is untouched (or the action waited).
- **Confidence:** confirmed by reading for dropped files (`transfer::drop_ui` → `take_dropped` → `Action::OpenModule`, drawn every frame; `run_actions`/`run` never look at `busy()`; `finish_job` hands over to whatever `app.ws` is); suspected for the menus (`commands::menu_bar` reads F10, Alt, Alt+letter, the arrows and Enter itself, outside `shortcuts`; egui's `Modal` takes only Escape and pointer input). An app test pressing F10, Down, Enter under a background job would settle it; `jobs.rs`'s test tries Ctrl+Z alone.
- **Read again by the reviewer:** holds for dropped files: `drop_ui` hands them to `take_dropped` with no look at `busy()` (transfer.rs:953-966), and `busy()` is asked only for the shortcuts and three other places (lib.rs:1577, :1773, :5336, :5864). The menus' own keys were not traced again.

## Medium

### S6. Save As, and a new module's first save, leave the old place's recovery copy behind

- **Kind:** bug
- **Severity:** medium (stale work offered as unsaved at the next start; Recover opens it over the old module)
- **Where:** lib.rs:4380-4389; recovery.rs:195-213, 312-318
- **Since:** before 2253fa78
- **Scenario:** new module, work ten minutes (copy `untitled-….mod`), Save (asks a place): the copy removed is the new path's, which never existed. Next start: "Moonglow closed without saving these modules: untitled, never saved". Same from `a.mod` saved as `b.mod`: `a`'s copy stays and Recover would put the pre-save state over `a.mod`.
- **Fix:** take `recovery_archive()` before the save and remove that copy (and the new one) after it succeeds.
- **Test:** unit test: autosave a never-saved module, `save(Some(loc))`, `find(dir)` is empty.
- **Confidence:** confirmed by reading (`Workspace::save_as` makes the new place the location before `forget_recovery()` works out the copy's name from the location; `file_stem` hashes the path)

### S7. A recovered module still counts the recovery copy as its file

- **Kind:** bug
- **Severity:** medium
- **Where:** recovery.rs:384-416
- **Since:** before 2253fa78
- **Scenario:** a module folder; rename or delete a blueprint, crash, Recover, Save: "Save failed: the folder holds resources that are not the module's (old.utc)"; the recovered work can only be saved elsewhere. Archive modules: after Recover the outside watch looks at `recovery/<name>-<hash>.mod`.
- **Fix:** as the project case does: open the module where it lives (`Module::open(note.module)`), then `adopt_resources` the copy's.
- **Test:** unit test: folder module, delete a resource, `write_recovery`, `recover`, `save(None)` succeeds and the file is gone; `outside_scan` names the module's own path.
- **Confidence:** confirmed by reading for the folder case (`recover` sets `location` only; `Module::write_folder` with no `folder` record refuses a folder holding any resource file the module lacks, mg-module/src/lib.rs:718-741); suspected for the watch (`outside_scan` reads `self.archive`, mg-module lib.rs:846, still the copy's path, which the autosave rewrites: `take_found` then sees the module's own autosave as a change outside, and a script being typed as a conflict). A test of recover, edit, autosave, look would settle it.

### S8. Conversation backups are written by a resref joined to a folder

- **Kind:** bug
- **Severity:** medium (path traversal out of the backup folder; a `.bak` of GFF bytes, a few folders up at most)
- **Where:** lib.rs:5727-5734 (`dir.join(format!("{}.bak", key.resref))`)
- **Since:** before 2253fa78
- **Scenario:** a downloaded module with a conversation named `../../../x` (or `..\..\..\x`); the builder opens it, changes anything, and five minutes later `<temp>/moonglow-backups/<module>/../../../x.bak` is written.
- **Fix:** `ResKey::path_in(&dir)` (then `.bak`), the error logged; and write through a temporary file.
- **Test:** unit test: `backup_conversations` with such a key open writes nothing outside the folder.
- **Confidence:** confirmed by reading (`ResRef::from_bytes` takes any bytes but NUL; `ResKey::path_in` is the guard and is not used here)

### S9. Paste Set goes to whichever Variables list is drawn first

- **Kind:** bug
- **Severity:** medium (the wrong object gets the variables, or nothing happens)
- **Where:** widgets.rs:1597-1615, 2825-2836
- **Since:** after (34b192a3, the Variables page)
- **Scenario:** two Properties windows on their Variables pages: Copy Set in A, Paste Set in B. If A is drawn first the set merges into A (no change) and B gets nothing; with other variables on the clipboard A's object is changed by a button pressed in B's window.
- **Fix:** `ui.id().with("var-set-paste")` (the page's `push_id` makes it its own).
- **Test:** app test with two Variables pages open: Paste Set in the second changes the second's object only.
- **Confidence:** confirmed by reading (`paste_id = egui::Id::new("var-set-paste")` is one id for the Variables window and every Properties window's Variables page; `asked_paste` gives the text to the first caller with that id and removes it; `color_button` uses `popup_id.with("asked")`)

### S10. Pictures and palettes read by name outlive the module and the game data

- **Kind:** bug
- **Severity:** medium (another module's or another install's icons and portraits shown; one found missing stays missing)
- **Where:** lib.rs:3444-3524, 3207-3258, 5780-5788 against 5112-5114
- **Since:** before 2253fa78
- **Scenario:** the user folder not set: open a module, look at a creature (portrait and icons from its haks cached as none); set the folder in Options › Folders, OK: the game data and the module are read again, the pictures still blank until a restart. Or module A's hak art under a name module B's haks (or none) have another picture for.
- **Fix:** empty `pictures` and `palettes` (and `model_views`) where the load order changes: in `close` and in the Options reload, or key the cache by the game data's generation.
- **Test:** unit test: a picture asked for with no hak, `close`, open a module whose hak has it: found.
- **Confidence:** confirmed by reading (`images.rs:305-384` keeps `Option<Picture>` by lowercase name, `None` too; `self.pictures`/`self.palettes` are emptied only in `game_data_changed`, which `close`, `use_module` and `apply_options`' reload don't call)

### S11. Exit with a never-saved hak open: "Save" saves the module and then nothing happens

- **Kind:** bug
- **Severity:** medium (asked again and again; the only way out discards the hak, which the question never names)
- **Where:** lib.rs:3664-3681, 3377-3382; side_doc.rs:51-54, 329-337; wizards.rs:247-267
- **Since:** before 2253fa78
- **Scenario:** Tools › Haks › Build Hak from Folder… (a new hak, unsaved), then Exit: "Save changes to castle.mod?" → Save: the module is saved, the hak not, `Quit` is dropped without a word. Exit again: the same question. With no module open the question is "Save changes to the new module?" and Save does nothing at all.
- **Fix:** after the save, save each side document left (asking where), or say what is still unsaved and name it in the question.
- **Test:** app test: a new hak with a file added, `Action::Quit`, answer Save: either the hak's Save As is asked or the log says what is left; `quit_requested` or a message, never neither.
- **Confidence:** confirmed by reading (`SaveThen` runs `next` only when `!has_unsaved_work()`; `save_side_docs` skips a hak with no path, `HakDoc::saved_with_module`; the question's text is the module's name, or "the new module" with none open)
- **Read again by the reviewer:** holds for the mechanism (drawn in the sample): after `self.save(None)` the next action runs only `if !self.has_unsaved_work()` (lib.rs:3677-3680).

### S12. A Variables page's list as edited is kept past its window, and held back while anything has the keyboard

- **Kind:** bug
- **Severity:** medium
- **Where:** widgets.rs:1693-1754
- **Since:** after (34b192a3)
- **Scenario:** Variables page: Add (a row with no name: "has no name", not set), Cancel the window. Open the object's Properties again: the row is back; name it and it is set, after a Cancel. Or an Int given `abc`, window closed: shown again next time in place of the stored value.
- **Fix:** remove the entry when its window closes (with `blueprint_pages`, in `follow` and at Cancel) and in `close()`; hold back only while a field of this page has the keyboard.
- **Test:** app test: add a row, Cancel, reopen: the list is the stored one.
- **Confidence:** confirmed by reading for the kept state (`var_pages` entries are only ever taken and put back in `variables_page`: nothing removes one at Cancel, OK, ×, or `close()`; one whose `read` still equals the list is taken up again, edits and all); suspected for the save (`typing` is `focused().is_some()` for any widget in the window, so a change made with the keys on a dropdown waits; Ctrl+S then saves without it and `has_unsaved_work` knows nothing of it). An app test would settle it.

### S13. Options' OK writes back every setting as it was when the tab opened

- **Kind:** bug
- **Severity:** medium (settings changed elsewhere meanwhile revert)
- **Where:** options.rs:341-463, 531-546
- **Since:** before 2253fa78
- **Scenario:** Options left open in a pane; an Export's "Always do this" sets `export_existing`, the area's Shadows menu sets `shadow_quality`, To Scratch sets `scratch_dir`, the update notice sets `update_skipped`; later OK in Options for something else puts all four back.
- **Fix:** keep the settings the draft was made from and write only the fields that differ from them.
- **Test:** unit test in `options.rs`: draft from `s`, change `s.files_there`, change one draft field, `apply(&s)` keeps `files_there`.
- **Confidence:** confirmed by reading (`OptionsDraft::apply` takes all but a few fields from the draft; only the graphics failures are merged, "those recorded meanwhile kept"; the tab is a tab like any other and is opened again with a module's tabs, `open_tabs.rs:1200-1205`, so a draft can be hours old)

### S14. One name or value this version can't read loses every setting

- **Kind:** bug
- **Severity:** medium
- **Where:** settings.rs:54-124 (`ExistingArchive`, `ModuleFolders`, `TalkReuse`, `FilesThere`, `MissingFeats`, `ShadowQuality`), 353-355
- **Since:** after for the newer enums (2a0125c0, f0dec956); the reading before
- **Scenario:** a later version adds `ShadowQuality::Ultra`; the builder runs an older build once (to check a regression): RON fails on the name, all settings are defaults (recent modules, keys, folders, plugin grants, allowed project commands) and are written back so.
- **Fix:** give the plain enums `GraphicsBackend`'s tolerant reading (one helper), and where the settings can't be read keep the file aside and say so rather than overwrite it.
- **Test:** `settings.rs`: `ron::from_str("(shadow_quality:Ultra,light_theme:true)")` reads with `light_theme` kept.
- **Confidence:** confirmed by reading (`apps/moonglow/src/main.rs:553-554`: `eframe::get_value(..).unwrap_or_default()`, saved over at the next autosave, main.rs:177-179; of the enums only `GraphicsBackend` reads an unknown name)

### S15. Delete and F2 in the module tree act on the row under the pointer, not the row the keys marked

- **Kind:** bug
- **Severity:** medium (a placed object other than the marked one deleted, with no question)
- **Where:** tree.rs:1843-1862
- **Since:** before 2253fa78
- **Scenario:** click an object's row (the pointer stays on it), Down three times (the fourth row is marked), Delete: the first row's object is deleted at once. For a resource the question names the first row while the fourth is marked.
- **Fix:** the keys' row first when the cursor moved last by the keys (`tree_cursor_moved` since the pointer last moved); the pointer's only after the pointer moved.
- **Test:** app test in `tests/app/tree.rs`: click row A, ArrowDown, Delete with the pointer still on A → B is asked about or deleted.
- **Confidence:** confirmed by reading (`under = pointed.or(pointed_object).or(tree_cursor)`). `docs/deferred.md:662` describes this order as how it is; reported for what it does in the common flow, the verifier to judge.

### S16. A key given to a window command fires in a text field unless it is a letter or a digit; on macOS Option + letter types and fires

- **Kind:** bug
- **Severity:** medium on macOS, low elsewhere
- **Where:** keys.rs:703-717, 431; options.rs:1488-1518; commands.rs:1196-1223
- **Since:** before 2253fa78 (Exit's Alt+X after, 905ed0b5)
- **Scenario:** give Close Tab the Delete key: Delete in any text field closes the tab and the field never sees it. On a Mac with a Polish layout, typing `ź` in a description asks to exit (or exits, with nothing unsaved).
- **Fix:** while a text field has the keyboard leave it every key without Ctrl/Cmd (and on macOS every Alt combination that came with a `Text` event); have the recorder warn of the editing keys.
- **Test:** `keys.rs`: `consume_outside_text` with `typing` leaves Space, Delete and Alt+X (mac) to the field.
- **Confidence:** confirmed by reading for the rule (`consume_outside_text::types` is "a key whose name is one character, without Ctrl or Alt": Space, Backspace, Delete, the arrows, Enter, `Minus`, `Comma`… are not, egui's `Key::name`; Options › Keyboard's recorder takes any key but Escape, and `conflicts` knows nothing of the editing keys). Suspected for macOS (Option is `alt`; egui falls back to the physical key, so Option+X, Polish Pro's `ź`, is Exit's Alt+X, and Option+E, `ę`, is Alt+E: `menu_bar`'s mnemonic is read whatever has the keyboard and opens Edit). A Mac would settle it.

### S18. Every action is formatted whole for the debug log, whether or not the log is on

- **Kind:** performance
- **Severity:** medium
- **Where:** lib.rs:3632-3633; transfer.rs:64-92, 1318; trace.rs:185-193
- **Since:** before 2253fa78
- **Scenario:** File › Import Folder… of another module's unpacked sources, or an ERF of 100 MB: Import's click formats every byte as `12, 34, …`, some 400 MB of text and a second or more on the UI thread (an allocation failure aborts with no recovery copy). `NoteCompiled` does the same with each compiled script.
- **Fix:** `if trace::on()`, and a short form of an action (its name and counts) in place of `{:?}`.
- **Test:** unit test: `Action::Import` with a 50 MB resource runs in milliseconds with the log off; the line written is short.
- **Confidence:** confirmed by reading (`trace::note(format!("action {action:?}"))`: the argument is built before `note` looks at `on()`, and `write` then cuts it to 1,200 characters; `Action` derives `Debug` through `ImportDraft` → `Incoming { resources: Vec<(ResKey, Vec<u8>)> }` and `Command` → `Edit::SetResource { data }`)

### S19. With text in the module tree's Filter, every placed object of every area is searched two to four times a frame

- **Kind:** performance
- **Severity:** medium
- **Where:** tree.rs:1162-1190, 1236-1242, 386-399, 1652-1661
- **Since:** before 2253fa78 (the read loop after, dcdc0b81)
- **Scenario:** a persistent world of 1,500 areas with 300 objects each (450,000 rows) and 20,000 resources: one word in the Filter costs tens of milliseconds every frame for as long as it is there (an area view animating keeps the frames coming), and while the areas are first read some 1,500² ResRef comparisons a frame on top.
- **Fix:** keep what the filter finds (resources matched, areas that hold a match, each kind's rows) by the filter's text, the groups' `shape` and each area's `Read`; make `read` a set.
- **Test:** a `frame_perf.rs` view: a module of 500 areas × 300 objects, a filter typed, inside the budget.
- **Confidence:** confirmed by reading (`holds` scans an area's rows for each area in `found`, again for `by_filter` before the row's `is_rect_visible`, and `contents_ui` twice more for an area opened out; `own` allocates two strings a resource; the areas still to read are gathered with `read.contains` inside the loop, quadratic until all are read). `frame_perf.rs` measures no tree with a filter typed.

### S20. A described variable's values from a table are copied every frame and drawn whole, each row looked up in the whole list; Select Resource filters its whole list every frame

- **Kind:** performance
- **Severity:** medium for the list, low for the picker
- **Where:** widgets.rs:651-694, 3819-3850; 1273-1275
- **Since:** after for the list (e727a3a7); the picker before
- **Scenario:** `variables.toml` gives a variable `rows` of `placeables.2da` or `appearance.2da` (CEP: 10,000 to 25,000 rows): every frame the Variables list shows costs that many clones a row, and the list open is n widgets and n²/2 comparisons a frame (seconds at 20,000). The picker over sounds or models with CEP's haks (20,000 to 60,000 names) allocates a string each a frame.
- **Fix:** the list through `row_choice` over choices kept in `VarHelp` (value → note in a map); the picker's `shown` kept by filter text.
- **Test:** as `a_long_list_lays_out_only_the_rows_in_sight`, for `value_choice` with 20,000 values.
- **Confidence:** confirmed by reading (`value_choice` builds `values: Vec<String>` each frame for each such row, and `Dropdown::show` draws every option with `text(o)` = `c.list.iter().find(..)`; `RowChoice`, which lays out the rows in sight, is not used; the picker rebuilds `shown` with a `to_string` a resref each frame)

## Low

### S17. The tree's names are not read again when the talk table is

- **Kind:** bug
- **Severity:** low
- **Where:** tree.rs:141-159, 183-202; lib.rs:3282-3326, 5104-5123
- **Since:** before 2253fa78
- **Scenario:** areas and blueprints named by StrRef, "List areas and blueprints by name" on; change a line in the talk table editor and save it: the tree and the tab titles show the old names until the module is opened again.
- **Fix:** `self.area_names = Default::default(); self.area_contents.clear()` where the custom talk table or the game data changes.
- **Test:** app test: an area named by a custom StrRef, edit the line, save the table, the tree's row has the new text.
- **Confidence:** confirmed by reading (`AreaNames::read`/`names` and `area_contents` are kept by the resource's bytes alone; `area_names` is reset in `use_module`, the Options' language change and `outside.rs:607`, not in `load_custom_tlk`, `game_data_changed` or `sync_haks`)

### S21. Options › Folders reads nwn.ini seven times a frame

- **Kind:** performance
- **Severity:** low
- **Where:** options.rs:627-655
- **Since:** after (f4b9669e)
- **Scenario:** the user folder in a OneDrive-synced Documents (Windows' usual place) or on a network share: seven opens and a stat every frame the page shows, the page Options opens on.
- **Fix:** keep the aliases in `ui.data` by the folder typed, as `why_not` is kept.
- **Test:** none below a counting test of `alias`; a `frame_perf` view of Options › Folders.
- **Confidence:** confirmed by reading (`gi.user_folder(sub)` for each of `ALIASED`; `mg_resman::install::alias` reads and parses `nwn.ini` at each call; the game folder's check beside it is kept "once per folder" for this reason; plus `user.is_dir()`)

### S22. Three log messages have a run of eighteen spaces in them

- **Kind:** bug
- **Severity:** low
- **Where:** lib.rs:4340, 4862, 4870
- **Since:** after (f0dec956, 04144768)
- **Scenario:** "The module was not saved: its talk table, which has lines typed for its fields,                  could not be (see above)"; the two "A game is running…" warnings likewise.
- **Fix:** put the `\` and the line break back.
- **Test:** a unit test over `Log` in the tests that reach these messages (the talk table's failed save, `other_game_allows`): no logged line has two spaces together.
- **Confidence:** confirmed by reading (string literals joined without their `\` line continuation)

### S23. Files written in place where a crash leaves half of one

- **Kind:** hygiene
- **Severity:** low (the recovery note: medium, a copy with a cut note is never offered)
- **Where:** recovery.rs:133-135 (the note: `find` drops a copy whose note doesn't parse, and an archive whose note was never written is neither offered nor ever removed); saved.rs:53 and 79 (`Shelf::write`, `restore`: prefabs, variable, parameter and area sets, lighting); lib.rs:5734 (a conversation's backup, over the last good one); lib.rs:4566 (`pack_target`, the project's packed module)
- **Since:** before 2253fa78
- **Scenario:** power lost between the archive's rename and the note's write: the unsaved work is on disk and Recover never lists it.
- **Fix:** one `write_whole(path, bytes)` (temporary file, rename) for all four; write the note first, or have `find` list archives without one.
- **Test:** `recovery.rs`: an archive with an empty `.json` beside it is still found (named from the file).
- **Confidence:** confirmed by reading

### S24. The same few lines in several places

- **Kind:** duplication
- **Severity:** low
- **Where:** (a) "the middle's pane": `palette = find_tab(Palette)…; iter_leaves().find(main && != palette)` at lib.rs:2328-2332, 2488-2490, 2612-2625, 3956-3966, 4014-4023 and open_tabs.rs:1244-1248, 1321-1325; (b) a place as text: written at lib.rs:2037, 2059-2063 and open_tabs.rs:537-544, read at lib.rs:1997-2019 and open_tabs.rs:546-564; (c) the module's hak layers taken out: lib.rs:3511-3520, 5252-5258, 5272-5281; (d) "The game started for the last test is still running…": lib.rs:4883-4888, 4904-4909, 4994-4999; (e) a command's id and name in both `commands::Id::text` and `keys::Cmd::about` (a test holds the ids together, not the names)
- **Since:** before 2253fa78
- **Scenario:** n/a
- **Fix:** `Moonglow::middle_pane()`, `Place::text`/`Place::parse` (open_tabs' as the one), `remove_hak_layers(&mut ResMan)`, one `fn game_still_running(&mut self) -> bool`; `Cmd::name` from `Id::name` for the general commands.
- **Test:** the existing ones.
- **Confidence:** confirmed by reading; no drift found yet

### S25. Doc comments left on the wrong item

- **Kind:** hygiene
- **Severity:** low
- **Where:** lib.rs:794-803 (the compile memory's on `text_typed`), 1925-1929 (`palette_split`'s on `place_of`), 5241-5245 (`sync_haks`' on `release_haks`), 5489-5494 (`verify`'s on `round_trip`), 5791-5793 (`script_text`'s on `compiled_scripts`); settings.rs:73-79 and 117-118 (`ShadowQuality`'s split around three enums put in the middle of it); widgets.rs:4185-4191 (`picture_tile`'s on `letterboxed`); commands.rs:1652-1657; tabs.rs:157-161, 221-224; options.rs:40-43, 1142-1145
- **Since:** mixed (settings.rs after)
- **Scenario:** rustdoc and a reader get the wrong text for a dozen items.
- **Fix:** move each to its item.
- **Test:** none.
- **Confidence:** confirmed by reading

### S26. Smaller state kept past its time

- **Kind:** bug
- **Severity:** low
- **Where:** (a) tree.rs / lib.rs:939: `tree_selected` is never pruned when its resources are deleted or renamed, and `plugins.rs:1209` hands it to a plugin's command as the selection; (b) widgets.rs:3105-3112: an "Always do this" box's tick is kept in egui's memory after its question is answered, so the next time it is asked (Options set back to Ask) it comes ticked and one answer becomes the setting; (c) lib.rs:1053: `dialog_backed_up` keeps every backed-up conversation's bytes for the session, across modules; (d) tree.rs:149, 168, 193, 274 and script_view.rs:560: caches told apart by a buffer's address and length, which a resource replaced twice at the same size can show again (suspected; a counter of replacements, `Workspace::rewrites`-like, would be exact)
- **Since:** before 2253fa78 ((b) after, 71dcf9e1)
- **Scenario:** as said with each.
- **Fix:** retain `tree_selected` to the module's keys when the groups are remade; remove the box's temp value when the question closes; clear `dialog_backed_up` in `close()`.
- **Test:** app test for (a): delete two chosen resources, the selection is empty.
- **Confidence:** confirmed by reading, except the last (suspected)

### S27. Smaller wrong results in the shared windows

- **Kind:** bug
- **Severity:** low
- **Where:** (a) widgets.rs:1078-1115: String Edit's Move to Talk Table adds the line at the click, and Cancel leaves it in the table; (b) widgets.rs:160-181: `merge_variables` goes by name alone: on objects with an int and a string of one name (the game keeps both), a change to one replaces both on the others edited with it; (c) trace.rs:157-160: `take(1200)` counts characters and `len() == 1200` bytes, so the `…` is missing after cut text with letters past ASCII and added to text of exactly 1,200; (d) text.rs:463: `with_english` sets the editing language's text
- **Since:** before 2253fa78
- **Scenario:** as said with each.
- **Fix:** (a) add the line at OK; (b) key by name and `Type`; (c) compare character counts; (d) rename to `with_edited_text`.
- **Test:** (b) a unit test beside `a_variable_edit_merges_into_the_others_edited_with_it` with two types of one name.
- **Confidence:** confirmed by reading

### S28. A filtered list's Enter with nothing found closes it as if a row were chosen

- **Kind:** bug
- **Severity:** low
- **Where:** widgets.rs:3990-4003, 4030-4033
- **Since:** uncommitted (the other session's `Dropdown::filtered` hunk)
- **Scenario:** Module Properties › Add hak…, type a name no hak has, Enter: the list closes and nothing is added, with no word.
- **Fix:** press only where `keys.rows > 0` once the rows are drawn, else leave the list open on "Nothing matches the filter".
- **Test:** the hunk's own app test: Enter with no match leaves the list open.
- **Confidence:** confirmed by reading (`None if entered => keys.pressed = Some(keys.cursor.unwrap_or(0))` with `keys.rows == 0`, then `pressed.is_some()` closes the list)

## Not read

the `#[cfg(test)]` modules (tests are not under review): tree.rs:2219-2450, options.rs:1805-2088, text.rs:476-793, and those at the foot of the other files beyond a look at what they hold. Everything else of the sixteen files was read in full. Read outside the part only to check findings: `apps/moonglow/src/main.rs` (95-225, 540-640), parts of `mg-edit/src/lib.rs`, `mg-module/src/{lib,archive_file,transfer}.rs`, `outside.rs`, `project_commands.rs`, `side_doc.rs`, `images.rs`, `script_view.rs`, `blueprint/follow.rs`.

## Looked at and found sound

- open_tabs.rs: a record's lists are never empty when read (`parse_list`), so `tabs.rs`'s `expect("at least one")` and `keys[0]` hold; tabs whose resource, object or file is gone are passed over; the record is written through `.partial` and a rename and never while the kept tabs still wait to be opened.
- jobs.rs and recovery.rs's threads: a job's panic is joined and logged and the next job runs; a canceled job hands nothing over and `project_commands::window` (837-843) takes back what its command let go; an autosave overtaken by a save removes itself (`forgotten`), and `Done` frees `busy` after a panic.
- widgets.rs: commit-on-blur fields set their text before a click or a command's key takes them away (`text_begin`, `leave_text` with `command_after_text`); `RowChoice` over tens of thousands of rows (rows in sight, found once a filter); `ResRefText` (16 characters, Aurora's, pasted text cut).
- Workspace revisions are a range of their own for each workspace (`mg_edit::first_revision`), so what is kept by revision (the tree's groups, model views, thumbnails) is not taken for another module's.
- keys.rs: the defaults share no key (tested), and AltGr on Windows is not Ctrl+Alt to winit 0.30.13 (`keyboard_layout.rs:277-281`), so the Ctrl+Alt+letter defaults don't fire on `ą` or `ś` there.
- text.rs's line-end diff: bounded work (`DIFF_WORK`), pairs in rising order, an untouched text byte for byte.
