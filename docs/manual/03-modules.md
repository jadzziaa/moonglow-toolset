# Modules

## Opening and saving

Moonglow opens modules as `.mod` archives, as folders (a module unpacked
into a directory) and the game's campaign files (`.nwm`, read as modules).

- **File › Open Module…** (Ctrl+O), or **File › Recent Modules**, or a
  module named on the command line (`moonglow path/to/module.mod`).
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
