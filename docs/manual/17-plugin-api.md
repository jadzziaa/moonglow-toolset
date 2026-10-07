---
type: Manual Page
title: Plugin API reference
description: Reference of the plugin API - mg, ctx.module, ctx.game, ctx.edit, ctx.hak, ctx.terrain, logging, progress and UI (files and folders the user chooses), pictures, ctx.plugin, Luau, limits and what the API doesn't have yet.
tags: [manual, plugins, api]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-07T08:30:00Z }
---

# Plugin API reference

Everything a plugin's code can use, for plugin API 0.2. The API is
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
| `mg.api` | The plugin API of the Moonglow running, as text: `"0.2"`. |
| `mg.command(id, handler)` | Registers the function for the command the manifest declares as `id`. `handler(ctx)` makes its edits with `ctx.edit`. It may return what Undo calls the change, as `{ label = "…" }` or the text alone; else that is the command's title. |
| `mg.check(id, handler)` | Registers the function for the check the manifest declares as `id`. `handler(ctx)` returns a list of findings (below), or nothing for none. A check that edits fails. |
| `mg.image(bytes, kind?)` | A picture decoded, as an `image` (below): a PNG, a TGA or a DDS, told apart by how the bytes start, or by `kind` (`"png"`, `"tga"`, `"dds"`). |
| `mg.path(struct)` | Where a struct read with `gff` is, as a path for `ctx.edit`: `""` for the root, `"/ClassList[0]"`. Nil for a table that is no such struct. |

An id is registered once; registering one the manifest doesn't declare,
or not registering one it does, is a fault `mg plugin check` reports.

A **finding** is a table:

| Field | |
| --- | --- |
| `resource` | The resource at fault, `name.ext`. Needed. |
| `message` | What is wrong. Needed. |
| `at` | Where in the resource: a field, a row. |
| `severity` | `"warning"` or `"error"`; else the check's, from the manifest. |

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
| `ctx.game:gff(name)` | As `ctx.module:gff`, of the game's resource (a stock blueprint, say). |
| `ctx.game:raw(name, path?)` | As `ctx.module:raw`. |
| `ctx.game:text(name)` | As `ctx.module:text` (`nwscript.nss`, say). |
| `ctx.game:bytes(name)` | As `ctx.module:bytes`. |
| `ctx.game:table(name)` | A 2DA by its name without extension (`"classes"`), as a `twoda` (below). |
| `ctx.game:string(strref)` | A talk-table string, from the custom talk table for a reference with the custom flag; nil if there is none. |

A `twoda`:

| | |
| --- | --- |
| `twoda.rows` | How many rows it has. |
| `twoda.columns` | Its columns' names, a list. |
| `twoda:get(row, column)` | A cell's text; rows count from 0, as in the file, and columns are named without regard to case. Nil for an empty cell (`****`), and for a row or column the table lacks. |

Without a game installation `has` is false, `string` is nil and the rest
fail. `ctx.game` is only read: a plugin changes the module, never the
game's data or a hak.

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
| `ctx.ui:open_folder(options?)` | Asks the user for a folder to read the files of: a `folder` (below), or nil. `options` is `{ title = "…" }`. |
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
| `folder:text(name)` | A file as text: UTF-8 if it is, else from the Windows-1252 codepage. |
| `folder:to_hak(hak, file, name?)` | Puts a file of the folder into a hak as `ctx.hak:write` does, without reading it into the plugin's memory: as `name`, or under its own. |

An `image`, from `mg.image`:

| | |
| --- | --- |
| `image.width`, `image.height` | Its size in pixels. |
| `image:pixel(x, y)` | A pixel's red, green, blue and alpha, each 0 to 255: four values. Pixels count from 0, from the top left. A gray picture has the same red, green and blue; 16 bits a channel are read as 8. |

Where there is nobody to ask (a check; `mg plugin run`): a message is
logged, `confirm` is false (true with `--yes`), and a form is nil in a
check and takes its defaults and `--answer`s with `mg`. A file or a
folder is nil in a check; with `mg` it is the next `--file PATH` given.
Writing a hak is refused in a check, and with `mg` unless `--yes` is
given (the haks go to `--hak-dir`, or the user folder's `hak`).

## `ctx.plugin`

| | |
| --- | --- |
| `ctx.plugin.id` | The plugin's id, from its manifest. |
| `ctx.plugin.name` | Its name. |
| `ctx.plugin.version` | Its version. |

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
`ctx.ui:open_folder`) and allows (`ctx.hak`).

## Limits

- A job has 256 MB of memory; past that it fails.
- A job is stopped when the user cancels; no `pcall` catches that.
- Nothing is kept between jobs, and a plugin can't run by itself: only
  when its command is chosen, or its check runs with Verify Module.
- Text must fit the Windows-1252 codepage.

## What the API doesn't have yet

Plugins can't yet react to events (a module opened, a resource saved),
add panels or their own editors, draw in the area view, read what is
selected in an area, write files other than haks, or keep settings. These are planned for later versions of the API; what
you'd use them for is welcome on the
[issue tracker](https://github.com/jadzziaa/moonglow-toolset/issues).
