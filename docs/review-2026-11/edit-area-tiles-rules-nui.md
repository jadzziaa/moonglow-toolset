---
type: Report
title: 'Code review of November 2026: the workspace, areas, tiles, rules and NUI documents'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the workspace, areas, tiles, rules and NUI documents (mg-edit, mg-area, mg-tiles, mg-rules, mg-nui, mg-schema): 24 in all (1 high, 7 medium, 16 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-edit, mg-area, mg-tiles, mg-rules, mg-nui, undo]
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

# Code review of November 2026: the workspace, areas, tiles, rules and NUI documents

One part of [the review of 9 November 2026](overview.md): the workspace, areas, tiles, rules and NUI documents (mg-edit, mg-area, mg-tiles, mg-rules, mg-nui, mg-schema). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## High

### E1. The module reads as saved after an Undo past the save and a new edit: unsaved work is lost without a question

- **Kind:** bug
- **Severity:** high (data loss)
- **Where:** crates/mg-edit/src/lib.rs:336 (`changes_since_save != 0`), :417 (`apply` adds 1), :451, :823
- **Since:** before 2253fa78
- **Scenario:** edit A, Save (counter 0), Undo (−1, title shows `*`), edit B (`apply`: −1 + 1 = 0). The module is "saved − A + B" and `is_modified()` is false: the `*` goes, closing asks nothing, the recovery copy is skipped. Same shape with `mark_modified` (it sets the counter to `max(n, 1)`): Save, keep "Moonglow's" in an outside-change conflict (counter 1), Undo an older command (0): reads as saved with the conflict's answer and the undone step unwritten. `join_last` across a save subtracts `n − 1` the same way.
- **Fix:** keep the saved point as a place in the history, not a count: `saved_at: Option<usize>` (the undo depth at the save; `None` once a command is applied below it or the redo holding it is dropped), plus a flag that `mark_modified` sets and only a save clears. `is_modified` is `flag || saved_at != Some(undo.len())`.
- **Test:** unit test in `mg-edit/src/lib.rs`: apply, `save_as`, undo, apply another, `assert!(ws.is_modified())`; and `mark_modified` then undo of a pre-save command.
- **Confidence:** confirmed by reading (`Workspace::apply`/`undo`/`save`/`mark_modified`/`is_modified` in `mg-edit/src/lib.rs`; the callers `Moonglow::has_unsaved_work` and `title` in `mg-ui/src/lib.rs:3378,1394`, `recovery.rs:335`; no test covers it, and neither `docs/deferred.md` nor `docs/log.md` has it)
- **Read again by the reviewer:** holds: `apply` adds one, `undo` takes one, `is_modified` is `changes_since_save != 0` (lib.rs:337, :417, :451): save, undo, another edit gives 0.

## Medium

### E2. Rotate Area leaves an encounter's outline unturned and writes `PointX`/`PointY` into its points (Resize at the west or south edge writes them too)

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-area/src/reshape.rs:357-363
- **Since:** before 2253fa78 (2c700cf7)
- **Scenario:** an area with an encounter, Rotate Area by a quarter. For every object with a `Geometry` list the code reads and sets `PointX`/`PointY`. An encounter's points have `X`/`Y`: they read as 0, each point gains two new fields (`PointX = -0.0`, `PointY = 0.0`), and its real `X`/`Y` stay, so the encounter's position turns and its outline does not (a 10 by 4 rectangle still lies west–east). `resize_at` with `turns != 0` turns the copy and back and then writes every object list: the outline ends unturned (net nothing) but the junk fields are saved on every encounter point.
- **Fix:** take the labels from `mg_module::placement::point_fields(kind)` (skip kinds with none), as `edit.rs` does.
- **Test:** extend `rotating_four_times_is_the_identity` with an encounter (Geometry of `X/Y/Z`): after one turn its points are turned and no point has a `PointX`; after four the GIT equals the original.
- **Confidence:** confirmed by reading (`rotate_once`; `mg_module::placement::point_fields`: a trigger's points are `PointX/PointY/PointZ`, an encounter's `X/Y/Z`; `edit.rs::changes` uses `point_fields`; the unit test and the Aurora capture have no encounter)
- **Read again by the reviewer:** holds: every `Geometry` list is read and set as `PointX`/`PointY` (reshape.rs:357-363).

### E3. Merging a GIT: a placed store changed on both sides is duplicated, and its panes are merged as placed objects

- **Kind:** bug
- **Severity:** medium (wrong merge, no conflict reported)
- **Where:** crates/mg-schema/src/merge.rs:48 (`("GIT", l) if PLACED.contains(&l)` comes first, and `git::STORE_LIST.label == utm::STORE_LIST.label == "StoreList"`), :65; crates/mg-gff/src/merge.rs:535-545 (the steps key on `TemplateResRef`)
- **Since:** before 2253fa78 for the label clash (abbaf914); the UTM rule that it shadows is after (14ddfe48)
- **Scenario:** (a) the rule gets only the file type and the label, so a placed store's own `StoreList` (its five panes, struct ids 0–4) in a GIT gets `ListRule::Placed`. Panes have no `TemplateResRef`/`Tag`/position, so a pane pairs only when identical. Ours adds a sword to the Weapons pane, theirs a bow: both sides "removed" the base pane and "added" one, the result has two panes of id 4, each with the base items. (b) A placed store names its blueprint in `ResRef`, so every `Placed` step's key is `None` for it: ours moves the store, theirs changes its markup, and the merged area has two stores.
- **Fix:** give the rule the depth or the parent list (`rules(file_type, path, label)`), so a nested `StoreList` is `ByStructId`; in `placed_steps` read `ResRef` where there is no `TemplateResRef`.
- **Test:** `merge.rs` unit tests: a GIT with one placed store, each side adding an item to the same pane (one pane, both items); one side moving it and the other changing `MarkUp` (one store).
- **Confidence:** confirmed by reading (`list_rule`; `mg_gff::merge::matching`/`placed_steps`/`fields_key`; the generated `git::store_list` has `ResRef`, no `TemplateResRef`; tests cover `list_rule("UTM", "StoreList")` only)

### E4. Renaming a control's ID or a bind in the Creator writes the new name raw into the builder's NWScript strings

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-nui/src/actions.rs:508-509, :528-529
- **Since:** before 2253fa78 (43e0ca51)
- **Scenario:** a handler's section holds `NuiSetBind(oPlayer, nToken, "ok", …)`. The builder types `"` into the bind's name (each keystroke is a rename): the code becomes `"ok""`. Deleting the quote does not bring it back (`"ok""` is no longer one string token that equals the old name), so the event script no longer compiles until fixed by hand. A backslash does the same; a name outside Latin-1 is written raw and becomes `?` in `text_bytes`. With an old name that is empty (an ID field cleared, then typed in) `quoted` is `""` and `inner` is `\"\"`: every empty string given to a `Nui*` call or compared, and every `\"\"` inside the builder's JSON strings, is rewritten to the new name (suspected for this last part: it needs the ID field to write `"id": ""` rather than drop the key).
- **Fix:** write the new name with `string_expr(to)` (and its JSON form inside strings via `ascii_json`), match the old name by decoding the token rather than by `"{from}"`, and do nothing for an empty `from`.
- **Test:** `actions.rs` unit test beside `rename_in_code`'s: rename `ok` to `o"k` and back gives the original script; `ok` to `żółć` yields a `JsonGetString(JsonParse(…))` argument; `from == ""` changes nothing.
- **Confidence:** confirmed by reading (`rename_in_code`, its caller `mg-ui/src/nui_view/events.rs:251`, `validate::plain_name`, which allows `"` and `\` in an ID; the generated guards go through `string_expr` and are right)

### E5. `Edit::InsertItem` panics on a list label over 16 bytes, and a refused insert leaves an empty list behind

- **Kind:** bug
- **Severity:** medium (a plugin's call takes the job down; "on failure nothing is changed" not kept)
- **Where:** crates/mg-edit/src/lib.rs:577-590
- **Since:** before 2253fa78 (cd074a54)
- **Scenario:** (a) a plugin calls `ctx.edit:insert("x.utc", "/AListNamedTooLongForGff", {…})` on a creature without that list: `made` is true and `s.set(list, …)` hits `expect("GFF label longer than 16 bytes")`. (b) `insert` at index 3 into a list the struct lacks: the list is created, then `index > len` returns the error; `run` reverts only earlier edits, the struct keeps the new empty list, the document is in `edited` and the next flush writes it.
- **Fix:** check `Label::new(list)` and the index before creating the list (create it only when `index == 0`).
- **Test:** add to `failed_commands_change_nothing`: an insert at index 1 into a missing list, and one with a 17-byte label, both `Err` with the document equal to before.
- **Confidence:** confirmed by reading (`run_edit`'s `InsertItem` arm against the `SetField` arm, which checks `Label::new`; `Struct::set` panics on a long label; `mg-plugin/src/runtime.rs:683-697` passes `field_from_str`'s label unchecked and `GffPath` parsing has no length limit; `wire.rs` does check, so `mg apply` is safe)

### E6. Advanced Search's Change acts on what was found before later edits: the wrong object can be changed

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-edit/src/found.rs:125-129 (`Reader::get` by path), :199-246; crates/mg-ui/src/search.rs:641-656
- **Since:** before 2253fa78
- **Scenario:** Find creatures with tag `GUARD` (a hit at `Creature List[5]`), switch to the area, delete creature 2 (or Undo something), come back and press Change with "Remove variable": `Creature List[5]` is now another creature and loses its variable. With a shorter list the edit fails; with a longer one it lands silently.
- **Fix:** keep `ws.revision()` with the hits and search again (or refuse, saying so) when it has moved before `found::edits`; or carry what identifies the object in the hit and check it in `Reader::get`.
- **Test:** app test in `tests/app` beside the Advanced Search ones: find, delete an earlier object of the list, Change: the object found is the one changed (or the window says to find again).
- **Confidence:** confirmed by reading (`mg-ui/src/search.rs`: `AdvancedSearch::hits` is filled only by Find and after a change, the window is a tab, `change_found` flushes and passes `draft.chosen()`; `found::edits` resolves each hit by `key` and `path` alone). Partly in another part (mg-ui).

### E7. A tile grid is allocated by the ARE's `Width` × `Height` before its tile count is looked at

- **Kind:** bug
- **Severity:** medium (panic or allocation abort on a hostile or broken ARE)
- **Where:** crates/mg-tiles/src/lib.rs:447 (and :331-333, :454 `y * width + x` in `u32`); crates/mg-edit/src/terrain.rs:20-31
- **Since:** before 2253fa78
- **Scenario:** an ARE with `Width = Height = 2147483647` and a short `Tile_List`: selecting a terrain brush, Change Tileset, Resize or a plugin's terrain call builds the grid; `vec![fill; (w+1)*(h+1)]` is 2^62 corners ("capacity overflow" panic); with 100000 by 100000 it asks for 80 GB plus the edge vectors (an allocation failure aborts, past the recovery handler) before the first `tiles.get(..)?` would have returned `None`.
- **Fix:** in `from_tiles` (or `Grid::new`) return `None` unless `tiles.len() == width as usize * height as usize` (checked), before allocating; index with `usize`.
- **Test:** `mg-tiles` unit test: `Grid::new(&index, u32::MAX, u32::MAX, vec![])` and `(100_000, 100_000, one tile)` are `None`.
- **Confidence:** confirmed by reading (`mg_edit::terrain::grid` takes any `u32`; `Grid::new` → `Lattice::from_tiles` → `Lattice::new` first; `AreaModel::read` clamps to 1024 but the painting grid does not)

### E8. The area scene keeps every look an object ever had (its GPU models) until the tab closes

- **Kind:** performance
- **Severity:** medium
- **Where:** crates/mg-area/src/scene.rs:287, :1387-1419 (`shown`), :490, :623 (`Models { cache: new }`)
- **Since:** before 2253fa78 (the per-object path that feeds it is after)
- **Scenario:** a builder drags the Tint section's color picker over a placed placeable: every distinct color is a new `Preview`, so a new cache entry with its own vertex and index buffers; a few hundred per drag, never freed while the area is open. The same for every appearance tried in a placed creature's Properties (about 18 models an entry) and every blueprint hovered as a ghost. An afternoon in one area holds hundreds of MB of GPU buffers.
- **Fix:** after `update`/`update_objects` drop cache entries nothing in `self.objects` holds (`Arc::strong_count == 1`, with their `failed` keys), keeping a few recent ones for the ghost; keep the parsed-model cache in the scene (cleared with Reload Resources).
- **Test:** unit/GPU test in `mg-area`: `update_objects` with 50 looks of one object leaves `object_cache.len()` bounded; `frame_perf`'s `refresh_parts` with a parts-based creature's look changed settles the second half.
- **Confidence:** confirmed by reading for the growth (`shown` keys `object_cache` by the preview's `Debug` text, nothing removes entries; `Composed::new` → `place` makes a new `GpuModel` per model per entry; `preview_variables` in `mg-ui/src/area_view.rs:2222` reads the object again for each color a picker passes); suspected for the cost of each miss (the `Models` cache is made anew in every `update`, `update_objects`, `ghost` and `preview_tiles` call, so a new look parses its models and their supermodel chain from the resources again)

## Low

### E9. Resize Area panics on an area whose ARE has a width or height of 0

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-area/src/reshape.rs:128-146
- **Since:** before 2253fa78
- **Scenario:** an ARE written by a tool with `Width 0`, `Height 0` and an empty `Tile_List` (the view shows it empty); the builder resizes it to 4 by 4 to repair it: `x.min(ow - 1)` underflows (debug) or wraps and `old.lattice.cell(x, u32::MAX)` asserts.
- **Fix:** return `None` (or fill the new area with the Default terrain) when `ow == 0 || oh == 0`.
- **Test:** `reshape.rs` unit test: `resize` of a 0 by 0 area returns `None` or a filled area, no panic.
- **Confidence:** confirmed by reading (`resize`: `ow - 1`/`oh - 1` in `u32`, then `Lattice::cell`'s assert; `Grid::new(index, 0, 0, [])` is `Some`)

### E10. `reshape::groups_in` is a second, drifted "which group stands here": groups whose first slot is empty are never found

- **Kind:** duplication (a drift that gives a wrong result)
- **Severity:** low
- **Where:** crates/mg-area/src/reshape.rs:36-70; crates/mg-tiles/src/lib.rs:252-289 (the source of truth)
- **Since:** before 2253fa78
- **Scenario:** `groups_in` anchors on `group.tiles.first()` being the tile under the cell, compares orientations without `% 4` and ignores heights; `group_at` starts from any slot and checks turn and height. For an L-shaped group whose south-west slot is empty, Resize cutting it does not turn its remaining tiles into terrain or take the doors on its hooks, and Change Tileset's count of groups misses it.
- **Fix:** build `groups_in` on `Grid::group_cells` (walk the cells, collect each whole group once, find its index from the tile).
- **Test:** `reshape.rs` unit test with a 2 by 2 group whose `Tile0` is `-1`, placed and cut by a resize.
- **Confidence:** confirmed by reading (`groups_in` against `TileIndex::group_at`); whether a shipped tileset has a group with `Tile0=-1` was not checked

### E11. Exact Tile with a group refuses to replace another group whole when the two share a tile in the same cell

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-tiles/src/paint.rs:158-170, used by :671-677
- **Since:** after (5b7e7904 made it reachable: `place_group_exact` passes many cells)
- **Scenario:** House A stands; the builder places House B (same size, sharing the roof tile in one slot, same turn and height) exactly over it. Every cell of A is covered, but the shared cell is "not changed", so `cells.iter().all(changed)` is false: A is reported broken and the placement refused, though it would take A away whole.
- **Fix:** for the "taken whole" test ask whether the cell is among the changes' cells at all; keep "tile differs" only for which groups are touched.
- **Test:** `paint.rs` unit test beside the Exact ones: two groups of two tiles sharing one tile; `place_group_exact` of the second over the first is `Some`.
- **Confidence:** confirmed by reading the logic (`groups_broken`: `changed(c)` is "in the changes and a different tile", and it is used both to pick the groups touched and for "all of the group is taken"); how many shipped groups share tiles was not checked

### E12. Undo of a field removed from a resource's root gives the file Moonglow's layout, not its own bytes back

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-edit/src/lib.rs:649-655, :516-531
- **Since:** before 2253fa78 (9f4bc8dc, the day of the review)
- **Scenario:** a module folder kept in git, an ARE as Aurora laid it out. Verify's "Move the variables to the GIT" (removes `VarTable`, not the root's last field), then Undo: the ARE in the module is now Moonglow's serialization of the same document; the next save rewrites the file and git shows it changed (what pull request 36 fixed for the other paths).
- **Fix:** in the `SetResource` arm (or after it in `undo`), when the new bytes read as the same document as `saved`'s, set `saved`'s bytes; or undo the root case by restoring `doc.root` through a struct-level inverse so `write_back` decides.
- **Test:** extend `an_edit_undone_gives_the_bytes_back` with a removal of a root field that is not the last.
- **Confidence:** confirmed by reading (`removed_in_place`'s root case returns `SetResource` of `doc.to_bytes()`; the `SetResource` arm sets those bytes and drops the document, so `write_back`'s "as saved: the file's bytes" never runs; `an_edit_undone_gives_the_bytes_back` covers only a last field)

### E13. Advanced Search's Remove Variable with a name the codepage cannot hold removes the variables with an empty name

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-edit/src/found.rs:231
- **Since:** before 2253fa78
- **Scenario:** a Windows-1252 module, the name typed in Cyrillic: `name_bytes` is empty and every variable whose `Name` is empty on the objects found is removed, reported as "Remove variable …".
- **Fix:** return the error `SetVariable` gives ("has characters the module can't hold").
- **Test:** `found.rs` unit test: an object with a variable of empty name, `RemoveVariable { name: "имя" }` is an `Err` and makes no edit.
- **Confidence:** confirmed by reading (`Change::RemoveVariable`: `codepage.encode(name)…unwrap_or_default()`, then `string("Name") == Some(&name_bytes[..])`; `SetVariable` returns an error for the same name)

### E14. An area's "N missing" models never shrinks

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-area/src/scene.rs:475-477, :497-499
- **Since:** before 2253fa78
- **Scenario:** a placeable with a missing model shows "1 missing"; the builder gives it another appearance or deletes it: the status line still says "1 missing: plc_x99" until the tab is reopened.
- **Fix:** compute `missing` from what the current tiles and objects resolve to (keep the names with the `None` cache entries, list those still referenced).
- **Test:** `mg-area` GPU test: update with an object of a missing model, then without it: `missing` is empty.
- **Confidence:** confirmed by reading (`AreaScene::missing` is only extended, in `update`/`update_objects`; shown by `mg-ui/src/area_view.rs:3125`)

### E15. Loops bounded only by numbers read from files

- **Kind:** bug
- **Severity:** low (a hang on hostile or corrupt content)
- **Where:** crates/mg-area/src/grass.rs:169 (`clumps = area(f) / 5` per walkmesh face, not thinned by `kept_on`); crates/mg-area/src/retile.rs:172-173 (`for level in 1..=highest`, `highest` from `Tile_Height as i32`); crates/mg-tiles/src/lib.rs:296 (`fits`: one hash lookup per step of the tileset's highest corner)
- **Since:** before 2253fa78 (retile.rs is after)
- **Scenario:** a hak's `.wok` with a face of 1e12 m² (or a non-finite corner: `inf.floor() as usize` is `usize::MAX`) hangs the window when the area is shown with grass; an ARE with `Tile_Height = 2000000000` makes Change Tileset loop two billion times over the lattice; a SET with a corner height of 2e9 does the same in every `fits`.
- **Fix:** skip faces that are not finite or larger than a tile (100 m²) in `tufts`; clamp `Tile_Height` in `terrain::grid` as `AreaModel::read` does (±1024); clamp `max_height` in `TileIndex::new`.
- **Test:** unit tests: `tufts` with a 1e9-wide face returns at once; `retile` on a grid with a tile of height `i32::MAX` returns.
- **Confidence:** confirmed by reading; whether `mg-mdl`/`mg-set` bound these values on reading was not checked

### E16. A fix that takes several items out in ascending order misleads the other fixes of the same Verify

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-edit/src/fix.rs:433-439, :359-362, :394-413
- **Since:** before 2253fa78
- **Scenario:** a plugin's fix removes items 3 and 4 of a list as `remove 3, remove 3`. `removed` holds `[3, 3]`. Another fix found on original item 4 (now gone) is not refused (`contains(4)` is false) and is moved to index 2: it lands on another object.
- **Fix:** in `made`, turn each removal's index back into the Verify-time place (add the count of this fix's earlier removals at or below it), or refuse to follow a fix with more than one removal of a list unless they descend.
- **Test:** `fix.rs` unit test: a fix with two ascending removals made, then a fix on the second removed item is `Err`.
- **Confidence:** suspected (by reading `Guard::made`/`check`/`path_now`: `removed` is documented as places "as Verify read them", but `made` records each `RemoveItem`'s index as the edit has it, which for the second removal of one fix is a place after the first). The doctor's own fixes remove one item or go from the end; a plugin's `ctx.fix` settles it.

### E17. NUI Import reads a whole archive into memory and keeps every script and text file of it

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-nui/src/import.rs:74, :119-131, :27-29
- **Since:** after (b8df5386)
- **Scenario:** Import… pointed at a 3 GB hak or a persistent world's `.mod` with 20,000 scripts to take two windows from: the file is read whole (3 GB), then every `.nss` and `.txt` is copied into `found` before `incoming` looks at which are beside a `.jui`.
- **Fix:** read the archive's table first (a file-backed ERF reader), list the JUIs, and read only each one's `NAME.txt` and `NAME_e.nss`; apply `MOST` to ERF entries too.
- **Test:** `import.rs` unit test: an ERF with one window and 1,000 other scripts leaves three entries in `found`.
- **Confidence:** confirmed by reading (`gather`: `whole(path)` then `Erf::read(&bytes)` for anything not a folder, `.jui` or zip; `kept` takes every JUI, TXT and NSS; `MOST` bounds only loose files and zip entries)

### E18. `edits_between` (and NUI `rename`) compare every resource's bytes to find what changed

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-edit/src/lib.rs:101-111; crates/mg-nui/src/rename.rs:131-138; crates/mg-nui/src/script.rs:460-462 is already limited to the keys it touched
- **Since:** before 2253fa78
- **Scenario:** any operation staged on a copy of the module (`Workspace::edits_to`, `build_renamed`, a window's rename) on a 600 MB module of 30,000 resources reads all 600 MB twice to find three changed scripts.
- **Fix:** compare `Module::shared` pointers first (`Arc::ptr_eq` is equal; only others are compared by content).
- **Test:** unit test: `edits_between` of a module and its clone with one resource set returns one edit; a counter or timing check in `frame_perf` style on a large synthetic module.
- **Confidence:** confirmed by reading (`after.get(k) != before.get(k)` over all keys; `Module` clones share their data, but slices compare by content)

### E19. Particles are kept by the object's place in the area's flat list: placing or deleting an object restarts every emitter after it

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-area/src/scene.rs:806, :819-826 (and :853, :945 for tiles, whose index is stable)
- **Since:** before 2253fa78
- **Scenario:** a tavern with 100 candle placeables; the builder places a creature (first in the list): all 100 keys change, 100 × 80 simulation steps run in the next frame, at every object placed or deleted.
- **Fix:** key by `(kind, index in its list)` plus the pointer, or carry the sims over by pointer when only places shifted.
- **Test:** `frame_perf`: `particles` after an object is added at the front of an area with 100 emitters stays inside the frame.
- **Confidence:** suspected (by reading: the key is `(i, Arc ptr)` with `i` the index in `AreaModel::objects`, which is every kind in a row, and a new key is run ahead 20 s in 80 steps; what one `Particles::update` costs was not measured)

### E20. A brush preview over many tiles is looked up linearly for every tile, three times a frame

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-area/src/scene.rs:1019, :1036, :1078, :1091
- **Since:** before 2253fa78
- **Scenario:** a paste or fill preview covering a 32 by 32 area: `hidden` has 1,024 entries and each of 1,024 tiles scans it up to three times: about 3 million comparisons a frame, a noticeable share of 6.9 ms.
- **Fix:** make `hidden` a bit set (`Vec<bool>` by tile index) once per call.
- **Test:** `frame_perf`: `scene_hiding` with every tile hidden against none.
- **Confidence:** confirmed by reading (`hidden.contains(&i)` in the tile loop, in the grass closure and in the chunk test; `failed_tiles` too)

### E21. The same tile and store facts are written in several places

- **Kind:** duplication
- **Severity:** low
- **Where:** (a) "the tile under a point": crates/mg-area/src/lib.rs:726-729 (`hook_near`, no check of `y`), crates/mg-area/src/walk.rs:229-236 (`Ground::tile_at`), crates/mg-area/src/edit.rs:289-293 (`tile_height_edits`); each multiplies `y * width` unchecked (a point 1.3e9 m north wraps onto row 0 in `tile_at`; a debug build panics). (b) a tile's door hooks placed in the area: crates/mg-edit/src/terrain.rs:84-111 (`typed_hooks`), crates/mg-area/src/lib.rs:421-442 (`tiles_hooks`), crates/mg-area/src/reshape.rs:185-202. (c) a tile's struct as written: crates/mg-edit/src/terrain.rs:54-65, crates/mg-area/src/reshape.rs:73-87, crates/mg-area/src/retile.rs:239-248. (d) a store's panes by struct id: crates/mg-edit/src/lists.rs:480 (`STORE_PANEL_PAGE`), crates/mg-rules/src/rows.rs:196 (`STORE_PAGES`). (e) a tile's corners and edges turned as it lies: crates/mg-area/src/border.rs:121-138 (`sides`), crates/mg-tiles/src/lib.rs:104-108, :211-217 (`TileIndex::cell`). (f) a tile model's source-light node: crates/mg-area/src/scene.rs:204-207 (`ends_with("sl1")`, first match) against crates/mg-area/src/lib.rs:139-152 (`<model>sl1` exactly): a node `torchesl1` is a flame's place for the scene and not for Tile Properties.
- **Since:** before 2253fa78
- **Scenario:** see (a) and (f) for where the copies differ.
- **Fix:** one `tile_at(width, height, p) -> Option<usize>` in `mg-area`; `typed_hooks` as the only hook placement (`tiles_hooks` and reshape built on it); one `tile_struct` in `mg-edit::terrain`; `STORE_PAGES` as the one table; `Loaded::new` using `TileFeatures`' naming.
- **Test:** unit test for `tile_at` with points outside every edge; a tile model with a node `xsl1` and a real `<model>sl1` puts the flame at the real one.
- **Confidence:** confirmed by reading; no wrong result found beyond finding 10 and the bounds noted

### E22. Numbers from files are narrowed with `as` and used in unchecked arithmetic

- **Kind:** hygiene
- **Severity:** low
- **Where:** crates/mg-edit/src/terrain.rs:26-28 (`Tile_ID … as u32`, `Tile_Height … as i32`), crates/mg-tiles/src/lib.rs:214 (`c.height += p.height`), :296; crates/mg-rules/src/items.rs:127-133 (`PropertyName as u16`, `CostTable as u8`…), crates/mg-rules/src/creatures.rs:122,161 and levelup.rs:112 (`Feat`/`Spell as u16`); crates/mg-edit/src/found.rs:436-437 with lists.rs:489 (`tx + tw` in `u32`); crates/mg-rules/src/levelup.rs:259 (`*rank += 1` on a `u8` when a creature's levels from the file pass 252)
- **Since:** before 2253fa78
- **Scenario:** an item property with `PropertyName = 70000` reads as property 4464 and is shown, costed and matched (`lists::property_key`) as that one; a `Tile_Height` near `i32::MAX` overflows in `TileIndex::cell` (a panic in a debug or test build, a wrapped height in a release).
- **Fix:** `try_from` with the value left out or refused where it does not fit; `checked_add`/`saturating_add` for heights and grid places.
- **Test:** unit tests per reader with out-of-range values (`ItemProperty::from_gff` with 70000 is not property 4464).
- **Confidence:** confirmed by reading (a scan of the casts; none found that corrupts a file)

### E23. The Appearance lists' marks are cached under the list's kind, length and first name

- **Kind:** hygiene
- **Severity:** low
- **Where:** crates/mg-rules/src/appearance.rs:397
- **Since:** after (the function is new since 2253fa78)
- **Scenario:** any second list of one kind of object that has the same length and first text (the same rows in another order, a filtered list that keeps the first row) gets the first one's cached rows back.
- **Fix:** key by the `Arc` of the list it was made from (`appearance_choices(kind, &Arc<Vec<Choice>>)`, pointer and length), or hash the rows.
- **Test:** unit test: two lists of one kind, same length and first name, other rows swapped, give different marked lists.
- **Confidence:** confirmed by reading the key; no caller found today that passes two different lists of one kind with the same length and first name

### E24. A command that removes one object of a list and places another is taken for a move: no duplicate-tag warning

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-edit/src/tags.rs:30-34, :54-58
- **Since:** after (tags.rs is new since 2253fa78)
- **Scenario:** a command that deletes creature 2 and inserts a new creature with the tag another already has (a replace, a paste over a selection, `mg apply` with both): the insert is passed over as "moved" and Duplicate Tags says nothing. A `SetField` of `Tag` after a removal in the same command is also read at the index as it was before the command.
- **Fix:** count an insert as a move only when an item equal to it (or of its tag and blueprint) was removed in the same command, pairing them one to one.
- **Test:** extend `a_command_s_tags_are_those_it_changes_or_places`: remove the barrel and insert a different placeable with tag `WP_A`: it is made.
- **Confidence:** confirmed by reading (`made_by`: `removes(key, list)` is true for any `RemoveItem` of that list in the command, and then no `InsertItem` into it counts)
- **Read again by the reviewer:** holds (drawn in the sample): `removes(key, list)` is true for any `RemoveItem` of the list in the command, and then no `InsertItem` into it counts (tags.rs:30-34, :54-58).

## Not read

- `crates/mg-rules/src/`: `items.rs` (only the lines changed since 2253fa78 and a scan of its casts), `creatures.rs`, `challenge.rs`, `item_wizard.rs`, `names.rs`, `spell_warnings.rs`, `placeables.rs` (not read beyond the same scan); `levelup.rs` after line 275; `objects.rs` after line 250; `tint.rs` after line 130.
- `crates/mg-nui/src/`: `schema.rs` (not read), `properties.rs` after line 135 (the tables), `actions.rs` lines 640–730 (the body of `fit_returns`), `lib.rs` lines 472–543 (`bind_names`, `element_ids`, `image_names`), `error.rs`.
- `crates/mg-area/src/marker.rs`.
- `crates/mg-schema/src/lib.rs` after line 250 (`root_fields` and the rest).
- Test modules were read only where a finding needed them.

## Looked at and found sound

- `Workspace::run`/`run_edit`: the inverses of `SetField`, `InsertItem` and `RemoveItem` (a field kept in place, duplicate labels, a nested struct put back whole, a list made by an insert taken away again), a failed command reverted, and `touched_since`/`changed_at`/`rewrites` (they only ever say too much, never too little).
- The NUI scripts Build writes: every string from a JUI or its settings (IDs, bind names, values, variant and group names, the window ID) reaches NWScript through `string_expr`, `json_string`'s `\\`/`\"`-only literals, `identifier` or `key_text`; a quote, backslash, line break or non-ASCII letter in a field cannot break or inject code there (finding 4 is the one place that writes a name raw).
- `mg-tiles/src/paint.rs`'s lookups: brushes go through `get_corner`/`get_cell`/`all_inside`, `place_tiles` and `place_group_exact` check every turned offset against the grid before touching it, `Grid::apply` refuses a stroke of another size, and `mg-set` holds a group to 32 a side.
- `GameData`'s caches (`tables`, `choices`, `marked`, `judged`, `codepage`): all dropped by `invalidate`, the name lists by `set_custom_tlk`, and the UI calls `invalidate` when a 2DA of the module layer or the haks change; `choice_list` hands out an `Arc`.
- Depth and shape of read JSON: `mg_nui::parse` goes through serde_json's 128-deep limit, so `validate`, `Names::of`, `find_group` and `last_draw_list` recurse no deeper; the validator indexes `Value`s only in ways that give `Null`, and `wire.rs` turns every bad label, index and key into an error.
