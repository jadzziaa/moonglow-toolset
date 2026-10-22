---
type: Changelog
title: Plugin API changes
description: What changed in each version of the plugin API, and what a plugin written for the version before has to change.
tags: [plugins, api, changelog]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-12T04:32:35Z }
---

# Plugin API changes

A plugin's manifest names the API it was written for (`api = "0.2"`).
Moonglow runs a plugin only if it has that API; while the API is at 0.x,
each version may change what the one before had. This file lists each
version's changes and what a plugin written for the version before has
to change.

## 0.4

Not yet in a release. Only additions: a
plugin written for 0.1, 0.2 or 0.3 runs as it is (its `ctx` has the two
new fields too, and nothing it had changed). A plugin that uses what is
new says `api = "0.4"`; a manifest that gives a command a `for` and says
an earlier API is refused, with the reason.

- `ctx.selection`: what a command is run on, a list of entries that each
  say what it is (`kind`: `"placeable"`, `"area"`…; `blueprint`;
  `module`), where it is (`resource` and `path`, as `ctx.edit` takes
  them; `area`, `index`) and its `tag` and `resref`. From a right-click
  menu, what was clicked; from the Plugins menu, the Command Palette or
  a key, the objects selected in the area view in front or else the
  blueprints selected in the palette. Empty in checks and events.
- `ctx.open`: the area or editor in front, as such an entry, or nil.
- A command's `for` in the manifest (`for = "placeable, creature"`, or
  `"area"`): the command is in the right-click menus of those kinds, in
  the area view, the module tree and the palettes, above the
  Variables and Properties that end a menu. A command with no `for` is
  in the Plugins menu alone, as before; one with it is there too.
- `mg plugin run` takes `--select` (as often as needed: `guard.utc`,
  `tavern.are`, `tavern.git/Placeable List[3]`) and `--open`, and
  `mg plugin list` says what each command is for.
- From the Domains of Dread's builder proposal (A1).

## 0.3

In Moonglow 1.21.0. Only additions: a plugin written for 0.1 or 0.2 runs
as it is. A plugin that uses what is new says `api = "0.3"`.

- `mg.twoda`: `parse` and `new` make a 2DA a plugin can change
  (`twoda:set`, `add_row`, `add_column`) and write out (`twoda:write`,
  laid out as the game's own tables are), and `kinds` says which columns
  of a table hold a talk-table string or a model, as Verify Module knows
  them. What `ctx.game:table` gives can be changed and written too: it
  is the plugin's own copy. Since Moonglow 1.21.2 `kinds` also says
  which columns name a row of another table (`"row:feat"`), still as
  API 0.3: a plugin that reads only `"strref"` and `"model"` is as it
  was. Since Moonglow 1.23.0 `mg.twoda.describe` reads a table's
  description (`name.2da.toml`) and holds a table to it, still as API
  0.3.
- `mg.tlk`: `parse` and `new` make a talk table to read, change and
  write (`tlk:get`, `set`, `add`, `write`).
- `folder:write`: files written into a folder the user chose
  (`ctx.ui:open_folder`), which they are asked to allow: into that
  folder only, by name.
- Events: an `[event]` section in the manifest (`on = "module-opened"`
  or `"module-saved"`) and `mg.on(id, handler)`; the handler runs as a
  command does, with what happened as `ctx.event`. In the window only;
  from Moonglow 1.21.3, `mg plugin run --event` runs one too.

## 0.2

In Moonglow 1.16.0. Only additions: a plugin written for 0.1 runs as it
is, and its manifest may go on saying `api = "0.1"`. A plugin that uses
what is new says `api = "0.2"`, so that a Moonglow without it refuses it
plainly.

- `ctx.ui:open_file` and `ctx.ui:open_folder`: a file or a folder the
  user chooses, to read (a folder's files, and nothing outside it).
- `ctx.hak:write` and a folder's `to_hak`: resources for a hak of the
  user's hak folder, which the user is asked to allow; `ctx.hak:attach`
  lists a hak in the module. What a job puts into a hak, `ctx.game`
  reads in that job.
- `ctx.terrain`: `tilesets`, `new_area` (an area as the Area Wizard
  makes it) and `open`, an area's terrain to read and paint as the area
  editor's brushes do: `corner`, `tile`, `paint`, `raise`, `set_height`,
  `place_group`, `set_tile`; and, from Moonglow 1.16.1, `cross` (roads,
  streams, walls), `erase` and the list `crossers`.
- `mg.image`: a PNG, TGA or DDS picture decoded, to read its pixels.
- `mg plugin run` takes `--file` (what the plugin gets where it asks for
  a file or folder) and `--hak-dir`.

## 0.1

The first version, in Moonglow 1.0.0. Experimental.

- A manifest, `plugin.cfg`: `[plugin]`, and a `[command]` or `[check]`
  for each thing added.
- `mg.command` and `mg.check` register handlers; typed values
  (`mg.int`…) and `mg.path`.
- A handler's `ctx`: `ctx.module` and `ctx.game` to read, `ctx.edit` for
  a command's changes (fields, list items, whole resources),
  `ctx.log`, `ctx.progress`, `ctx.ui` (message, confirm, form) and
  `ctx.plugin`.
- `require` for the plugin's own files.
