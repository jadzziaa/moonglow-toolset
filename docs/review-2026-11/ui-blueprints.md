---
type: Report
title: 'Code review of November 2026: the blueprint editors, palettes and pictures'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the blueprint editors, palettes and pictures (mg-ui): 30 in all (6 high, 19 medium, 5 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-ui, blueprints, palette]
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

# Code review of November 2026: the blueprint editors, palettes and pictures

One part of [the review of 9 November 2026](overview.md): the blueprint editors, palettes and pictures (mg-ui). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it. Paths are under `crates/mg-ui/src/` unless they name a crate.
## High

### B1. The Faction Editor indexes `repute.fac` unchecked, and replaces an unreadable one with the standard five

- **Kind:** bug
- **Severity:** high (crash; data loss)
- **Where:** faction_view.rs:119-125 (`Err(_) => Factions::of_module(&ws.module)`), 204-205, 260, 326-327 (`f.factions[sel]`), 348-349 (`for p in 1..STANDARD.len() as u32 { … &f.factions[p as usize].name`), 142
- **Since:** before 2253fa78
- **Scenario:** a `repute.fac` with an empty `FactionList` panics as the tab opens (`selected.min(n.saturating_sub(1))` is 0, then `f.factions[0]`); one with one to four factions panics at Add Faction…; one that fails to parse is shown as the standard five with no message, and the first slider drag writes them over the file. The journal editor shows the error instead (journal_view.rs:90-93).
- **Fix:** in `factions()` tell "no file" from "unreadable" from "fewer than `STANDARD.len()`", and show a message in place of the editor for the last two.
- **Test:** tests/app/modules.rs beside `faction_editor_adds_and_removes_factions`: a repute.fac of three factions, of none, of garbage; open the tab, click Add Faction…; no panic, file unchanged.
- **Confidence:** confirmed by reading (sub-reader; I re-read `faction_view.rs:116-125, 198-206, 255-262, 315-350` and `mg-module/src/factions.rs:59-96`)
- **Read again by the reviewer:** holds: `Err(_) => Factions::of_module(..)` and `f.factions[..]` unguarded (faction_view.rs:119-125, :322, :326).

### B2. Change Faction Name keeps a faction's index behind a window that doesn't block: OK panics or renames another faction

- **Kind:** bug
- **Severity:** high (crash)
- **Where:** faction_view.rs:321-323 (`view.renaming = Some((view.selected, …))`), 368-377 (`f.factions[id as usize].name = …`)
- **Since:** before 2253fa78
- **Scenario:** select the last custom faction, Change Name…; with the window up click Remove Faction (or Edit › Undo of its Add); OK indexes past the end. With faction 6 in the window, removing faction 5 makes OK rename the one that slid into 6.
- **Fix:** clear `renaming` and `adding` whenever the factions change, or keep the name as opened and close the window when `factions.get(id)` no longer has it.
- **Test:** app test in modules.rs: two custom factions, rename window on the last, apply a removal, click OK.
- **Confidence:** confirmed by reading (sub-reader; I re-read faction_view.rs:315-380 and `widgets::modal`, widgets.rs:2992-2997, a plain `egui::Window`)
- **Read again by the reviewer:** holds for the kept index and the unguarded `f.factions[id as usize]` (faction_view.rs:322, :377).

### B3. The Levelup Wizard gives the level field a range that runs backwards for a class above level 60, and an arrow click panics

- **Kind:** bug
- **Severity:** high (crash, on a rare file)
- **Where:** levelup_view.rs:55 (`ClassLevel` taken as it is, `max(1)` only), 136 (`spin_draft(ui, level, (*had).max(1)..=60, …)`); widgets.rs:2226-2229 (`let to = (v + by).clamp(lo, hi);`: `f64::clamp` panics when `lo > hi`)
- **Since:** after (a9ebcfde put `spin_draft` here; `spin_with` is before)
- **Scenario:** a creature whose file has `ClassLevel` 61 or more (another tool's, a hand-edited one). Classes › Levelup Wizard, then an arrow beside that class (or Up/Down in the field): range 61..=60, panic.
- **Fix:** `had.clamp(1, 60)` for the range's start (and the level shown), and make `spin_with` order its bounds before clamping.
- **Test:** unit test in widgets.rs: `spin_with` with `5..=3` and a step doesn't panic; app test: Levelup on a creature of ClassLevel 70, click the arrow.
- **Confidence:** confirmed by reading (levelup_view.rs:48-58, 134-137; `spin_draft` and `spin_with`, widgets.rs:2183-2229)
- **Read again by the reviewer:** holds: `spin_draft(ui, level, (*had).max(1)..=60, ..)` and `(v + by).clamp(lo, hi)`, which panics when `lo > hi` (levelup_view.rs:55, :136; widgets.rs:2227).

### B4. Edit Copy slices a ResRef's text at byte 13, which panics on a non-ASCII name

- **Kind:** bug (the same line three times)
- **Severity:** high (crash, on a rare name)
- **Where:** palette_view.rs:515 (`naming.numbered(&stem[..stem.len().min(13)], 1, taken)`); mg-module/src/palette_add.rs:64 and 136 (`let stem = &stem[..stem.len().min(13)];`)
- **Since:** before 2253fa78 (palette_view; palette_add.rs:136 after, 357e4c34)
- **Scenario:** a hak's resource named with twelve ASCII bytes and then 0xE9: its text has `é` across bytes 12-13. Edit Copy… on it slices inside the character.
- **Fix:** cut by characters in one helper (or drop the cut: `Naming::fit` cuts by characters already).
- **Test:** extend `copies_take_the_next_free_number` (palette_view.rs:2701) and palette_add's with such a name.
- **Confidence:** confirmed by reading (sub-reader; I re-read palette_view.rs:502-516, `mg-module/src/palette_add.rs:55-66, 128-139`, `mg-core/src/resref.rs:76-80` and copy_as.rs:76-88)
- **Read again by the reviewer:** holds: `&stem[..stem.len().min(13)]` on a ResRef's text, where a byte above 0x7F is two bytes (palette_add.rs:64, :136; palette_view.rs:515). Found by the clippy pass too.

### B5. A hak's loading-screen mesh whose U coordinates are all NaN panics the picture's stitching

- **Kind:** bug
- **Severity:** high (crash, from a malformed model only)
- **Where:** images.rs:144-150 (`lo`/`hi` left at `f32::MAX`/`f32::MIN`), 167-171 (the rect `[w, y0, 0, y1]` passes the `saturating_sub` checks), 179 (`&whole.data[(row + r[0]) * 4..(row + r[2]) * 4]`, start past end)
- **Since:** after (ef32ee7b)
- **Scenario:** a hak's `pnl_loadscreen` with a `gui_replace01` mesh whose tverts' U are `nan`; Area Properties shows a loading screen.
- **Fix:** skip a mesh whose bounds aren't finite and ordered; `stitched` returns `None` when `r[2] <= r[0]`.
- **Test:** extend `a_loading_screen_follows_a_hak_s_own_model` with such a quad.
- **Confidence:** confirmed by reading (sub-reader; not re-read by me)
- **Read again by the reviewer:** holds for the arithmetic: NaN leaves `lo` at `f32::MAX` and `hi` at `f32::MIN`, and `stitched` slices `[(row + r[0]) * 4..(row + r[2]) * 4]` (images.rs:144-150, :179).

### B6. Windows that hold a placed object by its list index are not followed, and their OK edits another object

- **Kind:** bug
- **Severity:** high (another object's data replaced; one Undo away, but nothing says so)
- **Where:** blueprint/mod.rs:1665 (Variables…: `VarTableEdit::new(self.target("VarTable"), &list)`), 1121 and 1133 (String Edit), 780-781 (Gallery…); widgets.rs:1237-1243 (OK sets the whole list at the stored path); levelup_view.rs:36-44, 167-170; store_wizard.rs:37-41, 372-381, 437 (the shopkeeper by index) and 487-490, 547 (Add Popup Text); blueprint/follow.rs:394-466 (tabs, `window_ids`, `blueprint_pages`, `properties_changed` are moved, these are not)
- **Since:** before 2253fa78
- **Scenario:** Properties of placed placeable #5 › Advanced › Variables…. With the window up, delete placeable #2 in the area view. The Properties tab follows to #4; the window still names `Placeable List[5]`. Add a variable, OK: the former #6's variables are replaced by #5's. The same with String Edit (a name), the Appearance Gallery (a click gives the former #6 the appearance), the Levelup Wizard (levels another creature) and Setup Store (another creature gets the conversation, and the store is placed where it stands).
- **Fix:** have `follow::retarget` move (or, where the object is gone, close) `var_edit`, `loc_edit`, `placeable_gallery`, `levelup`, `store_wizard` and `popup_text`; or keep with each the value it opened on and refuse with a message when the field at its path differs.
- **Test:** app test: Variables… on a placed object, a `RemoveItem` before it in the same list, OK; assert the neighbor's `VarTable` unchanged and the object's own set.
- **Confidence:** confirmed by reading (blueprint/mod.rs:1108-1144, 1660-1667, 772-784; widgets.rs:28-46, 1204-1254; blueprint/follow.rs:394-466; `rg 'loc_edit|var_edit'` over the crate: nothing follows or closes them; the gallery's by a sub-reader, appearance_gallery.rs:147, 366-371, 459)
- **Read again by the reviewer:** holds (the same lines as S3): nothing follows or closes these windows when their object's place moves.

## Medium

### B7. Windows that hold a target survive closing the module, and their OK is applied to the next one

- **Kind:** bug
- **Severity:** medium
- **Where:** lib.rs:3444-3512 resets none of `palette_categories` (731), `loc_edit` (633), `var_edit` (892), `levelup` (763), `store_wizard` (759), `popup_text` (761), `creature_wizard` (765), `blueprint_wizard` (622), `placeable_gallery` (827); palette_categories.rs:486-503 (OK: `SetResource` of the skeleton plus `SetField` moves); palette_categories.rs:130 (`remove()` reads `ws.module` without the flush `open()` does at 60)
- **Since:** before 2253fa78 (the categories' `moves` after, 96607eb0)
- **Scenario:** More › Categories… in module A, File › Open module B with the window still up, OK: B gets A's `creaturepal.itp`, and B's blueprints that share ResRefs with A's are moved. Variables… on `area001`'s object, open another module that has an `area001`, OK: its object's variables are replaced.
- **Fix:** reset these in `close()` beside `tile_props` and `resize_area`; flush in `palette_categories::remove`.
- **Test:** tests/app/palettes.rs: open the window, `close()`, open another module, assert no window.
- **Confidence:** confirmed by reading (`Moonglow::close`, lib.rs:3444-3512: I listed every field it resets; the Palette Categories part by a sub-reader, palette_categories.rs:251, 486-503)

### B8. `retarget` moves a window's noted changes only when the window's first object moved

- **Kind:** bug
- **Severity:** medium
- **Where:** blueprint/follow.rs:449 (`let Some(to) = now(&key, &path) else { continue };`): a `WindowChanges` whose key didn't move is skipped whole, so its other `Was::Field` paths (453-462) and its `tab` copy (452) stay as they were
- **Since:** before 2253fa78
- **Scenario:** Properties of placeables #1 and #7 together, tick Plot. Delete placeable #3 in the area view: the tab's paths become [1, 6], the notes still say `Placeable List[7]`. Cancel: #6 keeps Plot, and the former #8 (now at 7) has its Plot cleared if it had one. Closing by × no longer cancels either: `closing` compares `c.tab == *tab` (blueprint/mod.rs:425).
- **Fix:** walk every entry: retarget its `tab` and each `Was` path; re-key it only when its first object moved; drop a `Was` whose object is gone.
- **Test:** unit test beside follow's: two noted objects, a removal between them, the second's path one less.
- **Confidence:** confirmed by reading (blueprint/follow.rs:394-466, `cancel_command` and `closing` in blueprint/mod.rs:279-309, 424-434; follow.rs's unit tests cover `follow`, not this loop)

### B9. Setup Store writes a creature's `FactionID` as a DWORD

- **Kind:** bug
- **Severity:** medium
- **Where:** store_wizard.rs:461 (`edits.push(set(faction_field(w.list), Value::Dword(w.faction)))`; `faction_field` is `FactionID` for a creature)
- **Since:** before 2253fa78
- **Scenario:** Setup Store on a hostile creature, "Use selected faction instead" left on: its `FactionID` WORD becomes a DWORD. If the game reads the field by type, the shopkeeper falls back to faction 0, which the game treats as Hostile: the opposite of what was asked. The app test only reads `integer("FactionID")` (tests/app/blueprints.rs:3264).
- **Fix:** keep the stored type, else the schema's (`blueprint::integer(creature.get(field), faction, FieldType::Word)`, as `Form::set_int` does; it is `pub(super)` today).
- **Test:** in the Setup Store app test assert `get("FactionID") == Some(&Value::Word(3))`.
- **Confidence:** confirmed by reading for the write (store_wizard.rs:78-81, 437-462; `mg-schema/src/generated/utc.rs:778` and `git.rs:1188`: `Field<u16>`; `mg-module/src/instances.rs:116` writes `D::Word`); what the game does with a DWORD there is suspected (an engine test beside `engine_factions.rs` would settle it: the form's own faction field keeps the stored type, blueprint/situated.rs:134)
- **Read again by the reviewer:** holds for the write (drawn in the sample): `set(faction_field(w.list), Value::Dword(w.faction))` (store_wizard.rs:461).

### B10. Setup Store replaces an existing conversation or script of the name typed without a word; a wizard's typed ResRef is checked on its Name page only

- **Kind:** bug (drift between four windows)
- **Severity:** medium (a resource replaced; undoable, unannounced)
- **Where:** store_wizard.rs:279-280 (`names_ok`: a valid name is enough), 410-435 (`SetResource` of the DLG, NSS and NCS); against 510-526 (Add Popup Text: "A conversation of that name is replaced."); blueprint_wizard.rs:232-233 (`own.unwrap_or_else(…)`: the typed ResRef as it is) against creature_wizard.rs:693-697 (the next free one when taken)
- **Since:** before 2253fa78
- **Scenario:** in Setup Store a builder types the name of the shopkeeper's existing conversation, `merchant`, meaning to keep it: Finish replaces it with the three-line store conversation (and an existing `openstore…` script likewise). In the Item Wizard (Name is not its last page) a ResRef free on the Name page and taken by the time Finish is clicked on the next page replaces that blueprint.
- **Fix:** say "replaced" on the Conversation page as Add Popup Text does (or refuse); check `taken` again in `BlueprintWizard::build`, as the Creature Wizard's `make` does.
- **Test:** app test: Setup Store with the dialog named after an existing DLG; expect the note, or the DLG unchanged.
- **Confidence:** confirmed by reading (store_wizard.rs:193-209, 278-280, 359-436, 506-526; blueprint_wizard.rs:225-233, 582-591, 606-620; creature_wizard.rs:688-697)

### B11. Questions and pickers that stopped blocking keep a list index, and answer for another entry

- **Kind:** bug
- **Severity:** medium
- **Where:** blueprint/creature_lists.rs:2189 (`insert_temp(ask, (bit, r, missing, moving))`: `moving.taken` holds `RemoveItem` indices of that frame, from 2090, 2115 or `taking`), 2247 (Yes runs them as they were); blueprint/replacements.rs:371 (`TexturePick.entry`), 76-81 (`items[i].set(self.new, …)` with no check that entry `i` is still `old`'s); journal_view.rs:289, 336-337 (String Edit on `Categories[c]`/`EntryList[e]`)
- **Since:** after (576682f4, 2026-10-29) for the first two; the journal before
- **Scenario:** backpack [dagger, longsword, potion]; drag the longsword to a slot: Missing Feats asks. Behind it remove the dagger, then Yes: index 1 is now the potion, which is taken out, and the stored copy of the longsword is equipped: one item twice, one gone. Texture replacements A, B, C: Select Texture… on B, Reset A behind it, OK: C is drawn with the pick. Journal: String Edit on entry 2, delete entry 1, OK: the text goes to the former third. The question also stays in temp data when the page is left unanswered, and comes back at whatever creature has that path later.
- **Fix:** Missing Feats: keep the lists' lengths with the question (as `Selected.lens` does at 1837-1840) and drop it when they differ; Select Texture: find the entry by `OldTexture` at OK; journal: close `loc_edit` when the list changes.
- **Test:** extend `equipping_without_the_feat_asks_to_add_it`: remove a backpack row while it asks, Yes, assert the third item is still held.
- **Confidence:** confirmed by reading (blueprint/creature_lists.rs:2068-2253; blueprint/replacements.rs:55-87, 366-385, 512-544, 673-687; docs/log.md:391 says both were an `egui::Modal` until 576682f4; the journal's by a sub-reader, journal_view.rs:163-177, 289, 336-337)

### B12. Remove Faction looks for the faction's users before the flush that the removal itself does

- **Kind:** bug
- **Severity:** medium
- **Where:** faction_view.rs:390 (`faction_users(&w.module, id)`) against 137 (`ws.flush()` in `store_factions`); mg-module/src/factions.rs:247 (`map(old).unwrap_or(old)`)
- **Since:** before 2253fa78
- **Scenario:** a creature just given the faction in its Properties isn't in the stored bytes yet: the removal goes ahead, and the creature keeps the number, which now names the next faction (or none). The reverse too: a refusal that still names a blueprint changed a moment ago.
- **Fix:** `ws.flush()` before `faction_users`, refusing on its error.
- **Test:** app test: `SetField FactionID = 5` without a frame between, Remove Faction 5: refused, naming the blueprint.
- **Confidence:** confirmed by reading (sub-reader; I re-read faction_view.rs:128-147, 388-395)
- **Read again by the reviewer:** holds (drawn in the sample): `faction_users(&w.module, id)` (faction_view.rs:390) before the `ws.flush()` in `store_factions` (:137).

### B13. An item's part and icon numbers, and a body's part numbers, are listed once a session

- **Kind:** bug
- **Severity:** medium
- **Where:** blueprint/item.rs:274-290 (`model_numbers`), 311-327 (`icon_numbers`): kept under `("model-numbers", prefix)` and never removed; used at item.rs:792, 843, 891 and blueprint/creature.rs:527
- **Since:** before 2253fa78 (0700a743)
- **Scenario:** a builder adds `iit_midmisc_250` or `pmh0_chest180` to a hak (or `override`) and reloads resources, or opens another module with other haks: the item's Appearance lists and the creature's Body Parts still offer the first scan's numbers until Moonglow is restarted.
- **Fix:** keep the lists on `Moonglow`, cleared in `load_order_changed`.
- **Test:** app test: open an item's Appearance, add an icon to the module layer, reload, expect its number in the grid.
- **Confidence:** confirmed by reading (blueprint/item.rs:260-327; `rg '"model-numbers"|"icon-numbers"'`: only there; `game_data_changed`, lib.rs:5104-5124, clears pictures, thumbnails and palettes, not egui's temp data)

### B14. Listings stamped with the game data's address stay when Reload Resources changes the game data in place

- **Kind:** bug
- **Severity:** medium
- **Where:** blueprint/replacements.rs:140 (`offered`'s stamp), 482 (`texture_names`); blueprint/tint.rs:134 (`("tintable", Arc::as_ptr(game) as usize, appearance)`); model_textures.rs:76; palette_view.rs:1348-1351 (`grouped` keyed by `Arc::as_ptr(&palette) as usize` with the `Arc` not held, so a later palette at the same address shows the old grouping)
- **Since:** after (51573b07, 1c2e081f, 84d05106, d8ce060a)
- **Scenario:** Tile Properties says "2 missing"; the builder puts the two textures in `override`, Reload Resources: still "2 missing" until some edit moves the revision. Select Texture doesn't list a texture added the same way; a model whose MTR gained `tdodTint` shows no Tint section.
- **Fix:** a counter on `Moonglow` moved in `load_order_changed`, used in the stamps; hold the `Arc<Palette>` and compare with `Arc::ptr_eq` (as blueprint/picker.rs:55-65 does).
- **Test:** app test: a tile naming a missing texture; write it to the scratch `override`, `reload_resources(true)`; the heading has no "missing".
- **Confidence:** confirmed by reading (blueprint/replacements.rs:137-176, 479-495; blueprint/tint.rs:132-141; sub-reader for model_textures.rs:66, 76-80 (I re-read 74-81), `exclusive` = `Arc::get_mut` at lib.rs:5912-5914, and palette_view.rs:1348-1351)

### B15. The renderer's textures, kept by name, are never forgotten when resources change

- **Kind:** bug
- **Severity:** medium
- **Where:** model_view.rs:123-132 (`release_game_data`: "those read are kept"); lib.rs:5104-5124, 3207, 3545: none calls `clear_textures` (its only caller is lib.rs:1504); mg-render/src/renderer.rs:1405-1406, 1979-1986 (a missing texture kept as `None`)
- **Since:** before 2253fa78
- **Scenario:** a builder repaints `plc_x.dds` in `override`, Reload Resources: views are drawn again from the cached texture, and one that was missing stays missing; another module's hak texture of the same name likewise. The manual (09-build-and-test.md:480-494) says textures are picked up.
- **Fix:** `clear_textures()` in `game_data_changed` and `use_module`, and in `update_module_layer` when a key is a texture, MTR or TXI.
- **Test:** tests/app/drawing.rs beside `reload_resources_reads_an_area_again_on_a_thread`.
- **Confidence:** confirmed by reading (sub-reader; not re-read by me)

### B16. Model views are never let go of, and a model's view never looks at its bytes again

- **Kind:** bug (and memory)
- **Severity:** medium
- **Where:** model_view.rs:752-758 (`if model || (revision == self.revision && female == self.female) { return; }`), 981-998, 1013-1027; lib.rs:2292-2306 (`closed` drops nothing), 3472-3478 (`close` clears area, dialog and NUI views, not these)
- **Since:** before 2253fa78
- **Scenario:** a module folder's `plc_mine.mdl` open in a Model tab is exported again from Blender and read again from the files: the tab draws the old model, still after the tab or the module is closed and opened again. With the Preview window, every blueprint chosen in the palette adds a view with its GPU models and parsed supermodels, for the session.
- **Fix:** drop a view when its tab closes or it goes undrawn for some seconds; clear in `close()`; compare a model source's bytes (`Module::shared`).
- **Test:** app test (`gpu_memory.rs`): open and close a Model tab, `model_views` empty.
- **Confidence:** confirmed by reading (sub-reader; I saw `model_views.clear()` only in `game_data_changed`, lib.rs:5118)

### B17. Decoded pictures and PLT palettes outlive the module they were read from

- **Kind:** bug
- **Severity:** medium
- **Where:** images.rs:285 (misses kept as `None` too), 294-301, 308-310; lib.rs:3444-3512, 3207-3257
- **Since:** before 2253fa78
- **Scenario:** module A's hak reskins an item icon or `pal_cloth01`; open module B: B shows A's picture, or "no picture" for a name B's hak has, until a hak change or Reload Resources. A module's own TGA replaced by Import keeps its old picture too.
- **Fix:** clear both in `close()`/`use_module`; drop a name's entries when the module's resource of that name is set.
- **Test:** unit test in images.rs: one `Pictures` map, the workspace swapped, the picture expected to change.
- **Confidence:** confirmed by reading (sub-reader; I confirmed `close()` resets neither `pictures` nor `palettes`, and lib.rs:5112-5114 is their only reset)

### B18. A gallery click does nothing for the whole selection when the first selected object already has that appearance

- **Kind:** bug
- **Severity:** medium
- **Where:** appearance_gallery.rs:343-347 (`current`: the first selected object's field), 442-446 (`.filter(|row| Some(*row) != current)`)
- **Since:** after (2c333e73)
- **Scenario:** three creatures of appearances A, B, C selected; click A in the gallery: nothing changes, though "a click gives them the appearance".
- **Fix:** skip only when every selected object has the row.
- **Test:** app test: two placed placeables of different appearances, click the first's picture, the second changes.
- **Confidence:** confirmed by reading (sub-reader; I re-read appearance_gallery.rs:340-348, 440-447)

### B19. One source shown in two places in a frame shares one view and one set of targets

- **Kind:** bug
- **Severity:** medium
- **Where:** model_view.rs:1027, 1233 (`size_targets`), 148-169; no guard at tile_view.rs:39, hak_view.rs:773-779, area_tools.rs:720 (only blueprint/situated.rs:885 checks `open_models`)
- **Since:** before 2253fa78
- **Scenario:** the Preview window shows the palette's blueprint while its Properties' Appearance page shows the same one (or Pop Out from a tile's view): each frame the targets are made twice at two sizes and both images sample the last.
- **Fix:** key the targets by place, or say "The preview is in a window of its own" there too.
- **Test:** app test with a GPU: both places for ten frames, the view's targets not made anew.
- **Confidence:** the double allocation confirmed by reading (sub-reader); the wrong picture suspected (a `screens` picture of both places would settle it)

### B20. Lists edited together clone every object, and its inventory once per row, each frame

- **Kind:** performance
- **Severity:** medium
- **Where:** blueprint/mod.rs:944-954 (`list_targets`: `self.root.clone()` and each other object's struct, every frame, no cap: `MIXED_LIMIT` bounds `differs` only); blueprint/inventory_together.rs:168 (`held(t, holder, r)` → `bags()` at 32-41 copies the target's whole `ItemList` once per merged row per target), 180-184 (every item of every target walked per row per flag)
- **Since:** after (9297e773, 826abcf9)
- **Scenario:** 100 placed creatures of an area selected (its guards), Properties › Inventory: placed creatures hold whole items, so with 20 kinds of item a frame copies 2,000 item lists (some 20,000 item structs with their properties) besides the 100 creatures; the Feats, Skills and Classes pages copy the 100 creatures each frame.
- **Fix:** `bags()` borrows; count each target's items once into a map; keep `list_targets` per `ws.revision()`.
- **Test:** frame_perf.rs: an `Instances` window of 100 creatures on the Inventory page.
- **Confidence:** confirmed by reading (blueprint/mod.rs:944-954; blueprint/inventory_together.rs:32-49, 103-198; the callers at blueprint/creature_lists.rs:191, 830, 1284, blueprint/creature.rs:717, 884, blueprint/item.rs:926, blueprint/encounter.rs:237, blueprint/store.rs:447); not measured

### B21. Setup Area Transition… parses every area's GIT on the window's thread and leaves them all in the workspace

- **Kind:** performance
- **Severity:** medium
- **Where:** blueprint/situated.rs:410-413 (`for key in gits { let Ok(g) = ws.doc(&key) …`)
- **Since:** before 2253fa78
- **Scenario:** a persistent world's module of 1,000 areas: one click on the button of a door's Area Transition page parses 1,000 GITs before the next frame, and their trees stay in memory for the session.
- **Fix:** read `ws.loaded(key)` where there is one, else parse the module's bytes without keeping them (on a worker, the window showing as they come), or keep an index of tagged doors and waypoints by revision.
- **Test:** editor_perf.rs: 300 areas, the click's frame time and the number of documents the workspace holds after.
- **Confidence:** confirmed by reading (blueprint/situated.rs:407-430, 665-683; `Workspace::doc`/`doc_mut`, mg-edit/src/lib.rs:298-328: parsed on first use and kept, dropped only by `derive` and reloads); not measured

### B22. Whole 2DAs are copied and sorted by name every frame

- **Kind:** performance
- **Severity:** medium
- **Where:** appearance_gallery.rs:235-239, 252; creature_wizard.rs:425-433 (appearance.2da) and 499-500 (`portraits()`: every row, `has_portrait` for each, images.rs:528-540); blueprint/creature_lists.rs:190 (feats), 316-319 with 515-518 and 960-963 (spells, twice), 1282; blueprint/door.rs:66-68. On the Feats page also 196 (`merged.iter().find` per feat), 233 (`offered`: a lowercase copy per row), 230-250 (every row laid out, none skipped out of sight) and 264
- **Since:** before 2253fa78 (the gallery's d83666b0); the wizard's page after
- **Scenario:** a server's appearance.2da of 15,000 rows: the gallery and the Creature Wizard's appearance page copy 15,000 names, build 30,000 sort keys and sort, each frame. A feat.2da of some 20,000 rows (a PRC-style hak) does the same on the Feats page, with a checkbox laid out for each.
- **Fix:** `choice_list` (the `Arc`), with the by-name order kept per `Arc`; `is_rect_visible` or `show_rows` on the Feats list.
- **Test:** frame_perf.rs with a 15,000-row appearance.2da on the gallery and the wizard's page.
- **Confidence:** confirmed by reading (`GameData::choices` copies, `by_name` builds a key per row: mg-rules/src/lib.rs:61-64, 412-414; the gallery's by a sub-reader, re-read by me at appearance_gallery.rs:232-240); not measured

### B23. The palette pane does whole-palette work every frame

- **Kind:** performance
- **Severity:** medium
- **Where:** palette_view.rs:1395-1398 (`own`: a set of all the module's keys each frame); 1458-1459 and 2281-2286 (`palette.blueprints()` twice, then per Favorites/Recent entry `all.iter().find(|(_, b)| b.resref.to_string() == r)`); 2029-2037 (in the Gallery each row's name and label are built before `picture_tile` finds it out of sight; the List skips at 2025)
- **Since:** before 2253fa78
- **Scenario:** a Custom palette of 20,000 blueprints with 12 Recent entries placed from Standard: 240,000 `to_string` comparisons a frame.
- **Fix:** keep `own` and a ResRef map per revision; in the Gallery allocate the tile first.
- **Test:** frame_perf.rs: a 20,000-blueprint custom palette, Recent filled from Standard, List and Gallery.
- **Confidence:** confirmed by reading (sub-reader; not re-read by me)

### B24. A search in the Custom palette reads and parses every blueprint again at every edit; a module without `…palcus.itp` rebuilds its custom palette likewise

- **Kind:** performance
- **Severity:** medium
- **Where:** palette_view.rs:708-730 (`tags()`, keyed by revision, called at 1181), 657-658; 560-576 with mg-module/src/palette.rs:969-971, 567-577, 603-604
- **Since:** before 2253fa78
- **Scenario:** "sword" left in the search (it is restored at the next start, palette_view.rs:938-940) over 5,000 to 20,000 item blueprints: every command anywhere (a keystroke committed in a field, an object moved) copies and parses them all before the next frame.
- **Fix:** keep each tag with its blueprint's `Module::shared` `Arc` and read again only those that changed.
- **Test:** editor_perf.rs: a typed search, 5,000 custom blueprints, one `SetField` on an area.
- **Confidence:** confirmed by reading (sub-reader; not re-read by me)

### B25. Pictures read apart are served first in, first out, and nothing scrolled past is taken out of the queue

- **Kind:** performance
- **Severity:** medium
- **Where:** model_view.rs:1620-1621 (only `ready` is pruned), 1726-1728, 1811-1812, 1883, 1911
- **Since:** after (8bddd6f3)
- **Scenario:** the Appearance Gallery over thousands of rows with the scroll bar dragged to the end: every row seen for a frame is read (model and supermodels), uploaded and dropped as no longer asked for, and the rows in sight wait behind them.
- **Fix:** each frame take out of `loading` and the queue what the `asked` set no longer has, or serve the newest first.
- **Test:** unit test on `Thumbnails` with its readers held: 1,000 keys asked in one frame, 10 others in the next, the 10 served first.
- **Confidence:** confirmed by reading (sub-reader; not re-read by me)

## Low

### B26. Grids and pickers that make or test every row each frame, in sight or not

- **Kind:** performance
- **Severity:** low
- **Where:** blueprint/item.rs:596-627 (`part_grid`: for every number `with_model` clones the item and its icon layers are loaded, all at once on first show, kept in `app.pictures`); blueprint/situated.rs:179-197 (`portrait()`: a `Choice` with a new `String` for every portraits.2da row each frame on every page with a portrait), 239-257 (`has_portrait` for every row each frame while Select Portrait is open); blueprint/replacements.rs:531-534 (every texture name filtered each frame), 679-683 (its pictures let go only by OK or Cancel, not when the page is left)
- **Since:** before 2253fa78 (Select Texture's list after)
- **Scenario:** a base item with over 1,000 icons (CEP's miscellaneous items): opening Appearance decodes them all in one frame. A portraits.2da of 10,000 rows; 60,000 textures with a search typed in Select Texture.
- **Fix:** skip tiles out of sight in `part_grid`; keep the portrait rows per table and filters; keep `shown` per (filter, switch).
- **Test:** frame_perf.rs: an item's Appearance page with 1,500 icons in a hak layer.
- **Confidence:** confirmed by reading (blueprint/item.rs:569-637; blueprint/situated.rs:173-257; blueprint/replacements.rs:526-534, 679-687); not measured

### B27. Form numbers and checkboxes in a list's entry fall back to a BYTE where the entry lacks the field

- **Kind:** bug
- **Severity:** low
- **Where:** blueprint/mod.rs:1061 (`mg_schema::root_field_type(self.restype(), label).unwrap_or(default)`), 1187, 1282, 1339, 1354; journal_view.rs:323, 352, 354
- **Since:** the form's fallback before 2253fa78; the journal's fields came onto it after (3908a27d)
- **Scenario:** a `module.jrl` from a tool that leaves out zero fields: a category with no `XP` given 500 stores BYTE 255; an entry with no `End` ticked stores a BYTE where the game reads a WORD.
- **Fix:** give `Form` the field's schema type for the struct it edits (`jrl::categories::XP.field_type`), not the root's.
- **Test:** tests/app/fields.rs: a category without `XP`, an entry without `End`; expect `Dword(500)` and `Word(1)`.
- **Confidence:** confirmed by reading (blueprint/mod.rs:683-695, 1058-1064, 1277-1285, 1330-1356: every `number`, `spin`, `check` and `slider` passes `FieldType::Byte`, and `root_field_type` looks at root fields only; the journal's lines by a sub-reader); what the game makes of a BYTE there is suspected

### B28. Window state is kept by the document, not the object, so two Properties windows of one area share it; the several-objects note is never taken down

- **Kind:** bug
- **Severity:** low
- **Where:** blueprint/mod.rs:1964 (`("several-note", key)`), 1400 (`("palette-color", self.key, label)`); blueprint/situated.rs:194 and 261 (`("portrait-pick", f.key)` and its window), 865 (`("pick", key, name)`); blueprint/item.rs:939 (`("uti-props", key)`); blueprint/creature_lists.rs:197, 457; blueprint/sound.rs:75; blueprint/store.rs:181; blueprint/encounter.rs:160. `Form::id` (mod.rs:822-831) exists for this and is not used there
- **Since:** before 2253fa78
- **Scenario:** after a window of several placed objects drops a change or says "Added to 2", every Properties window of that area's objects, single ones too, shows the note for the rest of the session. Two placed objects of one area side by side: Select Portrait opened from one is drawn, and its OK taken, by whichever is drawn first.
- **Fix:** `Form::id` (or the tab) for all of them; remove the note when the window closes or its next change is made.
- **Test:** app test: an `Instances` window drops a change; a single `Instance` of the same area shows no note.
- **Confidence:** the note confirmed by reading (blueprint/mod.rs:1964-1967, 2025-2030: `rg 'several-note'` finds no removal, and 1965 is not under `several`); the shared dialogs suspected (two `Tab::Instance` of one area on screen at once, Portraits… in the second, would settle it)

### B29. Written more than once, and drifted

- **Kind:** duplication
- **Severity:** low
- **Since:** before 2253fa78, but `param_sets.rs` (ec2b2540), inventory_together.rs (826abcf9) and the node name's drift, after
- **Scenario:** as said with each.
- **Fix:** one function each, in the crate named first as the source of truth.
- **Test:** a unit test per pair that feeds both the same input (`place` with a negative position; a node with both NAME and STRREF; `from_text(&to_text(rows))`).
- **Confidence:** confirmed by reading (the places named; the palette's node name and the sets' by sub-readers)
- where and drift:
  - Where a new item goes in an inventory: blueprint/inventory.rs:198-211, 244-262, 314-323 (`place`); blueprint/inventory_together.rs:74-100; mg-edit/src/found.rs:389-455 with mg-edit/src/lists.rs:484. found.rs clamps `Repos_PosX`/`Posy` with `.max(0)`; inventory.rs:249-250 casts `as u32`, so an INT -1 in a file overflows `tx + tw` in `place` (a panic in a debug or test build). Source of truth: `mg_edit::lists::place` and one adding function in mg-edit.
  - A palette node's name: palette_categories.rs:95-101 reads NAME then STRREF; `mg_module::palette::node_name` (palette.rs:366-373) and Aurora read STRREF first (drifted in 944c0ed8).
  - A type's categories: blueprint_wizard.rs:278-301 (the game's resman, by name) against palette_view.rs:474-498 and mg-module/src/palette.rs:33 (the module's own skeleton first).
  - A class changed: blueprint/creature.rs:638-640 lets Class 2 be made Class 1's class; several together refuse it (mg-edit/src/lists.rs:406).
  - A tag's length: `Form::tag` cuts at `MAX_TAG` (blueprint/mod.rs:1092-1094); the wizards' Tag fields take any length (blueprint_wizard.rs:575, creature_wizard.rs:643).
  - A variable's value as text: var_sets.rs:39-45, 67 against param_sets.rs:51, 78-85 and `Value::from_text` (mg-gff/src/value.rs:217-241): an int typed `0x10` is left out of a saved set without a word, a float `nan` is written as `null` and the set then reads "(not a set)".

### B30. Files written in place, and 2DA rows cut to a field's width with `as`

- **Kind:** hygiene
- **Severity:** low
- **Where:** blueprint/mod.rs:1581 (Save Script Set: `std::fs::write`), blueprint/creature_lists.rs:726 (Save Class Spell List), saved.rs:48-55 and 79, var_sets.rs:178, param_sets.rs:182, model_view.rs:606 (Export Files); Save Set replaces a set of the same name without asking. Casts: blueprint/creature.rs:639 (`Value::Int(v as i32)`), 667, 679, 769 (`Value::Byte(v as u8)` for a domain or school: row 300 is written as 44), blueprint/creature_lists.rs:400, 1322 (`as u16`)
- **Since:** before 2253fa78 (param_sets.rs and model_view.rs:606 after)
- **Scenario:** a full disk or a crash mid-write leaves a set, a script set or an exported model cut short where a whole one stood; a domains.2da past 255 rows writes another domain than the one picked.
- **Fix:** make `mg_module::write_replacing` public and use it; `u8::try_from(v)` with a message where a row doesn't fit the field.
- **Test:** saved.rs unit test: writing over a name leaves the whole text and no temporary file.
- **Confidence:** confirmed by reading (the lines named; saved.rs by a sub-reader)

## Not read

- blueprint/door.rs:1-19, blueprint/trigger.rs:1-13, blueprint/waypoint.rs:1-9 (module comments, imports and the page lists).
- The `#[cfg(test)]` module of blueprint/follow.rs (468-694).
- The ten files named at the top were not read by me but by the two sub-readers, each in full by their account; of their findings I re-read the cited lines only where an entry says so.

## Looked at and found sound

- The Spells pages' indices: `casters[which]` and `casters[0]` can't be empty (`spell_slots` is `Some` by the class's row alone), `lists[cast]` is guarded by `cast <= 9`, `slots.max` is `[usize; 10]`, removals are built last-first from the same frame's lists, and `lists::set_count`, `add` and `remove` take their indices from the targets as they are that frame.
- `follow()` itself (inserts and removals before a window's object, a move found by `same_item`, a list set whole) and the picker (`Found` kept by `Arc::ptr_eq` and the search, rows skipped out of sight, a row's Equip taken by the same window in the same frame, `kept_blueprint` dropped at each revision).
- The form's integers: `integer()` clamps to the stored type's range, `set_int` takes the schema's type for a root field the blueprint lacks, and the item's cost and the creature's hit points and rating are amended into the same command (`ws.amend`).
- A store's pages (`PANEL_PAGE`, a page made once per step in `add_items`' `made`), `price()` in u64, an encounter's entry as Aurora writes it, `MiscVisuals` set per object with its other fields kept, the creature's `equip_edits` (slot order, the old occupant to the backpack, `Selected` dropped when a list's length changes).
- By the sub-readers: the palette's actions carry ResKeys and category ids, never row indices; `Factions::remove` renumbers parents and reputations; thumbnails are dropped at module close, game data change and the Shaders switch; sets live in Moonglow's own data folder under checked names; a hostile model in the viewer (a looping supermodel chain, an animation of length 0) is handled.
