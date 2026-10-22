---
type: Manual Page
title: Build, verify and test
description: Build Module, publishing to NWSync, long work, Verify Module and the fixes beside its findings, Test Module, reloading haks and 2DAs, and Area Statistics.
tags: [manual, build, verify, test, nwsync]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-22T20:06:21Z }
---

# Build, verify and test

## Build Module

**Build › Build Module…** prepares the module for play and checks it, as
Aurora's Build does:

- **Compile**: the **Scripts** (with their debug files, where **Generate
  Debug Information** is on in Options), recalculated **Creature CR** (challenge
  ratings), **Encounters** (each encounter's creature list updated with
  its creatures' challenge ratings and appearances; creatures that no
  longer exist are removed) and **Palettes** (the custom palettes rebuilt
  from the module's blueprints).
- **Check that resources are available**: **Missing Resources** (scripts,
  blueprints, conversations, models… that something in the module names
  but neither the module, its haks nor the game has) and, if ticked,
  **Unused** resources (in the module but used by nothing).

Compile and Missing Resources are on by default, Unused off. The
**Results** list what was found; double-click a result to open what it is
about, and **Export…** saves the list as text. With **Build module on
save** (Options › General), every save builds first.

## Publish to NWSync

A server's players download its haks and talk table through NWSync:
before joining, the game fetches them from a web server that nwserver
names (`-nwsyncurl ADDRESS`). **Build › Publish to NWSync…** writes what
that web server serves into a folder, as neverwinter.nim's
`nwn_nwsync_write` does (the same manifest, byte for byte):
- **What goes in:** the module's haks (the first-listed hak's copy where
  several have one) and its talk table. Scripts' source and debug files
  and area comments are left out, since players don't need them. Files
  over 15 MB are refused, as `nwn_nwsync_write` refuses them.
- **With the module itself:** adds the module's own resources, under the
  haks', with a name and description players see. Use it for a
  single-player module that players download whole, not for a persistent
  world.
- **Group ID:** for servers sharing one repository.
- **Latest:** **Make it the latest** points the repository's `latest` at
  it; nwserver serves `latest` unless given `-nwsynchash`.

Publishing again writes only the changed files. It runs in the
background; the window shows progress, then the manifest's hash
(**Copy** copies it, for `-nwsynchash`). Upload the folder to the web
server. `mg nwsync` does the same from the command line.

## Long work

Compile All Scripts, Build Module (its Build button) and Verify Module
run in the background: a small window names the work, shows how far it
is, and has **Cancel**. Until it is done the rest of the window takes no
input, so nothing changes under the work; what it made (compiled scripts,
say) then goes in as one step that Undo takes back. Canceled, it changes
nothing. The build before saving (Options › General) and the compile
before a test run wait instead, since saving and testing need their
result.

## Verify Module

**Build › Verify Module** writes to the log, as errors and warnings, what
is missing and what is wrong with the module's custom content. To leave
some of it out (a hak's tileset errors you are not going to fix),
untick its group in **Options › Verify**; the summary line says what
was left out and how many problems it had. **Clear**, in the log's
corner, empties the log before a second run.

**Fixes.** A problem whose right change is plain has a button beside it
in the log that says what it does: **Retag the other as …** for a tag
the project wants unique, **Move the variables to the GIT** for an
area's variables in its ARE, and whatever a plugin's check offers.
Nothing changes until you click it; then the change goes in as one step,
which **Undo** takes back, and the button says **Fixed**. A fix is
worked out when Verify runs, on the module as it was then: if what it
would change has changed since (by hand, or by another fix that did more
than set fields), the log says so, and Verify run again offers it anew.
Only the last Verify's fixes are offered, and Clear takes them away with
the lines.

**Missing resources** are things something names that exist nowhere.
- **Errors:** scripts, conversations and areas named by areas, placed
  objects, conversations or the module, and scripts that were never
  compiled.
- **Warnings:**
  - what a blueprint names, which matters only once something places it;
  - sounds and portraits.

Errors come first in the log, then warnings. Objects placed from a
blueprint the module no longer has come last, one line a blueprint with
how many are placed from it and in which areas. They are no fault: a
placed object holds all it needs and works in the game, and many modules
(BioWare's too) have them. Only placing more of it and Update Instances
need the blueprint.

**Custom content** covers the module, the haks in your `hak` folder, and
`override` and `development`. Each problem names the hak or folder, the
file, and the row, column, section or object at fault. Most of them
crash Aurora with an access violation that names nothing:

| Problem | Where it shows |
| --- | --- |
| A tileset whose `[TILES]` or `[GROUPS]` count disagrees with its sections | Aurora's "Range Check Error" when opening an area |
| A tile whose model is missing | Aurora crashes painting it |
| A group whose first tile is `-1` or shared with another group, or names a tile that doesn't exist | the group won't paint, or Aurora crashes |
| A tileset door type that isn't a `doortypes.2da` row | the door has no appearance |
| A material (`.mtr`) naming a texture longer than 16 characters | Aurora's "Pure virtual function called" |
| A shader program that doesn't compile: one a material names (`customshaderVS`, `customshaderFS`), one of the haks' `.shd` programs, or a stock program broken by a hak's copy of a stock include (`inc_lighting`). Named at the file and line with the shader compiler's message, the programs it breaks listed; compiled as the game at its highest settings has them | the game draws the meshes that use it magenta |
| A 2DA row with more cells than columns (two rows run together) | Aurora's access violation; the game drops the cells |
| `baseitems.2da` over 256 rows, `lightcolor.2da` over 32 | Aurora fails (the game is fine) |
| A 2DA string reference past the end of its talk table | "Bad Strref" |
| A hak's 2DA row naming a row of another table that isn't there (a feat's required feat, a spell's feat, a race's appearance, a package's class; some sixty columns of eleven tables, and the feat, skill and spell lists that a class, a race or a package names) | the feat, spell or class is missing or wrong in the game (a warning) |
| A custom talk table the module names that isn't in its haks, the module or the `tlk` folder (or is named with `.tlk`, or, on Linux, in a different case from the file) | the game won't load the module |
| A hak's 2DA hiding another hak's longer copy | the rows past it are lost (an older copy?) |
| A creature, placeable, door or item naming a 2DA row that doesn't exist or whose model is missing | Aurora's area view crashes on it; in the game it has no appearance, or the game crashes (a creature with a class that doesn't exist) |
| A creature, placeable, door or encounter of the PC faction (a warning; not a trigger, which Aurora gives faction 0 when its blueprint has none) | the PC faction is the players' alone: the game puts such a creature in Hostile, and creatures regard the others as they regard players |
| An area with variables in its ARE (as Moonglow wrote them before 1.34.3) | the game reads an area's variables from its GIT, so a script reads 0 for them; opening the area's Variables moves them, as does its fix |
| A tag the [project file](03-modules.md#the-project-file-moonglowtoml) says must be unique (`[tags] unique`) on more than one area or placed object (a warning, `tag-unique`) | `GetObjectByTag` finds only one of them: a start waypoint copied and left behind keeps new characters there. Its fix gives the others tags of their own (`TAG_2`, `TAG_3`…; an area keeps its tag); what the scripts look for is yours to check |
| An area the project's [claims table](03-modules.md#protected-areas) says is merged that the module no longer has (`claims-merged`), or whose Tag is not the one the table records (`claims-tag`); errors | the game keeps players' first visits by an area's ResRef and their positions by its Tag: those who were there lose them |
| A tile model with over 10,000 faces | can crash Aurora when painting |
| A hak over 2 GiB with files starting past that mark | the game can't read those files and doesn't look for them in lower haks (a 2DA reads as empty, a script doesn't run); nor do Aurora and nwsync |

Moonglow reads a hak over 2 GiB as the game does: files that start past
the mark are listed but can't be opened. Move them to another hak.

A blueprint's problems are warnings, since they matter once it's placed;
a placed object's are errors. A hak that hides the game's own longer
2DA isn't reported, because haks made before Enhanced Edition routinely
do that. When it matters, an object naming one of the lost rows is
reported instead.

The checks of enabled [plugins](15-plugins.md) run with these, and their
findings are listed with the rest: a team's own rules (naming, required
scripts) become part of Verify Module, with the fixes they offer.

`mg verify` does the same from a terminal, and can write its results as
JSON for a build pipeline (`mg --json verify`; see [Command-line
tools](11-command-line.md)). There each problem carries the id of the
check that found it (`set-model`, `2da-row`…); `mg checks` lists the
checks and the group each is in, and `mg verify --skip GROUP` leaves a
group out (`references`, `tilesets`, `models`, `2das`, `objects`,
`module`, `plugins`; several separated by commas): its checks aren't
run, which makes a Verify of a module with many haks much quicker, and
the JSON's `skipped` names the groups (`"tilesets": null`: not
checked). `mg verify --timings` says how long each part of the check
took.
A finding's `fix` in the JSON is the fix it offers, as an edit file `mg
apply` takes; `mg verify --fix CHECK` makes the fixes of a check's
findings and saves (`--dry-run`: only says which it would make).

**Models kept as text.** A model in a hak, the override or the module
that is text (ASCII, not compiled) is read as the game would read it
(`mdl-text`): what the game refuses or crashes on is an error (a
point-to-point emitter without its reference node, a node without a
type or a name), what it would misread a warning (a node without its `endnode`,
a second node of a name whose children hang from the first, more than
four bones on a vertex; the first five of a model). Keywords the game
skips are not named: old content is full of them.
Each names the line. Compiled models are not read so.

**Build › Compile Models** compiles the module's own models kept as
text, each against its supermodel from the module, its haks or the
game, in the background and as one step to undo. A model with errors,
with no nodes, or whose supermodel isn't found stays as text, and the
log says why. (The hak editor's Compile Models does the same for a
hak, `mg pack --compile-models` for a folder.)

## Test Module

**Build › Test Module** (F9) saves the module and starts the game on it:
your first local character appears at the module's start location. The
game looks for the module by name in your user folder's `modules`
folder, so the module must be saved there (File › Save As…). The game is
started on that same user folder (`-userdirectory`): its characters,
override and settings. To test with another, choose it in Options ›
Folders, or start Moonglow with `--user-dir FOLDER` for one run (see
[Options](10-options.md)). With
**Minimize Toolset on test module** (Options › General), Moonglow's window
is minimized.

A Steam copy of the game is started so that it reaches Steam when Steam
is running, and loads your Steam Workshop content as when you start it
from Steam. With **Start a Steam copy of the game through Steam for a
test** off (Options › General), the game's program is started on its
own instead: no Steam overlay and no Workshop content, which some find
steadier.

- **Test Module, Choose Character** (Shift+F9) opens the game's
  character selection for the module instead, so you can test with any
  of your characters. You no longer need to rename one to sort it first.
- **Test From Here** (an area's right-click menu) starts the game at the
  point you clicked, facing the way the camera looks, using the module as
  it is now, without saving it. It writes the module to `modules` as
  `moonglow-test.mod`, replaced each time, so your module and its start
  location are left as they are.
  **Test From Here, Choose Character**, below it, opens the game's
  character selection first, as Test Module, Choose Character does.

Moonglow starts the game as a separate program (`nwmain +TestNewModule`),
as nwn.wiki recommends, to avoid the problems it lists for Aurora's F9:
lagging combat, AI that overflows its time, and modules damaged when the
game crashes. The module is always saved before
the game starts, and Test From Here plays a copy.

`mg serve` runs a module in the game's headless server from a terminal,
and `mg test` runs a module's test scripts there and reports what they
assert: see [Command-line tools](11-command-line.md).

## Reloading haks and 2DAs

While you work on custom content in another program, Moonglow picks up
these changes without a restart:
- a hak rewritten;
- 2DAs, models, textures and talk tables added, changed or removed in
  `override` or `development`;
- the module's custom talk table.

It checks every few seconds (Options › General, on by default), or when
you choose **Tools › Reload Resources**. Palettes, previews and open areas are
redrawn from the new files, keeping the camera, and the log says what
was reloaded.

## Area Statistics

**Build › Area Statistics** shows, for the area shown last, its tiles,
its objects and the memory its models take.
