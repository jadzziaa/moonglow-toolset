---
type: Report
title: 'Code review of November 2026: the 2DA, conversation, talk table and hak editors and the module-wide tools'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the 2DA, conversation, talk table and hak editors and the module-wide tools (mg-ui): 30 in all (4 high, 22 medium, 4 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-ui, 2da, conversations, haks, search]
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

# Code review of November 2026: the 2DA, conversation, talk table and hak editors and the module-wide tools

One part of [the review of 9 November 2026](overview.md): the 2DA, conversation, talk table and hak editors and the module-wide tools (mg-ui). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## High

### D1. The hak editor's right-click Remove indexes the hak's items after they shrank: a panic

- **Kind:** bug
- **Severity:** high (crash; the hak's unsaved changes are lost)
- **Where:** crates/mg-ui/src/hak_view.rs:646-648 (`for &r in &rows[range] { let item = doc.hak.items()[r]`), :704-707 (`doc.hak.remove(&[item.key])` inside the loop, in the context menu's closure); crates/mg-module/src/hak_edit.rs:505-515, :276-280
- **Since:** before 2253fa78
- **Scenario:** Tools › New Hak, Add Files `a.tga`, `b.tga`, `c.tga` (all three rows in sight). Right-click `a.tga` › Remove. The hak now has two items, and the loop goes on over `rows`, indexes worked out before: `items()[2]` is out of bounds. It panics whenever the hak's last-stored item is drawn below the row removed: in a hak that fits the list and is stored in name order (one built from a folder, most archives), any row but the last. In a long hak with that item out of sight there is no panic; the rows under the removed one show their neighbor's item for a frame.
- **Fix:** Note the key and remove it after the list is drawn, as Rename and the toolbar's Remove do (`todo`, or a local read after `show_rows`).
- **Test:** `tests/app/resources.rs`: a new hak with three files, `click_secondary` on the first row, click "Remove", `h.run()`: no panic, two rows left, Undo brings three.
- **Confidence:** confirmed by reading (`hak_view::ui`'s row loop; `Hak::remove` → `commit` → `apply`, which does `self.items.remove(at)`; `recovery.rs` catches the frame's panic, writes the module's recovery copy and resumes the panic; no app test clicks this Remove: `resources.rs:794` is Module Properties' hak list)
- **Read again by the reviewer:** holds: `doc.hak.remove(&[item.key])` inside the loop over `rows`, whose next turn reads `doc.hak.items()[r]` (hak_view.rs:647-648, :704-705).

### D2. Advanced Search changes what its hits pointed at when they were found, not what is there now; Duplicate Tags the same

- **Kind:** bug
- **Severity:** high (the wrong objects are edited, many at once, silently)
- **Where:** crates/mg-ui/src/search.rs:641-677, :1203, :1341-1352; crates/mg-ui/src/lib.rs:3444-3510; the same pattern in crates/mg-ui/src/tag_clash.rs:224-262
- **Since:** before 2253fa78 (fb8fbc1f); the list changes that add and remove feats, classes, properties and items on those hits came after (5150246a)
- **Scenario:** Search "Variables: nQuest": a hit at `keep.git /Creature List[3]`. Click it, delete creature 1 in the area (or Undo, or a file read again from outside), go back to the tab. "Find again" shows only when the question changed, so the list looks current. "Change N Found" lands on whatever creature is at index 3 now. A conversation's script hit (`EntryList[i]`) goes wrong the same way after a line is deleted. Second: close the module and open another with the same resrefs (a backup): the tab still lists the first module's hits, and Change applies them by path to the second. Third: the Duplicate Tags window is not modal and keeps positions; delete another object of that list, then Retag: a different object gets `LAMP_2`.
- **Fix:** Keep `ws.revision()` with the hits (and with `TagClash`); in `change_found`, `together` and go, when it differs, find again first (the kept search makes that cheap) or refuse with "Find again". Reset `advanced_search.hits`/`searched`/`ticked` and `tag_clash` in `close()`.
- **Test:** `tests/app/search.rs`: two placed creatures found by a variable, `Edit::remove_item` on creature 0, "Change 1 Found": the variable is on the creature that was found. A second: open another module, the tab lists nothing.
- **Confidence:** confirmed by reading (fork read `search.rs` whole and `mg_edit::found::{edits, objects, set_fields}`, which read the live module at each hit's key and path with no check; I re-read `change_found` at search.rs:641-677, `current` at :1203 and `Moonglow::close` at lib.rs:3444-3510, which resets `search_kept` and not `advanced_search`, `tag_clash`, `import` or `dialog_clip`)
- **Read again by the reviewer:** holds: `change_found` gives `draft.chosen()`, the hits as found, to `found::edits` on the module as it is, with no revision kept (search.rs:641-677).

### D3. A project command whose script leaves a program running never ends, and Cancel does nothing

- **Kind:** bug
- **Severity:** high (the job's modal stays for good; Moonglow has to be killed)
- **Where:** crates/mg-ui/src/project_commands.rs:603-607 (the joins), :592-600 (the only place Cancel is read)
- **Since:** before 2253fa78
- **Scenario:** `update = "git pull && (mg watch . &)"`, a script that ends with `nwserver … &`, or on Windows `start tool.exe`: the shell exits 0, the loop breaks, and the join waits on the grandchild's copy of stdout. Cancel shows "Canceling…" forever. The comment at :603 knows of the case; only the Canceled path skips the join. After a Cancel, a reader thread left behind also keeps pushing into the shared `output`: its lines land in the next command's window.
- **Fix:** After the shell ends, wait for the readers a short while (`is_finished()` polled for a second, `cancelled()` still read), then leave them. Give each run its own `Output`.
- **Test:** unix unit test beside `stopping_a_command_stops_what_its_shell_started`: `run_child(shell("sleep 30 & echo started", ..))` returns `Exited(Some(0))` within two seconds with "started" in the output.
- **Confidence:** confirmed by reading (fork read `project_commands.rs` whole and `jobs.rs`; I re-read `run_child` at :548-609: after the shell exits, `out.join()` and `errs.join()` wait for end of file on pipes a background child still holds, and `progress.cancelled()` is read only inside the loop above)
- **Read again by the reviewer:** holds: `out.join()` and `errs.join()` after the shell has ended, unless canceled (project_commands.rs:603-607).

### D4. Export as Ink/Twine/Text/CSV joins the folder with a conversation's ResRef unchecked

- **Kind:** bug
- **Severity:** high by the brief's scale (a file written outside the folder chosen), limited (16 bytes of name, a fixed extension, a crafted module and the builder's Export)
- **Where:** crates/mg-ui/src/dialog_export.rs:41-43, :116-119, :147-149
- **Since:** after (efb65958)
- **Scenario:** A downloaded module holds a conversation named `../../../x`. Select the conversations in the tree, Export as CSV…, pick a folder: `x.csv` is written three folders above it, and the "there already" question probes outside too.
- **Fix:** Build the path with `path_in`'s check (`key.resref.is_file_name()`, or a helper that takes the extension) and list a refused name among `failed`.
- **Test:** unit test in `dialog_export.rs`: `write_conversations` with a key `../x` into a scratch folder writes nothing outside it and names it as failed.
- **Confidence:** confirmed by reading (fork read `dialog_export.rs` whole; I re-read `file_name` at :41-43, the `exists()` probe at :116-119 and `std::fs::write(dir.join(&name), text)` at :147; `ResKey::path_in` in mg-resman/src/lib.rs:55-61 is what every other export of this part goes through: `hak_edit::extract`, `transfer::export_files`, `browser::export`)
- **Read again by the reviewer:** holds: `format!("{}.{}", key.resref, ..)` joined to the folder and written (dialog_export.rs:41-43, :147).

## Medium

### D5. A conflict waiting in Changed Outside Moonglow is forgotten when any other file changes

- **Kind:** bug
- **Severity:** medium (the question closes unanswered; Save is then refused with nothing left to ask)
- **Where:** crates/mg-ui/src/outside.rs:466-469 (`changes.is_empty()` → `conflicts.clear()`), :502 (`self.outside.conflicts = conflicts`: this look's alone), :172-176 and :204-212 (a look of the heard files alone)
- **Since:** after (a671cc83, "the files it names looked at alone": the two lines were right while every look was of every file)
- **Scenario:** `a.nss` is typed in Moonglow and saved by another editor: the window lists it. Before it is answered, another file of the module changes in the folder: `b.nss` saved outside, the second burst of a pull. That look is of those files alone; its conflicts (none) replace the list, the window closes, and the log line is not said again (`said`). Save then fails naming `a.nss` (`ModuleError::ChangedOnDisk`), every time, until `a.nss` changes again or something makes a full look (a project command, a watch that lost count).
- **Fix:** A look of some files replaces the conflicts of those files only: keep the others (those that still `stand`), and clear on `changes.is_empty()` only for a full look.
- **Test:** `tests/app/projects.rs` with the told watch (or `look_outside_at(Some(..))`): a conflict on `a.nss`, then `b.nss` changed on disk; after the look `outside_conflicts()` still lists `a.nss`.
- **Confidence:** confirmed by reading (`reload_changed_files` → `look_outside_at(heard.files())` → `outside_scan_of` → `folder::scan`/`nasher::scan`, which pass over every file not in `only`; `take_found`; `Module::write_folder` and `nasher/project.rs:982` return `ChangedOnDisk`, which the window only logs; the app tests of conflicts use the full look `reload_project_files`)

### D6. A command that changes several protected areas is asked about for the first only, and "Anyway" lets them all through

- **Kind:** bug
- **Severity:** medium (the other areas' Tags change, or the areas are deleted, without being named: what `[claims] protected` is for)
- **Where:** crates/mg-ui/src/claims.rs:175-216, :239-243
- **Since:** before 2253fa78
- **Scenario:** Two merged areas `a` and `b` chosen in the tree, Delete: the Protected Area window names `a`. "Delete It Anyway" passes `Pass { a, Deleted }`; the command is applied again, `protected_change` finds `a` first again, the pass matches, and `b` is deleted with it, never named. The same for Advanced Search › Set a field `Tag` over hits in several protected areas.
- **Fix:** `protected_change` returns every protected change of the command; the window lists them all, and the pass holds that set.
- **Test:** `tests/app/projects.rs`: `[claims] protected = ["a*", "b*"]`, `Action::DeleteResources([a.are, b.are])`: the window names both (or asks twice), and neither goes before both are answered.
- **Confidence:** confirmed by reading (`protected_change` returns at its first hit; `gate` compares that first hit with `let_through` and returns `Ok` for the whole command; `Action::DeleteResources(keys)` → `delete_resources` makes one command for all keys; Advanced Search's Set a Field makes one for all hits)

### D7. A `.mod` dropped on the window while a job runs opens over it, and the job's result lands on the other module

- **Kind:** bug
- **Severity:** medium (Build Module's compiled scripts of module A applied to module B as one step)
- **Where:** crates/mg-ui/src/transfer.rs:316-318, :963-965; crates/mg-ui/src/lib.rs:3689; landing in crates/mg-ui/src/build_view.rs:258-271
- **Since:** after (0b7c8c57, dropped files)
- **Scenario:** Save a persistent world, start Build Module (tens of seconds), drop another `.mod` on the window. Nothing is unsaved, so it opens at once. When the build ends, its edits are applied to the new module.
- **Fix:** Ignore or queue drops (and `OpenModule`/`Close`) while `busy()`, with a line in the log; in `finish_job`, drop a result whose module is no longer the open one.
- **Test:** app test with `background_jobs = true`: a job waiting on a channel, `take_dropped(vec![other_mod])`, a frame: the open module is unchanged and the log says why.
- **Confidence:** confirmed by reading (fork read `build_view.rs` and `jobs.rs`; I re-read `drop_ui`/`take_dropped` at transfer.rs:316-328 and :953-966, called from `transfer::ui` at lib.rs:1730 whatever `busy()` says, and `Action::OpenModule` at lib.rs:3689, which has no such check; lib.rs:1577 keeps only the shortcuts off under a job)

### D8. Paste in the 2DA editor under a Find writes into the rows the Find hides

- **Kind:** bug
- **Severity:** medium (cells of rows not in sight overwritten; Undo takes it back, if the builder knows to)
- **Where:** crates/mg-ui/src/table_view.rs:1661-1662, :1687-1716; crates/mg-module/src/table_edit.rs:318-326
- **Since:** before 2253fa78
- **Scenario:** Find "elf" shows rows 3, 17 and 42. Select the three Label cells, Ctrl+X: rows 3, 17 and 42 are emptied. Ctrl+V on the same selection: the three lines go into rows 3, 4 and 5. Rows 4 and 5, hidden by the Find, are overwritten; 17 and 42 stay empty. Nothing on screen shows either.
- **Fix:** Paste line by line into the shown rows from the cursor on (`rows[place + dr]`), adding rows only past the table's end with no Find; or refuse a paste of several lines while a Find narrows the rows, saying why.
- **Test:** `tests/app/tables.rs`: a filter that leaves rows 1 and 3, select both, cut, paste: rows 1 and 3 hold the text again and row 2 is as it was.
- **Confidence:** confirmed by reading (`keys_in`: Copy, Cut and Delete act on `chosen`, the rows shown inside the selection; Paste calls `Editing::paste(row, column, text)`, which writes `row + dr` for each line; `shown_rows`)

### D9. Rename decides which compile failures are its own by a substring of the error's message

- **Kind:** bug
- **Severity:** medium (a script loses its compiled code and the builder isn't told)
- **Where:** crates/mg-ui/src/references.rs:288-297
- **Since:** before 2253fa78 (565d7403)
- **Scenario:** Rename `inc_old`. `quest_main.nss` includes it, is rewritten and loses its `.ncs`; its compile fails in another include, so the message reads `inc_utils.nss(12): …` and holds no "quest_main": nothing is logged, `compiled_ok` stays true, and the game stops running the script. The other way: a recompiled script named `ai` matches the error of an unrelated `main.nss`.
- **Fix:** `report.recompile.contains(&r.script)`.
- **Test:** app test: a script with a stale `.ncs` whose include has an error, a rename that rewrites its `#include`: the log warns it isn't compiled; an unrelated failing script whose name contains a recompiled one's gives no warning.
- **Confidence:** confirmed by reading (fork read `references.rs` whole, `mg_module::rename::recompile` and `RenameReport::recompile`; I re-read references.rs:288-297: `e.message.contains(&k.resref.to_string())`)

### D10. Every search overwrites the change's name with the field searched for

- **Kind:** bug
- **Severity:** medium (a variable of the wrong name written to everything found)
- **Where:** crates/mg-ui/src/search.rs:633-636
- **Since:** before 2253fa78 (fb8fbc1f)
- **Scenario:** Look for Appearances, choose Change › Set a variable, type `nBoss`, value 1, Change. The search runs again and the box reads `Appearance_Type`. Change the value and click again: every object found gets a variable named `Appearance_Type`.
- **Fix:** Follow the searched field only for `Act::SetField`, and only when the question changed.
- **Test:** `first = Appearance`, `act = SetVariable`, `name = "nBoss"`: after `advanced_search` and after `change_found`, `act.name == "nBoss"`.
- **Confidence:** confirmed by reading (fork; I re-read search.rs:619-637, `Condition::field` at :227-235 and `ActDraft::change` at :325-340, where Set a Variable, Remove a Variable and Set a Field all read `self.name`; `change_found` searches again at :677)

### D11. The conversation clipboard keeps line numbers across deletes, Undo and a change of module

- **Kind:** bug
- **Severity:** medium (Paste As Link, and a pasted branch's grey links, point at other lines)
- **Where:** crates/mg-ui/src/dialog_view.rs:786-794, :1005-1028, :1030-1063; crates/mg-ui/src/lib.rs:3477
- **Since:** before 2253fa78
- **Scenario:** Cut line Reply 4, select an NPC line, Paste As Link: the button is on (`c.from == key`) and links to whatever is Reply 4 now. Copy a line, delete an earlier one, Paste As Link: the same. Copy a branch that holds a link, delete an earlier line, Paste: the pasted link keeps the old index. Close the module and open one with a conversation of the same ResRef: the clip is taken for that conversation's.
- **Fix:** Renumber the clip in `follow` (its `target` and the branch's links out, dropping what was deleted); clear `target` on Cut; clear the clip, or mark it another module's, in `close()`.
- **Test:** `tests/app/conversations.rs`: copy a line, delete an earlier line, Paste As Link: the link's target has the copied line's text; the same after Cut.
- **Confidence:** confirmed by reading (fork read `dialog_view.rs` whole and `mg_module::dialog::{copy_branch, paste_branch, link_lines}`; I re-read `DialogClip` at :786-794, Cut/Paste/Paste As Link at :1005-1063 and `follow` at :155-200, which renumbers bookmarks, folds and the selection, not the clip; `close()` does not clear `dialog_clip`)

### D12. "Run a Project Command?" does not block the window: Allow and Run runs over what is unsaved or open by then

- **Kind:** bug
- **Severity:** medium (a checkout runs over unsaved edits; the manual says it saves first)
- **Where:** crates/mg-ui/src/project_commands.rs:845-894, :254-263
- **Since:** before 2253fa78
- **Scenario:** Choose Update for the first time: the module is saved and the question shows. Edit an area with the question open, then Allow and Run: `git pull` runs with the edits unsaved. Or open another project meanwhile: the old project's command runs in the old root while `pause_outside` and `release_haks` act on the new module.
- **Fix:** On Allow, record the permission and push `Action::ProjectCommand(name)` again, so it goes through the save question; drop `ask` when the module closes or its root differs.
- **Test:** app test: an unallowed command, an edit while the question shows, Allow and Run: the save question comes (or the file has the edit) before the command's output.
- **Confidence:** confirmed by reading (fork read the whole file and the save question at lib.rs:3597-3606; I re-read `allow_and_run` at :254-263 and the window at :845-894: `widgets::dialog` is a plain `egui::Window` (widgets.rs:3018), and `start` saves nothing)

### D13. Closing the window while a project command or upload runs leaves its shell running

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-ui/src/project_commands.rs:357, :594-596; crates/mg-ui/src/lib.rs:3618-3623, :4135-4139
- **Since:** before 2253fa78
- **Scenario:** "Don't ask again" ticked once on closing. Run Update, click the window's X while it merges: Moonglow exits, and the shell goes on in its own process group with closed pipes (a `git stash; git pull; git stash pop` dies at its next line of output, between the steps). Without that setting the Close question opens under the job's modal and can't be answered.
- **Fix:** While a job runs, the X asks for Cancel first; or keep the running child's group in `CommandsState` and `stop` it on `Quit`.
- **Test:** unix app test: `sleep 60 & echo $!; wait` as a command, `Action::CloseWindow` with `close_without_asking`: the sleeper's pid is gone, or the quit is refused with a log line.
- **Confidence:** fork's reading, not re-read by me (apps/moonglow/src/main.rs:143-145 pushes `CloseWindow` whatever runs; lib.rs:3622 turns it into `Quit` with `close_without_asking`; `Quit` at lib.rs:4135-4139 only sets `quit_requested`; `stop()` is called from the Cancel branch alone)

### D14. The raw fields view shows a localized string's English text and writes what is typed into the editing language

- **Kind:** bug
- **Severity:** medium (with Options › Language set to another language)
- **Where:** crates/mg-ui/src/gff_view.rs:364-371; crates/mg-ui/src/text.rs:463-474
- **Since:** before 2253fa78 (b57ef0c1)
- **Scenario:** Editing language Polish. In a blueprint's raw fields, type in a `LocalizedName` box: the box showed the English text, the commit replaces the Polish text, the English is unchanged, so the box shows the old value again and the edit looks refused. Clearing the box removes the Polish text.
- **Fix:** Show `text::edited_text(ls)` (the variant written), or a row a language. Rename `with_english` to what it does.
- **Test:** `tests/app/fields.rs`: edit language French, a UTC as `Tab::Gff`, type in a LocString field: the box showed the French text and the English is untouched.
- **Confidence:** confirmed by reading (fork; I re-read gff_view.rs:364-371 and `text::with_english` at text.rs:463-474, which sets or removes `edit_language()`'s text)
- **Read again by the reviewer:** holds (drawn in the sample): the box shows `text_in(Language::ENGLISH, ..)` and writes with `with_english`, which sets the editing language's text (gff_view.rs:364-370; text.rs:463-473).

### D15. Windows that no longer block the editor hold a line by its number, or a command made earlier

- **Kind:** bug
- **Severity:** medium for the typed-text question (edits taken back, or an edit dropped), low for the others
- **Where:** crates/mg-ui/src/dialog_view.rs:993, :1447-1500 (`input.parent`), :1839-1843 (String Edit's path); crates/mg-ui/src/talk_typed.rs:268 (the one `ask` slot holding `cmd`), :499-503
- **Since:** after (576682f4 for Input Text; f0dec956 for the held command)
- **Scenario:** (a) With typed text going to the talk table and a reply the table has already ("Goodbye."), the question holds `[SetResource(the conversation as it was), SetField]`. Edit the conversation behind the question, then answer: the snapshot replaces it, and the edits made meanwhile are gone. (b) While that question shows, type another text the table has in another field: `app.typed.ask` is replaced, and the first field's edit is dropped without a word. (c) Add opens Input Text under a line; delete an earlier line behind it; OK: the new line goes under whatever has the parent's number now, or is appended with no link.
- **Fix:** Make the talk-table question blocking (an `egui::Modal`: it stops the edit it belongs to), or queue the asks and build the Add command when it is answered. Map `input.parent` and an open String Edit's path in `follow`, closing the popup when its line is deleted.
- **Test:** `tests/app/typed_text.rs`: two typed texts the table has, the second while the first is asked: both fields end set. `conversations.rs`: Input Text under line B, delete line A, OK: the new line is under B's text.
- **Confidence:** confirmed by reading (fork read `dialog_view.rs`; I re-read Input Text at dialog_view.rs:1447-1500, `widgets::modal` at widgets.rs:2992-2997 (a plain `egui::Window`: docs/log.md:391 moved these off `egui::Modal`), and `talk_typed::apply_with` at :263-270 and `window` at :441-505)

### D16. On Windows a command that begins with a quoted program and has another quoted argument loses its first and last quote

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-ui/src/project_commands.rs:271-274; the same shape in crates/mg-module/src/nwsync.rs:598-600
- **Since:** before 2253fa78
- **Scenario:** `windows = "\"%MOONGLOW_MG%\" tlk-alloc \"my module\""`: cmd runs `C:\Program Files\Moonglow\mg.exe" tlk-alloc "my module` and fails with "'C:\Program' is not recognized".
- **Fix:** `cmd /S /C "<text>"`: the raw text in one more pair of quotes, which `/S` strips exactly.
- **Test:** Windows unit test: a `.cmd` in a scratch folder with a space in its name, run as `"<path>" "a b"` through `shell()`: exit code 0 and `a b` echoed.
- **Confidence:** suspected (fork's reading of `cmd /C` plus `raw_arg(text)`, which I re-read at project_commands.rs:268-274, and of cmd's documented rule for a command line that starts with a quote; a run on Windows of `"C:\Program Files\x\mg.exe" tlk-alloc "my module"` would settle it. CI's test has two quotes and no space, so it passes either way)

### D17. A tag with a letter outside ASCII is compared as UTF-8 text against the module's codepage bytes

- **Kind:** bug
- **Severity:** medium for modules in Spanish, Polish and the like; low otherwise
- **Where:** crates/mg-ui/src/references.rs:918; crates/mg-ui/src/tag_clash.rs:121-123, :130-135, :166-167
- **Since:** after (d9d78567, 136f27e9)
- **Scenario:** Find References for the tag `Señor`: UTF-8 `C3 B1` is compared with the file's `F1`: "Nothing in the module names it". Paste a copy of an object tagged `Señor`: the Duplicate Tags window lists "Se\u{fffd}or", matches no object, and Retag skips it.
- **Fix:** Carry tags as bytes: `Clash` keeps the raw tag and the window decodes it; `tag_usages` and `mentions` take bytes encoded by the caller.
- **Test:** mg-module: `tag_usages` and `clashes` over `b"Se\xF1or"`; app test: Retag on such a paste gives `Señor_2`.
- **Confidence:** fork's reading, not re-read by me (`UsageIndex::tag_usages` rename.rs:486-490 and `mentions` :539-553 compare `name.as_bytes()` with raw bytes; `mg_module::tags::clashes` makes `Clash.tag` with `from_utf8_lossy` at tags.rs:350; `tag_clash::blueprint_copy_warning` does it right with `text::encode`)

### D18. Import Folder (and a folder dropped) compares every file with every other and reads them all on the window's thread

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-ui/src/transfer.rs:286-288, :316-321; crates/mg-module/src/transfer.rs:301-341 (:313, :328-331)
- **Since:** after (0b7c8c57)
- **Scenario:** A builder merging two worlds drops the other module's unpacked folder (20,000 files) on the window: 200 million path comparisons and as many key comparisons before the first file is read, then every file read whole: the window stands still for tens of seconds, with no progress and no Cancel.
- **Fix:** A `HashSet<PathBuf>` for the files seen and a `HashMap<ResKey, Vec<usize>>` for the names given twice; run it as a job (`start_job_anywhere`), the Import window opened in `finish`.
- **Test:** mg-module unit test: `Incoming::files` over 20,000 small scratch files inside a time bound; app test: `busy()` is true right after `import_files` of a large folder.
- **Confidence:** confirmed by reading (`Moonglow::import_files` → `Incoming::files`: for each file `keyed.iter().any(|(_, p)| *p == f)`, then for each key a scan of all keyed for a second file of its name, then `std::fs::read` of every file; all inside the frame)

### D19. Hak Save and Extract, the browser's Export as Files and Add Haks write gigabytes on the window's thread; two of them write over files there unasked

- **Kind:** performance (and a drift from the "Files there already" question)
- **Severity:** medium
- **Where:** crates/mg-ui/src/hak_view.rs:404-431, :818-824; crates/mg-module/src/hak_edit.rs:592-606; crates/mg-ui/src/browser.rs:505-545; crates/mg-ui/src/module_props.rs:854, :937
- **Since:** before 2253fa78 (Add Haks' archives after)
- **Scenario:** Open a 3 GB hak of 20,000 resources and click Extract All…, or change one file and Save: the window freezes for the whole copy. In the browser, narrow to `.mdl` (30,000 with CEP) with Models as Text and Export: every model is decompiled in one frame. Extract All into the folder the hak was built from writes the hak's older copies over the builder's edited files without the question that adding files, a model's Export Files and the conversation export ask (docs/log.md:32 lists those; these two are not among them).
- **Fix:** Run them as jobs with progress by bytes and Cancel (the `.part` removed). Ask `widgets::files_already` before Extract and Export as Files write over files there.
- **Test:** app tests with `background_jobs = true`: after Extract All of a large hak `app.busy()` is true; Extract onto existing files asks first (Replace, Skip Those, Cancel).
- **Confidence:** confirmed by reading for the hak editor and the browser (`hak_view::save` → `Hak::save` streams every resource into `.part` inside the frame; `Do::Extract` → `Hak::extract`, a `std::fs::write` a resource; `browser::export` reads, decompiles with Models as Text and writes each listed resource, and stops at the first error without saying how many were written); fork's reading for Add Haks (`attach::placements`, `attach::copy`, `archive::extract`). Compile Models, beside them, is a job.

### D20. The 2DA grid looks each cell's fault up by scanning the whole list, and lays out the columns out of sight

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-ui/src/table_view.rs:2223-2258 (:2258); the same scan nested in `faults()` at :910-917 (`theirs.iter().any` inside `retain`) and :935-942 (`found.iter().any` for each of the description's faults), run at every change
- **Since:** before 2253fa78
- **Scenario:** A hak's `placeables.2da` or `appearance.2da` opened from the hak editor with no module open, or with one that doesn't name the hak's talk table: every custom StrRef cell is a fault ("the module has no custom talk table"), 15,000 to 30,000 of them. With 50 rows and 20 to 40 columns laid out, that is 15 to 60 million comparisons a frame. Apart from faults: `spells.2da` (55 columns) or `baseitems.2da` on a tall window lays out some 3,000 cells a frame, most of them scrolled out sideways.
- **Fix:** Keep the faults sorted by (row, column) always (today only with a description) and take each row's run with `partition_point`; a `HashSet<(usize, usize)>` for the two `retain`s. Skip a cell whose rect is out of the clip (`allocate` its width, no text).
- **Test:** `frame_perf.rs`: the 2DA editor on a 15,000-row table whose StrRef column is all faults, and on `spells.2da` at its full width, inside `FRAME`.
- **Confidence:** confirmed by reading that the cost is cells laid out × faults (`grid_in`: `faults.iter().find(|f| f.row == r && f.column == c)` for every column of every row in sight; rows are culled by `show_rows`, columns are not: a cell out of sight sideways still makes its text and its `Label`); the frame time is suspected: `frame_perf` measures placeables.2da with its talk table, about 20 columns

### D21. The 2DA inspector builds Choose Model's list from every key of the load order each frame, and clones a whole open table to read one row of it

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-ui/src/table_view.rs:3012, :2946-2962; :842-844, :2506-2509, :2519-2520, :2616, :926
- **Since:** after (the described tables and Choose Model…)
- **Scenario:** Choose Model… with nothing typed on an install with CEP's haks: 40,000 to 60,000 model names made, sorted and cut to 200, every frame the list shows. A project whose `feat.2da.toml` says `PREREQFEAT1` is a row of `feat` (the table itself), or two project tables open of which one names the other's rows: the whole table (for a 20,000-row table, over a million cells) is cloned every frame a cell of that column is selected.
- **Fix:** Keep the model list by `(find, load order)` as `string_choice` keeps its strings (or the sorted names once, in `GameData`). `Editing` holds its table in an `Arc` (or `project_table` hands back a borrow), and the faults ask for `len()` without a copy.
- **Test:** `frame_perf.rs`: the inspector with Choose Model… open; and a described table with a `Row` column naming itself, a cell of it selected.
- **Confidence:** confirmed by reading (`model_choice` calls `models_named(game, &find, 200)` every frame while the list is open: every layer's keys, a `String` for each MDL, sort, dedup; `string_choice` beside it keeps its list by what was typed. `project_table` returns `Arc::new(t.edit.table().clone())` when the named table is open in the editor, and `inspector`'s `named_table` calls it every frame for a description's `rows = "…"` and for a `Row` column's cell; `faults()` calls it to read a length)

### D22. Take Rows looks every changed cell's row up in a list of the rows taken

- **Kind:** performance
- **Severity:** medium (a freeze of seconds to a minute, once)
- **Where:** crates/mg-module/src/table_edit.rs:362-367, :388-391 (called from crates/mg-ui/src/table_view.rs:3387, on the window's thread)
- **Since:** after
- **Scenario:** A module's copy of `placeables.2da` taking CEP's 20,000 new rows (tick New, Take): 200,000 cells change, each looked up twice in a 20,000-long list: four billion comparisons. A 30,000 by 60 table is ten times that.
- **Fix:** A `HashSet<usize>` (or the sorted `rows` with `binary_search`).
- **Test:** unit test in `table_edit.rs`: `take` of 20,000 rows of ten cells inside a time bound.
- **Confidence:** confirmed by reading (`Editing::take`: `changed.iter().map(|c| c.0).filter(|r| rows.contains(r))` and again for `one_by_one`, `rows` a `Vec`; and `.filter(|(r, _)| !rows.contains(r))` over the cells)

### D23. Advanced Search's result list lays out every hit every frame, and with a plugin command it is quadratic

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-ui/src/search.rs:1174-1185, :1292-1332, :1314-1324
- **Since:** the rows before 2253fa78 (a1dc8a05); the plugin selection after (7d3af96e)
- **Scenario:** "Variables: *" on the persistent world of `docs/research/notes_scale.md`: 8,785 hits, all ticked: 8,785 rows of widgets a frame. With one enabled plugin that has a command `for` something, each frame also builds 8,785 × 8,785 `Selected` values.
- **Fix:** Skip rows out of sight as `references.rs:1001-1009` does and name a place only for rows drawn; build the ticked selection once a frame, or inside `context_menu` when it opens.
- **Test:** `frame_perf.rs`: the tab with 10,000 hits ticked, with and without a plugin menu command.
- **Confidence:** confirmed by reading (fork read :1165-1365, `plugins::selected_at` and `kinds_of`; I re-read :1286-1332: a horizontal, checkbox, link, labels and an eager `format!` hover for every hit with no `is_rect_visible`, and for each ticked row a `Vec` of every ticked hit's `Selected`)

### D24. Four more lists are made and laid out whole every frame: the Claims Table, a conversation's Search results, Build Module's results, an opened list of the raw fields

- **Kind:** performance
- **Severity:** medium for the Claims Table and the conversation search, low for the others
- **Where:** crates/mg-ui/src/claims.rs:552-555, :574-638 with crates/mg-module/src/project_file.rs:1607-1633; crates/mg-ui/src/dialog_view.rs:2487-2502 and :2445-2461 (Find over all files parses and clones every conversation on the window's thread); crates/mg-ui/src/build_view.rs:161-187; crates/mg-ui/src/gff_view.rs:198-227; crates/mg-ui/src/browser.rs:810-820
- **Since:** the Claims Table window after; the others before 2253fa78
- **Scenario:** The Domains of Dread's 1,400 areas (the project the claims were made for): Edit › Claims Table… makes about eight million ResRef comparisons and 1,400 rows of widgets every frame it is open. A conversation search for "the" over All Files in Module: tens of thousands of results copied and drawn a frame. Build Module's first Unused check on a world: 20,000 results. A 32 by 32 area's `Placeable List` opened in the fields view: a header an object.
- **Fix:** `ScrollArea::show_rows` (or `is_rect_visible`) in each; the claims listing kept until the project file or the module's areas change, its lookups through a map; the search results by reference, the click acted on after the loop.
- **Test:** `frame_perf.rs` cases: the Claims Table with 1,400 areas; a conversation with 20,000 matching lines and the Search pane open; Build Module with 20,000 results; a GIT of 3,000 placeables as `Tab::Gff` with the list open.
- **Confidence:** confirmed by reading for the Claims Table (`table_window` calls `claims_listing(Some(&areas))` each frame: a clone of the table, then for each area a linear `areas.contains`, `claims.of`, `protected` and `module.contains`; then an `egui::Grid` row with labels and buttons for every area, none skipped); fork's reading for the rest (`dialog_view::search_pane` clones `s.results` and draws a label with each result's whole text; `build_view` a `selectable_label` a result; `gff_view::struct_ui` and `browser::gff_tree` a header an item)

### D25. Module Properties › Custom Content reads and parses the project's `[talk] map` twice every frame

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-ui/src/module_props.rs:676-690, :763-775; crates/mg-ui/src/talk_typed.rs:144
- **Since:** after (f0dec956)
- **Scenario:** A project that uses `mg tlk-alloc` with a map of 20,000 lines: each frame the page shows, two file reads, two JSON parses of about a megabyte, two sets of 20,000 lines, and a walk of the used part of the range.
- **Fix:** Keep the map's lines (an `Arc<BTreeSet<u32>>`) in `ProjectFile`, read when the project file is; work `said` out when the table, the range or the map changes.
- **Test:** `frame_perf`: Module Properties on Custom Content with a map of 20,000 lines; or a unit test counting reads of the map over two frames.
- **Confidence:** confirmed by reading (fork; I re-read module_props.rs:676-690 and :763-766, talk_typed.rs:140-152 and `ProjectFile::talk_map` at project_file.rs:1690-1700: a `read_to_string` and `Map::read` each call; `next_free` then walks the range from its start past every used line)

### D26. Every edit of a conversation works out every line's signature twice, hashing each ancestor's text again for each descendant

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-ui/src/dialog_view.rs:161-175; crates/mg-module/src/dialog.rs:497-541
- **Since:** before 2253fa78 (bc2bac0b)
- **Scenario:** A 1,000-line conversation whose greeting is 30,000 characters: every committed edit formats that greeting once for each descendant, for the conversation before and after. A plain 2,000-line conversation 30 deep makes 120,000 format-and-hash calls an edit.
- **Fix:** In `signatures`, hash each line once and build paths from the hashes. The editor remembers its own edits that renumber nothing as `Change { renumbered: None }`.
- **Test:** mg-module: `by_lines` on a 2,000-line chain under a 30,000-character root inside a time bound.
- **Confidence:** fork's reading, not re-read by me beyond `follow` at dialog_view.rs:155-175 (any change that is not a remembered delete goes to `Renumbering::by_lines(&before, g)`); the cost in `mg_module::dialog::signatures` (dialog.rs:497-541: `own(n)` formats `Text` and `Speaker` with `{:?}` and is called for every line on every descendant's path) is the fork's

## Low

### D27. Each text typed into the talk table copies the whole table and drops every table and choice list the game data has read; with the game data held, the new line is silently not shown

- **Kind:** performance (and an error swallowed)
- **Severity:** low to medium
- **Where:** crates/mg-ui/src/talk_typed.rs:369-378 (called at :319, :392, :426); crates/mg-rules/src/lib.rs:284-290
- **Since:** after (f0dec956)
- **Scenario:** Options › Language › Put text typed into the talk table on, a world with CEP's haks and a custom table of 50,000 lines. Every name, description or conversation line committed copies the 50,000 entries and forgets `appearance.2da`, `placeables.2da` and every choice list, which the open editors read and build again in the next frame: a hitch after each field. A text committed while a worker holds the game data past the wait gets its line in the table and no line in what the fields read from.
- **Fix:** Give `GameData` a way to set one custom line (or swap the table) that forgets only what was made from custom strings; say in the log when the game data could not be had, and try again when it is free.
- **Test:** `tests/app/typed_text.rs`: after a typed name, `game.table("appearance")` is the same `Arc` as before; `editor_perf.rs`: a typed line on a world-sized table inside a bound.
- **Confidence:** confirmed by reading (`talk_typed::shown`: `t.tlk.clone()`, `release_game_data()` (stops the plugins' panels, lets the pictures' models go, waits up to `RELEASE_WAIT` for other holders), `game.set_custom_tlk`, `game.invalidate()`, which clears `tables`, `choices`, `marked` and `judged`; and `if let Some(game) = exclusive(..)` with no `else`); what a builder sees when it is skipped is suspected (a field naming a line the game data's table lacks would show no text until the next reload)

### D28. The browser's and the hak editor's 2DA views are kept by the address of the table they were made for

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-ui/src/browser.rs:888-902; crates/mg-ui/src/hak_view.rs:769-771
- **Since:** after
- **Scenario:** View `mg_tab.2da` in the hak editor, Edit Table, change a cell (no row or column added), Save into the hak, View again: where the new `Layered` lands at the old one's address, the view is "kept" and shows the old cells from its own copy, under the new layers' marks.
- **Fix:** Key the view by something that changes with the content: a counter in `Viewed` and in the hak's `revision`, or compare the tables once when the pointer matches.
- **Test:** `tests/app/tables.rs`: view, edit a cell through the 2DA editor, save into the hak, view: the cell shown is the new one (with the view's cache made to collide, e.g. by keying on a test hook).
- **Confidence:** suspected (confirmed by reading that the view is kept while `v.of == l as *const Layered as usize` and the row count and column names are the same; `load_order_changed` clears `viewed` and not `browser.table_views`, and the hak editor drops its `Box<Layered>` at every change. Whether the allocator hands a new table the old one's address is its own business: a test that saves a cell's change into a hak and views the table again, repeated, would settle it)

### D29. The read-only viewer highlights a script again every frame, and copies a text it shows every frame

- **Kind:** performance
- **Severity:** low to medium
- **Where:** crates/mg-ui/src/browser.rs:733-744; crates/mg-ui/src/script_view.rs:293-303
- **Since:** before 2253fa78
- **Scenario:** Double-click `nwscript.nss` in the Resources browser (some 700 KB, over a hundred thousand tokens): tokenized, a hundred thousand sections appended and hashed, every frame the tab shows. An overridden copy of a large compiled model, shown as its text (megabytes), is copied and hashed every frame.
- **Fix:** Keep the `LayoutJob` (or the galley) in `Viewed`, made once for the palette; draw long texts by rows in sight.
- **Test:** `frame_perf.rs`: `Tab::Resource(nwscript.nss)` inside `FRAME`.
- **Confidence:** confirmed by reading (`resource_ui`: `script_view::highlight(text, &palette)` each frame builds a `LayoutJob` with a section a token, which egui then hashes whole to find its galley; `ui.monospace(text)` copies the text into a new job each frame); the frame time is suspected

### D30. Which tabs show a resource is written three times beside `Tab::tree_keys`, and the copies differ

- **Kind:** duplication (with a drift)
- **Severity:** low
- **Where:** crates/mg-ui/src/references.rs:507-517 (`retarget` on rename), :686-695 (`delete_resources`); crates/mg-ui/src/outside.rs:595-604 (files gone outside); the source of truth should be `Tab::tree_key`/`tree_keys` in crates/mg-ui/src/tabs.rs:231-258
- **Since:** before 2253fa78 for the lists; outside's rewritten after
- **Scenario:** An area deleted outside Moonglow leaves its `Instance`, `Instances` and `InstanceModel` tabs open, which `delete_resources` closes. None of the three closes or retargets a `Tab::Blueprints` or `Tab::AreasProperties` holding the key, which `tree_keys` knows of: an Edit Together tab keeps a deleted or renamed blueprint.
- **Fix:** Close with `!tab.tree_keys().iter().any(|k| gone.contains(k))`, and a `Tab::retarget(from, to)` beside it.
- **Test:** app test: delete an area on disk with an Instance tab open: the tab closes; delete a blueprint a Blueprints tab holds: it leaves the tab.
- **Confidence:** fork's reading of the three lists and `tabs.rs:229-260`; I read the one in outside.rs:595-604

## Smaller, not written up as entries

Mine:
- **A table's file is written in place** (`std::fs::write`), not through a temporary file: crates/mg-ui/src/table_view.rs:628-630 (the 2DA editor's Save to a file; the tileset editor beside it writes `.set.part` and renames). The same for an Export onto an archive that is there, Append included (the archive is read, merged and written over itself): crates/mg-ui/src/transfer.rs:836-848, :730. A full disk leaves half a table or half the team's ERF. Before 2253fa78 (the table), after (Append).
- **Save As File… makes the file the table's home before the save succeeds**: table_view.rs:667-670. A write that fails leaves a module's table pointing at a file that isn't there; its next Save goes there again, not into the module.
- **A renamed column keeps its width only for the module's tables**: `rename_column_width(settings, &doc.name(), ..)` at table_view.rs:3114, while the widths are kept by `widths_key` (`cep_2da.hak/appearance`, a file's folder) at :1785 and :2083. In a hak's or a file's table the renamed column falls back to the default.
- **Hidden columns are told by `to_lowercase()` in three places and `to_ascii_lowercase()` in the one that draws** (table_view.rs:1837 against :1401, :1639, :2104): a column named with a capital outside ASCII is "hidden" (counted, passed over by the keys) and still drawn.
- **After an Undo that takes rows away (Add Rows, a paste past the end) the selection stays on a row that is gone**: the arrows, Enter and Delete do nothing until a click (`keys_in` returns at table_view.rs:1609); Empty Row and Delete Row stay enabled.
- **The typed text's log pairs texts with what was said by position** (`wanted.iter().zip(&said)` at talk_typed.rs:330-332), and `said` has no entry for a text kept in the file or one whose line failed: with two texts in one command, the log says the first "uses line N" when it was the second.
- **The hak editor's list is sorted with `sort_by_key(|&i| items[i].key.to_string())`** (hak_view.rs:233-240): for 20,000 resources some 300,000 `String`s for each letter typed into Find, and again by type. `sort_by_cached_key`, or sort by the key itself.
- **Once a second, on the window's thread, an open 2DA reads its whole source again and compares it** (`read_again`: a file's `std::fs::read`, a hak's `data`, the module's `to_vec`, table_view.rs:700-721), and looks for its description: for a hak built from a folder, a walk of that folder (`folder_files`, :686-690), else `families_in`, which lists the table's folder and reads and parses every `*.2da.toml` in it (mg-module/src/table_desc.rs:186-200). A module folder of 20,000 files, or a hak's source folder of that size, is a hitch a second while the tab is in front. Compare the file's time and size first, and look for the description on the thread that walks `Beside`.
- **The talk table's list is made anew every frame**: `(0..t.len()).collect()` and, with Only Lines with Text, a binary search a row (talk_view.rs:886-902); and `texted` and `found` are worked out again for every letter typed into a line (`v.revision += 1` at :1120). Fine at 20,000 lines; a table reaching into a range at line 1,000,000 (`Table::put` resizes to the row: mg-module/src/talk.rs:527-529) is not.
- **Export Resources asks every listed key of the selection twice a frame** (`all` and `any` over `listed`, transfer.rs:1134-1144): 25,000 `BTreeSet` lookups each.
- **Import clones the whole import to run it** (`Action::Import(draft.clone())`, transfer.rs:1318, then `app.import = None`): every incoming resource's bytes copied once; `take()` would do.

From the forks' reports (their reading):
- Callers of `Moonglow::apply` in search.rs:672-675, tag_clash.rs:346-347, copy_as.rs:151 and references.rs:320-323, :417-420, :651-654, :681-684 leave `claims.held` set, and the next real Apply error is swallowed; a command held for the Protected Area window runs later without its caller's follow-ups (Delete's closing of tabs, Copy's log and tag check).
- A search result or a reference selects its conversation line without opening its parents or scrolling to it (dialog_view.rs:2493-2497, references.rs:789-790); Paste and a drop under a folded line the same (:1039, :1436).
- `NumWords` is not kept up when a line's text changes (dialog_view.rs:1878-1884, :1906-1912, :2464-2483): the status line and the saved file keep the old count.
- `replace()` and its siblings in dialog_view.rs pass `g.to_bytes().ok()`: a write that failed would become "delete the conversation".
- The Copy window's Tag takes more than the 32 characters the game keeps (copy_as.rs:204, :116).
- Any rename points the References tab at the renamed resource, whatever it was showing (references.rs:335-339).
- A line a command has begun and not ended grows without bound and is scanned from its start after every read; progress written with `\r` never shows (project_commands.rs:339-343, :66-70).
- After an arrow key Module Properties' hak list keeps pulling the cursor's row back into view against the wheel (module_props.rs:402-404, :472-516).
- `search::choices` copies a table's whole choice list every frame (search.rs:735-739).
- The export's "files there already" question is a second copy of `widgets::files_already` (dialog_export.rs:171-225 against widgets.rs:3045-3098), with other buttons.
- `ps`, `tasklist` and `taskkill` are run with no time limit (test_module.rs:175, :192; project_commands.rs:379): `mg_module::process::output_within` is `pub(crate)`.
- (suspected) pipelines_kept.rs:88-97: opening module B after A in one run writes B's kept-pipelines file with A's list before a 3D view has drawn.

## Not read

- crates/mg-ui/src/dialog_view.rs:2766-3094: the `#[cfg(test)]` module (not under review).
- The diffs since 2253fa78 were not read as diffs: every file was read whole as it is on disk, and the hunks behind a finding dated with `git show 2253fa78:<file>` or `git log`.
- Of the forks' findings I did not re-read the code of 13, 17 and 26, nor the "from the forks' reports" lines above.
- Everything else of the part: read.

## Looked at and found sound

- **The 2DA editor's steps and their Undo** (`mg_module::table_edit`): cells, a paste that adds rows, rows taken, a row deleted, a column added, removed, moved or renamed are each taken back exactly by `back`; `put` refuses a quote before it touches the table, so no step is half made; `cell`, `set`, `clear_row`, `remove_row` and the grid's `get`s are bounds-checked, so a selection past the table's end never panics.
- **A look at the module's files on its own thread** (`outside.rs` with `mg_module::outside`): a result for another root is dropped; `stands()` drops a change whose known bytes were replaced since the look's snapshot, so a save between the look's start and its taking is not read back over itself; `take_found` compares the module's bytes now, not the snapshot's.
- **Typed text's lines and the module's Undo** (`talk_typed.rs`): the lines are kept by undo depth, `prune` lets go of what no longer goes with a step, a command that was held, viewed or failed has its lines taken back, and Redo refuses a row given another text.
- **Where files go**: `Hak::save` writes `.part` and renames, and the hak editor refuses the game's folder; `hak_edit::extract`, `transfer::export_files` and `browser::export` go through `ResKey::path_in`, a layer's folder through `layer_folder`; the claims table is written through a temporary file; imports refuse names that are no ResRef and say each.
- **From the forks**: a conversation's graph edits (delete with prune, copy, paste, move, link) each go through one workspace step, and the walks have visited sets; Find References' worker is tied to `Workspace::revision`, so a stale or another module's answer is never shown; what the project command question shows is what runs, and `stop()` ends the shell's process group.
