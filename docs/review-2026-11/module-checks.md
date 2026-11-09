---
type: Report
title: 'Code review of November 2026: verify, search, rename and the module''s queries'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in verify, search, rename and the module''s queries (mg-module): 25 in all (2 high, 9 medium, 14 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-module, verify, rename, conversations]
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

# Code review of November 2026: verify, search, rename and the module's queries

One part of [the review of 9 November 2026](overview.md): verify, search, rename and the module's queries (mg-module). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it. Paths are under `crates/mg-module/src/` unless they begin with `crates/` or `apps/`.
## High

### V1. An area's minimap panics on Width 0 and allocates by whatever Width and Height the ARE claims

- **Kind:** bug
- **Severity:** high (crash)
- **Where:** minimap.rs:61-66 (Width and Height taken as any `u32`), :95 (`Rgba::new(width * size, height * size)`), :100 (`i as u32 % width`)
- **Since:** before 2253fa78
- **Scenario:** an ARE with `Width` 0 and a non-empty `Tile_List` (a broken or hostile module): `i % width` divides by zero and the window (or `mg minimap`) panics. An ARE with `Width`/`Height` of 60000: `vec![0; w*h*4]` asks for terabytes and the process aborts; larger values wrap `width * size` (no overflow checks in release), after which the picture's size and the loops over it no longer agree. `doctor::tile_seams` clamps the same two fields to 1024 (doctor.rs:3537); this reader has no bound.
- **Fix:** refuse Width or Height of 0 or over a bound (the game's 32, or 1024 as `tile_seams`) with an `Err` before anything is sized; multiply in `usize`/checked.
- **Test:** unit tests in minimap.rs: `minimap()` of an ARE root with Width 0 and one tile, and of one with Width 70000, both return `Err`.
- **Confidence:** confirmed by reading (`minimap.rs:61-106`, `mg_image::Rgba::new` at `crates/mg-image/src/lib.rs:140`, the callers `crates/mg-ui/src/minimap.rs:63-66` on the window's thread and `apps/mg/src/main.rs:4302`)
- **Read again by the reviewer:** holds: `i as u32 % width` with `Width` unchecked, and `Rgba::new(width * size, height * size)` (minimap.rs:61-66, :95, :100).

### V2. Export as Text walks owning links with no guard: a conversation whose lines own each other never ends, one with a line owned twice grows as two to the depth

- **Kind:** bug
- **Severity:** high (crash: stack overflow, or a hang with memory growing)
- **Where:** dialog_io.rs:656-689 (`to_text`); the same shape in dialog.rs:775-797 (`outline`, used by tests only today)
- **Since:** before 2253fa78
- **Scenario:** a DLG from another tool or a hak where EntryList[0] › RepliesList[0] and ReplyList[0] › EntriesList[0] both point back with `IsChild` 0 (or `IsChild` missing or not a BYTE: `is_link` is `byte("IsChild") == Some(1)`). Export › Text (window) or `mg dialog-export --format text` recurses until the stack overflows. Without a loop but with one line owned by two parents at each of 30 levels, the text is 2^30 branches.
- **Fix:** carry the path (as `play_outline`) or a `seen` set (as `to_csv_in`): a line met again is written once with "(shown above)", as a link is.
- **Test:** unit test in dialog_io.rs: build that two-line loop by hand, `to_text` returns; and a line owned twice is written once.
- **Confidence:** confirmed by reading (`dialog_io.rs:655-693`: `walk` recurses on `!is_link(l)` with no `seen`, no depth and no limit; `to_csv_in` beside it has `seen` (:834), `play_outline` a limit and a path, `dialog::owned`/`copy_branch` sets; the editor's own walk guards with a path and says such files exist: `crates/mg-ui/src/dialog_view.rs:331-334` "in a file where several lines own one … grows as two to the power of how deep such lines go")
- **Read again by the reviewer:** holds: `walk` recurses on every owning link with no visited set and no depth limit (dialog_io.rs:656-689).

## Medium

### V3. The walkmesh bake's pending layer (`bake:rows`) is taken for a real one: Make Light always refuses while a baked row is pending, and Verify skips the haks' placeables.2da

- **Kind:** bug
- **Severity:** medium
- **Where:** lights.rs:239 (`rm.find(&key)` finds the pending layer) and :243-248; doctor.rs:989 (`resman.find(key) == Some(i)`: the hak's copy no longer wins) with `is_custom` not knowing the pending layer
- **Since:** after (lights.rs 192a4e4d, bake.rs 9aab919c)
- **Scenario:** a project with `[bake]` and `[lights] generated`. Bake one placeable; until the project's build adds the row the layer exists. (a) Make Light into a listed hak: `top < i` is true, into a new one: `priority > HAK_USER` is true; either way "placeables.2da is read from bake:rows over X.hak … move X.hak above it in the module's list", advice that can't be followed. (b) Verify: the hak's `placeables.2da` is not "the winning copy", so `two_da` never runs on it (2da-row, 2da-read, 2da-limit, 2da-shadow, 2da-strref, 2da-ref all silent for that table), and the pending layer itself is not custom, so nothing checks it either.
- **Fix:** one function for "the layer whose placeables.2da the game reads" that steps over `PENDING_LABEL` (what `table_under` does), used by `lights::plan`; in `examine_groups`, look for the winner among layers other than the pending one (or give ResMan a `find_where`).
- **Test:** lights.rs unit test: `plan` with a ResMan that has a hak with placeables.2da and a `bake:rows` layer succeeds; doctor test: a hak's placeables.2da with an overlong row is reported with the pending layer present.
- **Confidence:** confirmed by reading (bake.rs:57 `PENDING_PRIORITY = HAK_USER + 1`; `Module::stack_onto` lib.rs:417-420 and `crates/mg-ui/src/lib.rs:5291` put the layer in every ResMan; lights.rs:239-261; doctor.rs:797-807 and :981-994; `bake::table_under` (bake.rs:343-353) is the one reader that steps over the layer)

### V4. Rename leaves the module's 2DA cells that name the script as they were, and says nothing

- **Kind:** bug
- **Severity:** medium
- **Where:** rename.rs:850-911; verify.rs:362-438 (the cells are known there)
- **Since:** before 2253fa78 (the rename); the tables' cells joined Find References later
- **Scenario:** a module (a nasher project) holding its own `spells.2da`, or a described table with a `kind = "script"` column. Rename `x_s0_mine` to `x_s0_mine2`: references 0, the 2DA's `ImpactScript` cell still says `x_s0_mine`, which no longer exists; the spell runs nothing, and Verify's missing references is silent. For a hak's table the module can't rewrite it, but should say so.
- **Fix:** in `rename`, for scripts: rewrite the module's own 2DAs' cells in `doctor::SCRIPT_COLUMNS` and described script columns (case-insensitive whole-cell match), and return the cells of haks' tables that still name the old script so both front ends warn of them, as they do of strings.
- **Test:** rename.rs unit test: a module with `spells.2da` naming the script in ImpactScript; after `rename` the cell names the new one and `report.changed` has the table.
- **Confidence:** confirmed by reading (`rename::rename` rename.rs:803-934 rewrites GFF fields, the custom palette, `#include`s and, asked, script strings; no `ResType::TWODA` anywhere in it. Find References does list those cells: `verify::table_scripts`/`described_scripts`, shown by `crates/mg-ui/src/references.rs:938-946` and `apps/mg/src/main.rs:3238-3243`. `verify::missing` reads only `module_references`, which has no table cells, so nothing reports the cell afterwards. `mg rename` and `warn_still_spelled` warn of script strings only.)

### V5. Import Lines fails on a CSV saved with a byte order mark ("no id column")

- **Kind:** bug
- **Severity:** medium
- **Where:** dialog_io.rs:878-883
- **Since:** before 2253fa78
- **Scenario:** export a conversation as CSV, open it in Excel, save as "CSV UTF-8 (Comma delimited)" (which writes a BOM), Import Lines: "no id column", since the header's first cell is `\u{feff}id` and `id` is the first column Moonglow writes. The module's own header says the CSV is for a spreadsheet.
- **Fix:** drop a leading U+FEFF in `read_csv` (and consider writing one in `to_csv_in`: without it Excel reads the UTF-8 export as ANSI and shows accents as mojibake).
- **Test:** dialog_io.rs unit test: `update_from_csv` of `"\u{feff}id,text\r\nE0,Hello\r\n"` changes one line.
- **Confidence:** confirmed by reading (`read_csv` dialog_io.rs:707 keeps U+FEFF in the first cell; `col("id")` at :881-882 compares `h.trim()`, and `str::trim` does not take U+FEFF off; both callers pass `std::fs::read_to_string` straight in: `crates/mg-ui/src/dialog_view.rs:1172-1174`, `apps/mg/src/main.rs:3463,3480`; no BOM handling anywhere in mg-module, none in the manual or the deferred list)

### V6. Verify parses every area's GIT again for each check, one area after another: up to eleven parses a GIT, none in parallel

- **Kind:** performance
- **Severity:** medium
- **Where:** doctor.rs:3059-3905 (`objects`, `pc_faction`, `static_transforms`, `static_tints`, `tints_stored`, `appearance_range`, `area_variables`, `tile_replacements`, `tile_seams`, `described_variables`, `variables_read`)
- **Since:** before 2253fa78 for the pattern; five of the loops are after (static-tint, tint-stored, appearance-range, tile-seam, var-script)
- **Scenario:** `docs/research/notes_scale.md` measured 0.9 s for the doctor and 2.3 s for missing references at 300 areas, before those five checks. A persistent world of 5,000 areas (the size this review was asked to weigh) is 17 times that on one core: a minute of Verify spent parsing the same GITs again while the other cores idle.
- **Fix:** one pass over the module's GFF resources (`par_iter`, key order kept as the hak pass keeps it): parse once, hand the tree to every check that wants its type, collect findings per key. The checks become `fn(&mut Out, key, &Gff)`.
- **Test:** `crates/mg-ui/tests/world_perf.rs` "content doctor" with the budget tightened; a unit test counting `Gff::read` calls per GIT is not possible without a hook, so a test that the findings are byte-identical before and after (as was done for the parallel hak pass).
- **Confidence:** confirmed by reading (`Module::gff` is `Gff::read` each call, lib.rs:340; one `for key in keys { module.gff(&key) }` loop per check: doctor.rs:3077, 3128, 3178, 3211, 3271, 3304 for GITs always, plus tags.rs:94 (`tag-unique`), doctor.rs:3713 and 3873 (`var-*`), bake.rs:832 (`bake-unused`) and refs.rs:358 via `verify::module_references`; AREs: doctor.rs:3433, 3465, 3527 plus the var checks. The per-resource hak checks were made parallel on 2026-10-22; these loops were not.)

### V7. A conversation exported to Twine or Ink and read back has Aurora's default end scripts, not its own

- **Kind:** bug
- **Severity:** medium
- **Where:** dialog_io.rs:990-1052, 1285-1394, 1238-1243
- **Since:** before 2253fa78
- **Scenario:** `gate.dlg` ends with `gate_done` (or with no end script, as the Store Setup wizard makes them: store_setup.rs:57-58). `mg dialog-export --format ink`, edit a line in Inky, `mg dialog-import gate.ink`: `gate.dlg` now runs `nw_walk_wp` when it ends or is aborted; `DelayEntry`, `DelayReply`, `PreventZoomIn`, each line's `AnimLoop` and every language but English are gone too (only the last is said in the header).
- **Fix:** carry the root's settings in the story (Twine: `StoryData`'s own JSON has room for a `moonglow` object; Ink: `# end:SCRIPT` / `# abort:SCRIPT` tags before the first knot) and read them back; when a story replaces a conversation, keep the replaced one's root fields the story doesn't give.
- **Test:** dialog_io.rs unit test: a dialog with `EndConversation` `x` round-trips through `to_ink`/`from_ink` and `to_twee`/`from_twee` with `x`.
- **Confidence:** confirmed by reading (`to_twee` dialog_io.rs:990-1052 and `to_ink` :1285-1394 write nothing of the root but its starting links; `Builder::build` :1238-1243 starts from `new_dialog()`, whose `EndConversation` and `EndConverAbort` are `nw_walk_wp` (dialog.rs:75-88); `mg dialog-import` writes the result over a conversation of the file's name without asking, `apps/mg/src/main.rs:3489`). The module's header says "A conversation read back plays as the original did (`play_outline`)"; the outline doesn't hold the end scripts.

### V8. A generated light can take a placeables.2da row past 65535, and `first_row` is looped to with no bound

- **Kind:** bug
- **Severity:** medium
- **Where:** lights.rs:283-289
- **Since:** after (192a4e4d)
- **Scenario:** `[lights] first_row = 70000` (or a placeables.2da already 65,536 rows long): Make Light writes row 70000 and places a placeable of it; the game carries the row as 16 bits and draws row 4464's model. `first_row = 4000000000` pads four billion rows and runs out of memory.
- **Fix:** refuse in `plan` when the row would be past `bake::LAST_ROW`, saying so; warn of a `first_row` past it where the project file is read.
- **Test:** lights.rs unit test: `plan(…, first_row = Some(65536))` is an `Err` naming the limit.
- **Confidence:** confirmed by reading (lights.rs:283-289: `while table.len() < first_row.unwrap_or(0) { add_row }` then `add_row()`; `first_row` is any `usize` the project file gives, `project_file.rs:1230-1240`; bake.rs holds the same table to `LAST_ROW` (bake.rs:42, :82) and Verify's `appearance-range` says what the game does past it)

### V9. The baked rows file is written in place

- **Kind:** bug
- **Severity:** medium (a crash mid-write leaves half a file, and "a file Moonglow can't read whole is not written over": nothing bakes, and the rows the areas' placeables name are gone from it)
- **Where:** bake.rs:320
- **Since:** after (9aab919c)
- **Scenario:** power loss or a full disk while `baked_rows.toml` is written after a bake or Tidy Up: the file is cut short, `RowsFile::parse` fails, `doctor::bake` says "nothing is baked until it reads", and the project's build no longer adds rows its areas use.
- **Fix:** `crate::write_replacing(path, text.as_bytes())`.
- **Test:** none needed beyond a unit test that `write` leaves no partial file when the target folder is read-only (the house rule's shape elsewhere).
- **Confidence:** confirmed by reading (bake.rs:315-321 `std::fs::write(path, …)`; `crate::write_replacing` exists and locks.rs:263 uses it; callers `crates/mg-ui/src/bake_view.rs:315,531`, `apps/mg/src/bake.rs:144`)

### V10. `var-requires` scans every variable of a file for each object of it

- **Kind:** performance
- **Severity:** medium
- **Where:** doctor.rs:3718-3724
- **Since:** after (996e92f6, 2026-10-29)
- **Scenario:** a project with `variables.toml` whose placeables carry tint variables (The Domains of Dread): an area of 1,000 placeables with four variables each compares 1,000 × 4,000 paths, each a `Vec<Step>` with a string label; over 500 such areas that is two billion path compares in one of Verify's sequential loops.
- **Fix:** `found.chunk_by(|a, b| a.path == b.path)` and take each chunk as the object's variables.
- **Test:** unit test in doctor.rs's tests that findings are unchanged; `world_perf`'s doctor budget with a world whose objects have variables.
- **Confidence:** confirmed by reading (doctor.rs:3718-3724: `objects.dedup()` then, per object, `found.iter().filter(|f| f.path == *path)`; `var_desc::variables_of` (var_desc.rs:1339-1352) pushes an object's variables together, so the groups are already contiguous)

### V11. Edit Areas Together / `mg areas` parses every area's GIT whatever the change, and keeps every changed tree in memory to the end

- **Kind:** performance
- **Severity:** medium
- **Where:** areas.rs:746-973
- **Since:** before 2253fa78
- **Scenario:** `mg areas MODULE --set Interior=yes` (or a lighting scheme) over 5,000 areas parses 5,000 GITs it never touches. `--var x=1` or `MusicDay=57` over them holds 5,000 parsed GITs at once: a tree is several times its file, so a gigabyte of GITs is many gigabytes of RAM before the first is written.
- **Fix:** read the GIT lazily (first change that needs it), and turn each changed tree into bytes as its area is done (`write: Vec<(ResKey, Vec<u8>)>`): the all-or-nothing promise holds, and `to_bytes` errors surface before anything is set.
- **Test:** unit test: `apply` with an ARE-only change succeeds on an area whose GIT does not parse (today it fails on `read(&git_key)?`), which also shows the GIT is no longer read.
- **Confidence:** confirmed by reading (areas.rs:756 `read(&git_key)?` for every area before any change is looked at; :964-969 pushes the parsed `Gff`s into `write`; :971-973 serializes them only after the last area)

## Low

### V12. Missing references asks every layer about the same name again for each reference to it

- **Kind:** performance
- **Severity:** low
- **Where:** verify.rs:245-287, 465-495
- **Since:** before 2253fa78 (`only_local` added after, b6173ad2: a second walk per reference where a userpatch, override or Workshop layer exists)
- **Scenario:** 150 haks, 250,000 placed creatures each naming a dozen of the game's default scripts: three million references whose targets are found only below all 150 hak layers, 450 million container probes for a few hundred distinct names.
- **Fix:** memoize by `(kind, target)`: satisfied, local-only and uncompiled are the same for every reference to it.
- **Test:** unit test that `missing_of` gives the same list; `world_perf` "verify" budget.
- **Confidence:** suspected (read: verify.rs:254-279 computes `candidates`, `has` and `only_local` for every reference and only then drops repeats at :274; `ResMan::find` walks the layers, `crates/mg-resman/src/lib.rs:255`; the same per-string walk in `outside_references` :472-483, eleven kinds a string. What would settle the size: `mg verify --timings` on a module with 100+ haks.)

### V13. Ink: three texts that don't come back as written

- **Kind:** bug
- **Severity:** low
- **Where:** dialog_io.rs:1365 with :1502 (a reply's line breaks are written as `\n` and read back with `replace("\\n", "\n")` before unescaping, so a literal backslash followed by `n`, written `\\n`, becomes a line break); :1434 (a line of an NPC's text that starts `VAR ` or `CONST ` is skipped as a declaration: `ink_text` escapes `- * + =` at a line's start, not these); :470-475 with :1365 and :1433 (a whitespace-only line inside a reply becomes `//` inside the choice's brackets, and `ink_uncomment` then cuts the line there: "a choice's [text] doesn't end")
- **Since:** before 2253fa78
- **Scenario:** a reply "Path: C:\new" comes back as "Path: C:" + line break + "ew"; an NPC line "VAR is a joke, right?" comes back empty; a reply of two paragraphs separated by a line holding a space exports to an `.ink` Moonglow refuses.
- **Fix:** unescape first and split on an unescaped `\n` marker (walk the characters once); escape a leading `VAR `/`CONST ` (a backslash before the first letter); write a blank line inside a choice as nothing (an empty segment), not `//`.
- **Test:** dialog_io.rs round-trip unit tests for the three texts.
- **Confidence:** confirmed by reading

### V14. What Verify keeps of a hak is named by the hak's size and time alone

- **Kind:** bug
- **Severity:** low
- **Where:** verify_cache.rs:70-75, 180-185, 238-248
- **Since:** after (d12e7d29)
- **Scenario:** (wrong result) a hak rebuilt with a fixed mtime after a same-length edit of a model's text (a digit changed): Verify reports the old text's findings, or none, until the release changes. (lost work) two checkouts of a project with haks of the same names and different times: each Verify deletes the other's file as "the archive's earlier version" (:241-247) and reads 5 GB of models again.
- **Fix:** put the ERF's own key table into the name (a hash of its entries' offsets and sizes, already read when the hak is opened), and the hak's folder into the stem.
- **Test:** verify_cache.rs unit test: two archives of equal size and time and different entries get different file names.
- **Confidence:** confirmed by reading for the name (verify_cache.rs:70-75, :178-185; `Container::fingerprint` hashes `len` and `modified` only, `crates/mg-resman/src/container.rs:77-83, 286-292`); suspected for how often it bites (a build that stamps haks with a fixed time, as reproducible builds do)

### V15. Advanced Search with "haks and game" on compares every kept blueprint with every wanted one on each Find after the first

- **Kind:** performance
- **Severity:** low
- **Where:** search.rs:984-988
- **Since:** after (2e85815d, 2026-11-02)
- **Scenario:** the game's own ten thousand or so blueprints, more with blueprint haks (a PRC or CEP starter hak): the second Find and every Find after a change is n²/2 key compares (5×10⁷ at 10,000, 10⁹ at 45,000) before anything is searched.
- **Fix:** build `HashMap<ResKey, (label index, fingerprint)>` from `wanted` once and `retain` by lookup.
- **Test:** `crates/mg-ui/tests/world_perf.rs`: a second `Kept::search` with `haks_and_game` inside a budget.
- **Confidence:** confirmed by reading (search.rs:984-988: `self.outside.retain(… wanted.iter().any(…))`; runs on the window's thread, `crates/mg-ui/src/search.rs:629`)

### V16. `links_into` scans every tag of the module for each tag of the area being deleted

- **Kind:** performance
- **Severity:** low
- **Where:** rename.rs:399-441
- **Since:** after (d9d78567, 2026-10-29)
- **Scenario:** deleting 20 areas of a 5,000-area world: each area's hundred tags that no other area has are each compared with 2.5 million tag entries, 5 billion compares before the "what names it" list shows.
- **Fix:** collect the other areas' own tags into a `HashSet<&[u8]>` once per call (or once per index refresh).
- **Test:** unit test for equal results; a `world_perf` step for Delete's question.
- **Confidence:** confirmed by reading (rename.rs:410-419 `elsewhere` walks `self.seen` and each GIT's `tags` per tag; called per area from `crates/mg-ui/src/references.rs:749-751`)

### V17. Rename reads and walks every GFF of the module on the window's thread, though the index knows who names the resource

- **Kind:** performance
- **Severity:** low
- **Where:** rename.rs:850-864
- **Since:** before 2253fa78
- **Scenario:** renaming one script in a 5,000-area world: the same work as Find References' first read (0.8 s on every core, by rename.rs:96-97) done on one core with the window frozen.
- **Fix:** let `rename` take the keys to rewrite (from the index's `usages`, plus the custom palette and scripts), falling back to all for the command line.
- **Test:** unit test that `rename` given the index's keys gives the same module as without.
- **Confidence:** confirmed by reading (rename.rs:851-864 parses each GFF in turn and rewrites; `UsageIndex::usages` (:444) already answers which resources reference the target, in parallel, kept between questions; `rename_resource` runs it inline, `crates/mg-ui/src/references.rs:269`)

### V18. Paste of a copied branch maps lines wrongly when the branch holds a link to a line that isn't there

- **Kind:** bug
- **Severity:** low
- **Where:** dialog.rs:677-700, 705-748
- **Since:** before 2253fa78
- **Scenario:** copy a branch whose third line owns a link to index 999 of a 40-line list; paste: every line after the third is mapped one off, so the pasted lines' own links point at the wrong copies (and if the missing one is the top, `placed[0]` panics).
- **Fix:** in `copy_branch`, push to `order` only what `node()` finds (or build `lines` and `origin` together).
- **Test:** dialog.rs unit test: copy and paste such a branch; `outline` of the pasted copy equals the original's less the dangling link.
- **Confidence:** confirmed by reading for the misalignment (dialog.rs:694 `lines` leaves out a missing node while `origin` keeps its place; :711-722 zips `origin` with `placed`, made from `lines`); the input needs a DLG with a dangling owning link inside the branch (another tool's file)

### V19. Removing a faction leaves any object still of it with the id of the faction that moves into its place

- **Kind:** bug
- **Severity:** low
- **Where:** factions.rs:238-262
- **Since:** before 2253fa78
- **Scenario:** give an open creature a custom faction, don't save, remove the faction: no user is found, the factions after it move down, and the creature (flushed afterwards) keeps the old number: another faction's, or none's.
- **Fix:** `renumber_faction_users` puts a removed faction's users in its parent (or returns them as an error) instead of leaving the number; the caller flushes before asking for users.
- **Test:** factions.rs unit test: an object of the removed faction is not left with the old id.
- **Confidence:** suspected (read: factions.rs:247 `map(old).unwrap_or(old)`; the window refuses a removal while `faction_users` finds users, but asks it of `ws.module` before `store_factions` flushes the open documents, `crates/mg-ui/src/faction_view.rs:390` then `:138`. What would settle it: whether an open, unsaved creature blueprint just given the faction is in `ws.module`'s bytes at :390.)

### V20. Verify is put together twice, and the two have drifted

- **Kind:** duplication
- **Severity:** low
- **Where:** `crates/mg-ui/src/lib.rs:5556-5612` and `apps/mg/src/main.rs:5228-5282` each call, in order, missing references, `doctor::examine_groups`, `talk_range`, `bake`, `palette_groups`, the plugins, `Skip::leave_out`, `fix::attach`
- **Since:** after for most of the parts (talk-range, bake, palette checks are post-review)
- **Scenario:** the copies differ already: the command line reads placeables.2da for `doctor::bake` in `Codepage::default()`, the window in the game's; the command line gathers `module_references` once for missing and unused, the window reads the module twice (`verify::missing` in the job, again in `build_view.rs:547`). The next check added to one is forgotten in the other ("Every tool says the same", doctor.rs:1184, is kept by hand).
- **Fix:** one `mg_module::verify::run(module, game data, tables, skip, timings, cache) -> (Vec<Missing>, Vec<Finding>)` both call; the plugins' findings appended by the caller.
- **Test:** a test in `apps/mg` or mg-corpus-tests that the window's job and `mg verify --json` give the same findings for a fixture module.
- **Confidence:** confirmed by reading

### V21. A 2DA cell's rule is written twice in doctor.rs, for Verify and for the 2DA editor

- **Kind:** duplication
- **Severity:** low
- **Where:** doctor.rs:2719-2770 (`two_da`'s loop over column kinds) and :2572-2620 (`table_faults`, "checked by this as it is typed"); :2707-2718 repeats `column_kinds_as_named` (:2535-2552) line for line
- **Since:** before 2253fa78 for the pair; both grew after
- **Scenario:** drift so far is small and one-way: Verify passes over a cell the game's own table has wrong too (`kept`, :2749-2755), the editor's copy does not, so a hak's `appearance.2da` that keeps the game's rows 562 to 568 (FOOTSTEPTYPE 30) is faulted in the editor and not in Verify. A rule changed in one (a new column kind, a new exemption) has to be remembered in the other.
- **Fix:** `two_da` calls `table_faults` (given the base copy, for the changed-rows filter and the game's-own exemption) and `column_kinds_as_named` with a closure over `d.table`.
- **Test:** a unit test that, for a table with one fault of each kind, Verify's `2da-strref`/`2da-ref` findings and `table_faults` name the same cells.
- **Confidence:** confirmed by reading

### V22. Small tables kept twice

- **Kind:** duplication
- **Severity:** low
- **Where:** a sound's priority by how it plays: instances.rs:1094-1099 and `blueprints::sound_priority` (blueprints.rs:144-151); an item's color fields: instances.rs:510-511 `ITEM_COLORS` and blueprints.rs:443-444 `COLORS`; the visual transform's two fields: instances.rs:1181 `KEPT` and placement.rs:122 `VISUAL_TRANSFORM_FIELDS`; "a variable type's empty value" closure: doctor.rs:3706-3712 and :3856-3862; the project file's shown name: doctor.rs:1192-1196, :1299-1303, :3334-3338; `doctor::shader_cache_dir` (doctor.rs:763-765) is `build::shader_cache_dir` (build.rs:381) again and is called by nothing (dead; `Cache::user`'s comment at :785 already points at build's)
- **Since:** mixed; `doctor::shader_cache_dir` after (9fd2149a)
- **Scenario:** none wrong today.
- **Fix:** instances.rs calls `blueprints::sound_priority` and uses blueprints' `COLORS`; `KEPT` built from `VISUAL_TRANSFORM_FIELDS`; delete `doctor::shader_cache_dir`.
- **Test:** none (the Aurora captures already hold the values).
- **Confidence:** confirmed by reading

### V23. Compiling a folder of text models looks each supermodel and material up by walking every file, a `String` made per file per lookup

- **Kind:** performance
- **Severity:** low
- **Where:** models.rs:66-74
- **Since:** before 2253fa78
- **Scenario:** `mg pack --compile-models` over a hak folder of 43,000 models kept as text (verify_cache.rs:6 gives that count for one project): each model with a supermodel in the game misses in `files`, 43,000 allocations and compares a lookup, at least two lookups a model.
- **Fix:** a `HashMap<(ResType, lower-case name), &[u8]>` made once before the threads start.
- **Test:** none beyond the existing compile tests (same results).
- **Confidence:** confirmed by reading for the shape (models.rs:66-71 `own`: `files.iter().find(… k.resref.to_string().eq_ignore_ascii_case(name))`, called through `find` and `material`); suspected for the size (how often `mg_mdl::compile` asks)

### V24. A variable typed `inf`, `nan` or `infinity` with no type given is stored as a float

- **Kind:** hygiene
- **Severity:** low
- **Where:** vars.rs:67-77; reached from `areas::set_var` (areas.rs:702-705) for `mg areas --var NAME=VALUE` and Edit Areas Together
- **Since:** before 2253fa78
- **Scenario:** `mg areas M --var mood=nan` writes a FLOAT NaN where a builder meant the string.
- **Fix:** take a float only when the text is digits with a point (and a sign), as the game's own text-to-float does; else a string.
- **Test:** vars.rs unit test: `of_text("nan")`, `of_text("inf")` are `String`.
- **Confidence:** confirmed by reading (vars.rs:67-77: `t.parse::<f32>().is_ok()` accepts them, and `1e5`)

### V25. The store script is written with the tag put between quotes as it is

- **Kind:** hygiene
- **Severity:** low
- **Where:** store_setup.rs:44-49
- **Since:** before 2253fa78
- **Scenario:** a store whose tag holds `"` or `\` (another tool's, or a project's `{area}` pattern over such an area tag): the script Setup Store writes doesn't compile, or reads another tag.
- **Fix:** escape `\` and `"` in the tag as NWScript strings take them, or refuse such a tag in the wizard.
- **Test:** store_setup.rs unit test: `script("a\"b", false)` tokenizes as one terminated string.
- **Confidence:** confirmed by reading (store_setup.rs:38-50 formats `GetNearestObjectByTag("{tag}")`)

## Not read

nothing of the part's source was left unread. Not under review and not read: the `#[cfg(test)]` modules and `bake/tests.rs` (opened only where a test said what the code is meant to do). Callees outside the part were read only where a finding rests on them (`Module::gff`, `ResMan::find`, `Container::fingerprint`, `Struct::walk_fields`, `var_desc::variables_of`, `Rgba::new`, the callers named above); `mg_rules::appearance` (the model and parts judged by `objects`), `mg_tiles::seams`, `var_desc` and `table_desc` were not followed.

## Looked at and found sound

- rename.rs / refs.rs: finding and rewriting a reference go through one `classify` (the two walkers pass the enclosing list the same way, through nested structs); a ResRef is compared whole and without case, so a name that begins another is left alone; own-name fields, the family (`.ncs`/`.ndb`, `.git`/`.gic`), the custom palette and compiled scripts of changed sources are handled; both front ends rename a staged copy and apply the difference as one command, so an error mid-way loses nothing.
- verify_cache.rs: a model's facts are always read from and kept under the layer whose copy wins; merges keep each fact apart; files are release-keyed, written whole and renamed; a folder or the module is never kept.
- instances.rs: Update Instances keeps exactly where the object stands, its outline, its visual transform and an encounter's spawn points, as the Aurora captures say; held items keep their place, `Dropable`, `Pickpocketable`, `Infinite`; the walks through bags are depth-bounded; the facing written is the bit pattern Aurora writes.
- dialog_io.rs Twine/Ink: syntax characters in text, parameters and comments are escaped and come back; a line that loops to itself is made before its replies are placed, and a choice that names itself ends at the depth cap with an error; link order under a parent is kept.
- text.rs: Find and Replace refuses a ResRef over 16 characters and a tag over 32, reports a codepage that can't hold the new text instead of writing "?", and leaves variants it didn't change byte for byte.
- grass.rs plans (indices taken out last-first, fields set before removals), tags.rs (`retag`, `clashes`, per-area counting), locks.rs (atomic write, unreadable file kept beside).
