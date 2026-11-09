---
type: Report
title: 'Code review of November 2026: what was read, measured and found'
description: 'A dated record (2026-11-09, develop at a46ba28a, version 3.18.1) of a review of the whole workspace for bugs found by reading, performance, logic written more than once and hygiene: how it was done (fourteen readers over 305,000 lines of source, clippy''s stricter lints, a clone finder, the timing tests), the count (349 findings, 37 of them high), what the timing tests measured (every view inside its budget but the two the deferred list holds; a dense scene''s draws a fifth to a third slower to encode than on 27 October, in steps of 1 and 2 November), the patterns that repeat across the parts, the high-severity findings one by one, what was not read, and an order to fix in. Nothing in the code was changed by the review: the findings are in the part pages beside this one.'
tags: [review, code-review, bugs, performance, duplication, hygiene, frame-perf]
status: draft
generated: { by: claude-code/claude-fable-5-2, at: 2026-11-09T22:00:43Z }
stale_after: 2027-01-01T00:00:00Z
sources:
  - id: owner
    resource: human:august
    title: 'The ask (2026-11-09): a code review of the codebase for code hygiene, performance, bugs inferred from digesting the code and deduplication of repeating functions or methods, as before'
  - id: code
    resource: 'crates/*/src and apps/*/src at commit a46ba28a (develop, 2026-11-09): 305,000 lines of Rust, without the tests, the generated schema and vendor/'
    title: The code as read
  - id: last-review
    resource: commit 2253fa78 (2026-10-27), and the log's entries of that day
    title: The last whole-workspace review; 996 commits lie between
  - id: clippy
    resource: 'cargo clippy --workspace --lib --bins with clippy::pedantic, clippy::nursery and a dozen restriction lints, clippy 0.1.99, 2026-11-09 (15,689 hits)'
    title: Clippy's stricter lints as a finder
  - id: timing
    resource: 'crates/mg-ui/tests: frame_perf.rs (the six frames_at_144 tests, and game_shaders_frames_at_144 with MOONGLOW_HAKS at ~/Projects/tdod-module/build/hak, with and without FRAME_SHADOWS=off), editor_perf.rs, nui_perf.rs, world_perf.rs, campaign_perf.rs, release builds on the development machine, 2026-11-09'
    title: The timing tests' runs
  - id: bisect
    resource: 'Three git bisects between 53633232 (2026-10-27) and a46ba28a in a scratch worktree, each step a release build and one, two or three runs of game_shaders_frames_at_144 with the builder''s haks and FRAME_SHADOWS=off (target/review/bisect/ of the main checkout, not kept in the repository)'
    title: The bisects for the slower encoding of a dense scene's draws
  - id: oracle
    resource: 'nwn_gff and nwn_tlk of neverwinter.nim (~/.local/opt/neverwinter/bin) and the installed mg 3.18.1, on scratch files'
    title: The two findings that were run against the oracle tools
---

# Code review of November 2026: what was read, measured and found

The whole workspace was read on 9 November 2026 for four things, in this
order: bugs that reading shows, work done more often than it needs to be,
logic written more than once, and hygiene. It was last read whole on 27
October[^last-review]; 996 commits have landed since, and what they
changed was read first and most closely. This page is the record: how it
was done, the count, what was measured, what repeats, the high-severity
findings, what was not read, and an order to fix in. The findings
themselves, each with its place, a scenario, a fix and the test that
would catch it, are in the part pages listed under [The count](#the-count).

**Nothing in the code was changed by the review.** A finding is a claim
from reading unless its entry says it was run; the reviewer read the
high-severity ones again against the code, and says so in each.

## How it was done

- **Fourteen readers.** The source (`crates/*/src` and `apps/*/src`,
  305,000 lines[^code]) was cut into twelve parts of 20,000 to 30,000
  lines, each read by a reviewing agent under one brief: what to look
  for, the house rules a finding is judged by (parsers never panic, a
  file named after a resource goes through `ResKey::path_in`, external
  programs through `output_within`, lossless editing, every edit through
  the workspace and undone exactly, files written through a temporary
  file, nothing written to the player's NWN folder), what the last
  review settled (a per-frame cost is a finding only with the input that
  makes it matter; what `docs/deferred.md` holds is not reported again),
  to read the code and not a summary of it, to check a bug through its
  callers before reporting it, and to say what was not read. Two parts'
  readers ran out of room; two more readers read what they had left
  (most of `mg-rules`, and the NUI Creator's drawing side). Several
  readers split their part among sub-readers; each entry says who read
  what. Tests, the generated schema and `vendor/` were not under review.
- **Clippy's stricter lints as a finder**[^clippy]: 15,689 hits, nearly
  all style. The ones that point at defects or waste (results thrown
  away, string slices by byte, redundant clones, loops over floats,
  identical match arms and a dozen more) were gone through one by one.
- **A clone finder**: runs of equal tokens between any two places of the
  source, and functions of one name whose bodies are alike.
- **The timing tests**[^timing], as the measure of what is slow today.
- **Reading again.** The reviewer read all 37 high-severity findings
  again at the lines they name, a random sample of ten medium and four
  low bugs, and seven more mediums met on the way: all 58 hold at
  those lines (each entry says what was seen). That is a reading of the cited code, not a reproduction:
  the scenarios were not run. The rest stand on their readers' reading.
  Two findings were run: the talk table's infinite length
  ([Z1](mechanical.md)) against the installed `mg` and
  `nwn_tlk`[^oracle], and the slower draws ([Z2](mechanical.md)) by
  three runs each of two builds and three bisects[^bisect].

## The count

| Part | Id | High | Medium | Low | All |
| --- | --- | --- | --- | --- | --- |
| [Formats and core](formats-core.md): mg-core, mg-gff, mg-erf, mg-key, mg-2da, mg-tlk, mg-ssf, mg-set, mg-image, mg-resman, mg-audio, mg-script, mg-serve, mg-testkit, mg-update | F | 4 | 7 | 14 | 25 |
| [Models, shaders and previews](models-shaders-previews.md): mg-mdl, mg-shader, mg-preview | M | 2 | 6 | 9 | 17 |
| [The renderer](renderer.md): mg-render | R | 2 | 7 | 21 | 30 |
| [The workspace, areas, tiles, rules and NUI documents](edit-area-tiles-rules-nui.md): mg-edit, mg-area, mg-tiles, mg-rules, mg-nui, mg-schema | E | 1 | 7 | 16 | 24 |
| [The rules data and what the first pass left](rules-leftovers.md): mg-rules, parts of mg-nui, mg-area, mg-schema and mg-module | G | 0 | 4 | 7 | 11 |
| [Verify, search, rename and the module's queries](module-checks.md): mg-module | V | 2 | 9 | 14 | 25 |
| [Opening, saving and the project's files](module-project.md): mg-module | P | 2 | 5 | 16 | 23 |
| [The plugin host and the command line](plugins-cli.md): mg-plugin, apps/mg, apps/moonglow | C | 2 | 8 | 18 | 28 |
| [The window](ui-shell.md): mg-ui: lib, widgets, commands, keys, tabs, tree, settings, options, recovery, jobs | S | 5 | 14 | 9 | 28 |
| [The area editor](ui-area.md): mg-ui | A | 4 | 8 | 17 | 29 |
| [The blueprint editors, palettes and pictures](ui-blueprints.md): mg-ui | B | 6 | 19 | 5 | 30 |
| [The 2DA, conversation, talk table and hak editors and the module-wide tools](ui-editors.md): mg-ui | D | 4 | 22 | 4 | 30 |
| [Plugins in the window, the NUI Creator's editor side and the script editor](ui-plugins-nui-scripts.md): mg-ui | X | 1 | 7 | 17 | 25 |
| [The NUI Creator's drawing side, the Script Wizard's window, the manual and the icons](ui-nui-drawing.md): mg-ui | N | 2 | 4 | 12 | 18 |
| [The finders and the timing runs](mechanical.md): clippy's stricter lints, a clone finder, the frame and budget tests | Z | 0 | 2 | 4 | 6 |
| **All** | | **37** | **129** | **183** | **349** |

By kind:

| Kind | High | Medium | Low | All |
| --- | --- | --- | --- | --- |
| Bugs | 37 | 91 | 83 | 211 |
| Performance | 0 | 37 | 52 | 89 |
| Written more than once | 0 | 1 | 23 | 24 |
| Hygiene | 0 | 0 | 25 | 25 |

An id is the part's letter and the reader's number. *High* is data
loss, a crash, a hang, a file written where it mustn't be, or a
sandbox's limit that isn't one; *medium* a wrong result a builder would
see, or a cost that stops the window at a size builders have; *low* the
rest.

## What was measured

Every timing test passes, and no view is over its budget that the
deferred list does not already hold[^timing].

| Test | What it showed |
| --- | --- |
| `frames_at_144` and the four beside it (Tyrants of the Moonsea) | Every view inside 6.94 ms but the largest area (`area084`, 676 tiles) seen whole or turned with the wheel: 8.2 to 9.4 ms, the deferred list's "The largest areas in full view". The same area with everything selected and its walkmesh shown: 5.98 ms, worst 7.46. |
| `frames_at_144_in_a_persistent_world` | Every view inside the budget (the log of 10,000 lines 1.7 ms, the Export window of 25,000 resources 1.7 ms). |
| `game_shaders_frames_at_144` with The Domains of Dread's haks | The synthetic woods seen whole: 16.5 ms with Shaders off, 19.2 on; turning 25.5 and 33.3: the deferred list's "A dense area seen whole is still slower with the switch on", and slower than its figures of 27 October (below). `area092` and the water area inside the budget either way. |
| `editor_perf` | Typing in `nwscript.nss` (13,869 lines): 1.86 ms a key, the frame after it at a typist's pace 7.07 ms (worst 8.64: the one figure of this run over 6.94). A 15,100-row 2DA idle 0.72 ms, a cell set each frame under a Find 3.4 ms. |
| `nui_perf` | A large window idle 1.14 ms, 2.47 ms after an edit. |
| `world_perf` (25,028 resources, 300 areas) | Every step inside its budget: Verify 1.18 s, the content doctor 1.67 s, compiling 4,000 scripts 1.12 s, a custom palette of 8,000 items 3.8 ms a frame. |
| `campaign_perf` | The three campaigns pass; its two tests on a builder's haks were not run (no `MOONGLOW_HAKS` in that run). |

**A dense scene's draws take longer to encode than on 27 October**
([Z2](mechanical.md)). The deferred entry's figures were taken before
the frame test drew shadows, so the woods were timed again with
`FRAME_SHADOWS=off`, and the build of 27 October (`53633232`) was timed
beside today's on the same machine, three runs each, turn about. The
scene is the same (25,023 draws of 1,778 materials then, 24,965 of 1,796
now).

| TDoD's woods, shadows off, ms | 27 October's build | Today's | Change |
| --- | --- | --- | --- |
| Overview, draws encoded, Shaders off | 10.8 to 12.1 | 15.0 to 15.6 | +32% |
| Overview, draws encoded, Shaders on | 13.8 to 15.7 | 16.9 to 18.0 | +18% |
| Turned, draws encoded, Shaders off | 16.2 to 17.5 | 20.0 to 21.5 | +23% |
| Overview, the GPU, Shaders off | 7.6 | 5.9 | −22% |
| Whole frame, wheel turning, overview, Shaders off | 13.7 to 14.4 | 15.1 to 16.9 | +11% |
| Whole frame, wheel turning, overview, Shaders on | 16.7 to 19.8 | 17.3 to 17.8 | within a run's spread |

Bisected (two and three runs a step, the lower figure judged), the
slowdown has no one cause: it came in steps on 1 and 2 November. About
1.5 ms came with the fourteen commits of the audit of frames and GPU
memory (11.8 ms before them, 13.4 after), of which the device's smaller
memory blocks (`3674c6b3`) and one of the three commits after it each
show a part, near what one run moves; 1 ms came with the cap on the
meshes' chunks (`a41b5bbb`: 13.2 to 14.4 ms), which saved 46 MiB on this
scene; the rest (to 15.0) with the commits after. The GPU's fall is the
frame test's own change of that day (`8c0a9203`: its device takes BC
textures, as the app's), not the renderer's. A first bisect of one run a
step named a commit that could not be it (a change to how tabs scroll:
one run of 18.5 ms where four later runs gave 12.4 to 13.0), which is
why the later ones took two and three. What is left is a profile's work,
not a bisect's: [Z2](mechanical.md) has the figures commit by commit.

What the timing tests do not measure is where the readers' performance
findings are: none of the 88 was timed (the readers ran nothing), each
names the input that makes it matter, and most name the `frame_perf`
view that would show it. The last review's rule stands: measure before
changing a per-frame cost.

## What repeats

The same few mistakes account for most of the high and medium findings.
Each is better fixed in one place than finding by finding.

1. **Unsaved work lost without a word.** Whether the module is modified
   is a count of commands, which an Undo past the save and a new edit
   bring back to zero ([E1](edit-area-tiles-rules-nui.md)). The module's
   Undo and Redo drop every script editor's unsaved text
   ([S1](ui-shell.md)); text typed in script editors alone never gets a
   recovery copy ([S4](ui-shell.md)); Recover, New Module's Create and
   Open Directory as a Module replace the open module without asking
   ([S2](ui-shell.md)); a pull that adds an area counts unsaved
   Module Properties edits as read ([P1](module-project.md)); settings
   that do not read are replaced by the defaults and saved
   ([C8](plugins-cli.md), [S14](ui-shell.md)); recovery copies left
   behind or taken for the module's file ([S6, S7](ui-shell.md)); a
   conflict waiting to be answered forgotten when another file changes
   ([D5](ui-editors.md)).
2. **A folder joined with a name that came from a file.** The rule
   (`ResKey::path_in`) was made at the last review; these writers, added
   since or missed then, do not follow it: the external compiler's scratch
   files ([P2](module-project.md)), Export Minimaps and Render Minimap
   Pictures ([A2, A3](ui-area.md)), the conversations' export
   ([D4](ui-editors.md)) and backups ([S8](ui-shell.md)), a module's hak
   and talk-table names ([F10](formats-core.md),
   [X15](ui-plugins-nui-scripts.md)), paths the project file names
   ([P12](module-project.md)), an archive's `D:x.hak`
   ([P21](module-project.md)), `mg nui render`'s pictures
   ([C26](plugins-cli.md)). Beside them, two commands that write before
   they check where: `mg test --dir` into the player's folder
   ([C1](plugins-cli.md)), and an update over whatever `$APPIMAGE` names
   ([F1](formats-core.md)).
3. **A size, a count or a depth taken from a file.** Allocations and
   loops bounded only by what a hak or module claims: a tileset's
   section numbers ([F2](formats-core.md)), `ENGINE_STRUCTURE_<n>`
   ([F3](formats-core.md)), a compressed resource's header
   ([F8](formats-core.md)), a model's controller keys and an ASCII key
   list ([M1, M2](models-shaders-previews.md)), an ARE's width and
   height ([V1](module-checks.md), [E7](edit-area-tiles-rules-nui.md)),
   2DA cells ([G7](rules-leftovers.md)), a talk-table import's line
   number ([P4](module-project.md)), a plugin's talk table
   ([C2](plugins-cli.md)); recursion with no bound in the conversation's
   text export ([V2](module-checks.md)), the model writer
   ([M3](models-shaders-previews.md)), a constant's brackets
   ([P9](module-project.md)), a NUI bind that names itself
   ([G1](rules-leftovers.md)), NWNT ([F24](formats-core.md)); a box of
   squares whose count overflows ([R1](renderer.md)).
4. **An object held by its place in a list, across edits that move the
   list.** Six readers reported it independently, in more than a dozen
   windows: Variables and String Edit ([S3](ui-shell.md),
   [B6](ui-blueprints.md)), Advanced Search's Change and Duplicate Tags
   ([D2](ui-editors.md), [E6](edit-area-tiles-rules-nui.md)), the area
   view's selection, Adjust Location, Bake Walkmesh, Change Light,
   Redraw Polygon, Update Instances, Find Instance
   ([A5, A6, A8](ui-area.md)), a plugin panel's buttons
   ([X7](ui-plugins-nui-scripts.md)), the conversation clipboard and
   Input Text ([D11, D15](ui-editors.md)), Missing Feats, Select Texture
   and the journal ([B11](ui-blueprints.md)), the Faction Editor's
   rename ([B2](ui-blueprints.md)), a Properties window's noted changes
   ([B8](ui-blueprints.md)). `blueprint::follow` already moves the
   Properties tabs with every command: the fix is to give it these
   windows' places too, and to keep the workspace's revision with
   every list of hits. Many of these windows stopped blocking the editor
   on 29 October (the log's entry on `egui::Modal`), which is when
   their kept indices began to matter.
5. **What outlives the module, the game data or the frame that made
   it.** `close()` and Reload Resources each reset some caches and
   windows and miss others: windows that hold a target
   ([B7](ui-blueprints.md)), pictures and palettes
   ([S10](ui-shell.md), [B17](ui-blueprints.md)), the renderer's
   textures ([B15](ui-blueprints.md)), model views
   ([B16](ui-blueprints.md)), part and icon numbers
   ([B13](ui-blueprints.md)), listings stamped with an address
   ([B14](ui-blueprints.md)), the tree's names ([S17](ui-shell.md)),
   the search's hits and the conversation clipboard
   ([D2, D11](ui-editors.md)), a script's laid-out lines
   ([X9](ui-plugins-nui-scripts.md)), the script index
   ([X1](ui-plugins-nui-scripts.md)), a plugin's grants
   ([X2](ui-plugins-nui-scripts.md)); in the renderer, the stencil
   target and framebuffer copies ([R5, R10](renderer.md)), a scene's
   every past look ([E8](edit-area-tiles-rules-nui.md)), the shader
   cache on disk ([M9](models-shaders-previews.md)).
6. **A job runs, and the window goes on.** Only the command keys are
   stopped: a module dropped on the window opens under a running job,
   whose result then lands on it ([S5](ui-shell.md),
   [D7](ui-editors.md)); closing the window leaves a project command's
   shell running ([D13](ui-editors.md)); a command whose script leaves a
   program behind never ends ([D3](ui-editors.md)); the project
   command's question does not block ([D12](ui-editors.md)).
7. **Bytes taken for text.** A name or a tag with a byte above 0x7F:
   slices that land inside a character ([B4](ui-blueprints.md),
   [P7, P8](module-project.md), [M10](models-shaders-previews.md)), a
   ResRef that does not survive a folder ([F7](formats-core.md)) or a
   plugin's edit ([C13](plugins-cli.md)), tags compared as UTF-8
   against the codepage's bytes ([D17](ui-editors.md)), Create Set's
   tags written as UTF-8 ([A10](ui-area.md)), a model's text read
   strictly ([M8](models-shaders-previews.md)), files rewritten lossily
   ([P10, P11](module-project.md)).
8. **The plugin host's limits.** What a plugin makes the host hold is
   outside its memory limit ([C2](plugins-cli.md)); a picture is
   decoded before its size is held ([C11](plugins-cli.md)); a plugin's
   folder is read before its limit ([C10](plugins-cli.md)); a choice
   field's options and a number field's range are not held
   ([X4, X5](ui-plugins-nui-scripts.md)); `mg plugin check` runs code
   with no time limit ([C15](plugins-cli.md)).
9. **Whole lists made or drawn every frame.** The last review's rule
   (rows out of sight are skipped; `choice_list`, not `choices`) is not
   followed in some twenty lists added or missed since:
   [A18](ui-area.md), [B22, B23, B26](ui-blueprints.md),
   [D20, D23, D24](ui-editors.md), [S19, S20](ui-shell.md),
   [X8, X24](ui-plugins-nui-scripts.md), [G4](rules-leftovers.md); and
   work that is the selection times the area
   ([A16](ui-area.md), [B20](ui-blueprints.md)).
10. **Every GIT parsed, on the window's thread.** Verify parses each
    area's GIT up to eleven times, one area after another
    ([V6](module-checks.md)); Edit Areas Together, Create Waypoint,
    Setup Area Transition and Rename each read them all
    ([V11, V17](module-checks.md), [A25](ui-area.md),
    [B21](ui-blueprints.md)); gigabytes written inside a frame
    ([D19](ui-editors.md)).
11. **Files written in place.** One helper (`write_replacing`) and a
    dozen writers that do not use it: [P17](module-project.md) lists
    eleven, and [V9](module-checks.md), [C20](plugins-cli.md),
    [S23](ui-shell.md), [A27](ui-area.md), [B30](ui-blueprints.md)
    the rest.
12. **Written more than once.** Exact copies are few (31 runs of 90
    tokens or more in the whole source: [Z5](mechanical.md)); what the
    readers found are rules written two or three ways that have
    drifted: Verify put together twice ([V20](module-checks.md)), a 2DA
    cell's rule ([V21](module-checks.md)), which group stands on a tile
    ([E10](edit-area-tiles-rules-nui.md)), a body part's model name
    ([M10](models-shaders-previews.md),
    [M7](models-shaders-previews.md), [G9](rules-leftovers.md)), a float
    as Nim writes it ([F18](formats-core.md)), which commands ask the
    plugins before a save ([C21](plugins-cli.md)), which tabs show a
    resource ([D30](ui-editors.md)), the `Frame` uniform and the hasher
    ([R29](renderer.md)).

## The high-severity findings

- **[F1](formats-core.md)** An update replaces whatever file `$APPIMAGE` names, also when Moonglow is not that AppImage. `crates/mg-update/src/kind.rs:92`; new since the last review. Read again: holds.
- **[F2](formats-core.md)** A tileset's highest-numbered section sizes the tile list: 60 bytes of `.set` ask for billions of tiles. `crates/mg-set/src/lib.rs:217-241`; there at the last review. Read again: holds.
- **[F3](formats-core.md)** `#define ENGINE_STRUCTURE_<n>` in a module's or hak's `nwscript.nss` sizes a vector by `n`. `crates/mg-script/src/spec.rs:125-131`; there at the last review. Read again: holds.
- **[F4](formats-core.md)** Rename of a struct member panics when the script changed while the Rename window was open. `crates/mg-script/src/analysis.rs:751`; new since the last review. Read again: holds.
- **[M1](models-shaders-previews.md)** A compiled model's controller keys are copied outside the copy budget: a 0.5 MB model allocates gigabytes. `crates/mg-mdl/src/binary.rs:147-194`; there at the last review. Read again: holds.
- **[M2](models-shaders-previews.md)** An ASCII key list pads every row to its widest row: a 400 KB text asks for 40 GB. `crates/mg-mdl/src/ascii.rs:389-416`; there at the last review. Read again: holds.
- **[R1](renderer.md)** The batcher's count of a box's squares overflows, and a huge solid mesh then loops over 2^64 squares, allocating each. `crates/mg-render/src/batch.rs:243`; there at the last review. Read again: holds.
- **[R2](renderer.md)** Vertex animation on a mesh with a mirrored texture seam indexes past the mesh's vertices and panics. `crates/mg-render/src/anim.rs:557-567, :588-590`; new since the last review. Read again: holds.
- **[E1](edit-area-tiles-rules-nui.md)** The module reads as saved after an Undo past the save and a new edit: unsaved work is lost without a question. `crates/mg-edit/src/lib.rs:336`; there at the last review. Read again: holds.
- **[V1](module-checks.md)** An area's minimap panics on Width 0 and allocates by whatever Width and Height the ARE claims. `minimap.rs:61-66`; there at the last review. Read again: holds.
- **[V2](module-checks.md)** Export as Text walks owning links with no guard: a conversation whose lines own each other never ends, one with a line owned twice grows as two to the depth. `dialog_io.rs:656-689`; there at the last review. Read again: holds.
- **[P1](module-project.md)** Unsaved `module.ifo` edits are counted as read when a pull adds an area, and are never written. `crates/mg-module/src/lib.rs:891-902`; new since the last review. Read again: holds.
- **[P2](module-project.md)** The external compiler's scratch files are named by raw resrefs: a script named `../../x` is written outside the scratch folder. `crates/mg-module/src/external_compiler.rs:148-152`; there at the last review. Read again: holds.
- **[C1](plugins-cli.md)** `mg test --dir` and `mg watch --serve --dir` write into the player's own NWN folder before the check that refuses it. `apps/mg/src/test.rs:245-248`; partly new since the last review. Read again: holds.
- **[C2](plugins-cli.md)** What a plugin makes the host hold is outside its 256 MB: one call takes a gigabyte, a loop takes all the memory. `crates/mg-plugin/src/tables.rs:348-349`; partly new since the last review. Read again: holds.
- **[S1](ui-shell.md)** The module's Undo and Redo throw away every script editor's unsaved text. `lib.rs:3850`; there at the last review. Read again: holds.
- **[S2](ui-shell.md)** Recover, New Module's Create and "Open Directory as a Module" replace the open module without asking, and delete its recovery copy. `recovery.rs:464-467 and 383-417`; there at the last review. Read again: holds.
- **[S3](ui-shell.md)** The Variables and String Edit windows keep their object by its place in a list, and write to whatever is there at OK. `widgets.rs:1054-1255`; partly new since the last review. Read again: holds.
- **[S4](ui-shell.md)** A recovery copy is made only when the workspace changes: text typed in script editors alone is never kept. `recovery.rs:330-352`; there at the last review. Read again: holds.
- **[S5](ui-shell.md)** Only the command keys are stopped while a job runs: the menus' own keys and files dropped on the window still run, and a job's result is then applied to another module. `lib.rs:1577-1579, 3574-3630`; partly new since the last review. Read again: holds.
- **[A1](ui-area.md)** A group brush kept across Reload Resources indexes the tileset's groups unchecked: panic. `terrain_mode.rs:607, 613`; there at the last review. Read again: holds.
- **[A2](ui-area.md)** Export Minimaps… joins the folder with an area's ResRef unchecked: writes outside the folder chosen. `minimap_export.rs:73-76`; new since the last review. Read again: holds.
- **[A3](ui-area.md)** Render Minimap Pictures writes `<ImageMap2D>.tga` wherever the `.set` says. `tileset_view.rs:1125-1132`; there at the last review. Read again: holds.
- **[A4](ui-area.md)** The view's minimap and Export Minimap divide by an ARE's `Width`. `crates/mg-module/src/minimap.rs:100`; new since the last review. Read again: holds.
- **[B1](ui-blueprints.md)** The Faction Editor indexes `repute.fac` unchecked, and replaces an unreadable one with the standard five. `faction_view.rs:119-125`; there at the last review. Read again: holds.
- **[B2](ui-blueprints.md)** Change Faction Name keeps a faction's index behind a window that doesn't block: OK panics or renames another faction. `faction_view.rs:321-323`; there at the last review. Read again: holds.
- **[B3](ui-blueprints.md)** The Levelup Wizard gives the level field a range that runs backwards for a class above level 60, and an arrow click panics. `levelup_view.rs:55`; partly new since the last review. Read again: holds.
- **[B4](ui-blueprints.md)** Edit Copy slices a ResRef's text at byte 13, which panics on a non-ASCII name. `palette_view.rs:515`; partly new since the last review. Read again: holds.
- **[B5](ui-blueprints.md)** A hak's loading-screen mesh whose U coordinates are all NaN panics the picture's stitching. `images.rs:144-150`; new since the last review. Read again: holds.
- **[B6](ui-blueprints.md)** Windows that hold a placed object by its list index are not followed, and their OK edits another object. `blueprint/mod.rs:1665`; there at the last review. Read again: holds.
- **[D1](ui-editors.md)** The hak editor's right-click Remove indexes the hak's items after they shrank: a panic. `crates/mg-ui/src/hak_view.rs:646-648`; there at the last review. Read again: holds.
- **[D2](ui-editors.md)** Advanced Search changes what its hits pointed at when they were found, not what is there now; Duplicate Tags the same. `crates/mg-ui/src/search.rs:641-677, :1203, :1341-1352`; partly new since the last review. Read again: holds.
- **[D3](ui-editors.md)** A project command whose script leaves a program running never ends, and Cancel does nothing. `crates/mg-ui/src/project_commands.rs:603-607`; there at the last review. Read again: holds.
- **[D4](ui-editors.md)** Export as Ink/Twine/Text/CSV joins the folder with a conversation's ResRef unchecked. `crates/mg-ui/src/dialog_export.rs:41-43, :116-119, :147-149`; new since the last review. Read again: holds.
- **[X1](ui-plugins-nui-scripts.md)** Rename Symbol, References and Go to Definition work on script text that is not the module's: Rename replaces the wrong bytes, References can panic. `script_nav.rs:82-88, 140, 177-191`; partly new since the last review. Read again: holds.
- **[N1](ui-nui-drawing.md)** A control's `aspect` makes it millions of points high, and its scrollbar's cursor is tiled along all of it: the Creator hangs and takes gigabytes. `nui_view/skin.rs:882-894`; there at the last review. Read again: holds.
- **[N2](ui-nui-drawing.md)** The canvas's own walk for picture names recurses without end on a bind whose value names a bind: opening the window's tab overflows the stack. `nui_view/skin.rs:504-510`; there at the last review. Read again: holds.

## What was not read

- Tests (`tests/`, `#[cfg(test)]` modules: 137,000 lines), the generated
  schema and `vendor/winit`: not under review.
- The readers' own lists are at the foot of each part page. After the
  two extra readers, what is left unread is small:
  the two built-in table descriptions (`crates/mg-module/src/table_desc/*.toml`)
  and the first lines (comments and imports) of three blueprint pages.
  Where a reader split its part among sub-readers (the renderer, the
  blueprint editors, the other editors), the lead read again only the
  lines its entries say it did.
- No reader ran anything. The performance findings' costs are worked out
  from the code; the bugs' scenarios are traced, not reproduced, but for
  the two that say they were run.
- The plugin API's reference (`docs/manual/17-plugin-api.md`) was held
  to the code only where a finding needed it, not function by function.

## An order to fix in

1. The data-loss findings of pattern 1, each with its test: E1, S1, S4,
   S2, P1, then C8 and S14 together.
2. One helper for a file named by a name from a file (pattern 2), and
   the two commands that write before they check (C1, F1).
3. The bounds of pattern 3, format by format, each with a hostile-file
   test beside the truncation tests the last review added.
4. `blueprint::follow` given every window's place, and the revision kept
   with every list of hits (pattern 4): one change, a dozen findings.
5. One sweep in `close()` and one in `load_order_changed` (pattern 5).
6. One guard for actions while a job runs (pattern 6).
7. The remaining high findings (the panics: R2, F4, A1, B1 to B3, B5,
   D1, X1), then the mediums part by part.
8. The performance findings: a `frame_perf`, `editor_perf` or
   `world_perf` view for each first, then the change, then the figures.
9. The duplications and the hygiene, last.

[^last-review]: The last whole-workspace review; 996 commits lie between
[^code]: The code as read
[^clippy]: Clippy's stricter lints as a finder
[^timing]: The timing tests' runs
[^bisect]: The bisects for the slower encoding of a dense scene's draws
[^oracle]: The two findings that were run against the oracle tools
