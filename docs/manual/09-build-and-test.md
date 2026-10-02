# Build, verify and test

## Build Module

**Build › Build Module…** prepares the module for play and checks it, as
Aurora's Build does:

- **Compile**: the **Scripts**, recalculated **Creature CR** (challenge
  ratings), **Encounters** (each encounter's creature list brought up to
  date with its creatures' challenge ratings and appearances; creatures
  that no longer exist are taken out) and **Palettes** (the custom
  palettes rebuilt from the module's blueprints).
- **Check that resources are available**: **Missing Resources** (scripts,
  blueprints, conversations, models… that something in the module names
  but neither the module, its haks nor the game has) and, if ticked,
  **Unused** resources (in the module but used by nothing).

Compile and Missing Resources are on by default, Unused off. The
**Results** list what was found; double-click a result to open what it is
about, and **Export…** saves the list as text. With **Build module on
save** (Options › General), every save builds first.

## Publish to NWSync

A server's players download its haks and talk table through NWSync: the
game fetches them from a web server that nwserver names
(`-nwsyncurl ADDRESS`) before it joins. **Build › Publish to NWSync…**
writes what that web server serves into a folder, as neverwinter.nim's
`nwn_nwsync_write` does (the same manifest, byte for byte):
- **What goes in:** the module's haks (the first-listed hak's copy where
  several have one) and its talk table, but not scripts' source or debug
  files or area comments, which players don't need. Files over 15 MB are
  refused, as `nwn_nwsync_write` refuses them.
- **With the module itself:** for a single-player module players download
  whole (not for a persistent world): its own resources too, under the
  haks', with a name and description players see.
- **Group ID:** for servers sharing one repository.
- **Latest:** **Make it the latest** points the repository's `latest` at
  it, which nwserver serves unless given `-nwsynchash`.

Publishing again writes only the files that changed. It runs in the
background; the window shows how far it is, then the manifest's hash
(**Copy** copies it, for `-nwsynchash`). Upload the folder to the web
server. `mg nwsync` does the same from the command line.

## Verify Module

**Build › Verify Module** writes to the log what is missing and what is
wrong with the module's custom content, as errors and warnings.

**Missing resources** are things something names that exist nowhere.
- **Errors:** scripts, conversations and areas named by areas, placed
  objects, conversations or the module, and scripts that were never
  compiled.
- **Warnings:**
  - what a blueprint names, which matters only once something places it;
  - a placed object's blueprint that is gone (the object keeps working);
  - sounds and portraits.

**Custom content** covers the module, the haks in your `hak` folder, and
`override` and `development`. Each problem names the hak or folder, the
file, and the row, column, section or object at fault. Most are things
that crash Aurora with an access violation that names nothing:

| Problem | Where it shows |
| --- | --- |
| A tileset whose `[TILES]` or `[GROUPS]` count disagrees with its sections | Aurora's "Range Check Error" when opening an area |
| A tile whose model is missing | Aurora crashes painting it |
| A group whose first tile is `-1` or shared with another group, or names a tile that doesn't exist | the group won't paint, or Aurora crashes |
| A tileset door type that isn't a `doortypes.2da` row | the door has no appearance |
| A material (`.mtr`) naming a texture longer than 16 characters | Aurora's "Pure virtual function called" |
| A 2DA row with more cells than columns (two rows run together) | Aurora's access violation; the game drops the cells |
| `baseitems.2da` over 256 rows, `lightcolor.2da` over 32 | Aurora fails (the game is fine) |
| A 2DA string reference past the end of its talk table | "Bad Strref" |
| A custom talk table the module names that isn't in its haks, the module or the `tlk` folder (or named with `.tlk`, or in another case than the file on Linux) | the game won't load the module |
| A hak's 2DA hiding another hak's longer copy | the rows past it are lost (an older copy?) |
| A creature, placeable, door or item naming a 2DA row that doesn't exist or whose model is missing | Aurora's area view crashes on it; in the game it has no appearance, or the game crashes (a creature with a class that doesn't exist) |
| A tile model with over 10,000 faces | can crash Aurora when painting |
| A hak over 2 GiB with files starting past that mark | the game can't read those files and doesn't look for them in lower haks (a 2DA reads as empty, a script doesn't run); nor do Aurora and nwsync |

Moonglow reads a hak over 2 GiB the way the game does: what starts past
the mark is listed but can't be opened. Move those files to another hak.

A blueprint's problems are warnings, since they matter once it's placed;
a placed object's are errors. A hak that hides the game's own longer
2DA isn't reported, because haks made before Enhanced Edition do that as
a rule. When it matters, an object naming one of the lost rows is
reported instead.

`mg verify` does the same from a terminal, and can write its results as
JSON for a build pipeline (`mg --json verify`; see [Command-line
tools](11-command-line.md)).

## Test Module

**Build › Test Module** (F9) saves the module and starts the game on it:
your first local character appears at the module's start location. The
game looks for the module by name in your user folder's `modules`
folder, so the module must be saved there (File › Save As…). With
**Minimize Toolset on test module** (Options › General), Moonglow's window
gets out of the way.

- **Test Module, Choose Character** (Shift+F9) opens the game's
  character selection for the module instead, so you can test with any
  of your characters. You no longer need to rename one to sort it first.
- **Test From Here** (an area's right-click menu) starts the game at the
  point you clicked, facing the way the camera looks. It uses the module
  as it is now, without saving it. The module is written to `modules` as
  `moonglow-test.mod`, which is replaced each time, so your module and its
  start location are left as they are.

Moonglow starts the game as a program of its own (`nwmain +TestNewModule`),
which is how nwn.wiki recommends testing to avoid the problems it lists
for Aurora's F9: combat that lags, AI that overflows its time, and
modules damaged when the game crashes. The module is always saved before
the game starts, and Test From Here plays a copy.

## Reloading haks and 2DAs

While you work on custom content in another program, Moonglow picks up
changes without a restart:
- a hak written again;
- 2DAs, models, textures and talk tables added, changed or removed in
  `override` or `development`;
- the module's custom talk table.

It looks every few seconds (Options › General, on by default), or when you
choose **Tools › Reload Resources**. Palettes, previews and open areas are
redrawn from the new files, keeping the camera, and the log says what
was reloaded.

## Area Statistics

**Build › Area Statistics** shows, for the area shown last, its tiles,
its objects and how much memory its models take.
