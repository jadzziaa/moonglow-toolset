---
type: Report
title: 'Code review of November 2026: the finders and the timing runs'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the finders and the timing runs (clippy''s stricter lints, a clone finder, the frame and budget tests): 6 in all (0 high, 2 medium, 4 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, clippy, duplication, performance, frame-perf]
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

# Code review of November 2026: the finders and the timing runs

One part of [the review of 9 November 2026](overview.md): the finders and the timing runs (clippy's stricter lints, a clone finder, the frame and budget tests). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## Medium

### Z1. `mg tlk-export` to JSON panics on a talk table with an infinite sound length

- **Kind:** bug
- **Severity:** medium (a panic on a file's bytes; nothing is lost but the export)
- **Where:** crates/mg-tlk/src/json.rs:276-288 (`nim_float`: `format!("{:e}", inf)` is `inf`, which has no `e` for `split_once` and the `expect`), reached from :68-71; the same function is what a nasher project's talk table kept as JSON is saved through
- **Since:** before 2253fa78 (7a304d38)
- **Scenario:** a `.tlk` from another tool (or a damaged one) with a sound length of `7F800000`: `mg tlk-export` to JSON, `mg tlk-diff`'s JSON side, and the save of a nasher project that keeps its talk table as JSON all panic. `nwn_tlk` writes `"soundLen":"inf"` for it (a string) and then cannot read its own file back ("`not e.hasKey("soundLen") or e["soundLen"].kind == JFloat`"), so there is no oracle's reading to match.
- **Fix:** with F18 (one `nim_float` in mg-core for both crates): give a length that is not finite an error naming the line, or write it as `nwn_tlk` does and read `"inf"` back; either way no `expect` on a number's text.
- **Test:** unit test in `mg-tlk/src/json.rs`: `to_json` of a table with an entry of `f32::INFINITY` returns (an error, or text that `from_json` reads back to the same table).
- **Confidence:** confirmed by running (`mg` 3.18.1: a table made by `nwn_tlk` with the first entry's sound length set to the bytes of +infinity; `mg tlk-export bad.tlk bad.json` ends with "panicked at crates/mg-tlk/src/json.rs:287:47: {:e} has an e". The CSV export of the same table writes `inf` and exits 0. A length that is not a number is left out by the `length > 0.0` test and does not panic.)

### Z2. Encoding a dense scene's draws takes a fifth to a third longer than on 27 October, in steps of 1 and 2 November

- **Kind:** performance
- **Severity:** medium (in a view already over the budget and on the deferred list: "A dense area seen whole is still slower with the switch on")
- **Where:** crates/mg-render/src/pool.rs (a41b5bbb, "The meshes' chunks capped at half a million vertices": 13.2 to 14.4 ms across it); and the fourteen commits of 1 November's audit of frames and GPU memory (de4319b4 to 87f882e3: 11.8 to 13.4 ms), where a third bisect, three runs a step, puts a part at 3674c6b3 ("The window's device takes the GPU's memory in blocks of 8 to 64 MB": 12.2 ms before it, 12.7 to 13.1 at it) and a part at one of the three commits after it (30a2a09f, 94e4a46e, a4df4a0c: 13.5 to 14.0 at the last), each near what one run moves
- **Since:** after
- **Scenario:** TDoD's synthetic woods (24 by 24 tiles, 400 placeables: 24,965 draws of 1,796 materials, as on 27 October's 25,023 of 1,778) seen whole, Shaders off: draws encoded 10.8 to 12.1 ms then, 15.0 to 15.6 ms now; Shaders on 13.8 to 15.7 then, 16.9 to 18.0 now; turned, Shaders off 16.2 to 17.5 then, 20.0 to 21.5 now. The whole frame with the wheel turning: 13.7 to 14.4 ms then, 15.1 to 16.9 now (Shaders off); with Shaders on it is within what one run moves. The stock campaign's areas are unchanged (1.4 to 1.9 ms encoded either way). Commit by commit (the lower of two runs, overview, Shaders off): de4319b4 11.8 ms, 87f882e3 13.4, c28105dd 13.3, dcc4ee37 13.2, a41b5bbb 14.3, ee50505f 14.8, a46ba28a 15.0. The GPU's time falls from 7.6 to 5.8 ms between de4319b4 and 87f882e3 and stays there: that is the frame test's own change of that day (8c0a9203: its device takes BC textures, as the app's), not the renderer's.
- **Fix:** for the chunk cap, see what it costs a draw (more vertex buffers bound in turn, fewer draws joined) and either order the pass's draws by chunk or cap the chunks for what is not a tile; for the first step, a profile of the overview's encoding at de4319b4 and at a46ba28a (sampled stacks, as the frames note has it) rather than more bisecting: the steps left are each within a run's spread. A guess to try first, not tested: 3674c6b3's smaller memory blocks against the staging buffers a frame's writes take. Then the deferred entry's figures (13.1 and 15.8 ms seen whole, 21 and 28 turning, of 27 October) brought up to date: they were taken before either step and before the frame test drew shadows.
- **Test:** `frame_perf`'s `game_shaders_frames_at_144` with the builder's haks and `FRAME_SHADOWS=off`, the woods' "draws encoded", two runs before and after each change (one run moves by up to 2 ms).
- **Confidence:** confirmed by running (`game_shaders_frames_at_144` with The Domains of Dread's haks and `FRAME_SHADOWS=off`: the builds of 53633232 (27 October) and a46ba28a three runs each, turn about, on the development machine; then two bisects of two and three runs a step, the lower figure judged. A first bisect of one run a step named a commit that changes only `tabs.rs`: one noisy run of it, 18.5 ms where four later runs gave 12.4 to 13.0)

## Low

### Z3. A folder's linked files are not in the load order

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-resman/src/container.rs:318 (`DirContainer::rescan`: the user folder's `override/`, `development/`, a `--folder` layer, the tileset editor's folder); the same test in crates/mg-module/src/hak_edit.rs:52-55 (`folder_files`: Build Hak from Folder) and crates/mg-module/src/table_desc.rs:308-315 (the tables' descriptions beside a project's files)
- **Since:** before 2253fa78
- **Scenario:** on Linux a builder links a model or a script from a project's build folder into `development/` rather than copying it. If the game reads it (it opens files by name), Moonglow does not list it, draws the object without it, and Verify says it is missing.
- **Fix:** if the engine test shows the game reads them: `t.is_file() || (t.is_symlink() && e.path().is_file())` in the three places (a link to a folder is still not followed).
- **Test:** `tests/engine_*.rs`: a linked `.nss`/`.ncs` in the scratch `development/`, run by the server; then a unit test of `DirContainer` with a linked file.
- **Confidence:** suspected (read: `DirEntry::file_type` does not follow a link, so `is_file()` is false for a link to a file; what the game does with a linked file in `development/` or `override/` is not known: an engine test with a scratch user directory holding a linked script in `development/` would settle it)

### Z4. `mg gff` writes JSON as serde lays it out, though `mg_gff::to_json_text` writes it as neverwinter.nim does

- **Kind:** duplication
- **Severity:** low (the owner's call: the values are the same, the text is not)
- **Where:** apps/mg/src/main.rs:5769, :5776 (`serde_json::to_string_pretty`); crates/mg-gff/src/json.rs:300-309 (`to_json_text`: "laid out as neverwinter.nim (Nim's `pretty`) writes it", used by the nasher save, `mg merge-gff` and the prefabs alone)
- **Since:** before 2253fa78
- **Scenario:** a builder compares `mg gff x.utc` with `nwn_gff -i x.utc -p` (or keeps both tools' output in git): small and large floats, and control characters in strings, are spelled differently, beside the field order the manual already names. The command's help says "neverwinter.nim / nasher format".
- **Fix:** `to_json_text(&value, TextStyle::default())` for the file and the plain print (`--json` keeps the value); or say in the help that the notation is serde's.
- **Test:** `apps/mg/tests`: `mg gff` of a GFF with floats of 2.5e-7 and 1.5e16 gives `nwn_gff`'s spelling of both (the differential test's oracle, where it is installed).
- **Confidence:** confirmed by running (404 random FLOAT fields written by `nwn_gff` and by `mg gff` 3.18.1: 12 differ in notation, none in value: `0.000007345084668486379` against `7.345084668486379e-6`, `35986004112310270.0` against `3.598600411231027e+16`; the tie Rust's shortest digits round the other way, `2.4235000610351562`, comes out as Nim's)

### Z5. Written more than once, by the clone finder

- **Kind:** duplication
- **Severity:** low
- **Where:**
  - A byte offset's line number, counted nine ways: crates/mg-module/src/var_desc.rs:1465, doctor.rs:1569, refs.rs:267, areas.rs:575, project_file.rs:720; crates/mg-script/src/return_types.rs:40; apps/mg/src/lsp.rs:104; crates/mg-ui/src/script_nav.rs:140; crates/mg-nui/src/actions.rs:680.
  - `#rrggbb` read four times: crates/mg-rules/src/tint.rs:67-74 (`parse_bytes`), crates/mg-plugin/src/lib.rs:297-305 (`color_of`), crates/mg-ui/src/plugins.rs:2773-2779 (`rgb_of`), crates/mg-ui/src/color_button.rs:14-21 (`parse`, which also takes eight digits and `rgb()`).
  - The parameter sets' and the variable sets' files: crates/mg-ui/src/param_sets.rs:109-133 and :169-187 against var_sets.rs:95-121 and :165-183 (`load`, `read_all`, `save_file` and `dir`, the same but for the type): onto `saved::Shelf`, which both already keep their names by.
  - The line and line-copy pipelines: crates/mg-render/src/renderer.rs:1237-1270 and :1272-1305 (one descriptor twice, but for the entry point and the buffers).
  - The Spells page's two lists: crates/mg-ui/src/blueprint/creature_lists.rs:464-479 = :886-897, :521-532 = :963-974, :614-639 = :1055-1077.
  - crates/mg-module/src/instances.rs:278-323 = :583-628; crates/mg-module/src/nasher/glob.rs:132-142 = :164-175 (a pattern's folder and its rest); crates/mg-module/src/project_file.rs:821-830 = :866-875; crates/mg-shader/src/spirv.rs:526-535 = :822-830; apps/mg/src/nui.rs:157-164 = :244-249; crates/mg-area/src/scene.rs:735-743 = :756-764; crates/mg-preview/src/compose.rs:739-753 = :807-820; crates/mg-render/src/probe.rs:174-181 = crates/mg-ui/src/model_view.rs:916-923.
  - A blueprint page's first rows (name, tag, ResRef), written a kind: crates/mg-ui/src/blueprint/door.rs:85-94, placeable.rs:75-82, store.rs:67-75, sound.rs:26-34, trigger.rs:51-58, waypoint.rs:23-30, encounter.rs:48-55.
  - Functions of one name: `size_slider` (crates/mg-ui/src/appearance_gallery.rs:202, tile_select.rs:1392), `png` (crates/mg-module/src/minimap.rs:290, crates/mg-ui/src/nui_view/headless.rs:330), `outside_scan_of` and `outside_found` (crates/mg-module/src/folder.rs:98, :113 and nasher/project.rs:760, :778), `hold_save` (crates/mg-ui/src/claims.rs:287, bake_view.rs:459), `light_colors` (crates/mg-ui/src/tile_select.rs:969, crates/mg-area/src/scene.rs:1433).
- **Since:** mixed
- **Scenario:** none wrong found in these; each is a place the next change is made in one copy.
- **Fix:** `mg_core::text::line_of(text, at)`; one `#rrggbb` reader in mg-core (mg-rules, mg-plugin and mg-ui all sit above it); `Shelf::load`, `read_all` and `save_file` taking the set's reader; one function for the two pipelines; the rest as small helpers where they are.
- **Test:** the existing ones; a unit test of `line_of` at a text's ends and of the color reader with `#aéb23` (seven bytes, not a color, no panic).
- **Confidence:** confirmed by reading (each pair read after the finder named it; 31 runs of 90 or more equal tokens in the whole source, and 23 pairs of functions of one name with like bodies: the workspace has little copied text, and what follows is most of it that the readers did not name)

### Z6. What clippy's stricter lints leave, beyond style

- **Kind:** hygiene
- **Severity:** low
- **Where:**
  - Copies nothing needs (`redundant_clone`, 22): crates/mg-render/src/probe.rs:91, game.rs:2068; crates/mg-ui/src/tree.rs:1954, search.rs:393, blueprint/creature.rs:608, light_maker.rs:132, nui_view/headless.rs:592, commands.rs:1621, plugin_kinds.rs:341, :366, :384, outside.rs:155, area_view.rs:2440; crates/mg-plugin/src/runtime.rs:597; crates/mg-module/src/doctor.rs:2091, :3360, lights.rs:294, search.rs:1328, areas.rs:502; crates/mg-nui/src/properties.rs:144, :145, :149.
  - Lists made to be walked once (`needless_collect`, 11): crates/mg-ui/src/lib.rs:5450, paint_mode.rs:798, talk_view.rs:231, area_view.rs:4573, :7951, bulk.rs:62, area_props.rs:1186, references.rs:671, plugins.rs:1192; crates/mg-module/src/factions.rs:225; crates/mg-preview/src/usage.rs:1091.
  - Methods that read nothing of `self` (`unused_self`, 11): the NUI preview's `button`, `check`, `choices`, `chart`, `color_picker`, `image`, `label`, `slider`, `textedit` (crates/mg-ui/src/nui_view/preview/*.rs); crates/mg-ui/src/area_view.rs:2024; crates/mg-area/src/scene.rs:986.
  - Functions that can't fail and say they can (`unnecessary_wraps`): apps/mg/src/claims.rs:122; crates/mg-render/src/dx12.rs:156, assets.rs:131.
  - 246 string slices by byte (`string_slice`) were gone through: the ones that can fall inside a character are B4, P7, P8 and M10; the others cut where a search or an ASCII test put them.
  - 181 results thrown away (`let_underscore_must_use`) were listed, and those that are not a cleanup after a failure, a channel whose other end may be gone or a `write!` into a `String` were read (`ws.flush()` before a read in nine places of mg-ui, the 2DA editor's undo, `add_haks` for a codepage, the language server's cache file): none was found to hide an error the user needs; where a flush fails the reader gets the module as last flushed.
- **Since:** mixed
- **Scenario:** none at run time.
- **Fix:** take them out with the lint's own suggestions.
- **Test:** none (clippy, should the lints be turned on for the workspace: `redundant_clone` and `needless_collect` are cheap to keep clean).
- **Confidence:** confirmed by reading (`cargo clippy --workspace --lib --bins -- -W clippy::pedantic -W clippy::nursery` and a dozen restriction lints; the workspace's own lints are clean)

## Not read

Nothing was read whole by this pass: it looked only where a finder pointed. The 871 `cast_possible_truncation` and 3,111 `arithmetic_side_effects` hits were not gone through one by one (the readers of the format crates read those casts in place: F25, E22, B30).

## Looked at and found sound

- Exact duplication is low: 31 runs of 90 or more equal tokens in 305,000 lines, 3,492 tokens in all.
- The timing tests: `frames_at_144`, `frames_at_144_in_a_persistent_world`, `grass_frames_at_144`, `reload_frames_at_144`, `sound_ranges_frames_at_144`, `game_shaders_frames_at_144`, `editor_perf`, `nui_perf`, `world_perf` and `campaign_perf` all pass; no view is over the 6.94 ms budget that the deferred list does not already hold (the figures are in the overview).
- The four `#rrggbb` readers each check the text is ASCII hex before they slice it by bytes; `ResRef::name_of` makes a name ASCII before `nui_view/images.rs` cuts it.
