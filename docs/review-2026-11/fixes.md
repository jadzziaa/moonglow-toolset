---
type: Report
title: The fixes of the review of November 2026
description: 'What became of the 349 findings of the code review of 2026-11-09 in the fix pass of 9 and 10 November, for 4.0.0: a table a part (fixed whole, fixed in part, found fixed by another finding''s commit, does not hold, left), every finding not fixed whole with what is left and why, those fixed on 10 November (four settled by the game client and Aurora, R4 by the owner''s decision, Z2 in part after the reviewer''s timing run, which also found and fixed a slowdown of the content doctor), the findings fixed with a choice for the owner or a check that could not be made here, the last pass''s 24 items seen on the way, and the owner''s word on S15. Hand-written from the fixers'' reports and the commits; the part pages beside it are the record of what was found.'
tags: [review, code-review, fixes, deferred]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-10T09:23:56Z }
sources:
  - id: overview
    resource: overview.md
    title: 'Code review of November 2026: what was read, measured and found'
  - id: merges
    resource: 'develop: the merge commits 0991dd04 (F1 to F25, Z1), a5bcb20c (M1 to M17), 596f64b2 (P2 to P23), 68d36705 (C1 to C28, S14), f5e09b9e (R1 to R30), 64ef55b1 (E2 to E24, G1 to G11), 0d56a14c (V1 to V25), 9b7f3203 (D1 to D30), 6f41a5c6 (what the fixers saw on the way), 5bccca02 (S19 to S27, C27''s recovery half, D19 to D27''s rest), 16ffb761 (X1 to X25), 0c86c7e4 (N1 to N18), 682d2ef4 (B1 to B30), 993a29ce (A1 to A29)'
    title: The fourteen fixers' branches, merged into develop on 2026-11-09
  - id: reviewer
    resource: 'develop: the reviewer''s 20 commits between 325b07ea and 0991dd04 (git log --no-merges 325b07ea..0991dd04), E1, S1 to S13, S16 to S18, S22, S28, P1, A2, A3, D4, D7, D13, B6 in part, B7, B8, B16, B17'
    title: The reviewer's own fixes, before the fixers began
  - id: reports
    resource: target/review/reports/*.md of the main checkout (the fourteen fixers' final reports) and target/review/fix-notes.md (the reviewer's running notes), not kept in the repository
    title: The fixers' reports and the reviewer's notes
  - id: later
    resource: 'develop: the merge commits 27924c61 (the last pass''s items 1 to 8, 2026-11-09), bfe949ac (R18, R29, N9, G8, settled by the client and Aurora, 2026-11-10) and d82e6390 (items 9 to 24, Z3 to Z6, 2026-11-10); their reports in target/review/reports/ of the main checkout (a79b65aac5ec49eff.md, abad2bb7134eaad26.md, ab3b479dd6ddc02fd.md)'
    title: The last pass and the client and Aurora readings, merged on 9 and 10 November
  - id: timing
    resource: 'crates/mg-ui/tests: frame_perf.rs, editor_perf.rs, nui_perf.rs, world_perf.rs, campaign_perf.rs, three runs each, the lowest figure, on the development machine, 2026-11-10; the commits 26cf02cc, e8c4b049, 6b26e702 and 452bba34'
    title: The reviewer's timing run of 10 November and the fixes it led to
  - id: owner
    resource: human:august
    title: 'The owner (2026-11-09 and 10): fix the findings; the release is 4.0.0; S15 settled; Z4 and the last pass''s item 11 decided'
---

# The fixes of the review of November 2026

The owner asked for the review's findings to be fixed the day it was
written.[^owner] Twelve fixers took a part's page each (one took the
workspace's and the rules' together), a thirteenth what the window's
shell and the 2DA and hak editors left after the first round, and a
fourteenth what the fixers saw on the way; the reviewer had fixed the
lost-work findings of the window himself before they began.[^reviewer]
Each fixer read a finding's place again before changing it, wrote a
test with each fix that could have one, and made about a commit a
finding; each branch was merged into develop.[^merges] What follows
counts the findings of [the overview](overview.md)[^overview] by what
the fixers' reports and the commits say became of them.[^reports] On
10 November the game client and Aurora settled four findings left
waiting on them, a last pass fixed the finders' Z3 to Z6 and what the
fixers had seen on the way, and the reviewer's timing run measured
the fixes and Z2; the counts below are as of then.[^later][^timing]

The counts: **fixed whole**, the finding's fix made and tested (by its
part's fixer or the reviewer); **in part**, a part its fixer names as
left or not done; **by another**, found already fixed by another
finding's commit; **does not hold**; **left**, nothing changed.

## By part

| Part (page) | Findings | Fixed whole | In part | By another | Does not hold | Left |
| --- | --- | --- | --- | --- | --- | --- |
| [Formats and core](formats-core.md) (F) | 25 | 25 | 0 | 0 | 0 | 0 |
| [Models, shaders and previews](models-shaders-previews.md) (M) | 17 | 17 | 0 | 0 | 0 | 0 |
| [The renderer](renderer.md) (R) | 30 | 29 | 1 | 0 | 0 | 0 |
| [The workspace, areas, tiles, rules and NUI documents](edit-area-tiles-rules-nui.md) (E) | 24 | 23 | 0 | 1 | 0 | 0 |
| [The rules data and what the first pass left](rules-leftovers.md) (G) | 11 | 10 | 1 | 0 | 0 | 0 |
| [Verify, search, rename and the module's queries](module-checks.md) (V) | 25 | 23 | 2 | 0 | 0 | 0 |
| [Opening, saving and the project's files](module-project.md) (P) | 23 | 22 | 1 | 0 | 0 | 0 |
| [The plugin host and the command line](plugins-cli.md) (C) | 28 | 28 | 0 | 0 | 0 | 0 |
| [The window](ui-shell.md) (S) | 28 | 27 | 0 | 0 | 1 | 0 |
| [The area editor](ui-area.md) (A) | 29 | 28 | 0 | 1 | 0 | 0 |
| [The blueprint editors, palettes and pictures](ui-blueprints.md) (B) | 30 | 28 | 0 | 2 | 0 | 0 |
| [The 2DA, conversation, talk table and hak editors and the module-wide tools](ui-editors.md) (D) | 30 | 28 | 2 | 0 | 0 | 0 |
| [Plugins in the window, the NUI Creator's editor side and the script editor](ui-plugins-nui-scripts.md) (X) | 25 | 25 | 0 | 0 | 0 | 0 |
| [The NUI Creator's drawing side, the Script Wizard's window, the manual and the icons](ui-nui-drawing.md) (N) | 18 | 15 | 2 | 1 | 0 | 0 |
| [The finders and the timing runs](mechanical.md) (Z) | 6 | 5 | 1 | 0 | 0 | 0 |
| **All** | **349** | **333** | **10** | **5** | **1** | **0** |

Found fixed by another finding's commit: E6 by D2 (`e0a9f0f7`, Advanced
Search and Retag act only in the module as they found it), A4 by V1
(`8cbd8e7d`), B4 by P8 (`b8138ba3`), B12 by V19 (`3ab7bf24`), N2 by G1
(`b47753a3`; its tests in mg-ui added with N1). Counted as fixed whole
with another's help: X15 (its main half F10's `75603b23`), B6 and B16
(the reviewer's in part), C27 (its settings half by the plugin host's
fixer, its recovery half by the shell's).

## Not fixed whole

| Id | What is left | Why |
| --- | --- | --- |
| R5 | (d): the shadow volume buffers kept in each `GpuMesh` for the model's life; (a) to (c) fixed | beyond what was asked: freeing them is a restructure of the casters' path; the Z2 measurement it waited on was made on 10 November |
| G4 | the "only when the header is open" half: while the search has text, the Properties page matches every subtype row each frame | not done by its fixer; its page is timed (1.6 ms) but not with search text: filed with the views still to be made |
| V5 | whether the CSV export should begin with a byte order mark (Import Lines reads one now) | the owner's decision |
| V14 | a hak rebuilt with the same time, size and layout, only its data changed, is taken for the old one | the owner's decision: hashing a hak's data means reading it each run |
| P4 | (c): no bound on how far past its end an import may grow a table | the owner's decision: refuse or ask past some number of empty lines |
| D19 | the module's Save and the NUI picture hak still write a hak at once; Add Haks has no byte progress or Cancel mid-copy | a real unintended consequence: what follows those saves in the same step reads the files written |
| D26 | the half that remembers the editor's own edits as `renumbered: None` | not done: its fixer judged it would save little after the fix; not filed |
| N13 | `check`'s own copy a frame, Interact's two whole comparisons of its session, a variant's context document cached | suspected, not measured: the Design page of 300 controls has no timing view (filed with the views still to be made) |
| N16 | Options' and Tabs' entries, named only in the entry's confidence line | bounded by the file's size; filed with N13 |
| Z2 | in part: Moonglow's own see-through draws are drawn chunk by chunk (`6b26e702`, `452bba34`); against the build of 27 October with the same haks, TDoD's woods seen whole still take 2.4 ms more to encode with the Shaders switch off (12.6 against 10.2) and 2.7 more on, 5 ms more turned with it on | profiled on 10 November: no one cause is left (wgpu's encoding of the pass, the batches, the frame's values, sorting, the lights' reach); the game's see-through draws keep their runs by material, as splitting them by chunk cost the GPU more than it saved |

Each of these but D26 is on [the deferred list](../deferred.md) with
its reason (G4's and N16's in the entries of the timing views and of
N13; Z2's in the entry of a dense area seen whole). D26's half was
judged not worth doing.

## Fixed on 10 November

Ten findings counted as left or in part on 9 November were fixed the
next day, Z2 of them in part.[^later] The game client (on the off-screen display) and
Aurora settled four of them, each with a new reading or capture:

| Id | Fixed by | How it was settled |
| --- | --- | --- |
| R18 | `1aec9e42` | the client: a chunk turns `particlerot` radians a second about its emitter's z, born facing as the emitter does (`reading_chunk_spin`); Moonglow turned chunks 6.28 times faster about a random axis |
| R29 | `ece0fe7a` | the client: a face flat in its texture coordinates takes the compiler's tangent, the normals' mean turned (`reading_tangents_flat_in_uv`); the renderer now calls `mg_mdl::compile::triangle_tangent`, one rule instead of two |
| N9 | `2f4d8c4b` | the client: an end button scrolls every frame it is held, by 1% of the bar, not by a step a click (`reading_scrollbar_end_buttons`) |
| G8 | `c9830f12` | Aurora: its Levelup Wizard, captured adding Paladin 4 at Wisdom 14, Sorcerer 1 and Bard 1 (`levelup/spells-after.mod`), gives the paladin its first-level spell; the sorcerer's and bard's were already Aurora's |

The finders' four, given to the last pass's fixer:

| Id | Fixed by | What is left |
| --- | --- | --- |
| Z3 | `69262a8b` | nothing: the engine test `engine_linked_files` shows the game reads linked files in `development/` and `override/`, and Moonglow's load order now has them |
| Z4 | `fdd28e9d` | nothing: by the owner's decision of 10 November ("mg gff should write in neverwinter.nim's notation"), its file and plain print go through `to_json_text`, checked against `nwn_gff`; `--json` keeps the value |
| Z5 | fifteen commits, `b3e77f32` to `2e52cbf3` | three pairs found not to be true copies: `outside_found` (its times kept in different places), `hold_save` (each reads its own state and asks its own question), `light_colors` (different columns and types); the blueprint pages' ResRef row was already gone |
| Z6 | `9d3fba98` | collects that are needed (in `bulk.rs`, `lib.rs`'s fixes, `follow.rs`'s pages and `situated.rs`) and dx12's `load` off Windows; `assets.rs:131` was already clean |

What Z5 and Z6 leave does not hold as a finding, so they count as fixed
whole and nothing of them is on the deferred list. Choices the client
reading and the capture made are in the next table (N9, G8).

The two left at the end of 10 November's first documents, the owner's
decision and the reviewer's measurement:[^timing]

| Id | Fixed by | How |
| --- | --- | --- |
| R4 | `07e71245` | by the owner's decision of 10 November ("I'm fine with also disabling light shadows when creature shadows are off."): Light Shadows need Creature Shadows, and are grayed out without them, the tick kept; one rule, `mg_area::View::light_shadows_cast`, read by the scene and the menu |
| Z2 | `6b26e702`, `452bba34` (in part) | the reviewer's timing run: half the draws of TDoD's woods seen whole bound another of its 13 mesh chunks than the draw before; Moonglow's own see-through draws now go in runs of one chunk too where nothing they meet is between, the same picture (the frame tests' views compared pixel by pixel): 12,621 chunk changes to 2,159, draws encoded 14.8 ms to 12.6 seen whole and 19.7 to 17.9 turned (Shaders off; lowest of three) |

The same run found world_perf's content doctor ten times slower than on
9 November (16.9 s, its budget 3 s), since V6 had each worker parse the
2DAs again at every split of the module's resources; fixed (`26cf02cc`,
0.51 s), with Advanced Search's frame over the budget with 8,805 hits
ticked (`e8c4b049`, 7.8 ms to 4.3).

## Fixed, with a choice or a check left

| Id | What | Why it stays open |
| --- | --- | --- |
| F5 | NWNT refuses text with a typed `\n` or `\r` rather than saying so in the log | the owner's decision |
| F7 | a name past ASCII through a folder is kept as text, a byte a character, not as raw bytes | the owner's decision |
| P17 | bulk saves (a module folder's, a project's, NWSync's data files) are not flushed file by file | the owner's decision (a flush costs about 1 ms a file) |
| C2 | what a plugin's job makes the host hold is bounded at 512 MB | the owner's decision |
| R (the renderer's page) | the model viewer's ground shadow is always a shadow map | the owner's decision |
| B19 | a preview shown twice in a frame says "The preview is shown elsewhere in the window." in the second place | the owner's decision (its wording and behavior) |
| A12 | a drag is dropped, not committed, when a key changes the mode | the owner's decision |
| A14 | in Paint, Select Tiles and with a tile brush, Alt + drag always moves the view, whatever is selected | the owner's decision |
| N9 | the preview's end buttons scroll at the client's rate for 60 frames a second; the text control at a group's rate; the list by pixels, not whole rows | the owner's decision (the client's rate follows its frame rate; why some presses on its text control moved nothing is not settled) |
| G8 | the capture's FeatList is compared sorted by name, as Aurora's Properties window writes it, for that capture alone | the owner's decision; whether Moonglow's Properties window should sort them is not checked |
| F19, F23 | `hdiutil` and `ditto` by full path; the copied-out app's seal checked before the swap | not on this machine: macOS |
| F20, P21, R8, D16, C27 | the game looked for before drive letters (not timed); a hak named `D:evil.hak` not written; a real driver reset; `cmd /S /C`; the recovery lock's Windows branch | not on this machine: Windows (compiled; D16's test runs on CI) |
| R25, S16 | `SystemVersion.plist`; Option with a key that types kept for a text field | not on this machine: macOS (from reading) |
| M10, M13, F22 | a two-digit phenotype's own textures; a held item with no model; the client's CD key copy removed | waits on a client run |
| V7 | whether Twine keeps the `moonglow` object in `StoryData` | waits on Twine |
| N17 | the gigabyte glyph itself not reproduced (the test's font has no glyphs) | waits on such a font |

Parts that did not hold, inside findings fixed whole: D6's delete half
(a deletion of several already leaves a protected area out); M12 holds
only for Moonglow's own compiler (no null tree pointer among 42,782
walkmesh nodes of the 147,175 compiled models swept); S26 (d) holds in
principle and is fixed; S27 (c) was closed by the first pass's `cut`.
Two questions came from what the fixers saw on the way, not from a
finding: whether Verify should report every mistake of a table's
description or the first, and what `mg-set` should do with a junk
`Count`; both are on the deferred list.

## The last pass: what the fixers saw on the way

After the fourteen branches, a last pass took up 24 things the fixers
had seen in passing, outside any finding: items 1 to 8 by one fixer
(merged on 9 November), 9 to 24 by another (merged on 10 November,
with Z3 to Z6 above).[^later]

| Item | What | Outcome |
| --- | --- | --- |
| 1 | the creature page's sound-set sample name | fixed `1628dfe0`: named by its field up to the first NUL |
| 2 | an update downloaded into the system's temporary folder | fixed `4e6b9729`: never; with no cache folder only the release page is offered |
| 3 | a name with a letter outside ASCII spelled in a script (Find References, the rename's count and warning, a delete's message) | fixed `4934c416`: looked for by the name's bytes (`mentions_of`) |
| 4 | a module's kept pipelines after the resources change | fixed `a104c93b`: those made since a mark that holds |
| 5 | Remove Unused Data not stopping at Cancel | fixed `84804e30`: it stops and says what it had removed |
| 6 | the renderer's textures after an Import | fixed `44621f99`: forgotten where the module's DDS, TGA, PLT, MTR or TXI changed; the rest of B15's entry stays on the deferred list (a hak's model from its own load order) |
| 7 | `mg rename` of a NUI window whose opener does not compile | fixed `6a7e7afc`: the command fails, the rename saved |
| 8 | unit tests' scratch folders in the system's temporary folder | fixed `344e9f17` (about 45 places), and `f3b8e27a` for the scratch name it made twice |
| 9 | Move Name to Talk Table and a table's New Line adding a line at the click | fixed `bbadd43d`: lines kept pending and added at OK or the module's Save |
| 10 | the hak editor's game-folder check by the path as written | fixed `bdb1b4b9`: through a link too |
| 11 | an installed plugin's grants held by id and version, not its code | decided by the owner on 10 November 2026: yes; fixed `cdc2feac`: held to its code's digest, the old grants carried over once |
| 12 | a draw layer's open header kept by index | fixed `8c8d2627`: it follows the layer through Up, Down and Remove |
| 13 | String Edit of a conversation's line behind another tab | fixed `599d12be` (not in `follow.rs`: the dialog view's renumbering run each frame) |
| 14 | `--state` asking for a row past those laid out | fixed `1665c94b`: the list's real row count is said |
| 15 | a combo popup's wheel and bar; five scroll maps cleared by hand | fixed `50c63cc4` |
| 16 | the parameter sets' Save Set replacing a set silently | fixed `fd569757` |
| 17 | four things made every frame or every job (the wizard's categories, row notes, the tree's row order, a hak-only job's module) | fixed `b7b6bb6c`; its views wait on the timing run (deferred list) |
| 18 | blueprint-page state kept by document, not window | fixed `a0afbbf4` (no new test) |
| 19 | `view.drag` and `app.area_select` not followed | `view.drag` does not hold (a spawn-point drag is already dropped; the others act on the selection); `app.area_select` and `area_focus` fixed `c9569be4` |
| 20 | the visual transforms' engine test's step name | fixed `fd0e41ca` |
| 21 | `Watch::restart` warm-up on the Shaders switch; `particle_depth` one slot | `Watch::restart` does not hold (`judge` resets its warm-up while the game's programs are pending); `particle_depth` fixed `d3602354`, per view |
| 22 | a failed shader cache write leaving its temporary file | fixed `8e65e9ba` |
| 23 | text naming the release "3.18.2" or "the release after" | does not hold: no such text of the fixers'; `Cargo.toml` says 4.0.0 |
| 24 | backups, Print, Round-Trip copies, the external editor's script and `mg lsp`'s fallback under guessable names in the temporary folder | fixed `4f419d47`: the data folder's `conversation-backups` and a per-run folder of the user's own (`own_scratch_dir`) |

Of the 24: 21 fixed whole (11 by the owner's decision), 2 fixed in part
with the rest not holding (19, 21), and 1 does not hold (23). Seen on the way and filed on the deferred list, beyond
what was asked: `mg serve`'s run folders under `/tmp/moonglow-serve`,
tests that write under the real `~/.cache/moonglow/scratch`, and the
area view never drawing chunk emitters.

S15: settled by the owner on 9 November 2026: the deferred list's account of the behavior is correct; nothing changed.

[^overview]: Code review of November 2026: what was read, measured and found
[^merges]: The fourteen fixers' branches, merged into develop on 2026-11-09
[^reviewer]: The reviewer's own fixes, before the fixers began
[^reports]: The fixers' reports and the reviewer's notes
[^later]: The last pass and the client and Aurora readings, merged on 9 and 10 November
[^timing]: The reviewer's timing run of 10 November and the fixes it led to
[^owner]: The owner (2026-11-09 and 10): fix the findings; the release is 4.0.0; S15 settled; Z4 and the last pass's item 11 decided
