---
type: Manual Page
title: Command-line tools
description: The mg command-line tool - archives and files, the game's resources, modules, setting fields and edit files, several areas at once, JSON output, nasher projects and NWScript in other editors.
tags: [manual, command-line, mg]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-10T15:46:23Z }
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
mg [--root GAME] [--user-dir DIR | --no-user-dir] [--userpatch] [--workshop] [--no-aliases] [--json] COMMAND ...
```

`--root` is the game's folder (default: `$NWN_ROOT`, else Steam's);
`--user-dir` the user folder (default: `$NWN_HOME`, else the platform's);
`--no-user-dir` reads the game alone; `--userpatch` reads the haks the
user folder's `userpatch.ini` names and `--workshop` the Steam Workshop
items you are subscribed to, as the game does and as Options › General's
switches have the window read them (both off unless given: see
[Options](10-options.md)); `--no-aliases` leaves the user folder's
`nwn.ini` folder aliases unfollowed, its own `hak/`, `tlk/` and the rest
read, as Options › Folders' switch does; `--json` prints the result as
JSON (below).
`mg COMMAND --help` describes each command.

## Archives and files

| Command | Does |
| --- | --- |
| `mg tileset-palette SET [-o OUT]` | make a tileset's palette (`<tileset>palstd.itp`) from its `.set`: groups and features (in a folder a terrain), terrains and crossers |
| `mg ls ARCHIVE` | list a `.mod`, `.hak`, `.erf`, `.nwm` or `.sav`: each resource's size and name (`--json`: `resources`, each with `name` and `size`); `--sha256` adds each one's SHA-256 (`sha256`), which reads the archive whole (a 1.5 GB hak takes about four seconds) |
| `mg unpack ARCHIVE OUT` | unpack it into folder `OUT` |
| `mg pack DIR ARCHIVE` | pack a folder into an archive (type from the extension), which takes the place of one that was there only once it is whole; warns when files would start past 2 GiB, where the game stops reading. `--compile-models` compiles the models kept as text (ASCII `.mdl`) on the way in, each against its supermodel from the folder or the game (which it then needs, as the other commands find it); a model that doesn't compile, has errors `mg verify` would name, or whose supermodel isn't found goes in as the text it is, and is named |
| `mg model compile IN.mdl OUT.mdl` | compile one model kept as text into the binary model the game loads, as the game's own compiler makes it: its supermodel (whose part numbers its nodes take) and the materials (`.mtr`) its meshes name are looked for beside it, then in the game. The compiler's notes are printed (what it left out or made up, by line), among them a node with no `parent` line, which the game leaves out of the model with the nodes under it |
| `mg model decompile IN.mdl OUT.mdl` | write a binary model as the text it compiles from, as `mg cat --text` does a game resource's. A body part's skin compiled against the creature's skeleton it is worn on (Beamdog's HD bodies) has its bones named from that skeleton (`pmh0` for `pmh0_neck001`), found beside it or in the game; without it the bones are left unnamed and the text says so |
| `mg model info MODEL` | a model's facts, from its file (text or compiled) or by its name from the load order (`plc_a01`; `--module` and `--hak` as for `mg cat`): its nodes by kind, its faces (`faces`, every mesh's triangles; `faces_rendered`, those of the meshes the game draws: `render` on, not a walkmesh's `aabb`) and vertices, the box its drawn meshes fill (`bounds`, `min` and `max` in the model's space at rest: each node placed by its parents' positions, orientations and scales, a skin at its bind pose as its vertices lie; emitters' particles, lights, walkmeshes and meshes not drawn left out; `null` for a model that draws no mesh), its skins with the bones their weights move and their triangles (`skins`, each `node`, `bones` and `faces`; `bones`, those of all of them, each once), the textures it names (its meshes', emitters' and flares'), the materials, its supermodel (`null` for none) and its own animations, each with `length`, `transtime` and how many `events` (those its supermodel gives it are not listed). `--json` for scripts; the facts are the same for a model's text and the model compiled from it |
| `mg gff INPUT [-o OUTPUT] [--module PATH] [--names]` | a GFF file to JSON (the format of neverwinter.nim and nasher, written as `nwn_gff -p` writes it: its notation of floats, `0.0000002499999993688107` rather than `2.499999993688107e-7`, and its string escapes, the fields in the file's order), or JSON back to GFF; its text in Windows-1252, or by the `encoding.2da` table of the module given (in its haks). `--names` names what a field's number stands for, as the window's fields view does (read from the game's 2DAs): `"row": "Human"` beside a race's `"value"`, an item property's subtype and value by the tables its other fields name, a conversation line's animation, a talk-table string, a faction by the module's own (with `--module`), and `"__row"` on each of a creature's skills and a store's pages; JSON with them reads back as without |

## The game's resources

| Command | Does |
| --- | --- |
| `mg which NAME.EXT` | where a resource comes from in the game's load order: the layer the game reads it from, then each layer whose copy that one overrides ("hak:low (overridden by hak:top)"); `--module MODULE` puts the module's haks and the module in the load order too; `--hak PATH` adds your own ([below](#haks-and-folders-of-your-own)) |
| `mg cat NAME.EXT` | print a resource to standard output; `--text` prints a compiled model (`.mdl`) as the text it compiles from; `--module` and `--hak` as for `mg which` |
| `mg cat NAME.EXT… --to FOLDER` | write many resources into a folder in one run, each by its name, from where the game reads it (every archive opened once: thousands take a second or two, where a run of `mg` a resource takes about a tenth of a second each); more names from `--list FILE`, a line each (`-`: standard input; blank lines and lines that begin with `#` passed over), each name once. Each is printed with its layer, size and SHA-256 (`--json`: `resources`, each with `resource`, `found`, `layer`, `file`, `size` and `sha256`, or `found: false` and `why`; and `missing`). A name found nowhere, or no resource's name, is said, and the run fails once the rest are written. `--dry-run` says the same and writes nothing; files of those names in the folder are written over only with `--force`; `--text` writes compiled models as text |
| `mg layers` | the load order's layers and their sizes |
| `mg tlk STRREF…` | talk-table strings by number |
| `mg tlk-export TABLE.tlk OUT [--module MODULE]` | write a talk table as JSON (as neverwinter.nim's `nwn_tlk` writes it) or CSV, by `OUT`'s extension (`.json`, `.csv`); the CSV has the feminine table's text too, where `TABLEf.tlk` lies beside it. The JSON, as `nwn_tlk`'s, is one table's: export `TABLEf.tlk` on its own for the feminine one. A CSV's StrRefs are a custom table's, as a module's files name its lines (16777216 and up: line 131 is 16777347); `--base` numbers the lines from 0, as the game's own `dialog.tlk`'s are named. `--module` reads the text by the module's `encoding.2da` (in its haks), where it has one. A sound length that is no number (infinite, in a damaged table or another tool's) is written `"soundLen":"inf"` in the JSON, as `nwn_tlk` writes it, and `inf` in the CSV. `mg tlk-import` reads either |
| `mg tlk-import TABLE.tlk FILE [--language CODE] [--module MODULE]` | read a `.json` or `.csv` file of lines into a talk table (made if it isn't there) and save it: the lines the file has are set, the others stay; `--dry-run` says what would change. A table of which no line changes is not written again (with `--json`, `saved` says whether it was). A table it makes is in `--language` (`en`, `fr`, `de`, `it`, `es`, `pl`), else the JSON's own `language`, else English, its text written in that language's codepage (Polish's Windows-1250); `--module` writes it by the module's `encoding.2da` (in its haks), where it has one |
| `mg 2da-check FOLDER` | hold your own tables to their descriptions: every `name.2da` under the folder that has a `name.2da.toml` beside it ([what a description says](03-modules.md#a-tables-description): a number's range, a list or flags, read from a script's constants if you like, a row of another table, a cell that must be filled) is checked, and each wrong cell printed with its row and why. It fails if any is, so a build can run it. `mg 2da-check TABLE.2da` checks one (by its own description, its family's, or for a `spells.2da` or `baseitems.2da` of yours with none, Moonglow's of the game's). Scripts and tables a description names are found in the folder (for one table, its repository or nasher project; or `--beside FOLDER`), else in the game. A `strref` column's custom strings (16777216 and on) are held to the talk table `--tlk FILE` names (a `.tlk`, or a nasher project's `.tlk.json`), else to the folder's own if it has one; with neither they are not looked up, and it says so (a game install is not needed) |
| `mg 2da-set TABLE.2da COLUMN=VALUE… (--row N \| --where COLUMN=VALUE \| --append)` | set cells of a 2DA file and save it as it is laid out (its columns in line or not, their widths, its rows' labels, tabs and line ends, as the 2DA editor saves): in the row of that number, in the one row whose column has that value (`--where Label=Sword`; none, or several, is an error that names them), or in a row added at the end (numbered after the last, its other cells empty). A value with a space is written quoted; `COLUMN=` or `COLUMN=****` empties a cell; a column the table hasn't, a row it hasn't or a value a cell can't hold (a double quote, a line break) is refused and nothing written. `-o FILE` writes it there instead; `--dry-run` says what would change. `--json`: `row`, `appended`, `cells` (each `column`, `from`, `to`) and `written` |
| `mg 2da-diff A.2da B.2da` | what differs between two tables, by row number and column name (not by layout, case or column order): columns and rows added and removed, and each cell that differs. `--edits FILE` writes an edit file for `mg apply` that sets the table in a module to B |
| `mg 2da-diff A.2da B.2da --base BASE.2da -o MERGED.2da` | merge two tables that were both made from BASE: what either changed is taken. A cell both changed, each to something else, is a conflict (two rows added under one number are, where they differ): it fails and writes nothing, unless `--prefer a` or `--prefer b` says whose to take. `--edits FILE` writes the merged table as an edit file |
| `mg tlk-diff A.tlk B.tlk [--base BASE.tlk -o MERGED.tlk]` | the same for talk tables, by line. `--lines FILE.json` writes the lines that differ as a file for `mg tlk-import` |
| `mg tlk-alloc TABLE.tlk --from NAMES.toml` | write a custom talk table from strings kept by name: see [Strings by name](#strings-by-name) |

### Haks and folders of your own

`--hak PATH`, which `mg which`, `mg cat` and `mg model-files` take as
often as needed, adds a hak, ERF or module (`.hak`, `.erf`, `.mod`) or a
folder of loose files (its own files, not its folders') to the load order:
the first given wins over the next, and all of them over every hak (the
game's, the user folder's, a `--module`'s), under the portraits folders
and the user folder's `development/` (`--no-user-dir` leaves the user
folder's out). Its layer is named by its
kind and the path as given: `hak:build/hak/top.hak`, `folder:assets/models`.

## Modules

| Command | Does |
| --- | --- |
| `mg info MODULE` | the module's name, tag, entry area, areas (those its area list names, which the game loads; an ARE it leaves out is warned of, `unlisted_areas` in the JSON), haks, talk table, game version and resources by type |
| `mg find MODULE` | blueprints and objects placed in the areas: `--type utc,utp`, `--tag` (`*` matches any run), `--name` (words it contains, in the name as the module tree shows it: the talk table's string where it has none of its own, another language's where it has no English), `--resref` (a placed object's blueprint), `--area`, `--placed` or `--blueprints`, and `--where Label=Value` (a field's value, `*` as in tags; `Label` alone: has the field), each as often as needed. A field that holds a row of one of the game's tables takes the row's label or name too, for each type it is a row of (`--where Race=Human`; `Appearance=Chest` is a placeables.2da row for a placeable, a doortypes.2da row for a door); two rows of a name are said, to be given by number or label. `--palette-group GROUPING/GROUP` lists a [palette grouping](03-modules.md#palette-groupings)'s group instead (`Kind/Tables`, by the grouping's id or title; `Kind/Other`: those of no group): the blueprints of the standard palettes (the game's and the haks') and the module's custom ones it is offered in (with no custom palette file, the one Build's Palettes would make, as the palette pane shows), each with its palette, Tag, name and category, narrowed by `--type`, `--name` and `--resref` |
| `mg search MODULE` | blueprints, placed objects (and the items they hold), areas and conversations by what they hold, each condition given met (several at once: all of them, `--class fighter --feat "weapon focus (longsword)"`; with `--any`, any of them; `--not` before a condition leaves out what meets it, `--class fighter --not --feat alertness`, also what holds or is held by it): `--var NAME` (`*` matches any run; with `--var-type int` and `--value`), `--property TYPE` (with `--subtype`, `--value`, `--param`), `--class` (with `--levels N` or `MIN..MAX`), `--feat`, `--spell` (with `--spell-class CLASS`: known or memorized as that class), `--skill` (with `--min-rank N`), `--appearance ROW` (with `--of creature`, `placeable`, `door`, `generic-door`, `waypoint` or `item`), `--script NAME`, or any field by `--field LABEL`, `--int N`, `--number RANGE` (`5..10`, `0.5..`, a float) or `--text TEXT` (with `--regex`, the names and texts are regular expressions: `--var '^tdod_(rent|own)' --regex`). A row is its number or its name in the game's 2DAs and talk table (the module's haks' too): `--property "skill bonus" --subtype hide`. `--type uti,utc` and `--placed` or `--blueprints` narrow it as in `mg find`; an item blueprint found is also found in the inventories of the blueprints that carry it. `--haks-and-game` searches the haks' and the game's blueprints the module has none of too, read-only, each line ending with the layer it is read from (`key:nwn_base (read-only)`; `"layer"` in JSON) |
| `mg verify MODULE [--unused]` | missing resources and problems in the custom content (and, with `--unused`, unused resources: a script once, `unused\tx.nss\tsource and compiled`, `compiled only (no source)` or `source only (not compiled)`; what counts as used is Build Module's [Unused](09-build-and-test.md#what-counts-as-used)); fails if there are errors, so a build pipeline stops; a project's scripts are compiled first, in memory, as [Verify Module](09-build-and-test.md#verify-module) compiles them |
| `mg roundtrip MODULE [--keep COPY.mod]` | checks that Moonglow keeps the module as it is: saves a copy (a temporary one; the module is not touched), reads it back and compares every resource byte for byte, then writes every GFF and 2DA anew and compares what it reads back (the same fields and cells, the fields it has no name for too); lists what differs and fails if anything does. **Build › Round-Trip Check** does the same in the window |
| `mg model-files WHAT FOLDER [--module MODULE] [--hak PATH…] [--all] [--force] [--dry-run]` | writes the files the game draws a model (`plc_a08`), a blueprint (`chest.utp`; also `.utd`, `.uti`, `.utc`, and a waypoint's `.utw`: its flag) or an area (`tavern.are`) with into a folder: models, walkmeshes, textures, materials, from the module's haks, the haks and folders `--hak` gives ([as for `mg cat`](#haks-and-folders-of-your-own); no module is needed) and wherever else they are, each with the layer it came from (`from` in `--json`); a blueprint with itself and its row of the table its look is chosen by (`placeables.2da-row-312.txt`), an area with its `.are`, `.git`, tileset, tiles, skybox and the models of what is placed in it; the game's own are left out unless all are (or `--all`); files already in the folder are not written over without `--force`; what is named and found nowhere is said (`missing` in `--json`) |
| `mg vars MODULE [NAME]` | the variables the project describes (`variables.toml`, see [Modules](03-modules.md#described-variables)): each one's type, what it goes on, its values (a script's constants and a table's rows read) and its help; `--on KIND` those that go on one kind of object (`placeable`, `area`, `module`…), never one the scripts alone set (`runtime`); `--undescribed`, in their place, the module's variables no description names (of those on `KIND`, of the names the glob `NAME` takes), each with its types, what it is on, where, and the description it is near: one in another case, or a typo of a name `undescribed` takes (since Moonglow 2.13.0; `--json`: `not_described`) |
| `mg claims PROJECT [list\|claim\|release\|hand\|merged] [AREA…]` | the project's [claims table](03-modules.md#claimed-areas) (`[claims] file` in its `moonglow.toml`): every area of the table and of the module, whose each is (`--json` too); or the areas named claimed or released by your builder code, as Claim This Area and Release Claim do, or handed to another builder (`hand --to CODE`, as the Claims Table window's Hand To… does) (`--force`: another builder's claim), or marked merged with their Tags as the module has them. It writes the table, a file of the checkout: commit it |
| `mg used MODULE [--add NAME] [--remove NAME] [--kind KIND]` | what Unused counts as used though nothing in the module names it (the project file's `[used]`; beside a `.mod`, `NAME.moonglow.toml`, made where there is none): listed, and names or globs added or taken out (repeatable), of `scripts` (the default), `conversations` or `blueprints`: a script `ExecuteScript` runs by a name made as it runs ([What counts as used](09-build-and-test.md#what-counts-as-used)) |
| `mg builder-code PROJECT [CODE]` | your builder code in a module folder or nasher project (its `.moonglow/user.toml`, which version control leaves out): shown, set, or taken out with `--clear`; the ResRefs the window proposes begin with it, and what the team's `[resrefs] pattern` makes of a name (its own prefix, without a code, where it has no `{code}`); a code set that holds claims in the project's claims table is said ([Modules](03-modules.md#your-builder-code)) |
| `mg checks` | the checks `mg verify` makes of custom content: each one's id (a finding's `check` in the JSON), its group and what it holds to be true |
| `mg verify MODULE --skip WHAT` | leave something out (repeatable, or several separated by commas: `--skip tilesets,models,2das`): a group (`references`, missing resources; `tilesets`, `models`, `2das`, `objects`, `module` or `plugins`), one check by its id (`set-hole`, as `mg checks` lists them, or a plugin's as its findings name it), a category of missing references (`references:creatures`, Build Module's Missing Resources' categories), the references only the player's own content has (`references:override`, `references:userpatch`, `references:workshop`, or all three: `references:local`), a hak (`hak:NAME` or `NAME.hak`), `override`, `development`, or one resource (`ttr01.set`, a tileset with its tiles' models; `appearance.2da`; a model). It isn't checked (the tilesets and models of a hundred haks are most of the time a Verify takes); the JSON's `skipped` names each word, as `null` (not checked), and only the rest count toward failing |
| `mg verify MODULE --timings` | also print how long each part of the check took, to stderr (the report is unchanged): opening the module, the haks, each group's checks, the tilesets', 2DAs', materials' and models' time added up with the longest of each, and the time of each check of the module's areas, objects and blueprints added up (each of those files is read once, on every core, for all of them) |
| `mg verify MODULE --plugins DIR` | also run the checks of the plugins in `DIR` (a plugin's folder, or a folder of plugins); their problems count with the rest. A project's own plugins (its `plugins/`) run as you allowed them in Moonglow, and each that did not is said in a note |
| `mg verify MODULE --allow-project-plugins` | run the checks of every plugin the project comes with, allowed or not: for a build pipeline, where nobody is asked ([A project's own plugins](15-plugins.md#a-projects-own-plugins)) |
| `mg verify MODULE --fix CHECK` | make the fixes the findings of a check offer (repeatable; a check's id, as a finding's `check` says: `tag-unique`, `area-variables`, `var-runtime`, `static-transform`, `example.tag-conventions/tag-case`) and save: each as Verify Module's button makes it, refused (and named) where an earlier fix changed what it would change (one that takes items out is followed: two variables of one object both go). The report is of the module as it was found, with a `fixed` line a fix; only errors not fixed fail. `--dry-run` says which it would make and changes nothing. Without `--fix`, nothing is changed. A check there is none of (mistyped, or a plugin's that `--plugins` doesn't name) is an error, not "nothing to fix"; a plugin's fix that asks (runs a command) is said and not made |
| `mg plugin list DIR…` | the plugins in the folders and what each adds: its commands (each by its submenus and title, `Make › Crafting › Workbench…`), checks, events, exporters, importers, wizards, panels and settings (each setting's kind, scope and value). `--project PROJECT` adds the project's own plugins, each with what you answered (`allowed`, `not asked`, `changed since allowed`, `denied`), its digest, and its settings' values in the project |
| `mg plugin allow PROJECT [ID…]` | allow a project's own plugins to run, as they are now (no ids: every one of its plugins), as Moonglow asks when the project is opened; `--deny` keeps them off. The answer is kept in Moonglow's own folder (`project-plugins.toml`), by the project's place on disk, the plugin and a digest of its files, never in the project: a changed plugin is asked about again, and a yes holds for the same files wherever the project is (moved or copied) |
| `mg plugin check DIR…` | check plugins: the manifest reads, and the code registers what it declares; fails if not, or if the code takes more than 5 seconds to load. A file is a plugin's archive (or several plugins'): checked as the installer checks it, and each plugin's code as it is in the archive. A folder's test files (`tests/*.toml`) are read too, not run: one that doesn't read, a test of what the manifest lacks, a fixture not there |
| `mg plugin test PLUGIN` | (PLUGIN a plugin's folder, or a folder of plugins: each one's that has tests, its lines under its id) run a plugin's own tests (`tests/*.toml` in its folder: a fixture module and what its checks, commands and events are to make of it; [Writing plugins](16-writing-plugins.md#a-plugins-own-tests)): a line a test, `ok` or `FAIL` with what was not as wanted, a `note` under a test that passed without a game resource it read, and a summary; fails if a test fails or there are none. `--module FIXTURE` runs every test on another module. The fixture is never written. A test reads the game's data from the game `mg` finds (`--root`), the one at its own `root` (none with `root = false`), and over either its own folder of game resources (`game/` beside the test file), which is all it reads without a game. A test may run one of Moonglow's own checks on its fixture (`builtin = "tag-unique"`), by the fixture's project file |
| `mg plugin run MODULE PLUGIN COMMAND` | run a plugin's command on a module and save: its edits as one command, all or none. `--answer ID=VALUE` fills a form's field (the others take their defaults), `--file PATH` is the file or folder it gets where it asks for one (in the order it asks), `--yes` agrees where it asks, and to the haks it writes (into `--hak-dir DIR`, else the user folder's `hak`); `--dry-run` prints the edits (a file `mg apply` reads) and changes nothing. With `--event`, COMMAND is the id of what the plugin does by itself, run as the window would run it then ([its events](17-plugin-api.md#ctxevent)): `--changed NAME` names a resource the save changed or writes, or that was made (as often as needed; none named: all of the module's), `--was MODULE` is the module as it was before a save (its resources are the old ones, and those that differ are what the save writes), `--select` names the objects placed or pasted, and `--by` what made them; what a `resource-saving` handler says is in the notes (and the JSON's `said`), and a refusal fails the run. The plugin's settings are what the module's project files hold, else the defaults; `--setting ID=VALUE` gives one for the run. What the plugin sets (`ctx.settings:set`) is kept in the project's files, and one of scope `user` where Moonglow keeps those (its own folder's `plugin-settings.json`, shared with the window), which a run reads too (with `--dry-run`, said and not kept) |
| `mg plugin export MODULE PLUGIN ID OUT` | run a plugin's exporter (plugin API 0.6) as File › Export does: it writes the file OUT, or (an exporter to a folder) into the folder OUT, made if it isn't there; the module is not changed. `--select` names what it exports (an exporter for what is selected warns without it), `--answer`, `--answers`, `--setting` and `--file` as for `mg plugin run`; `--dry-run` writes nothing. The JSON's `files` are the files written |
| `mg plugin import MODULE PLUGIN ID SOURCE` | run a plugin's importer (plugin API 0.6) as File › Import does: it reads SOURCE, a file of the kinds its manifest names (another is refused) or a folder, and its edits are saved as one command, all or none; the options of `mg plugin run` (`--yes` and `--hak-dir` for a hak it writes, `--dry-run`) |
| `mg plugin wizard MODULE PLUGIN ID` | run a plugin's wizard (plugin API 0.6): each page answered from `--answers FILE` (a JSON object of the fields' values by id, `{ "name": "Smithy" }`) and `--answer ID=VALUE` over it, the rest by their defaults, each going on to the next; its Finish's edits saved as one command. A page whose check objects is an error, saying what it said, and nothing is saved. `mg plugin run` takes `--answers` too |
| `mg plugin tidy PROJECT [PLUGIN…]` | tidy what is kept of plugins' settings, as the settings' form's Tidy Up does: for the project's own plugins and those named, a value not of its kind or of no setting is taken out (the default is used, as it was), one in the other file than its scope's moved there; in the project's files and Moonglow's own (scope `user`). Another plugin's table is left alone (said). `--dry-run` says what it would do |
| `mg plugin pack PLUGIN` | pack a plugin's folder into an archive to hand around (`ID-VERSION.zip`, or `-o FILE`): checked first, hidden files and its `tests` folder left out |
| `mg plugin install FILE FOLDER` | install a plugin from its archive into a folder of plugins, as Plugins › Install Plugin from File… does: the archive is checked and nothing of it runs. `--replace` installs over the same plugin installed from an archive before. An archive of several plugins installs each into its own folder, all of them or (one installed already, without `--replace`) none; its JSON is `{ "plugins": [...] }` |
| `mg plugin remove ID FOLDER` | remove a plugin that was installed from an archive from a folder of plugins, by its id; one put there by hand is left alone |
| `mg nui new MODULE NAME` | add a NUI window to the module and save: its layout (`NAME.jui`, a window with a Close button) and what the [NUI Creator](18-nui-creator.md) keeps about it (`NAME.txt`). NAME is 1 to 14 lower-case letters, digits or underscores (the scripts add `_o` and `_e`); a name whose resources the module has is refused; `--dry-run` lists the resources and adds none. In a nasher project whose sources leave out `.jui` and `.txt` files (`src/**/*.{nss,json}`) it says so, with the `include` line to add (with `--json`: `left_out` and `include`); `validate` and `generate` say it too |
| `mg nui validate MODULE NAME` | check a NUI window as the NUI Creator does (the layout is what the game's NUI functions take, every bind has a value) and print each problem; fails if there are errors, and changes nothing. With `--json`: `errors`, `warnings` and `findings` (each with `severity`, `path` and `message`) |
| `mg nui generate MODULE NAME` | write the window's scripts and save, as Build does in the NUI Creator: the include that opens it (`NAME_o.nss`) and its event script (`NAME_e.nss`, compiled; the code you wrote in it is kept), and the scripts that include the opener compiled again, as `mg compile` compiles them (what was compiled is remembered in the same place). The window's scripts are all or none: a window with errors, an opener or event script edited by hand, or one that does not compile, changes nothing. A script of yours that includes the opener and does not compile is printed and keeps the compiled script it had; the window is built and saved all the same, and the command fails, as `mg compile` does, so that a pipeline stops there (with `--json`: `failed` and `errors`, as `mg compile` gives them); `--dry-run` checks and compiles, lists the resources and writes none; `--ndb` makes each compiled script's debug file too, as `mg compile --ndb` does. To see the window in the game, use Build › Test Module. A window whose events go to the module's own event script (`event_function` in its settings) gets `NAME_e.nss` as an include instead, compiled through a caller, with the scripts that include it; a bare `return;` in a handler's section becomes `return TRUE;` there (and `return TRUE;` or `return FALSE;` becomes `return;` in a script of its own), each change printed with its section and line (with `--json`: `returns`, each with `section`, `line`, `was` and `now`) |
| `mg nui render MODULE NAME` | draw a NUI window into a PNG as the NUI Creator's preview draws it: the game's skin, pictures and fonts from the install and the module's haks, nothing selected. `-o FILE` (default `NAME.png`), `--scale S` the UI scale (1 is 100%, 0.25 to 4; the picture is S pixels a point), `--size WxH` the window's size in points in place of its geometry's. `--state STATE[:ID[@ROW]]` draws a control (by its ID; every row's, in a list; with `@ROW`, a list's control in that row alone, from 0) or, without an ID, every control in a state: `normal`, `hover`, `pressed` or `disabled`, as the Creator's State menu shows the selected one; again for more (`--state hover:send --state disabled:cancel --state pressed:read@2`), a row's state winning over its ID's and an ID's over every control's; a control disabled by its ID disables what is in it, and one its binds disable stays disabled. `--binds FILE.json` sets binds as a script would with `NuiSetBind`, in place of their first values: one JSON object of names and values (a string, number, boolean, an array for a list's rows, `{"r","g","b","a"}` for a color, `{"x","y","w","h"}` for a rectangle), so that a list is drawn with the rows a character would see; a bind no property reads is said, and a value the properties that read the bind don't take, as the checks say it of a first value (a color without its `a`, text for a list's row count; a number where text is bound is drawn as text, and nothing is said). `--variant NAME` shows a swap layout variant (a key of the settings' `views`) in its group, or as the whole window, in place of what it holds at first, as `NuiSetGroupLayout` would; again for variants of other groups (`--variant details --variant notes`; the whole window's first, then in the order given, so a group a variant holds can show another). An ID no control has, a row its list doesn't have (or `@ROW` for a control in no list), a variant the window doesn't have, two variants for one group or a state given twice is refused. MODULE may instead be a window's `NAME.jui` file alone, with the `NAME.txt` and the pictures it names beside it. Needs a GPU; without the game it draws without the skin and says so. It is the preview, not the game: spacing and events are seen in Test Module. Changes nothing |
| `mg nui layout MODULE NAME` | where the preview puts each control of a NUI window, by the same layout `render` draws: each element's JSON pointer, `kind`, `id`, the binds it reads, the text it shows (a text field's value, or its placeholder while it is empty), and `x`, `y`, `w`, `h` in the window's points (its top left corner, title bar included, at 0, 0), and whether it is `hidden`, `zero_sized`, `clipped`, `out_of_view`, `overlaps` a neighbor (the canvas lays neighbors out apart, so this finds only what its own layout puts over another, not where the game would lay the window out otherwise) or cuts its text (`cut_text`; for text that runs past the bottom of a box that doesn't scroll, also `needs_height`, the height in points that shows it all), and its `state` when it isn't normal (`hover`, `pressed`, `disabled`); a list's rows are each one's, its rows out of its view too, `out_of_view` where scrolling would bring them (the first 512 of a list, and 4,096 rows of a window's lists together; a list inside another list's row has the rows that row shows of it; a list with rows left out is named in `issues`); with the window's `width`, `height`, `body`, its `title` and whether the title is longer than its bar and cut (`title_cut`), and the pictures and fonts not found and what is said of the binds given (`issues`). `--scale`, `--size`, `--state`, `--binds`, `--variant` and a `.jui` file as for `render`, so that its rectangles are the picture's (with a variant, paths are of the window as shown). A line a control without `--json`. A JUI that reads as JSON but is no object (a list, a number) is refused, as the editor refuses it, by `render` too. Needs no GPU |
| `mg nui schema` | print the form of a window's settings file (`NAME.txt`, `moonglow.nui/1`) as a JSON Schema: each key, its type, default and use, and each handler's form. `--jui`: the layout's form (`NAME.jui`) as a JSON Schema, made of the checks' own tables: the window's properties and each control's by its `type`, the kind of value each takes and whether a bind may give it or `nw_inc_nui` makes it static (what the checks hold beyond that, `validate` says). `--layout`: a new window's layout and each control as the Creator adds it, without a width (its description says what that makes of them: most controls keep 150 points, labels, lists and layouts take their column's width, and a column is as wide as its widest control, not as the window) |
| `mg nui import MODULE SOURCE…` | bring NUI windows into the module and save: each window's layout, its settings (when they are the Creator's) and its event script's source (`NAME_e.nss`), from `.jui` files (with what is beside them), folders, ERF archives (`.erf`, `.hak`, `.mod`) or zip files, as the NUI Creator's **Import…** brings them. The opener and compiled scripts are not brought: `generate` writes them. A window the module has differently is refused but with `--replace`; `--dry-run` lists the resources. Each window is checked as `validate` checks it |
| `mg haks MODULE` | what the module's haks provide, where they conflict and which game resources they override |
| `mg haks MODULE --usage` | which of each hak's resources the module reaches (areas' tilesets, tiles, edge tiles and replacement textures, objects' and blueprints' looks and sounds, scripts and what their strings name, every model's textures and materials, the icons and visuals of the feats, spells, skills and classes of a used table) and which nothing does: a line a hak (used of all, bytes used of all; unused, shadowed, not judged); with `--json` each resource used with what reached it first ([Modules](03-modules.md#what-the-module-uses-of-its-haks)) |
| `mg minimap MODULE AREA OUT.png [--size PX] [--map-notes]` | an area's minimap as a PNG, laid out as the game's map draws it (`PX` pixels a tile); `--map-notes` marks its enabled map notes as the game's map does (with `--json`, `map_notes` lists each one's tag, text and where it is in the picture) |
| `mg minimap MODULE AREA… --to FOLDER [--all] [--force]` | several areas' minimaps (`--all`: every area), each `AREA.png` in `FOLDER`; files there of those names are written over only with `--force` |
| `mg nwsync MODULE REPOSITORY` | publish the module's haks and talk table for NWSync into a repository folder, as `nwn_nwsync_write` does: `--with-module` (with `--name`, `--description`, `--uuid`), `--group-id`, `--no-latest`, `--limit-file-size MB`, `--force`, `--dry-run`, `--level N` (the data files' zstd level, 1 to 22, by default 3, `nwn_nwsync_write`'s); prints the manifest's hash. `--upload` then sends the folder to the web server by the project's `[nwsync] upload`, or `--upload "COMMAND"` by that one: run in the folder once for each part, the data, then the manifests, then `latest` (left out with `--no-latest`), each after the one before succeeded, its output on standard error (give `--upload` after the folder, or as `--upload=COMMAND`: a word after it is taken for its command) ([Uploading to the web server](09-build-and-test.md#uploading-to-the-web-server)) |
| `mg nwsync-prune REPOSITORY` | tidy an NWSync repository as `nwn_nwsync_prune` does: remove the data files no manifest names that were written more than `--min-age` seconds ago (two weeks by default), the folders they leave and the `.json` of manifests taken away, and say what it can't mend (`latest` naming no manifest, a manifest without its `.json`, a data file a manifest needs that is missing); `--dry-run` says what would go. Delete the manifests no server names any more first ([Publish to NWSync](09-build-and-test.md#publish-to-nwsync)). `--upload "COMMAND"` then sends the folder as `mg nwsync --upload` does, the manifests before the data (`rsync -a --delete {part} …` takes away on the server what the prune took away, a manifest before its data) |
| `mg attach MODULE FILE…` | copy haks and a talk table (from anywhere, or from the `.zip`, `.7z` or `.rar` archive they came in) into the user folder's `hak` and `tlk`, list the haks at the top of the module's hak list in the order given, name the talk table, and save (`--replace` replaces different files of the same names there) |
| `mg serve MODULE` | run the module in the game's headless server and print its log and console: see [Running a module in the server](#running-a-module-in-the-server) |
| `mg test MODULE [--filter TEXT]` | run the module's `test_*` scripts in the server and report what they assert: see [Testing scripts in the server](#testing-scripts-in-the-server) |
| `mg watch PROJECT --dev-dir FOLDER` | follow a module kept as files while you edit it, putting changed scripts (compiled) and blueprints into a development folder: see [Following a project as you edit](#following-a-project-as-you-edit) |
| `mg compile MODULE [--uncompiled] [--all] [--ndb]` | compile the module's scripts and save it: those changed since they were last compiled, by `mg compile` or by Compile All Scripts in the window (both remember it in Moonglow's data folder, not in the module); `--all` compiles every script, `--uncompiled` only those without a compiled version (remembered as compiled too); `--ndb` makes each one's debug file (`.ndb`) too, and compiles again a script that has none. A script that does not compile is printed, what compiled is saved, and the command fails (with `--json` too, whose `failed` and `errors` say which), so that a pipeline stops there |
| `mg export MODULE NAME.EXT… -o OUT.erf` | export resources with what they use (`--keep-factions`, `--comment`); the archive's description says what it was made from (git commit, Moonglow's version, the module) |
| `mg export MODULE NAME.EXT… --files -o FOLDER` | write the resources named as loose files in FOLDER (a script with its compiled `.ncs` and `.ndb`, an area with its `.git` and `.gic`): for the game's `development` or `override`, or a server's |
| `mg import MODULE PATH… [--overwrite] [--dry-run]` | import an archive (an `.erf` alone: says what a Moonglow export was made from, and warns when the module's git checkout is at another commit now), or loose files and folders (each file a resource by its name, `guard.utc`; a folder's with those of the folders in it, as `mg export --files` writes them), into the module and save it. Without `--overwrite`, the resources the module has are kept and named. What is no resource of a module's is refused and said why (no ResRef, no type, a GFF that doesn't read as its type, a resource given twice, `module.ifo`, an archive among files), as File › Import Files refuses it; a script that comes without its compiled script is compiled, with the scripts that include it, and one that doesn't compile is named. `--dry-run` says what would be done and saves nothing. The JSON has `added`, `replaced`, `skipped`, `new_areas`, `refused` (`file`, `why`), `compiled`, `not_compiled` and `warnings` |
| `mg refs MODULE NAME.EXT` | where a resource is used, and script strings that spell it (`--tag` for a tag); of a script, also the cells of the game's 2DAs that run it, as the module sees them (`run by the game\tspells.2da row 12, ImpactScript`, where the game is found), and the project file's `[used]` glob that counts it as used. A script string's line is shown in the codepage the game reads scripts in, as `mg lsp` reads them (your user folder's `settings.tml`: Windows-1250 for a Polish game), or the module's own `encoding.2da` table; the scripts' bytes are not changed |
| `mg rename MODULE NAME.EXT NEW` | rename a script, area, conversation or blueprint everywhere and save (`--strings` changes script strings too); a script's name in the module's own 2DAs follows (`in_tables` in `--json`), and the cells of the haks' tables that still name it are warned of (`hak_cells`); `NEW` is letters, digits and `_` (lowered, as Aurora writes ResRefs), another character refused; a NUI window (`NAME.jui`) goes with its settings and its two scripts, the scripts that open it follow, and it is built again under its new name as `mg nui generate` builds (with `--json`: `built`, and `compiled` with what did not compile). A script of yours that opens the window and does not compile under the new name is printed (it is left without a compiled script: the one it had was of its text before the rename); the window is renamed, built and saved all the same, and the command fails, as `mg nui generate` does, so that a pipeline stops there. It fails too, the rename saved, where the window can't be built under its new name at all (the warning says why: run `mg nui generate`) |
| `mg update-instances MODULE [NAME.EXT…]` | remake placed objects from their blueprints (all of the module's if none are named, as **Edit › Update All Instances…** does) and save (`--area AREA` for one area) |
| `mg bake-walkmesh MODULE --area AREA --object N…` | [bake](04-areas.md#selecting-and-arranging-objects) placed placeables' walkmeshes to their visual transforms, as **Position › Bake Walkmesh to Transform…** does, by the project's `[bake]`: each a copy of its model in the project's hak, its row in the rows file, its Appearance that row, and save. `N` is the placeable's number in the area's list, from 0 (`mg find MODULE --type utp --area AREA` gives it); several share one bake where their transform is the same. `--unbake` sets each back to the row it copies; `--dry-run` says what it would make and writes nothing |
| `mg dialog-export MODULE NAME.dlg OUT [--language CODE]…` | write a conversation as plain text, CSV, Twine or Ink, by `OUT`'s extension (`.txt`, `.csv`, `.twee`, `.ink`). A CSV has a column for each language its lines have text in (`text_fr`, `text_de_f`…), and `--language fr` adds one to translate into; `mg dialog-import` reads them back |
| `mg dialog-export MODULE NAME.dlg… --to FOLDER --format FORMAT` | write several conversations into a folder (made if missing), each a file of its own named after it (`NAME.ink`, `.twee`, `.txt` or `.csv` for `--format ink`, `twine`, `text` or `csv`), as the module tree's [Export as Ink… and the rest](02-main-window.md#the-module-tree) write them; `--all` for every conversation of the module. Files of those names in the folder are written over only with `--force` (without it, nothing is written and they are named). A name the module hasn't is said, the rest are written, and the command fails; with `--json`: `written` (each `conversation` and `file`) and `failed` (each `conversation` and `error`) |
| `mg dialog-import MODULE FILE [--name NAME] [--force]` | read a Twine or Ink story into the module as a conversation named after the file (or `--name`), or a CSV export's text back into its conversation, and save. A story takes the place of a conversation of that name only with `--force` (without it the command fails and nothing is written); that conversation's end scripts and other [settings of its own](06-conversations.md) stay where the story says none |
| `mg replace MODULE FIND WITH` | replace text in the module's names, descriptions, conversations and journal and save (`--match-case`, `--whole-word`, `--only names,journal,…`, `--dry-run` to list the strings only); `--only` takes `tags`, `resrefs` (not a resource's own name: `mg rename` renames it), `variables` (names and string values), `fields` (the other text fields) and `all` too, and `--regex` makes FIND a regular expression whose groups WITH may name (`--regex '^wp_(.*)_1$' 'WP_${1}_EXIT'`). A tag it makes another object's is warned of, as `mg set` warns of it |
| `mg set MODULE NAME.EXT FIELD=VALUE…` | set fields of a resource and save: a field is a path from the resource's root (`Tag`, `/ClassList[0]/ClassLevel`) and keeps its type; `--remove FIELD`; `--dry-run` prints the change and makes none. `--plugins DIR` (repeatable) and the project's own plugins as you allowed them look at what is saved, as before a save in the window: a refusal is an error and nothing is written, a warning is a note ([what plugins do by themselves](15-plugins.md#what-plugins-do-by-themselves)) |
| `mg apply MODULE EDITS.json` | apply a file of edits (below) and save: all of them, or none if one does not apply; `-` reads standard input; `--dry-run` only checks; `--plugins DIR` as for `mg set`. Both warn of a tag they put on more than one object of the module, as the window's [Duplicate Tags](04-areas.md#duplicate-tags) says it `--strict` (and on `mg set`): a plugin's warning of the save stops it too, as a refusal does |
| `mg areas MODULE [AREA…]` | list the module's areas, narrowed by `--match TEXT` (every word of it in the name, tag or ResRef), `--tileset`, `--interior` or `--exterior`, `--underground` or `--above-ground`, `--natural` or `--artificial`, and what they hold: `--where FIELD=VALUE` (a field of the area or its sounds and music, `*` as in tags; a row by its label or name too), `--where-var NAME=VALUE` (a variable, its value if given), `--music ROW` (by day, by night or in battle); with `--set FIELD=VALUE`, `--var NAME=VALUE`, `--remove-var NAME`, `--static-placeables` or `--dynamic-placeables`, `--grass MATERIAL:TEXTURE:DENSITY:HEIGHT[:R,G,B[:R,G,B]]`, `--remove-grass MATERIAL`, `--tileset-grass yes|no`, `--lighting-scheme SCHEME` or `--lighting NAME`, make each change to each of those areas and save (`--dry-run` to list the changes only; `--all` to change every area). A field that holds a row takes its label or name too (`--set MusicDay=mus_cave`, `LoadScreenID=...`); `--names` shows the rows changed by name, `57 (Cave)` (`from_name` and `to_name` in the JSON) |

A module is a `.mod` archive, a module folder or a nasher project. `mg
compile` and `mg import` write the module in place; an archive's previous
version is kept beside it (`mymodule.mod.bak`). A `.mod` is read and
written as the `.mod`, as the game's server reads it, even with a
folder of its name beside it (which the window asks about, its name in
any case): the command says so in a note (in the JSON's `notes` too),
and that the folder is left as it is. Name the folder to work on the
folder. A command that saves a module folder leaves what another
program changed in it meanwhile as it is, as the window does.

A command that changes blueprints keeps the module's custom palettes
as a save in the window does ([the custom palettes](05-blueprints.md)):
`mg set`, `mg apply`, `mg import`, `mg replace`, `mg rename` and `mg
plugin run` bring the entries of the blueprints they changed up to date
(a name, a category, a creature's challenge rating and faction) and say
how many palettes changed. It reads the game for the names (a module
without custom palettes gets none made); without a game install the
palettes are left, said in a warning, for the next save in the window or
a Build.

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

Fighters without Alertness (`--not` goes before the condition it
leaves out), then creatures that are fighters or have the feat:

```text
mg search mymodule.mod --class fighter --not --feat alertness
mg search mymodule.mod --any --class fighter --feat alertness
mg search mymodule.mod --spell heal --spell-class cleric --haks-and-game
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
- A creature's, placeable's, door's or encounter's faction set to 0, or
  taken away (which the game reads as 0), is set, and warned of: the PC
  faction is the players' alone, and Verify warns of it after
  (`pc-faction`; a trigger's, no).
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
the areas chosen: ResRef, name, tileset and kind. Every ARE the module
holds is listed, and one its area list (`Mod_Area_list`) leaves out is
warned of: the game doesn't load it (`listed` in the JSON). Names are
shown as the module tree shows them: the talk table's string where an
area has no name of its own, another language's where it has no
English.

```text
mg areas mymodule.mod --underground
mg areas mymodule.mod --underground --set MusicDay=57 --set MusicNight=57 --dry-run
mg areas mymodule.mod --tileset tdc01 --set SunFogAmount=8 --set SunFogColor=0x201810
mg areas mymodule.mod cave1 cave2 --var nSpawnLevel=4 --remove-var bOld --set Natural=yes
mg areas mymodule.mod --all --static-placeables --dry-run
mg areas mymodule.mod --tileset tin01 --grass Wood:grass:2:0.4 --tileset-grass no
mg areas mymodule.mod --interior --lighting-scheme InteriorDark --set SunFogAmount=4
mg areas mymodule.mod --where-var 'tdod_rental' --music mus_cave --set MusicNight=mus_cave
mg areas mymodule.mod --where 'Fog*=0' --where OnEnter=kz_enter
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

- **`--lighting NAME`**: lighting kept in Moonglow (Area Properties ›
  Visual › Saved Lighting, by its name: `--lighting "Damp Crypt"`), or a
  file of it (`dusk.lighting.json`, anywhere), as Saved Lighting gives
  it: the Visual page's settings it keeps; the tiles' lights are left as
  they are, unless **`--tile-lights`**: each tile's picked at random from
  the colors kept with them (`--seed` picks them again).
  It is made after `--lighting-scheme` and before `--set`. The kept
  lighting is in Moonglow's data folder, `lighting` (`MOONGLOW_DATA_DIR`
  names another data folder).

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
  animation switched on, those with variables, those the project
  file keeps dynamic, nor those whose tags the module's scripts name as
  text); see [Areas](04-areas.md#static-placeables-many-at-once). Each placeable left
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
- Names and comments, which are each area's own, aren't set by this
  command (`mg replace` changes text in names), nor list fields one by
  one (a tile's own lights; variables are `--var`'s). `--set Tag=…` sets
  the tag it is given on each area, and warns of one now on more than
  one object (as `mg set` does).

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
  serve`'s. Your own NWN folder as `--dir` is refused, before anything
  is written there.

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
- The project is read once, at the start. After that the system says
  which files changed (where it can't, a network share, say, their
  times and sizes are looked at a few times a second), and only those
  are read again, as the window reads files changed outside Moonglow.
  It is read whole again only where reading it settles something:
  `nasher.cfg` changed, an area added or taken away, or `module.ifo`
  changed (its area list is filled as nasher fills it).
- `--target NAME` follows that target of a nasher project (default: the
  target that packs a module), as `mg build --target` builds it.
- `--mod FILE` builds the module to a file, at the start and after every
  change, with every script compiled.
- `--test` runs the module's tests (its `test_*` scripts) as
  [`mg test`](#testing-scripts-in-the-server) runs them, in a server of
  their own on a copy, at the start and again after each change taken
  up, and prints their results (a `tested` event with `--json`, with
  `mg test`'s fields). The tests compile with Moonglow's `mg_test`
  where the module has none of its own. A change that doesn't compile
  runs no tests. `--test-filter TEXT` runs only those with that text in
  their name, as `mg test --filter` does. With `--serve` the tests have
  their server and the module served has its own, apart.
- `--ndb` makes each compiled script's debug file (`.ndb`) too, for the
  game's script debugger: put out beside the compiled script, and built
  into the module.
- `--serve` also runs the module in the server, as `mg serve` does, on a
  scratch user directory whose `development` folder is the one written
  into: `--dir`, `--port`, `--player-password`, `--dm-password`,
  `--admin-password`, `--env`, `--preload` and the arguments after `--`
  are `mg serve`'s, and so is what you type (your own NWN folder as
  `--dir` is refused, before anything is written there). The passwords not given are
  random ones, and the player and DM passwords are printed when the
  server starts (a `passwords` event with `--json`), as `mg serve`
  prints them. Started again, the server keeps its port and its
  passwords: a DM who has joined keeps the one they have. These options
  are refused without `--serve`.
- A script that does not compile is said, with its line, and nothing is
  put out for it: the server keeps the one it has. A warning of the
  compiles (a function's return type declared one way and defined
  another, as `mg compile` says it) is said once, with how many scripts
  met it (`warnings` in the JSON).
- `--json` prints each of these as a line of JSON (`watching`, `synced`,
  `rebuilt`, `tested`, and with `--serve` the server's events).

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
farewell = { text = "Farewell, sir.", feminine = "Farewell, my lady." }
```

- A key is its groups and its name with dots (`feat.cleave.name`). The
  names file is `.toml` or `.json`. New keys take their lines in the
  order the file has them.
- **Feminine text:** the game shows a female character the feminine
  table's line (`TABLEf.tlk`). Where there is one, a key's line there is
  its text, or its `feminine` where it has one; a key with a
  `feminine` makes the feminine table beside the table if there is
  none yet.
- **The map** (`NAMES.strrefs.json`, or `--map FILE`) says which line of
  the table each key has. Keep it with the project. A new key gets the
  next line that was never anyone's (from `--start`, passing over lines
  of the table that have someone else's text); a key you take away keeps
  its line reserved, and gets it back if you name it again. Its line
  keeps its last text in the table: a number saved in a character or an
  item still shows what it said, not an empty line.
- `--constants FILE.nss` writes the keys as constants with the game's
  numbers (`const int STRREF_FEAT_CLEAVE_NAME = 16777217;`; `--prefix`
  changes `STRREF_`), `--constants FILE.json` as JSON.
- `--check` writes nothing and fails unless the table, the map and the
  constants are as they would be written: for a build to refuse a names
  file that was changed and not run. `--against OLDER.json` fails if a
  key has another line than in an older map (the main branch's).
- **The project's lines for typed text** (`[talk] lines` and `[talk.builders]`
  of its `moonglow.toml`: [Text typed into the talk
  table](03-modules.md#text-typed-into-the-talk-table)) are passed over by
  new keys, so a key and a text typed in the window never share a line.
  The project is the one whose `moonglow.toml` is in the names file's
  folder or a folder above it, or `--project DIR`'s; `--json` lists the
  lines passed over (`typed_lines`). A key that has a line already keeps
  it.

## JSON output

With `--json`, every command prints one JSON object on standard output
and nothing on standard error, for scripts, build pipelines and tools
(including an AI assistant's):
- **The result:** a command's result as fields: `mg find` a `found` list
  (each with `kind`, `type`, `resref`, `tag`, `name`, and for placed
  objects `area`, `index` and `position`); `mg verify` its `errors`,
  `warnings`, `missing`, `findings` and `unused` (an unused resource's
  `resource`, its files' `names`, and `files`: of a script `both`,
  `compiled` or `source`, else null; a finding's `fix` is
  null, or the fix it offers: its `label` and its `command`, an edit
  file `mg apply` takes; with `--fix`, `fixes` lists those `made` and
  those `refused`); `mg plugin test` its `tests` (each `file`, `name`,
  `passed`, `problems` and `notes`), `passed`, `failed` and `noted`
  (how many passed with notes); `mg refs` its `uses`,
  `script_strings`, `tables` (each `table` and `at`) and `marked`; `mg used` its `scripts`, `conversations`, `blueprints` and `file`; `mg compile` its `scripts` (how many were compiled), `unchanged` (how many were passed
  over), `failed` and `errors` (each with `script`, `line` and `message`),
  and `warnings`, each once however many scripts met it (with `script`,
  `line`, `message` and `scripts`, how many: a function's return type
  declared one way and defined another, as in the base game's
  `x2_inc_cutscene`, which compiles as in Aurora; see
  [Scripts](07-scripts.md)); and so on.
- **Notes:** warnings and summaries, otherwise sent to standard error,
  are in `notes`.
- **Errors:** a command that fails prints `{"error": "…"}` and exits with
  an error. `mg verify` prints its result and exits with an error when it
  found errors.
- **What doesn't change:** `mg gff` prints the GFF's JSON as it does
  without `--json` (the same values, its floats in serde's notation), and nothing else: its notes go to standard error
  either way, so that what it printed reads back as the GFF. `mg cat` gives the resource's `text`, or its bytes as
  `hex`. `mg lsp` speaks JSON already, and refuses `--json`.

```text
mg --json verify mymodule.mod | jq '.findings[] | select(.severity == "error")'
```

## nasher projects

| Command | Does |
| --- | --- |
| `mg init MODULE DIR` | put a module into a nasher project in `DIR`: its resources as source files, with a `nasher.cfg` like `nasher init` writes |
| `mg build DIR` | compile the project's scripts and pack the module, as `nasher pack` does (`--target`, `-o OUT`): those changed since they were last compiled, as `mg compile` does (a project keeps no compiled scripts, so those are all of them unless it keeps some among its sources); `--ndb` makes each script's debug file (`.ndb`) too, packed with it. A `--target` that packs a talk table (a `.tlk`) makes it of its `<name>.tlk.json` as nasher has `nwn_tlk` make it, byte for byte. Where the project file says `[pack] compile_models = true` ([Modules](03-modules.md#the-project-file-moonglowtoml)), the models kept as text go in compiled, each against its supermodel from the project, its haks or the game, and those that can't be are named |
| `mg merge-claims BASE OURS THEIRS` | merge two copies of a project's [claims table](03-modules.md#claimed-areas), both made from BASE, area by area, and write the merge over OURS (`-o FILE` elsewhere): what one side changed of an area's row is taken, both sides' new areas kept (theirs after ours, numbered on), an area both changed each its own way a conflict, which fails and writes nothing unless `--prefer ours` or `--prefer theirs`. Git runs it as its merge driver: see [Merging in git](03-modules.md#merging-in-git) |
| `mg merge-gff BASE OURS THEIRS` | merge two copies of a GFF resource (an area, `module.ifo`, a blueprint), both made from BASE, as a GFF or as nasher's JSON or NWNT of one, and write the merge over OURS (`-o FILE` elsewhere; `--project DIR` names the nasher project whose float places and codepage the text is written with, where it isn't above the file). Placed objects merge by object, the area list as a union, variables by name and type, inventories by item, localized strings by language, a list's order in sequence. Two different changes to one value are a conflict: it fails, names each by its path and writes nothing, unless `--prefer ours` or `--prefer theirs`. Git runs it as its merge driver: see [Merging in git](03-modules.md#merging-in-git) |

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
--codepage cp1250` names it, and `mg lsp --module PATH` reads them in
that module's own `encoding.2da` table (in its haks), where it has one.
The files open in your editor come as the
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

