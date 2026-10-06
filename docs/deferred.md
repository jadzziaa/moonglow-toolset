---
type: Backlog
title: 'Deferred: what was left out, for a decision later'
description: What the post-parity work left undone, by area (script intelligence, nasher projects, content doctor, test loop, scale, EE fields, bulk editing, conversations, palettes, custom content, haks, options), each awaiting a verdict - fix, add or drop.
tags: [backlog, deferred, after-parity]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-06T08:26:57Z }
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

## EE fields Aurora hides

- **Custom shader effects:** shader parameters and the extra area flags
  are saved and the game reads them. Moonglow's renderer doesn't run
  custom shaders, so they don't change what the area view shows. (L)
- **Item costs:** Additional Cost keeps Aurora's limit. The game prices a
  created item itself, as Moonglow computes the stored Cost (see
  [Findings](findings.md#created-items-are-priced-anew)), so nothing seemed
  to need it. (S to check)
- **An armor's part colors:** edited and previewed, and the game reads
  them (`engine_armor_colors.rs`). Not compared with the game client's
  drawing (on bare parts, say), and the inventory icon is drawn in the
  armor's own colors; whether the game's icon shows a part's isn't
  known. (S–M, a client capture)
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
- **Areas edited together show the first area's values:** a field whose
  value differs among the areas isn't marked as mixed (nor is it for
  blueprints edited together). (S–M)
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
  titles, Find Instance and the area transition's destinations; the
  references list shows files (`town.git`) and the log ResRefs. (S)
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
- **Special abilities' flags against the game:** Ready, Spontaneous and
  Unlimited are named from BioWare's creature format document; what the
  game does with each was not run. (S)
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
  the HD bodies, and the same with the game's own; that the game takes
  the wearer's scale is inferred from where its cloaks sit, not measured
  in the client. (S)
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
- **Copy… of a script** copies the source only: the copy is compiled
  when the module is built or the script saved. (S)
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
  without it scripts go out as they are, unremarked. (S)
- **One at a time from the tree:** the tree exports one resource at a
  time (the Export window takes several). The area view's To Scratch
  copies the area alone, not the blueprints or scripts it uses. (S)
- **One scratch folder:** To Scratch copies to one folder, for every
  module; a folder per module, or a remote server's (over SSH), isn't
  offered. Export as Files takes any folder, and remembers the last. (S)

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
  sound's random position range (`RandomRangeX`, `RandomRangeY`) isn't
  drawn. (S)
- **The start location in the view:** set from the menu, and its marker
  dragged (ring, shaft) and turned (arrow's tip). Aurora's palette entry
  for it isn't there; selected, it is no object, so the keys (Q and E,
  the arrows) and Adjust Location don't act on it; a drag stays in its
  area; the pointer doesn't change over it. (S)
- **Deleting from the module tree:** Delete… removes the resource (an
  area with its GIT and GIC and its area-list entry, a script with its
  NCS) and nothing else: transitions, scripts and conversations that name
  it are left as they are, with no warning of them; one resource at a
  time; no Delete key in the tree. That Aurora refuses to delete the
  start area was not checked against Aurora (Moonglow refuses). (S)
- **See-through meshes** are drawn in two parts: what is nearly opaque
  of them (alpha from 0.95) with the depth written, then the rest over
  everything solid, back to front, writing none. (A builder's report:
  blue-white patches round plants and along shores at a slant, in a
  base-game tileset: a plant's soft edge drawn before the ground behind
  it kept the ground from showing, and the water under it showed. Shown
  in a renderer test; not confirmed on the reporter's area, nor compared
  with the client.) Two see-through surfaces crossing each other still
  sort by their middles. Each see-through mesh costs two draws. The
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
  viewer. It grows without limit while it is on. (S)
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
  puts the token at the end of the text, not at the cursor. (S)
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
- **Tilting against the game:** that the client draws a tilted placeable
  as Moonglow does (the order of the three angles, and that a static
  placeable's transform is ignored) is taken from Aurora's output and
  the wiki, not from a client screenshot. (S)
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
- **Sharing key sets:** no import or export of the keys (they're in the
  settings file). (S)

## Automation

- **Plugins, beyond API 0.1** (`plugin-proposal.md`; commands and checks
  in sandboxed Luau are built):
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
  5; measured in the client), and Moonglow now does. Whether it also
  ignores right-to-left (bit 4), which Moonglow still honors, was not
  measured; no game file sets it. (S)
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
