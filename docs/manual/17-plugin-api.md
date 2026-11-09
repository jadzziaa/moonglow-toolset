---
type: Manual Page
title: Plugin API reference
description: Reference of the plugin API - mg, findings and their fixes (ctx.fix), exporters, importers, wizards and panels (their fields), ctx.selection, ctx.open and ctx.values, ctx.event (the events, a resource's own around a save, objects placed or pasted, resources made, and what a handler may edit), ctx.module, ctx.game, ctx.edit, ctx.hak, ctx.terrain, logging, progress and UI (files and folders the user chooses), pictures, ctx.plugin, ctx.settings, Luau, limits and what the API doesn't have yet.
tags: [manual, plugins, api]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-09T00:38:51Z }
---

# Plugin API reference

Everything a plugin's code can use, for plugin API 0.9. The API is
experimental: a later release may change it, and each change is listed
in the API's [change list](https://github.com/jadzziaa/moonglow-toolset/tree/develop/docs/plugins).
How plugins work and how to write one is in
[Writing plugins](16-writing-plugins.md).

A plugin's script gets the API with `require("@moonglow")` (called `mg`
here) and registers handlers; a handler gets a context (`ctx`) for the
job it is run for. A call written with a colon here is made with a colon
(`ctx.module:gff(name)`), one with a dot with a dot. An argument with `?`
may be left out.

Every function raises an error when it can't do what it is asked (no
such resource, a path that leads nowhere, a value that doesn't fit). An
error not caught with `pcall` ends the job: a command then changes
nothing, and a check is reported as failed.

## `mg`

| | |
| --- | --- |
| `mg.api` | The plugin API of the Moonglow running, as text: `"0.9"`. |
| `mg.command(id, handler)` | Registers the function for the command the manifest declares as `id`. `handler(ctx)` makes its edits with `ctx.edit`. It may return what Undo calls the change, as `{ label = "…" }` or the text alone; else that is the command's title. |
| `mg.check(id, handler)` | Registers the function for the check the manifest declares as `id`. `handler(ctx)` returns a list of findings (below), or nothing for none. A check that edits fails. |
| `mg.on(id, handler)` | Registers the function for the event the manifest declares as `id` (`[event]`, its `on` one of the events of [`ctx.event`](#ctxevent)). `handler(ctx)` is called when it happens, as a command's is, with what happened as `ctx.event`: `ctx.event.name`, and what the event is of. It may return what Undo calls its change. |
| `mg.exporter(id, handler)` | Registers the exporter the manifest declares as `id` (`[exporter]`; since API 0.6). `handler(ctx, out)` writes with `out` (below), and only reads the module. It may return text, which the log says. |
| `mg.importer(id, handler)` | Registers the importer the manifest declares as `id` (`[importer]`; since API 0.6). `handler(ctx, source)` reads `source`, the `file` or the `folder` the user chose (as `ctx.ui:open_file` and `ctx.ui:open_folder` give them), and edits with `ctx.edit`. It may return what Undo calls the change; else that is "Import" and its title. |
| `mg.wizard(id, wizard)` | Registers the wizard the manifest declares as `id` (`[wizard]`; since API 0.6): `wizard` is `{ pages = { … }, finish = function(ctx, answers) … end }` (below). `finish` makes the edits, and may return what Undo calls them; else that is the wizard's title. |
| `mg.panel(id, handler)` | Registers the panel the manifest declares as `id` (`[panel]`; since API 0.6). `handler(ctx)` returns what the panel shows, a list of elements (below), and only reads. |
| `mg.image(bytes, kind?)` | A picture decoded, as an `image` (below): a PNG, a TGA or a DDS, told apart by how the bytes start, or by `kind` (`"png"`, `"tga"`, `"dds"`). |
| `mg.twoda` | 2DAs made, read and written: `parse(bytes)`, `new(columns)` and `kinds(name)` (below). |
| `mg.tlk` | Talk tables made, read and written: `parse(bytes)` and `new()` (below). |
| `mg.path(struct)` | Where a struct read with `gff` is, as a path for `ctx.edit`: `""` for the root, `"/ClassList[0]"`. Nil for a table that is no such struct. |
| `mg.struct_id(struct)` | A struct's id (since Moonglow 2.2.0): of one read with `gff`, kept beside its table as its path is, never among its fields (an equipped item's is its slot, `Equip_ItemList`); of one read with `raw`, its `__struct_id`. Nil for a table that is neither. |
| `mg.vars` | The variables the module's project describes (its `variables.toml`): `describe(name)` and `described()` (below; since Moonglow 2.13.0). |

An id is registered once; registering one the manifest doesn't declare,
or not registering one it does, is a fault `mg plugin check` reports.

A **finding** is a table:

| Field | |
| --- | --- |
| `resource` | The resource at fault, `name.ext`. Needed. |
| `message` | What is wrong. Needed. |
| `at` | Where in the resource: a field, a row. |
| `severity` | `"warning"` or `"error"`; else the check's, from the manifest. |
| `fix` | Its fix, made with `ctx.fix` (below; since API 0.4): offered beside the finding in Verify Module's report, made only when the user asks. |

### `mg.vars`

The [described variables](03-modules.md#described-variables) of the
module's project, as the Variables window and `mg vars` read them
(since Moonglow 2.13.0). `mg.vars.describe(name)` is the description of
a variable so named: of that very name, else of the first pattern that
takes it (in its case only, as the game tells names apart); nil where
none names it. `mg.vars.described()` is every description, in the
file's order. A description is a table:

| Field | |
| --- | --- |
| `name` | The name it describes, or its pattern (`tdod_rental_*`). |
| `pattern` | Whether it is a pattern. |
| `file`, `line` | Where it is written: `variables.toml` (or a `.mod`'s own, `krezk.variables.toml`) and its line. |
| `type` | `"int"`, `"float"`, `"string"`, `"object"` or `"location"`; nil for any. |
| `on` | What it goes on: a list of kinds (`"placeable"`, `"area"`, `"module"`…); empty for anything. |
| `help` | Its help, or nil. |
| `values` | Its values, each `{ value, note }`: those written, a script's constants and a table's rows, read as the Variables window reads them (the project's scripts and tables, the module's, the game's). |
| `rows` | For an Int whose values are a table's rows, how many it has; else nil. |
| `problem` | What of its values could not be read (a script or table not there), or nil. |
| `other` | A value of the builder's own is right too. |
| `min`, `max` | Its bounds, or nil. |
| `runtime` | The scripts set it while the server runs: never kept in a file. |
| `requires` | The variables it needs beside it on the same object: a list of names, empty for none. |
| `default` | The value the scripts take where it isn't set, as text, or nil. |

### `ctx.fix`

A check's handler gets `ctx.fix` too, and so does a `resource-saving`
handler, whose warnings and refusals may carry one (since API 0.4; a
command's and other events' do not):

| | |
| --- | --- |
| `ctx.fix(label, asks?)` | A fix to give a finding (`fix = …`), or a warning or a refusal of a save (`ctx.event:warn(text, fix)`): `label` is what its button says ("Upper-case this tag"). Called with a dot. With `asks` (a finding's alone; since API 0.5), `{ command = "retag", on = s? }`, it is a fix that asks: its button runs the plugin's command `command` (one its manifest declares) on `on` (an entry with `resource` and `path`, as `ctx.selection`'s; left out, the finding's resource), whose form asks what it needs; it records no edits. |
| `fix.label` | Its label. |
| `fix:set`, `fix:remove`, `fix:insert`, `fix:remove_item`, `fix:write`, `fix:write_bytes`, `fix:write_gff`, `fix:delete` | As `ctx.edit`'s (below), with the same arguments: each records an edit of the fix. Nothing is changed: the user makes the fix (Verify's button, `mg verify --fix`), as one step of Undo named by its label. |

Each fix starts from the module as the check reads it, and its edits
apply to it alone: the check's reads (`ctx.module`) and the other fixes
do not see them; a fix's later edits see its earlier ones. A fix with no
edits is none. A fix is made only while the resources it changes are as
the check read them (else the user is told to verify again), or changed
since only by fixes of the same Verify that set fields. A fix that edits
asks the user nothing; one that asks runs a command, whose edits are its
own step of Undo, as a command's are (since API 0.5). Undo of a fix
offers its button again (since Moonglow 3.3.0).

### Typed values

A value with its type, for a field that has none yet, or wherever a
typed value is written (a list's item, a whole GFF). Each returns
`{ type = "…", value = … }`, the form `mg gff` writes.

| | Type | Takes |
| --- | --- | --- |
| `mg.byte(n)` | `byte` | 0 to 255 |
| `mg.char(n)` | `char` | -128 to 127 |
| `mg.word(n)` | `word` | 0 to 65,535 |
| `mg.short(n)` | `short` | -32,768 to 32,767 |
| `mg.dword(n)` | `dword` | 0 to 4,294,967,295 |
| `mg.int(n)` | `int` | -2,147,483,648 to 2,147,483,647 |
| `mg.dword64(n)` | `dword64` | 0 to 2⁶⁴-1 |
| `mg.int64(n)` | `int64` | -2⁶³ to 2⁶³-1 |
| `mg.float(n)` | `float` | a number |
| `mg.double(n)` | `double` | a number |
| `mg.string(text)` | `cexostring` | text |
| `mg.resref(name)` | `resref` | a resource's name, to 16 characters |
| `mg.locstring(text)` | `cexolocstring` | text, or a table of texts |

- A Luau number holds whole numbers exactly only to 2⁵³: give a larger
  `dword64` or `int64` as text (`mg.dword64("18446744073709551615")`).
- A resource's name is given without its extension.
- `mg.locstring` takes text (the English text), or a table of texts by
  language number with `strref`, the shape `gff` reads one as.
- There is none for a struct, a list or a `void` (bytes): write those
  as tables, `{ type = "list", value = { … } }`,
  `{ type = "struct", value = { … }, __struct_id = 0 }`.

## Exporters, importers, wizards and panels

What a plugin may add besides commands, checks and events (since API
0.6); [Writing plugins](16-writing-plugins.md#exporters-and-importers)
shows each in an example. Each handler gets `ctx` as a command's does,
with what is selected and open.

An **exporter** runs when its entry of File › Export is chosen, after
the user has chosen where it writes in the save dialog (or, for one
whose manifest says `to = "folder"`, the folder dialog): that choice is
their leave, and nothing more is asked of it. Its `out`:

| | |
| --- | --- |
| `out.name` | The name of the file, or the folder, the user chose. |
| `out:write(bytes)` | To a file: the whole file (a second call replaces what the first gave). To a folder, `out:write(name, bytes)`: a file of it, by name (a name alone, not a path; not one that begins with a dot), in place of one of that name there. |

What it wrote is written when the handler has returned, and not if it
fails. An exporter only reads: one that edits the module, writes a hak
or writes into another folder fails. Undo has nothing to take back: the
module is not changed, and the files are not the module's. An exporter
whose manifest says what it is `for` exports what is selected
(`ctx.selection`), and is offered only when what is selected is all of
those kinds.

An **importer** runs when its entry of File › Import is chosen, after
the user has chosen what it reads: a file of the `extensions` its
manifest names (a file of another kind is refused before its code runs),
or with `from = "folder"` a folder. Its edits are one step of Undo, and
a hak it writes (`ctx.hak`, `folder:to_hak`) goes with them, as a
command's.

A **wizard**'s `pages` are asked one at a time, in the window of its
job: each with **Back** (not on the first), **Next** (on the last,
**Finish**) and **Cancel**. A page is a table, or a function `(ctx,
answers) -> page?` that makes one from what was answered so far; one
that returns nil is skipped (where every page after the one shown is
skipped, its Next finishes). A page:

| Field | |
| --- | --- |
| `id` | Its name, for the errors. |
| `title` | What it is called, above it (else the wizard's title). |
| `text` | What it says above its fields. |
| `fields` | Its fields, as a form's (`ctx.ui:form`). |
| `check` | `function(ctx, answers)`, called on Next: text, what is wrong, keeps the page with that text shown; nil lets it go on. |

`answers` is one table of every page's values by field id, so each
field's id is its own across the pages (one on two pages is an error).
Back keeps what was typed: a page shown again starts with what was given
on it. The pages, their functions and their checks only read (one that
edits fails); `finish(ctx, answers)` edits, as a command, all of it one
step of Undo. Cancel, or closing the window, makes nothing.

A **panel** is a pane of its own (Plugins › Panels), which shows what
its handler returns: a list of elements, each a table with its `type`,
or text alone (a `text`). Moonglow draws them; the panel never draws.

| Element | Fields |
| --- | --- |
| `heading` | `text` |
| `text` | `text`; `weak` (true: of less weight) or `color` (`"warning"`, `"error"`) |
| `value` | `label` and `text`, shown side by side (`Areas` `12`) |
| `separator` | none |
| `button` | `text`, `hint`, `command` (one of the plugin's commands, by its id) and `on` (an entry, or a list of them; left out, what is selected when it is clicked) |
| `list` | `items`: each text, or a table with `text`, `hint`, `open` (an entry: a click opens it, an object in its area, a resource in its editor) and `command` and `on` (a click runs the command) |
| `field` | A form's field (since API 0.9): `id`, `label`, `kind` (`"text"`, `"number"`, `"check"`, `"choice"` or `"color"`, as a form's field's `type`), `value` (what it shows), `min`, `max`, `choices`, `variable` (as a form's) and `command` (one of the plugin's commands, run when the user sets it) |

An entry is as `ctx.selection` has one (`{ resource = "tavern.git",
path = "/Creature List[0]" }`), or its text (`"tavern.git/Creature
List[0]"`, `"guard.utc"`).

A panel's **field** stays as the user leaves it: Moonglow keeps its
value, by its id, across the handler's runs. When the handler gives a
`value` other than the one it gave the last time (the tint of another
placeable now selected), the field takes it; else what the user set
stays. (A form's `default` is a panel field's `value`: a field given a
`default` is an error.) Each field's id is its own in the panel. A field
with a `command` runs it when the user sets the field to another value:
a color when its picker closes (or a color is pasted on its swatch),
text when the user leaves it (or presses Enter), a number when it is let
go, a check or a choice at once. The command runs as a button's does,
on what was selected when the user began at the field, one step of
Undo. A color field's `variable` draws the color on the placed objects
selected while its picker is open, as a form's does; nothing is set
until its command sets it. Every command the panel runs (a button's, a
list item's, a field's) gets the values of all its fields in
`ctx.values`, by field id, as a form's come back.

```lua
mg.panel("tint", function(ctx)
  return {
    { type = "field", kind = "color", id = "tint", label = "Tint",
      value = "#2a4d8f", variable = "tdod_tint", command = "set-tint" },
  }
end)

mg.command("set-tint", function(ctx)
  local tint = ctx.values.tint   -- "#rrggbb"; nil when not run from the panel
end)
``` The handler runs when the panel opens, by its
**Refresh**, after a job of its plugin's ends (a command, an import, a
wizard), and as its manifest's `refresh` says: after each change of the
module (`"edits"`: an edit, Undo, Redo; edits one after another, a
drag's steps, draw it once, when they have stopped for 200
milliseconds) and when what is selected changes (`"selection"`). It runs beside the window, not in a frame: the
panel shows what it returned last until the next is done, one render at
a time. A panel only reads: nothing is asked (`ctx.ui:confirm` is false,
a form nil, no file is given), it can't set a setting or write outside
the module, and what it logs goes nowhere; its buttons run commands,
which ask and edit as commands do. It has 250 milliseconds, past which
it is stopped and says so; it shows at most 2,000 elements (a list's
items counted each) and 1,000 characters a text, the rest cut, saying
so. A panel is not in `mg`, which has no window to show it in; a
plugin's own tests hold what it shows (`shows`; a field's as its label
and the value it is given, `Tint: #2a4d8f`), and give a command the
values a panel would (`values`). The example `tint-panel` (in
`docs/plugins/examples`) is a panel whose color field follows the
placeable selected and sets its tint.

## `ctx.selection`, `ctx.open`

What a command was run on, and what is open in the window (since API
0.4). They are read as `ctx.module` is, with nothing asked: they only
say which of the module's objects the user means, and a plugin reads the
whole module anyway.

| | |
| --- | --- |
| `ctx.selection` | What the command is run on: a list of entries (below), from 1. From a right-click menu (a command whose manifest says what it is `for`), what the menu was opened on: the objects selected in the area view (a right-click on an object selects it, as for every command of that menu), the blueprints chosen in a palette (Ctrl+click chooses several), the object or blueprint of a row of the module tree (or of each of the rows chosen), an area, the tiles selected in an area's view (since API 0.5), every blueprint of a palette's category or every resource of a group of the module tree (since 0.5), what a Properties window edits (its tab's menu; since 0.5), the script open (the script editor's menu; since 0.5), or what a row of Find References or of Advanced Search is on (the ticked ones, where it is one of them; since 0.5). From the Plugins menu, the Command Palette or a key: what is chosen in the pane last clicked in (the module tree, a palette, an area's view: its objects, or in tile mode its tiles; since 0.5), else the objects selected in the area view in front, else the blueprints selected in the palette, else none. Empty in a check and in an event's handler. |
| `ctx.values` | The values of the fields of the panel the command was run from (a button's, a list item's or a field's command; since API 0.9), by field id, as a form's come back: text, a number, true or false, a choice's text, a color as `"#rrggbb"`. An empty table for a command run from anywhere else (a menu, a key, a fix, `mg plugin run`), and in every other handler. |
| `ctx.open` | The area or editor in front (of the middle of the window, or of the pane with the focus), as an entry; nil if there is none. An area's view and its Properties are the area; an object's or a blueprint's Properties the object or the blueprint; another editor its resource, with its type as `kind` (`"nss"`, `"dlg"`); a copy the resource browser opened as one layer has it (an overridden copy, read-only), that resource with its type as `kind` and its `layer`. Several edited together in one Properties window: the first, with `together` listing them all (since API 0.5). |

An entry:

| Field | |
| --- | --- |
| `selected.kind` | What it is: `"creature"`, `"door"`, `"encounter"`, `"item"`, `"placeable"`, `"sound"`, `"store"`, `"trigger"` or `"waypoint"` (placed, or a blueprint), `"area"`, `"tile"` (since API 0.5), or another resource's type (`"nss"`). |
| `selected.resource` | The resource it is in: for a placed object its area's GIT (`"tavern.git"`), for a blueprint the blueprint (`"guard.utc"`), for an area its ARE (`"tavern.are"`). |
| `selected.path` | Where in the resource (see [Paths](#paths)): `"/Placeable List[3]"` for a placed object, `""` for one that is the resource's root. A field of it is `s.path .. "/Tag"`, and `ctx.module:raw(s.resource, s.path)` reads it. |
| `selected.blueprint` | True for a blueprint; false for a placed object and an area. |
| `selected.module` | Whether it is the module's: false for a blueprint of the game's (of a standard palette), which `ctx.game` reads and no edit can change. |
| `selected.area` | The area's resref, for a placed object and an area; else nil. |
| `selected.index` | A placed object's place in its list, or a tile's in its area's `Tile_List`, from 0 as in its path; else nil. |
| `selected.x`, `selected.y` | A tile's column and row, from 0 at the area's south-west, as `ctx.terrain` counts cells (since API 0.5); else nil. |
| `selected.together` | For `ctx.open` edited together with others in one Properties window, every one of them as an entry, itself first (since API 0.5); else nil. |
| `selected.tag` | Its tag, as the job's copy of the module has it (nil if it has none). |
| `selected.resref` | A blueprint's or an area's resref; for a placed object the blueprint it was made from (its `TemplateResRef`). |
| `selected.layer` | For a layer's copy (`ctx.open` only), the layer's label, which `ctx.game:bytes(s.resource, s.layer)` reads it by; else nil. Its `module` is false, and its `tag` is that copy's. |

The entries are what was selected when the job began; its own edits
don't change them (a placed object deleted by `ctx.edit:remove_item`
leaves those after it a place lower in their list).

## `ctx.event`

What happened, in an event's handler (`mg.on`). The manifest's
`[event]` names the event (`on`), and may narrow it: `types`, for a
save's events, the types of resource (`"are, git"`), and `for`, for
objects placed or pasted and resources made, kinds of object or `area`
(as a command's `for`). A handler is not called for what it is not for,
and none of the plugin's code runs when nothing is.

| Event | When | Called | Its edits |
| --- | --- | --- | --- |
| `module-opened` | a module is opened | once | one step of Undo, after |
| `module-saved` | the module was saved | once | one step of Undo, after the save (the module has unsaved changes again) |
| `resource-saving` (0.4) | a save is about to write | for each resource it writes | none: it warns, refuses, offers fixes |
| `resource-saved` (0.4) | a save wrote | for each resource it wrote | none |
| `object-placed` (0.4) | objects are placed in an area: from a palette, the Placeable Gallery, a light, Create Waypoint, Setup Store's store | once | in the placing's own step of Undo |
| `object-pasted` (0.4) | objects are pasted into an area (a prefab too) | once | in the paste's own step of Undo |
| `resource-created` (0.4) | a wizard (area, creature, item, placeable, door…, and since Moonglow 3.3.0 the Script Wizard), Add to Palette, Setup Store, a copy (Copy As), and since 3.3.0 New Script, a new NUI window and an import (Import, a NUI window imported) make resources | for each resource made | in the making's own step of Undo |

| | |
| --- | --- |
| `ctx.event.name` | The event's name (`"resource-saving"`). |
| `ctx.event.resources` | `module-saved`: the names of the resources not as they were at the last save (or when the module was opened). `resource-saving`, `resource-saved`: every resource the save writes. |
| `ctx.event.resource` | The resource the handler is called for (`"tavern.are"`): of a save, or made. |
| `ctx.event.kind` | What that resource is: `"area"`, `"creature"`… (as a selection's `kind`), else its type (`"nss"`, `"dlg"`). |
| `ctx.event.old` | Of a save: the resource as it was, a GFF as plain values (as `ctx.module:gff` reads one) and another as text; nil for one the save writes for the first time. As it was is as its file was last read or saved; for a module kept in one file, as it was when last saved or opened. Read when first used. |
| `ctx.event.new` | Of a save: the resource as the save writes it (the script editors' unsaved text too), in the same forms; nil for one it deletes. `ctx.module` reads the whole module so. |
| `ctx.event.changed` | Of a save: for a GFF that was there and still is, what differs, as lines (`/Tag: cexostring "TAVERN" != cexostring "INN"`, at most 100); for a script or another text (since Moonglow 3.3.0), the lines between those it begins and ends with alike, each taken out and put in by its number (`3: -     int a = 1;`, `3: +     int a = 2;`, at most 100); else nil. |
| `ctx.event.made` | `resource-created`: every resource the user's action made (an area by its ARE, a script by its source). |
| `ctx.event.objects` | `object-placed`, `object-pasted`: the objects (of the kinds `for` names), each an entry as `ctx.selection`'s (`resource`, `path`, `kind`, `index`, `tag`, `resref`…). |
| `ctx.event.area` | Their area's resref. |
| `ctx.event.by` | What made the objects or the resources: `"wizard"`, `"add-to-palette"`, `"setup-store"`, `"copy"`, `"new-script"`, `"new-nui"`, `"import"`, `"palette"`, `"gallery"`, `"light"`, `"create-waypoint"` or `"paste"`. |
| `ctx.event:warn(text, fix?)` | `resource-saving`: a warning of the resource. Before anything is written the user sees it, with its fix (made with `ctx.fix`) as a button, and chooses Save Anyway or Cancel. |
| `ctx.event:refuse(text, fix?)` | `resource-saving`: the save stops before anything is written, and the user is told why, with the fix if there is one. |

**Edits: on the user's word.** A plugin changes the module only on the
user's word, and that decides what an event's handler may do. A paste,
or a wizard's Finish, is the user's own action: what a plugin's handler
adds to it is part of that action, said in the log ("House rules:
object-pasted changed 1 resource with Paste (one Undo takes both
back)") and undone with it, as one step. A save is not an edit: it
writes what the user sees, and a plugin changing the module under it
would write what nobody saw. So around a save a plugin only reads: a
`resource-saving` or `resource-saved` handler that edits fails (as one
that sets a setting before a save does), and what it would change it
offers as a fix, made only if the user clicks it. A fix made from the
Before saving window is a step of Undo of its own, and the save is then
asked again: the plugins read the module as it now is.

**In what order.** Moonglow's own work comes first: the builder code in
a new ResRef, the project's script and tag for Setup Store (the
project's `moonglow.toml`). The plugins' handlers then see what Moonglow
made, each in the order of the Plugins window and seeing the edits of
those before it. The warning of a tag a paste doubled comes last, and
sees what the plugins did: a tag a plugin made the copy's own is not
doubled.

**At once.** `resource-saving`, `object-placed`, `object-pasted` and
`resource-created` run at once, within what the user does: a save waits
for them, a paste goes in with their edits. Each plugin's handlers of
one event have 5 seconds (all of a save's resources together; Manage
Plugins sets another time, since Moonglow 3.3.0); past
that, the handler is stopped. Nobody is asked anything
(`ctx.ui:confirm` is false, a form nil, no file or folder is given),
and nothing is written outside the module (a hak, a folder: the
handler fails). A handler that fails, or runs out of time, is said in
the log; its edits are left out, and the paste or the making goes on as
Moonglow made it. Before a save it is a warning ("could not check the
save"), never a refusal: a broken plugin can't keep a module from being
saved (Manage Plugins disables it). `module-opened`, `module-saved` and
`resource-saved` run afterwards, as jobs, as a command does.

**What fires none:** what a plugin makes itself (its command's
`ctx.terrain:new_area`, `ctx.edit:write_gff`: a plugin's making is not
another plugin's event, or plugins would answer each other), the saves
of what is no resource of the module (a hak's, a talk table's, a 2DA
file's own editor), a script's Save (it puts the text into the module;
the module's save is asked), Build Module and `mg watch` (they write a
build and a development folder, not the module), and `mg`'s own work
but for a save. Save As (to another place) is asked about as Save is
(since Moonglow 3.3.0).

**With `mg`.** `mg set` and `mg apply` run the `resource-saving` and
`resource-saved` handlers of the plugins `--plugins` names and of the
project's own plugins as the builder allowed them: a refusal is an error
and nothing is written; a warning is said, and the save goes on (the
JSON's `said` has each), or with `--strict` stops it too. `mg rename`,
`mg replace`, `mg areas`, `mg update-instances`, `mg import`, `mg verify
--fix` and `mg plugin run`'s save run the project's own plugins' alike
(since Moonglow 3.3.0). `mg` makes nothing by a wizard or a paste, so
the other events fire there only by hand: `mg plugin run --event` runs
any event's handler, with `--changed` naming the resources (of a save,
or made), `--was MODULE` the module as it was before a save, `--select`
the objects placed or pasted, and `--by` what made them.

## `ctx.module`

The module, as the job's private copy: with the job's own edits so far,
and without anything that changes in the window while it runs.

| | |
| --- | --- |
| `ctx.module:resources(kind?)` | The names (`name.ext`) of the module's resources, sorted; with `kind` (`"utc"`, `"nss"`…) those of one type. |
| `ctx.module:has(name)` | Whether the module has the resource. |
| `ctx.module:gff(name)` | A GFF resource as a table of plain values (below). |
| `ctx.module:raw(name, path?)` | A GFF resource with every field's type, as `mg gff` writes it: fields as `{ type = "…", value = … }`, structs with `__struct_id`, the root with `__data_type`. With `path`, the struct there alone. |
| `ctx.module:text(name)` | A resource as text (a script, a 2DA), converted from the module's codepage. |
| `ctx.module:bytes(name)` | A resource as it is, a string of bytes. |
| `ctx.module:new_name(name, kind)` | A name (`name.ext`) for a new resource of a kind (`"utc"`, `"dlg"`, `"are"`…), made from `name` (`"Gate Guard"`) as the window makes one: with the builder's code where the project's pattern puts it (Edit › Builder Code…, `[resrefs] pattern`), the first the module hasn't (`ab_gateguard.utc`, `ab_gateguard001.utc`; an area's as the Area Wizard numbers them). Write it with `ctx.edit`. Since Moonglow 2.13.0. |

### Plain values

What `gff` returns for each type of field:

| Field | Value |
| --- | --- |
| `byte`, `char`, `word`, `short`, `dword`, `int`, `float`, `double` | a number |
| `dword64`, `int64` | a number (exact to 2⁵³; `raw` gives the rest) |
| `cexostring`, `resref` | a string |
| `cexolocstring` | a table: its texts by language number (0 English, 2 French, 4 German, 6 Italian, 8 Spanish, 10 Polish; plus 1 for the feminine form), and `strref` when it has a talk-table reference |
| `void` | a string of bytes |
| struct | a table of its fields |
| list | a table of its items (structs), from 1 |

A field that is absent is nil. The tables are copies. Two fields with
one label (which the format allows, and nothing sane has) show as one;
`raw` has the same limit.

## `ctx.game`

What the game would load for this module: the game's own resources, with
the module's haks over them and its custom talk table.

| | |
| --- | --- |
| `ctx.game:resources(kind?)` | The names of everything the game can load (there are tens of thousands: give a `kind`). |
| `ctx.game:has(name)` | Whether the game has the resource. |
| `ctx.game:gff(name, layer?)` | As `ctx.module:gff`, of the game's resource (a stock blueprint, say). With a `layer` (a layer's label, as `selected.layer` and `mg layers` give it: `"key:nwn_base"`, `"hak:castle"`), that layer's copy, though a higher layer's overrides it. |
| `ctx.game:raw(name, path?)` | As `ctx.module:raw`. |
| `ctx.game:text(name, layer?)` | As `ctx.module:text` (`nwscript.nss`, say); a `layer`'s copy, as for `gff`. |
| `ctx.game:bytes(name, layer?)` | As `ctx.module:bytes`; a `layer`'s copy, as for `gff`. |
| `ctx.game:table(name)` | A 2DA by its name without extension (`"classes"`), as a `twoda` (below). |
| `ctx.game:string(strref)` | A talk-table string, from the custom talk table for a reference with the custom flag; nil if there is none. |

A `twoda`:

| | |
| --- | --- |
| `twoda.rows` | How many rows it has. |
| `twoda.columns` | Its columns' names, a list. |
| `twoda:get(row, column)` | A cell's text; rows count from 0, as in the file, and columns are named without regard to case. Nil for an empty cell (`****`), and for a row or column the table lacks. |

A `twoda` is the plugin's own: one from `ctx.game:table` can be changed
and written out, and the game's table stays as it is. `mg.twoda.parse`
reads one from a file's bytes (`ctx.game:bytes("feat.2da")`, a file from
a folder the user chose), and `mg.twoda.new({ "Label", "Name" })` makes
an empty one:

| | |
| --- | --- |
| `twoda:set(row, column, value?)` | Sets a cell: text, or a number; nil empties it. The row and the column must be there. |
| `twoda:add_row(cells?)` | Adds a row at the end and returns its number; `cells` are its values by column (`{ Label = "x", Cost = 5 }`), the others empty. |
| `twoda:add_column(name)` | Adds a column at the end, its cells empty. |
| `twoda:write()` | The table as a file's bytes, for `ctx.hak:write`: one read from a file (`ctx.game:table`, `mg.twoda.parse`) as that file is laid out, the file itself where the table is unchanged, its columns at least as wide as there, its line ends (since Moonglow 2.13.0); one made with `mg.twoda.new` as the game's own tables are. |

`mg.twoda.kinds(name)` says what the columns of one of the game's tables
hold, where Verify Module checks them: a table by column name, `"strref"`
(a talk-table string's number: `kinds("feat").FEAT`), `"model"` (a
model's name: `kinds("placeables").ModelName`) or `"row:"` and a table's
name (a row's number in that table: `kinds("feat").MASTERFEAT` is
`"row:masterfeats"`; since Moonglow 1.21.2), `"table"` (another
table's name, without `.2da`: `kinds("classes").FeatsTable`; since
Moonglow 2.13.0), `"script"` (a script the game runs:
`kinds("spells").ImpactScript`, `kinds("vfx_persistent").ONENTER`, and
the scripts of `statescripts`, `traps`, `disease` and `poison`), or,
for a column whose cells name a row of the table
another column's value chooses (a class's requirements,
`cls_pres_*`'s `ReqParam1` by its `ReqType`; since 2.13.0), a table
`{ by = "ReqType", rows = { FEAT = "feat", SKILL = "skills", … } }`: a
value not in `rows` names no row. A table it knows nothing of gives an
empty one. The lists a class, a race or a package names are known by
how the game's own are named (`cls_feat_*`, `cls_skill_*`, `cls_pres_*`,
`race_feat_*`, `packft*`, `packsk*`, `packsp*`), and, with the game's
data, by the column that names them, whatever their names (since
Moonglow 2.13.0): so are the tables an item property names
(`itempropdef.2da`'s `SubTypeResRef`, `iprp_costtable.2da`'s `Name` and
`iprp_paramtable.2da`'s `TableResRef`), whose `Name` is a string. Of the
item properties' own, `itempropdef.2da`'s `Name`, `GameStrRef` and
`Description` are strings and its `CostTableResRef` and `Param1ResRef`
rows of `iprp_costtable.2da` and `iprp_paramtable.2da`. The tables a
class names for its bonus feats, spells, saves, attack bonus and
ability gains hold numbers alone: `kinds` has nothing to say of them. A
cell that names a row holds `****` or a negative number for none, and a
spell's `FeatID` keeps the feat in its low 16 bits.

`mg.twoda.describe(text, scripts)` reads
[a table's description](03-modules.md#a-tables-description) from the
text of its `name.2da.toml` (since Moonglow 1.23.0); `scripts`, where the
description takes a list from a script's constants, gives each such
script's source by its name (`{ my_inc = source }`). It gives:

| | |
| --- | --- |
| `d.rows` | The table whose rows the described table's are, if it says. |
| `d.columns` | Each described column's kind, by name: `"text"`, `"int"`, `"float"`, `"list"`, `"flags"`, `"strref"`, `"script"`, or `"row:"` and a table's name. |
| `d.notes` | What the description said that was not understood. |
| `d:name(column, value)` | A value's name in a list, or the names of the flags it has; nil where it has none. |
| `d:faults(twoda, lengths)` | The cells of `twoda` that are not what the description says, each `{ row, column, value, message }`. `lengths` gives how many rows the tables it names have (`{ classes = 42 }`); a table not given is not held to. Strings are not looked up. |

A `tlk`, from `mg.tlk.parse(bytes)` or `mg.tlk.new(language?)` (a
language's two-letter code, `"fr"`; English if none; since Moonglow
2.13.0); lines count from 0 as in the file (a custom table's line 0 is
the game's string 16777216). Its text is written in its language's
codepage (Windows-1250 for Polish), so a letter the language's
codepage lacks fails. A feminine table is a file of its own, as the
game keeps it (`dialogf.tlk`, `mytablef.tlk`): read and write it as
another `tlk`.

| | |
| --- | --- |
| `tlk.lines` | How many lines it has. |
| `tlk.language` | Its language's two-letter code (`"en"`, `"fr"`, `"de"`, `"it"`, `"es"`, `"pl"`), as its file says (since Moonglow 2.13.0). |
| `tlk:set_language(code)` | Makes it a table of another language (since Moonglow 2.13.0). |
| `tlk:get(line)` | A line as `{ text, sound, sound_length }`; nil for none. |
| `tlk:set(line, value?)` | Sets a line (the table grows to hold it): its text, or a table with `text`, `sound` (1 to 16 letters, digits, `_` and `-`, as the talk-table editor takes one; kept in lower case) and `sound_length`; nil empties it. |
| `tlk:add(value)` | Adds a line at the end and returns its number. |
| `tlk:write()` | The table as a file's bytes. |

Without a game installation `has` is false, `string` is nil and the rest
fail. `ctx.game` is only read: a plugin changes the module, never the
game's data or a hak.

In a plugin's own tests ([Writing plugins](16-writing-plugins.md#a-plugins-own-tests)),
`ctx.game` is the test's game data: the game `mg` finds, the one at the
test's `root`, or none with `root = false` (since Moonglow 2.3.0), and
over it the test's game folder. A resource it reads that is not there
is named in the test's result: where the test fails, as a reason it may
have; where it passes all the same (the plugin read it in a `pcall`),
as a note (since 2.3.0). A test with no game that calls
`ctx.game:resources()` is told that the list is only the fixture's and
its game folder's (with no folder, the fixture's alone: with no game
and no game folder, `ctx.game` is the fixture, since Moonglow 2.13.0).
The game folder's `dialog.tlk` and the module's custom talk table are
what `ctx.game:string` reads (since 2.13.0).

## `ctx.edit`

A command's changes. Each is applied to the job's copy at once and
recorded; when the handler returns, Moonglow applies the recorded edits
to the module as one undoable step.

| | |
| --- | --- |
| `ctx.edit:set(name, field, value, type?)` | Sets a field of a GFF resource. `field` is its path. `value` is a plain value, written as the type the field has; for a field the resource lacks, as `type` (`"byte"`, `"int"`, `"cexostring"`…) if given, else as the type the game's files give a field of that name at the root of that kind of resource; otherwise it is an error. A typed value (`mg.int(3)`) is written as it says. Text for a `cexolocstring` sets its English and keeps the rest; a table (texts by language number, `strref`) replaces all of it. |
| `ctx.edit:remove(name, field)` | Removes a field. |
| `ctx.edit:insert(name, list, item, index?)` | Adds an item to the list at path `list`: a struct as `raw` writes one (`{ __struct_id = 2, Class = mg.int(4) }`), at `index` (0 is first), or at the end. Returns the index it is at. |
| `ctx.edit:remove_item(name, item)` | Removes a list's item, by its path (`"/ClassList[0]"`). |
| `ctx.edit:write(name, text)` | Writes a resource as text (new, or in place of what was there), converted to the module's codepage. |
| `ctx.edit:write_bytes(name, bytes)` | Writes a resource as the bytes given. |
| `ctx.edit:write_gff(name, document)` | Writes a GFF resource from its `raw` form (with `__data_type`). |
| `ctx.edit:delete(name)` | Removes a resource from the module. |

### Paths

A path leads from a resource's root to a struct or a field: labels after
`/`, a list's item as `Label[index]`, **items counted from 0**:
`/ClassList[0]/ClassLevel`, `/Creature List[3]/Tag`. For a field of the
root the leading `/` may be left out (`Tag`). In a label, `/`, `[`, `]`
and `~` are written `~1`, `~2`, `~3` and `~0`, and a label that is empty
`~e`. These are the paths of `mg set` and of edit files
([Command-line tools](11-command-line.md)).

## `ctx.hak`

Resources for a hak in the hak folder of the user's Neverwinter Nights
folder: the one thing a plugin writes outside the module. The first time
a job writes to a hak, the user is asked whether to allow it, and a
refusal ends the job. The hak is written when the command's handler has
returned, before its edits go in: resources are added to a hak that is
there (those of the same name replaced, a copy of the hak kept beside it
as `name.hak.bak`), and a hak that isn't there is made. The hak goes
with the edits (since Moonglow 3.3.0): taken back if they fail, by their
Undo (as it was, or gone if the job made it), and written again by
Redo, unless it was changed since. A command that edits nothing has no
Undo, and its hak stays. A check can't write one.

| | |
| --- | --- |
| `ctx.hak:write(hak, name, bytes)` | Puts a resource into the hak named `hak` (letters, digits, `_` and `-`; `.hak` may be left out), as `name` (`name.ext`: at most 16 letters, digits, `_` and `-`, and a type the game reads). |
| `ctx.hak:attach(hak)` | Lists the hak at the top of the module's haks (an edit of the module, undone with the command). |

A folder the user chose copies a file straight into a hak, however large
(`folder:to_hak`, below). What a job has put into a hak, `ctx.game`
reads in that job as if the hak were there already, and `ctx.terrain`
takes a tileset from it.

## `ctx.terrain`

Areas made, and their terrain painted as the area editor's brushes paint
it: the plugin says what a corner is, and Moonglow chooses the tiles
that fit, as it does under the pointer. It needs a game installation.

| | |
| --- | --- |
| `ctx.terrain:tilesets()` | The tilesets the game offers (with the module's haks), as the Area Wizard lists them: a list of `{ resref = "ttr01", name = "Rural" }`. |
| `ctx.terrain:new_area(spec)` | Makes an area as the Area Wizard does (its three files, and its place in the module's list). `spec` is `{ name = "…", tileset = "ttr01", width = 8, height = 8 }`, the sizes in tiles, 2 to 32. Returns the area's resref. |
| `ctx.terrain:open(area)` | An area's terrain, by the area's resref, as an `area` (below). It fails for an area whose tiles are not all the tileset's. |

An `area`. Cells (tiles) and corners count from the area's south-west,
from 0: cell (x, y) has the corners (x, y) to (x + 1, y + 1), so an area
`width` cells across has corners 0 to `width`.

| | |
| --- | --- |
| `area.resref` | The area's resref. |
| `area.tileset` | Its tileset's resref. |
| `area.width`, `area.height` | Its size, in tiles. |
| `area.step` | How high a step of height is, in metres (the tileset's `Transition`). |
| `area.terrains` | The tileset's terrains' names, a list. |
| `area.crossers` | The tileset's crossers' names (roads, streams, walls), a list. |
| `area.groups` | The tileset's groups, a list of `{ name = "…", rows = 2, columns = 3 }`. |
| `area:corner(x, y)` | A corner's terrain (its name) and its height in steps: two values. |
| `area:tile(x, y)` | A cell's tile: `{ id = 12, orientation = 1, height = 0 }`, as the ARE has it. |
| `area:paint(x, y, terrain)` | The terrain brush on a corner: its terrain set, the corners around changed as the tileset's rules have it, new tiles chosen. True, or false when no tile fits there (nothing changes then, as under the brush). |
| `area:raise(x, y, steps?)` | Raise on a corner, `steps` times (once; a negative number lowers). Corners around follow so that no two neighbors are more than a step apart. False when a step was refused (those before it stay). |
| `area:set_height(x, y, height)` | Raises or lowers a corner until it is `height` steps high. False when it could not get there. |
| `area:place_group(group, x, y, turns?)` | A group of the tileset (by name) with its first tile in cell (x, y), turned `turns` quarter turns, as placing it from the palette. False when it doesn't fit. Over part of another group, it takes that one away whole. |
| `area:cross(crosser, path)` | A crosser drawn through cells, as the crosser brush dragged through them: `path` is a list of cells, each beside the one before (`{ { 1, 5 }, { 1, 6 }, { 2, 6 } }`), and the crosser goes on the edges between them, all or none: false when a cell on the way has no tile for it (where the brush, as Aurora's, would lay the rest). |
| `area:erase(x, y)` | The Eraser on a cell: its crossers go, and a group or a feature that has a tile there goes whole. False when refused. |
| `area:set_tile(x, y, id, orientation?, height?)` | Puts one tile into a cell as it is given, whether it fits its neighbors or not (Verify's `tile-seam` names a tile that doesn't fit one: `mg verify` checks what a plugin put in). |
| `area:batch(f)` | Calls `f()` with the area's strokes held (since API 0.5): its tiles are read once, each stroke made on them as `f` goes (`corner` and `tile` read what they left; `ctx.module` reads the area as it was until `f` returns), and the tiles that changed go in when it returns, as one edit of its tile list. If `f` fails, none of them go in. A `set_tile` in it puts in the strokes before it first. Returns what `f` returns. |

Tiles are chosen at random among those that fit, as in the area editor:
two runs paint the same ground with different tiles. New tiles get
lights from the area's lighting scheme, and bring and take the doors
their tiles have.

## `ctx.log`, `ctx.progress`, `ctx.ui`

| | |
| --- | --- |
| `ctx.ui:open_file(options?)` | Asks the user for a file to read: a `file` (below), or nil if they chose none. `options` is `{ title = "…", extensions = { "png", "tga" } }`: what it is for, and the kinds it reads. |
| `ctx.ui:open_folder(options?)` | Asks the user for a folder to read the files of, and to write into with their leave (`folder:write`): a `folder` (below), or nil. `options` is `{ title = "…" }`. |
| `ctx.log:info(text)` | A line in the log, under the plugin's name. |
| `ctx.log:warn(text)` | A warning in the log. |
| `ctx.log:error(text)` | An error in the log (the job goes on; to stop, raise one: `error("…")`). |
| `ctx.progress(done, total, note?)` | How far the job is, for its window: `done` of `total`, and what it is at. Called with a dot. |
| `ctx.ui:message(text)` | Shows text, and waits until it is read. |
| `ctx.ui:confirm(text)` | Asks yes or no: true for yes. |
| `ctx.ui:form(form)` | Asks for values. `form` is `{ title = "…", fields = { … } }`; returns a table of the values by field id, or nil if it was closed without OK. |

A form's **field** is a table:

| Field | |
| --- | --- |
| `id` | The key its value comes back under. Needed, and different for each. |
| `label` | What it is called in the form; else its id. |
| `type` | `"text"` (as when left out), `"number"`, `"check"`, `"choice"` or (since API 0.8) `"color"`. |
| `default` | What it starts with: text, a number, true or false, one of the choices, or a color as `"#rrggbb"` (white when left out). |
| `min`, `max` | A number's limits. |
| `choices` | A choice's options; needed for one. Each is text, or (since API 0.9) `{ text = "Crimson", color = "#a01010" }`: its text, with a swatch of that color drawn before it, in the list and in the box. Its value is the text chosen. |
| `variable` | A color's (API 0.8): the name of a string variable. While the color is being picked (its picker open), the placed objects the command was run on are drawn in the area with that variable set to it, as the Variables page's picker draws them (The Domains of Dread's `tdod_tint`); nothing is set by it, and the plugin sets what it chose once the form comes back. |

A color's value is `"#rrggbb"`, in lower case: the color the swatch's
picker (the one the Properties windows have) was left at.

```lua
local answers = ctx.ui:form({
  title = "Tint",
  fields = {
    { id = "tint", label = "Color", type = "color", default = "#2a4d8f", variable = "tdod_tint" },
  },
})
-- answers.tint is "#rrggbb", or answers is nil: closed without OK.
```

The example plugin `pick-tint` (in `docs/plugins/examples`) is such a
command in a placed placeable's right-click menu: it starts from the
placeable's own `tdod_tint` and sets what was picked.

A `file`, and a `folder`: what the user chose, and nothing else of
their disk. A file in a folder is named by its path from the folder,
with `/` (`"textures/grass.tga"`); a name that leads out of the folder
fails. A file of more than 64 MB is not read.

| | |
| --- | --- |
| `file.name` | The file's name, without its folder. |
| `file.bytes` | What is in it, a string of bytes. |
| `folder.name` | The folder's name. |
| `folder:files()` | The names of its files and of those in its folders, sorted. |
| `folder:bytes(name)` | A file's bytes. |
| `folder:text(name)` | A file as text: UTF-8 if it is, else from the module's codepage (Windows-1252, unless a hak of the module has an `encoding.2da`). |
| `folder:write(name, bytes)` | Writes a file into the folder, when the handler has returned: as `name` (a name alone, not a path; not one that begins with a dot), in place of a file of that name there. The first time a job writes into a folder the user is asked whether to allow it, and a refusal ends the job. Undo does not take it back, and a check can't write. |
| `folder:to_hak(hak, file, name?)` | Puts a file of the folder into a hak as `ctx.hak:write` does, without reading it into the plugin's memory: as `name`, or under its own. |

An `image`, from `mg.image`:

| | |
| --- | --- |
| `image.width`, `image.height` | Its size in pixels. |
| `image:pixel(x, y)` | A pixel's red, green, blue and alpha, each 0 to 255: four values. Pixels count from 0, from the top left. A gray picture has the same red, green and blue; 16 bits a channel are rounded to the nearest of 8 bits, as a NUI window's picture is read. |

With `mg` an exporter writes where `mg plugin export` says, an importer
reads what `mg plugin import` names, and a wizard's pages take their
defaults and `--answers` (a JSON file of values by field id) and
`--answer`s: its pages go on in turn, and a page's check that objects is
an error, with what it said ([Writing plugins](16-writing-plugins.md#exporters-and-importers)).

Where there is nobody to ask (a check; `mg plugin run`): a message is
logged, `confirm` is false (true with `--yes`), and a form is nil in a
check and takes its defaults and `--answer`s with `mg`. A file or a
folder is nil in a check; with `mg` it is the next `--file PATH` given.
Writing a hak is refused in a check, and with `mg` unless `--yes` is
given (the haks go to `--hak-dir`, or the user folder's `hak`). With
`mg`, `ctx.selection` is what `--select` names (each
`tavern.git/Placeable List[3]`, `guard.utc` or `tavern.are`) and
`ctx.open` what `--open` names (a layer's copy as its name and the
layer's label, `nw_chest.utp@key:nwn_base`); else nothing.

## `ctx.plugin`

| | |
| --- | --- |
| `ctx.plugin.id` | The plugin's id, from its manifest. |
| `ctx.plugin.name` | Its name. |
| `ctx.plugin.version` | Its version. |

## `ctx.settings`

The plugin's settings, as its manifest declares them (`[setting]`; since
API 0.4): their values as Moonglow keeps them, by each one's scope, or
their defaults ([Writing plugins](16-writing-plugins.md#settings)).

| | |
| --- | --- |
| `ctx.settings.<id>` | A setting's value: text (a `string`, a `choice`, a `path`, a `color` as `"#rrggbb"`), a whole number (an `int`), a number (a `float`), true or false (a `bool`), or a list of texts (a `list`, from 1; a copy). An id the manifest doesn't declare is an error. `float`, `list` and `color` since API 0.5. |
| `for id, value in ctx.settings` | Each setting and its value, in the manifest's order. |
| `ctx.settings:set(id, value?)` | Sets one: `value` of its kind (a whole number does for a `float`; a table of texts for a `list`), within its limits, among its choices, matching its pattern (else an error), or nil for its default. The job reads it back at once; Moonglow keeps it where its scope says when the job is done, and not if the job fails. A setting of scope `project` or `user-project` can't be set without a project (a module kept in one file has none). Not in a check, nor in the console. |

`ctx.settings.x = …` is an error: a setting is set with `set`. With `mg
plugin run`, a setting of scope `user` has the value Moonglow keeps (its
own folder's `plugin-settings.json`, shared with the window; since
Moonglow 3.3.0), or `--setting ID=VALUE`, and what the plugin sets of that
scope is kept there; those of the project's scopes are read from, and
kept in, the project's files.

## Luau

| | |
| --- | --- |
| `require(name)` | `"@moonglow"`: the API. Else a script of the plugin's own folder, by its name without `.luau` (`"rules"`, `"lib/names"`): run once per job, giving what it returns. |
| `print(...)` | Writes its arguments to the log, as `ctx.log:info`. |

The rest is [Luau's library](https://luau.org/library): `string`,
`table`, `math`, `utf8`, `bit32`, `buffer`, `vector`, `coroutine`,
`os.clock`, `os.time`, `os.date`, `os.difftime`, and the globals
(`pairs`, `pcall`, `tostring`, `tonumber`, `error`, `assert`, `type`…).
There is no `io`, no `package`, and nothing else of `os`: files are
reached only through what the user chooses (`ctx.ui:open_file`,
`ctx.ui:open_folder`) and allows (`ctx.hak`, `folder:write`).

## Limits

- A job has 256 MB of memory; past that it fails.
- What Moonglow holds for a job besides (the tables and talk tables it
  makes, the pictures it has decoded, the edits it makes and the files
  it wants written) may be 512 MB more: a call that would go past that
  fails, and so does the job unless it catches the error. What the
  plugin no longer has (a table it let go of) does not count.
- A picture of more than 64 million pixels is not decoded (`mg.image`).
- The handlers of an event that runs at once, within what the user
  does (a save, a paste, a wizard's Finish), have 5 seconds (Manage
  Plugins sets another); past that
  they are stopped ([`ctx.event`](#ctxevent)).
- A job is stopped when the user cancels; no `pcall` catches that.
- Nothing is kept between jobs but the plugin's settings
  (`ctx.settings:set`). A plugin runs when its command is chosen, when
  its check runs with Verify Module, and by itself only on the events its
  manifest declares (`mg.on`).
- Text must fit the module's codepage (Windows-1252, unless a hak of the
  module has an `encoding.2da`).

## What the API doesn't have yet

Plugins can't yet react to the selection changed (but a panel's
`refresh`), an area opened or a field typed into (but a panel's field's
`command`), warn of a paste or refuse one, add their own editors, draw
in the area view or a panel, be offered in File › Open by a file's type,
change what is selected, write files other than haks and those of a folder the user
chose, or read a file a setting's path names. These are planned for later versions of the API; what
you'd use them for is welcome on the
[issue tracker](https://github.com/jadzziaa/moonglow-toolset/issues).
