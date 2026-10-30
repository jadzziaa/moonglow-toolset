---
type: Manual Page
title: Plugin API reference
description: Reference of the plugin API - mg, findings and their fixes (ctx.fix), ctx.selection and ctx.open, ctx.event (the events, a resource's own around a save, objects placed or pasted, resources made, and what a handler may edit), ctx.module, ctx.game, ctx.edit, ctx.hak, ctx.terrain, logging, progress and UI (files and folders the user chooses), pictures, ctx.plugin, ctx.settings, Luau, limits and what the API doesn't have yet.
tags: [manual, plugins, api]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-28T23:49:22Z }
---

# Plugin API reference

Everything a plugin's code can use, for plugin API 0.4. The API is
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
| `mg.api` | The plugin API of the Moonglow running, as text: `"0.4"`. |
| `mg.command(id, handler)` | Registers the function for the command the manifest declares as `id`. `handler(ctx)` makes its edits with `ctx.edit`. It may return what Undo calls the change, as `{ label = "…" }` or the text alone; else that is the command's title. |
| `mg.check(id, handler)` | Registers the function for the check the manifest declares as `id`. `handler(ctx)` returns a list of findings (below), or nothing for none. A check that edits fails. |
| `mg.on(id, handler)` | Registers the function for the event the manifest declares as `id` (`[event]`, its `on` one of the events of [`ctx.event`](#ctxevent)). `handler(ctx)` is called when it happens, as a command's is, with what happened as `ctx.event`: `ctx.event.name`, and what the event is of. It may return what Undo calls its change. |
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
| `ctx.fix(label)` | A fix to give a finding (`fix = …`), or a warning or a refusal of a save (`ctx.event:warn(text, fix)`): `label` is what its button says ("Upper-case this tag"). Called with a dot. |
| `fix.label` | Its label. |
| `fix:set`, `fix:remove`, `fix:insert`, `fix:remove_item`, `fix:write`, `fix:write_bytes`, `fix:write_gff`, `fix:delete` | As `ctx.edit`'s (below), with the same arguments: each records an edit of the fix. Nothing is changed: the user makes the fix (Verify's button, `mg verify --fix`), as one step of Undo named by its label. |

Each fix starts from the module as the check reads it, and its edits
apply to it alone: the check's reads (`ctx.module`) and the other fixes
do not see them; a fix's later edits see its earlier ones. A fix with no
edits is none. A fix is made only while the resources it changes are as
the check read them (else the user is told to verify again), or changed
since only by fixes of the same Verify that set fields. A fix asks the
user nothing.

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

## `ctx.selection`, `ctx.open`

What a command was run on, and what is open in the window (since API
0.4). They are read as `ctx.module` is, with nothing asked: they only
say which of the module's objects the user means, and a plugin reads the
whole module anyway.

| | |
| --- | --- |
| `ctx.selection` | What the command is run on: a list of entries (below), from 1. From a right-click menu (a command whose manifest says what it is `for`), what the menu was opened on: the objects selected in the area view (a right-click on an object selects it, as for every command of that menu), the blueprints chosen in a palette (Ctrl+click chooses several), the object or blueprint of a row of the module tree, or an area. From the Plugins menu, the Command Palette or a key: the objects selected in the area view in front, else the blueprints selected in the palette, else none. Empty in a check and in an event's handler. |
| `ctx.open` | The area or editor in front (of the middle of the window, or of the pane with the focus), as an entry; nil if there is none. An area's view and its Properties are the area; an object's or a blueprint's Properties the object or the blueprint; another editor its resource, with its type as `kind` (`"nss"`, `"dlg"`); a copy the resource browser opened as one layer has it (an overridden copy, read-only), that resource with its type as `kind` and its `layer`. |

An entry:

| Field | |
| --- | --- |
| `selected.kind` | What it is: `"creature"`, `"door"`, `"encounter"`, `"item"`, `"placeable"`, `"sound"`, `"store"`, `"trigger"` or `"waypoint"` (placed, or a blueprint), or `"area"`. |
| `selected.resource` | The resource it is in: for a placed object its area's GIT (`"tavern.git"`), for a blueprint the blueprint (`"guard.utc"`), for an area its ARE (`"tavern.are"`). |
| `selected.path` | Where in the resource (see [Paths](#paths)): `"/Placeable List[3]"` for a placed object, `""` for one that is the resource's root. A field of it is `s.path .. "/Tag"`, and `ctx.module:raw(s.resource, s.path)` reads it. |
| `selected.blueprint` | True for a blueprint; false for a placed object and an area. |
| `selected.module` | Whether it is the module's: false for a blueprint of the game's (of a standard palette), which `ctx.game` reads and no edit can change. |
| `selected.area` | The area's resref, for a placed object and an area; else nil. |
| `selected.index` | A placed object's place in its list, from 0 as in its path; else nil. |
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
| `resource-created` (0.4) | a wizard (area, creature, item, placeable, door…), Add to Palette, Setup Store or a copy (Copy As) makes resources | for each resource made | in the making's own step of Undo |

| | |
| --- | --- |
| `ctx.event.name` | The event's name (`"resource-saving"`). |
| `ctx.event.resources` | `module-saved`: the names of the resources not as they were at the last save (or when the module was opened). `resource-saving`, `resource-saved`: every resource the save writes. |
| `ctx.event.resource` | The resource the handler is called for (`"tavern.are"`): of a save, or made. |
| `ctx.event.kind` | What that resource is: `"area"`, `"creature"`… (as a selection's `kind`), else its type (`"nss"`, `"dlg"`). |
| `ctx.event.old` | Of a save: the resource as it was, a GFF as plain values (as `ctx.module:gff` reads one) and another as text; nil for one the save writes for the first time. As it was is as its file was last read or saved; for a module kept in one file, as it was when last saved or opened. Read when first used. |
| `ctx.event.new` | Of a save: the resource as the save writes it (the script editors' unsaved text too), in the same forms; nil for one it deletes. `ctx.module` reads the whole module so. |
| `ctx.event.changed` | Of a save: for a GFF that was there and still is, what differs, as lines (`/Tag: cexostring "TAVERN" != cexostring "INN"`, at most 100); else nil. |
| `ctx.event.made` | `resource-created`: every resource the user's action made (an area by its ARE, a script by its source). |
| `ctx.event.objects` | `object-placed`, `object-pasted`: the objects (of the kinds `for` names), each an entry as `ctx.selection`'s (`resource`, `path`, `kind`, `index`, `tag`, `resref`…). |
| `ctx.event.area` | Their area's resref. |
| `ctx.event.by` | What made the objects or the resources: `"wizard"`, `"add-to-palette"`, `"setup-store"`, `"copy"`, `"palette"`, `"gallery"`, `"light"`, `"create-waypoint"` or `"paste"`. |
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
one event have 5 seconds (all of a save's resources together); past
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
`ctx.terrain:new_area`, `ctx.edit:write_gff`), Save As (to another
place), an import, the Script Wizard and New Script, a NUI window, and
`mg`'s own work but for a save.

**With `mg`.** `mg set` and `mg apply` run the `resource-saving` and
`resource-saved` handlers of the plugins `--plugins` names and of the
project's own plugins as the builder allowed them: a refusal is an error
and nothing is written; a warning is said, and the save goes on (the
JSON's `said` has each). `mg` makes nothing by a wizard or a paste, so
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
| `twoda:set(row, column, value)` | Sets a cell: text, or a number; nil empties it. The row and the column must be there. |
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
| `tlk:set(line, value)` | Sets a line (the table grows to hold it): its text, or a table with `text`, `sound` and `sound_length`; nil empties it. |
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
as `name.hak.bak`), and a hak that isn't there is made. **Undo does not
take a hak back.** A check can't write one.

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
| `area:cross(crosser, path)` | A crosser drawn through cells, as the crosser brush dragged through them: `path` is a list of cells, each beside the one before (`{ { 1, 5 }, { 1, 6 }, { 2, 6 } }`), and the crosser goes on the edges between them. False when a cell on the way has no tile for it. |
| `area:erase(x, y)` | The Eraser on a cell: its crossers go, and a group or a feature that has a tile there goes whole. False when refused. |
| `area:set_tile(x, y, id, orientation?, height?)` | Puts one tile into a cell as it is given, whether it fits its neighbors or not. |

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
| `type` | `"text"` (as when left out), `"number"`, `"check"` or `"choice"`. |
| `default` | What it starts with: text, a number, true or false, or one of the choices. |
| `min`, `max` | A number's limits. |
| `choices` | A choice's options, a list of texts; needed for one. Its value is the text chosen. |

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
| `ctx.settings.<id>` | A setting's value: text (a `string`, a `choice`, a `path`), a whole number (an `int`) or true or false (a `bool`). An id the manifest doesn't declare is an error. |
| `for id, value in ctx.settings` | Each setting and its value, in the manifest's order. |
| `ctx.settings:set(id, value)` | Sets one: `value` of its kind, within its limits or among its choices (else an error), or nil for its default. The job reads it back at once; Moonglow keeps it where its scope says when the job is done, and not if the job fails. A setting of scope `project` or `user-project` can't be set without a project (a module kept in one file has none). Not in a check, nor in the console. |

`ctx.settings.x = …` is an error: a setting is set with `set`. With `mg
plugin run`, a setting of scope `user` has its default (or `--setting
ID=VALUE`), and what the plugin sets of that scope is not kept; those of
the project's scopes are read from, and kept in, the project's files.

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
- The handlers of an event that runs at once, within what the user
  does (a save, a paste, a wizard's Finish), have 5 seconds; past that
  they are stopped ([`ctx.event`](#ctxevent)).
- A job is stopped when the user cancels; no `pcall` catches that.
- Nothing is kept between jobs but the plugin's settings
  (`ctx.settings:set`). A plugin runs when its command is chosen, when
  its check runs with Verify Module, and by itself only on the events its
  manifest declares (`mg.on`).
- Text must fit the module's codepage (Windows-1252, unless a hak of the
  module has an `encoding.2da`).

## What the API doesn't have yet

Plugins can't yet react to the selection changed, an area opened or a
field typed into, warn of a paste or refuse one, add
panels or their own editors, draw in the area view, change what is
selected, write files other than haks and those of a folder the user
chose, or read a file a setting's path names. These are planned for later versions of the API; what
you'd use them for is welcome on the
[issue tracker](https://github.com/jadzziaa/moonglow-toolset/issues).
