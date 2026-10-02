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
- **32-bit.** Aurora can run out of memory on big modules and areas.
  Moonglow is 64-bit.
- **Graphics crashes.** Aurora's area view uses OpenGL, which some graphics
  drivers and overlays crash. Moonglow's uses Vulkan, Metal or Direct3D 12.
  If the graphics card can't draw, only the area view and previews are
  unavailable; every editor still works.
- **Error boxes that keep coming back** until you end the program. In
  Moonglow, problems are written to the log at the bottom of the window,
  and Moonglow carries on.
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
- **Find In Files** searches every script in the module and can replace
  in all of them, including scripts with unsaved edits.
- **Compiler messages**: click one to go to its line, also when the line
  is in an include.
- **Palette search**: the **Find** box above a palette filters it by name
  or resref.
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
  open in the model viewer.
- **Command line**: `mg` packs, unpacks, verifies and compiles from a
  terminal or a build pipeline (see [Command-line tools](11-command-line.md)).

## Going back to Aurora

Nothing Moonglow writes locks you in. A module saved in Moonglow opens in
Aurora, and the reverse. Moonglow writes the same files Aurora would, so
the game sees the same module. Use the same game version in both,
because Aurora updates with the game.
