---
type: Manual Page
title: Options
description: The Options dialog page by page (folders, area, general with the graphics backend, script editor, conversation editor, sounds, verify, language, keyboard, updates) and where the settings are kept.
tags: [manual, options]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-28T00:19:15Z }
---

# Options

**Tools › Options…** has Aurora's pages (and Moonglow's own Updates),
in a window that docks and tabs with the others. **OK** applies the changes; the module stays open.
**Cancel**, closing the window or Escape discards them. Only another game or user folder, the Workshop and userpatch switches or a language the game has installed (the Language page)
has the game's data read again: unsaved work is asked about first, and
the module is opened again from its file.

## Folders

- **Neverwinter Nights installation**: the game's folder (with `data/`
  and `bin/` in it). Moonglow reads the game's data from here.
- **NWN user folder**: where your modules, haks, talk tables, override and
  characters are. Empty: `NWN_HOME` (or `NWN_USER_DIRECTORY`) if set,
  else the game's usual folder. Started as `moonglow --user-dir FOLDER`,
  Moonglow uses that folder instead for as long as it runs, and this page
  says so; what you set here is kept for the next start. A shortcut for
  each user folder you test with can name its folder this way.
- **Scratch folder**: where **To Scratch** (the script editor, the area
  view) and **Copy to Scratch Folder** (the module tree) copy a script
  with its compiled script, or an area, as loose files. Empty: asked for
  the first time. See [Modules](03-modules.md).

## Area

- **Background Color** of the area viewer (Custom, or the default).
- **Show Encounter Spawnpoint Markers**, and their **Height** and
  **Width**: at each spawn point, the game's marker (see-through), a post
  of that size, and an arrow the way what spawns there faces.
- **Show stores as $ signs**: the game's marker for a store, rather
  than an arrow along its facing.
- **Show the turning and tilt rings around selected objects**: the ring
  that turns the selection when dragged round, and the two that tilt
  its models, with the arrows that move it along one axis, while Shift
  is held (see [Areas](04-areas.md)). On by
  default.
- **Show Door Orientation Arrows**.
- **Allow editing tiles of groups and features** (off by default):
  Moonglow's own. Off, a group or feature (a barn, a gate, a statue) is
  one thing: Next and Previous Tile Appearance, Rotate Tile, Refine
  Tile, Shift + click and painting terrain leave its tiles as they are
  (the status line says why, and its tiles flash red), Tile Properties
  offers it no variants, no group goes over part of another (as Aurora
  refuses it), and the Eraser and Delete take it away whole. On, each
  of its tiles can be changed alone, a group placed over another takes
  that one away whole, and Delete takes a group's tile alone, as
  Aurora's does; the page and the status line warn, since a group left
  half changed is hard to see and to put right later (Undo does). See
  [Areas](04-areas.md#painting-terrain).

## General

- **Create backups of modules**: each save also keeps the module as it
  was in `<name>.BackupMod`.
- **Build module on save**: run Build Module (with its defaults) before
  each save.
- **Write a .mod beside a module folder on save** (on): a module opened
  as a folder is saved into the folder and as `<folder>.mod` next to the
  folder too, as Aurora saves a module directory, so that the game has a
  module file to load. Switch it off to keep a folder a folder (a source
  tree that something else packs). The `.mod`
  that was there is kept as its `.BackupMod` (with backups on). A nasher
  project is not packed this way: it has **Build › Pack** *file*.
- **A .mod with its folder beside it** (Ask which to open): a `.mod`
  opened with a folder of its name beside it (`mymodule.mod` and
  `mymodule/`, a module directory as Aurora keeps one) asks whether to
  open the folder or the `.mod`, as Aurora asks; **Open the folder**
  opens the folder without asking (Aurora's Always open module
  directories), **Open the .mod** the `.mod`. See
  [Modules](03-modules.md#opening-and-saving).
- **Minimize Toolset on test module**.
- **Start a Steam copy of the game through Steam for a test** (on):
  off, the game's program is started on its own, without Steam's overlay
  or Workshop content (see [Build, verify and test](09-build-and-test.md)).
- **Interface size**: the whole interface, text and all, from 90% to 200%
  of its usual size.
- **Light theme**: dark text on light, rather than the dark theme.
- **Graphics backend** (Automatic): what the window, the area view and
  the previews draw with. **Automatic** is the graphics library's
  (wgpu's) choice, except that an AMD GPU on Windows draws through
  Direct3D 12 rather than Vulkan: AMD's Windows Vulkan driver drew
  areas' tiles wrong (see [Troubleshooting](13-troubleshooting.md#tiles-disappear-on-an-amd-gpu-under-windows)).
  Or choose one: **Vulkan** or **Direct3D 12** on Windows, **Vulkan**
  on Linux, **Metal** on macOS (the only one there); only the
  platform's are offered. (Not OpenGL: Moonglow's renderer draws
  particles, the game's shaders and more wrong there.) It takes effect the next time
  Moonglow starts, and the page says what this run draws with. A backend
  with no adapter on this machine falls back to Automatic, and the log
  says so. The `WGPU_BACKEND` environment variable (`vulkan`, `dx12`,
  `metal`), when set, decides over this setting: a way out if the
  one chosen doesn't start.
- **Open a module on the area opened last**: on by default; the first
  time, the first area the module tree lists (see
  [Modules](03-modules.md)).
- **Open a module with the tabs it had when it was closed**: on by
  default. The areas, scripts, blueprints, conversations, side editors
  (a hak, a 2DA, a tileset) and tools open when a module was closed open
  again with it, in their order and places (windows split and where
  they were), the one in front in front, each view as it was left (an
  area's camera, a script's scroll and cursor, a conversation's
  selection, bookmarks and folds, a Properties window's page); kept in
  Moonglow's data folder, not in the module (see
  [Modules](03-modules.md)). With none kept, the module opens
  on the area opened last, as above. Off, a module's tabs are neither
  kept nor opened again.
- **Show the object selected in an area in the module tree**: on by
  default. An object selected in an area's view opens its area out in
  the module tree and goes to its row there, as Aurora does; off, the
  tree goes to it only where its area is opened out already (the arrow
  beside the area's row), and an area left closed stays closed.
- **Show the tab chosen in the module tree**: on by default. A tab
  clicked, reached by the keys or opened marks its resource (a script,
  conversation, blueprint, area…) in the module tree, opens its group
  and brings its row into view; the keys stay with the tab. Off, the
  tree stays where it is and marks only the area in front, as Aurora's
  does. Show in Module Tree on a tab's menu goes to it either way.
- **Open Properties as tabs of one window**: on by default. A
  blueprint's or a placed object's Properties open as a tab of the window
  Properties are open in already; off, each in a window of its own. A
  tab dragged out is a window of its own again, to set two side by side.
  (Scripts have the same on the Script Editor page.)
- **Move windows by dragging anywhere in them**: off by default. A
  window of tabs (Properties, conversations, scripts…) moves by its tab
  bar alone, so a click that misses a control doesn't move it; on, any
  part of it that isn't a control moves it too.
- **Ask before the window is closed**: on by default. The window's
  close button asks before Moonglow closes where nothing is unsaved (with
  unsaved changes it asks about those, as always); File › Exit closes
  without this question.
- **Ctrl+Tab goes to the tab used last**: Next Tab and Previous Tab go
  through a pane's tabs in the order they were last used, for as long
  as Ctrl is held (one press: back to the tab before); off, to the next
  tab as they lie.
- **Write a debug log**: what Moonglow does, step by step, in
  the `logs` folder of its data folder, a file for each start; on
  unless you switch it off (see
  [Troubleshooting](13-troubleshooting.md)).
- **List a blueprint's categories alphabetically**: a blueprint's
  Category list in alphabetical order, rather than in the palette's
  (the order Palette › Categories… gives them): case and accents aside
  (`Árboles` beside `arboles`, not after `z`), and a group's own
  categories (`Oficios / Herrero`) together under it.
- **List areas and blueprints by name** in the module tree, rather than
  by ResRef, and **Show ResRefs beside names** (`Name (resref)`, in the
  tree and the palettes). Areas are by name in Find Instance and the
  area transition's destinations too.
- **Show challenge ratings in the creature palette**: on by default.
- **Show armor and cloaks on a woman**: an armor's or a cloak's model on
  a woman's body, and an armor's inventory icon as a woman's inventory
  shows it (`ipf_` parts; a cloak's is the man's, the game having no
  other), in inventories, a store's pages, the palette's pictures and
  Preview and the item's Properties, whose Appearance page's **Shown
  on** changes it too. Off by default: a man's.
- **Read Steam Workshop content**: off by default, as in Aurora: an item
  you subscribed to for play is not your module's, and a module that
  needs it works only for players who have it too. On, the Steam
  Workshop items you are subscribed to are read with the game's data, as
  the game started through Steam reads them: each item's `override`
  under your own, its haks and talk tables found by name. Changing it
  reads the game's data again.
- **Read userpatch.ini's haks**: off by default, as in Aurora, for the
  same reason. On, the haks your
  `userpatch.ini` names (`PatchFile000=…` and on, the highest number
  winning), from the `patch` folder in your user folder (or the folder
  `nwn.ini` names as `PATCH`), are read under the module and its haks
  and over `override`, as the game reads them: the HD Pack and the like
  show in the area view and the previews. Aurora doesn't read them. A
  module that needs what only they provide works only for players who
  have them too, so Verify warns of each such reference ("only in
  userpatch.ini's haks"). Their own files are not checked as the
  module's custom content. Changing it reads the game's data again.
- **Reload haks, override and development when they change**: on by
  default. Moonglow checks every few seconds and rereads what changed
  (see [Build, verify and test](09-build-and-test.md)). The same switch
  has a nasher project's own files read again when another program
  changes them (see [Modules](03-modules.md)).
- **Keep a recovery copy of unsaved work every … minutes** (5 by
  default; see [Modules](03-modules.md)).
- Warnings: **Show reserved Blueprint ResRef namespace warning** (a
  blueprint named like the game's, `nw_` or `x0_` to `x3_`), **Show
  standard resource overwrite warning** (the module adds a resource the
  game has), **Show resource in Hak Pak warning** (a hak has a resource
  the module adds, and the game uses the hak's), **Show duplicate tags
  warning after a paste or a copy** (off by default, as Aurora has none:
  a project's `moonglow.toml` that sets `[tags] paste_warning` is
  followed whatever it says; see [Areas](04-areas.md#duplicate-tags)),
  **Show invalid creature spell assignment warning** and **Show creature
  inventory warning** (see [Blueprints](05-blueprints.md)).

## Script Editor

- **Code Templates Directory**: your own templates, listed with the
  game's.
- **Automatically Compile Scripts on Save**: a script's own Save, and
  saving the module, compile the scripts whose text they save; so is a
  script another program saved in the module's folder or nasher
  project, when it is read again.
- **Generate Debug Information When Compiling Scripts** (`.ndb` files):
  for the script compiled in its editor, and for Compile All Scripts,
  which compiles again a script that has no debug file yet.
- **Check for errors as you type**: on by default. The script is
  compiled a moment after you stop typing, and its first error shows
  under the toolbar, its line underlined (see [Scripts](07-scripts.md)).
  Off, there is no check and no line kept for it.
- **Indent with** Spaces (the default) or Tabs, and **Indent width**
  (4): what Tab puts in, and Shift+Tab takes off, in a script with no
  indented line to go by (one that has some is indented as it is: see
  [Scripts](07-scripts.md)). With spaces, Tab goes to the next multiple
  of the width. The width is also a tab's, for Convert Indentation.
- **Open scripts as tabs of one window**: on by default. A script opens
  as a tab of the window scripts are open in already; off, each in a
  window of its own. (Scripts docked in the main pane open there either
  way.)
- **External Script Editor**: the program the External Editor button
  opens scripts in.
- **Open scripts in the external editor**: a script opened from the
  module tree opens there as well; what it saves comes back into
  Moonglow's editor. In a nasher project the external editor gets the
  project's own file (see [Scripts](07-scripts.md)).
- **…and not in Moonglow's editor**: with the option above, only the
  external editor opens, for a script kept as a file of its own (a
  nasher project's, a module folder's): what it saves is read as files
  changed outside are (General). A script in a `.mod` file still opens
  in Moonglow's editor too, which is how the external editor's saves
  come back; so does one Moonglow holds unsaved text of.
- **External Script Compiler**: a compiler program to use in place of the
  built-in one (empty: the built-in one, which is the game's and
  Aurora's own). Compile in the script editor, Compile All Scripts, the
  build and what is compiled before a test or an export go through it.
  Errors as you type stay the built-in compiler's.
- **Its arguments** (shown once a compiler is chosen): how the program is
  run. Empty, Moonglow uses the usual line for the program, by its name:
  `nwn_script_comp`'s (neverwinter.nim), or `nwnsc`'s. The line shows in
  the empty field, and resting the pointer on it lists the places
  Moonglow fills in: `{files}` (the scripts to compile), `{src}` (a
  scratch folder the module's scripts are written to), `{out}` (the
  folder the compiled scripts are read from), `{game}`, `{user}` and
  `{haks}` (the module's haks, comma-separated: a hak whose path has a
  comma in it can't be passed so, and the compile says which). An option whose place
  has nothing to give is left out. Generate Debug Information adds `-g`
  to the usual lines.
- The editor's **font size**, syntax **colors** and **Background**
  (the theme's unless Custom is ticked), with a preview. **Reset to
  defaults** sets these back; it leaves the indentation alone.

## Conversation Editor

- **Show popup when creating a new text entry**: a new line asks for its
  text first.
- **Open conversations as tabs of one window**: on by default. A
  conversation opens as a tab of the window one is open in already; off,
  each in a window of its own.
- **Show speaker name before text**, and the **NPC** and **Player** text
  colors.
- **Font size**: the size of the conversation editor's text (its lines,
  its buttons, the fields under them, and the lists and right-click
  menus opened from it), apart from the rest of the window; **Default**
  goes back to the window's own.
- **Paste Link Options** and **Drag Link Options**: whether Paste As
  Link and Ctrl + drag link the source to the destination or the reverse.
- **Automatically backup the conversation files** every … minutes.

## Sounds

- **Play placed sound objects in area**, **Play ambient sound in area**,
  **Play ambient music in area**: what plays when an area view opens (the
  area viewer's toolbar also turns each on and off).
- **Ambient music volume**.

## Verify

What **Build › Verify Module** reports, in groups to leave out (a hak's
tileset errors you are not going to fix, say, that hide the rest):
**Missing references**, **Tilesets**, **Models and materials**,
**2DAs**, **Objects' and areas' 2DA rows and variables**, **Talk table
and archives** and **Plugins' checks**. Hover over one for the checks it
holds. A group left out isn't checked at all, so leaving out the
tilesets and models of a module with many haks makes Verify much
quicker. The log's summary line after a Verify names what it left out,
not checked, so a module isn't taken to be clean for what wasn't
looked at. `mg verify --skip` does the same from a terminal.

## Language

The language that text is shown and edited in, for modules made for
players of another language. Names, descriptions and conversation lines
show and edit that language's text, and String Edit opens on it. With
Polish chosen, String Edit also reads and writes a text's English (and
other western) entries as a Polish game shows them, in the Polish
codepage: Polish modules keep Polish names there. Scripts are read and
saved in the chosen language's codepage too (Windows-1250 for Polish):
see [Scripts](07-scripts.md).
Where a text has none in that language, its text in another is shown
instead, English if it has any (as Aurora shows a name written in
English alone): in the palette, the module tree, a conversation's lines
and the journal's lists as it is, and in the fields that edit it (a
blueprint's, an area's, Module Properties', the journal's, a
conversation line's) in the talk table's color with the language named
beside it. Typing there gives it text in the language you edit.

Where your game has that language installed (a folder for it under the
game's `lang`), the game's own text is read in it too: names and
descriptions kept as talk-table references, and the names of races,
classes, feats and the rest. Otherwise they stay in English. Choosing
another language here reads the game's data again, and the open module
is opened again from its file (Moonglow asks first if it has unsaved
changes).

## Keyboard

Every command and its keys, by where it works (anywhere, the area view,
the script editor, the conversation editor, the NUI Creator). Every command of the menus
is listed, in the menus' order, whether it has a key or not: give Build
Module or Verify Module one here. **Find a command** narrows the list to
what you type (a command's name, or its group's).
- **+** then the keys adds a key (Escape: none); a key's **×** removes
  it. A command can have several keys, or none.
- **Reset** gives a command Moonglow's keys back, **Reset All** every
  command.
- **Export…** writes the keys you chose as a file, to keep or to hand to
  someone; **Import…** takes such a file's keys in place of yours (every
  command the file doesn't name goes back to Moonglow's keys). As with
  the rest of Options, OK keeps them.
- **Conflicts:** a key two commands share where both work is named above
  the list. (A key of the whole window wins over the editors' own.)
- **NUI Creator** holds the keys of its Design page: Delete Element
  (Delete), Duplicate Element (Ctrl+D), Move Element Up and Down
  (Alt+Up, Alt+Down), Select Previous and Next Element (Up, Down). See
  [NUI Creator](18-nui-creator.md).
- The commands of enabled [plugins](15-plugins.md) are listed under
  **Plugins**, with the key each plugin suggests; they take keys like
  any other.

Save's key saves the module, but in a script editor that has the
keyboard it is that script's own Save (see [Scripts](07-scripts.md)),
whichever key Save has.

Keys match exactly: Shift+Q is not Q. Text editing, Escape and Enter,
Copy, Cut and Paste, Delete in the area view, and the script editor's
numbered bookmarks (Ctrl and a digit) keep their keys. A letter or digit
given to a command of the whole window (alone or with Shift) stays a
character while a text field has the keyboard; with Ctrl or Alt, and
function keys, work there too.

## Updates

Moonglow's own: Aurora has no such page.

- **Check for updates automatically** (on by default): once a day at
  most, in the background, Moonglow asks GitHub for its latest release,
  and when a newer one is out says so in a line under the toolbar (see
  [Help › Check for Updates…](02-main-window.md#menus)). It never holds
  up starting or the window, and nothing is downloaded or installed.
  Offline, or on any error, it says nothing (a line in the
  [debug log](13-troubleshooting.md#something-fails-without-a-word) at most) and asks again a day
  later.
- **What is sent:** one HTTPS request a day at most, to
  `api.github.com`, for the latest release's version and page, with
  Moonglow's version as its User-Agent (`Moonglow/2.4.0`), as GitHub
  asks of every program. Nothing about you, your computer or your
  modules: no identifier, path or setting. GitHub sees the request as it
  sees any, from your address (or your proxy's, where `HTTPS_PROXY` or
  `ALL_PROXY` names one).
- **Last checked**: when the automatic check last asked.
- **Not reminded about**: the version **Don't remind me about this
  version** put aside; **Remind Me Again** tells of it again. A later
  version is told of anyway.

Where a package manager installed Moonglow (Flatpak, Scoop, or Nix: the
program in `/nix/store`), it updates Moonglow, so the automatic check is
off and the page says how that copy is updated (`flatpak update
io.github.moonglow_toolset.Moonglow`, `scoop update moonglow`, or the
flake or channel Nix took it from). Help › Check for Updates… still
asks, and says the same; in the Flatpak, whose sandbox has no network,
it asks nothing and only says to update it with Flatpak. The AppImage,
the Windows installer and zip, and the macOS app check as described
above.

The command-line tool, `mg`, never checks.

## Where the settings are kept

| System | Folder |
| --- | --- |
| Linux | `~/.local/share/moonglowtoolset` |
| Windows | `%APPDATA%\Moonglow Toolset\data` |
| macOS | `~/Library/Application Support/Moonglow-Toolset` |
