---
type: Manual Page
title: The main window
description: The main window - menus (with the update check and installing an update from the app), Quick Open (go to a resource or a command by its name; the Command Palette is Quick Open among the commands), keyboard shortcuts, the module tree, tabs and windows, the palette, the resource browser, the model viewer, the log, the Status Bar (hints and counts), and screen readers and desktop agents.
tags: [manual, window, shortcuts]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-13T23:49:17Z }
---

# The main window

From top to bottom: the **Top Bar** (the menus, the common
commands and the **Quick Open** field, in one row), then the **Left Dock** with its two panels, each
on a tab of its own (**Module**, the module tree, and **Resources**, the
resource browser), the
**tabs** in the middle and the **Right Dock** on the right, with the
**Palette** and **Preview** panels on two tabs, the **log** in the Bottom Dock under the tabs, and the
**Status Bar** (the log's
switch at its left and, while the log is closed, its last line; in its
middle the **hints** of the tab you work in, each a key or a gesture and
what it does; at its right that tab's counts, a job while one runs, the
module's name, "(modified)" while there are unsaved changes, and the
game's folder: rest the pointer on the name for the module's file, on
the folder for its whole path). See [The Status Bar](#the-status-bar).
The palette comes with each module, in the Right Dock, 320 points wide
at first. The docks follow the tab you work in: the Right Dock holds
the panels of an area, so it is there while an area's tab (or, with no
area open, the tab saying so) is in front of the middle, and not at all
while another tab is (Module Properties, a script, a blueprint, the
manual: the middle then has the room); a tool in a window of its own
leaves it as the area under it has it. Where you
work in two panes side by side, it follows the one you last clicked in.
If you fold the dock away, **Tools › Palettes** brings it back; beside a
tab that brings no panels, it shows the dock all the same until an
area's tab is in front again. With no game data read (no game folder
found), Tools › Palettes still shows the dock, its panel saying "No
game data.".

The Top Bar's buttons are icons: **New**, **Open** and **Save** after
the menus, then **Undo** and **Redo**; in its middle the **Quick Open**
field ("Go to resource or command", with its key, Ctrl+P, at its right
end: see [Quick Open](#quick-open)); at its right end **Compile**,
**Verify** and **Test Module**, with an arrow beside Test Module whose
menu has **Test Module, Choose Character**. Rest the pointer on a button
and its tip names it, with its key. In a window under 1300 points wide
Compile, Verify and Test Module show their icons alone too, and the
Quick Open field is at most 300 points wide (440 in a wider one). As the
window narrows, the field narrows with the room the bar leaves it, then
becomes its icon (a magnifying glass, its key in its tip); narrower
still the build actions go (Compile, Verify and Test Module), then the
Quick Open icon, and last the quick actions (New, Open, Save, Undo and
Redo): each is in the menus as well (File, Edit, Build), and Quick Open
still opens by its key and by Help › Command Palette…, its field then
drawn over the bar's middle. The window can't be made narrower than the menus
(420 by 320 points, which grow with Options › General › Interface size
and follow it as soon as it changes); one left smaller opens at that
size. Where the Status Bar is short of room, its parts give way in
one order (see [The Status Bar](#the-status-bar)). The Top Bar
is the same whatever **Tools › Options › General › Editors' toolbar
buttons** says: that setting is the area view's and the editors'
toolbars'. The icons are Moonglow's own drawings, one to a command. The
toolbar's other buttons of earlier versions have homes of their own:
**Module Properties** (a box) and **New Area** (the Area Wizard, a
plus) are icons on the Module panel's tab bar, beside **Expand All** and
**Collapse All**; **Start** is **View ›
Go to Start Location** (the area view's Go to Start Location and its
Start filter show its circled arrow); **Resources** is the Left Dock's
Resources tab (also Tools › Resource Browser); **Palettes** is the
Right Dock's Palette panel (also Tools › Palettes); **Preview** is the
Right Dock's Preview panel, on the tab beside the palette's (also
**Tools › Preview**). Each kind of object has one icon wherever it appears: on
the area view's toolbar and in the palette's row of types.

## Menus

| Menu | Commands |
| --- | --- |
| File | New Module…, Open Module…, Open Folder…, Recent Modules, Save, Save As…, Save As nasher Project…, Project ([the project's commands](03-modules.md#the-projects-commands): its update and submit), Import…, Import Files…, Import Folder… ([files dropped onto the window](03-modules.md#import-and-export) are imported too), Export…, Import Conversation…, Close, Exit |
| Edit | Undo, Redo (each names the module's step it takes, or, from a hak's or a 2DA's editor, that editor's: see Undo and redo under [Tabs and windows](#tabs-and-windows); with the keyboard in a script's editor they are the script's own and say plain Undo and Redo: see [Scripts](07-scripts.md)), Module Properties, Builder Code… ([your code](03-modules.md#your-builder-code), for a module folder or a nasher project), Claims Table… ([whose each area is](03-modules.md#claimed-areas), in a project that keeps claims), Resize Area…, Rotate Area…, Find Instance…, Prefabs, Edit Areas Together…, Update All Instances… ([every blueprint's](05-blueprints.md#update-instances)), Find and Replace Text…, Find References…, Advanced Search…, Convert Indentation to Tabs, Convert Indentation to Spaces (the script in front: see [Scripts](07-scripts.md)) |
| View | Left Dock (the Module and Resources panels), Right Dock (the Palette and Preview panels), Log (each ticked while shown), Copy Log, Save Log…, Clear Log, Go to Start, Split Editor (see [Tabs and windows](#tabs-and-windows)), Hide All Panels, Reset Layout |
| Wizards | Area Wizard…, Creature Wizard…, and a wizard for each other blueprint type (Door, Encounter, Item, Placeable, Sound, Store, Trigger, Waypoint) |
| Tools | New Conversation…, Faction Editor, Journal Editor, Talk Table, 2DA File…, New Script…, Palettes, Preview, Appearance Gallery…, Resource Browser, Tilesets (New Tileset…, Open Tileset…), Haks (New Hak, Open Hak…, Build Hak from Folder…), Reload Resources, Options… |
| Build | Compile All Scripts, Compile Models, Build Module…, Publish to NWSync…, Verify Module, Round-Trip Check, Test Module, Test Module, Choose Character, Pack *file* (nasher projects), Area Statistics |
| Plugins | the commands of the plugins you have enabled, Manage Plugins…, Install Plugin from File… ([Plugins](15-plugins.md)) |
| Help | User Manual, Command Palette…, Check for Updates…, About Moonglow Toolset |

**The menus by the keyboard.** Alt, pressed and let go on its own, or
F10 opens the first menu; the same closes the menus. (With the pointer
over an area's view, F10 is Select Tiles or Objects, as in Aurora: Alt
opens the menus there.) Left and Right go
from menu to menu, round the ends; Up and Down from row to row, past the
lines and the rows that can't be chosen now; Enter or Space chooses the
row marked. Right on a row with a submenu opens it, Left closes it;
Escape closes the submenu the keys are in, else the menus. With a menu
open, the pointer over another menu's name opens that one. Alt and a
menu's first letter (underlined: Alt+F for File, Alt+V for View) opens
that menu, and with a menu open a letter goes to the next row that
begins with it. A command given Alt and a menu's letter in Options ›
Keyboard has that key instead of the menu; AltGr and a letter (another
layout's character) opens none.

## Quick Open

**Quick Open** (the Top Bar's field, or **Ctrl+P**) goes to a resource
or a command by its name: click the field or press its key, type part
of a name, choose with the arrow keys, and Enter opens or runs it (a
click on a row does too). Under the field a list shows what it finds,
best first, at most 50 at a time; a last line says how many more there
are, and typing more narrows them. In three groups:

- **the module's resources**, by their file's name (`hello.nss`, so
  by their ResRef too) and, for an area or a blueprint, by its name, as
  the Module panel's Filter finds them; the objects placed in an area,
  their names and tags, which the Filter also looks through, are left
  out. One opens as a double-click in the module tree opens it: an
  area's tab, a script's editor, a blueprint's Properties (one already
  open comes to the front of its window); one with no editor (a
  compiled script, a texture) opens read-only, as the Resources panel
  shows it;
- **the commands**: those of the menus, the enabled plugins' commands,
  exporters, importers, wizards and panels, and the open project's
  commands (File › Project), each with its menu and its key. One that
  can't be used now is dimmed, its tip says why, and Enter leaves it;
- **the load order's resources** (the game's, your haks', override's),
  which open read-only, as the Resources panel opens them, each with
  where the game reads it from. One your module has too (a script it
  overrides) is listed once, as the module's.

A last word that names a type narrows the resources to that type:
`.utc` (alone, or after other words) lists the creatures' blueprints,
and `hello nss` finds `hello.nss` and not `hello.ncs`.

Within a group, the resources that open in an editor or a viewer of
their own (a script, an area, a blueprint, a conversation, a model, a
2DA, a text) come before those shown only as their bytes (a compiled
script, a script's debug file, a texture, a sound), each half in this
order: a name equal to what you typed first, then one that begins with
it, then one where it begins a word (`formosa` in `m1q3_formosa_9`),
then one that has it anywhere, then one that has every word you typed
apart (`sword iron` finds "Iron Sword"), and last a command found only
with its menu's name (`mod build` finds Verify Module, in the Build
menu). At each step a match on the name comes before one on the ResRef,
then one on the file's name, then the shorter name, then the
alphabetical order. Case and accents don't count: `arbol` finds
"Árbol".

With nothing typed it offers the areas of this module you opened last,
most recent first (an area of another module needs its module opened:
File › Recent Modules). With no module open, it offers the recent
modules (choosing one opens it, asking first about unsaved work as File ›
Open does) and the commands that work without a module.

**Help › Command Palette…** (Ctrl+Shift+P) is Quick Open among the
commands alone: it opens the field with `>` typed, and whatever follows
a `>` is looked for among the commands only.

In the field, Down and Up move the choice (from the last back to the
first), Page Down and Page Up by a page; Tab stays in the field while
the list is open; **Escape** closes it and gives the keys back to what
had them (an area's view keeps its keys: W moves its camera again; a
field you were typing in takes your typing again). A click outside
closes it without running anything. While you type in it, a key that
types is the field's (a command given a plain letter waits), and Undo
and Redo (Ctrl+Z, Ctrl+Shift+Z) take back your typing, never the
module's last change. Of the commands with keys, those that save,
build, test, show a panel, open a view or a tool, or help run from the
field and close Quick Open (Ctrl+S saves the module, F9 tests it, F1
opens the manual); those that change or close the module, a resource
or a tab wait (Undo, Redo, New, Open and Close Module, Import, the
wizards, New Script, Close Tab and the tabs' and panes' keys, Reload
Resources, Clear Log), as do the plugins' and the project's commands.
A menu opened by its key (Alt, F10) closes Quick Open. While an input
method composes a character, Enter, the arrows and Escape are its.
While a job has the window, Quick Open is dimmed.

Ctrl+P is Quick Open's key unless you gave Ctrl+P to another command
(in Tools › Options › Keyboard, or a plugin or a project gave it to
one of theirs): that command keeps it, and Quick Open is then without a
key (its field shows none) until Ctrl+P is free again, or until you
give Quick Open a key of your own, which its field then shows. Tools ›
Options › Keyboard shows it so before you apply: no conflict, and on
Quick Open's row "No key while … has Ctrl+P". Any command can be given
a key there.

**Help › Check for Updates…** (also a button in Help › About) asks
GitHub for Moonglow's latest release and says either that yours is the
newest or that a newer one is out, with its notes (what is new in it)
and a link to its release page in your browser; drafts never count, and
pre-releases only with Tools › Options › Updates › Include pre-releases.
Offline, or when GitHub refuses (it limits how often it is asked), the
window says why. What is sent is that one request and nothing else (no
identifier, path or anything about your modules). Once a day at most
Moonglow also asks by itself, in the background, unless Tools › Options
› Updates says not to; when a newer release is out, this window opens
on it, with its notes, **Install Now**, **Later** and **Always install
updates** (Options › Updates › Install updates automatically: this
release downloaded at once and installed as Moonglow closes, and later
ones without the window); it opens once for each release while
Moonglow runs, and once
it is closed a line under the Top Bar says so, with **What's New…** (the
window again),
**Install Now**, **Later**, **Don't Remind Me About This Version** (a
later version is told again) and **Close Update Notice** (its close
button) to close it until the next check.

### Installing an update

Where Moonglow updates itself, it installs the new release from the
window or the notice:

- **Install Now** downloads the package for your copy, verifies it, and
  installs it: the AppImage is replaced where it is (the file keeps its
  name and place, so a link to it, such as `~/.local/bin/moonglow` or
  `mg`, still works); the Windows installer, the Windows zip's files and
  the macOS app are installed as Moonglow closes. The window then asks
  **Restart Moonglow now?**: **Restart Now** asks about unsaved work
  first, as Exit does, then closes Moonglow, installs the update and
  starts it again with the module that was open. **Later** leaves it for
  the next start.
- **Later** downloads and verifies the package now and installs it when
  Moonglow closes, so the next start is the new version. If Moonglow
  doesn't close normally (it crashes, or the computer is turned off),
  nothing is installed, and the notice offers the update again.
- The progress shows in the notice and the window. The Windows installer
  runs by itself as Moonglow closes (into the same folder, for the same
  users; under Program Files Windows asks for an administrator) and
  starts Moonglow again when it is done. The zip's files replace those
  of the folder you unpacked it into, and nothing else there is touched.

**What is checked.** Each release publishes the SHA-256 checksums of its
packages in a file, `SHA256SUMS`, signed by the project with a
[minisign](https://jedisct1.github.io/minisign/) key whose public half
is built into Moonglow. Moonglow installs a package only when that
signature verifies, it was made for this release, and the downloaded
file's checksum is the one signed. Otherwise the window says why (for
example, "the release's signature doesn't verify") and links the release
page, with **Try Again** (the download started over), and nothing is installed. A download is written beside what it
replaces, under a name starting `.moonglow-update-`, and renamed only
once it is checked; a broken or interrupted download is removed, and
what one leaves behind is removed at the next start.

With two Moonglow windows open, the one that downloaded the update
installs it as it closes. The other leaves that download alone, and if
asked to install the update too it says that another Moonglow that is
running has it.

**When Moonglow doesn't install it**, the window says why and links the
release page:

- Flatpak, Scoop or Nix installed it: the package manager updates it
  (`flatpak update io.github.moonglow_toolset.Moonglow`, `scoop update
  moonglow`), and Moonglow doesn't ask by itself
  ([Options](10-options.md#updates)).
- A copy built from source, or installed by a distribution's package
  (also when it was started from a terminal or an editor that is itself
  an AppImage: that program's file is not Moonglow's to replace).
- A folder Moonglow can't write to (an AppImage in `/opt`, the app in
  `/Applications` for a user who isn't an administrator): it says so
  before downloading anything. Update it as you installed it.
- A build without the project's key (one built before the key was
  made): it doesn't install updates.
- No cache folder of yours is found to download into (no home folder
  is set, for example): Moonglow never downloads an update into the
  system's shared temporary folder.

## Keyboard shortcuts

On macOS, Cmd takes the place of Ctrl. These are the default keys
(Aurora's, where Aurora has the command). **Tools › Options › Keyboard**
changes them; the menus show the current keys.

| Keys | Command |
| --- | --- |
| Ctrl+N | New module |
| Ctrl+O | Open module |
| Ctrl+S | Save the module; in a script editor that has the keyboard, the script into the module (its own Save: see [Scripts](07-scripts.md)) |
| Ctrl+Z | Undo |
| Ctrl+Y, Ctrl+Shift+Z | Redo |
| Ctrl+H | Find and Replace Text |
| Ctrl+Alt+A | Area Wizard |
| Ctrl+Alt+F | Faction Editor |
| Ctrl+Alt+J | Journal Editor |
| F7 | Compile all scripts |
| F9 | Save and test the module in the game |
| Shift+F9 | Save and test it, choosing the character in the game |
| F1 | User Manual |
| Ctrl+Alt+V | New Conversation |
| Ctrl+Alt+S | New Script |
| Ctrl+Alt+C | Creature Wizard |
| Ctrl+Alt+I | Item Wizard |
| F11 | Full screen |
| Alt+X | Exit (File › Exit, Aurora's key) |
| Ctrl+W, Ctrl+F4 | Close the tab in front (of the editor group with the focus) |
| Ctrl+Tab, Ctrl+Page Down | Next tab (of the group with the focus) |
| Ctrl+Shift+Tab, Ctrl+Page Up | Previous tab |
| Ctrl+1 to Ctrl+9 | The group's tab of that number (Ctrl+9: its last) |
| Ctrl+\ | Split Editor: the tab in front to another editor group (a new one at the right where there is one group) |
| Ctrl+Shift+T | Reopen the tab closed last |
| Ctrl+Shift+M | Maximize the window of the tab in front, or put it back |
| F6, Shift+F6 | The arrow keys to the next pane, left to right and then down (the Left Dock's panel shown, the middle, the Right Dock's panel in front where the dock is drawn, the log), or the one before: to what the pane has (in the Left Dock the module tree's rows or the resource browser's list, at the row they were at, else the first; in the middle what its tab in front has: a talk table's lines, a 2DA's cells, at its first cell if none is selected, else an area's view; a palette at its first row; the Preview's model, which Left and Right then turn about and Up and Down over or under it, as a drag does). In the log (also after a click in it) they scroll it: Up and Down a line, Page Up and Page Down a page, Home and End to its start and end; Escape hands them back |
| Ctrl+Alt+1, Ctrl+Alt+2, Ctrl+Alt+3 | Show or fold away the Left Dock (on whichever panel it shows, Module or Resources; with no module open too, where it shows Resources), the Right Dock (the palettes and the Preview), the log |
| Ctrl+Alt+0 | Hide all three, or show them all |
| Ctrl+P | Quick Open: go to a resource or a command |
| Ctrl+Shift+P | Command Palette (Quick Open among the commands) |

**Ctrl and a digit** goes to the main pane's tab of that number (Ctrl+9:
its last), except while you type in a text field (in the script editor
Ctrl and a digit is a bookmark). These are not in the Options.

**Copy, cut and paste** have two keys each, the usual ones and the
classic ones Aurora and Windows programs also take:

| Keys | Command |
| --- | --- |
| Ctrl+C, Ctrl+Insert | Copy |
| Ctrl+X, Shift+Delete | Cut |
| Ctrl+V, Shift+Insert | Paste |

They work wherever there is something to copy: text fields and the
script editor, objects in the area viewer (with the pointer over it),
conversation lines, items in an inventory, cells in the 2DA editor and
elements of a NUI window. Shift+Delete always cuts, into the clipboard:
where Delete alone deletes (the area viewer's selection), Shift+Delete
does not. These keys are not in the Options either.

**Text fields of several lines** (descriptions, comments, a
conversation line's text, the talk table's strings, String Edit's
texts) take **Page Down** and **Page Up**: the cursor goes down or up by
as many lines as the field shows (less one), and the field scrolls with
it; with Shift, the selection reaches to there. (A NUI window's text
fields, tried in the NUI Creator's preview, don't take them: the game's
don't.) They show plain line ends: a text stored with Windows line ends
(CRLF, as most of the game's own text of several lines is, its
conversation lines and comments) has no hidden character at the end of
each line for the cursor to stop on, and text you paste comes with plain
line ends too. What you type is stored with the text's own line ends:
lines you didn't change keep theirs, and new lines take the kind most of
its lines have; a one-line field keeps those its text has. A text that
had a single line gets the line end Aurora gives a line typed there:
plain in a localized text (a description, a name, a conversation line's
text, a journal entry), as in a talk table's string, and CRLF in a
comment.

The area viewer and the editors have their own keys; their chapters list
them.

## The module tree

The Left Dock's **Module** panel lists what the module holds, by kind:
Areas, Conversations, Scripts, the blueprints (Creatures, Doors,
Encounters, Items, Placeables, Sounds, Stores, Triggers, Waypoints) and
the journal and factions, each group's row with its kind's icon and,
at the right, how many it holds; **Module Properties** is at the top,
and under the groups how many other resources the module has (those no
group lists). (The dock's other tab, **Resources**, is [the resource
browser](#the-resource-browser); where the dock is too narrow for the
tabs' words beside the Module panel's four buttons, the tabs show their
icons alone, a box and a folder, with their names in their tips: the
same on either tab, so a tab stays where it was when you choose the
other. With a tab reached by the keyboard, Left and Right go to the
other tab, as Enter and Space choose it.) With
the Module tab chosen, the tab bar has four buttons: **Module Properties**,
**New Area** (the Area Wizard), **Expand All** and **Collapse All**
(below). **Filter** (the field under the tab bar) narrows every group to the
rows with every word typed in their name or ResRef, in any order, case
and accents aside (`arbol` finds `Árbol`, `sword iron` finds `Iron
Sword`, as every Filter and Find field of names does: the palettes', the
resource browser's, a list's, a wizard's; its clear button, **Clear
Filter**, clears it; each group's count is then of what it finds); it
stays in sight while the tree scrolls, and Home and End go to the
tree's top and bottom with the pointer over it (over the Areas group's
well, to the well's: below). Cleared, the tree and the well are where
they were before anything was typed.

**The Areas group scrolls by itself:** its rows (and the objects of the
areas opened out) are in a well of their own, with a scroll bar of its
own, so the groups under it stay in sight and where they are however
many areas the module has. The well is as high as the tree's view less
a row for each of the tree's other rows (Module Properties, each
group's row and the line under them), in whole rows, however many of
the other groups are open: never fewer than five rows in a low window
(the groups under it are then scrolled to), and no more rows than the
module has areas (three areas, three rows, the next group right under
them). In a window 800 pixels high, with the groups closed, all of them
are in sight under the Areas group. The mouse wheel over the well
scrolls it (at its top or foot, the tree around it), and Home and End
over it go to its top and foot; elsewhere over the tree, the Areas
group closed too, to the tree's. An area or an object the tree goes to (a tab
chosen, an object selected in an area's view, the keys) is scrolled to
in the well, and the well into the tree's view. A name too long for the
pane goes on past its edge, and the tree scrolls sideways to it (Shift
and the mouse wheel, or the scroll bar under it), as Aurora's does; the
well scrolls sideways by itself to a long area name.

Resources are listed by their ResRefs, as in Aurora. With **Tools ›
Options › General › List areas and blueprints by name**, areas and
blueprints are listed by their names instead (in the names' order,
case and accents aside, so `Árboles` stands beside `arboles`, as
polytonic Greek's `Ἀθῆναι` beside `Αθηναι` and Russian's `ёлка` beside
`елка`; one without a name keeps its ResRef), areas' tabs are titled by name, the
filter finds either, and the ResRef shows when the pointer rests on one.
**Show ResRefs beside names** adds it in parentheses, `Name (resref)`,
there and in the palettes: for telling apart resources of one name.
A ResRef you type (a blueprint's, Copy's or Rename's new name, a
script's in its field) takes letters, digits and `_` alone, at most 16,
in lower case, as in Aurora: other characters typed or pasted are left
out (`My Chest!` goes in as `mychest`). A name the game finds that has
others (a hak's `my-tex`, picked with **…**) is kept as it is.
A name is shown in the language you edit (**Options › Language**), else
as the game shows it (its talk-table string), else in another language,
English first; `mg areas` and `mg find` name them the same way.

**An area the module's area list leaves out** (an ARE the module holds
that its `Mod_Area_list` doesn't name, as a copy made outside the toolset
can be) is marked *(not in the area list)*: the game loads only the
areas the list names, so nothing in the game reaches it. Its right-click
menu's **Add to the Module's Area List** lists it (one Undo takes it
away again); `mg areas` and `mg info` say the same of it.

**What is placed in an area:** the arrow before an area opens it out to
its objects, kind by kind as Aurora lists them (Creatures, Doors,
Encounters…; a kind with nothing placed, and a group of the tree with
nothing in it, are left out with Tools › Options › General › **Hide
empty groups in the module tree**), sorted by name (its tag, without one). Waypoints are listed
by tag instead, as Aurora lists them (they often share a name, and their
tags are what scripts and transitions go by); the pointer resting on a
row shows the other. Click an object to go to it in the area's view,
selected (the tree stays still, so a second click makes a double click);
double-click for its Properties; right-click for **Go To**, for
**Adjust Location…** (the object, or those chosen, selected in the
area's view and [Adjust Location](04-areas.md) opened for them, as the
view's own menu has it), for **Preview** (creatures, doors, items and placeables, as placed),
**Copy** (pasted in any area with Ctrl+V, as high above the ground as it
stood, its area's view open or not), **Delete** and **Export as
Blueprint…** (written into an archive as a blueprint of its own, as Add
to Palette makes one, for another module's Import: see
[Modules](03-modules.md#import-and-export)), and last, as
in Aurora, **Variables…** (its variables' window, as Aurora's list has
it) and **Properties**. Ctrl and a click adds an object to those chosen
(or takes it away), Shift and a click chooses those from the one clicked
before to it, as listed; the area's view selects them, and objects
selected together in the view are marked in the tree too. A right click
on one of several chosen acts on them all: **Go To** (each selected in
the view), **Copy**, **Delete** (one step to undo), **Properties** (of
objects of one kind, edited together) and the plugins' commands. The
Filter finds objects by name or tag in every area, opened out or not
(an area that holds one it finds is shown opened out to it), and narrows
an opened area's objects to those it finds. The areas and the kinds it
finds objects in open as it is typed, and close again with a click on
their arrow (or Collapse All), staying closed until what is typed
changes.
**Edit** on the right-click menu of any other row (a conversation, a
script, a blueprint) opens it, as a double click does; a blueprint's
also has **Preview** (the model viewer).
A double click on a script opens its editor (and, with **Tools › Options
› Script Editor › Open scripts in the external editor**, your own editor
too).

**Expand All** and **Collapse All** (the two icons on the Module panel's
tab bar; the palettes have their own) open and close every group of the
tree at once; with the keys in the tree (or a palette),
**Ctrl+Shift+Right** and **Ctrl+Shift+Left** do the same (Options ›
Keyboard, under Module Tree and Palettes). Collapse All also closes
every area opened out to its objects, and the kinds under it; Expand All
leaves them closed (an area's objects can be hundreds of rows).

A double click opens a resource in its editor. A blueprint dragged onto an
area's view is placed there, as from the palette. A right click on an area
offers **View Area**, **Placeables** (Make Placeables Static and
Dynamic), **Fields** (the raw fields of its `.are` and `.git` files),
and last, as in Aurora, **Properties** (Area Properties); on a script,
area, conversation or blueprint, **Find References** and **Rename…** (see
[Modules](03-modules.md)); on any resource, **Copy…**, **Delete…** and
**Export**: **Export › Export…**, **Export › Export as Files…** and
**Export › Copy to Scratch Folder** (an area's adds **Export Model
Files…**, **Export Minimap…** and **Export Minimap with Map Notes…**
([Minimaps](04-areas.md#minimaps)); a conversation's, **Export as
Ink…**, **Export as Twine…**, **Export as Text…** and **Export as
CSV…**, below).

**Several resources at once:** Ctrl and a click adds a row to those
chosen (or takes it away), Shift and a click chooses those from the row
clicked before to it, as listed; they are marked. A right click on one
of them offers what is done to them all: **Edit N Together** (blueprints
of one kind, or areas: one Properties window, what you change set on
each), **Export** (**Export…**, **Export as Files…**, **Copy to Scratch
Folder**; for areas, **Export Minimaps…** and **Export Minimaps with
Map Notes…**, each area's map into a folder as `NAME.png`
([Minimaps](04-areas.md#minimaps)); for conversations, the four below), **Delete…** (and the
Delete key on one of them), for areas **Placeables › Make Placeables
Static** and **Dynamic**, and the plugins' commands for them (where
they are all of one kind). Resources of different kinds chosen together
have Export and Delete… too.
A click alone chooses one row again.

**Conversations as stories:** on one conversation, or several chosen
that are all conversations, **Export › Export as Ink…**, **Export as
Twine…**, **Export as Text…** and **Export as CSV…** ask for a folder
and write each conversation into it as a file of its own named after
its ResRef (`gate.ink`, `gate.twee`, `gate.txt`, `gate.csv`), exactly as
the conversation editor's [Export](06-conversations.md) writes it (a
conversation open in an editor goes as it is there, changes not saved
yet included). Files of those names in the folder are asked about once,
each named, before they are overwritten (**Replace** them, **Skip Those**
and write the rest, or **Cancel**).
The log says how many were written and where; one that can't be
written is named there with why, and the rest are written. `mg
dialog-export --to` does the same from the [command
line](11-command-line.md).

**Copy…** makes a copy in the module under a ResRef you give (a free one
is offered), and for a blueprint or an area a Tag. An area is copied with
everything placed in it and joins the module's area list; one Undo takes
the copy away.

**Delete…** (or the Delete key, on the row under the pointer) asks
first, and Undo brings back what it deleted. An area goes
with everything placed in it and leaves the module's area list; a script
goes with its compiled script, a NUI window with its settings and its two
scripts. A script that goes whose editor holds unsaved text is named
when it asks: the text goes as typed, and Undo brings it back so. The
area the module's start location is in can't be deleted: set the start
location in another area first. Nothing
that names what is deleted is changed, and the window says what does,
before you answer: the places that name it (a creature's OnSpawn, a
door's conversation, an object placed from a blueprint, the module's
events), strings in scripts that spell its name, and for an area the
doors and triggers of other areas that lead to its objects (by a tag no
other area's object has); **Find References** there opens the whole
list. Of several chosen, the window lists them and deletes them as one
step; the start area, and a protected area of a project (delete it
alone, to be asked), stay out of it, said so.

A right click on a group (Areas, Scripts, Creatures…) or on one of its
resources offers **New …**: the group's wizard, or the New Script or New
Conversation window, as the Wizards menu has them.

The raw fields view lists every field of a resource, each editable. A
number that is a row of one of the game's tables is shown with the row's
name beside it, in a list to choose another by name: an area's music and
ambient sounds, a creature's race, classes, feats and spells, an item's
base item and properties, an item property's subtype, cost table, value,
parameter and its value (each of the table the property's other fields
name: Ability Bonus, Strength, +2), an object's appearance, a trap's type,
a loading screen, the base items a store buys or won't. So is an object's
faction (by the module's own factions), and a conversation line's
animation (as the Other Actions page lists them). A creature's skills
and a store's pages are named in their lists, and a talk-table string's
text is shown beside its number, a palette's `STRREF` too. An object's faction of 0 (a
blueprint's, or one an area places) is marked as the PC faction, the
players' alone, which Verify warns of (a trigger's, no). From a
terminal, `mg gff --names` names them the same way (a faction with
`--module`), and `mg find
--where` and `mg areas --set` take a row's label or name in place of its
number.

**The docks beside the middle** (the module tree and the resource
browser in the Left Dock, the palettes and the Preview in the Right
Dock, the log in the Bottom Dock) fold
away to give the area's view the room: **View** has each (Left Dock,
whichever of its panels it shows; Right Dock; Log), ticked while
shown, and **Hide All Panels** (beside a tab that brings no Right Dock,
it hides and shows the other two and leaves the Right Dock folded or
not as it was, for the next area; View › Right Dock is grayed and not
ticked then). Each also has one control of its own for
it. The **Left Dock** has a small arrow beside its inner edge, by the
middle, half way down (**Fold the Left Dock**), clear of the dock's
lists and their scroll bars; drag the edge above or below it to make
the dock wider or narrower. Tab reaches it after the dock's own
controls. Folded, the dock leaves a thin strip at the window's left,
with the arrow that brings it back (**Show the Left Dock**) beside the
strip's edge, at the same height and size, pointing the other way: it
brings the dock back on the panel it showed. At the strip's top is a
button for each of its panels (**Module**, a box; **Resources**, a
folder), which brings it back on that panel. It comes back as wide as
it was. Its panels stay in it: neither can be
dragged to another side or into the middle. The **Right Dock** is the
same, mirrored: its arrow beside its inner (left) edge, half way down
(**Fold the Right Dock**), its edge dragged above or below it to make it
wider (at least 280 points, at most two fifths of the window); folded,
a thin strip at the window's right with **Show the Right Dock** beside
its edge and a button for each of its panels (**Palette**, and
**Preview**, an eye), which brings it back on that panel. It holds the
palettes and the Preview, each on a tab of its tab bar, the one in
front filling the dock (their words, or their icons alone where the
dock is too narrow for the words; Left and Right on a tab go to the
other); they stay there: they can no longer be dragged into the
middle, to the left or into a window of their own. The tab in front
stays so while Moonglow runs; each module opens on the palette. The
Right Dock is there only
beside an area (see the start of this chapter): with another tab in
front it is not drawn at all, and comes back as it was, folded or not,
as wide. The **log** opens and closes by its switch, **Log**, at
the left of the Status Bar, lit while the log is open; it comes back as
high as it was. What is folded stays so the next time Moonglow starts,
and so do the docks' sizes. In a window too small for a dock's size it
is drawn smaller (a side dock at most two fifths of the window, the log
at most half its height, and in the narrowest windows the Right Dock
narrower than 280 so as to stay inside), and it is as large again when
the window is; only dragging its edge changes the size it keeps.
**View › Reset Layout** puts the window back as Moonglow first opens
it: the Left and Right Docks unfolded at their first widths and the log
closed (opened again, it is at its first height); and it forgets where
each kind of editor was put, so that each opens again as Options ›
General › Open editors says (see Tabs and windows).

**The keyboard in the tree.** A click on a row gives the tree the arrow
keys, wherever the pointer goes after, until it is in an area's view,
where they move the camera again (the same click in the palette gives
them to the palette); a thin outline shows which pane has them:
Up and Down go from row to row, Page Up and Page Down by ten, Home and
End to the first and the last. Right opens a row that opens (a group,
an area out to what is placed in it, a kind of object there), then goes
to the first row in it; Left closes a row that is open, else goes to
the row it is in. Enter opens the row's area, script, conversation or
blueprint, goes to a placed object in its area's view, and opens or
closes a group or a kind. Typing a name goes to the row it begins, while
the tree has the keys, wherever the pointer is: the letters typed in
quick succession go by all of them (`shopt` to `ShopThing`, a space among them as in `beggar's nest:
t`), case and accents aside, and a pause of a second starts over; one
letter, or the same letter again and again, goes to the next row that
begins with it, as in Aurora's tree and the conversation tree. The
letters go by the rows in sight, as they show: a group's name, a
resource's (an area's own name where areas are listed by name, else its
ResRef), and in an area opened out a kind's name and, in a kind open,
a placed object's name (a waypoint's tag). The area's camera
keys among the letters (W, A, S, D…) don't move the camera meanwhile;
with the pointer back in an area's view, they are the camera's again. F2 renames and Delete deletes the resource of the row
under the pointer, or else of the row the keys are at (both ask first,
as the row's menu does). What is typed in the window that asks (the
Rename window's name, Escape, Enter, which is **Delete** in Delete's
question) is the window's alone: closed, the tree is as it was, the
keys its own at the same row (at the renamed row's new place once it is
renamed). Once resources are deleted, the keys are at the row that takes
their place: the next in the group, else the one before, else the
group's row. At a placed object, Delete deletes it as its
menu's **Delete** does (Undo brings it back), and the keys go on to the
row after it; F2 does nothing there (its name and tag are in its
Properties). Ctrl+F (**Find**, under Script Editor in **Options › Keyboard**) goes to the
Filter; there, Escape clears what is typed, and Escape again goes back
into the rows, where they were; Down goes to the first row the Filter
finds (the palette's search has the same keys, and Down goes to its
first row). In the rows, Escape hands the keys back. The row the keys are at is marked, and brought into view.

## Tabs and windows

**A document tab** shows its kind's icon (an area's, a script's, a
creature's, the tool's own for Module Properties, the manual or a plugin's
panel), its name, a **dot** after the name where what it shows has
changes not saved (a script typed in and not saved into the module, a hak,
2DA, tileset or the talk table not saved to its file, an area,
conversation, blueprint or Module Properties changed since the module was
last saved; the dot goes when it is saved, or undone back to what was
saved: for the module's resources a moment after the change rests, as
the dot comes at once with a change and the resource is compared with
what was saved only once it is no longer changing), and a **close button**: on the tab in front of its group always,
on another while the pointer is over it (its room is kept, so the name
does not move as it comes). A screen reader says the tab's name and, with
the dot, "modified". The window's title keeps its star for the module's
unsaved changes. Where a group has more tabs than its bar holds, their
names are cut short alike (to about eight letters at least, the whole name
in the tab's tip), and past that the bar scrolls (the wheel over it); the
tab brought to the front is scrolled into sight.

**Editor groups.** The main pane can hold groups of tabs side by side:
**Split Editor** (the button at the right end of the focused group's tab
bar, View › Split Editor, Ctrl+\\) moves the tab in front to a group of
its own at the right, half the room, and gives it the focus; with two
groups or more it moves the tab into the group beside (the next, or for
the last the one before). It is there while the group has more than one
tab. Dragging a tab to a side of a group makes a group there too, and
into another group's tab bar moves it there. The group with the focus has
the accent line over its tab in front, the others a gray one; the docks
follow its tab in front (an area brings the Palette and Preview), and the
tab keys act in it. A group whose last tab closes goes, its neighbor
taking its room; with every tab closed the main pane says no area is
open. The groups are kept with the module's open tabs and come back as
they lay. Moonglow 4.1.0 reading them brings two groups back as they
were, and three or more as groups split off the main pane's sides, with
every tab. **F6** gives the arrow keys to the editor area as one pane, the
tab in front of the focused group.

Areas always open as tabs in the main pane. Everything else (blueprint
and object editors, scripts, conversations, Module Properties, the Faction
and Journal Editors, a resource opened read-only, this manual, Options, a
plugin's panel) opens as **Options › General › Open editors** says:
**As Tabs**, a tab of the focused editor group (the group last clicked;
a new installation starts so), or **As Floating Windows**, a window of
its own over the area, sized for what it shows, so the area view stays put
(settings from before this switch keep it). Two open in a window of their
own either way: Tile Properties, which follows the tiles chosen in an
area view, and a model's view (a blueprint's Pop Out, the palette's
Preview, a tile's model), opened to look at beside the editor it came
from. **A kind you move opens where you last put one**: drag a window by
its tab into the main pane, or beside another, to dock it; drag a tab
out, or choose **Eject** on its menu, to float it; the next editor of
its kind opens there too (one moved to the side of the areas' group opens
in the group there), in either mode, until **View › Reset Layout**
forgets every kind's place. With editors as tabs, a tab moved out to a
window is kept as "goes to a window"; one moved among the groups is kept
only when you moved it (dragged it, or Split Editor did), not when the
groups around it changed. With editors as windows, a window is where one
opens anyway, and is not kept. The palettes are not a tab: they are the
Right Dock's panel. A dialog (a question, a picker such as Select
Portrait, a wizard) opens in the middle of the main window, whichever
docks are open. It is never taller or wider than the main window: in a
short window its body scrolls, so its title row and every button stay
in reach (and a dialog in the middle of a short window can cover what
is behind it, the Welcome page's title too). Its **title row** runs
across its top, the title at the left: drag the row to move the dialog
(a drag on the rest of it, where no control is, moves nothing). A
dialog that Escape closes (Find and Replace Text, Build Module, Rename,
About) has a **Close** at the row's right, after its own buttons when
you go through it with Tab, and so do the pickers Select Portrait and
Select Texture, whose Close is their Cancel; one that only its own
buttons answer (a question, a wizard) has none. A dialog that folds
(Find and Replace Text, Find Instance, Plugins and most dialogs that
Escape closes) has the arrow at the row's left: **Collapse** folds it
to its row, **Expand** shows it again as it was. A job's window (a
running command's progress, a plugin's question) has the same row, with
no Close: its Cancel answers it.

**Escape** closes the window in front: a dialog (as its Cancel or close
button does; a picker opened from a Properties window closes alone, and
the window stays), then a window with **OK** and **Cancel** (a
Properties window, Options, Tile Properties) as its **Cancel** does, as in
Aurora: what was changed in it is set back (Tile Properties with changes
asks first, as its close button does); a model's window while the pointer
is over it. **A docked tab with OK and Cancel** (Module Properties, which
the module opens docked, a Properties tab, Options as a tab) is canceled
by Escape the same way, in either mode, while it is in front of its
group. **The keyboard decides first**: Escape goes where the keyboard is,
the field that has the keys (clicked, or reached with Tab), or else where
you last clicked, whatever the pointer is over. A window there keeps it
(a floating script's window with the keys takes Escape for itself, and no
docked tab under the pointer is canceled); a docked tab with OK and
Cancel there is canceled, not the one the pointer is on. Only where the
keyboard is in nothing that takes Escape (your last click on a dock, a
bar, a tab without OK and Cancel, no field with the keys) does the tab
under the pointer take it. Escape that sets changes back says so in the
log, which the Status Bar shows while the log is closed: "Escape canceled
Module Properties: its changes are set back. Edit › Undo brings them
back." One **Edit › Undo** brings back what a Properties tab or window
set back (Module and Area Properties too); what Options had not applied is
dropped, and Tile Properties with changes asks first. Nothing is said
where nothing was changed. Escape goes first to what is open over it or has it: an
open menu or list, Quick Open, a dialog, a window in front with OK and
Cancel where the keyboard is not in the tab, a search or filter field (the module tree's Filter, an
inventory's Find), and nothing happens while a mouse button is held (a
drag). It does not close a tab without OK and Cancel (a script, a
conversation, a side editor, a model's view docked). In a text or number
field too, Escape is the window's or the tab's Cancel at once, as in
Aurora (what was typed there is set back with the rest): to keep what you
typed and leave the field, click elsewhere (or press Tab in a field of one line). In a search
field of a Properties window or tab (an inventory's or a list's Find) it
clears what is typed, then leaves the field, and the next Escape is the
window's. In a dialog's search field (Open Script's name, a wizard's
Find, Open 2DA, the Area chooser, the Command Palette, a long dropdown's
Filter) Escape clears what is typed, and the next Escape closes the
dialog (or the list). With something in hand in the area (a blueprint to
place, a paste, a terrain brush) and the pointer over the area, Escape
lets go of that and leaves the window open. Right-click a tab for **Close**, **Close
Others**, **Close Tabs to the Right** and **Close All** (the tabs of its
group or window) and **Eject** (to a window of its own), and for an area,
script, conversation or blueprint of the module, **Rename…**: it is renamed
everywhere the module names it (an area with its instances and its entry in
the module's area list), and its tab with it. A window with **OK** and
**Cancel** closed by its tab's close button, Close, Close Others, Close Tabs to the
Right, Close All or Ctrl+W keeps what was changed in it, as OK does;
with **Closing a window cancels its changes (as Aurora)** on in
Options › General, it does what its Cancel does instead.

**The tab keys** work on the editor group with the focus, or on a
window with the focus that has more than one tab: Ctrl+W (or Ctrl+F4) closes the tab in
front (File › Close, which closes the module, has no key until you give
it one in the Options: in Aurora it is Ctrl+F4),
Ctrl+Tab and Ctrl+Shift+Tab go round them, Ctrl and a digit goes to
one by its number (9: the last) (with **Ctrl+Tab goes to the
tab used last** on, in Options › General, in the order they were last
used for as long as Ctrl is held: one press goes back to the tab before,
and again returns), and Ctrl+Shift+T opens the tab
closed last again (the last twenty are kept; one whose area or script is
gone from the module is passed over). A middle click on a tab closes it.
A hak, a 2DA or a tileset with unsaved changes asks first, as its close
button does, and as the module asks: **Save**, **Don't Save** or
**Cancel**. Closing several at once (Close All) asks of each in turn;
Cancel keeps that one and the rest open. Closing Moonglow's window, File
› Exit, File › Close and opening another module ask the same way: of
each hak, 2DA or tileset with unsaved changes by its name, in turn, after
the module's own question if the module has unsaved changes too (its
**Don't Save** drops only the module's), and then go on. **Cancel** at
any of them closes nothing. A save that fails says why, in the question
and under its editor (the talk table's too), which stays open: put the
cause right (a file added to a hak that has gone from disk since, say)
and **Save** again, or **Don't Save**, or take the change back with Undo.
The module's own question does the same: when its **Save** fails (a
place that can't be written), it is asked again, saying why. A hak,
2DA or tileset open when another module is opened stays open.

**A window opens where one of its kind was last dragged to.** Drag a
window by its tab into the main pane (beside the areas' tabs), and the
next window of that kind (the next script,
the next conversation, the Faction Editor again) opens there as a tab,
not as a window over the area. Dropped on a side of the main pane (a
pane of its own split off its left, right, top or bottom), the next
opens in a pane split off that side again, as large as it was. (A kind
kept in the palettes' pane, before the palettes were the Right Dock's
panel, opens where its kind opens, a window of its own at first.) Drag
one out to a window of its own and the next
opens as a window again. This is kept between runs; **View › Reset
Layout** forgets it.

**Scripts gather in one window.** A script opened while another's
window is open becomes a tab of that window, not one more window
(**Options › Script Editor › Open scripts as tabs of one window**, on by
default; off, each script has a window of its own).

**With no area open**, the main pane says so and lists the areas opened
lately in this module, then the module's areas, each to open with a
click: the panes beside it keep their widths.

A window opens as large as one of its kind was last left: resize a
conversation's window, and the next conversation opens at that size (each
kind of editor has its own, and each type of blueprint: a waypoint's
Properties opens at the size the last waypoint's was left, not an
encounter's), and a size dragged at its edge or corner is kept so.
A window has a border, rounded corners and a shadow under it, and a
**bar** across its top: the **Collapse** arrow at its left (folds the
window to its bar; **Expand**, the same arrow, opens it again), its
tabs, and at its right **Maximize** and **Close Window**, which closes
the window and its tabs as closing each tab would (what asks before it
closes asks, and **Closing a window cancels its changes** in Options ›
General decides whether its changes are kept). A window split in panes
has a bar on each, whose Close is **Close Pane**, closing that pane's
tabs (Shift changes nothing on these buttons; the right-click menu of
the Close offers to close the whole window). The tabs never run under
Maximize and Close: in a narrow window they are cut short, then scroll
with the wheel, and a window is never narrower than 211 points, room
for its buttons and one tab. The **grip** in its bottom right corner
shows where to drag to resize it (any edge or corner does), lit where a
drag takes it. **Double-click a window's tab** (or the bar beside it) to maximize
it over the whole of Moonglow's window under the Top Bar, and again to
put it back; **Maximize** and
**Restore** are on the tab's right-click menu too, and the button at the
right of the window's bar, before Close Window, does it and shows which it is:
a maximize icon, or restore while the window is maximized. **Ctrl+Shift+M** (Maximize Window in Options ›
Keyboard) does it for the window of the tab in front. Any window does
this, one of a tab dragged out of a pane (the main pane's Module
Properties too) as well. A maximized
window stays over the whole of Moonglow's window as that is resized. **The module tree
follows the tab chosen**: a tab clicked, reached by the keys (Ctrl+Tab,
Ctrl+Page Down, Ctrl and a digit) or opened (from the tree, a script
field's Edit, Find References…) marks its resource's row in the tree as
the one in hand, opens its group and brings the row into view: an area,
a script, a conversation, a blueprint, a NUI window, the journal or the
factions (a placed object's Properties: its area); Module Properties
marks its row at the top. A tab of several (Properties of several
blueprints or areas) chooses each of them, as Ctrl and a click would, the
first brought into view. The keys stay with the tab, and a row in view
stays where it is; one out of sight comes to the middle of the pane, the
rows after it in sight too. A tab that comes to the front because the
one before it was closed (an area, a Properties window opened from the
tree) was not chosen: the tree stays where it is. Nor is the first
click in a tab the tree has gone to already (one just opened): rows
chosen in the tree since, several to delete or export together, stay
chosen. A tab whose row the Filter hides, chosen
while the Filter has something typed, is listed under it all the same
(the Filter stays as it is) until what is typed changes. With **Show the tab
chosen in the module tree** unticked in Options › General, the tree
stays where it is and only the area in front is marked, as in Aurora
(whose other editors are dialogs, and whose tree marks the area in
front alone). Where the area is opened out in the tree (the arrow beside
its row), an object selected in the area's view is shown there too: its
kind's list opens and its row is marked and brought into view (a row in
view stays where it is; one out of sight comes to the middle). An area
left closed in the tree is opened out to it, as in Aurora; with **Show
the object selected in an area in the module tree** unticked in Options
› General, it stays closed, and the tree goes to an object only where its
area is opened out already. **Show in Module Tree** on the
tab's menu goes to its row whatever the option says, and opens an area
out to what is placed in it. The bar beside the
tab does the same on a double click and a right click, and the window is
dragged by it, and only by it: a drag on a part of the window that isn't
a control doesn't move it (Options › General › **Move windows by
dragging anywhere in them** does). The arrow at the bar's left folds the
window to its bar. Window sizes kept by Moonglow 4.1.0 and earlier, taken
when rows were shorter, are made as much taller as their windows now are,
once, the first time this version reads them (a size kept 640 points high
for a creature's Properties opens 792 high, a script's 700 opens 756);
a window never opens larger than Moonglow's window.

**A module opens again with its tabs**: the areas, scripts, blueprints,
conversations, side editors and tools open when it was closed, in the
same panes and windows (split as they were, where they were moved to),
each pane's tab in front in front, and each view as it was left: an
area's camera, a script's scroll, cursor and bookmarks, a
conversation's selected line, bookmarks and folded branches, the talk
table's line, a Properties window's page (Options › General; see
[Modules](03-modules.md)).

Closing a tab or window never loses work: every change is part of the
module (and undoable) the moment you make it. **Save** writes the module.

**Undo and redo** cover every change, in every editor, without limit: a
field typed into, an object moved, a tile painted, a script compiled.
The editors of files of their own (a hak, a 2DA, a tileset, the talk
table) keep their own steps: Undo and Redo, by their keys (Ctrl+Z,
Ctrl+Y and Ctrl+Shift+Z, or those set in Options › Keyboard), are that
editor's with the pointer over it (and a 2DA's while the keys are in
its grid), and never the module's last change there. Edit › Undo and
Redo and the Top Bar's act on the editor you are working in: the one
the pointer went from straight to them, else the one you last clicked
in or typed in (its tab in front, docked or in a window of its own).
After a click in the module tree, an area, the log or the Status Bar,
they are the module's again, whatever editor's tab is in front;
passing over a view on the way to the Top Bar is not working in it.
With the pointer on the Top Bar, Ctrl+Z does what its Undo and Edit ›
Undo say; elsewhere it follows the pointer, so over the module tree it
is the module's even while the Top Bar's is still the editor's you
last clicked in (its Find field clicked, say, and not typed in). They name the step where the editor names its steps ("Undo
Add files" in a hak's, "Undo Empty 1 cell" in a 2DA's; a tileset's and
the talk table's say plain Undo), and an editor with nothing of its
own to take back leaves them grayed, not the module's. A text field in one of
these editors (its Find field) takes back its own typing once you have
typed in it; clicked into and not typed in, it has nothing of its own,
and Undo is the editor's, as a field elsewhere leaves Undo to the
module's. A script's editor
keeps its own steps of its text: while it has the keyboard, Ctrl+Z and
Ctrl+Y, the keys set in Options › Keyboard, Edit › Undo and Redo and
the Top Bar's are all the script's (see [Scripts](07-scripts.md)).

**Colors.** A color's swatch (an area's lighting, a light, a NUI color,
the Options' colors) opens a picker. Its copy button copies the color's values
(`158, 151, 211`), and **Paste Color** under it, or **Ctrl+V** while it is
open or the pointer is on the swatch, takes one from the clipboard:
`158, 151, 211`, `#9e97d3`, `rgb(158, 151, 211)`, or values from 0 to 1.
Beside Paste Color, the color's hex code (`#9e97d3`) follows the picker
as it is dragged; type or paste a color there and the picker takes it.
The swatch's right-click menu has **Copy Color** and **Paste Color** too.

## The palette

The Right Dock's **Palette** panel is Aurora's palette: choose a
blueprint type (the row of icons at its top, in two rows where the dock
is narrow),
**Standard** (the game's blueprints) or **Custom** (the module's), and a
blueprint from the categories. Click in an area to place the chosen
blueprint (see [Areas](04-areas.md)). With an area's terrain mode on, the
palette shows the tileset's brushes instead. The row's last three are
**Prefabs**, **Lights** (the game's invisible lights, one chosen and
then put where an area is clicked) and **Start Location** (where a
player's character enters the module: chosen, a click in an area puts
it there). The **Preview** tab beside it shows the chosen blueprint
(Aurora's Preview window): its name, tag, resref, comments and a few
fields of its type, an item's inventory icon, a store's stock by page,
and creatures, doors, items and placeables in 3D under their fields,
with the model viewer's controls. A 3D view is drawn in one place at a
time: where the same blueprint or model is in a tab already, the tab
keeps it and the Preview says the preview is shown elsewhere. The
Preview is drawn only while its tab is in front (behind the palette's
it reads and draws nothing), and then shows whatever is chosen in the
palette at that moment; choosing a blueprint does not change the tab.
While a blueprint stays chosen, its preview keeps its camera, the
animation chosen, Pause or Play and its Lights switch, behind the
palette's tab or beside another tab, however long; another blueprint
chosen, they start afresh. Escape with the pointer over the Preview
(or the palette) leaves a blueprint in hand to place: only over the
area does it let go of it.
**Tools › Preview** (or the strip's Preview button, the dock folded)
brings it to the front, with the dock, beside any tab. After a click
in the palette, the
arrow keys move through its rows and open and close its categories (see
[Blueprints](05-blueprints.md)). Tools › Options › General › **Hide
empty categories in the palettes** leaves out the categories with no
blueprint in them ([Options](10-options.md#module-tree-and-palettes)).

## The resource browser

The Left Dock's **Resources** tab lists every resource the game sees in
its load order (the game's files, haks, override, the module), with the
layer each one comes from. **Tools › Resource Browser** shows it: the
dock unfolded on that tab (with no module open, the Left Dock with the
Resources tab alone). Click **Module** to go back to the module tree;
the dock shows the tab chosen last while Moonglow runs, and a module
whose browser was open as a tab in an earlier version opens with the
Resources tab shown. At the top of the panel are **Type** (one type of
resource, or all), the **Name** field and **Overridden copies**, and
under them how many resources are listed, of how many ("12 of 113538
resources", and the overridden copies listed). Each row has its kind's
icon, its name, and at the right the layer it comes from. Where the
dock is too narrow for both, the name comes first: the layer is cut
short with an ellipsis to the room the name leaves it, or left out
where only a few letters of it would fit, and a name longer than the
row is cut in its middle, its start and its end in sight
(`act_q2dji…stone.nss`), since names that begin alike differ at their
ends. Resting the pointer on the row shows both whole ("portraits.2da
from hak:low, overridden by hak:top"). Widen the dock by dragging its
edge. A file linked into `override` or `development`
(a symbolic link) is read as the game reads it, as a file copied there; a
link to a folder is not followed. Double-click a resource (or press
Enter on it) to view it read-only in a tab or window of the editor area
(GFF files as a field tree, 2DA tables, scripts and other text; models,
and blueprints with **Preview**, in the model viewer), copy it into the
module, or save it to a file (**Save As…**, also on its right-click
menu in the list; a compiled model shows as the text it compiles from,
and **Save As Text…** saves it so). The panel's tab bar has two buttons:
**Export N as Files…** (an arrow out of a tray) writes all the
resources listed into a folder, once a type or part of a name narrows
the list (its tip says how many: "Write the 627 listed resources into
a folder as files"; with the whole load order listed it says to narrow
the list first), and **Refresh** reads the load order
again. Files go out as they are; with **Models as text** ticked (under
Overridden copies, shown when models are listed), compiled models go
out as the text they compile from. Where the folder has some of the
files already, it asks first (**Replace** them, **Skip Those**, or
**Cancel**), and the files are written while a window shows how far it
is, with **Cancel**.

A resource that several layers have (a portraits.2da in two haks, a
texture in a hak and in the game's files) is listed once for the copy
the game reads, and under it, set in and dimmed, each copy that copy
overrides: "hak:low, overridden by hak:top". Open one of those to view
it as its layer has it (a 2DA from that copy down, with the layers below
it), copy it into the module or save it (**Save As…**, also on its
right-click menu), or see a model or blueprint of it in the 3D viewer
(**Preview** on its right-click menu: that copy, drawn with the rest of
the load order as the game reads it; the viewer's Export Files… is left
out there, as what it gathers is the game's); for a table in one of your own haks, **Edit in
name.hak** opens it there. With **Overridden copies** unticked, it lists
the copy the game reads alone. **Export N as Files…** writes the copies
the game reads into the folder chosen, and each overridden copy listed
into a folder of its layer's under it (`hak_low/portraits.2da`). `mg which NAME.EXT --module
MODULE` says the same on the command line.

Its **Name** field has the side panes' keys: Ctrl+F, with the pointer
over the panel, goes to it; Escape clears what is typed, and Escape
again leaves it; Down goes into the list, where Up and Down (Page Up,
Page Down, Home, End) choose a row and Enter opens it. A click on a row
gives the list the keys too, and **F6** steps to the list and on from
it. Choosing the Module or Resources tab takes the keys from the one
list to the other. The **Find** fields of the hak editor, the
tileset editor, the 2DA editor and the Appearance Gallery take Ctrl+F
and Escape the same way, with the pointer over them. Ctrl+F is
**Find** in **Options › Keyboard** (under Script Editor): the keys given to it there go to
every search field (the wizards', Open Script's, Open 2DA's and the
Area chooser's with the pointer over their window; not the script
editor's side lists, where they are the script's Find).

Every search field finds a row with every word typed, in any order,
in any one of the row's fields; what those are differs by list:

| List | Looks in |
|---|---|
| The module tree | a resource's name and ResRef; a placed object's name and tag |
| The palettes, the item picker | name, ResRef and tag (with nothing that has every word, the palettes show close matches: letters in order) |
| Find Instance | the tag and the blueprint, each field its own words |
| The resource browser, the hak editor, Transfer | the ResRef or the file name |
| The tileset editor | the model and its terrains |
| A 2DA | every cell, and a cell's value by its name |
| The Area chooser | ResRef and name (not the tag) |

Down leaves the field for its list where the list has keys of its own
(the module tree, the palettes, the resource browser; in Open Script
and Quick Open, Up and Down choose while you type); elsewhere
Down is the field's.

A 2DA shows the copy the game reads, read-only, with its StrRef columns
(Name, Description, StrRef and the like) as their text; **StrRefs as
numbers** shows the numbers. It is the 2DA editor's grid with the
editing left out: its rows scroll with scroll bars at the right and the
bottom, the column names staying at the top and the rows' numbers at the
left; a click selects a cell (its whole value is shown over the grid),
the arrow keys, Tab, Page Up and Down, Home and End move (Shift held,
the selection grows), and **Ctrl+C** copies the cells selected as a
spreadsheet copies them; **Find** narrows the rows to those with the
text in a cell, **Go to** takes a row's number; a column's width is set
by dragging the edge at the right of its name (a double click there: as
wide as its widest cell), and is the 2DA editor's for that table too,
and the menu a right click on its name opens hides it (**Show Hidden
Columns** brings it back) or resets the widths. Nothing typed, pasted or
deleted changes it. **Edit…** opens the table in the 2DA editor (see
[the 2DA editor](03-modules.md#the-2da-editor)). When several layers have the table (a hak's copy over
the game's), the view adds:
- a **From** column with the layer each row comes from: the lowest one
  it has come down from unchanged;
- cells a higher layer changed, colored, with what they were below
  (hover over one);
- a line on what each layer adds and changes;
- a menu to show only one layer's rows.

## The model viewer

Models open in the model viewer from the resource browser, and blueprints
(creatures, items, placeables, doors) through **Preview**: in the palette,
the module tree, a blueprint's window; a placed object, as it is placed,
from its right-click menu in the area or the module tree, or its
Properties. Drag to turn
the model (right or middle button to move it), use the wheel to zoom
(finer with Shift or Ctrl held, or pinch on a touchpad; a sideways
scroll moves it left and right, as in the area view), and play its
animations, including those from its supermodels. An item modeled lying
(a sword, an axe) is stood on its end, its broad side to you: a blade
shows its flat, not its edge. **Lights**
lights it with its own light nodes. **Shadow** (on unless you turn it
off, and remembered) has the model cast the sun's shadow on a ground
under it, as an area's ground takes a creature's shadow: the ground
itself isn't drawn, so the model looks as it did, with its shadow on the
background (the sun high over your shoulder as the viewer opens; a
model reaching below its feet stands on the ground at its lowest
point). The pictures
of the galleries, the palettes and a hovered row's preview have no
shadow. **Shaders** draws it with the game's
own shaders, a material's custom ones among them (wind-swayed leaves move
while the viewer plays): the same switch as the area view's toolbar
(Areas › The toolbar).

## The log

Messages go to the log, in the Bottom Dock under the tabs: what was
opened or saved, compiler errors, warnings about missing resources.
Warnings are yellow, errors red. Each line begins with the time it was
written (`14:03:27`), as the script editor's Compiler messages do; a
line selected and copied from the log takes its time with it. On the
right of its tab bar, **Copy Log** (or View › Copy Log) puts the whole
log on the clipboard and **Save Log** (View › Save Log…) writes it into
a text file, each line with its time; **Clear Log** (View › Clear Log)
empties it: a second Verify's report isn't then mistaken for the
first's. Where the Bottom Dock is narrow (in a narrow window, beside
both side docks), its tab shows the log's icon alone, and narrower
still the three buttons give way from the end (Clear Log first): they
are in the View menu too. The **Log** switch at the left of the Status
Bar (or View › Log, Ctrl+Alt+3) opens and closes it; drag its top edge
to make it higher or lower, down to a line. A new installation of
Moonglow starts with the log closed; one that has run before keeps it
as you left it.

While the log is closed, its last line follows the Log switch in the
Status Bar, without its time: a warning in yellow, an error in red, cut
short with an ellipsis where the bar is short of room. Only a message's
first line is shown, and a blank line (a command's empty output) is
passed over for the message before it. Rest the pointer on it for its
time and its first lines (a long message says "… more in the log");
click it to open the log. A screen reader hears it as "Last log line:"
and the line.

While long work runs as a job (Build Module, Compile All Scripts,
Verify Module, a hak saved, an NWSync publish, a project's or a
plugin's command, among others), the right of the Status Bar shows its name and a bar: how much is done
where the job says, a mark going to and fro where it does not. Rest the
pointer on it for how far it is. One job runs at a time, in a window of
its own with **Cancel**, which is how it is stopped; work asked for
meanwhile waits (the log says so).

## The Status Bar

The row along the window's foot. At its left, the **Log** switch (and,
while the log is closed, its last line: see [The log](#the-log)); in
its middle the **hints**; at its right the counts, a running job, the
module's name and the game's folder.

**Hints.** What the keys and the mouse do in the tab you work in (the
one in front of the pane you last clicked in, as the docks follow it),
each a key cap or two and a word: **Drag** Move, **Alt** + **Drag**
Raise, **Q** **E** Turn by a step. They change as you work: in an area
with nothing selected they are Click Select, Drag Move or box, Ctrl +
Drag Move view and Ctrl + Right Drag Turn view; with objects selected,
Drag Move, the turning ring Turn and Q and E Turn by a step (where
something selected turns: a sound does not), Shift Handles, Alt + Drag
Raise (where something selected is not a creature: creatures stand on
the ground) and G Drop to ground (Alt + Right Drag turns the selection
too; the ring is what is hinted while it shows); with Shift held, the
handles' (Move along an axis, and Tilt and Scale where the selection's
models take them); during a drag, where the one object dragged or
turned is now (its kind, tag, blueprint, place and facing, read as it
moves, also while Q or E turns it), then Esc Cancel, Middle Drag Turn
view and the snap; and for each tool in hand (a blueprint, a light or the start
location being placed, a paste, Paint, Select Tiles, a tile brush,
Redraw Polygon, Ctrl + wheel's scaling) its own, after a note of what is
in hand ("Placing nw_chicken", "In hand: Raise/Lower", "Painting 2
placeables"). A reason a step was refused (a tile stroke, a static
placeable's scale) is a note at the head of the hints. The area's keys
(Q, E, G, Page Up and Page Down, B) and presses act while the pointer is
over the area's view: the hints say what they do there, wherever the
pointer is. While a dialog, a menu or a job's window is in front, the
view takes neither, and the area shows no hints (its counts stay). The
keys shown
are yours: a command you gave another key in Options › Keyboard shows
that key, and one you left without a key has no hint; on a Mac,
Ctrl and Alt show as Cmd and Option. The Welcome page shows New module,
Open module and Quick Open's keys; a script's tab its keys (Find, Find
next, Definition, References, Complete, Bookmark, Compile all,
Replace); the Resources panel, while the pointer is over it or its list
has the keys, its own (Up or Down to move in the list and Enter to open,
where the list has the keys; Double Click to open, Right Click for more,
and Find's key, which goes to its Name field with the pointer over it); a tab with OK and Cancel (Options, a Properties tab) Tab and
Shift + Tab for its fields, Esc Cancel where an Escape would cancel it
(by the rule under [Tabs and windows](#tabs-and-windows): not while a
field has the keys, where which tab the Escape is for shows only as you
press it, nor while the module tree, the Resources panel or the log has
the keys, which take it first, nor while a menu is open, nor with the
pointer over a floating model's window, which an Escape closes), and,
for Options, Enter OK while the pointer is over it and neither a field
nor the module tree has the keys (the tree takes Enter first). Where an Escape would cancel a tab other than the
one you work in (a Properties tab beside the area, the pointer over
it), its hint comes first and names it ("Esc: Cancel Module
Properties"). A
screen reader reads each hint as one label ("Shift, Right Drag:
Turn"); the hints take no Tab stop.

**Counts.** An area's tab shows its size and how many objects it holds
("11 by 11 tiles", "192 objects"; the objects of the kinds shown), and
before them what is selected ("1 selected", rest the pointer on it for
the object's kind, tag, blueprint, place and facing, which a drag of it
shows among the hints as it moves; "3 selected";
in Select Tiles "2 by 3 tiles selected" or "Tile (4, 4) selected") and,
with a tile brush, the tile under the pointer. Another tab has none.

**When it is short of room**, its parts give way in this order: the
game folder goes first; then the hints, the last one first, each whole
(a note at their head is cut short with an ellipsis, its whole words in
its tip); then the log's last line, cut short from its usual width (a
short one beside an area's hints, a little more beside another tab's)
down to a few words, then gone; then the counts, the last first; then a
running job's name is cut short (its bar kept); and last the module's
name and the note of files being read are cut short ("(modified)" kept;
rest the pointer on any of them for the whole).

## Screen readers and desktop agents

Moonglow's controls are offered to the system's accessibility interface
(UI Automation on Windows, the accessibility API on macOS, AT-SPI on
Linux), so a screen reader, or a program that drives desktop
applications, can read them by name and press them. Moonglow builds that
description only while such a program asks for it; otherwise it costs
nothing. On Linux it is offered while the desktop's accessibility bus
says a screen reader is on (its `ScreenReaderEnabled` setting, which a
screen reader such as Orca turns on). Newer accessibility buses no
longer have that setting (at-spi2-core 2.62 hasn't), and there Moonglow
doesn't offer its controls yet: the version of the toolkit it is built
with asks for that setting alone. The Flatpak is allowed to ask the
desktop for it.
