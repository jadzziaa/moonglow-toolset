---
type: Manual Page
title: Plugins
description: Plugins for the people who use them - what a plugin can and cannot do, installing, enabling and removing, a project's own plugins (asked about when the project is opened), a plugin's settings, commands, checks and the fixes beside their findings, the console, the command line and when something goes wrong.
tags: [manual, plugins]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-22T10:41:09Z }
---

# Plugins

A plugin adds to Moonglow what a team or a builder needs that Moonglow
doesn't have: **commands** (a batch edit, a report, a generator) and
**checks** that run with Verify Module (a server's naming rules, say). A
plugin is a folder with a few text files; anyone can write one
([Writing plugins](16-writing-plugins.md)).

**Plugins are experimental.** They are new in Moonglow 1.0, and the
plugin API (version 0.4) may change in a later release as the people who
write plugins try it. A plugin written for an earlier version of the API still
runs (0.2, 0.3 and 0.4 only added to it); one written for a version this
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
  module, not the hak;
- write files into a folder **you chose** for it, **if you allow it**:
  it asks the first time it writes into each folder. Files of the same
  names there are replaced, and no copy of them is kept. It writes into
  that folder only, never into a folder inside it or above it;
- act by itself when a module is opened or saved, if it says so (the
  Plugins window lists what each plugin does by itself). What it does
  then is a change like a command's, which Undo takes back; where it
  asks for a folder or for leave to write, it asks once, not at every
  save: what you allowed is kept between runs (for that version of the
  plugin), the Plugins window says how much, and **Forget** there makes
  it ask again. Disabling, removing or installing the plugin anew
  forgets it too.

It cannot open files of its own choosing, write any other file, use the
network, start other programs, or reach anything of Moonglow's beyond
the above: the language it is written in is given no means to. It has a limit on memory (256 MB), and
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

- The archive is checked before anything is written: it must hold one
  plugin whose manifest reads and that was written for this Moonglow's
  plugin API. One that isn't a plugin, or whose files would land outside
  the plugin's own folder, is refused, with the reason.
- If the plugin is installed already, Moonglow asks before replacing it
  (a newer version, say). Whether it is enabled stays as it was.
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
([The project file](03-modules.md#the-project-file)). Every builder who
checks the project out has them, in step with the module, with nothing
to install.

**None of them runs until you allow it.** When you open the project,
Moonglow lists each of its plugins you have not answered for, with what
it adds: its commands (and the right-click menus they are in), its
checks, what it does by itself, and the settings it keeps (those the
team shares are written into `moonglow.toml`). For each, **Allow** or
**Don't Allow**; **Decide Later** leaves them all off now and asks
again at the next opening.

- What you allow is the plugin **as it is now**: Moonglow reads its
  files when the project is opened, runs it from what it read, and
  keeps your answer with a digest of them. When the plugin changes (a
  pull brings another version, or other code under the same one), it is
  off again, and you are asked again, told what it was. Its `tests`
  folder is not part of it.
- **Don't Allow** keeps it off, however it changes, and it is not asked
  about again; tick it in Manage Plugins to allow it later.
- Your answers are **yours alone**, and never in the project: Moonglow
  keeps them in its own folder (`project-plugins.toml` beside the plugins
  folder), by the project's place on your disk, the plugin and the
  digest of its files. A project's files can't answer for you: a
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

- **yours**: in Moonglow's settings, for every module;
- **the team's**: in the project's `moonglow.toml`, which goes with the
  project to every builder (Save writes it there, the rest of the file as
  it was);
- **yours, for this project**: in the project's `.moonglow/user.toml`,
  never shared.

**Save** keeps what changed; **Defaults** puts each back to the plugin's
own value (a value back at its default is taken out of where it was
kept). Without a project open, the team's and the project's settings
can't be changed. A plugin's command may set one itself (your builder
code, asked for once); the log says what it set, and where. What a
project's files hold that a plugin doesn't know, or of the wrong kind, is
said in the log when the project is opened, and left as it is.

## Commands

An enabled plugin's commands are in the **Plugins** menu and in the
Command Palette (Ctrl+Shift+P), with the plugin's name. A plugin may
suggest a key for a command; **Tools › Options › Keyboard** lists plugin
commands with the rest, and any of them can be given a key or have it
changed there.

A command made for a kind of object (placeables, creatures…) or for
areas is also in their right-click menus: the area view's menu on the
objects selected (or, with nothing selected, on the area), the module
tree's rows, and the palettes' blueprints. It comes after the menu's own
commands, above Variables and Properties, and acts on what you clicked:
every object selected in the area view, or every blueprint chosen in the
palette. Manage Plugins says which menus each command is in. Chosen from
the Plugins menu instead, it acts on the objects selected in the area
view, or else on the blueprints selected in the palette.

A command needs a module open. It runs as other long work does
([Long work](09-build-and-test.md)): a window names it, shows how far it
is, and has **Cancel**. If it asks something, the question is in that
window. When it is done, the log says what it changed:

```text
Tag conventions: Fix Creature Tags changed 12 resources (Edit › Undo takes it back)
```

In the window, what Moonglow keeps in step when you edit stays in step
after a plugin's edits too: an item's cost, a creature's hit points and
challenge rating.

## Checks

An enabled plugin's checks run with **Build › Verify Module**, along with
Moonglow's own. Their findings are in the log with the rest, as warnings
or errors, each naming the resource and the field. A check that fails is
reported as an error too, and the other checks still run.

A finding may come with a **fix**: a button beside it in the log, which
says what it does ("Upper-case this tag"). Nothing changes until you
click it; then its edits go in as one step, which **Undo** takes back,
and the button says **Fixed**. A fix is worked out when Verify runs: if
what it would change has changed since (by hand, or by another fix that
did more than set fields), it is refused, and the log says to verify
again. Moonglow's own checks have fixes where the right change is plain
(a tag that is to be unique: the others retagged; an area's variables
in its ARE: moved to its GIT).

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
mg plugin run mymodule.mod plugins/my-plugin on-save --event --changed guard.utc
mg plugin run mymodule.mod plugins/set-local set-local --select "tavern.git/Placeable List[3]" --answer name=role --answer value=workbench
mg verify mymodule.mod --plugins plugins/tag-conventions
mg verify mymodule.mod --plugins plugins/tag-conventions --fix example.tag-conventions/tag-case
mg verify myproject --allow-project-plugins
mg plugin list --project myproject
mg plugin allow myproject example.builder-tags
mg plugin run myproject myproject/plugins/builder-tags prefix-tags --select guard.utc --setting code=ab
mg plugin test plugins/tag-conventions
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
([Writing plugins](16-writing-plugins.md#a-plugins-own-tests)).

## When something goes wrong

- **A plugin's command fails.** The log has the error, with the script's
  line. Nothing was changed.
- **A plugin is not listed.** Its folder needs `plugin.cfg` directly in
  it (not a folder deeper); press **Reload**.
- **An archive is not installed.** The log and Manage Plugins say why.
  "It holds several plugins" means the archive is a collection: unpack
  it and install each plugin's folder by copying.
- **Moonglow misbehaves with a plugin enabled.** Start it with
  `--no-plugins`: no plugin is loaded, and Manage Plugins says so.
  A crash report names the enabled plugins; when you report a problem,
  say whether it happens without them.
- **"It was written for plugin API …".** The plugin needs another
  version of Moonglow; see its author's notes.
