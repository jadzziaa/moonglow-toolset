# Modules

## Opening and saving

Moonglow opens modules as `.mod` archives, as folders (a module unpacked
into a directory), as nasher projects (see below) and the game's campaign
files (`.nwm`, read as modules).

- **File › Open Module…** (Ctrl+O), **File › Open Folder…** (a module
  folder or a nasher project), **File › Recent Modules**, or a module named
  on the command line (`moonglow path/to/module.mod`).
- **File › Save** (Ctrl+S) writes the module where it came from, as an
  archive or a folder. **Save As…** writes it as a `.mod` somewhere else;
  a new module is offered as `<name>.mod` in the user folder's `modules`,
  where the game (and Test Module) finds it.
- Saving is safe: a module archive is written to a temporary file first
  and then put in place, so a failure never leaves a half-written module,
  and the previous version is kept beside it (`mymodule.mod.bak`). With **Create
  backups of modules** (Options › General) each save also keeps the module
  as it was in `<name>.BackupMod`.
- **File › Close** closes the module, asking first if there are unsaved
  changes; so does quitting.

Moonglow keeps everything in a module it does not understand: fields it
has no editor for, resources it does not know, unusual orderings. An
untouched module saves as it was.

## Where things are used, and renaming

**Find References** shows everywhere the module names a script, area,
conversation or blueprint. It's in the module tree's right-click menu, a
palette's right-click menu, the script editor's toolbar and **Edit ›
Find References…**.
- **Places are readable**, such as `keep › creature GUARD › OnSpawn` or
  `guard_talk › NPC line 3 “Halt!” › action`. Click one to go there: the
  area with the object selected, the conversation line, the script line.
- **Tags work too.** Type a tag in the tab's field to find the objects with
  it, and the transitions, locked doors and conversation lines (journal
  categories) that name it.
- **Strings in scripts** that spell the name are listed separately, such as
  `ExecuteScript("guard_spawn", …)` or `CreateObject(…, "guard", …)`. A
  script may build names at run time, so a name can be used where no list
  shows it.

**Rename…** (in the same menus, or the References tab) renames a
resource and everything that names it, as one step that **Undo** takes
back:
- **What moves with it:** a script's compiled code, and an area's objects
  and comments.
- **What follows the new name:** objects' and areas' events, conversation
  lines, the module's area list and start area, objects placed from a
  blueprint, inventories and stores, `#include` lines.
- **Strings in scripts** change too if you tick **Also change … in
  scripts**. Check them in Find References first: a string may also be a
  tag or something else (Neverwinter Chess compares tags to `"pawn_b"`, its
  pawns' blueprint name). Strings left spelling the old name are named in
  the log.
- **Recompiling:** scripts whose text changed, and every script that
  includes them, are compiled again.

## Find and Replace Text

**Edit › Find and Replace Text…** (Ctrl+H) finds text in what players
read: the module's names, descriptions, conversation lines, the journal
and the rest (map notes and such), in every language each string is
written in.
- **What's searched:** choose the kinds, and whether case matters and
  whether only whole words count.
- **What's found** is listed by place, as Find References lists them; click
  one to go there, and untick any to leave alone.
- **Replace** changes the ticked strings as one step that **Undo** takes
  back, then lists what's left.

Text that comes from the game's talk table (a string number with no text
of the module's own) isn't the module's to change, and isn't searched.
Scripts have their own **Find in Files**.

## nasher projects (version control)

A `.mod` is one binary file, so version control (git) can't show what
changed in it or merge two people's work. Many teams therefore keep their
module as a [nasher](https://github.com/squattingmonk/nasher) project: a
folder of text files, one per resource (`module.ifo.json`,
`area001.git.json`, `my_script.nss`), with a `nasher.cfg` describing it.
Moonglow opens such a project and saves into it directly, so there is no
unpacking or packing between the toolset and git.

- **Open a project:** **File › Open Folder…** and choose the project's
  folder (the one with `nasher.cfg`). Moonglow edits the target that packs
  a module (the default target if it does).
- **Save** writes only the files of the resources you changed, exactly as
  `nasher unpack` would write them, so `git diff` shows your changes and
  nothing else:
  - new resources go where the project's rules put them;
  - deleted ones lose their files;
  - other files in the folder are left alone.
- **Changes made elsewhere are never overwritten.** If a file Moonglow
  would replace or delete changed on disk since Moonglow read it (after
  a `git pull`, say), the save writes nothing and the log names the
  files. Reopen the project to load them.
- **Areas added on another branch** are added to the module's area list
  when you open the project.
- **Start a project from any module** with **File › Save As nasher
  Project…**, choosing an empty folder. Moonglow writes a `nasher.cfg` like
  `nasher init` does, with everything under `src/`.
- **Build › Pack** *file* writes the module the project builds, as `nasher
  pack` does. **Test Module** (F9) packs it into the game's `modules`
  folder and starts the game on it.
- **Compiled scripts are build output.** A project keeps scripts as `.nss`
  only, and Moonglow compiles them when it packs. A compiled script
  without its source can't be kept in a project; Moonglow names any it
  leaves out.

nasher rounds the module's numbers to 4 decimal places (its
`truncateFloats` setting), so objects in a project may sit up to a
ten-thousandth of a meter from where they were in a `.mod`. In the game
that can show as a turn of up to a degree. Moonglow uses the project's
own settings (`.nasher/user.cfg`): the number of places, the codepage and
whether the area list is kept up to date. It reads projects that keep
resources as JSON, nasher's default. Projects in NWNT format aren't
supported yet.

`mg init` and `mg build` do the same from a terminal or a build pipeline
(see [Command-line tools](11-command-line.md)).

## Recovering unsaved work

While a module has unsaved changes, Moonglow writes a recovery copy of it
every 5 minutes (Options › General sets how often, or turns it off). The
copies live in Moonglow's own data folder, never next to the module or in
the game's folders, and are removed when you save or discard the
changes. If Moonglow (or the computer) stops before that, the next start
offers the copy back in **Recover Unsaved Work**: open it as an unsaved
module, then save it where you want it. Conversations are also backed up
as `<name>.bak` every 5 minutes while they are open, as Aurora does.

## Module Properties

**Edit › Module Properties** (or the top of the module tree), with
Aurora's pages:

- **Basic**: name, tag, and the start area and location (the start
  location marker in the area viewer moves it).
- **Events**: the module's event scripts, each with Edit to open it.
- **Advanced**: the starting date and hour, minutes per hour, dawn and
  dusk hours, the experience scale, the starting movie, Variables.
- **Description**: the module's description.
- **Custom Content**: the haks the module uses, highest priority first
  (where two have the same resource, the higher one wins; changes apply
  when the module is reopened), and its custom talk table.

Text fields that the game shows to players (names, descriptions) are
localized strings: the field shows the language chosen in Options ›
Language, and the **…** button opens **String Edit**, where each language
and gender has its own text, or a talk-table reference.

## Haks and custom talk tables

Haks listed in Custom Content are searched before the game's own files,
in their order, as the game does: their tilesets, models, 2DA tables and
blueprints appear in the area viewer, the palettes and the editors. Haks
are looked for in the user folder's `hak` and then the game's `data/hk`.

A custom talk table (`.tlk`) gives the strings numbered from 16,777,216
up. Moonglow looks for it in the user folder's `tlk` and then the game's
`data/tlk` (where the premium campaigns keep theirs).

**Resources in haks**: when the module has a resource a hak also has, the
hak's wins in the game. Moonglow warns in the log when you add such a
resource, or one that replaces the game's own (Options › General).

## Import and export

- **File › Export…** writes chosen resources to an `.erf` archive,
  optionally with the module resources they use (an area its blueprints,
  scripts and conversations, a conversation its scripts…), and optionally
  moving creatures out of the module's own factions into the standard
  ones. Right-click a resource in the tree to export it.
- **File › Import…** adds an `.erf`'s resources to the module; for those
  the module already has, you choose which to overwrite. It also lists
  what the imported resources refer to that neither the module nor the
  game has.

## Areas

**Wizards › Area Wizard…** makes an area from a name, a tileset and a
size (2 to 32 tiles each way); **Edit › Resize Area…** and **Rotate Area…**
change the area shown last, as in Aurora. See [Areas](04-areas.md).
