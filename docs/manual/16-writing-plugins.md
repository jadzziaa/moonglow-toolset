---
type: Manual Page
title: Writing plugins
description: Writing plugins - a first plugin, how a plugin runs, the manifest, reading and editing the module, the game's data, talking to the user, files and haks outside the module, areas and terrain, checks, several files, trying and testing, editor types and versions.
tags: [manual, plugins, luau]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-08T22:46:53Z }
---

# Writing plugins

A plugin is a folder with a manifest (`plugin.cfg`: who it is and what it
adds) and scripts in [Luau](https://luau.org), a small, fast variant of
Lua with optional types. This chapter builds one and explains how plugins
work; the [Plugin API reference](17-plugin-api.md) lists everything a
plugin can call. What plugins are to the people who use them is in
[Plugins](15-plugins.md).

You need a text editor and Moonglow. Nothing is compiled or built.

The plugin API is **experimental** (version 0.2): it may change in a
later release, and Versions, below, says what that means for a plugin.

## A first plugin

Open **Plugins › Manage Plugins…** and press **Open Folder**. In the
plugins folder, make a folder `hello` with two files. The manifest,
`plugin.cfg`:

```ini
[plugin]
id = "example.hello"
name = "Hello"
version = "0.1.0"
api = "0.1"
license = "GPL-3.0-or-later"
description = "My first plugin: counts the module's creatures, and stamps them."
author = "Your Name"

[command]
id = "count"
title = "Count Creatures"
hint = "How many creature blueprints the module has"
```

and the code, `main.luau`:

```lua
local mg = require("@moonglow")

mg.command("count", function(ctx)
    local creatures = ctx.module:resources("utc")
    ctx.log:info(`The module has {#creatures} creature blueprints`)
end)
```

Back in Manage Plugins press **Reload**, tick **Hello**, and with a
module open choose **Plugins › Count Creatures**. The log says how many
there are.

What happened: the manifest declared a command, so Moonglow could list it
without running anything. When you chose it, Moonglow ran `main.luau`,
which registered a function for the command's id with `mg.command`, and
called that function with `ctx`: the plugin's whole view of the world.

Now a command that changes the module. Add to `plugin.cfg`:

```ini
[command]
id = "stamp"
title = "Stamp Creatures…"
hint = "Write a note into every creature blueprint's comment"
key = "Ctrl+Alt+H"
```

and to `main.luau`:

```lua
mg.command("stamp", function(ctx)
    local creatures = ctx.module:resources("utc")
    if not ctx.ui:confirm(`Write a note into {#creatures} creatures' comments?`) then
        return
    end
    for i, name in creatures do
        ctx.progress(i, #creatures, name)
        local creature = ctx.module:gff(name)
        ctx.edit:set(name, "Comment", `{creature.Tag}: seen by Hello`)
    end
    return { label = "Stamp creature comments" }
end)
```

**Reload**, and run it (Plugins menu, the Command Palette, or
Ctrl+Alt+H). It asks, shows its progress, and the log says what it
changed. **Edit › Undo Stamp creature comments** takes all of it back at
once.

This plugin is the `hello` example; it and the others
([examples](https://github.com/jadzziaa/moonglow-toolset/tree/develop/docs/plugins/examples))
are run by Moonglow's tests, so they work with the Moonglow they come
with.

## How a plugin runs

**A fresh start for every job.** Each time a command or a check runs,
Moonglow starts a new Luau machine, runs the plugin's `main.luau` from
the top, and calls the one handler it needs. Nothing is kept from one
run to the next: a global set in one run is gone in the next. So
`main.luau` should do little more than register handlers.

**A private copy of the module.** The job works on a snapshot of the
module as it was when the job began. An edit is applied to that copy at
once (the code reads back what it wrote, and an edit that can't be made
fails on the line that makes it) and is recorded. The module you see is
not touched while the job runs.

**Edits are data.** When a command's handler returns, Moonglow takes the
recorded edits and applies them to the real module the way it applies an
editor's change: as one undoable step, all or none. If the handler fails
or is canceled, the edits are thrown away and nothing changes. A plugin
has no other way to change anything.

**A sandbox.** The code has [Luau's own libraries](https://luau.org/library)
(`string`, `table`, `math`, `utf8`, `bit32`, `buffer`, `vector`,
`coroutine`, and of `os` only the clock and the date) and `ctx`. There
are no files of its own choosing, no network, no other programs, and the
libraries can't be replaced. What it reaches outside the module, the
user hands it: a file or a folder they choose for it to read, and a hak
they allow it to write ([Outside the module](#outside-the-module)). `print` writes to the log. A job may use 256 MB of memory.
**Cancel** stops it between any two steps, even in a loop that never
ends; `pcall` does not catch that.

**Checks only read.** A check gets the same `ctx`; if it makes an edit,
it fails. Nobody is there to answer a check's questions (it runs among
many during Verify Module): `ctx.ui:confirm` says no and `ctx.ui:form`
returns nothing.

## The manifest

`plugin.cfg` is written like `nasher.cfg`: sections in brackets,
`key = "value"` lines, `;` or `#` for comments. Moonglow reads it without
running the plugin, to list what it adds.

`[plugin]`, once:

| Key | |
| --- | --- |
| `id` | Needed. The plugin's name for programs, the same wherever it is installed: `author.plugin-name`, in lower-case letters, digits, `.`, `-` and `_`. Settings and keys are kept under it, so don't change it between versions. |
| `name` | Needed. Its name for people. |
| `version` | Needed. The plugin's own version, as you number it. |
| `api` | Needed. The plugin API it was written for: `"0.2"`, or `"0.1"` for a plugin that uses nothing 0.2 added (it then runs in Moonglow before 1.16 too). |
| `license` | Needed. Its license, as an [SPDX](https://spdx.org/licenses/) name. Moonglow is GPL-3.0, and a plugin runs inside it: choose a license compatible with the GPL (`GPL-3.0-or-later`, `MIT`, `Apache-2.0`…). |
| `description` | One or two sentences on what it does, shown in Manage Plugins. |
| `author` | Who wrote it. Repeat the line for each author. |
| `entry` | The script that registers the handlers; `main.luau` unless given. A file of the plugin's folder. |

`[command]`, once for each command:

| Key | |
| --- | --- |
| `id` | Needed. The id the code registers it under. |
| `title` | Needed. Its name in the Plugins menu, written as menu commands are (`Fix Creature Tags`; with `…` at the end if it asks before it acts). |
| `hint` | What its tip says. |
| `key` | A key it has until the user chooses another: `Ctrl+Alt+T` (`Ctrl`, `Alt`, `Shift` and a key's name; Ctrl is Cmd on macOS). A key something else has already is reported as a conflict in Options › Keyboard, so choose an unusual one, or none. |

`[check]`, once for each check:

| Key | |
| --- | --- |
| `id` | Needed. The id the code registers it under. Findings carry `plugin-id/check-id`. |
| `title` | Needed. What the check holds to be true (`Creature tags are upper case`). |
| `severity` | `"warning"` (as when left out) or `"error"`: how bad its findings are, unless a finding says otherwise. |

A key or a section Moonglow doesn't know is an error (a misspelled one,
most likely), and so is a command declared but not registered, or
registered but not declared. `mg plugin check` reports all of these.

## Reading the module

Resources are named as files are: `guard.utc`, `on_spawn.nss`.

```lua
ctx.module:resources()         -- every resource's name, sorted
ctx.module:resources("utc")    -- those of one type
ctx.module:has("guard.utc")    -- true or false
```

`ctx.module:gff(name)` reads a GFF resource (a blueprint, an area, the
module's `module.ifo`…) as a table of plain values:

```lua
local guard = ctx.module:gff("guard.utc")
guard.Tag                      -- "GATE_GUARD": text
guard.Str                      -- 14: a number
guard.ClassList[1].ClassLevel  -- a list's items, counted from 1
guard.FirstName[0]             -- a name's English text
guard.FirstName.strref         -- its talk-table reference, if it has one
guard.Nothing                  -- nil: no such field
```

- Every number field is a Luau number, text and resource names are
  strings, a struct is a table, a list is its items.
- A localized string is a table of its texts by language number (0 is
  English, 2 French, 4 German…; add 1 for the feminine form), with
  `strref` if it has a talk-table reference.
- The table is a copy: changing it changes nothing. Edits are made with
  `ctx.edit`.
- Field names are the file format's (`Tag`, `ClassList`, `ScriptSpawn`),
  as `mg gff` prints them. The console shows a resource's fields:
  `return ctx.module:gff("guard.utc")`.

`ctx.module:raw(name)` reads the same resource with every field's type,
as `mg gff` prints it (`{ type = "cexostring", value = "GATE_GUARD" }`),
for when the type matters. `ctx.module:text(name)` reads a script or any
other text; `ctx.module:bytes(name)` reads anything as it is.

## Editing

```lua
ctx.edit:set("guard.utc", "Tag", "GATE_GUARD")
ctx.edit:set("guard.utc", "/ClassList[0]/ClassLevel", 5)
ctx.edit:remove("guard.utc", "Comment")
```

A field is named by its **path** from the resource's root: its label
alone for a field of the root, else labels joined with `/`, a list's item
as `List[index]`. **In a path, items count from 0**, as in the file and
in `mg set`; in the tables that `gff` returns they count from 1, as
everything in Luau does. To avoid the arithmetic, ask a struct you have
read where it is:

```lua
local class = guard.ClassList[1]
ctx.edit:set("guard.utc", mg.path(class) .. "/ClassLevel", class.ClassLevel + 1)
```

**Types.** `set` writes a plain value as the type the field has. A field
the resource lacks takes the type the game's files give that field; a
field Moonglow has never seen needs its type from you, either way:

```lua
ctx.edit:set("guard.utc", "Rank", 3, "byte")
ctx.edit:set("guard.utc", "Rank", mg.byte(3))
```

A number that doesn't fit the type, text where a number goes, or a name
too long for a resource name is an error, not a silent truncation.

**Names and descriptions.** Text set on a localized string becomes its
English text; its talk-table reference and other languages stay. To set
all of it, give a table: `{ strref = 1234 }`, `{ [0] = "Guard", [4] =
"Wache" }`.

**Lists.** An item is added as a struct written the way `mg gff` writes
one (its fields as typed values, `__struct_id` for its id), and removed
by its path:

```lua
local at = ctx.edit:insert("guard.utc", "/ClassList", {
    __struct_id = 2,
    Class = mg.int(4),
    ClassLevel = mg.short(1),
})                                              -- at the end; `at` is its index
ctx.edit:remove_item("guard.utc", "/ClassList[0]")
```

**Whole resources.**

```lua
ctx.edit:write("on_spawn.nss", "void main() { }\n")    -- text
ctx.edit:write_bytes("map.tga", bytes)                  -- anything
ctx.edit:write_gff("copy.utc", ctx.module:raw("guard.utc"))
ctx.edit:delete("old_guard.utc")
```

What Moonglow keeps in step when you edit in the window (an item's cost,
a creature's hit points and challenge rating) it keeps in step after a
plugin's edits in the window too. Everything else is written as given: a
plugin that renames a blueprint's resource must also set its
`TemplateResRef`, as the file format has it.

Text in a module is in the Windows-1252 codepage, or in the module's own
table of letters where a hak of its has one (`encoding.2da`). Luau
strings are UTF-8; Moonglow converts both ways, and text with a character
the codepage lacks is an error.

## The game's data

`ctx.game` reads what the game would load, with the module's haks and
custom talk table: the same `resources`, `has`, `gff`, `raw`, `text` and
`bytes` as `ctx.module`, and

```lua
local classes = ctx.game:table("classes")   -- a 2DA, by name
classes.rows                                -- how many rows
classes.columns                             -- its columns' names
classes:get(6, "Label")                     -- "Paladin"; rows count from 0
classes:get(6, "Nothing")                   -- nil: no such column, or ****
ctx.game:string(12)                         -- a talk-table string, or nil
```

A cell is text, as in the file: use `tonumber` for numbers. Without a
game installation (possible with `mg`), there is nothing to read:
`ctx.game:has` is false, `ctx.game:string` is nil, and the others fail.

## Talking to the user

```lua
ctx.log:info("…")     ctx.log:warn("…")     ctx.log:error("…")
ctx.progress(done, total, "what it is at")
ctx.ui:message("Something to read, with OK.")
if ctx.ui:confirm("Delete 12 unused scripts?") then … end
```

A form asks for several things at once:

```lua
local answers = ctx.ui:form({
    title = "Markup",
    fields = {
        { id = "sell", label = "Sells at (%)", type = "number", default = 150, min = 1, max = 1000 },
        { id = "only", label = "Only tags starting", type = "text", default = "" },
        { id = "stores", label = "Kind", type = "choice", choices = { "All", "Shops", "Inns" } },
        { id = "dry", label = "Only report", type = "check", default = false },
    },
})
if not answers then return end     -- closed without OK
answers.sell                       -- 150
```

Ask before the first edit: a command that is going to change nothing
should leave no step for Undo. With `mg plugin run` there is nobody to
ask: a message goes to the output, `confirm` is no unless `--yes` is
given, and a form takes its defaults and `--answer ID=VALUE`. So give
every field a sensible default. A file or folder asked for is the next
`--file PATH` on the command line, and haks are written (with `--yes`)
into `--hak-dir`, or the user folder's `hak`.

A command's handler may return what Undo calls its change,
`return { label = "Upper-case creature tags" }`; without one, Undo names
the command's title.

## Outside the module

A plugin opens no files. It asks, and the user chooses:

```lua
local file = ctx.ui:open_file({ title = "The height map", extensions = { "png", "tga" } })
if not file then return end                 -- they chose none
file.name                                   -- "hills.png"
local image = mg.image(file.bytes)          -- a PNG, TGA or DDS decoded
image.width, image.height
local r, g, b, a = image:pixel(0, 0)        -- 0 to 255, from the top left

local folder = ctx.ui:open_folder({ title = "The folder with the tileset's files" })
folder:files()                              -- { "tiles.set", "textures/grass.tga", … }
folder:text("tiles.set")                    -- a file of it; nothing outside it
```

And it writes one kind of file, a hak in the user's hak folder, once the
user has said it may (they are asked the first time the job writes to
each hak; if they refuse, the job ends there):

```lua
ctx.hak:write("my_tiles", "tiles.set", text)        -- a resource, from bytes
folder:to_hak("my_tiles", "textures/grass.tga")     -- a file of the folder, copied
ctx.hak:attach("my_tiles")                          -- the module lists the hak
```

The hak is written when the handler returns. Resources are added to a
hak that is already there, and a copy of it is kept as `my_tiles.hak.bak`.
Unlike the module's edits, a hak is not undone by Undo: say what the
command writes in its title or its form. `folder:to_hak` copies a file
whatever its size; `ctx.hak:write` is for what the plugin computed.

## Areas and terrain

`ctx.terrain` makes areas and paints them the way the area editor does:
the plugin says what the ground is at each corner, and Moonglow chooses
tiles that fit.

```lua
local resref = ctx.terrain:new_area({ name = "High Field", tileset = "ttr01", width = 8, height = 8 })
local area = ctx.terrain:open(resref)       -- or an area the module has
area.terrains                               -- { "Grass", "Water", "Trees", … }
area:corner(3, 3)                           -- "Grass", 0: its terrain and height
area:paint(1, 1, "Water")                   -- the terrain brush on a corner
area:set_height(5, 5, 2)                    -- raised until it is two steps high
area:place_group("Farm House", 2, 2)        -- a group, as from the palette
```

Corners count from the south-west, from 0 to the area's width and
height; a cell has the corners at its own number and the next. Each of
these returns false, and changes nothing, where the area editor would
refuse the brush: no tile of the tileset fits there. Heights are whole
steps (`area.step` metres each), and neighbors are never more than a
step apart, so raising one corner by two lifts the corners around it by
one.

A tileset the job is putting into a hak can be used at once, before the
hak exists: `tileset-import` in the examples takes a folder of tileset
files (as NWN Mapper exports them), puts them into a hak, and makes an
area of one of the tileset's groups.

## Checks

A check returns its findings, each naming a resource:

```lua
mg.check("tag-case", function(ctx)
    local findings = {}
    for _, name in ctx.module:resources("utc") do
        local tag = ctx.module:gff(name).Tag
        if tag ~= string.upper(tag) then
            table.insert(findings, {
                resource = name,
                at = "Tag",
                message = `tag "{tag}" is not upper case`,
            })
        end
    end
    return findings
end)
```

`at` says where in the resource (a field, a row); `severity` (`"warning"`
or `"error"`) overrides the one the manifest gives the check. A check
that finds nothing returns an empty list.

A rule that both a check and a command need (the check finds, the command
fixes) belongs in a function both call: the `tag-conventions` example
does that.

## More than one file

`require("rules")` runs `rules.luau` of the plugin's folder, once, and
gives what it returns; `require("lib/names")` reads from a folder inside
it. Nothing outside the plugin's folder can be required, except
`"@moonglow"`, which is the API.

```lua
-- rules.luau
local rules = {}
function rules.wanted(tag: string): string
    return string.upper(tag)
end
return rules
```

## Trying and testing

- **The console** (Manage Plugins) runs a few lines on the open module
  with `mg` and `ctx` at hand: the quickest way to try a call.
- **Reload** in Manage Plugins reads the manifest again. The code is
  read anew each time a command runs, so after changing only `main.luau`
  just run the command again.
- **An error** names the script and the line, in the log; nothing was
  changed.
- **`mg plugin check DIR`** checks a plugin without a module: the
  manifest reads, the code loads, and each registers what the other
  declares. It is a good first step in a plugin's own tests. Given an
  archive (`mg plugin check hello.zip`), it checks what the installer
  would, and the code as it is in the archive: do that before you send
  one to anybody.
- **`mg plugin run MODULE DIR COMMAND --dry-run`** prints the edits a
  command would make, as an edit file
  ([Command-line tools](11-command-line.md)), and changes nothing: run it
  on a copy of a module and compare with what you expect.
- While you work on a plugin in the plugins folder, Install from File
  leaves its folder alone: it only ever replaces what it installed.

Try a new plugin on a copy of your module first.

## Types in your editor

Luau can check types, and an editor with the
[Luau language server](https://github.com/JohnnyMorganz/luau-lsp) completes
`ctx.` and flags a misspelled call. Moonglow's source has the API's
types in
[`docs/plugins/types/moonglow.luau`](https://github.com/jadzziaa/moonglow-toolset/tree/develop/docs/plugins/types).
Copy it next to your plugins and point the alias `moonglow` at it in a
`.luaurc` file in or above the plugin's folder:

```json
{ "aliases": { "moonglow": "./types/moonglow" } }
```

Moonglow never reads that file; it only tells the editor what
`require("@moonglow")` gives. A handler's `ctx` is then typed:

```lua
local mg = require("@moonglow")

mg.command("count", function(ctx: mg.Context)
    …
end)
```

## Versions

The manifest's `api` names the plugin API the plugin was written for.
While the API is at 0.x, a new version may change it, and a plugin
written for another version is refused with a message that says so
(rather than failing somewhere inside). The API's changes are listed in
[`docs/plugins/CHANGES.md`](https://github.com/jadzziaa/moonglow-toolset/tree/develop/docs/plugins)
with what a plugin has to change. `mg.api` is the API version of the
Moonglow that is running.

## Sharing

A plugin is shared as an archive of its folder:

```text
mg plugin pack hello
```

writes `example.hello-0.1.0.zip` (the plugin's id and version; `-o FILE`
names it otherwise). The plugin is checked first, as `mg plugin check`
does, and hidden files (`.git`, `.luaurc`) are left out. Whoever gets
the file installs it with **Plugins › Install Plugin from File…**.

A zip made any other way installs too, if it has what the installer
looks for:

- `plugin.cfg` at the archive's top, or in the one folder at its top
  (what zipping the folder gives, and a repository's download when the
  plugin is at the repository's top). One plugin to an archive.
- At most 500 files and 16 MB unpacked: a plugin is scripts and a little
  data.
- File names every system can have: no `:`, `*`, `?`, `"`, `<`, `>` or
  `|`, none that end in a dot or a space, none of Windows's device names
  (`aux.luau`), and no two that differ only in case.
- No links, nothing encrypted, and compressed the usual way (deflate)
  or not at all.

The plugin is installed into a folder named by its `id`, so keep `id`
the same in every version (a new version then replaces the old) and
raise `version`. Say in the plugin's notes which Moonglow it needs, and
choose a license others can use it under.
