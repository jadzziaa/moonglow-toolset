# Plugins: a proposal

Status: accepted on 2026-10-03 (see [Decisions](#decisions)). Phase 0
(the groundwork) is done, with what each item left noted under [What
Moonglow needs first](#what-moonglow-needs-first), and Phase 1 (commands
and checks, API 0.1) is built and in Moonglow 1.0.0, marked
experimental: see [Where it stands](#where-it-stands) for what it has
and what is left. This expands the one line on "a plugin or scripting
API" in [deferred.md](deferred.md) and [PLAN.md](PLAN.md). Sizes are the
project's: S (an hour or two), M (a day or so), L (several days).

## Summary

- **A plugin is a folder**: a manifest that says what it adds, and
  script files that do it. No compiling, nothing to install beside it.
- **Plugins add things Moonglow already has kinds of**: menu commands,
  checks for Verify Module, import and export formats, wizards. Later:
  panels and area tools.
- **A plugin never edits the module itself.** It reads a snapshot and
  hands back a list of edits; Moonglow checks them and applies them as
  one undo step, as it does for its own editors. Untouched data stays
  byte for byte.
- **Plugins are sandboxed.** They can read the open module and the game
  data, propose edits, ask the user questions, and read or write a file
  the user picks in a dialog. They cannot reach other files, the network
  or other programs.
- **The API is defined once, as a protocol** (named requests carrying
  plain data), and runs the same in the application and in `mg` (so a
  plugin's checks run in CI). A sandboxed script runtime speaks it first;
  external programs in any language can speak it later.
- **Most of the work is in Moonglow, not in the plugin runtime**: its
  menus, keys, checks and formats are closed lists today. Opening them is
  groundwork that pays off without plugins too.

The runtime is Luau (the Lua dialect built for running untrusted
scripts), embedded.

## What it is for

Aurora has no plugin mechanism, so its extensions patch or inject into
the executable (the axs modification, NWNTX) or work on the files from
outside (Moneo and LetoScript for batch edits, AuroraExt, nasher). The
NWN2 toolset had plugins (.NET assemblies with the whole object model),
and the community built terrain importers, copy tools, wizards, a spell
checker and persistent-world utilities with them; every toolset patch
could break them. Builders have asked Beamdog for menu entries of their
own ([wishlist](https://forums.beamdog.com/discussion/70087/nn-ee-toolset-wishlist)),
and persistent-world teams keep their batch tools and lint suites
outside the toolset ([research](research/community_pain_points.md)).

What plugins would be written for, most likely first:

1. **Batch edits** over a module: tags, variables, scripts, properties,
   by a rule ("every merchant's markup", "rename by pattern").
2. **A team's own checks**: persistent-world teams keep lint suites
   outside the toolset today. A check written once runs in Verify Module,
   in `mg verify` and in CI.
3. **Converters**: conversations, spreadsheets to and from 2DAs and
   blueprints, a server's loot or spawn tables.
4. **Generators and wizards**: creatures, items and stores by a server's
   rules; script templates.
5. **A server's conventions**, shown as a form (its local variables with
   names, types and choices) where builders now type them from memory.
6. **Views and area tools**: a quest graph, statistics; scatter, align
   and placement helpers.

Items 1 to 4 need only commands, checks, formats and forms. That is the
first API. Item 5 can be data alone. Item 6 needs more of Moonglow and
comes last.

What already works, and stays: `mg` answers every command in JSON
([manual](manual/11-command-line.md)), so a script in any language can
query and change a saved module today. Plugins add what that cannot do:
commands inside the application, on the open module with its unsaved
changes, with undo.

## Principles

1. **Edits are data, and Moonglow applies them.** The edit model
   (`mg-edit`: set a field, insert or remove a list item, set a resource)
   is the one way in. This keeps lossless editing, undo and validation in
   Moonglow's hands.
2. **Declared before run.** The manifest lists a plugin's commands,
   checks and formats. Moonglow shows them, and what the plugin may do,
   without running any of its code; code loads when first used.
3. **Safe to try.** Installing a stranger's plugin should risk nothing
   outside the open module, and nothing in it that Undo does not take
   back.
4. **The same in the window and on the command line.** The plugin host
   sits below the user interface, used by both.
5. **Moonglow draws the interface.** Plugins describe forms and, later,
   panels; they never draw. The look stays one program's, and no plugin
   code runs per frame.
6. **A small API that can be kept.** Each addition is a promise. Start
   with few, mark the rest experimental, and freeze only after real
   plugins exist.

## What a plugin is

```
tag-conventions/
  plugin.cfg        the manifest
  main.luau         the code
  README.md
```

The manifest, in `nasher.cfg`'s format (sections and `key = value`),
which builders already write and Moonglow already reads:

```ini
[plugin]
id = "example.tag-conventions"
name = "Tag conventions"
version = "1.0.0"
api = "0.1"
license = "GPL-3.0-or-later"
description = "Checks tags against the server's rules, and fixes them."
entry = "main.luau"

[command]
id = "fix-tags"
title = "Fix Creature Tags"
menu = "Plugins"
when = "module"

[check]
id = "tag-case"
title = "Creature tags are upper case"
severity = "warning"

[export]
id = "yarn"
title = "Yarn Spinner"
resource = "dlg"
extension = "yarn"
```

(API 0.1 has `[plugin]`, `[command]` with `id`, `title`, `hint` and
`key`, and `[check]`; `menu`, `when` and `[export]` belong to later
versions, and a key the running API doesn't know is an error.)

Where plugins live:

- **The user's**: `plugins/` in Moonglow's data folder, beside
  `prefabs/` and `variable-sets/`. Installed by copying a folder there,
  or with Tools › Plugins › Install from File… (a zip).
- **A project's**: `plugins/` beside a nasher project's `nasher.cfg`, so
  a team shares its checks and commands through git with the module.
  These never run until the user allows that project, in a prompt that
  lists them.

## What plugins can add

| Kind | Appears as | Runs in `mg` | API |
| --- | --- | --- | --- |
| Command | Plugins menu, context menus (module tree, area objects, tiles), a key in Options › Keyboard | `mg plugin run` | 0.1 |
| Check | Build › Verify Module, with Moonglow's own | `mg verify` | 0.1 |
| Exporter, importer | File › Export and Import, the conversation editor's formats | `mg plugin run` | 0.2 |
| Wizard, generator | Wizards menu, with a form Moonglow draws | with answers given as arguments | 0.2 |
| Quick fix | a button beside a check's finding | `mg verify --fix` | 0.2 |
| Event | after a module opens, after it saves | yes | 0.2 |
| Data only (no code): variable forms, script templates | the Variables window, New Script | n/a | 0.2 |
| Panel | a window described as a tree of tables, text and fields | no | later |
| Area tool | a brush: a ground point in, a preview and edits out | no | later |

A command says where it shows and when it is enabled (`when`: a module
is open, a resource of a type is selected, objects are selected in an
area). Moonglow works that out itself.

## How a plugin runs

```
 Moonglow (window)      mg (command line)
        \                  /
         plugin host (mg-plugin): manifests, permissions, jobs,
         |                        the protocol, edit checking
         |
   +-----+--------------------+
   |                          |
 sandboxed scripts        external programs (later)
 (embedded runtime,       (JSON-RPC over standard input and
  one thread per job)      output, any language, full trust)
```

A command, check or conversion runs as a **job**:

1. Moonglow takes a snapshot of the module (cheap: resources are shared,
   not copied) and starts the plugin on its own thread. The window shows
   the job's name, its progress and Cancel; the module cannot be edited
   meanwhile, so there is nothing to reconcile afterward.
2. The plugin reads from the snapshot and the game data. Its edits go
   into a private copy, so it reads back what it wrote, and a bad edit
   (no such resource, a path that leads nowhere, a value of the wrong
   type) fails at the line that made it.
3. When it needs the user (a form, a question, a file), it asks
   Moonglow, which shows the dialog and answers.
4. At the end Moonglow applies the collected edits as one command named
   by the plugin, and says what changed ("Fix Creature Tags: 14
   resources changed"). Undo takes it all back. Nothing is saved until
   the user saves.

In `mg`, the same job runs without a window: answers come from
arguments, `--dry-run` prints the edits as JSON instead of saving.

## The API

Requests are named and carry plain data. The first set:

| Group | Requests |
| --- | --- |
| `module` | information; resources by type; a resource as fields, text or bytes; find objects (as `mg find`); where a resource or tag is used |
| `game` | a 2DA; a talk-table string; a game or hak resource, and which layer it comes from |
| `edit` | set or remove a field; insert or remove a list item; set, add or delete a resource; rename everywhere |
| `ui` | message; confirm; a form (fields with types, choices and defaults) returning its values; pick a resource; pick a file to read or to write |
| `context` | what the command was invoked on: the resource, the selected objects or tiles, the area |
| `log` | information, warning, error (tagged with the plugin; a resource named in it can be clicked); progress |
| `storage` | the plugin's own settings |

Fields keep their types. Setting a field keeps the type it has, and a
new field takes the type the game's files give it (as Moonglow's editors
do); the raw form (`{type, value}`, the JSON of nasher and
neverwinter.nim) is there when a plugin must choose.

What it looks like (Luau):

```lua
local mg = require("@moonglow")

mg.command("fix-tags", function(ctx)
    local changed = 0
    for _, key in ctx.module:resources("utc") do
        local tag = ctx.module:gff(key):get("Tag")
        if tag ~= string.upper(tag) then
            ctx.edit:set(key, "Tag", string.upper(tag))
            changed += 1
        end
    end
    ctx.log:info(`{changed} creature tags changed`)
    return { label = "Upper-case creature tags" }
end)

mg.check("tag-case", function(ctx)
    local findings = {}
    for _, key in ctx.module:resources("utc") do
        local tag = ctx.module:gff(key):get("Tag")
        if tag ~= string.upper(tag) then
            table.insert(findings, {
                resource = key,
                at = "Tag",
                message = `tag "{tag}" is not upper case`,
            })
        end
    end
    return findings
end)
```

One table in the code describes every request (name, parameters,
result, the API version it came in). The reference manual, the editor
definition file for completion, the JSON schema and the conformance
tests are generated from it, so they cannot drift apart.

## Safety

What has gone wrong elsewhere decides this section. VS Code's and
Obsidian's plugins run with the user's full rights; signed marketplaces
did not stop releases made with stolen credentials (Nx Console, May
2026), automatic updates delivered them, and a shared Obsidian vault
that carried enabled plugins installed a trojan (April 2026).

- **Sandbox.** A script plugin has no file, network or process
  functions at all: the runtime is built without them. Its reach is the
  API above.
- **Files by the user's hand.** A plugin reads or writes a file only
  through a dialog the user answers, and gets that file's contents, not
  a path to roam from.
- **Limits.** A memory cap and a time budget per job, with Cancel. A
  plugin's error is caught and logged with its name; it does not take
  the application down.
- **Off until enabled.** Each plugin is enabled by the user, who sees
  what it adds. A project's plugins need the project allowed first.
  Content (a module, a hak) never carries plugins.
- **No automatic updates.** An update is installed by the user, and
  says what it changes in what the plugin adds.
- **A way out.** `--no-plugins` starts without them; a plugin that
  fails at startup is disabled and named; crash reports list the enabled
  plugins.
- **Edits are untrusted input.** They are checked like a file from
  disk: bounds, types, sizes, never a panic. The checker gets property
  tests.

External programs (the later tier) cannot be sandboxed on every system.
They are marked as running with the user's rights, enabled one by one,
and never come with a project without that being said.

**License.** Script plugins run inside a GPL program through its API;
by the FSF's reading they should carry a GPL-compatible license when
distributed, as Blender requires of its add-ons. External programs
speaking the protocol are separate programs. Proposed: say this in the
author's guide and require a license field in the manifest. This is a
policy to settle, not legal advice.

## Runtimes compared

| | Sandbox | To write one | Added to Moonglow | Verdict |
| --- | --- | --- | --- | --- |
| **Luau, embedded** (mlua) | built for untrusted scripts; memory limit, interrupts | a text file; typed, with a language server | small; C++ sources built with the compiler the script compiler already needs | **first** |
| JavaScript, embedded (QuickJS) | no ambient access; memory limit, interrupts | a text file; the largest pool of authors; completion from a definition file | about 1 MB, C sources | the alternative to Luau; one or the other |
| External program, JSON-RPC | none portable | any language and its own debugger; needs it installed | almost nothing (`mg lsp` has the framing) | **second**, for teams' own tools |
| WebAssembly (wasmtime, wasmi) | strongest | needs a compile toolchain | 2 to 19 MB and a long build | later, if compiled plugins are asked for |
| Embedded Python | none | the best libraries | a Python per platform, tens of MB | no; Python is served by the external tier |
| Native libraries | none; a crash takes Moonglow down | Rust or C, per platform | needs `unsafe`, no stable ABI | no (NWN2's breakage; Bevy removed its own) |
| NWScript | would need a virtual machine written | the language builders know; poor at lists and tables | a VM to maintain | no |

Why script first and external programs second, when the second needs no
new dependency: what users ask for is a thing to download that adds a
feature. That must be safe from strangers and work on a builder's
Windows machine with nothing else installed, in every package. External
programs serve the teams that already script in Python or Nim, who are
also served by `mg` today. Both speak one protocol, so the second is a
small step after the first.

Why Luau over JavaScript: the sandbox is the reason for the tier, and
Luau's is the better proven. JavaScript's syntax is nearer NWScript's
and more builders may know it. Either is workable; both is twice the
upkeep.

## What Moonglow needs first

The groundwork, from a survey of the code. Each item is useful without
plugins.

1. **A command registry (M–L). Done for the window's commands.**
   `mg_ui::commands` is the table: each command's id, name, tip, when it
   can be chosen and what it does. The menu bar (`MENUS`), the toolbar
   (`TOOLBAR`), the keys and Options › Keyboard are drawn from it, so
   every menu command can be given a key (a deferred item, closed), and
   keys are kept by command id, whoever the command belongs to. Left:
   the context menus (the module tree's, a tab's, an object's, the
   palette's) are still written where they show, since their commands
   act on the thing clicked; a plugin's commands join them, and the
   Plugins menu, with the host in Phase 1. The Command Palette (Help,
   Ctrl+Shift+P) is the table's fourth reader.
2. **Edits on the wire (M). Done.** A command is written as JSON and
   read back (`mg_edit::wire`): six kinds of edit, values as nwn-lib
   JSON, field paths that parse as they print. `mg apply EDITS.json` and
   `mg set` use it, each with `--dry-run`
   ([manual](manual/11-command-line.md)); what is read is treated as
   untrusted and property-tested. Left: `--dry-run` on the other
   commands that change a module.
3. **One way to apply a change (M). Done.** Every command goes through
   `Moonglow::apply` (the nine places that applied their own now do, so
   an import, a rename or a palette copy warns of what it shadows as an
   editor's change does), and what is made from the module at save or
   build goes through `Workspace::derive`, which moves the revision
   without an undo step. `Workspace::edits_to` turns "run it on a copy,
   keep the difference" into a command. A test reads the sources and
   fails on a new way round. Left: the module field itself is still
   public (about 140 reads); making it private would close the gate in
   the compiler.
4. **Jobs (M–L). Done.** A job (`mg_ui::jobs`) reads a snapshot of the
   module and the game data (now shared, `Arc<GameData>`, and changed
   only between jobs) on a thread of its own, reports progress, can be
   called off, and hands what it made to the interface's thread, where
   edits go in through the one gate. One job at a time, the window
   modal meanwhile. Compile All, Verify and the Build window's Build run
   so. Tests run jobs to completion where they start (the same thread
   and hand-over). Left: Find References is a view that refreshes as the
   module changes, not a command, so it stays where it is until the
   where-used index ([deferred](deferred.md)); the build before saving
   and the compile before a test still wait; the compiler reports no
   progress between scripts, so Compile All shows time, not a bar.
5. **Open lists for checks and formats (S–M each). Done, as far as it
   goes before there is a plugin host.** The doctor's checks are a
   catalog (`doctor::Check`: an id and what each holds to be true; `mg
   checks` lists them) and a finding's check id is its own string, so
   findings from elsewhere sit beside the doctor's. A conversation format
   says in one place what it reads (`Format::reads`, `read`, `update`),
   where the window and `mg` each matched on formats before. Left for
   Phase 1, when the host gives them their shape: the registries
   themselves (a plugin's checks run with Verify; its formats offered in
   Export and Import).
6. **Forms from a description (M).** A form given as data (fields,
   types, choices), drawn with the editors' own widgets. The wizards can
   use it too.
7. **Seeing what plugins do (M).** Log entries tagged with their source;
   a Plugins window (each plugin, what it adds, enabled or not, its last
   error, time and memory used, Reload); a console to try the API
   against the open module; the safe-mode switch; plugins named in crash
   reports.
8. **Events (S–M), for API 0.2.** There are none today, only a revision
   counter that views poll. Module opened, saved and closed, and a
   command applied, are the first.

## Documentation

For users, in the manual (and so in Help › User Manual):

- **Plugins**: installing, enabling, what a plugin can and cannot do,
  a project's plugins, safe mode, what to do when one misbehaves.

For authors, in `docs/plugins/` and built into the application:

- **Your first plugin**: a command and a check in ten minutes, tried in
  the console, with the editor completing the API.
- **Concepts**: jobs and the snapshot, edits and undo, findings, forms,
  resource keys, field paths and field types.
- **Reference**: every request, generated; each with its types, an
  example and the API version it came in.
- **Cookbook**: six maintained examples (a batch edit, a check with a
  quick fix, an exporter, an importer, a wizard with a form, a
  variable-form package). They run as tests in CI, so they stay true.
- **Testing and sharing**: `mg plugin check` (the manifest), `mg plugin
  test` (a plugin's own tests against a fixture module), how to pack and
  list one.
- **The protocol**, for external programs, when that tier exists.
- **Changes**: what each API version added, deprecated or removed.

## Testing and stability

- Every request has a test in the host and an example in the reference.
- The request table's schema is kept as a snapshot; a change to it
  fails CI until the API version and the change list say so.
- The example plugins and the manual's snippets run in CI on the three
  systems.
- Until 1.0 the API is **0.x and may change with any release**, each
  change listed. A manifest names the API it was written for; Moonglow
  refuses one it cannot serve, with the reason.
- New areas start in an experimental namespace a manifest must opt
  into, and listed plugins may not use it (VS Code's rule).
- 1.0 is declared only after outside authors have shipped plugins on
  0.x. From then: additions only within 1.x; the previous major stays
  supported for a stated time.

## Phases

| Phase | Contents | Size | Done when |
| --- | --- | --- | --- |
| 0. Groundwork | items 1 to 5 above; `mg apply`, `mg set`; background Verify and Compile | L | no behavior lost; those deferred items closed |
| 1. Commands and checks (API 0.1) | manifest, loader, Plugins window, sandboxed runtime, `module`, `game`, `edit`, `ui`, `log`; commands and checks; `mg plugin`; console; three examples; the user chapter, first-plugin guide and reference | L | two authors outside the project have built something they use |
| 2. Formats, wizards, teams (API 0.2) | exporters, importers, wizards, quick fixes, events, storage, data-only packages, project plugins with their prompt; the cookbook | L | a team runs its checks in CI from its project |
| 3. External programs | the protocol over standard input and output; the full-trust prompts; the protocol document | M | a Python and a Nim example pass the conformance tests |
| 4. API 1.0 | the freeze; a reviewed index of plugins (a repository, entries by pull request, pinned by content hash) | M | the stability rules above hold for a release |
| 5. Panels and area tools | described panels; tools with previews and overlays as data | L | experimental until used |

Phases 2 and 3 can swap if teams ask for Python first.

## Where it stands

**Phase 1 is built**, and released in Moonglow 1.0.0 as experimental
(the window, the manual and the README say so; the API stays at 0.1 and
may change):

- `crates/mg-plugin`: the manifest, discovery, and the runtime: a Luau
  machine per job, sandboxed, with a memory limit, stopped by Cancel;
  `ctx.module`, `ctx.game`, `ctx.edit`, `ctx.log`, `ctx.progress`,
  `ctx.ui`; commands hand back edits, checks findings.
- In the window: the Plugins menu with enabled plugins' commands (also
  in the Command Palette and Options › Keyboard), Manage Plugins (off
  until enabled, what each adds, the console), a plugin's questions in
  the job's window, its checks in Verify Module, `--no-plugins`, and the
  enabled plugins named in a crash report.
- Install Plugin from File: a plugin's archive (a zip) checked as
  untrusted input and unpacked into the plugins folder, off; one
  installed before is replaced after a question, a folder put there by
  hand never. Remove… in Manage Plugins deletes one that was installed
  that way, after a question. `mg plugin pack` makes the archive, `mg
  plugin install` and `remove` do the same from the command line, and
  `mg plugin check` takes an archive as well as a folder. (The `zip`
  crate reads it, with the deflate code that was built in already.)
- `mg plugin list`, `check` and `run` (`--dry-run`, `--answer`,
  `--yes`), and `mg verify --plugins`.
- Documentation: the manual's Plugins, Writing plugins and Plugin API
  reference; `docs/plugins/` with four examples, the API's types for
  editors and the change list. Tests run the examples, hold the
  reference and the type file to name everything the runtime offers,
  and the guide to carry the code of its example.

**Left of Phase 1:**

- Its "done when": nobody outside the project has written a plugin yet.
  The questions under [Decisions](#decisions) are still open, and the
  API should not grow before some of them are answered.
- The reference is written by hand and checked for completeness, not
  generated; a plugin's own tests (`mg plugin test`) are not built.
- The type file was not tried in an editor with the Luau language
  server.
- The running application was driven with plugins on Linux (an
  off-screen display); on Windows and macOS the tests pass in CI, but
  nobody has used plugins in the window there.

Where the build differs from the text above: the window's commands are
under a **Plugins** menu (Manage Plugins…, Install Plugin from File…),
not Tools › Plugins; a
project's plugins and their prompt are Phase 2; the examples are four,
with the cookbook's others to come with the things they would add.

## Decisions

Made on 2026-10-03:

1. **The language is Luau.**
2. **Script plugins first, external programs second.**
3. **The script runtime may be added** (mlua with Luau's sources), when
   Phase 1 starts.
4. **License**: script plugins carry a GPL-compatible license when
   distributed, and the manifest has a license field; external programs
   speaking the protocol are separate programs. The author's guide says
   so.
5. **Sharing** starts with install-from-file and a list in the
   repository; an index in the application comes with API 1.0.
6. **Phase 0 is done regardless**: it closes two deferred items (a key
   for any menu command, `mg set`) and removes the window freezes on big
   modules, plugins or not.

Still to learn from the people asking, before the API is drawn in
detail:

- Which three plugins would you write, or want, first?
- What would they read, and what would they change?
- Would they need files, the network or another program?
- Which language would you write them in?
- Would you share them with strangers, or keep them in your team?

## Not proposed

- Native library plugins, and plugins that draw their own interface
  (egui, or a web view).
- Code that runs every frame, or that hooks saving itself: lossless
  saving stays Moonglow's.
- Automatic updates, and a marketplace.
- Embedding Python.
- Renderer or shader plugins.

## Sources

Checked in October 2026 for this proposal; sizes and version numbers
should be measured again before they are relied on.

- VS Code: [extension host](https://code.visualstudio.com/api/advanced-topics/extension-host),
  [proposed API](https://code.visualstudio.com/api/advanced-topics/using-proposed-api),
  [runtime security](https://code.visualstudio.com/docs/configure/extensions/extension-runtime-security)
- Zed: [capabilities](https://zed.dev/docs/extensions/capabilities),
  [how extensions work](https://zed.dev/blog/zed-decoded-extensions)
- Blender: [manifest](https://developer.blender.org/docs/features/extensions/schema/1.0.0/),
  [licenses](https://docs.blender.org/manual/en/latest/advanced/extensions/licenses.html)
- Tiled: [scripting](https://doc.mapeditor.org/en/stable/manual/scripting/)
- Aseprite: [extensions](https://www.aseprite.org/docs/extensions/)
- Defold: [editor scripts](https://defold.com/manuals/editor-scripts/),
  [their interface](https://defold.com/manuals/editor-scripts-ui/)
- Figma: [how plugins run](https://developers.figma.com/docs/plugins/how-plugins-run/)
- Nushell: [plugin protocol](https://www.nushell.sh/contributor-book/plugin_protocol_reference.html)
- xi-editor: [retrospective](https://raphlinus.github.io/xi/2020/06/27/xi-retrospective.html)
  (what asynchronous plugins cost)
- Obsidian: [plugin security](https://obsidian.md/help/plugin-security)
- [mlua](https://github.com/mlua-rs/mlua),
  [rquickjs](https://docs.rs/rquickjs/latest/rquickjs/struct.Runtime.html),
  [Bevy removing dynamic plugins](https://github.com/bevyengine/bevy/pull/14534)
- The FSF's [GPL FAQ](https://www.gnu.org/licenses/gpl-faq.html) on
  plugins
- Neverwinter Nights: [NWNTX](https://github.com/virusman/nwntx),
  [the axs modification](https://neverwintervault.org/project/nwn1/other/tool/nwn-toolset-modification-axs),
  [Moneo](https://neverwintervault.org/project/nwn1/script/moneo-v40224),
  Beamdog's [toolset wishlist](https://forums.beamdog.com/discussion/70087/nn-ee-toolset-wishlist)
- Incidents: [Nx Console](https://labs.cloudsecurityalliance.org/research/csa-research-note-vscode-extension-supply-chain-breach-20260/),
  [Obsidian vaults](https://thehackernews.com/2026/04/obsidian-plugin-abuse-delivers.html)
