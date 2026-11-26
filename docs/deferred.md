---
type: Backlog
title: 'Deferred: what was left out, for a decision later'
description: What finished work left undone or unchecked, filed by area of the program (an entry's last words say which report, issue or review it came from), each awaiting a verdict - fix, add or drop; what is settled by decision or design is gathered at the end.
tags: [backlog, deferred, after-parity]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-26T12:52:22Z }
---

# Deferred: what was left out, for a decision later

What finished work left undone or unchecked, in one place: each entry
awaits a verdict (fix, add, or drop). Work is deferred only when doing
it would have real unintended consequences or would go beyond what was
asked, and the entry says which; what can't be done or checked on the
development machine is filed as waiting on it (`CLAUDE.md`, Rules). The list is filed by area of the program, one entry a gap, in
the section of its area; an entry that came from a builder's report, a
GitHub issue or a review says so in its last words. What was fixed is in
the [log](log.md) and the release notes, what was measured in the game
or Aurora is in the [findings](findings.md), a question asked and not
answered is in the section before the last, and what stays as it is by
decision or by design is in the last section.

Size: S (an hour or two), M (a day or so), L (several days).

## Tests and the build (the [review](test-review.md) of October 2026)

- **Moonglow Viewer's shader cache across toolset tags:** the libraries
  are 0.0.0 and read the release from the program (`mg_core::release`),
  which the viewer doesn't set, so its compiled-shader cache is named by
  the passes' `FORMAT` alone and no longer starts anew when it moves to a
  newer toolset tag. One line at the viewer's start
  (`mg_core::release::set`, its own version) names it as before. Not
  yet a fault: the viewer is on v2.8.0, whose `mg-shader` names the
  cache by its own version (`CARGO_PKG_VERSION`, 2.8.0), and
  `mg_core::release` is in v2.9.0 and after, so the line can only go in
  with the viewer's move to a newer tag (in its repository, not this
  one). (S, waits on the viewer's next toolset tag)
- **Tests of the review of November 2026 not seen failing before their
  fix:** each of its fixes came with a test, but these were not run
  against the code before the fix, and rest on the review's trace or
  the fixer's reading: the command line's and plugin host's C1 to C28
  but C7 (C4's timed before the fix); the formats' timed tests (F9, F13
  to F15) and hostile-file tests (F2, F8), whose old code would take
  gigabytes or minutes; opening and saving's P2 to P23 but P7, P16, P20
  and P21. D28's test passed before its change too, as did the test of
  a rename's list made through a set (seen on the way): both findings
  were suspected. Each test run at its fix's parent commit would show
  which fail there. The X and B parts were run on 10 November: each
  test of X3 to X25 and of the blueprint editors' fixer (but B15's and
  B19's) that can run at its fix's parent failed there for its
  finding's reason; those of X11, X18, X20, X22 and X23 cannot run
  there, as they call functions their fixes added (`found`,
  `spans_lines` and `lines_text`, the `reads` counter, `binds_uses`
  and the new `element`, `holds`), and X25's commit has no test of its
  own. (S, waits on the reviewer's run against the code before the
  fixes) (the review of November 2026, [its
  fixes](review-2026-11/fixes.md))
- **Editor setups:** the manual's Neovim and Helix snippets are untried
  in those editors. On Windows, `mg lsp` is checked only by CI's
  tests. (S) (waits on Neovim and Helix: neither is on the
  development machine, and putting them there is a download)
- **The external compiler with `nwnsc`** (Options › Script Editor):
  its line of arguments (`-q -y -o -n {game} -i {src} -b {out}
  {files}`) is from its documentation, not run, and its messages are
  matched to scripts by their file names and the word "error":
  nwnsc's own wording, not seen, may be missed (the script is then
  said to have made no compiled script). `nwn_script_comp` is tried
  (its bytecode is the built-in compiler's, byte for byte). (S) (waits
  on a copy of nwnsc: none is on the development machine) (from
  builders' reports after 1.14.1)

## nasher projects

- **`mg merge-gff`, what it leaves out** (the [manual](manual/03-modules.md#merging-in-git)
  has the rules):
  - **Conversations** merge their entry and reply lists by place, and the
    links name nodes by number: two builders adding nodes to one
    conversation conflict, where renumbering one side's new nodes (and
    the links to them: `RepliesList`, `EntriesList` and `StartingList`
    `Index`) would keep both; a node deleted on one side renumbers every
    node after it, so matching by place pairs the wrong nodes too, and
    nodes would first need matching by what they are. (M)
  - **On a conflict nothing is written** and git leaves ours in the file;
    there is no file of the conflicts to edit, and no way to take one
    side for one conflict and the other for the next but `-o` from
    git's stages. A conflict file (the edits each side wants, for `mg
    apply`) or a Moonglow window would. (M)
  (The Domains of Dread's builder proposal, C9)

## Test loop

- **`mg serve` on Windows:** not tried there: the server's console on
  pipes, the links for haks, and Ctrl+C (the server in a console
  process group of its own, `CREATE_NEW_PROCESS_GROUP`, told to `exit`
  by `mg`). (S) (waits on a Windows machine)
- **Test Module and Steam:** a test of a Steam copy follows Read Steam
  Workshop content (5.1.0, the owner's ruling of 2026-11-22, after a
  builder's test looked other than the views): on, the game is told
  which Steam game it is (`SteamAppId`), so that it reaches a running
  Steam and loads the Workshop's content, as when started from Steam;
  off, it is started bare, as Aurora starts it, without the Workshop
  items (`SteamAppId` taken away). Not tried, as the client is only
  ever run here with Steam hidden: whether a bare start on Windows or
  macOS still reaches a running Steam (the install's `steam_appid.txt`
  is in its root, the client started in `bin/<platform>`), and that
  the start through Steam loads every item the view reads. (needs a
  builder's word) (from builders' reports after 1.14.1 and 5.0.0)
- **`mg serve`'s run folders under a shared, guessable parent:** `mg
  serve` and `mg test` make a module's user directory under
  `/tmp/moonglow-serve/<stem>-<hash>` (`apps/mg/src/serve.rs`,
  `run_dir`), the class of what the review's last pass moved out of the
  system's temporary folder (its item 24). Not changed: outside that
  item's list, and the names are stable on purpose so that a module's
  run reuses its folder; whether to move them is the owner's call. (S)
  (what the fixers of the review of November 2026 saw on the way)

## Persistent-world scale

- **Other machines:** budgets were measured on the development machine
  alone: a Ryzen 9 9950X with a Radeon RX 9070 XT and, since the
  [performance audit](performance-audit.md) of 2026-11-01, its CPU's
  integrated Radeon (2 compute units, weaker than most laptops') as the
  stand-in for weaker hardware. Not measured: an Intel integrated GPU,
  Direct3D 12's heaps on Windows, macOS. Not seen either: the game
  client's black of a custom shader's normal map without tangents on
  Windows and macOS (the stock include normalizing a zero tangent; read
  on Linux, `reading_tangents`; Moonglow draws the same black on CI's
  Direct3D 12 and Metal). (S, waits on a builder's machine)

## Frames (a steady 144 a second)

The budget and the measurements are in
[the frames note](research/notes_frames.md). Done: an area's picture kept
between its animations' steps, its drawing at a fifth of the cost (lights
by the ground they reach, materials worked out once, a draw's values by
its number, what is out of sight left out, tiles of one model posed
once, meshes of one model drawn as instances of one draw), the
galleries' pictures made for 2 ms a frame and kept while in sight.

- **The largest areas in full view:** Tyrants of the Moonsea's area084
  (676 tiles, 16,000 meshes) with the wheel zooming over all of it: 7.5
  to 9.6 ms a frame since 2 November (9.1 to 10.8 before;
  `frame_perf`'s `sound_ranges_frames_at_144`, named in its
  `KNOWN_OVER`; since 21 November, when M5's V3 opened the area with
  its camera over its ground, about 34 m up, also the wheel of the nine
  views after "close", which zooms out over all of it from where that
  view left the camera: 8.6 to 10.3 ms, and its "all selected,
  walkmesh" view, the whole area drawn from 120 m while turning: 11.4
  to 13.0 ms, of which the overlays are about 2, both named there too),
  its draws
  encoded in 6.1 ms (7.4); TDoD's synthetic
  woods (33,000 meshes in sight, 25,000 draws of meshes mostly drawn
  once each) 14 to 31 ms, as before (its overview with 4x MSAA and the
  Shaders on lands in one of two GPU levels from run to run, the code
  unchanged: the median pinned near 11.2 ms and the quickest tenth 6.8
  to 8.0, or a median of 6 to 8.5 and a tenth of 5.3 to 6.9; the slow
  level goes with the card's automatic clocks settling low, not with
  memory. Compare two builds in one run on alternate frames, or by many
  runs sorted by level; [the frames note](research/notes_frames.md), 26
  November). Done that day: the lights' sets
  numbered as a mesh's lights are chosen and a mesh's choice found by
  its place among the scene's meshes, the sun's shadows planned and the
  game's solid draws sorted on threads beside the other work (the
  pictures the same to the bit). What is left is a draw's and a
  material's own cost, the most of it wgpu's: of area084's encoding,
  wgpu's `finish` a quarter (most of it the see-through draws' 1,270
  changes of material and 2,600 of the meshes' chunk among 9,500 draws, a
  group's textures tracked and their initialized state checked each
  time), the game's draws' values (23,000 blocks, 2.7 MB a frame, each
  written and then copied) a fifth, the batcher a tenth. Tried and
  given up: one indirect draw for draws in a row of one state (wgpu's
  validation of each sub-draw costs what it saves: the woods were a
  millisecond slower), each pass's batcher on a thread of its own (no
  gain), the meshes' chunk in the batcher's state (a third fewer draws,
  no quicker, the GPU slower). Untried: render bundles recorded on
  threads (at most half of `finish` off the window's thread: a bundle's
  initialized-state checks and the driver's calls stay on it), and the
  draws' values written on threads straight into the GPU's staging
  memory. ([the frames note](research/notes_frames.md), 27 and 30
  October, 2 November) (L) (After 3.0.0, the owner's word, 2026-11-01:
  10th by complexity.)
- **The performance audit's leftovers** ([the audit](performance-audit.md)
  of 2026-11-01; beyond what it was asked, filed by the owner's word the
  same day):
  - **The animation step could follow what a picture costs:** a slow GPU
    stepping less often while the window is in front, measured with the
    GPU's timestamps (not on every backend). (M)
- **The grass field's window frame close up, slower than in 3.18.1:**
  the grass test's field close up without grass, the window's frames
  (pointer, idle, wheel), took 1.17 to 1.25 ms where 3.18.1 took 0.98
  to 1.02 (load about 10), and 2.63 to 4.14 where 1.62 to 2.50 at load
  16 to 21. A bisect on a quieter machine (load 2.5 to 4, three runs a
  step, the idle line) found three steps: b4da2984 (the box around
  points in one place, the review's R20), +0.2 to 0.3 ms (0.84 to 1.14);
  a small one between 992ed8d2 and 27a4050c, +0.06; and 89e62b7f (the UI
  redesign's M3 shell), +0.11, which is Interface's. The first is given
  back: the sun's shadow plan, on a thread of its own beside the meshes'
  draws, put `box_of` around the receivers' box and each receiving mesh's
  (about 10,000 a frame in the field), so it took 0.31 ms against the
  draws' 0.18 and the frame waited about 0.18 ms for it; the receivers'
  box is now taken as it goes (their boxes are already known to be
  somewhere), and `box_of` is a plain loop. The plan takes 0.09 ms and
  the frame no longer waits (temporary timers, load 15 to 22); the idle
  line, develop and the fix alternated four rounds at load 15 to 16:
  1.37 to 1.65 ms against 1.16 to 1.33. Confirmed on a quiet machine
  (26 November, load 2.8 to 4.8, six rounds of the four alternated):
  the idle line 0.84 ms before b4da2984, 1.21 on develop, 1.04 to 1.06
  with the fix (spread 0.02 to 0.03). The fix gives back 0.17 of the
  0.37 between 3.18.1's base and develop. Left: the small step between
  992ed8d2 and 27a4050c, and the shell's. The review saw it as waiting
  rather than work. The rest of the old entry is settled: area092
  is quicker than in 3.18.1, the field's GPU step is dd8d0bc1 (grass
  taking the shadows, kept) and the woods' is 6b26e702 (kept), as [the
  frames note](research/notes_frames.md) of 25 November says. The
  last step, 89e62b7f (M3's shell) +0.11 ms: profiled against 4.2.1
  (abe3e024), it is the shell's own chrome drawn each frame, three fifths
  widgets (the Outliner beside the module tree, the Status Bar, the Top
  Bar's Quick Open, the tree's guide lines and eye icons) and two fifths
  their shapes' tessellation, nothing drawn twice or for nothing: left,
  since under the line would mean taking parts of the shell away
  (Interface's, 26 November). (S, waits on a calm machine: load under 5
  to time the small step) (the review of
  November 2026, [its fixes](review-2026-11/fixes.md))
- **The review of November 2026's performance fixes without a timing
  view:** its fixers ran no timing test and named the views that would
  show each change. The reviewer's run of 10 November timed every view
  the timing tests have against the review's figures (nothing slower
  but where the builder's rebuilt haks changed what is drawn, and
  world_perf's content doctor, fixed: V6, V10, V12), and gave views to
  the five largest costs that grew with the module (D20, D23, B22, A18,
  N16; [the log](log.md), 10 November). The renderer's six (R9, R12,
  R13, R15, R16, R22) have theirs since 26 November
  (`frame_perf::render_views::renderer_fixes_at_scale`, [the
  log](log.md)); what they leave, each an area's draw alone: 2,000 of
  The Domains of Dread's placeables tinted in 50 colorways encode 0.6
  to 0.8 ms more than untinted, as much before R22's fix as after (its
  lookup was not this cost; their 288 more draws, each colorway of a
  model batched apart, are about a third by the average time a draw
  takes, the rest not found): under a millisecond, and the area's draw
  of 2,000 placeables (5.8 to 7.6 ms encoded) is "The largest areas in
  full view"'s cost, so left; and a second view drawn through a large
  area's renderer between its frames still costs the area's encoding
  0.20 to 0.43 ms more than the same view drawn through a renderer of
  its own (develop, three runs at a load of 10 to 17; the shadow map is
  drawn again in neither): something of a view's still redone that R9's
  fix did not keep, not found within the 5.4.0 release's time for these
  views (finding it is a profile of the renderer's own caches), under a
  millisecond, so left.
  The command line's and Verify's (V15 to V17, C3, C4, M17, P19) were timed on 26 November
  (`mg-module`'s `tests/timing.rs`, `mg-plugin`'s `tests/timing.rs`, `apps/mg`'s
  `tests/timing.rs`; three alternating rounds at load 10 to 30, the lowest, the
  median, the highest of nine runs, [the log](log.md)). All held; what is
  left, each with its reason: a second Find with the haks and the game
  over 45,000 blueprints (V15) 1,952 / 2,169 / 3,731 ms before, 54 / 77 / 112
  after, 29 / 36 / 41 with the sort's rank looked up once; the rest
  (the layers' key lists walked and sorted again at each Find, about 25 ms)
  is a cache of the wanted list by the game's generation and the module's
  revision, a design of its own (beyond a local change). Delete's question
  of 20 areas in 5,000 (V16) 29.8 / 39.0 / 58.7 s before, 0.65 / 0.70 / 1.05
  s after, 0.35 s the first time and 0.14 to 0.17 s after with the tag
  counts kept per index; the rest is the index's walk for the transitions
  (7 ms an area, over the line; beyond a local change: a map of the
  transitions by their destination tag, made with the index). Rename of a
  script in 5,000 areas (V17) 21.4 / 23.1 / 40.1 ms before, 5.6 / 6.6 /
  11.1 after (the module's copy and the 255 references' rewrite, at the
  line). A panel over 500 selected objects (C3) 389 / 411 / 564 ms before
  (it ran out of time), 3.5 / 4.3 / 6.8 after; 1,024 tiles 13.4 to 2.7 ms.
  A check of 2,000 fixes over 10,000 resources (C4) 1,752 / 2,262 / 3,114
  ms before, 61 / 90 / 92 after (a job's thread, not a frame).
  `mg haks --usage` of 20,000 creatures and 10,000 scripts (M17) 3.1 / 3.3 /
  4.6 s before, 1.4 / 1.5 / 2.8 after, against 0.94 / 1.0 / 1.4 and 0.82 /
  0.85 / 1.3 for a small module (the process's own cost): +2.2 s to +0.6
  s (a command, no frame). Opening a project of 30,000 sources (P19) 272 /
  289 / 522 ms before, 230 / 239 / 255 after, its walk for the files left
  out 117 / 118 / 214, then 69 / 71 / 78 (what is left is the folder's own
  walk). Quiet-machine confirmation of all: waits on a calm machine
  (load under 5).
  The area editor's have
  theirs since 25 November (`frame_perf::area_editor_at_scale`, [the
  log](log.md)); what they leave, timed in a quiet slot (26 November
  07:10, the load 3 to 4, three runs): with 5,000 placeables selected,
  +0.54 to +0.80 ms a frame at rest (from +2.07; the Properties panel
  keeps its objects' places and counts their items anew only when they
  change), most of it their turning rings, 96 pieces each (now kept on
  screen while the view and the rings stand, not yet timed quietly),
  and 0.4 to 0.7 ms more at rest after a drag and its undo than before
  it (measured before these changes, cause not found, not timed again);
  in a drag +3.3 to +3.5 ms (from +8.3), most of it their boxes' 480,000
  vertices made anew each frame as the drag poses them (Engine and
  Rendering's task: the renderer is to draw selection boxes instanced,
  the hand-off in the area view's overlay code agreed with Interface
  first); every tile of a hilly area selected, its outlines
  put on screen each frame the camera turns (0.8 to 1.1 ms); and the
  Position menu's walkmesh items, untimed, since the scratch module has
  no `[bake]` (a project with one would time them).
  The blueprint editors' have theirs since 26 November
  (`frame_perf::blueprint_editors_at_scale`, [the log](log.md)); what
  they leave, timed in the same slot: Select Texture's first opening
  11.7 to 12.6 ms (from 27; 135,094 textures, their names sorted as
  numbers) and a letter typed in its Find 6.0 to 6.3 (within the
  budget: it looks among what the last filter found); Select Portrait's
  opening 4.8 ms with the install's portraits.2da (from 9.9) and 8.1 to
  8.6 with 32,000 rows (from 18.8; the portraits' names gathered in one
  walk); an item's Appearance page of 999 icons opened in 8.75 to 8.87
  ms (its icons' numbers 1.8, the grid 1.1 to 1.3, the rest the
  window's first frame); the
  Custom palette with 8,000 items found against 100, List +105 (to 108),
  Gallery +150 (to 168) (likely a header built each frame for every
  category holding a find, in sight or not: laying the palette's tree
  out by rows is a change of its own); the Instances window of 100
  creatures on its Feats page +90 (to 130) µs, its timers' remaining
  costs each under the line (Classes 20 µs, the inventory's rows 40);
  and the Appearance Gallery's drag to the end, its median 3.5 (to 4.6)
  ms at 30,000 rows but worst frames of 10.4 to 14.0, and its pictures
  in sight made in frames of up to 4.9 to 7.9 ms (a creature's picture
  drawn on the window's thread at about 5 ms, past the 2 ms a frame
  gives pictures, one drawn whatever it costs: drawing them apart is a
  renderer change).
  The editors' have theirs since 26 November
  (`frame_perf::editors_at_scale`, [the log](log.md)); what they leave,
  timed again at the final commit, three runs each, each view against
  its rest (the line 70 µs). Truly quiet (26 November 03:22 to 03:24,
  the load 3 to 5, the encoding job paused): `spells.2da` at its full
  width +58 to +82 µs over the same rows cut to the 26 columns in sight
  (at the line; the cells out of sight sideways are one place a run
  since 26 November; a run at 00:00, the processors idle but the load
  still 12 to 18, had read +194 to +330); Choose Model open over 82,856
  models +34 to +41 (within). At 00:00 (load 12 to 18): the raw fields
  view with an area's Placeable List of 3,000 opened, +79 to +80 µs
  over 20: each item out of sight is still asked whether it is open (an
  id made and egui's memory looked up, 3,000 a frame), and passing them
  over in blocks needs to know which are open without asking, which
  egui's memory can't say: left, beyond a local change; within then: a
  hak's Save with the large module open (the click 0.8 to 0.9 ms, its
  frames until saved 3.8 to 5.5 ms), the typed line (2.5 to 3.6 ms on
  the small module, 4.1 to 5.2 on the large), the Claims Table (+26 to
  +31 µs), the conversation's Search of 20,000 (+37 to +43 µs, Find's
  click 3.8 to 4.3 ms), Build Module's 20,000 (+11 to +22), Custom
  Content (−100 to +23), `nwscript.nss` (−2 to +5), the Variables
  window (+2 to +61), the module tree (−32 to −28).
  The NUI Creator's and the script editor's have theirs since 26
  November (`nui_perf::the_creator_at_scale`, `NUI_VIEW` picks its
  groups, and `editor_perf::scripts_at_scale`, `EDITOR_VIEW`; [the
  log](log.md)), timed truly quiet at their final commit (26 November
  03:22 to 03:24, the load 3 to 5; three runs), and the four that were
  over the line timed again at batch 5's (05:15, the load 2; three
  runs): the column of 1,000 rows +35 to +40 µs (+826 to +867 before),
  the combo of 10,000 entries in Interact +3 to +11 (+160 to +197), a
  key a frame in Choose Image's Find +10 to +15 (+1.34 to +1.61 ms):
  the canvas keeps its layout, the pictures and strings the window
  names and Interact's check of its session from one frame to the next
  while the window is known unchanged (a quiet frame `end` found it
  unchanged in), Find looks among what the filter before it found, and
  a picture lingers 30 passes after it is last asked for. Left, for the
  reason given: Design of 511 controls eight deep +157 to +164 µs
  (+356 to +372 before): what is left is the 239 more containers drawn
  than the shallow window, a third of it their properties looked up,
  a sixth their hit areas and the rest their drawing; with no lookups
  at all it would still be about +105, and under the line would mean
  containers in sight without a hit area, which could not be clicked,
  dragged or dropped on. Within: two
  Creator tabs side by side (+13 to +23), Split source as a picture's
  name is typed (−0 to +3), the Bindings page of 60 binds (+10 to +11),
  the list's cell of 5,000 rows (+17 to +23) and New Bind open on it
  (+12 to +18), 2,000 findings (−127 to −109), Choose Image with a name
  typed (−35 to −25), Search Results of 10,000 lines (+4 to +10; the
  loaded runs' "over in every run" was the load), Open Script with All
  Resources (+1 to +9), the completion list at a huge script's end (+12
  to +18), a plugin panel's choice of 2,000 options (+5 to +6).
  (M) (the review of
  November 2026, [its fixes](review-2026-11/fixes.md))
## Blueprint editors and inventories

- **Random item properties, what is left** (done: the Item Wizard's
  Magical, Item Level and Item Quality, with rules fitted to 127 items
  Aurora's wizard made, `docs/findings.md`): the rules are fitted, not
  Aurora's code: whether a later property's tier is drawn (one or two
  below the first) or tried in order, and Low items' most worth (levels
  1, 5, 8 and 13 fit what was seen; nothing in itemvalue.2da says so)
  would need more of Aurora's items to settle; amulets, belts, boots,
  bracers, cloaks and rings get the armor table, where Aurora may use
  the helmet's (they differ in Weight Reduction only). All of it waits
  on Aurora: a few hundred of its wizard items, made by driving its
  wizard on the oracle (Low items at each level band; later properties'
  tiers; an amulet, belt, boots, bracers, cloak and ring each at 16-20
  Godly, whose Weight Reduction would tell the table), read against
  `aurora_item_wizard.rs` (M). (Gems and gauntlets are offered Magical
  since 2026-10-30.) (a second builder's review, October 2026)
- **The feats' order in the Levelup Wizard's spell capture, the
  owner's call:** the capture that settled the review's G8
  (`levelup/spells-after.mod`) was made through Aurora's creature
  Properties window, whose OK writes the FeatList sorted by name, so
  `levelling_up_matches_aurora` sorts Moonglow's feats the same way for
  that capture alone. Whether Moonglow's Properties window should sort
  them so too is not checked; the choice is to confirm. (S) (the review
  of November 2026, G8)

## Palettes

- **A custom palette's names in the game, issue 13's other half** (a
  builder's "HipÃ³lito" in the DM's Creator, after a save in Moonglow, is
  fixed and seen right in the DM client: the deferred sprint's log,
  2026-10-29). With Polish the editing language, String Edit read and wrote a string's
  English text as Windows-1252 (mojibake, and Polish letters typed
  could not be saved, under an error that named the String Ref). It
  reads and writes the Windows-1252 languages' text as Windows-1250
  then, as the lists did already and as a Polish game shows it, and
  says which letters a text cannot hold (`text.rs`'s `entry_codepage`).
  Left, waiting on the reporter: String Edit on their module with Polish
  the editing language, its names seen right and one saved. (S)
  (from builders' reports after 1.19.1)

## Custom content data (2DAs and talk tables)

- **Two of a hak's saves stay synchronous:** a hak tab's OK, its closing
  question's Save, Extract, Export as Files, Add Haks and the hak
  editor's own Save and Save As are jobs (the review's D19), but the
  module's Save still writes an open hak at once, as what follows it in
  the same step reads the files written (the game started for a test,
  a project's pack, the plugins' saved event, Exit), and so does the
  NUI Creator's picture hak, whose caller reports at once whether it
  was written: made jobs, each would need a continuation
  (`side_doc.rs`, `lib.rs`), and until then a real unintended
  consequence. Add Haks has no byte progress or Cancel mid-copy
  (`attach::copy` has no hook for one). (M) (the review of November
  2026, D19)


## Area visibility


- **An armor lying as plate here and as a tunic in Aurora**, in the
  same builder's picture. The model is chosen by the torso's
  parts_chest.2da `ACBONUS` (`mg_preview::item::item_placed`, measured
  with the game's own armors): a torso the table lacks, or whose
  `ACBONUS` is blank, lies as cloth (`gi_armor01`), so plate here means
  the table Moonglow read gives his torso 6 or more; that Aurora shows a
  tunic says it read another table or another rule. The game's own
  armors of each `ACBONUS`, 0 to 8, lie in Aurora as Moonglow lays them
  now (captured 2026-10-30: 0 and 1 the tunic, 2 and 3 leather, 4 and 5
  chain, 6 to 8 plate; padded armor, of 1, lay as leather before), so no
  stock item shows it. Waits on the builder's item (its torso's number)
  and his haks' parts_chest.2da. (S) (from builders' reports after
  1.19.1)
## Editing in the area view

- **Heed's interiors' lighting, waiting on Heed** (what he sees, a
  picture of it, and his haks): every area of his reports measured
  draws as the client draws it. His armourer (`hss_armourer`, the area
  he sent in October 2026 as still showing it: `tni01`, 60/170 by day,
  black fog of 5, the smithy's tile lights, no placeables) is within
  0.1 to 1.6 of 255 of the client's picture from four cameras, the
  Shaders switch's two pictures the same, his file and the stock one
  built from it alike (`heed_armourer_look`; findings, "An area's
  ambient and diffuse beside its tile lights"); the one difference seen
  (the forge's fire dark for its first seconds after the area opened or
  changed) is fixed. Before it: a stock smithy's tile lights within 2.4
  (`area_lighting_look`; his "tile lights don't show" was his
  placeables' light nodes, since lit); a `tin01` interior set as his
  within 0.7 to 1.5 at 60/170, 60/150, 255/170 and 0/0, the white
  ambient as flat in the client (`heed_interior_look`); his walls' custom
  pair fed `tverts2`/`tverts3` (up to 2.5.3) and lit as `fslit_nm`
  (findings, "What a custom shader gets of its mesh"). Since his area
  draws as the game draws it on stock content, what he sees comes from
  what his module loads over it (a hak's `tni01` models, textures or
  MTRs, a `lightcolor.2da`, a shader include) or is something other than
  the game's look: his own words for it, a picture of the view beside
  the game's, and his haks are what is needed. 2.13.1's lit check on his
  Vulkan (Radeon RX 580, AMD 25.5.1) drew its red mesh [212, 133, 133]
  through the game's `vslit`/`fslit` where this machine draws [204, 56,
  56] (Moonglow's own shader too): exactly the stock program's color
  with `fNdotV` zero, the surface lit as if seen edge on (the Fresnel
  term at a grazing angle, environment specularity 0.29 for 0.04, the
  diffuse unchanged), which no uniform, texture, buffer or uninitialized
  value of Moonglow's here gives (all are written or zeroed; findings,
  "The lit check's pink on Heed's Vulkan"). The check then read the
  programs through the whole load order, his `override/` and
  `development/` included: a builder's `inc_lighting` there draws so
  (`a_builder_s_include_leaves_the_lit_check_to_the_game_s_own`). It
  reads the game's own copies now and its debug-log line names any
  copy the load order puts over them, with each shader's color. Which
  of the two it was is settled: Philos's RX 480 through AMD's Vulkan
  draws the same pink with the game's own copies (November 2026), so
  it is that driver on Polaris cards, and the pink is not what Heed
  reports of his interiors (findings, "The lit check's pink on Heed's
  Vulkan"). (S) (Heed's reports, October 2026)
- **A drag when a key changes the mode, the owner's call:** a drag under
  way is dropped, not committed, when a key changes the mode (the
  review's A12); committing it would be the other way. Asked. (S) (the
  review of November 2026, A12)
- **Alt + drag in Paint, Select Tiles and with a tile brush, the
  owner's call:** there Alt + drag always moves the view, whatever is
  selected (the review's A14; elsewhere, with a selection, it raises or
  lowers it). Whether that is wanted is asked. (S) (the review of
  November 2026, A14)

## Rendering: creatures, particles, water and shaders

- **A scene with little light draws the client's boxes 4 to 8 times
  brighter than Moonglow's:** read with `chrome1_reflection_look`'s dark
  light (ambient `0x080808`, no diffuse): the client's boxes of
  `chrome1` stay several times brighter than Moonglow's, in the
  placeables' boxes and the worn helmet's alike; the light uniforms the
  two read (`frontLightModelProductSceneColor`, `lightAreaAmbient`) are
  equal, so the cause is not found. Under the sun, and with none, the
  two agree within a few of 255. Deferred as beyond what was asked (the
  helmet fix of 2026-11-25 was the reflection's map; this is the dark
  scene's whole brightness, and wants a reading of its own: which term
  the client adds that Moonglow lacks). M. (the chrome1 reading,
  2026-11-25)

- **A see-through mesh's soft part and shiny water:** read with
  `copies::reading_copy_order` (`MG_COPY_ORDER=front`): a standard
  DXT5 of alpha 0.5 texels with `transparencyhint 1` standing in front
  of a pool is drawn over its ground in the client, the water not
  drawn behind it (its green, painted by a patched `fswater`, 39 of
  255 there against 255 beside), as though the soft mesh wrote its
  depth; and the water's copy holds the soft meshes behind it.
  Moonglow draws a see-through mesh's soft part after the water,
  without writing depth: in front of the pool it is laid over the
  water (green 132), and behind it the copy lacks it (a render test
  with the soft part drawn before the copy took it in, but then the
  water was laid over the soft card in front, green 255). Deferred
  for its consequences: whether the soft parts write depth decides
  how every see-through mesh's soft texels overlap (grass, leaves,
  decals over decals), which wants a reading of its own in the client
  before it changes. M. (The framebuffer's copies entry, 2026-11-26)
  That reading is made (`soft::reading_soft_depth`): the client's soft
  texels write their depth above alpha 0.2. A change that wrote it so
  (kept on the `soft-water` branch for after 5.4.0) matched the client
  (cards in front of water 25.5 to 16.3 off its picture, in front of
  particles 62.4 to 20.4) but was held out of the release for its cost,
  timed by an in-process A/B (alternate frames, so both builds share
  the GPU's clock level; an A/A control flat, 12.61 against 12.37):
  TDoD's woods turned, quickest tenth 9.5 to 9.6 ms before against
  11.4 to 12.4 after, about 2.5 ms (median about 11.5 to 17); the woods
  overview 0.15 to 0.5 ms; the grass, water and area092 views within
  0.05. Deferred for its consequence, a cost too high for the look.
  Cheaper ways to try: write depth only where a soft part stands in
  front of a mesh that reads the copy (shiny water, the particles); a
  depth pre-pass of the soft parts; and see whether the discard with a
  depth write loses early depth testing for the meshes drawn after.
  (2026-11-26)

- **The frame budget of the game's shaders (M3 of [the shaders
  proposal](shaders-proposal.md)):** what the Shaders switch, on by
  default, leaves over the budget or unchecked. (from M3, 2026-10-21) (An area's first look: since 2026-11-02 the pipelines a module's
  areas drew with are kept for the next session and made as the module
  opens, the owner's choice of the two ways the entry offered; with the
  programs cached, area092's look changed 28 ms after it showed and The
  Domains of Dread's woods' 77 ms, and now neither: drawn by the game's
  programs from the first frame they show (asked for with the module's
  first frame), the area opened as the module opens or a second later. The first session's, and a module never opened before, as
  before.)
  - **A dense area seen whole is still slower with the switch on.**
    TDoD's synthetic woods (24 by 24 `mst04` tiles, 400 placeables,
    33,000 meshes on screen) zoomed out: 15.8 ms a frame on against 13.1
    off (was 34 and 26), turning 28 against 21 (was 51 and 36); the GPU
    3.7 against 3.5 (was 6.1 and 7.3). Both over the budget; the bar (on
    no slower than off) not cleared. The game's draws bind two groups a
    material change and more pipelines (796 changes against 349), write
    their values through each program's plan, and are more draws (a
    swaying program's box grown by a meter joins fewer). `NO_DISCARD` 0
    for every material (notes §M.10) not looked at. ([the frames
    note](research/notes_frames.md), 27 October) The review of November
    2026 measured the woods' draws encoded a fifth to a third slower
    than on 27 October, in steps of 1 and 2 November (its Z2, [the
    finders' page](review-2026-11/mechanical.md)); the figures above are
    27 October's. Profiled on 10 November (shadows off, The Domains of
    Dread's haks as rebuilt on 9 November, three runs, the lowest; the
    GPU's figures with the switch on are those haks', not comparable
    with 27 October's): half
    the draws seen whole bound another of the meshes' 13 chunks than the
    draw before (12,621 of 24,965), and Moonglow's own see-through draws
    now go in runs of one chunk too (2,159 changes); the game's keep
    their runs by material, whose pipelines cost more to set again.
    Draws encoded, seen whole: 12.6 ms with the switch off (14.8 before,
    10.2 for the build of 27 October with the same haks), 16.1 on (16.4,
    13.4); turned 17.9 off (19.7, 15.6), 27.4 on (27.7, 22.3). The whole
    frame with the wheel turning: 14.3 ms seen whole and 22.2 turned off,
    16.9 and 29.9 on. What is left against 27 October (2.4 ms seen whole
    off, 2.7 on, 5 turned on) is no one thing in the profile (taken
    before the change): wgpu's
    encoding of the pass (29% of the frame's samples), placing the draws
    in batches (16% with its look for draws met), writing the frame's
    values (12%), sorting (9%), the reach of the lights (8%); the game's
    see-through draws still change chunk at 13,900 draws seen whole.
    Taken apart again on 25 November ([the frames
    note](research/notes_frames.md), 25 November): the game's draws set
    group 1 and the zero vertex buffer again at every draw (5,514 binds
    each seen whole); now each is set only where another is bound, about
    11,000 commands and 1.0 ms of encoding less, measured as both paths
    on alternate frames of one `render_area` run; `frame_perf` at load 2
    to 9 cannot tell 1 ms apart (draws encoded seen whole, on, 16.95
    before and 16.1 after; turned 28.1 and 29.1; the window's frame with
    the wheel turning seen whole, on, 17.5 before and 19.5 after, not
    confirmed either way). Draws encoded seen whole 16.1 ms on against
    12.8 off, turned 29.1 against 19.2 (the lowest of three). What is left is the game's programs
    themselves: 5,500 to 7,500 pipeline changes against 3,200 off, two
    groups set at each (the vertex program reads `texUnit0`, so group 2
    is the material's), and the game's see-through runs by material
    changing chunk 21,800 times against 5,900. Tried and dropped: the
    chunk in their state (slower), a run split by chunk (inside the
    spread). Left, as a change to how every mesh is stored for less
    than the gap: the meshes' indices in one buffer for every chunk, so
    that a chunk change binds a vertex buffer alone (5,300 fewer chunk
    changes were worth 0.4 to 0.5 ms; halving the cost of 21,800 would
    be about 1 ms at most, against 3.3 seen whole). (M)
  - A program whose uniforms can't be split falls back to Moonglow's
    shader, reported. Since 2026-11-02 a block a program names besides
    its loose uniforms (`uniform Name { … };`) is split with them; what
    is left is an array of named blocks read by a variable's index (none
    of the 102 programs seen has one). Drawing that would need its blocks
    written for each draw (its own, its lights', its bones' and the
    frame's values in one) and bound at the draw's own offset, instancing
    off for it: M, for a program not seen. (M)
  - **Pictures drawn once wait for the pipelines** (a gallery's or
    palette's thumbnails, a hovered row's preview: drawn once and kept,
    they can't take the fallback): the first ones a session needs block
    the UI thread for the worker's time (28–113 ms for area092's pairs).
    `frames_at_144` with the default on (nothing over the budget by its
    median) shows the Gallery's worst frames while scrolling at 10 and 49
    ms; not yet compared with the switch off. (S)
  - The tests that compare pictures of the area view and the model
    viewer choose the switch off; only the switch tests and TDoD's wind
    test draw the game's shaders through the window. (S)
- **The engine's choice of program (M4 of [the shaders
  proposal](shaders-proposal.md)):** what M4 left to read in the
  client. (from M4, 2026-10-21) (Static geometry's buckets are drawn as
  the client's since 2026-11-02: a tile's or a static placeable's meshes
  naming the same textures take the first one's alpha and `NO_DISCARD`,
  [the findings](findings.md), "Static geometry is drawn in buckets";
  two static placeables of one texture keep each model's apart in the
  client too, as Moonglow does: `batch8d_bucket_models`, 2026-11-25.)
  - Moonglow's own shader in the builders' areas (TDoD's 690 DXT1
    textures with transparent texels black, its 21 materials naming maps
    without a render hint) waits on those areas' files (`tdod.mod` here
    holds three staging areas). (A DDS with the rippling procedure is
    drawn flat gray as the client draws it since 2026-11-02:
    [findings](findings.md), "A DDS with a rippling procedure loses its
    picture".) (S)
- **The special surfaces (M5 of [the shaders
  proposal](shaders-proposal.md)):** water, grass, the sky, additive
  meshes and particles, read in the client (`surfaces_in_the_client`)
  and drawn with the game's programs or kept Moonglow's. (from M5,
  2026-10-21)
  - **Soft particles** fade as the client's with the Shaders switch on
    (`particleSoftness` 10 over the emitter's larger size, a `Linked`
    emitter's and a lightning bolt's too: read on 2026-10-30,
    `reading_soft_particles_of_lightning_and_linked`, and Moonglow's bolt
    made soft). A particle's foot fades about half a row of pixels
    earlier in Moonglow than in the client, the same in every blend
    (the fade painted into the color by a patched `fsparticle`: 0.5 at
    2.1 rows from the edge against 2.65, `batch8d_soft_foot`,
    2026-11-25; Lighten fades no more than the others, as the old
    reading's edge made it seem). Unsettled: the depth lookup, the
    multisampled depth's resolve, or the camera. (S)
- **Emitters a builder does not see** (GitHub issue 8: a demon's wings,
  braziers, shafts of light, all without particles in his area): the
  wings are likely explained and fixed on 2026-11-10. Wings a creature
  wears from `wingmodel.2da` are a part, and a part's emitters were not
  simulated (only a preview's base model's): CEP's "Balor: Flame" wings
  (`c_wingsbalr`, row 9) are wings full of fire emitters, as its `@VFX`
  rows are effects; the area view draws them now ([findings](findings.md),
  "A creature's parts' emitters"). The game's own balor carries its fire
  in its own model, which was drawn. The braziers and shafts of light
  are not reproduced: the game's brazier and campfire show theirs here
  (`look_particles_of`), and a static brazier's fire runs in the client
  as a dynamic one's, as in the area view
  (`reading_static_placeable_emitters`, 2026-10-30), so it is his
  content, a setting (Animations off shows none) or his machine. Waits
  on him: which wings and which braziers. (?) (from builders' reports
  after 1.16.2)
- **Builders' reports after 1.21.0:** a creature's body part can be
  set to none (0), as in Aurora; a body part whose mesh is a skin is
  drawn as the client draws it ([findings](findings.md), "A skin mesh
  in a body part or a held item": not where it is compiled, a plain
  mesh where it is text; a held item's never). Not done: a skin
  weighted to nodes of its own part, which the client draws out of
  place, is not drawn (S). The compiled skins tried were `mgv
  compile`'s: the game's own compiler (`nwmain compilemodel`) compiles
  none (2026-10-30, `reading_game_compiler_skins` in the fifth client
  batch: a part as a plain mesh written, 1,400 bytes, the same as a skin
  nothing), so a compiled skin is always another compiler's. Not
  reproduced: "unequipping an armor sets all body parts to 0": nothing
  in the code writes a body part on unequip, and the preview draws the
  creature's own parts then (waits on the creature it was seen on).
  (from builders' reports and the owner's notes after 1.21.0)
- **See-through meshes** are drawn in two parts: what is nearly opaque
  of them (alpha from 0.95) with the depth written, then the rest over
  everything solid, back to front, writing none. (A builder's report:
  blue-white patches round plants and along shores at a slant, in a
  base-game tileset: a plant's soft edge drawn before the ground behind
  it kept the ground from showing, and the water under it showed. Shown
  in a renderer test; not confirmed on the reporter's area, which isn't
  here; compared with the client on a rural shore, ttr01's tile 7 and
  its water (`reading_shore`, 2026-10-29): no patches in either, the
  water as the client's with the Shaders switch on.) A mesh of mesh
  `alpha` under 1 (of an instance drawn whole, not a cut-out) is drawn
  in one part since 2026-11-02, blended and writing its depth, in its
  model's order, as the client draws them (`reading_see_through_crossing`:
  a card listed after another and behind it is hidden where they
  overlap; a ribbon behind one is hidden too); its texture's soft edges
  blended as they are, writing their depth as the client's do. Two
  models' such meshes are put in order by their instances' places
  (where each stands from the eye), the meshes of one in its order, as
  the client orders two dynamic placeables' and two static ones' of
  textures of their own (`batch8d_see_through_order`, 2026-11-25). Not
  explained: two static placeables whose textures dynamic placeables
  share as well are not drawn farthest first in the client (the nearer
  drawn first, the farther hidden), by no listing order, model, texture
  or row name tried; nor is it read whether the order is the
  instance's origin's or its mesh's. Each see-through mesh
  of alpha 1 costs two draws. On macOS the solid part's discarded
  fragments wrote their depth (the plant's edge hid the ground again)
  until the shader stopped at each discard: seen and mended on the CI's
  Mac, a virtual one; no real Mac was looked at. Left: the smaller
  levels made of an uncompressed texture leave transparent pixels'
  colors out; a compressed DDS's own levels are used as stored. (S)
- **Custom creatures from the Vault and CEP** (a builder's report,
  [issue 3](https://github.com/jadzziaa/moonglow-toolset/issues/3)),
  fixed against the content itself but none of it compared with the
  client: a supermodel's position keys scaled by `setanimationscale`
  (taken as the size against the model whose animation is played); a
  body part drawn only where the base has its node; a bitmap name over 16 characters
  cut to 16; a blank line among ASCII skin weights read as a vertex
  without any (its faces not drawn since 2026-10-29, as the game draws
  none of a vertex no bone weighs), and since 2026-10-30 as spacing only
  in a list that has all its rows without it. The reporter's "stopped
  animating" (Mindwitness) wasn't seen as such: here it animated, with
  its skin torn by the shifted weights. An animation through a chain
  of scaled supermodels plays at each link's scale but the owner's, as
  the client's (since 2026-11-02; [the findings](findings.md), "An
  animation played through a chain of supermodels is scaled by each
  link"). A mesh with `bitmap NULL` drawn in the part's own PLT is the
  client's (2026-11-10, `part_textures_look`, row F). Waiting: the model
  viewer's own check of names over 16 characters (the Moonglow Viewer
  session's); and the Mindwitness's pose, other than in Aurora and the
  game (the builder's model). (S)
- **Shadows:** the area view draws the sun's and moon's shadows
  (2026-10-28: a shadow map standing in for the game's stencil volumes,
  laid as the game's shadow plane lays them, checked against the client
  in `shadows_look`; [the research note](research/notes_shaders.md) §G),
  and creatures' and lights' shadows (the Shadows button's menu, done
  2026-10-28); since 2026-10-30 also as the game's own stencil volumes
  (Shadows quality High: [the notes](research/notes_shaders.md) §G.2),
  beside the client in `high_shadows_beside_the_client`; their volumes
  made apart from the frames and kept by the renderer, let go of with
  their models, since 2026-11-25 ([the frames
  note](research/notes_frames.md)). Left:
  - every creature in a lamp's reach casts at once (eight ogres in a
    row, each its shadow: `batch6_light_shadow_casters`), as Moonglow
    casts up to 16; and a player's own light casts the shadows of the
    creatures about the player (`batch6_player_light_at_night`), which
    the toolset, placing no player, has nothing of. (A light's shadow
    darkens by the light's share of the light there, as the client's,
    since 2026-11-02: [the findings](findings.md), "A light's shadow
    takes the light's share".);
  (Grass and the see-through meshes of tiles and static placeables,
  their solid and soft parts, take the shadows as the client's since
  2026-11-25: [the findings](findings.md), "Grass and a tile's
  see-through meshes take the shadows"; water takes none, as the
  client's, and a shallow pool shows the shadow on its floor through
  the water, as the client's, since 2026-11-26.)
  (The sun's and moon's casters cast from the faces whose front faces
  the light only, as the client's, since 2026-11-02: [the
  findings](findings.md), "A face casts the sun's shadow from its front
  only"; the city street under a low western sun lit but for its spikes
  as the client's, the ground at a cliff's foot lit, its shadows' pixels
  agreeing 0.80 with the client's where they agreed 0.62. The fade with
  the height between plane and caster that 8193.31 names is not seen:
  armoires raised 0, 1.5 and 3 m darken the ground alike, 0.44 to 0.49,
  and casters above the camera still cast, as in Moonglow:
  `reading_shadows_of_doors_items_and_heights`, 2026-10-29.)
  (The model viewer's model casts the sun's shadow on a ground under it
  that takes only shadows, its Shadow switch on, since 2026-11-02; the
  other previews draw none: settled by the owner's word, below. With
  Fade Geometry on, the faded roofs and upper walls cast their shadows,
  as the game's, since 2026-11-02.)
  (asked by the Domains of Dread module session and by a builder,
  Troubletcat)
- **The renderer's textures by name, across sources:** the window's one
  renderer keeps its textures by name; they are forgotten when the game
  data changes, a module closes, or a texture, MTR or TXI of the
  module's own files changes outside (the review's B15), but not when
  the model viewer draws a hak's model from the hak's own load order
  through the same renderer. Since the review's last pass (its item 6)
  an Import forgets them where a DDS, TGA, PLT, MTR or TXI of the
  module changed, was added or was removed (`refresh_module_layer`).
  Clearing them at every change of source would read every texture
  again, and a hak's model drawn beside an area view would clear them
  twice a frame: a real unintended consequence. Caches per source in the
  renderer would do. (M) (the review of November 2026, B15, and what its
  fixers saw on the way)

## Models and textures

- **Heed's previews after 1.33.0** (not reproduced): the Drow shields of
  the standard palette shown as bags in their thumbnails and the
  palette's hover preview. In the base game they show whole
  (`item_previews` in `screens.rs`: `x2_adrowshl001`). What makes a bag
  (2026-10-30): the Drow shields are large shields (base item 56,
  `ItemClass` `AShLw`, `ModelPart1` 33: `ashlw_033`), and baseitems.2da
  row 56's `DefaultModel` is `it_bag`, which `mg_preview::item_parts`
  falls to where the class and number name no model in the load order:
  a hak's `baseitems.2da` whose row 56 names another `ItemClass`, or a
  load order without `ashlw_033`. Waits on his hak list or a blueprint.
  (His armor showing only a head and pelvis was Beamdog's HD bodies,
  drawn whole since 1.34.1.) (S–M)
## The main window and its panes

- **The light theme's own colors:** it is the redesign's Light theme
  (`mg-uikit`'s tokens, since its M2; this entry said egui's); the script
  editor's syntax colors and the conversation editor's line colors
  follow it, the Faction Editor's sliders keep Aurora's red and blue
  (legible on both: `conversation_and_factions_in_both_themes`, which
  shows Basic's chart), and so do its matrix cells' frames and the
  Reactions panel's bars (legible on both: `screens.rs`'s
  `board_factions` in the light theme, Advanced with Reactions beside
  it, shows the red and blue cells and a friendly bar), and the area
  view's overlays are drawn over the 3D view, whatever the theme. What
  else a builder who uses it finds wanting waits on such a pass. (S) (a
  senior builder's review, October 2026)
- **The docks' fold controls and a window dropped on the middle, by
  hand** (GitHub issues 10 and 21, open: wait on a hand in the running
  program). Issue 10's one-button rule is built as the shell's (M3, the
  proposal's U3): each dock has one control of its own to fold it away,
  the Left and Right Docks a button at the end of their tab bar (folded,
  the chevron at the top of their strip, which brings the dock back as
  wide as it was), the Bottom Dock its Log switch in the Status Bar;
  View › Left Dock, Right Dock, Log, Hide All Panels and Reset Layout,
  with their keys; the state kept in the settings. It waits on its
  reporter's retest on the release that carries M3. Issue 21: a tool
  window dragged from a window of its own onto a side target of the
  middle was seen to move and not to dock, by the tests' drag, which
  can't make a window of its own; it waits on a drag by hand, the
  owner's (a tool's tab dropped on a side of the middle is docked there,
  `a_tool_tab_dropped_on_a_side_is_docked_there`). (S) (GitHub issues 10
  to 24, after 1.19.1 and 1.20.0; 21 after 1.30.2)
- **The menus by the keyboard** (GitHub issue 9), on egui's own menus:
  Alt alone or F10 opens the first menu and closes them (over an area's
  view F10 is Select Tiles', as in Aurora), Left and Right
  go between menus (round the ends), Up and Down between rows (past
  separators and rows that can't be chosen), Enter and Space choose,
  Right and Left open and close a submenu, and the pointer over another
  menu's name opens it while one is open. The marked row is drawn as
  selected. Alt and a menu's first letter (underlined) opens it (unless
  a command was given that key in Options › Keyboard; not AltGr), a
  letter with a menu open goes to the next row that begins with it, and
  Escape closes the submenu the keys are in before the menus; all of it
  under test (`menus_by_the_keyboard`, Alt alone by its modifier
  events). Waits: Alt alone on Windows, where the system may take Alt
  for its own menu (a Windows build). (S) (from builders' reports
  after 1.19.1)
- **A long dropdown that grew to the window's edge, not seen growing
  here:** the open list of a dropdown with a Filter field (Area
  Properties › Audio › Ambient Sound, Day) widened over 5 to 10 seconds
  under Windows. On Linux it grew once, by its scroll bar's width, on the
  frame after the one that measures it, and never again (100% to 175%
  interface sizes, frames of 1/144 s, the window at fractional
  positions, the pointer moving and the wheel turning over it); its
  width is now set when it opens, the same every frame, which closes
  the way any growth took. To check under Windows at the builder's
  interface size. The other dropdowns (no Filter field, no list of
  their own inside) keep egui's sizing, which can grow by the scroll
  bar's width once. (S) (Troubletcat's report, October 2026)

## Remappable keys

- **Shift+Insert in the window itself:** it asks the backend for the
  clipboard (`RequestPaste`), and the paste comes a frame later; the UI
  tests stand in for the backend, and the keys were not pressed in the
  running program on Linux, Windows or macOS. Shift+Delete in the module
  tree no longer deletes the resource (it is a cut, which the tree does
  not take). (S) (Sacha's report on 2.5.4) (waits on the keys pressed
  in the running program: the owner's on Linux, Sacha's on his system)

## NUI Creator (the review of its pull request, October 2026)

The NUI Creator came as a contributor's pull request and was brought in
line with the codebase before it was merged: its frames (26 to 35 ms after
an edit, now 1.4 to 2.4), one Undo step a field, the toolset's labels,
headings, name window and OK and Cancel, typed errors and lossless
settings in `mg-nui`, Build through the module's compile path, its tests
where the crates keep theirs, and a client test of what its checks say of
the game. This is what that review left, and after it what the
contribution itself states as not done or not checked.

- **Left by The Domains of Dread's NUI Creator asks (October 2026)**
  (AccessKit; the asks are the module project's
  probe, `tdod-modulework/docs/systems/nui-creator-probe.md`; done on
  2026-10-29: a state for one row, variants of several groups, binds held
  to their properties' rules, a list's rows out of view and the title in
  `layout`, the layout's schema, the Creator's Import…, the hand-run
  `a_module_s_windows_are_laid_out_as_json` gone; moved: "`mg` links
  `mg-ui`" to "`mg` builds the GPU stack by default", "the preview is not
  the game" and `layout`'s `overlaps` to "Where the preview is known to
  differ from the game"):
  - *AccessKit doesn't start on newer Linux desktops*: eframe 0.36's
    `accesskit_unix` (0.21) offers the tree only when the session bus's
    `org.a11y.Status.ScreenReaderEnabled` turns true, and at-spi2-core
    2.62 (the development machine's) has only `IsEnabled`, so there it
    never starts; `accesskit_unix` 0.24 watches `IsEnabled`, and comes
    with an egui/eframe whose `accesskit_winit` uses it (0.34; eframe
    0.36.2, the newest on 2026-10-29, has 0.32). Until then the desktop
    agents here can't read the window. (S, with that upgrade)
  - *AccessKit was checked on Linux only*: the GUI checks for Windows
    (`x86_64-pc-windows-gnu`) and eframe's AccessKit for macOS
    (`aarch64-apple-darwin`); CI builds them, and no one has read the
    tree with a screen reader on any of the three, nor in the Flatpak
    (`--talk-name=org.a11y.Bus`, from reading `accesskit_unix`). Its
    cost with an assistive program connected, `FRAME_ACCESSKIT=1` in
    `frame_perf` on the campaign's 117 views: 0.86 ms a frame on
    average without the tree, 0.97 with it, the most 0.55 ms more (a
    long script, 0.73 to 1.28); none over the budget. Without one,
    nothing. (S, waits on a screen reader's user)
- **A scroll bar's end button held, the owner's call:** the client
  scrolls a bar every frame its end button is held, by 1% of the bar's
  length in whole pixels (`reading_scrollbar_end_buttons`), and the
  preview's list, group and text bars now do too (the review's N9).
  Three choices of the client reading's to confirm:
  - the rate is the client's at 60 frames a second: the client's speed
    follows its frame rate, so no one value matches every player;
  - some presses on the client's text control moved nothing, for a
    reason not settled, so the text scrolls at the rate of a group of
    its height;
  - the list scrolls by pixels at the same rate, where the client moves
    one in whole rows: snapping to rows was left out.
  (S) (the review of November 2026, N9)
- **The NUI Creator's per-frame walks, what is left** (measured
  2026-11-25 in release, `nui_perf`; `check`'s copy, 0.3 µs on an idle
  frame, and Interact's two comparisons, 47 µs at 304 controls, need
  nothing):
  - **A variant's context document** is built anew each frame while the
    variant is edited: 277 µs at 304 controls (86 at 96). A cache would
    compare the variant, the settings and the window (90 to 100 µs), and
    the canvas takes the document owned, so a hit would also clone it
    (168 µs): it saves nothing unless the canvas borrows the cached
    document, and every frame that changes the window (typing) would
    pay the cache's copies on top. Not done for that unintended
    consequence; the saving is at most 180 µs a frame.
  - **A long chart's walk:** a series of a million samples is walked each
    frame, 1.3 ms for a line (the fingerprint that keeps its points) or
    for columns (their least and most value). The window is edited in
    place, so there is no cheaper sign that a value changed; a revision
    kept by every writer of the window and its binds would be one. Not
    done: beyond the frame budget's need (the idle frame is 2.0 ms).
  (S) (the review of November 2026, N13, N16)
- **A font of a sliver of a line, not reproduced:** a font whose line is
  a sliver of its em is refused and the skin's numbers are held to a
  range (the review's N17), but the gigabyte glyph itself was not
  drawn: the test's font has no glyphs, so the size it would be drawn
  at is tested, not the drawing. (S, waits on such a font with glyphs)
  (the review of November 2026, N17)

## Automation

- **Plugins, beyond API 0.4** (`plugin-proposal.md`; commands and checks
  in sandboxed Luau are built; 0.2 added a file or folder the user
  chooses, haks written with their leave, areas and terrain, pictures;
  0.3 files written into a chosen folder, 2DAs and talk tables as
  values, and events, whose own gaps are below, under "Plugins writing
  into a folder"; 0.4 what is selected and
  open, commands in right-click menus, fixes for findings, a plugin's
  own tests, settings, a project's own plugins, a resource's own events
  and the after-create hook; 0.5 (3.3.0) a batch of strokes, more kinds
  of setting, more of what a command is for and the menus it is in, a
  fix that asks, archives of several plugins and the rest of this
  round's gaps; 0.6 (2026-11-02) exporters, importers, wizards and
  panels, the proposal's "API 0.6"):
  - **A builder's terrain tool (NWN Mapper) as the measure:** it sculpts
    a free height field, paints texture layers and a walkmesh, bakes
    placeables and a lightmap in, and writes a tileset's group (a model
    and walkmesh per tile, textures, materials, a patched `.set`). A
    plugin can now take such an export into a hak and make the area
    (`examples/tileset-import`, tried on the builder's earlier export:
    149 files, an area of one of its groups drawn from the hak). What
    the tool itself does is out of a plugin's reach: there is no view
    to draw in or brush to take input from, and baking textures in
    Luau would be slow. Sculpted terrain inside Moonglow would be a
    feature of its own. (L) (After 3.0.0, the owner's word, 2026-10-30:
    9th of 9 by complexity.) Designed on 2026-11-02, the owner's word
    being design first, then build: [the terrain
    proposal](terrain-proposal.md) (a mode of the area view, a tileset
    of the area's own, layers blended in the game by a shader Moonglow
    writes, six milestones, the readings to take in the engine and
    client, eleven questions), its recommendations accepted by the owner
    the same day, the terrain shader to be offered under MIT. Nothing is
    built: waits on the owner's word to start M1, and on dafena's answer
    (a license and an export, which the owner asks for).
  - **Not tried in the application on a desktop:** the folder and the
    hak question were driven through the UI harness and `mg`, not with
    the real file dialogs (the file dialog of `open_file` filters by
    the extensions asked for; that filter was not seen on a desktop
    either); nor were a project's plugins' question (and Allow All, the
    folder looked at while open), the settings' form (the kinds of 0.5,
    Choose… for a folder, Tidy Up; rendered as `plugin-settings` and
    `plugin-settings-kinds`), the menus of 0.5 (tiles, categories,
    groups, tabs, the script editor, Find References, Advanced Search),
    Verify's fixes (a fix that asks, Fix All, Undo) and the events of
    3.3.0 (Save As, the makers, Manage Plugins' time), and what API 0.6
    added (File › Export's and Import's submenus, the save dialog's
    filter of an exporter's kind of file, a wizard's pages, a panel
    docked beside the area; rendered as `plugin-export-menu`,
    `plugin-wizard-check`, `plugin-wizard-page`, `plugin-panel`): all
    driven through the UI harness. Waits on a desktop session. (S)
  - **No list to install or update from:** plugins come as files; the
    proposal has an index with API 1.0. Remove is for what Install from
    File installed; a folder copied in by hand is deleted by hand. (M)
  - **The 2DA editor's faults with fixes:** the 2DA editor is another
    package's this round; its faults offer no fix yet. Waits on that
    work. (S)
  - **The type file was not tried in an editor:** it loads as Luau, and
    tests hold it to have every name and the reference's parameters
    (`the_type_file_has_the_whole_api`, `the_reference_says_what_the_code_has`),
    but it was not checked with the Luau language server: neither
    `luau-lsp` nor `luau-analyze` is on the development machine, and the
    Luau that mlua builds in has no type checker (no `Analysis`). Waits on
    a download of `luau-lsp` (or `luau-analyze`), the owner's to allow.
    (S)
  - **External programs** as plugins (a protocol over standard input and
    output, for Python or Nim): designed (the
    [proposal](plugin-proposal.md#external-programs-the-design-2026-11-01):
    JSON-RPC over standard input and output, the reference's names as
    its methods, marked as running with the user's rights), not built:
    left for its own round, after this one's S items, in the owner's
    order of the brief ("last, if room remains"). (M)
  - **The window with plugins was run on Linux only:** the application
    itself was driven on an off-screen display there (plugins installed
    from archives listed and enabled, a command as a background job, a
    form answered, Undo, a check in Verify Module, the enabled plugins
    kept over a restart, `--no-plugins`), and the rest through the UI
    harness. On Windows and macOS the tests pass in CI, but nobody has
    used plugins in the window there; and the file dialog of Install
    from File was not driven anywhere (the harness answers for it). (S)
  - **A table's description's `faults` looks no strings up**
    (`mg.twoda.describe`): a `strref` column's string is not held to a
    talk table, which would come from the plugin host. The last of the
    2DA editor's entry (the rest done in the deferred sprint,
    2026-11-02); left because `crates/mg-plugin` was another package's
    that round, not begun. (S)
- **What a plugin's job makes the host hold, the owner's call:** 512 MB
  (`HELD_LIMIT`), beside Luau's 256 MB, since one picture of the 64
  million pixels a plugin may decode is 256 MB by itself; tables and
  pictures give their bytes back when the plugin lets go of them, so
  there is no line limit on `tlk:set` and no count limit on fixes (the
  review's C2). Whether 512 MB is the bound wanted is asked. (S) (the
  review of November 2026, C2)
- **A byte order mark on a conversation's CSV, the owner's call:** Import
  Lines reads a CSV that begins with one (the review's V5); should the
  CSV export begin with one, so that Excel reads its accents (other
  readers then see the mark)? Asked. (S) (the review of November 2026,
  V5)
- **Twine and the `moonglow` object:** a conversation through Twine
  keeps its end scripts, delays and zoom in a `moonglow` object of the
  story's `StoryData` (the review's V7); whether Twine itself keeps that
  object when it saves the story is not checked. Where a story says
  nothing, `mg dialog-import` keeps the replaced conversation's
  settings. (S, waits on Twine) (the review of November 2026, V7)

## Tileset authoring

- **A tileset with holes** (a builder's report: a big old tileset,
  merged by hand, drew no tiles at all) is read tile by tile, as the
  game reads it: a tile without its section or a model draws nothing,
  the area's problems and Verify (`set-hole`) name the holes, the line
  ends and lengths the server refuses are errors and a mix it reads a
  warning (`set-line-ends`, the rule read in the engine 2026-10-30), the
  tileset editor shows the holes, and a model that is there and can't
  be read is said to be, with why, not "not found". Not done:
  - The builder's own tileset (trf01, 2813 tiles) has no holes and CRLF
    line ends, and Moonglow draws its tiles whose models the game has:
    why it draws none of theirs is not known without their hak (its
    models, or the module and the haks it lists). The log now says,
    for each model left out, whether it was found and why it can't be
    read. (S, waits on the builder's hak)

## The install, starting and crashes

- **The recovery copy written at a panic** holds the work as the frame
  that panicked left it (a change applied in it is in the copy); one in
  a thread of its own has the next frame write it, and one in eframe's
  own code has the application write it as the unwinding drops it
  (2026-10-29, `recovery.rs`'s tests). Not tried in a real crash: on
  macOS a panic unwinding into the system's code aborts (after a
  frame's copy is written; one in eframe's code there may abort before
  the application is dropped), and eframe's unwinding through the event
  loop was read in its code (`run_app_on_demand` on the stack), not
  crashed on Linux, Windows or macOS. (S) (waits on a crash on Windows
  and macOS) (Sacha's report, 2026-10-25)
- **The fixes of the review of November 2026 on Windows and macOS,
  compiled, not run there:**
  - Windows: the recovery copy's lock (`File::try_lock` on
    `<stem>.lock`, a second window's copy `-2`: C27's second half), its
    Windows branch compiled for `x86_64-pc-windows-gnu`, not run; a lost
    device (R8), tested here by closing a device, not by a real driver
    reset; an archive's hak named with a colon or a control character
    left out (P21), the names taken on Linux, the write on Windows not
    tried.
  - macOS: the system's version read from `SystemVersion.plist` for a
    graphics record (R25), from reading, its pure parts unit-tested;
    Option with a key that types kept for a text field (S16), from
    reading; Test Module's look for a running game asking `ps` within a
    time limit (the editors' smaller notes), unchecked there.
  (S, waits on CI's runs and on a Windows machine and a Mac) (the
  review of November 2026)

- **Steam Workshop content** is read (Options › General): each item's
  `override` folder as a layer under the user's own, its `portraits`,
  `ambient` and `music` under the user's and over the game's, its `hak`
  and `tlk` folders searched. Which of two items' wins in the game isn't
  known (the lower number here), and where the game puts them against
  the user's own folders is from how it is described, not measured: the
  client runs here with Steam hidden, never started, so it waits on a
  builder's reading with Steam. (S) (from builders' reports after 1.14.1)
- **Finding the game:** Steam's libraries are read from its
  `libraryfolders.vdf` (a game on a second drive), GOG's usual folders
  tried, Steam's own folder read from Windows' registry (`reg query`
  of `SteamPath` and `InstallPath`: held by Wine's `reg` in
  `steam_folder_from_this_machine_s_registry`), and Beamdog's client's
  libraries from its `settings.json` (as neverwinter.nim's
  `findNwnRoot` reads them; else `~/Beamdog Library`), 2026-10-29. Not
  tried where they live: a real Windows with Steam elsewhere (a folder
  of other than ASCII letters, which `reg` prints in the console's
  codepage), Beamdog's client on any system, and the reporter's case (a
  second drive on Linux), covered by the list. Since the review of
  November 2026 (F20) the usual folders are tried before any drive
  letter, so that a GOG or Beamdog install is found before one in an
  unlisted `X:\Steam` or `X:\SteamLibrary`: not timed (that needs
  Windows with an unreachable mapped drive), and its `reg.exe` path not
  run. (S) (waits on a Windows
  machine and a Beamdog install) (from builders' reports after 1.14.1)
- **The graphics backend** (Options › General, `mg_render::backend`;
  El Grillo's report, October 2026: an AMD Radeon PRO's tiles gone one
  by one through AMD's Windows Vulkan driver, 25.10.37.03, drawn right
  with `WGPU_BACKEND=dx12`; Heed's, since 2.11.0: a Radeon RX 580 on
  Windows 10 drawing nothing of an area through Direct3D 12, right
  through Vulkan). Automatic keeps Direct3D 12 first for AMD on Windows;
  the startup check (`mg_render::probe`) takes the other backend where
  it draws nothing, the lit check (`probe::probe_lit`, once the game's
  data is read) checks a lit, textured mesh by Moonglow's own shader and
  by the game's, and the area view's watch (`mg_render::watch`) offers
  Restart with … where drawing collapses later. Neither case was
  reproduced: each is imitated (`guard::blank_draws`, `fail_on`,
  `fail_program`, `stall_textures`). Left over:
  - Heed's machine: 2.13.0 drew nothing there through Direct3D 12 on
    Automatic and neither check said so (2026-10-28). What could get
    past them was closed after 2.13.0 (frames that never get ready are
    judged after 20 s with nothing finished; tiles whose models failed
    are expected; a look the GPU doesn't give back is given up; the
    pipelines' internal errors are caught, FXC's failures on
    `fslit_nm` and `fslit_sm_nm` among them; the lit check; the watch
    writes what it sees to the debug log), and Direct3D 12 compiles with
    the DXC shipped beside the programs since (`mg_render::dx12`;
    FXC, where `dxcompiler.dll` is missing, can't compile those two nor
    Heed's `fhss_skintile`, a copy of `fslit_nm`). The startup check passed there and draws unlit meshes alone,
    so Moonglow's own unlit pipelines draw on that driver; why the rest
    doesn't is not known. Waiting on Heed, with the next release: the session's
    debug log (`%APPDATA%\Moonglow\logs`: its `graphics` and
    `graphics watch` lines, and any `[WARN` or `[ERROR` ones; the
    `graphics:` line now ends with the compiler taken, which should be
    DXC), any
    `crash-*.txt` or `model-failure-*.txt` in `%APPDATA%\Moonglow` from
    2.13.0 and the next (2.13.0 wrote one where FXC failed a program, as a
    crash though it went on), whether a notice appeared, whether the
    area is drawn with the Shaders button off on Direct3D 12, and whether
    the skin tile is drawn by the game's shaders now (DXC compiling for a
    real driver is checked on CI's software one alone, WARP). (S, needs
    the builder)
  - El Grillo's machine: 2.13.0 renders well on his Radeon PRO
    (2026-10-28), on Automatic's Direct3D 12: that confirms Direct3D 12
    there, not the watch on his Vulkan case. Waiting on him: Vulkan
    chosen in Options, whether the watch's notice appears as the tiles
    go. The owner decides Automatic's order after both. (S, needs the
    builder)
  - Why AMD's Vulkan driver loses the tiles isn't known: Moonglow's use
    of Vulkan may be at fault (a validation run on that driver would
    say), nor why Heed's Direct3D 12 draws nothing, and nothing was
    reported to AMD. Until then every AMD GPU on Windows takes Direct3D 12
    first (the renderer's tests run there on CI, through Microsoft's
    Basic Render Driver, and Metal on Apple's Paravirtual device: the
    2.13.0 run drew the shared buffers at a base vertex, textures read
    on threads, the game's programs written for the tests and the
    framebuffer's copies). (M, needs the hardware)
  - The window's adapter is chosen by Moonglow (egui-wgpu's adapter
    selector), which leaves wgpu's power preference and surface check
    to it: `pick_adapter` repeats wgpu 30's order of adapter kinds and
    the app checks the surface. A later wgpu that orders otherwise is
    not followed by itself: look at it at the next wgpu bump. (S)
  - **OpenGL is not offered, and is tried on one machine.** The
    renderer draws on wgpu's OpenGL back end (`WGPU_BACKEND=gl`): its
    93 picture tests pass on Mesa radeonsi, and the three causes of the
    failures are guarded without OpenGL. What stays open: NVIDIA and
    Intel drivers, Windows and GLES are untried (waiting on machines to
    try them on); whether Options offers OpenGL among the backends in
    the adapter pick, and the wording of the fallback message where it
    is taken (the owner's decision); and anti-aliasing on OpenGL, which
    draws one sample a pixel until wgpu-hal makes multisampled textures
    there (waiting on wgpu). (S each)
  - `mg` and the other headless renderers take Automatic (or
    `WGPU_BACKEND`), not the GUI's setting; Moonglow Viewer, which builds
    on mg-render, doesn't use `pick_adapter` yet. (S each)
- **The texture and picture readers end detached, holding the GPU:**
  the threads "moonglow textures" (`mg_render::renderer`) and "moonglow
  pictures" (`model_view`) hold `Gpu` clones and end by themselves, so
  one still working as the process exits can race its end, as the
  shader worker and the lit check did (a crash at exit, debug builds
  with Vulkan's validation layer only, which release builds do not
  load; none with the layers off). Those two are joined at drop since
  M5's S1; these are not: they are replaced mid-session (the texture
  reader whenever the game data changes), where a join would hold the
  window. Not seen to crash (picture readers 0 in 10 lone runs).
  Deferred for the unintended consequence, by the owner's ruling of
  2026-11-22. (S, the owner's ruling; M5 S1's review)

## Documentation

What the [audit](docs-audit.md) of 2026-10-12 could not put right with a
few proven words needs a decision or a rewrite and is listed there, per
document; the entries here that name the audit are that list in short.

- **The wiki corrections were never submitted** (`wiki-proposal.md`);
  all fifteen pages are unchanged upstream (the mirror's sync of
  2026-10-26). Each page's edit is ready to paste in
  [wiki-corrections/](wiki-corrections/index.md) (the passage, its
  replacement, an edit summary, the evidence); submitting them waits on
  the owner. (S) (the documentation audit of 2026-10-12)

## The code itself (the code review of October 2026)

A review of the whole workspace for dead code, clumsy code and code
written twice found more than it changed. What it changed (some forty
commits, with a test for all but one of the bugs it fixed) is in the
history; what it left is done, the last of it (typing in a very large
script) after 3.0.0, on 2026-11-02 (the [log](log.md)).

## Written more than once (the review of 2026-10-21)

Five reviewers read the workspace for logic written more than once, a
builder having said that AI-written code "reimplements a lot of stuff in
minorly different ways instead of building one source of truth". Where
copies had drifted into a wrong result, the fix went through one place
with a test (the log, 2026-10-21). This is what is left: places by file
and line as they were that day, found by reading, not by running unless
it says so. One question settles most of them: which copy is the source
of truth, and does each caller's difference matter.


## Claims, protected areas and the project's commands (the builders' proposal, C6 to C8)

The Domains of Dread's builder proposal (its section 3, and the hand-back
proposal's section 8.2 to 8.4): the claims table, claimed areas read-only
or asked about before a save, Claim This Area and Release Claim, merged
areas' Tag and ResRef asked about before they change, `mg claims`, and
File › Project. What is left:

- **The project's commands on Windows** (`cmd /S /C "<text>"`, the
  command's text as written, no console window, since the review of
  November 2026's D16, as for `mg nwsync`'s upload) are tested by CI's
  runs of the UI tests, and D16's Windows-only unit test
  (`a_quoted_program_with_a_quoted_argument_runs_as_typed`) was
  compiled here for `x86_64-pc-windows-gnu`, not run: CI runs it. Not
  on a builder's machine with a real update script. (S, needs Windows)

## Release and packaging

- **Run only here: the macOS app, and an update installed from the app
  on Windows and macOS** (GitHub issue 28's install, built 2026-11-02:
  [the design](update-install-proposal.md)):
  - The app and disk image are built by CI and run by a builder on
    macOS since 3.15 (a report of 2026-11-08: resizing the Open dialog
    aborted Moonglow). That crash is winit's (rust-windowing/winit#3992:
    a native file dialog runs its own loop inside the frame, and an
    event it brought made winit panic where nothing can unwind); winit
    0.30.13 is vendored with the event kept for after the frame instead
    (`vendor/winit/MOONGLOW.md`), checked here by compiling it for macOS
    and by CI's macOS build, not by running it: it waits on the
    reporter's word that a resized dialog no longer crashes. The lasting
    way, should winit not fix it, is file dialogs that don't block the
    frame (asked for and answered a frame later), which touches the 63
    places that ask for a file: beyond what was asked. (S, waits on the
    reporter)
  - Installing an update on a Mac (the disk image's app copied out with
    `hdiutil attach` and `ditto`, swapped in as Moonglow closes, started
    with `open -n`) and on Windows (the installer run silently with
    `/MOONGLOWRESTART=1`, which `moonglow.iss`'s second `[Run]` entry answers;
    the zip's files moved aside and in while `moonglow.exe` runs). Their
    steps are unit tests here (`mg_update::install`), the AppImage's and
    the zip's swap run on scratch copies; none was run on the system it
    is for. Since the review of November 2026 `hdiutil`, `ditto` and
    `reg` run by their full paths with a time limit (F19), and the
    unpacked zip and the copied-out app are checked again before they
    are swapped in (F23): on a Mac, anything that writes into the staged
    app before Moonglow closes (a `.DS_Store`) would refuse the install,
    which needs a Mac to rule out. (S, needs a Mac and a Windows machine)
- **Windows signing:** the installer isn't code-signed, so SmartScreen
  warns. (S, needs a certificate)

## Questions asked, unanswered

Each waits on its reporter: with no word by 2026-11-12 (a month from
the owner's ruling of 2026-10-12) it is dropped, and said so in the
log. None waits now.

## Settled: as it is, by decision or by design

Nothing is left to do on these; they are kept so that a report of one
is not taken for a new gap.

- **An include's own `main`: Moonglow's entry point kept, and warned
  of** (the owner's word, 2026-11-26, "Compile: 1 (keep, warn)"; closing
  the builder's "41 changed" after Aurora's Build): a script that
  includes a file with a `void main()` of its own compiles, in Aurora
  8193.37, to the include's `main` (an `int StartingConditional()`
  script, or one that is only the `#include`; an include's
  `StartingConditional` likewise for a script with none), which in the
  game always fails as a condition (`engine_include_entry`). Moonglow
  compiles the script's own, or nothing for a script with none, and
  each such script is warned of in Compile All's log and `mg compile`
  (`mg_script`'s `included_entry`; `aurora_scripts::codegen_compiles_as_aurora_builds_it`
  holds the warned scripts to the ones Aurora builds otherwise).
  ([Scripts](manual/07-scripts.md), [findings](findings.md))

- **TGA textures compressed as read, PLT colored copies kept as RGBA8**
  (closing the performance audit's "TGA and PLT textures are
  uncompressed", 2026-11-25; kept off by the owner's word, 2026-11-25,
  shown the crops and the memory it would save): a PLT's layers meet in sharp edges between
  unrelated colors inside a 4×4 block, which BC blurs (up to 72 of 255
  in a picture of dyed armor, the trim and rivets dulled at 4×); a copy
  is made for each object's colors. The cost: an area of many creatures
  keeps its PLTs' memory (moonseabattle 156.7 MiB of textures, 45.2
  with them compressed). Compressing them without the loss would need a
  BC7 encoder. ([the frames note](research/notes_frames.md))

- **The previews' shadows: the model viewer's alone** (the owner's
  word, 2026-11-02): the model viewer's model casts the sun's shadow on
  a ground under it that takes only shadows (its Shadow switch, on by
  default); the pictures of the galleries, the palettes, a hovered
  row's preview and a tile's variants draw none, as they did (a
  creature or a dynamic placeable casts only onto tiles and static
  placeables in the game, and a ground under every picture would change
  them all). `the_shadow_switch_leaves_the_pictures_as_they_were` holds
  them pixel for pixel.
- **The grass is drawn across the whole area view, not faded with the
  camera's distance as in the game** (the owner's word, 2026-11-02): the
  client gives the grass's diffuse an alpha that falls with the tufts'
  distance from its camera, about `(32 − d) / 16` (0.94 at 17 m, 0.71 at
  20, 0.49 at 24: `batch6_grass_material`, [the findings](findings.md),
  "The grass's material"), its grass ending at its render distance; the
  area view's camera stands farther off (often 40 m and more), so the
  rule would leave most of the view without grass. Its places, sizes
  and count are Moonglow's own too (the field's mean color 0.1 of 255
  from the client's). Said in the manual's
  [differences](manual/12-differences.md).
- **The game client's clock's start** (`worldtimerTimeOfDay`; the
  shaders proposal's M1): Moonglow's clock starts at 0, by design: the
  client's starts from the machine it runs on, so no client's value is
  there to match. Read on 2026-11-02 in the client's code
  (`CWorldTimer`) and in runs with its clocks moved
  (`batch7_timer_origin`, `tools/nwclient/timeshift.c`; [the
  findings](findings.md), "The client's clock starts from the
  machine's"): its world timer is made as the game's services start (a
  second or so before the player enters the area) at 23:59:56 of a day
  of 5 minutes an hour (10,496,000 ms of 7,200,000) against
  `CLOCK_MONOTONIC`, and the module's minutes an hour, set as the module
  loads, change the day's length and not that offset: the timer is
  `(10,496,000 + 7,200,000 × ⌊the boot clock then / 7,200,000⌋ + the
  milliseconds since) mod the module's day`; the earlier batches' two
  values two hours apart were steps of the boot clock. A comparison
  reads the client's from its picture (`batch7_fountain_phase`).
  (settled in the deferred sprint, 2026-11-02)
- **The feminine talk table over NWSync** isn't published, as
  `nwn_nwsync_write` doesn't publish it, by design: the client never
  reads it for its own lookups. Read in single player (2026-10-29,
  `reading_feminine_talk_table`) and over NWSync (2026-11-02,
  `joined_nwsync_download` with `MG_NWSYNC_CLICK=1` and
  `MG_NWSYNC_FEMININE=1`: the feminine table published and downloaded
  beside the masculine one, the player a woman in the game): the client
  draws a line by StrRef from the masculine table for her, while the
  server's `GetStringByStrRef` with `GENDER_FEMALE` reads the feminine
  one from its own `tlk`. Getting there needed the server to keep its
  module loaded when the last player left: [the findings](findings.md),
  "A module reloaded when empty is advertised without NWSync".
  (settled in the deferred sprint, 2026-11-02)

- **The 2DA editor puts no row between others** (Delete Row takes any
  row away since 1.22.1), sorts no rows and edits no `DEFAULT:`, by
  decision: a row's number is what every other table, blueprint and
  script names it by, so a row put between others would move every row
  after it under what names them (settled in the deferred sprint,
  2026-11-01).

- **The custom palettes where a save differs from Aurora's, or sees no
  change** (settled in the deferred sprint, 2026-11-01; the rest of the
  entry was fixed then: the log). By choice: a save puts in order only
  the categories an entry went into and keeps the fields it doesn't
  know, where Aurora's puts every category in order and drops them, so
  that a palette no change reached is the same bytes (a commit shows
  only what the builder changed); a new entry for a talk-table name is
  its `STRREF`, as Aurora's Build writes it, where Aurora's Edit Copy
  writes the text; a module without a `palcus.itp` gets one made in
  full at a save, where Aurora writes the categories alone, empty (of
  use to no one until a Refresh). As in Aurora, by design: an entry
  made stale where Moonglow saw no blueprint change while the module
  was open (another tool while it was closed, a merge, an `mg` command
  without a game install, work recovered after a crash) stays until
  its blueprint next changes, Refresh Palette or Build (finding it would
  mean reading every blueprint at every save, which is what Refresh
  does); a module's own categories changed outside are taken in as
  they are; a hak's skeleton changed, or a hak's blueprints renamed or
  removed, leave the module's palette as its file is (Refresh Palette
  and Build make it anew; Remove from Palette takes one entry out); a
  blueprint of category 255 or of one the palette lacks is left out by
  Refresh Palette, as Aurora leaves it out. (El Grillo's and
  Jasperre's report, October 2026)
- **A project's plugin is allowed by a digest of its files, not by a
  signature** (the deferred list's "Plugins, beyond API 0.4", 2026-11-01):
  the digest says the plugin is the very one the builder allowed, which
  is what the question is about; who wrote it is the team's checkout,
  which the builder chose to open. Signing would need keys for every
  team and a way to trust them, for no more than the digest already
  holds the builder to. Kept with it, by the same decision: a no holds
  for the plugin's id in that project whatever it becomes (Manage
  Plugins allows it later); what its events were allowed (a folder, a
  yes) is kept by its digest, so a changed one asks again; `mg plugin
  run` runs the folder it is given, allowed or not (it is named on
  purpose); a project's plugin and one installed of its id share their
  settings of scope `user` (it stands in for that plugin, and the
  user's own values are the user's); a module kept in one file has no
  project, so no project plugins.

- **Which findings have fixes, and where** (the deferred list's
  "Plugins, beyond API 0.4", 2026-11-01): a fix is worked out when
  Verify runs and held to the resources being as Verify read them, so a
  change since asks for Verify again (a fix made on what nobody looked
  at would land elsewhere); its Undo now offers it again. The doctor's
  fixes are those whose answer is plain (`tag-unique`, `area-variables`,
  `var-runtime`, `static-transform`): `pc-faction`'s faction, a missing
  reference's resource, a model's or a tileset's repair are the
  builder's to choose, and a plugin's check may offer one that asks. Build
  Module's results are missing and unused resources, which have no fix
  but Mark as Used, there already; the tileset editor's faults are of a
  tileset being made, edited in its own fields. Fixes of different
  checks are made one at a time, or a check's all at once (Fix All).

- **What a resource's own events are not for** (the deferred list's
  "Plugins, beyond API 0.4", 2026-11-01): a save of what is no resource
  of the module (a hak's, a talk table's, a 2DA file's own editor) asks
  no plugin, whose events are of the module's resources; a script's Save
  puts its text into the module, and the module's save is asked; Build
  Module and `mg watch` write a build and a development folder, not the
  module; the custom palettes Moonglow makes again at a save are made
  from the blueprints a handler reads already. What a plugin makes
  itself fires none (or plugins would answer each other), and spawn
  points, the start location and an encounter's points are no placed
  objects. A handler of a placing, a paste or a making edits and logs:
  warning of or refusing what the user just did is the save's to do,
  where the Before saving window asks. A fix there is made one at a
  time, the save asked again on what it now is; "as it was" is as each
  file was last read or saved (a module kept in one file: as it was
  saved or opened, in this run), what the window can know. A handler is
  stopped between steps of its code: a call of the API is Rust code that
  runs to its end, and the time is looked at when it returns.

- **How a plugin's tests hold a job** (the deferred list's "Plugins,
  beyond API 0.4", 2026-11-01): a command's edits are held by the fields
  they leave, their count and the resources they change, not compared
  as an edit file (whose order and paths change with any refactoring of
  the plugin that makes the same module); a finding's message is matched
  whole, as a test of a check's words is meant to be; the game's data is
  the game `mg` finds, the one at a test's `root`, or the test's own
  `game/` folder, which is how a pipeline without the game tests what
  reads `ctx.game` (the game's files are never in a repository).

- **What a plugin's command is not offered on** (the deferred list's
  "Plugins, beyond API 0.4", 2026-11-01): a trigger's or an encounter's
  points and the start location are no selection in the window (a
  polygon is redrawn whole, Redraw Polygon; the start location is set
  where the area is right-clicked), so there is nothing of them to hand
  a plugin: it edits a trigger's `Geometry` through the trigger, and the
  start location through `module.ifo`. A conversation's line is not an
  entry of `ctx.selection` either: the conversation's tab menu has the
  commands for `dlg`. A palette's blueprint menu has no Properties to
  end it, so there the plugins' commands come last, by design.

- **A plugin's settings outside Undo, and a `path` it reads no file by**
  (the deferred list's "Plugins, beyond API 0.4", 2026-11-01): what the
  settings' form saves, and what a job sets, is not a step of Undo: the
  values are files outside the module (Moonglow's own, the project's
  `moonglow.toml` and `.moonglow/user.toml`), as Options are, and Save in
  the form, or the plugin's own command that sets one, is the asking (the
  form says beside each where it is kept; the log says what was set, and
  where). A `path` stays text to the plugin, which reads no file by it
  and still asks with `ctx.ui:open_file`: a value of the team's comes
  with the checkout, not from the user, and a plugin reading a file by
  it would read what the user never chose.

- **A geometry program beside a material's pair** (`customshaderGS` with
  `customshaderVS` and `FS`): the client runs it (one that emits nothing
  hid its card, `program_choice_in_the_client`, 2026-10-29); wgpu has no
  geometry stage, so Moonglow draws its own shader there and reports it.
  (Named alone, the client ignores it, and so does Moonglow since then.)

- **A creature's Equipment in a narrow window** (about 480 points, as
  a placed creature's Properties open beside the area) shows each slot's
  name over its item's beside a smaller icon, Equip over Remove; the
  longer slot names (Secondary Weapon or Shield, the creature weapons)
  are cut short there, whole in their tips. Kept: Aurora's inventory
  names no slot at all (a paper doll of icons, seen again in the
  capture of October 2026), so a name cut short with its tip says more
  than Aurora does, and the item's name, which matters more, stays
  whole in sight.

- **Seams between tiles:** drawn as instances, a sample on the seam of
  two tiles may show the other tile's floor (12 of 192 pictures, one to
  three pixels each): which of two floors that both cover a sample is
  drawn last was the order of the area's tiles, and is now the order of
  the draws. A mesh at a time (`Renderer::instancing` off) it is as it
  was.
- **Pictures for the rest:** waypoints have their flags as pictures;
  sounds, triggers, encounters and stores say what they are on hover, in
  a line or two (a store's and a sound's markers are the same for every
  one, and the others have no model).
- **Standard palette categories** can't be changed (the game's); only
  custom blueprints move between categories, as in Aurora.
- **Areas by name elsewhere:** areas are by name in the module tree, tab
  titles, Find Instance, the area transition's destinations and the
  references list; the log has ResRefs.
- **The Options tab** opens as wide as the window allows, where the
  other editors leave some of the area showing beside them; Enter is OK
  only while the pointer is over it.
- **Column headings** in forms' tables ("Base", "Total", "Sun", "Moon")
  keep the plain strong style; only section headings were made larger.
- **Scripts and conversations by name:** they have no names; the tree
  lists them by ResRef.
- **Copy… of a script** takes its compiled script along; its debug
  information (`.ndb`) is made when the copy is next compiled.
- **Deleting a prefab** isn't Edit › Undo's: the palette's Undo Delete
  puts back the one deleted last, until Moonglow is closed.
- **Compiling before export** needs the game's data (for nwscript):
  without it scripts go out with the compiled scripts they have, and the
  log says so.
- **Include files** aren't compiled on their own by Compile All, the
  build or Compile in the editor (they were, and one leaning on what its
  includer brings was reported as an error). Watched in Aurora since
  (Build › Build Module, Compile): it compiles every script, says
  nothing of one that only lacks `main`, and reports "Compile Error" for
  an include that leans on what its includer brings, as Moonglow did
  before this change. Skipping includes is Moonglow's own, kinder than
  Aurora and not like it, and kept by decision (2026-10-07); seen so
  again on 2026-10-28 (capture `script-compile`: Compile Error for three
  include files that don't compile alone, nothing for one that does). An include
  is still checked as you type, on its own, so one that leans on its
  includer shows an error there.
- **The grid seen from the area's edge:** red and plain as the view
  comes down (Aurora's is red from every side, and mostly lost in the
  ground from above), and drawn at the area's foot under raised ground.
  Looked at in Aurora on a raised plateau since, at a low pitch: its
  red lines show on top of the raised ground, at that ground's height,
  as Moonglow's do; none could be made out on the low ground in front
  (lost in the ground, as from above). Whether Aurora also draws lines
  at the area's foot under raised tiles could not be told.
- **A .mod beside a module folder on save**, as a builder says Aurora
  writes one for a module directory: on unless Options › General
  switches it off (the user's decision: a folder opened from elsewhere,
  a repository's source tree, gets a `.mod` written beside it unless it
  is off). A nasher project is left to Pack Target. Captured in Aurora
  since (`docs/findings.md`, A module directory beside its .mod): it
  writes the changed files into the folder and packs it into the `.mod`
  beside it, the old one kept as `.BackupMod`, as Moonglow does.
- **Clicking between tiles and objects:** a click on an object while
  tiles are selected goes to objects, and a double click on the ground
  to its tile. So a tile wholly under an object is selected by a box,
  not a click.
- **Behind things:** the rings and arrows are painted over the view, not
  hidden by what stands in front of them (as 3D editors draw theirs:
  left so).
- **A palette's rows are one line** (a builder: names with ResRefs and
  challenge ratings wrapped onto a second line in a narrow pane), cut
  short, whole under the pointer (the module tree scrolls sideways
  instead, as Aurora's does). Not an option.
- **The module tree and the object selected in the area:** the tree
  goes to it only where its area is opened out there; an area left
  closed stays closed. Asked for by the user (it was opened out and
  scrolled to on every selection, written to follow Aurora, whose own
  tree was not captured doing so). With several objects selected, or
  none, it stays where it is; there is no way to turn it off.
  (from builders' reports after 1.19.1)
- **The keypad's − and * in the conversation tree:** both − fold, and *
  typed either way (the keypad's, or Shift+8) opens everything under the
  line. egui 0.36.2 can't tell them apart: egui-winit maps `Minus` and
  `NumpadSubtract` both to `Key::Minus`, has no key for `NumpadMultiply`
  (the keypad's * comes only as the text `*`) and drops a key's location
  on the keyboard (`location: _` in its key event).
- **Names that fold alike, against Windows:** `Arboles`, `arboles` and
  `Árboles` are put in order by their text after the folded name ties
  (`mg_core::text::sort_key`), not as Windows' `CompareString` ties them
  (by accents, then case, at its second and third levels). Aurora's
  order of such names can't be watched here: it runs under Wine, whose
  `CompareString` is Wine's own collation, not Windows'. One order,
  the same whatever the names came in, is what the lists need. (Sacha's
  report, October 2026)
- **Escape in the conversation tree** hands its keys back, as the module
  tree's Escape does; Aurora's Escape does nothing in its tree (watched,
  2026-10-26). Moonglow's lists keep the keys from the area's view, which
  Aurora's tree has no counterpart of: this is the way to give them back
  without pointing at the area.
- **Backspace on macOS** deletes a conversation's line (Delete Line has
  the Mac's delete key, which is Backspace, besides Delete), where it goes
  to the line above on the other systems: a Mac laptop has no Delete key.
- **Bookmarks and folded lines across a change Moonglow didn't make** (a
  plugin's, the file changed outside it) follow their lines as they are
  found again: by each line's text and speaker and those of the lines
  owning it up to Root. Of lines alike in all that, the same number of them
  is taken in order, else only the one keeping its number; a line whose
  text the same change edited (and the lines under it) keeps its number
  only where its kind has as many lines as before. One not found goes, and the debug log says so. The
  editor remembers its own last 256 deletes to follow them exactly; a
  change further back is followed by finding the lines again, so the
  limit stays. Of two deletes that leave the same conversation (either of
  two lines alike), undo and redo follow the one done last.
- **Page keys in a NUI window's preview:** every text field of several
  lines has Page Down and Page Up (`widgets::page_keys`) but those of a
  NUI window tried in the preview (Interact), drawn as the game's: the
  game's own NUI text field of several lines takes neither (its caret
  stays, its text doesn't scroll; the arrow keys move it), tried in the
  client (`docs/findings.md`, "NUI text fields have no page keys").
  (from Sacha's report, 2026-10-21)
- **A script with both kinds of line end, changed a lot:** every line
  not changed keeps its own line end, by a diff of the lines; a run of
  changed lines too big for one table of the diff (2,000 lines each
  side) is cut at the lines each side has once, and the lines between
  are diffed the same way. Lines in a part with no line found once on
  each side and too big for the table, or past the diff's bound of work
  (16 million cells, so it takes a bounded time), are taken as changed
  in place, keeping the ends where they were. Two lines swapped count as
  one kept and one added (the added one takes the text's line end).
  (Jasperre's report, 2026-10-21)
- **Line ends of a line typed into a text of one line** are Aurora's by
  the kind of field (`docs/findings.md`, "Line ends"): plain in a
  localized string (descriptions, names, a conversation line's text,
  journal entries), CRLF in a `CExoString` (comments) and in scripts; a
  text that has line ends gives a line added its own (most of its
  lines'), and a one-line field (a Tag) keeps those its text has. Aurora
  also makes every line end of a blueprint's comment CRLF when it saves
  the blueprint, typed in or not; Moonglow keeps the line ends of what
  is not typed in, so that an untouched text comes back byte for byte.
  A hak's description (the archive's, not a GFF field; Aurora writes no
  hak) gets CRLF. (Jasperre's report, 2026-10-21)
- **Verify's `tag-unique` covers the project's patterns only:** no check
  of every duplicate tag (scenery shares tags by design). (S) (Settled in the triage of 2026-10-28.)
- **Verify's `var-name` with `undescribed` looks at the names only:** a
  variable described nowhere and not named as the project's is not
  said; variables set by scripts are not seen (Verify reads the module's
  resources). (S) (Settled in the triage of 2026-10-28.)
- **Overlapping tufts blend in another order:** a tile's tufts are drawn
  in no order of place (so that the first of them are spread over the
  tile, for the levels), not face by face: the same tufts in the same
  places, but where two overlap their fringes blend the other way round
  (1.3 % of a close view's pixels differ by more than 12 of 255). (S) (Settled in the triage of 2026-10-28.)
- **The camera's height** is each view's and lasts until its tab closes.
  (Lighting and Sound Ranges are kept: a view opens as they were last
  left, and one already open keeps its own.) (S) (Settled in the triage of 2026-10-28.)
- **The start location's marker** is still drawn over the view, not
  hidden by what is in front of it, as the selection's boxes and the
  outlines of triggers and encounters are by design. (S) (Settled in the triage of 2026-10-28.)
- **Plugins writing into a folder, and acting on events** (plugin API
  0.3): `folder:write` into a folder the user chose, with their leave,
  and `mg.on` for a module opened or saved. Not done: a file goes into
  the chosen folder itself, not into a folder inside it (a project's
  `src/` tree of folders is chosen one folder at a time); no copy of a
  replaced file is kept; a script's own Save is no save of the module
  and fires none (a resource's own events, around a module's save, came
  with API 0.4: above); what an event was allowed is kept between
  runs by plugin and version (the settings' `plugin_grants`; a
  plugin's own settings came with API 0.4, above); `mg plugin run
  --event` runs an event by hand, and `mg watch` and `mg build` fire
  none; an event's handler that asks a form asks it at every save. (S
  each) (Settled in the triage of 2026-10-28.)
- **The Spells page against Aurora** (watched in Aurora: see
  [findings](findings.md), "Aurora's Spells page"; Moonglow has its
  Meta-Magic list, casting levels and counts by metamagic now). Left
  different on purpose: the levels show how many the class may have
  ("Level 1: 2 of 3") where Aurora's summary has the bare level, the
  spells known or prepared are beside the list rather than a count on
  every row, and a spell whose casting level would pass 9 can't be
  prepared (Aurora lists it at 10). Not seen: what Aurora writes for a
  spell prepared with two metamagic feats, which its list can't choose.
  (S) (from builders' reports after 1.16.2) (Settled in the triage of 2026-10-28.)
- **Claims are not checked against who commits:** the hand-back's CI
  check (a pull request touching areas its author didn't claim) is the
  team's script's; Moonglow knows the builder by the code they set, not
  by their git identity. A code is not reserved to one builder: one that
  already holds claims is only warned of when set (Builder Code, `mg
  builder-code`). (S) (Settled in the triage of 2026-10-28.)
- **The claims table is a 2DA only** (the hand-back proposal's own
  choice): a TOML or CSV table isn't read; a codepage other than UTF-8
  isn't considered (the table is ASCII in practice). (S) (Settled in the triage of 2026-10-28.)
- **Verify's merged-area checks read the table only:** an area merged
  with no Tag recorded is held to its ResRef alone; the hand-back's
  check against the merge base (a Tag changed since `main`) is the
  team's CI's, as git is not Moonglow's. (S) (Settled in the triage of 2026-10-28.)
- **The conversation tree's letters on Windows:** Aurora was watched
  under Wine, whose tree control is Wine's own (findings, "The
  conversation tree's letters and Escape"): there the same letter typed
  again quickly stayed on the first row beginning with it, and how long a
  pause starts the letters over is Wine's. Moonglow goes round the rows
  beginning with the letter and starts over after a second, as asked;
  Aurora on Windows wasn't watched doing either. (S) (Settled in the triage of 2026-10-28.)
- **Inserting or removing lines in the middle** isn't offered: it
  renumbers every later line, and their users would have to be
  renumbered with it (Find References finds them now: the entry above;
  the renumbering is not written). (M) (Settled in the triage of 2026-10-28.)
- **`mg 2da-diff` and `mg tlk-diff`** compare and merge by row number
  and column name (line number for a talk table). Not done: an edit
  file takes the table whole (`set_resource`), since edit files have no
  change for one cell; a row is its number, so a row inserted in the
  middle reads as every row after it changed, and two rows added under
  one number are a conflict to be settled by hand (no renumbering, which
  would need every reference to the row changed too); a talk table's
  merge takes a line whole (its text, sound and length together); a
  feminine table is compared as a file of its own. (M, M, S, S) (Settled in the triage of 2026-10-28.)
- **The Aurora UI inventory's hand-made parts:** its inputs are made
  from the installed game now (`tools/aurora/uiinv/inputs.py`), but
  twelve StrRef table offsets and the mappings in `mapping.py` were
  settled by eye for one build of the toolset (26c6e573, 89.8193.37):
  another build needs them looked at again (the generator warns). (S,
  when the toolset changes) (the documentation audit of 2026-10-12) (Settled in the triage of 2026-10-28.)
- **A brush's right click is the tile menu,** not Aurora's putting the
  brush down (Escape or the menu's Put Down does); Select Tiles is in
  the Tiles palette too, but isn't turned on by choosing the Tiles
  palette (Aurora's Terrain tab doesn't, and objects are still worked on
  with it shown). Out of Select Tiles, a right click on the ground with
  no object there starts the area's menu with the tile's items, whatever
  objects are selected; Delete there is still the selection's. (S)
  (from a builder's tile feedback, October 2026) (Settled in the triage of 2026-10-28.)
- **A click is on the model, not its box:** an object with a model is
  picked where the pointer's ray meets its triangles (at rest: a creature
  mid-stride is picked where it stands still), so a click through the
  empty part of a wide box reaches the trigger or the object behind;
  where the ray meets nothing so, the nearest box passed through is
  taken, so a thin post is still clicked beside. Aurora's own picking
  wasn't measured. Things without a mesh (markers, an emitter alone) are
  picked by their boxes still. (S) (from builders' reports after 1.14.1) (Settled in the triage of 2026-10-28.)
- **A refused tile step out of Select Tiles and with no brush** (the
  area's own right-click menu on the ground) flashes the tiles red and
  says why in the log, not on the status line, which shows the objects'
  line then (a notice there would stay after the brush or the mode that
  set it is gone). (S) (from Jasperre's tile feedback, October 2026) (Settled in the triage of 2026-10-28.)
- **An area full of far-reaching lights:** the Txpple pack's light-node
  placeables have multipliers of 6 to 10, so a "15 m" one reaches 300 m
  (as in the game), and every mesh of an area of them is lit by up to 32
  lights, as the client lights it. In The Domains of Dread's woods
  (`game_shaders_frames_at_144`, `FRAME_TDOD_LAMPS`) seen close with the
  game's shaders: 2 lights, 4.7 ms of CPU and 2.3 of GPU a frame; 20 of
  those placeables, 4.3 and 3.0 (within the budget); 100, 7.0 and 8.0
  (over it: each fragment adds 32 lights). A light's reach isn't
  shortened for the frames: what a light past the 32 adds is the
  client's question too. (S–M) (Settled in the triage of 2026-10-28.)
- **An object's light nodes, as it stands:** the light nodes of an
  object's models (its base's and its parts', a held torch's) are taken
  where they are at the start of its idle animation, with the colour,
  radius and multiplier keyed there: a light an animation moves, flickers
  or turns on and off (a lamp's "on" and "off" states keying its light's
  colour) is drawn as it is at that moment. A part's light nodes are taken
  in the part's rest pose. The model viewer's **Lights** shows the base
  model's light nodes (animated), the appearance's light and, since
  2026-11-10, its parts' (a tail's effect's) as they stand. Since
  2026-10-30 the area view plays a dynamic placeable's base model's
  light keys (`point_lights_at`), which Custom Light's flicker
  (2026-11-25) shows by. (S) (from Troubletcat's and Heed's reports, October 2026) (Settled in the triage of 2026-10-28.)
- **Moonglow draws the rooms the player can't see:** in an interior the
  client draws only the tiles the player's tile sees (the tileset's
  visibility); the area view draws every tile, as Aurora does. Comparing
  pictures with the client's, the prison's main floor and a `tdm01` mine
  were 13 to 15 of 255 away for this, their lighting alike. (S) (Settled in the triage of 2026-10-28.)
- **Tile appearance and rotation:** Next and Previous Tile Appearance
  step through the tiles that fit by their index in the tileset (Aurora
  has no such step to compare with; its Shift + right click cycles the
  tiles and their turns together, as Refine Tile does); the new tile
  gets fresh lights, as a painted one does, not the old tile's colors.
  Rotate Tile turns a tile only where the same tile still fits, and
  refuses elsewhere: it doesn't turn the terrain under it (that would
  repaint the neighbors), and whether Aurora or the engine has a turn
  for a placed tile was not looked into (the engine draws any
  `Tile_Orientation`; a tile turned against its neighbors' terrain
  would show seams). A tile at any turn, whether it fits or not, is put
  in with Tile Properties' All tiles and its Turn (issue #29); the steps
  themselves stay fit-only. (S) (from a builder's tile feedback,
  October 2026) (Settled in the triage of 2026-10-28.)
- **Tile Properties' lights and loops from the model:** a model not
  found (or not readable) falls back on the tileset's flags, which say
  every light and loop; a tile model's `animloop` animations from a
  supermodel are not looked for (the game's tile models have none).
  (Its discard question is gone since Tile Properties became the Right
  Dock's panel, M4's U5: there is no OK, Cancel or close to ask about.)
  (S) (from Jasperre's tile feedback, October 2026) (Settled in the
  triage of 2026-10-28; restated by M4's U5 review, 2026-11-15.)
- **Uniforms a script sets:** a custom shader's uniform that a module's
  scripts set in the game (`SetMaterialShaderUniformInt`,
  `SetMaterialShaderUniformVec4` on an object that isn't static) shows
  the MTR's value in the area view, which runs no scripts. (S) (Heed's
  report, October 2026) (Settled in the triage of 2026-10-28.)
- **Which tiles are a group's is worked out, not recorded:** the area's
  file doesn't say what a group placed, so a tile is a group's when the
  tileset lists it in a group (`TileIndex::is_grouped`), and the group is
  whole where its tiles stand at their places, turned and raised alike
  (`TileIndex::group_at`). What is left of a group cut by an earlier
  edit (or by Aurora's Delete) is still locked tile by tile: only the
  Eraser (which takes such a tile alone) or the option changes it. A
  tile listed in two groups is taken for the first that stands whole
  there. Groups placed with Aurora's Delete applied to them were not
  surveyed. (S) (from Jasperre's tile feedback, October 2026) (Settled in the triage of 2026-10-28.)
- **Item costs:** Additional Cost keeps Aurora's limit. The game prices a
  created item itself, as Moonglow computes the stored Cost (see
  [Findings](findings.md#created-items-are-priced-anew)), so nothing seemed
  to need it. (S to check) (Settled in the triage of 2026-10-28.)
- **Animations step 25 times a second** (`ANIMATION_STEP`, 40 ms) while
  only they move; before, they stepped with every frame the window drew
  (20 a second left alone, 144 with the pointer moving). A setting, or
  every frame once the picture is cheap enough. (S) (Settled in the triage of 2026-10-28.)
- **Tokens by language:** a language's own tokens are read from the
  install's `lang/<code>/data/ovr/stringtokens.2da` (Polish has one);
  a hak's or module's `stringtokens.2da` isn't per language. Token…
  puts the token where the caret was last in the text (in place of a
  selection), or at the end if it was never there. (S) (Settled in the triage of 2026-10-28.)
- **Locks kept apart in projects only:** a module folder still writes
  `MG_Locked` into its area files (binary GFF, so no line diff), and a
  `.mod` keeps it as before. A project's locks live on one machine
  (Moonglow's data folder), not in the checkout; an object moved by
  another program or `mg` loses its lock where it can't be found again
  (by list place, blueprint, tag and position). (S) (Settled in the triage of 2026-10-28.)
- **Interact does not run the window** (stated in the manual): handlers
  are NWScript and run only in the game, so event timing, and the
  game's own layout solver, are seen only in Test Module. (L, if at
  all) (Settled in the triage of 2026-10-28.)
- **Nothing short of the game proves a window** (`apps/mg/src/nui.rs`,
  `crates/mg-nui/src/validate.rs`): `mg nui validate` and `generate`,
  like the checks and Build in the editor, check the document and
  compile its scripts; that the game builds the window and draws it as
  previewed is seen only in the client (Test Module). (L, if at all) (Settled in the triage of 2026-10-28.)
- **Resizing leaves bound and template sizes** (stated in the manual;
  `crates/mg-ui/src/nui_view/design.rs`): a resize gesture does not
  replace a dimension that is a bind, and a list template's cell gets
  its width in the cell's settings or the JUI source, not by a handle.
  (S) (Settled in the triage of 2026-10-28.)
- **Portraits, music and ambient sound** in folders (not haks) aren't
  gathered; they go in a hak first, as with `nwn_nwsync_write`. (S) (Settled in the triage of 2026-10-28.)
- **A picture of a model that emits** is drawn four seconds in, framed
  with its particles (a flame is small under its smoke). (S)
  (from builders' reports after 1.10.1) (Settled in the triage of 2026-10-28.)
- **Inventory lists:** a store page or inventory lays out the rows in
  sight and builds their icons only (the persistent world's store, 449
  items on its first page: 30 ms to show where it took 0.4 s, 0.5 ms a
  frame where it took 6.3). A row's height is its icon's, known once
  it has been in sight: until then the rows out of sight take the
  middle height of those drawn, so the scroll bar's size is a guess
  that changes as the rows are seen, and a jump of the scroll bar lands
  among rows by those guesses (the rows in sight are drawn as before).
  The list still goes through every row a frame, to count the room of
  those out of sight. (S) (Settled in the triage of 2026-10-28.)
- **Aurora's keys Moonglow lacks:** Plot Wizard (Ctrl+Alt+P; no plot
  wizard), Refresh (F5: it is the script editor's Bookmark here, and a
  key of the whole window would take it from there; Reload Resources
  can be given a key in Options), and the script editor's Save As
  (Ctrl+Alt+S, now New Script's). (S) (Settled in the triage of 2026-10-28.)
- **Not every key:** text editing, Escape and Enter, Copy, Cut and Paste
  (Ctrl+C, X, V and the classic Ctrl+Insert, Shift+Delete, Shift+Insert),
  Delete in the area view, the script editor's numbered bookmarks, and
  mouse bindings (drags, Ctrl+click) are fixed. (S) (Settled in the triage of 2026-10-28.)
- **A cut-out lit from behind, by Moonglow's own shader:** the game's
  programs at High Quality light a cut-out's kept texels below opaque
  (its edges) through from behind (`inc_lighting`'s translucence: the
  light behind times one minus the alpha), and the client draws the
  light line that makes along a backlit cut edge (the Domains of Dread's
  dire wolf, `dd_n2e_wolfdire`, in the area view; `creatures_look` with
  the wolf, gone with the translucence defined away; held by
  `a_backlit_cut_out_s_edge_is_lit_through_by_the_game_s_programs`).
  Moonglow's own shader lights it so too since 2026-11-02 (a cut-out's
  alpha, the mesh's times the texel's, weighs the light from behind
  though it is drawn solid: `batch6_cut_out_lighter`'s disc of mesh
  `alpha 0.5` 156 of 255 as the client's, where it was 126). With The Domains of
  Dread's `inc_lighting` (a wrapped diffuse, N·L never below 0) there is
  no translucence; what shows there is the fur card lit otherwise than
  the normal-mapped body under it, which the client draws too. Also left:
  the game's programs are made for High Quality whatever the user's
  `settings.tml` says (at Performance, the game's default, there is no
  translucence nor light softening). (S)
  (from the Domains of Dread's report on the wolf's edges in the area
  view, 2026-10-22; seen in the client) (Settled in the triage of 2026-10-28.)
- **Compiled on saving the module:** with Automatically Compile Scripts
  on Save, File › Save compiles the scripts whose text it stores (only a
  script's own Save did; a builder found scripts left uncompiled). A
  script saved earlier and never compiled is not caught by it (Compile
  All is); one the external editor saved is compiled when its text is
  next stored here. (S) (from builders' reports after 1.16.1) (Settled in the triage of 2026-10-28.)
- **Find In Currently Open Scripts** searches the scripts whose editors
  hold their text, which includes one closed with text typed and not
  saved. (S) (from builders' reports after 1.16.2) (Settled in the triage of 2026-10-28.)
- **Programs Moonglow starts:** the game (Test Module) and the external
  script editor are waited for on a thread each, so they leave no
  "defunct" process; checked on Linux only. (S) (Settled in the triage of 2026-10-28.)
- **The sample is fixed:** an everyday run sweeps the same sixth of the
  modules and tenth of the resources each time; what only the rest
  holds is seen before a release. A sample that moves with the date
  would cover everything over a week, at the price of failures that
  come and go. (S) (Settled in the triage of 2026-10-28.)
- **Placed objects posed again on each redrawn frame**, static ones too,
  and the animation's node maps made again per pose: 0.3 ms of a frame
  for 416 objects, so left. (S) (Settled in the triage of 2026-10-28.)
- **The Flatpak and the update check:** its sandbox has no network
  (`finish-args` lacks `--share=network`), so Help › Check for Updates…
  in the Flatpak asks nothing and says to update it with Flatpak (the
  owner's choice: no network asked for one request). Builds from source (`cargo run`, a distribution's
  own package) aren't told apart: they check as the AppImage does. (S)
  (GitHub issue 28) (Settled in the triage of 2026-10-28.)
- **Edit on a script that doesn't exist** makes it and says so in the
  log; Aurora shows a message box first ("Resource not found. Creating
  new script."). A mistyped name makes a script (Undo takes it back). (S) (Settled in the triage of 2026-10-28.)
- **The ring over another object:** a click on the ring picks the
  object under it, but a drag from there turns the selection: an object
  under the ring's line can't be dragged from there while the selection
  lasts. (S) (Settled in the triage of 2026-10-28.)
- **Minimap pictures** for a tileset without them are made by the
  tileset editor's Render Minimap Pictures (an area's Export Minimap
  writes the pictures the tileset has): they have Moonglow's renderer's
  look (lit models, as in
  the area view), not the game's painted look; and models must be
  readable (beside the `.set` or in the game data). (S–M) (Settled in the triage of 2026-10-28.)
- **The terrain brush's cancel** leaves out the right click
  (`terrain_mode.rs:543`), where five others take it: a right click
  turns a group there. (S) (Settled in the triage of 2026-10-28.)
- **No one question for every tab with unsaved work** (GitHub issue 15's
  tabs by the keyboard): closing tabs together asks nothing of its own;
  only a hak's and a tileset's editors have work of their own to lose,
  and each asks for itself as it closes. (S) (Settled in the triage of 2026-10-28.)
- **`mg serve --restart-on-change`** looks at a file's size and time
  twice a second, and at a folder six levels deep (no file-system
  notifications). (S) (Settled in the triage of 2026-10-28.)
- **`mg serve` and Beamdog's master server:** the server looks the
  master server up at its start even when not listed (`-publicserver
  0`): `mg serve` does not cut its network off, which would take a
  sandbox of its own. (S) (Settled in the triage of 2026-10-28.)
- **An external compiler and what is remembered:** with an external
  compiler chosen in the window, the window and `mg compile` (always
  the built-in compiler) do not share what they remember of the
  scripts they compiled: it holds for one compiler. (S) (Settled in the triage of 2026-10-28.)
- **`nwnsc` and the haks' scripts:** scripts in haks reach
  `nwn_script_comp` through `{haks}` (its `--erfs`); nwnsc has no such
  option. (S) (Settled in the triage of 2026-10-28.)
- **`mg areas`, what it doesn't set:** names, tags and comments, which
  are each area's own (Edit Areas Together doesn't offer them either;
  `mg replace` changes text in names; a name in one language would
  need a language to be given), nor list fields one by one (an ARE's
  tiles and `Expansion_List`: a tile's own lights aren't set;
  variables are `--var`'s, in the GIT where the game reads them). Copy
  This Page to Every Area (Visual's) gives an area of another
  scheme the first's tiles' lights picked anew; an area of the same
  scheme keeps its own, and the first area's own picks aren't copied
  tile by tile (the areas' tiles differ). (S) (Settled in the triage of 2026-10-28.)
- **A plugin's talk table and its feminine table** are two files, as the
  game keeps them (`dialogf.tlk`, `NAMEf.tlk`): `mg.tlk` reads and
  writes each as a talk table of its own, and nothing pairs a line with
  its feminine line. (S) (Settled with the rest of "Tables for plugins",
  done in the deferred sprint of 2026-10-28: it is the game's format.)
- **The plugin API's `place_group` ignores Options › Area's freed group
  tiles:** it keeps its documented way (over part of another group, that
  one goes whole), so a plugin's strokes keep groups whole whatever the
  builder's option. (Settled 2026-10-29, when the window's freed strokes
  were checked in the engine.)
- **A mesh reading the framebuffer's copy without `sample_framebuffer`
  gets this frame's copy, not the frame before's picture:** in the
  client a solid or `transparency 1` mesh whose program reads
  `texFBColor` reads the frame before's finished picture (its additive
  meshes in it: `reading_copy_order`, 2026-11-26), where Moonglow gives
  it the copy the water reads, made after the see-through meshes' solid
  parts. Not built: no program the game or The Domains of Dread draws
  reads it so (shiny water and `sample_framebuffer` meshes read this
  frame's copy, particles a depth copy without the water, as Moonglow's
  do), and it would take a second pair of copies the size of the view
  (40 MB at 3440×1440, 6 MB at 1024×768: a color and an `R32Float`
  depth texture of 4 bytes a pixel each) for every view, a
  copy a frame, and still nothing for a picture drawn once (a thumbnail,
  a preview), which has no frame before. (Settled with the deferred
  entries "The framebuffer's copies" and The Domains of Dread's
  fountain, 2026-11-26; left unbuilt: the owner's word, 2026-11-26.)
- **What stays with Moonglow's own shader with the Shaders switch on:**
  the sky's fade, unlit markers, shiny water without the targets'
  framebuffer copies (thumbnails, previews), textures that only ripple
  (a procedure the engine runs on the CPU) and skins of more than 64
  bones (the game's own `MAX_NUM_BONES`). (M1, 2026-10-21)
- **Keyholing is compiled out** (`KEYHOLING_ENABLED 0`) of the game's
  programs: the editor has no player. (M1)
- **The game's uniforms the editor has no source for** are the game's
  defaults or zero: `vColor`, `vStaticLightDir` (`staticLighting` 0,
  as the client has it at fragment lighting), the weather (clear: the
  game rolls it from the ARE's chances as it runs), wind sources,
  targeting, the scriptable floats and ints, and the scriptable vectors
  but `scriptableVec1` of an area with The Domains of Dread's
  `tdod_wallpaper` (`mg_rules::tint`, 2026-11-06); the model viewer and previews have no
  module (noon, two minutes an hour); `cameraViewAngle` is the editor's
  field of view. (M1)
- **A program the renderer can't take falls back** to Moonglow's own
  shader, reported once (the log, `Renderer::game_shader_failures`, the
  log pane): more than one uniform block a stage, storage buffers, 3D or
  array textures, more than 16 textures a stage. (M1)
- **An instance drawn see-through** (a blueprint about to be placed, a
  tile preview) draws with Moonglow's own shader even when its material
  names a pair: the ghost is the editor's, which the game has none of.
  (M2)
- **`texture8`-`texture15` of a material are white** to its programs
  (Moonglow keeps eight slots): no shader of TDoD's reads past
  `texUnit6`, and none of its 18,146 materials, nor the game's 7, names
  one. (M2, checked in M4)
- **The two-part see-through split cuts a custom shader's alpha at
  0.95**, as it does any mesh's: every fragment program of The Domains
  of Dread writes the stock opacity (`fs_plt_hair` its texture's alpha),
  none an alpha meant otherwise (read 2026-10-29). (M2)
- **The model viewer's area for the shaders is outdoors without wind**
  (`areaFlags` 0): it has no area, so an interior-only effect (TDoD's
  roofs) shows as outdoors there. (M2)
- **Moonglow's own shader draws as the client does** since M4 (both
  paths): a texture's `bumpyshinytexture` reflects nothing (water keeps
  it for Moonglow's own water); maps named without a render hint are not
  read; a DXT1's transparent texels are black, opaque; a 32-bit TGA all
  opaque is solid; every lit mesh reflects the area's map where it has
  none of its own; `default` in appearance.2da's `ENVMAP` is the area's
  map (`chrome1` without an area), in placeables.2da's `Reflection`
  `chrome1`. (M4, each read from the client)
- **A pair drawn both with and without `NO_DISCARD`** is compiled twice
  (TDoD: 26 programs, 22 before): the cost of the engine's choice made
  per mesh. (M4)
- **The shadows in two qualities** ([notes](research/notes_shaders.md)
  §G.2): Fast (the default), a shadow map standing in for the game's
  stencil volumes, its edges a texel of the map soft (stepped up close
  in a large area seen to its far side); and High (Options › Area ›
  Shadows quality, and the Shadows button's menu), the game's own
  stencil shadow volumes, as the owner asked on 2026-10-30. Settled
  2026-10-28 as the map alone; rewritten 2026-10-30 for both. In
  either, a caster that isn't closed (an edge of three faces or more)
  casts its faces' shadow, no spike, as the client (89.8193.37) drew
  the wiki's "finhead".
- **Lighting off against Aurora:** Aurora's Use Area Lighting off draws
  the textures unshaded at full brightness (no side darker than another;
  shadows still drawn; captured 2026-10-29, `target/test-output` shots
  of the wave 1 probe); Moonglow's working light is white and even with
  a little from the sun's side, so that shapes read: close enough, kept.
- **Static and the transform elsewhere:** ticking Static takes a placed
  placeable's visual transform off in its Properties and Make Static, as
  Aurora does; `mg set`, `mg apply`, a plugin and Update Instances set
  fields as given and keep an object's transform, and Verify warns of a
  static object with one (`--fix static-transform`). Aurora drops a
  visual transform from every blueprint its Edit and OK writes, static
  or not, and from a static placed placeable at every area save
  (captured 2026-10-29, `captures/wave1/static-bp.mod`,
  `pal2-vtdyn.mod`, `area-saved.mod`): Moonglow keeps what files have
  (lossless editing).
- **Delete takes a whole group by default,** where Aurora's Delete takes
  the tile alone and leaves the rest standing (the option gives
  Aurora's). Pasted tiles over a group are refused, as Aurora refuses
  them (captured 2026-10-29: "Pasting tiles to current area failed.
  There was a feature or a group within the region…").
- **The areas' sweeps walked five times:** the corpus tests `areas`,
  `hlsl`, `minimaps`, `object_walkmeshes` and `tiles` each walk every
  module's areas (`area_common::each_module`), sharing the module's
  set-up, not the walk, as the renders hold the GPU and the others read
  each area differently; no walk of the five together. (Settled by the
  owner, 2026-10-29.)
- **Undo chosen in Quick Open, with a script in front,** is the module's
  (the field has the keys), where Edit › Undo, the Top Bar's Undo and
  Ctrl+Z are the script's while its editor has the keyboard. Ctrl+Z typed
  in the field is the field's own (M3 T8's review). (a builder's report on
  1.25.1, of the Command Palette Quick Open took the place of in M3 T8)
  (Settled by the owner, 2026-10-29.)
- **A module's files changed outside, read on a thread of their own**
  (`mg_module::outside`), leave the files' memory in that thread's
  allocator arena: the process keeps about 200 MB more after a 970-file
  pull on Linux (25 MB with `MALLOC_ARENA_MAX=1`; it is reused, not
  leaked). (Settled by the owner, 2026-10-29.)
- **Advanced Search's Appearance** covers the 2DA rows a field names
  (creatures, placeables, doors, waypoints, items' base types); items'
  model parts and colors, portraits and sound sets are found through
  Any field. (Settled by the owner, 2026-10-29.)
- **A module's categories in its own order:** the palette and the
  `palcus.itp` built on save keep the order a module's own skeleton has
  (a builder's arrangement, kept until Aurora saves the module). Aurora
  shows every level by name whatever the files' order and writes the
  `palcus.itp` back in name order on Save As (captured 2026-10-29,
  `captures/wave1/pal2-saveas.mod`); Moonglow keeps its own by the
  owner's decision. (Settled by the owner, 2026-10-29.)
- **A retired key's line keeps its text** in the table `mg tlk-alloc`
  writes (a key taken out of the names file): a number saved in a
  character or an item still shows what it said, not an empty line, and
  the line stays the key's for when it is named again. Said in the
  manual's Strings by name. (Settled by the owner, 2026-10-29.)
- **A talk table's JSON is one table's** (`mg tlk-export`, the talk
  table editor's Export JSON), as `nwn_tlk`'s is and a nasher repository
  keeps it: the feminine table is a file of its own there, exported on
  its own (`mg tlk-export TABLEf.tlk`); the CSV carries both. (Settled
  by the owner, 2026-10-29.)
- **Marker models unlit:** stores, sounds and waypoints use the game's
  marker models in their materials' colors, unlit, so that their colors
  tell apart in a dark area. Aurora lights them as any model (captured
  2026-10-29 in the probe made always dark: a waypoint's and a store's
  yellow arrow from 243, 216, 0 by day to 76, 100, 0); Moonglow keeps
  its own by the owner's decision. (Settled by the owner, 2026-10-29.)
- **Alt for the camera and the selection:** Alt + right drag turns the
  selection and Alt + middle drag moves the camera; on a desktop that
  takes Alt + drag, the camera moves by Ctrl + drag and the selection is
  raised by Shift + drag of its blue arrow and turned by its ring or Q
  and E. Aurora turns the selection with Shift + right drag, which turns
  the camera in Moonglow (the handles shown), and there is no key for
  the camera's focus (Alt + middle click): Moonglow keeps its own design
  by the owner's decision. (from builders' reports after 1.10.1)
  (Settled by the owner, 2026-10-29.)
- **Tile Properties' Defaults** sets the lighting scheme's first colors
  and every loop. Aurora's, compared on Interior Evil (2026-10-29,
  `captures/wave1/tile-defaults.mod`), changed nothing shown; Moonglow
  keeps its own by the owner's decision. (from the research notes)
  (Settled by the owner, 2026-10-29.)
- **Exact tile's reach** (issue #29): the Gallery offers a terrain's and
  a crosser's tiles, not a group's pieces (one goes in alone through
  Tile Properties' All tiles); an exact tile takes the replaced tile's
  height; a corner it disagrees on with a neighbor is read as the tile
  to the north or east has it (`Lattice::from_tiles`). The client draws
  such a seam as the area view does (`reading_exact_tile_seams`,
  2026-10-29); the Gallery's reach and the corner rule are Moonglow's
  own design. (Settled by the owner, 2026-10-29.)
- **Tile Height and what stands on a tile** (issue #29): Raise/Lower's
  corner strokes move no object, and the control's ±1024 is Moonglow's
  reading bound, not the engine's; an object over a tile's square goes
  up or down with it, as the server and the client put it
  (`a_tile_s_own_height_stands_where_the_engine_puts_it`,
  `reading_tile_heights`, 2026-10-29). (Settled by the owner,
  2026-10-29.)
- **Escape during a drag** drops it (the drag's other tools, brushes and
  pastes, drop on Escape too). Aurora's ignores it: Escape pressed before
  the button is let go changes nothing, and the object is put down where
  it was dragged (captured 2026-10-29, `captures/wave1/esc-drag.mod`);
  Moonglow keeps its own by the owner's decision. (Settled by the owner,
  2026-10-29.)
- **What a module's tabs don't open again, by design:** a hak never
  saved and a 2DA in one, and a 2DA's Edit a Copy not saved yet (no
  file to open again); a camera moved since the tabs were last written,
  after a crash (they are written when they change, a window is moved,
  the module closes or Moonglow quits, not as the camera moves); a
  placed object moved outside Moonglow with another of its tag and
  blueprint added (neither is taken for it); a kept Options tab making
  its draft as Tools › Options does, which looks for the game's install
  (Steam's library files) as the module opens. (a builder's report,
  October 2026) (Settled by the owner, 2026-10-29.)
- **The round trip** (`mg roundtrip`, Build › Round-Trip Check) compares
  a GFF written anew by its fields, not its bytes: Moonglow lays a GFF
  out its own way, as every edit does. It doesn't ask the game or Aurora
  (the engine and Aurora tests do, on the game's own modules), and the
  formats it doesn't write anew (scripts, models) are held to their
  bytes by the saved copy. (Settled by the owner, 2026-10-29.)
- **Described variables (C2): an object's or a location's value is not
  described** (The Domains of Dread's builder proposal): what a file
  holds of one is an object's id or a place in an area, which no list,
  range or default describes; `values`, `min`, `max` and `default` are
  refused for them, said by line. (Settled by the owner, 2026-10-29.)
- **Reset Factions to Parent Factions in Export** starts unticked.
  Aurora's starts ticked (`xbFactionReset`, the parity inventory and the
  capture of Aurora's Export, 2026-10-29); Moonglow keeps its own by the
  owner's decision. (Settled by the owner, 2026-10-29.)
- **The start location has no Shift axis arrows:** they stay the
  objects'; the marker lies on the ground, where the game puts a player,
  and its drag and Adjust Location place it exactly: left out by the
  owner's decision. (Settled by the owner, 2026-10-29.)
- **Variable sets** hold int, float and string variables only, as the
  Variables window edits: object and location variables mean nothing
  outside their module. A set's Undo Delete lasts while the Variable
  Sets window is open and a rename is put back by renaming it (neither
  is in Edit › Undo, the module's), and `variable-sets` has no folders
  inside it. (Settled by the owner, 2026-10-29: the triage's proposals.)
- **Tile Properties on several tiles** writes only the control changed
  into every tile chosen: since Tile Properties became the Right Dock's
  panel (M4's U5) there is no OK, and "set" means changed in the panel,
  each change written at once as its own step (a control chosen, even
  to the value the first tile shows, goes to every tile that has it; a
  control not touched writes nothing). Aurora's OK writes every value
  shown, the first tile's, into all of them, and the source lights and
  loops it shows disabled as 0 (captured 2026-10-30,
  `captures/wave2/tiles-one.mod`: Main Light 1 changed alone on two
  tiles of lights 5, 6, 1, 2 and 10, 11, 3, 4 left both 27, 6, 0, 0,
  their loops 0); Moonglow keeps its own, as the owner's report on Tile
  Properties asked. (Settled by the owner, 2026-10-30; restated by M4's
  U5, 2026-11-15.)
- **The game's text in the language edited, and the feminine talk
  table:** the install is read as that language when its
  `lang/<code>/data/dialog.tlk` is there (its talk table, and the rest
  of its `lang` folder); a module's custom talk table is one file
  whatever the language (`the_install_s_language_is_the_one_edited`).
  The feminine table is not shown: Aurora shows a line by
  its StrRef from the masculine table alone (a custom table's line 1 in
  its conversation editor, an item named by line 0 in its palette:
  "W2 MASCULINE", with `w2f.tlk` beside it saying "W2 FEMININE",
  2026-10-30), as Moonglow does: the feminine table is the game's for a
  woman's character. (Settled in Aurora.)
- **`mg` builds the GPU stack by default** for `mg nui render` (drawn
  with wgpu) and `mg nui layout` (a frame of the NUI Creator's canvas,
  egui, no GPU), both of mg-ui: 229 crates, 2 min 42 s cold (`cargo
  build --release -p mg`, 4 jobs), 37.8 MB. Without the feature
  `nui-preview` (`--no-default-features`) `mg` is 140 crates, 87 s and
  22.7 MB, and has neither command (clap says "unrecognized
  subcommand"). The packages keep the default, so only builds of `mg`
  from source would gain from the canvas in a crate of its own, with
  egui and no wgpu, which was looked into on 2026-10-29 (the deferred
  sprint's package m11) and is several days: the crate would take
  `nui_view/preview/` (the canvas, about 3,400 lines), `skin.rs`
  (1,400), the painting half of `draw.rs`, `layouts.rs`'s variant in its
  window and `interaction.rs`'s session, and the 25 or so fields of
  `nui_view::State` the canvas reads; and what the canvas reaches into
  the editor for would become hooks (`design::node_name` and `Insert`,
  `structure::select`, `drag_target` and `Drag`, `widgets::field_label`,
  `widgets::dropdown` and `fields::hinted`). Kept as it is while no one
  builds `mg` alone. (Settled by the owner, 2026-10-30.)
- **`mg merge-gff`: an object both moved and retagged on one side** is
  taken as removed and another added (a conflict where the other side
  changed it): matched by its blueprint alone, an object one side
  removed while it placed another of the blueprint elsewhere would take
  the other side's change (the merge review's ruling, 2026-10-30; the
  chest A / chest B test). (Settled by the owner, 2026-10-30.)
- **A module's files changed outside, read on a thread of their own**
  (`mg_module::outside`; their reads' allocator arena is settled above):
  a reload that touches resources the undo history has changes to
  clears the whole history (undoing an earlier change to them could not
  be done from what they are now; a command's other edits are not taken
  apart from it); a script open in the editor with typing not yet
  applied counts as changed here (it is unsaved work, and the window
  asks which to keep); the haks and the `override`, `development` and
  portrait folders (and the custom talk table) are read again on the
  window's thread after a change, measured on an `override` of 20,000
  files at 20 ms each time something in it changes (the look that finds
  the change 7 ms; the machine's load 2.2 on 32 CPUs): three frames at
  144 a second, once a change. The three kept as they are. (Settled by
  the owner, 2026-10-30.)
- **An inventory's item made from the blueprint, between a placed object
  and a blueprint:** an item carried (dragged, pasted) between them is
  made from its blueprint where it lands, keeping its flags, as a
  blueprint's inventory holds only a blueprint's name. (a second
  builder's review, October 2026) (Settled by the owner, 2026-10-30.)
- **An inventory's moved row keeps its grid place:** a row dragged
  within a list takes its place in the list, and keeps its `Repos_PosX`
  and `Repos_Posy` (where the game's inventory grid shows it), as
  Moonglow shows a list, not Aurora's grid. (a second builder's review,
  October 2026) (Settled by the owner, 2026-10-30.)
- **A Polish name in the English text, with another editing language:**
  String Edit reads and writes the Windows-1252 languages' text as
  Windows-1250 when Polish is the editing language (`text.rs`'s
  `entry_codepage`); with another editing language a Polish name kept
  in the English text is read as Windows-1252, as an English game and
  Aurora (the system's codepage) show it: Moonglow has nothing else to
  know the text's codepage by. (issue 13's other half, from builders'
  reports after 1.19.1) (Settled by the owner, 2026-10-30.)
- **What the Module Uses, a script's strings and the release cut**
  (`--usage`, `mg_preview::usage`): a script's string counts as a use of
  whatever it spells, whatever the string is for (a release cut keeps
  too much rather than too little; said in the manual), and a per-hak
  list for the release cut (a folder of the used assets to pack smaller
  haks) is not written: `--json` has the lists, and writing haks from
  them goes beyond what was asked. (Settled by the owner, 2026-10-30.)
- **Custom lights' rows and models no light uses any more** stay in the
  hak (Custom Light…, `mg_module::lights`), as a row's number is what
  placed lights know it by. (from builders' reports after 1.16.2)
  (Settled by the owner, 2026-10-30.)
- **A tile's emitters' keys:** a tile's emitters take their keys from
  the animations playing on it as one animation as long as the longest
  (`scene.rs`); the game's tiles' keys are constant, and one that
  changed over a shorter loop would run slow, but no tile of the game's
  has such keys. (from builders' reports after 1.14.1) (Settled by the
  owner, 2026-10-30.)
- **Grass to the end of the view:** the client's Grass Far Render
  Distance (`graphics.grass.render-distance`, 900 by default and at the
  least, to 30,000) is in square meters, the grass fading out over the
  last fifth or so of its root from the eye: at 900 the white tufts are
  whole to about 22 m and gone by 28 to 30 m (`reading_grass_placement`
  with `MG_GRASS_DISTANCE`, 2026-10-29), at 3,600 they reach the horizon
  of the 4 by 4 area. Moonglow's go on, whole, to the end of the view,
  as the owner asked of an editor's camera on 2026-10-23 (grass seen
  only about its target was what he took for a fault; from an editor's
  height, 30 m from the eye is no grass at all): not cut at the
  client's distance. (Settled by the owner, 2026-10-30.)
- **`mg pack` compiles models only when asked** (and a nasher project's
  Pack, by its project file's `[pack] compile_models`; the compiler and
  writers in `mg-mdl`, moved from the viewer's `crates/mgv-mdl` at its
  0.1.6): the game reads text and a pack's bytes should not change
  unasked, so it stays off by default. (Settled by the owner,
  2026-10-30.)
- **A module's `encoding.2da`, the four kept by choice** (EE 1.87: a
  table of the Unicode character each byte of the game's text stands
  for; the toolset, plugins and `mg` read and write a module's text by
  it wherever they are given the module, as the server reads it,
  `engine_encoding.rs`): a prefab's file keeps the bytes (written as
  Windows-1252 text, so that it is the same object in any module); a
  hak's description is the hak's, not the module's; a nasher project's
  sources are converted by its `nasher.cfg`'s `--nwn-encoding`, as
  nasher's own tools convert them, so that the bytes nasher packs are
  the module's; and the toolset reads the game's text by Options ›
  Language, not by the user folder's `game.language.codepage`, which
  the server honors and `mg lsp` reads. An exported or imported
  conversation is spelled over as a whole, its punctuation too (of no
  matter unless a table moves ASCII's own characters). (a builder who
  knows the engine, after 1.19.1) (Settled by the owner, 2026-10-30.)
- **Where the NUI preview is known to differ from the game** (stated in
  [the manual](manual/18-nui-creator.md);
  `crates/mg-ui/src/nui_view/preview.rs`; `mg nui render` and `layout`
  show the canvas): an encouraged control shows its hover look and does
  not pulse (the pulse's period not measured): a still preview shows
  the pulse's peak; and `mg nui layout`'s `overlaps` come of the
  canvas's own layout, which places neighbors apart, so it finds none
  but where the game would lay the window out otherwise, which it can't
  know: said in the manual's command line chapter, as it is. (the
  review of the NUI Creator's pull request, October 2026) (Settled by
  the owner, 2026-10-30.)
- **F6 against a builder's own binding:** a command a builder already
  gave F6 to is in conflict with Next Pane; Options › Keyboard lists
  the conflict, and the pane's wins until one of them is changed: the
  builder's binding is theirs to change. (Settled by the owner,
  2026-10-30.)
- **JPEG and BMP pictures of a NUI window are read with the `image`
  crate:** nothing else in Moonglow reads them, and the client takes
  them for a NUI picture alone (JPG first, BMP after PNG); the NUI preview's
  pictures (`skin::Assets`) and the UI's (`images::Loader`) keep their
  own orders on purpose, and both read a picture the module changes
  again at once (since 2026-11-25). (By design.)
- **Advanced Search: conversations edited together** (the last of
  "Advanced Search, what it doesn't do", from builders' asks on Discord,
  October 2026; closed 2026-11-25). Its other parts are built: a field
  set is added where it is missing with **Add where missing** checked
  (off by default, since the engine reads a missing field as its
  default; only where the object's kind has the field in the game's
  files, `mg_schema`, of the type given there); objects of one kind
  found in several areas open in one Edit Together, one undo step
  across the areas; and the items placed objects hold or stores sell
  open together as well. Conversations still open one at a time, by
  design: a conversation is one tree in its own editor, with no form of
  fields that several could share, and what a builder changes on many
  (a line's script or a field found on it) Change the N checked
  already sets on each line found, in one step. An item inside a bag
  held is not among the held items edited together, also by design:
  Properties open on the items a placed object holds itself, the bag
  among them. (By design.)
- **NWNT and a typed line break: refused** (the owner's word,
  2026-11-25): a nasher NWNT project refuses to save a file whose text
  holds a typed `\n` or `\r` (`C:\new`) and names the field, as it
  refuses a label with a dot, since NWNT would read it back as a line
  break. Saving it with a note in the log was the other choice; a file
  that comes back changed was not wanted. Labels with `[`, `]` or a
  trailing space are kept (Moonglow's reader reads them back). (the
  review of November 2026, F5)
- **A resource name past ASCII in a module folder: the rule kept**
  (the owner's word, 2026-11-25): a byte above 0x7F goes through a
  folder as the character of its number and is read back as that byte,
  so file names are the same on every system and valid in a
  repository. A loose file another tool named in UTF-8
  (`pièce01.utc`) is read as byte 0xE8, not as the game on Linux reads
  it from `override/`; such files are rare. (the review of November
  2026, F7)
- **Bulk saves are not flushed file by file** (the owner's word,
  2026-11-25): a module folder's save, a project's save and NWSync's
  data files are written to a temporary name and renamed
  (`write_replacing_one_of_many`), without a flush. A flush costs about
  1 ms a file (3,000 files 0.07 s without, 2.95 s with); the rename
  already leaves the old file or the new one, and only a power cut
  during the save is at risk. (the review of November 2026, P17)
- **Verify's hak cache and a hak rebuilt alike: kept** (the owner's
  word, 2026-11-25): what Verify keeps of a hak is named by its folder,
  its entries, its name, time and size; a hak rebuilt with the same
  time, size and layout, only its data changed, is taken for the old
  one. A rebuild changes the time, and hashing every hak's data on every
  run was not wanted. (the review of November 2026, V14)
- **A talk-table import far past the end: no limit** (the owner's word,
  2026-11-25): an import that reaches a row far past the table's end is
  one undo step, with no bound on the gap; a team's range may begin
  high, and one undo removes the empty rows. (the review of November
  2026, P4)
- **The model viewer's ground shadow: a shadow map always** (the
  owner's word, 2026-11-25): drawn as a shadow map whatever Options'
  Shadows quality says (`shadow_volumes = false` before its draw),
  since stencil volumes cast no skins and a creature's body would cast
  no shadow in the viewer. (the review of November 2026, the renderer's
  page)
