---
type: Backlog
title: 'Deferred: what was left out, for a decision later'
description: What finished work left undone or unchecked, filed by area of the program (an entry's last words say which report, issue or review it came from), each awaiting a verdict - fix, add or drop; what is settled by decision or design is gathered at the end.
tags: [backlog, deferred, after-parity]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-29T09:42:01Z }
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

- **Sweeps still walked more than once:** the areas of every module by
  four tests (they share the module's set-up, not the walk: the renders
  hold the GPU, and the others read each area differently), and
  `decompiled_models_compile_back` decompiles each model a second time
  (10 s of the full run's 28 s for models). nasher still unpacks each
  edited module once more, as the test is about that. (M)
- **The `app` tests' set-ups left by hand** (`crates/mg-ui/tests/app/`,
  split by subject, with `harness`, `game_app` and `gpu` in `common.rs`):
  81 harnesses at kittest's default size (`Harness::new_ui_state`), 26
  installs made apart from the application (read before it, in another
  language, or by a test's own helper), 14 scratch user folders, each
  with its own subfolders, and 6 harnesses with a step of their own;
  and nothing fails when a file in `tests/app/` is not named in
  `main.rs` (its tests would not run), as the corpus groups' check does.
  (S)
- **`mg` builds the GPU stack by default** for `mg nui render` (drawn
  with wgpu) and `mg nui layout` (a frame of the NUI Creator's canvas,
  egui, no GPU), both of mg-ui:
  229 crates, 2 min 42 s cold (`cargo build --release -p mg`, 4 jobs),
  37.8 MB. Without the feature `nui-preview` (`--no-default-features`)
  `mg` is 140 crates, 87 s and 22.7 MB, and has neither command (clap
  says "unrecognized subcommand"). The canvas in a crate of its own,
  with egui and no wgpu, would leave `render` alone behind the feature;
  the packages keep the default. (M)
- **Moonglow Viewer's shader cache across toolset tags:** the libraries
  are 0.0.0 and read the release from the program (`mg_core::release`),
  which the viewer doesn't set, so its compiled-shader cache is named by
  the passes' `FORMAT` alone and no longer starts anew when it moves to a
  newer toolset tag. One line at the viewer's start
  (`mg_core::release::set`, its own version) names it as before. (S)
- **Shared test fixtures written many times, what is left:** `fn lines`
  of the Aurora tests is one (`aurora_common`, 2026-10-27) and reading
  `mg --json`'s answer is `json_out` and `quiet_json_out` in the command
  line's group, but each file of `apps/mg/tests` still has its own
  wrapper for the arguments it adds (`--root`, `--user-dir`, a home),
  and the corpus tests' `gpu()` (`game_shaders.rs`) repeats
  mg-render's tests'. (S)

## Script intelligence (`mg lsp`, the script editor)

- **A script's codepage** (GitHub issue 27) is the language's of
  Options › Language, for every script alike; there is no choosing
  another for one script (reading one again in another, saving one in
  another), and a script kept in UTF-8 is read as that codepage's
  letters. `mg` other than `mg lsp` has no language: scripts' bytes pass
  through it untouched, but the lines it shows of them (`mg refs`'
  strings that spell a name, the toolset's Used By) read them as UTF-8,
  a Polish letter as "�". The game reading the compiled strings in
  Windows-1250 was not seen in a Polish client: the bytes were checked
  in the compiled script. (S each)
- **The Save key in a script editor** (El Grillo's request, October
  2026) is the script's own Save while its text has the keyboard, as its
  Undo and Redo are; with the pointer over the editor but the keyboard
  elsewhere (a toolbar button just clicked, the side list's Filter) it
  saves the module, where the side editors' Undo goes by the pointer.
  File › Save and the toolbar's 💾 are the module's always. A Save with
  nothing unsaved in the script does nothing, as its grayed button.
  Aurora's script editor (a window of its own there) has its own Save on
  Ctrl+S; it was not driven to see what it writes. (S)
- **To Scratch for a script that doesn't compile** (El Grillo's
  request): the script editor's To Scratch and the module tree's Copy to
  Scratch Folder are one rule (the source alone; the folder's
  `name.ncs` and `name.ndb` from an earlier copy removed, and said).
  Not done: Export as Files to a folder chosen still sends a script that
  no longer compiles with the module's older compiled script, with a
  warning (nothing is removed from a folder the builder picks); with no
  game data nothing is compiled, so scripts go out with the compiled
  scripts the module has; only those exact lower-case names are
  removed (a `HELLO.NCS` another program wrote on a case-sensitive file
  system stays); a debug file left by an earlier copy of a script that
  now compiles without debug information stays. (S)
- **Open Script's list** takes a click, Ctrl+click, Shift+click, the
  arrows (Shift to widen), Page Down and Up, Enter and a double click;
  not Home and End, Ctrl+A, or a right-click menu. Typing in the name
  clears the selection (Enter then opens the first listed, as before);
  scripts selected and then hidden by choosing other resources to show
  stay selected but aren't opened. (S)
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
- **Indentation by file:** a script's indentation is guessed when its
  text is read (opened, read again from the module, a Save of it
  included, or from outside), from its indented lines (tabs or spaces as
  most are; the step most often taken), and kept until it is read again
  or converted: not as it is typed, and not remembered from one session
  to the next. A conversion taken back with the editor's Ctrl+Z gives
  the text back but not the guess: Tab indents as converted until the
  script is next saved or read. Enter copies the line's own leading
  spaces and tabs as they are. Convert Indentation is the whole
  script's, not a selection's; from the Edit menu it is the script in
  front, which for scripts in windows of their own (the tab in front
  not a script) is the one whose editor had the keyboard last. (S)
  (Jasperre's report, 2026-10-21)
- **Edit › Undo and the toolbar's Undo in a script** are the script's
  own while its editor has the keyboard, as Ctrl+Z is (the click on
  the menu or button counts as in it), say plain "Undo" and "Redo",
  and give the keys back to the editor; the module's history was not
  given the script's typing. Left: they are not grayed with nothing in
  the script's history (reading egui's history copies all of it, each
  frame); a menu opened from a script and closed with nothing chosen
  leaves the keyboard nowhere, so Ctrl+Z is the module's then, as
  after any click outside the editor; a key given to Undo or Redo in
  Options, other than Ctrl+Z, Ctrl+Y and Ctrl+Shift+Z, does nothing in
  a script (read from the code, not tried); the Command Palette's Undo
  is the module's (its field has the keys). Seen meanwhile: the
  toolbar's Undo clicked after the pointer was over a hak's editor
  took back the module's step, not the hak's (the pointer has left the
  editor by then), so Edit › Undo and the toolbar's are the module's
  for a side editor; its own Undo is Ctrl+Z with the pointer over it,
  or with the keys in a 2DA's grid. (S each) (a builder's report on
  1.25.1)

## nasher projects

- **NWNT projects:** projects using NWNT instead of JSON for GFF files
  are refused with a message. (M)
- **`mg merge-gff`, what it leaves out** (the [manual](manual/03-modules.md#merging-in-git)
  has the rules):
  - **Lists merged by place** that have an identity of their own: an
    inventory (`ItemList`, which could pair by `InventoryRes` and its
    `Repos_PosX`/`Repos_Posy`), a store's panes, an encounter's
    creatures, item properties, a creature's special abilities. Two
    sides adding one each is a conflict today, not both kept. (S)
  - **Conversations** merge their entry and reply lists by place, and the
    links name nodes by number: two builders adding nodes to one
    conversation conflict, where renumbering one side's new nodes (and
    the links to them) would keep both. (M)
  - **An object both moved and retagged on one side** is no longer
    recognized: a removed object and an added one (a conflict if the
    other side changed it). A nearest-position match could pair it. (S)
  - **Order:** where both sides changed a list, one side's reordering
    (a list sorted, an area moved up the area list) is lost; additions
    go at the end. A sequence merge would keep both. (S)
  - **Localized strings** merge whole: one side changing the English
    name and the other the French one is a conflict. (S)
  - **On a conflict nothing is written** and git leaves ours in the file;
    there is no file of the conflicts to edit, and no way to take one
    side for one conflict and the other for the next but `-o` from
    git's stages. A conflict file (the edits each side wants, for `mg
    apply`) or a Moonglow window would. (M)
  - **NWNT files** aren't merged (the projects aren't read either). (S)
  - **The settings:** a JSON file's float places and codepage come from
    the nasher project above `--path` (or OURS's folder); a repository
    whose project sits elsewhere gets nasher's defaults. (S)
  (The Domains of Dread's builder proposal, C9)

## Modules kept as folders

- **A module's files changed outside** (a nasher project's sources, a
  module folder's files; a `.mod` file changed outside isn't noticed)
  are found by a thread of its own (their names, times and sizes every
  3 s, said once a burst of changes has settled), and since The Domains
  of Dread's builder proposal (C12) read on a thread of their own too
  (`mg_module::outside`), then taken by the window at once. Measured
  (`frame_perf` `reload_frames_at_144`: the persistent world kept as
  a nasher project, 21,000 files, a pull of 970 written over a second):
  the longest frame was 1,630 ms before, 3.5 ms after, with every change
  in 4.6 s from the pull's start (2,870 files, every area and 2,000
  blueprints: 6.5 ms, in 6.2 s). The files' times and sizes decide what
  is read, so a change that keeps both within the clock's grain is
  missed (a project's files' times are noted at open now, so this holds
  from the start). A reload that touches resources the undo history has
  changes to clears the whole history. A script open in the editor with
  typing not yet applied counts as changed here. The area list isn't
  settled again for areas added outside. Not done: the watch's thread
  still walks every file every 3 s (on its own core; not measured), and
  the look after it looks at every known file's time
  again rather than taking the watch's list; what the window's thread
  does when a look begins and ends (a list of the known files, the
  changes taken) grows with the project and is under the budget at
  21,000 files, not measured above; read on another thread, the files'
  memory comes from that thread's allocator arena, so the process keeps
  about 200 MB more after a 970-file pull on Linux (25 MB with
  `MALLOC_ARENA_MAX=1`; it is reused, not leaked); more than 24 scripts compiled after a
  pull still go to a job, which holds the window's input; a change is
  in up to about 7 s after it settles (the watch's 3 s, the window's
  3 s). The haks and the `override`, `development` and portrait folders
  (and the custom talk table) are watched the same way, and read again
  on the window's thread after a change (not measured on a large
  `override`). (S each)
## Content doctor

- **Verify's groups, not its checks:** Options › Verify and `mg verify
  --skip` leave out a group (seven: missing references, tilesets, models
  and materials, 2DAs, objects' rows and variables, talk table and
  archives, plugins), not one check, one hak or one tileset; a group
  left out isn't checked since 22 October (so nothing is counted of
  it: the summary and the JSON's `skipped` only name it), and Build
  Module's Missing Resources has its own categories, apart from these.
  (S) (Taro49's report, October 2026)
- **A whole Verify of a module with many haks is still 1.4 s:** the
  Domains of Dread with its 145 haks (28 GB) went from 70 s to 9 s on
  16 threads, and on 27 October from 4 s to 1.4 s on 32 (70 s of the
  workers' time to 25 s): the linter no longer looks through the whole text for
  each node named twice (that made a 6 MB tile, `twcy1_s02_93`, take
  1.4 s: now 15 ms), a tile model's faces are told from its `faces`
  counts unless there are more than 10,000 rows, compiled models are
  told by their first bytes (8.5 GB of them were read whole), and the
  shader programs compile in parallel. What is left: linting 43,000
  models kept as text (1.9 GB, 13 s of the workers' time, 300 to
  400 MB/s on one thread), the tilesets' 7 to 9 s (each compiled tile
  model read whole to count its faces, and the text tiles with more
  than 10,000 `faces` rows, at least the 269 warned of, read in full at
  80 MB/s, where the faces could be counted without building the
  meshes), and the 2DAs' 2 s. (M) (the
  Domains of Dread builders' report, October 2026)

## Test loop

- **`mg serve` on Windows:** not tried there: the server's console on
  pipes, the links for haks, and Ctrl+C (the server in a console
  process group of its own, `CREATE_NEW_PROCESS_GROUP`, told to `exit`
  by `mg`). (S) (waits on a Windows machine)
- **`mg watch`** (a project followed into a development folder): the
  project is read whole again at each change, and looked at by its
  files' sizes and times some twice a second (no file-system
  notifications); a nasher project's default target only; debug files
  (`.ndb`) are made where `--ndb` asks (`mg watch`, `mg compile`), and
  by Compile All Scripts and Build Module with Generate Debug
  Information on, and `mg build --ndb`; `mg rename` and the window's Rename give a script that had one a
  new one; nothing
  is pushed through NWNX's console; with `--serve`, tests are not run
  again after a change (`mg test` is a run of its own). A test holds
  the `--serve` half on Linux (a change put out, the server started
  again at its port after a build, `/quit`). (M, S, S, M, S)
- **Choose Character:** Test Module, Choose Character uses the game's
  `+LoadNewModule` (from nwn.wiki), never run in the game client. (S)
- **Test Module and Steam:** the game started for a test is told which
  Steam game it is (`SteamAppId`), so that a Steam copy reaches a running
  Steam and loads the Workshop's content, as when started from Steam;
  started bare it had none of it (a builder's "F9 won't load the stuff
  correctly either"). Not tried: the client is only ever run here with
  Steam hidden. With Steam running, the test then shows in Steam as the
  game being played. (needs a builder's word)
  (from builders' reports after 1.14.1)
- **A running game** is known only if Moonglow started it for a test:
  one started otherwise still has the test module written under it. (S)
  (GitHub issue 6)

## Persistent-world scale

- **Other machines:** budgets were measured only on the development
  machine (Ryzen 7 5800X3D, Radeon RX 9070). (S, needs other hardware)
- **Where-used at scale:** the References window and Rename keep the
  module's references between questions (`rename::UsageIndex`): the
  first Find References in the persistent world reads the module, on
  every core (0.2 s; 1.4 s on one), the next ones only the resources
  changed since (7 ms). Tags and the strings in scripts are answered from it too.
  Not done: the first read is on the window's thread; `mg refs` reads the module each time, as a
  command run once must. (S each)

## Frames (a steady 144 a second)

The budget and the measurements are in
[the frames note](research/notes_frames.md). Done: an area's picture kept
between its animations' steps, its drawing at a fifth of the cost (lights
by the ground they reach, materials worked out once, a draw's values by
its number, what is out of sight left out, tiles of one model posed
once, meshes of one model drawn as instances of one draw), the
galleries' pictures made for 2 ms a frame and kept while in sight.

- **The largest areas in full view:** a 784-tile stock area takes 2.5 ms
  a frame zooming out; TDoD's synthetic woods (33,000 meshes in sight)
  13 ms (21 turned to the horizon), 25,000 draws of meshes mostly drawn
  once each. What is left is each draw's own work: wgpu's call and the
  chunk of shared buffers it switches to (a large area spans ten), each
  mesh's lights chosen and its values written. Fewer, larger chunks, and
  values kept from frame to frame for what doesn't move, would cut it.
  ([the frames note](research/notes_frames.md), 27 October) (M–L)
- **An area filling in as it opens:** read on a thread and its textures
  apart (no frame holds it: TDoD's woods held the window 3.8 s), its
  picture is drawn again as textures arrive, each such frame costing what
  the view costs drawn whole (13 to 22 ms for the woods); and the frame it
  is first shown in looks its materials (MTRs) up on the window's thread
  (19 to 30 ms for the woods, 23 for a water area of 160 fires: not taken
  apart). Its models show all at once, once the area is read, not one by
  one. Edits, Reload Resources and tests still read in the
  frame. (S–M)
- **A slow picture in a gallery** (a model with large textures) takes a
  frame of 7 to 9 ms: a picture's models and textures are read on the
  window's thread. Read on another. (M)
- **`Workspace::flush` writes every document read,** changed or not: its
  cost grows with what a session has opened. It is called once a revision
  by the pictures and by each open model view, and on saving. Marking the
  documents an edit changes would make it what was changed. (S)
- **The tiles' animations are looked up by name** every time one is
  posed (three times a tile's model a frame: the pose, the lights, the
  meshes), lower-casing each node's name. Bound once when the model is
  loaded, a scene's 0.5 ms would be less. (S)
- **Pictures that differ from run to run, with the game's shaders:**
  with Moonglow's own shader an area's picture is the same from run to
  run (`render_area` twice, in two processes: all 140 of Tyrants of the
  Moonsea's areas and the 118 of Prelude and Chapter 1, 2026-10-29).
  With the game's (`MG_SHADERS=1`, the app's default), 8 of Tyrants'
  140 differ, by 1 to 366 pixels and 2 to 15 of 255 (area085,
  aquarium001, area083, area091, dm_thentnpc1 to 3, ir_roadtovoonlar):
  single pixels scattered over dark ground, a different few each run.
  In one process the same scene drawn again, by the same renderer or a
  new one, is the same to the bit; across runs the scene's lights are
  the same, and so are every pipeline's naga modules and bind group
  layouts. So something set once a process in the game shaders' draw
  path changes the values drawn by a hair (a `std` HashMap's order, the
  likeliest: `game.rs`'s `pipelines` and `value_ids`, its worker's
  `programs`; or the order the worker finishes pipelines in). Fixed, the
  game shaders' picture tests could compare to the bit. In `mg-render`:
  the renderer's CI branch. (S-M)
- **The frame test only times:** `frame_perf.rs` names the views over
  the budget and fails nothing. With budgets it would keep them. (S)

## EE fields Aurora hides

- **An armor's part colors:** edited and previewed, and the game reads
  them (`engine_armor_colors.rs`) and draws them: a worn armor with a
  torso of its own colors looks the same in the client and in Moonglow
  (`creatures_look` in `client_render.rs`, by eye). The inventory icon is
  drawn in the armor's own colors; whether the game's icon shows a
  part's isn't known. (S)
- **What a creature can't wear:** the game leaves out an equipped item
  the creature may not use when it loads the area (a stock cloak with
  its item properties came out of a commoner's cloak slot, and
  `ActionEquipItem` was refused too; without the properties it was
  worn). Moonglow draws whatever the file has equipped. Which
  properties keep an item off wasn't looked into. (S–M)
- **A placeable's illumination, what's left:** **Illuminated** off now
  leaves out the appearance's light in the area view and the preview
  (read from the client: `model_light_uniforms`), and the box can be set
  whatever the appearance. Not looked at: the glow that goes with the
  light (a lamp's self-illuminated flame, which is the model's own
  animation state, the separate Initial State), and whether Aurora shows
  or keeps the field. Which `LightColor` the engine counts as a light
  (Moonglow: a row above 0) was checked on rows 1 and 25. (S) (Jasperre's
  reports, October 2026)
- **The PC faction, what's left:** the faction lists, the Store Wizard
  and Verify (`pc-faction`) keep objects out of faction 0. Not done:
  `mg set` and the raw fields view still write 0 if told to (Verify
  reports it after); Verify
  doesn't report an object with no faction field (the engine's reading of
  a missing one wasn't measured); Aurora's own faction lists were not
  captured (whether they offer PC is not known), and Aurora writes
  faction 0 for a trigger whose blueprint has none (`instances.rs`
  `TRIGGER`), which Moonglow copies when placing; so Verify leaves
  triggers out of `pc-faction` (the owner's decision), and such a
  trigger's faction field shows "PC (not for objects)". How an object of faction
  0 regards a player, or a player it, was not measured (no player joins
  a headless server); an encounter's creatures were not looked at. (S)
  (Jasperre's report, October 2026)
- **The Visuals page's texture and animation pickers, what's left:**
  waiting on the client: whether the game replaces a PLT texture by name (the page and the
  preview take it that it doesn't, from `ReplaceObjectTexture`'s comment
  that the new one must be simple), and whether a creature's replacement
  reaches what it holds, its helmet or cloak (left out of its list). A
  client screenshot of `ReplaceObjectTexture` on each settles it. (S)
  (GitHub issue 32)

- **A tile's replacement texture, out of Tile Properties:** the form
  had a Replacement Texture (`Tile_ReplaceTex`, a `replacetexture.2da`
  row for a texture named `replace_tex` in the tile's model); a builder
  called it untested and likely not working, and it was taken out
  (2026-10-22). What is known: the server keeps the byte
  (`engine_ee_fields.rs`, `ObjectToJson`); nwn.wiki calls it BioWare's
  unused functionality ("Replacing Simple Textures", "2da Files"); the
  game's `replacetexture.2da` has two rows, both `T_Flag02`; the one
  model with a `replace_tex` texture, `tms01_c08_01`, is in no tileset
  (not in `tms01.set`); none of BioWare's 20 campaign modules
  (`data/nwm`) writes the field. Not known: whether the client draws
  it. To settle it: a scratch user directory whose `override` has a
  copy of `tms01.set` with a tile's model made `tms01_c08_01` (its
  `.wok` checked), an area of that tile with and without the field,
  screenshotted with `tools/nwclient/run-client.sh` (two client runs;
  the pictures differing is the answer). One in a file is kept, shown
  and editable in the raw fields, and the area view still draws it
  (`mg-area` `scene.rs`), unverified. If the client draws it, the
  field can come back with that test. (S to M)
  (from a builder's tile feedback, October 2026)

## Blueprint editors and inventories

- **Text typed and its field gone before the keyboard left it:** a
  text field that sets its text when the keyboard leaves it gives the
  keyboard up at a press of the pointer anywhere else, and before a
  command asked for by its key is run (Save, Close Tab, Next Tab: the
  command is run a frame on), so that it sets what was typed while it
  is still drawn. So in every window's fields of the kind, the hak
  editor's description, a conversation script's parameters and a 2DA's
  cell being typed in among them. Not so: a plugin's command by its key
  (run at once), and a window taken away by nothing the builder did (a
  module's files changed outside and read again). (S) (a builder's
  report, October 2026)
- **An appearance with no model, what the warning leaves out:** the rule
  (`mg_rules::appearance`) checks a placeable's, a door's and a
  single-model creature's row and model by name, and an item's base item
  row. Not checked by it: an item's own models (its parts, `ModelPart1`…
  and an armor's), which the preview alone says are missing; a body of
  parts' skeleton and parts; a waypoint's flag (waypoint.2da), which the
  preview checks on its own; a model that is there but cannot be read (a
  broken file is found when it is drawn). The Appearance lists in the
  Properties don't mark the rows with no model (the Appearance Gallery
  does, for those in sight), and the palette's own Gallery doesn't mark
  blueprints whose appearance has none. (S) (dansen9191's report,
  October 2026)
- **Number fields' limits against Aurora:** measured in Aurora
  (v89.8193.37) by typing past them: a creature's save bonuses 250 (and,
  it seems, no lower than 0: Moonglow keeps -100, for the penalties files
  have), its ability scores 100, natural AC 1000 (Moonglow: 255, the
  byte the file keeps), base hit points 10000; a placeable's and a door's
  hardness and saves 250, hit points 10000; lock and trap DCs 250. These
  are Moonglow's now. The other fields the forms give "spin 0…100"
  (`aurora-ui-inventory.md`: encounters, stores, alignment and the rest)
  were not measured: the forms' limits are only what they start with.
  (S) (a second builder's review, October 2026)
- **Which rows Aurora offers** as feats, spells and special abilities
  (by `UserType`, `ALLCLASSESCANUSE` or else) was not captured: Moonglow
  leaves out rows without a name and those labelled DELETED or Padding.
  (S) (a second builder's review, October 2026)
- **Random item properties, what is left** (done: the Item Wizard's
  Magical, Item Level and Item Quality, with rules fitted to 127 items
  Aurora's wizard made, `docs/findings.md`): the rules are fitted, not
  Aurora's code: whether a later property's tier is drawn (one or two
  below the first) or tried in order, and Low items' most worth (levels
  1, 5, 8 and 13 fit what was seen; nothing in itemvalue.2da says so)
  would need more of Aurora's items to settle; amulets, belts, boots,
  bracers, cloaks and rings get the armor table, where Aurora may use the
  helmet's (they differ in Weight Reduction only); gems and gauntlets
  aren't offered Magical (Aurora makes them unidentified with no
  properties). (S)
  (a second builder's review, October 2026)
- **Inventories, what is left** (done: the Creature Wizard's ResRef and Tag, its appearances
  as pictures with Find, and every portrait; the inventory's Equip to a
  free slot, To Backpack and Open Blueprint; talk-table text in its own
  color; a Variables window that is sized; drag and drop and right-click
  menus in the inventory, items dragged between inventories and within
  a list, Copy and Paste; an item moved as it is, in one step; what a
  taken slot held sent to the backpack, as Aurora does): items
  are carried one at a time; of the drags between two objects' windows
  only a chest's item onto a creature's backpack is under test; an item
  carried between a placed object and a blueprint is made from its
  blueprint there (a blueprint holds only that); Copy and Paste carry
  the blueprint, so what is pasted is made from it; a moved row keeps
  its place in the game's inventory grid (only the list's order
  changes); an item moved from one slot to another asks the question
  about a missing feat as any Equip does (Aurora asks when a weapon is
  dropped on another weapon slot, but a true move of an item that needs
  a feat between two slots it takes was not seen); a weapon dropped on
  the secondary weapon slot while the primary is empty goes in the
  secondary slot, where Aurora puts it in the primary (seen with a
  longsword only: whether every item that fits both does so was not
  looked at); blueprints made in a category the palette hides;
  new strings written to a talk-table range. (S–M)
  (a second builder's review, October 2026)
- **Appearances as pictures** are placeables', creatures' and doors';
  an item's model, and each of a weapon's three parts, are chosen from
  its icons (the item as it would be with each). An armor's parts are
  chosen from lists of numbers: the pointer resting on a row shows the
  armor with that part on a body and its icon (the part is small in the
  picture of the whole body), but there is no picture of a part alone
  and no gallery of them. A creature's look is a plain body of the
  appearance, male unless the creature is female. (S–M) (from builders'
  reports after 1.10.1)
- **Armor and cloaks shown on a woman, what it doesn't follow:** the
  choice (Options › General, or Shown on in the item's Appearance page)
  is kept, and the item's page, inventories, a store's pages and the
  palette's pictures and Preview follow it. A creature's own Inventory
  page follows the choice too, not the creature's Gender, and whether
  the game shows a woman's inventory with `ipf_` icons (as nwn.wiki's
  icon names say) was not looked at in the client. The game has no
  woman's icons of cloaks (`icloak_m_` alone): a cloak's icon is the
  man's either way. (S) (from builders' reports after 1.10.1)
- **Arrows after numbers, what is left:** the blueprint editors', Module
  Properties' and Area Properties' number fields that have them in
  Aurora have them (a click, Up and Down in the field, a held arrow
  repeating after 0.4 s twenty times a second, each run one step). Not
  given them: the Creature Wizard's and Levelup Wizard's class levels,
  Adjust Location's (an instance's position, bearing, scale, rotation and
  translation), the Area Wizard's and Resize Area's rows and columns,
  Options' (spawn point marker size, undo levels, backup interval) and
  the Script Wizard's conditions. The repeat's pause and rate are
  Moonglow's choice, not measured in Aurora; a sound's cutoff distance
  and intervals step by ten, as Aurora's forms say. (S) (from builders'
  reports after 1.19.1)
- **What a store sells an item for** (a builder: Aurora lists it): each
  item of a store's pages is listed with its cost at the store's sell
  mark up. Not compared with Aurora's list (its rounding, whether it
  counts a stack); the buy price (mark down) is not shown. (S)
  (GitHub issues 10 to 24, after 1.20.0)
- **Arrow keys on lists of choices:** every dropdown is drawn by
  `widgets::dropdown` (a test reads the sources for an egui combo box
  made elsewhere) or, for a table's rows, `widgets::row_choice`, and
  takes them while it has the focus (clicked, open or after a choice;
  Tab), as does Area Properties' Lighting Scheme grid (Left and Right
  across its columns); a list with the keyboard keeps them from the
  conversation tree. Escape gives the focus up, as everywhere in the
  interface, so after closing a list with Escape it is clicked again.
  Not done: the lists of commands (Module Properties' Add hak…, a
  trigger's Setup Area Transition…), the 2DA editor's list of flags (a
  set of ticks) and the Variables window's ▾ menu of a value's
  suggestions take none; under test are the area's lists and dialogs,
  Options' Interface size, Advanced Search's kinds and variable types, a
  creature's Alignment, a conversation's Play Animation and the
  Lighting Scheme, the others by the source rule alone. A list whose
  options are read only while it is used (Module Properties' movies and
  talk tables, a conversation's speaker tags, the NUI editor's targets
  and binds) reads them every frame it has the focus. (S) (GitHub issue 7)
- **Pictures in the Appearance list, what is left:** a creature's (a
  plain male body of the appearance), a placeable's, a door's (Appearance
  Type and Generic Appearance) and an item's (its model's, a weapon's
  parts' shapes and colors, an armor's parts: the item as it would be
  with each, its icon beside) show beside the row the pointer rests on.
  A tileset door type's row **Use Generic Appearance** shows "(no
  picture)", not the generic door chosen. A ring or an amulet, which
  has no model of its own, shows the bag the game drops it as (its icon
  tells the rows apart). The pictures are made on the UI thread after
  the frame, 2 ms of a frame at most, as every gallery's are. (S)
  (GitHub issue 7)
- **An item stood up in its viewer** is turned a quarter about its
  length's axis from how it lies; which side then faces the viewer isn't
  chosen (a sword shows its edge more than its flat). Items in the
  palette's Gallery lie as modelled; on the ground in an area they are
  turned as baseitems.2da's `RotateOnGround` says. (S)
  (GitHub issue 7)
- **Dropable and Pickpocketable against Aurora:** the inventory capture
  (`inventory/after.mod`) equips items only, which Aurora writes without
  either field; what Aurora writes for an item it puts in the backpack,
  for its boxes ticked and cleared, and for a placed creature made a
  blueprint (Add to Palette now keeps an equipped item's flags in the
  blueprint's entry, as the backpack's were kept) was not captured. The
  game's own modules and blueprints have them as 1 when set and left out
  when not, which is what Moonglow writes. Several items selected at once
  is Moonglow's (Aurora selects one). (S) (Sacha's report on 2.5.4)
- **OK and Cancel where there is none yet:** the hak editor, the
  tileset editor, the talk table, the raw field editor and a script
  (each saved, or asked about, in its own way). In the windows that
  have them: a text field's change set after its page was left by a key
  (not by the pointer) is not among what Cancel sets back, and a
  faction's removal is set back by the whole of each area and creature
  it renumbered, as an area's sounds and music (one struct of its GIT)
  are set back together. (S) (a builder's ask, October 2026)

## Bulk editing

- **Make Placeables Static, what it can't see:** a placeable that a
  script destroys, animates or makes usable by its tag, and has no
  variables, looks like scenery and is made static; nothing scans the
  scripts for its tag (a project names such tags in `[make_static]`
  `keep_tags`). (M)
- **Lists across several blueprints:** Edit Together leaves out the
  pages that edit lists (inventories, classes, skills, feats, spells,
  item properties); those are edited one blueprint at a time. Adding the
  same item or feat to each would need its own design. (M)
- **Choosing areas by more:** the chooser filters by name, ResRef,
  tileset and the three kinds; not by a property's value (areas with a
  given music or variable), and its choice isn't kept as a named set.
  Advanced Search finds such areas (a variable, any field's value), but
  the chooser can't take what it found. (S–M)
- **Lighting of one's own:** Saved Lighting and Copy From Area give an
  area the Visual page's fields; not the tiles' lights (each tile's
  own, as a scheme picks them at random from its colors), and lighting
  kept isn't made an environment.2da row of a hak, for the toolset's
  scheme list and the game's `SetAreaLightingScheme` (`mg areas` takes
  no saved lighting either). Saved lighting is listed, not renamed or
  deleted in the toolset (its files are). (S–M) (dansen9191's ask,
  October 2026)
- **Find and Replace beyond strings players read:** tags, resrefs,
  variables and other plain fields are found (Advanced Search, `mg
  search`) but not replaced, and neither has regular expressions (`*`
  only). (S–M)
- **Advanced Search, what it doesn't do** (from builders' asks on
  Discord, October 2026; done: changing what it found, ticked or all, in
  one step: a variable set or removed, an event's script, a field;
  Edit Together and Edit Areas Together from its results; conditions
  together, a class by its levels, number ranges and floats):
  - Changes are a variable, a script or a field: item properties aren't
    added or removed on the items found, nor list entries (feats,
    classes, inventory) on the creatures; a field is set only where the
    object has it (none is added). Edit Together takes blueprints of one
    type and objects of one kind placed in one area; items held by
    placed objects, and conversations, open one by one. (S–M)
  - No spell by the class that knows it (a spell found is any class's);
    conditions are all ANDed (no OR, no NOT); a conversation's lines and
    other things on no object drop out of a search with more than one
    condition; `mg search` takes one condition at a time, without class
    levels or ranges. (S)
  - The module only: blueprints in haks and the game's own palettes
    aren't searched. An item blueprint found is followed into the
    inventories of the module's blueprints, one step only (a chest
    blueprint holding a bag blueprint that holds it isn't listed); a
    creature blueprint found isn't followed into the encounters that
    spawn it. (S)
  - A question asked again (Find again, or found again after a change
    made to what it found) reads only the resources changed since
    (`search::Kept`); another question reads the module again, once
    for all its conditions, on every core but on the window's thread
    (Chapter One, 5,271 resources: 7 to 40 ms, where it took 70 to
    250; the persistent world's 25,000: 0.2 s, where it took 0.7 to
    2.1). What is found isn't drawn from Find References' index, which
    would keep every GFF read (0.6 GB more in the persistent world).
    The events Set an event's script offers still read the objects
    found anew: 0.1 s of the 0.11 s a question asked again takes with
    8,785 found in the persistent world. (S)
  - Appearance covers the 2DA rows a field names (creatures,
    placeables, doors, waypoints, items' base types); items' model
    parts and colors, portraits and sound sets are found through Any
    field. (S)
- **Variable sets** hold int, float and string variables only, as the
  Variables window edits; object and location variables aren't kept. (S)
  Renamed, deleted, saved to and added from files of one's choosing
  (builders' requests, October 2026), with these gaps:
  - Undo Delete lasts while the Variable Sets window is open, and a
    rename has no undo (rename it back); neither is in Edit › Undo,
    which is the module's. (S)
  - A hand-edited file that isn't a set (bad JSON, an unknown type) is
    said only when it is added, in the log; the menu and the Variable
    Sets window list it by its file name as any other. (S)
  - No preview of what a set holds before it is added (the prefab
    palette's hover summary has no counterpart in the Add Set menu). (S)
  - No folders inside `variable-sets`, and Add from File… takes one file
    at a time. (S)
- **Update Instances and held items** (a builder): an item's blueprint
  now reaches the items that placed objects hold (chests, creatures'
  packs and equipment, stores' pages, bags in them), in the window and
  in `mg update-instances`. A held item keeps its place, its slot, how
  many there are (`StackSize`) and whether it drops, can be stolen or is
  endless in a store; its charges and the rest come from the blueprint.
  What Aurora's Update Instances does with held items was not
  captured. Items held in other blueprints (a
  creature blueprint's inventory) name the item's blueprint and are
  made from it when placed, so there is nothing to update there. (S)
  (from builders' reports after 1.19.1)

## Conversation authoring

- **Other languages in exports:** Twine, Ink and CSV carry the English
  text only. A CSV could carry a column per language for translators. (S)
- **Animations, comments and delays** don't go to Twine or Ink (script
  parameters do, since dansen9191's report). Parameters on a link with
  no condition script, or a line with no action script, are dropped (the
  game reads none of them); the CSV's condition and action columns name
  the script only. (S–M)

## Palettes

- **Names in order, accents aside** (Sacha's report, October 2026):
  the Category lists (Options › General), the module tree, the lists of
  a table's rows and the palettes' own order (Windows' word sort) fold
  case and the accents of Latin letters (Latin-1, Extended-A and B,
  Extended Additional: Vietnamese) and Greek's tonos (`mg_core::text`),
  so `Árboles` stands beside `arboles`; every Filter and Find of names
  (`mg_core::text::Needle`: the module tree, the palettes, the lists of
  rows, the resource browser, the 2DA editor, the script side list, the
  wizards, Edit Areas Together, `mg`'s area and object filters) finds
  `Árbol` for `arbol`, and the searches of text (Find and Replace Text,
  a script's Find, the conversation Search) set accents aside only
  without Match case; tags and `*` patterns (Find Instance, Advanced
  Search, `mg find`) too. Not done: Cyrillic's (ё for е) and polytonic
  Greek's marks (Greek Extended) are not folded. (S)

- **Palette categories against Aurora:** a module's own categories
  (Palette › Categories…) are a skeleton kept in the module, with names
  written out (`NAME`, and BioWare's `DELETE_ME`). That Aurora reads a
  skeleton from the module is a builder's report (Taro49, 2026-10-05:
  they keep their custom categories so), not a capture of ours; that it
  shows a name written out was not captured; the game's DM palette reads the
  `palcus.itp` built from it. (S, an Aurora capture)
- **Palette categories, what the editor lacks:** no translated names
  (one text for every language), and Remove refuses a category in use
  rather than offer to move its blueprints. (S–M)
- **A module's categories in its own order:** the palette and the
  `palcus.itp` built on save keep the order a module's own skeleton has
  (the game's are by name, as Aurora writes them). What Aurora makes of
  a `palcus.itp` in another order wasn't captured. (S)
- **The Gallery:** up to 192 pictures are kept at once (more are made
  again as they come back into sight) and all are made again when a
  blueprint changes. (S) (from builders' reports after 1.10.1)
- **A custom palette's names in the game** (a builder: after a save in
  Moonglow the DM's Creator showed "HipÃ³lito", and color tokens'
  colors wrong; Aurora's Custom palette showed the same, reading the
  same files): the custom palettes (`<type>palcus.itp`) carried their
  blueprints' and factions' names as UTF-8, and so did categories named
  in Moonglow; the game reads them as its text: Windows-1252 for English
  and the western languages, the language's codepage otherwise
  (Windows-1250 for Polish). They now carry the
  names' bytes as the module has them (not read and written again: a
  color token keeps every byte), and a category is written in the
  codepage of the game's language, refused where that cannot hold it.
  Read back, names show in that codepage (a Polish module's in
  Windows-1250, which also answers the palette half of GitHub issue
  13), UTF-8 ones from before still as they were meant. A module is put
  right by Build › Palettes (after 2.5.0 a save changes only the
  entries of blueprints changed: a name left in UTF-8 stays so until its
  blueprint changes), and categories left in UTF-8 are written in the
  game's bytes there and in the module's own category lists (a test
  spoils a saved palette as 1.19.3 wrote it, saves it unchanged and
  builds it). Not seen in the game's DM client here (the bytes are
  tested; the client was not run as a DM). Issue 13's other half: with
  Polish the editing language, String Edit read and wrote a string's
  English text as Windows-1252 (mojibake, and Polish letters typed
  could not be saved, under an error that named the String Ref). It
  reads and writes the Windows-1252 languages' text as Windows-1250
  then, as the lists did already and as a Polish game shows it, and
  says which letters a text cannot hold. Not seen on the reporter's
  module; with another editing language a Polish name kept in the
  English text is still read as Windows-1252. (S)
  (from builders' reports after 1.19.1)

- **The custom palettes kept, what is left** (since a save changes only
  the entries of the blueprints changed, as Aurora does:
  `update_custom_palette`, the captures in `palettes/`):
  - *Where Moonglow differs from Aurora, by choice:* a save doesn't put
    each category's entries in order by name or drop fields it doesn't
    know, as Aurora's does, so a palette Moonglow didn't change is the
    same bytes; a new entry for a talk-table name is its `STRREF` (as
    Aurora's Build writes it) where Aurora's Edit Copy and Add to
    Palette write the text; a module with no `palcus.itp` gets one made
    in full at a save, where Aurora writes the categories alone, empty.
    (S, if any should follow Aurora)
  - *An entry made stale outside stays so, where Moonglow didn't see the
    blueprint change:* Aurora rewrites an entry at Edit and OK even when
    nothing changed (it writes the blueprint again); Moonglow when the
    blueprint's bytes changed since the module was opened or saved. A
    blueprint added, changed or deleted in an open module folder or
    nasher project is such a change (read again, it is not taken into
    what the save compares with: Troubletcat's report, October 2026),
    and the palette changed so is unsaved work. An entry left stale by
    another tool while the module was closed, a merge, or an `mg`
    command on an open `.mod` (no archive is watched) is made right when
    its blueprint next changes, by the palette's More › Refresh Palette
    (that type's, as Aurora's right-click Refresh Palette: its capture
    changed the item palette alone) or by Build. Work recovered after a
    crash counts as opened: its blueprints' entries wait for a change
    too. A module's own categories (`<type>pal.itp`) changed outside are
    taken in as they are, without remaking the palette's categories
    (the palette pulled with them has them); a search typed doesn't open
    the category of a blueprint that arrives after it was typed (typed
    again, it does). (S each)
  - *A blueprint whose category the palette lacks* (a `PaletteID`
    another module's own categories name, or none): Moonglow leaves it
    out of the custom palette, at a save, a reload, Refresh Palette and
    Build alike, where the game's Special › Custom 1 (ID 0, in every
    type's palette) would show it; what Aurora's Refresh does with one
    wasn't captured. (S, an Aurora capture)
  - *A category left with an empty `LIST`:* an entry put into a
    category that had no `LIST` field makes one, and the entry taken
    out again (a blueprint added, then deleted, before a save) leaves
    it empty, where the file had none: the same palette to the game,
    not the same bytes. (S)
  - *What moves an entry besides its blueprint:* a faction renamed
    (`repute.fac`) leaves its creatures' `FACTION` until they change; a
    hak's skeleton changed, or a hak's blueprints changed, renamed or
    removed, don't touch the module's palette (the module's own
    skeleton changed remakes its categories, each with its entries; a
    category taken out takes its entries, a hak's too). What Aurora
    does with a creature whose CR or faction alone changed, and with a
    blueprint given category 255 or one the palette lacks (Moonglow
    takes its entry out), wasn't captured. (S, an Aurora capture)
  - *An entry put in a category out of order* goes after the last entry
    whose name doesn't come after it (in order, its place); Aurora puts
    the whole category in order at its save. (S)
  - *The command line:* `mg rename` renames a blueprint's entry in
    place, but `mg set`, `mg apply`, `mg import`, `mg update-instances`
    and plugin commands run from `mg plugin` change blueprints without
    touching the palettes; the next save in the GUI or a Build brings
    them up to date. (S–M)
  - *No command takes out one entry* whose blueprint isn't the
    module's (a hak's, the override folder's, one there is no more);
    Build takes out those of the override folder and the ones there
    are no more, and keeps a hak's. (S)
  - Not seen in the game's DM Creator (the files are compared with
    Aurora's). (S)
  (El Grillo's and Jasperre's report, October 2026)

## Custom content data (2DAs and talk tables)

- **`mg tlk-alloc`** (a custom talk table from strings kept by name):
  new keys take lines in the order of their names, not of the file; a
  key's feminine text is not set (the feminine table's lines stay as
  they are); a retired key's line keeps its last text in the table. (S
  each)
- **The 2DA editor** ([its proposal](2da-editor-proposal.md), built
  after 1.21.3: a table's cells in the module, a hak or a file, checked
  as typed, with rows taken from another copy). Not done:
  - A model's cell has no list to choose from and is not checked by the
    row; a talk-table string's cell offers the talk table, not a new
    line in it. (S each)
  - Find References to Row looks in the tables' known columns, not in
    blueprints, areas or scripts. (M)
  - A row both copies changed is taken whole or left: its cells are not
    chosen one by one. (S)
  - A table saved is laid out anew, columns in line or compact as its
    file was (its own widths, its tabs and its row labels are not
    kept: rows are numbered), and an empty row at the end is not
    written. `mg 2da-diff -o` and a plugin's `twoda:write` write
    columns in line only. (S)
  - A table open in the editor is not read again when what it was read
    from changes under it (the module's Undo taking its save back, the
    hak's table replaced, the file changed outside). (S)
  - Column widths (dansen9191's report, 2026-10-21) are kept by the
    table's name and the column's, in the settings (the last 200
    tables): two different tables of one name (two haks' `custom.2da`)
    share them, and there is no way to put a column back to its first
    width but to drag it there. The resource browser's **From** column
    (the layer a row comes from) keeps its width. A double click on an
    edge fits the column to its rows as shown then (Find's rows, the
    strings as text or not), not to all of them. (S each)
  - The resource browser's read-only view (with its scroll bars since
    2026-10-21) has no keys of its own: no cell to select, nor the
    arrows to move with, nor Find. It is drawn by code of its own beside
    the editor's grid, which it copies (the column names and rows'
    numbers that follow the scroll); one grid for both, with the editing
    left out, would serve. (M)
  - Large tables were not timed while edited (a persistent world's
    `appearance.2da`); Find looks through every cell at each frame
    (Take Rows reads the copies once, and works the rows out once for
    a copy). (S)
  - No row is put between others, by decision (Delete Row takes any
    row away since 1.22.1); no sorting, no `DEFAULT:` edited. (—)
  - Asked for by builders on 1.22.0 and 1.22.1 (October 2026), none
    begun:
    - **Columns hidden** from view. (S)
    - **A table's description** (`name.2da.toml`: what each column
      holds, lists from a script's constants, `mg 2da-check`; built
      after 1.22.1 from a builder's workbook,
      [the proposal](2da-editor-proposal.md#columns-that-say-what-they-hold-a-builders-spreadsheet-read)).
      Not done:
      - One description for a family of tables (`loot_*`); the game's
        own tables described the same way. (M)
      - Verify Module reads the descriptions of the module's own
        tables, not those of its haks' (`mg 2da-check` on the hak's
        sources does). (S)
      - A constant whose value is a function's result is left out of a
        list, and said; one made of constants of another script it
        includes is too. (S)
      - A description's folder is walked when it is read (once, and
        again when the file changes; the folders a build fills are
        passed over): a second and a half for a repository of 144,000
        files in a build made for testing, during which the window
        waits. A table of the project that a description names is read
        from its file once, not as it is open in the editor. (S)
      - Without a game install, `mg 2da-check` knows no talk table: a
        `strref` column's custom string is said not to exist; a
        plugin's `faults` does not look strings up at all. (S)
      - A hak's table's description (beside its file in the hak's
        folder) was tried beside the hak's file, not in a folder a hak
        is built from. (S)
    - **A talk-table string chosen from the talk table** in the cell,
      not gone to. (S; with "a new line in it" above)
    - **A class's spell tables checked against `spells.2da`** (each
      spell a class's table names is one the class can learn): Verify
      Module holds the lists a class names to rows that exist, not to
      what those rows allow. (M)
    One builder's view, to weigh the rest by: a spreadsheet does most
    of this already, and the editor earns its place on a module's own
    tables (treasure, spawns), not on the game's.
  - The keys: a letter typed does not begin typing in the cell (Enter,
    F2 or a double click does); a drag selecting cells does not scroll
    the table when it reaches its edge. (S each)
  - Edit… › The Table of a hak, in the resource browser, was tried
    through the functions under it (the hak opened by its file, its
    table opened in it), not by its menu on a module with a hak. (S)
  - The hak editor's Edit Table and the Take Rows window against two
    haks' copies were tried through the functions under them and with
    the game's copy, not by their own buttons. (S)
- **Talk tables in a hak or the module** are read-only: Moonglow edits
  only tables in the `tlk` folder. A table kept in the module could be
  edited and saved with it. (S)
- **Talk tables from a terminal:** `mg tlk-export` and `mg tlk-import`
  write and read a `.tlk` file as JSON or CSV. The JSON is `nwn_tlk`'s
  (compared with it: `apps/mg/tests/tlk_io.rs`, which found the sound
  length named `soundLength` where `nwn_tlk` has `soundLen`; both are
  read). Not done: a table made by an import is English; the text is
  not read by a module's `encoding.2da`; the JSON carries the main
  table only, as `nwn_tlk`'s does, and so does the talk table editor's
  Export JSON. (S each)
- **Where a StrRef is used:** Find References takes a talk-table line
  (a number typed in its field, or Find References in the talk table's
  editor) and lists the module's localized strings that name it
  (blueprints, areas, conversations, the journal) and its 2DAs' cells
  in the columns known to hold StrRefs. Not done: the haks' 2DAs are
  not looked in (they are not the module's), nor fields that hold a
  StrRef as a plain number, nor scripts (`GetStringByStrRef`). (S each)
- **The client and custom talk tables:** the engine test runs the server;
  whether the client also reads a table from the module (not only from
  haks and the `tlk` folder) for 2DA text isn't tested. (S)
- **Text put in the talk table as it is written** (a conversation's
  lines, names), in a range set aside for it, and reusing a line that
  already has the same text, asked first: proposed by builders, not
  begun. Move to Talk Table does one string at a time. (L)
  (from builders' reports after 1.19.1)

## Haks in the GUI

- **What the module uses of its haks** is `mg haks --usage` alone: the
  window has no report of it (Module Properties' Check for Conflicts
  could show it). The walk is in the command line (`apps/mg/src/usage.rs`)
  until the window wants it. (S–M)
- **Not followed by `--usage`:** what scripts reach by a number
  (visual effects, spells, item properties, `SetCreatureAppearanceType`),
  a creature's footstep, weapon and combat sounds, an item property's
  visual effect, armor's inventory icon, what players choose at
  character creation (heads, portraits, sound sets, parts), and 2DAs the
  game reads by itself (judged only where the walk reads one). A
  script's string counts as a use of whatever it spells. A per-hak list
  for the release cut (a folder of used assets to pack smaller haks) is
  not written: `--json` has the lists. (M)

- **No zip, 7z or rar:** content downloaded as an archive is unpacked
  first; Add Haks and Talk Table takes the haks and talk table. (S; `zip`
  is a dependency already, 7z and rar would each be a new one)
- **Viewing in the hak editor** shows fields, text, a compiled model as
  its text, or the first bytes: no pictures or sounds, a model not as a
  picture, and a 2DA as its text. (S)
- **Update from Folder** takes the folder's files in place of the hak's
  (two undo steps: the removal, then the adding); a hak that was built
  from several folders, or added to by hand, loses what the folder
  hasn't. (S)

## Raw fields

- **Rows the fields view doesn't name yet:** an item property's subtype,
  cost and parameter (their table depends on the property), a creature's
  or object's faction (the module's `repute.fac`, not a 2DA), a
  conversation's animations, a store's categories, and fields that hold
  a talk-table number as a plain int. (S–M)
- **Only in the window:** `mg gff`, `mg find --where` and `mg areas`
  print and take the numbers; names aren't shown or accepted there. (S)

## Getting files out

- **Overridden copies in the resource browser** open read-only as their
  layer has them (fields, tables, text, a model as its text), copy to
  the module and save; the model viewer (Preview) shows only the copy
  the game reads, Export as Files writes only those, and a plugin's
  selection sees a copy's tab as nothing. (S) (a builder's report on
  2.0.0)

- **Export into an archive** is a checkbox, off unless ticked; a builder
  says Aurora adds to an existing archive itself. Aurora's own behavior
  (whether it asks) was not captured. (S)
  (from builders' reports after 1.19.1)

## Area visibility

- **What the toolbar hides, kept between runs:** the kinds hidden in the
  area views are kept in the settings, as Lighting and Sound Ranges are,
  so an area can open looking empty in the next run; Aurora was not
  checked for whether it keeps them past a restart (they stay from area
  to area there, the builder's report says). Set Start Location shows
  the marker again in every view. (S) (Sacha's report, October 2026)

- **Grass in patches:** the client's tufts gather in patches across a
  field (dark clumps, seen in `client_render.rs` grass_overrides_look);
  Moonglow scatters them evenly by face area and density. The mean color
  near the camera matches; how the engine places them (per face, by a
  noise) isn't known. (S–M) (Bee AI's ask, October 2026)
- **A grass tuft's shape:** one upright quad turned at random, a texture
  quarter, as tall as wide; the client's geometry (crossed quads, a lean,
  its sway in the wind) wasn't read, and Moonglow's tufts don't sway. (S)
- **Aurora and an area's grass:** Aurora has no grass page; whether it
  keeps `GrassDefDisabled` and `GrassOverrides` in a GIT's
  `AreaProperties` when it saves the area wasn't checked (a module saved
  in Aurora may lose them). (S)
- **Grass on placeables:** grass is grown on tiles' walkmeshes only;
  whether the game grows it on a static placeable's walkmesh (merged into
  the tiles) wasn't checked. (S)
- **The sun's and moon's directions in an area's GIT:** the server reads
  a module's GIT's `AreaProperties` `SunDirectionX/Y/Z`,
  `MoonDirectionX/Y/Z` (kept into a saved game) and `DisplayName` (the
  area's `GetName`), not the root's `CurrentWeather`
  (`engine_git_runtime.rs`). Whether the client lights the area by those
  directions, and so whether the area view should, isn't known. (S)
  (waits on the client batch: an outdoor area at noon whose GIT's
  `SunDirection` is low from one side, against the same area without,
  its walls' lit sides and the shadows compared)
- **Walkmesh cutters:** not drawn, and not placed (the trigger kind EE
  added). (S–M)
- **A walkmesh's own node position:** one shipped placeable
  (`ptm_candle02.pwk`, Tyrants of the Moonsea) puts its mesh 72 m from
  itself. Moonglow draws it there; whether the engine does isn't known.
  Tried in the server (2026-10-29, a probe not kept): a creature made
  where the mesh would be, at the placeable or 72 m off, is not moved at
  either, where one made on a wagon is moved off it; with the module's
  own copy of the walkmesh twenty times as wide (3.4 m), the same. So
  the engine ignores this mesh, or a module's copy of it, or making a
  creature doesn't see it: telling which needs a creature's path across
  each place, or the client. (M)
- **Lighting off against Aurora:** the Lighting switch's working light
  (white, even, a little from the sun's side) is Moonglow's; how Aurora
  lights an area with its lighting off was not captured. (S)
- **Marker models against Aurora:** stores, sounds and waypoints use
  the game's marker models in their materials' colors, unlit; that Aurora
  draws them so was not captured. Waypoints' flags are drawn unlit so
  their colors tell apart in a dark area; how Aurora lights them was
  not captured either. (S)
- **Sound ranges:** level circles at the sound's height, not spheres; a
  sound's random position range is a rectangle of `RandomRangeX` and
  `RandomRangeY` each way (that the game takes them as half-widths about
  the sound was not measured). (S)
- **The minimap in Moonglow:** the area view has no map panel of its own,
  and Area Transition setup picks its target from a list rather than a
  map. (S–M)
- **Fade Geometry:** Never, Object Mode Only and Always, as Aurora
  names them; Moonglow's object mode is "not Select Tiles and no terrain
  brush", which wasn't compared with when Aurora fades. That the meshes
  with `tilefade` 2 (the black caps) stay while those with 1 and 4 go
  was taken from the game's tiles and their names, not compared with
  Aurora or the game. (S) (from builders' reports after 1.10.1)
- **A cloak lying in an area** is the game's model of a cloak dropped
  (`gi_cloak01`, folded, its PLT texture in the cloak's colors), as
  Aurora and the game draw it; Moonglow drew the bag (the cloak's
  `DefaultModel`, which neither uses for it). Compared with the game
  client (`items_look` with three of the game's cloaks: the same model
  and colors, lying the same way). Where the model is missing: the
  `DefaultModel`, then the bag. Whether a custom base item of cloaks
  that names a model of its own should lie as that instead was not
  measured (the game's own names the bag, which it does not use). (S)
  (from builders' reports after 1.19.1) It has the environment map it
  has worn (its cloakmodel.2da row's; dropped armor the items' one):
  without one a shiny color's palette alpha was drawn see-through, the
  floor showing through (a builder's report after 1.32.1, solid and
  shiny in Aurora; seen in `area_items` before and after), and a color
  whose alpha is 0 (61) not drawn at all. Compared with Aurora from
  above (`area_dropped_cloaks`, and Aurora's own view of the same probe
  module, `captures/items/`: the game's cloak, colors 0, 56 to 59 and
  61): the same shapes, colors and solid shine. Aurora's reflection is
  a little bluer and brighter (both `chrome1`, cloakmodel.2da's
  `default`); not compared with the client. (S)
- **An armor lying as plate here and as a tunic in Aurora**, in the
  same builder's picture: not looked into. The model is chosen by the
  torso's parts_chest.2da `ACBONUS` (measured with the game's own
  armors); his armor, or his content's table, may be a case that rule
  misses. Needs the item. (S) (from builders' reports after 1.19.1)

## Editing in the area view

- **Visual transforms in Aurora's view:** xtul saw his mirror upright
  and at full size in Aurora, which was not run on it: whether Aurora
  draws a visual transform at all, and its rotations as the game does,
  is unchecked (an Aurora capture would settle it). (S) (xtul's report,
  October 2026)
- **Static and the transform elsewhere:** ticking Static takes a placed
  placeable's visual transform off in its Properties (with others
  edited together) and Make Static, as Aurora does. Not so: a plugin
  setting `Static`, `mg apply` and `mg set` (fields set as given),
  and Update Instances, which keeps an object's transform whatever its
  blueprint's Static (Aurora's Update Instances was captured on dynamic
  objects only). Whether Aurora takes it off when Static is ticked on a
  blueprint was not looked at (a blueprint has none from Aurora). (S)
- **Lerps, animation speed and other scopes:** Adjust Location edits
  scope 0's scale, rotation and translation only; a transform's
  `AnimationSpeed`, a lerp in progress (`ValueFrom`, `LerpDuration`…)
  and a creature's head, wings, tail or cloak scopes are kept but not
  shown, and the area view draws scope 0 only. (M) (xtul's report,
  October 2026)

- **Alt for the camera and the selection:** Alt + right drag turns the
  selection and Alt + middle drag moves the camera (they were Shift's,
  which now leaves the camera alone while the handles show). Aurora
  turns the selection with Shift + right drag; and some desktops take
  Alt + drag to move the window. Not in Options › Keyboard. (S)
  (from builders' reports after 1.10.1)
- **Painting placeables** (the area view's Paint, from builders'
  reports on 1.31.0) is in: Scatter, Row and Fill (a rectangle in rows
  on a grid Q and E turn, each piece by its own size, a builder's ask
  after 1.32.2). Not done: a fill's rows of pieces of different depths
  lined up by their middles rather than their near sides; an eraser
  (taking the chosen blueprints' placeables out of the circle); the
  brush's own Lock (painted placeables locked, so later clicks pass
  through them); counting placeables that aren't the chosen blueprints
  towards Density (Spacing keeps away from them; Density counts only the
  brush's own); Spacing from each placeable's size rather than one
  distance; a row along a path of several legs (a drag is one straight
  line); random scale on static placeables (the game gives them none, so
  Scale passes them by). The see-through dab is held by the tests'
  counts; the pictures show strokes laid. (S each)
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
  copy the load order puts over them, with each shader's color: that
  line from the next release says which of the two it was. (S) (Heed's
  reports, October 2026)
- **A custom shader's tangents, not every case read:** the client's
  rule (tangents from the model or a render hint, else zeros) was read
  with an ASCII model without tangents, with and without an MTR
  `renderhint`; a model that names `texture1` itself without a hint,
  and a compiled one, under a custom pair, weren't tried, nor the black
  of a normal map without tangents on Windows and macOS (the stock
  include normalizing a zero vector). (S) (Heed's report, October 2026)
- **A texture's `envmaptexture default` on a placeable whose
  `Reflection` is `default` too:** Moonglow takes the placeable's map
  (`chrome1`) for both; the client wasn't asked (the armoire, read, has
  no `Reflection`). (S)
- **Lights placed on the fly** ([the proposal](lights-proposal.md)):
  way A is in (the game's seven invisible light placeables from the
  area's menu, a sun for a marker, Light Color, a ring for the reach).
  The reach is measured in the client (`placeable_light_uniforms`): a
  light of radius 10 in the lightcolor.2da color, ending 20 m out
  (further for a color brighter than 1), static or not, as Moonglow
  drew it already. Not in: a palette entry beside the menu, and way B (any color and reach, by generated content), which
  needs generated 2DA rows merged into the module's tables (`mg 2da-diff
  --base` merges from a terminal; nothing in the window does). An
  object whose model is too small to see
  (lying flat and under half a metre across, or empty) is now drawn and
  picked by a marker's box, lights and the Invisible Object alike. (M)
  (Small models with height, a potion, a gem or a healer's kit, were
  taken for it too and had a marker's metre-wide box: a builder's
  report after 1.32.1.)
  (from builders' reports after 1.16.2)
- **Ctrl + wheel scales the selection** (a builder: Aurora's way to
  resize a placeable, which did nothing here but zoom): the models of
  the objects selected that take a scale, shown as the wheel turns and
  one command when Ctrl is let go or the wheel rests. Not measured in
  Aurora: how much a notch scales by there (here about a twentieth,
  multiplied), and what its Ctrl + wheel does with nothing selected that
  takes a scale (here the slow zoom it was). A static placeable takes
  none and the status line says so. A turn under way is made a command
  before a save; saved within a moment of the wheel's last notch, what
  is left of its smoothed turning scales a hair more after. (S)
  (from builders' reports after 1.19.1)
- **Tile Properties' Defaults:** not yet compared with Aurora for a
  lighting scheme whose colors aren't black
  (`docs/research/notes_tilesets.md`). (S) (from the research notes)
- **Delete takes a whole group by default,** where Aurora's Delete takes
  the tile alone and leaves the rest standing (the option gives
  Aurora's). Pasted tiles over a group are refused as a group is; how
  Aurora pastes over a group wasn't checked. (S) (from Jasperre's tile
  feedback, October 2026)
- **Exact tile's reach:** the Gallery shows a terrain's and a crosser's
  tiles, not a group's pieces, so a piece of a group goes in alone only
  through Tile Properties' All tiles (the Features and Groups pictures,
  in the same Gallery as the switch, are placed whole whatever it says:
  its tip and the manual say so, the pictures don't); an exact tile takes the height of
  the tile it replaces (no height to choose in the palette: Tile
  Properties' Height after). Where an exact tile and its neighbor
  disagree about a shared corner, the brushes read the corner as the
  tile to the north or east has it (`Lattice::from_tiles`), so a stroke
  beside it may choose it again. How the client draws the seams was
  looked at in Moonglow's view only. (S) (issue #29)
- **Tile Height and what stands on a tile:** an object goes up or down
  with a tile whose square its position is over (doors on its hooks,
  placeables, creatures, waypoints, a trigger's or encounter's position
  with its outline), whatever its height above the tile; Raise/Lower's
  corner strokes still move nothing. Heights below 0 and far above were
  stood on in the server (`a_tile_s_own_height_stands_where_the_engine_puts_it`),
  not looked at in the game client; the control's ±1024 is Moonglow's
  reading bound, not the engine's. (S) (issue #29)
- **Tile Properties on several tiles, in Aurora:** Moonglow writes only
  the controls set into every tile chosen (2.4.0's OK wrote every shown
  value, the first tile's, into all of them); how Aurora writes them for
  several tiles was not looked at on the oracle: it waits on a session
  driving Aurora. (S) (the owner's report on Tile Properties)

## The turning ring

- **Escape during a drag** drops it now (it used to put the object down
  where it was); Aurora's behavior was not checked. (S)
- **The scale handle** scales every way alike, as Adjust Location's
  field does; the file keeps a scale for each axis. It stands where it
  was taken while it is dragged, rather than follow the pointer. (S)
  (from builders' reports after 1.10.1)

## Rendering: creatures, particles, water and shaders

- **A placeable without its state's animation, in the client:** a
  placeable whose model (and supermodels) lack the animation of its
  state (`default`, `open`…) is drawn in its rest pose, as a corpse
  placeable of CEP's on a creature's supermodel (`plc_orc2`) lies in its
  Properties and, by every account, in the game. Not yet seen in the
  client (`client_render.rs`) with the hak; the wiki doesn't say. A
  creature's idle renamed by AnimationReplace to one its model lacks
  stands in its rest pose too now, where it stood in `pause1`. (S)
  (from builders' reports on 1.31.0)
- **Dynamic body parts from custom content (GitHub issue 5):** fixed
  with the reporter's haks over CEP 2.71. Under armor a creature's own
  part shows where the armor's is bare skin (part 1) or none, as the
  game shows a pale master's arm; the rule is taken from that and from
  the reporter's Aurora screenshot, which the previews now match, not
  measured in the client (nor what the game does for armor part 0 over
  a body part). A model whose appearance says full-body but whose
  skeleton has a creature's animations stands in `cpause1` (the
  werebat stood in its rest pose). Left: the report's werejackal wears
  a helmet whose model (`helm_129`) is in neither download, and is
  drawn headless; what the game draws for a missing helmet model isn't
  known. (needs the file) (from builders' reports after 1.10.1)
- **Emitters against the client:** an emitter new to the view, an
  object's or a tile's, is run twenty seconds ahead so that it shows as
  it does once going (`scene.rs`, `PARTICLES_AHEAD`), and the nearest
  256 objects and 128 tiles with emitters are simulated while Animations
  is on (`nearest`). Not compared with the client: that a wide fog
  (eight slow particles a second) is as dense from above as the game's,
  and Magic Sparks, which show faintly from above. A tile's emitters take
  their keys from the animations playing on it as one animation as long
  as the longest (the game's tiles' keys are constant; one that changed
  over a shorter loop would run slow). (S) (waits on the client batch:
  the fog placeable and Magic Sparks seen from above in the client and
  in the area view, side by side) (from builders' reports after 1.14.1)
- **The game's shaders, compiled (M0 of [the shaders
  proposal](shaders-proposal.md)):** what the compiler (`mg-shader`) and
  Verify's `shader-compile` check leave out. (from M0, 2026-10-21)
  - Verify compiles each program once, as the game at its highest
    settings has it (`Preamble::default`: quality 2, enhanced lighting,
    32 lights, keyholing on, `NO_DISCARD 0`). A program that fails only
    at low or medium quality, with enhanced lighting off
    (`FRAGMENT_LIGHTING 0`), for an opaque material (`NO_DISCARD 1`) or
    with another light count isn't reported. Nor is it compiled again
    per Verify run cheaply: each run compiles every custom program (TDoD
    with its shader haks: about a second). (S)
  - Arrays of samplers are refused (not split), and `customShaderGS`
    (a geometry shader) is not read; no shader seen has either. (S)
  - What the engine does is assumed in three places, none checked in the
    client: an indented `#include` is spliced (14 of the stock ones
    are), a missing include fails the program, and splicing stops 16
    deep. (S)
  - A uniform that is a struct is described as `Other`, without its
    members (none in the stock blocks). (Locations, which glslang numbers
    in the order a stage first uses them, are settled in M1: vertex
    inputs bound by name, varyings matched by name.) (S)
  - The C++ build: glslang is in every program and test binary that
    links `mg-module` (about 40 s of C++ on a cold build, once).
- **The game's lit shaders drawn (M1 of [the shaders
  proposal](shaders-proposal.md)):** what `Renderer::game_shaders`
  leaves out (the Shaders switch, on by default since M3; Moonglow
  Viewer draws them since its 0.2.0). (from M1, 2026-10-21)
  - **The environment map's level under a normal map:** the WGSL port
    takes `_nm`'s derivatives from the normal map's normal before the
    parallax and the bend of a normal facing away, which the game takes
    after (2 of 255 at the 99th percentile). (S)
  - `gl_FragCoord.y` counts from the top in wgpu and from the bottom in
    OpenGL: counted from the bottom (in IR) only in a program that reads
    the framebuffer's copies (`fswater`, `fsparticle`), whose size gives
    the height. Of the other stock ones keyholing (off),
    `fs_shadowplane` and `fs_cg_btn_col` (not drawn) read it, and none
    of The Domains of Dread's 37. A screen-space custom shader that
    reads no copy would see it upside down; its height would come from
    `screenParams`. (S)
  - The height map's parallax direction on a tile's `rotatetexture`
    meshes (`inc_displacement` turns it by `mat2(m_texture)`) is not
    turned in Moonglow's own shader (`displace`; the normal map's is).
    (S)
  - `worldtimerTimeOfDay` counts real milliseconds and
    `sceneCurrentFrame` frames (measured, notes §B.4); Moonglow's clock
    starts both at 0 when the scene's time does. The client's timer has
    an origin that follows the module's minutes an hour (about 424,800
    a few seconds in at 2 minutes an hour, 3,304,900 at 10) and neither
    its hour nor its day: not worked out. It is the phase of the
    falling water's flow and the grass's sway, which are not compared
    with the client frame by frame until it is. (S) (waits on the client
    batch: the timer read back in the first shot, as
    `time_uniforms_in_the_client` reads it, at three or four minutes an
    hour and start hours, for the rule)
  - The game's programs draw on CI's Metal and Direct3D 12 (the
    renderer's CI branch, 2026-10-29: three draws, no failures), but a
    discarding one's depth is not tested there: naga's `discard` from
    the game's, and the alpha discard `keep_alpha` adds (a bare `Kill`),
    run on, and on Metal a discarded fragment that runs on writes its
    depth (`a_shader_returns_where_it_discards` holds Moonglow's own
    shader to returning). (S: a see-through mesh's solid part through
    the game's programs, as `a_see_through_edge_does_not_hide_the_ground_behind_it`
    draws Moonglow's, on the renderer's CI branch)
- **Tier A of the game's shaders (M2 of [the shaders
  proposal](shaders-proposal.md)):** what the Shaders switch leaves out
  or unchecked. (from M2, 2026-10-21)
  - **The framebuffer's copies** (`texFBColor`/`texFBDepth`, [the
    framebuffer proposal](framebuffer-proposal.md)) are made after the
    solid meshes only. A material's `sample_framebuffer` (TDoD's two
    flow MTRs; their programs read no copy) is read and not drawn late,
    after the shadows, as the engine draws it (S); whether the client
    copies again after the water (a see-through mesh or a particle over
    water sees the solid meshes only) wasn't read (S) (waits on the
    client batch: a see-through mesh reading `texFBColor` over water,
    whether the water shows in its copy).
  - The tangents the renderer makes (a mesh with a render hint and none
    of its own) are summed per face (`made_tangents`), where the game's
    compiler copies a vertex whose handedness flips (`engine_tangents`,
    MDL-MESH-043): whether the client makes an ASCII model's so at load
    is not known, and a seam may shade otherwise. (S) (waits on the
    client batch: an ASCII mesh with a mirrored UV seam under a render
    hint, its tangents written as colour through a patched include,
    beside the same model compiled)
  - `areaGlobalWind`'s direction is taken from TDoD's shader comments
    ((0.707, 0.707, 0) for a medium wind), not read from the client. (S)
    (waits on the client batch: `areaGlobalWind` read back in an
    outdoor area at `WindPower` 1 and 2)
  - Verify's `shader-pair` reports a material naming one program alone;
    whether the engine also ignores a lone `customshaderGS`, and what it
    does with a geometry program at all, is not checked (no content has
    one; Moonglow fails one, reported, and draws its own). (S) (waits on
    the client batch: an MTR naming `customshaderGS` alone, and one
    naming it beside a pair, on a card)
  - Pictures already made (the galleries' thumbnails, a hovered row's
    preview) are kept across a flip of the switch, drawn as they were,
    until they are asked for anew: `set_game_shaders` changes the
    renderer only. (S)
- **The frame budget of the game's shaders (M3 of [the shaders
  proposal](shaders-proposal.md)):** what the Shaders switch, on by
  default, leaves over the budget or unchecked. (from M3, 2026-10-21)
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
    note](research/notes_frames.md), 27 October) (M)
  - **A program that moves its vertices farther than a meter** is batched
    by its box grown by `MOVED` (1 m): TDoD's grass lifted by
    `fTilesetMeshOffsetZ` (2 m) could be blended in another order where
    it reaches past it. Its MTR's parameters could say how far. (S)
  - **An area changes its look a moment after it opens**, the first time
    its programs are compiled (stock: 50–113 ms of the worker's; TDoD:
    250–277 ms) and, with them cached, while naga reads them and the
    driver makes the pipelines (28–40 and 123–146 ms): Moonglow's own
    shader draws until they are ready. Making the pipelines of the
    programs a module needs when it opens, or keeping the drivers'
    pipeline caches (wgpu's `PipelineCache`), would shorten it. (S)
  - `mg` and `mgv`'s previews compile the programs each run: no shader
    cache is set outside the window (`Renderer::set_shader_cache`). (S)
  - A program whose uniforms can't be split (its block used otherwise
    than member by member: none of the 102 seen) falls back to
    Moonglow's shader, reported, rather than drawing with one block. (S)
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
  client. (from M4, 2026-10-21)
  - **A static placeable's meshes** sharing a texture seemed to share
    one `NO_DISCARD` in the client (the plain and transparency-hint cards
    of a static copy were alpha-tested beside an `alpha 0.5` card of the
    same texture): the game bakes static geometry into buckets (.37).
    Moonglow decides it per mesh. Not worked out: which meshes share a
    bucket, whether tiles' do (they are baked too). (S) (waits on the
    client batch: the cards as a static and a dynamic placeable, and on
    a tile)
  - A BioWare DDS's alpha is taken from its header's alpha mean (below
    1: alpha), not read from the client: 22 of the game's 3,717
    four-channel ones have a mean of 1 (roofs, bark, flagstones) and are
    drawn solid. (S) (waits on the client batch: one of the 22 on a card
    with a transparency hint, cut or solid)
  - Not read in the client: which map an item on the ground reflects (it
    draws `_sm`, as Moonglow's `chrome1` has it; the band didn't reach
    it), what `cameraFocus` holds (read back as (34.31, 20, 0) once). (S)
    (waits on the client batch: both through a patched include)
  - Moonglow's own shader draws as the client does since M4 (Settled),
    not yet looked at in the builders' areas: TDoD's DXT1 textures (690)
    with their transparent texels black, and its 21 materials naming
    maps without a render hint. (S) (waits on the client batch: a TDoD
    area with both, beside the client's picture)
  - `program_choice_in_the_client` reads the cards by where they should
    be on the client's picture, so a card behind something reads as
    that: the cards stand clear of the tcn01 tile's tree, and one
    straddling its cut (DXT5 alpha) isn't read. (S) (waits on the client
    batch: that card moved clear and read)
  - Against the client, the switch on is as close as off on the stock
    scenes and closer on TDoD's room, but for its far floor (1.0 of 255
    from the client on, 0.4 off), beside a dark wall where the fitted
    camera is a fraction of a pixel off. (S) (waits on the client batch:
    the room with the camera fitted to that wall)
  - `time_uniforms_in_the_client` fails now and then: the client's
    window, screenshot every three seconds, gave a picture it had shown
    before (the same frame count), leaving two of five. Alone it passes.
    Shooting until three distinct frames would hold it. (S)
- **The special surfaces (M5 of [the shaders
  proposal](shaders-proposal.md)):** water, grass, the sky, additive
  meshes and particles, read in the client (`surfaces_in_the_client`)
  and drawn with the game's programs or kept Moonglow's. (from M5,
  2026-10-21)
  - **Shiny water is the game's** (`vswater`/`fswater` with the
    framebuffer's copies) with the Shaders switch on: a rural area's far
    water 10.9 of 255 from the client against Moonglow's own 12.6,
    TDoD's fountain 10.6 against 11.7, a standing card 14.6 against
    57.1. Still 10.9 away: not looked into. (S) (waits on the client
    batch: the far water's pixels against Moonglow's, by part: the
    reflection, the refraction's copy, the fog)
  - **Soft particles** fade as the client's with the Shaders switch on
    (`particleSoftness` 10 over the emitter's larger size); what the
    client gives lightning and `Linked` emitters wasn't read (Moonglow
    gives lightning none); a Lighten particle's foot fades a little more
    than the client's (6.2 of 255 away over its last rows). Moonglow's
    own particle shader (the switch off) doesn't fade. (S) (waits on the
    client batch: `particleSoftness` read through the marked
    `fsparticle` for a lightning and a `Linked` emitter)
  - **The sky's fade** (the horizon's band in the fog's colour) stays
    Moonglow's: what the engine draws it with and gives it
    (`globalColor`, its blending) wasn't read; the skybox is the game's
    `vsvt`/`fst`. (S) (waits on the client batch)
  - **How the engine blends an additive mesh** (one and one, or by its
    alpha) is not read: Moonglow adds its colour whole by either path;
    the same for an opaque texture. A Lighten particle is added by its
    alpha. (S) (waits on the client batch: an additive card whose
    texture's alpha runs 0 to 1, over a gray ground)
  - **Particles' size and turn:** a still emitter's square particles
    are drawn upright, as in the client, 28 pixels a side against the
    client's 30 in `surfaces_in_the_client` (the fitted camera's error,
    or the size: not told apart). A spinning particle's starting angle
    wasn't read. (S) (waits on the client batch)
  - **The grass** is the game's program on Moonglow's tufts: their
    places, sizes and count are Moonglow's (the field's mean colour 0.1
    of 255 from the client's, but per pixel 17.6 against Moonglow's own
    17.0). How the engine fades its grass in, and the material colours
    it uploads (`.set` [GRASS]'s), are not read. (S) (waits on the client
    batch: both through a patched `vsgrass`)
  - **Cube maps have one level** (`GpuTexture::upload_cube`), so the
    level Moonglow's shader works out for them (`cube_level`) and the
    bias the game's programs add come to level 0 either way: a rough
    surface's cube reflection isn't blurred. The stored levels would be
    uploaded. (S)
  - **The switch on still costs the water view about 0.9 ms of
    encoding** (3.6 ms against 2.7 off, at 6.5 ms a frame within the
    budget by 0.4): about 0.75 ms the draws' own values (the material's
    template copied, `m_mv`, `m_mvp` and `m_normal` with an inverse each
    draw), the upload 0.09 ms, the submit 0.07 ms. `m_normal` from the
    3×3 part, or values that the camera alone changes kept per instance
    and multiplied in the vertex program, would cut it. (M)
  - The area views saved with `FRAME_SHOTS=1` are not comparable from
    one run to the next where tiles animate (up to 31 of 255 over
    area092's sea): they are drawn at the running clock, where
    `FRAME_PARTS_SHOTS` draws at a fixed one. A fixed clock for the
    shots would make them comparable. (S)
- **Emitters a builder does not see** (GitHub issue 8: a demon's wings,
  braziers, shafts of light, all without particles in his area): not
  reproduced. The game's own balor, fire elemental, campfire and brazier
  show theirs in the area view here (`look_particles_of`), so it is his
  content, a setting (Animations off shows none) or his machine; asked
  of him. (?) (from builders' reports after 1.16.2)
- **A `Linked` ribbon and a flip-book** (a builder's custom fog was
  drawn as bright streaks): what the client draws is in [the
  findings](findings.md) ("Particles: kept, flip-books and ribbons"),
  and the fog shows as an even haze, as in the game. Not looked at: the
  turn of the picture within a ribbon's piece (the client's looked
  mirrored beside a plain particle's), and how the client orders a
  ribbon's pieces among other see-through things. A model with no mesh
  to draw (this fog) has no bounds of its own to frame a view by. (S)
  (from builders' reports after 1.19.1)
- **Client tests' scripted cameras:** the client takes a
  `SetCameraFacing` only when it is set a second after the player
  enters ([findings](findings.md), "An upright particle turns to the
  eye"; the module's `Mod_Entry_Dir_X` and `_Y` turn it too, as the
  program-choice tests do); `particles_look`'s closer camera,
  `grass_and_sky_look`'s
  `MG_CLOSE` and `grass_overrides_look`'s `MG_DISTANCE` and `MG_PITCH`
  set it at once, so their pictures were likely taken from the client's
  own camera. Not changed: the numbers measured from them (in
  `docs/research/notes_models.md`) would be taken again with it. (S)
- **Builders' reports after 1.21.0:** a creature's body part can be
  set to none (0), as in Aurora; a body part whose mesh is a skin is
  drawn as the client draws it ([findings](findings.md), "A skin mesh
  in a body part or a held item": not where it is compiled, a plain
  mesh where it is text; a held item's never). Not done: a skin
  weighted to nodes of its own part, which the client draws out of
  place, is not drawn; the compiled models tried were `mgv compile`'s,
  not the game's own compiler's. Not reproduced: "unequipping an armor
  sets all body parts to 0": nothing in the code writes a body part on
  unequip, and the preview draws the creature's own parts then; the
  creature it was seen on would tell. (S)
  (from builders' reports and the owner's notes after 1.21.0)
- **See-through meshes** are drawn in two parts: what is nearly opaque
  of them (alpha from 0.95) with the depth written, then the rest over
  everything solid, back to front, writing none. (A builder's report:
  blue-white patches round plants and along shores at a slant, in a
  base-game tileset: a plant's soft edge drawn before the ground behind
  it kept the ground from showing, and the water under it showed. Shown
  in a renderer test; not confirmed on the reporter's area, nor compared
  with the client.) Two see-through surfaces crossing each other still
  sort by their middles. Each see-through mesh costs two draws. On
  macOS the solid part's discarded fragments wrote their depth (the
  plant's edge hid the ground again) until the shader stopped at each
  discard: seen and mended on the CI's Mac, a virtual one; no real Mac
  was looked at. The
  smaller levels made of an uncompressed texture leave transparent
  pixels' colors out; a compressed DDS's own levels are used as stored.
  (S–M)
- **Custom creatures from the Vault and CEP** (a builder's report,
  [issue 3](https://github.com/jadzziaa/moonglow-toolset/issues/3)),
  fixed against the content itself but none of it compared with the
  client: a supermodel's position keys scaled by `setanimationscale`
  (taken as the size against the model whose animation is played; a
  chain of scaled supermodels wasn't tried); a body part drawn only where
  the base has its node; a mesh with `bitmap NULL` given the part's own
  texture; a bitmap name over 16 characters cut to 16; a blank line among
  ASCII skin weights read as a vertex without any (a blank line that is
  only spacing in a list with all its rows would shift the rows). The
  reporter's "stopped animating" (Mindwitness) wasn't seen as such: here
  it animated, with its skin torn by the shifted weights. The model
  viewer's own check of names over 16 characters is as it was. The
  report's Mindwitness also poses differently than in Aurora and the
  game; not reproduced without the model. (S; the pose needs the
  files)
- **Shadows:** the area view draws the sun's and moon's shadows
  (2026-10-28: a shadow map standing in for the game's stencil volumes,
  laid as the game's shadow plane lays them, checked against the client
  in `shadows_look`; [the research note](research/notes_shaders.md) §G),
  and creatures' and lights' shadows (the Shadows button's menu, done
  2026-10-28). Left:
  - the lights' shadows' darkness was measured at one distance from one
    lamp: the opacity times the plane's fade there, which a fall-off with
    the distance from the light (`shadowradius`) would look alike to (S)
    (waits on the client batch: a creature's light shadow at two more
    distances from the lamp);
  - which light of two equally near the client takes (Moonglow: the
    first), how many creatures cast the lights' shadows at once
    (`max-casting-lights` changed nothing with one; Moonglow: 16), and a
    PC's own light at night (Moonglow places no PC) (S) (waits on the
    client batch);
  - what is drawn after the shadows receives none: the area's grass
    (see-through tufts: in `shadows_look` with `MG_SHADOW_GRASS=1` the
    shadowed ground darkens by 0.34 to 0.38, the client's by 0.42, its
    grass sparser and darkened), the solid parts of a tile's see-through
    meshes (alpha-blended leaves), shiny water drawn after the
    framebuffer's copy; the client's volumes darken whatever was drawn
    before its plane (M: those drawn later would read the map
    themselves);
  - with Fade Geometry on, the faded roofs and upper walls cast no
    shadow, where the game's still do (S);
  - doors cast as placeables do and items on the ground cast none (the
    game since 8193.35: "non-character models cast shadows only if
    opaque") (S) (waits on the client batch: a door and an item on the
    ground in the sun);
  - the shadow's fade with the height between plane and caster
    (8193.31's "shadow alpha by vertical distance") and sun shadows
    never above the camera's height are not drawn (S) (waits on the
    client batch: a caster raised over the ground in steps, and a tall
    one seen from below its top);
  - the previews (blueprints, the model viewer) draw none (S).
  (asked by the Domains of Dread module session and by a builder,
  Troubletcat)
- **Where a cut-out is cut:** at alpha 0.2 and solid where it is not
  cut; that, two-sided faces and decals as the client draws them are in
  [the findings](findings.md) ("Two-sided faces, cut-outs and decals").
  Left: the tiles and the armoires were not
  painted by the probe (another shader path: their value was not
  read); a cut-out that is also see-through as a whole (a mesh's alpha
  under 1, a transparency hint) is still blended, rim and all, which
  was not compared. Cut-outs are no longer sorted back to front with
  the see-through meshes: they write their depth as solid meshes do.
  (S)
  (asked by the Domains of Dread module session; checked in the client,
  2026-10-09)
- **A rippling glow (TXI `proceduretype arturo`) does not look like the
  client's:** `dd_ce3_zombieash`'s `dd_ce3_txhotcoal` (additive, arturo
  `distort 1`) shows in Moonglow as bright red and yellow patches over
  the body, in the client as dim orange veins, with either shader (the
  game's programs leave a rippling texture to Moonglow's). How the
  client makes an arturo texture (its channel scale and translation, the
  distortion) was not read. (M)
  (from the Domains of Dread's report on the flickering zombie, 2026-10-22;
  seen in the client, `creatures_look`)

## Models and textures

- **Heed's previews after 1.33.0** (not reproduced): the Drow shields of
  the standard palette shown as bags in their thumbnails and the
  palette's hover preview. In the base game they show whole
  (`item_previews` in `screens.rs`: `x2_adrowshl001`), so his haks are the
  likely cause (a `baseitems.2da` of their own); his hak list or a
  blueprint is needed. (His armor showing only a head and pelvis was
  Beamdog's HD bodies, drawn whole since 1.34.1.) (S–M)
- **Where mg-mdl still differs from nwn-mdl-spec** (August's pointer to
  the shared spec, 2026-10-19; [the comparison](research/notes_mdl_spec.md)):
  after 1.34.4's changes its suite passes 46 of 46 compiles and 45 of 46
  decompiles. Left: an animated mesh's frames over vertices split by their
  UVs are decompiled over the welded vertices (read back the same in the
  game); a node whose `parent` line is missing or in capitals hangs from
  the root where the game leaves it out (no game model but `c_quiver`'s
  malformed lines); a skin vertex with no valid bone keeps its rest pose
  where the game collapses it (`c_wings2`); face records over the source
  faces after a deletion (SHOULD); a body part's bones named from its
  skeleton when decompiled (not checked). The suite could run in the
  corpus tests (its probes and goldens are CC0). (S–M)
- **Export Files of a model** (a builder: exporting a bugged placeable
  to whoever mends it meant digging through the haks for its files).
  The model viewer, in an editor's page and in its own window, writes
  what the game loads to draw the thing into a folder
  (`mg_preview::files`): models, `.pwk` and `.dwk`, textures as the game
  picks them (and a PLT of the name), MTRs with their textures and
  shaders, TXIs and the textures they name, emitters' chunk models,
  light flares, supermodels. The game's own files are left out unless
  every file is the game's. Not done: the blueprint itself and its
  `placeables.2da` row are not written; a tile's or an area's files are
  not offered this way; a texture named and found nowhere is not
  reported. (S each) It asks before writing over files of the same
  names in the folder, and `mg model-files` does the same from a
  terminal (`--force` to write over).
  (from builders' reports after 1.19.1)
- **The model compiler and writers in `mg-mdl`** (moved from the
  viewer's `crates/mgv-mdl` at its 0.1.6, 2026-10-09; in use since
  1.20.5).
  Not done: `mg pack` compiles only when
  asked (the game reads text, and a pack's bytes should not change
  unasked: on by default later?); Compile Models is a job (off the window's
  thread, on every core, with its progress and Cancel, which leaves
  the hak as it was; one step, named in Undo); a compiled model is not
  compared with the game's own compiler's here (the viewer's `engine.rs`
  does that); Build › Compile Models compiles a module's own
  (a job, one step to undo); nothing compiles a nasher project's on
  Pack Target, whose sources stay text; the linter's notes (Info) are not shown;
  the viewer keeps its copy until its pin
  moves past the release that has these, and keeps the tests that need
  its `nwnmdlcomp` wrapper, the game's compiler and the client
  (`nwnmdlcomp.rs`, `engine.rs`, `client.rs`); `PartNumbers::read`
  still reads a file's part numbers a second time, where the reader
  now keeps them on each node (`Node::part`), but not the header's
  node count. (M for the uses)
  (proposed by the Moonglow Viewer session, 2026-10-07: its
  `docs/PLAN.md` §10.1 has the full text; not checked here beyond that
  the code named is there)
- **Animations bind by part number, not by name:** done (by name
  where either is from text, and for a worn cloak or robe).
  Seen in the viewer session's client pictures (a
  skeleton numbered afresh under the same names leaves its creatures
  unposed), not measured again here; a human and an elf in armor and
  cloak still stand as the client draws them. Of the game's models, 70
  have nodes the two ways move differently, none of them looked at one
  by one. A node an animation numbers −1 is not moved, on the reading
  that the game moves none for it. (S)
  (proposed by the Moonglow Viewer session, 2026-10-07: its
  `docs/PLAN.md` §10.1 has the full text; not checked here beyond that
  the code named is there)
- **Other findings against `notes_models.md`,** each theirs to confirm
  here before the note is changed: part numbers follow file order and a
  compiled tree does not keep it (5,079 of 24,298 models without a
  supermodel); function pointers are ignored; a mesh's box holds the
  origin too; shininess is 1 where a text gives none; EE's 0x3B0 skin
  layout with bone numbers as `i16[64]` is what the game takes; the
  game compiler's normals and tangents equal `mg-mdl`'s ASCII reader's
  to four decimals; ten `c_wingdrg2_*` wing skins are bound in another
  pose than they rest in. (S each, the note's upkeep)
  (proposed by the Moonglow Viewer session, 2026-10-07: its
  `docs/PLAN.md` §10.1 has the full text; not checked here beyond that
  the code named is there)
- **Smaller:** `mg-ui`'s form widgets and palette chooser exported for
  the viewer to use (it keeps copies); a camera framed tightly on posed
  vertices, so that `mg-preview`'s pictures of long models (dragons)
  fill their frame. (S each)
  (proposed by the Moonglow Viewer session, 2026-10-07: its
  `docs/PLAN.md` §10.1 has the full text; not checked here beyond that
  the code named is there)
- **A model that fails while it is drawn:** a model that breaks what
  builds or poses it is left out and the module stays open
  (`mg_render::guard`: tiles, objects, the model viewer, thumbnails). A
  failure inside the renderer's own drawing (uploading its textures, the
  GPU's calls) is not caught and still ends Moonglow, with a crash
  report. (M) (from the research notes)
- **Index lists against the client:** that the game draws a compiled
  mesh's index list where it has more triangles than the face list
  (`Mesh::drawn`: most Forest - Facelift cliffs) is settled from the
  files (the list reaches every vertex, the faces are its first
  triangles, drawing it closes the holes), not by a client screenshot
  (`client_render.rs`). (S) (from the research notes)

## The main window and its panes

- **What a module's tabs open again without:** a module opens again with
  its tabs, side editors and tools, split windows and views as they were
  (Options › General, `crates/mg-ui/src/open_tabs.rs`). Not kept: a hak
  never saved (no file to open) and a 2DA in one; a 2DA's Edit a Copy
  not saved yet (its file isn't there); what a side editor's view
  showed (a hak's selection, a 2DA's cell, a tileset's page), a
  script's bookmarks, the talk table's line, a model view's camera,
  Advanced Search's question (the next session's tab is empty); Tile
  Properties that is waiting for its area's view (its area's tab not
  shown since the module opened) when the module is closed again. What
  a view showed is written with the tabs (when they change, a window is
  moved, the module closes, Moonglow quits), not as the camera moves or
  the cursor does: a crash loses a camera moved since. A conversation's marks are checked by a hash of its contents
  (Rust's `DefaultHasher`): a toolchain that changes it lets them go
  once. A placed object moved outside Moonglow and another of its tag
  and blueprint added: neither is taken for it. A kept Options tab
  makes its draft as Tools › Options does, which looks for the game's
  install (Steam's library files) as the module opens. (S each) (a
  builder's report, October 2026)
- **Deleting from the module tree:** Delete… removes the resource (an
  area with its GIT and GIC and its area-list entry, a script with its
  NCS) and nothing else: transitions, scripts and conversations that name
  it are left as they are, with no warning of them; one resource at a
  time (the Delete key asks the same of the row under the pointer). Moonglow refuses to delete the
  start area; Aurora removes it (after asking twice) and then refuses
  to save: "A module must have a starting location to be valid." (S)
- **Panes' sizes at start:** the module tree is held to two fifths of
  the window and the log to half its height, whatever egui remembered
  (a builder's module opened with the tree over the whole window: a
  long name widened it, and it could not be dragged back); a name too
  long for the tree scrolls sideways into view since 2.8.0. Docked
  panes' splits are held between 12% and 88% (the palettes'), and the
  lower halves of the script, conversation and journal editors to a
  share of their editor, and the talk table editor's to seven tenths.
  The limits on the script, conversation and journal editors' lower
  halves aren't under test (the talk table's are). (S)
- **Windows' sizes and Maximize:** a window's size is remembered by the
  kind of editor, not its place on screen; Maximize is a double click on
  the tab or its menu, with no button in the window's bar (egui_dock
  offers no place for one) and no key; a maximized window isn't kept so
  when the main window is resized, and nothing shows that it is
  maximized. That a size dragged by hand is remembered is not under
  test (the tests set the size). (S–M)
- **The light theme's own colors:** it is egui's light theme; the script
  editor's syntax colors follow it, but conversation and faction colors
  are the dark theme's (legible on both), and the area view's overlays
  are drawn for the 3D view. A pass with a builder who uses it would
  settle the rest. (S–M) (a senior builder's review, October 2026)
- **Panes folded away** (GitHub issue 10): a View menu (Module Tree,
  Palettes Panel, Log, ticked while shown; Hide All Panels; Reset
  Layout), with keys (Ctrl+Alt+1, 2, 3 and 0, in Options › Keyboard); a
  pane folded away leaves a strip at its edge that brings it back, as
  wide as it was; the palettes' pane closed by its ✕ counts as folded;
  the state is kept in the settings. Each pane has one button of its
  own to fold it away, by the reporter's rule on 1.31.0: a chevron on
  the edge it faces the middle by (the tree's right, the palettes'
  left, or right where they were split off the middle's left, the
  log's top), centered along it, in a thin strip that is all a folded
  pane leaves, the button in the same place (the tree's «, the
  palettes' » among their type icons, the log's ⏷ and ⏶ Log, and
  egui_dock's arrow in the palettes' tab bar went). Not done: the
  strip names its pane only in the tip; egui_dock's arrow on the
  palettes' tab bar is covered over (egui_dock has it on every pane or
  none, and windows fold to their bar by it), so the bar keeps its
  blank room before the first tab, the covered arrow can still be
  reached by Tab and pressed by Enter, and the middle's panes keep
  theirs;
  palettes joined to the middle's tabs, split above or below it or in
  a window have no edge button (View and their ✕ fold them), and those
  split above or below leave their strip on the right. (S)
  (from builders' reports after 1.19.1)
- **The menus by the keyboard** (GitHub issue 9), on egui's own menus:
  Alt alone or F10 opens the first menu and closes them (over an area's
  view F10 is Select Tiles', as in Aurora), Left and Right
  go between menus (round the ends), Up and Down between rows (past
  separators and rows that can't be chosen), Enter and Space choose,
  Right and Left open and close a submenu, and the pointer over another
  menu's name opens it while one is open. The marked row is drawn as
  selected. Since 1.20.5: Alt and a menu's first letter (underlined)
  opens it, a letter with a menu open goes to the next row that begins
  with it, and Escape closes the submenu the keys are in before the
  menus. Not done: a letter of its own for each row (they go by first
  letters, and a letter only marks the row: Enter chooses); Alt and a
  letter opens a menu while text is typed in too, as menus do, and
  takes the key from a command given Alt and that letter in Options ›
  Keyboard; Alt alone was not tried in a test (the harness sends no
  modifier on its own: F10 was), nor on Windows, where the system may
  take Alt for its own menu; a submenu inside a submenu has no keys
  (there is none today); the bar itself is never "focused" without a
  menu open. (S each) (from builders' reports after 1.19.1)
- **Undo in a text field** (issue 18): Ctrl+Z and Ctrl+Y with a text
  field in hand were the module's Undo and Redo, and took back an
  earlier change somewhere else, unseen. They are the field's own once
  it has been typed in (until it is left); a field only stepped into
  has nothing of its own to take back, and there they are the module's
  still (a tag typed, Tab, Ctrl+Z takes the tag back). Not done: the
  script editor and the other text fields were not tried one by one
  (the rule is the window's, for every field). (S)
  (GitHub issues 10 to 24, after 1.20.0)
- **The middle with a tab beside the palettes** (issue 11): the middle
  was let go where another tab (the resource browser) shared the
  palettes' pane; the pane's tabs no longer count as the middle's. The
  palettes dropped among the middle's areas are the middle's (issue 20:
  they were taken for the side pane, and the areas pushed to the
  right), and the resource browser's pane alone is a side pane once the
  palettes are closed (issue 23: it took the whole window). Another
  tool docked alone on the right (the talk table, a script) is not
  known as a side pane. Issue 21's drop on a side target is docked
  there from a pane; from a window of its own, the window was seen to
  move and not to dock, by the tests' drag (not confirmed either way in
  the window). (S) (GitHub issues 10 to 24, after 1.20.0; 20, 21 and
  23 after 1.30.2) (the window's drop waits on a drag by hand in the
  running program, the owner's or issue 21's reporter's: the UI tests
  can't make a window of its own, and issue 21 has no answer since
  2026-10-10)
- **The module tree and the palettes by the keyboard** (GitHub issue
  12, after 1.20.0, and
  a builder: Aurora's arrows go to the parent on Left and fold the
  tree): after a click in a list (the module tree, a palette, Module
  Properties' haks) the arrow keys, Page Up and Down, Home and End are
  that list's wherever the pointer is, until it is in an area's view,
  and a thin outline shows the pane; Right and Left open and close a
  row and go in and out, into an area opened out too (its kinds and
  its placed objects); Enter opens (the resource, the blueprint's
  editor, a category), Escape hands the keys back; F2 and Delete are
  for the row under the pointer, else the cursor's; both panes' filter
  fields have Ctrl+F, Escape (clear, then leave) and Down. Not done:
  - F6 and Shift+F6 give the keys to the next pane (the module tree,
    the middle, the palettes). Not done: the log, a talk table's lines,
    the resource browser's list and the 2DA editor's cells are not
    among them; the palettes'
    cursor starts at no row until an arrow is pressed; a command a
    builder already gave F6 to is now in conflict with Next Pane
    (Options › Keyboard lists it; the pane's wins until one is
    changed). (S)
  - F2 at a placed object does nothing: its row's menu has no Rename,
    nor has Aurora's object menu (the parity inventory); whether
    Aurora's tree edits an object's label in place on F2 (its
    `tvMainEditing` renames resources) was not watched. Delete there
    deletes it as its menu does. (S)
  - The resource browser has them (its Name field and its list), and
    Open Script has Down and Up for Enter; the hak editor's, the
    tileset editor's and the Appearance Gallery's Find fields have
    Ctrl+F and Escape. Not done: Down from those three into their
    lists (which have no keys of their own); the tileset's palette and
    the Prefabs list have no arrow keys; in Open
    Script, Escape closes the window at once (it does not clear the
    name first). (S each)
  - In the Gallery Up and Down go a picture at a time, not a row of
    them; the keys repeat only as the system repeats them. (S)
  - With the keys on a category the blueprint selected stays in hand;
    whether Aurora keeps one in hand then was not checked. (S)
- **The look of the two side panes** (the owner's notes after 1.21.0):
  the module tree's header is the palettes' (a search field that says
  what it looks in, with a clear button; Expand All and Collapse All as
  icons, with a count); the palettes' type icons are larger (a line's height; twice that
  was tried and was too large), in rows where the pane is narrow; the palette shown last is remembered between runs (its type,
  and Standard or Custom); Walkmesh and Object Walkmeshes are one
  toolbar button. Not done: the module tree's rows are as they were
  (no pictures, no "more" menu as the palettes' list has); the
  palette's search text and its open categories are not remembered;
  holding the walkmesh button is not held by a test (a right click
  is); Expand All and Collapse All have no keys, and an area opened out
  to its objects, and the kinds under it, keep their own state. (S
  each) (from builders' reports and the owner's notes after 1.21.0)
- **Toolbar buttons: Icons only** (Options › General,
  `icons::Tools`): the main toolbar and the area view's toolbar only.
  Not done: the palette's tile tools (Select Tiles, Eraser, Refine
  Tile…), the script editor's, the conversation editor's, the NUI
  Creator's and the model view's toolbars keep their words (S each);
  All, None, Together, To Scratch and Save as Prefab… have no icon and
  keep their words, the Fade Geometry, Snap and Turn dropdowns their
  text (S, if fitting glyphs are found in egui's fonts); one icon
  means two buttons in places: ℹ is the main toolbar's Module
  Properties and the area's Area Properties, 🏃 Start and Go to Start
  Location (nearly one command), ⊞ the area's Grid and the palette's
  pictures (S); with Icons only a screen reader reads the glyph as the
  button's name, the name being in its tip (S). (ShadowM's request,
  Discord, October 2026)
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

## Languages, tokens and loading screens

- **The game's text in the language edited:** the install is read as
  that language when its `lang/<code>/data/dialog.tlk` is there (its
  talk table, and the rest of its `lang` folder); a module's custom
  talk table is one file whatever the language (checked for each of
  the five other languages installed, `the_install_s_language_is_the_one_edited`).
  Whether the feminine table (`dialogf.tlk`) is shown where the game
  would use it was not looked at: the game reads it for a woman's
  character, which the editor has none of. (S) (waits on Aurora: whether
  it ever shows the feminine table's text, a line by StrRef in its
  conversation editor say; if never, this is settled)
- **Loading screen pictures** are put together as the game's stock
  `pnl_loadscreen` model maps them (two halves of a square texture); a
  module with its own `pnl_loadscreen` model, or a texture that isn't
  laid out so, shows wrong. Aurora's small `<name>s` preview isn't used.
  The picture isn't in the list itself, only on hover. (S)
- **Text in another language where the one edited has none** (English
  first), as a builder says Aurora shows it: the palettes, the module
  tree, conversation lines, the journal's lists, and, marked with the
  language named, every field that edits such text: a blueprint's and an
  area's, Module Properties' name and description, the journal's names
  and entries, a conversation line's text. That Aurora prefers English
  to the first language found was not captured. A token put into a line
  shown in another language goes into that text, which then becomes the
  line's own in the language edited. (S)
  (from builders' reports after 1.16.1)
- **A module's `encoding.2da`** (EE 1.87: a table of the Unicode
  character each of the 256 byte values stands for, shipped with fonts
  to match, so that a module can be in a language the game has no
  codepage for; a builder who knows the engine pointed it out, after
  1.19.1) is read
  now (`GameData::codepage`, `mg_core::Codepage::table`): the
  toolset shows and writes the module's text by it (names and other
  localized strings in the editors, tags and comments, scripts,
  conversations, the journal, factions, the talk tables, the palettes,
  the area list, Find and Replace Text, the 2DA view), and reads it
  anew when the hak list changes. Without a table every path is as it
  was (Windows-1252, Windows-1250 for Polish). What the game does was
  settled in it (`engine_encoding.rs`): the table is read from a hak,
  not from the module file; a cell is hexadecimal with or without `0x`
  (`305` is U+0305), of which the low 16 bits count; a blank or missing
  row keeps the game's own character; a character two bytes stand for is
  written as the last of them; ASCII's bytes can be moved too. Left, or
  not known:
  - `mg` goes by the table where it is given a module (`set`, `apply`,
    `find`, `info`, `areas`, `replace`, `dialog` export and import, a
    plugin's edits), finding the hak through the game install (without
    one: Windows-1252, and nothing said). `mg gff` on a lone file,
    nasher sources (theirs is `--nwn-encoding`), `mg nwsync`'s module
    name still read and write Windows-1252 whatever the table, and the
    script language server reads the game's codepage (`settings.tml`,
    or `--codepage`), not a module's table. (S each)
  - In the toolset, by choice: a prefab's file keeps the bytes (written
    as Windows-1252 text, so that it is the same object in any module),
    and a hak's description is the hak's, not the module's. The Store
    Wizard's lines, a conversation exported or imported, the name Save
    As offers and plugins' strings go by the table
    (`Codepage::spelled_in` for code that writes Windows-1252); a
    letter of an imported conversation the table lacks is a "?" (as is,
    now, one Windows-1252 lacks in a module without a table: a
    conversation's new line, a faction's name and an export's comments
    were written as UTF-8 there). An exported or imported file is
    spelled over as a whole, its punctuation too: of no matter unless a
    table moves ASCII's own characters.
  - A table is taken to stand for every language of one byte a character
    (a Polish name in an English game is read by it too), and for none of
    the East Asian ones; a missing row keeps the character of the game's
    language. Only English was tried in the game.
  - Whether the game reads the table from `override` and the like was
    not tried (Moonglow reads it from whatever lies outside the module
    file, haks first); nor was the game's `game.language.codepage`
    setting, which only `mg lsp` reads (the toolset goes by Options ›
    Language).
  - A text field already being typed in when the hak list changes keeps
    what it showed until it is left.

## Remappable keys

- **Shift+Insert in the window itself:** it asks the backend for the
  clipboard (`RequestPaste`), and the paste comes a frame later; the UI
  tests stand in for the backend, and the keys were not pressed in the
  running program on Linux, Windows or macOS. Shift+Delete in the module
  tree no longer deletes the resource (it is a cut, which the tree does
  not take). (S) (Sacha's report on 2.5.4) (waits on the keys pressed
  in the running program: the owner's on Linux, Sacha's on his system)
- **Sharing key sets:** Options › Keyboard › Export… and Import… write
  and take the keys chosen as a file; the dialogs themselves weren't
  driven.
- **Undo's name for a side editor's step:** with the pointer over a
  hak's, a 2DA's, a tileset's or the talk table's editor, or the keys in
  a 2DA's grid, Edit › Undo and Redo (and the toolbar's) are that
  editor's, and say plain "Undo" rather than the step (the module's step
  is named; the hak's and the 2DA's have names to give, the tileset's
  and the talk table's none). The tileset's editor has no test of its
  own for its keys (it shares the hak's path, `side_doc.rs`). A text
  field in one of these editors that has the keyboard but has not been
  typed in since it took it (the Find field, clicked again) passes
  Undo's key to the editor's step, as fields elsewhere pass it to the
  module's; the 2DA grid's own reading of Ctrl+Z left it to the field. (S)

## NUI Creator (the review of its pull request, October 2026)

The NUI Creator came as a contributor's pull request and was brought in
line with the codebase before it was merged: its frames (26 to 35 ms after
an edit, now 1.4 to 2.4), one Undo step a field, the toolset's labels,
headings, name window and OK and Cancel, typed errors and lossless
settings in `mg-nui`, Build through the module's compile path, its tests
where the crates keep theirs, and a client test of what its checks say of
the game. This is what that review left, and after it what the
contribution itself states as not done or not checked.

- **A picture cache of its own** (`crates/mg-ui/src/nui_view/skin.rs`):
  `skin::Assets` keeps its textures beside `images::Loader`
  (`crates/mg-ui/src/images.rs`). The two look pictures up differently:
  a window's are JPG, TGA, PNG, BMP, then DDS, with a live module below
  its haks, read again when their bytes change and reported with where
  they came from; the loader's are DDS or TGA, the module first, kept by
  name. JPEG and BMP are still read with the `image` crate (nothing else
  here reads them). 15- and 16-bit TGAs, which were read with the
  `image` crate before, come out of `mg-image` a level off in places
  and with their attribute bit as alpha. (M)
- **Where the preview is known to differ from the game** (stated in
  [the manual](manual/18-nui-creator.md);
  `crates/mg-ui/src/nui_view/preview.rs`): after a control with a width
  of its own, rows of buttons without widths make the game pick a
  narrower width than the preview's (308 against 462 in the case
  measured); at a UI scale that isn't whole, the preview's rows drift
  from the game's by up to a pixel each (the game rounds each scaled
  size down); an encouraged control shows its hover look and does not
  pulse; numbers in text are shown to 16 digits. (S each)
- **Left by The Domains of Dread's NUI Creator asks (October 2026)**
  (`mg nui render`, `layout`, `schema`, `import`, the `nui-sources`
  check, AccessKit, the event function; the asks are the module
  project's probe, `tdod-modulework/docs/systems/nui-creator-probe.md`):
  - *What `render`'s `--state`, `--binds` and `--variant` leave:*
    - *A state by ID is every row's*: a control of a list's row template
      is hovered (pressed, disabled) in every row; there is no
      `ID@ROW`. (S)
    - *One variant at a time*: `--variant` shows one; a window with two
      swap layouts is drawn with the other's initial contents. (S)
    - *A bind's value is checked only for its kind*: a value of another
      kind than the bind's first is said, not what the game would make
      of it (a number where a string is drawn, a color missing `a`). (S)
  - *What `layout` cannot see*: rows of a list out of its view are not
    reported (the canvas draws only those in view); the title's text,
    cut or not, isn't checked; `overlaps` comes of the canvas's own
    layout, which places neighbors apart, so it is empty but for
    windows the game would lay out differently, which it can't know. (S)
  - *The preview is not the game* (as stated above): `render` and
    `layout` show what the canvas shows, with its known differences.
  - *The JUI has no schema of its properties*: `mg nui schema --layout`
    gives the Creator's templates, the checks' rules of which property
    takes what are in `validate.rs`'s code, not a table a schema could
    be made from. (M)
  - *The Creator has no Import*: windows come in from files with `mg nui
    import` or Import's ERF import (which also brings `.jui` and `.txt`),
    not from the NUI Creator itself. (S)
  - *`mg` links `mg-ui`* for `render` and `layout`: its build takes
    longer and its binary is larger; the preview could become a crate of
    its own if that matters. (M)
  - *AccessKit doesn't start on newer Linux desktops*: eframe 0.36's
    `accesskit_unix` (0.21) offers the tree only when the session bus's
    `org.a11y.Status.ScreenReaderEnabled` turns true, and at-spi2-core
    2.62 (the development machine's) has only `IsEnabled`, so there it
    never starts; `accesskit_unix` 0.24 watches `IsEnabled`, and comes
    with an egui/eframe whose `accesskit_winit` uses it. Until then the
    desktop agents here can't read the window. (S, with that upgrade)
  - *AccessKit was checked on Linux only*: the GUI checks for Windows
    (`x86_64-pc-windows-gnu`) and eframe's AccessKit for macOS
    (`aarch64-apple-darwin`); CI builds them, and no one has read the
    tree with a screen reader on any of the three, nor in the Flatpak
    (`--talk-name=org.a11y.Bus`, from reading `accesskit_unix`). Its
    cost with an assistive program connected, `FRAME_ACCESSKIT=1` in
    `frame_perf` on the campaign's 117 views: 0.86 ms a frame on
    average without the tree, 0.97 with it, the most 0.55 ms more (a
    long script, 0.73 to 1.28); none over the budget. Without one,
    nothing. (S)
  - *The event function in the game*: compiled against the game's
    `nwscript.nss`, not yet run in the client (`client_nui.rs` builds
    windows with their own event scripts). (S)
  - *`a_module_s_windows_are_laid_out_as_json`* (`crates/mg-ui/tests/nui.rs`,
    run by hand) measured windows through the editor's accessibility
    tree; `mg nui layout` does it from the command line now. (S)

## Automation

- **Plugins, beyond API 0.4** (`plugin-proposal.md`; commands and checks
  in sandboxed Luau are built; 0.2 added a file or folder the user
  chooses, haks written with their leave, areas and terrain, pictures;
  0.3 files written into a chosen folder, 2DAs and talk tables as
  values, and events, whose own gaps are below, under "Plugins writing
  into a folder"; 0.4 what is selected and
  open, commands in right-click menus, fixes for findings, a plugin's
  own tests, settings, a project's own plugins, a resource's own events
  and the after-create hook):
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
    feature of its own. (L)
  - **Not tried in the application on a desktop:** the folder and the
    hak question were driven through the UI harness and `mg`, not with
    the real file dialogs (the file dialog of `open_file` filters by
    the extensions asked for; that filter was not seen on a desktop
    either). (S)
  - **A hak is written before the module's edits go in,** and stays if
    they then fail; Undo leaves it too. A job's haks are held in memory
    until it ends, except files copied from a chosen folder. (S)
  - **Painting is a tile at a time:** each call reads the area's tiles
    again, so a plugin that sets every corner of a 32 × 32 area makes
    thousands of small edits (one undo step, but a large one). (S–M)
  - **No list to install or update from:** plugins come as files; the
    proposal has an index with API 1.0. Remove is for what Install from
    File installed; a folder copied in by hand is deleted by hand. (M)
  - **Archives of several plugins** are refused: one plugin to an
    archive. (S)
  - **A project's own plugins** (API 0.4, the Domains of Dread's
    builder proposal A2: `plugins/` of a project, or `[plugins] folder`,
    run once the builder allows each, asked when the project is opened,
    the answer kept in Moonglow's own folder (`project-plugins.toml`, by
    the project's canonical path, the plugin's id and a SHA-256 digest of
    its files, which it runs from), never read from the project; `mg
    verify` as allowed or with `--allow-project-plugins`; `mg plugin list
    --project`, `mg plugin allow`). Not done: a project moved or copied
    elsewhere on disk is asked about again (answers are by its path);
    the question comes when the project is opened (and at Reload), not
    when its plugins' folder changes while it is open (the folder is not
    watched; until then the plugins run as they were read, which is what
    was allowed); a digest says a plugin is the one allowed, not who
    wrote it (no signatures); a no holds for the plugin's id whatever it
    becomes, and there is no Allow All in the window; `mg plugin run` runs a
    plugin's folder it is given whether allowed or not (it is named on
    purpose); a project's plugin and one installed of its id share their
    settings of scope `user`; what a project's plugin's events were
    allowed (a folder, a yes) is kept by its digest, so a changed one
    asks again; a module kept in one file has no project, so none. The
    question and the Plugins window were driven through the UI harness
    and looked at as rendered pictures (one plugin asked about), not
    used on a desktop. (S each)
  - **A plugin's settings** (API 0.4, A5: `[setting]`, `ctx.settings`,
    `ctx.settings:set`, the settings' form, scopes `user`, `project`
    and `user-project`). Not done: five kinds only (no number with a
    fraction, list, color or pattern a text is held to); a `path` is
    text, the plugin reads no file by it (it still asks with
    `ctx.ui:open_file`), and the form's Choose… picks a file, not a
    folder (typed); what the form saves and what a job sets is not part
    of Undo (files outside the module), and a team value saved writes
    the project's `moonglow.toml`, committed with it, without asking;
    what a job set is kept when the job did not fail, even if its edits
    then do not apply; `mg` keeps no settings of scope `user` and reads
    none of Moonglow's (`mg plugin run --setting` for one run; `mg
    verify` takes the project's files and the defaults); a value kept of
    the wrong kind, or of a setting no plugin declares, is said when the
    project is opened and left there, and nothing tidies it. Driven
    through the UI harness, with a rendered picture of the form; not
    used on a desktop. (S each)
  - **More kinds of things to add:** exporters and importers, wizards,
    panels. (L)
  - **Commands on what is selected** (API 0.4, the Domains of Dread's
    builder proposal A1: `ctx.selection`, `ctx.open`, a command's
    `for`). Not done: `for` names a kind for its placed objects and its
    blueprints alike (a command for one only checks `blueprint` itself);
    a selection is objects, blueprints or one area, never tiles, a
    trigger's points, the start location, several areas or several of
    the module tree's rows; the commands are in the area view's, the
    module tree's and the palettes' menus, not in Find References,
    Advanced Search, the Properties editors or the other editors' own;
    an area's commands are in the area view's menu only when nothing is
    selected there; from the Plugins menu or a key the selection is the
    area view shown last, else the palette's, not that of the pane with
    the focus; `ctx.open` is nil for several objects or blueprints edited
    together; a palette's blueprint menu has no Properties to end it, so
    the plugins' commands come last there; a palette's categories and
    the tree's groups have none. Driven through the UI harness, with one
    rendered picture of the area view's menu; not used on a desktop.
    (S each)
  - **Fixes for findings** (API 0.4, the Domains of Dread's builder
    proposal A3: `ctx.fix`, Verify's button beside a finding, `mg verify
    --fix CHECK`, a finding's `fix` in `mg verify --json`). Not done: a
    fix asks nothing (no form: "retag to what?" is a command's); it is
    worked out when Verify runs and refused once a resource it changes is
    not as Verify read it (compared as bytes) or was changed by another
    fix that did more than set fields and take items out, so after
    Undo or a second fix
    there, Verify runs again; Undo of a fix leaves its button saying
    Fixed; the buttons are in Verify Module's log alone (not Build
    Module's results, the 2DA editor's faults or the tileset editor's),
    for the last Verify, with no "fix all of this check" in the window
    (`mg verify --fix CHECK` is that); the doctor has fixes for
    `tag-unique` (the others retagged `TAG_2`…, scripts that look for
    the tag not followed), `area-variables` (moved to the GIT),
    `var-runtime` (removed) and `static-transform` (the transform off)
    only:
    `pc-faction` has none (the builder chooses the faction), nor do the
    references, models, 2DAs and tilesets; the fixes of different
    plugins' findings are made one at a time in the window; `mg verify
    --fix` with a check's id that no finding with a fix has (mistyped,
    or a plugin's not given) only warns, since a plugin's ids are known
    only once its checks ran, so a pipeline reads "nothing to fix" (fail
    instead?). Driven
    through the UI harness, with one rendered picture of the log's
    buttons; not used on a desktop. (S each)
  - **A resource's own events and the after-create hook** (API 0.4, the
    Domains of Dread's builder proposal A4 and A6: `resource-saving`,
    which warns of or refuses what a save would write, with fixes, in
    the Before saving window; `resource-saved`; `object-placed`,
    `object-pasted` and `resource-created`, whose edits join the user's
    own command; a 5-second budget; `mg set` and `mg apply --plugins`;
    `mg plugin run --event --was/--select/--by`; a test's `was`, `by`,
    `warned`, `refused`). Not done: Save As (to another place) asks no
    plugin, nor do the 2DA, talk table and hak editors' own saves, a
    script's own Save, Build Module or `mg watch`; the custom palettes
    Moonglow rebuilds at a save are not among what a handler reads;
    what fires `resource-created` is the Area Wizard, the creature and
    blueprint wizards, Add to Palette, Setup Store and Copy (Copy As,
    Edit Copy), not the Script Wizard, New Script, a NUI window, an
    import or what a plugin makes, and `object-placed` is a palette's
    blueprint, the Placeable Gallery, a light, Create Waypoint and Setup
    Store's store, not a spawn point, the start location or an
    encounter's points; a handler of a placing, a paste or a making can
    only edit and log (no warning beside what was made, no refusing a
    paste); a fix in the Before saving window is made one at a time, the
    save asked again after each; "as it was" is as each file was read
    (a module folder, a project) or, for a module kept in one file, as
    it was at the last save or opening, in this run; `ctx.event.changed`
    says what differs in a GFF only (100 lines at most), not in a
    script's text; the budget is fixed (5 seconds), not a setting, and a
    handler is stopped between steps of its code, not inside a long call
    of the API (a large `ctx.module:gff`); `mg` runs a save's events for
    `mg set` and `mg apply` alone (not `mg areas`, `mg rename`, `mg
    replace`, `mg update-instances`, `mg import`, `mg verify --fix` or a
    plugin command's own save), and a warning there never fails it (no
    `--strict`). Driven through the UI harness (a save refused, warned
    of, its fix made, Save Anyway and Cancel; a paste and a copy edited,
    one Undo), with rendered pictures of the window warning and
    refusing; not used on a desktop. (S each)
  - **A plugin's own tests** (API 0.4, A8: `tests/*.toml`, `mg plugin
    test`). Not done: what a plugin writes outside the module (haks, a
    chosen folder) is not written and not held to anything; a command's
    edits are held by the fields they leave, their count and resources,
    not as an edit file to compare; a finding's message is matched
    whole; `mg plugin test` runs one plugin, not a folder of them, and
    the window does not run them; `mg plugin check` does not read the
    test files; the plugin gets the game's data only where `mg` finds a
    game, so a pipeline without one cannot test what reads `ctx.game`.
    (S each)
  - **The reference is written by hand.** A test holds it to list every
    name of the API, not that each description is right. (S)
  - **The type file was not tried in an editor:** it loads as Luau and a
    test holds it to have every name, but it was not checked with the
    Luau language server. (S)
  - **External programs** as plugins (a protocol over standard input and
    output, for Python or Nim): not started. (M)
  - **The window with plugins was run on Linux only:** the application
    itself was driven on an off-screen display there (plugins installed
    from archives listed and enabled, a command as a background job, a
    form answered, Undo, a check in Verify Module, the enabled plugins
    kept over a restart, `--no-plugins`), and the rest through the UI
    harness. On Windows and macOS the tests pass in CI, but nobody has
    used plugins in the window there; and the file dialog of Install
    from File was not driven anywhere (the harness answers for it). (S)
- **`mg roundtrip`** (a builder asked what keeps work from being
  corrupted): a module saved, read back and each GFF and 2DA written
  anew, compared with what it was. It does not ask the game or Aurora
  (the engine and Aurora tests do, on the game's own modules), does not
  write the other formats anew (scripts, models and the rest are kept as
  bytes, and compared as such), and is not in the toolset's menus. A
  written GFF is compared by its fields, not its bytes: Moonglow lays a
  GFF out its own way, as every edit already does. (S)
  (from builders' reports after 1.19.1)

## NWSync publishing

- **No download tested:** the repository matches neverwinter.nim's, but a
  game client fetching it from a server isn't run (the test sandbox has
  no network). (M)
- **The feminine talk table** isn't published, as `nwn_nwsync_write`
  doesn't; whether the client would use one isn't known. (S)
- **No pruning** of old manifests' data (`nwn_nwsync_prune` does it), and
  no upload: the folder is copied to the web server by hand. (S)
- **Compression:** Moonglow's encoder offers only zstd's fastest level,
  so the data is about a quarter bigger than
  `nwn_nwsync_write`'s (9.8 MB against 7.7 MB in `nwsync.rs`): more to
  upload and download. A stronger encoder would be a new dependency. (S)

## Tileset authoring

- **Rules and doors:** primary and secondary painting rules and a tile's
  door hooks (`[TILEnDOORm]`) aren't on the editor's pages; they stay as
  the file has them (edit the text for now). (M)
- **A tileset with holes** (a builder's report: a big old tileset,
  merged by hand, drew no tiles at all) is read tile by tile, as the
  game reads it: a tile without its section or a model draws nothing,
  the area's problems and Verify (`set-hole`) name the holes, a bare-LF
  file is an error (`set-line-ends`: the server never loads the
  module), and a model that is there and can't be read is said to be,
  with why, not "not found". Not done:
  - The builder's own tileset (trf01, 2813 tiles) has no holes and CRLF
    line ends, and Moonglow draws its tiles whose models the game has:
    why it draws none of theirs is not known without their hak (its
    models, or the module and the haks it lists). The log now says,
    for each model left out, whether it was found and why it can't be
    read. (S, once the hak is in hand)
  - A file with lines ending both ways was not run in the engine; only
    one whose every line ends in LF is called an error. (S)
  - The tileset editor opens such a file now (it opened none before);
    how its pages show a hole was not looked at. Painting never chooses
    one. (S)

## The install, starting and crashes

- **The recovery copy written at a panic** holds the work as the frame
  that panicked left it (a change applied in it is in the copy), and only
  a panic in a frame writes one: one in a background thread (an
  autosave, a plugin's run, a model's load) or in eframe's own code
  writes none. On macOS a panic unwinding into the system's code aborts
  after the copy is written. Not tried on a real crash on Windows or
  macOS. (S) (Sacha's report, 2026-10-25)

- **userpatch.ini's haks** are read (Options › General, off by default), at the
  game's priority for them as the wiki's load order gives it (13: under
  the module, over `override/`); not measured in the client. Not done:
  `mg` doesn't read them (as it doesn't read the Workshop: a flag would
  do); Verify warns of a reference only they satisfy, but not of one
  only the user's `override/` or a Workshop item satisfies, which a
  player lacks just as much; their talk tables, music and ambient are
  not looked for apart from the haks' own resources. (S each) (El
  Grillo's question, 2026-10-21)
- **Steam Workshop content** is read (Options › General): each item's
  `override` folder as a layer under the user's own, its `hak` and `tlk`
  folders searched. Which of two items' overrides wins in the game isn't
  known (the lower number here), and that the game puts them under the
  user's override is from how it is described, not measured: the client
  runs here with Steam hidden. An item's other folders (portraits,
  modules, music) aren't read. (S) (from builders' reports after 1.14.1)
- **Finding the game:** Steam's libraries are read from its
  `libraryfolders.vdf` (a game on a second drive), and GOG's usual
  folders tried. Steam itself installed outside its usual folder is
  found on Windows only as `X:\Steam` or `X:\SteamLibrary` (the registry
  isn't read); Beamdog's client isn't looked for. The reporter's case (a
  second drive on Linux) is covered by the list, not tried on their
  machine. (S) (from builders' reports after 1.14.1)
- **Installing updates from the app** (the rest of GitHub issue 28):
  Help › Check for Updates… and the daily check only say that a newer
  release is out and link its page. Not done, for the owner to decide:
  downloading the package for this system and installing it (the
  AppImage replaced in place or through AppImageUpdate and zsync; the
  Windows installer run; the macOS app swapped), restarting after
  asking about unsaved work or installing at the next start, and the
  issue's Options (Install updates automatically, off; Include
  pre-releases, off). Before any of that, verifying what is downloaded:
  the release would publish SHA-256 checksums and a signature checked
  against a public key built into the app (minisign or ed25519), so that
  a tampered download or a compromised GitHub account can't push code
  to every user; the packages aren't signed yet. Building blocks the
  issue names, unevaluated: Velopack (a Rust SDK, Windows, macOS and
  Linux, delta updates), cargo-dist with axoupdater, the `self_update`
  crate over GitHub Releases. Also not done: the release notes shown in
  the window (only the page is linked); the issue's "Install Now, Later"
  buttons (the notice has the page, Don't remind me about this version
  and ×). (L) (GitHub issue 28)
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
  - **OpenGL draws wrong.** Not offered in Options (the owner's
    decision), but wgpu still falls back to it where neither Vulkan,
    Direct3D 12 nor Metal draws, and `WGPU_BACKEND=gl` takes it:
    `WGPU_BACKEND=gl cargo test -p mg-render --test render` fails 20
    of its 44 tests on the development machine (no visible particles,
    the game's programs, the framebuffer's copies, animations...): the
    renderer was never made for wgpu's GL backend. (M)
  - `mg` and the other headless renderers take Automatic (or
    `WGPU_BACKEND`), not the GUI's setting; Moonglow Viewer, which builds
    on mg-render, doesn't use `pick_adapter` yet. (S each)

## Documentation

What the [audit](docs-audit.md) of 2026-10-12 could not put right with a
few proven words needs a decision or a rewrite and is listed there, per
document; the entries here that name the audit are that list in short.

- **The manual's tables:** the built-in manual viewer can't wrap a
  table's cells, so a table wider than the page is shown there as a
  list (`manual::fitted`; it was cut off at the page's edge before, and
  the text after it too). Tables with wrapped cells would read better.
  (S–M) (from the research notes)
- **Open-question lists the notes answered since:** `notes_models.md`
  §I.2, `notes_tilesets.md` §B.4 and §I, `notes_shaders.md` §M,
  `notes_gff_changelog.md` §G. (S each)
  (the documentation audit of 2026-10-12)
- **The wiki corrections were never submitted** (`wiki-proposal.md`);
  all fifteen pages are unchanged upstream. (S)
  (the documentation audit of 2026-10-12)
- **The manual, what the audit named and is not put right:** GOG's
  folder is not looked for on macOS (chapter 1 does not say so);
  chapter 14 says "There's no OK or Cancel", which is true of property
  editors only, and repeats chapter 12 in "Habits to change". (S each)
  (Chapter 15's `--event` example now names the `house-rules` example,
  which declares events.)
  (the documentation audit of 2026-10-12)
- **The parity checklists:** the main menus are not compared with
  Aurora's item by item; "eight tests not in
  `app.rs` named without their file" is as the audit found it. (S
  each) (the documentation audit of 2026-10-12)

## The code itself (the code review of October 2026)

A review of the whole workspace for dead code, clumsy code and code
written twice found more than it changed. What it changed (some forty
commits, with a test for all but one of the bugs it fixed) is in the
history; this is what it left, found by reading and not by running
unless it says so.

- **Still made every frame, each needing something kept:** a blueprint's
  category list (`Form::category` reads and parses the palette),
  Module Properties' list of the hak folder, the script
  editor's sorted symbols and its templates folder, Find in Files'
  results (copied twice), the thumbnails' lists for the Appearance
  Gallery and the Creature Wizard, the module tree's rows for the
  keyboard (each group's resources copied). (S each)
- **Written more than once, in the tests:** some corpus tests still
  stack the game, a module's haks and the module by hand, where
  `Module::stack_onto` does it for mg and the window. (S)
- **Never used, or never otherwise:** the second half of
  `ModelView::frame` (the model is there exactly when the composed
  preview is); arguments that are always the same
  (`situated::beside_model`'s `model_first`, `Form::palette_color`'s
  `game`, `dialog_view::script_with_params`' label, list and salt,
  `render_minimaps`' size, `GameInstall::talk_table`'s `feminine`);
  fields nothing reads (`widgets::Picker.types`, `LoadedTexture.mtr`);
  `commands::Id::defaults` (the table behind it is kept in step with
  the key commands and has a test, but nothing asks it for keys);
  `DebugView::MaterialDiffuse` (never set); `Renderer::color_format`,
  `Reader::data` and `Reader::pos`, `Txi::alpha_mean`,
  `SetFile::is_empty`. A `pub` item of a crate Moonglow Viewer builds on
  may be in use there: `plt::LAYERS` is. (S each)

### The review of 2026-10-27: what is left

Six readers over the workspace and clippy's pedantic lints as a finder
found some forty defects and costs; what was done about each is in the
[log](log.md) (2026-10-27). The editors' costs were measured first
(`frame_perf`, with views added at a persistent world's sizes): those
within the 144 fps budget were left as they are (the 2DA editor filtered
on 6,000 rows 1.7 ms a frame, a conversation 1.4, blueprint editors and
an item's Properties page 0.6 to 1.5, palettes, the manual and the model
viewer under 2). What is left:

- **Typing in a very large script** (`nwscript.nss`, 14,000 lines) is
  13 ms a key: egui lays out the whole text again on each change. An
  editor that lays out the lines in sight would be needed. (L)
- **The blueprint picker over a large palette** (inventory, store and
  encounter pages) folds every blueprint once a level of its tree and
  draws every match: searched for what all of the persistent world's
  8,000 custom items have, the store's Inventory page takes 8.0 ms a
  frame (`frame_perf`'s "store inventory, picker searched", 2026-10-29;
  the budget 6.94). The matches kept by palette and search, and only
  the rows in view drawn, would cut it. (M) Dragging objects copies the
  whole area model each frame: not timed, `frame_perf` has no drag of a
  selection (S to add). (The tiles palette searched as it is typed is
  timed since: 5.0 ms at worst over area092's 784 tiles, in budget.)
- **Skinned and vertex-animated meshes are never culled** (`renderer.rs`):
  an off-screen creature's bones are posed and drawn. A renderer change,
  so through CI on a branch first. (M)
- **Smaller:** the remembered-files cache (`Remembered::file`) is named by
  Rust's own hasher (a cache, so harmless across releases);
  `moonglow-hak-<pid>` folders left by a crashed Moonglow are not
  cleaned up; the hak editor's Save copies the old hak to `.bak` where
  the module's Save now links it; a temporary `.moonglow-tmp` left in a
  project by a crash is not in a project's ignored files. (S each)

## Written more than once (the review of 2026-10-21)

Five reviewers read the workspace for logic written more than once, a
builder having said that AI-written code "reimplements a lot of stuff in
minorly different ways instead of building one source of truth". Where
copies had drifted into a wrong result, the fix went through one place
with a test (the log, 2026-10-21). This is what is left: places by file
and line as they were that day, found by reading, not by running unless
it says so. One question settles most of them: which copy is the source
of truth, and does each caller's difference matter.

### To build once (the structure the copies want)

- **Dialog windows built by hand:** `widgets::window` and
  `widgets::dialog` exist; `script_wizard.rs:916`, `module_props.rs:690`,
  `palette_categories.rs:194`, `hak_view.rs:696`, `outside.rs:363`,
  `references.rs:681`, `model_view.rs:270` are one of them by hand, and
  six more (`store_wizard.rs:141`, `:348`, `levelup_view.rs:87`,
  `faction_view.rs:257`, `:290`, `NameWindow` at `widgets.rs:1268`)
  repeat a movable variant with no helper; `creature_wizard.rs:257`
  and `situated.rs:247` (an `egui::Modal`) are two more styles, and
  centering is done two ways. A `widgets::modal`. (S)
- **The placed objects' kind, still spelled out in the window:** the
  GIT's lists and the nine types are one `mg_rules::ObjectKind` now
  (`mg_area::ObjectKind` and `mg_module::palette::BlueprintKind` are
  that type under the names their callers use), but mg-ui keeps tables
  of its own that the merge with the editors' work in progress left
  alone: Advanced Search's kinds (`search.rs:58`), the module tree's
  categories (`tree.rs:22`) and its New wizard per type
  (`tree.rs:1280`), the palettes' New menu items (`palette_view.rs:1117`),
  the tabs' kind names (`tabs.rs:116`, five types), `icons::blueprint`
  (a kind turned into itself). (S)
- **A localized string's text, without the toolset's language or the
  talk table:** `query.rs:74`, `dialog.rs:598`, `:502`, `rename.rs:513`,
  `hak_edit.rs:120`, `areas.rs:63`, `tree.rs:85` and `:112` (beside
  `GameData::locstring`): `mg find --name` doesn't match a name in
  French alone or in the talk table, and `mg areas` names an area
  otherwise than the module tree. A `LocString::shown_in(language,
  codepage)` for where there is no game. (M)
- **The module's areas, two ways:** the IFO's list (`Module::areas`:
  `query.rs:168`, `bulk.rs:74`, `mg update-instances`) and the AREs
  (areas, the doctor, search, the module tree); an ARE missing from the
  IFO is seen by half the tools. And areas listed and filtered three
  times (`areas.rs:40`, `tree.rs:105`, `area_props.rs:706`, the last
  without the tag). (M)
- **Where an object stands:** visual transforms read two ways
  (`areas.rs:191`: any entry; `mg-area lib.rs:233`: scope 0, else the
  first), and placement fields (X/Y/Z and Bearing, or XPosition and
  orientation; PointX or X) read and written in six places
  (`instances.rs:1149`, `mg-area lib.rs:323`, `reshape.rs:92`,
  `edit.rs:50`, `query.rs:114`, `palette_add.rs:58`). (M)
- **GFF paths as strings:** two `Step` enums (`mg-module text.rs:77`,
  `mg-edit lib.rs:36`), hand parsers (`search.rs:255`, `rename.rs:83`,
  `:436`, `verify.rs:38`, `references.rs:489`), hand walkers beside
  `Struct::walk_fields` (`rename.rs:192`, `:209`, `text.rs:207`,
  `doctor.rs:1250`). `GffPath` into mg-gff, carried by `Reference`,
  `Usage` and `Hit`. (L, mechanical)
- **Filters: one matcher and one search field, what is left** (built
  2026-10-27: `mg_core::text::Needle`, every word in any order, in any
  of a row's fields; `widgets::SearchField`, with Find's keys from
  Options › Keyboard; every Ctrl+F read by hand is gone). What differs
  still, by choice: the palettes alone fall back to letters in order
  when nothing has every word; each list looks in its own fields (the
  tree: name and ResRef, an object's name and tag; the palettes and the
  item picker: name, ResRef, tag; Find Instance: tag and blueprint as
  two fields, each its own words; the resource browser, the hak editor
  and Transfer: the ResRef or the file name alone; the tileset editor:
  model and terrains; a 2DA: every cell and its value's name; the Area
  chooser: ResRef and name, not the tag). Down leaves a field for its
  list only where the list has keys of its own (the tree, the palettes,
  the resource browser); elsewhere it stays in the field. Not moved to
  `SearchField` (plain fields, no clear button nor Find's keys, the
  matcher already theirs): the Creature Wizard's appearances, the
  Script and Store Wizards', the Area chooser's, a table's row list and
  the lists' dropdown filters (`widgets.rs`), the store page's, Options
  › Keyboard's, the creature pages' lists, the script editor's help and
  Open Script, the Command Palette. (S)
- **A list's click to select several, three times:**
  `widgets::select_in_list` (Open Script, from October 2026), the hak
  editor's list (`hak_view.rs:457`, the same rule) and Module
  Properties' haks (`module_props.rs:427`: Ctrl wins over Shift there,
  and Ctrl+Shift adds no range). The hak editor's could call it as it
  is; the module's haks, once the difference is chosen. (S)
- **Module Properties and the journal redo `Form` by hand**
  (`module_props.rs`, `journal_view.rs:296`): a tag's 32 characters held
  three ways (the module's not at all), the module's XP slider adds an
  empty undo step when a drag ends where it began, neither keeps typing
  as `hold_text` does; the "…" button for every language seven times.
  (M)
- **The game data for a module, three times in `mg`**
  (`search.rs:98`, `plugins.rs:34`, `main.rs:2671`, with the custom
  talk table found three more ways: `main.rs:2713`, `nwsync.rs:122`,
  `mg-ui lib.rs:2184`), each warning differently. A
  `Module::game_data`. (M)
- **A blueprint's preview, three times** (`mg model-files`,
  `model_view.rs:164`, `mg-area lib.rs:347`): `mg model-files` has no
  waypoint. A `mg_preview::of_object`. (M)
- **The blueprint-to-instance pipeline four times** (`area_view.rs:3944`,
  `:3784`, `paint_mode.rs:559`, `bulk.rs:144`; the paint stroke's cache
  is deliberate). (M)

### Settled in the engine or Aurora first

- **A valid resource name, three rules:** ASCII of 16 (`mg-plugin
  tables.rs:253`), letters, digits and `_` (`talk.rs:237`), those and
  `-` (`hak_edit.rs:114`, `mg-plugin files.rs:247`), lower case alone
  (`mg-nui lib.rs:209`, `talk_view.rs:332`); also `talk.rs:82`,
  `attach.rs:72`, `grass.rs:222`, `doctor.rs:708`,
  `tileset_view.rs:142`. A plugin can write a sound `mg tlk` refuses.
  The game takes any name of up to 16 bytes but NUL, whatever its case
  (found by either, listed in lower case), with `-`, `.`, a space or a
  byte past ASCII (`engine_resource_names.rs`), as `ResRef` does; what
  is left is the one rule for names Moonglow makes, chosen (narrower
  than the game's, for names that are also a project's file names)
  and put in mg-core for the eleven places. (S–M)
- **A palette node's name, four readers:** `palette.rs:430` (NAME, then
  STRREF, then DELETE_ME), `:323`, `new.rs:242`, and the tile palette's
  `mg-area terrain.rs:119` (STRREF first, no DELETE_ME, NAME as UTF-8).
  Aurora's order, by a capture. (S)
- **A copy's resref:** `copy_resref` (`palette_view.rs:320`, keeps
  `nw_`) and `new_resref` (`palette_add.rs:37`, drops it and goes on
  from the template's number), with three more generators
  (`store_setup.rs:27`, `blueprints.rs:35`, `dialog_view.rs:1612`) and
  two from a file's stem (`dialog_view.rs:1608`, `nui_view/images.rs:91`).
  Aurora's Copy against its Add to Palette, by a capture. (S)
- **The two portrait filters:** the Creature Wizard's
  (`creature_wizard.rs:92`: the gender chosen, then rows of both or
  none, Plot rows left out, not the module's portraits) and Select
  Portrait's (`situated.rs:228`: the gender exactly, the module's too):
  Outsider and Male shows nothing in Select Portrait. Aurora's, by a
  capture. (S)
- **The module's codepage, four ways** (`mg main.rs:2661` builds a
  ResMan for it; `mg-plugin runtime.rs:950`, `mg plugins.rs:414`,
  `mg-ui text.rs:23`; `mg-plugin terrain.rs:65`): the same on an English
  game; with a partial `encoding.2da` on another, to be checked. (S)

### Smaller, each S

- Left by the one row list (`widgets::row_choice`, 2026-10-21): it lays
  out every row while open, where the 2DA editor's "a row of X.2da"
  laid out only those in sight (a table of tens of thousands of rows,
  not measured); Advanced Search's Value list goes by name past 12
  rows, where the item editor's cost list keeps the numbers' order (+10
  before +2 there); the Appearance and loading screen lists have no "≠"
  mark for blueprints edited together, which the other fields' lists
  have; and lists that had the table's order and now go by name (a
  creature's classes, a door's generic appearance, an item property's
  parameter, the portraits) were not held to Aurora's order.
- The model viewer's wheel lacks Ctrl and pinch zoom and Shift's
  sideways, which the area view's has (`model_view.rs:745`,
  `area_view.rs:3657`), and its orbit is `area_view::Orbit` again.
- A tileset read by hand where it is open: `tile_view.rs:19`,
  `model_view.rs:1208`, `tabs.rs:396`, `area_grass.rs:266`,
  `area_reshape.rs:146`, `area_props.rs:261`. One `Moonglow::tileset`.
- The eight corners of a box five times (`model_view.rs:441`, `:459`,
  `area_view.rs:3898`, `mg-preview compose.rs:546`, `mg-render
  renderer.rs:1709`); lightcolor.2da's toolset colors twice
  (`tile_select.rs:375`, `area_view.rs:5080`).
- A module's hak list read four ways (`Module::haks`, `lib.rs:3658`,
  `nui_view/images.rs:29` with no fallback, `module_props.rs:351`);
  `hak_dir` and `hak_folder` the same (`hak_view.rs:194`,
  `nui_view/images.rs:44`); a hak's file found by hand five times
  (`install.rs:299` and four `hak_dirs().find`).
- Scripts opened past Options' external editor: Open Script, Save As,
  New Script and the NUI Creator's openers push the tab themselves
  (`script_view.rs:1695`, `:1852`, `:1877`, `nui_view.rs:743`,
  `nui_view/events.rs:144`); whether they mean to, to be asked.
- Compile Models' report twice (`hak_view.rs:650`, `lib.rs:3195`), the
  "Files Already in…" window twice (`hak_view.rs:710`,
  `model_view.rs:264`), "A, B and N more" by hand five times beside
  `transfer::listed`, and the 2DA and talk-table file dialogs with no
  kind of their own (`FileKind::Any`).
- NWSync publishing on a thread of its own, without the jobs' cancel
  and panic handling (`nwsync_view.rs:113`).
- Compile reads an include's module copy, the live check the open
  editor's (`script_view.rs:1295`, `script_nav.rs:230`): an include
  with unsaved changes passes one and fails the other.
- `DialogSearch` keeps its own match options and rebuilds
  `SearchOptions` twice (`dialog_view.rs:138`, `:1464`, `:1547`).
- The wizards' "choices of a table, none without a game" six times,
  each copying the kept list (`area_props.rs:25`, `creature_wizard.rs:85`,
  `creature.rs:115`, `levelup_view.rs:77`, `situated.rs:207`,
  `Form::choices`); the Variables button three times (`Form::variables`,
  `widgets::variables_button`, `area_props.rs:645`); the model-number
  scan and `set_part` twice (`item.rs:244`, `:308`, `creature.rs:405`,
  `:451`; the creature's drops the three-digit check); the autosave and backup default of five
  minutes in four places; `ui.label` where `field_label` is meant
  (`wizards.rs:67`, `:86`, `:106`, `faction_view.rs:263`,
  `store_wizard.rs:355`, `:358`, `widgets.rs:369`, `:604`).
- A value as a whole number or text, three times each (`search.rs:415`,
  `gff_view.rs:41`, `new.rs:178` without the 64-bit types;
  `areas.rs:249`, `query.rs:84`, `search.rs:396`); a 2DA row's name
  through the talk table (`rows.rs:146` filters `****` labels,
  `choice_list` doesn't) and some ten `game.string(StrRef(..))` by hand.
- `rename`'s `usages`, `tag_usages` and `mentions` restate what
  `UsageIndex` answers. (`#include`, read three ways, is read one way
  since 2026-10-28: `mg_script::lex::includes`, which knows comments.)
- In mg-mdl: `compile.rs:93` `tangents` has no caller (a different
  normalize from `engine_tangents`); `sub`, `dot` and `cross` three
  times; `PartNumbers::read` (`binary_write.rs:296`) walks nodes beside
  `Bin`. PLT coloring twice (`mg-render assets.rs:119`,
  `images.rs:221`).
- The grass mutations (`mg_module::grass` and `mg_area::grass`) agree
  now but are still two (the edits wanted by the area view against the
  module's mutations). (Making placeables static is one since
  2026-10-22: `areas::static_plan` and `static_fields`.)
- The plugin's PNG reading differs from the NUI skin's (16-bit samples
  cut against rounded, limits by pixels against bytes and sides:
  `mg-plugin files.rs`, `nui_view/skin.rs:342`); the TGA's are one
  since 2026-10-21.

## A project's rules (the builders' proposal, C1 to C5)

The Domains of Dread's builder proposal (its section 3): the project file
(`moonglow.toml`), Make Placeables Static and Setup Store following it,
and duplicate tags on paste. What is left:

- **A builder code isn't checked against the team's:** two builders who
  choose one code aren't told; a claims table (C6) could list the codes.
  (S)
- **Described variables (C2) are not described further:** an object's or
  a location's value has no description of its values; no `default`,
  no "required on" (a proprietor needs both `tdod_rental` and
  `tdod_rental_door`), no rule between variables (as a table's
  `[[rules]]`), no description of a variable by the script that reads
  it. (M)
- **The project file is read again only with reloading on** (Options ›
  General), when the folder's watch sees a change; a window open then
  (Setup Store) keeps the rules it opened with. (S)
- **Setup Store's blueprint list** doesn't scroll to the project's
  blueprint, chosen at first, when it is far down the list. (S)
- **Duplicate tags are said after a paste and an area's Copy… only:**
  not when a tag is typed in Properties, an object is placed from the
  palette (its blueprint's tag), Create Set names waypoints, Find and
  Replace or `mg` edits change tags, nor for a blueprint copied with its
  tag. (M)
- **Retag changes the copies' tags only:** what names the old tag (a
  copied area's doors' `LinkedTo`, scripts) still names the original's;
  an area's copy's other duplicated tags are counted, not retagged. (M)
- **A paste reads every area's objects for their tags:** those not open
  are parsed from the module each time (not kept, so that a save doesn't
  rewrite them); in a persistent world of hundreds of areas a paste may
  take a moment. An index kept by the workspace's revision would answer
  at once. (S)
- **`unique_in_area` is quiet about other areas whatever
  `paste_warning` says:** with `"any"`, a store's tag pasted into
  another area isn't said, as the project says it may be there; and
  Setup Store's `{n}` still takes the first number free in the module,
  not in the area. (S) (the Domains of Dread's setup on 2.1.0)

## Claims, protected areas and the project's commands (the builders' proposal, C6 to C8)

The Domains of Dread's builder proposal (its section 3, and the hand-back
proposal's section 8.2 to 8.4): the claims table, claimed areas read-only
or asked about before a save, Claim This Area and Release Claim, merged
areas' Tag and ResRef asked about before they change, `mg claims`, and
File › Project. What is left:

- **A read-only area still takes the pointer:** its objects can be
  selected and dragged, its Properties typed in; each change is then
  refused with a line in the log (the area view doesn't turn its tools
  off, nor does a form go gray). (S)
- **The window can't take another builder's claim** (only `mg claims
  --force` does), nor hand one to another builder, nor show the table
  whole (`mg claims` lists it). (S)
- **A claims table changed by git while a save waits** is read again by
  the folder's watch (with reloading on); the Areas Not Yours window
  keeps the list it opened with. (S)
- **The protected-name question is asked per change:** "Change It
  Anyway" lets one change of that area through; a Rename window opened
  by "Rename It Anyway" and then closed leaves that one pass for the
  area's next change. (S)
- **A project's command's output is in the log only:** no window of its
  own to read a long output in, and no input (a command that asks for
  a password waits until canceled; its stdin is closed). (S)
- **The project's commands on Windows** (`cmd /C` with the command's
  text as written, no console window) are tested by CI's runs of the
  UI tests, not on a builder's machine with a real update script. (S,
  needs Windows)

## Release and packaging

- **macOS:** the app and disk image are built by CI but have never been
  run on a Mac. (S, needs a Mac)
- **Windows signing:** the installer isn't code-signed, so SmartScreen
  warns. (S, needs a certificate)

## Questions asked, unanswered

Each waits on its reporter: with no word by 2026-11-12 (a month from
the owner's ruling of 2026-10-12) it is dropped, and said so in the
log.

- **xtul's mirror at full size in the game:** the client draws a
  dynamic placeable with Scale 0.46 at 0.46 (`visual_transforms_look`),
  upright and turned on its stand by X Rotation -50.8; at full size and
  unturned only when it is static. Whether his mirror is static in the
  module the game ran (a build step that makes placeables static, or
  Static ticked after the transform), or the full size was by eye,
  would settle it. (S) (xtul's report, October 2026)

- **The rectangle by two clicks on a builder's Linux:** 1.25.1 made the
  right Alt place it, which was not his trouble; the next release has Alt
  pressed and let go before the click, on the reading that his window
  manager keeps a click with Alt held (the rectangle went when he
  clicked and came back when Alt was pressed again). Not seen on his
  system: his word that it places would settle it. (a builder's report,
  October 2026)
- **Creature Properties' overlapping elements:** reported, not
  reproduced: every page is checked for widgets drawn over each other at
  four window sizes (`blueprint_editor_pages_draw_nothing_over_anything_else`)
  and none are. A screenshot, or the page and the interface size, would
  find it. (S) (a senior builder's review, October 2026)
- **A drag on a number field moving objects in the area behind:**
  reported, not reproduced
  (`a_drag_in_a_window_over_the_area_moves_nothing_there`). (needs the
  steps) (a second builder's review, October 2026)
- **"It kept randomly exiting"** on a persistent world's module (ten
  times before it stayed open), reported without a log. One cause was
  found and fixed: the compiler walks a script by recursion, and a very
  long `else if` chain or expression overflowed the stack of the thread
  it ran on, which ends the program at once and leaves no crash report
  (a 60,000-link chain did it on Linux; Windows' main thread has an
  eighth of that stack). Compiles run on a stack of their own now.
  Whether that was the reporter's crash isn't known: asked for the debug
  log (Options › General) and what was being done. (needs the log)
  (from builders' reports after 1.14.1)
- **A hak's 2DA not seen until a restart** (a builder: a creature
  showed "appearance.2da has no row 8241", from a hak above CEP's whose
  table has more rows; gone after restarting Moonglow): not reproduced.
  Haks and folders that change on disk are read again every few seconds
  and their tables forgotten, which a test holds for the override
  folder. Suspected and not shown: `reload_resources` and `sync_haks`
  give up without a word while anything else holds the game data (a
  job), and try again later; on Windows a hak Moonglow has open cannot
  be replaced by another program at all, so a hak tool may have failed
  to write it. If it comes back: the log around it, and whether Tools ›
  Reload Resources cures it. While
  something else holds the game data, the look is put off and the
  debug log says so ("the game data is held"), which would show this.
  (S–M) (from builders' reports after 1.19.1)
- **"Area transitions can't be selected visually":** not understood;
  asked.

## Settled: as it is, by decision or by design

Nothing is left to do on these; they are kept so that a report of one
is not taken for a new gap.

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
- **A window's bar:** egui_dock gives a folded window the height of its
  tab bar, frame included, so the dock's windows are drawn without a
  frame margin and their tabs' contents are given one instead.
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
  by their git identity. Builder codes aren't checked to be unique. (S) (Settled in the triage of 2026-10-28.)
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
  model's light nodes (animated) and the appearance's light, not a
  part's. (S) (from Troubletcat's and Heed's reports, October 2026) (Settled in the triage of 2026-10-28.)
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
  Aurora's ✕ discards without asking (seen on the oracle); Moonglow asks,
  as the builder asked. (S) (from Jasperre's tile feedback, October 2026) (Settled in the triage of 2026-10-28.)
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
- **The palette's header, not as drawn in the issue:** "More" is a
  word, and the clear button "×" and the Gallery's "⊞" are the glyphs
  egui's fonts have (the mock-up's were boxes); the icons are the
  toolbar's glyphs, not a drawn set; the tile palette and the prefabs
  keep their own bodies under the same two top rows. Seen in one
  rendered picture, with the dropdown (not with the row of icons). (S)
  (GitHub issue 16) (Settled in the triage of 2026-10-28.)
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
  Moonglow's own shader makes a cut-out solid before it lights it, so it
  draws no line: it differs from the client there. With The Domains of
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
- **`mg test`'s results outside a test:** a result reported while no
  test is running (a delayed assertion after its test ended, a
  module's own driver) goes under "(outside a test)", since NWScript
  does not say which script is running. (S) (Settled in the triage of 2026-10-28.)
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
  variables are `--var`'s, in the GIT where the game reads them). Give
  Every Area This Page's Visual Settings gives an area of another
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
  targeting, `scriptable*`; the model viewer and previews have no
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
- **The shadow map stands in for the game's stencil volumes**
  ([notes](research/notes_shaders.md) §G.2): a volume torn by a
  non-manifold caster isn't drawn as the game draws it, and the map's
  edges are a texel soft. (2026-10-28)
