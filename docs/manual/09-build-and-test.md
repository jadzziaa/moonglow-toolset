---
type: Manual Page
title: Build, verify and test
description: Build Module, publishing to NWSync and uploading it to the web server, a module's haks read from its NWSync server instead of hak files, long work, Verify Module and the fixes beside its findings, the Round-Trip Check, Test Module, reloading haks and 2DAs, and Area Statistics.
tags: [manual, build, verify, test, nwsync]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-27T23:23:12Z }
---

# Build, verify and test

## Build Module

**Build › Build Module** prepares the module for play and checks it, as
Aurora's Build does. Its window has a row of what to do, **Unused**,
**Missing Resources** and **Compile** (and **Spell Check**, grayed:
Moonglow has no dictionary), and **Advanced Controls**, which shows
under each what it covers, each with a check box of its own:

- **Compile**: the **Scripts** (with their debug files, where **Generate
  Debug Information When Compiling Scripts** is on in Options › Script
  Editor), recalculated **Creature CR** (challenge
  ratings), **Encounters** (each encounter's creature list updated with
  its creatures' challenge ratings and appearances; creatures that no
  longer exist are removed) and **Palettes** (the custom palettes made
  anew, as Aurora's Build and Refresh Palette make them: the module's and
  its haks' blueprints, by name, a name from the talk table as its
  `STRREF`; entries of the override folder's blueprints and of blueprints
  there are no more go. A plain save changes only the entries of the
  blueprints you changed: [Blueprints](05-blueprints.md#palettes)).
- **Missing Resources**: scripts, blueprints, conversations, models…
  that something in the module names but neither the module, its haks
  nor the game has, by kind (Areas, Creatures, Doors and the rest, in
  two columns).
- **Unused**: resources in the module that nothing uses (its
  **Scripts**, **Conversations** and **Blueprints**: see [What counts
  as used](#what-counts-as-used)).

Compile and Missing Resources are on by default, Unused off, and every
kind under them on. The window's foot has **Build** (the main button,
which Enter presses), **Mark as Used** (on the Unused result chosen in
Problems: [below](#what-counts-as-used)), **Export…**, which saves the
findings as text (a line each, `Error: x.nss: message`), and **Done**
(or Escape), which closes it. The build runs as [long work](#long-work)
does. Once it has built, the foot says how many errors and warnings it
found, "in Problems" ([below](#build-modules-results)). With **Build module on
save** (Options › General), every save builds first, with the window's
defaults rather than what is checked in it: so every save also makes the
palettes anew, as Aurora's Build does.

### Build Module's results

The results go to **Problems** in the Bottom Dock, under a **Build
Module** heading, its counts at the right of the dock's tab bar, errors
first. Each row has its severity, its place (a link, "Go to it" in its
tip, that opens the resource) and its message, cut where it is long
(the tip has the whole). Click a row to choose it; the arrow keys,
Page Up, Page Down, Home and End move the choice; Enter or a double click
goes to the resource. A right click on a row (which chooses it) or
Shift+F10 on the one chosen offers **Copy**, the finding as one line with
its whole message, and **Copy All**, every finding listed under every
heading; Ctrl+C copies the one chosen. While the build runs, the heading says "Build
Module is running". Where the build finds nothing, the heading says "No
problems found" and the log "No errors found". With **Build module on
save**, the results go to Problems the same way; Problems comes to the front, and the Bottom
Dock opens only where something was found.

When the module changes after a build, the heading says "Changed since
the check: not counted", and its findings are no longer counted in the
Status Bar's Problems switch. A right click on the heading, or its
**Problems Actions** button (`…`) at the right, offers **Copy These
Findings**, **Save These Findings As…** (one a line, into a text file)
and **Remove from Problems** (Delete with the heading focused), each on
that heading's findings alone. Closing the module
takes them away (Verify's too): a click on one would open another
module's resource.

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
those in the project file: choose the Unused result in Problems and
click the Build Module window's **Mark as Used**, or right-click the
result: **Mark as Used** (both need the module saved where the project
file can go beside it). Its name is written into
[the project file](03-modules.md#the-project-file-moonglowtoml)'s
`[used]` (beside a `.mod`, `NAME.moonglow.toml`, made where there is
none); the result leaves Problems, and so do the others it names. The
**Counted as Used** button, on the Build Module heading in Problems (it
shows where the build was asked for its Unused results), opens
what `[used]` names, with **Remove** beside each, and the kind, a field and
**Add** for a name or a glob (`j_ai_*`: `*` any letters) of a
script, a conversation or a blueprint (Enter adds). Aurora never reads or writes the
file, so the marks stay when the module is saved there. `mg used`
does the same from the command line.

Find References says where a script is used in the same terms: under
the module's uses, **Run by the game, from its 2DAs** (a table's row and
column; the module's own table opens), and the `[used]` glob that
counts it as used.

## Publish to NWSync

A server's players download its haks and talk table through NWSync:
before joining, the game fetches them from a web server that nwserver
names (`-nwsyncurl ADDRESS`). **Build › Publish to NWSync** writes what
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
- **Latest:** **Make it the latest (what nwserver serves unless told a
  hash)** points the repository's `latest` at
  it; nwserver serves `latest` unless given `-nwsynchash`.
- **Origins:** **Write an origins file** (`mg nwsync --write-origins`,
  as `nwn_nwsync_write --write-origins`) writes `manifests/HASH.origin`
  beside the manifest: each hak (and the talk table's file, and the
  module with the module itself) a resource came from, with the
  resources it gave. The game reads none of it; Moonglow does, where
  your team reads the module's haks from NWSync
  ([below](#nwsync-content-instead-of-haks)): it then knows which of the
  module's haks the manifest holds, and writes each of them with exactly
  its resources for a test. Each is named by its file's name alone
  (`Erf:pw_parts.hak`), where `nwn_nwsync_write` writes the whole path it
  was given: the folders of your computer stay off the web server. Upload
  sends it with the manifests. A manifest published again with it is the
  same manifest (the same hash): Moonglow asks for its origins file at
  the next Sync even where it kept the manifest before.
- **Compression:** the data files are compressed with zstd at level 3,
  as `nwn_nwsync_write` compresses them (the reference library, built
  into Moonglow: nothing to install). `mg nwsync --level` takes another,
  1 (fastest) to 22 (smallest); the game reads every level. Measured on
  the game's own `potsc_resources.hak` (110 MB), on a lightly loaded
  machine: level 1, 52.1 MB in 0.4 s; 3, 47.3 MB in 0.5 s
  (`nwn_nwsync_write`: 47.4 MB in 0.6 s); 9, 45.5 MB in 1.0 s; 15,
  44.5 MB in 3.6 s; 19, 42.2 MB in 10 s; 22, 42.2 MB in 18 s. A level
  above 19 saves little more.

Publishing again writes only the changed files. It runs as the other
[long work](#long-work) does; its **Cancel** stops before the next file and
writes no manifest (`latest` stays as it was; the data files written by
then stay, for the next publish to find). Then the window shows the
manifest's hash (**Copy** copies it, for `-nwsynchash`). Send the
folder to the web server: by hand, or with **Upload** (below). The
window asks for the **Repository folder** (**Browse…**) over the
choices above; its foot has **Publish** (the main button), **Remove
Unused Data** and **Upload** (grayed until an upload command is set),
and the × on its title row, or Escape, closes it. `mg nwsync` does the
same from the command line.

Keep the server's module loaded when nobody is on it: set
`reload-when-empty = false` under `[server]` in its `settings.tml`
(Server Options' **Reload Module When Empty** off). The game's server
(89.8193.37) reloads the module when its last player leaves, and the
module reloaded is offered without its NWSync manifest: a player who
leaves to download (the game disconnects for it) and joins again finds
the haks missing and is turned away, whenever nobody else was on the
server.

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
module again). **Cancel** stops it before the next data file: the window
and the log say how many it had removed by then, and the rest stay for
the next time. `mg nwsync-prune` does the same from the command line.

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
`MOONGLOW_NWSYNC_PART` (the part), `MOONGLOW_MG` (the `mg` that came
with Moonglow) and, in a project, `MOONGLOW_PROJECT`.
**Upload after publishing and after removing unused data** runs it at
the end of each (not after a Cancel or an error). After **Remove Unused
Data** it sends the manifests before the data, and not `latest` (which
is as it was): a command that takes away on the server what the folder
no longer has (`rsync -a --delete {part} …`) takes away a manifest
before the data it names. Without such a command, what a prune took away
stays on the server until you take it away there.

It runs as a project's command does: as a job, its output under its
name as it comes and in the log, with **Cancel** (which stops it, and
sends nothing after it); **Show Output** opens **Command Output** on
all it said (also File › Project › Command Output), and **Last upload**
says how it ended. With `input = true` (in Options: **It
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

## NWSync content instead of haks

A persistent world whose players download its haks through NWSync can
be opened without the hak files: Moonglow reads the haks' content from
the same NWSync server. Name the server in the project file
(`moonglow.toml` in a nasher project or module folder; for a module kept
in one file, the file named by it beside it: `krezk.moonglow.toml`
beside `krezk.mod`):

```toml
[nwsync]
source = "https://nwsync.example.org"   # as nwserver's -nwsyncurl
manifest = "latest"                     # or a manifest's SHA-1, to stay on it
```

**A repository folder** on this computer or a shared drive (the folder
Publish to NWSync or `mg nwsync` writes: `latest`, `manifests/`,
`data/sha1/…`) can be the source too, with no web server: `source =
"file:///srv/nwsync"` (on Windows with its drive: `"file:///D:/nwsync"`), a
whole path (`"/srv/nwsync"`, `"D:\\nwsync"`), or a
path from the project's folder starting `./` or `../` (`"../nwsync"`;
for a module kept in one file, from the `.mod`'s folder). A name alone
(`"nwsync"`) is taken for a server's name that lacks its `https://`, and
said. Sync reads the folder in place, without a server, and copies the
manifest and the data not here into Moonglow's cache, each checked
against its SHA-1, just as it downloads them from a server: the module
then opens from the cache whatever becomes of the folder (a drive taken
away, the repository pruned or published again), and a clean-up of
either never takes what the other holds. The cost is the space of the
copy. Where the folder isn't there, the manifest here is used and the
log says so.

When the module opens, the server's manifest is put into the game data
as one layer, where the game puts it: above the haks, below the
`development` folder and other loose files. Each resource is read from
**the game's own NWSync storage** (`nwsync` in the game's user folder,
what the game downloaded to join the server) when it has it, else from
**Moonglow's cache**. The game's storage is only read, never written: the
game checks it at start-up and downloads everything again when anything
else changed it. Moonglow reads it while the game runs, a moment at a
time. The custom talk table comes from the manifest too.

Opening asks no server: the manifest used is the newest one here (the
one Moonglow last downloaded from the server, or the one the game last
downloaded from it, the newer of the two). **Build › NWSync Content…**
shows which, how many of its data files are in the game's storage, in
Moonglow's cache, and not downloaded yet; **Sync** asks the server for
its latest manifest and downloads what neither has into the cache, a
few at a time, each checked against its SHA-1 before it is kept.
Downloading runs while you work: the Status Bar shows it as it shows
long work (Sync NWSync Content, how many data files are done and a bar),
and the window has **Cancel** (open it again from the Build menu or
Quick Open to stop a Sync); what was downloaded stays, so the next Sync
goes on from there. When it is done the layer is
put in again. When the server can't be reached, the newest manifest here
is used and the log says so. A resource of the manifest that is not
downloaded yet can't be read (the log says so): Sync first.

**Haks:** the game loads no hak files for a server that has a manifest
(the manifest holds the haks' content), so a hak of the module's list
that isn't on disk isn't reported missing. Where the server publishes
its origins file (`mg nwsync --write-origins` and `nwn_nwsync_write
--write-origins` write `manifests/<sha1>.origin`, which names each hak
and its resources), only
the haks it names are taken from NWSync, and any other is still
reported. Haks that are on disk are read too, under the manifest, as the
game would read them, except one newer than the manifest (below).

**Your own files newer than the manifest win.** A hak of the module's
list that is on disk (in your `hak` folder, the game's, or a Workshop
item's, as the game finds it), and whose file is newer than the
manifest (the time the game registered it, for one read from the
game's storage; else the time Moonglow's cache kept it), is your work
in progress: it is read over the manifest (the load order shows it
above the NWSync layer), and every test takes it from disk instead of
writing it from the manifest. The custom talk table likewise: your
project's own (`src/tlk/*.tlk.json`) or the one in `tlk`, when newer,
comes from disk, and its feminine table with it. Each such file is
said (the log, **Newer on disk** in the NWSync Content window, `mg`'s
warnings and its server's notes: "hak x on disk is newer than the
NWSync manifest: used over it"). A file older than the
manifest stays under it, as in the game, and nothing is said. To take a
newer manifest's copy again, sync (the manifest is then newer) or move
your file away. This is Moonglow's rule, not the game's: a player's
game reads the manifest over every hak.

**Turning it off for yourself.** A team's `moonglow.toml` that names
the server turns it on for everyone. `source = ""` under `[nwsync]` in
your own `.moonglow/user.toml` turns it off for you: the haks and the
talk table are then read and tested from disk, as without a server.
`mg serve`, `mg test` and `mg watch` take `--no-nwsync` for one run.

The cache is `nwsync` in Moonglow's cache folder (`~/.cache/moonglow` on
Linux, `%LOCALAPPDATA%\Moonglow\cache` on Windows,
`~/Library/Caches/Moonglow` on macOS); `[nwsync] cache` in your own
`.moonglow/user.toml` puts it elsewhere. It is laid out as the server's
repository is (`data/`, `manifests/`, `latest/`), with Moonglow's own
folders beside: `haks/` for the haks written for testing, `written/`
saying which hak is which for each manifest, and `used/`, an empty file
per manifest whose time is when it was last used (what Clean Up Cache
counts the last used by).

From the command line, `mg nwsync-status MODULE` says the same as the
window (`--online` asks the server for its latest manifest first), and
`mg nwsync-pull MODULE` downloads (`--sample N` downloads N data files
alone, to try a server); both take `--source URL` for a server the
project doesn't name.

A plugin reads the same state with `mg.nwsync.status()` (since plugin
API 0.11; [the plugin API](17-plugin-api.md#mgnwsync)): the source,
the manifest in the game data, the haks it holds, your files newer than
it, the data not downloaded yet (as the NWSync Content window last
looked) and a Sync under way, never the sign-in.

### A private server

A server that hands out its content only to those signed in (a web
server asking for a token or a password) is signed in to with a token,
sent as `Authorization: Bearer`, or with a user and password (`Basic`).
Put them in **your own `.moonglow/user.toml`**, never in the team's
`moonglow.toml`:

```toml
[nwsync]
host = "nwsync.example.org"   # the server the sign-in is sent to
token = "…"                   # or: user = "…" and password = "…"
```

`host` names the one server the sign-in is sent to; it may be left out
where the same file names the source (`source = "https://…"`), whose
host it then is. It is never taken from the team's file: a checkout
that named another server would otherwise get your token. The sign-in
is sent to that host alone: never with a redirect to another server, or
to another port or scheme of the same one, and never over plain
`http://` except to this computer (`localhost`, `127.0.0.1`): a
source on `http://` to another computer is refused with a sign-in, and
said, where anyone on the way could read it. A token, user, password or
host in the team's `moonglow.toml` (or a `.mod`'s file beside it, which
has no `.moonglow/user.toml`) is not read, and the log and `mg` say so
with the file and line: take it out and have it replaced on the server,
as everyone with the checkout has it. A user and password written into
the source's address (`https://user:password@…`) are refused too.
`.moonglow/` stays out of version control by the `.gitignore` of `*`
Moonglow makes in it (one made by hand needs one too: the log warns).

The token is never shown: not in the log, a message, `mg`'s output or
its JSON. Build › NWSync Content… says *Signed in with a token from
.moonglow/user.toml*; `mg nwsync-status --online` and `mg nwsync-pull`
say *signed in with a token from .moonglow/user.toml* (JSON:
`"signed_in": "a token"`). Where the server refuses it, the message
says the sign-in was refused, or that the server asked for one.

There is no environment variable for it: a sign-in is bound to a host
that the builder's own file names, and a variable would follow
whatever source a checkout names. On a build machine, write
`.moonglow/user.toml` from the machine's secret before `mg` runs.

### Testing with NWSync content

The game's server, and the game itself on a module of its own, read hak
files, not NWSync. So when the module is tested (**Test Module**, **Test
From Here**, `mg serve`, `mg test`, `mg watch --serve`), the haks the
server hands out are written from its manifest into Moonglow's cache
first, and given to the game:

- **Which resources go into which hak.** Where the server publishes its
  origins file, each hak it names gets exactly the resources it lists
  for it (the copy of each name that won from that hak when the server
  built the manifest). A hak of the module's list that the origins file
  doesn't name comes from disk, as before, and so does one on disk newer
  than the manifest (above). A hak the origins file names
  that the module doesn't list is not written (the game loads the
  module's haks alone; `mg nwsync-haks --all` writes them). Where the
  server publishes no origins file, the manifest's resources (but for
  talk tables) are spread over the module's haks that are **not on
  disk**, each resource in one, each hak kept under the 2 GiB the game
  can read (an entry can't start 2 GiB or more into a hak); a manifest
  too big for that many haks is an error that says so. A hak on disk is
  your file, perhaps never published, and is never replaced by a guessed
  slice of the manifest: it is taken from disk, and that is said. (So on
  the server a hak on disk listed above one written from the manifest is
  read over it, where the game data reads the manifest over every hak
  older than it; have the server publish its origins file for an exact
  copy.) Where every hak is on disk, nothing is written from the
  manifest.
- **The talk table** comes from the manifest where it has the module's
  (`world.tlk`), over the project's own and the one in `tlk`, as the
  game data has it, unless the one on disk is newer (above). Its
  feminine table (`worldf.tlk`) always comes from where it comes from.
- **Written once.** A hak is kept in the cache by what it holds, written
  the same way every time (resources in the order of their names, no
  build date), so the next test writes nothing, and a newer manifest
  writes only the haks it changed: the others are the files already
  there. The first time, for a persistent world, it is gigabytes and
  takes minutes: the window does it as a job (with its progress, and
  Cancel; the Status Bar shows it as it shows any job), `mg` says how
  far it is.
- **Not downloaded yet**: data the haks need that neither the game's
  storage nor the cache has stops the test before anything is written,
  saying so: Sync first (or `mg nwsync-pull`).
- **Where the game finds them.** `mg serve` and `mg test` put them into
  their server's own folder (hard links to the cache's files, which stay
  while the server runs whatever a cleanup of the cache does; links or
  copies where the system has no hard links). With `mg serve --dir
  YOUR_NWN_FOLDER --real-user-dir` a file of your own there (not a link
  Moonglow made) is never replaced: one the manifest's would replace
  stops the run, saying so. Nothing is ever written into your own `hak`
  folder, nor into the game's NWSync storage. Their folder has your
  `development` folder as a link, as `test-user` has it, so work in
  progress there is the server's too.

**What Test Module does with your user folder.** Test Module (and Test
From Here) starts the game with a user folder of Moonglow's, `test-user`
in its cache folder, made again from your own at every test:

- **Everything at the top of your folder but `hak`, `tlk` and
  `nwn.ini` is linked**: each folder (`localvault`, `saves`, `modules`,
  `logs`...) and each file (`settings.tml`, `nwnplayer.ini`,
  `cdkey.ini`...). So the characters, saves, logs and settings the game
  writes land in your own folder, as in any Test Module (and as in
  Aurora's). Folders are symbolic links (junctions on Windows where
  Moonglow can't make a symbolic link); files are symbolic links on
  Linux and macOS, and hard links on Windows, else symbolic links,
  else copies (another drive, without the right to make links). A file's link or copy is made again at every test, so a copy
  follows your file; but what the game saves into a copy is not kept.
- **`hak` and `tlk` are folders of `test-user`'s own**, holding links
  to this module's haks and talk table: those written from the manifest,
  and those of your own folder the module lists besides. A hak or talk
  table of yours newer than the manifest (the rule above) is linked from
  your folder instead of written; a project's own talk table kept as
  JSON is made into a `.tlk` in the cache first. A hak in the game's own
  folder or a Workshop item the game finds itself.
- **`nwn.ini` is a copy** without the `HAK` and `TLK` lines of its
  `[Alias]` section (the game would look for haks where they lead),
  made again at every test: options the game saves there are not kept.
- **Folders and files the game creates that your folder didn't have**
  stay in `test-user`, and are kept from test to test.

Nothing in your own folder is changed by making `test-user`: only what
the game itself writes through the links.

`mg nwsync-haks MODULE` writes them without a test and says what it
wrote and how long it took (`--out DIR` puts them in `DIR/hak` and
`DIR/tlk`, hard links where it can, for a server of your own; `--copy`
for copies; `--jobs N`, 4 at once).

### Cleaning up the cache

**Clean Up Cache…** in the window says what of Moonglow's cache no kept
manifest needs (data files, written haks, older manifests and their
files) and removes it when you say **Remove** (the Status Bar shows
either while it runs); **Keep Everything** leaves the cache as it is.
**Cancel** stops either before the next file, and so does closing the
module (the look at what is here as the window opens, and a Sync, stop
the same way). A clean-up stopped leaves the cache whole: it takes away
the manifests it doesn't keep before their haks and data, so every
manifest still in the cache has all it had, and what was left goes with
the next clean-up; the window and the log say what it removed by then.
A look for what would go that was stopped offers nothing to remove.
Kept: the module's
manifest (and the one pinned), the newest each server named, and the
three last used (`[nwsync] keep` in the project file changes the
number). What changed in the last hour stays whatever it is (a download
may be under way), and where a kept manifest's resources can't be read
here, no data file is removed. `mg nwsync-cache-prune [MODULE]` does the
same (`--dry-run` to see first, `--keep N`, `--min-age HOURS`); it is not
`mg nwsync-prune`, which tidies a server's repository. The game's NWSync
storage is never touched.

## Long work

Compile All Scripts, Compile Models, Build Module (its Build button),
Verify Module, the Round-Trip Check, Publish to NWSync, Find Instance,
Find and Replace Text and the Hak Pak Conflict Analysis run in the
background: a small window names the work, shows how far it is, and has
**Cancel** in its footer (**Escape** cancels the work too). The words
saying how far it is (`40 of 312`) are under the bar, and the right of
the Status Bar shows the work's name and a bar as well. Until it is done the rest of the window takes no
input, so nothing changes under the work; what it made (compiled scripts,
say) then goes in as one step that Undo takes back. Canceled, it changes
nothing. The build before saving (Options › General) and the compile
before a test run wait instead, since saving and testing need their
result. A module dropped on the window, Open, Close and the window's
close button wait too: the log says the work is still running, and they
are yours again once it is done or canceled.

## Verify Module

**Build › Verify Module** (or **Verify** at the Top Bar's right end)
writes to the log, as errors and warnings, what
is missing and what is wrong with the module's custom content. Its errors
and warnings also go to **Problems** in the Bottom Dock, under a **Verify
Module** heading, each with its resource as its place (Enter, a double click
or the link go there) and its fix button beside it where one is offered;
Problems comes to the front when anything was found. To leave
some of it out (a hak's tileset errors you are not going to fix),
uncheck its group, or one of its checks, in **Options › Verify**, or name
the hak or the tileset there; the summary line says what was left out,
not checked. Verify writes no file of its own: where it found anything,
its summary line says the findings are in Problems, where a right click
copies or saves them (the heading's **Save These Findings As…**). **Clear Log**, on the log's
tab bar (or View › Clear Log), empties the log before a second run.

A [project](03-modules.md)'s compiled scripts are build output, so its
tree holds its scripts as sources: Verify compiles those its build
compiles first, in memory, so a script reference is satisfied by a
source that compiles, and one that doesn't compile is said not compiled
(an error). Nothing is written.

**Fixes.** A problem whose right change is plain has a button beside it
in the log, and beside the finding in Problems, whose words are the fix
itself (in Problems, **Fix All (n)** stands beside the first of several
findings of one check, and a check mark, **Fixed**, takes the button's
place once it is made): **Retag the other as …** for a tag
the project wants unique, **Move the variables to the GIT** for an
area's variables in its ARE, **Remove the variable** for one only the
scripts set kept in a file (`var-runtime`), **Take the transform off**
for a static placeable's visual transform, which the game doesn't draw
(`static-transform`; to have it drawn transformed, uncheck Static in its
Properties instead), and whatever a plugin's check offers.
Nothing changes until you click it; then the change goes in as one step,
which **Undo** takes back, and the button says **Fixed** (the module has
changed, so Verify Module's heading in Problems then says "Changed since
the check: not counted"). A fix is
worked out when Verify runs, on the module as it was then: if what it
would change has changed since (by hand, or by another fix that did more
than set fields and take items out), the log says so, and Verify run
again offers it anew. Fixes that take items out follow one another: of
two variables of one object removed, the second is removed from where
it is after the first.
Only the last Verify's fixes are offered, and Clear Log takes them away
with the lines, in Problems too (the findings stay there, without their
buttons).

**Missing resources** are things something names that exist nowhere.
- **Errors:** scripts, conversations and areas named by areas, placed
  objects, conversations or the module, and scripts that were never
  compiled.
- **Warnings:**
  - what a blueprint names, which matters only once something places it;
  - sounds and portraits. A sound kept only as `.bmu` is missing too, and
    says so (`only mg_snd.bmu is there, which the game does not look
    for`): the game looks for a placed sound's, a sound blueprint's and a
    sound `PlaySound` plays only as `.wav` (read in the game), and a
    conversation line's is taken to be looked for the same way (not
    read).

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
| A tileset whose `[TILES]` or `[GROUPS]` count is below its sections (`set-count`, an error), or whose count of tiles, groups, terrains, crossers or rules is past its last section (a count that is no count, `Count=1869573190`: a warning; the holes it calls for are named by `set-hole`), or whose terrain, crosser or rule sections run past its count (a warning) | Aurora's "Range Check Error" when opening an area; for the others the game reads up to the count and no further |
| A tileset whose lines end in a bare LF instead of CRLF (as an editor on Linux or macOS may save it), or with a line of more than 255 characters | the game's server never finishes loading a module that uses it: a file all in LF, one with more than 256 bytes after its last CRLF, or one with a line longer than 255 characters (however it ends; the error names the line). With only some of its lines in LF elsewhere (a warning, naming the first), the server reads them as CRLF ones |
| A hole in a tileset: a `[TILEn]` section its count calls for that is missing, or a tile without `Model=` (a terrain's, a crosser's or a rule's section missing too) | the game draws the tile as nothing, with no walkmesh, and reads the rest; so does Moonglow, and an area's problems name the holes it uses |
| A tile whose model is missing | Aurora crashes painting it |
| A group whose first tile is `-1` or shared with another group, or names a tile that doesn't exist | the group won't paint, or Aurora crashes |
| A tileset door type that isn't a `doortypes.2da` row | the door has no appearance |
| A material (`.mtr`) naming a texture longer than 16 characters | Aurora's "Pure virtual function called" |
| A material (`.mtr`, the one the load order gives the game) naming a texture (`texture0` to `texture5`, not `null`) that no layer holds as a `.dds`, `.tga` or `.plt` (`mtr-texture`: an error for `texture0`, its color; a warning for a map in another slot) | the game draws the material untextured, or without that normal, specular or other map |
| A custom `.dds` of one luminance channel (a standard header, uncompressed, one gray channel; `dds-luminance`, a warning) that the game takes for the texture: no `.plt` of its name is in the load order (a creature's or an item's colored part is drawn from the PLT instead), and no `.tga` of a higher folder is drawn in its place | the game draws it in the wrong colors (a gray of 60 comes out blue, 128 dark red, 0 and 255 black; Moonglow draws it gray). Save it as a DXT1 `.dds` or a true-color `.tga` |
| An item that names no model (`item-model`, a warning): an item blueprint, or an item a blueprint's or an area's placed creature holds or wears (a blueprint's entries name the item's blueprint; a placed creature's are the items), or an item lying in an area, none of whose models (`ItemClass` and its part number, a cloak's, a composite's three pieces) is in the load order and whose base item's `DefaultModel` is empty or missing too. An armor is the body's and not looked at. Named with the creature where there is one, or the area for an item lying in it | the game can't make the item, and the area of a creature holding or wearing it never finishes loading (the client stays on the loading screen; its log says "CreateItem: Model … nor the default model could be loaded") |
| A shader program that doesn't compile: one a material names (`customshaderVS`, `customshaderFS`), one of the haks' `.shd` programs, or a stock program broken by a hak's copy of a stock include (`inc_lighting`). Named at the file and line with the shader compiler's message, the programs it breaks listed. A program whose includes loop, or add up to more than 8 MB of text (a file included again and again), is named the same way, at the include. Compiled as the game's players have them: at the game's highest settings, then at Performance and Low shader quality, with enhanced lighting off, for an opaque material and with 3 lights; a program that fails only at one of these is named with it (`fs_water (fragment, at Low shader quality)`). Programs that compiled are kept in Moonglow's cache folder, so the next Verify compiles only what changed | the game draws the meshes that use it magenta |
| A 2DA row with more cells than columns (two rows run together) | Aurora's access violation; the game drops the cells |
| `baseitems.2da` over 256 rows, `lightcolor.2da` over 32 | Aurora fails (the game is fine) |
| A 2DA string reference past the end of its talk table (also of the item property tables, and the `Name` of the tables they name) | "Bad Strref" |
| A hak's 2DA row naming a row of another table that isn't there or is blank, a padding row (a feat's required feat, a spell's feat, a race's appearance, a package's class, an appearance's footsteps, an item property's cost table; some sixty columns of twelve tables, the feat, skill and spell lists that a class, a race or a package names, and a class's requirements, each by its kind: a feat, a skill, a race, a class), or naming a table that isn't there (a class's feat list, bonus feats, spells or saves table, an item property's subtypes). A cell the hak keeps as the game has it, wrong in the game's own table too, is the game's and not said (seven appearances name footstep 30 of 18) | the feat, spell or class is missing or wrong in the game (a warning) |
| A class's spells, where `classes.2da`, `packages.2da`, `spells.2da`, `domains.2da` or a package's spell table is the module's or a hak's (`2da-spells`): a spellcasting class's `SpellTableColumn` naming a column `spells.2da` doesn't have, and a spell a package's `SpellPref2DA` table prefers that its class can't learn (no level in its column of `spells.2da`, nor, for a class that picks domains, granted by a domain). A table of the game's whose package's class can no longer learn some of its spells (a hak's `spells.2da` took them from the class) is said once, with how many; a fault the game's own tables have too is the game's and not said | the class has no spells; an NPC leveled up by the package passes over the spell (a warning) |
| A custom talk table the module names that isn't in its haks, the module or the `tlk` folder (or is named with `.tlk`, or, on Linux, in a different case from the file) | the game won't load the module |
| A hak's 2DA hiding another hak's longer copy | the rows past it are lost (an older copy?) |
| A creature, placeable, door or item naming a 2DA row that doesn't exist or whose model is missing | Aurora's area view crashes on it; in the game it has no appearance, or the game crashes (a creature with a class that doesn't exist) |
| A waypoint whose Appearance is a `waypoint.2da` row with no flag model (a warning) | the area view draws no flag for it; the game draws no waypoint |
| A creature, placeable, door or encounter of the PC faction, or a creature, placeable or door with no faction, which the game reads as the PC faction (a warning; not a trigger, which Aurora gives faction 0 when its blueprint has none) | the PC faction is the players' alone: the game puts such a creature in Hostile, and creatures regard the others as they regard players |
| A static placeable whose file has a visual transform (a warning) | the game client draws a static placeable without it, merged into the area's tiles: uncheck Static to have it scaled, tilted or moved |
| A static placeable, placed or a blueprint, with a tint's variable (`tdod_tint`, `tdod_tint_<mtr>`: a warning, `static-tint`) | the game client draws it as made, without the tint: uncheck Static to have it tinted |
| A placeable, placed or a blueprint, whose shader parameters store a `tdodTint` for a material its tint's variables also tint (a warning, `tint-stored`) | The Domains of Dread's script sets the variables' tint over the stored one as the module loads, so the stored one is never seen: take one of the two out |
| A placeable, placed or a blueprint, whose Appearance is past row 65535 (an error, `appearance-range`) | the game carries an object's placeables.2da row as a 16-bit number: it draws row 65536 as row 0, 65537 as row 1, another model, without a word |
| An area with variables in its ARE (as Moonglow wrote them before 1.34.3) | the game reads an area's variables from its GIT, so a script reads 0 for them; opening the area's Variables moves them, as does its fix |
| A tile whose model draws a `replace_tex` texture and whose `Tile_ReplaceTex` is 0 or past `replacetexture.2da` (an error, `tile-replace`) | the game crashes as it loads the area: choose a row in Tile Properties' Replacement texture |
| Neighboring tiles of an area that don't fit together: a corner whose terrain or height, or an edge whose crosser (a road, a stream), one tile has and its neighbor hasn't, as an area written by hand, by a script, by a plugin's `area:set_tile`, by the tiles palette's Exact Tile or by another tool can have (a warning, `tile-seam`, naming the tile, its neighbor and what each has). What a tileset's own group makes is the tileset's and not said: its tiles that disagree with each other (ttu01's two-tile group), and on its outside a corner they disagree about or a crosser no tile outside groups has. A tile the tileset lacks, or a hole in it, is the area's problems' to name | the ground of the two tiles doesn't meet (a step, a gap, a terrain cut off), or a road or a stream stops at the edge. Painting, Aurora's and Moonglow's, chooses tiles that fit, so it doesn't make one where the tileset has them. Paint the corner or the crosser again (painting chooses tiles that fit), or choose such tiles. The game loads the area as it is: of its own 1,462 areas, two ship areas of Tyrants of the Moonsea have eight (the plain deck beside a gangplank, where the tileset has the same deck with one) and Darkness over Daggerford one (a sandbank that ends at a cliff) |
| An area's music or ambient sound, or a placed sound's or sound blueprint's sound, that the game plays nothing of (a warning, `sound-silent`, in the Objects group). Of an area: a `MusicDay`, `MusicNight` or `MusicBattle` row of `ambientmusic.2da`, or an `AmbientSndDay` or `AmbientSndNight` row of `ambientsound.2da` (the table the module's haks give, over the game's), whose `Resource` is found nowhere, is a file of a type the game doesn't look for there (music only as `.wav`), holds what the game doesn't play there (a WAVE as music, MP3 under `.wav` as an ambient sound, a WAVE under `.bmu` as one), or is no sound at all (neither WAVE nor MP3); and a row past the table's end. Of a placed sound object or a sound blueprint, each sound of its list: MP3 under `.wav` without the `BMU V1.0` header (bare, tagged, or inside a WAVE), no sound at all, and, where the sound is positional, a stereo WAVE (a sound found nowhere or only as `.bmu` is a missing reference, above). Each names the area or blueprint, the field and the row, or the sound: `Music mg_track.bmu is a WAVE, which the game does not play as music (it must hold MP3)`, `Sound mg_wind.wav is stereo, which the game does not play from a positional sound (make it mono, or the sound area-wide)`. A `Resource` longer than 16 characters is said too (`Music mus_wc_ruralguitar is a name longer than 16 characters, which the game does not find`: read for music, an ambient row's by the same limit on names). Row 0 and a blank `Resource` are no music, and nothing is said of them; nor of the stingers (`Stinger1` to `Stinger3`), or of stereo MP3, which the game was not seen to play or not. Of the game's own content it says three things, each silent in the game by its reading: `ambientsound.2da` row 200 (`al_mus_throne2.wav`, MP3 under `.wav`), Wyvern Crown of Cormyr's music row 83 (`mus_wc_ruralguitar`; the same track as `mus_wc_ruralg` plays), and the positional blueprints `lavalakeloop` and `sludgelake`, whose sounds are stereo IMA ADPCM WAVEs, silent as shipped and heard made area-wide (and Hordes of the Underdark's `q3_area_sanctum`, which places one). A positional sound is silent on a stereo WAVE of any kind (PCM and IMA ADPCM were read) at any distance. All of it as read in the game: battle music as music (started by script; in real combat not tried), placed sounds as `PlaySound`'s. Each file is read once a run, and only its first bytes (a mebibyte at most for a bare MP3), however large the track. `--skip sound-silent` leaves it out | the area loads and plays on silent where Moonglow plays the file: music must be a `.bmu` holding MP3 (with the `BMU V1.0` header or without, or inside a WAVE whose format is MP3); an ambient sound a `.wav` holding a PCM or ADPCM WAVE, or a `.bmu` holding MP3; a placed sound a `.wav` holding a WAVE, mono where it is positional, or MP3 behind the header ([Area sounds](04-areas.md#area-sounds)) |
| An area transition trigger (`Type` 1) whose corners run clockwise seen from above, which the game neither shows under the pointer nor lets a player click (a warning, `transition-hidden`, in the Objects group): `its points run clockwise`. Also, as a possibility, one left at another level: none of its corners within 1 m of the walkable ground under it (each one's `ZPosition` + `PointZ`), as a trigger is left when the ground is raised or lowered under it, when it is lifted by the blue arrow or Adjust Location's Z, or when Aurora pasted it: `an area transition left at another level, which the game may neither show under the pointer nor take a click on: none of its points is within 1 m of the ground (the nearest, point 1, is 1.98 m below it by its ZPosition + PointZ)`. Why 1 m: nearer than that, whether the game shows it depends on the place, and what decides it isn't known (in the game: on flat ground 0.1 m off hides one; on a slope of Darkness over Daggerford's `c1ar1900_lizard` it still shows 0.6 m above and not 1 m above; the game's own transitions 0.23 to 0.31 m off are shown). A corner alone off the ground is not said (the game shows such an outline in part and takes a click on it), nor a corner over no walkable ground, nor a generic or trap trigger. Its fix, **Put the outline in order on the ground**, does in one step to undo what **Redraw Polygon** on the same corners does: the corners counterclockwise in Aurora's order, the trigger at height 0 and each corner 0.025 m above the ground under it, where the game shows a transition on every ground it was read on (`mg verify --fix transition-hidden` makes it too). Of the game's own modules it says nothing. `--skip transition-hidden` leaves it out | the game draws an area transition only with its corners counterclockwise, near enough to the ground under it; one it doesn't draw can't be clicked either, so the player never goes through ([Areas](04-areas.md#placing-objects)) |
| A tag the [project file](03-modules.md#the-project-file-moonglowtoml) says must be unique (`[tags] unique`) on more than one area or placed object, or one it says must be unique in each area (`unique_in_area`) on more than one of an area (a warning, `tag-unique`) | `GetObjectByTag` finds only one of them: a start waypoint copied and left behind keeps new characters there. Its fix gives the others tags of their own (`TAG_2`, `TAG_3`…; an area keeps its tag); what the scripts look for is yours to check |
| An area the project's [claims table](03-modules.md#protected-areas) says is merged that the module no longer has (`claims-merged`), or whose Tag is not the one the table records (`claims-tag`); errors | the game keeps players' first visits by an area's ResRef and their positions by its Tag: those who were there lose them |
| The project's [lines for typed text](03-modules.md#text-typed-into-the-talk-table) (`[talk]`) with a tenth or less left (a warning; none left, an error), lines of `mg tlk-alloc`'s map inside them, two builders' parts that share lines, or you without a part where other builders have one (`talk-range`) | text typed stays in the file once they are used; a key and a typed text, or two builders' texts, given one line |
| The project's [palette groupings](03-modules.md#palette-groupings) (`[palette.*]`): a key, a `from` file or an entry not understood (`palette-rules`), a group no blueprint of the standard or custom palettes it is offered in falls in (`palette-group-empty`), blueprints a `from` file lists that are nowhere (`palette-list-missing`); warnings | the grouping works without what isn't understood; an empty group isn't shown in the palette, and a blueprint listed under another ResRef isn't in its group |
| The project's [walkmesh bake](03-modules.md#baked-walkmeshes) (`[bake]`): its rules or rows file not right, or rows of its range the game's table holds already (`bake-rules`); a row of the rows file the module's haks' `placeables.2da` lacks, or has with another model (`bake-row-pending`); a row or a baked model no placeable of the module uses any longer (`bake-unused`); warnings, and an error for a rows file that can't be read | until the project's build adds a row, the game has no such row and draws nothing for a placeable of it (Moonglow draws it as the build will); rows no one uses are offered to be tidied when you save |
| A tile model with over 10,000 faces | can crash Aurora when painting |
| A hak over 2 GiB with files starting past that mark | the game can't read those files and doesn't look for them in lower haks (a 2DA reads as empty, a script doesn't run); nor do Aurora and nwsync |

Moonglow reads a hak over 2 GiB as the game does: files that start past
the mark are listed but can't be opened. Move them to another hak: the
[hak editor](03-modules.md#the-hak-editor) reads them where they lie, to extract them
or to save the hak without the others.

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
only your own content has (`references:override`,
`references:userpatch`, `references:workshop`, or all three as
`references:local`: your override folder, userpatch.ini's haks and
Workshop items), a hak
(`tdod_tiles01.hak` or `hak:tdod_tiles01`), the `override` or
`development` folder, or one resource (a tileset, `ttr01.set`, with its
tiles' models; a 2DA; a model). What is left out isn't checked, which
makes a Verify of a module with many haks much quicker, and the JSON's
`skipped` names each (`"tilesets": null`: not checked). `mg verify --timings` says how long each part of the check
took.
A finding's `fix` in the JSON is the fix it offers, as an edit file `mg
apply` takes; `mg verify --fix CHECK` makes the fixes of a check's
findings and saves (`--dry-run`: only says which it would make). A check
there is none of (mistyped, or a plugin's not given) fails it, so that a
pipeline doesn't read it as nothing to fix.

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

**Build › Test Module** (F9, or **Test Module** at the Top Bar's right
end, whose arrow has Choose Character) saves the module and starts the
game on it:
your first local character appears at the module's start location. The
game looks for the module by name in your user folder's `modules`
folder, so the module must be saved there (File › Save As). The game is
started on that same user folder (`-userdirectory`): its characters,
override and settings. To test with another, choose it in Options ›
Folders, or start Moonglow with `--user-dir FOLDER` for one run (see
[Options](10-options.md)). With
**Minimize Toolset on test module** (Options › General), Moonglow's window
is minimized.

A Steam copy of the game follows **Read Steam Workshop content**
(Options › Folders), so that the test and Moonglow's views show the
same content. Off (the default), Moonglow starts the game's program
without Steam's app ID, as Aurora does, and the game is expected to run
without your Workshop items (this was tried here only with Steam not
running). On, it is started through Steam: with Steam running it loads
the Workshop items you subscribe to (their textures and materials over
the game's, say), as when you start it from Steam. The log says which way
each test started. (Before 5.1.0 a separate option on General started
every test through Steam, so a test could show Workshop content that
Moonglow's views left out.)

- **Test Module, Choose Character** (Shift+F9) opens the game's
  character selection for the module instead, so you can test with any
  of your characters.
- **Test › Test From Here** (an area's right-click menu) starts the game at the
  point you clicked, facing the way the camera looks, using the module as
  it is now, without saving it. It writes the module to `modules` as
  `moonglow-test.mod`, replaced each time, so your module and its start
  location are left as they are.
  **Test › Test From Here, Choose Character**, below it, opens the game's
  character selection first, as Test Module, Choose Character does.

Moonglow starts the game as a separate program (`nwmain +TestNewModule`),
as nwn.wiki recommends, to avoid the problems it lists for Aurora's F9:
lagging combat, AI that overflows its time, and modules damaged when the
game crashes. The module is always saved before
the game starts, and Test From Here plays a copy.

### Test Module with NWNX

A module whose project uses [NWNX](11-command-line.md#nwnx)
(`[nwnx] use = true` and your `home`, on Linux) can't be tested by the
game alone: `+TestNewModule` starts the game's own server, which NWNX
never loads into, so most of such a module's scripts would do nothing.
Test Module serves it instead on a private `nwserver` with NWNX, in a
folder of Moonglow's cache (`test-server`, never your NWN folder), with
the module, its haks and talk table, and your `development` folder
linked, as `mg serve` has it without `--no-development`.
This is what **The game, on the module** in Options › Test Module does
for such a project; with **A command of my own** your command runs
instead ([below](#a-command-of-your-own)), and Moonglow starts no server
of its own. Once the server says the module is
loaded, the game starts on your user folder and joins it
(`+connect 127.0.0.1:PORT +password …`), to the game's character list
with your own characters on it. The Log follows the server (its
preparing, starting, the module loaded, the game joining, what NWNX
says and the module's log lines), and the status bar says how far it
is. The checks before the start are `mg serve`'s: an error is said in
the Log and nothing is started.

**Quick start.** With **Join with a chosen character** ticked in
[Options › Test Module](10-options.md#test-module), the game joins with
the project's chosen character instead: one of your local vault's, or a
`.bic` file of your own, kept for that project (the one chosen last). It
is put alone in the server's vault (`-servervault 1`, a copy: your file
is only read), so the character list shows it alone. One confirm
remains: you still choose it once on that list, for the game has no way
past it. Without a character chosen, or with its file gone, the game
shows your own characters, and the Log says why.

When the game ends, the server is told to `exit`. Only the server and
the game this test started are ever stopped. While they run, the status
bar shows the test as a job, and another test is refused: close the
game, or choose **Build › Stop Test Server** (offered while the local
server runs), which stops the server and the game, then test again.
(A server your own command started is not Moonglow's to stop there: the
next test or closing Moonglow stops it.)
Closing Moonglow stops them too, and on Linux they end with Moonglow
however it ends. On Windows and macOS such a module is tested by the game
alone, as any other, and the Log says once that NWNX runs only on Linux
servers.

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

### A command of your own

A persistent world is often tested on its own server: one with NWNX, a
database, settings in its environment, or one that runs in WSL on
Windows. **Options › Test Module** can have Test Module (and Test From
Here) run your own command in place of the game: choose **A command of
my own** and type it, as you would in a terminal. Moonglow saves the
module as Test Module always does, then runs the command with the
system's shell (`sh -c` on Linux and macOS, `cmd` on Windows) in the
module's folder. `{module_path}` in it is the module's file (or folder;
a nasher project's packed module, Test From Here's
`moonglow-test.mod`) and `{module_name}` its name, filled in as they
are: put quotes around them where they may have spaces. The module's
path is also in the `MOONGLOW_MODULE` environment variable.

```text
wsl.exe -d Ubuntu -- bash -lc "~/start-nwnx-server.sh '{module_path}'"
bash -lc "~/nwn/server start '{module_name}'"
```

The window goes on while the command runs: what it prints comes into the
log as it comes (what it writes to its error output as warnings), and
the log says how it ended. It runs until it ends, until the next test
of either kind, your command again or the game's (which stops it first,
and what it started, so that its server lets go of the port), or until
Moonglow closes, which stops it too.

With **Then start the game, joining the server**, Moonglow then waits
for a server to answer at **Server address** (`127.0.0.1:5121`, the
game's own port on this machine, unless you type another), asking it as
the game's server list does, for up to two minutes, and starts the game
connected to it (`+connect`, and `+password` with the **Player
password** you give), as it joins Test Module's own
[local server](#test-module-with-nwnx). The game shows the server's character list. If no
server answers in time, the log says so and no game is started. A server
in WSL answers at `127.0.0.1` only where WSL passes UDP through to
Windows (its mirrored networking mode); otherwise give the address WSL's
own network has. (Moonglow can't read that server's log, so it asks the
server, as the game does; a server Moonglow starts itself, the local
server or `mg serve`, is ready only once its log says the module is
loaded, as a game joining before then can quit.)

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
where the system can't say; **Reload haks, override and development
when they change** in Options › Folders, on by default), or when
you choose **Tools › Reload Resources**. Palettes, previews and open areas are
redrawn from the new files, keeping the camera, and the log says what
was reloaded. An open area's models and textures are read again as it
was first opened, apart from the window (which stays at hand, the area
saying "Reading area" and its name meanwhile), and its textures fill in as they are
read.

## Area Statistics

**Build › Area Statistics** shows, for the area shown last, its tiles,
its objects and the memory its models take. The report is a tab of the
Bottom Dock, **Resources Used**, which comes to the front: a table of
what is used (Tiles, Objects, Tile models, Object models, Meshes,
Triangles, Model memory, Textures), under the area's label. Click a row
and the arrow keys move; Ctrl+C copies the row. **Close Report**, at the
end of the dock's tab bar (also in the command palette), closes it; it
goes too when the module is closed.
