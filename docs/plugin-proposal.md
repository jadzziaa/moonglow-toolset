---
type: Proposal
title: 'Plugins: a proposal'
description: The decision record of Moonglow's plugins, proposed and accepted 2026-10-03 - what plugins are for, the principles, safety, the runtimes compared and why sandboxed Luau, the decisions and what was not proposed - with what shipped noted beside it. The API as it is now is in the manual and the API change list.
tags: [plugins, proposal, luau, api, decision]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-02T11:04:23Z }
---

# Plugins: a proposal

Status: accepted on 2026-10-03 (see [Decisions](#decisions)) and built.
This is the record of what was proposed and decided, and why. It was
written for API 0.1; where what shipped differs, a note says so. It is
not the description of the API and not the list of what is planned:

- **What was built:** plugins came in Moonglow 1.0.0 (API 0.1: commands
  and checks in sandboxed Luau) and are marked experimental. API 0.2
  (1.16.0) added files and folders the user chooses, haks, areas and
  terrain, and pictures; API 0.3 (1.21.0) added 2DAs and talk tables,
  files written into a chosen folder, and events; API 0.4 (2.1.0) what is
  selected and open, commands in right-click menus, fixes for findings, a
  plugin's own tests, settings, a project's own plugins, a resource's own
  events around a save, and events on objects placed or pasted and on
  what a wizard makes; API 0.5 (3.3.0) more menus, settings and events;
  and API 0.6 (not yet in a release) exporters, importers, wizards and
  panels ([below](#api-06-exporters-importers-wizards-and-panels-2026-11-02)).
  For users:
  [Plugins](manual/15-plugins.md). For authors:
  [Writing plugins](manual/16-writing-plugins.md), the
  [Plugin API reference](manual/17-plugin-api.md), the
  [API change list](plugins/CHANGES.md) and the
  [examples](plugins/README.md). The host is `crates/mg-plugin`.
- **What is planned or left:** the [deferred list](deferred.md), under
  "Automation" ("Plugins, beyond API 0.4").

## Summary

- **A plugin is a folder**: a manifest that says what it adds, and
  script files that do it. No compiling, nothing to install beside it.
- **Plugins add things Moonglow already has kinds of**: menu commands and
  checks for Verify Module first; import and export formats, wizards,
  panels and area tools were proposed for later. (Shipped: commands,
  checks and events; exporters, importers, wizards and panels with API
  0.6. Area tools are not built.)
- **A plugin never edits the module itself.** It reads a snapshot and
  hands back a list of edits; Moonglow checks them and applies them as
  one undo step, as it does for its own editors. Untouched data stays
  byte for byte.
- **Plugins are sandboxed.** They can read the open module and the game
  data, propose edits, ask the user questions, and read or write a file
  the user picks in a dialog. They cannot reach the network or other
  programs, or files of their own choosing. (As proposed for 0.1, a
  plugin reached no file but one picked in a dialog. Since 0.2 and 0.3
  it can also read every file of a folder the user chooses, write files
  into such a folder by name, and write haks into the user's hak folder,
  each only after the user allows it.)
- **The API is defined once, as a protocol** (named requests carrying
  plain data), and runs the same in the application and in `mg` (so a
  plugin's checks run in CI). A sandboxed script runtime speaks it first;
  external programs in any language could speak it later. (Shipped: the
  script runtime. External programs are not started.)
- **Most of the work was in Moonglow, not in the plugin runtime**: its
  menus, keys, checks and formats were closed lists. Opening them was
  groundwork that paid off without plugins too (a table of the window's
  commands, edits as JSON with `mg set` and `mg apply`, one way to apply
  a change, background jobs, the doctor's checks as a catalog).

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
2. **A team's own checks**: a check written once runs in Verify Module,
   in `mg verify` and in CI.
3. **Converters**: conversations, spreadsheets to and from 2DAs and
   blueprints, a server's loot or spawn tables.
4. **Generators and wizards**: creatures, items and stores by a server's
   rules; script templates.
5. **A server's conventions**, shown as a form (its local variables with
   names, types and choices) where builders now type them from memory.
6. **Views and area tools**: a quest graph, statistics; scatter, align
   and placement helpers.

What already worked, and stays: `mg` answers every command in JSON
([manual](manual/11-command-line.md)), so a script in any language can
query and change a saved module. Plugins add what that cannot do:
commands inside the application, on the open module with its unsaved
changes, with undo.

## Principles

1. **Edits are data, and Moonglow applies them.** The edit model
   (`mg-edit`: set a field, insert or remove a list item, set a resource)
   is the one way in. This keeps lossless editing, undo and validation in
   Moonglow's hands.
2. **Declared before run.** The manifest lists what a plugin adds.
   Moonglow shows it, and what the plugin may do, without running any of
   its code; code loads when first used.
3. **Safe to try.** Installing a stranger's plugin should risk nothing
   outside the open module, and nothing in it that Undo does not take
   back. (Since 0.2 a plugin can also write a hak, and since 0.3 files
   into a chosen folder, when the user allows it; Undo takes a hak back
   with the command's edits, since Moonglow 3.3.0, but not a folder's
   files.)
4. **The same in the window and on the command line.** The plugin host
   sits below the user interface, used by both.
5. **Moonglow draws the interface.** Plugins describe forms and, later,
   panels; they never draw. The look stays one program's, and no plugin
   code runs per frame.
6. **A small API that can be kept.** Each addition is a promise. Start
   with few, mark the rest experimental, and freeze only after real
   plugins exist.

## Safety

What has gone wrong elsewhere decided this section. VS Code's and
Obsidian's plugins run with the user's full rights; signed marketplaces
did not stop releases made with stolen credentials (Nx Console, May
2026), automatic updates delivered them, and a shared Obsidian vault
that carried enabled plugins installed a trojan (April 2026).

- **Sandbox.** A script plugin has no file, network or process
  functions at all: the runtime is built without them. Its reach is the
  API.
- **Files by the user's hand.** As proposed: a plugin reads or writes a
  file only through a dialog the user answers, and gets that file's
  contents, not a path to roam from. As shipped (0.2, 0.3): it reads a
  file or a folder the user chooses (the files in that folder and
  nothing outside it), and writes only into a hak of the user's hak
  folder or into a folder the user chose, by name, after the user allows
  each. It still never gets a path to roam from.
- **Limits.** Proposed: a memory cap and a time budget per job, with
  Cancel. Shipped: a memory cap (256 MB) and Cancel; and (API 0.4) a
  time budget, 5 seconds, for the events that run within what the user
  does (a save waits for them), none for a command, which the user
  starts and can cancel. A plugin's error is caught and logged with its name; it does
  not take the application down.
- **Off until enabled.** Each plugin is enabled by the user, who sees
  what it adds. Content (a module, a hak) never carries plugins. (Since
  API 0.4 a project's checkout may: its `plugins/`, each run only after
  the builder, asked when the project is opened, allows it, the answer
  holding for the plugin's content by a digest of its files, which it
  runs from. A changed one is asked about again, and the answers are the
  builder's own, kept in Moonglow's own folder and never read from the
  project's files, so that a checkout can't come with its plugins
  allowed. A build pipeline runs them with `mg verify
  --allow-project-plugins`.)
- **No automatic updates.** An update is installed by the user.
- **A way out.** `--no-plugins` starts without them; crash reports list
  the enabled plugins.
- **Edits are untrusted input.** They are checked like a file from
  disk: bounds, types, sizes, never a panic.
- **A check's edits on the user's word.** A check never edits; the fix
  it offers a finding (0.4) is edits recorded, made only when the user
  clicks its button (or names its check to `mg verify --fix`), as one
  step of Undo, and only on what Verify read.
- **An event's edits on the user's word** (0.4). A paste, a placing or
  a wizard's Finish is the user's own action: a plugin's handler may
  edit what it made, and its edits go into that action's command, said
  in the log and taken back with it by one Undo. A save is not an edit:
  it writes what the user sees, so around a save a plugin only reads.
  It may warn (the user is asked before anything is written), refuse
  (the save stops, saying why) and offer a fix the user makes; a plugin
  that fails or runs out of time is a warning, never a refusal, so a
  broken plugin can't keep a module from being saved. What a plugin
  makes itself fires no event.

External programs (the later tier, not started) cannot be sandboxed on
every system. They would be marked as running with the user's rights,
enabled one by one, and never come with a project without that being
said.

**License.** Script plugins run inside a GPL program through its API;
by the FSF's reading they should carry a GPL-compatible license when
distributed, as Blender requires of its add-ons. External programs
speaking the protocol are separate programs. The manifest has a required
license field. This is a policy, not legal advice.

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
also served by `mg`. Both would speak one protocol, so the second is a
small step after the first.

Why Luau over JavaScript: the sandbox is the reason for the tier, and
Luau's is the better proven. JavaScript's syntax is nearer NWScript's
and more builders may know it. Either is workable; both is twice the
upkeep.

## External programs: the design (2026-11-01)

The second tier, designed and not yet built (the deferred list's
"Plugins, beyond API 0.4"). A plugin whose code is a program of its own
(Python, Nim, a shell script) speaks to Moonglow over its standard input
and output, so that a team's own tools in any language plug in, with
their own libraries and debugger.

- **The manifest.** `[plugin] runtime = "external"` and `program =
  "tool.py"` (a file of the plugin's folder) with `interpreter`
  (`"python3"`, found on the PATH) where the file is no executable;
  everything else as a script plugin's: `[command]`, `[check]`,
  `[event]`, `[setting]`, the same ids and keys. Refused under an `api`
  before the version that brings it.
- **The protocol** is the API, not a second one: JSON-RPC 2.0 framed as
  the language server's is (`Content-Length` headers; `mg lsp` has the
  framing), one process for one job, started when the job begins and
  ended when it does. Moonglow sends `run` (`{ kind: "command" | "check" |
  "event", id, selection, open, settings, event }`); the program calls
  back the API by its names as the reference writes them, `{ method:
  "ctx.module:gff", params: ["guard.utc"] }`, `"ctx.edit:set"`,
  `"ctx.ui:form"`, and answers `run` with what a handler returns (a
  label, findings with fixes recorded through `"ctx.fix"` handles). So
  one reference serves both tiers, and the test that holds it to the
  code holds the protocol too.
- **The sandbox it can't have.** A program runs with the user's rights:
  no system sandboxes a process the same way everywhere. So an external
  plugin is marked as such everywhere it shows (Manage Plugins, the
  project's question: "runs as a program of yours, with your rights"),
  is enabled only by its own question, never by Allow All, and a
  project's is never allowed by a yes to the same files elsewhere (its
  digest says it is the same file, not what the interpreter it names
  does). Its edits still come back as data and go in through
  `Moonglow::apply`, as one step of Undo.
- **Limits** as a script's where they can be kept (the time an event
  has; a job's edits all or none; nothing written outside the module but
  through `ctx.hak` and `folder:write`, by the same questions), and the
  process killed when the job is canceled or out of time.
- **Tests.** `mg plugin test` runs an external plugin's tests as a
  script plugin's; CI runs one example written in Python against each
  platform's `python3`.

Built in that order: the manifest and the process with `run` and
`ctx.log`, then the reading half of the API (`ctx.module`, `ctx.game`),
then edits, questions and checks' fixes, then events. M.

## API 0.6: exporters, importers, wizards and panels (2026-11-02)

The four kinds of thing the summary proposed for later, designed here
before they were built (the deferred list's "More kinds of things to
add", the owner's 8th large entry). Each is declared in the manifest as
a command is, registered by the code as a command is, and gated as every
addition is: a manifest that declares one and says an `api` before 0.6
is refused, with the reason. What all four keep from the rest of the
API: the code runs sandboxed in a job of its own on a copy of the
module; it reads with `ctx.module` and `ctx.game`, sees `ctx.selection`,
`ctx.open` and `ctx.settings`; edits come back as data and go in through
`Moonglow::apply`, one step of Undo; nothing outside the module is
written without the user's choice or leave.

### Exporters

- **Manifest.** `[exporter]` with `id`, `title`, `hint`, `to = "file"`
  (the default) or `"folder"`, `extension` (the file's, without the dot:
  the dialog's filter and the suggested name's), and `for` (kinds of
  object, as a command's): with `for` it exports what is selected and is
  offered only when what is selected is all of those kinds; without, it
  exports the module (and still sees `ctx.selection`).
- **Luau.** `mg.exporter(id, function(ctx, out) … end)`. `out.name` is
  the file's or the folder's name; to a file, `out:write(bytes)` is the
  whole file (a second call replaces what the first gave); to a folder,
  `out:write(name, bytes)` writes a file into it by name (a name alone,
  as `folder:write` takes). It may return text, which the log says.
- **Sandbox.** The user chooses where, in the save dialog (File ›
  Export › the exporter) or the folder dialog, before any of the code
  runs, and that choice is the leave: `out:write` asks nothing more. The
  files are written when the handler has returned, all or none, as
  `folder:write`'s are. An exporter only reads: one that edits the
  module, writes a hak or writes into another folder fails, as a check
  that edits does. It may ask with a form (options of the export).
- **`mg`.** `mg plugin export MODULE PLUGIN ID OUT` (a file, or with
  `to = "folder"` a folder, made if it isn't there), with `--select`,
  `--answer`, `--setting` and `--dry-run` as `mg plugin run` has them.
  File › Export and Import are submenus only with a plugin's exporter or
  importer enabled: without, they are Moonglow's own Export… and
  Import…, as Aurora's.
- **Tests.** A plugin's own tests: `export = "id"`, `to = "name"` (the
  file's or folder's name the plugin sees; nothing is written), and
  `written` (the files' names) and `content` (`{ "guards.csv" = [ …
  ] }`, text each file has).
- **Undo.** Nothing to undo: the module is not changed, and files written
  outside it are not taken back (as `folder:write`'s are not).

### Importers

- **Manifest.** `[importer]` with `id`, `title`, `hint`, `from = "file"`
  (the default) or `"folder"`, and `extensions` (`"csv, txt"`: the open
  dialog's filter; none, any file).
- **Luau.** `mg.importer(id, function(ctx, source) … end)`. `source` is
  what the user chose, as `ctx.ui:open_file` and `ctx.ui:open_folder`
  give them: a `file` (`name`, `bytes`) or a `folder` (`name`, `files`,
  `bytes`, `text`, `to_hak`, `write`). It edits with `ctx.edit`, and may
  put the files into a hak with the user's leave as a command does.
- **Sandbox.** The user chooses the file or folder (File › Import › the
  importer) before the code runs: it reads that and nothing else of
  their disk. A folder's `write` and `to_hak`, and `ctx.hak`, ask the
  user as a command's do.
- **`mg`.** `mg plugin import MODULE PLUGIN ID SOURCE`, saving the module
  as `mg plugin run` does (`--yes` for a hak, `--hak-dir`, `--dry-run`).
- **Tests.** `import = "id"`, its source the first of `files`, and the
  fields after (`[[test.field]]`) as a command's.
- **Undo.** Its edits are one step, named by what it returns or "Import"
  and its title; a hak it wrote goes with them, as a command's.

### Wizards

- **Manifest.** `[wizard]` with `id`, `title` and `hint`. The wizards
  are in the Wizards menu, under the blueprint wizards.
- **Luau.** `mg.wizard(id, { pages = { … }, finish = function(ctx,
  answers) … end })`. A page is `{ id, title, text?, fields, check? }`
  (`fields` as a form's), or a function `(ctx, answers) -> page?` that
  makes one from the answers so far (nil: the page is skipped). `answers`
  is one table of every page's values by field id, so a field's id is
  its own across the pages. `check(ctx, answers)` may return text: what
  is wrong, shown on the page, which stays. `finish` makes the edits.
- **The window.** One page at a time with **Back**, **Next** (on the
  last, **Finish**) and **Cancel**, "Page 2 of 3"; Back keeps what was
  typed; a page's defaults are its values when it is shown again. The
  last is the last of the list: where the pages after the one shown are
  all skipped, its Next finishes (which pages are skipped depends on
  what that page is given).
- **Sandbox.** The pages and their checks only read: one that edits
  fails ("a wizard edits in finish"). `finish` edits, and may write a
  hak with the user's leave, as a command does.
- **`mg`.** `mg plugin wizard MODULE PLUGIN ID`, each page answered from
  `--answers FILE` (a JSON object of values by field id) and `--answer
  ID=VALUE`, the rest by their defaults; a page's check that objects is
  an error, and nothing is saved. (Built so; `mg plugin run` takes
  `--answers` too, for a command's forms.) The host says whether a
  person answers (`Host::interactive`): in the window a check that
  objects asks the page again, saying why; without one it ends the
  wizard, or it would ask the same page for ever.
- **Tests.** `wizard = "id"` with `answers` by field id, and the fields
  after; `error` for a check that objects.
- **Undo.** Finish's edits are one step, named by what `finish` returns
  or the wizard's title; Cancel, or closing the window, changes nothing
  (nothing is made until Finish).

### Panels

- **Manifest.** `[panel]` with `id`, `title`, `hint` and `refresh`:
  `"edits"` (after each change of the module: an edit, Undo, Redo),
  `"selection"` (when what is selected changes), both, or none. Every
  panel is also drawn again when it opens, by its **Refresh** button, and
  after a job of its plugin's (a command, an import, a wizard) ends.
- **Luau.** `mg.panel(id, function(ctx) return { … } end)`: what it
  shows, a list of elements, each a table with its `type`: `heading`
  (`text`), `text` (`text`, `weak`, `color` `"warning"` or `"error"`),
  `value` (`label`, `text`), `separator`, `button` (`text`, `hint`,
  `command`: one of the plugin's commands, run on `on`, an entry or a
  list of entries as `ctx.selection` has them, else on what is
  selected), and `list` (`items`, each `{ text, hint?, open?, command?,
  on? }`: a click opens `open`, a resource's name or an entry, or runs
  `command`). Moonglow draws them; the plugin never draws.
- **Sandbox.** A panel's handler only reads: no edits, no questions
  (`confirm` is false, a form nil, no file is given), nothing written; a
  button runs one of the plugin's commands, which asks and edits as any
  command does. It runs on a thread of its own, never in a frame and
  not as a job (a job has the window to itself; while one runs, a panel
  waits), one at a time per panel (a change while it runs draws it again
  after), with 250 ms; past that it is stopped and the panel says so.
  What it logs goes nowhere: it shows what it has to say. The frame draws what it returned,
  data, which is held to 2,000 elements and items and 1,000 characters a
  text: the rest is cut, saying so. A render is stopped when the game's
  data is to change (a hak attached, the module layer refreshed), and
  run again after.
- **The window.** A panel is a pane like the others: opened from
  Plugins › Panels (and the Command Palette), it opens in a window of
  its own and is moved, docked or floated as any tab is (its place
  remembered as other windows' are); it is kept with the module's open
  tabs, and closes when its plugin is disabled. (Built so: proposed, it
  opened docked beside the area as the palettes do; a window of its own
  is what every other tab opens as, and docks where the user drags it.)
- **`mg`.** None: there is no window to show it in. `mg plugin check`
  holds its registration to the manifest as for the other kinds.
- **Tests.** `panel = "id"` with `shows` (texts it must show, each part
  of an element's text, a button's or an item's) and `select`/`open` as
  for a command.
- **Undo.** None of its own: it changes nothing. What its buttons run is
  a command, with that command's Undo.

### What stays out, and why

- **Exporters and importers in right-click menus, and drop or open by
  extension** (a `.csv` dropped on the window, or chosen in File ›
  Open): File › Export and Import are where the user expects them, and
  a file type claimed by several plugins needs a way to choose first.
  Asked for, they come later.
- **Importers firing `resource-created`**, and wizards too: what a
  plugin makes itself fires no event (Safety), or plugins would answer
  each other.
- **A wizard's fields beyond a form's** (pictures, lists, a 3D preview):
  pages are forms, the kinds the user already knows; a page's `text`
  says what a picture would.
- **Panels that take text, draw, or run on a clock**: a panel shows and
  links; what asks is a command's form, and plugin code on a timer is
  plugin code per frame by another name (Principle 5).
- **Panels in `mg`**: nothing to show them in; their tests run with
  `mg plugin test`.

## Proposed and not done so

Parts of the proposal that were built differently, so that this record
is not read as a description:

- **One table generating everything.** The proposal had one table in the
  code describe every request, with the reference, the editor's type
  file, a JSON schema and conformance tests generated from it. As built,
  the reference and the type file are written by hand, and tests hold
  them to name everything the runtime offers
  (`crates/mg-plugin/tests/examples.rs`).
- **The second API version.** The proposal's 0.2 was exporters,
  importers, wizards, quick fixes, events, storage and a project's
  plugins. The 0.2 and 0.3 that shipped were something else (files,
  haks, terrain, pictures, tables); of the proposed list, events came
  with 0.3, and quick fixes, storage (a plugin's settings) and a
  project's plugins with 0.4, and exporters, importers and wizards (with
  panels) with 0.6.
- **Where commands show.** Proposed: Tools › Plugins, context menus, and
  `menu` and `when` keys in the manifest. Built: a **Plugins** menu
  (with Manage Plugins… and Install Plugin from File…), the Command
  Palette and Options › Keyboard; a command's manifest keys are `id`,
  `title`, `hint` and `key`. API 0.4 added context menus: a command's
  `for` (kinds of object, or `area`) puts it in their right-click
  menus, run on what was clicked; there is no `menu` or `when` key.
- **Examples.** The proposal's cookbook had six; `docs/plugins/examples/`
  has six (four at 1.0.0).

## Decisions

Made on 2026-10-03:

1. **The language is Luau.**
2. **Script plugins first, external programs second.**
3. **The script runtime may be added** (mlua with Luau's sources).
4. **License**: script plugins carry a GPL-compatible license when
   distributed, and the manifest has a license field; external programs
   speaking the protocol are separate programs. The author's guide says
   so.
5. **Sharing** starts with install-from-file and a list in the
   repository; an index in the application comes with API 1.0.
6. **The groundwork is done regardless**: it closed two deferred items (a
   key for any menu command, `mg set`) and removed the window freezes on
   big modules, plugins or not.

The stability rule accepted with the proposal: until 1.0 the API is 0.x
and may change with any release, each change listed
([the change list](plugins/CHANGES.md)). A manifest names the API it was
written for, and Moonglow refuses one it cannot serve, with the reason.
1.0 is declared only after outside authors have shipped plugins on 0.x.

Asked of the people who want plugins, before the API grows:

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
