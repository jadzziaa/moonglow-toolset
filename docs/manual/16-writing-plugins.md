---
type: Manual Page
title: Writing plugins
description: Writing plugins - a first plugin, how a plugin runs (events too, a resource's own around a save and the after-create hook), the manifest, reading and editing the module, commands on what is selected, the game's data, talking to the user, settings (ctx.settings, their scopes and where each is kept), files and haks outside the module, areas and terrain, checks and the fixes of their findings, several files, trying and testing (a plugin's own tests, mg plugin test), editor types, versions, sharing, and a project's own plugins.
tags: [manual, plugins, luau]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-28T11:25:06Z }
---

# Writing plugins

A plugin is a folder with a manifest (`plugin.cfg`: who it is and what it
adds) and scripts in [Luau](https://luau.org), a small, fast variant of
Lua with optional types. This chapter builds one and explains how plugins
work; the [Plugin API reference](17-plugin-api.md) lists everything a
plugin can call. What plugins are to the people who use them is in
[Plugins](15-plugins.md).

You need a text editor and Moonglow. Nothing is compiled or built.

The plugin API is **experimental** (version 0.4): it may change in a
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
or that folder if they allow it to write ([Outside the module](#outside-the-module)). `print` writes to the log. A job may use 256 MB of memory.
**Cancel** stops it between any two steps, even in a loop that never
ends; `pcall` does not catch that.

**Events.** A plugin may also act by itself: an `[event]` in the
manifest and a handler registered with `mg.on(id, handler)`, called as a
command's is, with what happened as `ctx.event`
([the events](17-plugin-api.md#ctxevent) and what each says). They are
of two kinds.

*After something happened:* a module opened or saved (`module-opened`,
`module-saved`), and since API 0.4 each resource a save wrote
(`resource-saved`). The handler runs as a job of its own once Moonglow
is free. `module-saved` gets `resources`, the names of the resources
that are not as they were at the last save (or when the module was
opened); its edits go in as one undoable step after the save, so the
module then has unsaved changes again: an event that edits should have
little to say. `resource-saved` only reads. Where such a handler asks
for a folder (`ctx.ui:open_folder`) or for leave to write (a hak, a
folder), the user is asked once and the answer is kept for that
plugin's events; a form is asked every time.

```lua
mg.on("emit", function(ctx)
    local out = ctx.ui:open_folder({ title = "The server's development folder" })
    if not out then return end
    for _, name in ctx.event.resources do
        if name:sub(-4) == ".utc" then
            out:write(name, ctx.module:bytes(name))
        end
    end
end)
```

*Within what the user does* (API 0.4): before a save, for each resource
it would write (`resource-saving`); when objects are placed or pasted
(`object-placed`, `object-pasted`); when a wizard, Add to Palette, Setup
Store or a copy makes a resource (`resource-created`). These run at once
(a save waits for them), with 5 seconds and nobody to ask. Before a
save a handler only reads, as a check does: it reads each resource as
it was (`ctx.event.old`) and as it will be (`ctx.event.new`), and warns
(`ctx.event:warn`) or refuses (`ctx.event:refuse`), with a fix made with
`ctx.fix`. The user sees what it said before anything is written: Save
Anyway or Cancel for a warning, no save for a refusal, and the fix as a
button. On what was placed, pasted or made a handler may edit at once,
and its edits go in with the user's own action, after Moonglow's own
work (the builder code in a ResRef, the project's rules for a store):
one Undo takes the paste and what the plugin did to it back together.
A plugin edits only on the user's word: a paste is the user's action,
and what a plugin adds to it is part of it; a save is not an edit, so a
plugin never changes what is being saved ([why](17-plugin-api.md#ctxevent)).

The `house-rules` example has one of each. Before a save, it holds each
area it would write to the team's rules (its manifest says
`on = "resource-saving"` and `types = "are"`):

```lua
mg.on("area-tags", function(ctx)
    local e = ctx.event
    if e.new == nil then
        return -- (an area deleted)
    end
    local tag = e.new.Tag or ""
    local was = if e.old then e.old.Tag or "" else nil
    if was and was ~= tag then
        local back = ctx.fix(`Put back {was}`)
        back:set(e.resource, "Tag", was)
        local why = `its tag was {was}, now {tag}: players' places are kept by it`
        if ctx.settings.keep_tags then
            e:refuse(why, back)
        else
            e:warn(why, back)
        end
        return
    end
    if tag ~= string.upper(tag) then
        local fix = ctx.fix(`Make it {string.upper(tag)}`)
        fix:set(e.resource, "Tag", string.upper(tag))
        e:warn(`tag {tag} is not upper case`, fix)
    end
end)
```

Its other handlers give a pasted waypoint a tag of its own
(`on = "object-pasted"`, `for = "waypoint"`) and a new area the team's
land (`on = "resource-created"`, `for = "area"`), both as part of what
the user did. Events happen in the window and, for a save, with `mg
set` and `mg apply` (`--plugins`); `mg plugin run --event` runs any
event's handler by hand.

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
| `api` | Needed. The plugin API it was written for: `"0.4"`, or an earlier one (`"0.3"`, `"0.2"`, `"0.1"`) for a plugin that uses nothing added since (it then runs in older versions of Moonglow too). |
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
| `for` | What it is for (since API 0.4): kinds of object, `creature`, `door`, `encounter`, `item`, `placeable`, `sound`, `store`, `trigger`, `waypoint`, and `area`, as a list (`"placeable, creature"`) or one to a line. It is then in their right-click menus too, and runs on what was clicked ([Commands on what is selected](#commands-on-what-is-selected)). |

`[check]`, once for each check:

| Key | |
| --- | --- |
| `id` | Needed. The id the code registers it under. Findings carry `plugin-id/check-id`. |
| `title` | Needed. What the check holds to be true (`Creature tags are upper case`). |
| `severity` | `"warning"` (as when left out) or `"error"`: how bad its findings are, unless a finding says otherwise. |

`[event]`, once for each thing the plugin does by itself:

| Key | |
| --- | --- |
| `id` | Needed. The id the code registers it under (`mg.on`). |
| `on` | Needed. What it waits for: `"module-opened"`, `"module-saved"` and, since API 0.4, `"resource-saving"`, `"resource-saved"`, `"object-placed"`, `"object-pasted"` or `"resource-created"` ([the events](17-plugin-api.md#ctxevent)). |
| `types` | For `resource-saving` and `resource-saved` (API 0.4): the types of resource it is for, as a list (`"are, git"`); left out, every one. |
| `for` | For `object-placed`, `object-pasted` and `resource-created` (API 0.4): the kinds of object, or `area`, it is for, as a command's `for`; left out, every one. |

`[setting]`, once for each value the plugin keeps (since API 0.4;
[Settings](#settings)):

| Key | |
| --- | --- |
| `id` | Needed. Its name as the plugin reads it, `ctx.settings.<id>`: lower-case letters, digits and `_`, from a letter (not `set`). |
| `type` | `"string"` (as when left out), `"int"` (a whole number), `"bool"`, `"choice"` or `"path"` (a file's or a folder's path, as text). |
| `default` | Its value until one is set: text, a whole number, `true` or `false`, or one of the choices. Left out: empty text, 0 (or `min`), false, the first choice. |
| `label` | What the settings' form calls it; else its id. |
| `help` | What the form's tip says of it. |
| `scope` | Where its value is kept, and who shares it: `"user"` (as when left out), `"project"` or `"user-project"` (below). |
| `choices` | A choice's options, as a list: `"creatures, placeables"`. Needed for one. |
| `min`, `max` | An int's limits. |

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
- A struct's id is kept beside its table, not in it: `mg.struct_id(s)`
  (since Moonglow 2.2.0). An equipped item's is its slot (`for _, e in
  guard.Equip_ItemList do local slot = mg.struct_id(e) … end`), so its
  fields are all the table holds, whatever they are labeled, and a copy
  of it for `ctx.edit:insert` gets the id as `__struct_id =
  mg.struct_id(e)`.
- Field names are the file format's (`Tag`, `ClassList`, `ScriptSpawn`),
  as `mg gff` prints them. The console shows a resource's fields:
  `return ctx.module:gff("guard.utc")`.

`ctx.module:raw(name)` reads the same resource with every field's type,
as `mg gff` prints it (`{ type = "cexostring", value = "GATE_GUARD" }`),
for when the type matters. `ctx.module:text(name)` reads a script or any
other text; `ctx.module:bytes(name)` reads anything as it is.

## Commands on what is selected

Since API 0.4 a command can act on what the user points at. Say in its
manifest what it is for, and it is in the right-click menus of those
kinds as well as the Plugins menu:

```ini
[command]
id = "set-local"
title = "Set Local Variable…"
for = "creature, door, encounter, item, placeable, sound, store, trigger, waypoint"
```

- A kind of object (`placeable`) is the objects of that kind placed in
  an area and their blueprints: the area view's menu on them (all the
  objects selected must be of the kinds the command is for), the module
  tree's rows of an area's objects and of blueprints, and a palette's
  rows. `area` is an area: the module tree's row of it, and the area
  view's menu when nothing is selected.
- In each menu the plugins' commands come together, after the menu's
  own, above Variables and Properties (which end a menu, as in Aurora).
- The handler gets what the menu was opened on as `ctx.selection`: every
  object selected in the area view, every blueprint chosen in a palette
  (Ctrl+click chooses several), or the one row. Chosen from the Plugins
  menu, the Command Palette or a key, it gets the objects selected in
  the area view, else the blueprints selected in the palette; it should
  say so when that is nothing.

Each entry of `ctx.selection` says what it is and where, as `ctx.edit`
takes it: `resource` is the area's GIT for a placed object (and `path`
its place in it, `/Placeable List[3]`), or the blueprint or the area
itself (`path` is then `""`). So one handler serves a placed object and
a blueprint alike:

```lua
mg.command("upper-tags", function(ctx)
    for _, s in ctx.selection do
        if s.module and s.tag then   -- (not one of the game's blueprints)
            ctx.edit:set(s.resource, s.path .. "/Tag", string.upper(s.tag))
            ctx.log:info(`{s.kind} {s.tag}: upper case`)
        end
    end
end)
```

`ctx.open` is the area or editor in front, an entry of the same shape
(or nil): a command for the area in view can use it. The
[reference](17-plugin-api.md#ctxselection-ctxopen) lists every field;
the example plugin `set-local` (in `docs/plugins/examples`) sets a local
variable on what was clicked and counts an area's objects. With `mg
plugin run`, `--select guard.utc --select "tavern.git/Placeable List[3]"`
gives the selection and `--open tavern.are` what is open.

A plugin reads the selection as it reads the module: no question is
asked. Its edits are one undoable step, as any command's.

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

## Settings

A plugin keeps values of its own as **settings** (plugin API 0.4): a
team's prefix, the builder's code, how strict a check is. The manifest
declares each (`[setting]`), and Moonglow keeps their values, shows them
in a form (Manage Plugins › **Settings…**) and hands them to every job.
The `builder-tags` example has five:

```ini
[setting]
id = "prefix"
label = "Team prefix"
help = "What the tag of each of the module's own creatures and placeables begins with (none: not checked)"
scope = "project"

[setting]
id = "kinds"
type = "choice"
label = "Checked"
help = "Which blueprints the check holds to the prefix"
choices = "creatures and placeables, creatures, placeables"
scope = "project"

[setting]
id = "max_length"
type = "int"
label = "Longest tag"
help = "A prefixed tag is cut to this many characters"
min = 1
max = 32
default = 32
scope = "project"

[setting]
id = "code"
label = "Your code"
help = "Your builder code, which comes after the team's prefix in the tags you make (yours alone, for this project)"
scope = "user-project"

[setting]
id = "strict"
type = "bool"
label = "Report as errors"
help = "A tag without the prefix is an error, not a warning (yours, in every module)"
```

Each setting's **scope** says where its value is kept, and who shares it:

| Scope | Whose | Kept in |
| --- | --- | --- |
| `user` | The user's, for every module | Moonglow's settings (`mg` keeps none: a run takes the default, or `--setting`) |
| `project` | The team's, for the project | The project's `moonglow.toml`, `[plugins."<plugin id>"]`: committed with the project, so every builder has it |
| `user-project` | The user's own, for the project | The project's `.moonglow/user.toml`, `[plugins."<plugin id>"]`: never committed |

The two project scopes need a project: a module folder or a nasher
project, whose files they are kept in. With a module kept in one file
(`.mod`) they read their defaults, and can't be set. A team can write its
values into `moonglow.toml` by hand, as
[the project file](03-modules.md#the-project-file) shows.

The plugin reads them as `ctx.settings`, each by its id; `for id, value
in ctx.settings` goes through them in the manifest's order. A value is
text (a string, a choice, a path), a whole number or true or false:

```lua
local function wanted(ctx): string
    local s = ctx.settings
    if s.code ~= "" then
        return `{s.prefix}{s.code}_`
    end
    return s.prefix
end
```

A command or an event may set one, `ctx.settings:set(id, value)` (nil:
back to its default). The value is held to the setting's kind and limits
at once, and the job reads it back; Moonglow keeps it, where its scope
says, when the job is done (not if it fails). A check only reads them.

```lua
mg.command("remember-code", function(ctx)
    local answer = ctx.ui:form({
        title = "Your builder code",
        fields = { { id = "code", label = "Code", default = ctx.settings.code } },
    })
    if answer == nil then
        return
    end
    ctx.settings:set("code", answer.code)
    ctx.log:info(`Your builder code is {answer.code}`)
end)
```

A value kept that is not of its setting's kind (`max_length = "long"`),
or kept in the other file, is said in the log when the project is
opened, and the setting has its default; one for a setting the plugin
doesn't declare (an older version's) is said and left where it is.
Moonglow writes only the values it sets, and keeps the rest of each file
as it is written.

A `path` is text to the plugin: the form has a button to choose it, but
the plugin reads no file by it (the sandbox). It still asks for a file
with `ctx.ui:open_file`.

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

It may also write files into a folder the user chose, once they have
said it may (asked the first time the job writes into each folder):

```lua
local out = ctx.ui:open_folder({ title = "Where the generated tables go" })
out:write("feat.2da", table:write())                -- a file of that folder, by its name
```

The files are written when the handler returns, in place of those of the
same names (no copy is kept, and Undo does not take them back). A name
is a name: not a path into a folder of the folder, nothing above it,
nothing hidden.

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

### A fix for a finding

A finding may carry a **fix**: edits that put it right, offered beside
the finding in Verify Module's report as a button, and made only when
the user clicks it (since API 0.4). `ctx.fix(label)` makes one: its
`label` is what the button says, and it has `ctx.edit`'s methods
(`set`, `remove`, `insert`, `remove_item`, `write`…), which record the
edits and make none. The `tag-conventions` example gives each finding
the fix of its own tag:

```lua
mg.check("tag-case", function(ctx)
    local findings = {}
    for _, name in ctx.module:resources("utc") do
        local tag = ctx.module:gff(name).Tag
        if tag ~= rules.wanted(tag) then
            local fix = ctx.fix("Upper-case this tag")
            fix:set(name, "Tag", rules.wanted(tag))
            table.insert(findings, {
                resource = name,
                at = "Tag",
                message = `tag "{tag}" is not upper case`,
                fix = fix,
            })
        end
    end
    return findings
end)
```

- A fix starts from the module as the check reads it, and the check goes
  on reading it so: what one fix records, neither the check nor another
  fix sees. Within a fix, an edit sees the ones before it (an item
  inserted, then a field of it set).
- A fix that records no edits is none: the finding has no button.
- The fix is made as one step of Undo, named by its label, and only
  while what it changes is as Verify found it: after other changes to
  those resources, the user is told to verify again. Fixes of one Verify
  that only set fields can be made one after another in one resource.
- Give the label words that say what it does to this finding ("Retag
  this store", "Set tdod_lit"). For a fix that needs a choice, offer a
  command instead: a fix asks nothing.
- On the command line, `mg verify --fix CHECK` makes a check's fixes
  (`--dry-run` says what it would make), and `mg verify --json` gives
  each finding's fix as an edit file
  ([Command-line tools](11-command-line.md)).

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
- **`mg plugin test DIR`** runs the plugin's own tests, below.
- While you work on a plugin in the plugins folder, Install from File
  leaves its folder alone: it only ever replaces what it installed.

Try a new plugin on a copy of your module first.

### A plugin's own tests

A plugin's tests are files `tests/*.toml` in its folder (plugin API
0.4). Each names a **fixture**, a small module made for the tests, and
lists what to run on it and what is to come of it.
`mg plugin test DIR` runs them all: a line a test (`ok`, or `FAIL` and
what was not as wanted), a summary, and an exit that fails if one test
does (or there are none), so that a build pipeline can run it. The
`tag-conventions` example has one file, `tests/tags.toml`:

```toml
module = "module"

[[test]]
name = "a lower-case tag is found, with its fix"
check = "tag-case"
[[test.finding]]
resource = "guard.utc"
at = "Tag"
message = 'tag "gate_guard" is not upper case'
severity = "warning"
fix = "Upper-case this tag"

[[test]]
name = "the fix upper-cases that tag, and the check finds no more"
check = "tag-case"
fix = "Upper-case this tag"
edits = 1
left = 0
[[test.field]]
resource = "guard.utc"
field = "Tag"
value = "GATE_GUARD"

[[test]]
name = "the command upper-cases every tag, in one step"
command = "fix-tags"
log = ["1 creature tags changed"]
resources = ["guard.utc"]
[[test.field]]
resource = "guard.utc"
field = "Tag"
value = "GATE_GUARD"
[[test.field]]
resource = "captain.utc"
field = "Tag"
value = "CAPTAIN"
```

```text
$ mg plugin test docs/plugins/examples/tag-conventions
ok	builtin.toml › Moonglow's tag-unique finds the gate's post twice
ok	builtin.toml › its fix retags the second post
ok	tags.toml › a lower-case tag is found, with its fix
ok	tags.toml › the fix upper-cases that tag, and the check finds no more
ok	tags.toml › the command upper-cases every tag, in one step
example.tag-conventions: 5 tests, 5 passed, 0 failed
```

(`builtin.toml` runs one of Moonglow's own checks, below.)

An event's test names what happened. The `house-rules` example's tests
(`tests/events.toml`) hold its save's handler to a module as it was
(`was`, a second fixture whose cellar had another tag) and its paste's
to the object pasted:

```toml
[[test]]
name = "a save that changes an area's tag is refused, with a fix that puts it back"
event = "area-tags"
was = "was"
refused = ["cellar.are: its tag was OLD_CELLAR, now CELLAR"]
warned = []

[[test]]
name = "a pasted waypoint gets a tag of its own, with the paste"
event = "pasted-waypoints"
select = ["tavern.git/WaypointList[1]"]
by = "paste"
log = ["1 pasted waypoints have tags of their own"]
edits = 1
[[test.field]]
resource = "tavern.git"
field = "/WaypointList[1]/Tag"
value = "WP_DOOR_2"
```

`module` is the fixture, from the test file's folder: a module folder, a
`.mod`, or a nasher project. The example's is a nasher project
(`tests/module/`, its resources as JSON in `src/`), which reads well in
a review; `mg init` makes one from a module. A test may name its own
`module`, and `mg plugin test DIR --module FIXTURE` runs every test on
another. The fixture is read, never written: each test starts from it as
it is on disk.

**The game's data a test reads** (`ctx.game`) is the game's where `mg`
finds a game (as `mg plugin run` does), with the fixture's haks and the
fixture layered in, and over it the test's **own game folder**: the
files of a folder `game/` beside the test file, where there is one (2DAs,
blueprints, scripts; since Moonglow 2.2.0). They are read as the game's
development folder is, over the game, the haks and the module, so a test
gets the same `parts_chest.2da` on every machine, and one that reads
only what is there runs with no game at all, in a build pipeline say.
`game = "folder"` in the file (or in one test) names another folder,
from the test file's folder.

The game folder may also bring the game's talk tables and the fixture's
haks (since Moonglow 2.13.0): a `dialog.tlk` is read in place of the
game's (`ctx.game:string`, and the checks that count its strings), a
talk table of the name the fixture's `Mod_CustomTlk` gives is its custom
talk table (over the one the game would find in your `tlk/` folder; a
nasher project's own, in `src/`, is found too), and a hak the fixture
lists, in the folder's `hak/`, is read in place of the one in your hak
folder. With no game, a hak the fixture lists and the folder has not is
said in a note of a test of Moonglow's checks (a plugin's own reads say
what they lacked, below).

```text
tests/
  checks.toml      module = "module"
  game/            parts_chest.2da, skills.2da: what the checks read of the game
    dialog.tlk     the game's strings, mine.tlk the fixture's
    hak/           mine_2da.hak: the fixture's haks
  module/          the fixture
```

A test may also name a game, `root = "~/games/nwn-ee"` (the game's
folder, as `mg --root` takes it: absolute, from the test file's folder,
or from your home folder with `~/`), in the file for all its tests or in
one, to be read in place of the one `mg` finds; a root that is no game
fails the test, and says how to name none. **`root = false`** reads no
game at all (since Moonglow 2.3.0): the test's game data is its own game
folder and the fixture alone, on every machine, whatever game `mg`
finds, as with `mg --root` of an empty folder. `root = true` is the game
`mg` finds, as when `root` is left out: a test of a file that says
`root = false` reads it so.

A test that fails having read from the game something its game data has
not says which, and where to put it: `the game's data has no
parts_chest.2da, which it read (there is no game install, nor the test's
game folder, game): put it in game, or name a game with root = "…"`. A
test that passes all the same (its plugin read the game in a `pcall` and
made do without) is told so in a note, since Moonglow 2.3.0: it stays
passed, and the note is under its line (`note`), in the JSON (`notes`,
and `noted`, how many tests have one) and in the summary (`5 passed (1
with notes)`):

```text
ok	checks.toml › armor: the good module has none
note	checks.toml › armor: the good module has none: it passed without parts_chest.2da, which it read and the game's data has not (the test reads no game, root = false, nor the test's game folder, game): to test it with it, put it in game, or name a game with root = "…"
```

A plugin that lists the game's resources (`ctx.game:resources()`) with
no game is told that the list is only the fixture's and its game
folder's (or, with no game folder either, the fixture's alone).

**Moonglow's own checks** (Verify's: `tag-unique`, `var-runtime`, the
others `mg verify --json` names; since Moonglow 2.3.0) run on a
fixture with `builtin = "id"` in place of `check`, so that a project's
rules are tested beside its plugin's. A test runs several with a list,
`builtin = ["var-runtime", "tag-unique"]`, or every one with `builtin =
"all"` (since Moonglow 2.13.0): their findings are held together, in the
order Verify lists them (errors first). A fixture that is a project
(`moonglow.toml` and `variables.toml` beside its `nasher.cfg`, or in a
module folder) is checked by its project file, as Verify checks the
project; what of the project file isn't understood is a note of each
such test. The findings, their fixes (Verify's own, such as
`tag-unique`'s Retag) and `left` are held as a plugin's check's are; a
builtin check has no `select`, `open`, `answers`, `yes`, `files`,
`settings`, `log` or `error`. The `tag-conventions` example's
`tests/builtin.toml`, on a project whose `moonglow.toml` wants the
gate's tags unique (`[tags] unique = ["GATE_*"]`):

```toml
module = "project"
root = false

[[test]]
name = "Moonglow's tag-unique finds the gate's post twice"
builtin = "tag-unique"
[[test.finding]]
resource = "gate.git"
at = "waypoint 0 in gate, waypoint 1 in gate"
severity = "warning"
fix = "Retag the other as GATE_POST_2"

[[test]]
name = "its fix retags the second post"
builtin = "tag-unique"
fix = "Retag the other as GATE_POST_2"
left = 0
[[test.field]]
resource = "gate.git"
field = "/WaypointList[1]/Tag"
value = "GATE_POST_2"
```

`builtin` is a key of its own, not a `check` that falls back on
Moonglow's: a test says which it runs whatever checks the plugin
declares, now or later.

The checks that count talk-table strings (`2da-strref`, `custom-tlk`,
`2da-described`) count them by the test's talk tables: the game's, or
those of its game folder. With none (no game, and no `dialog.tlk` in the
game folder), they say nothing of strings they can't count, nor of a
custom talk table the test's data has not (the game may well find it in
your `tlk/` folder), and the test is told so: in a note, of a test that
runs them as part of more (`"all"`), and as a problem, of one that names
`2da-strref`, or `custom-tlk` with its table not there, since that test
can't test what it is for. Since Moonglow 2.13.0; before, they found
every custom string missing.

A test (`[[test]]`) has a `name` and runs one thing:

| Key | |
| --- | --- |
| `check`, `command` or `event` | The id of what is run, as the manifest declares it. |
| `builtin` | One of Moonglow's own checks, by its id (`tag-unique`), in place of the plugin's (since Moonglow 2.3.0); several, in a list, or `"all"` (since Moonglow 2.13.0). |
| `select`, `open` | What a command is run on and what is open (`ctx.selection`, `ctx.open`), named as `mg plugin run --select` names them: `guard.utc`, `tavern.are`, `tavern.git/Placeable List[0]`. For objects placed or pasted, the objects (`ctx.event.objects`). |
| `answers` | A form's values by field id (`answers = { sell = 200 }`); the other fields take their defaults. |
| `yes` | The answer to `ctx.ui:confirm` (else no). |
| `files` | The files and folders the plugin gets where it asks for one, in order, from the test file's folder. |
| `changed` | An event's resources: those a module saved changed, those a save writes, or those made (`resource-created`, which needs them). None: all of the fixture's (of a save with `was`: those that differ). |
| `was` | For a resource saving or saved: the module as it was before, a fixture from the test file's folder. Its resources are the old ones (`ctx.event.old`), and the save writes those that differ. Left out: each resource as it is. |
| `by` | What made the objects or resources (`ctx.event.by`). |
| `settings` | The plugin's settings for the run (`settings = { code = "ab" }`), over what the fixture's project files hold and the defaults. |
| `game`, `root` | The test's own folder of game resources and the game it reads, as the file's `game` and `root` (above) say them, for this test alone (since Moonglow 2.2.0; `root = false` and `true` since 2.3.0). |

and says what is to come of it (each key may be left out):

| Key | |
| --- | --- |
| `[[test.finding]]` | The check's findings, all of them, in order: each with `resource`, and as wanted `message` (the whole of it), `at`, `severity` and `fix` (its fix's label; `""` for none). |
| `findings` | How many findings, where their list is not given. |
| `fix` | Make the fixes of this label (as Verify's buttons would), then hold the module to the keys below. |
| `left` | How many findings the check makes once the fixes are made: on the module as fixed, which the game's data has too (a fixed 2DA of the module's is read so through `ctx.game`, and by Moonglow's checks; since Moonglow 2.13.0). |
| `[[test.field]]` | A field after the edits: `resource`, `field` (its path, as `ctx.edit` takes it) and `value` (text, a number, a localized string's English text), or `absent = true`. `field` may be a list's item (`Equip_ItemList[2]`, `/VarTable[0]`; since Moonglow 2.2.0): it is there, or with `absent = true` it is not; an item has no `value`. |
| `edits`, `resources` | How many edits there were, and which resources they change. |
| `log` | Lines the log must have: each is part of one. |
| `error` | The job fails, saying this (part of what it says). |
| `set` | The settings the command or event sets (`ctx.settings:set`), each with the value it ends with: `set = { code = "cd" }`. |
| `warned`, `refused` | What a `resource-saving` handler says: its warnings and its refusals, all of them in order, each part of one as `name.ext: text` (`"cellar.are: its tag was OLD_CELLAR"`); `[]`: none. |

Without a game, and with no game folder of its own, a test's
`ctx.game` is the fixture alone, as Moonglow's is the game with the
module over it (since Moonglow 2.13.0; before, it was empty): a
pipeline with no game runs the tests of a plugin that reads only the
module (through `ctx.module` or `ctx.game`), and those whose tests bring
what they read of the game. Haks and folders the plugin writes are not written, nor
held to anything yet. `mg plugin pack` leaves the `tests` folder out of the
archive: the tests and their fixtures are the author's.

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

### With a project

A team's plugin can come with the team's project instead: its folder in
the project's `plugins/` (beside `nasher.cfg`, or in the folder
`moonglow.toml` names: `[plugins] folder = "tools/plugins"`), committed
with the rest. Every builder who checks the project out has it, in step
with the module, with nothing to install. Moonglow asks each builder
before it runs (the first time, and again whenever the plugin's files
change), and keeps the answer in Moonglow's own folder, never in the
project ([Plugins](15-plugins.md#a-projects-own-plugins)): a plugin can't
come pre-allowed with the checkout. Its `tests/` stay with
it, and a change to them asks nobody again.

In a build pipeline nobody is asked:
`mg verify PROJECT --allow-project-plugins` runs the checks of every
plugin of the project, and `mg plugin test plugins/NAME` its tests. Keep
the plugin's `id` the same, and give a version you want builders to see
when they are asked.
