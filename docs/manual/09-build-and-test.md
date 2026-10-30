---
type: Manual Page
title: Build, verify and test
description: Build Module, publishing to NWSync and uploading it to the web server, long work, Verify Module and the fixes beside its findings, the Round-Trip Check, Test Module, reloading haks and 2DAs, and Area Statistics.
tags: [manual, build, verify, test, nwsync]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-30T05:15:28Z }
---

# Build, verify and test

## Build Module

**Build › Build Module…** prepares the module for play and checks it, as
Aurora's Build does:

- **Compile**: the **Scripts** (with their debug files, where **Generate
  Debug Information** is on in Options), recalculated **Creature CR** (challenge
  ratings), **Encounters** (each encounter's creature list updated with
  its creatures' challenge ratings and appearances; creatures that no
  longer exist are removed) and **Palettes** (the custom palettes made
  anew, as Aurora's Build and Refresh Palette make them: the module's and
  its haks' blueprints, by name, a name from the talk table as its
  `STRREF`; entries of the override folder's blueprints and of blueprints
  there are no more go. A plain save changes only the entries of the
  blueprints you changed: [Blueprints](05-blueprints.md#palettes)).
- **Check that resources are available**: **Missing Resources** (scripts,
  blueprints, conversations, models… that something in the module names
  but neither the module, its haks nor the game has) and, if ticked,
  **Unused** resources (in the module but used by nothing).

Compile and Missing Resources are on by default, Unused off. The
**Results** list what was found; double-click a result to open what it is
about, and **Export…** saves the list as text. With **Build module on
save** (Options › General), every save builds first (with Palettes
ticked, every save makes the palettes anew, as Aurora's Build does).

### What counts as used

**Unused** lists the module's scripts, conversations and blueprints
that nothing uses (Advanced Controls chooses which of the three). A
script is one line, whichever of its files the module has: `Unused:
x.nss (source and compiled)`, `x.ncs (compiled only (no source))` or
`x.nss (source only (not compiled))`. A resource is used when:

- **A field names it**: an event's script, a conversation, a placed
  object's blueprint, an inventory's item, a store's... as Missing
  Resources reads them.
- **A script includes it** (`#include`).
- **A string in a script spells its name**, whatever the string is
  passed to: `ExecuteScript("x")`, `SetEventScript`, `CreateObject`,
  `ActionStartConversation`, `SetLocalString` (not in a comment, nor a
  script's own name), and what a longer string quotes
  (`ExecuteScriptChunk("ExecuteScript(\"x\", OBJECT_SELF);")`).
- **The game runs a script from its 2DAs**, as the module sees them (its
  own 2DAs and its haks' over the game's): `spells.2da` ImpactScript (a
  spell's script), `vfx_persistent.2da` ONENTER, ONEXIT and HEARTBEAT
  (an area of effect's), `statescripts.2da` SCRIPTNAME (what a charmed,
  confused, frightened, dominated, stunned or sleeping creature runs),
  `traps.2da` TrapScript (a trap a script, a DM or a player sets),
  `disease.2da` End_Incu_Script and 24_Hour_Script, and `poison.2da`
  Script_1 and Script_2. (`areaeffects.2da` names scripts too, but the
  game doesn't read it.) A module that overrides the game's
  `nw_s0_fireball` keeps it without a 2DA of its own. And a 2DA of the
  module's own whose [description](03-modules.md#a-tables-description)
  says a column holds a script (`kind = "script"`): a table a script
  reads with `Get2DAString` and runs with `ExecuteScript`.
- **What the module reaches in its haks or the game names it**: a
  conversation in a hak that a placed creature has, a hak's blueprint a
  string creates, and what those name in turn (Kingmaker keeps its
  conversations and blueprints in its hak), and the scripts the game's
  own spell scripts name (Dirge's area of effect).
- **The project file names it** in `[used]` (below).

A name made as a script runs (`ExecuteScript("ai_" + sMode)`, a name
read from a variable or a 2DA of the module's own) is not seen. Name
those in the project file: select the line and **Mark as Used** (or
right-click it), which writes its name into
[the project file](03-modules.md#the-project-file-moonglowtoml)'s
`[used]` (beside a `.mod`, `NAME.moonglow.toml`, made where there is
none); the line goes, and so do the others it names. **Counted as
Used**, under the results, lists what `[used]` names, with **Remove**
beside each, and adds a name or a glob (`j_ai_*`: `*` any letters) of a
script, a conversation or a blueprint. Aurora never reads or writes the
file, so the marks stay when the module is saved there. `mg used`
does the same from the command line.

Find References says where a script is used in the same terms: under
the module's uses, **Run by the game, from its 2DAs** (a table's row and
column; the module's own table opens), and the `[used]` glob that
counts it as used.

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

Publishing again writes only the changed files. It runs as the other
long work does (see [Long work](#long-work)): a small window shows how
far it is and has **Cancel**, which stops before the next file and
writes no manifest (`latest` stays as it was; the data files written by
then stay, for the next publish to find). Then the window shows the
manifest's hash (**Copy** copies it, for `-nwsynchash`). Send the
folder to the web server: by hand, or with **Upload** (below). `mg
nwsync` does the same from the command line.

**Remove Unused Data** tidies the folder as neverwinter.nim's
`nwn_nwsync_prune` does: it takes away the data files that no manifest
in the folder names, once they are more than two weeks old (a publish
under way, or one canceled a moment ago, keeps its own), the folders
they leave empty, and the `.json` of a manifest that is gone. Publishing
never removes a manifest, since a server may still name an older one
with `-nwsynchash`: delete the manifests you no longer serve
(`manifests/HASH` and `manifests/HASH.json`) first, then Remove Unused
Data takes their data. What it can't mend it says, in the window and the
log: `latest` naming a manifest that isn't there, a manifest without its
`.json`, a data file a manifest needs that is missing (publish that
module again). `mg nwsync-prune` does the same from the command line.

### Uploading to the web server

**Upload** sends the folder to the web server that serves it, with a
command of your own: the tool you already reach that server with, and
its own login (an SSH key, a cloud tool's profile, a saved WinSCP
session). Moonglow keeps no password and talks to no server itself: a
server is reached a different way on every host (SSH, an S3 bucket, a
web host's FTP, a git push to GitLab Pages, as nwn.wiki's free hosting
page has it), and your tools already know how.

Set the command in **Options › Folders** (**NWSync upload command**),
or for a team in the project's `moonglow.toml`:

```toml
[nwsync]
upload = "rsync -a {part} builder@nwsync.example.org:/srv/nwsync/"
```

(or `{ run = "…", windows = "…", unix = "…", input = true }`, as a
[project's command](03-modules.md#the-projects-commands) is written).
A builder's own `.moonglow/user.toml` may have one too, which wins over
the team's; and the project's wins over Options'. The window says which
is used and where it was set.

**The command runs once for each part of the folder, in turn:** the
data (`data`), then the manifests (`manifests`), then `latest`, each
only after the one before ended without an error (exit code 0). In it,
`{part}` is the part's name and `{folder}` the folder (quoted where its
path needs it), and it runs in the folder, so `{part}` alone names what
to send. A part that fails stops the upload there: what came after it
isn't sent, and the log says what was. A part the folder doesn't have
(no `latest` yet) is left out; `latest` naming a manifest the folder
doesn't have stops it before anything is sent.

The order is why it runs in parts. A web server is read while you send
to it: nwserver reads `latest` when it starts (unless given
`-nwsynchash`), and a player's game fetches the manifest it is told of,
then the data the manifest names. A manifest that is there before its
data, or a `latest` before its manifest, fails a player who joins
meanwhile. `nwn_nwsync_write` writes the folder in that order (the data,
then the manifest, then `latest`), but one command over the whole folder
doesn't keep it: rsync sends its files in their names' order, so
`latest` goes before `manifests/`.

Some commands, each for its tool's own login:

| Server | Command |
| --- | --- |
| A server you reach by SSH (rsync: only what changed is sent) | `rsync -a {part} builder@host:/srv/nwsync/` |
| The same, without rsync (scp sends every file each time) | `scp -r {part} builder@host:/srv/nwsync/` |
| The same from Windows, by a site saved in WinSCP | `winscp.com /command "open mysite" "put {part} /srv/nwsync/" "exit"` |
| An S3 bucket (the AWS command line) | `aws s3 sync {folder} s3://bucket/nwsync --exclude "*" --include "{part}/*" --include "{part}"` |
| A bucket or host rclone knows | `rclone copy {folder} remote:nwsync --include "/{part}/**" --include "/{part}"` |
| A folder on this computer or a share (Linux, macOS) | `cp -R {part} /srv/nwsync/` |
| A network share from Windows | `if {part}==latest (copy /Y latest \\server\nwsync\) else (robocopy {part} \\server\nwsync\{part} /E & if errorlevel 8 (exit 1) else (exit 0))` (robocopy's exit codes below 8 mean done) |

rsync, rclone and `aws s3 sync` send only the files that aren't on the
server yet: the data of a manifest already there isn't sent again.

A script of your own gets `MOONGLOW_NWSYNC_FOLDER` (the folder),
`MOONGLOW_NWSYNC_PART` (the part) and, in a project, `MOONGLOW_PROJECT`.
**Upload after publishing and after removing unused data** runs it at
the end of each (not after a Cancel or an error). After **Remove Unused
Data** it sends the manifests before the data, and not `latest` (which
is as it was): a command that takes away on the server what the folder
no longer has (`rsync -a --delete {part} …`) takes away a manifest
before the data it names. Without such a command, what a prune took away
stays on the server until you take it away there.

It runs as a project's command does: as a job, its output under its
name as it comes and in the log, with **Cancel** (which stops it, and
sends nothing after it); **Show Output** shows all it said afterwards,
and **Last upload** how it ended. With `input = true` (in Options: **It
reads what I type**) a line under its output gives it what you type. A
program that asks on the terminal itself (ssh asking for a password)
can't be answered from Moonglow: give it a key or an agent. A command
from the project's files is a program you did not choose: it is
[asked about](03-modules.md#the-projects-commands) before it first runs,
and whenever its text changes, and what you allowed is kept in
Moonglow's own settings.

`mg nwsync MODULE FOLDER --upload` uploads after publishing, by the
project's command or one given (`--upload "COMMAND"`); `mg nwsync-prune
FOLDER --upload "COMMAND"` after a prune. There the command's output is
on standard error and it reads the terminal, so it may ask you for a
password.

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
untick its group, or one check of it, in **Options › Verify**, or name
the hak or the tileset there; the summary line says what was left out,
not checked. **Clear**, in the log's
corner, empties the log before a second run.

**Fixes.** A problem whose right change is plain has a button beside it
in the log that says what it does: **Retag the other as …** for a tag
the project wants unique, **Move the variables to the GIT** for an
area's variables in its ARE, **Remove the variable** for one only the
scripts set kept in a file (`var-runtime`), **Take the transform off**
for a static placeable's visual transform, which the game doesn't draw
(`static-transform`; to have it drawn transformed, untick Static in its
Properties instead), and whatever a plugin's check offers.
Nothing changes until you click it; then the change goes in as one step,
which **Undo** takes back, and the button says **Fixed**. A fix is
worked out when Verify runs, on the module as it was then: if what it
would change has changed since (by hand, or by another fix that did more
than set fields and take items out), the log says so, and Verify run
again offers it anew. Fixes that take items out follow one another: of
two variables of one object removed, the second is removed from where
it is after the first (since Moonglow 2.13.0).
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
| A tileset whose lines end in a bare LF instead of CRLF (as an editor on Linux or macOS may save it) | the game's server never finishes loading a module that uses it. With only some of its lines in LF (a warning, naming the first), it loads them at the file's head or in its middle, and may not at its end: a few hundred bytes of LF lines there stopped a module loading |
| A hole in a tileset: a `[TILEn]` section its count calls for that is missing, or a tile without `Model=` (a terrain's, a crosser's or a rule's section missing too) | the game draws the tile as nothing, with no walkmesh, and reads the rest; so does Moonglow, and an area's problems name the holes it uses |
| A tile whose model is missing | Aurora crashes painting it |
| A group whose first tile is `-1` or shared with another group, or names a tile that doesn't exist | the group won't paint, or Aurora crashes |
| A tileset door type that isn't a `doortypes.2da` row | the door has no appearance |
| A material (`.mtr`) naming a texture longer than 16 characters | Aurora's "Pure virtual function called" |
| A shader program that doesn't compile: one a material names (`customshaderVS`, `customshaderFS`), one of the haks' `.shd` programs, or a stock program broken by a hak's copy of a stock include (`inc_lighting`). Named at the file and line with the shader compiler's message, the programs it breaks listed. Compiled as the game's players have them: at the game's highest settings, then at Performance and Low shader quality, with enhanced lighting off, for an opaque material and with 3 lights; a program that fails only at one of these is named with it (`fs_water (fragment, at Low shader quality)`). Programs that compiled are kept in Moonglow's cache folder, so the next Verify compiles only what changed | the game draws the meshes that use it magenta |
| A 2DA row with more cells than columns (two rows run together) | Aurora's access violation; the game drops the cells |
| `baseitems.2da` over 256 rows, `lightcolor.2da` over 32 | Aurora fails (the game is fine) |
| A 2DA string reference past the end of its talk table (also of the item property tables, and the `Name` of the tables they name; since Moonglow 2.13.0) | "Bad Strref" |
| A hak's 2DA row naming a row of another table that isn't there or is blank, a padding row (a feat's required feat, a spell's feat, a race's appearance, a package's class, an appearance's footsteps, an item property's cost table; some sixty columns of twelve tables, the feat, skill and spell lists that a class, a race or a package names, and a class's requirements, each by its kind: a feat, a skill, a race, a class), or naming a table that isn't there (a class's feat list, bonus feats, spells or saves table, an item property's subtypes; since Moonglow 2.13.0). A cell the hak keeps as the game has it, wrong in the game's own table too, is the game's and not said (seven appearances name footstep 30 of 18) | the feat, spell or class is missing or wrong in the game (a warning) |
| A custom talk table the module names that isn't in its haks, the module or the `tlk` folder (or is named with `.tlk`, or, on Linux, in a different case from the file) | the game won't load the module |
| A hak's 2DA hiding another hak's longer copy | the rows past it are lost (an older copy?) |
| A creature, placeable, door or item naming a 2DA row that doesn't exist or whose model is missing | Aurora's area view crashes on it; in the game it has no appearance, or the game crashes (a creature with a class that doesn't exist) |
| A waypoint whose Appearance is a `waypoint.2da` row with no flag model (a warning) | the area view draws no flag for it; the game draws no waypoint |
| A creature, placeable, door or encounter of the PC faction, or a creature, placeable or door with no faction, which the game reads as the PC faction (a warning; not a trigger, which Aurora gives faction 0 when its blueprint has none) | the PC faction is the players' alone: the game puts such a creature in Hostile, and creatures regard the others as they regard players |
| A static placeable whose file has a visual transform (a warning) | the game client draws a static placeable without it, merged into the area's tiles: untick Static to have it scaled, tilted or moved |
| An area with variables in its ARE (as Moonglow wrote them before 1.34.3) | the game reads an area's variables from its GIT, so a script reads 0 for them; opening the area's Variables moves them, as does its fix |
| A tile whose model draws a `replace_tex` texture and whose `Tile_ReplaceTex` is 0 or past `replacetexture.2da` (an error, `tile-replace`) | the game crashes as it loads the area: choose a row in Tile Properties' Replacement Texture |
| A tag the [project file](03-modules.md#the-project-file-moonglowtoml) says must be unique (`[tags] unique`) on more than one area or placed object, or one it says must be unique in each area (`unique_in_area`) on more than one of an area (a warning, `tag-unique`) | `GetObjectByTag` finds only one of them: a start waypoint copied and left behind keeps new characters there. Its fix gives the others tags of their own (`TAG_2`, `TAG_3`…; an area keeps its tag); what the scripts look for is yours to check |
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

What Verify finds of each model in a hak (its text's problems, a tile
model's faces) is kept in Moonglow's cache folder, beside the shader
programs that compiled, so the next Verify reads only the haks that
changed (another time or size) since: a module with a hundred haks of
models is checked again in a fraction of the time. A new release of
Moonglow reads them all once more.

The checks of enabled [plugins](15-plugins.md) run with these, and their
findings are listed with the rest: a team's own rules (naming, required
scripts) become part of Verify Module, with the fixes they offer.

`mg verify` does the same from a terminal, and can write its results as
JSON for a build pipeline (`mg --json verify`; see [Command-line
tools](11-command-line.md)). There each problem carries the id of the
check that found it (`set-model`, `2da-row`…); `mg checks` lists the
checks and the group each is in, and `mg verify --skip WHAT` leaves
something out (several separated by commas): a group (`references`,
`tilesets`, `models`, `2das`, `objects`, `module`, `plugins`), one
check (`set-hole`, or a plugin's, `example.tag-conventions/tag-case`),
the missing references of one category (`references:creatures`), those
only your own content has (`references:override`, `references:local`
for your override folder, userpatch.ini's haks and Workshop items), a hak
(`tdod_tiles01.hak` or `hak:tdod_tiles01`), the `override` or
`development` folder, or one resource (a tileset, `ttr01.set`, with its
tiles' models; a 2DA; a model). What is left out isn't checked, which
makes a Verify of a module with many haks much quicker, and the JSON's
`skipped` names each (`"tilesets": null`: not checked). `mg verify --timings` says how long each part of the check
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
log says why; the compiler's notes on a model compiled (three of each)
are in the log too, with the linter's notes on its text, which Verify
Module leaves out (what the game or nwnmdlcomp does not read, such as
`setfillumcolor`; a name longer than a resource's). (The hak editor's
Compile Models does the same for a hak, `mg pack --compile-models` for a
folder, a nasher project's Pack where its project file asks, and `mg
model compile` for one file.)

## Round-Trip Check

**Build › Round-Trip Check** checks that Moonglow keeps the module as it
is, as `mg roundtrip` does: in the background, it saves a copy of the
module as it is in the window (unsaved changes too) to a temporary file,
reads it back and compares every resource byte for byte, then writes
every GFF and 2DA anew and compares what it reads back (the same fields
and cells, the fields Moonglow has no name for too; a GFF written twice
must come out the same). The log says how much was compared and names
each resource that came back otherwise; a file that can't be read as its
type says (no GFF under a GFF's name) is kept as it is, never written
anew, and named too. The module and its file are not touched.

It doesn't ask the game or Aurora (the tests do, on the game's own
modules), and a GFF is compared by its fields, not its bytes: Moonglow
lays a GFF out its own way, as every edit does. Scripts, models and the
other formats are kept as their bytes, which the saved copy holds them
to.

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

A test looks for a game already running first (Test Module before it
saves), for it would write the module under one that has it open (the
game then says the module is damaged). While the game Moonglow started for the last test runs, a
test asks you to close it first. So it does for a game Moonglow did not
start (one you started, or one left from an earlier session) that was
started on the same module (`+TestNewModule moonglow-test`, as a test
starts it); another game running is named in the log as a warning, and
the test goes on: close it first if it has the module open. On Windows
Moonglow can't see what a game was started on, so every game running
is a warning. A game is never closed for you.

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

It looks a moment after the system says one changed (every few seconds
where the system can't say; Options › General, on by default), or when
you choose **Tools › Reload Resources**. Palettes, previews and open areas are
redrawn from the new files, keeping the camera, and the log says what
was reloaded. An open area's models and textures are read again as it
was first opened, apart from the window (which stays at hand, the area
saying "Reading area…" meanwhile), and its textures fill in as they are
read.

## Area Statistics

**Build › Area Statistics** shows, for the area shown last, its tiles,
its objects and the memory its models take.
