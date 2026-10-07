---
type: Manual Page
title: Options
description: The Options dialog page by page (folders, area, general, script editor, conversation editor, sounds, language, keyboard) and where the settings are kept.
tags: [manual, options]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-07T13:30:00Z }
---

# Options

**Tools › Options…** has Aurora's pages, in a window that docks and
tabs with the others. **OK** applies the changes; the module stays open.
**Cancel**, closing the window or Escape discards them. Only another game or user folder (the Folders page)
has the game's data read again: unsaved work is asked about first, and
the module is opened again from its file.

## Folders

- **Neverwinter Nights installation**: the game's folder (with `data/`
  and `bin/` in it). Moonglow reads the game's data from here.
- **NWN user folder**: where your modules, haks, talk tables, override and
  characters are.
- **Scratch folder**: where **To Scratch** (the script editor, the area
  view) and **Copy to Scratch Folder** (the module tree) copy a script
  with its compiled script, or an area, as loose files. Empty: asked for
  the first time. See [Modules](03-modules.md).

## Area

- **Background Color** of the area viewer (Custom, or the default).
- **Show Encounter Spawnpoint Markers**, and their **Height** and
  **Width**: at each spawn point, the game's marker (see-through), a post
  of that size, and an arrow the way what spawns there faces.
- **Show merchants as $ signs**: the game's marker for a merchant, rather
  than an arrow along its facing.
- **Show the turning and tilt rings around selected objects**: the ring
  that turns the selection when dragged round, and the two that tilt
  its models, with the arrows that move it along one axis, while Shift
  is held (see [Areas](04-areas.md)). On by
  default.
- **Show Door Orientation Arrows**.

## General

- **Create backups of modules**: each save also keeps the module as it
  was in `<name>.BackupMod`.
- **Build module on save**: run Build Module (with its defaults) before
  each save.
- **Minimize Toolset on test module**.
- **Interface size**: the whole interface, text and all, from 90% to 200%
  of its usual size.
- **Light theme**: dark text on light, rather than the dark theme.
- **Open a module on the area opened last**: on by default; the first
  time, the first area the module tree lists (see
  [Modules](03-modules.md)).
- **Write a debug log**: what Moonglow does, step by step, in
  `debug-log.txt` in its data folder (see
  [Troubleshooting](13-troubleshooting.md)).
- **List areas and blueprints by name** in the module tree, rather than
  by ResRef, and **Show ResRefs beside names** (`Name (resref)`, in the
  tree and the palettes). Areas are by name in Find Instance and the
  area transition's destinations too.
- **Show challenge ratings in the creature palette**: on by default.
- **Read Steam Workshop content**: on by default. The Steam Workshop
  items you are subscribed to are read with the game's data, as the game
  started through Steam reads them: each item's `override` under your
  own, its haks and talk tables found by name. Aurora doesn't read them;
  turn this off to see the module as a player without them would.
  Changing it reads the game's data again.
- **Reload haks, override and development when they change**: on by
  default. Moonglow checks every few seconds and rereads what changed
  (see [Build, verify and test](09-build-and-test.md)). The same switch
  has a nasher project's own files read again when another program
  changes them (see [Modules](03-modules.md)).
- **Keep a recovery copy of unsaved work every … minutes** (5 by
  default; see [Modules](03-modules.md)).
- Warnings: **Show reserved Blueprint ResRef namespace warning** (a
  blueprint named like the game's, `nw_` or `x0_` to `x3_`), **Show
  standard resource overwrite warning** (the module adds a resource the
  game has), **Show resource in Hak Pak warning** (a hak has a resource
  the module adds, and the game uses the hak's), **Show invalid creature
  spell assignment warning** and **Show creature inventory warning**
  (see [Blueprints](05-blueprints.md)).

## Script Editor

- **Code Templates Directory**: your own templates, listed with the
  game's.
- **Automatically Compile Scripts on Save**: a script's own Save, and
  saving the module, compile the scripts whose text they save.
- **Generate Debug Information When Compiling Scripts** (`.ndb` files).
- **External Script Editor**: the program the External Editor button
  opens scripts in.
- **Open scripts in the external editor**: a script opened from the
  module tree opens there as well; what it saves comes back into
  Moonglow's editor. In a nasher project the external editor gets the
  project's own file (see [Scripts](07-scripts.md)).
- **External Script Compiler**: a compiler program to use in place of the
  built-in one (empty: the built-in one, which is the game's and
  Aurora's own). Compile in the script editor, Compile All Scripts, the
  build and what is compiled before a test or an export go through it.
  Errors as you type stay the built-in compiler's.
- **Its arguments** (shown once a compiler is chosen): how the program is
  run. Empty, Moonglow uses the usual line for the program, by its name:
  `nwn_script_comp`'s (neverwinter.nim), or `nwnsc`'s. The line shows in
  the empty field, and resting the pointer on it lists the places
  Moonglow fills in: `{files}` (the scripts to compile), `{src}` (a
  scratch folder the module's scripts are written to), `{out}` (the
  folder the compiled scripts are read from), `{game}`, `{user}` and
  `{haks}` (the module's haks, comma-separated). An option whose place
  has nothing to give is left out. Generate Debug Information adds `-g`
  to the usual lines.
- The editor's **font size** and syntax **colors**.

## Conversation Editor

- **Show popup when creating a new text entry**: a new line asks for its
  text first.
- **Show speaker name before text**, and the **NPC** and **Player** text
  colors.
- **Paste Link Options** and **Drag Link Options**: whether Paste As
  Link and Ctrl + drag link the source to the destination or the reverse.
- **Automatically backup the conversation files** every … minutes.

## Sounds

- **Play placed sound objects in area**, **Play ambient sound in area**,
  **Play ambient music in area**: what plays when an area view opens (the
  area viewer's toolbar also turns each on and off).
- **Ambient music volume**.

## Language

The language that text is shown and edited in, for modules made for
players of another language. Names, descriptions and conversation lines
show and edit that language's text, and String Edit opens on it.
Where a text has none in that language, its text in another is shown
instead, English if it has any (as Aurora shows a name written in
English alone): in the palette, the module tree, a conversation's lines
and the journal's lists as it is, and in the fields that edit it (a
blueprint's, an area's, Module Properties', the journal's, a
conversation line's) in the talk table's color with the language named
beside it. Typing there gives it text in the language you edit.

Where your game has that language installed (a folder for it under the
game's `lang`), the game's own text is read in it too: names and
descriptions kept as talk-table references, and the names of races,
classes, feats and the rest. Otherwise they stay in English. Choosing
another language here reads the game's data again, and the open module
is opened again from its file (Moonglow asks first if it has unsaved
changes).

## Keyboard

Every command and its keys, by where it works (anywhere, the area view,
the script editor, the conversation editor). Every command of the menus
is listed, in the menus' order, whether it has a key or not: give Build
Module or Verify Module one here. **Find a command** narrows the list to
what you type (a command's name, or its group's).
- **+** then the keys adds a key (Escape: none); a key's **×** removes
  it. A command can have several keys, or none.
- **Reset** gives a command Moonglow's keys back, **Reset All** every
  command.
- **Export…** writes the keys you chose as a file, to keep or to hand to
  someone; **Import…** takes such a file's keys in place of yours (every
  command the file doesn't name goes back to Moonglow's keys). As with
  the rest of Options, OK keeps them.
- **Conflicts:** a key two commands share where both work is named above
  the list. (A key of the whole window wins over the editors' own.)
- The commands of enabled [plugins](15-plugins.md) are listed under
  **Plugins**, with the key each plugin suggests; they take keys like
  any other.

Keys match exactly: Shift+Q is not Q. Text editing, Escape and Enter,
Copy, Cut and Paste, Delete in the area view, and the script editor's
numbered bookmarks (Ctrl and a digit) keep their keys. A letter or digit
given to a command of the whole window (alone or with Shift) stays a
character while a text field has the keyboard; with Ctrl or Alt, and
function keys, work there too.

## Where the settings are kept

| System | Folder |
| --- | --- |
| Linux | `~/.local/share/moonglowtoolset` |
| Windows | `%APPDATA%\Moonglow Toolset\data` |
| macOS | `~/Library/Application Support/Moonglow-Toolset` |
