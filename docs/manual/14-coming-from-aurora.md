# Coming from Aurora

Moonglow is meant to feel like Aurora. The palettes, property pages,
fields, wizards and mouse bindings are Aurora's, and what Moonglow writes
is what Aurora writes. A module saved in one toolset opens in the other,
so you can switch back and forth with the same game version, or try
Moonglow on a copy first.

This chapter covers three things for builders who know Aurora:
- which of its familiar problems Moonglow doesn't have;
- what works differently;
- where to find the things that are easy to miss.

## Problems you can leave behind

Each is something Aurora does, then what Moonglow does instead.

- **Windows only.** On Linux and macOS, Aurora runs through Wine, Proton
  or CrossOver, which an update can break. Moonglow runs natively on
  Linux, Windows and macOS.
- **32-bit.** Aurora can run out of memory on big modules and areas, and
  doesn't read haks past 2 GB. Moonglow is 64-bit, and stays quick with
  hundreds of areas, thousands of blueprints and dozens of haks. The game
  itself doesn't read anything past 2 GiB into a hak, so Verify names what
  a hak that size holds past the mark.
- **Graphics crashes.** Aurora's area view uses OpenGL, which some graphics
  drivers and overlays crash. Moonglow's uses Vulkan, Metal or Direct3D 12.
  If the graphics card can't draw, only the area view and previews are
  unavailable; every editor still works.
- **Error boxes that keep coming back** until you end the program. In
  Moonglow, problems are written to the log at the bottom of the window,
  and Moonglow carries on. **Build › Verify Module** finds most of what
  makes Aurora crash in custom content (a tileset's counts, a missing tile
  model, two 2DA rows run together, an object on a 2DA row that isn't
  there) and names the hak, file and row (see
  [Build, verify and test](09-build-and-test.md)).
- **`temp0`.** Aurora unpacks the module into a `temp0` folder, which
  antivirus software and file locks can disturb, and crash recovery from
  it can damage the module. Moonglow keeps the module in memory. Saving
  writes a new file beside the old one, then replaces it, keeping the
  previous version as a `.bak`. Recovery copies of unsaved work are
  separate files, offered back at the next start (see
  [Modules](03-modules.md)).
- **Lost data.** Aurora drops the fields it doesn't know when it saves,
  such as EE's texture and animation replacements, and a talk-table name
  can become plain text. Moonglow saves everything it doesn't edit
  exactly as it was read.
- **EE data with no field.** Texture, animation and shader replacements,
  highlight colors, cursors and text bubbles (an editor's **Visuals**
  page), area flags past interior, underground and natural, a tile's
  replacement texture, a creature's familiar, animal companion, domains
  and school, and stacks past the base item's limit all needed a GFF
  editor or a script. Moonglow has fields for them, each checked against
  what the game reads (see [Blueprints](05-blueprints.md)).
- **One modal dialog at a time**, which can open behind the main window.
  Moonglow's editors are tabs and windows: keep a script, a conversation
  and the area open together, side by side or docked (see
  [The main window](02-main-window.md)).
- **Limited undo.** Aurora's undo has a fixed number of levels, and
  conversation nodes can't be undone. Moonglow's undo is unlimited and
  covers every change in every editor: a field, a node, a tile, a moved
  object. Edit › Undo names what it will undo.
- **The wheel zooms the area** while you scroll a palette. In Moonglow
  the wheel scrolls whatever is under the pointer.
- **Game patches break the toolset**, because it updates with the game.
  Moonglow is released separately, and you choose when to update.
- **Small text and icons** on high-resolution screens, and no dark mode.
  Moonglow follows the system's display scaling and its light or dark
  theme; Ctrl + plus and Ctrl + minus make everything larger or smaller.
- **Testing.** Aurora's F9 has problems the wiki warns about, always
  picks the first character, and needs a restart to see a changed hak.
  Moonglow starts the game the way the wiki recommends, offers **Test Module, Choose
  Character** (Shift+F9) and **Test From Here** (an area's right-click
  menu), and reloads changed haks and 2DAs as you work.
- **Outside editors.** Scripts edited in another editor have to be copied
  into `temp0`. Moonglow's **External Editor** opens the script in your
  editor, and its saves come straight back into the module.

## Habits to change

- **There's no OK or Cancel.** A change applies as you make it. To take
  one back, use Undo (Ctrl+Z), as many times as you need. Closing an
  editor loses nothing.
- **Save writes everything**: the module, and every script you've edited.
  A script's **Compile** button saves and compiles that script. F7
  compiles all of them.
- **Editors open in windows over the area**, so the area stays where it
  is. Drag a window by its tab into the main pane to dock it.
- **The Welcome tab** replaces Aurora's start dialog. Module folders and
  `.mod` files open the same way, so there is no "open module
  directories" option.
- **The camera** also moves with W, A, S and D, and a right drag turns it.
  The rest are Aurora's bindings (see [Areas](04-areas.md)).
- **Settings are Moonglow's own.** Aurora's `nwtoolset.ini` isn't read.
  Moonglow finds a Steam install by itself; otherwise set the game's
  folder once in Tools › Options › Folders.

[Differences from Aurora](12-differences.md) lists every deliberate
departure, and the few Aurora features not done yet.

## Easy to miss

Some of these are in Aurora too and are often overlooked; the rest are
Moonglow's own.

- **Completion in the script editor**: F2 or Ctrl+Space completes
  functions, constants and variables, from `nwscript.nss`, the script and
  its includes.
- **Code navigation**: F12 (or Ctrl+click) goes to a name's definition,
  Shift+F12 lists its uses, and Ctrl+Shift+R renames it everywhere. Errors
  show as you type. `mg lsp` brings the same features to VS Code, Neovim
  and other editors (see [Scripts](07-scripts.md)).
- **Find In Files** searches every script in the module and can replace
  in all of them, including scripts with unsaved edits.
- **Compiler messages**: click one to go to its line, also when the line
  is in an include.
- **Palette search**: the **Find** box above a palette finds blueprints by
  name, resref or tag, and close matches when nothing matches exactly;
  hover for a picture; Favorites and Recent at the top; drag custom
  blueprints between categories.
- **Edit › Find Instance…** lists the objects placed across the whole
  module. Double-click one to go to it.
- **Tile variants**: Shift + right click steps the tile under the pointer
  through its variants.
- **Unused resources**: Build › Build Module can list the resources that
  nothing in the module uses (**Unused** is off by default).
- **Hak conflicts**: Module Properties › Custom Content › Check for
  Conflicts… lists the resources several haks provide (and which one the
  game uses) and the game's resources the haks replace.
- **The resource browser** (Tools › Resource Browser) shows any resource
  in the game, the haks or the module, and where it comes from; models
  open in the model viewer. A 2DA shows its StrRefs as text, which hak
  each row comes from, and what a hak changed.
- **Talk tables**: Tools › Talk Table edits the module's own, and String
  Edit's **Move to Talk Table** puts a text there by StrRef.
- **Keys** can be changed (Tools › Options › Keyboard); they start as
  Aurora's.
- **Area visibility**: **Object Walkmeshes** shows where placeables and
  doors keep creatures out, beside the ground's **Walkmesh**; **Export
  Minimap…** saves an area's map as a PNG.
- **Haks**: Tools › Haks builds a hak from a folder and opens one to add,
  rename, extract and remove files (nwhak's job, without its cut-short
  names); Module Properties › Custom Content › **Add Haks and Talk
  Table…** attaches downloaded content in one step, and hak changes apply
  without reopening the module.
- **Placing and arranging**: snapping to a grid and to angles; Q and E
  to turn, G to drop to the ground; lining up, spacing out and mirroring;
  locking objects out of the way; prefabs (an area's right-click menu,
  and Edit › Prefabs); the pointer's position to the centimeter (see
  [Areas](04-areas.md)).
- **Conversations**: lines name their scripts and journal updates; Test
  follows the conditions as the game does (switch each one TRUE or
  FALSE); Export to plain text, CSV, Twine or Ink, and File › Import
  Conversation… from Twine or Ink (see [Conversations](06-conversations.md)).
- **Bulk edits**: Ctrl+click several custom blueprints to edit them
  together or update all their instances at once (or a whole category's,
  from its right-click menu); saved variable sets; **Edit › Find and
  Replace Text…** (Ctrl+H) across names, descriptions, conversations and
  the journal (see [Blueprints](05-blueprints.md) and
  [Modules](03-modules.md)).
- **Find References and Rename** (the module tree's right-click menu):
  where a script, area, conversation, blueprint or tag is used, and
  renaming a resource everywhere in one step (see [Modules](03-modules.md)).
- **Version control**: **File › Save As nasher Project…** keeps the
  module as text files for git; Moonglow opens and saves them in place
  (see [Modules](03-modules.md)).
- **Command line**: every `mg` command answers in JSON with `--json`,
  and `mg find` and `mg info` answer questions about a module. `mg` packs, unpacks, verifies and compiles from a
  terminal or a build pipeline (see [Command-line tools](11-command-line.md)).

## Going back to Aurora

Nothing Moonglow writes locks you in. A module saved in Moonglow opens in
Aurora, and the reverse. Moonglow writes the same files Aurora would, so
the game sees the same module. Use the same game version in both,
because Aurora updates with the game.
