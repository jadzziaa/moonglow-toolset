---
type: Manual Page
title: The main window
description: The main window - menus, keyboard shortcuts, the module tree, tabs and windows, the palette, the resource browser, the model viewer and the log.
tags: [manual, window, shortcuts]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-07T21:30:00Z }
---

# The main window

From top to bottom: the **menus**, the **toolbar**, then the module tree
on the left, the **tabs** in the middle and the **palette** on the right,
the **log**, and the **status bar** (the module's file, "(modified)" while
there are unsaved changes, and the game's folder).
The palette opens with each module, as wide as the module tree. If you
close it, **Palettes** on the toolbar brings it back.

## Menus

| Menu | Commands |
| --- | --- |
| File | New Module…, Open Module…, Open Folder…, Recent Modules, Save, Save As…, Save As nasher Project…, Import…, Export…, Close, Exit |
| Edit | Undo, Redo (each names what it undoes), Module Properties, Resize Area…, Rotate Area…, Find Instance…, Prefabs, Find References… |
| Wizards | Area Wizard…, Creature Wizard…, and a wizard for each other blueprint type (Door, Encounter, Item, Merchant, Placeable, Sound, Trigger, Waypoint) |
| Tools | New Conversation…, Faction Editor, Journal Editor, Talk Table, New Script…, Palettes, Resource Browser, Tilesets (New Tileset…, Open Tileset…), Haks (New Hak, Open Hak…, Build Hak from Folder…), Reload Resources, Options… |
| Build | Compile All Scripts, Build Module…, Publish to NWSync…, Verify Module, Test Module, Test Module, Choose Character, Pack *file* (nasher projects), Area Statistics |
| Plugins | the commands of the plugins you have enabled, Manage Plugins…, Install Plugin from File… ([Plugins](15-plugins.md)) |
| Help | User Manual, Command Palette…, About Moonglow Toolset |

**Help › Command Palette…** (Ctrl+Shift+P) finds a command by its name:
type part of it (or of its menu's name), choose with the arrow keys, and
Enter runs it; Escape closes. Each command shows its menu and its key. A
command that can't be chosen now is listed dimmed. Any command can be
given a key in Tools › Options › Keyboard.

## Keyboard shortcuts

On macOS, Cmd takes the place of Ctrl. These are the default keys
(Aurora's, where Aurora has the command). **Tools › Options › Keyboard**
changes them; the menus show the current keys.

| Keys | Command |
| --- | --- |
| Ctrl+N | New module |
| Ctrl+O | Open module |
| Ctrl+S | Save |
| Ctrl+Z | Undo |
| Ctrl+Y, Ctrl+Shift+Z | Redo |
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
| Ctrl+Shift+P | Command Palette |

The area viewer and the editors have their own keys; their chapters list
them.

## The module tree

The left pane lists what the module holds, by kind: Areas, Conversations,
Scripts, the blueprints (Creatures, Doors, Encounters, Items, Merchants,
Placeables, Sounds, Triggers, Waypoints) and the journal and factions;
**Module Properties** is at the top. **Filter** narrows every group to the
names containing the text; it and Expand All / Collapse All stay in sight
while the tree scrolls, and Home and End go to the tree's top and bottom
with the pointer over it.

Resources are listed by their ResRefs, as in Aurora. With **Tools ›
Options › General › List areas and blueprints by name**, areas and
blueprints are listed by their names instead (in the names' order; one
without a name keeps its ResRef), areas' tabs are titled by name, the
filter finds either, and the ResRef shows when the pointer rests on one.
**Show ResRefs beside names** adds it in parentheses, `Name (resref)`,
there and in the palettes: for telling apart resources of one name.

**What is placed in an area:** the arrow before an area opens it out to
its objects, kind by kind as Aurora lists them (Creatures, Doors,
Encounters…), each by its name (its tag, without one). Click an object to
go to it in the area's view, selected; double-click for its Properties;
right-click for those and for **Copy** (pasted in any area with Ctrl+V)
and **Delete**. The Filter narrows an opened area's objects to those it
finds by name.
**Edit** on the right-click menu of any other row (a conversation, a
script, a blueprint) opens it, as a double click does.
A double click on a script opens its editor (and, with **Tools › Options
› Script Editor › Open scripts in the external editor**, your own editor
too).

**Expand All** and **Collapse All** under the Filter open and close every
group at once (the Palettes pane has the same for its categories).

A double click opens a resource in its editor. A blueprint dragged onto an
area's view is placed there, as from the palette. A right click on an area
offers **View Area**, **Properties** (Area Properties) and the raw fields
of its `.are` and `.git` files; on a script, area, conversation or
blueprint, **Find References** and **Rename…** (see
[Modules](03-modules.md)); on any resource, **Copy…**, **Delete…**,
**Export…**, **Export as Files…** and **Copy to Scratch Folder**.

**Copy…** makes a copy in the module under a ResRef you give (a free one
is offered), and for a blueprint or an area a Tag. An area is copied with
everything placed in it and joins the module's area list; one Undo takes
the copy away.

**Delete…** (or the Delete key, on the row under the pointer) asks
first, and Undo brings back what it deleted. An area goes
with everything placed in it and leaves the module's area list; a script
goes with its compiled script. The area the module's start location is in
can't be deleted: set the start location in another area first. Nothing
that names what is deleted is changed (check **Find References** first).

A right click on a group (Areas, Scripts, Creatures…) or on one of its
resources offers **New …**: the group's wizard, or the New Script or New
Conversation window, as the Wizards menu has them.

The raw fields view lists every field of a resource, each editable. A
number that is a row of one of the game's tables is shown with the row's
name beside it, in a list to choose another by name: an area's music and
ambient sounds, a creature's race, classes, feats and spells, an item's
base item and properties, an object's appearance, a trap's type, a loading
screen. A creature's skills are named in their list, and a talk-table
string's text is shown beside its number.

## Tabs and windows

Areas open as tabs in the main pane. Everything else (blueprint and object
editors, scripts, conversations, previews, Module Properties, the Faction
and Journal Editors, the resource browser, this manual) opens in a window
of its own over the area, sized for what it shows, so the area view stays
put. Drag a window by its tab into the main pane, or beside another, to
dock it; drag a tab out to float it again. The palettes keep their pane on
the right. **Escape** closes the window in front: a dialog (as its Cancel
or close button does), then a Properties window (what was changed there
stays, and Undo takes it back); a model's window while the pointer is over
it. With something in hand in the area (a blueprint to place, a paste, a
terrain brush) and the pointer over the area, Escape lets go of that and
leaves the window open. Right-click a tab for **Eject** and **Close**, and for an area,
script, conversation or blueprint of the module, **Rename…**: it is renamed
everywhere the module names it (an area with its instances and its entry in
the module's area list), and its tab with it.

A window opens as large as one of its kind was last left: resize a
conversation's window, and the next conversation opens at that size (each
kind of editor has its own). **Double-click a window's tab** to maximize
it over the whole of Moonglow's window under the toolbar, and again to
put it back; **Maximize** and
**Restore** are on the tab's right-click menu too. Choosing an area's
tab brings the area's row into view in the module tree, marked as the
one in hand, and an object selected in the area's view is shown there
too (its area opened out, its row marked), as in Aurora; **Show in Module Tree** on the tab's menu opens it out to
what is placed in it as well. The bar beside the
tab does the same on a double click and a right click, and the window is
dragged by it. The arrow at the bar's left folds the window to its bar.

Closing a tab or window never loses work: every change is part of the
module (and undoable) the moment you make it. **Save** writes the module.

**Undo and redo** cover every change, in every editor, without limit: a
field typed into, an object moved, a tile painted, a script compiled.

## The palette

The right-hand pane is Aurora's palette: choose a blueprint type,
**Standard** (the game's blueprints) or **Custom** (the module's), and a
blueprint from the categories. Click in an area to place the chosen
blueprint (see [Areas](04-areas.md)). With an area's terrain mode on, the
palette shows the tileset's brushes instead. **Preview** (toolbar) shows
the chosen blueprint in a window of its own.

## The resource browser

**Tools › Resource Browser** lists every resource the game sees in its
load order (the game's files, haks, override, the module), with the layer
each one comes from. Open one to view it (GFF files as a field tree, 2DA
tables, scripts and other text; models, and blueprints with **Preview**,
in the model viewer), copy it into the module, or save it to a file.

A 2DA shows the copy the game reads, with its StrRef columns (Name,
Description, StrRef and the like) as their text; **StrRefs as numbers**
shows the numbers. When several layers have the table (a hak's copy over
the game's), the view adds:
- a **From** column with the layer each row comes from: the lowest one
  it has come down from unchanged;
- cells a higher layer changed, colored, with what they were below
  (hover over one);
- a line on what each layer adds and changes;
- a menu to show only one layer's rows.

## The model viewer

Models open in the model viewer from the resource browser, and blueprints
(creatures, items, placeables, doors) through **Preview**. Drag to turn
the model (right or middle button to move it), use the wheel to zoom, and
play its animations, including those from its supermodels.

## The log

Messages go to the log at the bottom: what was opened or saved, compiler
errors, warnings about missing resources. Warnings are yellow, errors red.
