---
type: Manual Page
title: Options
description: The Options tab page by page (folders and the NWSync upload command, area, general with Open editors (as tabs or as floating windows), the graphics backend, script editor with its tips over names, conversation editor, sounds, verify, language, keyboard, updates with installing them automatically and pre-releases) and where the settings are kept.
tags: [manual, options]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-26T01:48:00Z }
---

# Options

**Tools › Options** opens Options, with Aurora's pages (and Moonglow's
own Updates). It opens as a tab, or as a window of its own with **As
Floating Windows** (General, Editors), and docks and tabs as the other
editors do. Its ten pages are listed at the left, each with its icon;
**OK** and **Cancel** sit at the right end of its header. Where a page
groups its options under headings (Sounds, Verify, Keyboard and Updates
have none), this chapter does too: Aurora's own first in each, in its
order, then Moonglow's. The General page sets its
headings in three columns where Options is wide (Editors to Testing;
Windows and Tabs, Module Tree and Palettes, Appearance; Questions to
Troubleshooting), in two where it is narrower (Editors to Testing, then
the rest), and in one narrower still: no control is ever cut. Interface
size, Graphics backend and the Questions have their labels in a column
beside their controls; where a question's dropdown has no room beside
its label, the label goes over it. **OK** (or **Enter**, with the
pointer over Options and nothing typed in) applies the changes of every
page and writes the settings once; the module stays open. **Cancel**,
closing the tab or Escape drops every page's changes. Nothing changes
before OK, the theme and the interface size included. Only another game
or user folder, the folder aliases, Workshop and userpatch switches or a
language the game has installed (the Language page) has the game's data
read again: unsaved work is asked about first, and the module is opened
again from its file.

## Folders

### Game Installation

- **Neverwinter Nights installation**: the game's folder (with `data/`
  and `bin/` in it). Moonglow reads the game's data from here.
- **NWN user folder (haks, override, modules)**: where your modules, haks, talk tables, override and
  characters are. Empty: `NWN_HOME` (or `NWN_USER_DIRECTORY`) if set,
  else the game's usual folder. Started as `moonglow --user-dir FOLDER`,
  Moonglow uses that folder instead for as long as it runs, and this page
  says so; what you set here is kept for the next start. A shortcut for
  each user folder you test with can name its folder this way. The
  folders the game keeps in it are where its `nwn.ini` puts them, as the
  game finds them: a folder named under `[Alias]` (`HAK=`, `TLK=`,
  `OVERRIDE=`, `DEVELOPMENT=`, `MODULES=`, `PATCH=`…; one relative to the
  user folder within it) is read and written there, haks and talk tables
  kept with the original game beside Enhanced Edition, say, and this page
  lists each such folder under the user folder. `mg` follows them too.
  A Windows path the game wrote under Proton or Wine
  (`C:\users\steamuser\Documents\…`) is read through the Wine prefix
  the user folder is in.
  **Follow nwn.ini's folder aliases**, on by default, can be unchecked
  when that `nwn.ini` is another install's: the user folder's own folders
  are read then (`mg --no-aliases`). A folder
  typed with a leading `~` (`~/.local/share/Neverwinter Nights`) is your
  home folder's, as a shell takes it; one that doesn't exist is said under
  the field and in the log as the game data loads, as nothing is found
  there.

### Game Content

- **Read Steam Workshop content**: off by default, as in Aurora: an item
  you subscribed to for play is not your module's, and a module that
  needs it works only for players who have it too. On, the Steam
  Workshop items you are subscribed to are read with the game's data, as
  the game started through Steam reads them: each item's `override`
  under your own, its haks and talk tables found by name, its
  `portraits`, `ambient` and `music` under your own and over the
  game's (an item's module is opened as any module, by File › Open).
  Changing it reads the game's data again. `mg --workshop` reads them
  too. It also decides how Test Module starts a Steam copy of the game:
  through Steam where it is on, and without Steam's app ID where it is
  off, so that the test and Moonglow's views agree (the game through
  Steam loads the items you subscribe to, while Moonglow reads every item
  Steam has downloaded, so an item unsubscribed but still on disk is in
  the view alone; see [Build, verify and test](09-build-and-test.md)).
- **Read userpatch.ini's haks**: off by default, as in Aurora, for the
  same reason. On, the haks your
  `userpatch.ini` names (`PatchFile000=…` and on, the highest number
  winning), from the `patch` folder in your user folder (or the folder
  `nwn.ini` names as `PATCH`), are read under the module and its haks
  and over `override`, as the game reads them (measured in the game's
  server): the HD Pack and the like show in the area view and the
  previews. Aurora doesn't read them; `mg --userpatch` does. Their own
  files are not checked as the module's custom content. Changing it
  reads the game's data again.

  What only your own content provides, a player has only if they have
  the same: so Verify warns of each reference only userpatch.ini's
  haks, your `override` folder or a Workshop item satisfies ("only in
  userpatch.ini's haks", "only in your override folder", "only in a
  Steam Workshop item"), and of a custom talk table found first there
  (the game reads one from there too, and a player without it can't
  load the module). Content you keep in `override` on purpose need not
  fill Verify's report: leave it out in [Verify](#verify)'s **Also
  leave out**. Music and ambient sounds in these haks are read as any of
  their resources, as the game reads them.
- **Reload haks, override and development when they change**: on by
  default. Moonglow is told by the system when they change (it checks
  every few seconds where it can't be) and rereads what changed
  (see [Build, verify and test](09-build-and-test.md)). The same switch
  has a nasher project's own files, a module folder's and a `.mod` read
  again when another program changes them (see [Modules](03-modules.md)),
  and the tilesets open in the tileset editor (see [Areas](04-areas.md)).

### Sending Files

- **Scratch folder (To Scratch)**: where **To Scratch** (the script
  editor, the area header's More) and **Export › Copy to Scratch Folder** (the module tree) copy a script
  with its compiled script, or an area, as loose files. Empty: asked for
  the first time. See [Modules](03-modules.md).
- **NWSync upload command**: what **Upload** in Build › Publish to
  NWSync runs to send the repository to your web server, once for each
  part (`{part}`: `data`, then `manifests`, then `latest`), in the
  repository's folder (`{folder}`); **It reads what I type** gives it a
  line under its output, as a project's command with `input = true`.
  A project's `[nwsync] upload` is used in its place. See [Uploading to
  the web server](09-build-and-test.md#uploading-to-the-web-server).

## Area

### View

- **Background Color** of the area viewer: with **Custom** checked, the
  color you choose; otherwise the area's fog color.
- **Show Encounter Spawnpoint Markers**, and their **Height** and
  **Width** (in tenths of a meter: 12 and 4 at first): at each spawn point,
  a 3D wireframe prism of that size and an arrow the way what spawns
  there faces.
- **Show Door Orientation Arrows**.
- **Show stores as $ signs**: the game's marker for a store, rather
  than an arrow along its facing.
- **Show the turning and tilt rings around selected objects**: the ring
  that turns the selection when dragged round, and the two that tilt
  its models, with the arrows that move it along one axis, while Shift
  is held (see [Areas](04-areas.md)). On by
  default.
- **Middle drag moves the view** (off by default): a middle drag moves
  the camera, as Ctrl + drag does, and Alt + middle drag turns it: the
  two swapped (see [the camera](04-areas.md#the-camera)). A right drag
  turns it either way.

### Graphics

- **Shadows quality**: how the area view draws the game's shadows (its **Shadows**; also chosen in the area header's Lighting popover, see
  [Areas](04-areas.md#lighting)). **Fast** (the default): a shadow map stands in
  for the game's shadows, cheap in every area; in a large area seen out
  to its far side, its edges up close are a little soft and stepped.
  **High**: stencil shadow volumes, the game's own kind: each mesh
  flagged `shadow` extruded away from the sun (or a creature's light),
  edges as sharp as the picture's and smoothed with it, as the game
  draws them at its highest shadow setting. High costs more where many
  tiles cast at once (the largest areas seen whole) and keeps a depth
  and stencil buffer of the view's size (about 160 MB at 3440×1440 with
  four samples a pixel). Skins (a robe's or a dragon's wings' skinned
  meshes) cast none with High, as the wiki says of the game, where Fast casts them.
  A model's High shadow is made beside the frames the first time it
  casts, so an area opens without waiting for its shadows: they show a
  few frames later (about 30 ms for the largest stock area).
- **Largest textures**: for a GPU with little memory (an integrated one,
  or a card of 2 to 4 GB). **As they are** (the default) draws every
  texture at its own size; **1024**, **512** or **256 pixels** draws a
  larger one at that size, its larger levels left out as the game's
  lower texture quality leaves them (the textures are read again when
  this changes). Each halving takes a quarter of the memory: The
  Domains of Dread's woods, whose textures are mostly 1024 pixels, take
  870 MB of textures as they are and 240 MB at 512; the stock game's
  are mostly 512 pixels or less. Textures not drawn for a while (those
  of areas closed, of pictures scrolled past) are let go of once they
  take more than a gigabyte (a quarter of one on an integrated GPU),
  and read again when drawn again. TGA pictures of 64 pixels or more
  are compressed as they are read, as the game's DDS textures already
  are, in a quarter to an eighth of the memory: Tyrants of the
  Moonsea's area092 takes 21 MB of textures instead of 31. Kept as
  stored: the colored copies of PLTs (armor, clothes, bodies), whose
  sharp edges between colors compression would blur, so an area full
  of creatures in their own colors takes as much as before; normal,
  specular and roughness maps; and particles' pictures and textures
  drawn added to what is behind them, where compression's slight tints
  would add up to visible blotches.
- **Area view resolution**: for a GPU that draws the areas slowly (an
  integrated one). **Full** (the default) draws the area views' pictures
  at every pixel of the screen; **75%** or **50%** at that share of them
  each way, stretched to the view and softer for it, while what is
  drawn over them (outlines, handles, Sound Ranges' rings, the map)
  stays sharp. A picture takes about as long as its pixels: on the
  integrated GPU of a Ryzen 9 9950X, Tyrants of the Moonsea's largest
  area turning took 80 ms a picture at Full, 45 at 75% and 20 at 50%,
  and the view's targets a quarter of the memory at 50%.
- **Anti-aliasing**: for a slow GPU, or one with little memory.
  **4×** (the default) smooths the edges of the 3D views (the area
  views, the model viewer and the previews) with four samples a pixel;
  **2×** with two, and **Off** not at all. Only what the GPU can draw is
  offered (OpenGL draws Off only). On Tyrants of the Moonsea's area092
  at 3440×1400, Off took 192 MB less a view than 4× and a tenth less
  time a picture on the integrated GPU of a Ryzen 9 9950X (27 ms
  against 30); 2× sits between them.
- **Keep animating behind other windows** (off by default): the area
  views and the model viewer go on playing their animations while
  another program's window has the focus, for watching an area on one
  screen while working in another program. Off, they hold still until
  Moonglow's window has the focus again, which spares the GPU.

### Editing

- **Lights of any color and reach, written into a hak** (off by
  default): Moonglow's own. On, the area's **Add Light Here** and the
  palette's **Lights** offer **Custom Light…** (see
  [Areas](04-areas.md#lights-of-any-color-and-reach)), which writes a
  light's model and a placeables.2da row into a hak you choose, one
  the module's players must have. Off, nothing is ever written into a
  hak for a light, and nothing of it shows but this option. A project's
  `moonglow.toml` can say it for the team (`[lights] generated`), which
  wins over this.
- **Allow editing tiles of groups and features** (off by default):
  Moonglow's own. Off, a group or feature (a barn, a gate, a statue) is
  one thing: Next and Previous Tile Appearance, Rotate Tile, Refine
  Tile, Shift + click and painting terrain leave its tiles as they are
  (the Status Bar says why, and its tiles flash red), Tile Properties
  offers it no variants, no group goes over part of another (as Aurora
  refuses it), and the Eraser and Delete take it away whole. On, each
  of its tiles can be changed alone, a group placed over another takes
  that one away whole, and Delete takes a group's tile alone, as
  Aurora's does; the page and the Status Bar warn, since a group left
  half changed is hard to see and to put right later (Undo does). See
  [Areas](04-areas.md#painting-terrain).

### Header

Which controls the area's header shows, on its row, in its popovers and
in the wide row where they unfold: a check box for each (the objects'
filters as **Show Creatures**…, the switch that plays placed sounds as
**Play Sounds**, both walkmeshes as **Walkmesh…**; **Select Tiles** and
**Paint** are the view's tool shelf's, **Reorient Camera** and **Go to
Start Location** the buttons under its compass), all of them shown at
first; **Show All** puts back every one. A right click anywhere on the
header opens the same list. Moonglow's own (see
[Areas](04-areas.md#the-header)).

## General

### Editors

- **Open editors**: **As Tabs** or **As Floating Windows**. Where an
  editor (a script, a conversation, a blueprint's or a placed object's
  Properties, Module Properties, the Faction and Journal Editors, this
  manual, Options, a plugin's panel) opens when nothing of its kind has a
  place kept: as a tab of the focused editor group, or as a window of its
  own over the area. Areas always open as tabs. A new installation starts
  with tabs; a settings file that doesn't name this switch (an older
  Moonglow's) keeps floating windows.
  A kind of editor you have moved opens where you last put one, in either
  mode, until **View › Reset Layout** (see [The main window](02-main-window.md),
  Tabs and windows). A model's view and Tile Properties open in a window
  of their own in both. With the keyboard on the switch, Left and Right (or Home
  and End) choose; Tab goes on past it.

Under **Floating windows**, the three settings of editors' windows (a
window is one in As Tabs too, for a kind you moved to a window):

- **Open Properties as tabs of one window**: on by default. A
  blueprint's or a placed object's Properties open as a tab of the window
  Properties are open in already; off, each in a window of its own. A
  tab dragged out is a window of its own again, to set two side by side.
  (Scripts have the same on the Script Editor page.)
- **Closing a window cancels its changes (as Aurora)**: off by default
  (its tip: "Closing an editor with OK and Cancel takes back its changes,
  as Cancel does"). A window with **OK** and **Cancel** (a blueprint's or a placed
  object's Properties, several edited together, Module and Area
  Properties, a conversation, the journal, the factions, the raw fields
  view, a NUI window) closed any way but OK (its tab's close button, the tab's
  **Close**, **Close Others**, **Close Tabs to the Right** or **Close
  All**, Ctrl+W) does what its Cancel does, as Aurora's close button
  does: what was changed in it is set back, as one step that Undo takes
  back, and Edit Copy's or Add to Palette's new blueprint closed so is
  not made. A window with nothing changed just closes. Off, closing it
  keeps the changes, as OK does. Windows closed because the module
  closes, or because what they show was deleted, are only closed; the
  2DA, hak, tileset, talk table and script editors ask about their
  unsaved changes either way.
- **Move windows by dragging anywhere in them**: off by default. A
  window of tabs (Properties, conversations, scripts…) moves by its tab
  bar alone, so a click that misses a control doesn't move it; on, any
  part of it that isn't a control moves it too.

### Saving and Exporting

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
- **Keep a recovery copy of unsaved work every … minutes** (5 by
  default; see [Modules](03-modules.md)).

### Opening Modules

- **Open a module on the area opened last**: on by default; the first
  time, the first area the module tree lists (see
  [Modules](03-modules.md)).
- **Open a module with the tabs it had when it was closed**: on by
  default. The areas, scripts, blueprints, conversations, side editors
  (a hak, a 2DA, a tileset) and tools open when a module was closed open
  again with it, in their order and places (windows split and where
  they were), the one in front in front, each view as it was left (an
  area's camera, a model view's, a script's scroll, cursor and
  bookmarks, a conversation's selection, bookmarks and folds, the talk
  table's line, a Properties window's page, a hak's selection, a 2DA's
  cell, a tileset's page and tile, Advanced Search's conditions); kept in
  Moonglow's data folder, not in the module (see
  [Modules](03-modules.md)). With none kept, the module opens
  on the area opened last, as above. Off, a module's tabs are neither
  kept nor opened again.

### Testing

- **Minimize Toolset on test module**. Whether a test starts a Steam
  copy of the game through Steam follows **Read Steam Workshop content**
  on the Folders page (5.1.0 retired the separate option that was here).

### Windows and Tabs

- **Ctrl+Tab goes to the tab used last** (off): Next Tab and Previous Tab go
  through a pane's tabs in the order they were last used, for as long
  as Ctrl is held (one press: back to the tab before); off, to the next
  tab as they lie. Each pane and window keeps its own order: a tab that
  comes to the front of a window while you work elsewhere (a script
  opened from a field's Edit) counts as used there.

### Module Tree and Palettes

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
- **List areas and blueprints by name** in the module tree, rather than
  by ResRef, and **Show ResRefs beside names**: in the tree the ResRef
  dimmed after the name, in the palettes `Name (resref)`. Areas are by
  name in Find Instance's In Area and the area transition's destinations
  too.
- **Show tags beside names in the module tree** (off): a blueprint's tag
  dimmed after what it is listed by (after its dimmed ResRef where both
  show, a dot between them), and a placed object's dimmed after its
  name, as the Outliner shows it, a waypoint's too (otherwise listed by
  its tag alone, as Aurora lists them), waypoints then in their names'
  order. The Filter then finds blueprints by their tags too.
- **Show file extensions in the module tree** (off; its tip: "Show
  guard.utc rather than guard for blueprints and other files"): blueprints, NUI
  windows, the journal and the factions listed by their files' names
  (`guard.utc`); off, by their ResRefs alone (`guard`), but where the
  Journal and Factions group holds two files
  of one ResRef, which keep their extensions (a module's own are
  `module.jrl` and `repute.fac`, which don't share one: only a file put
  in by hand, a `repute.jrl` beside `repute.fac`, would).
  Areas, conversations and scripts are listed by their ResRefs either
  way, as in Aurora. The Filter finds a resource by its file's name,
  extension and all (`.utc`, `guard.utc`), whichever is shown.

  The tree's rows under the three, one rule: a row says the name where
  listed by name (else the ResRef, with its extension where shown), and
  dims after it the ResRef (where shown and the row says a name) and the
  tag (where shown); what doesn't fit the dock is cut with an ellipsis,
  and the pointer resting on the row shows the whole: the name, the
  file's name with its extension, and the tag. A screen reader hears the
  whole on one line, `Hent Fynolds (creature007) (GUARD_HENT)`, a row
  listed by its ResRef by its file's name (`creature007.utc`).
- **Hide empty groups in the module tree** (off): the tree's groups with
  nothing in them (Scripts (0)) and the kinds of object an area has none
  of (Doors (0)) are left out; off, they are listed, as Aurora lists
  them. A hidden group's New is in the Wizards and Tools menus.
- **Hide empty categories in the palettes** (off): the categories with
  no blueprint in them (or in the categories under them) are left out of
  the palettes, the Custom palette's too; a search still shows what it
  finds. A blueprint's Category list and Palette › Categories have
  every category either way. Off, every category is listed, as Aurora
  lists them.
- **List a blueprint's categories alphabetically**: a blueprint's
  Category list in alphabetical order, rather than in the palette's
  (the order Palette › Categories gives them): case and accents aside
  (`Árboles` beside `arboles`, not after `z`), and a group's own
  categories (`Oficios / Herrero`) together under it.
- **Show challenge ratings in the creature palette**: on by default.
- **Show armor and cloaks on a woman**: an armor's or a cloak's model on
  a woman's body, and an armor's inventory icon as a woman's inventory
  shows it (`ipf_` parts; a cloak's is the man's, the game having no
  other), in inventories, a store's pages, the palette's pictures and
  Preview and the item's Properties, whose Appearance page's **Shown
  on** changes it too. Off by default: a man's. A creature's own
  Inventory page shows the icons of its Gender, as the game does.
- **Set a creature's portrait with its appearance** (off): an
  appearance chosen brings the portrait its appearance.2da row names, in
  the same step ([Blueprints](05-blueprints.md)).

### Appearance

- **Interface size**: the whole interface, text and all, from 90% to 300%
  of its usual size (250% and 300% for a 4K screen at its usual scale).
  **View › Interface Size** sets it too, from the menu (the size in use
  checked), and so do its **Zoom In** (Ctrl+Plus or Ctrl+=), **Zoom Out**
  (Ctrl+Minus) and **Reset Zoom** (Ctrl+0, back to 100%), which are in
  the Command Palette as well: those set it at once, without OK, and the
  row here follows them. A size at which the window's least size would
  not fit the screen is not offered in the menu or here (a larger size
  saved for another screen stays as saved), and is used as the
  largest that fits (the setting is kept for a larger screen). A size
  that Ctrl with + and - made before 5.2, without this setting, becomes
  its nearest size here the first time 5.2 starts.
- **Light theme**: the suite's light theme (light gray panels and bars,
  dark text, the same blue for what is chosen) rather than its dark one,
  which is the default. It changes when OK closes Options,
  in every window, as Interface size does (Cancel leaves it as it was). What a 3D view, a model preview or a NUI window shows (the
  game's own look) is the same in both, and so are the script editor's
  colors where a Background of your own is set: a dark Background takes
  the dark theme's colors and a light one the light theme's, so its text
  stays readable whichever theme the window wears.
- **Smooth scrolling** (on): a mouse wheel's notch eases the list or the
  page over a moment, and so does a jump (Go to Line, a search result or
  a problem gone to, a row brought into sight). Off, both land at once:
  each notch moves its whole step in the frame it comes. A touchpad
  scrolls as it does either way (the system gives it its own glide), as
  does a wheel that turns in fine steps (a high-resolution wheel), and
  Ctrl with the wheel still zooms; a 3D view's wheel zoom, which follows
  the wheel's scrolling, arrives at once too.
- **Colored icons** (off): each kind's icon in a color of its own in the
  module tree, the Outliner, the palette's row of types, the Resources
  panel, Quick Open and the hak editor's list, so that the kinds tell
  apart at a glance: creatures red, encounters orange, areas
  tan, sounds yellow, stores lime, triggers green, waypoints cyan, doors
  blue, scripts indigo, placeables violet, items magenta and
  conversations pink (in the area view, encounters, triggers and sounds
  are drawn in the same families). The rest (folders, files, the journal,
  NUI windows, models, textures, commands, tools) stay gray. A selected
  row and the palette's type that is chosen keep their white icon, and a
  dimmed row (an empty kind, a kind hidden in the area, an overridden
  copy in the Resources panel) its gray. The colors are as clear in
  the light theme as in the dark one. It changes when OK closes Options,
  without a restart. Off, every icon is the one gray, as before.
- **Graphics backend** (Automatic): what the window, the area view and
  the previews draw with. **Automatic** is the graphics library's
  (wgpu's) choice, except that an AMD GPU on Windows draws through
  Direct3D 12 rather than Vulkan, and it is **checked as Moonglow
  starts**: before the window opens, a small scene is drawn offscreen
  through Moonglow's own pipelines and compared with what it must look
  like (under a tenth of a second here). Where it draws nothing, the same
  GPU's other backend is checked and taken, and the backend that failed
  is remembered: Automatic passes over it on this GPU until its driver
  changes, or Moonglow is updated. Below the choice, the page lists the
  backends found drawing wrong, with what found it and when: those
  Automatic passes over, and those it draws with all the same where no
  other backend is left (one backend on the platform, or every one
  found wrong); **Try Again** forgets them (when you choose OK). It lists
  too where the notice that the game's shaders draw wrong was hidden
  (**Don't Show Again**); **Show Again** shows it from the next start.
  See
  [Troubleshooting](13-troubleshooting.md#an-area-draws-nothing-or-loses-its-tiles-on-an-amd-gpu-under-windows).
  Or choose one: **Vulkan** or **Direct3D 12** on Windows, **Vulkan**
  on Linux, **Metal** on macOS (the only one there); only the
  platform's are offered. A backend chosen here is kept even where the
  check finds it drawing nothing: a notice under the Top Bar says so and
  offers the other. (Not OpenGL, which Moonglow draws with only where
  nothing else is there: with one sample a pixel, no anti-aliasing.) It takes effect the next time
  Moonglow starts, and the page says what this run draws with. A backend
  with no adapter on this machine falls back to Automatic, and the log
  says so. The `WGPU_BACKEND` environment variable (`vulkan`, `dx12`,
  `metal`), when set, decides over this setting (and is kept as the
  setting is): a way out if the one chosen doesn't start.

### Questions

What Moonglow asks, answered here once for all; each question's own
**Always do this** box (or **Don't ask again**) sets its choice here too.
The questions that would throw work away if answered once for all
(unsaved changes as a window closes, and Changed Outside Moonglow, for
files another program changed while you had unsaved edits of them)
always ask.

- **A .mod with its folder beside it** (Ask which to open): a `.mod`
  opened with a folder of its name beside it (`mymodule.mod` and
  `mymodule/`, a module directory as Aurora keeps one) asks whether to
  open the folder or the `.mod`, as Aurora asks; **Open the folder**
  opens the folder without asking (Aurora's Always open module
  directories), **Open the .mod** the `.mod`. See
  [Modules](03-modules.md#opening-and-saving).
- **Exporting to an archive there** (Ask: append or overwrite): File ›
  Export to an `.erf` that is there already asks whether to append to
  it or overwrite it, as Aurora asks; **Append to it** adds the
  resources without asking (those of the same names replaced, the rest
  it holds kept), **Overwrite it** replaces it with an archive of the
  resources alone. The question's **Always do this** sets it too. See
  [Modules](03-modules.md#import-and-export).
- **Files there already** (Ask: replace or skip them): files of the same
  names where Moonglow writes: files added to a hak (the hak editor's
  Add Files), files a hak is extracted into, the resource browser's
  **Export as Files**, a model's **Export Files**, conversations or
  minimaps exported into a folder, NUI windows imported, the tileset
  editor's **Make Palette** and **Render Minimap Pictures**. Asked each time
  (**Replace 3**, with the number of files, **Skip Those**, Cancel), or
  always replaced, or always
  left as they are, the others written.
- **Equipping without the feats** (Ask, as Aurora): equipping a creature
  with an item it lacks the feats for, on its Inventory page: asked, as
  Aurora asks (Yes adds the first feat it lacks and equips it, No leaves
  it unequipped); or always the one or the other.
- **Ask before deleting from the module** (on): Delete… in the module
  tree asks first. Unchecked, what nothing else in the module names is
  deleted at once (Undo brings it back); what something names is still
  asked about, the window saying what names it.
- **Ask before the window is closed**: on by default. The window's
  close button asks before Moonglow closes where nothing is unsaved (with
  unsaved changes it asks about those, as always); File › Exit closes
  without this question.

### Warnings

- **Show reserved Blueprint ResRef namespace warning** (a
  blueprint named like the game's, `nw_` or `x0_` to `x3_`), **Show
  standard resource overwrite warning** (the module adds a resource the
  game has), **Show resource in Hak Pak warning** (a hak has a resource
  the module adds, and the game uses the hak's), **Show duplicate tags
  warning after a paste or a copy** (off by default, as Aurora has none:
  a project's `moonglow.toml` that sets `[tags] paste_warning` is
  followed whatever it says; see [Areas](04-areas.md#duplicate-tags)),
  **Show invalid creature spell assignment warning** and **Show creature
  inventory warning** (see [Blueprints](05-blueprints.md)).

### Troubleshooting

- **Write a debug log**: what Moonglow does, step by step, in
  the `logs` folder of its data folder, a file for each start; on
  unless you switch it off (see
  [Troubleshooting](13-troubleshooting.md)).

## Script Editor

### Compiling

- **Automatically Compile Scripts on Save**: a script's own Save, and
  saving the module, compile the scripts whose text they save; so is a
  script another program saved in the module's folder or nasher
  project, when it is read again.
- **Generate Debug Information When Compiling Scripts** (`.ndb` files):
  for the script compiled in its editor, and for Compile All Scripts,
  which compiles again a script that has no debug file yet.
- **Check for errors as you type**: on by default. The script is
  compiled a moment after you stop typing, and its first error shows
  in the Status Bar, whole, and at the head of Compiler, its line
  underlined (see [Scripts](07-scripts.md)).
  Off, there is no check and no line kept for it.
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
  scratch folder the scripts to compile and the module's scripts they
  include, however indirectly, are written to), `{out}` (the
  folder the compiled scripts are read from), `{game}`, `{user}` and
  `{haks}` (the module's haks, comma-separated: a hak whose path has a
  comma in it can't be passed so, and the compile says which). An option whose place
  has nothing to give is left out. Generate Debug Information adds `-g`
  to the usual lines.

### Editing

- **Code Templates Directory**: your own templates, listed with the
  game's.
- **Indent with** Spaces (the default) or Tabs, and **Indent width**
  (4): what Tab puts in, and Shift+Tab takes off, in a script with no
  indented line to go by (one that has some is indented as it is: see
  [Scripts](07-scripts.md)). With spaces, Tab goes to the next multiple
  of the width. The width is also a tab's, for Convert Indentation.
- **Open scripts as tabs of one window**: on by default. A script opens
  as a tab of the window scripts are open in already; off, each in a
  window of its own. (Scripts docked in the main pane open there either
  way.)
- **Show declarations in tips**: on by default. Resting the pointer on
  the name of a function or constant, or moving the cursor onto one
  with the arrow keys, shows a tip with its declaration and the first
  line of its documentation (see [Scripts](07-scripts.md)). Off, neither
  tip shows; the completion list's detail stays.

### External Editor

- **External Script Editor**: the program the External Editor button
  opens scripts in.
- **Open scripts in the external editor**: a script opened from the
  module tree opens there as well, and so does one opened by the script
  editor's Open…, made by New Script or Save As, opened by the Script
  Wizard or by the NUI Creator's handlers and opener script, or by Edit
  beside a script's name; what it saves comes back into Moonglow's
  editor. Going to a line (a compiler message, Definition, References,
  a search result) stays in Moonglow's editor. In a nasher project the
  external editor gets the project's own file (see
  [Scripts](07-scripts.md)).
- **…and not in Moonglow's editor**: with the option above, only the
  external editor opens, for a script kept as a file of its own (a
  nasher project's, a module folder's): what it saves is read as files
  changed outside are (Folders). A script in a `.mod` file still opens
  in Moonglow's editor too, which is how the external editor's saves
  come back; so does one Moonglow holds unsaved text of, and a new one
  (New Script, Save As) until File › Save writes its file.

### Font and Colors

- The editor's **Font size** (its lines are spaced with it: 18 points
  apart at the usual 13), a color for each kind of text (comments,
  keywords, strings…) and **Background**, each the theme's unless its
  **Custom** is checked (colors not chosen follow the Background's
  lightness), with a preview. **Reset to Defaults** sets these back; it
  leaves the indentation alone.

## Conversation Editor

### Editing

- **Show popup when creating a new text entry**: a new line asks for its
  text first.
- **Show speaker name before text**.
- **Open conversations as tabs of one window**: on by default. A
  conversation opens as a tab of the window one is open in already; off,
  each in a window of its own.
- **Automatically backup the conversation files** every … minutes.

### Text

- **Font size**: the size of the conversation editor's text (its lines,
  its buttons, the fields under them, and the lists and right-click
  menus opened from it), apart from the rest of the window; **Default**
  goes back to the window's own.
- **NPC Text Color** and **Player Text Color**: by default red and blue,
  darker in the light theme, where a link's lines are a darker gray too,
  so that they stand out against white as they do against the dark
  theme's black; **Default** beside a color you chose goes back to it.

### Links

- **Paste Link Options** and **Drag Link Options**: whether Paste As
  Link and Ctrl + drag link the source to the destination or the reverse.

## Sounds

- **Play placed sound objects in area**, **Play ambient sound in area**,
  **Play ambient music in area**: what plays when an area view opens (the area header's Audio popover also turns each on and off).
- **Ambient music volume**.

## Verify

What **Build › Verify Module** reports (in the Problems panel and the
log), in groups to leave out (a hak's
tileset errors you are not going to fix, say, that hide the rest):
**Missing references**, **Tilesets**, **Models and materials**,
**2DAs**, **Objects' and areas' 2DA rows and variables**, **Talk table
and archives** and **Plugins' checks**. The arrow beside one lists the
checks it holds, each to leave out alone (by the id `mg checks` gives
it: hover over one for what it holds to be true).
Under **Missing references** are the categories of Build Module's
Missing Resources (the references the module's creatures make, its
doors'…), and under **Plugins' checks** each enabled plugin's checks.
A check you left out of a plugin that is not enabled now stays listed
there too, so you can check it again.
**Also leave out** takes haks (`tdod_tiles01.hak`), the `override` or
`development` folder, resources (a tileset, `ttr01.set`, with its
tiles' models; a 2DA or a model) and what only your own content has
(`references:override`, `references:userpatch`, `references:workshop`,
or all three as `references:local`: see [Game
Content](#game-content)), separated by commas. A word that is none of
these is said under the field and not kept. What you typed there is
shown there again the next time Options opens, so you can take it out.
What is left out isn't checked at all, so leaving out the tilesets and
models of a module with many haks makes Verify much quicker. The log's summary line
after a Verify names what it left out, not checked, so a module isn't
taken to be clean for what wasn't looked at. `mg verify --skip` does
the same from a terminal.

## Language

The language that text is shown and edited in, for modules made for
players of another language: **Use Default Language: English**, or
**Specified Language:** and one of the list (English, French, German,
Italian, Spanish, Polish). Names, descriptions and conversation lines
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

**Talk Table › Put text typed into the talk table** (off by default):
a name, a description or a conversation line typed becomes a line of the
module's custom talk table, at the next free line of the project's
lines, and the field names it by its StrRef; off, the text is the
field's own, as Aurora writes it. **When the talk table has the text
already**: **Ask** (the default: Use Line N, Add a Line or Keep in the
File), **Use that line**, **Add a line**, or **Keep it in the file**; the
question's **Always do this** sets it too. A project's `moonglow.toml`
with `[talk] typed` turns it on (or off) for everyone who opens its
module: the switch shows the project's word, grayed, with a line saying
so, and yours is kept for other modules. See [Text typed into the talk
table](03-modules.md#text-typed-into-the-talk-table).

Where your game has that language installed (a folder for it under the
game's `lang`), the game's own text is read in it too: names and
descriptions kept as talk-table references, and the names of races,
classes, feats and the rest. Otherwise they stay in English. Choosing
another language here reads the game's data again, and the open module
is opened again from its file (Moonglow asks first if it has unsaved
changes).

## Keyboard

Every command and its keys, in groups by where it works: **General**
(anywhere), **Plugins**, **Project Commands**, **Area**, **Script
Editor**, **Conversation Editor**, **Module Tree and Palettes** and
**NUI Creator**. Every command of the menus is listed under General, in
the menus' order, whether it has a key or not: give Build Module or
Verify Module one here. **Find a command** narrows the list to
what you type (a command's name, or its group's).
- **Add a key** (the plus button) then the keys adds a key; while it
  waits for one, every key is the key to add, Enter too (Enter is not
  OK then), and Escape gives up the waiting alone, Options staying open
  with the changes made so far. A key's own button (**Remove Ctrl+S
  from Save**, say) removes it. A command can have several keys, or
  none.
- **Reset** gives a command Moonglow's keys back, **Reset All** every
  command.
- **Export…** writes the keys you chose as a file (a `.json`, named
  `moonglow-keys.json` unless you name it otherwise), to keep or to hand to
  someone; **Import…** takes such a file's keys in place of yours (every
  command the file doesn't name goes back to Moonglow's keys). As with
  the rest of Options, OK keeps them.
- **Conflicts:** a key two commands share where both work is named above
  the list. (A key of the whole window wins over the editors' own.)
- **NUI Creator** holds the keys of its Design page (over its canvas, and
  in its Outliner while it has the arrow keys): Delete Element
  (Delete), Duplicate Element (Ctrl+D), Move Element Up and Down
  (Alt+Up, Alt+Down), Select Previous and Next Element (Up, Down), Fold
  Element or Select Its Parent (Left) and Unfold Element or Select Its
  First Child (Right). See
  [NUI Creator](18-nui-creator.md).
- **Module Tree and Palettes** holds Expand All (Ctrl+Shift+Right) and
  Collapse All (Ctrl+Shift+Left), while the module tree or a palette has
  the keys.
- The commands of enabled [plugins](15-plugins.md) are listed under
  **Plugins**, with the key each plugin suggests; they take keys like
  any other.
- The open project's [commands](03-modules.md#the-projects-commands)
  (`[commands]` in its `moonglow.toml`) are listed under **Project
  Commands**, with the key the project gives each (`key`); the keys you
  choose are kept by the command's name.

Save's key saves the module, but in a script editor that has the
keyboard it is that script's own Save (see [Scripts](07-scripts.md)),
whichever key Save has.

Keys match exactly: Shift+Q is not Q. Text editing, Escape and Enter,
Copy, Cut and Paste, Delete in the area view, and the script editor's
numbered bookmarks (Ctrl and a digit) keep their keys. A key given to a
command of the whole window alone or with Shift (a letter, a digit,
Space, Delete, an arrow) stays the text field's while one has the
keyboard; keys with Ctrl or Alt, the function keys and Escape work there
too. On a Mac, Option with a key that types is the field's as well: there
it is a letter of its own.

## Updates

Moonglow's own: Aurora has no such page.

- **Check for updates automatically** (on by default): at each start
  (and as you switch it on) and once a day while it runs, in the
  background, Moonglow asks GitHub for its latest release,
  and when a newer one is out opens the Check for Updates window on it,
  then a line under the Top Bar says so (see
  [Help › Check for Updates](02-main-window.md#updates)). It never holds
  up starting or the window. Offline, or on any error, it says nothing
  (a line in the
  [debug log](13-troubleshooting.md#something-fails-without-a-word) at most) and asks again a day
  later.
- **Install updates automatically** (off): a newer release the
  automatic check finds is downloaded and verified in the background,
  and installed when Moonglow closes, so the next start is the new
  version, without the window asking; the line under the Top Bar says
  so and offers **Restart Now**. The window's **Always install updates**
  switches it on. It is grayed where Moonglow doesn't install updates itself
  (see [Installing an update](02-main-window.md#installing-an-update)).
- **Include pre-releases** (off): a newer pre-release (a release
  candidate, to try before the release) is offered too. Releases are
  offered whatever this says.
- What is sent: one HTTPS request at each start and one a day while
  Moonglow runs, to
  `api.github.com`, for the latest release's version, notes and page
  (with Include pre-releases, for the twenty newest releases instead),
  with Moonglow's version as its User-Agent (`Moonglow/3.3.0`), as
  GitHub asks of every program. Nothing about you, your computer or
  your modules: no identifier, path or setting. GitHub sees the request
  as it sees any, from your address (or your proxy's, where
  `HTTPS_PROXY` or `ALL_PROXY` names one). Installing an update also
  downloads the release's package for your copy and its signed
  checksums from `github.com` (which hands the files over from its file
  host), with the same User-Agent and nothing else.
- **Last checked**: when the automatic check last asked.
- **Not reminded about**: the version **Don't Remind Me About This
  Version** put aside; **Remind Me Again** tells of it again. A later
  version is told of anyway.

Where a package manager installed Moonglow (Flatpak, Scoop, or Nix: the
program in `/nix/store`), it updates Moonglow, so the automatic check is
off and the page says how that copy is updated (`flatpak update
io.github.moonglow_toolset.Moonglow`, `scoop update moonglow`, or the
flake or channel Nix took it from). For a Scoop or Nix copy, Help ›
Check for Updates asks when you choose it, and says the same; the page's
note on what is sent names that one request, not one at each start. In
the Flatpak, whose sandbox has no network, Help › Check for Updates asks
nothing and only says to update it with Flatpak; the page says so too,
and describes no request, since nothing is sent from the Flatpak. The
AppImage, the Windows installer and zip, and the macOS app check as
described above, and install updates themselves.

The command-line tool, `mg`, never checks.

## Where the settings are kept

| System | Folder |
| --- | --- |
| Linux | `~/.local/share/moonglowtoolset` |
| Windows | `%APPDATA%\Moonglow Toolset\data` |
| macOS | `~/Library/Application Support/Moonglow-Toolset` |

The settings are the file `app.ron` there. A setting saved by a newer
Moonglow that this one can't read (you went back a version, or run two
side by side) is at its default, and the others are as you left them.
Where a setting, or the whole file, does not read at all, Moonglow
keeps the file as it was beside it before it saves its own
(`app.ron.unreadable-` and the time), and says which settings in the
log: to have them back, close Moonglow and put that file in `app.ron`'s
place, under the version that wrote it.

Two Moonglows open at once share the file. Each saves the options you
changed in it and leaves the others as the other window saved them, so
an option set in one window is not put back by the other; an option
changed in both is as the window that saved last has it, and a window
shows an option changed in the other only when it is started again.
