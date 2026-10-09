---
type: Backlog
title: 'Deferred: what was left out, for a decision later'
description: What the post-parity work left undone, by area (script intelligence, nasher projects, content doctor, test loop, scale, frames, EE fields, bulk editing, conversations, palettes, custom content, haks, options), each awaiting a verdict - fix, add or drop.
tags: [backlog, deferred, after-parity]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-09T00:21:01Z }
---

# Deferred: what was left out, for a decision later

What the post-parity work (`PLAN.md`, "After parity") left undone, in
one place. Once the map's current items are worked through, each gets a
verdict: fix, add, or drop. New gaps are added as work finishes.

Size: S (an hour or two), M (a day or so), L (several days).

## Script intelligence (`mg lsp`, the script editor)

- **The longest constants in the side lists:** the lists open wide
  enough for names of up to 44 characters (every function, all but some
  160 constants, `PLAYER_DEVICE_PROPERTY_…` and the like, of up to 67);
  those end in "…" until the panel is widened. (S)

- **Struct members:** go to definition, references and rename don't
  follow `p.nX` to the struct's declaration. (M)
- **Incremental compiles:** Compile All recompiles everything. It could
  compile only what changed and the scripts that include it, as Arelith's
  ARE_Compile does. A full compile of 4,000 scripts takes 1.4 s, so this
  matters less than expected. (M)
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

## nasher projects

- **NWNT projects:** projects using NWNT instead of JSON for GFF files
  are refused with a message. (M)
- **Compiled scripts without source:** scripts that exist only as `.ncs`
  are lost when a module becomes a project, as with nasher; Moonglow
  names them. It could keep such `.ncs` files in the project instead. (S)

## Content doctor

- **The walkmesh crash rule:** nwn.wiki names "walkmesh outside the
  tile" as an Aurora crash cause, but the game's own tiles reach 10 m
  out and work. The real cause is unknown, so there's no check.
  Finding it would mean crashing Aurora on purpose with test tiles. (M)

## Test loop

- **Choose Character:** Test Module, Choose Character uses the game's
  `+LoadNewModule` (from nwn.wiki), never run in the game client. (S)
- **Programs Moonglow starts:** the game (Test Module) and the external
  script editor are waited for on a thread each, so they leave no
  "defunct" process; checked on Linux only. (S)
- **Aurora's F9 problems:** laggy combat, AI timing errors and damaged
  modules after a crash. Moonglow launches the game as the wiki recommends
  to avoid them; nothing has checked that they're gone. (S–M)

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
- **Where-used at scale:** one Find References takes 0.8 s in the
  persistent world: it scans the module each time. An index kept up to
  date would make it instant. (M)

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
- **Seams between tiles:** drawn as instances, a sample on the seam of
  two tiles may show the other tile's floor (12 of 192 pictures, one to
  three pixels each): which of two floors that both cover a sample is
  drawn last was the order of the area's tiles, and is now the order of
  the draws. A mesh at a time (`Renderer::instancing` off) it is as it
  was. (—)
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
  languages, a slider, an area's flags (one field for several boxes)
  and its Audio page, and the lists of an editor's own. (S)
- **Choosing areas by more:** the chooser filters by name, ResRef,
  tileset and the three kinds; not by a property's value (areas with a
  given music or variable), and its choice isn't kept as a named set.
  (S–M)
- **`mg areas` and lighting schemes:** the command sets fields, flags
  and variables; it doesn't apply a lighting scheme (environment.2da,
  which also picks each tile's lights, and needs the game's data), nor
  set names or list fields. (S)
- **Find and Replace beyond strings players read:** tags, resrefs and
  other plain text fields aren't searched (Find References covers tags,
  Find in Files covers scripts), and there are no regular expressions.
  (S–M)
- **Prefabs in the palette:** listed by name, placed with a click,
  renamed and deleted from their menu; what one holds is told in words on
  hover (kinds and tags), not shown as a picture; no folders. (S)
- **Variable sets** hold int, float and string variables only, as the
  Variables window edits; object and location variables aren't kept. (S)

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

- **Pictures for the rest:** waypoints have their flags as pictures;
  sounds, triggers, encounters and stores say what they are on hover, in
  a line or two (a store's and a sound's markers are the same for every
  one, and the others have no model). (—)
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
- **Standard palette categories** can't be changed (the game's); only
  custom blueprints move between categories, as in Aurora. (—)

## Custom content data (2DAs and talk tables)

- **No 2DA editor or merger:** 2DAs are read-only (the view shows which
  hak each row comes from). Editing rows and merging several haks' copies
  into one is Eos's ground, left for when it's asked for. (L)
- **Talk tables in a hak or the module** are read-only: Moonglow edits
  only tables in the `tlk` folder. A table kept in the module could be
  edited and saved with it. (S)
- **Talk tables as CSV only:** Export CSV and Import CSV; nwn_tlk's
  JSON isn't read or written, and `mg` has no command for either. (S)
- **One language:** the editor edits the table in its own language;
  translated tables (the same name in other languages' folders) aren't
  shown side by side. (M)
- **Where a StrRef is used:** Find References doesn't find the 2DA cells,
  blueprints and conversations that name a talk-table line. (M)
- **Inserting or removing lines in the middle** isn't offered: it
  renumbers every later line, and renumbering their users too would
  need where-used first. (M)
- **The client and custom talk tables:** the engine test runs the server;
  whether the client also reads a table from the module (not only from
  haks and the `tlk` folder) for 2DA text isn't tested. (S)

## Haks in the GUI

- **No zip, 7z or rar:** content downloaded as an archive is unpacked
  first; Add Haks and Talk Table takes the haks and talk table. (S, a new
  dependency)
- **Hak order advice:** Moonglow doesn't know which haks go above which
  (CEP, PRC and the like each document theirs); the conflict report shows
  what each hak hides. (M)
- **Viewing in the hak editor** shows fields, text or the first bytes:
  no pictures, models or sounds, and a 2DA as its text. (S)
- **Update from Folder** takes the folder's files in place of the hak's
  (two undo steps: the removal, then the adding); a hak that was built
  from several folders, or added to by hand, loses what the folder
  hasn't. (S)
- **ERF version:** haks are saved as `V1.0`; EE's compressed `E1.0` is
  read but not written. (S–M)

## Options and lists

- **Areas by name elsewhere:** areas are by name in the module tree, tab
  titles, Find Instance, the area transition's destinations and the
  references list; the log has ResRefs. (—)
- **The Options tab** opens as wide as the window allows, where the
  other editors leave some of the area showing beside them; Enter is OK
  only while the pointer is over it. (—)
- **Column headings** in forms' tables ("Base", "Total", "Sun", "Moon")
  keep the plain strong style; only section headings were made larger.
  (—)

## From a senior builder's review (October 2026)

Done from it: New… in the module tree's menus, the Creature Wizard's
races and monsters' portraits, Escape closing a model's window, lists by
name, Interface size, spawn points' markers and facings, areas and
blueprints by name with ResRefs in parentheses, the palette's ResRefs and
challenge ratings as options, category filters for feats and spells,
special abilities as uses, the creature's model in its Appearance page,
a light theme. Left:

- **Creature Properties' overlapping elements:** reported, not
  reproduced: every page is checked for widgets drawn over each other at
  four window sizes (`blueprint_editor_pages_draw_nothing_over_anything_else`)
  and none are. A screenshot, or the page and the interface size, would
  find it. (S)
- **Scripts and conversations by name:** they have no names; the tree
  lists them by ResRef. (—)
- **The model in other editors' pages:** a placeable's and a door's
  Basic page show it beside their fields where the window is wide enough
  for both; narrower, Preview shows it. An item's Appearance page has it
  beside the fields, or in a narrower window beside the icon (a
  creature's Appearance page stacks them instead). The viewer frames an
  item as any model: a sword lying flat is small in it until turned. (S)
- **Special abilities' flags against the game:** settled in the engine
  (`engine_special_abilities.rs`): a use with any of Ready, Spontaneous
  or Unlimited set is one the creature has, a use with none is spent,
  and Unlimited doesn't make the uses unlimited (each entry is used up).
  The editor had the format's three names as switches until 1.19.5; a
  builder who knows the engine pointed out that it reads the byte as yes
  or no, which is what the test had found, so there is the one switch,
  Ready, now. (—)
- **The light theme's own colors:** it is egui's light theme; the script
  editor's syntax colors follow it, but conversation and faction colors
  are the dark theme's (legible on both), and the area view's overlays
  are drawn for the 3D view. A pass with a builder who uses it would
  settle the rest. (S–M)


## From a second builder's review (October 2026)

Done from it: scroll bars always showing, Escape closing the window in
front (dialogs, then a Properties window), dialogs kept above the docked
windows, number fields dragged to a new value (the drag did nothing),
deleted and unnamed rows left out of the feat and spell lists, a row's
number, label and innate level in a feat's or spell's hover, portraits
without pictures left out of Select Portrait, doors' portraits out of
the Creature Wizard, the Variables window's list scrolling, a read-only
View of the game's blueprints, Edit Copy… and the wizards asking for the
ResRef and Tag. Left:

- **A drag on a number field moving objects in the area behind:**
  reported, not reproduced
  (`a_drag_in_a_window_over_the_area_moves_nothing_there`). (needs the
  steps)
- **Number fields' limits against Aurora:** measured in Aurora
  (v89.8193.37) by typing past them: a creature's save bonuses 250 (and,
  it seems, no lower than 0: Moonglow keeps -100, for the penalties files
  have), its ability scores 100, natural AC 1000 (Moonglow: 255, the
  byte the file keeps), base hit points 10000; a placeable's and a door's
  hardness and saves 250, hit points 10000; lock and trap DCs 250. These
  are Moonglow's now. The other fields the forms give "spin 0…100"
  (`aurora-ui-inventory.md`: encounters, stores, alignment and the rest)
  were not measured: the forms' limits are only what they start with.
  (S)
- **Which rows Aurora offers** as feats, spells and special abilities
  (by `UserType`, `ALLCLASSESCANUSE` or else) was not captured: Moonglow
  leaves out rows without a name and those labelled DELETED or Padding.
  (S)
- **View is read-only by its look and by what it refuses:** fields are
  dimmed and a change tried is dropped with a note, but a field still
  takes the pointer and the keys until then (so its lists scroll and its
  pages open): what is typed goes when the field is left. (S)
- **Random item properties** in the Item Wizard, as Aurora's. (M)
- **"Area transitions can't be selected visually":** not understood;
  asked. (—)
- **Done since:** the Creature Wizard's ResRef and Tag, its appearances
  as pictures with Find, and every portrait; the inventory's Equip to a
  free slot, To Backpack and Open Blueprint; talk-table text in its own
  color; a Variables window that is sized; drag and drop and right-click
  menus in the inventory, items dragged between inventories and within
  a list, Copy and Paste. Left of these: an item carried between
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

## From builders' reports after 1.10.1 (October 2026)

Done: an area's missing music looked for once (it was every frame), Edit
beside a script opening the external editor where Options has scripts
open there, a conversation's tree in the language edited, a window
maximized from the bar beside its tab, a folded window showing its whole
bar, the Faction Editor's columns as wide as their numbers, the journal's
categories folding, prefabs deleted from the palette, Copy… in the
module tree (areas with what is placed in them), particles of placed
objects in the area view, Fade Geometry. Left:

- **Moonglow seeming to start twice on Windows** (a builder's video: a
  blank white window, gone again, then the real one): his window was
  last left maximized. eframe creates its window hidden until the first
  frame is drawn, and winit on Windows shows a window created hidden
  and maximized all the same. A window left maximized is now created
  unmaximized there and maximized after its first frame. Not tried on
  Windows: nothing here runs it; told by the video, eframe's source and
  the fix's own test of what it asks for. (?)
- **A body part drawn in another part's texture (GitHub issue 5, its
  second half):** a part was drawn with the texture its mesh names where
  that exists, but the game draws the part's own (`p<g><r><pheno>_<part>NNN`)
  whatever the mesh names. The game's own thigh 3 names thigh 2's
  texture and chest 32 chest 13's (598 such meshes in the base game
  alone), so this was wrong without any custom content: the reporter's
  "shows part 2, should be part 3". Seen in the client (`creatures_look`
  with `MG_PARTS`: plate thighs, where Moonglow drew thigh 2's cloth),
  and the same now. A mesh that names something that is no part's keeps
  it. The reporter's own armor (a CEP robe) was not drawn here: its
  part models are in CEP haks not on this machine. (—)
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
  known. (needs the file)
- **A cloak over the face of an elf (or any body smaller than a
  human's):** a worn part playing a skeleton's animation moved at its
  own animation scale (a cloak's model has none: 1) rather than its
  wearer's. It moves at the wearer's now, which puts its bones on the
  body's (`a_cloak_sits_on_a_smaller_body_s_shoulders`). Seen first with
  the HD bodies, and the same with the game's own. Compared with the
  client since (`creatures_look`): a cloak on an elf and on a human sits
  on the shoulders in both, in the item's colors, with the wings the
  cloak's row hides hidden. (—)
- **"The toolset must be restarted for the language to take effect":**
  reported; Options › OK reads the game's text and the names again, and
  text fields are now read again too. What still showed the old
  language wasn't found. (needs the steps)
- **Fade Geometry:** Never, Object Mode Only and Always, as Aurora
  names them; Moonglow's object mode is "not Select Tiles and no terrain
  brush", which wasn't compared with when Aurora fades. That the meshes
  with `tilefade` 2 (the black caps) stay while those with 1 and 4 go
  was taken from the game's tiles and their names, not compared with
  Aurora or the game. (S)
- **Particles in the area:** of at most 96 objects and 128 tiles, and
  only while Animations is on. A tile's emitters take their keys from
  the animations playing on it as one animation as long as the longest
  (keys that change over a shorter loop would run slow; the game's are
  constant). Magic Sparks show faintly from above; not compared with
  the client. (S)
- **Alt for the camera and the selection:** Alt + right drag turns the
  selection and Alt + middle drag moves the camera (they were Shift's,
  which now leaves the camera alone while the handles show). Aurora
  turns the selection with Shift + right drag; and some desktops take
  Alt + drag to move the window. Not in Options › Keyboard. (S)
- **A window's bar:** egui_dock gives a folded window the height of its
  tab bar, frame included, so the dock's windows are drawn without a
  frame margin and their tabs' contents are given one instead. (—)
- **Copy… of a script** takes its compiled script along; its debug
  information (`.ndb`) is made when the copy is next compiled. (—)
- **The scale handle** scales every way alike, as Adjust Location's
  field does; the file keeps a scale for each axis. It stands where it
  was taken while it is dragged, rather than follow the pointer. (S)
- **The Gallery:** up to 192 pictures are kept at once (more are made
  again as they come back into sight) and all are made again when a
  blueprint changes. (S)
- **Ctrl+V refused on Windows** (a builder's debug log: "arboard paste
  error … the clipboard is empty"): the key reaches Moonglow only as the
  system's paste, which Windows doesn't send with no text to paste.
  Copying objects, tiles or a conversation's lines now leaves a line of
  text there. Not tried on Windows. (—)
- **A picture of a model that emits** is drawn four seconds in, framed
  with its particles (a flame is small under its smoke). (S)
- **Appearances as pictures** are placeables', creatures' and doors';
  an item's model, and each of a weapon's three parts, are chosen from
  its icons (the item as it would be with each). An armor's parts are
  still numbers: no pictures of them, on a body or alone. A creature's
  look is a plain body of the appearance, male unless the creature is
  female. (S–M)
- **Replace Selected with This** makes the object anew from the
  blueprint: what the old one had of its own (its tag, variables, a
  tilt or scale) goes; triggers and encounters aren't replaced. (S)
- **Armor and cloaks shown on a woman** (the item's Appearance page,
  Shown on): the model and the page's icons. The icons in inventories
  and the palette stay the man's, and the choice lasts until Moonglow is
  closed. (S)
- **A held item's own Properties** (a placed object's inventory, right
  click › Properties) are found by the item's place in the list: with
  the window open, an item added, removed or moved before it in that
  inventory leaves the window on another item (or none). A blueprint's
  items (which are named, not held whole) open the item's blueprint
  instead; a placed store's weren't tried. (S)
- **Deleting a prefab** isn't Edit › Undo's: the palette's Undo Delete
  puts back the one deleted last, until Moonglow is closed. (—)

## Raw fields

- **Rows the fields view doesn't name yet:** an item property's subtype,
  cost and parameter (their table depends on the property), a creature's
  or object's faction (the module's `repute.fac`, not a 2DA), a
  conversation's animations, a store's categories, and fields that hold
  a talk-table number as a plain int. (S–M)
- **Only in the window:** `mg gff`, `mg find --where` and `mg areas`
  print and take the numbers; names aren't shown or accepted there. (S)

## Getting files out

- **Compiling before export** needs the game's data (for nwscript):
  without it scripts go out with the compiled scripts they have, and the
  log says so. (—)
- **One at a time from the tree:** the tree exports one resource at a
  time (the Export window takes several). The area view's To Scratch
  copies the area alone, not the blueprints or scripts it uses. (S)
- **One scratch folder:** To Scratch copies to one folder, for every
  module; a folder per module, or a remote server's (over SSH), isn't
  offered. Export as Files takes any folder, and remembers the last. (S)

## From builders' reports after 1.14.1 (October 2026)

- **"It kept randomly exiting"** on a persistent world's module (ten
  times before it stayed open), reported without a log. One cause was
  found and fixed: the compiler walks a script by recursion, and a very
  long `else if` chain or expression overflowed the stack of the thread
  it ran on, which ends the program at once and leaves no crash report
  (a 60,000-link chain did it on Linux; Windows' main thread has an
  eighth of that stack). Compiles run on a stack of their own now.
  Whether that was the reporter's crash isn't known: asked for the debug
  log (Options › General) and what was being done. (needs the log)
- **Include files** aren't compiled on their own by Compile All, the
  build or Compile in the editor (they were, and one leaning on what its
  includer brings was reported as an error). Watched in Aurora since
  (Build › Build Module, Compile): it compiles every script, says
  nothing of one that only lacks `main`, and reports "Compile Error" for
  an include that leans on what its includer brings, as Moonglow did
  before this change. Skipping includes is Moonglow's own, kinder than
  Aurora and not like it, and kept by decision (2026-10-07). An include
  is still checked as you type, on its own, so one that leans on its
  includer shows an error there. (—)
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
- **A click is on the model, not its box:** an object with a model is
  picked where the pointer's ray meets its triangles (at rest: a creature
  mid-stride is picked where it stands still), so a click through the
  empty part of a wide box reaches the trigger or the object behind;
  where the ray meets nothing so, the nearest box passed through is
  taken, so a thin post is still clicked beside. Aurora's own picking
  wasn't measured. Things without a mesh (markers, an emitter alone) are
  picked by their boxes still. (S)
- **Emitters start as they look once going:** an object's emitter new to
  the view is run twenty seconds ahead (a wide fog that lets out eight
  slow particles a second showed next to nothing for a quarter of a
  minute after every change to the area). The nearest 256 objects with
  emitters are simulated (it was the first 96 listed). Tiles' emitters
  start from nothing still. That the fog is as dense as the game's from
  above wasn't compared with the client. (S)
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
- **A nasher project's file that isn't UTF-8 throughout** (a Polish
  module's conversation: "invalid unicode code point") is read with its
  stray bytes as the project's code page has them. The reporter's file
  wasn't seen; a file broken another way still stops the project from
  opening, where it could be left out and said. (S)
- **What an area lacks is named in the log** (a tileset, models from a
  hak that isn't there), once, up to twelve names; the toolbar's count
  has the rest on hover. (—)
- **Steam Workshop content** is read (Options › General): each item's
  `override` folder as a layer under the user's own, its `hak` and `tlk`
  folders searched. Which of two items' overrides wins in the game isn't
  known (the lower number here), and that the game puts them under the
  user's override is from how it is described, not measured: the client
  runs here with Steam hidden. An item's other folders (portraits,
  modules, music) aren't read. (S)
- **Test Module and Steam:** the game started for a test is told which
  Steam game it is (`SteamAppId`), so that a Steam copy reaches a running
  Steam and loads the Workshop's content, as when started from Steam;
  started bare it had none of it (a builder's "F9 won't load the stuff
  correctly either"). Not tried: the client is only ever run here with
  Steam hidden. With Steam running, the test then shows in Steam as the
  game being played. (needs a builder's word)
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
  stand-in. (M)
- **Finding the game:** Steam's libraries are read from its
  `libraryfolders.vdf` (a game on a second drive), and GOG's usual
  folders tried. Steam itself installed outside its usual folder is
  found on Windows only as `X:\Steam` or `X:\SteamLibrary` (the registry
  isn't read); Beamdog's client isn't looked for. The reporter's case (a
  second drive on Linux) is covered by the list, not tried on their
  machine. (S)
- **The grid seen from the area's edge:** red and plain as the view
  comes down (Aurora's is red from every side, and mostly lost in the
  ground from above), and drawn at the area's foot under raised ground.
  Looked at in Aurora on a raised plateau since, at a low pitch: its
  red lines show on top of the raised ground, at that ground's height,
  as Moonglow's do; none could be made out on the low ground in front
  (lost in the ground, as from above). Whether Aurora also draws lines
  at the area's foot under raised tiles could not be told. (—)
- **A press with Ctrl held is the camera's** with a tileset brush chosen
  (it painted when let go). A right drag that turns the view and ends
  where it began is still a right click to the brush (Raise/Lower
  lowers; other brushes are put down). (S)

## From builders' reports after 1.16.1 (October 2026)

- **Compiled on saving the module:** with Automatically Compile Scripts
  on Save, File › Save compiles the scripts whose text it stores (only a
  script's own Save did; a builder found scripts left uncompiled). A
  script saved earlier and never compiled is not caught by it (Compile
  All is); one the external editor saved is compiled when its text is
  next stored here. (S)
- **A project's script in the external editor** opens in place (the
  project's own `.nss`), and its saves come back by the reading of files
  changed outside, so not at all with that turned off in Options ›
  General (the copy's way is used then). Scripts of a module file still
  open as a copy in the temporary folder. Tried with a stand-in editor;
  not with VS Code. (S)
- **Text in another language where the one edited has none** (English
  first), as a builder says Aurora shows it: the palettes, the module
  tree, conversation lines, the journal's lists, and, marked with the
  language named, every field that edits such text: a blueprint's and an
  area's, Module Properties' name and description, the journal's names
  and entries, a conversation line's text. That Aurora prefers English
  to the first language found was not captured. A token put into a line
  shown in another language goes into that text, which then becomes the
  line's own in the language edited. (S)
- **The module tree goes to the area of the tab chosen** (a click on the
  tab), marking its row; Show in Module Tree on the tab's menu opens it
  out too. A tab reached otherwise (opened, or by keys) doesn't move the
  tree, and other kinds of tab (a script, a blueprint) don't. (S)

## From builders' reports after 1.16.2 (October 2026)

Done: scripts opened from a script editor (Open…, the module's, its
haks' or all, as Aurora's Resources to Show); Enter finding the next
match; Find In Currently Open Scripts; a double click on a name bringing
its Help forward; New on the palettes' right-click menus; filtered
categories that close again; Maximize over the whole window; the module
tree showing the object selected in the area; the tree's Filter kept in
sight; Home and End in the tree and the palettes; a creature's feats
listed beside the feats to choose from; the module tree's rows out of
sight not laid out (a huge module slowed everything while a group was
open). Left, or to know:

- **Items lying in an area** are turned as baseitems.2da's
  `RotateOnGround` says (1: a quarter turn about the model's Y axis, a
  sword or a shield on its flat; 2: about X, a potion stood up; 0: as
  the model is), which a builder's picture of Aurora beside Moonglow
  showed missing (potions lay, shields stood on edge). Armour is the game's
  model of armour dropped (`gi_armor01`…`04`), as Aurora draws it: by
  its weight (the torso's parts_chest.2da `ACBONUS`: 0 cloth `01`, 1 to
  3 leather `04`, 4 and 5 chain `03`, more plate `02`); the base item's
  `DefaultModel` where the model is missing. Compared with the game
  client (`items_look` in `client_render.rs`): a shield lies with its
  face up, a potion stands, a sword lies the same way round, and
  clothing, leather, chain and plate are those four models, as drawn
  here. (A cloak was the bag until 1.19.3: see the reports after
  1.19.1.) (S)
- **Lights placed on the fly** ([the proposal](lights-proposal.md)):
  way A is in (the game's seven invisible light placeables from the
  area's menu, a sun for a marker, Light Color, a ring for the reach).
  The reach is measured in the client (`placeable_light_uniforms`): a
  light of radius 10 in the lightcolor.2da color, ending 20 m out
  (further for a color brighter than 1), static or not, as Moonglow
  drew it already. Not in: a palette entry beside the menu, and way B (any color and reach, by generated content), which
  needs a 2DA merger first. An object whose model is too small to see
  (under half a metre across, or empty) is now drawn and picked by a
  marker's box, lights and the Invisible Object alike. (M)
- **Raise/Lower dropped by a right click that found no ground** (a
  builder lowering terrain past placeables got their menu, or selected
  them): found from his debug log, which showed an object's Properties
  opened from the area with the brush thought in hand. A right click
  where no ground of the area is under the pointer (a cliff face with
  nothing to stand on, the ray going on past the area's edge) dropped
  the brush without a word, as it does for the other brushes, though
  for Raise/Lower a right click is the lowering; the next click was the
  object's. Raise/Lower stays in hand now (Escape drops it), and a
  brush dropped by a right click is noted in the debug log. (—)
- **The tiles that refuse a terrain stroke flash red** (a raise, a
  lowering, a painting, a crosser, a group, Delete on tiles): each cell
  that has no tile fitting what the stroke would make of it, or holds a
  group's tile the stroke would not keep, as Aurora flashes what is in
  the way. All of them, not the first found. (—)
- **Palette categories named in `DELETE_ME` alone** (no StrRef, no
  `NAME`: a Spanish builder's skeletons) keep their names in the Custom
  palette, which showed them blank; names written out are read in
  Windows-1252 ("Compañeros"). A name that is a talk-table string the
  table lacks now says which string, in place of nothing. (—)
- **Keys in a window over an area view**: no key moves the camera while
  a text field has the keyboard. The arrows did (letters and digits were
  already the field's), so moving the text cursor in a conversation or a
  script over the view slid the area. (—)
- **A tile's variant in Tile Properties**: with one tile chosen, the
  tiles that fit there show as pictures (each tile's model seen from the
  south-west, turned as it would lie, its quarter turns written under
  it); a click puts that one there, a step of its own, and
  the window becomes that tile's. Tile Properties is a tab now (a
  window to resize, maximize or dock), the pictures filling it. (—)
- **Emitters a builder does not see** (GitHub issue 8: a demon's wings,
  braziers, shafts of light, all without particles in his area): not
  reproduced. The game's own balor, fire elemental, campfire and brazier
  show theirs in the area view here (`look_particles_of`), so it is his
  content, a setting (Animations off shows none) or his machine; asked
  of him. (?)
- **Text typed in a field in another language than English** was
  written in Windows-1252 whatever the language: Polish "ł" became "?".
  It is written in the language's own codepage now, as the String Edit
  window always did. (—)
- **Names in UTF-8 in a Custom palette** (a module's blueprints, as
  Moonglow writes them) were read as Windows-1252 since 1.17.0
  ("KrysztaÅ‚owa"): UTF-8 where it is that, Windows-1252 otherwise. (—)
- **Done from the same round**: conversation lines dropped above a line
  of their kind, and their right-click menu; an encounter's spawn point
  moved by the foot of its post; Drop to Ground on triggers and
  encounters; several objects turned about their middle (Together);
  Resize Area at any two edges (the area turned, resized and turned
  back: what Aurora does at north and east, at the others); Previous
  Variant; the area opened marked in the module tree; the chosen
  appearance's picture on its list's box.
- **A shader with the environment map switched off reflects nothing**: a
  creature whose appearance.2da row names an environment map (`default`)
  showed a pale shiny band where its hair's texture is see-through; the
  hair's MTR names `fslit_nm`, whose source has `#define ENVIRONMENT_MAP
  0`. Seen in the client (`creatures_look`, `MG_OVERRIDE`): with that
  shader the map is not reflected, and the alpha is see-through with the
  MTR's `transparency` and drawn solid (the texture's own color) without
  it; with no shader named, or no MTR, the band shows in the game too,
  `transparency` or not. Moonglow reads the named shader's source for
  that one line and runs none of it. (—)
- **The Build Module window with Advanced Controls** grew to the
  screen's height, its Build button out of reach: the dividers between
  the three columns took all the height there was. Gaps now. (—)
- **Edit on every row's menu in the module tree**, as a double click;
  areas keep View Area. (—)
- **Find In Currently Open Scripts** searches the scripts whose editors
  hold their text, which includes one closed with text typed and not
  saved. (S)
- **The module tree follows every selection of one object** in the area
  in front, opening the area out each time; with several selected, or
  none, it stays where it is. No way to turn it off. (S)
- **Spells as the Feats page is now** (those known beside those to
  choose from): asked for, and left: the page also has levels, classes
  and memorized counts to lay out. (M)
- **Undo in the script editor** "doing weird stuff" was reported as
  seeming fixed; nothing was changed for it and nothing reproduced. (—)
- **Long lists opened out:** the tree's rows out of sight take their
  room only; its groups' names are still gathered and sorted every
  frame, which a module of tens of thousands of resources may feel. The
  palettes laid out only what is in sight already. Not measured on the
  reporter's module. (S–M)

## From builders' reports after 1.19.1 (October 2026)

Done: a trigger or an encounter turns by its outline, alone (about the
outline's middle) and with others (Together); the Shaft of Light's beam
stands and its glow lies (`Aligned_to_World_Z` and `Billboard_to_World_Z`
were the wrong way round: seen in the client); Export adds to an archive
that is there; the talk table as JSON, its empty lines left out of the
list, and a `.tlk` file opened or made on its own; the resource
browser's Save As on a row and Export as Files. Left, or to know:

- **A module's `encoding.2da`** (EE 1.87: a table of the Unicode
  character each of the 256 byte values stands for, shipped with fonts
  to match, so that a module can be in a language the game has no
  codepage for; a builder who knows the engine pointed it out) is read
  now (`GameData::codepage`, `mg_core::Codepage::table`): the
  toolset shows and writes the module's text by it (names and other
  localized strings in the editors, tags and comments, scripts,
  conversations, the journal, factions, the talk tables, the palettes,
  the area list, Find and Replace Text, the 2DA editor), and reads it
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
    name and the script language server still read and write
    Windows-1252 whatever the table. (S each)
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
    setting, which Moonglow does not read.
  - A text field already being typed in when the hak list changes keeps
    what it showed until it is left.
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
- **A waterfall's water ran back up the stream** (a builder, in
  Medieval Rural: "these don't act like falls"). Particles an emitter
  keeps in its own space (`inherit`) were pulled down along the
  emitter's own axis, and a waterfall's emitter points out over the
  edge: the water slowed, turned and went back. Down is the world's now,
  whatever way the emitter is turned (`waterfalls.rs`: the three
  waterfall tiles' water reaches the foot of the fall). Seen in the
  game client (`particles_look`, `MG_PARTICLES=inherit`): two weighted
  emitters thrown sideways, one keeping its particles and one not, fall
  in the same arc. The same
  builder's fog placeable ("Fog: White - Low", custom content) was not
  to hand: not looked at. (S)
- **`mg roundtrip`** (a builder asked what keeps work from being
  corrupted): a module saved, read back and each GFF and 2DA written
  anew, compared with what it was. It does not ask the game or Aurora
  (the engine and Aurora tests do, on the game's own modules), does not
  write the other formats anew (scripts, models and the rest are kept as
  bytes, and compared as such), and is not in the toolset's menus. A
  written GFF is compared by its fields, not its bytes: Moonglow lays a
  GFF out its own way, as every edit already does. (S)
- **A module folder with a resource under two spellings** (a builder:
  Aurora named two new items `x.UTI`, Moonglow wrote them again as
  `x.uti`, and the folder had both). A folder's resource under another
  spelling than Moonglow's (lower case) is renamed at the next save;
  where both files are there (a folder that sets capitals apart), the
  one changed last is the resource when the folder opens, and the save
  leaves the one file. Nothing is said in the log, and a nasher
  project's sources are not looked at this way. (S)
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
  box). (S)
- **A palette's rows are one line** (a builder: names with ResRefs and
  challenge ratings wrapped onto a second line in a narrow pane), cut
  short as the module tree's are, whole under the pointer. Not an
  option. (—)
- **Update Instances lists what it updated** in the log (a builder: as
  Aurora's does, which was not captured), up to 200 of them. (—)
- **The hak list of Module Properties** (GitHub issue 17) is the
  issue's recommended layout: a framed list numbered from the top, rows
  chosen (Ctrl and Shift for more) and moved by a drag, the arrows under
  the list or Alt + Up and Down, removed with Remove or Delete, one undo
  each (so nothing asks before a removal, as the issue has it); haks
  not in the hak folders flagged; the talk table in a section of its
  own. Not done: a drag does not scroll a list longer than its frame.
  (S)
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
  tested; the client was not run as a DM). Issue 13's other half, a
  name's English field decoded as Windows-1252 where the tree shows it
  in the language's codepage, was not looked at. (S)
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
  Reload Resources cures it. (S–M)
- **The hak editor asks before replacing** files the hak already has
  (Replace, Skip Those, Cancel), as a builder said Aurora's does. Update
  from Folder still replaces, as it says. (—)
- **Arrows after a creature's statistics** (abilities, natural AC, hit
  points, save bonuses), as Aurora's fields have; the other number
  fields have none. (S)
- **A .mod beside a module folder on save**, as a builder says Aurora
  writes one for a module directory: on unless Options › General
  switches it off (the user's decision: a folder opened from elsewhere,
  a repository's source tree, gets a `.mod` written beside it unless it
  is off). A nasher project is left to Pack Target. Aurora's own
  behavior was not captured. (—)
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
- **The talk table editor** (a builder): Go to takes a StrRef or a
  line's number; the text boxes share the room under the list and
  scroll, where a long text ran out of the window. Not done: the split
  between the list and the boxes cannot be dragged. (S)
- **The arrow keys in the palette** (a builder: Aurora's go to the
  parent on Left and let one move through and fold the tree): Up, Down,
  Left and Right move a cursor through the blueprint palettes' rows
  after a click in the palette, and the camera leaves the arrows alone
  until the pointer is back in an area's view. Not done: the tileset's
  palette, the Prefabs list and the module tree have no such keys; in
  the Gallery Up and Down go a picture at a time, not a row of them;
  the keys repeat only as the system repeats them. With the keys on a
  category the blueprint selected stays in hand; whether Aurora keeps
  one in hand then was not checked. (S–M)
- **The module tree and the object selected in the area:** the tree
  goes to it only where its area is opened out there; an area left
  closed stays closed. Asked for by the user (it was opened out and
  scrolled to on every selection, written to follow Aurora, whose own
  tree was not captured doing so).
- **A cloak lying in an area** is the game's model of a cloak dropped
  (`gi_cloak01`, folded, its PLT texture in the cloak's colors), as
  Aurora and the game draw it; Moonglow drew the bag (the cloak's
  `DefaultModel`, which neither uses for it). Compared with the game
  client (`items_look` with three of the game's cloaks: the same model
  and colors, lying the same way). Where the model is missing: the
  `DefaultModel`, then the bag. Whether a custom base item of cloaks
  that names a model of its own should lie as that instead was not
  measured (the game's own names the bag, which it does not use). (S)
- **An armor lying as plate here and as a tunic in Aurora**, in the
  same builder's picture: not looked into. The model is chosen by the
  torso's parts_chest.2da `ACBONUS` (measured with the game's own
  armors); his armor, or his content's table, may be a case that rule
  misses. Needs the item. (S)
- **Models as text from the resource browser** (asked with the export):
  files go out as they are, a compiled model compiled. Writing a model
  as text needs the Viewer's decompiler in the toolset (see "Proposed by
  the Moonglow Viewer session"). (M, with that move)
- **Export into an archive** is a checkbox, off unless ticked; a builder
  says Aurora adds to an existing archive itself. Aurora's own behavior
  (whether it asks) was not captured. (S)
- **An upright `Aligned_to_World_Z` particle** is turned about Z to the
  eye here; whether the game turns it or keeps a fixed side was not
  measured (a still quad looked the same from the one view taken). (S)
- **A talk table kept as JSON in a nasher project** is not read or
  written with the project (a builder keeps one, and may keep several,
  one a language): Import JSON and Export JSON by hand for now. (M)
- **Text put in the talk table as it is written** (a conversation's
  lines, names), in a range set aside for it, and reusing a line that
  already has the same text, asked first: proposed by builders, not
  begun. Move to Talk Table does one string at a time. (L)
- **The feminine table as JSON:** Export JSON writes the main table
  only, as `nwn_tlk` would of that file. (S)

## From GitHub issue 7 (October 2026)

- **Arrow keys on lists of choices:** the blueprint editors' 2DA lists
  and part lists take them while the list has the focus (clicked, open
  or after a choice; Tab). Escape gives the focus up, as everywhere in
  the interface, so after closing a list with Escape it is clicked
  again. The lists outside the blueprint editors (Area Properties, the
  area's toolbar) don't take them. (S)
- **Pictures in the Appearance list:** a creature's (a plain male body
  of the appearance) and a placeable's, beside the row the pointer rests
  on; a door's and an item's lists have none. (S)
- **Export from a menu** is on the palette's custom blueprints (and the
  module tree's resources, as before). A placed object in an area's list
  has no Export: it is not a resource; it would be written out as a
  blueprint of its own. (S)
- **An item stood up in its viewer** is turned a quarter about its
  length's axis from how it lies; which side then faces the viewer isn't
  chosen (a sword shows its edge more than its flat). Items in the
  palette's Gallery and on the ground in an area lie as modelled. (S)
- **Done:** a conversation's lines move up and down among their parent's;
  a potion's parts are listed (they were chosen by models, and a potion
  has icons only); Random Facing.

## From GitHub issue 6 (October 2026)

Done: linked particles drawn a frame each (a ground mist was a lattice
of its sprite sheet), Edit Areas Together giving every area a page's
settings, a test refused while the game of the last one runs, an area's
sounds coming up over three seconds (those that play now and then not
all at once), a folded window's size not remembered for the next of its
kind, any snapping angle, Escape deselecting. Already there: copy and
paste (and its Windows fix), missing music tried once, custom palette
categories, the galleries. Water ripples. Left:

- **Linked particles against the game:** each is stretched to reach the
  next, showing its own frame, as the wiki describes them; not compared
  with the client. (S)
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
- **A running game** is known only if Moonglow started it for a test:
  one started otherwise still has the test module written under it. (S)
- **Clicking between tiles and objects:** a click on an object while
  tiles are selected goes to objects, and a double click on the ground
  to its tile. So a tile wholly under an object is selected by a box,
  not a click. (—)
- **Give Every Area This Page's Settings** copies the fields the page
  shows, not the tiles' lights a lighting scheme sets. (S)

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
  draws them so was not captured. (S)
- **Sound ranges:** level circles at the sound's height, not spheres; a
  sound's random position range is a rectangle of `RandomRangeX` and
  `RandomRangeY` each way (that the game takes them as half-widths about
  the sound was not measured). (S)
- **The start location in the view:** set from the menu, and its marker
  dragged (ring, shaft) and turned (arrow's tip). Aurora's palette entry
  for it isn't there; selected, it is no object, so the keys (Q and E,
  the arrows) and Adjust Location don't act on it; a drag stays in its
  area; the pointer doesn't change over it. (S)
- **Deleting from the module tree:** Delete… removes the resource (an
  area with its GIT and GIC and its area-list entry, a script with its
  NCS) and nothing else: transitions, scripts and conversations that name
  it are left as they are, with no warning of them; one resource at a
  time (the Delete key asks the same of the row under the pointer). Moonglow refuses to delete the
  start area; Aurora removes it (after asking twice) and then refuses
  to save: "A module must have a starting location to be valid." (S)
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
- **Edit on a script that doesn't exist** makes it and says so in the
  log; Aurora shows a message box first ("Resource not found. Creating
  new script."). A mistyped name makes a script (Undo takes it back). (S)
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
  viewer's own check of names over 16 characters is as it was. (S)
- **Expand All and Collapse All** (the module tree, the palettes): the
  module tree's areas opened out to their objects, and the kinds under
  them, keep their own state; no keys for them. (S)
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
- **A project's files changed outside** are read again by polling (every
  3 s, the whole source tree listed each time: not measured on a project
  of thousands of files; no file-system watcher). The files' times and
  sizes decide what is read, so a change that keeps both within the
  clock's grain is missed. A reload that touches resources the undo
  history has changes to clears the whole history. A script open in the
  editor with typing not yet applied counts as changed here. Compiled
  scripts aren't made again on reload (the next build does). Only nasher
  projects: a `.mod` file or a module folder changed outside isn't
  noticed. The area list isn't settled again for areas added outside.
  Not tried against a real `git checkout` of many files. (M)
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
- **An area's objects in the module tree:** listed by name, kind by
  kind, with Go To, Properties, Copy and Delete on each; one at a time
  (no selecting several there), and the Filter finds them only in areas
  already opened out. A copy made there stands on the ground where it is
  pasted (its height above the ground isn't carried). (S)
- **The camera's height** is each view's and lasts until its tab closes.
  (Lighting and Sound Ranges are kept: a view opens as they were last
  left, and one already open keeps its own.) (S)
- **Minimap pictures for tilesets without them:** Moonglow exports the
  pictures the tileset has; rendering tiles from above to make them
  (NeverBlender's minimap tool) isn't offered. (M)
- **The minimap in Moonglow:** the area view has no map panel of its own,
  and Area Transition setup picks its target from a list rather than a
  map. (S–M)

## The turning ring

- **Several objects:** each has its ring (and with Shift its tilt rings
  and arrows), up to 32 of the selection; each turns about itself, as
  Alt + right drag does. Turning a group about its middle (positions
  swinging round too) is not done. (S–M)
- **Behind things:** the rings and arrows are painted over the view, not
  hidden by what stands in front of them (as 3D editors draw theirs:
  left so). (—)
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
- **Tilting against the game:** compared with the client
  (`placeables_look` in `client_render.rs`, by eye): a placeable turned
  about all three axes stands the same in both, and the game draws a
  static placeable upright whatever its file says. Moonglow drew the
  static one tilted (only its handles left it out); it is upright
  now. (—)
- **Escape during a drag** drops it now (it used to put the object down
  where it was); Aurora's behavior was not checked. (S)

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
  driven. (—)

## Automation

- **Plugins, beyond API 0.2** (`plugin-proposal.md`; commands and checks
  in sandboxed Luau are built; 0.2 added a file or folder the user
  chooses, haks written with their leave, areas and terrain, pictures):
  - **A builder's terrain tool (NWN Mapper) as the measure:** it sculpts
    a free height field, paints texture layers and a walkmesh, bakes
    placeables and a lightmap in, and writes a tileset's group (a model
    and walkmesh per tile, textures, materials, a patched `.set`). A
    plugin can now take such an export into a hak and make the area
    (`examples/tileset-import`, tried on the builder's earlier export:
    149 files, an area of one of its groups drawn from the hak). What
    the tool itself does is out of a plugin's reach: there is no view
    to draw in or brush to take input from, a plugin writes haks and
    nothing else (no loose files, no talk table), and baking textures
    in Luau would be slow. Sculpted terrain inside Moonglow would be a
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
    quick fixes for findings, events, settings a plugin keeps, panels.
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
- **Minimap pictures** have Moonglow's renderer's look (lit models, as in
  the area view), not the game's painted look; and models must be
  readable (beside the `.set` or in the game data). (S–M)
- **Making models** (Neverblender's job) and **testing a tileset** by
  painting an area with it straight from its folder (it has to be in a hak
  the module uses, or in development). (M)
- **The palette's arrangement:** it's generated flat, without the
  subfolders hand-made palettes have. (S)

## Release and packaging (v0.1.0)

- **macOS:** the app and disk image are built by CI but have never been
  run on a Mac. (S, needs a Mac)
- **Windows signing:** the installer isn't code-signed, so SmartScreen
  warns. (S, needs a certificate)

## Proposed by the Moonglow Viewer session (2026-10-07)

Passed on by the session that maintains Moonglow Viewer
(`/home/august/Projects/moonglow-viewer`, its `docs/PLAN.md` §10.1 has
the full text); nothing of it is taken up yet, and none of its findings
were checked here beyond that the code named is there and that the
toolset is as the proposal says it is.

- **A model compiler and writers for `mg-mdl`:** the viewer's
  `crates/mgv-mdl` has a binary MDL writer (every node type, both skin
  layouts), an ASCII-to-binary compiler that runs in process (tangents,
  a walkmesh's AABB tree, skins' inverse binds, part numbers), a
  lossless ASCII writer and a linter by line, built on `mg-mdl`'s public
  types with no new dependencies, under the same license. Its tests are
  said to round-trip all 25,597 compiled game models and to match the
  game's own `compilemodel`. Moved here it could serve `mg pack`, `mg
  verify` and the hak tools without `nwnmdlcomp`. It asks one addition
  of the reader: each node's part number and the header's node count on
  `mg_mdl::Model`. Decided (2026-10-07): it is to live in the toolset,
  since the viewer already builds on the toolset's crates and not the
  other way about. Not moved yet. (M–L)
- **Animations bind by part number, not by name:** done. The reader
  keeps each node's part number, and `mg-render` binds compiled models
  and animations by it (by name where either is from text, and for a
  worn cloak or robe). Seen in the viewer session's client pictures (a
  skeleton numbered afresh under the same names leaves its creatures
  unposed), not measured again here; a human and an elf in armor and
  cloak still stand as the client draws them. Of the game's models, 70
  have nodes the two ways move differently, none of them looked at one
  by one. A node an animation numbers −1 is not moved, on the reading
  that the game moves none for it. (S)
- **Other findings against `notes_models.md`,** each theirs to confirm
  here before the note is changed: part numbers follow file order and a
  compiled tree does not keep it (5,079 of 24,298 models without a
  supermodel); function pointers are ignored; a mesh's box holds the
  origin too; shininess is 1 where a text gives none; EE's 0x3B0 skin
  layout with bone numbers as `i16[64]` is what the game takes; the
  game compiler's normals and tangents equal `mg-mdl`'s ASCII reader's
  to four decimals; ten `c_wingdrg2_*` wing skins are bound in another
  pose than they rest in. (S each, the note's upkeep)
- **Smaller:** `mg-ui`'s form widgets and palette chooser exported for
  the viewer to use (it keeps copies); a camera framed tightly on posed
  vertices, so that `mg-preview`'s pictures of long models (dragons)
  fill their frame. (S each)

## From the research notes

- **A skin's bones that are no nodes:** a skinned mesh whose bone map
  names nodes past the model's last (custom content; it crashed 1.4.0 on
  loading a module) is drawn with those bones unmoved. What the game
  does with such a model wasn't looked at, and the model itself wasn't
  seen: the fix is from the crash report. (S, needs the model)
- **A model that fails while it is drawn:** a model that breaks what
  builds or poses it is left out and the module stays open
  (`mg_render::guard`: tiles, objects, the model viewer, thumbnails). A
  failure inside the renderer's own drawing (uploading its textures, the
  GPU's calls) is not caught and still ends Moonglow, with a crash
  report. (M)
- **Left out for the session:** a model that failed stays out until its
  area is opened again; the object shows no box in its place beyond what
  a missing model has. (S)

- **TGA, right-to-left:** the game ignores a TGA's top-left origin (bit
  5) and its right-to-left bit (bit 4) alike: rows and columns are taken
  as stored (both measured in the client; `placeables_look` with
  `MG_TGA=16`). Moonglow honored bit 4 until this was measured. (—)
- **A custom creature's animation "off" (GitHub issue 3):** the report's
  Mindwitness also poses differently than in Aurora and the game; not
  reproduced without the model. (needs the files)

- **Index lists against the client:** that the game draws a compiled
  mesh's index list where it has more triangles than the face list
  (`Mesh::drawn`: most Forest - Facelift cliffs) is settled from the
  files (the list reaches every vertex, the faces are its first
  triangles, drawing it closes the holes), not by a client screenshot
  (`client_render.rs`). (S)
- **Waypoint flags against Aurora:** the flags are drawn unlit so their
  colors tell apart in a dark area; how Aurora lights them was not
  captured. (S)
- **KTX textures:** `mg-image` doesn't read KTX yet (`PLAN.md` §4). (M)
- **Tile Properties' Defaults:** not yet compared with Aurora for a
  lighting scheme whose colors aren't black
  (`docs/research/notes_tilesets.md`). (S)
- **The manual's tables:** the built-in manual viewer can't wrap a
  table's cells, so a table wider than the page is shown there as a
  list (`manual::fitted`; it was cut off at the page's edge before, and
  the text after it too). Tables with wrapped cells would read better.
  (S–M)
