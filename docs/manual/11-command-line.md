# Command-line tools

`mg` does from a terminal what scripts and build pipelines need: archives,
GFF files, the game's resources, and building modules. It comes with every
Moonglow package:

| Package | Running `mg` |
| --- | --- |
| AppImage | Make a link to the AppImage named `mg`; started through it, the AppImage runs `mg` instead of the GUI: `ln -s ~/Applications/Moonglow-*.AppImage ~/.local/bin/mg` |
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
| `mg ls ARCHIVE` | list a `.mod`, `.hak`, `.erf`, `.nwm` or `.sav` |
| `mg unpack ARCHIVE OUT` | unpack it into folder `OUT` |
| `mg pack DIR ARCHIVE` | pack a folder into an archive (its type from the extension); warns when files would start past 2 GiB, where the game stops reading |
| `mg gff INPUT [-o OUTPUT]` | a GFF file to JSON (the format of neverwinter.nim and nasher), or JSON back to GFF |

## The game's resources

| Command | Does |
| --- | --- |
| `mg which NAME.EXT` | where a resource comes from in the game's load order |
| `mg cat NAME.EXT` | print a resource to standard output |
| `mg layers` | the load order's layers and their sizes |
| `mg tlk STRREF…` | talk-table strings by number |

## Modules

| Command | Does |
| --- | --- |
| `mg info MODULE` | what the module is: its name, tag, entry area, areas, haks, talk table, game version and resources by type |
| `mg find MODULE` | blueprints and objects placed in the areas: `--type utc,utp`, `--tag` (`*` matches any run), `--name` (words it contains), `--resref` (a placed object's blueprint), `--area`, `--placed` or `--blueprints`, and `--where Label=Value` (a field's value, `*` as in tags; `Label` alone: has the field), each as often as needed |
| `mg verify MODULE [--unused]` | missing resources and problems in the custom content (and, with `--unused`, unused resources); fails if there are errors, so a build pipeline stops |
| `mg haks MODULE` | what the module's haks provide, where they conflict and which game resources they override |
| `mg minimap MODULE AREA OUT.png [--size PX]` | an area's minimap as a PNG, laid out as the game's map draws it (`PX` pixels a tile) |
| `mg attach MODULE FILE…` | copy haks and a talk table (from anywhere) into the user folder's `hak` and `tlk`, list the haks at the top of the module's hak list in the order given, name the talk table, and save (`--replace` replaces different files of the same names there) |
| `mg compile MODULE [--uncompiled]` | compile the module's scripts (or only those without a compiled version) and save it |
| `mg export MODULE NAME.EXT… -o OUT.erf` | export resources with what they use (`--keep-factions`, `--comment`) |
| `mg import MODULE ERF [--overwrite]` | import an archive into the module and save it |
| `mg refs MODULE NAME.EXT` | where a resource is used, and script strings that spell it (`--tag` for a tag) |
| `mg rename MODULE NAME.EXT NEW` | rename a script, area, conversation or blueprint everywhere and save (`--strings` changes script strings too) |
| `mg update-instances MODULE [NAME.EXT…]` | make the objects placed from blueprints (all of the module's if none are named) again from them, and save (`--area AREA` for one area) |
| `mg dialog-export MODULE NAME.dlg OUT` | write a conversation as plain text, CSV, Twine or Ink, by `OUT`'s extension (`.txt`, `.csv`, `.twee`, `.ink`) |
| `mg dialog-import MODULE FILE [--name NAME]` | read a Twine or Ink story into the module as a conversation (replacing one of that name), or a CSV export's text back into its conversation, and save |
| `mg replace MODULE FIND WITH` | replace text in the module's names, descriptions, conversations and journal and save (`--match-case`, `--whole-word`, `--only names,journal,…`, `--dry-run` to list the strings only) |

A module is a `.mod` archive, a module folder or a nasher project. `mg
compile` and `mg import` write the module in place; an archive's previous
version is kept beside it (`mymodule.mod.bak`).

For example, every placed creature whose tag starts `GUARD` and that has
no OnSpawn script set, in the area `keep`:

```text
mg find mymodule.mod --type utc --placed --area keep --tag 'GUARD*' --where ScriptSpawn=
```

## JSON output

With `--json`, every command prints one JSON object on standard output,
and nothing on standard error, for scripts, build pipelines and tools
(an AI assistant's included):
- **The result:** a command's result as fields: `mg find` a `found` list
  (each with `kind`, `type`, `resref`, `tag`, `name`, and for placed
  objects `area`, `index` and `position`); `mg verify` its `errors`,
  `warnings`, `missing`, `findings` and `unused`; `mg refs` its `uses`
  and `script_strings`; `mg compile` its `scripts`, `failed` and
  `errors` (each with `script`, `line` and `message`); and so on.
- **Notes:** warnings and summaries, which otherwise go to standard
  error, are in `notes`.
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

`mg lsp` is an NWScript language server: it gives VS Code, Neovim, Emacs,
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
the game it still works, but doesn't know `nwscript.nss` or the game's
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

- **VS Code**: with an extension that starts a language server for a
  file type, such as a generic "LSP client" extension, set its command to
  `mg lsp` for `.nss`.

