---
type: Manual Page
title: Modules
description: Modules - opening and saving, where things are used and renaming, find and replace, nasher projects, recovering unsaved work, Module Properties, haks and talk tables, the hak editor, import and export.
tags: [manual, modules, haks, nasher]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-10T04:28:14Z }
---

# Modules

## Opening and saving

Moonglow opens modules as `.mod` archives, folders (a module unpacked into
a directory), nasher projects (see below) and the game's campaign files
(`.nwm`, read as modules).

- **File › Open Module…** (Ctrl+O), **File › Open Folder…** (a module
  folder or a nasher project), **File › Recent Modules**, or a module named
  on the command line (`moonglow path/to/module.mod`). A module opens on
  the area you opened last in it; the first time, or when that area is
  gone, on the first the module tree lists (Options › General switches
  this off).
- **File › Save** (Ctrl+S) writes the module where it came from, as an
  archive or a folder. **Save As…** writes it as a `.mod` elsewhere. A new
  module is offered as `<name>.mod` in the user folder's `modules`, where
  the game (and Test Module) finds it.
- In a module folder a resource is one file, named in lower case
  (`bread.uti`). A file Aurora named otherwise (`bread.UTI`) is read all
  the same and renamed at the next save; where a folder has both, the
  one changed last is the resource, and the save leaves that one file.
- **A module folder is worked on in place**, as in Aurora: keep its
  scripts open in an editor of your own while the module is open here.
  Every few seconds (Options › General: reloading), files saved, added
  or deleted in the folder by another program are read again, and the
  log says which. A save never writes over them: a file changed outside
  that you did not change here stays as the other program left it, and
  a file new to the folder is not removed. Where a file was changed
  outside and here both, yours is kept, Save writes nothing and says
  which files, and **Changed Outside Moonglow** asks which to keep. A
  script read again is compiled if **Automatically Compile Scripts on
  Save** is on (Options › Script Editor), as one saved here is, and so
  are the scripts that include it (more than 24 at once are compiled
  in the background, and the log says so). The script editor's external editor
  opens the folder's own file.
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
conversation or blueprint (the first question in a large module reads
all of it; the next ones, for a resource or a tag, only what changed
since). A number typed there that is no resource's name is a talk-table
line's StrRef: the module's names, descriptions, conversation lines and
2DAs that name that line are listed. It's in the right-click menus of the module
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
- **Changes made elsewhere are picked up while the project is open.**
  Every few seconds (Options › General, the reloading switch) Moonglow
  looks for source files that another program changed, added or deleted:
  an editor, a `git checkout` that discards a change, a `git pull`. Those
  are read again, the log names them, and open editors show them. It is
  not unsaved work. If the undo history had changes to one of them, the
  history is cleared (Undo could not take them back from the new file).
- **Where you have unsaved changes to a file that also changed
  elsewhere**, Moonglow keeps yours and the **Changed Outside Moonglow**
  window asks: **Keep Moonglow's** (the next Save writes yours over the
  file), **Take the Files'** (yours are lost) or **Later**.
- **Changes made elsewhere are never overwritten unasked.** Until you
  answer (and with reloading off), a save that would replace or delete a
  file changed on disk since it was read writes nothing, and the log
  names the files.
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

**A language the game has no letters for.** A hak can have an
`encoding.2da`: a table of the Unicode character each of the 256 bytes of
the game's text stands for, shipped with fonts drawn to match (a Turkish
module gives the bytes of "ð" and "þ" to "ğ" and "ş"). Moonglow reads the
table as the game does and shows and writes the module's text by it:
names, tags, descriptions, conversations, the journal, scripts and the
talk table show the table's letters, and a letter you type is saved as
its byte. A letter the table has no byte for is a "?" (or is refused,
where a field says so). The table must be in a hak on the list: the game
does not read one kept in the module itself. A module without one is
read as ever (Windows-1252; Polish text Windows-1250). Plugins' text
follows the table too, and so does the command line where it is given a
module (`mg set`, `apply`, `find`, `info`, `areas`, `replace`, `dialog`):
it needs the game install to find the hak. `mg gff` on a file of its own
is Windows-1252.

**The hak list** (Custom Content) has the module's haks numbered from the
top: where two haks have a resource of the same name, the game takes the
one from the hak higher in the list. A hak that is in none of the hak
folders is marked **not found**: the game will not load the module until
it is there, or taken off the list.
- **Choose** a row with a click; Ctrl + click adds or takes out a row,
  Shift + click chooses from the last one clicked to this one.
- **Move** what is chosen by dragging a row (by its name or the dots
  before it) to where the line shows, with the arrows under the list, or
  with Alt + Up and Alt + Down. Rows chosen together move together.
- **Remove** takes the rows chosen off the list; so does Delete.
- **The keys** work after a click in the list, with the pointer over it:
  Up and Down choose the row above or below (with Shift, more), Home and
  End the first and the last.
- Each move or removal is one step to undo.

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
  feminine table. **Find** finds lines by words or by StrRef. **Go to**
  takes a StrRef or a line's number: press Enter and the list goes to
  that line, among all the others, and it is chosen. **Find References**
  lists where the module names the chosen line (a hak's 2DAs are not
  looked in).
- **A line**: choose it to edit its text, its feminine text, and the
  sound spoken with it and its length. **Copy** copies its StrRef, to put
  in a 2DA or a script. The text boxes take the room under the list; a
  text longer than its box scrolls in it, and the sound's row stays in
  sight. A larger window gives them more.
- **Add Line** adds one at the end. **Remove Last Line** removes only the
  last, since removing another would renumber the lines after it.
- **Undo** and **Redo** in its toolbar undo the table's changes, as do
  Ctrl+Z and Ctrl+Y with the pointer over the editor. (The table is a
  file of its own: elsewhere, and in the main toolbar, Undo is the
  module's.)
- **Export CSV…** writes the lines to a file for a spreadsheet or a
  translator: each line's StrRef, text (and feminine text), sound and
  sound length. **Import CSV…** reads such a file back: each row sets
  the line of its StrRef, which keeps what the file has no column for;
  rows past the table's end add lines. Nothing changes unless every row
  can be read, and one Undo takes the import back.
- **Export JSON…** and **Import JSON…** do the same with the JSON that
  neverwinter.nim's `nwn_tlk` writes and a nasher repository keeps: the
  table's language and the lines that have text, each with its `id` (the
  line's number in the table). An import sets the lines the file has and
  leaves the others; the feminine table is a file of its own there and
  is not touched. (A line's sound length is `soundLen`, as `nwn_tlk` has
  it; files written before 1.20.5 named it `soundLength` and are still
  read.) From a terminal: `mg tlk-export` and `mg tlk-import`.
- **Only lines with text** leaves the empty lines out of the list, for a
  table with reserved ranges. The selected line stays selected: switch it
  off again and the list is at that line, with the empty ones after it
  to fill.
- **Open File…** edits a `.tlk` file anywhere (with its feminine table,
  if a file named with an `f` after its name is beside it) and **New
  File…** makes an empty one: with or without a module open. **Module's
  Table** goes back to the table the module names.
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
resource, or one that replaces the game's own (Options › General). The same
warnings come however the resource arrives: made in an editor, imported,
copied from the palette or renamed. An import that brings many says the
first few and how many more.

## The hak editor

**Tools › Haks** opens a hak: **New Hak**, **Open Hak…** (a `.hak` or an
`.erf`), or **Build Hak from Folder…**, which makes a new hak from a
folder's files, subfolders included, to look over and save.
- **The list:** every resource with its size, and for files just added,
  where they come from. **Find** narrows it by name or type. Click to
  select, Ctrl+click to add to the selection, Shift+click to select from
  the one clicked last to this one; right-click for **View**,
  **Rename…**, **Extract…** and **Remove**. The list is by name, by type
  or by size (the largest first).
- **Viewing:** **View**, or a double click, shows a resource under the
  list: a GFF's fields, a text file's text, a compiled model as the
  text it compiles from, else its first bytes.
- **Compile Models** compiles the hak's models kept as text (ASCII),
  each against its supermodel from this hak, the open module's haks or
  the game, as one step to undo (a hak of many models takes a while:
  the window shows how far it is, and Cancel leaves the hak as it was). A model that doesn't compile, has
  errors Verify Module would name, or whose supermodel isn't found stays
  as text, and the log says why. (`mg pack --compile-models` does the
  same for a folder.) The game reads models as text too: compiling
  makes them smaller and quicker to load.
- **Update from Folder:** a hak built with Build Hak from Folder
  remembers the folder (from one session to the next, once saved):
  **Update from Folder** takes the folder's files again in place of the
  hak's, as they are now.
- **Adding:** **Add Files…** and **Add Folder…** add files. Where the hak
  already has some of them, it asks first, listing them, as Aurora's hak
  editor does: **Replace** them, **Skip Those** and add the rest, or
  **Cancel**. A file the game couldn't read by its name
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
  Right-click a resource in the tree to export it. With **Add to the
  file if it exists**, choosing an archive that is already there adds
  the resources to it (those of the same names are replaced, the rest
  and its description stay) instead of replacing the file.
- **Export as Files…** (in the Export window, and on a resource's
  right-click menu in the module tree) writes resources as loose files,
  `name.ext`, into a folder you choose: as they are now in the toolset,
  saved or not. A script goes with its compiled script (`.ncs`, and
  `.ndb` if it has one), an area with its `.git` and `.gic`. From the
  window, **Include the resources they use** applies too.
- **Export Files…** (over the picture of a placeable, door, item or
  creature in its editor, and in the model viewer) writes the files the
  game draws it with into a folder you choose: its models, their
  walkmeshes (`.pwk`, `.dwk`), textures, materials and their settings
  (`.mtr`, `.txi`), the models its emitters throw and the model its
  animations come from, taken from the haks and wherever else in the
  load order they are. No digging through haks to hand a broken
  placeable to whoever mends it. The game's own files are left out,
  unless all of it is the game's; the log lists each file and the hak
  it came from. Where the folder has files of those names already, it
  asks first: replace them, skip those, or cancel. `mg model-files` does
  the same from a terminal.
- **Copy to Scratch Folder** (the same right-click menu; **To Scratch** in
  the script editor and the area view) writes them into the scratch
  folder: one you choose the first time, kept from then on (Tools ›
  Options › Folders changes it). Make it the game's or a server's
  `development` folder, which the game loads before the module's own
  resources: compile a script, send it to scratch, and a running server
  or game has the fix. (Aurora users pick such files out of its `temp0`
  folder; Moonglow keeps a module in memory, so it hands them over
  instead.) A script changed since it was compiled is compiled
  first, wherever it is sent from, and the log names it; one that no
  longer compiles goes with the compiled script it has, and the log says
  that too.
- A module opened as a folder (**File › Open Folder…**) or kept as a
  nasher project has its resources as files already.
- **File › Import…** adds an `.erf`'s resources to the module; for those
  the module already has, you choose which to overwrite. It also lists
  what the imported resources refer to that neither the module nor the
  game has.

## Areas

**Wizards › Area Wizard…** makes an area from a name, a tileset and a
size (2 to 32 tiles each way). It opens with the first tileset chosen
(Aurora chooses none). **Edit › Resize Area…** and **Rotate Area…**
change the area shown last, as in Aurora. See [Areas](04-areas.md).
