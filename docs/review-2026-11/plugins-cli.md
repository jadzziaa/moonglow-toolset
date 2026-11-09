---
type: Report
title: 'Code review of November 2026: the plugin host and the command line'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the plugin host and the command line (mg-plugin, apps/mg, apps/moonglow): 28 in all (2 high, 8 medium, 18 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, plugins, mg-plugin, command-line, mg]
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

# Code review of November 2026: the plugin host and the command line

One part of [the review of 9 November 2026](overview.md): the plugin host and the command line (mg-plugin, apps/mg, apps/moonglow). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## High

### C1. `mg test --dir` and `mg watch --serve --dir` write into the player's own NWN folder before the check that refuses it

- **Kind:** bug
- **Severity:** high (wrong file written, a module overwritten and its `.bak` deleted)
- **Where:** apps/mg/src/test.rs:245-248 (`create_dir_all(dir/modules)`, `m.save_as(Archive(copy))`, `remove_file(copy.mod.bak)`) before `serve::serve` at :258; apps/mg/src/main.rs:3034 and 3112-3121 (`Watcher::start` → `clear_written`, `build`) before `serve::serve` at :3160; the refusal is serve.rs:391-399
- **Since:** before 2253fa78 (test.rs, f53ec66a); the watch lines were last touched after (a7b39ecd)
- **Scenario:** `mg test ~/Projects/tdod --dir "$HOME/.local/share/Neverwinter Nights"` (to use the vault, NWNX settings or haks there). The scratch copy, with the test driver put in as OnModuleLoad and the `test_*` scripts added, is saved as `modules/tdod.mod` in that folder, over a module of that name (the project's own build, or the very `.mod` being tested if it lives there: "the module itself is not changed" no longer holds), the `.mod.bak` the save kept is deleted, and only then `serve` bails with "is your own NWN folder" (neither command has `--real-user-dir`, so it always bails). `mg watch P --serve --dir <that folder>` likewise builds `modules/<name>.mod` there, deletes its `.bak`, makes `development/` and removes what `.mg-watch` lists, then fails.
- **Fix:** make the user-folder and game-folder check a function of `serve` (`serve::refuse_dir(gi, dir, real_user_dir)`) and call it in `test::run_on` and `Cmd::Watch` before anything is created under `dir`; never delete a `.bak` of a file the run did not make.
- **Test:** `apps/mg/tests/test_cmd.rs`: `mg test M --dir USER` with `--user-dir USER` holding `modules/<name>.mod`: fails, and the file and folder listing are byte for byte as before. The same for `mg watch --serve --dir`.
- **Confidence:** confirmed by reading (`apps/mg/src/test.rs:241-258`, `apps/mg/src/serve.rs:389-402`, `apps/mg/src/main.rs:3019-3121` and `watch.rs:211-272`); not run
- **Read again by the reviewer:** holds: `create_dir_all`, `save_as` and `remove_file` of the `.bak` come before `serve::serve`, whose check is at serve.rs:391 (test.rs:245-258).

### C2. What a plugin makes the host hold is outside its 256 MB: one call takes a gigabyte, a loop takes all the memory

- **Kind:** bug
- **Severity:** high (security: the sandbox's memory limit is not the job's; the window is killed with the module open)
- **Where:** crates/mg-plugin/src/tables.rs:348-349 (`tlk:set(16777215, "x")` → `entries.resize(16_777_216, …)`, 1.07 GB a table, as many tables as `mg.tlk.new()` is called); tables.rs:99 (`add_row` in a loop); runtime.rs:157-160 (every `ctx.edit:write_bytes` of a 100 MB string is copied twice into `edits` and the workspace's undo, each kept to the job's end); files.rs:239-248 and kinds.rs:112-120 (`folder:write`/`out:write`: 64 MB a name, any number of names); runtime.rs:1954 (a module copy for each fix, see 4)
- **Since:** before 2253fa78 (5ed5cc77, f53ec66a); `out:write` after
- **Scenario:** `local t = mg.tlk.new(); t:set(16777215, "x")` in a check, an event handler or a panel (250 ms is enough): 1 GB of host memory in one call that the Luau allocator never sees. `while true do ctx.edit:write_bytes("x.txt", big) end` in a command (no time budget) ends with the system out of memory. The manual says "A job has 256 MB of memory; past that it fails" (17-plugin-api.md:758).
- **Fix:** charge host-side bytes to the job (`Shared` already counts `held_bytes` for haks): one budget for edits' data, folder files, table rows and talk lines, failing the call past it; refuse `tlk:set` more than a few thousand lines past the table's end; bound the number of fixes.
- **Test:** `crates/mg-plugin/tests`: a command that calls `mg.tlk.new():set(16777215, "x")`, and one that writes 5 × 100 MB, each fail with the memory message.
- **Confidence:** confirmed by reading (`tables.rs:340-353`, `runtime.rs:148-165`, `files.rs:209-250`, `kinds.rs:103-135`, `runtime.rs:1947-1958`; `TlkEntry` is 64 bytes, `mg-tlk/src/lib.rs:35`); not run
- **Read again by the reviewer:** holds for the talk table: `entries.resize(line + 1, ..)` for any line below 16,777,216 (tables.rs:343-349).

## Medium

### C3. `ctx.selection` copies the area's whole GIT (or ARE, twice) for each entry: a large selection stalls every command and times every panel out

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-plugin/src/runtime.rs:232-235, 902, 915-923, 1137-1156, 1690-1691
- **Since:** before 2253fa78 (c71028fb)
- **Scenario:** an area with 2,000 placed objects (a GIT of some megabytes), 200 of them selected with the rubber band (or Select All), a panel with `refresh = "selection"` docked. Each entry deep-copies the GIT (tens of thousands of fields, a few milliseconds), so `context()` takes 0.5 s and more before the handler runs: the panel's 250 ms budget is spent in the host and it shows "ran out of time" at every change of the selection; a right-click command waits as long. Tiles are worse: all 1,024 tiles of a 32×32 area selected copy the ARE 2,048 times.
- **Fix:** read under the borrow (`ws.doc(&key)` gives `&Gff`; `path.get(&doc.root)` needs no copy), or keep each resource read once in a map for the length of `context()`; start the deadline after `context()` is built.
- **Test:** unit test in `runtime.rs`: `context()` with 500 `Selected::Object` of one GIT of 2,000 placeables under a time bound; a plugin test `panel` with such a selection passes.
- **Confidence:** confirmed by reading (`runtime.rs:230-239` clones the document; `selected` calls it at :902 and :922; `context` at :1137 for every job; the window passes the selection to panels, `mg-ui/src/plugin_kinds.rs:495`); cost estimated, not measured

### C4. Each fix a check offers copies the module and makes a workspace of it, all kept until the check returns

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-plugin/src/runtime.rs:1954-1956, 2020
- **Since:** before 2253fa78 (d7dd0c22)
- **Scenario:** a tag-convention check on a project of 10,000 resources that finds 2,000 objects and gives each a fix (`ctx.fix("Upper-case this tag")`). Each fix that edits clones the module (10,000 map entries, 20,000 path allocations) and its saved map: about 2,000 × 2-3 MB held at once and 40 million allocations, for edits of one field each. Verify Module and `mg verify --plugins` both pay it.
- **Fix:** one scratch workspace for the check's fixes, each fix's edits applied and undone on it (or applied to a document-level copy of the one resource it touches).
- **Test:** `crates/mg-plugin/tests`: a check returning 2,000 fixes on a 5,000-resource module finishes under a time and memory bound (count `Module` clones through a test hook, or time it).
- **Confidence:** confirmed by reading (`runtime.rs:1947-1958`: `sh.ws.borrow().module.clone()` and `shared_in` → `Workspace::new`, which copies the resource map again, `mg-edit/src/lib.rs:192-194`; `Module` clones an `IndexMap` and, for a folder module, two maps with a `PathBuf` a file, `mg-module/src/folder.rs:17-22`)

### C5. `folder:files()` stops at 20,000 files without saying so, and which files it leaves out is the walk's order

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-plugin/src/files.rs:38, 73
- **Since:** before 2253fa78 (4c4cbbe0)
- **Scenario:** an importer given a folder of 30,000 textures (an unpacked content pack) gets a sorted list of 20,000 names, an arbitrary subset by `read_dir` order, with no error and no log line; the reference (17-plugin-api.md, `folder:files()`) says "the names of its files and of those in its folders". The import is silently partial. The user choosing their home folder also walks all of it with no way to cancel.
- **Fix:** fail past the limit ("the folder has more than 20,000 files"), or return them all; stop walking once the limit is reached; check `host.cancelled()` between folders.
- **Test:** unit test beside `a_folder_s_files_stay_inside_it` with `MAX_LISTED` made a parameter: a folder of limit + 1 files is an error, not a shorter list.
- **Confidence:** confirmed by reading (`files.rs:63-84`: the cap is applied while walking, before the sort; the walk goes on through every folder all the same)
- **Read again by the reviewer:** holds: the cap is tested as files are met, before the sort (files.rs:73).

### C6. `mg compile` exits 0 when scripts fail to compile

- **Kind:** bug
- **Severity:** medium
- **Where:** apps/mg/src/main.rs:2785-2803
- **Since:** before 2253fa78 (25cc2561)
- **Scenario:** `mg compile module.mod && scp module.mod server:` in a pipeline: three scripts have syntax errors, the errors are printed, the module is saved with their old (or no) compiled scripts, and the exit is 0, so the broken module is deployed. `--json` has `"failed": 3` and exit 0.
- **Fix:** `out.failed = failed > 0` (after the save and the note), and say so in the manual's row.
- **Test:** `apps/mg/tests/compile.rs`: a module with one script that does not compile: exit is a failure, the JSON still has `failed: 1` and `errors`.
- **Confidence:** confirmed by reading (`main.rs:2756-2804`: `failed` is counted for the note only, `out.failed` is never set; `compiled()` at :2164 sets nothing; `apps/mg/tests/compile.rs` asserts success only for clean compiles). The manual says only that `mg build` fails; it does not say `mg compile` succeeds
- **Read again by the reviewer:** holds: `failed` is read for the note only and `out.failed` is never set (main.rs:2785-2803).

### C7. `mg --json gff FILE --module M` puts `"notes"` among the GFF's fields

- **Kind:** bug
- **Severity:** medium
- **Where:** apps/mg/src/main.rs:1836-1840, 2365-2366, 5773
- **Since:** before 2253fa78 (2a265d3b)
- **Scenario:** `mg --json gff guard.utc --module castle.mod --names > guard.json` where a hak is not found, or the `.mod` has its module folder beside it (`open_module`'s note, said for every command). The file has `"notes": ["warning: …"]` beside `__data_type`; `mg gff guard.json -o guard.utc` then fails with "/notes: field must be an object", and `nwn_gff` reads it no better. The manual says `mg gff` prints the GFF's JSON "as it does without `--json`".
- **Fix:** for `mg gff` to standard output send the notes to standard error in both modes (an `Output` flag "the JSON is a document, not a report").
- **Test:** `apps/mg/tests/json.rs`: `--json gff X --module` on a `.mod` with its folder beside it: standard output reads back with `mg gff`, the note is on standard error.
- **Confidence:** confirmed by reading (`main.rs:5773` returns the GFF's JSON as the output's; `Output::print` at :1836-1840 inserts `notes` into any object; `mg_gff::from_json` refuses a root key that is no field object, `mg-gff/src/json.rs:158-164`)
- **Read again by the reviewer:** holds for the mechanism: `Output::print` inserts `notes` into any JSON object (main.rs:1836-1840).

### C8. Settings that do not read are replaced by the defaults without a word, and written over at the next save

- **Kind:** bug
- **Severity:** medium (the user's settings lost)
- **Where:** apps/moonglow/src/main.rs:501-504, 553-554, 177-179
- **Since:** before 2253fa78 (1597d837)
- **Scenario:** a newer Moonglow adds a variant to an option's enum and saves it; the user starts the older AppImage kept beside it (or rolls an update back), or `app.ron` is cut short by a crash. The window opens with every option at its default (keys, folders, recent modules, the plugins enabled and what their events were allowed, graphics failures recorded), nothing in the log says so, and thirty seconds later the old file is gone.
- **Fix:** where the file is there and the value does not read, keep the file aside (`app.ron.unreadable-<time>`), log an error naming it, and read what does parse field by field (deserialize to a RON map and take each field that reads).
- **Test:** unit test in `apps/moonglow` (or mg-ui's settings): a saved value with one unknown enum variant yields the other fields as saved and an error to log; a truncated file is kept aside.
- **Confidence:** confirmed by reading (`apps/moonglow/src/main.rs:553-554` `unwrap_or_default()`; `Settings` is `#[derive(Deserialize)] #[serde(default)]`, so one field of another type or an enum variant this build does not know fails the whole value; eframe's `get_value` only `log::warn`s; `App::save` at :177 writes the defaults back)
- **Read again by the reviewer:** holds: `eframe::get_value(..).unwrap_or_default()` (apps/moonglow/src/main.rs:553-554).

### C10. A project's plugin folder is read file by file, each whole, before the 16 MB limit is looked at

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-plugin/src/package.rs:503-510
- **Since:** before 2253fa78 (efef6741)
- **Scenario:** a checkout whose `plugins/exporter/` holds a 3 GB file (a sample export left there, or put there on purpose: the folder comes with the checkout and is read before anybody allowed it). Opening the project, and every `mg set`, `mg rename`, `mg areas` on it, reads 3 GB into memory to answer "the plugin is over 16 MB".
- **Fix:** sum `metadata().len()` over the walk and fail before reading anything; read each file with `take(left + 1)`, as `Package::unpack` does.
- **Test:** unit test in `package.rs`: a folder with a sparse 1 GB file fails without the file being read (a `files_of` that takes a reader hook, or a time bound).
- **Confidence:** confirmed by reading (`package.rs:501-511`: `std::fs::read` then `total > MAX_BYTES`; no `metadata().len()` first; called by `Plugin::load_project` ← `project::discover` ← every `save_guard`/`mg verify` in `apps/mg/src/plugins.rs:1401` and the window as a project opens)

### C11. `mg.image` holds a TGA or DDS to its pixel limit only after decoding it

- **Kind:** bug
- **Severity:** medium (a small string from untrusted code makes the host allocate gigabytes)
- **Where:** crates/mg-plugin/src/files.rs:362-371
- **Since:** before 2253fa78 (d624b9e8)
- **Scenario:** a plugin builds a run-length TGA of 16,384 × 16,384 (about 10 MB of packets) and calls `mg.image(s)`: `tga::read` makes 1 GB of RGBA, `to_rgba` and `top_down` copy it again, and then the call fails with "the picture is too large".
- **Fix:** read the header's size first (a `mg_image::size_of(kind, bytes)`) and refuse before decoding, as the PNG path does.
- **Test:** unit test in `files.rs`: a TGA header claiming 16,384² with a few bytes of data is refused as too large, not as truncated.
- **Confidence:** confirmed by reading (`files.rs:361-372`: decode, then `width × height > MAX_PIXELS`; `mg_image::check_size` allows 16,384 a side, `mg-image/src/lib.rs:86-92`; PNG is given the limit up front, :373)
- **Read again by the reviewer:** holds: a TGA or DDS is decoded before `width * height` is held to the limit (files.rs:361-370).

## Low

### C9. `mg build` that fails without `--json` drops every note (a hak not found, the compiler's warnings)

- **Kind:** bug
- **Severity:** low
- **Where:** apps/mg/src/main.rs:4065-4068
- **Since:** before 2253fa78 (2a265d3b)
- **Scenario:** a project whose include lives in a hak the machine lacks: "warning: hak tdod_core not found …" is a note; every script that includes it fails; `mg build` prints the compile errors and "31 of 400 scripts failed to compile", and not the one line that says why.
- **Fix:** set `out.failed = true`, add the count as a note and return `out` in both modes.
- **Test:** `apps/mg/tests/nasher.rs`: a project naming a missing hak and a script that fails: standard error has the hak warning.
- **Confidence:** confirmed by reading (`main.rs:4058-4069`: the text branch prints `out.text` itself and `bail!`s, so `out.notes` and the modules' notes never print; the `--json` branch returns `out` and keeps them)

### C12. A resource put into a hak with `folder:to_hak` is read whole, at any size, by `ctx.game:has` and `ctx.game:bytes`

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-plugin/src/runtime.rs:169-177, 184, 491
- **Since:** before 2253fa78 (4c4cbbe0)
- **Scenario:** an importer puts 2,000 textures of 4 MB into a hak with `folder:to_hak` (documented as "without reading it into the plugin's memory") and asks `ctx.game:has(name)` for each: 8 GB read from disk to answer yes 2,000 times. One 3 GB file put in and then asked for is read whole into host memory, past the 64 MB a plugin reads of a file.
- **Fix:** `in_haks` returns the `HakData` by reference; `has` answers from it; `bytes` applies `MAX_FILE` to a file's size before reading.
- **Test:** `crates/mg-plugin/tests`: `to_hak` of a file over 64 MB, then `ctx.game:has` is true and `ctx.game:bytes` fails with the size message.
- **Confidence:** confirmed by reading (`runtime.rs:169-177`: `in_haks` returns the bytes, `HakData::File(p) => std::fs::read(p)` with no `MAX_FILE`; `has` at :491 calls it for `.is_some()`; `Bytes` are copied with `to_vec`)

### C13. A ResRef field set from text goes through UTF-8 "lossy": a name with a letter outside ASCII is written as other bytes

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-plugin/src/runtime.rs:347-350
- **Since:** before 2253fa78 (5ed5cc77)
- **Scenario:** a module of a Polish or French team with a resource named `café_01`. `local g = ctx.module:gff("x.utc"); ctx.edit:set("x.utc", "Conversation", g.Conversation)` rewrites the field's bytes (`caf` + EF BF BD + `_01`): the reference no longer finds its resource, and the length is counted in the wrong bytes (a 15-letter name with one such letter is refused as too long).
- **Fix:** `ResRef::from_bytes(&bytes)`.
- **Test:** unit test of `typed(FieldType::ResRef, "café", …)` under Windows-1252: the value's bytes are `63 61 66 E9`.
- **Confidence:** confirmed by reading (`runtime.rs:346-351`: the codepage's bytes are turned back to a `String` with `from_utf8_lossy`, so byte E9 becomes EF BF BD, then `ResRef::from_str`; `plain()` at :262 reads the same field by the codepage, so read and write disagree)
- **Read again by the reviewer:** holds: the codepage's bytes go through `String::from_utf8_lossy` before `ResRef::from_str` (runtime.rs:347-349).

### C14. A plugin may give a new resource a name no file can have, and a folder module then cannot be saved

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-plugin/src/runtime.rs:129-134, 719-748
- **Since:** before 2253fa78 (5ed5cc77)
- **Scenario:** `ctx.edit:write("a/b.nss", "…")` (a generator joining a group and a name). The command applies; the next Save of the folder module or nasher project fails for the whole module with "the name is not a file's", until the user finds the plugin's step and undoes it.
- **Fix:** where the key is not yet in the module, hold the name to `ResRef::is_file_name` (or the new-name rule `ctx.hak:write` already uses) in the three writers.
- **Test:** `crates/mg-plugin/tests`: `ctx.edit:write("a/b.nss", "x")` fails naming the name; an existing oddly named resource of an archive can still be read and written.
- **Confidence:** confirmed by reading (`runtime.rs:129-134` `key_of` is `ResKey::from_filename`, any 16 bytes but NUL; `ctx.edit:write`, `write_bytes`, `write_gff` make the resource; a folder's or a project's save goes through `path_in`, which refuses `/`, `\`, `:` and control characters, `mg-module/src/lib.rs:705,778`); what the window then shows was not read

### C15. `mg plugin check` and `mg plugin pack` run a plugin's top-level code, an archive's too, with no time limit

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-plugin/src/runtime.rs:2099-2100; apps/mg/src/plugins.rs:584, 611, 766
- **Since:** after (af291f40 touched the callers; `inspect` never had a budget)
- **Scenario:** `mg plugin check downloaded.zip` ("checked as the installer checks it") on an archive whose `main.luau` is `while true do end`: it never returns; in a pipeline, until the pipeline's own timeout.
- **Fix:** give `inspect` a budget (the events' 5 seconds), reported as "it ran out of time".
- **Test:** `apps/mg/tests`: `mg plugin check` of such a plugin fails within the budget.
- **Confidence:** confirmed by reading (`runtime.rs:2094-2131`: `inspect` makes its `Input` with no budget; `CliHost::cancelled` is the default `false`; `plugins.rs:584, 611, 766`)

### C16. `module_codepage` builds the game's resource manager and opens the haks again, in commands that have one already

- **Kind:** performance
- **Severity:** low
- **Where:** apps/mg/src/main.rs:4809; twice in `mg areas` (:3731, :3790, besides `game_data_if_any` at :3650); `mg minimap` :4287-4288 (has `rm`; `mg nwsync` at :4581 does it right with `mg_rules::text_codepage(&rm, …)`); `mg find` :4434-4436; `mg search` search.rs:321, 345; `mg set`/`apply` :2371-2377 with `save_guard` and `keep_palettes` each building game data again
- **Since:** before 2253fa78 (f3365573)
- **Scenario:** a module with 40 haks of 30,000 resources each: every build reads the key files, lists the override folders and reads 40 hak indexes. `mg areas M --set …` on a project with a save-event plugin does it five times (codepage twice, game data for the listing, for the plugins' guard, for the palettes); a script that runs `mg set` over 500 blueprints pays it 2,000 times.
- **Fix:** one lazily built `ResMan`/`GameData` per run (a `OnceCell` beside `Cli`), with `text_codepage(&rm, …)` or `game.text_codepage()` read from it; `verify` computes the same codepage twice too (:5269, :5294).
- **Test:** none cheap; a counter of `ResMan::for_game` calls in a command-line test of `mg areas` (1).
- **Confidence:** confirmed by reading (`main.rs:4809-4816`: `ResMan::for_game` + `add_haks` for one `encoding.2da` lookup; 20 call sites); the cost of one build was not measured

### C17. `mg attach` copies the haks into the user folder before it has opened the module

- **Kind:** bug
- **Severity:** low
- **Where:** apps/mg/src/main.rs:4260-4262
- **Since:** before 2253fa78 (d54220d5)
- **Scenario:** `mg attach castel.mod a.hak b.hak --replace` with the module's name mistyped: both haks are copied into (and with `--replace` over those in) the user's `hak` folder, then the command fails with "no such file"; nothing says what was copied.
- **Fix:** open the module and read its IFO first; copy last, just before the save.
- **Test:** `apps/mg/tests`: attach to a missing module leaves the scratch user folder's `hak` empty.
- **Confidence:** confirmed by reading (`main.rs:4247-4261`: `copy(&placed, replace)` then `open_module(module)?`)

### C18. `mg bake-walkmesh` writes the hak and the rows file before the plugins are asked and the edits applied

- **Kind:** bug
- **Severity:** low
- **Where:** apps/mg/src/bake.rs:140, 144, 149
- **Since:** after (9aab919c)
- **Scenario:** a project plugin refuses the save of the GIT (or the module's save fails): the baked model is in the hak and its row in the rows file, the placeables still name their old rows, and the command says "not saved: a plugin refuses it".
- **Fix:** apply the edits and ask the guard with a dry run first, or write the hak through `HakWrite::write_kept` and take it and the rows file back on failure.
- **Test:** `apps/mg/tests`: bake with a `--plugins` plugin that refuses: the hak and rows file are as before.
- **Confidence:** confirmed by reading (`bake.rs:132-147` writes; `edits::run` at :149 then runs `guard.before` (a refusal, or `--strict`, is an error) and the save; `plugins.rs:1092-1160` takes its haks back in the same case, this does not)

### C19. `mg ls` and `mg unpack` read the whole archive into memory, and `mg pack` holds every file before it writes (its comment says "streamed")

- **Kind:** performance
- **Severity:** low
- **Where:** apps/mg/src/main.rs:2214, 2227, 2297-2303
- **Since:** before 2253fa78 (bootstrap; 9f4bc8dc)
- **Scenario:** `mg ls tdod_tiles.hak` on a 2 GB hak reads 2 GB to print names; `mg pack` of a 3 GB folder needs 3 GB of memory. `mg pack` also truncates the archive it replaces as it starts (`File::create`), so a failure half way leaves no hak.
- **Fix:** list and unpack from `ErfContainer` (or a mapped file); pack from paths, each read as it is written, into a temporary file renamed over the archive.
- **Test:** unit-level: `ls` through a reader that counts bytes read stays near the header's size.
- **Confidence:** confirmed by reading (`main.rs:2214, 2227` `std::fs::read(archive)`; :2292-2303: each file read and given to `ErfWriter::add`, which keeps it, `mg-erf/src/write.rs:64`; `ErfContainer::open` maps the file instead, `mg-resman/src/container.rs:186-189`)

### C20. Files written in place where the rule asks for a temporary file and a rename

- **Kind:** hygiene
- **Severity:** low
- **Where:** crates/mg-plugin/src/lib.rs:679 (`FolderWrite::write`: a plugin's files over the user's of the same names; the check at :670-676 and the write are also two steps a link can come between); apps/mg/src/main.rs:4081 (`mg build` writes the target `.mod` with `std::fs::write`, where `mg watch` and the window go through `save_as`); apps/mg/src/main.rs:2302 (`mg pack`); apps/mg/src/tlk_alloc.rs:228-236 (the table is saved, then the map: a failure between leaves lines whose keys the map does not know, and the next run gives those keys new lines)
- **Since:** before 2253fa78
- **Scenario:** `mg build` interrupted (Ctrl+C, a full disk) leaves half a module where the last good build was, which the server then fails to load. `mg tlk-alloc` stopped between its two writes renumbers strings on its next run.
- **Fix:** a shared `write_whole(path, bytes)` (temporary file beside it, rename) for these; in `tlk-alloc` write the map first.
- **Test:** unit test of the helper; `tlk-alloc`: a map path that cannot be written leaves the table unwritten.
- **Confidence:** confirmed by reading

### C21. Which `mg` commands ask the project's plugins before they save has drifted: `rename X.jui`, `dialog-import`, `compile`, `attach` and `mg nui` do not

- **Kind:** duplication
- **Severity:** low
- **Where:** apps/mg/src/main.rs:4845 (the one to keep) and the six saves above
- **Since:** before 2253fa78
- **Scenario:** a project's `resource-saving` plugin that refuses a conversation without an end node: `mg apply` of the same DLG is refused, `mg dialog-import` writes it.
- **Fix:** every command that changes resources saves through `save_asked` (or a `Workspace` flavor of it); the manual lists them.
- **Test:** one command-line test per command with a refusing plugin in the project's `plugins/`.
- **Confidence:** confirmed by reading (asked: `save_asked` in rename, replace, areas, update-instances, import, `edits::run`, `make_fixes`; not asked: `main.rs:2783` compile, :3490 dialog-import, :4274 attach, `nui.rs:264` (the `.jui` branch of `mg rename`, where the `.nss` branch asks), `nui.rs:328`, `nui/import.rs:62`). The manual names only `mg set` and `mg apply` (15-plugins.md:423-426)

### C22. Numbers that are not finite pass as a form's values and reach the plugin as a null userdata

- **Kind:** bug
- **Severity:** low
- **Where:** apps/mg/src/plugins.rs:123-134; crates/mg-plugin/src/runtime.rs:343, 790-793; crates/mg-plugin/src/files.rs:393
- **Since:** after for the command line's number field (607e4937); before for the rest
- **Scenario:** `mg plugin run M P cmd --answer markup=nan`: the plugin's `answers.markup * 2` fails with "attempt to perform arithmetic on a userdata value" instead of `--answer markup: "nan" is not a number`. `ctx.edit:set(k, "Weight", 1e300)` writes `inf` into the GFF.
- **Fix:** `is_finite()` where a number is parsed or taken (`form_field`, `CliHost::values`, `typed` for Float, `pixel`); refuse `min > max`.
- **Test:** unit tests: `--answer x=nan` is a fault; a form field with `default = 0/0` fails; `typed(Float, 1e300)` fails.
- **Confidence:** confirmed by reading (`apps/mg/src/plugins.rs:123-134`: `"nan".parse::<f64>()` is `Ok`, both range tests are false for NaN, `json!(NaN)` is `null`; `runtime.rs:790-793` takes any `default`, `min`, `max` (NaN, min above max); `lua.to_value(null)` is mlua's null userdata, not nil; `files.rs:393` lets `pixel(0/0, 0/0)` through as pixel (0, 0); `runtime.rs:343` makes a finite double past f32's range an infinite Float field)

### C23. `mg plugin tidy` takes a value out of the project's file before the user's file is written

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-plugin/src/settings.rs:360-368; apps/mg/src/plugins.rs:339-347
- **Since:** not checked
- **Scenario:** two plugins tidied; the first has a `user` setting kept in `moonglow.toml` (moved: removed there, held in memory), the second's tidy fails (a project file that cannot be written): the command ends, the user's file is never written, and the first plugin's value is in neither place.
- **Fix:** write the user's file after each plugin whose tidy changed it, before the next project write; or collect the project's changes and write them last.
- **Test:** unit test of `plugins::tidy` with a second manifest whose tidy errors: the moved value is in the user file.
- **Confidence:** confirmed by reading (`settings.rs:360-368`: `take_out` writes the project's file at once, the moved value goes into `user` in memory; `apps/mg/src/plugins.rs:336-348` writes `user` after every plugin, and any `?` before it returns)

### C24. `mg tlk-import --json` says `"saved": true` when nothing was written

- **Kind:** bug
- **Severity:** low
- **Where:** apps/mg/src/main.rs:3449
- **Since:** before 2253fa78 (9be02317)
- **Scenario:** an import that changes no line of an existing table reports `changed: 0, added: 0, saved: true`; a pipeline that commits when `saved` commits nothing, or re-signs a file that was not touched.
- **Fix:** report what the condition at :3441 decided.
- **Test:** `apps/mg/tests/json.rs`: importing a table's own export gives `saved: false`.
- **Confidence:** confirmed by reading (`main.rs:3441-3449`: saved only if something changed or the table is new; the JSON says `!dry_run`)

### C25. `mg update-instances` looks each blueprint up in a list of every blueprint

- **Kind:** performance
- **Severity:** low
- **Where:** apps/mg/src/main.rs:3986
- **Since:** after (f9512481)
- **Scenario:** a module of 8,000 blueprints and 60,000 placed objects and held items: 480 million key comparisons, seconds of the run.
- **Fix:** a `HashSet<ResKey>`.
- **Test:** none needed beyond the existing ones.
- **Confidence:** confirmed by reading (`main.rs:3950-3954` `wanted: Vec<ResKey>` of every blueprint when none is named; `wanted.contains(&k)` at :3986 for each object and each item held)

### C26. `mg nui render FILE.jui` reads pictures by joining the folder and names the file gives

- **Kind:** hygiene
- **Severity:** low
- **Where:** apps/mg/src/nui/preview.rs:225
- **Since:** before 2253fa78 (edee3cff)
- **Scenario:** a downloaded `.jui` naming the picture `../../x`: a `.tga`/`.png` two folders up is read and drawn into the PNG. Read only, 16 characters, and it must decode as a picture.
- **Fix:** `key.path_in(dir)`, skipping a name it refuses.
- **Test:** unit test of `from_file` with such a name: nothing outside the folder is in the module made.
- **Confidence:** confirmed by reading (`nui/preview.rs:222-227`: `dir.join(key.to_string())` for each name of `mg_nui::image_names`; `ResKey::parse` takes `/`; the house rule is `path_in`)

### C27. Nothing keeps a second Moonglow from sharing the first one's settings file and recovery folder

- **Kind:** bug
- **Severity:** low
- **Where:** apps/moonglow/src/main.rs:500-504, 545, 587, 621
- **Since:** before 2253fa78
- **Scenario:** `moonglow a.mod` twice (a file manager's double click on two modules). An option changed in the first window is written back to what the second holds when the second saves; and the second window, started while the first has an autosaved recovery copy, may offer that copy as a crashed session's.
- **Fix:** a lock file in the data folder: a second run hands its module argument to the first (or says one is running), or at least reads settings again before writing and skips recovery copies whose owner holds the lock.
- **Test:** mg-ui recovery test: a copy whose owner's lock is held is not listed.
- **Confidence:** suspected. Confirmed by reading that `main` has no single-instance check (`apps/moonglow/src/main.rs:489-649`), that each run holds `Settings` in memory and eframe writes `app.ron` whole (every 30 s and at close), and that every run calls `find_recoveries` on one folder (:587, :621). What would settle the second half: read `mg-ui/src/recovery.rs` `find` and the autosave for a mark of a live owner (none was found by searching for a process id)

### C28. `mg claims claim A B C` writes area by area, and an error on the third hides that two were claimed

- **Kind:** bug
- **Severity:** low
- **Where:** apps/mg/src/claims.rs:96-97
- **Since:** not checked
- **Scenario:** C is held by another builder: the table now has A and B claimed, the command prints only the error about C and exits with a failure; with `--json`, `{"error": …}`.
- **Fix:** check every area first (a dry pass of `write_change`), or say in the error what was written.
- **Test:** `apps/mg/tests`: claim of a free and a held area leaves the table as it was.
- **Confidence:** confirmed by reading (`claims.rs:90-98`: `write_change(…)?` in the loop; the lines for those done are in `out`, which the error discards)

## Not read

- The `#[cfg(test)]` modules (not under review): `manifest.rs:1346-1694`, `watch.rs:499-712`, and those at the ends of `settings.rs`, `files.rs`, `kinds.rs`, `serve.rs`, `lsp.rs`, `moonglow/main.rs` were read only in passing.
- `docs/manual/16-writing-plugins.md` was only searched, and `17-plugin-api.md` read at lines 236-300, 600-700 and 752-780 and searched elsewhere: the reference was not held to the code function by function (the two mismatches found are in findings 2 and 5).
- Everything else of the three folders was read whole, `apps/mg/src/main.rs` included (its clap definitions at 31-1545 at reading speed).

## Looked at and found sound

- The sandbox's reach: every chunk is loaded with `ChunkMode::Text` (mlua 0.12.2's own `loadstring` too), mlua's file-system `require` is replaced before `sandbox(true)`, `require` takes only names inside the plugin's folder (or the frozen copy), a cancel or time-out is raised from the interrupt and cannot be kept by `pcall`, and `lua.from_value` stops at 128 levels (`mlua/src/serde/mod.rs:15`), so a deep or cyclic table is an error, not a stack overflow.
- The archive reader (`package.rs`): absolute and `..` names, links, encrypted entries, device names, names equal but for case, a file where another's folder is, and the limits held by bytes read rather than sizes declared; install is staged in a hidden folder and renamed in.
- A project's plugins (`project.rs`, `lib.rs` `Frozen`): the digest is of the files that run, the job runs from that copy and not the folder, a no is the project's and a yes the content's; answers and user settings are written through a `.part` file and renamed.
- `mg verify`: the exit follows the errors left after `--fix`, an unknown `--fix` check is an error, fixes go through `Guard` and the plugins' save guard, and the error path of `main` prints one JSON object with `--json`.
