---
type: Changelog
title: Plugin API changes
description: What changed in each version of the plugin API, and what a plugin written for the version before has to change.
tags: [plugins, api, changelog]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-22T20:17:20Z }
---

# Plugin API changes

A plugin's manifest names the API it was written for (`api = "0.2"`).
Moonglow runs a plugin only if it has that API; while the API is at 0.x,
each version may change what the one before had. This file lists each
version's changes and what a plugin written for the version before has
to change.

## 0.4

In Moonglow 2.1.0. In short: a plugin learns what the user is
working on (the selection, what is open) and is offered in the menus of
what it is for; it comes with a project; it keeps settings; a check
offers fixes; a plugin has tests of its own; and it acts on what the
user does: before a save (warning of it or refusing it), and on what a
paste, a placing or a wizard makes (editing it within the same step of
Undo).

Only additions: a
plugin written for 0.1, 0.2 or 0.3 runs as it is (its `ctx` has the new
fields too, `selection`, `open` and `settings`, and a check's `fix`;
nothing it had changed, but for a finding's `fix`, below). A plugin that
uses what is new says `api = "0.4"`; a manifest that gives a command a
`for`, declares a `[setting]`, or an `[event]` on one of the new
events, and says an earlier API is refused, with the reason.

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
- Quick fixes: a finding's `fix`, made in a check with
  `ctx.fix(label)`, whose methods are `ctx.edit`'s and record edits
  without making them. Verify Module shows a button with the label
  beside the finding, and makes the edits only when it is clicked, as
  one step of Undo; `mg verify --fix CHECK` makes a check's fixes, and
  `mg verify --json` gives each finding's fix as an edit file. A
  finding's `fix` that is not one made with `ctx.fix` is now an error
  (before, a key the finding did not have was passed over).
- A plugin's own tests: files `tests/*.toml` in its folder, each naming
  a fixture module and the checks, commands and events to run on it,
  with what is to come of each; `mg plugin test DIR` runs them, with an
  exit and a summary for a build pipeline. `mg plugin pack` leaves
  `tests/` out of the archive. The `tag-conventions` example has both.
- Settings: a `[setting]` section in the manifest for each value the
  plugin keeps (`id`, `type`: `string`, `int`, `bool`, `choice` or
  `path`; `default`, `label`, `help`, `choices`, `min`, `max`; `scope`:
  `user`, kept in Moonglow's settings, `project`, in the project's
  `moonglow.toml` under `[plugins."<id>"]`, or `user-project`, in its
  `.moonglow/user.toml`). The plugin reads them as `ctx.settings.<id>`
  (and `for id, value in ctx.settings`) and sets one with
  `ctx.settings:set(id, value)` (nil: its default), which Moonglow keeps
  when the job is done; a check only reads them. Manage Plugins has a
  form for them (Settings…). `mg plugin run --setting ID=VALUE`, and a
  test's `settings` and `set` (`mg plugin test`). The `builder-tags`
  example uses them.
- A project's own plugins: plugins in a project's `plugins/` (or the
  folder its `moonglow.toml` names, `[plugins] folder`) come with its
  checkout and run once the builder allows them, asked when the project
  is opened; the answer holds for the plugin's content (a digest of its
  files, which it runs from), so a changed one is asked about again, and
  is kept in Moonglow's own folder, never in the project (a checkout
  can't come with its plugins allowed). No change to a plugin's code: one
  of a project is written as any other. `mg verify` runs a project's
  plugins as the builder allowed them, or every one with
  `--allow-project-plugins`; `mg plugin list --project` lists them and
  `mg plugin allow` answers for them.
- A resource's own events: `on = "resource-saving"` (before a save,
  for each resource it would write: `ctx.event.resource`, its `kind`,
  `old` and `new` as plain values or text, `changed` as lines; the
  handler warns, `ctx.event:warn(text, fix?)`, which the user is asked
  about before anything is written, or refuses, `ctx.event:refuse(text,
  fix?)`, which stops the save; a fix is one made with `ctx.fix`) and
  `"resource-saved"` (after, for each resource written). Around a save
  a plugin only reads: a handler of either that edits fails. A manifest
  narrows them to types of resource with `types = "are, git"`.
- Objects placed or pasted: `on = "object-placed"` and
  `"object-pasted"` (`ctx.event.objects`, entries as a selection's;
  `ctx.event.area`, `ctx.event.by`), narrowed with `for` as a command
  is. Their edits are part of the placing's or the paste's own step of
  Undo.
- The after-create hook: `on = "resource-created"`, for each resource a
  wizard (area, creature, item, placeable, door…), Add to Palette, Setup
  Store or a copy makes (`ctx.event.resource`, `kind`, `made`, `by`),
  narrowed with `for`; it may edit what was made at once, in the same
  step of Undo, after Moonglow's own work (the builder code, the
  project's store rules).
- The events that run within what the user does (all but those after a
  save, and `module-opened`) have a time budget of 5 seconds a plugin,
  nobody to ask and nothing written outside the module; one that fails
  is said and left out (before a save, a warning: never a refusal).
- `ctx.event.resources` is also every resource a save writes, for a
  resource saving or saved.
- With `mg`: `mg set` and `mg apply` take `--plugins` and run the save's
  events (a refusal is an error, nothing written); `mg plugin run
  --event` runs the new events' handlers (`--was`, `--select`, `--by`);
  a plugin's tests take `was`, `by`, `select` for an event, and hold a
  save's handler to what it says (`warned`, `refused`); `mg plugin list`
  says what each event does. The `house-rules` example has all three.
- From the Domains of Dread's builder proposal (A1 to A6, and A8).

Since Moonglow 2.1.1, still as API 0.4 (only additions: a plugin that
says `api = "0.4"` runs as it did):

- `mg.struct_id(struct)`: a struct's id, of a table read with `gff`
  (where it is kept beside the table, as its path is, so that its
  fields are all the table holds) or with `raw` (its `__struct_id`). An
  equipped item's is its slot, which needed `raw` before.
- A test's `[[test.field]]` may name a list's item (`Equip_ItemList[2]`):
  there, or gone with `absent = true`.
- A plugin's tests bring their own game data: a folder `game/` beside
  the test file (or the one `game = "…"` names, in the file or a test),
  whose files `ctx.game` reads over the game's, as the game's development
  folder is; a test that reads only those runs with no game. `root =
  "…"` names the game a test reads, in place of the one `mg` finds. A
  test that fails having read something its game data has not says which,
  and where to put it.
- From the Domains of Dread's notes on setting up their project.

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
