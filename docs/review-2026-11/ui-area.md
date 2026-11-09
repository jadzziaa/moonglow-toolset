---
type: Report
title: 'Code review of November 2026: the area editor'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the area editor (mg-ui): 29 in all (4 high, 8 medium, 17 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-ui, area-view, tiles]
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

# Code review of November 2026: the area editor

One part of [the review of 9 November 2026](overview.md): the area editor (mg-ui). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it. Paths are under `crates/mg-ui/src/` unless they say otherwise.
## High

### A1. A group brush kept across Reload Resources indexes the tileset's groups unchecked: panic

- **Kind:** bug
- **Severity:** high (crash, unsaved work lost)
- **Where:** terrain_mode.rs:607, 613 (`stroke`), 1439 (`cursor_shapes`): `&tools.set.groups[g]`
- **Since:** before 2253fa78 (613, 1439); 607 added by 5b7e7904 (today)
- **Scenario:** a tileset author has an area of their tileset open with its last group chosen in the tiles palette; removes that group (tileset editor's Remove Last Group, Save, the `.set` in development/ or a hak) and Reload Resources runs (asked, or the reload timer). The brush is still `Brush::Group(g)` for the same ResRef; the next frame the pointer is over the view, `preview` -> `click` -> `stroke` (and `overlay` -> `cursor_shapes`) index past the end.
- **Fix:** `groups.get(g)` in the three places (no stroke, no cursor), and drop `palette.tile_brush`/`tile_choice` in `forget_game_data` (a `Brush::Tile(t)`/`Terrain`/`Crosser` of the old set means another thing in the new one too).
- **Test:** app test (tiles.rs): choose the last group's brush, swap the view's `terrain` for `Tools` of the same set with that group removed (or reload from a scratch development folder), hover the view: no panic, brush gone.
- **Confidence:** confirmed by reading (`terrain_mode::active` 455-464 checks only the tileset's ResRef; `Moonglow::game_data_changed` lib.rs:5105-5124 and `PaletteView::forget_game_data` palette_view.rs:415-426 clear `tile_palettes` but not `tile_brush`/`tile_choice`; `view.reload()` makes new `Tools` from the new `.set`)
- **Read again by the reviewer:** holds for the three indexes (`tools.set.groups[g]`, terrain_mode.rs:607, :613, :1439); the Reload Resources path is the reader's.

### A2. Export Minimaps… joins the folder with an area's ResRef unchecked: writes outside the folder chosen

- **Kind:** bug
- **Severity:** high (path traversal, wrong file written)
- **Where:** minimap_export.rs:73-76 (`dir.join(name).exists()`), 95-98 (`dir.join(&name)` then `std::fs::write`)
- **Since:** after (237eb55c, 2026-11-05)
- **Scenario:** a downloaded `.mod` holds an area named `../../x` (7 bytes; a ResRef has room for 16). Its areas selected in the module tree, Export Minimaps… into `~/maps`: `~/maps/../../x.png` is written (created without a question; an existing one only after the "files already there" question, which shows the name).
- **Fix:** refuse an area whose `resref.is_file_name()` is false before either join (a `path_in` that takes the extension would do for both), and name it in the log. (`dialog_export::write_conversations`, the sibling of this path, is another reader's: same check.)
- **Test:** unit test of `write_minimaps` on a scratch module with an ARE keyed `../x`: nothing outside `dir`, an error in the log.
- **Confidence:** confirmed by reading (`ResRef::from_str` takes any 16 bytes but NUL; no `is_file_name`/`path_in` on the way from `Action::ExportMinimaps` lib.rs:4084-4087 through `ask_or_write` to the write)
- **Read again by the reviewer:** holds: `dir.join(format!("{resref}.png"))`, then `std::fs::write` (minimap_export.rs:73-76, :95-98).

### A3. Render Minimap Pictures writes `<ImageMap2D>.tga` wherever the `.set` says

- **Kind:** bug
- **Severity:** high (path traversal, wrong file written)
- **Where:** tileset_view.rs:1125-1132
- **Since:** before 2253fa78 (d864e8e3)
- **Scenario:** a downloaded tileset's tile has `ImageMap2D=../../../home/u/pictures/cat` or `=/home/u/pictures/cat`. Tools > Tilesets, open it, Render Minimap Pictures: `cat.tga` there is overwritten with a 32x32 picture.
- **Fix:** accept the name only if `ResRef::from_str(name)` holds and `is_file_name()`; go through `ResKey::path_in(&dir)`; name the refused ones in the log.
- **Test:** unit test: a `SetFile` with such a name, `render_minimaps` into a scratch folder: the file outside is untouched.
- **Confidence:** confirmed by reading (`image_map_2d` is the file's raw text, mg-set lib.rs:393; `PathBuf::join` with an absolute name replaces `dir`)
- **Read again by the reviewer:** holds: `dir.join(format!("{}.tga", name.to_lowercase()))` with the name the `.set`'s `ImageMap2D` (tileset_view.rs:1125-1131).

### A4. The view's minimap and Export Minimap divide by an ARE's `Width`

- **Kind:** bug (in mg-module, reached from this part)
- **Severity:** high (a panic on a module's file), though the line is another part's
- **Where:** crates/mg-module/src/minimap.rs:100; called from minimap.rs:63-67 and minimap_export.rs:33
- **Since:** after for the callers (237eb55c and the view's Minimap)
- **Scenario:** a crafted ARE with `Width` 0 and one tile: Export Minimap… from the module tree panics.
- **Fix:** in `minimap`: refuse a width or height of 0 or above 32 before anything is sized.
- **Test:** unit test in mg-module: such an ARE gives `Err`.
- **Confidence:** confirmed by reading `mg_module::minimap::minimap` (crates/mg-module/src/minimap.rs:61-70, 95-101: `i as u32 % width` with `Width` 0 and a non-empty `Tile_List`; `width * size` is also unchecked for a huge `Width`); whether the view opens such an area at all was not traced, the export path needs no view
- **Read again by the reviewer:** holds (the same lines as V1).

## Medium

### A5. The area view's selection is not carried across Undo, Redo or any edit that inserts or removes objects before it

- **Kind:** bug
- **Severity:** medium (the wrong object is selected, then moved or deleted)
- **Where:** area_view.rs:473 (`selection`), 2514
- **Since:** before 2253fa78
- **Scenario:** placeables P0..P9. Delete P2; click what was P5 (now index 4); Ctrl+Z. P2 is back at 2, the selection still says 4: the yellow box jumps to what was P4, and Delete, a drag or Q/E act on it. The same after Redo, a plugin's edit, a delete made from the module tree or another view of the area.
- **Fix:** give `blueprint::follow::followed`/`retarget` the open views' selections as places (`GffPath::root().item(list, index)`), as it has the Properties windows'; where a place is gone, drop it.
- **Test:** app test (area_view.rs): three waypoints with tags, delete the first, select the last, Undo: the selected object's tag is the same.
- **Confidence:** confirmed by reading (`show` 2514 keeps every `(kind, index)` that still exists; the Undo/Redo handler lib.rs:3812-3855 retargets only Properties windows through `blueprint::follow`; no test holds it: tests/app/area_view.rs:361 only clicks again)

### A6. Windows and tools that hold objects by list index while the area can change under them

- **Kind:** bug
- **Severity:** medium (an edit lands on another object)
- **Where:** Adjust Location: area_tools.rs:23, 101, 490-527 (and its preview 424-451); Bake Walkmesh: bake_view.rs:61-63, 328-338 (its `plan` too is "worked out as the window opened" and never again); Change Light: light_maker.rs:39, 279-281; Redraw Polygon: area_view.rs:592, 7349-7352, 5111-5137, 7746-7778; the eraser's stroke: paint_mode.rs:66, 637-650; Ctrl + wheel's `WheelScale.selection`: finding 13
- **Since:** Adjust Location, Redraw Polygon before 2253fa78; Change Light 192a4e4d, eraser 207dff5e (both 2026-10-29); Bake 9aab919c (2026-11-02)
- **Scenario:** Adjust Location open on placeable 3 (it shows a live preview, so the view is worked in meanwhile); placeable 1 is deleted, or Ctrl+Z puts one back before it; OK: X, Y, Z go to what is now at index 3. Bake: the baked row (a copy of the first object's model) is set as `Appearance` of another placeable, which then looks like the first; or the object is scaled again with the window open and the old plan's walkmesh is baked. Change Light: another placeable becomes an invisible light. Redraw Polygon begun on trigger 3, Ctrl+Z between two corners: the double click writes `Geometry` of the trigger now at 3.
- **Fix:** the same follow as finding 5 for each (`AdjustLocation.objects`, `BakeDraft.objects`, `LightDraft.change`, `AreaView::redraw`), closing the window or tool where its object is gone; `BakeDraft` also keeps the revision its plan was made at and plans again when it moved.
- **Test:** app test: Adjust Location on the second of two tagged placeables, the first deleted by a command, Apply with X changed: the object of the second's tag moved (or the window closed).
- **Confidence:** confirmed by reading (`widgets::dialog` 3018 is a plain window, not modal; none of these is among what `blueprint::follow::retarget` moves; `delete_objects` 6381-6399 closes Properties tabs only)
- **Read again by the reviewer:** holds for Adjust Location (drawn in the sample): `a.objects` are `(kind, index)` pairs applied at OK (area_tools.rs:499-503), and nothing in `blueprint/follow.rs` moves them.

### A7. An object edited, then set back while its tab is hidden, stays as edited in the view after a Save

- **Kind:** bug
- **Severity:** medium (the view shows, and later writes from, a place the module doesn't have)
- **Where:** area_view.rs:2298-2336, 2362-2385
- **Since:** after (3ed76b05, 2026-11-07)
- **Scenario:** in area A move a placeable (first path: model updated, `read_from` still the GIT before). Bring another tab in front of A; Ctrl+Z there (the move is undone: the GIT is as `read_from` again); Ctrl+S. Back to A: `touched_since` is `None`, `changed_objects(read_from, now)` is empty, "nothing of it changed": the model keeps the placeable where it was moved to. A later Together turn, Arrange or drag of a selection with it writes positions worked out from there.
- **Fix:** the first path remembers what it read again (a small `Vec<(ObjectKind, usize)>`, cleared whenever `read_from` is set) and the second path adds those to `changed`; or the first path sets `read_from = None` (a whole read at the next unknown revision).
- **Test:** app test: move, open a second tab in front, Undo, Save, select the area's tab: `area_views[&area].model` position equals the GIT's.
- **Confidence:** confirmed by reading (`refresh` 2298-2336 reads touched objects again without updating `read_from`; 2362-2385 then compares the stale `read_from` GIT with the current one; `Workspace::touched_since` mg-edit lib.rs:226-243 is `None` after `derive`, which every Save calls, lib.rs:4353-4356, and after 64 revisions). Not run.

### A8. Results kept by index and path, acted on after the module changed

- **Kind:** bug
- **Severity:** medium (Update Instances remakes an instance the builder unticked) to low
- **Where:** Update Instances: bulk.rs:20-34, 127-190; Find and Replace Text: bulk.rs:275, 314-341; Find Instance: area_tools.rs:556-562, 670-673, with area_view.rs:2709-2720
- **Since:** before 2253fa78
- **Scenario:** Update Instances lists ten instances; the builder unticks #7 to keep its own tag and scripts, deletes an object before it in that area with the window open, presses Update: the unticked one, now #6, is remade from the blueprint. Replace in N: a hit's path names another object's field of the same label, replaced if it has the text. Find Instance: a double click goes to another object; and where the result's object is gone `app.area_focus` is never cleared (2709-2720 clears it only when found), so the view jumps to the next object placed that gets that index.
- **Fix:** each draft keeps `ws.revision()` (or `changed_at` of the GITs) from when it searched; changed since: "Find again" (as Find and Replace already says for a changed query), nothing applied. Clear `area_focus` when the area's model is read and the object isn't there.
- **Test:** unit test: an `UpdateDraft` made, a `RemoveItem` before an unticked entry applied, `apply_update`: 0 updated and a note (or the same objects as ticked).
- **Confidence:** confirmed by reading (`mg_module::instances::update` instances.rs:1204-1212 asks `which(list, i)` of the GIT as it is at Apply; none of the three windows records the revision its results were found at)

### A16. Work per frame that is the selection times the area's objects

- **Kind:** performance
- **Severity:** medium
- **Where:** `object_at` area_view.rs:1212-1214 is a scan, under `selected_indices` 1225, `selected` 1229-1232 (`selection.contains`), `scaled_by` 1736-1739, `middle` 2012-2021 and `dragged` 2037-2133 (two passes, plus `with_turn`'s `find`), `turned` 6612-6637; `moved` 1379-1386 asks the walkmesh twice an object a frame (`lift_of`, then `height`); 3504-3510 clones the whole `AreaModel` every frame of a drag or key turn; `area_tools::previewed` 430-433; the context menu's three filters 7088-7095, 7256-7265, 7316-7325 each frame it is open; the selection's boxes are 12 egui segments an object (4556-4574)
- **Since:** before 2253fa78
- **Scenario:** a 32 by 32 area of 5,000 placed objects, all in the selection (a box over the area, or the module tree's Go To on them), dragged to make room after Resize Area: each frame 25 to 50 million comparisons, 10,000 walkmesh lookups, a clone of 5,000 objects and 60,000 line segments. The same with Adjust Location or the context menu open on them.
- **Fix:** a map from `(kind, index)` to the place in `model.objects`, made in `show` (and a `HashSet` for `selected`); the lifts worked out once as a drag begins; pose the dragged objects in a per-drag copy of only those; boxes of a large selection as one mesh or drawn in the scene.
- **Test:** frame_perf: a synthetic area of 5,000 placeables, "all selected, dragged" and "all selected, context menu open", held to the budget.
- **Confidence:** confirmed by reading that it is quadratic; the times are estimates (frame_perf's "all selected" is the largest campaign area, 549 objects: at 5,000 each pass is 80 times that)

### A17. The scatter brush's next dab is worked out anew every frame, and costs count x 30 x what is placed

- **Kind:** performance
- **Severity:** medium
- **Where:** paint_mode.rs:180-236, 377-394 (`around` walks every object), 413-426, 491-536; the ranges at 109 (radius to 50 m) and 931 (density to 400 a tile)
- **Since:** before 2253fa78 (c53920ea)
- **Scenario:** a forest: radius 25 m, density 50 a tile is 982 placeables a dab: between 1,000 and 29,000 tries (the more the fuller the circle), each against up to 1,000 neighbours, and 982 ghost objects built, every frame the pointer rests on the view, before any press. The ranges allow 31,000 a dab.
- **Fix:** keep the preview by (centre snapped to a few centimetres, seed, settings, revision, pending count); a grid of cells `spacing` wide for the neighbour test; show at most a few hundred ghosts and say how many more.
- **Test:** frame_perf: Paint on at radius 25 m and density 50 over an empty area, pointer still: in budget.
- **Confidence:** confirmed by reading (`ghosts` 524-527 calls `next_dab` each frame the pointer is over the view; `dab` 217-234 tries up to `count * TRIES` places, each against every `taken`; then one `scene.ghost` an object, area_view.rs:3427-3430)

### A18. Lists drawn whole every frame

- **Kind:** performance
- **Severity:** medium
- **Where:** Find Instance's results area_tools.rs:648-667 (four labels a row, three `String`s made a row), and 598-607 (every area's label a frame); Update Instances bulk.rs:246-250; Find and Replace Text's hits bulk.rs:441-452 (and the regex compiled each frame at 424-426); Edit Areas Together area_props.rs:1211-1230 (every area's `AreaInfo` cloned and sorted a frame, `chosen.retain` areas x chosen at 1225), 1393-1394 (`shows` clones the filter text and the info an area), 1410-1423; Area Properties > Visual's area list made and sorted every frame whether its menu is open or not, 496-503, 520-527
- **Since:** before 2253fa78 for most; the chooser's rows unchanged since 597b5fbc
- **Scenario:** Find Instance with nothing typed in a module of 300 areas of 150 objects lists every placed object: 45,000 rows, 180,000 labels a frame. Edit > Update All Instances…, or Find "the": tens of thousands of rows. Edit Areas Together in a world of 3,000 areas: 3,000 clones, a sort and 3,000 checkboxes a frame.
- **Fix:** `ScrollArea::show_rows` with a fixed row height in each; the chooser's and the Visual page's area lists made once a revision.
- **Test:** frame_perf: "Find Instance, 45,000 found", "Update Instances, 30,000 listed", "Edit Areas Together, 3,000 areas".
- **Confidence:** confirmed by reading (no `show_rows`, no `is_rect_visible`; the house rule is that lists skip rows out of sight)

### A19. Update Instances looks every object of every area up in the whole list of affected objects

- **Kind:** performance
- **Severity:** medium
- **Where:** bulk.rs:155-168
- **Since:** after (f9512481, 2026-10-29)
- **Scenario:** Update All Instances in a module of 90,000 placed objects with 30,000 affected: 2.7 thousand million closure steps on the window's thread, seconds of a frozen window; ten times the module, a hundred times that.
- **Fix:** per area, a `HashSet<(&str, usize)>` of the ticked places and a set of the ticked `Held` paths, made once before the loop.
- **Test:** unit test with a counter, or a timing in frame_perf: 300 areas of 300 objects, all ticked, applied in under a second.
- **Confidence:** confirmed by reading (`which` and `held` scan `draft.objects`; `mg_module::instances::update` calls `which` for every item of every list, instances.rs:1204-1206)

## Low

### A9. A pasted tile keeps the lights and loops copied but not the Replacement Texture; a painted tile inherits the old tile's

- **Kind:** bug (a list written twice that drifted)
- **Severity:** low (medium for a builder who uses replacement textures: a value past the table "crashes the game as the area loads", tile_select.rs:1489)
- **Where:** terrain_mode.rs:887-895 (kept labels) against tile_select.rs:177-186 (`FIELDS`); paste tile_select.rs:487-501
- **Since:** before 2253fa78 (the field's control acb4c370)
- **Scenario:** a tile with Replacement Texture row 3 is copied and pasted: the copy is drawn with whatever `Tile_ReplaceTex` the cell's old tile had (or row 0). Painting another tile over one that had the field leaves the old row on the new tile.
- **Fix:** `tile_command_made` keeps by `tile_select::FIELDS` (one list); `tile_edits` removes `Tile_ReplaceTex` where the tile's id changes and nothing is kept for the cell.
- **Test:** app test (tiles.rs): copy a tile whose struct has `Tile_ReplaceTex` 4, paste elsewhere: the pasted struct has 4.
- **Confidence:** confirmed by reading (`tile_edits` mg-edit/src/terrain.rs:54-65 never writes `Tile_ReplaceTex`; the kept list has seven labels, `tile_select::FIELDS` eight)

### A10. Create Set writes the waypoints' tags as UTF-8 bytes

- **Kind:** bug (two writers of a tag that drifted)
- **Severity:** low (medium for the Polish and other non-English modules of issues 13 and 27)
- **Where:** area_view.rs:7842 (`Value::String(tag.into_bytes())`)
- **Since:** before 2253fa78 (6ca48d21)
- **Scenario:** Create Set named `Straße`: tags are stored as UTF-8, shown by the game and by Moonglow as `StraÃŸe_01`; a second Create Set of that name finds no `Straße_01` among the decoded tags and makes `_01` again.
- **Fix:** `Value::String(crate::text::encode(&tag))`.
- **Test:** unit test with the game codepage set to Windows-1250: the edit's bytes are the codepage's.
- **Confidence:** confirmed by reading (`create_waypoint` 7812 uses `crate::text::encode`; `waypoint_tags` 7789 reads with `text::decode`)
- **Read again by the reviewer:** holds (drawn in the sample): `Value::String(tag.into_bytes())` (area_view.rs:7842).

### A11. Objects, pastes and the start location are put on the ground plane beyond the area, kilometres off near the horizon

- **Kind:** bug
- **Severity:** low
- **Where:** area_view.rs:1275-1278, 1394-1398 (`put_down_at`), 5458-5461 (`Drag::Move`), 5100-5104 (paste), 5152-5156 and 7450-7460 (start location), 5348 (`menu_at`)
- **Since:** before 2253fa78
- **Scenario:** with the view pitched low, a drag of the selection strays above the tiles' far edge and is let go: the objects are moved to X 4,000 and more (Undo brings them back, but nothing says where they went). A right click on the void, Set Start Location Here: the module starts off its walkmesh. Aurora refuses both.
- **Fix:** one `AreaView::area_point(pos)` that is `None` (or clamped to the area's rectangle) outside `0..width*10, 0..height*10`, used by placing, pasting, the start location and `Drag::Move`.
- **Test:** unit test: `put_down_at` of a screen point over the void is `None`; app test: a Move drag ended above the horizon leaves the object inside the area.
- **Confidence:** confirmed by reading (`ground_at` 1275-1278 falls back to `ray.at_height(z)`, which has no bound; no caller checks the area's size; the tile tools do, `terrain_mode::spot` 431-438 and `tile_select::tile_at`)

### A12. A drag outlives a change of mode or tool made by a key, and is committed later

- **Kind:** bug
- **Severity:** low
- **Where:** area_view.rs:5020 (`tool_keys`), 5039-5070, 5096-5108, 5551-5640; terrain_mode.rs:1036-1037 and 1087 (`terrain_drag` stays when the brush becomes one that doesn't take it); tile_select.rs:321-351 (`tile_box` stays when tile mode goes off)
- **Since:** before 2253fa78; the tile tools' keys after
- **Scenario:** dragging a placeable, the builder presses R (Rotate Tile's key). The tile brush is chosen, the view returns before the drag's code: the button let go commits nothing, the placeable stays drawn where it was dragged (the GIT has it where it was); Escape puts the brush down and the Move is applied then. With Ctrl+V during a drag the stale offset is applied to the pasted objects after the paste's click. F10 during a drag of the start location's marker moves it when tile mode is next left.
- **Fix:** one place that ends what is in hand (`view.drag`, `terrain_drag`, `tile_box`, `paint.pending`) whenever the mode or the tool changes, called from `tool_keys`, F10, B and where `pasting` is set; or run the release code before the mode branches.
- **Test:** app test: press on an object, move, press the Rotate Tile key, release: either one Move step and no drag, or no step and the object drawn where the GIT has it.
- **Confidence:** confirmed by reading (`input` returns at 5039-5070 and 5107 before the drag's own code at 5363-5640; `view.drag` is cleared only there and at 2744; `follows` 3439 keeps drawing it)

### A13. A Ctrl + wheel scaling still under way is applied to the next object in the list after Delete or Cut

- **Kind:** bug
- **Severity:** low
- **Where:** area_view.rs:6255-6263, 6374-6377; lib.rs:4219
- **Since:** before 2253fa78 (43dd07ba)
- **Scenario:** Ctrl + wheel on placeable 3 of 6, then Ctrl+X (or Delete with Ctrl still down) within 0.4 s: next frame the selection differs, the scaling is "committed" to `(Placeable, 3)`, which is now the old 4: a Scale step on an object never touched. (`commit_key_turn` 6671 has the mirror quirk: it turns the selection as it is now, not `KeyTurn.selection`, which is only compared; the events that commit it first hide it.)
- **Fix:** `apply` commits wheel scalings as it does key turns (`commit_wheel_scales`), and `delete` drops or commits both before it empties the selection; `commit_key_turn` turns `k.selection`.
- **Test:** app test: two placeables, Ctrl + wheel over the first, Delete in the next frame with Ctrl held: the second's `VisTransformList` is unchanged and the only step is Delete.
- **Confidence:** confirmed by reading (`wheel_scales` 6259-6262 runs `scaled_by(&wheel.selection)` against the model as read after the delete; `Moonglow::apply` lib.rs:4216-4221 commits Q and E's turning first but not the wheel's; Undo drops it, Redo doesn't)

### A14. Alt + drag moves the view in object mode only

- **Kind:** bug (the same gesture written once a mode, drifted)
- **Severity:** low
- **Where:** paint_mode.rs:619-631 (its comment says "Ctrl + drag, Alt + drag move the view": Alt + press lays nothing and moves nothing); tile_select.rs:321 (Alt + drag begins a tile box); terrain_mode.rs:952-960 (Alt + drag paints)
- **Since:** paint c53920ea (before 2253fa78)
- **Scenario:** issue 25's Alt + left drag with nothing selected pans; with Paint on, Select Tiles on or a terrain brush in hand it does nothing, selects tiles or paints.
- **Fix:** make the pan in `camera_input` (Alt + left with no object selection, the press remembered as `camera_press` is), so every mode has it and each mode's press test is `view.camera_press`.
- **Test:** app test: Paint on, Alt + drag: the orbit's target moved, nothing pending.
- **Confidence:** confirmed by reading (`camera_input` 5795 pans for Ctrl + left and the middle button only; `Drag::Pan` is made at 5414-5420, which Paint, tile mode and a tile brush return before)

### A15. The tileset's grass shown by Area Properties > Grass is cached for the life of the process

- **Kind:** bug (a cache not dropped)
- **Severity:** low
- **Where:** area_grass.rs:51-60
- **Since:** before 2253fa78 (4bdb6804)
- **Scenario:** a tileset's `[GRASS]` is changed and resources reloaded: the page still says the old texture, density and height, and what a drag "tries out" in the view (`growing_over(own)`) grows the old grass.
- **Fix:** keep it on `Moonglow` beside `tileset_names` and clear it in `PaletteView::forget_game_data`, or key it by the tileset's `Arc` from `app.tileset`.
- **Test:** unit test: read, change the set in a scratch resman, `game_data_changed`, read again: the new look.
- **Confidence:** confirmed by reading (egui temp data keyed by the tileset's ResRef, never removed; `game_data_changed` doesn't know it)

### A20. The Position menu reads every selected placeable's struct, and its walkmesh, every frame it is open

- **Kind:** performance
- **Severity:** low
- **Where:** bake_view.rs:110-157 (called from area_view.rs:7081)
- **Since:** after (9aab919c, 2026-11-02)
- **Scenario:** 2,000 trees selected, context menu > Position (for Drop to Ground or Random Facing): 2,000 struct clones a frame; in a project with `[bake]` and scaled placeables, as many walkmesh files read and parsed a frame.
- **Fix:** work `why_not` and `baked` out once when the menu opens (keyed by the selection and the revision), and read only `Appearance` and the transform from the document in place.
- **Test:** frame_perf: a selection of 2,000 placeables, the Position menu open.
- **Confidence:** confirmed by reading (`placed` 76-81 clones the GIT struct; `walkmesh_of` crates/mg-module/src/bake.rs:477-488 reads and parses the `.pwk` each call; the `baked` list at 151-157 clones each struct again, with or without `[bake]`)

### A21. placeables.2da is walked whole every frame while a light is being placed or the area's menu is open

- **Kind:** performance
- **Severity:** low
- **Where:** area_view.rs:7599-7616 (`stock_lights`), 7672-7693, called at 7254 (context menu, each frame), 3195-3196 (the status line, each frame of placing), 5189
- **Since:** before 2253fa78 (a436c5e6)
- **Scenario:** a persistent world's placeables.2da of 30,000 rows: 60,000 to 120,000 lookups a frame (an estimated 2 to 5 ms of the 6.9), for a list of some ten lights.
- **Fix:** keep the lights in `Kept`/the palette's game-data cache (dropped by `forget_game_data`), as the palettes' lists are.
- **Test:** frame_perf: "placing a light, placeables.2da +30,000 rows".
- **Confidence:** confirmed by reading (two hashed cell lookups a row; `light_of_row` may walk it twice)

### A22. The tileset editor finds a section by scanning the file from its first line, some thirty times a frame; and keeps a whole copy of the file for every undo step

- **Kind:** performance
- **Severity:** low
- **Where:** tileset_view.rs:382, 391, 404, 419, 729-745, 775, 916-967 (`tile_details`); 109-121 (`change` pushes `self.file.clone()`, and parses the whole file again)
- **Since:** before 2253fa78
- **Scenario:** The Domains of Dread's `tkc02` (1,872 tiles, some 28,000 lines) with tile 1,800 chosen: about 30 scans of 27,000 lines a frame (an estimated 3 to 8 ms). An afternoon of door hooks, 500 changes: 500 copies of the file's lines.
- **Fix:** `tile_details` takes the section's line range once (`SetFile::section(name) -> a view with get/set`), or `SetFile` keeps an index of headers made on change; undo keeps the lines a step changed.
- **Test:** frame_perf or editor_perf: the Tiles page of a 2,000-tile set with its last tile chosen.
- **Confidence:** confirmed by reading (`SetFile::body` crates/mg-set/src/edit.rs:74-84 is a scan with a `trim` a line; each `text_key`, `flag_key`, `int_key`, `name_key`, `float_key` is one `get`)

### A23. A terrain drag's preview paints every marked corner again from the first each time the pointer reaches a new one

- **Kind:** performance
- **Severity:** low
- **Where:** terrain_mode.rs:778-794, 1163-1203
- **Since:** before 2253fa78
- **Scenario:** Shift + drag of a terrain over a 32 by 32 area: 1,089 corners, each new corner under the pointer runs 1,089 strokes on a fresh grid, and the 1,024 preview tiles are cloned from the cache every frame (735, 3361-3368).
- **Fix:** keep the grid as the drag left it and apply only the corners added (a path run back, or the Shift rectangle changed, starts over); hand the cached preview out by `Arc`.
- **Test:** frame_perf: a Shift rectangle dragged corner to corner of a 32 by 32 area.
- **Confidence:** suspected (the replay is confirmed, 783-786 and 1180-1200, with a lattice clone a stroke and `changed.contains` at 1193; what a stroke costs in mg-tiles was not measured: time a Shift rectangle over a whole area)

### A24. The tile selection's outlines ask the walkmesh for every tile every frame

- **Kind:** performance
- **Severity:** low
- **Where:** tile_select.rs:2026-2031; area_view.rs:1305-1314, 1337-1358
- **Since:** after (c0a16eb6, 2026-10-28)
- **Scenario:** all 1,024 tiles of a mountain area selected to set their lights in Tile Properties: up to 80,000 lookups a frame; the points depend on the selection and the ground, not on the camera.
- **Fix:** keep each selected tile's outline in area space, made when the selection or the model changes; project it each frame.
- **Test:** frame_perf: "32 by 32, every tile selected" on a tileset with height transitions.
- **Confidence:** suspected (5 lookups a flat tile, 80 a tile that isn't, each a scan of the tile's faces: area_view.rs:1343-1358; the flat case was measured in the sprint, the sloped one not: measure a hill tileset's area with every tile selected)

### A25. Create Waypoint, Create Set and Find Instance parse every area's GIT on the window's thread and keep them all

- **Kind:** performance
- **Severity:** low
- **Where:** area_view.rs:7781-7794 (`waypoint_tags`, from 7800 and 7830); area_tools.rs:566-592
- **Since:** before 2253fa78
- **Scenario:** one click on Create Waypoint in a world of 2,000 areas: every GIT read into the workspace (seconds, and the memory stays until the next save).
- **Fix:** read the tags from the module's bytes without keeping documents (as `app.area_holds` does for Edit Areas Together), flushed first; kept by revision.
- **Test:** an app test counting `ws.loaded` documents after Create Waypoint: only the area's own.
- **Confidence:** confirmed by reading (`Workspace::doc` parses and keeps, mg-edit lib.rs:326-331)

### A26. Folders listed from disk every frame

- **Kind:** performance
- **Severity:** low
- **Where:** area_props.rs:505 (the Visual page, every frame it shows); prefabs.rs:195 (Save as Prefab); light_maker.rs:136 with 71-83 (Custom Light: `find_hak` for every hak of the module a frame)
- **Since:** mixed
- **Scenario:** Area Properties > Visual open beside the view: a directory read 144 times a second; a data folder on a network home makes each a round trip.
- **Fix:** read once as the page or window opens and after its own Save, Rename and Delete (as `AreaChooser::sets` does, area_props.rs:1099-1101).
- **Test:** unit test with a counting directory lister, or none: by reading.
- **Confidence:** confirmed by reading (`Shelf::list` saved.rs:26-36 is a `read_dir`; `GameInstall::find_hak` once a hak)

### A27. Files written in place

- **Kind:** hygiene
- **Severity:** low
- **Where:** tileset_view.rs:192-195 (the palette beside a `.set` at Save, after the `.set` itself went through `.set.part`), 1071-1074 (Make Palette), 1132; bake_view.rs:405-416 (`put`: the project's rows file at Undo, Redo and take-back); minimap_export.rs:50, 98
- **Since:** mixed
- **Scenario:** a crash or a full disk during Save of a tileset leaves half a `palstd.itp`; during Undo of a bake, half a rows file, which the project's build reads.
- **Fix:** the workspace's write-then-rename helper for the palette and the rows file (exports matter less).
- **Test:** none (by reading), or the helper's own.
- **Confidence:** confirmed by reading

### A28. Comments that say what the code doesn't, and doc comments on the wrong item

- **Kind:** hygiene
- **Severity:** low
- **Where:** area_view.rs:6-18 (the module's header: "Shift + right drag turns it", "a middle drag turns the camera (Shift: moves it)"; the code is Alt + right drag at 5443-5450 and Alt + middle at 5782-5785), 1174-1175 ("thousands selected, by Ctrl+A": the view has no Select All), 1316-1322 (`on_ground`'s comment sits on `square_on_screen`; `on_ground` at 1360 has none), 4185-4190 (`walkmesh_button`'s comment sits on `toolbar_menu`); tile_select.rs:1012-1016 (`ui`'s first line sits on `tiles_moved`); area_props.rs:199-203 (`page_fields`' comment split around a `use`), 486-491 (`apply_scheme`'s comment sits on `lighting_buttons`); paint_mode.rs:619-620 (finding 14). Command names in Edit > Undo mix cases: "Set start location", "Reverse door", "Add spawn point", "Redraw polygon", "Create waypoint", "Create set", "Adjust location", "Add to palette" beside "Move Start Location", "Turn Spawn Point", "Drop to Ground", "Paint Placeables", "Rotate Area".
- **Since:** mixed
- **Scenario:** a reader takes the header's bindings for the code's.
- **Fix:** move each comment to its item, rewrite the header from `input` and `camera_input`, title case for the command names (they are menu text).
- **Test:** none.
- **Confidence:** confirmed by reading

### A29. Written more than once

- **Kind:** duplication
- **Severity:** low
- **Where:** (a) which tile a ground point is in: `terrain_mode::spot` 431-451 (half a tile of grace, clamped), `tile_select::tile_at` 283-289 and `boxed_tiles` 295-308, `AreaView::ground_height` area_view.rs:1307-1311, the pointer's readout 3630-3633, `focus_at` 1290-1298; `minimap.rs:18` has its own `TILE = 10.0` beside `mg_area::TILE_SIZE`. (b) the tile fields a cell keeps: finding 9 (drifted). (c) `spawn_arrow_at` and `spawn_foot_at` area_view.rs:1818-1859, the same filter chain twice, and the "over a handle" test repeated at 4449-4490. (d) the triangle fan of a screen polygon three times: 3867-3873, 4119-4125, 4137-4143. (e) `light_button` and `light_toggle` 7619-7637. (f) the pointer for what follows it: `pointer_over` (not under a window) for the ghosts at 3390, `pointer_hover_pos` for the paste's outlines at 4578, the door hooks at 4613 and an outline's next edge at 4796 (drifted: outlines are drawn under a window in front where the ghosts are not).
- **Since:** mixed
- **Scenario:** (f) with the pointer over the minimap or a Properties window while pasting, the boxes show under the window and the see-through copies don't.
- **Fix:** (a) `AreaModel::tile_at(point) -> Option<(u32, u32)>` as the one source, `spot` built on it; (f) `view.pointer_over(ui)` in `overlays`; the rest as small helpers.
- **Test:** (f) app test: pasting, pointer over the map's rectangle: nothing of the paste drawn.
- **Confidence:** confirmed by reading

## Not read

Nothing of the part's code. The `#[cfg(test)]` modules (area_view.rs:8018-8219, paint_mode.rs:987-1323, terrain_mode.rs:2174-2270 and the smaller ones) were skimmed only, as the brief has it. Outside the part, read only as far as a finding needed: `blueprint/follow.rs`, `mg-edit` (`touched_since`, `derive`, `doc`), `mg-area` (`changed_objects`, `touched_objects`, `pick`, `walk`), `mg-set/edit.rs`, `mg-module` (`minimap`, `instances::update`, `bake::walkmesh_of`), the Undo and `apply` arms of `lib.rs`.

## Looked at and found sound

- Rays and projections: `Ray::at_height` refuses a level ray and one going away, `ray`/`aspect` guard a view of no size, `project_by` refuses what is behind the near plane, `near_segment` a piece of no length, `distance_along` and `angle_on` the edge-on cases; pitch is clamped, Spin and Tilt wrap by `rem_euclid`.
- The tile tools after the ground changes (issue 31): `ground_changed`, `Workspace::rewrites` and `tiles_moved` let go of spots, drags, selected tiles and Tile Properties; lattice and grid lookups go through `get_*`, `place_tiles` refuses what is off the grid, `spot` what is off the area.
- Q and E's turning: one step, committed before any other command (`Moonglow::apply`), a click or another key, dropped by Undo and Escape; spawn-point drags are dropped when the area is read again; placed sounds' slots notice a sound moved into another's place.
- Paint's strokes: one command a stroke, Escape, a right click or Paint going off drops what is pending; `row` and `fill` are bounded (`ROW_MAX`, `FILL_MAX`, `ROW_STEP`); a group's size is bounded in mg-set, so the tileset editor's Groups page can't be made to loop.
