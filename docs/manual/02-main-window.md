---
type: Manual Page
title: The main window
description: The main window - menus, keyboard shortcuts, the module tree, tabs and windows, the palette, the resource browser, the model viewer, the log, and screen readers and desktop agents.
tags: [manual, window, shortcuts]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-29T21:07:10Z }
---

# The main window

From top to bottom: the **menus**, the **toolbar**, then the module tree
on the left, the **tabs** in the middle and the **palette** on the right,
the **log**, and the **status bar** (the module's file, "(modified)" while
there are unsaved changes, and the game's folder).
The palette opens with each module, as wide as the module tree. If you
close it, **Palettes** on the toolbar brings it back.

The toolbar's buttons (New, Open, Save, Undo, Redo, Properties, New
Area, Start, Resources, Palettes, Preview, Compile, Verify) each show an
icon and a word. To save room, **Tools › Options › General › Toolbar
buttons: Icons only** shows the icons alone, as Aurora's toolbar does
(the area view's and the editors' toolbars too): rest the pointer on one
and its tip names it, with its key. Each icon is one command's: ℹ is
Module Properties, the area view's Area Properties is 🔧, and 🏃 Start
is the area view's Go to Start Location too (the same command).

## Menus

| Menu | Commands |
| --- | --- |
| File | New Module…, Open Module…, Open Folder…, Recent Modules, Save, Save As…, Save As nasher Project…, Project ([the project's commands](03-modules.md#the-projects-commands): its update and submit), Import…, Export…, Import Conversation…, Close, Exit |
| Edit | Undo, Redo (each names the module's step it takes, or, from a hak's or a 2DA's editor, that editor's: see Undo and redo under [Tabs and windows](#tabs-and-windows); with the keyboard in a script's editor they are the script's own and say plain Undo and Redo: see [Scripts](07-scripts.md)), Module Properties, Builder Code… ([your code](03-modules.md#your-builder-code), for a module folder or a nasher project), Claims Table… ([whose each area is](03-modules.md#claimed-areas), in a project that keeps claims), Resize Area…, Rotate Area…, Find Instance…, Prefabs, Edit Areas Together…, Find and Replace Text…, Find References…, Advanced Search…, Convert Indentation to Tabs, Convert Indentation to Spaces (the script in front: see [Scripts](07-scripts.md)) |
| View | Module Tree, Palettes Panel, Log (each ticked while shown), Copy Log, Save Log…, Clear Log, Hide All Panels, Reset Layout |
| Wizards | Area Wizard…, Creature Wizard…, and a wizard for each other blueprint type (Door, Encounter, Item, Placeable, Sound, Store, Trigger, Waypoint) |
| Tools | New Conversation…, Faction Editor, Journal Editor, Talk Table, 2DA File…, New Script…, Palettes, Appearance Gallery…, Resource Browser, Tilesets (New Tileset…, Open Tileset…), Haks (New Hak, Open Hak…, Build Hak from Folder…), Reload Resources, Options… |
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

**Help › Command Palette…** (Ctrl+Shift+P) finds a command by its name:
type part of it (or of its menu's name), choose with the arrow keys, and
Enter runs it; Escape closes. Each command shows its menu and its key. A
command that can't be chosen now is listed dimmed. The enabled plugins'
commands are among them, and the open project's (File › Project, under
Project). Any command can be given a key in Tools › Options › Keyboard.

**Help › Check for Updates…** (also a button in Help › About) asks
GitHub for Moonglow's latest release and says either that yours is the
newest or that a newer one is out, with a link to its release page in
your browser; drafts and pre-releases never count. Nothing is
downloaded or installed: you update as you installed. Offline, or when
GitHub refuses (it limits how often it is asked), the window says why.
What is sent is that one request and nothing else (no identifier, path
or anything about your modules). Once a day at most Moonglow also asks
by itself, in the background, unless Tools › Options › Updates says not
to; when a newer release is out, a line under the toolbar says so, with
the release page, **Don't remind me about this version** (a later
version is told again) and **×** to close it until the next check.
Where Flatpak, Scoop or Nix installed Moonglow, it doesn't ask by
itself, and the window says how that copy is updated
([Options](10-options.md#updates)).

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
| Ctrl+W, Ctrl+F4 | Close the tab in front |
| Ctrl+Tab, Ctrl+Page Down | Next tab |
| Ctrl+Shift+Tab, Ctrl+Page Up | Previous tab |
| Ctrl+Shift+T | Reopen the tab closed last |
| Ctrl+Shift+M | Maximize the window of the tab in front, or put it back |
| F6, Shift+F6 | The arrow keys to the next pane (the module tree, the middle, the palettes), or the one before: to what the pane's tab in front has (in the middle a talk table's lines, a 2DA's cells, at its first cell if none is selected, the resource browser's list, else an area's view; a palette at its first row) |
| Ctrl+Alt+1, Ctrl+Alt+2, Ctrl+Alt+3 | Show or fold away the module tree, the palettes, the log |
| Ctrl+Alt+0 | Hide all three, or show them all |
| Ctrl+Shift+P | Command Palette |

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

The left pane lists what the module holds, by kind: Areas, Conversations,
Scripts, the blueprints (Creatures, Doors, Encounters, Items,
Placeables, Sounds, Stores, Triggers, Waypoints) and the journal and factions;
**Module Properties** is at the top. **Filter** narrows every group to the
rows with every word typed in their name or ResRef, in any order, case
and accents aside (`arbol` finds `Árbol`, `sword iron` finds `Iron
Sword`, as every Filter and Find field of names does: the palettes', the
resource browser's, a list's, a wizard's; × clears it); it and Expand All / Collapse All stay in sight
while the tree scrolls, and Home and End go to the tree's top and bottom
with the pointer over it. A name too long for the pane goes on past
its edge, and the tree scrolls sideways to it (Shift and the mouse
wheel, or the scroll bar under it), as Aurora's does.

Resources are listed by their ResRefs, as in Aurora. With **Tools ›
Options › General › List areas and blueprints by name**, areas and
blueprints are listed by their names instead (in the names' order,
case and accents aside, so `Árboles` stands beside `arboles`, as
polytonic Greek's `Ἀθῆναι` beside `Αθηναι` and Russian's `ёлка` beside
`елка`; one without a name keeps its ResRef), areas' tabs are titled by name, the
filter finds either, and the ResRef shows when the pointer rests on one.
**Show ResRefs beside names** adds it in parentheses, `Name (resref)`,
there and in the palettes: for telling apart resources of one name.
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
Encounters…), sorted by name (its tag, without one). Waypoints are listed
by tag instead, as Aurora lists them (they often share a name, and their
tags are what scripts and transitions go by); the pointer resting on a
row shows the other. Click an object to go to it in the area's view,
selected (the tree stays still, so a second click makes a double click);
double-click for its Properties; right-click for **Go To**, for
**Preview** (creatures, doors, items and placeables, as placed),
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
an opened area's objects to those it finds.
**Edit** on the right-click menu of any other row (a conversation, a
script, a blueprint) opens it, as a double click does; a blueprint's
also has **Preview** (the model viewer).
A double click on a script opens its editor (and, with **Tools › Options
› Script Editor › Open scripts in the external editor**, your own editor
too).

**Expand All** and **Collapse All** (⏷ and ⏶, under the Filter, as
in the Palettes pane) open and close every group at once; with the keys
in the tree (or a palette), **Ctrl+Shift+Right** and **Ctrl+Shift+Left**
do the same (Options › Keyboard, under Module Tree and Palettes). Collapse All
also closes every area opened out to its objects, and the kinds under
it; Expand All leaves them closed (an area's objects can be hundreds of
rows). Beside them is how many resources the module has. The Filter says what it looks in,
and has a × to clear it.

A double click opens a resource in its editor. A blueprint dragged onto an
area's view is placed there, as from the palette. A right click on an area
offers **View Area** and the raw fields of its `.are` and `.git` files,
and last, as in Aurora, **Properties** (Area Properties); on a script,
area, conversation or blueprint, **Find References** and **Rename…** (see
[Modules](03-modules.md)); on any resource, **Copy…**, **Delete…**,
**Export…**, **Export as Files…** and **Copy to Scratch Folder**.

**Several resources at once:** Ctrl and a click adds a row to those
chosen (or takes it away), Shift and a click chooses those from the row
clicked before to it, as listed; they are marked. A right click on one
of them offers what is done to them all: **Edit N Together** (blueprints
of one kind, or areas: one Properties window, what you change set on
each), **Export…**, **Export as Files…**, **Copy to Scratch Folder**,
**Delete…** (and the Delete key on one of them), for areas **Make
Placeables Static** and **Dynamic**, and the plugins' commands for them.
A click alone chooses one row again.

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
base item and properties, an object's appearance, a trap's type, a loading
screen. A creature's skills are named in their list, and a talk-table
string's text is shown beside its number. An object's faction of 0 (a
blueprint's, or one an area places) is marked as the PC faction, the
players' alone, which Verify warns of (a trigger's, no). From a
terminal, `mg gff --names` names the rows the same way, and `mg find
--where` and `mg areas --set` take a row's label or name in place of its
number.

**The panes beside the middle** (the module tree on the left, the
palettes on the right, the log below) fold away to give the area's view
the room: **View** has each, ticked while shown, and **Hide All Panels**.
Each pane also has one button of its own for it: a small arrow on the
edge it faces the middle by, halfway along it (the module tree's right
edge, the palettes' left edge, the log's top edge). It points the way
the pane goes, and lights up under the pointer.
A pane folded away leaves a thin strip at that edge, with the same
button in the same place, pointing the other way: a click brings the
pane back, as wide as it was. Palettes you moved to the left of the
middle have their button on their right edge and leave their strip on
the left. Closing the palettes' pane by its ✕ folds it away the same.
What is folded stays so the next time Moonglow starts. **View › Reset
Layout** shows all three at the sizes they have at first.

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
as the row's menu does). At a placed object, Delete deletes it as its
menu's **Delete** does (Undo brings it back), and the keys go on to the
row after it; F2 does nothing there (its name and tag are in its
Properties). Ctrl+F (**Find**, under Script Editor in **Options › Keyboard**) goes to the
Filter; there, Escape clears what is typed, and Escape again goes back
into the rows, where they were; Down goes to the first row the Filter
finds (the palette's search has the same keys, and Down goes to its
first row). In the rows, Escape hands the keys back. The row the keys are at is marked, and brought into view.

## Tabs and windows

Areas open as tabs in the main pane. Everything else (blueprint and object
editors, scripts, conversations, previews, Module Properties, the Faction
and Journal Editors, the resource browser, this manual) opens in a window
of its own over the area, sized for what it shows, so the area view stays
put. Drag a window by its tab into the main pane, or beside another, to
dock it; drag a tab out to float it again. The palettes keep their pane on
the right. **Escape** closes the window in front: a dialog (as its Cancel
or close button does), then a Properties window as its **Cancel** does,
as in Aurora: what was changed in it is set back (Tile Properties with
changes asks first, as its close button does); a model's window while the
pointer is over it. In a text or number field too, Escape is the
window's Cancel at once, as in Aurora (what was typed there is set back
with the rest); in a search field of a Properties window (an inventory's
or a list's Find) it clears what is typed, then leaves the field, and
the next Escape is the window's. In a dialog's search field (Open
Script's name, a wizard's Find, Open 2DA, the Area chooser, the Command
Palette, a long dropdown's Filter) Escape clears what is typed, and the
next Escape closes the dialog (or the list). With something in hand in the area (a blueprint to place, a paste, a
terrain brush) and the pointer over the area, Escape lets go of that and
leaves the window open. Right-click a tab for **Eject** and **Close**,
for **Close Others**, **Close Tabs to the Right** and **Close All** (the
tabs of its pane), and for an area,
script, conversation or blueprint of the module, **Rename…**: it is renamed
everywhere the module names it (an area with its instances and its entry in
the module's area list), and its tab with it.

**The tab keys** work on the pane with the focus when it has more than
one tab, else on the main pane's: Ctrl+W (or Ctrl+F4) closes the tab in
front (File › Close, which closes the module, has no key until you give
it one in the Options: in Aurora it is Ctrl+F4),
Ctrl+Tab and Ctrl+Shift+Tab go round them (with **Ctrl+Tab goes to the
tab used last** on, in Options › General, in the order they were last
used for as long as Ctrl is held: one press goes back to the tab before,
and again returns), and Ctrl+Shift+T opens the tab
closed last again (the last twenty are kept; one whose area or script is
gone from the module is passed over). A middle click on a tab closes it.
A hak, a 2DA or a tileset with unsaved changes asks first, as its close
button does, and as the module asks: **Save**, **Don't Save** or
**Cancel**. Closing several at once (Close All) asks of each in turn;
Cancel keeps that one and the rest open.

**A window opens where one of its kind was last dragged to.** Drag a
window by its tab into the main pane (beside the areas' tabs) or into
the palettes' pane, and the next window of that kind (the next script,
the next conversation, the Faction Editor again) opens there as a tab,
not as a window over the area. Dropped on a side of the main pane (a
pane of its own split off its left, right, top or bottom), the next
opens in a pane split off that side again, as large as it was; the
palettes, split off another side than their right, come back there when
they are shown again. Drag one out to a window of its own and the next
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
**Double-click a window's tab** (or the bar beside it) to maximize
it over the whole of Moonglow's window under the toolbar, and again to
put it back; **Maximize** and
**Restore** are on the tab's right-click menu too, and the button at the
right of the window's bar, before its ✕, does it and shows which it is:
🗖 Maximize, 🗗 Restore. **Ctrl+Shift+M** (Maximize Window in Options ›
Keyboard) does it for the window of the tab in front. A maximized
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
rows after it in sight too. A tab whose row the Filter hides, chosen
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
window to its bar.

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
its grid), and never the module's last change there. So are Edit ›
Undo and Redo and the toolbar's, the pointer gone from the editor
straight to them: they name the step where the editor names its steps
("Undo Add files" over a hak's, "Undo Empty 1 cell" over a 2DA's; a
tileset's and the talk table's say plain Undo). A text field in one of
these editors (its Find field) takes back its own typing once you have
typed in it; clicked into and not typed in, it has nothing of its own,
and Undo is the editor's, as a field elsewhere leaves Undo to the
module's. A script's editor
keeps its own steps of its text: while it has the keyboard, Ctrl+Z and
Ctrl+Y, Edit › Undo and Redo and the toolbar's are all the script's
(see [Scripts](07-scripts.md)).

**Colors.** A color's swatch (an area's lighting, a light, a NUI color,
the Options' colors) opens a picker. Its 📋 copies the color's values
(`158, 151, 211`), and **Paste Color** under it, or **Ctrl+V** while it is
open or the pointer is on the swatch, takes one from the clipboard:
`158, 151, 211`, `#9e97d3`, `rgb(158, 151, 211)`, or values from 0 to 1.
The swatch's right-click menu has **Copy Color** and **Paste Color** too.

## The palette

The right-hand pane is Aurora's palette: choose a blueprint type (the
row of icons at its top),
**Standard** (the game's blueprints) or **Custom** (the module's), and a
blueprint from the categories. Click in an area to place the chosen
blueprint (see [Areas](04-areas.md)). With an area's terrain mode on, the
palette shows the tileset's brushes instead. The row's last two are
**Prefabs** and **Start Location** (where a player's character enters
the module: chosen, a click in an area puts it there). **Preview** (toolbar) shows
the chosen blueprint in a window of its own. After a click in it, the
arrow keys move through its rows and open and close its categories (see
[Blueprints](05-blueprints.md)).

## The resource browser

**Tools › Resource Browser** lists every resource the game sees in its
load order (the game's files, haks, override, the module), with the layer
each one comes from. Open one to view it (GFF files as a field tree, 2DA
tables, scripts and other text; models, and blueprints with **Preview**,
in the model viewer), copy it into the module, or save it to a file
(**Save As…**, also on its right-click menu in the list; a compiled
model shows as the text it compiles from, and **Save As Text…** saves
it so). **Export N as
Files…** writes all the resources listed into a folder, once a type or
part of a name narrows the list. Files go out as they are; with
**Models as text** ticked (shown when models are listed), compiled
models go out as the text they compile from.

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
name.hak** opens it there. With **Overridden copies** (at the top) unticked, it lists
the copy the game reads alone. **Export N as Files…** writes the copies
the game reads into the folder chosen, and each overridden copy listed
into a folder of its layer's under it (`hak_low/portraits.2da`). `mg which NAME.EXT --module
MODULE` says the same on the command line.

Its **Name** field has the side panes' keys: Ctrl+F, with the pointer
over the browser, goes to it; Escape clears what is typed, and Escape
again leaves it; Down goes into the list, where Up and Down (Page Up,
Page Down, Home, End) choose a row and Enter opens it. A click on a row
gives the list the keys too. The **Find** fields of the hak editor, the
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
and the Command Palette, Up and Down choose while you type); elsewhere
Down is the field's.

A 2DA shows the copy the game reads, read-only, with its StrRef columns
(Name, Description, StrRef and the like) as their text; **StrRefs as
numbers** shows the numbers. Its rows scroll as the 2DA editor's do, with
scroll bars at the right and the bottom, the column names staying at the
top and the rows' numbers at the left; a column's width is set by
dragging the edge at the right of its name (a double click there: as
wide as its widest cell), and is the 2DA editor's for that table too.
**Edit…** opens the table in the 2DA editor (see
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
lights it with its own light nodes. **Shaders** draws it with the game's
own shaders, a material's custom ones among them (wind-swayed leaves move
while the viewer plays): the same switch as the area view's toolbar
(Areas › The toolbar).

## The log

Messages go to the log at the bottom: what was opened or saved, compiler
errors, warnings about missing resources. Warnings are yellow, errors red.
Each line begins with the time it was written (`14:03:27`), as the
script editor's Compiler messages do; a line selected and copied from
the log takes its time with it. In its right corner, **Copy** (or View ›
Copy Log) puts the whole log on the clipboard and **Save…** (View › Save
Log…) writes it into a text file, each line with its time;
**Clear** (View › Clear Log) empties it: a second Verify's report isn't
then mistaken for the first's.

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
