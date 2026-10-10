---
type: Manual Page
title: Command-line tools
description: The mg command-line tool - archives and files, the game's resources, modules, setting fields and edit files, several areas at once, JSON output, nasher projects and NWScript in other editors.
tags: [manual, command-line, mg]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-10T00:00:51Z }
---

# Command-line tools

`mg` does from a terminal what scripts and build pipelines need: archives,
GFF files, the game's resources, and building modules. It comes with every
Moonglow package:

| Package | Running `mg` |
| --- | --- |
| AppImage | Link to the AppImage under the name `mg`; started through the link, it runs `mg` instead of the GUI: `ln -s ~/Applications/Moonglow-*.AppImage ~/.local/bin/mg` |
| Flatpak | `flatpak run --command=mg io.github.moonglow_toolset.Moonglow …` |
| Windows | `mg.exe` in the install folder (for an install for your user, `%LOCALAPPDATA%\Programs\Moonglow Toolset`) |
| macOS | `/Applications/Moonglow Toolset.app/Contents/MacOS/mg` |

```text
mg [--root GAME] [--user-dir DIR | --no-user-dir] [--json] COMMAND ...
```

`--root` is the game's folder (default: `$NWN_ROOT`, else Steam's);
`--user-dir` the user folder (default: `$NWN_HOME`, else the platform's);
`--no-user-dir` reads the game alone; `--json` prints the result as JSON
(below). `mg COMMAND --help` describes each command.

NUI authoring uses the same document checks and generator as the GUI:

```text
mg nui new path/to/module.mod my_window
mg --json nui validate path/to/module.mod my_window
mg nui generate path/to/module.mod my_window
```

`new` and `generate` save the module; `validate` is read-only and exits with an
error for invalid JSON, settings or API-contract errors. `generate` compiles
the opener and event handler before saving. An edited opener or a resource
collision stops generation. Output explicitly leaves runtime verification
unclaimed. See [NUI Creator](07-scripts.md#nui-creator) for binds, deployment
and testing in Aurora/NWN.

## Archives and files

| Command | Does |
| --- | --- |
| `mg tileset-palette SET [-o OUT]` | make a tileset's palette (`<tileset>palstd.itp`) from its `.set`: groups, features, terrains and crossers |
| `mg ls ARCHIVE` | list a `.mod`, `.hak`, `.erf`, `.nwm` or `.sav` |
| `mg unpack ARCHIVE OUT` | unpack it into folder `OUT` |
| `mg pack DIR ARCHIVE` | pack a folder into an archive (type from the extension); warns when files would start past 2 GiB, where the game stops reading. `--compile-models` compiles the models kept as text (ASCII `.mdl`) on the way in, each against its supermodel from the folder or the game (which it then needs, as the other commands find it); a model that doesn't compile, has errors `mg verify` would name, or whose supermodel isn't found goes in as the text it is, and is named |
| `mg gff INPUT [-o OUTPUT]` | a GFF file to JSON (the format of neverwinter.nim and nasher), or JSON back to GFF |

## The game's resources

| Command | Does |
| --- | --- |
| `mg which NAME.EXT` | where a resource comes from in the game's load order |
| `mg cat NAME.EXT` | print a resource to standard output; `--text` prints a compiled model (`.mdl`) as the text it compiles from |
| `mg layers` | the load order's layers and their sizes |
| `mg tlk STRREF…` | talk-table strings by number |
| `mg tlk-export TABLE.tlk OUT` | write a talk table as JSON (as neverwinter.nim's `nwn_tlk` writes it) or CSV, by `OUT`'s extension (`.json`, `.csv`); the CSV has the feminine table's text too, where `TABLEf.tlk` lies beside it |
| `mg tlk-import TABLE.tlk FILE` | read a `.json` or `.csv` file of lines into a talk table (made if it isn't there, as an English table) and save it: the lines the file has are set, the others stay; `--dry-run` says what would change |

## Modules

| Command | Does |
| --- | --- |
| `mg info MODULE` | the module's name, tag, entry area, areas, haks, talk table, game version and resources by type |
| `mg find MODULE` | blueprints and objects placed in the areas: `--type utc,utp`, `--tag` (`*` matches any run), `--name` (words it contains), `--resref` (a placed object's blueprint), `--area`, `--placed` or `--blueprints`, and `--where Label=Value` (a field's value, `*` as in tags; `Label` alone: has the field), each as often as needed |
| `mg verify MODULE [--unused]` | missing resources and problems in the custom content (and, with `--unused`, unused resources); fails if there are errors, so a build pipeline stops |
| `mg roundtrip MODULE [--keep COPY.mod]` | checks that Moonglow keeps the module as it is: saves a copy (a temporary one; the module is not touched), reads it back and compares every resource byte for byte, then writes every GFF and 2DA anew and compares what it reads back (the same fields and cells, the fields it has no name for too); lists what differs and fails if anything does |
| `mg model-files WHAT FOLDER [--module MODULE] [--all] [--force] [--dry-run]` | writes the files the game draws a model (`plc_a08`) or a blueprint (`chest.utp`; also `.utd`, `.uti`, `.utc`) with into a folder: models, walkmeshes, textures, materials, from the module's haks and wherever else they are; the game's own are left out unless all are (or `--all`); files already in the folder are not written over without `--force` |
| `mg checks` | the checks `mg verify` makes of custom content: each one's id (a finding's `check` in the JSON) and what it holds to be true |
| `mg verify MODULE --plugins DIR` | also run the checks of the plugins in `DIR` (a plugin's folder, or a folder of plugins); their problems count with the rest |
| `mg plugin list DIR…` | the plugins in the folders and what each adds: its commands and checks |
| `mg plugin check DIR…` | check plugins: the manifest reads, and the code registers what it declares; fails if not. A file is a plugin's archive: checked as the installer checks it, and its code as it is in the archive |
| `mg plugin run MODULE PLUGIN COMMAND` | run a plugin's command on a module and save: its edits as one command, all or none. `--answer ID=VALUE` fills a form's field (the others take their defaults), `--file PATH` is the file or folder it gets where it asks for one (in the order it asks), `--yes` agrees where it asks, and to the haks it writes (into `--hak-dir DIR`, else the user folder's `hak`); `--dry-run` prints the edits (a file `mg apply` reads) and changes nothing |
| `mg plugin pack PLUGIN` | pack a plugin's folder into an archive to hand around (`ID-VERSION.zip`, or `-o FILE`): checked first, hidden files left out |
| `mg plugin install FILE FOLDER` | install a plugin from its archive into a folder of plugins, as Plugins › Install Plugin from File… does: the archive is checked and nothing of it runs. `--replace` installs over the same plugin installed from an archive before |
| `mg plugin remove ID FOLDER` | remove a plugin that was installed from an archive from a folder of plugins, by its id; one put there by hand is left alone |
| `mg haks MODULE` | what the module's haks provide, where they conflict and which game resources they override |
| `mg minimap MODULE AREA OUT.png [--size PX]` | an area's minimap as a PNG, laid out as the game's map draws it (`PX` pixels a tile) |
| `mg nwsync MODULE REPOSITORY` | publish the module's haks and talk table for NWSync into a repository folder, as `nwn_nwsync_write` does: `--with-module` (with `--name`, `--description`, `--uuid`), `--group-id`, `--no-latest`, `--limit-file-size MB`, `--force`, `--dry-run`; prints the manifest's hash |
| `mg attach MODULE FILE…` | copy haks and a talk table (from anywhere) into the user folder's `hak` and `tlk`, list the haks at the top of the module's hak list in the order given, name the talk table, and save (`--replace` replaces different files of the same names there) |
| `mg compile MODULE [--uncompiled]` | compile the module's scripts (or only those without a compiled version) and save it |
| `mg export MODULE NAME.EXT… -o OUT.erf` | export resources with what they use (`--keep-factions`, `--comment`) |
| `mg export MODULE NAME.EXT… --files -o FOLDER` | write the resources named as loose files in FOLDER (a script with its compiled `.ncs` and `.ndb`, an area with its `.git` and `.gic`): for the game's `development` or `override`, or a server's |
| `mg import MODULE ERF [--overwrite]` | import an archive into the module and save it |
| `mg refs MODULE NAME.EXT` | where a resource is used, and script strings that spell it (`--tag` for a tag) |
| `mg rename MODULE NAME.EXT NEW` | rename a script, area, conversation or blueprint everywhere and save (`--strings` changes script strings too) |
| `mg update-instances MODULE [NAME.EXT…]` | remake placed objects from their blueprints (all of the module's if none are named) and save (`--area AREA` for one area) |
| `mg dialog-export MODULE NAME.dlg OUT` | write a conversation as plain text, CSV, Twine or Ink, by `OUT`'s extension (`.txt`, `.csv`, `.twee`, `.ink`) |
| `mg dialog-import MODULE FILE [--name NAME]` | read a Twine or Ink story into the module as a conversation (replacing one of that name), or a CSV export's text back into its conversation, and save |
| `mg replace MODULE FIND WITH` | replace text in the module's names, descriptions, conversations and journal and save (`--match-case`, `--whole-word`, `--only names,journal,…`, `--dry-run` to list the strings only) |
| `mg set MODULE NAME.EXT FIELD=VALUE…` | set fields of a resource and save: a field is a path from the resource's root (`Tag`, `/ClassList[0]/ClassLevel`) and keeps its type; `--remove FIELD`; `--dry-run` prints the change and makes none |
| `mg apply MODULE EDITS.json` | apply a file of edits (below) and save: all of them, or none if one does not apply; `-` reads standard input; `--dry-run` only checks |
| `mg areas MODULE [AREA…]` | list the module's areas, narrowed by `--match TEXT` (in the name, tag or ResRef), `--tileset`, `--interior` or `--exterior`, `--underground` or `--above-ground`, `--natural` or `--artificial`; with `--set FIELD=VALUE`, `--var NAME=VALUE`, `--remove-var NAME`, `--static-placeables` or `--dynamic-placeables`, make each change to each of those areas and save (`--dry-run` to list the changes only; `--all` to change every area) |

A module is a `.mod` archive, a module folder or a nasher project. `mg
compile` and `mg import` write the module in place; an archive's previous
version is kept beside it (`mymodule.mod.bak`).

For example, every placed creature whose tag starts `GUARD` and that has
no OnSpawn script set, in the area `keep`:

```text
mg find mymodule.mod --type utc --placed --area keep --tag 'GUARD*' --where ScriptSpawn=
```

## Setting fields, and edit files

`mg set` changes fields of one resource, and nothing else in it:

```text
mg set mymodule.mod guard.utc Tag=GATE_GUARD '/ClassList[0]/ClassLevel=5'
```

- A field is named by its path from the resource's root: its label, or
  through lists as `/List[item]/Label` (items count from 0; quote the
  brackets in a shell).
- A field keeps the type it has. A field the resource lacks takes the
  type the game's files give it, or the one you give:
  `MyFlag:byte=1` (`byte`, `char`, `word`, `short`, `dword`, `int`,
  `dword64`, `int64`, `float`, `double`, `cexostring`, `resref`,
  `cexolocstring`).
- Text set on a name or description becomes its English text; its
  talk-table reference and other languages stay.
- `--remove FIELD` takes a field away.
- `--dry-run` prints each field as it was and would be; with `--json`,
  the change under `"command"` is an edit file.

`mg apply` reads an **edit file**: every change the editors make is one
of these six, so a script can write any change as data, and Moonglow
applies it as the editors would.

```json
{ "version": 1, "label": "Promote the guard", "edits": [
  { "op": "set_field", "resource": "guard.utc", "field": "/Tag",
    "value": { "type": "cexostring", "value": "GATE_GUARD" } },
  { "op": "remove_field", "resource": "guard.utc", "field": "/Comment" },
  { "op": "insert_item", "resource": "guard.utc", "list": "/ClassList",
    "index": 1, "item": { "__struct_id": 2,
      "Class": { "type": "int", "value": 4 },
      "ClassLevel": { "type": "short", "value": 2 } } },
  { "op": "remove_item", "resource": "guard.utc", "item": "/ClassList[0]" },
  { "op": "set_resource", "resource": "on_spawn.nss",
    "text": "void main() { }" },
  { "op": "remove_resource", "resource": "old_guard.utc" }
] }
```

- Values and list items are written as `mg gff` writes them (the JSON of
  neverwinter.nim and nasher).
- `set_resource` takes `"text"`, `"base64"` (any bytes) or `"gff"` (a
  whole GFF as `mg gff` prints it).
- In a path, a label's `/`, `[`, `]` and `~` are written `~1`, `~2`, `~3`
  and `~0`, and an empty label `~e`.
- Edits apply in order, each to what the last left. If one does not
  apply (no such resource, a path that leads nowhere, no such item),
  nothing is changed and the error says which.
- The edits are applied as given: values that Moonglow's editors keep in
  step (an item's cost, say) are not recomputed.

## Several areas at once

`mg areas` does from the command line what **Edit › Edit Areas Together…**
does in the window (see [Areas](04-areas.md)). Without a change, it lists
the areas chosen: ResRef, name, tileset and kind.

```text
mg areas mymodule.mod --underground
mg areas mymodule.mod --underground --set MusicDay=57 --set MusicNight=57 --dry-run
mg areas mymodule.mod --tileset tdc01 --set SunFogAmount=8 --set SunFogColor=0x201810
mg areas mymodule.mod cave1 cave2 --var nSpawnLevel=4 --remove-var bOld --set Natural=yes
mg areas mymodule.mod --all --static-placeables --dry-run
```

- **`--static-placeables`**: sets Static on each area's placeables that
  lose nothing by it (not the Useable ones, those a visual transform
  changes, nor those with a script, conversation, trap, inventory or
  animation switched on); see [Areas](04-areas.md).
  **`--dynamic-placeables`** clears Static on every placeable.

- **`--set FIELD=VALUE`**: a field of the area's `.are` by its label
  (`SunAmbientColor`, `MoonFogAmount`, `ChanceRain`, `OnEnter`,
  `LoadScreenID`, `NoRest`…), a field of its ambient sounds and music
  (`MusicDay`, `MusicNight`, `MusicBattle`, `MusicDelay`, `AmbientSndDay`,
  `AmbientSndNight`, `AmbientSndDayVol`, `AmbientSndNitVol`, `EnvAudio`),
  or a flag (`Interior`, `Underground`, `Natural`: `yes` or `no`; each
  area keeps its other flags). A field keeps its type, and a value that
  doesn't fit it is refused. Numbers may be hexadecimal: colors are
  `0xBBGGRR`. Music and sounds are rows of ambientmusic.2da and
  ambientsound.2da.
- **`--var NAME=VALUE`**: a scripting variable, set on each area, which
  keeps its others. Its type is the one the area's variable has, else what
  the value reads as; `NAME:int=`, `NAME:float=` or `NAME:string=` says
  which. **`--remove-var NAME`** deletes one.
- All the changes are made, or none: one that can't be made to one of the
  areas stops the command before anything is saved. The output lists each
  change (`area`, field, `from -> to`); an area that already has a value
  isn't listed for it.
- Names, lighting schemes (which also pick each tile's lights) and list
  fields aren't set by this command.

## JSON output

With `--json`, every command prints one JSON object on standard output
and nothing on standard error, for scripts, build pipelines and tools
(including an AI assistant's):
- **The result:** a command's result as fields: `mg find` a `found` list
  (each with `kind`, `type`, `resref`, `tag`, `name`, and for placed
  objects `area`, `index` and `position`); `mg verify` its `errors`,
  `warnings`, `missing`, `findings` and `unused`; `mg refs` its `uses`
  and `script_strings`; `mg compile` its `scripts`, `failed` and
  `errors` (each with `script`, `line` and `message`); and so on.
- **Notes:** warnings and summaries, otherwise sent to standard error,
  are in `notes`.
- **Errors:** a command that fails prints `{"error": "…"}` and exits with
  an error. `mg verify` prints its result and exits with an error when it
  found errors.
- **What doesn't change:** `mg gff` prints the GFF's JSON as it does
  without `--json`. `mg cat` gives the resource's `text`, or its bytes as
  `hex`. `mg lsp` speaks JSON already, and refuses `--json`.

```text
mg --json verify mymodule.mod | jq '.findings[] | select(.severity == "error")'
```

## nasher projects

| Command | Does |
| --- | --- |
| `mg init MODULE DIR` | put a module into a nasher project in `DIR`: its resources as source files, with a `nasher.cfg` like `nasher init` writes |
| `mg build DIR` | compile the project's scripts and pack the module, as `nasher pack` does (`--target`, `-o OUT`) |

`mg build` fails if a script doesn't compile, which suits a build pipeline:
- `--keep-going` packs the module anyway.
- `--target` picks another target from `nasher.cfg`.
- `-o` writes somewhere other than the target's file.

It needs the game for `nwscript.nss` and the includes (`--root` or
`NWN_ROOT`).

## NWScript in other editors

`mg lsp` is an NWScript language server. It gives VS Code, Neovim, Emacs,
Helix and other editors that speak the Language Server Protocol the same
help as Moonglow's script editor:
- errors as you type, from the game's own compiler;
- go to definition, find references and rename (across the workspace);
- hover (a declaration and its comment);
- completion;
- each script's outline.

It finds scripts in this order:
1. the files open in the editor;
2. the `.nss` files anywhere in the folder you open (a nasher project's
   `src` included);
3. the game's scripts.

It finds the game as `mg` does (`--root`, `NWN_ROOT` or Steam's). Without
the game it still works, but without `nwscript.nss` or the game's
includes. Definitions in game scripts open read-only copies kept in your
cache folder.

To use it, tell your editor to run `mg lsp` for `.nss` files:
- **Neovim** (0.11):

  ```lua
  vim.lsp.config('nwscript', { cmd = { 'mg', 'lsp' }, filetypes = { 'nwscript' }, root_markers = { 'nasher.cfg', '.git' } })
  vim.filetype.add({ extension = { nss = 'nwscript' } })
  vim.lsp.enable('nwscript')
  ```

- **Helix** (`languages.toml`):

  ```toml
  [language-server.mg]
  command = "mg"
  args = ["lsp"]

  [[language]]
  name = "nwscript"
  scope = "source.nwscript"
  file-types = ["nss"]
  language-servers = ["mg"]
  ```

- **VS Code**: in an extension that starts a language server for a file
  type, such as a generic "LSP client" extension, set the command for
  `.nss` to `mg lsp`.

