---
type: Report
title: 'Code review of November 2026: opening, saving and the project''s files'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in opening, saving and the project''s files (mg-module): 23 in all (2 high, 5 medium, 16 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-module, saving, nasher, project-file]
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

# Code review of November 2026: opening, saving and the project's files

One part of [the review of 9 November 2026](overview.md): opening, saving and the project's files (mg-module). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## High

### P1. Unsaved `module.ifo` edits are counted as read when a pull adds an area, and are never written

- **Kind:** bug
- **Severity:** high (data loss, silent)
- **Where:** crates/mg-module/src/lib.rs:891-902; crates/mg-module/src/nasher/project.rs:842-879 (lines 875-878); caller crates/mg-ui/src/outside.rs:580-585
- **Since:** after (7ed4ce1f "The area list settled after a pull goes through the workspace")
- **Scenario:** a nasher project is open; the builder changes Module Properties (a hak, an event script, a module variable) and has not saved; a `git pull` (or a copy by hand) adds `x.are.json`/`x.git.json` that `module.ifo.json` does not list (the usual case: the list is only settled in memory). `take_outside` takes the area and calls `area_list_settled`, which hands `settle_area_list` the LIVE `module.ifo` (unsaved edits in it), appends the area, and sets `sources[module.ifo].resource` to those bytes ("counts as read"); `Workspace::adopt` then puts the same bytes in the module and in `saved`. At Save, `Project::save` finds `src.resource == bytes` and skips `module.ifo`; the title says saved; the edits are gone at the next open. `settle_area_list` was written for the open, where the module is the file.
- **Fix:** in `area_list_settled`, count as read only what the file stood for: settle the area list on `project.as_read(info)` for `sources.resource`, and on the live IFO for what the workspace takes (or leave `sources.resource` alone when the live IFO differs from it).
- **Test:** unit test beside `a_look_apart_hands_back_only_what_still_stands`: open a project, `m.set_info` with a changed tag, write a new area's files, `outside` + `took` + `area_list_settled` + set the returned IFO, `save`, reopen: the tag is the changed one.
- **Confidence:** confirmed by reading (`Module::area_list_settled`, `Project::settle_area_list`, `Project::save`, `mg-ui/src/outside.rs` `take_outside`, `mg_edit::Workspace::adopt`/`save`)
- **Read again by the reviewer:** holds for the mechanism: `settle_area_list` sets `sources[module.ifo].resource` to the bytes it was handed (project.rs:875-878), `area_list_settled` hands it the module's own (lib.rs:891-902), and `save` skips a resource equal to it (project.rs:938).

### P2. The external compiler's scratch files are named by raw resrefs: a script named `../../x` is written outside the scratch folder

- **Kind:** bug
- **Severity:** high (a file written where it mustn't be; limited to `*.nss`, at most five folders up, and to builders who set an external compiler)
- **Where:** crates/mg-module/src/external_compiler.rs:148-152; crates/mg-module/src/build.rs:705-711, 724-727
- **Since:** before 2253fa78
- **Scenario:** a downloaded `.mod` holds a script whose resref is `../../../../../x` (16 bytes) or `..\..\x`; with Options > Script Editor naming an external compiler, Compile (or the compile a project's open does) writes its text to `<temp>/moonglow-compile-PID-N/src/../../../../../x.nss`: on Windows that is `C:\Users\<name>\x.nss`, with the attacker's bytes. The house rule (a file named after a resource goes through `ResKey::path_in`) is not followed here.
- **Fix:** build both paths with `ResKey::new(resref, NSS).path_in(&src)` and say the script cannot be compiled where that refuses.
- **Test:** unit test in external_compiler.rs: `compile(&[("../evil".into(), b"x")], &[])` with a compiler of `true` writes nothing outside the scratch folder (or returns an error naming the script).
- **Confidence:** confirmed by reading (`build::external_sources`/`compile_externally` give `resref.to_lowercase().to_string()`; `ResRef` takes any 16 bytes but NUL and `Display` writes them as they are; `ExternalCompiler::compile` joins them)
- **Read again by the reviewer:** holds: `src.join(format!("{name}.nss"))` with the name a ResRef's text (external_compiler.rs:149; build.rs:705, :724).

## Medium

### P3. A Save As into a nasher project that fails leaves the module holding the new project: from a project, every later Save fails

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-module/src/lib.rs:501-513
- **Since:** before 2253fa78
- **Scenario:** `self.project = Some(new)` is set before `project.save(&self.resources)?`. When that fails (a GFF of a `.mod` that does not read, an NWNT label with a dot, a write refused, a full disk), `location`, `folder` and `archive` stay the old ones and `project` is the new, empty one. Opened from project A: the next Save is `save_as(Project A)`, `same` is false, `Project::create(A)` opens A, finds its module and returns "the project already has a module; open it instead", at every Save until Save As elsewhere succeeds. Opened from a `.mod` or folder: until the next save the module passes for a project (`outside_scan`, `file_of`, `as_read`, `write_made`, Pack and the compile rules all follow `project`).
- **Fix:** make the project in a local, `save` into it, and store it in `self.project` only with the location, after it succeeded.
- **Test:** unit test: a module opened from project A, `save_as` to a project root whose `src` is a file (so the write fails), then `save()` succeeds and writes A.
- **Confidence:** confirmed by reading (`Module::save_as`, `Project::create`, `Project::save`; `Workspace::save_as` saves the live module, no copy)

### P4. A talk-table import with one large line number adds millions of lines, each its own undo step

- **Kind:** bug
- **Severity:** medium (a freeze and gigabytes of memory from one cell of a CSV or one `id` of a JSON)
- **Where:** crates/mg-module/src/talk.rs:822-825 and 885-888 (the only bound: below 16,777,216), 934-937 (the loop)
- **Since:** before 2253fa78
- **Scenario:** a translator's sheet has a row `9000000,Hello` (a StrRef mistyped, or one of another numbering), or a JSON `"id": 9000000`. `import` runs `while self.len() < row { self.add_line(&empty)? }`: nine million `Change`s (four `Option<TlkEntry>` each, about 300 bytes) are pushed on the undo stack, about 2.7 GB, plus the entries. The same happens, smaller, in the intended case: a project whose `[talk] lines` begin at 1,000,000 and a first import into an empty table (a million undo steps). `put_line`/`put_typed` reach a far row in one step by `resize`.
- **Fix:** reach the first row past the end with one change (as `put` does for `put_line`: the gap is empty lines, undone by truncating to the length before), and refuse or ask about a gap past some bound.
- **Test:** unit test: `import_csv("StrRef,Text\n500000,x\n")` on an empty table leaves `undo.len() == 1`, `len() == 500001`, and one `undo()` empties it.
- **Confidence:** confirmed by reading (`import_csv`, `import_json`, `import`, `add_line`, `apply`)

### P5. The hak editor cannot read what lies past 2 GiB of the hak it has open: Extract and Save fail on the hak it tells the builder to split

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-module/src/hak_edit.rs:580-585 (and 592-606, 647-656); the refusal is crates/mg-resman/src/container.rs:224
- **Since:** before 2253fa78
- **Scenario:** a hak of 3 GiB (made by nwhak, or saved by Moonglow with the warning) is opened; the builder selects the resources past the mark and extracts them to put them in a second hak: "starts past 2 GiB, where the game stops reading". Removing the first half and saving fails the same way. Only removing the tail can be saved, which throws those resources away.
- **Fix:** give `ErfContainer` a read without the game's limit (the offset is a `u32`: all of it is addressable) and use it in `Hak::read`; the limit stays for the load order.
- **Test:** mg-module unit test with a sparse file: a hak whose second entry's offset is past 2^31 (header written by hand over a sparse file), `Hak::open`, `extract` of that entry succeeds.
- **Confidence:** confirmed by reading (`Hak::read` goes through `ErfContainer::read`, which refuses an entry whose offset is at or past `ERF_READ_LIMIT`, the game's limit, not the file's; `Hak::save` and `Hak::extract` read every kept item that way; `hak_view` lets such a hak be saved and says "split it into two")

### P6. Add Left-Out Files writes its `include` beside the first line of that pattern in `nasher.cfg`, which may be another target's

- **Kind:** bug
- **Severity:** medium (the builder's `nasher.cfg` is edited in the wrong place, and the command reports success)
- **Where:** crates/mg-module/src/nasher/project.rs:482-498
- **Since:** before 2253fa78
- **Scenario:** two targets each with `include = "src/**/*.{nss,json}"` in their own `[target.sources]` (a module and a demo of it); Moonglow edits the second. A window's `.jui` is left out; the command finds the pattern's first line in the file, which is the first target's, and inserts `include = "src/**/*.jui"` there: the other target now packs the files, the edited one still leaves them out, `left_out` is unchanged and the warning stays; a second click adds the line again.
- **Fix:** look for the line within the edited target's block (as `exclude_in_target` finds a target's lines), falling back to `[package.sources]` only where the target inherits its includes; refuse when `left_out()` is unchanged after the write.
- **Test:** unit test: a cfg with two targets of the same include, project opened on the second, `take_in_left_out` leaves `left_out()` empty and the first target's block untouched.
- **Confidence:** confirmed by reading (`take_in_left_out`; `Target::inherit` copies lists whole, so two targets commonly carry the same pattern)

### P7. `messages` cuts a compiler's line one byte after a character that may be longer: a panic on the compiler's output

- **Kind:** bug
- **Severity:** medium (crash)
- **Where:** crates/mg-module/src/external_compiler.rs:278-287 (`rfind(..).map_or(0, |p| p + 1)`, then `line[from..]`)
- **Since:** before 2253fa78
- **Scenario:** the character before the run of letters, digits and `_` that precedes `.nss(` is not ASCII: a script whose resref has a byte above 0x7F (written as a two-byte character in the scratch file's name, which the compiler prints back: `src/¿aba.nss(4): ERROR`), or a compiler that quotes the name in typographic quotes. `p + 1` is inside that character and `line[from..]` panics.
- **Fix:** `rfind` over `char_indices` and add the character's `len_utf8()`.
- **Test:** add to the file's tests: `messages("src/\u{bf}aba.nss(4): ERROR: x\n", &["\u{bf}aba".into()])` returns the line and does not panic.
- **Confidence:** confirmed by reading
- **Read again by the reviewer:** holds: `rfind(..).map_or(0, |p| p + 1)` and then `line[from..]` (external_compiler.rs:278-287).

## Low

### P8. Three more slices by byte count on text that can hold characters of two bytes

- **Kind:** bug
- **Severity:** low (crash, on a resref with a byte above 0x7F)
- **Where:** crates/mg-module/src/palette_add.rs:64 and :136 (`&stem[..stem.len().min(13)]`, Add to Palette and Edit Copy); crates/mg-module/src/testing.rs:38 (`&name[first.len()..name.len() - last.len()]`, taken before the `ends_with` check)
- **Since:** before 2253fa78
- **Scenario:** Add to Palette on an object whose `TemplateResRef` is `abcdefghijkl\xE9x`: byte 13 is inside the `é`. `mg test 'test_*x'` on a module with a script `test_\xE9`.
- **Fix:** cut at `char_indices().nth(13)`; in `matches`, test `ends_with(last)` first and slice by `strip_prefix`/`strip_suffix`.
- **Test:** unit tests: `new_resref("abcdefghijkl\u{e9}x", &|_| false)` and `matches(Some("test_*x"), "test_\u{e9}")` return without panic.
- **Confidence:** confirmed by reading (`ResRef`'s `Display` writes each byte as a `char`, two bytes of UTF-8 from 0x80 up)

### P9. `evaluate` recurses once (or seven times) a token with no bound: a script's constant of many brackets overflows the stack

- **Kind:** bug
- **Severity:** low (a crash, not a panic, on a crafted script)
- **Where:** crates/mg-module/src/table_desc.rs:526-570, reached from :445
- **Since:** before 2253fa78
- **Scenario:** a downloaded module carries `nwscript.nss` with `int X = ((((... 4,000 deep ...` and a `spells.2da`; the built-in description of `spells` names `nwscript`'s constants, so opening the table (or Verify, on a worker's 2 MB stack) reads the module's own `nwscript` and dies of a stack overflow. A `variables.toml` with `constants =` reaches it the same way.
- **Fix:** carry a depth through `level` and return `None` past some 64.
- **Test:** unit test: `evaluate(&"(".repeat(100_000), &[])` is `None`.
- **Confidence:** confirmed by reading (`level` calls itself for every `(` through six levels and for every unary `-`/`~`/`+`; `mg_script::outline` keeps a global's whole initializer as `value`; `constants_in` evaluates every global with a value before it filters by prefix)

### P10. A `nasher.cfg` that is not UTF-8 throughout cannot be opened, though the parser was written for bytes

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-module/src/nasher/cfg.rs:499 (and the two rewriters, nasher/project.rs:480 and :572)
- **Since:** before 2253fa78
- **Scenario:** a `description = "Módulo de pruebas"` or a comment with an accent, saved by a Windows editor as Windows-1252: nasher packs the project; Moonglow says "nasher.cfg: stream did not contain valid UTF-8" and opens nothing.
- **Fix:** read bytes; parse them as they are (the parser already does), and have the two rewriters splice lines into the bytes rather than a `String`.
- **Test:** `Package::read` of a cfg with a 0xF3 byte in a description returns the target.
- **Confidence:** confirmed by reading (`Package::read` uses `read_to_string`; `parse_events` works on bytes, takes bytes above 0x7F in symbols and ends in `from_utf8_lossy`); that nasher opens such a file is from Nim's parsecfg reading bytes, not tried

### P11. The TOML rewriters read a file lossily and write the result back: bytes that are not UTF-8 become U+FFFD for good

- **Kind:** bug (a drift: `claims::decode` was written for exactly this, "José" written back as "Jos\u{fffd}")
- **Severity:** low
- **Where:** crates/mg-module/src/project_plugins.rs:254-259 (`document`, used by `set_setting` and `ProjectFile::set_used`); crates/mg-module/src/project_file.rs:1730-1731 (`set_builder_code`), :1815-1816 (`set_talk_lines`)
- **Since:** before 2253fa78 (`document`); after (f0dec956, `set_talk_lines`)
- **Scenario:** `moonglow.toml` has a comment or a label typed in Windows-1252. Reading it lossily gives no warning; Mark as Used, a plugin's setting or Lines for Typed Text then writes the whole file with every such byte replaced.
- **Fix:** refuse to rewrite a file that is not UTF-8 ("mend it first", as for a file that is no TOML), and say so as a warning when it is read.
- **Test:** unit test: `set_used` on a file holding a 0xE9 byte in a comment returns an error and leaves the file's bytes as they were.
- **Confidence:** confirmed by reading

### P12. Paths the project file names are joined unchecked; Claim This Area writes at one of them

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-module/src/claims.rs:76-78 (written through :372-389); crates/mg-module/src/talk_range.rs:149-151; crates/mg-module/src/palette_groups.rs:291-293, :396
- **Since:** before 2253fa78
- **Scenario:** a checkout's `moonglow.toml` says `[claims] file = "../../../Documents/x.2da"` or an absolute path. `project_plugins::folder_problem` refuses just that for `[plugins] folder`, and the file takes care that a checkout cannot allow its own commands; here nothing checks. An existing file that is no 2DA is refused by `Table::read`, so the reach is making a new file (a 2DA of claims) anywhere the builder can write, and reading TOML/JSON from anywhere.
- **Fix:** hold these four to `folder_problem`'s rule when the section is read, with the warning by line.
- **Test:** `read("[claims]\nfile = \"../x.2da\"\n")` has a warning and `claims.file` is `None`.
- **Confidence:** confirmed by reading (`ClaimRules::path_in`, `claims::rewrite`); the same join is in `BakeRules::rows_path` (bake.rs:172, another reader's), `TalkRules::map_path` and `Groupings::from_paths`/`read_lists`, which are read

### P13. `free_lines` subtracts overlapping builders' parts twice and underflows

- **Kind:** bug
- **Severity:** low (a nonsense count in Module Properties and Verify; a panic in a debug build)
- **Where:** crates/mg-module/src/talk_range.rs:228-236
- **Since:** after (f0dec956)
- **Scenario:** `lines = "100000-149999"`, `ab = "100000-129999"`, `cd = "120000-149999"`, a builder with no part of their own: `taken` is 60,000 of a tail of 50,000, `tail.len() - taken` wraps to four billion.
- **Fix:** count the union of the `but` ranges within the tail (sort and merge), or `saturating_sub`.
- **Test:** add the case to `free_lines`' test: the result is `Some((0, 0))`-like, not a wrapped number.
- **Confidence:** confirmed by reading (`TalkRules::allowed` passes every builder's part that overlaps `lines` as `but`; overlapping parts are a warning of `problems()`, not refused; release builds have no overflow checks)

### P14. A project's save passes over a resource whose name is no file's without a word; an ERF's import lets such names in

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-module/src/nasher/project.rs:409-414 and :951 (`let Some(path) = self.new_file(key) else { continue }`); crates/mg-module/src/transfer.rs:271-290 (`Incoming::archive` has no `is_file_name` check, where `file_key` for loose files is strict)
- **Since:** before 2253fa78
- **Scenario:** an ERF with a resource `a/b.utc` is imported. In a nasher project the save skips it, clears the unsaved mark, and it is gone at the next open with no warning; in a module folder every save fails ("the name is not a file's") until the builder finds and deletes it.
- **Fix:** refuse such an entry in `Incoming::archive` (a `Refused` with the reason), and push a project warning where `new_file` returns `None` for a reason other than a `/dev/null` rule.
- **Test:** `Incoming::archive` of an ERF with `../x.utc` lists it under `refused`.
- **Confidence:** confirmed by reading

### P15. Where the disk has no hard links, `keep_previous` removes the good backup before the copy that may fail, and the new archive's temporary file is left

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-module/src/lib.rs:1001-1008; :652-655 (`write_archive` returns on the error without removing `tmp`)
- **Since:** after (782ef2a3)
- **Scenario:** a module on exFAT or a network share, the disk nearly full: the new archive is written to `x.mod.moonglow-tmp`, `x.mod.bak` is deleted, `hard_link` fails, `copy` runs out of room. Left behind: a cut-short `x.mod.bak` that looks like a backup, no good backup, and the temporary file holding the room.
- **Fix:** copy to `x.mod.bak.moonglow-tmp` and rename it over the old backup; remove `tmp` when the backup step fails.
- **Test:** hard to force without a file system; a unit test with `backup`'s place taken by a folder shows the temporary file is removed on the error.
- **Confidence:** confirmed by reading

### P16. `with_value_in` puts a new key on the line after the first header that names the section, which may be a sub-table's

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-module/src/project_file.rs:1867-1875
- **Since:** after (f0dec956)
- **Scenario:** a team's file has `[talk.builders]` and no `[talk]` header. `set_talk_lines` finds the `talk` table, takes its key's place (in the `[talk.builders]` line) and inserts `lines = "…"` under it: a builder named `lines` gets the range, `[talk] lines` stays unset, and the window reports success. The same with a dotted key (`talk.typed = true`) or an inline table.
- **Fix:** insert only after a header that is exactly `[section]`; otherwise append a `[section]` header (TOML allows it after `[section.sub]`), or do it with `toml_edit` as `set_used` does.
- **Test:** `with_value_in("f", "[talk.builders]\nab = \"1-2\"\n", "talk", "lines", Some("\"5-9\""))` read back has `talk.lines` and one builder.
- **Confidence:** suspected: it rests on the span `toml`'s `DeTable` gives the key of a table made by `[talk.builders]` alone. A test settles it.

### P17. Write-then-rename is written eleven times, with four names for the temporary file; one copy flushes and cleans up

- **Kind:** duplication
- **Severity:** low
- **Where:** source of truth crates/mg-module/src/lib.rs:1019-1037 (`write_replacing`: `sync_all`, the temporary file removed on failure). Copies: lib.rs:786-788 (`write_folder`); nasher/project.rs:506-508, :613-615, :1011-1013, :1063-1065; claims.rs:385-388 (all `.moonglow-tmp`, no flush, no removal); nasher/tlk.rs:181-184 (`.part` appended); talk.rs:1015-1018 (`with_extension("tlk.part")`); nwsync.rs:201-209, hak_edit.rs:646-661, attach.rs:232 (`with_extension("part")`: `x.hak` and `x.erf` share `x.part`). And crates/mg-module/src/build.rs:324 (`Remembered::write`) writes in place, against the house rule.
- **Since:** before 2253fa78 (most); after for the `left_writing` sweep
- **Scenario:** the drift shows in what a crash or a full disk leaves: `write_folder` and `Project::save` leave a half-written `name.ext.moonglow-tmp` (their `?` returns before any removal); in a project it is swept only if the target's include happens to match it (`Project::open` calls `left_writing` on `source_files` alone, and `src/**/*.{nss,json}` does not match `x.nss.moonglow-tmp`), so git offers it for commit, which the sweep's own comment says it is there to prevent; `.part` files are never swept at all.
- **Fix:** one helper (`write_replacing`, and a streamed form for the hak and the archive) used everywhere; one suffix; the project's sweep walks the include's folders, not its matches.
- **Test:** unit test: a `x.nss.moonglow-tmp` aged past ten minutes in `src/` of a default project is gone after `Project::open`.
- **Confidence:** confirmed by reading

### P18. Import Folder compares every file's path with every one before it, and every key with every other

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-module/src/transfer.rs:312-336 (`keyed.iter().any(|(_, p)| *p == f)` for each file; `others` over all `keyed` for each)
- **Since:** after (0b7c8c57)
- **Scenario:** a module folder imported whole, which the function's own comment names as the case (20,000 to 30,000 files for a persistent world): 200 to 450 million path comparisons plus as many key comparisons, seconds before the dialog shows.
- **Fix:** a `HashSet<PathBuf>` for the files seen and a `HashMap<ResKey, Vec<usize>>` for the twins.
- **Test:** none needed beyond the existing ones; a count of comparisons is not worth a test.
- **Confidence:** confirmed by reading; the time is an estimate

### P19. Whether a target's sources take a file in compiles a regular expression for each pattern, for each file

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-module/src/nasher/glob.rs:126-147 (`covers`: `to_regex` and `Regex::new` at every call); callers crates/mg-module/src/nasher/cfg.rs:433-436, nasher/project.rs:1300-1304 (`left_out_of`, at every open, after a save that wrote uncovered files, and in a scan that found a file gone), :1234 (a pull's files named by the watch)
- **Since:** before 2253fa78
- **Scenario:** a project of 30,000 source files with two includes and an exclude: `left_out_of` walks them all and compiles 60,000 to 90,000 expressions at each open (a second or so at 10 to 20 microseconds each), and a pull naming 5,000 files does 15,000 on the scan's thread.
- **Fix:** compile a target's includes and excludes once (a `Covers` made from the `Target`, kept by the `Project` and the scan's `Kind::Project`).
- **Test:** the existing `left_out` tests cover the behaviour.
- **Confidence:** confirmed by reading; the time is an estimate, a measurement on a large project would settle it

### P20. `git status` is run in the module's folder with the folder's own configuration

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-module/src/stamp.rs:189-203 (`commit_at`), reached by `Stamp::of` (Export) and `compare` (Import)
- **Since:** before 2253fa78
- **Scenario:** a module folder downloaded as an archive carries a `.git` whose config sets `core.fsmonitor`; Export ERF from it runs that program. The project file refuses to carry `[allowed_commands]` for the same reason.
- **Fix:** pass `-c core.fsmonitor=false -c core.hooksPath=` (and `--no-optional-locks`) to every git Moonglow starts.
- **Test:** a `#[cfg(unix)]` test with a scratch repository whose `core.fsmonitor` touches a file: `commit_at` leaves no file.
- **Confidence:** suspected (git's behaviour, from memory: `core.fsmonitor` in a repository's `.git/config` names a program `git status` runs; `safe.directory` guards only repositories owned by another user). Running `git -C <dir> status` in a folder with such a config settles it.

### P21. An archive's hak named `D:x.hak` is not refused

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-module/src/attach.rs:46-53 (`content_file` splits on `/` and `\` and refuses `..`, not `:`), :168
- **Since:** after (887ce018)
- **Scenario:** a zip entry `D:evil.hak`: `hak_dir.join("D:evil.hak")` is `D:evil.hak`, written in drive D's current folder; `a.hak:b.hak` writes an alternate stream.
- **Fix:** refuse a file name `ResRef::is_file_name`'s characters would refuse (`:` and control characters).
- **Test:** add `("D:evil.hak", "x")` to the zip of `a_zip_s_haks_and_talk_table_go_to_the_user_folder`: it is left out.
- **Confidence:** suspected (Windows only; `Path::join` with a drive-relative path replaces the folder; not tried)

### P22. A table description's mistakes are said without their line, and the first one drops the whole description

- **Kind:** hygiene
- **Severity:** low
- **Where:** crates/mg-module/src/table_desc.rs:717-821
- **Since:** before 2253fa78
- **Scenario:** a `loot.2da.toml` of forty columns with `kind = "flag"` in one: "column Damage: "flag" is no kind", no line, and no column is described until it is mended.
- **Fix:** read it with `DeTable` and the `Reader` of `project_file` (spans to lines), a wrong column a note and the rest kept.
- **Test:** a description with one wrong column still describes the others, and its note begins `name.2da.toml:LINE:`.
- **Confidence:** confirmed by reading (`Description::parse` returns `Err(String)` at the first wrong `kind`, value or rule; only TOML's own errors carry a line; `project_file` and `var_desc` warn by `file:line` and read the rest)

### P23. Doc comments left on the wrong item or twice

- **Kind:** hygiene
- **Severity:** low
- **Where:** crates/mg-module/src/talk.rs:515-522 (`put`: two comments, the first says `None` removes the last row only, which the code no longer does) and :910-915 (`import`: two); crates/mg-module/src/nasher/project.rs:206-210 (`resource_of`'s comment sits on `source_base`); crates/mg-module/src/nasher/cfg.rs:429-432 (`source_files`' comment sits on `covers`)
- **Since:** before 2253fa78
- **Scenario:** rustdoc shows `covers` as "The target's source files: what its include globs find…".
- **Fix:** move or delete them.
- **Test:** none.
- **Confidence:** confirmed by reading

## Not read

- `crates/mg-module/src/script_wizard.rs`: only its diff since 2253fa78 (five lines) and lines 395-430 and 700-726; the rest (about 650 lines, unchanged since the last review) not read.
- `crates/mg-module/src/var_desc.rs` lines 1-75 and 196-259 (the module's doc and the `Described`/`Rule`/`Needed` declarations).
- `crates/mg-module/src/table_desc/*.toml` (the two built-in descriptions): not read.
- Test modules (`#[cfg(test)]`, `palette/kept_tests.rs`, `palette_groups/tests.rs`) only where a finding needed one.
- `bake.rs`, `doctor.rs` and the rest of mg-module are another reader's; of them only `BakeRules::rows_path` was looked at.

## Looked at and found sound

- Saving a folder and a project (`Module::write_folder`, `Project::save`): the three-way comparison of what was read, what is on disk and what the module has; nothing is written or deleted when any file conflicts; a file is deleted only where it is as it was read; names go through `ResKey::path_in`.
- The outside look (`outside.rs`, `folder::scan`, `nasher::scan`, `archive_file.rs`): stamps taken before the read, `stands` dropping what a save or a take overtook, a moved file told from a deleted one.
- `nasher/cfg.rs` and `nasher/glob.rs` as parsers: no index or slice that a cut or odd file can push out of bounds; the walk does not follow links to folders (`DirEntry::file_type`), so a link loop ends.
- `attach.rs` and `attach/archive.rs`: entry names with `..` or a trailing separator refused, one pass over solid archives, every `.part` removed when any copy fails, passwords and volumes said as such.
- `claims.rs`: a change or a merge rewrites one row's line and keeps every other line, the line ends and the encoding; rows are found by Area, not by number.
- `palette.rs` `updated`: paths are found again (`Places`) after every change of the tree's shape; untouched categories keep their bytes; nothing is written when nothing moved.
- `watch.rs`: bursts settle before they are said, hidden folders are passed over, the thread ends when the watch is dropped.
