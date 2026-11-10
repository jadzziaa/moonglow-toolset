---
type: Changelog
title: Plugin API changes
description: What changed in each version of the plugin API, and what a plugin written for the version before has to change.
tags: [plugins, api, changelog]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-09T00:38:51Z }
---

# Plugin API changes

A plugin's manifest names the API it was written for (`api = "0.2"`).
Moonglow runs a plugin only if it has that API; while the API is at 0.x,
each version may change what the one before had. This file lists each
version's changes and what a plugin written for the version before has
to change.

## 0.9

In Moonglow 3.18.0. Only an addition: a plugin written for 0.8 or
before runs as it is, and has nothing to change. A plugin that uses
what is new says `api = "0.9"`, so that an older Moonglow refuses it
with the reason rather than failing in its panel.

- A panel's **fields**: an element `{ type = "field", kind = …, id,
  label, value, … }`, a form's field in a panel (`kind` is what a form's
  field's `type` is: `"text"`, `"number"`, `"check"`, `"choice"`,
  `"color"`; `value` is what it shows; `min`, `max`, `choices` and a
  color's `variable` as a form's). Moonglow keeps each field's value, by
  its id, while the panel is drawn again: the field takes the `value`
  the handler gives when it is not the one it gave the last time, and
  otherwise keeps what the user set. A color's `variable` draws the
  color on the placed objects selected while its picker is open, as a
  form's does. With `command`, one of the plugin's commands, the field
  runs it when the user sets it: a color when its picker closes, text
  when it is left, a number when it is let go, a check or a choice at
  once. The example `tint-panel` is a panel whose color field shows the
  tint of the placeable selected and sets it.

  ```lua
  mg.panel("tint", function(ctx)
    return {
      { type = "field", kind = "color", id = "tint", label = "Tint",
        value = "#2a4d8f", variable = "tdod_tint", command = "set-tint" },
    }
  end)
  ```

- `ctx.values`: a command run from a panel (its button, a list's item,
  a field set) gets the panel's fields' values, by field id, as a form's
  come back. Run from anywhere else (a menu, a key, a fix, `mg plugin
  run`), and in every other handler, it is an empty table. A plugin's
  own test gives a command's with `values = { … }`.
- A choice's options (a form's and a panel's) may be `{ text =
  "Crimson", color = "#a01010" }`: a swatch of the color is drawn before
  the text, in the list and in the box. Its value is still the text.
- The window of a plugin's job (its questions and forms) is headed by
  the command's title alone (`Tint`, not `Pick tint: Tint`; the plugin's
  name shows on hover), and no longer dims the window behind it.

Since Moonglow 4.0.0, still as API 0.9, a plugin meets limits and
refusals it did not before ([Limits](../manual/17-plugin-api.md#limits)).
A plugin that stayed within what the manual said has nothing to change;
one that leaned on what follows fails where it used to go on:

- What Moonglow holds for a job besides the plugin's own 256 MB (the
  tables and talk tables it makes, the pictures it has decoded, its
  edits and the files it wants written) may be 512 MB: the call that
  would go past that fails, and the job with it unless it catches the
  error. What the plugin has let go of does not count.
- `folder:files()` fails on a folder of more than 20,000 files; it
  returned the first 20,000 without a word.
- `ctx.edit:write`, `write_bytes` and `write_gff` refuse a new resource
  whose name no file can have (`a/b.nss`): a folder module or a nasher
  project could not be saved with it. A resource the module has already
  is written as before.
- A number that is not finite (`0/0`, `math.huge`) is refused where a
  plugin or its user gives one: a form's `default`, `min` and `max` (and
  a `min` above its `max`), an answer of `mg plugin run --answer`, a
  float field set with `ctx.edit:set` past a float's range, a pixel's
  place in `image:pixel`.
- `mg.image` refuses a picture of more than 64 million pixels by its
  header, before decoding it, and `ctx.game:bytes` reads no more than
  64 MB of a file put into a hak with `folder:to_hak`, as `folder:bytes`
  does.
- A plugin's folder of more than 16 MB or 500 files is refused before
  any of it is read, and `mg plugin check`, `mg plugin pack` and an
  archive about to be installed give a plugin's code 5 seconds to load.
- `ctx.edit:set` of a ResRef field keeps a letter outside ASCII as the
  module's codepage has it (`café_01`), and `ctx.module:gff`, `text` and
  the like take a resource's name as such a field reads (`œuvre.dlg`, a
  Polish module's letters) as well as the way `ctx.module:resources()`
  lists it.
- A choice (a form's field, a panel's) has at most 2,000 options, each
  of at most 1,000 characters; one of more, or with a longer option, is
  refused with the reason (the form is not shown, the panel shows the
  error). An option's text is its value, so it is not cut.
- A number field starts within its `min` and `max`: what it is given
  outside them (or 0, given nothing, with a `min` above it) is held to
  them. A panel's number field no longer runs its `command` when the
  panel is drawn with such a value.
- A panel's button or item that names a place (`on`, `open`) acts only
  on the module the panel was drawn from. Clicked after the module
  changed (and before the panel was drawn again), it does nothing, the
  log says so, and the panel is drawn again; a panel whose manifest
  says `refresh = "edits"` is seldom caught so. A button that names no
  place runs on what is selected, as before.

## 0.8

In Moonglow 3.16.0. Only an addition: a plugin written for 0.7 or
before runs as it is. A plugin that uses a color field says
`api = "0.8"`, so that an older Moonglow refuses it with the reason
rather than failing at the form.

- A form's field (`ctx.ui:form`, a wizard's page) may be a **color**:
  `type = "color"`, its `default` and its value `"#rrggbb"`. Its swatch
  opens the picker the Properties windows have (paste a color into it,
  copy one out). With `variable`, the name of a string variable, the
  placed objects the command was run on are drawn in the area with that
  variable set to the color while it is being picked, as the Variables
  page draws a tint being picked; nothing is set until the plugin sets
  it. `mg plugin run` takes one as `--answer ID=#rrggbb`. The example
  `pick-tint` is a right-click command that picks a placeable's
  `tdod_tint`.

  ```lua
  local answers = ctx.ui:form({
    title = "Tint",
    fields = {
      { id = "tint", label = "Color", type = "color", default = "#2a4d8f", variable = "tdod_tint" },
    },
  })
  ```

## 0.7

In Moonglow 3.14.0. Only an addition: a plugin written for 0.6 or
before runs as it is. A plugin that puts its commands in submenus says
`api = "0.7"`; a manifest with `menu` and an earlier `api` is refused,
with the reason.

- Submenus: a `[command]`'s `menu` names the submenu it is in, nested
  with `›` (or `>`) between the names, in every menu the command is in
  (its right-click menus and the Plugins menu). Each submenu is made
  once, where the first command naming it is, its commands in the
  manifest's order; a command without `menu` stays in the menu itself.
  The Command Palette, Options › Keyboard, the Plugins window and `mg
  plugin list` name the command by its whole path (`Make › Crafting ›
  Workbench…`), and `mg plugin list --json` gives it as `menu`.

  ```ini
  [command]
  id = "workbench"
  title = "Workbench…"
  for = "placeable"
  menu = "Make › Crafting"
  ```

## 0.6

In Moonglow 3.3.0. Only additions: a plugin written for 0.5 or before
runs as it is. A plugin that uses what is new says `api = "0.6"`; a
manifest with any of the new sections and an earlier `api` is refused,
with the reason. The design is the proposal's "API 0.6".

- Exporters: `[exporter]` (`id`, `title`, `hint`, `to = "file"` or
  `"folder"`, `extension`, `for`) and `mg.exporter(id, function(ctx,
  out))`, in File › Export: the user chooses the file or the folder
  first, and the exporter writes it with `out:write(bytes)` (a folder's
  files with `out:write(name, bytes)`; `out.name` is its name). It only
  reads; one that edits, or writes a hak or elsewhere, fails. With
  `for`, it exports what is selected, and is offered only then. `mg
  plugin export MODULE PLUGIN ID OUT`.
- Importers: `[importer]` (`id`, `title`, `hint`, `from = "file"` or
  `"folder"`, `extensions`) and `mg.importer(id, function(ctx, source))`,
  in File › Import: `source` is the file or the folder the user chose,
  as `ctx.ui:open_file` and `open_folder` give them; its edits are one
  step of Undo, named by what it returns or "Import" and its title. `mg
  plugin import MODULE PLUGIN ID SOURCE`.
- Wizards: `[wizard]` (`id`, `title`, `hint`) and `mg.wizard(id, {
  pages = { … }, finish = function(ctx, answers) })`, in the Wizards
  menu: pages asked in turn with Back, Next and Finish (each a table, or
  a function of the answers so far, nil to skip it; `check` keeps a page
  with what is wrong), `answers` by field id, the edits at Finish as one
  step of Undo, Cancel making nothing. `mg plugin wizard MODULE PLUGIN
  ID`, its pages answered by `--answers FILE` (JSON) and `--answer`;
  `mg plugin run` takes `--answers` too.
- Panels: `[panel]` (`id`, `title`, `hint`, `refresh = "edits,
  selection"`) and `mg.panel(id, function(ctx) return { … } end)`, a pane
  of Plugins › Panels showing the elements it returns (`heading`,
  `text`, `value`, `separator`, `button` running one of the plugin's
  commands, `list` of items that open what they name or run a command),
  drawn by Moonglow; run beside the window, within 250 ms, at most 2,000
  elements; it only reads. Not in `mg`.
- A plugin's own tests: `export`, `import`, `wizard` and `panel`, with
  `to` and `content` (an export's file and what it has) and `shows`
  (what a panel shows). `mg plugin list` lists the new kinds.
- The host: `Question::Page` and `Answer::Page` (a wizard's page and its
  answer), and `Host::interactive` (whether a page's check that objects
  asks again, or ends the wizard).

## 0.5

In Moonglow 3.3.0. Only additions: a plugin written for 0.4 or before
runs as it is. A plugin that uses what is new says `api = "0.5"`; a
manifest that declares what 0.5 added and says an earlier API is
refused, with the reason.

- `area:batch(f)`: an area's strokes made at once. Its tiles are read
  once, `f`'s strokes made on them (what it reads back is what they
  left), and the tiles that changed go in when it returns, as one edit
  of the area's tile list; if it fails, none of them. A stroke that
  changes many tiles outside a batch is one edit of the list too.
- Settings of three more kinds, `float` (a number with a fraction,
  with `min` and `max`), `list` (texts; with `choices`, each one of
  them) and `color` (`"#rrggbb"`); a `string` held to a `pattern` (a
  regular expression its whole text matches); a `path` whose form picks
  a folder (`pick = "folder"`). `ctx.settings` reads a float as a
  number and a list as a table of texts, and `ctx.settings:set` takes
  them so. A manifest with any of these and an earlier `api` is refused.
- A command's `for` (and an event's) takes a kind's placed objects
  alone (`"placed creature"`) or its blueprints alone (`"creature
  blueprint"`), where `"creature"` is both; `tile`, the tiles selected
  in an area's view (an entry's `kind` `"tile"`, with `index`, `x` and
  `y`); and another resource's type (`nss`, `dlg`…). The commands are
  also in the menus of a palette's category and a module tree's group
  (run on every one of them), a Properties window's tab, the script
  editor, and the rows of Find References and Advanced Search; an
  area's are in the area view's menu with objects selected too (**The
  Area**). From the Plugins menu, a command runs on what is chosen in
  the pane last clicked in. `ctx.open.together` lists those edited
  together in one Properties window. `mg plugin run --select` takes a
  tile as `tavern.are/Tile_List[5]`.
- A fix that asks: `ctx.fix(label, { command = "retag", on = s? })`,
  a finding's fix whose button runs one of the plugin's commands on what
  the finding is about (`on`, or the finding's resource), which asks with
  its form. It records no edits; `mg verify --fix` does not make it.
- Events: `resource-created` fires for the Script Wizard, New Script
  (`by = "new-script"`), a new NUI window (`"new-nui"`) and an import
  (`"import"`), and an event's `for` takes a resource's type (`nss`);
  Save As asks the `resource-saving` handlers as Save does;
  `ctx.event.changed` gives a script's (a text's) lines that differ;
  the time at-once handlers have is set in Manage Plugins; `mg set` and
  `mg apply` take `--strict`, and `mg rename`, `replace`, `areas`,
  `update-instances` and `import` run the project's plugins' save events.
- A plugin's tests: `haks` and `written` hold what a job writes
  outside the module (nothing is written); `mg plugin test` takes a
  folder of plugins; `mg plugin check` reads the test files; Manage
  Plugins runs them (Run Tests).
- Settings of scope `user` are kept in Moonglow's own folder
  (`plugin-settings.json`), which `mg` reads and keeps too: with `mg
  plugin run` a plugin reads the values the user set in the window, and
  what it sets of that scope is kept.

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

Since Moonglow 2.2.0, still as API 0.4 (only additions: a plugin that
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

Since Moonglow 2.3.0, still as API 0.4 (only additions: a plugin that
says `api = "0.4"` runs as it did, and its tests read as they did):

- A test's `root = false` (in the file, or in one test) reads no game:
  its game data is its game folder and the fixture alone, whatever game
  `mg` finds; `root = true` reads that one, over a file's `false`. A
  root that is no game names `root = false` in its error.
- A test that passes having read from the game what its game data has
  not (in a `pcall`) is told so in a note: under its line, in the JSON
  (`notes`, `noted`) and the summary; it stays passed. A test with no
  game that lists the game's resources (`ctx.game:resources()`) is
  told the list is only the fixture's and its game folder's.
- `builtin = "id"` in a test runs one of Moonglow's own checks
  (`var-runtime`, `tag-unique`…) on the fixture, by the fixture's
  project file, held as a plugin's check is (`[[test.finding]]`, `fix`,
  `left`, `[[test.field]]`). The `tag-conventions` example has one.
- From the Domains of Dread's notes on 2.2.0.

Since Moonglow 2.13.0, still as API 0.4 (only additions: a plugin that
says `api = "0.4"` runs as it did; of its tests, those that held what
changed, below, to what it was):

- `mg.vars.describe(name)` and `mg.vars.described()`: the variables the
  module's project describes (`variables.toml`), each a table (`name`,
  `pattern`, `type`, `on`, `values`, `help`, `runtime`…), as the
  Variables window and `mg vars` read them.
- `ctx.module:new_name(name, kind)`: a name for a new resource as the
  window proposes one, with the builder's code in the project's pattern,
  the first free (`ab_gateguard.utc`).
- A test's `builtin` takes a list of Moonglow's checks, or `"all"`.
- A test's game folder may bring a `dialog.tlk`, the fixture's custom
  talk table and, in its `hak/`, the fixture's haks; with no talk table
  to count by, `2da-strref`, `custom-tlk` and `2da-described` say
  nothing of strings they can't count (a test that names one of the
  first two is told it can't test it, one that runs them as part of
  more, in a note).
- With no game and no game folder, a test's `ctx.game` is the fixture
  (it was empty), as Moonglow's is the game with the module over it.
- What is `left` after a test's fixes is found on the module as fixed
  through the game's data too (a fixed 2DA of the module's, read with
  `ctx.game:table`), not only through `ctx.module`.
- `mg.twoda.kinds` knows more: `"table"` (a column naming another
  table, `classes.FeatsTable`), a class's requirements (`cls_pres_*`'s
  `ReqParam1`, a table `{ by, rows }`: a row of the table its `ReqType`
  chooses), the item property tables (`itempropdef.2da`'s strings and
  rows, the `Name` of the tables it names), `appearance.FOOTSTEPTYPE`,
  and, with the game's data, a table by the column that names it.
- `twoda:write()` of a table read from a file keeps that file's layout
  (the file itself, where the table is unchanged).
- `mg.tlk.new(language?)`, `tlk.language` and `tlk:set_language(code)`:
  a talk table in another language than English, its text in that
  language's codepage.
- From the Domains of Dread's notes on 2.1.0 and 2.2.0, and their
  builder proposal (C1).

Moonglow 3.0.0, still as API 0.4 (only additions: a plugin that
says `api = "0.4"` runs as it did):

- An overridden copy the resource browser opened (a resource as a lower
  layer of the load order has it) is `ctx.open`, with its type as
  `kind` and its layer's label as `selected.layer` (it was nil); `mg
  plugin run --open name.ext@LAYER` names one.
- `ctx.game:gff`, `text` and `bytes` take a layer's label after the
  name, and read that layer's copy.
- From a builder's report on 2.0.0.

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
