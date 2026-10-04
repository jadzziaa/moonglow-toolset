---
type: Backlog
title: 'Deferred: what was left out, for a decision later'
description: What the post-parity work left undone, by area (script intelligence, nasher projects, content doctor, test loop, scale, EE fields, bulk editing, conversations, palettes, custom content, haks, options), each awaiting a verdict - fix, add or drop.
tags: [backlog, deferred, after-parity]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-04T02:52:17Z }
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

- **Pictures for the rest:** sounds, triggers, encounters, stores and
  waypoints say what they are on hover, in a line or two; a waypoint's
  flag, which the area view draws, could be its picture. (S)
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
- **Shift+click ranges** in the hak editor's list. (S)
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
- **The Options window as a tab:** it is a floating window of a fixed
  size (resizable, the size kept while Moonglow runs), not a tab in the
  dock. (S)
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
  Basic page and an item's Appearance page show it beside their fields
  where the window is wide enough for both; narrower, Preview shows it
  (a creature's Appearance page stacks them instead). (S)
- **Special abilities' flags against the game:** Ready, Spontaneous and
  Unlimited are named from BioWare's creature format document; what the
  game does with each was not run. (S)
- **The light theme's own colors:** it is egui's light theme; the script
  editor's syntax colors follow it, but conversation and faction colors
  are the dark theme's (legible on both), and the area view's overlays
  are drawn for the 3D view. A pass with a builder who uses it would
  settle the rest. (S–M)

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

- **Several objects:** each turns about itself, as Shift + right drag
  does, with the ring around the first; turning a group about its
  middle (positions swinging round too) is not done. (S–M)
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
- **One bad model takes the window down:** a panic while a model is
  loaded or drawn ends Moonglow (with a crash report); catching it, and
  showing the object as a box with a problem noted, would keep the
  module open. (M)

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
