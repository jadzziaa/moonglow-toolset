---
type: Manual Page
title: Plugins
description: Plugins for the people who use them - what a plugin can and cannot do, installing, enabling and removing, a project's own plugins (asked about when the project is opened), a plugin's settings, commands, exporters and importers, wizards, panels, checks and the fixes beside their findings, what plugins do by themselves (before a save, on a paste, on what a wizard makes), the console, the command line and when something goes wrong.
tags: [manual, plugins]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-09T00:38:51Z }
---

# Plugins

A plugin adds to Moonglow what a team or a builder needs that Moonglow
doesn't have: **commands** (a batch edit, a report, a generator) and
**checks** that run with Verify Module (a server's naming rules, say). A
plugin is a folder with a few text files; anyone can write one
([Writing plugins](16-writing-plugins.md)).

**Plugins are experimental.** They are new in Moonglow 1.0, and the
plugin API (version 0.9) may change in a later release as the people who
write plugins try it. A plugin written for an earlier version of the API still
runs (0.2 to 0.9 only added to it); one written for a version this
Moonglow doesn't have is not run: Manage Plugins says so, and Moonglow,
or the plugin, needs an update. What you build with plugins, and what
you miss, is welcome on the
[issue tracker](https://github.com/jadzziaa/moonglow-toolset/issues).

## What a plugin can and cannot do

A plugin's code runs in a sandbox. It can:

- read the open module, and the game's data (2DA tables, the talk table,
  the game's own resources);
- propose changes to the module: fields set, list items added and
  removed, resources written and deleted;
- write to the log, show its progress, and ask you a question (a message,
  yes or no, or a short form);
- read a file or a folder **you choose** when it asks for one (the files
  in that folder, and nothing outside it);
- make areas and paint their terrain, as the area editor does;
- put resources into a hak in your hak folder, **if you allow it**: it
  asks the first time it writes to each hak. Resources are added to a
  hak that is there, and a copy of the hak as it was is kept beside it
  (`name.hak.bak`). Edit › Undo takes back what the plugin did to the
  module and the haks it wrote with it (each as it was, or gone if the
  plugin made it; one changed since is left as it is), and Redo writes
  them again; a command that wrote only haks has no step to undo;
- write files into a folder **you chose** for it, **if you allow it**:
  it asks the first time it writes into each folder. Files of the same
  names there are replaced, and no copy of them is kept. It writes into
  that folder only, never into a folder inside it or above it;
- act by itself, if it says so (the Plugins window lists what each
  plugin does by itself): when a module is opened or saved; before a
  save, to warn of what it would write or stop it; and when you place
  or paste objects, or a wizard makes something
  ([below](#what-plugins-do-by-themselves)). What it changes then is a
  change like a command's, which Undo takes back; where it
  asks for a folder or for leave to write, it asks once, not at every
  save: what you allowed is kept between runs (for that version of the
  plugin), the Plugins window says how much, and **Forget** there makes
  it ask again. Disabling, removing or installing the plugin anew
  forgets it too.

It cannot open files of its own choosing, write any other file, use the
network, start other programs, or reach anything of Moonglow's beyond
the above: the language it is written in is given no means to. It has a limit on memory (256 MB of its own, and 512 MB of what Moonglow holds for it), and
**Cancel** stops it.

A plugin never changes the module itself. What a command hands back is a
list of edits, and Moonglow applies them as it applies your own: **as one
step that Edit › Undo takes back**, all of them or (if one doesn't apply)
none. A check only reads; one that tries to edit fails.

That makes a plugin safe for your computer, not for your module: an
enabled plugin's command can change or delete anything in the module it
is run on. Undo takes it back, and nothing is on disk until you save, but
install plugins from people you'd trust with the module.

## Installing

A plugin comes as an archive (a `.zip` file) or as a folder.

**From an archive:** **Plugins › Install Plugin from File…** (also
**Install from File…** in Manage Plugins) takes the archive and puts the
plugin into Moonglow's plugins folder. Nothing of the plugin runs, and it
is off until you enable it.

- The archive is checked before anything is written: each plugin it
  holds must have a manifest that reads and was written for this
  Moonglow's plugin API. One that isn't a plugin, or whose files would
  land outside the plugin's own folder, is refused, with the reason.
- An archive may hold several plugins, each in a folder of its own at
  its top (or in the one folder at its top, as a repository's download
  has them): each is installed into a folder of its own, and each is off
  until you enable it.
- If a plugin is installed already, Moonglow asks before replacing it
  (a newer version, say), one question for each such plugin of the
  archive. Whether it is enabled stays as it was.
- A plugin's folder that was copied in by hand is never replaced by an
  install: remove that folder first.

**From a folder:** copy the plugin's folder (the one with `plugin.cfg`
in it) into the plugins folder and press **Reload**.

| Linux | Windows | macOS |
| --- | --- | --- |
| `~/.local/share/moonglow/plugins` | `%APPDATA%\Moonglow\plugins` | `~/Library/Application Support/Moonglow/plugins` |

**Plugins › Manage Plugins…** shows where the folder is on your system;
**Open Folder** opens it (and makes it, the first time), and **Reload**
reads it again after you copy a plugin in or change one.

## Enabling

Every plugin is **off until you enable it**: installing one runs none of
its code. Manage Plugins lists each plugin with its version, what it says
it does, who wrote it, its license, and the commands and checks it adds.
Tick its box to enable it; Moonglow remembers.

A plugin that can't be read is listed with what is wrong (its manifest
has a fault, its script is missing, or it was written for a version of
the plugin API this Moonglow doesn't have).

## Removing

**Remove…** beside a plugin in Manage Plugins deletes the plugin's
folder, after asking. Your modules are not touched: what the plugin's
commands changed in them stays.

Remove is there for plugins installed from an archive. A plugin whose
folder you copied in yourself has none (Moonglow deletes only what it
installed): delete its folder, and press **Reload**.

## A project's own plugins

A team can keep its plugins with its project: in a folder `plugins/`
beside `nasher.cfg` (or a module folder's own), or the folder the
project's `moonglow.toml` names
([The project file](03-modules.md#the-project-file-moonglowtoml)). Every builder who
checks the project out has them, in step with the module, with nothing
to install.

**None of them runs until you allow it.** When you open the project,
Moonglow lists each of its plugins you have not answered for, with what
it adds: its commands (and the right-click menus they are in), its
checks, what it does by itself, and the settings it keeps (those the
team shares are written into `moonglow.toml`). For each, **Allow** or
**Don't Allow**; **Allow All** allows each of them as it is;
**Decide Later** leaves them all off now and asks again at the next
opening.

- What you allow is the plugin **as it is now**: Moonglow reads its
  files when the project is opened, runs it from what it read, and
  keeps your answer with a digest of them. When the plugin changes (a
  pull brings another version, or other code under the same one), it is
  off again, and you are asked again, told what it was. Its `tests`
  folder is not part of it.
- While the project is open, Moonglow looks at its plugins' folder every
  few seconds: a plugin changed there (a pull, an edit) is read again,
  off until you allow it as it now is, and one new there is asked about,
  without opening the project again.
- **Don't Allow** keeps it off, however it changes, and it is not asked
  about again; tick it in Manage Plugins to allow it later.
- Your answers are **yours alone**, and never in the project: Moonglow
  keeps them in its own folder (`project-plugins.toml` beside the plugins
  folder), by the project's place on your disk, the plugin and the
  digest of its files. A yes holds for those very files wherever they
  are: the project moved or copied elsewhere on disk (another clone, a
  worktree) runs the plugins you allowed, as they were, without asking
  again. A no holds for that project alone. A project's files can't answer for you: a
  checkout that comes with answers of its own (a `[project_plugins]`
  section in a committed `.moonglow/user.toml`, say) allows nothing, and
  the log says so. `mg` reads the same answers, and `mg plugin allow`
  gives them from a terminal.
- Manage Plugins lists them under **This project's plugins**, each
  marked as the project's, with what you answered. Its box allows it (as
  it is now) or turns it off. They have no Remove: they are the
  project's files.
- A project's plugin of the same id as one you installed runs in its
  stead while the project is open, once allowed.
- An allowed project's plugin runs as any plugin does: sandboxed, and
  asking before it reads or writes a file of yours.

A module kept in one file (`.mod`) has no project, and no plugins of its
own.

## Settings

A plugin may keep settings: a team's prefix, your builder code, how
strict a check is. **Settings…** beside it in Manage Plugins opens them
in a form, each with what it is (a tip says more) and where it is kept:

- **yours**: in Moonglow's own folder, for every module (`mg` reads and
  keeps them too);
- **the team's**: in the project's `moonglow.toml`, which goes with the
  project to every builder (Save writes it there, the rest of the file as
  it was);
- **yours, for this project**: in the project's `.moonglow/user.toml`,
  never shared.

**Save** keeps what changed; **Defaults** puts each back to the plugin's
own value (a value back at its default is taken out of where it was
kept). Without a project open, the team's and the project's settings
can't be changed. A plugin's command may set one itself (your builder
code, asked for once); the log says what it set, and where, and a value
a command set whose edits did not go in is not kept. What a project's
files hold that a plugin doesn't know, or of the wrong kind, is said in
the log when the project is opened, and in the form, whose **Tidy Up**
takes it out (or moves it to the file its scope keeps it in).

A setting is a field of its kind: text (held, where the plugin says, to
a pattern, said as you type), a whole number or a number with a
fraction, a box, a choice, a list (a box for each of its choices, or a
text a line), a color (a swatch and its `#rrggbb`), or a path, whose
**Choose…** picks a file or, where the plugin says, a folder.

## Commands

An enabled plugin's commands are in the **Plugins** menu and in the
Command Palette (Ctrl+Shift+P), with the plugin's name. A plugin may
suggest a key for a command; **Tools › Options › Keyboard** lists plugin
commands with the rest, and any of them can be given a key or have it
changed there. A command run by its key while you are typing in a field
runs on what you typed: the field sets it first, as when you click
elsewhere.

A command made for a kind of object (placeables, creatures…), for
areas, for tiles or for a kind of resource (scripts) is also in their
right-click menus: the area view's menu on the objects selected (an
area's under **The Area**; with nothing selected, on the area), its
tile menu, the module tree's rows and groups, the palettes' blueprints
and categories, a Properties window's tab, the script editor's menu, and
the rows of Find References and Advanced Search. It comes after the
menu's own commands, above Variables and Properties, and acts on what
you clicked: every object selected in the area view, every blueprint
chosen in the palette, every one of a group or a category. A command may
be for placed objects alone or blueprints alone. Manage Plugins says
which menus each command is in. Chosen from the Plugins menu instead, it
acts on what is chosen in the pane you last clicked in (the module tree,
a palette, an area's view), or else on the objects selected in the area
view, or else on the blueprints selected in the palette.

A command needs a module open. It runs as other long work does
([Long work](09-build-and-test.md)): a window names it, shows how far it
is, and has **Cancel**. If it asks something, the question is in that
window. A plugin's window is headed by the command's title alone (its
plugin's name shows when you point at it), and leaves the window behind
it as it is, not dimmed, so that a color being picked in it can be
judged on the area. When it is done, the log says what it changed:

```text
Tag conventions: Fix Creature Tags changed 12 resources (Edit › Undo takes it back)
```

In the window, what Moonglow keeps in step when you edit stays in step
after a plugin's edits too: an item's cost, a creature's hit points and
challenge rating.

## Exporters and importers

A plugin may add ways to write the module, or what is selected, to a
file or a folder, and to read one into the module (since plugin API
0.6). With such a plugin enabled, **File › Export** opens a menu of
Moonglow's own **Export…** (to an ERF) and the plugins' exporters, and
**File › Import** likewise. Choosing one asks where first: the file to
write (in the save dialog, with the exporter's kind of file), the file
to read (the open dialog shows the kinds it reads), or a folder. The
plugin's code runs after you have chosen, and reaches only that.

An exporter changes nothing in the module: there is nothing to undo, and
the files it wrote stay (the log says where). One made for what is
selected (a creature sheet of the creatures chosen in the palette, say)
is grayed until what is selected is of its kind. What an importer
reads goes into the module as one step: **Edit › Undo** takes it all
back.

## Wizards

A plugin's wizards are in the **Wizards** menu, under Moonglow's own
(since plugin API 0.6). A wizard asks page by page in its window, with
**Back**, **Next** (on the last page, **Finish**) and **Cancel**; what
you give on one page may change the pages after it, and a page that
finds something wrong with what you gave says so and stays. Back keeps
what you typed. Nothing is made until Finish, and what Finish makes is
one step of Undo; Cancel makes nothing.

## Panels

A plugin's panels are panes of their own: **Plugins › Panels** shows and
hides each (a tick beside those shown), and the Command Palette finds
them by name (since plugin API 0.6). A panel opens in a window, which
you may dock beside the area or the palettes as any window, and it is
reopened with the module's other windows. It shows what its plugin
says, in Moonglow's own headings, text, lists, buttons and fields: a
list's item opens what it names, and a button runs one of the plugin's
commands, with its own step of Undo. A field (since plugin API 0.9) is
a form's (text, a number, a tick, a choice, a color) that stays: what
you set in it stays until the plugin shows another value there (the
tint of another placeable you select, say), and a field may run one of
the plugin's commands when you set it (a color when its picker closes,
text or a number when you leave it, a tick or a choice at once), with
its own step of Undo. A color field may show the color on the objects
selected while its picker is open, as a form's does; a choice may show
a swatch beside each of its options. **Refresh** draws it again; it is
drawn again by itself after the plugin's commands, and, if its plugin
says so, after each change of the module (once you stop, while you drag
or type) or when what is selected changes. Its plugin's code runs beside the window, so a panel never
slows it down: while it draws, the panel shows what it showed before. A
panel goes when its plugin is disabled.

## Checks

An enabled plugin's checks run with **Build › Verify Module**, along with
Moonglow's own. Their findings are in the log with the rest, as warnings
or errors, each naming the resource and the field. A check that fails is
reported as an error too, and the other checks still run.

A finding may come with a **fix**: a button beside it in the log, which
says what it does ("Upper-case this tag"). Nothing changes until you
click it; then its edits go in as one step, which **Undo** takes back,
and the button says **Fixed** (Undo offers it again). **Fix All** beside
the first of a check's fixes makes every one of them, as one step. A fix
whose label ends in "…" asks first: it runs the plugin's command on the
finding, whose form asks what it needs. A fix is worked out when Verify runs: if
what it would change has changed since (by hand, or by another fix that
did more than set fields), it is refused, and the log says to verify
again. Moonglow's own checks have fixes where the right change is plain
(a tag that is to be unique: the others retagged; an area's variables
in its ARE: moved to its GIT).

## What plugins do by themselves

A plugin may act on what you do, as its manifest says (Manage Plugins
lists it: "By itself, before a save, reads each resource it writes
(are): may warn of one, or stop the save").

**Before a save.** A plugin may read what the save would write, each
resource as it was and as it will be, and hold it to a team's rules (an
area's tag that players' places are kept by, say). If it has something
to say, nothing is written yet: a window lists it. A **warning** is
yours to weigh: **Save Anyway** writes the module as it is, **Cancel**
writes nothing. A **refusal** stops the save: the window says why, and
**Close** leaves the module unsaved. Beside either, a plugin may offer a
fix (Put back TAVERN); clicking it makes the change, as a step Edit ›
Undo takes back, and saves again, the plugins reading the module as it
now is. Save As is asked about as Save is. Before a save a plugin never
changes the module itself: you save what you see. A plugin that fails,
or takes longer than it may (5 seconds, or as Manage Plugins' **Time
within a save or a paste** sets it), is a warning in that window ("could not check the save"),
never a refusal, so a broken plugin never keeps you from saving
(disable it in Manage Plugins). After a save, a plugin may read what
was written (to copy it somewhere you chose, say).

**On a paste, and on what a wizard makes.** When you place or paste
objects in an area, or a wizard (area, creature, item, placeable,
door…, the Script Wizard), Add to Palette, Setup Store, Copy, New
Script, a new NUI window or an import makes something, a plugin
may change what was made at once: give a pasted waypoint a tag of its
own, put the team's variable on a new area. Moonglow's own work comes
first (your builder code in the ResRef, the project's store script),
the plugin's after, and both are **one step of Undo**: one Edit › Undo
takes back the paste and what the plugin did to it. The log says what
each plugin changed. Moonglow's warning of a tag a paste doubled comes
after the plugins, so a tag a plugin made the copy's own is not
reported. A plugin that fails there is said in the log, and what you
made is as Moonglow made it.

## The console

Manage Plugins has a **Console**: a place to try the plugin API on the
open module, a line or a few at a time. `mg` and `ctx` are at hand, as in
a command; what the code returns is shown, and its edits go in as one
step, as a command's do.

```lua
return ctx.module:resources("utc")
```

```lua
for _, name in ctx.module:resources("utm") do
    ctx.edit:set(name, "MarkUp", 150)
end
```

The console needs no plugin to be installed or enabled. It is the
quickest way to a one-off batch edit, and to finding out what a
resource's fields are called (`return ctx.module:gff("guard.utc")`).

## From the command line

`mg` runs plugins without the window, for build pipelines
([Command-line tools](11-command-line.md)):

```text
mg plugin list ~/.local/share/moonglow/plugins
mg plugin run mymodule.mod plugins/tag-conventions fix-tags --dry-run
mg plugin run mymodule.mod plugins/house-rules area-tags --event --was last-release.mod
mg set mymodule.mod tavern.are Tag=INN --plugins plugins/house-rules
mg plugin run mymodule.mod plugins/set-local set-local --select "tavern.git/Placeable List[3]" --answer name=role --answer value=workbench
mg verify mymodule.mod --plugins plugins/tag-conventions
mg verify mymodule.mod --plugins plugins/tag-conventions --fix example.tag-conventions/tag-case
mg verify myproject --allow-project-plugins
mg plugin list --project myproject
mg plugin allow myproject example.builder-tags
mg plugin run myproject myproject/plugins/builder-tags prefix-tags --select guard.utc --setting code=ab
mg plugin test plugins/tag-conventions
mg plugin export mymodule.mod plugins/creature-sheet creatures-csv creatures.csv
mg plugin import mymodule.mod plugins/creature-sheet creatures-csv creatures.csv
mg plugin wizard mymodule.mod plugins/store-wizard new-store --answers store.json --answer kind=Smith
mg plugin install tag-conventions-1.0.0.zip plugins
mg plugin remove example.tag-conventions plugins
```

`mg` has no list of enabled plugins: it runs the plugin you name, and
a project's own plugins as you allowed them in Moonglow (`mg verify`
says each that did not run, and why); `--allow-project-plugins` runs
every one of them, for a build pipeline, where nobody is asked. `mg
plugin list --project` lists them with what you answered and their
settings. A plugin's settings with `mg` are those the project's files
hold, else the defaults, and `--setting ID=VALUE` for one run: `mg`
keeps none of yours (those of scope "user"). It applies a command's
edits as given (an item's cost is not recomputed).
`mg verify` makes a finding's fix only when `--fix` names its check, and
`mg plugin test` runs a plugin's own tests
([Writing plugins](16-writing-plugins.md#a-plugins-own-tests)). `mg
set`, `mg apply` and `mg bake-walkmesh` ask the plugins `--plugins`
names, and the project's as you allowed them, before they save: a
refusal is an error and nothing is written (by `mg bake-walkmesh`, no
hak and no rows file either), a warning is said and the save goes on.
Every other command that changes a module's resources asks the
project's plugins the same way before it saves: `mg rename`, `mg
replace`, `mg areas`, `mg update-instances`, `mg import`, `mg
compile`, `mg dialog-import`, `mg attach` (which copies nothing into
the user folder where they refuse), `mg nui new`, `mg nui generate`,
`mg nui import`, `mg verify --fix` and `mg plugin run`. `mg`
has no wizards and no paste, so a plugin's handler of those runs only
by hand (`mg plugin run --event`). A plugin's exporters and importers
run with `mg plugin export` and `mg plugin import`, and its wizards with
`mg plugin wizard`, each page answered from `--answers` (a JSON file of
the fields' values by id) and `--answer`, the rest by their defaults
(since plugin API 0.6). A panel is the window's alone.

## When something goes wrong

- **A plugin's command fails.** The log has the error, with the script's
  line. Nothing was changed.
- **A plugin is not listed.** Its folder needs `plugin.cfg` directly in
  it (not a folder deeper); press **Reload**.
- **An archive is not installed.** The log and Manage Plugins say why;
  of an archive of several plugins, which one is at fault, and none of
  them is installed.
- **Moonglow misbehaves with a plugin enabled.** Start it with
  `--no-plugins`: no plugin is loaded, and Manage Plugins says so.
  A crash report names the enabled plugins; when you report a problem,
  say whether it happens without them.
- **"It was written for plugin API …".** The plugin needs another
  version of Moonglow; see its author's notes.
