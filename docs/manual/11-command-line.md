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
mg [--root GAME] [--user-dir DIR | --no-user-dir] COMMAND ...
```

`--root` is the game's folder (default: `$NWN_ROOT`, else Steam's);
`--user-dir` the user folder (default: `$NWN_HOME`, else the platform's);
`--no-user-dir` reads the game alone. `mg COMMAND --help` describes each
command.

## Archives and files

| Command | Does |
| --- | --- |
| `mg ls ARCHIVE` | list a `.mod`, `.hak`, `.erf`, `.nwm` or `.sav` |
| `mg unpack ARCHIVE OUT` | unpack it into folder `OUT` |
| `mg pack DIR ARCHIVE` | pack a folder into an archive (its type from the extension) |
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
| `mg verify MODULE [--unused]` | missing resources (and, with `--unused`, unused ones) |
| `mg haks MODULE` | what the module's haks provide, where they conflict and which game resources they override |
| `mg compile MODULE [--uncompiled]` | compile the module's scripts (or only those without a compiled version) and save it |
| `mg export MODULE NAME.EXT… -o OUT.erf` | export resources with what they use (`--keep-factions`, `--comment`) |
| `mg import MODULE ERF [--overwrite]` | import an archive into the module and save it |

A module is a `.mod` archive, a module folder or a nasher project. `mg
compile` and `mg import` write the module in place; an archive's previous
version is kept beside it (`mymodule.mod.bak`).

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
