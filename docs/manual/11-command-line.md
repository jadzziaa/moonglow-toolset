---
type: Manual Page
title: Command-line tools
description: The mg command-line tool - archives and files, the game's resources, modules, setting fields and edit files, several areas at once, JSON output, nasher projects and NWScript in other editors.
tags: [manual, command-line, mg]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-28T11:11:23Z }
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

## Archives and files

| Command | Does |
| --- | --- |
| `mg tileset-palette SET [-o OUT]` | make a tileset's palette (`<tileset>palstd.itp`) from its `.set`: groups and features (in a folder a terrain), terrains and crossers |
| `mg ls ARCHIVE` | list a `.mod`, `.hak`, `.erf`, `.nwm` or `.sav` |
| `mg unpack ARCHIVE OUT` | unpack it into folder `OUT` |
| `mg pack DIR ARCHIVE` | pack a folder into an archive (type from the extension); warns when files would start past 2 GiB, where the game stops reading. `--compile-models` compiles the models kept as text (ASCII `.mdl`) on the way in, each against its supermodel from the folder or the game (which it then needs, as the other commands find it); a model that doesn't compile, has errors `mg verify` would name, or whose supermodel isn't found goes in as the text it is, and is named |
| `mg gff INPUT [-o OUTPUT]` | a GFF file to JSON (the format of neverwinter.nim and nasher), or JSON back to GFF |

## The game's resources

| Command | Does |
| --- | --- |
| `mg which NAME.EXT` | where a resource comes from in the game's load order: the layer the game reads it from, then each layer whose copy that one overrides ("hak:low (overridden by hak:top)"); `--module MODULE` puts the module's haks and the module in the load order too |
| `mg cat NAME.EXT` | print a resource to standard output; `--text` prints a compiled model (`.mdl`) as the text it compiles from |
| `mg layers` | the load order's layers and their sizes |
| `mg tlk STRREF…` | talk-table strings by number |
| `mg tlk-export TABLE.tlk OUT` | write a talk table as JSON (as neverwinter.nim's `nwn_tlk` writes it) or CSV, by `OUT`'s extension (`.json`, `.csv`); the CSV has the feminine table's text too, where `TABLEf.tlk` lies beside it. A CSV's StrRefs are a custom table's, as a module's files name its lines (16777216 and up: line 131 is 16777347); `--base` numbers the lines from 0, as the game's own `dialog.tlk`'s are named. `mg tlk-import` reads either |
| `mg tlk-import TABLE.tlk FILE` | read a `.json` or `.csv` file of lines into a talk table (made if it isn't there, as an English table) and save it: the lines the file has are set, the others stay; `--dry-run` says what would change |
| `mg 2da-check FOLDER` | hold your own tables to their descriptions: every `name.2da` under the folder that has a `name.2da.toml` beside it ([what a description says](03-modules.md#a-tables-description): a number's range, a list or flags, read from a script's constants if you like, a row of another table, a cell that must be filled) is checked, and each wrong cell printed with its row and why. It fails if any is, so a build can run it. `mg 2da-check TABLE.2da` checks one. Scripts and tables a description names are found in the folder (for one table, its repository or nasher project; or `--beside FOLDER`), else in the game |
| `mg 2da-diff A.2da B.2da` | what differs between two tables, by row number and column name (not by layout, case or column order): columns and rows added and removed, and each cell that differs. `--edits FILE` writes an edit file for `mg apply` that sets the table in a module to B |
| `mg 2da-diff A.2da B.2da --base BASE.2da -o MERGED.2da` | merge two tables that were both made from BASE: what either changed is taken. A cell both changed, each to something else, is a conflict (two rows added under one number are, where they differ): it fails and writes nothing, unless `--prefer a` or `--prefer b` says whose to take. `--edits FILE` writes the merged table as an edit file |
| `mg tlk-diff A.tlk B.tlk [--base BASE.tlk -o MERGED.tlk]` | the same for talk tables, by line. `--lines FILE.json` writes the lines that differ as a file for `mg tlk-import` |
| `mg tlk-alloc TABLE.tlk --from NAMES.toml` | write a custom talk table from strings kept by name: see [Strings by name](#strings-by-name) |

## Modules

| Command | Does |
| --- | --- |
| `mg info MODULE` | the module's name, tag, entry area, areas, haks, talk table, game version and resources by type |
| `mg find MODULE` | blueprints and objects placed in the areas: `--type utc,utp`, `--tag` (`*` matches any run), `--name` (words it contains), `--resref` (a placed object's blueprint), `--area`, `--placed` or `--blueprints`, and `--where Label=Value` (a field's value, `*` as in tags; `Label` alone: has the field), each as often as needed |
| `mg search MODULE` | blueprints, placed objects (and the items they hold), areas and conversations by what they hold, one kind at a time: `--var NAME` (`*` matches any run; with `--var-type int` and `--value`), `--property TYPE` (with `--subtype`, `--value`, `--param`), `--class`, `--feat`, `--spell`, `--skill` (with `--min-rank N`), `--appearance ROW` (with `--of creature`, `placeable`, `door`, `generic-door`, `waypoint` or `item`), `--script NAME`, or any field by `--field LABEL`, `--int N` or `--text TEXT`. A row is its number or its name in the game's 2DAs and talk table (the module's haks' too): `--property "skill bonus" --subtype hide`. `--type uti,utc` and `--placed` or `--blueprints` narrow it as in `mg find`; an item blueprint found is also found in the inventories of the blueprints that carry it |
| `mg verify MODULE [--unused]` | missing resources and problems in the custom content (and, with `--unused`, unused resources); fails if there are errors, so a build pipeline stops |
| `mg roundtrip MODULE [--keep COPY.mod]` | checks that Moonglow keeps the module as it is: saves a copy (a temporary one; the module is not touched), reads it back and compares every resource byte for byte, then writes every GFF and 2DA anew and compares what it reads back (the same fields and cells, the fields it has no name for too); lists what differs and fails if anything does |
| `mg model-files WHAT FOLDER [--module MODULE] [--all] [--force] [--dry-run]` | writes the files the game draws a model (`plc_a08`) or a blueprint (`chest.utp`; also `.utd`, `.uti`, `.utc`) with into a folder: models, walkmeshes, textures, materials, from the module's haks and wherever else they are; the game's own are left out unless all are (or `--all`); files already in the folder are not written over without `--force` |
| `mg vars MODULE [NAME]` | the variables the project describes (`variables.toml`, see [Modules](03-modules.md#described-variables)): each one's type, what it goes on, its values (a script's constants and a table's rows read) and its help; `--on KIND` those that go on one kind of object (`placeable`, `area`, `module`…), never one the scripts alone set (`runtime`) |
| `mg claims PROJECT [list\|claim\|release\|merged] [AREA…]` | the project's [claims table](03-modules.md#claimed-areas) (`[claims] file` in its `moonglow.toml`): every area of the table and of the module, whose each is (`--json` too); or the areas named claimed or released by your builder code, as Claim This Area and Release Claim do (`--force`: another builder's claim), or marked merged with their Tags as the module has them. It writes the table, a file of the checkout: commit it |
| `mg builder-code PROJECT [CODE]` | your builder code in a module folder or nasher project (its `.moonglow/user.toml`, which version control leaves out): shown, set, or taken out with `--clear`; the ResRefs the window proposes begin with it ([Modules](03-modules.md#your-builder-code)) |
| `mg checks` | the checks `mg verify` makes of custom content: each one's id (a finding's `check` in the JSON), its group and what it holds to be true |
| `mg verify MODULE --skip GROUP` | leave a group out (repeatable, or several separated by commas: `--skip tilesets,models,2das`): `references` (missing resources), `tilesets`, `models`, `2das`, `objects`, `module` or `plugins`. Its checks aren't run (the tilesets and models of a hundred haks are most of the time a Verify takes); the JSON's `skipped` names each group left out, as `null` (not checked), and only the rest count toward failing |
| `mg verify MODULE --timings` | also print how long each part of the check took, to stderr (the report is unchanged): opening the module, the haks, each group's checks, the tilesets', 2DAs', materials' and models' time added up with the longest of each |
| `mg verify MODULE --plugins DIR` | also run the checks of the plugins in `DIR` (a plugin's folder, or a folder of plugins); their problems count with the rest. A project's own plugins (its `plugins/`) run as you allowed them in Moonglow, and each that did not is said in a note |
| `mg verify MODULE --allow-project-plugins` | run the checks of every plugin the project comes with, allowed or not: for a build pipeline, where nobody is asked ([A project's own plugins](15-plugins.md#a-projects-own-plugins)) |
| `mg verify MODULE --fix CHECK` | make the fixes the findings of a check offer (repeatable; a check's id, as a finding's `check` says: `tag-unique`, `example.tag-conventions/tag-case`) and save: each as Verify Module's button makes it, refused (and named) where an earlier fix changed what it would change. The report is of the module as it was found, with a `fixed` line a fix; only errors not fixed fail. `--dry-run` says which it would make and changes nothing. Without `--fix`, nothing is changed |
| `mg plugin list DIR…` | the plugins in the folders and what each adds: its commands, checks, events and settings (each setting's kind, scope and value). `--project PROJECT` adds the project's own plugins, each with what you answered (`allowed`, `not asked`, `changed since allowed`, `denied`), its digest, and its settings' values in the project |
| `mg plugin allow PROJECT [ID…]` | allow a project's own plugins to run, as they are now (no ids: every one of its plugins), as Moonglow asks when the project is opened; `--deny` keeps them off. The answer is kept in Moonglow's own folder (`project-plugins.toml`), by the project's place on disk, the plugin and a digest of its files, never in the project: a changed plugin is asked about again |
| `mg plugin check DIR…` | check plugins: the manifest reads, and the code registers what it declares; fails if not. A file is a plugin's archive: checked as the installer checks it, and its code as it is in the archive |
| `mg plugin test PLUGIN` | run a plugin's own tests (`tests/*.toml` in its folder: a fixture module and what its checks, commands and events are to make of it; [Writing plugins](16-writing-plugins.md#a-plugins-own-tests)): a line a test, `ok` or `FAIL` with what was not as wanted, a `note` under a test that passed without a game resource it read, and a summary; fails if a test fails or there are none. `--module FIXTURE` runs every test on another module. The fixture is never written. A test reads the game's data from the game `mg` finds (`--root`), the one at its own `root` (none with `root = false`), and over either its own folder of game resources (`game/` beside the test file), which is all it reads without a game. A test may run one of Moonglow's own checks on its fixture (`builtin = "tag-unique"`), by the fixture's project file |
| `mg plugin run MODULE PLUGIN COMMAND` | run a plugin's command on a module and save: its edits as one command, all or none. `--answer ID=VALUE` fills a form's field (the others take their defaults), `--file PATH` is the file or folder it gets where it asks for one (in the order it asks), `--yes` agrees where it asks, and to the haks it writes (into `--hak-dir DIR`, else the user folder's `hak`); `--dry-run` prints the edits (a file `mg apply` reads) and changes nothing. With `--event`, COMMAND is the id of what the plugin does by itself, run as the window would run it then ([its events](17-plugin-api.md#ctxevent)): `--changed NAME` names a resource the save changed or writes, or that was made (as often as needed; none named: all of the module's), `--was MODULE` is the module as it was before a save (its resources are the old ones, and those that differ are what the save writes), `--select` names the objects placed or pasted, and `--by` what made them; what a `resource-saving` handler says is in the notes (and the JSON's `said`), and a refusal fails the run. The plugin's settings are what the module's project files hold, else the defaults; `--setting ID=VALUE` gives one for the run. What the plugin sets (`ctx.settings:set`) is kept in the project's files (with `--dry-run`, said and not kept); one of scope `user` is not kept: `mg` keeps none of Moonglow's settings |
| `mg plugin pack PLUGIN` | pack a plugin's folder into an archive to hand around (`ID-VERSION.zip`, or `-o FILE`): checked first, hidden files and its `tests` folder left out |
| `mg plugin install FILE FOLDER` | install a plugin from its archive into a folder of plugins, as Plugins › Install Plugin from File… does: the archive is checked and nothing of it runs. `--replace` installs over the same plugin installed from an archive before |
| `mg plugin remove ID FOLDER` | remove a plugin that was installed from an archive from a folder of plugins, by its id; one put there by hand is left alone |
| `mg nui new MODULE NAME` | add a NUI window to the module and save: its layout (`NAME.jui`, a window with a Close button) and what the [NUI Creator](18-nui-creator.md) keeps about it (`NAME.txt`). NAME is 1 to 14 lower-case letters, digits or underscores (the scripts add `_o` and `_e`); a name whose resources the module has is refused; `--dry-run` lists the resources and adds none. In a nasher project whose sources leave out `.jui` and `.txt` files (`src/**/*.{nss,json}`) it says so, with the `include` line to add (with `--json`: `left_out` and `include`); `validate` and `generate` say it too |
| `mg nui validate MODULE NAME` | check a NUI window as the NUI Creator does (the layout is what the game's NUI functions take, every bind has a value) and print each problem; fails if there are errors, and changes nothing. With `--json`: `errors`, `warnings` and `findings` (each with `severity`, `path` and `message`) |
| `mg nui generate MODULE NAME` | write the window's scripts and save, as Build does in the NUI Creator: the include that opens it (`NAME_o.nss`) and its event script (`NAME_e.nss`, compiled; the code you wrote in it is kept), and the scripts that include the opener compiled again, as `mg compile` compiles them (what was compiled is remembered in the same place). The window's scripts are all or none: a window with errors, an opener or event script edited by hand, or one that does not compile, changes nothing. A script of yours that includes the opener and does not compile is printed and keeps the compiled script it had (with `--json`: `failed` and `errors`, as `mg compile` gives them); `--dry-run` checks and compiles, lists the resources and writes none. To see the window in the game, use Build › Test Module. A window whose events go to the module's own event script (`event_function` in its settings) gets `NAME_e.nss` as an include instead, compiled through a caller, with the scripts that include it; a bare `return;` in a handler's section becomes `return TRUE;` there (and `return TRUE;` or `return FALSE;` becomes `return;` in a script of its own), each change printed with its section and line (with `--json`: `returns`, each with `section`, `line`, `was` and `now`) |
| `mg nui render MODULE NAME` | draw a NUI window into a PNG as the NUI Creator's preview draws it: the game's skin, pictures and fonts from the install and the module's haks, nothing selected. `-o FILE` (default `NAME.png`), `--scale S` the UI scale (1 is 100%, 0.25 to 4; the picture is S pixels a point), `--size WxH` the window's size in points in place of its geometry's. `--state STATE[:ID]` draws a control (by its ID; every row's, in a list) or, without an ID, every control in a state: `normal`, `hover`, `pressed` or `disabled`, as the Creator's State menu shows the selected one; again for more (`--state hover:send --state disabled:cancel`), an ID's state winning over every control's; a control disabled by its ID disables what is in it, and one its binds disable stays disabled. `--binds FILE.json` sets binds as a script would with `NuiSetBind`, in place of their first values: one JSON object of names and values (a string, number, boolean, an array for a list's rows, `{"r","g","b","a"}` for a color, `{"x","y","w","h"}` for a rectangle), so that a list is drawn with the rows a character would see; a bind no property reads, or given another kind of value than its first, is said. `--variant NAME` shows a swap layout variant (a key of the settings' `views`) in its group, or as the whole window, in place of what it holds at first, as `NuiSetGroupLayout` would. An ID no control has, a variant the window doesn't have or a state given twice is refused. MODULE may instead be a window's `NAME.jui` file alone, with the `NAME.txt` and the pictures it names beside it. Needs a GPU; without the game it draws without the skin and says so. It is the preview, not the game: spacing and events are seen in Test Module. Changes nothing |
| `mg nui layout MODULE NAME` | where the preview puts each control of a NUI window, by the same layout `render` draws: each element's JSON pointer, `kind`, `id`, the binds it reads, the text it shows (a text field's value, or its placeholder while it is empty), and `x`, `y`, `w`, `h` in the window's points (its top left corner, title bar included, at 0, 0), and whether it is `hidden`, `zero_sized`, `clipped`, `out_of_view`, `overlaps` a neighbor or cuts its text (`cut_text`; for text that runs past the bottom of a box that doesn't scroll, also `needs_height`, the height in points that shows it all), and its `state` when it isn't normal (`hover`, `pressed`, `disabled`); with the window's `width`, `height` and `body`, and the pictures and fonts not found and what is said of the binds given (`issues`). `--scale`, `--size`, `--state`, `--binds`, `--variant` and a `.jui` file as for `render`, so that its rectangles are the picture's (with a variant, paths are of the window as shown). A line a control without `--json`. Needs no GPU |
| `mg nui schema` | print the form of a window's settings file (`NAME.txt`, `moonglow.nui/1`) as a JSON Schema: each key, its type, default and use, and each handler's form. `--layout`: a new window's layout and each control as the Creator adds it, without a width (its description says what that makes of them: most controls keep 150 points, labels, lists and layouts take their column's width, and a column is as wide as its widest control, not as the window) |
| `mg nui import MODULE SOURCE…` | bring NUI windows into the module and save: each window's layout, its settings (when they are the Creator's) and its event script's source (`NAME_e.nss`), from `.jui` files (with what is beside them), folders, ERF archives (`.erf`, `.hak`, `.mod`) or zip files. The opener and compiled scripts are not brought: `generate` writes them. A window the module has differently is refused but with `--replace`; `--dry-run` lists the resources. Each window is checked as `validate` checks it |
| `mg haks MODULE` | what the module's haks provide, where they conflict and which game resources they override |
| `mg haks MODULE --usage` | which of each hak's resources the module reaches (areas' tilesets and tiles, objects' and blueprints' looks and sounds, scripts and what their strings name, every model's textures and materials) and which nothing does: a line a hak (used of all, bytes used of all; unused, shadowed, not judged); with `--json` each resource used with what reached it first ([Modules](03-modules.md#what-the-module-uses-of-its-haks)) |
| `mg minimap MODULE AREA OUT.png [--size PX]` | an area's minimap as a PNG, laid out as the game's map draws it (`PX` pixels a tile) |
| `mg nwsync MODULE REPOSITORY` | publish the module's haks and talk table for NWSync into a repository folder, as `nwn_nwsync_write` does: `--with-module` (with `--name`, `--description`, `--uuid`), `--group-id`, `--no-latest`, `--limit-file-size MB`, `--force`, `--dry-run`; prints the manifest's hash |
| `mg attach MODULE FILE…` | copy haks and a talk table (from anywhere) into the user folder's `hak` and `tlk`, list the haks at the top of the module's hak list in the order given, name the talk table, and save (`--replace` replaces different files of the same names there) |
| `mg serve MODULE` | run the module in the game's headless server and print its log and console: see [Running a module in the server](#running-a-module-in-the-server) |
| `mg test MODULE [--filter TEXT]` | run the module's `test_*` scripts in the server and report what they assert: see [Testing scripts in the server](#testing-scripts-in-the-server) |
| `mg watch PROJECT --dev-dir FOLDER` | follow a module kept as files while you edit it, putting changed scripts (compiled) and blueprints into a development folder: see [Following a project as you edit](#following-a-project-as-you-edit) |
| `mg compile MODULE [--uncompiled] [--all] [--ndb]` | compile the module's scripts and save it: those changed since they were last compiled, by `mg compile` or by Compile All Scripts in the window (both remember it in Moonglow's data folder, not in the module); `--all` compiles every script, `--uncompiled` only those without a compiled version (remembered as compiled too); `--ndb` makes each one's debug file (`.ndb`) too, and compiles again a script that has none |
| `mg export MODULE NAME.EXT… -o OUT.erf` | export resources with what they use (`--keep-factions`, `--comment`); the archive's description says what it was made from (git commit, Moonglow's version, the module) |
| `mg export MODULE NAME.EXT… --files -o FOLDER` | write the resources named as loose files in FOLDER (a script with its compiled `.ncs` and `.ndb`, an area with its `.git` and `.gic`): for the game's `development` or `override`, or a server's |
| `mg import MODULE ERF [--overwrite]` | import an archive into the module and save it; says what a Moonglow export was made from, and warns when the module's git checkout is at another commit now |
| `mg refs MODULE NAME.EXT` | where a resource is used, and script strings that spell it (`--tag` for a tag) |
| `mg rename MODULE NAME.EXT NEW` | rename a script, area, conversation or blueprint everywhere and save (`--strings` changes script strings too) |
| `mg update-instances MODULE [NAME.EXT…]` | remake placed objects from their blueprints (all of the module's if none are named) and save (`--area AREA` for one area) |
| `mg dialog-export MODULE NAME.dlg OUT` | write a conversation as plain text, CSV, Twine or Ink, by `OUT`'s extension (`.txt`, `.csv`, `.twee`, `.ink`) |
| `mg dialog-import MODULE FILE [--name NAME]` | read a Twine or Ink story into the module as a conversation (replacing one of that name), or a CSV export's text back into its conversation, and save |
| `mg replace MODULE FIND WITH` | replace text in the module's names, descriptions, conversations and journal and save (`--match-case`, `--whole-word`, `--only names,journal,…`, `--dry-run` to list the strings only) |
| `mg set MODULE NAME.EXT FIELD=VALUE…` | set fields of a resource and save: a field is a path from the resource's root (`Tag`, `/ClassList[0]/ClassLevel`) and keeps its type; `--remove FIELD`; `--dry-run` prints the change and makes none. `--plugins DIR` (repeatable) and the project's own plugins as you allowed them look at what is saved, as before a save in the window: a refusal is an error and nothing is written, a warning is a note ([what plugins do by themselves](15-plugins.md#what-plugins-do-by-themselves)) |
| `mg apply MODULE EDITS.json` | apply a file of edits (below) and save: all of them, or none if one does not apply; `-` reads standard input; `--dry-run` only checks; `--plugins DIR` as for `mg set` |
| `mg areas MODULE [AREA…]` | list the module's areas, narrowed by `--match TEXT` (every word of it in the name, tag or ResRef), `--tileset`, `--interior` or `--exterior`, `--underground` or `--above-ground`, `--natural` or `--artificial`; with `--set FIELD=VALUE`, `--var NAME=VALUE`, `--remove-var NAME`, `--static-placeables` or `--dynamic-placeables`, `--grass MATERIAL:TEXTURE:DENSITY:HEIGHT[:R,G,B[:R,G,B]]`, `--remove-grass MATERIAL`, `--tileset-grass yes|no` or `--lighting-scheme SCHEME`, make each change to each of those areas and save (`--dry-run` to list the changes only; `--all` to change every area) |

A module is a `.mod` archive, a module folder or a nasher project. `mg
compile` and `mg import` write the module in place; an archive's previous
version is kept beside it (`mymodule.mod.bak`). A `.mod` is read and
written as the `.mod`, as the game's server reads it, even with a
folder of its name beside it (which the window asks about, its name in
any case): the command says so in a note (in the JSON's `notes` too),
and that the folder is left as it is. Name the folder to work on the
folder. A command that saves a module folder leaves what another
program changed in it meanwhile as it is, as the window does.

For example, every placed creature whose tag starts `GUARD` and that has
no OnSpawn script set, in the area `keep`:

```text
mg find mymodule.mod --type utc --placed --area keep --tag 'GUARD*' --where ScriptSpawn=
```

And every item, in a blueprint, placed or carried, with a variable whose
name has `sDeidad` in it, then every one giving a bonus to Hide:

```text
mg search mymodule.mod --var '*sDeidad*' --type uti
mg search mymodule.mod --property 'skill bonus' --subtype hide
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
mg areas mymodule.mod --tileset tin01 --grass Wood:grass:2:0.4 --tileset-grass no
mg areas mymodule.mod --interior --lighting-scheme InteriorDark --set SunFogAmount=4
```

- **`--lighting-scheme SCHEME`**: a lighting scheme, as Area Properties'
  Lighting Scheme gives it (an `environment.2da` row, by number, label
  such as `InteriorDark`, or the name the list shows, such as `Interior
  Dark`): the area's sun and moon colors, fog, shadows, day and night and
  weather, and each tile's lights, picked at random from the scheme's
  colors as the toolsets pick them. It reads the game's `environment.2da`
  (the module's haks' over it), so it needs the game installed (`--root`).
  It is made before `--set`, which can change what it set. Each area is
  listed with its `LightingScheme` and how many tiles' lights changed.
  The tiles' lights are picked with a number the command prints (a
  `--dry-run` too; `seed` in the JSON): **`--seed NUMBER`** picks them
  so again, so a dry run's are a run's with its number, and a run can be
  made again the same.

- **`--grass MATERIAL:TEXTURE:DENSITY:HEIGHT[:R,G,B[:R,G,B]]`**: grass of
  each area's own on a surface material (a `surfacemat.2da` row, by
  number or label), in place of what grows there, as the game's
  `SetAreaGrassOverride` sets it: its texture, density (tufts to a square
  meter), height (meters), ambient color and diffuse color (red, green
  and blue, 0 to 1; the diffuse the ambient's when it is left out, both
  the game's grass color, 0.5,0.6,0.3, when neither is). One an area has
  for the material already changes in place. **`--remove-grass
  MATERIAL`** takes it off; **`--tileset-grass no`** turns the tileset's
  grass off (`yes` on). Kept in the area's GIT, where the game reads it;
  see [Areas](04-areas.md#grass).

- **`--static-placeables`**: sets Static on each area's placeables that
  lose nothing by it (not the Useable ones, those a visual transform
  changes, those with a script, conversation, trap, inventory or
  animation switched on, those with variables, nor those the project
  file keeps dynamic); see [Areas](04-areas.md). Each placeable left
  dynamic is named with why (a note, and `left_dynamic` in the JSON).
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
  keeps its others. Its type is the one the area's variable has, else the
  one the project's `variables.toml` describes it with, else what the
  value reads as; `NAME:int=`, `NAME:float=` or `NAME:string=` says
  which (a type given against the description is said, and a variable
  described for other objects than areas). **`--remove-var NAME`**
  deletes one.
- All the changes are made, or none: one that can't be made to one of the
  areas stops the command before anything is saved. The output lists each
  change (`area`, field, `from -> to`); an area that already has a value
  isn't listed for it.
- Names, tags and comments, which are each area's own, aren't set by
  this command (`mg replace` changes text in names), nor list fields one
  by one (a tile's own lights; variables are `--var`'s).

## Running a module in the server

`mg serve MODULE` starts `nwserver` (Linux and Windows: the game has none
for macOS) on a `.mod`, a module folder or a nasher project. A folder or
a project is packed first: the server loads an archive only. Its
scripts that have no compiled script, or changed since they were last
compiled (by `mg compile` or the window), are compiled for the packed
copy, as `mg build` would (the server runs compiled scripts only), and
one that does not compile is said; the sources are left as they are. A
project's own talk table, kept as JSON, is made into the server's `tlk`
folder (in the game's language, where the project keeps several), in
place of the one in your tlk folder.

- **Private unless asked.** The server is not listed (`--public` lists
  it) and gets random player, DM and admin passwords, which are printed;
  `--player-password`, `--dm-password` and `--admin-password` set them.
  It still looks up Beamdog's master server when it starts, listed or
  not.
- **Its own user directory.** The server's vault, database, settings and
  logs go in a scratch folder in the system's temporary folder, the same
  one for each run of a module (a second server of the module started
  while the first runs gets one of its own), with the module's haks and talk table
  linked in from your NWN folder (which is only read). `--dir FOLDER`
  names another; your own NWN folder is refused without
  `--real-user-dir`.
- **What it prints.** Where the server runs (the port, the folders, the
  two log files), then the server's console, its log (what scripts write
  with `WriteTimestampedLogEntry`) and the errors and warnings of the
  engine's own log (a script that failed). "module loaded" comes from
  the server saying so, not from its port, which answers before a module
  is loaded. `--marker TEXT` marks the log lines with that text.
- **What you type** goes to the server's console (`status`, `say`,
  `exit`, and the commands NWNX adds). `/restart` starts the server
  again, and `/quit` stops it. So does Ctrl+C: the server is told to
  `exit` (it closes its database and logs in order) rather than ending
  with `mg`.
- **Until.** `--until TEXT` stops the server at the first log line with
  that text and `--timeout SECONDS` after that long: a failure if
  `--until` was not seen. `--restart-on-change` starts the server again
  when the module's file (or folder) changes.
- **NWNX and the like.** `--env FILE` adds a file of `NAME=value` lines
  to the server's environment and `--preload LIBRARY` loads a library
  into it (`LD_PRELOAD`, on Linux); Moonglow knows nothing more of NWNX.
  Arguments after `--` go to `nwserver` as they are (`-- -maxclients 4
  -servervault 1`).
- **`--json`** prints each of these as a line of JSON (`event`:
  `started`, `passwords`, `ready`, `log`, `marker`, `console`, `engine`,
  `note`, `changed`, `ended`, the last with `how` it ended and whether
  it `failed`, as `mg watch --serve` ends too).

## Testing scripts in the server

`mg test MODULE` runs the module's scripts named `test_*` (each with a
`main()`) in the server and reports what they assert. A test includes
`mg_test`, which Moonglow provides (`mg test --include` prints it; a
module with its own `mg_test.nss` keeps it):

```c
#include "mg_test"
void main()
{
    Describe("fighter");
    AssertEqualInt(GetHitDice(OBJECT_SELF), 0, "the module has no level");
    Assert(GetIsObjectValid(GetFirstArea()), "an area", "the module has none");
}
```

| Function | |
| --- | --- |
| `Assert(bCondition, sName, sDetail = "")` | passes if the condition holds |
| `AssertEqualInt`, `AssertEqualString`, `AssertEqualFloat(…, fWithin)` | pass if the two are equal; a failure says what was got and expected |
| `AssertValid(oObject, sName)` | passes if the object is valid |
| `Pass(sName)`, `Fail(sName, sDetail = "")` | a result of your own |
| `Describe(sGroup)` | a name the following results are grouped under |

- **Your module is not changed.** The tests run in a copy in a scratch
  folder: there a driver becomes the module's OnModuleLoad script. The
  tests run your scripts as they are: the copy gets each script
  compiled that has no compiled script or changed since it was last
  compiled (by `mg compile` or the window: what they remember), and the
  run names those whose compiled script in your module is not what their
  source compiles to now: changed since, or made by another compiler
  (`mg compile` compiles them there). One that does not
  compile now runs with the compiled script it has, and the run says so;
  one with none stops the run, as a test that does not compile does. It
  starts each test as an event of its own, half a second after the
  module's own OnModuleLoad script, which it runs too. A test that runs
  out of instructions, or isn't there, is an error and stops no other.
  So is one that stops inside its `main()` (a division by nothing, a
  function the server has not got, as an NWNX one with NWNX not loaded):
  in the copy each test says when its `main()` has returned, and one
  that never says so did not run to its end, whatever it had asserted.
  A test that ran to its end and asserted nothing passes, and is said to
  have made no assertion.
- `--filter TEXT` runs the tests with that text in their name (`*` for
  any run of characters). `--tests FOLDER` adds a folder's scripts to
  the copy. `--settle SECONDS` waits that long after the last test for
  what tests assert later (`DelayCommand`). `--timeout SECONDS` (120)
  gives up.
- The result is a line a test, the failures with what was got and
  expected, and a failing exit if a test failed or did not run to its
  end. `--json` gives the same as fields (`tests`, each with its
  `cases`), `--junit FILE` writes a JUnit report, and `--log` prints the
  server's log meanwhile.
- `--driver none` puts nothing into the copy: the module runs its tests
  itself (from NWNX's start script, say) and calls `TestsDone()`.
- `--dir`, `--env`, `--preload` and the arguments after `--` are `mg
  serve`'s.

What the include writes to the log is its format 1 (`MG_TEST_FORMAT`):
`MG_TEST PASS name` and `MG_TEST FAIL name :: detail`.

## Following a project as you edit

`mg watch PROJECT` follows a module kept as files (a nasher project or a
module folder) and takes up each change as you save it:

| What changed | What happens |
| --- | --- |
| A script | it and the scripts that include it are compiled, and the compiled scripts are put into the development folder |
| A blueprint (`ut*`) or a conversation | it is put into the development folder as it is |
| Anything else (a 2DA, an area, `module.ifo`), or a file taken away | the module is built again; with `--serve`, the server is started again on it |
| The project's own talk table (its `<name>.tlk.json` or `.csv`) | the same: the server, started again, has the table made anew of it |

A running server takes up what is in its `development` folder at the next
use (the next time the script runs, the next creature made from the
blueprint), without a restart; 2DAs, areas and the module's own settings
it reads only at its start.

- `--dev-dir FOLDER` is the development folder to write into. Moonglow
  keeps a list there (`.mg-watch`) of what it wrote, and takes those
  files away again when the module is built anew (the module has them
  then); files of yours in the folder are left alone.
- `--mod FILE` builds the module to a file, at the start and after every
  change, with every script compiled.
- `--ndb` makes each compiled script's debug file (`.ndb`) too, for the
  game's script debugger: put out beside the compiled script, and built
  into the module.
- `--serve` also runs the module in the server, as `mg serve` does, on a
  scratch user directory whose `development` folder is the one written
  into: `--dir`, `--port`, `--env`, `--preload` and the arguments after
  `--` are `mg serve`'s, and so is what you type. Started again, the
  server keeps its port and its passwords.
- A script that does not compile is said, with its line, and nothing is
  put out for it: the server keeps the one it has.
- `--json` prints each of these as a line of JSON (`watching`, `synced`,
  `rebuilt`, and with `--serve` the server's events).

## Strings by name

A project that makes its rules from data needs talk-table numbers that
never change: a number saved in a character or an item must not come to
mean another text. `mg tlk-alloc TABLE.tlk --from NAMES.toml` writes a
custom talk table from strings kept by name:

```toml
[feat.cleave]
name = "Great Cleave"
desc = "A follow-up attack after a kill."

[voice]
greeting = { text = "Well met.", sound = "vs_greet", sound_length = 1.2 }
```

- A key is its groups and its name with dots (`feat.cleave.name`). The
  names file is `.toml` or `.json`.
- **The map** (`NAMES.strrefs.json`, or `--map FILE`) says which line of
  the table each key has. Keep it with the project. A new key gets the
  next line that was never anyone's (from `--start`, passing over lines
  of the table that have someone else's text); a key you take away keeps
  its line reserved, and gets it back if you name it again.
- `--constants FILE.nss` writes the keys as constants with the game's
  numbers (`const int STRREF_FEAT_CLEAVE_NAME = 16777217;`; `--prefix`
  changes `STRREF_`), `--constants FILE.json` as JSON.
- `--check` writes nothing and fails unless the table, the map and the
  constants are as they would be written: for a build to refuse a names
  file that was changed and not run. `--against OLDER.json` fails if a
  key has another line than in an older map (the main branch's).

## JSON output

With `--json`, every command prints one JSON object on standard output
and nothing on standard error, for scripts, build pipelines and tools
(including an AI assistant's):
- **The result:** a command's result as fields: `mg find` a `found` list
  (each with `kind`, `type`, `resref`, `tag`, `name`, and for placed
  objects `area`, `index` and `position`); `mg verify` its `errors`,
  `warnings`, `missing`, `findings` and `unused` (a finding's `fix` is
  null, or the fix it offers: its `label` and its `command`, an edit
  file `mg apply` takes; with `--fix`, `fixes` lists those `made` and
  those `refused`); `mg plugin test` its `tests` (each `file`, `name`,
  `passed`, `problems` and `notes`), `passed`, `failed` and `noted`
  (how many passed with notes); `mg refs` its `uses`
  and `script_strings`; `mg compile` its `scripts` (how many were compiled), `unchanged` (how many were passed
  over), `failed` and `errors` (each with `script`, `line` and `message`); and so on.
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
| `mg build DIR` | compile the project's scripts and pack the module, as `nasher pack` does (`--target`, `-o OUT`): those changed since they were last compiled, as `mg compile` does (a project keeps no compiled scripts, so those are all of them unless it keeps some among its sources); `--ndb` makes each script's debug file (`.ndb`) too, packed with it. A `--target` that packs a talk table (a `.tlk`) makes it of its `<name>.tlk.json` as nasher has `nwn_tlk` make it, byte for byte |
| `mg merge-gff BASE OURS THEIRS` | merge two copies of a GFF resource (an area, `module.ifo`, a blueprint), both made from BASE, as a GFF or as nasher's JSON of one, and write the merge over OURS (`-o FILE` elsewhere). Placed objects merge by object, the area list as a union, variables by name and type. Two different changes to one value are a conflict: it fails, names each by its path and writes nothing, unless `--prefer ours` or `--prefer theirs`. Git runs it as its merge driver: see [Merging in git](03-modules.md#merging-in-git) |

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

It reads scripts in the codepage the game reads their text in: the one
your user folder's `settings.tml` gives the game (its codepage, else its
language's: Windows-1250 for Polish), else Windows-1252. `mg lsp
--codepage cp1250` names it. The files open in your editor come as the
editor read them: set it to the same codepage for `.nss` files.

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

