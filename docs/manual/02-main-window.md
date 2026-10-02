# The main window

From top to bottom: the **menus**, the **toolbar**, then the module tree
on the left, the **tabs** in the middle and the **palette** on the right,
the **log**, and the **status bar** (the module's file, "(modified)" while
there are unsaved changes, and the game's folder).

## Menus

| Menu | Commands |
| --- | --- |
| File | New Module…, Open Module…, Open Folder…, Recent Modules, Save, Save As…, Save As nasher Project…, Import…, Export…, Close, Exit |
| Edit | Undo, Redo (each names what it undoes), Module Properties, Resize Area…, Rotate Area…, Find Instance…, Prefabs, Find References… |
| Wizards | Area Wizard…, Creature Wizard…, and a wizard for each other blueprint type (Door, Encounter, Item, Merchant, Placeable, Sound, Trigger, Waypoint) |
| Tools | New Conversation…, Faction Editor, Journal Editor, Talk Table, New Script…, Palettes, Resource Browser, Tilesets (New Tileset…, Open Tileset…), Haks (New Hak, Open Hak…, Build Hak from Folder…), Reload Resources, Options… |
| Build | Compile All Scripts, Build Module…, Publish to NWSync…, Verify Module, Test Module, Test Module, Choose Character, Pack *file* (nasher projects), Area Statistics |
| Help | User Manual, About Moonglow Toolset |

## Keyboard shortcuts

On macOS, Cmd takes the place of Ctrl. These are the keys Moonglow starts
with (Aurora's, where Aurora has the command); **Tools › Options ›
Keyboard** changes them, and the menus show the keys as they are.

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

The area viewer and the editors have their own keys; their chapters list
them.

## The module tree

The left pane lists what the module holds, by kind: Areas, Conversations,
Scripts, the blueprints (Creatures, Doors, Encounters, Items, Merchants,
Placeables, Sounds, Triggers, Waypoints) and the journal and factions;
**Module Properties** is at the top. **Filter** narrows every group to the
names containing the text.

A double click opens a resource in its editor. A right click on an area
offers **View Area**, **Properties** (Area Properties) and the raw fields
of its `.are` and `.git` files; on a script, area, conversation or
blueprint, **Find References** and **Rename…** (see
[Modules](03-modules.md)); on any resource, **Export…**.

## Tabs and windows

Areas open as tabs in the main pane. Everything else (blueprint and object
editors, scripts, conversations, previews, Module Properties, the Faction
and Journal Editors, the resource browser, this manual) opens in a window
of its own over the area, sized for what it shows, so the area view stays
where it is. Drag a window by its tab into the main pane, or beside
another, to dock it; drag a tab out to float it again. The palettes keep
their pane on the right.

Closing a tab or window never loses work: every change is already part of
the module (and undoable) the moment you make it; **Save** writes the
module.

**Undo and redo** cover every change, in every editor, without limit: a
field typed into, an object moved, a tile painted, a script compiled.

## The palette

The right-hand pane is Aurora's palette: choose a blueprint type,
**Standard** (the game's blueprints) or **Custom** (the module's), and a
blueprint from the categories. Chosen, a blueprint is placed by clicking
in an area (see [Areas](04-areas.md)). With an area's terrain mode on, the
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
the model (right or middle button: move it), use the wheel to zoom, and
play its animations, including those its supermodels give it.

## The log

Messages go to the log at the bottom: what was opened or saved, compiler
errors, warnings about missing resources. Warnings are yellow, errors red.
