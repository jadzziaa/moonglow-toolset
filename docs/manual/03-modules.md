# Modules

## Opening and saving

Moonglow opens modules as `.mod` archives, folders (a module unpacked into
a directory), nasher projects (see below) and the game's campaign files
(`.nwm`, read as modules).

- **File › Open Module…** (Ctrl+O), **File › Open Folder…** (a module
  folder or a nasher project), **File › Recent Modules**, or a module named
  on the command line (`moonglow path/to/module.mod`).
- **File › Save** (Ctrl+S) writes the module where it came from, as an
  archive or a folder. **Save As…** writes it as a `.mod` elsewhere. A new
  module is offered as `<name>.mod` in the user folder's `modules`, where
  the game (and Test Module) finds it.
- Saving is safe. A module archive is written to a temporary file, then
  put in place, so a failure never leaves a half-written module. The
  previous version is kept beside it (`mymodule.mod.bak`). With **Create
  backups of modules** (Options › General), each save also keeps the
  module as it was in `<name>.BackupMod`.
- **File › Close** closes the module, asking first if there are unsaved
  changes; so does quitting.

Moonglow keeps everything in a module it does not understand: fields with
no editor, unknown resources, unusual orderings. An untouched module saves
as it was.

## Where things are used, and renaming

**Find References** shows everywhere the module names a script, area,
conversation or blueprint. It's in the right-click menus of the module
tree and palettes, the script editor's toolbar and **Edit › Find
References…**.
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
  pawns' blueprint name). The log names strings left spelling the old
  name.
- **Recompiling:** scripts whose text changed, and every script that
  includes them, are compiled again.

## Find and Replace Text

**Edit › Find and Replace Text…** (Ctrl+H) finds text players read: the
module's names, descriptions, conversation lines, the journal and the rest
(map notes and such), in every language each string has.
- **What's searched:** choose the kinds, whether case matters and whether
  only whole words count.
- **What's found** is listed by place, as in Find References. Click one to
  go there; untick any to leave alone.
- **Replace** changes the ticked strings as one step that **Undo** takes
  back, then lists what's left.

Text from the game's talk table (a string number with no text of the
module's own) isn't searched: it isn't the module's to change. Scripts
have their own **Find in Files**.

## nasher projects (version control)

A `.mod` is one binary file, so version control (git) can't show what
changed in it or merge two people's work. Many teams keep their module as
a [nasher](https://github.com/squattingmonk/nasher) project instead: a
folder of text files, one per resource (`module.ifo.json`,
`area001.git.json`, `my_script.nss`), described by a `nasher.cfg`.
Moonglow opens such a project and saves into it directly, with no
unpacking or packing between the toolset and git.

- **Open a project:** **File › Open Folder…** and choose the project's
  folder (the one with `nasher.cfg`). Moonglow edits the target that packs
  a module (the default target if it does).
- **Save** writes only the files of the resources you changed, exactly as
  `nasher unpack` would, so `git diff` shows your changes and nothing
  else:
  - new resources go where the project's rules put them;
  - deleted ones lose their files;
  - other files in the folder are left alone.
- **Changes made elsewhere are never overwritten.** If a file Moonglow
  would replace or delete has changed on disk since it was read (after a
  `git pull`, say), the save writes nothing and the log names the files.
  Reopen the project to load them.
- **Areas added on another branch** join the module's area list when you
  open the project.
- **Start a project from any module** with **File › Save As nasher
  Project…**, choosing an empty folder. Moonglow writes a `nasher.cfg` as
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
resources as JSON, nasher's default; NWNT format isn't supported yet.

`mg init` and `mg build` do the same from a terminal or a build pipeline
(see [Command-line tools](11-command-line.md)).

## Recovering unsaved work

While a module has unsaved changes, Moonglow writes a recovery copy of it
every 5 minutes (Options › General sets how often, or turns it off). The
copies live in Moonglow's own data folder, never next to the module or in
the game's folders, and are removed when you save or discard the changes.
If Moonglow (or the computer) stops first, the next start offers the copy
in **Recover Unsaved Work**: open it as an unsaved module, then save it
where you want. As in Aurora, open conversations are also backed up as
`<name>.bak` every 5 minutes.

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

Text fields players see (names, descriptions) are localized strings. The
field shows the language chosen in Options › Language. The **…** button
opens **String Edit**, where each language and gender has its own text, or
a talk-table reference.

## Haks and custom talk tables

Haks listed in Custom Content are searched before the game's own files, in
their order, as the game does: their tilesets, models, 2DA tables and
blueprints appear in the area viewer, the palettes and the editors. Haks
are looked for in the user folder's `hak`, then the game's `data/hk`. A
change to the list (adding, removing, reordering, an undo) takes effect at
once.

**Add Haks and Talk Table…** (Custom Content) attaches downloaded content
in one step. Choose the haks and a talk table wherever they are, and put
the haks in order (highest priority first). Moonglow copies them into the
user folder's `hak` and `tlk` (asking before replacing different files of
the same names), lists the haks at the top of the module's list and names
the talk table. Undo takes back the module's changes; the copies stay.
`mg attach` does the same from the command line.

A custom talk table (`.tlk`) gives the strings numbered from 16,777,216
up. The game looks for it in the module's haks, then the module, then the
user folder's `tlk` (Moonglow also looks in the game's `data/tlk`, where
the premium campaigns keep theirs). Its feminine table, `<name>f.tlk`, is
looked for the same way. If the game can't find the table the module
names, it won't load the module; Verify reports it. Name it without
`.tlk`, in lower case: on Linux the file's name must match exactly.

### The talk table editor

**Tools › Talk Table** (or **Edit…** beside Custom TLK) edits the
module's talk table:
- **The lines** are listed by the StrRef the game knows them by
  (16777216 and up), with the feminine text beside them when there's a
  feminine table. **Find** finds lines by words or by StrRef.
- **A line**: choose it to edit its text, its feminine text, and the
  sound spoken with it and its length. **Copy** copies its StrRef, to put
  in a 2DA or a script.
- **Add Line** adds one at the end. **Remove Last Line** removes only the
  last, since removing another would renumber the lines after it.
- **Undo** and **Redo** in its toolbar undo the table's changes. The
  table is a file of its own, so Ctrl+Z still undoes the module's.
- **Saving**: **Save** saves the table, as does saving the module.

With no talk table, the editor makes one in the user folder's `tlk` (with
a feminine table if you ask) and names it in Module Properties. A table in
a hak or in the module is shown read-only: change it where it is built.

**Move to Talk Table** in the String Edit window (the **…** beside a name
or description) adds the text to the talk table as a new line and puts its
StrRef in the string in place of the text. That's for translations, and
for text a server sends many times.

**Resources in haks**: when the module and a hak have the same resource,
the hak's wins in the game. Moonglow warns in the log when you add such a
resource, or one that replaces the game's own (Options › General).

## The hak editor

**Tools › Haks** opens a hak: **New Hak**, **Open Hak…** (a `.hak` or an
`.erf`), or **Build Hak from Folder…**, which makes a new hak from a
folder's files, subfolders included, to look over and save.
- **The list:** every resource with its size, and for files just added,
  where they come from. **Find** narrows it by name or type. Click to
  select, Ctrl+click to add to the selection; right-click for **Rename…**,
  **Extract…** and **Remove**.
- **Adding:** **Add Files…** and **Add Folder…** add files, replacing
  resources of the same name. A file the game couldn't read by its name
  is left out, and the log says why: a name over 16 characters (other hak
  tools silently cut it short), or a type the game doesn't know. Hidden
  files, `Thumbs.db` and `desktop.ini` are skipped.
- **Extracting:** **Extract…** and **Extract All…** write resources as
  files into a folder.
- **Undo** and **Redo** in its toolbar. **Description** is the hak's own
  note, for people.
- **Saving:** **Save** writes the hak beside the old one, one resource at
  a time, then moves it over the old one. A hak of gigabytes needn't fit
  in memory, and a failed save leaves the old one whole. It warns when
  resources would start past 2 GiB, which the game can't read. Saving the
  module saves open haks too.
- **Reloading:** saving a hak the module uses reads it again.
- **The game's own haks** (in its `data/hk`) aren't written: **Save As…**
  keeps your changes in a copy.

Closing a hak with unsaved changes asks first.

## Import and export

- **File › Export…** writes chosen resources to an `.erf` archive.
  Two options add the module resources they use (an area its blueprints,
  scripts and conversations, a conversation its scripts…), and move
  creatures out of the module's own factions into the standard ones.
  Right-click a resource in the tree to export it.
- **File › Import…** adds an `.erf`'s resources to the module; for those
  the module already has, you choose which to overwrite. It also lists
  what the imported resources refer to that neither the module nor the
  game has.

## Areas

**Wizards › Area Wizard…** makes an area from a name, a tileset and a
size (2 to 32 tiles each way). It opens with the first tileset chosen
(Aurora chooses none). **Edit › Resize Area…** and **Rotate Area…**
change the area shown last, as in Aurora. See [Areas](04-areas.md).
