---
type: Report
title: The fixes of the review of November 2026
description: 'What became of the 349 findings of the code review of 2026-11-09 in the fix pass of the same day, for 4.0.0: a table a part (fixed whole, fixed in part, found fixed by another finding''s commit, does not hold, left), then every finding not fixed whole with what is left and why, the findings fixed with a choice for the owner or a check that could not be made here, and the owner''s word on S15. Hand-written from the fixers'' reports and the commits; the part pages beside it are the record of what was found.'
tags: [review, code-review, fixes, deferred]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-10T04:33:40Z }
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
    resource: 'target/review/reports/*.md of the main checkout (the fourteen fixers'' final reports) and target/review/fix-notes.md (the reviewer''s running notes), not kept in the repository'
    title: The fixers' reports and the reviewer's notes
  - id: owner
    resource: human:august
    title: 'The owner (2026-11-09): fix the findings; the release is 4.0.0; S15 settled'
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
the fixers' reports and the commits say became of them.[^reports]

The counts: **fixed whole**, the finding's fix made and tested (by its
part's fixer or the reviewer); **in part**, a part its fixer names as
left or not done; **by another**, found already fixed by another
finding's commit; **does not hold**; **left**, nothing changed.

## By part

| Part (page) | Findings | Fixed whole | In part | By another | Does not hold | Left |
| --- | --- | --- | --- | --- | --- | --- |
| [Formats and core](formats-core.md) (F) | 25 | 25 | 0 | 0 | 0 | 0 |
| [Models, shaders and previews](models-shaders-previews.md) (M) | 17 | 17 | 0 | 0 | 0 | 0 |
| [The renderer](renderer.md) (R) | 30 | 26 | 2 | 0 | 0 | 2 |
| [The workspace, areas, tiles, rules and NUI documents](edit-area-tiles-rules-nui.md) (E) | 24 | 23 | 0 | 1 | 0 | 0 |
| [The rules data and what the first pass left](rules-leftovers.md) (G) | 11 | 9 | 1 | 0 | 0 | 1 |
| [Verify, search, rename and the module's queries](module-checks.md) (V) | 25 | 23 | 2 | 0 | 0 | 0 |
| [Opening, saving and the project's files](module-project.md) (P) | 23 | 22 | 1 | 0 | 0 | 0 |
| [The plugin host and the command line](plugins-cli.md) (C) | 28 | 28 | 0 | 0 | 0 | 0 |
| [The window](ui-shell.md) (S) | 28 | 27 | 0 | 0 | 1 | 0 |
| [The area editor](ui-area.md) (A) | 29 | 28 | 0 | 1 | 0 | 0 |
| [The blueprint editors, palettes and pictures](ui-blueprints.md) (B) | 30 | 28 | 0 | 2 | 0 | 0 |
| [The 2DA, conversation, talk table and hak editors and the module-wide tools](ui-editors.md) (D) | 30 | 28 | 2 | 0 | 0 | 0 |
| [Plugins in the window, the NUI Creator's editor side and the script editor](ui-plugins-nui-scripts.md) (X) | 25 | 25 | 0 | 0 | 0 | 0 |
| [The NUI Creator's drawing side, the Script Wizard's window, the manual and the icons](ui-nui-drawing.md) (N) | 18 | 14 | 3 | 1 | 0 | 0 |
| [The finders and the timing runs](mechanical.md) (Z) | 6 | 1 | 0 | 0 | 0 | 5 |
| **All** | **349** | **324** | **11** | **5** | **1** | **8** |

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
| R4 | Light Shadows ticked with Creature Shadows unticked draws no light shadows | the owner's decision: should Light Shadows cast with Creature Shadows off, or be grayed out while it is off |
| R5 | (d): the shadow volume buffers kept in each `GpuMesh` for the model's life; (a) to (c) fixed | waits on the reviewer's measurement: freeing them is a restructure of the casters' path that Z2's measurement would have to cover |
| R18 | chunk particles still read `particlerot` as turns (the module doc says so) | waits on a client run: a chunk emitter beside `reading_particle_spin_and_order`'s quads |
| R29 | the tangents of a mesh whose UVs are one point; the rest fixed | waits on a client run: `reading_tangents` with such a card |
| G4 | the "only when the header is open" half: while the search has text, the Properties page matches every subtype row each frame | not done by its fixer; filed with the views for the reviewer's timing run |
| G8 | a class new to a creature gets no spells of a level of bonus slots alone | waits on an Aurora capture (Paladin 4 at Wisdom 14; with it a sorcerer's and a bard's first level) |
| V5 | whether the CSV export should begin with a byte order mark (Import Lines reads one now) | the owner's decision |
| V14 | a hak rebuilt with the same time, size and layout, only its data changed, is taken for the old one | the owner's decision: hashing a hak's data means reading it each run |
| P4 | (c): no bound on how far past its end an import may grow a table | the owner's decision: refuse or ask past some number of empty lines |
| D19 | the module's Save and the NUI picture hak still write a hak at once; Add Haks has no byte progress or Cancel mid-copy | a real unintended consequence: what follows those saves in the same step reads the files written |
| D26 | the half that remembers the editor's own edits as `renumbered: None` | not done: its fixer judged it would save little after the fix; not filed |
| N9 | what a click on a scroll bar's end button scrolls by still differs by kind | waits on a client run |
| N13 | `check`'s own copy a frame, Interact's two whole comparisons of its session, a variant's context document cached | waits on the reviewer's timing run (suspected, not measured) |
| N16 | Options' and Tabs' entries, named only in the entry's confidence line | bounded by the file's size; filed with N13 |
| Z2 | a dense scene's draws encoded a fifth to a third slower than on 27 October | waits on the reviewer's profile of the encoding at both builds |
| Z3 | a folder's linked files are not in the load order | not assigned to a fixer in this pass; an engine test here would settle whether the game reads them |
| Z4 | `mg gff` writes JSON as serde lays it out, not as `to_json_text` does | the owner's decision (the page's own word); not taken up |
| Z5 | the code written more than once that the clone finder found | not assigned to a fixer in this pass |
| Z6 | what clippy's stricter lints leave | not assigned to a fixer in this pass |

Each of these but D26, Z3, Z5 and Z6 is on [the deferred
list](../deferred.md) with its reason (G4's and N16's in the entries of
the timing run and of N13; Z2's in the entry of a dense area seen
whole). D26's half was judged not worth doing; Z3, Z5 and Z6 were given
to no fixer and wait on the reviewer's word.

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
| A14 | in Paint, Select Tiles and with a tile brush, Alt + drag always moves the view | the owner's decision |
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

S15: settled by the owner on 9 November 2026: the deferred list's account of the behavior is correct; nothing changed.

[^overview]: Code review of November 2026: what was read, measured and found
[^merges]: The fourteen fixers' branches, merged into develop on 2026-11-09
[^reviewer]: The reviewer's own fixes, before the fixers began
[^reports]: The fixers' reports and the reviewer's notes
[^owner]: The owner (2026-11-09): fix the findings; the release is 4.0.0; S15 settled
