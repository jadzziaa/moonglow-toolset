---
type: Report
title: 'Code review of November 2026: models, shaders and previews'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in models, shaders and previews (mg-mdl, mg-shader, mg-preview): 17 in all (2 high, 6 medium, 9 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, models, shaders, previews, mg-mdl, mg-shader, mg-preview]
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

# Code review of November 2026: models, shaders and previews

One part of [the review of 9 November 2026](overview.md): models, shaders and previews (mg-mdl, mg-shader, mg-preview). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## High

### M1. A compiled model's controller keys are copied outside the copy budget: a 0.5 MB model allocates gigabytes

- **Kind:** bug
- **Severity:** high (the toolset or `mg` is killed for memory; unsaved work lost)
- **Where:** crates/mg-mdl/src/binary.rs:147-194 (the copies at 165 and 173-186, `out.retain` at 190)
- **Since:** before 2253fa78
- **Scenario:** a hak's `.mdl` with one node, a controller data array of 65,535 floats (262 KB) and 20,000 key records (240 KB) of distinct ids, each saying rows 65,535, times at 0, values at 0, one column. The two arrays are charged once (`array`), but each record's `times` (`to_vec`) and values (`v.to_vec()`) are copied uncharged: 524 KB a record, 10 GB in all. (With Bézier keys of 15 columns one 12-byte record copies 12 MB.) Any read of the model does it: an area with the placeable, a preview, Verify, the usage walk. `out.retain(..)` per record is also quadratic in the record count (a 12 MB key array, a million records, takes hours).
- **Fix:** charge each record's `rows * 4 + times.len() * stride * 4` with `self.charge` before copying; replace the `retain` with a map from name to place in `out`.
- **Test:** a unit test in `binary.rs`: such a file built by hand reads as `MdlError::Binary` (the copy limit), as the shared-vertex-stream test does.
- **Confidence:** confirmed by reading (`binary.rs` `controllers`, `array`, `f32s`, `charge`; `mg_core::bin::CopyBudget`; `ctrl::name`, which gives every unknown id its own name `ctrl<id>`, so the "repeated controller replaces the earlier" line does not merge them)
- **Read again by the reviewer:** holds: `times` and the values are copied with `to_vec` for every key record, uncharged, and `out.retain` runs for each (binary.rs:165-190).

### M2. An ASCII key list pads every row to its widest row: a 400 KB text asks for 40 GB

- **Kind:** bug
- **Severity:** high (the process is killed for memory, from `Model::read` of a text model)
- **Where:** crates/mg-mdl/src/ascii.rs:389-416 (`width` is the longest row, each row then pushes `columns` values), called at 801, 882, 893; rows read at 326-347
- **Since:** before 2253fa78
- **Scenario:** an animation node with `xkey`, one row of 100,000 numbers and 100,000 rows of `0`: `values` grows to 100,001 x 100,000 floats. Any keyword ending in `key` will do in an animation node (no controller table is asked there); in the geometry a known one (`alphakey` on a trimesh).
- **Fix:** cap a key's width at what a compiled model can hold (15 values, 45 with handles: `binary_write` refuses more anyway) and note the rows cut; or keep rows ragged and pad only to the controller's own column count.
- **Test:** a unit test in `ascii.rs`: that text reads in bounded memory, the controller's `columns` at most 15, with a note.
- **Confidence:** confirmed by reading (`node_block`'s key rows, `key_controller`, its three callers)
- **Read again by the reviewer:** holds: `width` is the longest row and every row pushes `columns` values (ascii.rs:391-413).

## Medium

### M3. The binary writer walks node trees and walkmesh trees by recursion; the decompiler indents a walkmesh tree by its depth

- **Kind:** bug
- **Severity:** medium (stack overflow, which no `catch_unwind` holds, or gigabytes of spaces; only when a hostile model is compiled, baked or decompiled)
- **Where:** crates/mg-mdl/src/binary_write.rs:882-945 (`model_node`, recursing at 941), 949-1031 (`anim_node`, 1027), 765-806 (`aabb`'s `place`); crates/mg-mdl/src/ascii_write.rs:796-820 (`" ".repeat(d + 6)` a line)
- **Since:** before 2253fa78
- **Scenario:** (a) a text model of 20,000 dummies each the child of the one before (under 1 MB), compiled: `model_node` recurses 20,000 deep with three `Controller`s a frame (some 10 MB of stack, more than a thread has). (b) an `aabb` list of 100,000 rows whose face is -1: `place` recurses once a row. (c) a compiled model whose walkmesh tree is a chain (the reader allows 400,000 entries), decompiled (bake's copy, `mg model decompile`): line n is indented n spaces, 8 x 10^10 bytes for the full chain.
- **Fix:** walk with an explicit stack, or refuse a tree deeper than a few hundred with `CompileError::Invalid`; cap the decompiler's indentation.
- **Test:** unit tests in `binary_write.rs` run on a thread with a 256 KB stack: the chain of nodes and the chain of `aabb` rows each give `Ok` or a `CompileError`, not a crash.
- **Confidence:** confirmed by reading (the readers are iterative: `binary::walk`, `binary::aabb`, `ascii::preorder`; the writers are not. Callers: `mg_module::bake::copy_model` decompiles and recompiles a hak's model, `mg_module::models`, `mg model compile`)

### M4. What the Module Uses (and Export Model Files) leaves out two things the area view draws: the tile border's models and tiles' replacement textures

- **Kind:** bug (a drift between two copies: mg-area's scene and `Gatherer::area`)
- **Severity:** medium (a release cut by the report loses them)
- **Where:** crates/mg-preview/src/files.rs:90-144 (`Gatherer::area`), crates/mg-preview/src/usage.rs:585-602 (`Walk::area`)
- **Since:** after (390430a4 `Gatherer::area`, 7c56925d the walk)
- **Scenario:** a custom tileset hak with its edge tiles' models (named only in `xyz_edge.2da`) and a hak's replacement textures. `mg haks --usage` lists the edge models, their textures and the replacement textures as unused (an MDL and a TGA are judged; nothing reaches them); the builder cuts them and the game draws the areas without their border and the tiles with their own texture. Export Model Files of the area leaves them out too.
- **Fix:** in `Gatherer::area`, add the models of `mg_area::border::edge_rows` that the area's edge can take (or all rows' `Model`) and each tile's `replacetexture.2da` texture; best by moving "the files an area is drawn with" to where mg-area's scene and the gatherer both read it.
- **Test:** `files.rs`'s area test: a tileset with an edge table and a tile with `Tile_ReplaceTex` lists both; `usage.rs`'s `a_module_reaches_into_its_haks`: both come out used.
- **Confidence:** confirmed by reading (`mg_area` reads `<tileset>_edge.2da` `Model` for the border, `border.rs`, `lib.rs:695`, and `replacetexture.2da` `TEXTURENAME` by `Tile_ReplaceTex`, `lib.rs:630-665`; `rg -i edge` and `ReplaceTex` find nothing in mg-preview; the manual's list of what an area reaches has neither)

### M5. What the Module Uses says "unused" of files only a cell of a 2DA the game reads by itself names (feat, spell, skill and class icons, a spell's visuals)

- **Kind:** bug
- **Severity:** medium (the report's purpose is the release cut; the manual says it keeps too much rather than too little)
- **Where:** crates/mg-preview/src/usage.rs:371-375, 735-763 (`table_names`), 542-556 (`row_files`), 1120-1128 (`attribute`)
- **Since:** after (e03a5d8c)
- **Scenario:** a hak with `feat.2da` (a replacement of the game's: used, "reached from itself") and the new feats' icons `ife_x.tga`; or `spells.2da` with a class's new spells, cast from the spellbook, their `IconResRef`, cast and conjure models and sounds. No script names them, so every one is listed unused.
- **Fix:** for the tables whose rows the game shows or plays without a script (feat, spells, skills, classes, domains, racialtypes, iprp tables' icons), follow the icon and visual columns of every row when the table is reached; or count what a reached 2DA's cell names as not judged. (Not appearance.2da or placeables.2da: there the row is an object's.)
- **Test:** in `usage.rs`'s `sounds_icons_effects_tables_and_what_players_choose`: a hak's `feat.2da` over the game's with `ICON ife_mine` and `ife_mine.tga` in the hak is not in `unused`.
- **Confidence:** confirmed by reading (`follow` sends a 2DA to `table_names` alone, which reaches only cells that are names of 2DAs; `row_files`, which follows a row's every cell, is called from `row` only, for rows a script names by a constant; `attribute` then gives a TGA that nothing reached and the game lacks as `unused`; the test `a_compiled_script_s_numbers_and_strings` reaches `is_fire` through the script's call only)

### M6. The usage walk reaches a worn part's and a creation-screen head's model but not the texture the game draws on it

- **Kind:** bug (drift within mg-preview: `plt_fallbacks` knows the rule, the walk doesn't use it)
- **Severity:** medium
- **Where:** crates/mg-preview/src/usage.rs:1000-1017 (`worn`), 815-832 (`players_choose`), 943-952 (armor); crates/mg-preview/src/files.rs:240-246; the rule at crates/mg-preview/src/creature.rs:393-445
- **Since:** after (e03a5d8c)
- **Scenario:** a hak's player head `pmh0_head123.mdl` with `bitmap NULL` and `pmh0_head123.plt`, worn by no NPC: the model is used, the PLT unused. The same for an armor part held in a store whose other-race model names the human texture while the hak has the race's own PLT. Also: a cloak lying in an area (`placed`) never has its cloakmodel.2da `TEXTURE` PLT gathered (`item` skips `gather.preview` and `worn` adds models only).
- **Fix:** have `worn` and `players_choose` gather a `Part` made as `creature.rs`'s `with` makes it (model plus `plt_fallbacks`' textures), and gather a placed cloak's worn preview too.
- **Test:** `usage.rs`: a hak head with `bitmap NULL` and its PLT, and a part whose mesh names another's texture beside its own PLT: both PLTs used.
- **Confidence:** confirmed by reading (`creature.rs` `plt_fallbacks`: the part's own-named PLT is drawn whatever the mesh names, and a mesh with `bitmap NULL` takes it, "several of CEP's heads"; `usage.rs` `worn` and `players_choose` call `gather.add_model` only; `Gatherer::model` follows the textures the meshes name, nothing by the model's name; no test has a head or part whose mesh names another or no texture)

### M7. A creature blueprint's Body Parts lists go by the phenotype's own skeleton only, where the preview (and Verify's rule) fall back to `DefaultPhenoType`

- **Kind:** duplication (drifted: a wrong result in mg-ui)
- **Severity:** medium
- **Where:** crates/mg-ui/src/blueprint/creature.rs:429-444 and 527 (its own `p{gender}{race}{phenotype}`); crates/mg-rules/src/models.rs:100-128 (`skeletons`, the source of truth); crates/mg-preview/src/creature.rs:185-206 (a third copy of the gender letter, phenotype and default)
- **Since:** before 2253fa78
- **Scenario:** a creature of a phenotype whose phenotype.2da row has `DefaultPhenoType 0` and no parts of its own (a server's or ACP-style phenotype on the normal body): the preview draws `pmh0_chest005`, but each Body Parts list is built from `pmh7_chest*` and is empty (or holds only the few parts the phenotype has of its own).
- **Fix:** mg-ui and `dressed` both call `mg_rules::models::skeletons`; the lists take the numbers of both prefixes.
- **Test:** `crates/mg-ui/tests/app/blueprints.rs`: a phenotype row 7 with default 0, a creature of phenotype 7: the Torso list offers the `pmh0_chest` numbers.
- **Confidence:** confirmed by reading (`mg-ui` `body_prefix` and `appearance_fields`; `mg_rules::models::skeletons`; `mg_preview::creature::dressed`'s `part_model`, which tries `[skeleton, fallback]`)

### M8. `compile` reads the text a second time as strict UTF-8: one Latin-1 byte and every mesh's shininess is compiled as 1

- **Kind:** bug
- **Severity:** medium (a wrong compiled model, silently)
- **Where:** crates/mg-mdl/src/compile.rs:348-355 (`std::str::from_utf8(text).unwrap_or("")`), used at 383
- **Since:** before 2253fa78
- **Scenario:** a text model with `shininess 26` and a comment or `filedependancy` line holding `é` in Windows-1252 (an exporter's file name). `text_lines` is empty, `says(i, "shininess")` is false for every node, and each mesh gets `shininess = 1.0`.
- **Fix:** split `String::from_utf8_lossy(text)`; or give `ascii::geometry_node` the game's default of 1 where the keyword is absent and drop `says`.
- **Test:** `compile.rs`: the test model with a 0xE9 byte in a comment compiles with the shininess its text gives.
- **Confidence:** confirmed by reading (`compile`'s `text_lines` and `says`; the reader itself uses `from_utf8_lossy`, so the model reads fine and the line numbers agree)
- **Read again by the reviewer:** holds: `std::str::from_utf8(text).unwrap_or("")` (compile.rs:348).

## Low

### M9. The compiled-shader cache is never pruned: every release leaves its entries behind

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-shader/src/cache.rs:69-77 (the key), 123-131 (`write`); crates/mg-module/src/build.rs:374-383 ("two releases keep theirs apart")
- **Since:** before 2253fa78 (the release in the key: 70a92287)
- **Scenario:** each update names every program anew (Verify compiles each at six settings), and the previous release's files are never read again nor removed; the folder grows by a release's worth with every release.
- **Fix:** put the release in the folder's name or the entry's header and remove other releases' entries when the cache is opened (or entries not read for a month).
- **Test:** `cache.rs`: after a compile under release B, an entry written under release A is gone.
- **Confidence:** confirmed by reading (`ProgramCache` only reads and writes; the key holds `mg_core::release::version()`; no caller removes anything: `rg` for remove, prune in mg-shader, mg-render, `doctor.rs`, `build.rs`) and measured here: `~/.cache/moonglow/shaders` holds 7,195 files, 245 MB

### M10. Three parsers of a body part's model name disagree on the phenotype, and one panics on a name that isn't ASCII

- **Kind:** duplication (drifted)
- **Severity:** low (medium for a server whose own phenotypes are numbered 10 and up)
- **Where:** crates/mg-preview/src/usage.rs:1055-1064 (`armor_part`: any number of digits); crates/mg-preview/src/creature.rs:395-409 (`named`: exactly one character, of any kind); crates/mg-mdl/src/ascii_write.rs:99-106 (`skeleton_of`: exactly one digit). `mg_rules::models::skeleton` writes `p{gender}{race}{phenotype}` with as many digits as the number has.
- **Since:** `skeleton_of` after (518f93e2); the others before or with the walk
- **Scenario:** phenotype 15's own part `pmh15_chest001`: `named` returns nothing, so the part gets neither its own PLT nor the fallbacks, and a mesh with `bitmap NULL` no texture; `skeleton_of("pmh15_neck001")` is `None`, so the decompiler leaves a bound skin's bones unnamed. And `skeleton_of("pa\u{e9}_x")` (a compiled model's name with byte 0xE9: 4 bytes before the `_`) slices `skeleton[3..]` inside the `é` and panics.
- **Fix:** one function in `mg_rules::models` (beside `skeleton`) that splits `p`, gender, race, digits, `_`, rest by characters; the three call it.
- **Test:** unit tests of that function: `pmh15_neck001`, `PFE2_robe003`, `paé_x` (no panic), `plc_a01`.
- **Confidence:** confirmed by reading for the code; suspected for the game's behavior with a two-digit phenotype's own textures (a client picture of a `pmh15_chest001` whose mesh names another texture would settle it)

### M11. A shader's includes have no bound on what they add up to; and `Assembled::files` is not what its comment says

- **Kind:** bug (the bound), hygiene (the comment)
- **Severity:** low
- **Where:** crates/mg-shader/src/assemble.rs:306-313, 317-332, 340-383; the comment at 253
- **Since:** before 2253fa78 (the depth limit went in b9095940; there was never one on size)
- **Scenario:** a hak's `a.shd` of 60,000 lines `#include "big"` and a `big.shd` of 2 MB: Verify's `shader-compile` (or drawing a material that names `a`) assembles 120 GB of text. Separately: `file()` pushes an entry for every splice, so a file included twice is in `files` twice, where the comment says "every file in the order it was first spliced".
- **Fix:** an error once the assembled text passes a size no real program nears (a few MB); reuse a file's entry when it is spliced again (or correct the comment).
- **Test:** `assemble.rs`: 40 includes of a 1 MB file is `ShaderError::Include`; a file included twice is one entry of `files`.
- **Confidence:** confirmed by reading (`splice` refuses only a loop; `file` allows 65,535 splices; each `load` reads the file again and appends all of it)

### M12. A walkmesh node whose tree pointer is 0 has the model's header read as its tree

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-mdl/src/binary.rs:302, 386-392
- **Since:** before 2253fa78
- **Scenario:** a compiled tile whose walkmesh has no faces: read as corrupt (the tile missing), or with one entry of nonsense.
- **Fix:** `root == 0` is an empty tree.
- **Test:** `binary.rs`: a hand-built `aabb` node with pointer 0 reads with `MeshExtra::Aabb(vec![])`.
- **Confidence:** suspected (by reading: `aabb(0)` passes the `e == 0 && !out.is_empty()` test for the first entry, reads `min`, `max` and the children from model offset 0, where the name field's bytes 16-23 are the two child pointers: zeros give one bogus entry, garbage after the name's NUL, which EE's compiler leaves, gives offsets that fail and the whole model with them. Whether any compiled tile has an `aabb` node with no tree is what would settle it: a sweep over a builder's haks for flag 0x200 with 0 at +0x270)

### M13. A creature holding an item that names no model at all is not drawn

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-preview/src/creature.rs:377; crates/mg-preview/src/item.rs:187-189, 261-264
- **Since:** before 2253fa78
- **Scenario:** a creature whose right-hand item is of a hak's base item row not yet given an `ItemClass` (or the hak isn't installed): the area view and the blueprint show no creature, where the missing piece is its weapon.
- **Fix:** skip the item (`let Ok(parts) = .. else { continue }`), as the helmet is.
- **Test:** `crates/mg-preview/tests/previews.rs`: such a creature previews with its body and no hand part.
- **Confidence:** suspected (by reading: `item_parts(..)?` in the hands' loop returns `NoRow` for a base item row without `ItemClass` and `NoModel` where neither the part's model nor `DefaultModel` is in the load order, and the `?` fails the whole creature; the helmet and cloak beside it are left off instead. A client picture of a creature holding such an item would settle what the game draws)

### M14. `uniform_defaults` reads declarations the preprocessor leaves out

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-shader/src/defaults.rs:23-52
- **Since:** after (defaults.rs is new since the review)
- **Scenario:** a hak's shader with `#if SHADER_QUALITY_MODE > 1` / `uniform float fGlow = 1.0;` / `#else` / `uniform float fGlow = 0.25;` / `#endif`: the default is 1.0 at every quality setting.
- **Fix:** strip block comments, and follow `#if`/`#else` on the preamble's defines for the simple cases (or leave out a name declared more than once with different values).
- **Test:** `defaults.rs`: that text at quality 0 gives 0.25 (or no default), and a declaration in a block comment gives none.
- **Confidence:** confirmed by reading (it drops lines beginning `#` and `//` comments, then takes the first declaration of each name: both branches of an `#if` are seen, and a declaration inside `/* */` too)

### M15. `Composed::stance` and `Composed::posed` each work out the parts' poses, and differ

- **Kind:** duplication (drifted)
- **Severity:** low
- **Where:** crates/mg-preview/src/compose.rs:487-497 and 678-689
- **Since:** before 2253fa78
- **Scenario:** a base model without the animation asked whose wings or tail have it: drawn moving, boxed and picked (`bounds_in`, `ray_hit_in`) as at rest.
- **Fix:** one function gives a part's pose for (asked name, base animation, t); both call it.
- **Test:** `compose.rs`'s GPU tests: for such a pair, the stance's part nodes equal the instance's pose at the same t.
- **Confidence:** confirmed by reading (`posed`: an animated part plays its own animation of the asked name even where the base has none, `own.or(base_anim)`; `stance`: `playing` is `None` where the base has none, so the part stays at rest)

### M16. The reader and the linter keep their own tables of list keywords and their own idea of a number

- **Kind:** duplication
- **Severity:** low
- **Where:** crates/mg-mdl/src/ascii.rs:117-120 (`is_number`: must begin with a digit, sign or dot), 149-179 (`LISTS`, `NAMED_LISTS`); crates/mg-mdl/src/keywords.rs:50-81 (the same lists plus `vertexindices`); crates/mg-mdl/src/lint.rs:140-142 (`is_number`: any `f32`, so `inf` and `nan` rows count), 415 (`key != "aabb"` in a branch `aabb` never enters)
- **Since:** before 2253fa78
- **Scenario:** the linter says how many rows "follow" by a rule the reader doesn't use; a keyword added to one table is missed in the other.
- **Fix:** the reader uses `keywords::LISTS`, `NAMED_LISTS` and one `is_number`; drop the dead test.
- **Test:** a unit test that the linter's row count of a list equals the reader's for a list ended by `inf 0 0`.
- **Confidence:** confirmed by reading (no wrong result found: with `vertexindices` the reader files the rows as unknown properties, which nothing reads)

### M17. The usage walk repeats lookups it has made: per placed creature, per script string, per texture

- **Kind:** performance
- **Severity:** low (the manual gives about 5 s for The Domains of Dread with 143 haks; the cost grows with placed creatures times parts and with string occurrences, not distinct names)
- **Where:** crates/mg-preview/src/creature.rs:218, 422; crates/mg-preview/src/usage.rs:433-445; crates/mg-preview/src/files.rs:60-64, 181-215
- **Since:** after (the walk); `plt_fallbacks` before
- **Scenario:** a persistent world several times TDoD's size (20,000 placed part-based creatures, 19 parts each: 380,000 model reads; 10,000 scripts): the job's time is mostly these repeats.
- **Fix:** memoize `plt_fallbacks` by model name within a walk, keep a set of names `named` has tried, give `Gatherer` a seen-set of textures and make `missing` a set with an order.
- **Test:** a usage test over a counting container: 100 placed copies of one creature read each part's model once.
- **Confidence:** confirmed by reading, not measured (`plt_fallbacks` reads and parses each body part's model through `Lookup::model` for every placed creature and blueprint; `Walk::named` tries 15 types, a model and a texture for every occurrence of a string, in the source and again in the compiled script, each a pass over every layer in `ResMan::find`; `Gatherer::texture` has no seen-set, so each use re-reads the MTR and TXI, and `miss` scans the list of missing names)

## Not read

Nothing of the part's non-test source. The `#[cfg(test)]` modules and `tests/` folders were not reviewed (three were opened to see what a function is meant to do). Outside the part I read only what a finding needed: `mg_core::bin` (`CopyBudget`, `slice`), `ResMan::find` and its neighbors, `mg_rules::models`, `mg_script::ncs::calls`, `mg_area`'s border and tile reading, `mg-ui/src/blueprint/creature.rs:405-545`, `mg_module::bake::copy_model`.

## Looked at and found sound

- `mg-shader/src/spirv.rs` and `link.rs`: every operand goes through `arg()` or `get`, each index follows a length test or a map built from the same instructions, no loop depends on the module's content; the passes only ever see glslang's words (a cache hit goes to naga alone), and mg-render builds a pipeline under `catch_unwind`.
- The shader cache's integrity: an entry is named by release, `FORMAT`, stage and text, echoes key, stage and text length, and ends in a hash of all before it; a cut, damaged or foreign entry is passed over; it is written beside and renamed, by process id, and two threads of one process writing the same entry leave a whole file.
- The binary model reader apart from finding 1: every read is bounds-checked, counts are held against the file before anything is sized by them, vertex streams, node walks and walkmesh entries are charged, and node and walkmesh trees end by seen-sets and caps.
- The usage walk's ends and its attribution: `from` and the queue visit a resource once, `table_names` skips the table itself, GFF nesting is capped at 128, `ncs::calls` yields only routines the spec has, and a hak's label (`hak:{name}`) is made the same way in `add_haks` and `attribute`.
- `compose.rs`'s skeleton-bound body parts: `tree_order` returns every node, so the stand-ins, the bones moved by `from` and the lengthened bind list stay in step.
- `stdsort` under `compile`'s comparator (a strict weak order: the unguarded loops stop).
