---
type: Backlog
title: 'Deferred: what was left out, for a decision later'
description: What finished work left undone or unchecked, filed by area of the program (an entry's last words say which report, issue or review it came from), each awaiting a verdict - fix, add or drop; what is settled by decision or design is gathered at the end.
tags: [backlog, deferred, after-parity]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-18T21:05:31Z }
---

# Deferred: what was left out, for a decision later

What finished work left undone or unchecked, in one place: each entry
awaits a verdict (fix, add, or drop), and new gaps are added as work
finishes. The list is filed by area of the program, one entry a gap, in
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
- **The sample is fixed:** an everyday run sweeps the same sixth of the
  modules and tenth of the resources each time; what only the rest
  holds is seen before a release. A sample that moves with the date
  would cover everything over a week, at the price of failures that
  come and go. (S)
- **`game_ascii_census` is run by hand** (`mdl_lint.rs`, ignored): it
  asserts nothing, and what it should hold the game's ASCII models to is
  undecided. (S)
- **macOS:** the command-line tests' own data folder is set through
  `XDG_DATA_HOME` and `APPDATA`; on macOS it follows `HOME`, which the
  tests leave alone (it finds the game). (S)
- **`app.rs` in one file:** 15,343 lines in chronological order; a
  dozen subject modules under the same binary, a harness helper for the
  47 repeated installs and 115 hand-built harnesses, and the GPU lock
  only for tests that draw an area (a dozen take it for nothing). (L)
- **`mg` links the GPU stack** through `mg-preview` and never uses it:
  96 crates of its build. (M)
- **A version bump rebuilds all 28 crates** (`version.workspace`); the
  version in the two apps alone would keep the libraries' artifacts. The
  viewer takes the crates by git tag, which this does not change. (S)
- **Shared test fixtures written many times:** the `mg` runner's
  wrappers (10 files), `fn lines` in the Aurora tests (6). (M)
- **CI's time with the new workflow is not measured:** its first run
  passed on the three systems from an empty cache (14 to 24 minutes);
  compare a warm run with the 8 to 10 minutes of before. (S)

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
- **The script editor's selection** is drawn by Moonglow, to each
  line's last character (the text widget's own drew a selected line end
  as half a character more, which a builder read as a space added).
  Only there: a script or a text shown in the resource browser, and the
  other text fields, have the widget's own. (S)
- **The longest constants in the side lists:** the lists open wide
  enough for names of up to 44 characters (every function, all but some
  160 constants, `PLAYER_DEVICE_PROPERTY_…` and the like, of up to 67);
  those end in "…" until the panel is widened. (S)
- **Struct members:** go to definition, references and rename don't
  follow `p.nX` to the struct's declaration. (M)
- **Incremental compiles:** Compile All Scripts passes over a script
  that, with what it includes, is as it was when this window last
  compiled it (`mg_module::build::script_stamps`), as Arelith's
  ARE_Compile does. What it remembers
  (`build::Remembered`: each script's stamp and its compiled script's)
  is kept from one session to the next in Moonglow's data folder, a
  file a module (not beside the module: nothing is written into a
  builder's repository or module folder), and holds only for the same
  compiler, settings, haks and content folders, and while a compiled
  script is the one that was made. `mg compile` reads and writes the
  same file (`build::compile_changed`, `build::made_with`), so each
  passes over what the other compiled. Not done: with an external
  compiler chosen in the window the two do not share (`mg compile` is
  always the built-in one, and what is remembered holds for one
  compiler); a project named by its `nasher.cfg` to one and by its
  folder to the other is two modules to them (what is remembered goes by
  the path as given, made whole); `mg compile --uncompiled` and
  `mg build` neither read nor write it; a script
  compiled by
  its own Compile or by a save is compiled once more by the next
  Compile All (to the same bytes). (S each)
- **Scripts in the external editor only:** Open scripts in the external
  editor opens Moonglow's editor too (it holds the script and takes the
  external editor's saves back); opening the external one alone isn't
  offered. (S)
- **Editor setups:** the manual's Neovim and Helix snippets are untried
  in those editors. On Windows, `mg lsp` is checked only by CI's
  tests. (S)
- **A VS Code extension:** today VS Code needs a generic language-server
  extension pointed at `mg lsp`. A small Moonglow extension would make
  that one click. (M)
- **The external compiler** (Options › Script Editor): run as a program
  on the module's scripts written to a scratch folder, with what it made
  read back. Tried with `nwn_script_comp` (its bytecode is the built-in
  compiler's, byte for byte); `nwnsc`'s line is from its documentation,
  not run. All of the module's scripts are written out for every
  compile, one script's too. Scripts in haks reach it through `{haks}`
  (nwn_script_comp's `--erfs`); nwnsc has no such option. Errors as you
  type, the wizards' own scripts and the `mg` commands stay with the
  built-in compiler. Its messages are matched to scripts by their file
  names and the word "error": another compiler's wording may be missed
  (the script is then said to have made no compiled script). (S–M)
  (from builders' reports after 1.14.1)
- **Compiled on saving the module:** with Automatically Compile Scripts
  on Save, File › Save compiles the scripts whose text it stores (only a
  script's own Save did; a builder found scripts left uncompiled). A
  script saved earlier and never compiled is not caught by it (Compile
  All is); one the external editor saved is compiled when its text is
  next stored here. (S) (from builders' reports after 1.16.1)
- **A project's script in the external editor** opens in place (the
  project's own `.nss`), and its saves come back by the reading of files
  changed outside, so not at all with that turned off in Options ›
  General (the copy's way is used then). Scripts of a module file still
  open as a copy in the temporary folder. Tried with a stand-in editor;
  not with VS Code. (S) (from builders' reports after 1.16.1)
- **Find In Currently Open Scripts** searches the scripts whose editors
  hold their text, which includes one closed with text typed and not
  saved. (S) (from builders' reports after 1.16.2)

## nasher projects

- **NWNT projects:** projects using NWNT instead of JSON for GFF files
  are refused with a message. (M)
- **Compiled scripts without source:** scripts that exist only as `.ncs`
  are lost when a module becomes a project, as with nasher; Moonglow
  names them. It could keep such `.ncs` files in the project instead. (S)
- **A nasher project's file that isn't UTF-8 throughout** (a Polish
  module's conversation: "invalid unicode code point") is read with its
  stray bytes as the project's code page has them. The reporter's file
  wasn't seen; a file broken another way still stops the project from
  opening, where it could be left out and said. (S)
  (from builders' reports after 1.14.1)
- **A talk table kept as JSON in a nasher project** is not read or
  written with the project (a builder keeps one, and may keep several,
  one a language): Import JSON and Export JSON by hand for now. (M)
  (from builders' reports after 1.19.1)

## Modules kept as folders

- **A module folder worked on in place** (a builder keeps the scripts
  open in Notepad++, as with Aurora, and asked whether Moonglow reads
  and writes them where they are). It read the folder once, when the
  module opened: a script saved outside since was written over by the
  next save here, with what Moonglow had read, and a file added outside
  was removed (it was no resource of the module). A folder's files are
  now followed as a nasher project's are (`mg_module::folder`): those
  changed, added or deleted outside are read again every few seconds;
  a save leaves a file changed outside whose resource Moonglow did not
  change, leaves a file new to the folder, and writes nothing (saying
  which files) where a file was changed outside and here both, for
  Changed Outside Moonglow to ask. The external editor works on the
  folder's file. A script read again is compiled
  where Automatically Compile Scripts on Save is on (a nasher project's
  too), and so are the scripts that include it (a script's own Save
  and Compile, and saving the module, compile the includers too; of
  these only the reading of a file changed outside is under test). More than 24 at once (a checkout, or an include that many scripts
  use) are compiled as a job, off the window's thread: a save that
  starts one is written before the job's compiled scripts are in, and
  the module has unsaved changes after. With an external compiler, which is run
  once for each script compiled on the window's thread, more than one
  go to the job too. Not done: up to 24 are compiled on the window's
  thread by the built-in compiler (a few milliseconds each; a job
  would leave every such save with unsaved changes after it); a module
  whose script was compiled so has unsaved changes (the compiled
  script, written by the next save); the reloading needs the game's data
  open, as the game's own reloading does; a first Save As into a folder
  that has other files still removes the resource files the module
  lacks; `mg` commands save over a folder as before (they open, change
  and save at once). (S each) (from builders' reports after 1.19.1)
- **A module folder with a resource under two spellings** (a builder:
  Aurora named two new items `x.UTI`, Moonglow wrote them again as
  `x.uti`, and the folder had both). A folder's resource under another
  spelling than Moonglow's (lower case) is renamed at the next save;
  where both files are there (a folder that sets capitals apart), the
  one changed last is the resource when the folder opens, and the save
  leaves the one file; the log says which was read. A nasher project
  takes the first of two it finds and says so, where a folder takes
  the one changed last: the two are not alike. (S)
  (from builders' reports after 1.19.1)
- **A module's files changed outside** (a nasher project's sources, a
  module folder's files; a `.mod` file changed outside isn't noticed)
  are found by a thread of its own since 1.20.2 (their names, times and
  sizes every 3 s) and read again on the window's thread. The files'
  times and sizes decide what is read, so a change that keeps both
  within the clock's grain is missed. A reload that touches resources
  the undo history has changes to clears the whole history. A script
  open in the editor with typing not yet applied counts as changed
  here. The area list isn't settled again for areas added outside. Not
  tried against a real `git checkout` of many files, nor measured on a
  large module (a builder's window stalled every few seconds with a
  large module folder or nasher project, after 1.20.1, while the
  window's own thread did the looking); the read after a change
  (another program's save, or Moonglow's own) still looks at every file
  once on the window's thread. The haks and the `override`,
  `development` and portrait folders (and the custom talk table) are
  watched the same way: looked at on the window's thread only after a
  change. A watch said nothing of a change made before its thread's
  first look; it says so after that look now. (M; what is left of the
  stall S)

## Content doctor

- **The walkmesh crash rule:** nwn.wiki names "walkmesh outside the
  tile" as an Aurora crash cause, but the game's own tiles reach 10 m
  out and work. The real cause is unknown, so there's no check.
  Finding it would mean crashing Aurora on purpose with test tiles. (M)

## Test loop

- **`mg serve`** (the server from a terminal): a scratch user directory
  is one for a module, so two servers of the same module at once share
  it (name one with `--dir`); Ctrl+C ends `mg` at once, and the server
  with it without its `exit` (its database is not closed in order); a
  nasher project is packed as it is, its scripts not compiled first
  (`mg compile` or `mg build` before); `--restart-on-change` looks at a
  file's size and time twice a second, and at a folder six levels deep.
  The server looks up Beamdog's master server at its start even when not
  listed: `mg serve` does not cut its network off. Not tried on Windows
  (the console on pipes, the links for haks). (S each)
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
- **`mg test`**: a result reported while no test is running (a delayed
  assertion after its test ended, a module's own driver) goes under
  "(outside a test)", since NWScript does not say which script is
  running; a test that needs a player in the module has none (no client
  is started); the module's own scripts are compiled only where they
  have no compiled script (run `mg compile` first); `--driver none` (a
  module that runs its tests itself) is held by a test, but was not
  tried with NWNX. (S, M, S, S)
- **Choose Character:** Test Module, Choose Character uses the game's
  `+LoadNewModule` (from nwn.wiki), never run in the game client. (S)
- **Programs Moonglow starts:** the game (Test Module) and the external
  script editor are waited for on a thread each, so they leave no
  "defunct" process; checked on Linux only. (S)
- **Aurora's F9 problems:** laggy combat, AI timing errors and damaged
  modules after a crash. Moonglow launches the game as the wiki recommends
  to avoid them; nothing has checked that they're gone. (S–M)
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

- **Real CEP haks:** the test world's 150,000 hak files are tiny stand-ins.
  Loading real CEP content at scale isn't measured; the campaign budgets
  and model corpus tests cover real models. (S, needs CEP downloaded)
- **Other machines:** budgets were measured only on the development
  machine (Ryzen 7 5800X3D, Radeon RX 9070). (S, needs other hardware)
- **Inventory lists:** a store page or inventory lays out every row and
  builds every item icon when first shown. 449 icons take
  0.68 s; a page of thousands would take seconds. Laying out only the
  rows on screen would fix it. (S–M)
- **Where-used at scale:** the References window and Rename keep the
  module's references between questions (`rename::UsageIndex`): the
  first Find References in the persistent world reads the module as
  before (0.8 s), the next ones only the resources changed since
  (7 ms). Tags and the strings in scripts are answered from it too.
  Not done: the first read is on the window's thread; `mg refs` reads the module each time, as a
  command run once must. (S each)
- **Long lists opened out:** the tree's rows out of sight take their
  room only; its groups' names are still gathered and sorted every
  frame, which a module of tens of thousands of resources may feel. The
  palettes laid out only what is in sight already. Not measured on the
  reporter's module. (S–M) (from builders' reports after 1.16.2)

## Frames (a steady 144 a second)

The budget and the measurements are in
[the frames note](research/notes_frames.md). Done: an area's picture kept
between its animations' steps, its drawing at a fifth of the cost (lights
by the ground they reach, materials worked out once, a draw's values by
its number, what is out of sight left out, tiles of one model posed
once, meshes of one model drawn as instances of one draw), the
galleries' pictures made for 2 ms a frame and kept while in sight.

- **The largest areas in full view:** a 784-tile area's picture takes
  4.1 ms with all of it in sight (1.9 close up). An area of 32 by 32 tiles
  of a tileset with more meshes a tile is near the budget in full view.
  What is left is each mesh's own work, instances or not: its lights
  chosen (0.7 ms of 2.7) and its values written. Kept from frame to frame
  for what doesn't move, with the camera's part worked out in the shader,
  it would be done once. (M–L)
- **Animations step 25 times a second** (`ANIMATION_STEP`, 40 ms) while
  only they move; before, they stepped with every frame the window drew
  (20 a second left alone, 144 with the pointer moving). A setting, or
  every frame once the picture is cheap enough. (S)
- **Opening an area holds the window** for 0.2 s on a large one: its
  models and textures are read and uploaded in the frame. In a job, with
  the view filling in. (M)
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
- **Pictures that differ from run to run:** 10 of 192 pictures of the
  campaigns' areas differ by a few pixels to a few hundred between two
  runs of the same program. Something is ordered by chance (a hash map's
  order, likely among lights or an object's parts). Not looked for. (S)
- **The frame test only times:** `frame_perf.rs` names the views over
  the budget and fails nothing. With budgets it would keep them. (S)
- **The tree and the lists at persistent-world sizes** are as they were
  (see above and "Long lists opened out"): 3.4 ms with 25,000 resources
  listed, 5.9 ms for a store's 1,000 items.

## EE fields Aurora hides

- **Custom shader effects:** shader parameters and the extra area flags
  are saved and the game reads them. Moonglow's renderer doesn't run
  custom shaders, so they don't change what the area view shows. (L)
- **Item costs:** Additional Cost keeps Aurora's limit. The game prices a
  created item itself, as Moonglow computes the stored Cost (see
  [Findings](findings.md#created-items-are-priced-anew)), so nothing seemed
  to need it. (S to check)
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
- **The Classes page in a narrow window:** the second domain picker
  sits at the window's edge. (S)

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
- **The model in other editors' pages:** a placeable's and a door's
  Basic page show it beside their fields where the window is wide enough
  for both; narrower, Preview shows it. An item's Appearance page has it
  beside the fields, or in a narrower window beside the icon (a
  creature's Appearance page stacks them instead). The viewer frames an
  item as any model: a sword lying flat is small in it until turned. (S)
  (a senior builder's review, October 2026)
- **Aurora's faults Moonglow was checked for:** an epic creature of
  several classes had Aurora's wrong saves (nwn-issues 816) and has the
  game's now, measured in the server ([findings](findings.md)); the
  abilities count the Great feats. Not done: whether the game adds the
  Great Constitution feats' hit points itself was not measured (the
  stored maximum stays Aurora's); an item appearance of 100 or more
  (nwn-issues 658) is listed by what models there are (three digits),
  with no base item's range read, and was not tried with such a model;
  a custom caster class's spells (566) are offered by `classes.2da`'s
  `SpellCaster` and `SpellTableColumn` alone, and were not tried with a
  custom class. (S each)
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
- **View is read-only by its look and by what it refuses:** fields are
  dimmed and a change tried is dropped with a note, but a field still
  takes the pointer and the keys until then (so its lists scroll and its
  pages open): what is typed goes when the field is left. (S)
  (a second builder's review, October 2026)
- **Random item properties** in the Item Wizard, as Aurora's. (M)
  (a second builder's review, October 2026)
- **Inventories, what is left** (done: the Creature Wizard's ResRef and Tag, its appearances
  as pictures with Find, and every portrait; the inventory's Equip to a
  free slot, To Backpack and Open Blueprint; talk-table text in its own
  color; a Variables window that is sized; drag and drop and right-click
  menus in the inventory, items dragged between inventories and within
  a list, Copy and Paste): an item carried between
  inventories is made anew from its blueprint where it lands (what a
  placed object's own copy had is lost), one at a time, and the drag
  between two objects' windows is not under test; a moved row keeps its
  place in the game's inventory grid (only the list's order changes); a
  slot-to-slot move and
  equipping from the backpack are two undo steps each, and skip the
  question about a missing feat when moved between slots;
  blueprints made in a category the palette hides;
  new strings written to a talk-table range; equipping from the
  backpack makes a placed creature's item anew from its blueprint. (S–M)
  (a second builder's review, October 2026)
- **Appearances as pictures** are placeables', creatures' and doors';
  an item's model, and each of a weapon's three parts, are chosen from
  its icons (the item as it would be with each). An armor's parts are
  still numbers: no pictures of them, on a body or alone. A creature's
  look is a plain body of the appearance, male unless the creature is
  female. (S–M) (from builders' reports after 1.10.1)
- **Armor and cloaks shown on a woman** (the item's Appearance page,
  Shown on): the model and the page's icons. The icons in inventories
  and the palette stay the man's, and the choice lasts until Moonglow is
  closed. (S) (from builders' reports after 1.10.1)
- **A held item's own Properties** (a placed object's inventory, right
  click › Properties) are found by the item's place in the list: with
  the window open, an item added, removed or moved before it in that
  inventory leaves the window on another item (or none). A blueprint's
  items (which are named, not held whole) open the item's blueprint
  instead; a placed store's weren't tried. (S)
  (from builders' reports after 1.10.1)
- **Spells as the Feats page is now** (those known beside those to
  choose from): asked for, and left: the page also has levels, classes
  and memorized counts to lay out. (M)
  (from builders' reports after 1.16.2)
- **Arrows after a creature's statistics** (abilities, natural AC, hit
  points, save bonuses), as Aurora's fields have; the other number
  fields have none. (S) (from builders' reports after 1.19.1)
- **What a store sells an item for** (a builder: Aurora lists it): each
  item of a store's pages is listed with its cost at the store's sell
  mark up. Not compared with Aurora's list (its rounding, whether it
  counts a stack); the buy price (mark down) is not shown. (S)
  (GitHub issues 10 to 24, after 1.20.0)
- **Arrow keys on lists of choices:** the blueprint editors' 2DA lists
  and part lists take them while the list has the focus (clicked, open
  or after a choice; Tab). Escape gives the focus up, as everywhere in
  the interface, so after closing a list with Escape it is clicked
  again. The lists outside the blueprint editors (Area Properties, the
  area's toolbar) don't take them. (S) (GitHub issue 7)
- **Pictures in the Appearance list:** a creature's (a plain male body
  of the appearance) and a placeable's, beside the row the pointer rests
  on; a door's and an item's lists have none. (S) (GitHub issue 7)
- **An item stood up in its viewer** is turned a quarter about its
  length's axis from how it lies; which side then faces the viewer isn't
  chosen (a sword shows its edge more than its flat). Items in the
  palette's Gallery lie as modelled; on the ground in an area they are
  turned as baseitems.2da's `RotateOnGround` says. (S)
  (GitHub issue 7)

## Bulk editing

- **Make Placeables Static, what it can't see:** a placeable that a
  script destroys, animates or makes usable by its tag looks like scenery
  and is made static; nothing scans the scripts for its tag. (M)
- **Make Placeables Static against the game:** that static placeables
  load and draw more cheaply is from the wiki (the client merges them
  into the tiles' mesh); no load times were measured. (S)
- **Lists across several blueprints:** Edit Together leaves out the
  pages that edit lists (inventories, classes, skills, feats, spells,
  item properties); those are edited one blueprint at a time. Adding the
  same item or feat to each would need its own design. (M)
- **The multi-object editor's list pages:** with several placed objects
  selected, those pages still change only the first object.
  Edit Together's guard (leave the pages out, drop changes that reach only
  the first) could apply there too, once Aurora's multi-editor is checked
  for what it allows. (S, plus an Aurora capture)
- **Edited together, shown as the first:** a field whose value differs
  among the areas, blueprints or placed objects edited together has "≠"
  before it (of up to 65 of them). Not marked: a field in several
  languages, a memo, a slider, a palette color, the appearance and
  loading screen choices, the category and the conversation, an area's
  flags (one field for several boxes) and its Audio page, and the lists
  of an editor's own (a code review found the first seven never call
  `Form::mark_mixed`; nothing says which fields are meant to have the
  mark, and a painted mark is in no test). (S, needs the intent)
- **Choosing areas by more:** the chooser filters by name, ResRef,
  tileset and the three kinds; not by a property's value (areas with a
  given music or variable), and its choice isn't kept as a named set.
  (S–M)
- **`mg areas` and lighting schemes:** the command sets fields, flags
  and variables; it doesn't apply a lighting scheme (environment.2da,
  which also picks each tile's lights, and needs the game's data), nor
  set names or list fields. Nor does Give Every Area This Page's
  Settings: it copies the fields the page shows, not the tiles' lights
  a lighting scheme sets (GitHub issue 6). (S)
- **Find and Replace beyond strings players read:** tags, resrefs and
  other plain text fields aren't searched (Find References covers tags,
  Find in Files covers scripts), and there are no regular expressions.
  (S–M)
- **Prefabs in the palette:** listed by name, placed with a click,
  renamed and deleted from their menu; what one holds is told in words on
  hover (kinds and tags), not shown as a picture; no folders. (S)
- **Variable sets** hold int, float and string variables only, as the
  Variables window edits; object and location variables aren't kept. (S)
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

- **Spell checking** in the conversation editor (and text fields). The
  game ships no dictionary, so it means the system's:
  - Linux: hunspell dictionaries, read by a pure-Rust crate (`spellbook`),
    a new dependency to download.
  - Windows: the Windows Spell Checking API (Windows 8 and later).
  - macOS: NSSpellChecker.

  One interface over the three, words underlined in the text fields. (M–L)
- **Other languages in exports:** Twine, Ink and CSV carry the English
  text only. A CSV could carry a column per language for translators. (S)
- **Parameters, animations and delays** don't go to Twine or Ink.
  Conditions and actions go by script name only. (S–M)
- **Articy:draft** isn't read; its JSON export could map like Twine. (M)
- **Ink beyond the subset** (stitches, gathers, inline logic) is refused,
  not converted. (M)

## Palettes

- **The palette's header, not as drawn in the issue:** "More" is a
  word, and the clear button "×" and the Gallery's "⊞" are the glyphs
  egui's fonts have (the mock-up's were boxes); the icons are the
  toolbar's glyphs, not a drawn set; the tile palette and the prefabs
  keep their own bodies under the same two top rows. Seen in one
  rendered picture, with the dropdown (not with the row of icons). (S)
  (GitHub issue 16)
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
- **Favorites and Recent** are Moonglow's, not the module's: a custom
  blueprint's favorite shows only in the module that has it. Per-module
  lists could live beside the module. (S)
- **The Gallery:** up to 192 pictures are kept at once (more are made
  again as they come back into sight) and all are made again when a
  blueprint changes. (S) (from builders' reports after 1.10.1)
- **A picture of a model that emits** is drawn four seconds in, framed
  with its particles (a flame is small under its smoke). (S)
  (from builders' reports after 1.10.1)
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
  right by opening and saving it once: the palettes are made again at
  every save, and categories left in UTF-8 are written in the game's
  bytes there and in the module's own category lists (a test spoils a
  saved palette as 1.19.3 wrote it, opens the module and saves it). Not seen in the game's DM client here (the bytes are
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

## Custom content data (2DAs and talk tables)

- **`mg tlk-alloc`** (a custom talk table from strings kept by name):
  new keys take lines in the order of their names, not of the file; a
  key's feminine text is not set (the feminine table's lines stay as
  they are); a retired key's line keeps its last text in the table. (S
  each)
- **`mg 2da-diff` and `mg tlk-diff`** compare and merge by row number
  and column name (line number for a talk table). Not done: an edit
  file takes the table whole (`set_resource`), since edit files have no
  change for one cell; a row is its number, so a row inserted in the
  middle reads as every row after it changed, and two rows added under
  one number are a conflict to be settled by hand (no renumbering, which
  would need every reference to the row changed too); a talk table's
  merge takes a line whole (its text, sound and length together); a
  feminine table is compared as a file of its own. (M, M, S, S)
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
- **One language:** the editor edits the table in its own language;
  translated tables (the same name in other languages' folders) aren't
  shown side by side. (M)
- **Where a StrRef is used:** Find References takes a talk-table line
  (a number typed in its field, or Find References in the talk table's
  editor) and lists the module's localized strings that name it
  (blueprints, areas, conversations, the journal) and its 2DAs' cells
  in the columns known to hold StrRefs. Not done: the haks' 2DAs are
  not looked in (they are not the module's), nor fields that hold a
  StrRef as a plain number, nor scripts (`GetStringByStrRef`). (S each)
- **Inserting or removing lines in the middle** isn't offered: it
  renumbers every later line, and their users would have to be
  renumbered with it (Find References finds them now: the entry above;
  the renumbering is not written). (M)
- **The client and custom talk tables:** the engine test runs the server;
  whether the client also reads a table from the module (not only from
  haks and the `tlk` folder) for 2DA text isn't tested. (S)
- **Text put in the talk table as it is written** (a conversation's
  lines, names), in a range set aside for it, and reusing a line that
  already has the same text, asked first: proposed by builders, not
  begun. Move to Talk Table does one string at a time. (L)
  (from builders' reports after 1.19.1)
- **The talk table editor** (a builder, and issue 24): Go to takes a
  StrRef or a line's number; the text boxes share the room under the
  list and scroll; the whole row chooses its line and shows the pointer
  over it, the arrow keys choose from there, and Enter or a double
  click goes to the text. Not done: the split between the list and the
  boxes cannot be dragged. (S) (GitHub issues 10 to 24, after 1.20.0)

## Haks in the GUI

- **No zip, 7z or rar:** content downloaded as an archive is unpacked
  first; Add Haks and Talk Table takes the haks and talk table. (S; `zip`
  is a dependency already, 7z and rar would each be a new one)
- **Hak order advice:** Moonglow doesn't know which haks go above which
  (CEP, PRC and the like each document theirs); the conflict report shows
  what each hak hides. (M)
- **Viewing in the hak editor** shows fields, text, a compiled model as
  its text, or the first bytes: no pictures or sounds, a model not as a
  picture, and a 2DA as its text. (S)
- **Update from Folder** takes the folder's files in place of the hak's
  (two undo steps: the removal, then the adding); a hak that was built
  from several folders, or added to by hand, loses what the folder
  hasn't. (S)
- **ERF version:** haks are saved as `V1.0`; EE's compressed `E1.0` is
  read but not written. (S–M)
- **The hak list of Module Properties** (GitHub issue 17) is the
  issue's recommended layout: a framed list numbered from the top, rows
  chosen (Ctrl and Shift for more) and moved by a drag, the arrows under
  the list or Alt + Up and Down, removed with Remove or Delete, one undo
  each (so nothing asks before a removal, as the issue has it); haks
  not in the hak folders flagged; the talk table in a section of its
  own. Not done: a drag does not scroll a list longer than its frame.
  (S) (from builders' reports after 1.19.1)

## Raw fields

- **Rows the fields view doesn't name yet:** an item property's subtype,
  cost and parameter (their table depends on the property), a creature's
  or object's faction (the module's `repute.fac`, not a 2DA), a
  conversation's animations, a store's categories, and fields that hold
  a talk-table number as a plain int. (S–M)
- **Only in the window:** `mg gff`, `mg find --where` and `mg areas`
  print and take the numbers; names aren't shown or accepted there. (S)

## Getting files out

- **One at a time from the tree:** the tree exports one resource at a
  time (the Export window takes several). The area view's To Scratch
  copies the area alone, not the blueprints or scripts it uses. (S)
- **One scratch folder:** To Scratch copies to one folder, for every
  module; a folder per module, or a remote server's (over SSH), isn't
  offered. Export as Files takes any folder, and remembers the last. (S)
- **Export into an archive** is a checkbox, off unless ticked; a builder
  says Aurora adds to an existing archive itself. Aurora's own behavior
  (whether it asks) was not captured. (S)
  (from builders' reports after 1.19.1)
- **Export from a menu** is on the palette's custom blueprints (and the
  module tree's resources, as before). A placed object in an area's list
  has no Export: it is not a resource; it would be written out as a
  blueprint of its own. (S) (GitHub issue 7)

## Area visibility

- **Walkmesh cutters:** not drawn, and not placed (the trigger kind EE
  added). (S–M)
- **A walkmesh's own node position:** one shipped placeable
  (`ptm_candle02.pwk`, Tyrants of the Moonsea) puts its mesh 72 m from
  itself. Moonglow draws it there; whether the engine does isn't tested.
  (S)
- **Lighting off against Aurora:** the Lighting switch's working light
  (white, even, a little from the sun's side) is Moonglow's; how Aurora
  lights an area with its lighting off was not captured. (S)
- **The start location's marker** is still drawn over the view, not
  hidden by what is in front of it, as the selection's boxes and the
  outlines of triggers and encounters are by design. (S)
- **Marker models against Aurora:** merchants, sounds and waypoints use
  the game's marker models in their materials' colors, unlit; that Aurora
  draws them so was not captured. Waypoints' flags are drawn unlit so
  their colors tell apart in a dark area; how Aurora lights them was
  not captured either. (S)
- **Sound ranges:** level circles at the sound's height, not spheres; a
  sound's random position range is a rectangle of `RandomRangeX` and
  `RandomRangeY` each way (that the game takes them as half-widths about
  the sound was not measured). (S)
- **The start location in the view:** set from the menu, and its marker
  dragged (ring, shaft) and turned (arrow's tip). Aurora's palette entry
  for it isn't there; selected, it is no object, so the keys (Q and E,
  the arrows) and Adjust Location don't act on it; a drag stays in its
  area; the pointer doesn't change over it. (S)
- **The camera's height** is each view's and lasts until its tab closes.
  (Lighting and Sound Ranges are kept: a view opens as they were last
  left, and one already open keeps its own.) (S)
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

- **Alt for the camera and the selection:** Alt + right drag turns the
  selection and Alt + middle drag moves the camera (they were Shift's,
  which now leaves the camera alone while the handles show). Aurora
  turns the selection with Shift + right drag; and some desktops take
  Alt + drag to move the window. Not in Options › Keyboard. (S)
  (from builders' reports after 1.10.1)
- **Alt + left drag to move the camera** (GitHub issue 25): begun off
  the selection it moves the camera (the ground following the pointer,
  a closed hand, on until the button is let go, Alt held or not); begun
  on the selection it raises or lowers it, as Alt + drag anywhere did.
  Not done of the issue: the camera's buttons and modifiers in Options ›
  Keyboard (the entry above). (S)
- **Placing onto a placeable as it is placed:** Drop onto Object
  (Shift+G) sets an object on the placeable below it after it is placed;
  the see-through copy that follows the pointer still lies on the ground
  (the walkmesh), under a table or an altar. Shift, Ctrl and Alt are each
  taken while placing (place another, the camera, a rectangle), and on by
  default every drop in a forest would land in a canopy: it wants a
  switch on the toolbar, or a key of its own. (S) (from builders'
  reports on 1.31.0)
- **A see-through copy where it can't be seen:** what is about to be
  placed (a blueprint, a paste, the brush's dab) is drawn 0.8 opaque
  and lit as the area lights it, behind what stands before it like any
  model: in a wall or behind a building only its box's outline shows,
  and a dark model before dark stone reads less than a light one. It
  could show its hidden part faintly through what covers it, or take a
  light of its own. (S) (found looking into the ghosts that didn't show
  in the test harness's pictures)
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
- **Replace Selected with This** makes the object anew from the
  blueprint: what the old one had of its own (its tag, variables, a
  tilt or scale) goes; triggers and encounters aren't replaced. (S)
  (from builders' reports after 1.10.1)
- **A click is on the model, not its box:** an object with a model is
  picked where the pointer's ray meets its triangles (at rest: a creature
  mid-stride is picked where it stands still), so a click through the
  empty part of a wide box reaches the trigger or the object behind;
  where the ray meets nothing so, the nearest box passed through is
  taken, so a thin post is still clicked beside. Aurora's own picking
  wasn't measured. Things without a mesh (markers, an emitter alone) are
  picked by their boxes still. (S) (from builders' reports after 1.14.1)
- **Groups go whole:** the Eraser on a tile of a group takes the group
  away, and a group placed over part of another takes that one away
  (it left the other's remaining tiles standing, and the Eraser did
  nothing on a group's tile). A group whose tiles' corners no plain tile
  fits can't be taken away (the brush is refused). Looked at in Aurora
  since: the Eraser on one tile of a barn takes the barn away, as here;
  but a group over part of another is refused there (its outline turns
  red, the click places nothing; a feature clicked onto a feature's tile
  leaves it too). Moonglow takes the other group away instead, by
  decision (2026-10-07): replacing in place, with Undo, over refusing.
  Aurora asks before a tile brush deletes doors that were edited;
  Moonglow doesn't ask (Undo brings them back). Delete on selected
  tiles still takes a group's tiles one at a time. (S)
  (from builders' reports after 1.14.1)
- **A press with Ctrl held is the camera's** with a tileset brush chosen
  (it painted when let go). A right drag that turns the view and ends
  where it began is still a right click to the brush (Raise/Lower
  lowers; other brushes are put down). (S)
  (from builders' reports after 1.14.1)
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
- **A creature's box in an area** is around it as it stands (the start
  of its pause), not around its model at rest: a builder's dragon had a
  box several times its size (at rest it lies stretched out, wings
  spread: 3.1 by 14.0 by 10.2 m against 5.6 by 7.3 by 6.3 standing).
  Skinned meshes (its wings) are counted where their bones put each
  vertex, as the renderer draws them; the joints are no longer counted;
  wings, tails and robes that are parts of their own are counted in
  their pose too.
  The box does not follow the animation as it plays, and a click is
  still tried against the model's triangles at rest (then against the
  box). (S) (from builders' reports after 1.19.1)
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

## The turning ring

- **The ring over another object:** a click on the ring picks the
  object under it, but a drag from there turns the selection: an object
  under the ring's line can't be dragged from there while the selection
  lasts. (S)
- **Led by the screen:** a tilt ring seen nearly edge-on and an arrow
  pointing nearly at the camera follow the pointer's travel on screen
  (from where they were taken), not the point under it; the change of
  one way of leading for the other, as the view turns, is at a fixed
  angle and can be felt. (S)
- **A spawn point's place:** its arrow's tip turns it; the point itself
  isn't dragged in the area view. (S)
- **The arrows' axes:** east, north and up, the area's; arrows along the
  object's own facing are not offered. The tilt rings, in the same red
  and green, are about the model's own axes. (S)
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
- **Emitters start as they look once going:** an object's emitter new to
  the view is run twenty seconds ahead (a wide fog that lets out eight
  slow particles a second showed next to nothing for a quarter of a
  minute after every change to the area). The nearest 256 objects with
  emitters are simulated (it was the first 96 listed). Tiles' emitters
  start from nothing still. That the fog is as dense as the game's from
  above wasn't compared with the client. Particles show only while
  Animations is on, and of at most 128 tiles. A tile's emitters take
  their keys from the animations playing on it as one animation as long
  as the longest (keys that change over a shorter loop would run slow;
  the game's are constant). Magic Sparks show faintly from above; not
  compared with the client. (S)
  (from builders' reports after 1.14.1)
- **A tileset's custom shader:** Moonglow runs none (see Custom shader
  effects above, and the `shaders` branch's note for what running them
  would take). Two it knows by name are drawn by its own shader instead.
  The `mzlm_vs`/`mzlm_fs` pair that NWN Mapper's exports name: a
  lightmap (`texture7`, on the second texture coordinates) multiplied in
  where `LightmapON` is set. That one's source was not at hand (the tool
  doesn't ship it): what it does is taken from the materials the
  exporter writes and its own description, and no export that uses it
  was there to look at. And
  the `vertexalpha_vs`/`vertexalpha_fs` pair a builder's tilesets ship,
  read from its source. Of what that shader does, Moonglow does the
  vertex colors multiplied in (with the material's `fShadowReduction`
  and `fShadowBrightening`), the alpha from the vertex color or from
  `texture6` on the second texture coordinates (layers blended into each
  other), `texture6` as a lightmap under `fLightmapMode`, and the
  texture slid along the flow map `texture7` by `fSlideSpeed` (lava).
  Not done: its dithered edges (blended here), the framebuffer sampling
  and bloom its materials ask for, and its lighting where that differs
  from the stock shader's. Unknown: how fast the game's
  `worldtimerTimeOfDay` runs (the slide is by seconds here), and how any
  of it compares with the game, which wasn't run on it. Another shader
  of another name, or this one renamed, is not known. The builder's
  package had no `flowmap_s` texture: the slide was tried with a
  stand-in. (M) (from builders' reports after 1.14.1)
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
- **An upright `Aligned_to_World_Z` particle** is turned about Z to the
  eye here; whether the game turns it or keeps a fixed side was not
  measured (a still quad looked the same from the one view taken). (S)
  (from builders' reports after 1.19.1)
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
- **Water** was flat and still. The game's water textures (every base
  tileset's, Medieval Rural 2's too) say two things in their TXI:
  `proceduretype arturo`, the old game's rippling of the picture, and
  `bumpmaptexture shinywater`, which Enhanced Edition draws with its
  water shader (`fswater`: waves driven by the area's wind, reflections).
  Moonglow now does both in its own way: the picture is pushed about
  with the TXI's amplitude and speed, and on water small waves, running
  on from tile to tile, tip the surface so its reflections move. Neither
  is the game's procedure or its shader (no wind, no local sources, no
  displacement), and neither was compared with the client. (M)
  (GitHub issue 6)
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
- **Shadows:** the area view and the previews draw none. The client
  draws them for placeables with a shadow caster, static or not (an
  armoire, and a library cart with a `render 0`, `shadow 1` node), on the
  ground and on static placeables; an area's shadow settings are only
  edited. (L: the game's stencil shadows, or a shadow map standing in
  for them)
  (asked by the Domains of Dread module session; checked in the client,
  2026-10-09)
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

## Models and textures

- **Heed's previews after 1.33.0** (not reproduced): armor that shows only
  a head and pelvis (sometimes shoulders) in the preview, and the Drow
  shields of the standard palette shown as bags in their thumbnails and
  the palette's hover preview. In the base game they show whole
  (`item_previews` in `screens.rs`: `nw_aarcl004`, `x2_adrowshl001`), so
  his haks are the likely cause (a `baseitems.2da` or armor parts of
  their own); his hak list or a blueprint is needed. (S–M)
- **A compiled model is known by its first byte** being zero, not its
  first four (troubletcat's report after 1.32.2: a fancy-mapped CTP
  Babylon hak's 587 tile models begin `\0CTP` or `\0OMB`, drawn in the
  game and Aurora, by the builder's word; Moonglow read them as text,
  found them empty and drew the tiles as missing). All 1,408 of that hak
  read now. nwnmdlcomp refuses them ("isn't a binary model file"); the
  game's own reading was not tried in the client. (S)
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
- **A skin's bones that are no nodes:** a skinned mesh whose bone map
  names nodes past the model's last (custom content; it crashed 1.4.0 on
  loading a module) is drawn with those bones unmoved. What the game
  does with such a model wasn't looked at, and the model itself wasn't
  seen: the fix is from the crash report. (S, needs the model)
  (from the research notes)
- **A model that fails while it is drawn:** a model that breaks what
  builds or poses it is left out and the module stays open
  (`mg_render::guard`: tiles, objects, the model viewer, thumbnails). A
  failure inside the renderer's own drawing (uploading its textures, the
  GPU's calls) is not caught and still ends Moonglow, with a crash
  report. (M) (from the research notes)
- **Left out for the session:** a model that failed stays out until its
  area is opened again; the object shows no box in its place beyond what
  a missing model has. (S) (from the research notes)
- **Index lists against the client:** that the game draws a compiled
  mesh's index list where it has more triangles than the face list
  (`Mesh::drawn`: most Forest - Facelift cliffs) is settled from the
  files (the list reaches every vertex, the faces are its first
  triangles, drawing it closes the holes), not by a client screenshot
  (`client_render.rs`). (S) (from the research notes)
- **KTX textures:** `mg-image` doesn't read KTX yet (`PLAN.md` §4). (M)
  (from the research notes)

## The main window and its panes

- **A module's open tabs are not remembered:** opened again, a module
  has none of the areas, scripts and blueprints that were open when it
  was closed. Asked for: kept by module (in Moonglow's data folder, not
  in the module), an option. (M) (a builder's report, October 2026)
- **Deleting from the module tree:** Delete… removes the resource (an
  area with its GIT and GIC and its area-list entry, a script with its
  NCS) and nothing else: transitions, scripts and conversations that name
  it are left as they are, with no warning of them; one resource at a
  time (the Delete key asks the same of the row under the pointer). Moonglow refuses to delete the
  start area; Aurora removes it (after asking twice) and then refuses
  to save: "A module must have a starting location to be valid." (S)
- **Edit on a script that doesn't exist** makes it and says so in the
  log; Aurora shows a message box first ("Resource not found. Creating
  new script."). A mistyped name makes a script (Undo takes it back). (S)
- **The debug log** (Options › General) covers actions, tabs and panes,
  an area's view as it loads, the palettes, jobs, the log pane and the
  graphics libraries' warnings. Not in it: the editors' own steps
  (blueprints, conversations, scripts), saving, plugins, the model
  viewer. Past 4 MB it is set aside and begun
  again, so a session takes 8 MB at most and the five kept 40 MB. Each line has the time of
  day (UTC); a file for each start, named by when, in `logs`, the last
  five kept (a builder's log was gone after a crash and a restart: the
  new start emptied the one file); a crash is noted in it. It is on
  unless switched off, by decision (a builder who had not turned it on
  had none to send): the option is a new one, so those who never asked
  for a log get one too. (S)
- **Panes' sizes at start:** the module tree is held to two fifths of
  the window and the log to half its height, whatever egui remembered,
  and names too long for the tree are cut short (a builder's module
  opened with the tree over the whole window: a long name widened it,
  and it could not be dragged back). Docked panes' splits are held
  between 12% and 88% (the palettes'), and the lower halves of the
  script, conversation and journal editors to a share of their editor.
  A tree name cut short shows whole only where the row already had a
  tooltip; the limits on the editors' lower halves aren't under test.
  (S)
- **Windows' sizes and Maximize:** a window's size is remembered by the
  kind of editor, not its place on screen; Maximize is a double click on
  the tab or its menu, with no button in the window's bar (egui_dock
  offers no place for one) and no key; a maximized window isn't kept so
  when the main window is resized, and nothing shows that it is
  maximized. That a size dragged by hand is remembered is not under
  test (the tests set the size). (S–M)
- **An area's objects in the module tree:** listed by name, kind by
  kind, with Go To, Properties, Copy and Delete on each; one at a time
  (no selecting several there), and the Filter finds them only in areas
  already opened out. A copy made there stands on the ground where it is
  pasted (its height above the ground isn't carried). (S)
- **The light theme's own colors:** it is egui's light theme; the script
  editor's syntax colors follow it, but conversation and faction colors
  are the dark theme's (legible on both), and the area view's overlays
  are drawn for the 3D view. A pass with a builder who uses it would
  settle the rest. (S–M) (a senior builder's review, October 2026)
- **The module tree goes to the area of the tab chosen** (a click on the
  tab), marking its row; Show in Module Tree on the tab's menu opens it
  out too. A tab reached otherwise (opened, or by keys) doesn't move the
  tree, and other kinds of tab (a script, a blueprint) don't. (S)
  (from builders' reports after 1.16.1)
- **Panes folded away** (GitHub issue 10): a View menu (Module Tree,
  Palettes Panel, Log, ticked while shown; Hide All Panels; Reset
  Layout), with keys (Ctrl+Alt+1, 2, 3 and 0, in Options › Keyboard); a
  pane folded away leaves a strip at its edge that brings it back, as
  wide as it was; the palettes' pane closed by its ✕ counts as folded;
  the state is kept in the settings. A pane's own button to fold
  it away: the module tree («), the palettes (», after their type
  icons: they had only their ✕, the reporter's word on 1.21.1) and the
  log (⏷, on the right, where the strip's ⏶ Log brings it back: they
  were a – on the right and an arrow on the left). Not done: a folded
  pane's strip shows an arrow, and its name only for the log and in
  the tip. (S)
  (from builders' reports after 1.19.1)
- **Tool windows where they were last docked** (GitHub issue 14): each
  kind of window (script, conversation, blueprint, Faction Editor…)
  opens where one of its kind was last dragged to: a tab of the main
  pane, a tab of the palettes' pane, or a window of its own (as all do
  at first); kept in the settings, forgotten by Reset Layout. A window
  seen in another place than the frame before is taken as dragged
  there. A pane split off a side of the middle is a place kept too, its
  side and its share (GitHub issue 22), and so is the palettes' pane's
  side where it is not the right. Not done of the issue: a tab's
  position among the tabs; a split off another pane than the middle (a
  side of the palettes') is kept as the middle's; the dock's whole
  layout is not kept between runs; the resource browser still opens as
  a window at first. The reporter
  closed the issue as fixed in 1.20.7 (2026-10-10), every point of it
  ticked: what is left here is by our own account, not asked for. (M)
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
- **Tabs by the keyboard** (GitHub issue 15): Close Tab (Ctrl+W), Next
  and Previous Tab (Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+Page Down and Up),
  Reopen Closed Tab (Ctrl+Shift+T), all in Options › Keyboard and the
  command palette; Ctrl and a digit for the main pane's tab of that
  number (fixed keys, off while text is typed in: the script editor's
  bookmarks are Ctrl and a digit); a middle click closes a tab; Close
  Others, Close Tabs to the Right and Close All on a tab's menu. Ctrl+F4
  closes the tab too, as the issue asks, and no longer the module as in
  Aurora (the user's choice): File › Close has no key at first. Ctrl+Tab goes by when the tabs were last used where Options › General
  says so. Not done: that order with Ctrl held over several presses is
  not held by a test (one press and back is), and it is kept for the
  pane the keys work on, not for a window's own tabs (which go as they
  lie); one question
  listing every tab with unsaved work (only a hak's and a tileset's
  editors have work of their own to lose, and each asks for itself);
  the keys work on the focused pane or the main one, not across
  windows. (S each) (from builders' reports after 1.19.1)
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
  23 after 1.30.2)
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
  - A letter typed goes to the tree only with the pointer over it
    (letters are commands elsewhere), to a group's name or a
    resource's name as its row shows it: not to a kind or a placed
    object. (S)
  - F2 and Delete are for resources, not for a placed object the keys
    are at. (S)
  - Down from the tree's Filter goes to the first group, not the first
    match. (S)
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

## Languages, tokens and loading screens

- **The game's text in the language edited:** the install is read as
  that language when its `lang/<code>/data/dialog.tlk` is there (its
  talk table, and the rest of its `lang` folder); a module's custom
  talk table is one file whatever the language. Whether the feminine
  table (`dialogf.tlk`) is shown where the game would use it was not
  looked at. Checked on Polish only. (S)
- **Tokens by language:** a language's own tokens are read from the
  install's `lang/<code>/data/ovr/stringtokens.2da` (Polish has one);
  a hak's or module's `stringtokens.2da` isn't per language. Token…
  puts the token where the caret was last in the text (in place of a
  selection), or at the end if it was never there. (S)
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

- **Not every key:** text editing, Escape and Enter, Copy, Cut and Paste,
  Delete in the area view, the script editor's numbered bookmarks, and
  mouse bindings (drags, Ctrl+click) are fixed. (S)
- **Aurora's keys Moonglow lacks:** Plot Wizard (Ctrl+Alt+P; no plot
  wizard), Refresh (F5: it is the script editor's Bookmark here, and a
  key of the whole window would take it from there; Reload Resources
  can be given a key in Options), and the script editor's Save As
  (Ctrl+Alt+S, now New Script's). (S)
- **Sharing key sets:** Options › Keyboard › Export… and Import… write
  and take the keys chosen as a file; the dialogs themselves weren't
  driven.

## NUI Creator (the review of its pull request, October 2026)

The NUI Creator came as a contributor's pull request and was brought in
line with the codebase before it was merged: its frames (26 to 35 ms after
an edit, now 1.4 to 2.4), one Undo step a field, the toolset's labels,
headings, name window and OK and Cancel, typed errors and lossless
settings in `mg-nui`, Build through the module's compile path, its tests
where the crates keep theirs, and a client test of what its checks say of
the game. This is what that review left, and after it what the
contribution itself states as not done or not checked.

- **Left by the read before the release (2026-10-14):** found and not
  changed, each with the steps that show it.
  - *A control's ID or a bind renamed* is changed in the event script
    where the code names it (an argument of a NUI function, a
    comparison, a JSON string): a name passed through a function of the
    builder's own (`SetMine("ok")`) is not followed. (S)
  - *The event script with unsaved text in its editor* is written into
    that text by the Creator, outside Undo and Cancel. (S)
  - *Closing a module leaves the Creator's pages and drafts* for a
    window of the same name in the next (`nui_views`,
    `nui_build_error` are not cleared). (S)
  - *Deleting a window closes its scripts' tabs unasked*, unsaved text
    with them (as deleting a script does). (S)
  - *Settings fields Moonglow does not know* are kept, in name order
    after the known ones, not where they were. (S)
  - *`mg nui generate` succeeds* where a script of the module's that
    includes the opener does not compile (it is printed), as `mg
    compile` does. (S)
- **A hak open in the hak editor takes no pictures**
  (`crates/mg-ui/src/nui_view/images.rs`): Images writes a listed hak's
  file in place, as the hak editor's Save does (no question; the log names
  the hak and the `.bak` kept), and refuses one that is open in the hak
  editor rather than adding the pictures to that document. (S)
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
- **Names from before the handlers** (`crates/mg-nui/src/actions.rs`,
  `lib.rs`): the checks and the editor say "handler" and "variant", and
  the names in the code stayed: a handler is a `Route`, the handlers
  `Settings::actions`, a variant shown `Action::View` and the variants
  `Settings::views`; the settings file's own field names (`actions`,
  `views`, `view_targets`) stay whatever the code calls them, for the
  files there are. (S)
- **A structured edit prints the whole file again**
  (`crates/mg-nui/src/lib.rs`, `crates/mg-ui/src/nui_view.rs`): fields
  nobody knows are kept at every level of the settings and the JUI, in
  their order, but the file is written anew (its spacing is the
  writer's), and a JUI with a key twice in one object is not read at
  all: it is edited as text. (M)
- **Functions still long** (`crates/mg-ui/src/nui_view/preview/`,
  `nui_view.rs`, `nui_view/shortcuts.rs`): of the preview, split by kind
  of control, `Renderer::node` is 180 lines and `list` and `text_block`
  140 each, and `draw::paint` takes nine arguments; in the editor
  `property` is 160 lines, and cut, copy and paste are the numbers 0 to 2
  (`shortcuts::clipboard`'s `op`). (S)
- **The canvas's tests are one file of 1,560 lines**
  (`crates/mg-ui/src/nui_view/preview/tests/canvas.rs`): they hold the
  canvas's own state and stay unit tests; by subject (scrolling,
  Interact, the window's handles) they would be three files. The rename's
  three tests (`nui_view/rename_tests.rs`) drive the application and
  belong in `crates/mg-ui/tests/nui.rs`. The pictures
  (`tests/screens.rs`) no longer check that the game's skin and fonts
  were found: a picture drawn without them is seen, not failed. (S)
- **Build with an external compiler is tested only where it fails**
  (`crates/mg-corpus-tests/tests/nui.rs`): the test gives Build a
  compiler that cannot be run; none builds a window with a real one.
  `mg nui generate` has no `--ndb` as `mg compile` has, and like it
  never takes an external compiler. What Build adds to the kept compile
  memory (`Moonglow::remembered_scripts`) has no test. (S)
- **The checks read a layout three times**
  (`crates/mg-nui/src/validate.rs`): once for its shapes, once for its
  names, and once for the draw list painted last, which follows
  `children` of any element where the shapes follow only those of rows,
  columns and groups; folded into one, an unknown widget with children
  would be judged differently. (S)
- **What the checks say of the client is tested by hand, and not all of
  it** (`crates/mg-nui/src/validate.rs`,
  `crates/mg-corpus-tests/tests/client_nui.rs`): the client test (run by
  hand, as the renderer's; 32 windows, 2 minutes) holds the errors that
  cite the client to it (NWN EE 8193.37): the layouts it refuses, Clip to
  control on the last draw list painted, the labels of Options from a
  bind. No test covers the warnings about what a control does when used
  (a Toggle button with a fixed value, a disabled slider, a text field's
  `max` below 4): they need a mouse and a keyboard in the client. The
  probes the checks named as open were run, and the checks still warn
  where the client was found to accept: a window whose root is a label,
  a button or a spacer is shown, a window is found by an ID with
  punctuation, spaces or non-ASCII letters, an arc is drawn the same
  with its center a point or a rectangle, and alignment, render and
  paint order constants out of range are shown. Those warnings could go
  or say so. (S)
- **Small things of the editor's fields** (`crates/mg-ui/src/nui_view/`):
  Split source shows what is typed on the canvas only once its field is
  left; the option and picture name fields lost their hint text
  (`widgets::commit_text_in` has none); a color being picked has no
  test; a press and release within one frame on a check's "Select it"
  link, drawn before the form, would select another control first and
  lose what was being typed. Left and Right in the layer tree are
  outside the keymap, so Options › Keyboard cannot change them. (S)
- **Renaming a window compiles nothing** (`mg_nui::rename`,
  `crates/mg-nui/src/rename.rs`; `Moonglow::rename_nui`,
  `crates/mg-ui/src/references.rs`): the compiled scripts of the old
  name, and of the scripts that open the window, are removed, and the log
  asks for a Build; the other renames compile what they changed. A
  window's ID in the game follows its name unless the settings give
  another, and a script that finds the window by it
  (`NuiFindWindow(oPlayer, "old")`) is only reported, unless the Rename
  window's strings option is on. Rename… on `NAME_e.nss` or `NAME_o.nss`
  alone is a script's rename, which knows nothing of the window, and
  `mg rename` does not rename a window. (S)
- **Where the preview is known to differ from the game** (stated in
  [the manual](manual/18-nui-creator.md);
  `crates/mg-ui/src/nui_view/preview.rs`): after a control with a width
  of its own, rows of buttons without widths make the game pick a
  narrower width than the preview's (308 against 462 in the case
  measured); at a UI scale that isn't whole, the preview's rows drift
  from the game's by up to a pixel each (the game rounds each scaled
  size down); an encouraged control shows its hover look and does not
  pulse; numbers in text are shown to 16 digits. (S each)
- **Interact does not run the window** (stated in the manual): handlers
  are NWScript and run only in the game, so event timing, and the
  game's own layout solver, are seen only in Test Module. (L, if at
  all)
- **Resizing leaves bound and template sizes** (stated in the manual;
  `crates/mg-ui/src/nui_view/design.rs`): a resize gesture does not
  replace a dimension that is a bind, and a list template's cell gets
  its width in the cell's settings or the JUI source, not by a handle.
  (S)
- **Export leaves pictures behind** (stated in the manual;
  `crates/mg-ui/src/nui_view.rs`): Export… takes the pictures the
  module holds; those a window uses from a hak or the game are not
  copied, and nothing says which a client would miss. (S)
- **Nothing short of the game proves a window** (`apps/mg/src/nui.rs`,
  `crates/mg-nui/src/validate.rs`): `mg nui validate` and `generate`,
  like the checks and Build in the editor, check the document and
  compile its scripts; that the game builds the window and draws it as
  previewed is seen only in the client (Test Module). (L, if at all)

## Automation

- **Plugins, beyond API 0.3** (`plugin-proposal.md`; commands and checks
  in sandboxed Luau are built; 0.2 added a file or folder the user
  chooses, haks written with their leave, areas and terrain, pictures;
  0.3 files written into a chosen folder, 2DAs and talk tables as
  values, and events, whose own gaps are below, under "Plugins writing
  into a folder" and "Tables for plugins"):
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
  - **A project's plugins:** `plugins/` beside a nasher project's
    `nasher.cfg`, with a prompt before they run, so that a team shares
    its checks through git. `mg verify --plugins DIR` covers a build
    pipeline meanwhile. (M)
  - **More kinds of things to add:** exporters and importers, wizards,
    quick fixes for findings, settings a plugin keeps, panels.
    A plugin's command can't yet read what is selected or open in the
    window. (L)
  - **A plugin's own tests** (`mg plugin test`, against a fixture
    module): `mg plugin check` and `mg plugin run --dry-run` are what
    there is. (S–M)
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
- **`mg find` beyond objects:** it searches blueprints and placed objects;
  conversation lines, scripts' text and 2DA rows have `mg replace
  --dry-run`, the script editor's Find in Files and the resource browser.
  (S–M)
- **A JSON schema** for each command's output: the fields are described
  in the manual, not in a machine-readable schema. (S)
- **Plugins writing into a folder, and acting on events** (plugin API
  0.3): `folder:write` into a folder the user chose, with their leave,
  and `mg.on` for a module opened or saved. Not done: a file goes into
  the chosen folder itself, not into a folder inside it (a project's
  `src/` tree of folders is chosen one folder at a time); no copy of a
  replaced file is kept; there is one event for a save, with the
  resources changed since the last, not one for each resource, and none
  for a script's own Save; what an event was allowed is kept between
  runs by plugin and version (the settings' `plugin_grants`), but a
  plugin still keeps no settings of its own; `mg plugin run --event`
  runs an event by hand, and `mg watch` and `mg build` fire none; an
  event's handler that asks a form asks it at every save. (S each;
  settings M)
- **Tables for plugins** (`mg.twoda`, `mg.tlk`, plugin API 0.3): a 2DA
  and a talk table as values to change and write; `mg.twoda.kinds`
  says what Verify Module's tables say (ten columns that hold a
  talk-table string, four that name a model, sixty-one that name a row
  of another table, and the feat, skill and spell lists a class, a race
  or a package names). Not done: a class's other tables (`cls_pres_*`,
  whose row depends on the requirement's kind; `cls_bfeat_*`,
  `cls_spgn_*`), the `iprp` tables, columns the game's own tables
  break (`appearance.FOOTSTEPTYPE`); that the table a column names is
  there at all is not checked; `mg.twoda.kinds` knows a named list by
  its name only (`cls_feat_*`), not by the column that names it; a row that is
  there but blank (a padding row) counts as there; a written 2DA is laid out as the game's are, not with the
  widths of the file it was read from; a talk table's language is
  English and its feminine table is another file. (S each)
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
- **Portraits, music and ambient sound** in folders (not haks) aren't
  gathered; they go in a hak first, as with `nwn_nwsync_write`. (S)

## Tileset authoring

- **Rules and doors:** primary and secondary painting rules and a tile's
  door hooks (`[TILEnDOORm]`) aren't on the editor's pages; they stay as
  the file has them (edit the text for now). (M)
- **Renaming a terrain or crosser** isn't offered: every tile naming it
  would change with it. (S)
- **Minimap pictures** for a tileset without them are made by the
  tileset editor's Render Minimap Pictures (an area's Export Minimap
  writes the pictures the tileset has): they have Moonglow's renderer's
  look (lit models, as in
  the area view), not the game's painted look; and models must be
  readable (beside the `.set` or in the game data). (S–M)
- **Making models** (Neverblender's job) and **testing a tileset** by
  painting an area with it straight from its folder (it has to be in a hak
  the module uses, or in development). (M)
- **The palette's arrangement:** it's generated flat, without the
  subfolders hand-made palettes have. (S)

## The install, starting and crashes

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
- **The Aurora UI inventory's hand-made parts:** its inputs are made
  from the installed game now (`tools/aurora/uiinv/inputs.py`), but
  twelve StrRef table offsets and the mappings in `mapping.py` were
  settled by eye for one build of the toolset (26c6e573, 89.8193.37):
  another build needs them looked at again (the generator warns). (S,
  when the toolset changes) (the documentation audit of 2026-10-12)
- **The plan's "Held by" lists were chosen by reading:** the record of
  the phases names files and tests for each phase from their names and
  headers, not from a run. (S) (the documentation audit of 2026-10-12)
- **The wiki corrections were never submitted** (`wiki-proposal.md`);
  all fifteen pages are unchanged upstream. (S)
  (the documentation audit of 2026-10-12)
- **Not audited:** nothing was run, so no measurement in any document
  was re-made; parts of four research notes were not read (the audit
  lists them). (the documentation audit of 2026-10-12)
- **The manual, what the audit named and is not put right:** GOG's
  folder is not looked for on macOS (chapter 1 does not say so);
  chapter 14 says "There's no OK or Cancel", which is true of property
  editors only, and repeats chapter 12 in "Habits to change"; chapter
  15's `--event` example names a plugin of no one's (the examples in
  `docs/plugins` declare no event). (S each)
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

- **Area Properties copies the area's fields each frame it is open**
  (`area_props.rs`: its form owns the root it edits, as every
  blueprint's does), without the tile list since: a large area's
  thousand tiles were most of the copy. Not timed (`frame_perf.rs` was
  not run for it). (S)
- **Still made every frame, each needing something kept:** a blueprint's
  category list (`Form::category` reads and parses the palette),
  `picker::blueprint_names` and `inventory::entry_icon` (a blueprint
  read per row), Module Properties' list of the hak folder, the script
  editor's sorted symbols and its templates folder, Find in Files'
  results (copied twice), the thumbnails' lists for the Appearance
  Gallery and the Creature Wizard, the module tree's rows for the
  keyboard (each group's resources copied). (S each)
- **A dropdown's choices are kept but copied to who asks:**
  `GameData::choice_list` hands the kept list out shared; the editors
  still call `choices`, which copies it. (S)
- **Written more than once, in the editors:** the store's inventory
  page adds an item its own way (to the page of its kind, at a place in
  its grid), where the placeable's and the creature's share
  `Form::apply_list`; the tools' radius by distance was not found as
  the review saw it (two sites in `area_view.rs` and one in
  `model_view.rs` scale by the camera's distance, each its own way).
  (S each)
- **Written more than once, in the tests:** some corpus tests still
  stack the game, a module's haks and the module by hand, where
  `Module::stack_onto` does it for mg and the window. (S)
- **Looked at as written more than once, and left:** three recursive
  GFF walks that `Struct::walk_fields` does not fit as it is (the text
  search's, which keeps its path as steps; the doctor's objects, which
  goes by the struct and the list it is in; the references' rewriting
  one, which changes what it walks); the GIT list table in four shapes
  (each maps a list to another thing: a category, a struct id, a
  resource type, a word); the conversation's owning links walked three
  times (three questions: who owns a line, which lines are reached, the
  tree printed); the talk table's and the GFF reader's own table
  checks, which `Reader::table` (the key and archive readers') does not
  fit as written. (S each, if at all)
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

## Release and packaging

- **macOS:** the app and disk image are built by CI but have never been
  run on a Mac. (S, needs a Mac)
- **Windows signing:** the installer isn't code-signed, so SmartScreen
  warns. (S, needs a certificate)

## Left from the builders' reports on 1.25.1

- **Edit › Undo and the toolbar's Undo in a script:** they are the
  module's still; only the keys are the script editor's while it has
  the keyboard. A script's own history as steps of the module's would
  make them one. (M) (a builder's report, October 2026)
- **Export Resources lists an area as three kinds** (`.are` under
  Areas, its `.git` and `.gic` among the rest by extension), as it did
  in one list; exporting the `.are` takes the other two only with
  **Include the resources they use**. (S)

- **OK and Cancel where there is none yet:** the hak editor, the
  tileset editor, the talk table, the raw field editor and a script
  (each saved, or asked about, in its own way). In the windows that
  have them: a text field's change set after its page was left by a key
  (not by the pointer) is not among what Cancel sets back, and a
  faction's removal is set back by the whole of each area and creature
  it renumbered, as an area's sounds and music (one struct of its GIT)
  are set back together. (S) (a builder's ask, October 2026)
- **The model viewer's and a Properties page's view of a shield:** from
  the back still (the galleries' pictures are from the front). (S)

## Questions asked, unanswered

Each waits on its reporter: with no word by 2026-11-12 (a month from
the owner's ruling of 2026-10-12) it is dropped, and said so in the
log.

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
  Aurora and not like it, and kept by decision (2026-10-07). An include
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
  is off). A nasher project is left to Pack Target. Aurora's own
  behavior was not captured.
- **Clicking between tiles and objects:** a click on an object while
  tiles are selected goes to objects, and a double click on the ground
  to its tile. So a tile wholly under an object is selected by a box,
  not a click.
- **Behind things:** the rings and arrows are painted over the view, not
  hidden by what stands in front of them (as 3D editors draw theirs:
  left so).
- **A palette's rows are one line** (a builder: names with ResRefs and
  challenge ratings wrapped onto a second line in a narrow pane), cut
  short as the module tree's are, whole under the pointer. Not an
  option.
- **The module tree and the object selected in the area:** the tree
  goes to it only where its area is opened out there; an area left
  closed stays closed. Asked for by the user (it was opened out and
  scrolled to on every selection, written to follow Aurora, whose own
  tree was not captured doing so). With several objects selected, or
  none, it stays where it is; there is no way to turn it off.
  (from builders' reports after 1.19.1)
