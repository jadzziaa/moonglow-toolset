---
type: Manual Page
title: Options
description: The Options dialog page by page (folders, area, general, script editor, conversation editor, sounds, language, keyboard) and where the settings are kept.
tags: [manual, options]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-04T02:52:17Z }
---

# Options

**Tools › Options…** has Aurora's pages. **OK** applies the changes.

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
- **List areas and blueprints by name** in the module tree, rather than
  by ResRef, and **Show ResRefs beside names** (`Name (resref)`, in the
  tree and the palettes).
- **Show challenge ratings in the creature palette**: on by default.
- **Reload haks, override and development when they change**: on by
  default. Moonglow checks every few seconds and rereads what changed
  (see [Build, verify and test](09-build-and-test.md)).
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
- **Automatically Compile Scripts on Save**.
- **Generate Debug Information When Compiling Scripts** (`.ndb` files).
- **External Script Editor**: the program the External Editor button
  opens scripts in.
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
