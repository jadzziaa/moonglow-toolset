# Deferred: what was left out, for a decision later

Things each piece of the post-parity work (`PLAN.md`, "After parity")
left undone, collected in one place. Once the map's current items are
worked through, each gets a verdict: fix, add, or drop. New gaps are
added here as work finishes.

Size: S (an hour or two), M (a day or so), L (several days).

## Script intelligence (`mg lsp`, the script editor)

- **Struct members:** go to definition, references and rename don't
  follow `p.nX` to the struct's declaration. (M)
- **Incremental compiles:** Compile All recompiles everything. It could
  compile only what changed and the scripts that include it, as Arelith's
  ARE_Compile does. At 4,000 scripts a full compile is 1.4 s, so this
  matters less than expected. (M)
- **Editor setups:** the manual's Neovim and Helix snippets haven't been
  tried in those editors. On Windows, `mg lsp` is checked only by CI's
  tests. (S)
- **A VS Code extension:** today VS Code needs a generic language-server
  extension pointed at `mg lsp`. A small Moonglow extension would make
  that one click. (M)

## nasher projects

- **NWNT projects:** projects using NWNT instead of JSON for GFF files
  are refused with a message. (M)
- **Compiled scripts without source:** a module whose scripts exist
  only as `.ncs` loses them when it becomes a project, as with nasher.
  Moonglow names them. It could keep such `.ncs` files in the project
  instead. (S)

## Content doctor

- **The walkmesh crash rule:** nwn.wiki names "walkmesh outside the
  tile" as an Aurora crash cause, but the game's own tiles reach 10 m
  out and work. The real cause is unknown, so there's no check.
  Finding it would mean crashing Aurora on purpose with test tiles. (M)

## Test loop

- **Choose Character:** Test Module, Choose Character uses the game's
  `+LoadNewModule`, taken from nwn.wiki. It hasn't been run in the game
  client. (S)
- **Aurora's F9 problems:** laggy combat, AI timing errors and damaged
  modules after a crash. Moonglow launches the game the way the wiki
  recommends to avoid them, but nothing has checked that they're
  gone. (S–M)

## Persistent-world scale

- **Real CEP haks:** the test world's 150,000 hak files are tiny stand-ins.
  Loading real CEP content at scale isn't measured; the campaign budgets
  and model corpus tests cover real models. (S, needs CEP downloaded)
- **Other machines:** budgets were measured only on the development
  machine (Ryzen 7 5800X3D, Radeon RX 9070). (S, needs other hardware)
- **Inventory lists:** a store page or inventory lays out every row and
  builds every item icon the first time it's shown. 449 icons take
  0.68 s; a page of thousands would take seconds. Laying out only the
  rows on screen would fix it. (S–M)
- **Where-used at scale:** one Find References takes 0.8 s in the
  persistent world, since it scans the module each time. An index kept
  up to date would make it instant. (M)

## EE fields Aurora hides

- **Custom shader effects:** shader parameters and the extra area flags
  are saved and the game reads them. Moonglow's renderer doesn't run
  custom shaders, so they don't change what the area view shows. (L)
- **Item costs:** Additional Cost keeps Aurora's limit. The game uses the
  stored Cost, which Moonglow computes, so nothing seemed to need it. (S
  to check)
- **The Classes page in a narrow window:** the second domain picker
  sits at the window's edge. (S)

## Bulk editing

- **Lists across several blueprints:** Edit Together leaves out the
  pages that edit lists (inventories, classes, skills, feats, spells,
  item properties); those are edited one blueprint at a time. Adding the
  same item or feat to each would need its own design. (M)
- **The multi-object editor's list pages:** with several placed objects
  selected, those pages still change the first object only, as before.
  Edit Together's guard (leave the pages out, drop changes that reach only
  the first) could apply there too, once Aurora's multi-editor is checked
  for what it allows. (S, plus an Aurora capture)
- **Find and Replace beyond strings players read:** tags, resrefs and
  other plain text fields aren't searched (Find References covers tags,
  Find in Files covers scripts), and there are no regular expressions.
  (S–M)
- **Variable sets** hold int, float and string variables only, as the
  Variables window edits; object and location variables aren't kept. (S)

## Conversation authoring

- **Spell checking** in the conversation editor (and text fields). The
  game ships no dictionary, so it means the system's:
  - Linux: hunspell dictionaries, read by a pure-Rust crate (`spellbook`).
    That's a new dependency to download.
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
  waypoints have no picture on hover (they have no model); an icon or a
  summary could stand in. (S)
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
- **One language:** the editor edits the table in its own language;
  translated tables (the same name in other languages' folders) aren't
  shown side by side. (M)
- **No talk-table import or export** (CSV or nwn_tlk's JSON, for
  spreadsheets and git); neverwinter.nim's `nwn_tlk` does it. (S)
- **Where a StrRef is used:** Find References doesn't find the 2DA cells,
  blueprints and conversations that name a talk-table line. (M)
- **Ctrl+Z in the talk table** undoes the module, not the table (its own
  Undo is in its toolbar). (S)
- **Inserting or removing lines in the middle** isn't offered, since it
  renumbers every line after; renumbering their users along with them
  would need where-used first. (M)
- **The client and custom talk tables:** the engine test runs the server;
  whether the client also reads a table from the module (not only from
  haks and the `tlk` folder) for 2DA text isn't tested. (S)

## Haks in the GUI

- **No viewing inside the hak editor:** a resource is extracted to look at
  it, or seen in the resource browser once the module uses the hak. (S)
- **Build Hak from Folder doesn't remember the folder:** rebuilding after
  changing the folder is Build again (or `mg pack`). A remembered folder
  could update the hak in one click. (S)
- **No zip, 7z or rar:** content downloaded as an archive is unpacked
  first; Add Haks and Talk Table takes the haks and talk table. (S, a new
  dependency)
- **Hak order advice:** Moonglow doesn't know which haks go above which
  (CEP, PRC and the like each document theirs); the conflict report shows
  what each hak hides. (M)
- **Sorting** the hak editor's list by size or type, and shift+click
  ranges. (S)
- **ERF version:** haks are saved as `V1.0`; EE's compressed `E1.0` is
  read but not written. (S–M)

## Area visibility

- **Walkmesh cutters:** not drawn, and not placed (the trigger kind EE
  added). (S–M)
- **A walkmesh's own node position:** one shipped placeable
  (`ptm_candle02.pwk`, Tyrants of the Moonsea) puts its mesh 72 m from
  itself. Moonglow draws it there; whether the engine does isn't tested.
  (S)
- **Minimap pictures for tilesets without them:** Moonglow exports the
  pictures the tileset has; rendering tiles from above to make them
  (NeverBlender's minimap tool) isn't offered. (M)
- **The minimap in Moonglow:** the area view has no map panel of its own,
  and Area Transition setup picks its target from a list rather than a
  map. (S–M)

## Remappable keys

- **Not every key:** text editing, Escape and Enter, Copy, Cut and Paste,
  Delete in the area view, the script editor's numbered bookmarks, and
  mouse bindings (drags, Ctrl+click) are fixed. (S)
- **Menu commands without keys** (Verify, Build Module, the other
  wizards, Find Instance…) can't be given one yet: each command must be
  listed. (S)
- **Aurora's keys Moonglow lacks:** Plot Wizard (Ctrl+Alt+P; no plot
  wizard), Refresh (F5), the script editor's Save As (Ctrl+Alt+S, now
  New Script's), Close (Ctrl+F4), and the conversation editor's Find and
  Replace (Ctrl+F, F3, Ctrl+R). (S)
- **Sharing key sets:** no import or export of the keys (they're in the
  settings file). (S)

## Automation

- **A plugin or scripting API** for the GUI (menu commands and panels of
  one's own): not started. `mg --json` and the `mg-module` crate are the
  interface for now. (L)
- **`mg find` beyond objects:** it searches blueprints and placed objects;
  conversation lines, scripts' text and 2DA rows have `mg replace
  --dry-run`, the script editor's Find in Files and the resource browser.
  (S–M)
- **Editing fields from the command line** (`mg set MODULE NAME.EXT
  Label=Value`): `mg gff` round trips through JSON instead. (S)
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
- **Compression:** zstd's fastest level (all the encoder Moonglow has
  offers), so the data is about a quarter bigger than
  `nwn_nwsync_write`'s (9.8 MB against 7.7 MB in `nwsync.rs`): more to
  upload and download. A stronger encoder would be a new dependency. (S)
- **Portraits, music and ambient sound** in folders (not haks) aren't
  gathered; they go in a hak first, as with `nwn_nwsync_write`. (S)

## Release and packaging (v0.1.0)

- **macOS:** the app and disk image are built by CI but have never been
  run on a Mac. (S, needs a Mac)
- **Windows signing:** the installer isn't code-signed, so SmartScreen
  warns. (S, needs a certificate)

## From the research notes

- **KTX textures:** `mg-image` doesn't read KTX yet (`PLAN.md` §4). (M)
- **Tile Properties' Defaults:** not yet compared with Aurora for a
  lighting scheme whose colors aren't black
  (`docs/research/notes_tilesets.md`). (S)
- **The manual's tables:** the built-in manual viewer can't wrap a
  table's cells, so chapters use lists where a table would read
  better. (S–M)
