---
type: Checklist
title: 'Where every control goes: today''s screens and their homes in the UI redesign'
description: The checklist implementation of the UI redesign works from - every control of today's Moonglow Toolset and Moonglow Viewer screens, grouped by screen as the three "Where Things Moved" boards group them, with its home in the redesign, what is new and what the mockups do not draw.
tags: [ui-redesign, mapping, checklist, toolset, viewer]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-17T21:32:50Z }
status: stable
stale_after: 2027-06-01T00:00:00Z
sources:
  - id: moves-boards
    resource: target/ui-mock/src/Moves.html, Moves2.html, Moves3.html (the mockup working files, not in git)
    title: The three "Where Things Moved" boards of the design canvas, converted table by table
  - id: canvas
    resource: https://claude.ai/artifact/A82Wjnuz9umTkAhsRDMWpj
    title: The design canvas (private to the owner), Overview page
---

# Where every control goes

This is the checklist for [the UI redesign](ui-redesign-proposal.md): every control of today's screens and the place it has in the redesign. It is the content of the three "Where Things Moved" boards of the design canvas (a private artifact of the owner; the pictures are there), converted to tables without dropping a row or rewording a cell beyond what Markdown needs (a `|` inside a cell is written `\|`).

How to read it:

- The boards have two columns, and so do the tables: **Today** names the control as the code names it, **In the redesign** says its home. Where a row says "Unchanged" the control keeps its place and behavior.
- Each screen has a **New** list (what the redesign adds that does not exist today; the full list across screens is in [the proposal](ui-redesign-proposal.md)) and, where the boards have one, **Not Drawn** (decided but not shown on any board, so the implementer works from the row and the rulings, not from a picture) and **Findings** (probable bugs seen in today's app).
- "Right Dock", "Left Dock", "Bottom Dock", "Top Bar", "Status Bar", "Outliner" and "Quick Open" are the shell's regions and panels, defined in the proposal.
- The three parts are ordered as the boards are: the first round (main window and the four editors of the first mock, the creature blueprint and the viewer), the second round, and the second round continued. A screen that appears twice (the viewer, the area editor) has a section in each part for the board that mocks it.

Implementation ticks a row when its control has reached the home named here and its test (or picture, compared with the board) says so. A table with a **Done** column says which task ticked each row (✓), and where a row's control is until its task.

## Part 1: Where Things Moved (the first round)

Every control of today's screens and its home in the mockups, by the names the code gives them. Left: today. Right: the mockup.


### Main Toolbar and Window

| Today | In the redesign | Done |
|---|---|---|
| New, Open, Save, Undo, Redo | Top Bar, icon buttons after the menus | ✓ M3 T2 |
| Properties | Module panel header button (Module Properties) | ✓ M3 T2 (named Module Properties in M3 T3) |
| New Area | Module panel header button | ✓ M3 T2 |
| Start | Area view, camera button under the compass; also in the View menu | View menu ✓ M3 T2; the button under the compass ✓ M4 U4 (Go to Start Location) |
| Resources | Left Dock tab beside Module | ✓ M3 T4 (Tools › Resource Browser shows it) |
| Palettes | Right Dock, Palette panel | ✓ M3 T5 (also Tools › Palettes, which shows the dock on it) |
| Preview | Right Dock, Preview panel | ✓ M3 T6 (the tab beside the Palette's; Tools › Preview brings it to the front, by the Preview Window command's id) |
| Compile, Verify | Top Bar, right end | ✓ M3 T2 |
| Test Module (F9) | Top Bar, right end, with Choose Character under its arrow | ✓ M3 T2 |
| The eight menus | Top Bar, unchanged | ✓ M3 T2 |
| Module tree: Filter, groups, Expand All, Collapse All | Module panel | ✓ M3 T3 (Expand All and Collapse All on its tab bar; each group's icon and count; the Areas group scrolls in its own well) |
| Log panel | Bottom Dock tab, opened by the Log switch in the Status Bar | ✓ M3 T2; T7: its last line beside the switch while the dock is closed, Copy Log, Save Log and Clear Log on its tab bar (they give way from the end in a narrow dock), closed for a new installation |
| View › Module Tree | View › Left Dock: it folds the dock on whichever panel it shows (Module or Resources), its key (Ctrl+Alt+1) as before | ✓ M3 T4 (its review: the boards name no row; the dock holds both panels) |
| View › Palettes Panel | View › Right Dock: it folds the Right Dock, its key (Ctrl+Alt+2) as before | ✓ M3 T5 (the boards name no row; the dock holds the Preview's tab too since T6, and is drawn beside an area's tab alone) |
| Status line: module path, game folder | Status Bar, right | ✓ M3 T2 (the module's name, its path in the tip); T7: a running job, its name and bar, before the module's name |
| Help › Command Palette… (Ctrl+Shift+P), a window over the toolset (no board draws it: the proposal's section 6.1 and the second rulings' third make it Quick Open) | Quick Open, the Top Bar's field "Go to resource or command" (Ctrl+P): resources and commands; Ctrl+Shift+P and Help › Command Palette… open it among the commands | ✓ M3 T8 (the window is gone) |
| The main pane's tabs: name, " *" after a script's, the talk table's or a side editor's with unsaved changes, egui_dock's close button on every tab | Document tabs: the kind's icon, the name, the modified dot (for an area, conversation, blueprint or Module Properties changed since the module was saved too), a close button on the tab in front and the one under the pointer; titles cut alike with their whole name in their tip where the bar is crowded | ✓ M3 T9 (the tab menu's Close is Moonglow's own, first of the close rows) |
| egui_dock's arrow at the left of the main pane's tab bar (folds a pane split above or below another to its tab bar) and its close all button at the bar's right (closes the pane's tabs; with Shift, a window's) | Gone from the editor groups' bars, which the boards draw with neither: the tab menu's Close All closes a group's tabs, and a group is made smaller by dragging its seam (no fold). A floating window's bar keeps both, as the Floating board draws them | ✓ M3 T9 (`vendor/egui_dock`: egui_dock decides them for every bar or none) |
| Panes split off the middle by dragging a tab to a side | Editor groups: the same, and **Split Editor** at the right end of the focused group's tab bar (also View › Split Editor, Ctrl+\\); the docks and the tab keys follow the focused group | ✓ M3 T9 |


### Area Editor

| Today | In the redesign |
|---|---|
| Show Creatures, Doors, Encounters, Items, Placeables, Sounds, Stores, Triggers, Waypoints, Start | Header, a group of icon toggles; the same eyes in the Outliner ✓ M3 T13 (the eyes: one state with the toggles; nine eyes, no row for Start, as the boards draw them); the header's joined group ✓ M4 U3 |
| All, None | The arrow at the end of that group ✓ M4 U3 (a menu; the Show popover's last rows in the compact header) |
| Snap, Turn | Header combos (magnet and rotate icons) ✓ M4 U3 (each as wide as its value) |
| Together, Onto Objects | Header icon toggles beside Turn ✓ M4 U3 |
| Night | Header, Day \| Night ✓ M4 U3 |
| Lighting, Shadows and their kinds and quality, Fog, Fade Geometry | Lighting popover ✓ M4 U3 (Lighting its switch; in the wide row one Shadows toggle with the kinds held or right-clicked) |
| Shaders, Grass, Tile Border, Animations | Lighting popover ✓ M4 U3 (in the wide board's order: Animations, Grass, Tile border, Shaders) |
| Grid, Minimap, Ground walkmesh, Object walkmeshes, Sound Ranges | Overlays popover, with a master switch ✓ M4 U3 (Walkmesh… folded into its two rows; one Walkmesh toggle in the wide row) |
| Sounds, Ambient, Music | Audio popover ✓ M4 U3 |
| Area Properties | Header, right end ✓ M4 U3 |
| To Scratch, Save as Prefab… | Header, More button ✓ M4 U3 (Select Tiles, Paint, Reorient Camera and Go to Start Location were there too until U4's shelf and compass) |
| Select Tiles (F10), Paint (B) | Tool shelf at the left of the 3D view ✓ M4 U4 (with Select Objects and Terrain; one tool on at a time, by the shelf, F10, B or a tile brush chosen) |
| Paint's second strip (Scatter, Row, Fill, Erase, Radius, Density and the rest) | Takes the header's left group while Paint is the active tool (not drawn) ✓ M4 U4 (the strip is gone; the Show popover in More meanwhile) |
| Reorient Camera, Go to Start Location | Buttons under the compass ✓ M4 U4 (Reorient Camera a command too: Ctrl+Alt+R, Quick Open) |
| "11 by 11 tiles, 192 objects" | Status Bar, right ✓ M3 T12 (two counts, "11 by 11 tiles" and "192 objects", after what is selected; they follow the active editor); "problems" and "N missing" a warning sign with its count at the header's right group's end ✓ M4 U3 (Part 2, decision 8) |
| Hint line | Status Bar, as key caps ✓ M3 T12 (each state's hints, a note of what is in hand or why a step was refused at their head; the line under the toolbar is gone) |
| The strip's right-click menu (which buttons to show) | The header's right-click menu ✓ M4 U3 (anywhere on the row; a hidden control gone from the row, its popover and the wide row) |
| Palette: type picker, Standard and Custom, search, List and Gallery, New…, More, Group By | Palette panel, in the same order ✓ M4 U6 (in the narrow dock the types' dropdown with Standard and Custom beside it, Expand All and Collapse All at the Group By row's end, that row drawn there with Category alone, grayed, where the project offers no grouping) |
| A placed object's properties window | Properties panel under the Palette, its pages as an icon rail, with Open in Tab ✓ M4 U6 (the Right Dock's lower section with every tool but Select Tiles; one object or several of one type, a mixed selection said; each change applied at once, one Undo step; a double click shows it in front, ruling Q1; Open in Tab and the context menu's Properties open the window with OK and Cancel; one page at a time, the window's) |


#### At 3440 by 1440

- The three popovers unfold into the header row. Day, Night, Lighting, Shadows, Fog, Grid, Minimap, Walkmesh and Sound Ranges carry their words. Fade Geometry is a combo. Animations, Grass, Tile Border, Shaders and the three Audio switches are icon toggles. ✓ M4 U3 (those that fit, from the left; the Overlays switch shows as an icon toggle in the row)
- The Right Dock stands two columns wide (Palette, Properties), a script is open in a second editor group, and the Log is open in the Bottom Dock. (M3 T14: the two columns are built with the Palette and the Preview, where the dock is wider than 560 in a window from 2560, 741 at first there; the Properties column is M4's area editor, S2. The Left Dock 320 and the Module panel 760 high there at first ✓ M3 T14.) ✓ M4 U6 (Palette and Preview as tabs in the first column, Properties in the second; the seam between them dragged, kept for the editor's kind, each column at least 280)

### Script Editor

| Today | In the redesign |
|---|---|
| Save, Compile | Header, icon and word ✓ M4 U12 (the first commands: into More last) |
| Save As…, Print…, Open…, External Editor | Header icons ✓ M4 U12 |
| Find…, Replace… | Header icons, their own group ✓ M4 U12 (between lines) |
| Bookmark | Header icon ✓ M4 U12 |
| Definition, References, Rename Symbol…, Used By | Header icons ✓ M4 U12 |
| To Scratch | Header, More button ✓ M4 U12 |
| Codepage label | Header, right end ✓ M4 U12 (quiet, then a line, More and OK and Cancel; into More where the row is short, after the left group's secondary commands, before Save and Compile) |
| "modified" label | The dot on the document tab ✓ M3 T9 (the dot) ✓ M4 U12 (the editor's own label gone with its header; nothing else read it) |
| Live error line under the toolbar | Status Bar message, and a wavy underline on the line ✓ M4 U13 (the compiler's words in red in the log's last line's place while the script is the docked tab in front, a click going to the line; the underline in `error` in either host, the error in its tip; the row under the header gone) |
| Functions, Variables, Constants, Templates and the filter | Right Dock, Reference panel ✓ M4 U12 (a segmented control over the filter; the Right Dock stays while a script's group has the focus) |
| Help tab (bottom) | Right Dock, Help under Reference ✓ M4 U12 (one tab bar with Reference under 1400; prose with a row per parameter and a line for each other line of a list; what is laid out in columns and code examples kept as written) |
| Compiler, Bookmarks, Search Results tabs (bottom) | Bottom Dock tabs, with switches in the Status Bar ✓ M4 U13 (Compiler and Bookmarks the script's own, following the script in front; Search Results global, its switch's count the lines found; tabs Compiler, Bookmarks, Search Results, Log, switches Log, Compiler, Bookmarks, Search Results; no Close on the tab bar: the switch closes the dock, M3's rule) |
| Key hints | Status Bar, as key caps ✓ M3 T12 (Find, Find next, Definition, References, Complete, Bookmark, Compile all, Replace, as bound, while a script's tab is the active editor; the line and column count is M4's) ✓ M4 U13 (the count "Line 17, column 12" at the bar's right) |


### Conversation Editor

| Today | In the redesign |
|---|---|
| Add, Copy, Cut, Paste, Paste As Link, Delete | Header, left (Add with its word) ✓ M4 U14 (the others icons, named and tipped; into More where the editor is narrow) |
| Move the line up, down | Header, arrow icons ✓ M4 U14 |
| Expand All, Collapse All | Header icons ✓ M4 U14 |
| Comments, Actions, Quests, Animations, Sounds (highlight) | Header, a group of icon toggles ✓ M4 U14 (the kit's joined group, "Highlight" in More) |
| Scripts | Header, word toggle ✓ M4 U14 |
| Export and its formats, Import Lines… | Header, right ✓ M4 U14 |
| Test | Header, right end; also in the Node panel header ✓ M4 U14 (the header's last before OK and Cancel, its end slot; the Node section's named "Test: click through the conversation", the board's) |
| Data and Search tabs (bottom pane) | Right Dock, Node panel tabs ✓ M4 U14 ("Node" the section's title before them, the board's; a floating conversation's in its window's right pane) |
| Bookmarks tab | Outliner, under the Module panel ✓ M4 U15 (a group "Bookmarks" with its count, a row a line, a click or Enter going to it; a floating conversation's at its window's left) |
| Bookmark button | Node panel header ✓ M4 U14 |
| Speaker tag, Text with its languages button and Token… | Node panel, Text section ✓ M4 U14 |
| Text Appears When…, Actions Taken (script, Script Wizard…, parameters, preview) | Node panel, a section each ✓ M4 U14 (Script Wizard… at each section's heading; Add Parameter, Add Set and the sets' … menu on the Parameters row) |
| Other Actions (animation, sound, journal), Comments | Node panel, a section each ✓ M4 U14 |
| Current File (Normal end, Aborted, Stop camera zoom in, word count) | Right Dock, Conversation panel under Node ✓ M4 U14 (the word count in its header) |
| Row context menu, drag to move, Ctrl+drag to link | Unchanged; the common ones are named in the Status Bar hints ✓ M4 U15 (Drag Move line, Ctrl + Drag Link, Double Click Go to linked, with Ctrl + A Add, Alt + Up Move up, * Open branch, Ctrl + F Find and the word count) |


### Creature Blueprint

| Today | In the redesign |
|---|---|
| The floating blueprint window | A document tab; its header carries the name, the resref, OK and Cancel ✓ M3 T10 (a document tab with Open editors As Tabs, the default of a new installation; its header is M4's) ✓ M4 U9 (the header: the kind's icon, the name, the resref quieter, OK and Cancel at its right end; Module and Area Properties the same) |
| The wrapped row of 14 page tabs | A page list at the left, in the same order; the pages follow one another on one scroll ✓ M4 U9 (each page with its icon, the page at the top filled, the others in view marked; each page under its bar, drawn only where in sight; a window too narrow for the words lists them as the Properties panel's icon rail) |
| Basic: First name, Last name (languages button, Random name), Tag | Identity section ✓ M4 U10 (the StrRef inside the name's field at its right end, the languages button and Random Name after it) |
| Race, Appearance with Gallery…, Phenotype, Gender, Preview | Race and Appearance section ✓ M4 U10 |
| Challenge rating, Category | Palette section ✓ M4 U10 |
| Portrait picture, its combo, Portraits… | Portrait section ✓ M4 U10 |
| Conversation with its picker, Edit, No interrupt | Conversation section ✓ M4 U10 |
| Description with its language button | Description section, full width ✓ M4 U10 (its heading names the language, the languages button at its right end) |
| Preview window: info line, Animation, Pause, Frame, Lights, Shadow, Shaders, Export Files… | Right Dock, Preview panel, one header row over the 3D view ✓ M4 U11 (the blueprint's tab brings its own Preview alone, a floating editor's in its window's right pane; the kit's `EditorHeader`, what has no room in its More, one row at 280; the info line in the panel's header, cut with "…" before its tools; the Creature board's five hints in the Status Bar) |
| The Appearance page's own preview | The same Preview panel ✓ M4 U11 (and a placeable's, a door's and an item's model beside their fields; Pop Out in the header's More, Bring Back in the panel while popped) |
| OK, Cancel | Editor header, right end, meaning unchanged ✓ M4 U9 |


#### At 3440 by 1440

- All 14 pages are in view at once, in five columns: Basic, Statistics, Appearance, Classes and Skills head the columns and the other pages stack under them. The page list keeps the real order and marks the pages in view. ✓ M4 U10 (as many columns of 480 or more as the room beside the list holds: six at 3440 today, the board's five beside a Right Dock as the board's, which the Preview panel, U11, brings ✓ M4 U11: five beside its 549; each page under the column that ends highest, so the stacking under the heads differs from the board's hand-laid one)

### Viewer

| Today | In the redesign |
|---|---|
| Menus File, Model, View, Help and every item | Top Bar, unchanged |
| Open…, Save, Decompile, Compile, Compile and View | Also Top Bar icon buttons |
| Resources panel (search, type filter, count, list) | Left Dock, top |
| Nodes panel | Left Dock, bottom, named Outliner |
| 3D View: Frame, the View combo | 3D View header, left |
| Grid, Axes | 3D View header, icon toggles |
| Overlays: Walkmesh, Wireframe, Normals, Nodes, Shadow casters | Overlays popover |
| Shaders, Shadow, Model lights, background color | 3D View header, icon toggles and a swatch |
| Texture tab | A tab beside 3D View, unchanged inside |
| ASCII editor: state text, Save, Compile, Compile and View | Editor header in the second editor group; the " *" becomes the tab's dot |
| Diagnostics panel | A strip under the ASCII text, with the error and warning counts |
| Inspector, Lighting | Right Dock, stacked |
| Animation panel | Bottom Dock, named Timeline: one transport row over a full-width scrub bar |
| Effects, Log | Bottom Dock tabs |
| Status line: document, origin, game folder | Status Bar, right; the 3D view's mouse and key actions as key caps |
| View menu: Ground Grid, Axes, Model Lights, Game Shaders, Shadow | Renamed to match the toggles: Grid, Axes, Model lights, Game shaders, Shadow |

## Part 2: Where Things Moved, Second Round

Every control of the screens mocked in the second round and its home in the mockups, by the names the code gives them. Left: today. Right: the mockup.


### Options

| Today | In the redesign | Done |
|---|---|---|
| Ten pages: Folders, Area, General, Script Editor, Conversation Editor, Sounds, Verify, Language, Keyboard, Updates | Page list at the left, styled like Creature's, with icons |
| OK, Cancel | Editor header, right end ✓ M4 U18 |
| Enter for OK, Esc for Cancel | Status Bar hints: Enter OK, Tab Next field, Shift+Tab Previous field; Esc cancels Options docked too (ruling R1), so the hints show Esc | Esc: ✓ M3 T10; the hints ✓ M3 T12 (with Esc Cancel where an Escape would cancel it; Enter OK while the pointer is over it and nothing has the keys) |
| Editors, Saving and Exporting, Opening Modules, Testing | Column 1, in the code's order | Editors first: ✓ M3 T10 (the columns are M4's) |
| Windows and Tabs, Module Tree and Palettes, Appearance | Column 2 |
| Questions, Warnings, Troubleshooting | Column 3 |
| Toolbar buttons | Retired: the Top Bar's buttons are icons with tooltips |
| Open Properties as tabs of one window, Closing a window cancels its changes, Move windows by dragging anywhere | Editors section, under the switch | ✓ M3 T10 (under "Floating windows", as the Options board) |
| Right Dock | None on this screen |


#### New

- Open editors: As Tabs | As Floating Windows, with a note that areas always open as tabs and moved kinds open where you last put one, until View › Reset Layout. ✓ M3 T10
- The page icons in the page list.
- The General page bar, as on Creature.

#### Findings for the Design Notes

- Today a kind of editor moved to a window is never stored, so it would be lost once As Tabs is the default. Settled in M3 T10: stored where editors open as tabs.
- Today Esc cancels Options only when it floats. Settled in M3 T10 (ruling R1): docked too.

### Floating Editors

| Today | In the redesign |
|---|---|
| A script in a window of its own (egui_dock surface) | A 1040 by 640 window over the main window, with a border, radius and shadow ✓ M3 T11 (the border, radius 4 and `--shadow`; the window opens at its kind's size as before) |
| The tab row as the window's bar, with the collapse arrow | Bar row: collapse arrow, tabs with ×, Maximize, Close ✓ M3 T11 (Collapse, Maximize and Close Window, named and tipped) |
| Double-click and right-click on the empty bar | Unchanged: Maximize or Restore ✓ M3 T11 |
| Tab menu: Eject, Close, Rename…, Show in Module Tree, Close Others, Close Tabs to the Right | Unchanged, not drawn |
| Script header, Functions, Variables, Constants, Templates | Script board's header, and Reference as a right pane inside the window ✓ M4 U12 (the pane narrows to half the window under 560, never dropped) |
| Help tab of the script's bottom pane | Under Reference, as on the Script board ✓ M4 U12 |
| Compiler, Bookmarks, Search Results | Bottom pane inside the window ✓ M4 U13 (the window's bottom pane under the text, at most a third of the window's height) |
| Resize corner | A grip at the bottom right ✓ M3 T11 |
| Main window behind | Area editor with Module Properties and an area as tabs; the Right Dock keeps Palette and Properties ✓ M3 (Module Properties and the area as document tabs, T9 and T10; the Right Dock's Palette, T5, with the Preview beside it, T6; its Properties panel is M4's, S2) ✓ M4 U6 (Properties under the Palette and the Preview, as the Floating board draws it) |


#### New

- The docks and Status Bar follow the active docked editor, and a floating editor brings its own panels. ✓ M3 (the docks T6, the Status Bar's hints and counts T12; a floating editor keeps its own panes inside its window, the plan's Part 2, decision 3)

### 2DA Table Editor

| Today | In the redesign |
|---|---|
| Save, Add Row with its count | Header, left ✓ M4 U16 (Save with its word, the first command, into More last; Add Row and its count joined, named "Add Row" whatever its count says) |
| Undo, Redo | The Top Bar's Undo and Redo, which act on the active editor (the Top Bar's act on the editor last worked in ✓ M3 T2, its fix round; the editor's own buttons go with its header, M4) ✓ M4 U16 (the editor's own gone; the Top Bar's, Edit's and Ctrl+Z reach the grid's and the Cell panel's steps, docked and floating) |
| Empty Row, Delete Row, Add Column… | Header, icon buttons ✓ M4 U16 (eraser, trash and column, named and tipped) |
| Find, Go to, "N cells are wrong" | Header ✓ M4 U16 (Find and Go to into More after the rest, before Save; the wrong cells a warning button) |
| Values as names, Strings as text | Header, right, icon toggles ✓ M4 U16 (joined as "How cells show"; Values as names where a description names values) |
| Save As File…, Take Rows…, Compact, Show Hidden Columns | More menu ✓ M4 U16 (More with its words; Show Hidden Columns says how many, and Reset All Widths with them, as the board's More names it) |
| OK, Cancel | Header, right end ✓ M4 U16 (the end slot, never yielding) |
| Info line: rows, columns, where it is kept | Status Bar ✓ M4 U16 ("840 rows, 59 columns, in the module"; in a hak, in a file; a table only looked at, whose copy, read-only) |
| "Described by …" line | Note in the Cell panel ✓ M4 U16 (at its foot: "Described by Moonglow (the game's spells.2da): 34 of 59 columns. A description of yours comes first."; with none, how to give one) |
| Cell inspector under the grid (value, list or flags choice, kind words) | Right Dock, Cell panel ✓ M4 U16 (Value typed in, kept by its cell; a list's values and the flags as rows of their own; the column's name and kind words as its heading; a floating table's in its window's right pane) |
| Find References to Row N and its results | Right Dock, References panel ✓ M4 U16 (under Cell; looked for on a thread of its own, stopped as another row is chosen; In the game's tables, In the module, each a link) |
| Column right-click menu | Stays on the column header. Hide Column is also the Outliner's eye ✓ M4 U16 |
| Keys | Status Bar hints ✓ M4 U16 (Enter Type, Shift + Click Select to, Delete Empty, Ctrl + C Copy, Ctrl + V Paste, Tab Next column, Esc Leave grid, while the grid has the keys; Tab now the grid's, not the first control's) |


#### New

- An Outliner of the columns with kind icon, kind words and a show or hide eye. ✓ M4 U16 (a click or Enter going to the column, the keys and letters along the columns, F6; a floating table's at its window's left, or a strip where the grid has not the room)
- The selection count ("6 cells") in the Status Bar. ✓ M4 U16

#### Not Drawn

- Talk Table, Choose String…, New Line…, Choose Model… and the row chooser, which appear in the Cell panel only for those column kinds. ✓ M4 U16 (each under the column's heading, at the panel's width)
- The read-only resource view with Copy to Module and Save As….

### Talk Table Editor

| Today | In the redesign |
|---|---|
| Find, Go to, Add Line, Find References | Header ✓ M4 U17 (Find and Go to first, into More last; Add Line with its word; Find References runs the References panel's look) |
| Undo, Redo | The Top Bar's Undo and Redo, which act on the active editor (the Top Bar's act on the editor last worked in ✓ M3 T2, its fix round; the editor's own buttons go with its header, M4) ✓ M4 U17 (the editor's own gone; the Top Bar's, Edit's and Ctrl+Z reach the list's and the String panel's steps, docked and floating) |
| Remove Last Line, Save | Header, icon buttons ✓ M4 U17 (minus and disk, named and tipped; off for a table only looked at) |
| Only lines with text | Header, icon toggle ✓ M4 U17 (the filter icon, at the right) |
| Export CSV…, Export JSON… | Export menu ✓ M4 U17 (its rows Export CSV… and Export JSON…; the game's dialog.tlk by its own numbers) |
| Import CSV…, Import JSON…, Open File…, New File…, Module's Table | More menu ✓ M4 U17 (More with its words; Module's Table where another table is shown; the imports off for a table only looked at) |
| OK and Cancel (Close when read-only) | Close in the header ✓ M4 U17 (the end slot: Close for a table only looked at, OK and Cancel otherwise) |
| Info line | Talk Table panel and Status Bar ✓ M4 U17 (Lines, Language, Feminine and File in the panel; "30 lines", "112228 lines, read-only" in the Status Bar) |
| Line editor under the list | Right Dock, String panel ✓ M4 U17 (Text, Feminine, Sound and Length; each burst of typing one step; the copy button at its header; read-only for a table only looked at; a floating table's in its window's right pane) |
| Draggable splitter | Gone, no longer needed ✓ M4 U17 (its kept shares, `talk_splits`, read without a fault and no longer written) |
| The page for a module with no talk table | Right Dock, Talk Table panel ✓ M4 U17 (its section Module's Table; the tab says where to make one) |


#### New

- A header row on the list. ✓ M4 U17 (StrRef, Text, Feminine where there is one, Sound; only the rows in sight laid out)
- dialog.tlk opened read-only and numbered by its own StrRefs. ✓ M4 U17 (in the Talk Table tab, the module's table waiting behind it, by the owner's ruling of 2026-11-17; from the Resources panel's row and Quick Open; its lines one copy made as it is shown, shared after, let go of as the tab shows another table or closes)
- The Talk Table panel as a home for the no-table page. ✓ M4 U17 (Module's Table, its section)

#### Not Drawn

- The Language dropdown (nasher projects only), which would sit in the Talk Table panel. ✓ M4 U17 (the panel's Language row, a dropdown where the project keeps several)

### NUI Designer

| Today | In the redesign |
|---|---|
| Design, Bindings, Events & variants pages | Page switch in the design group: Design, Bindings, Events & Variants, Images ✓ M4 U18 (the header's switch, first on its row, into More last) |
| Advanced › JUI source, Binding JSON | Page switch in the source group |
| Advanced › Images | Images, in the design group's page switch ✓ M4 U18 |
| Advanced › Opener Script | Icon button in the source group's header |
| Event Script… | Button in the source group's header |
| Editor, Preview only, Split source | View popover (✓ M4 U18: its Layout dropdown; the docks stay in each, as on every page, the coordinator's ruling in its review). Split source is the second editor group (U19) |
| Build & compile | Primary icon button, with a tooltip ✓ M4 U18 (Build & Compile, the hammer's accented button; its tip counts the errors that hold it off) |
| Export… | Icon button ✓ M4 U18 |
| Import…, How to open it… | More menu ✓ M4 U18 (Import…, How to Open It…, a dialog of its code to copy) |
| OK, Cancel | Editor header, right end ✓ M4 U18 |
| "Not built", "Build failed" | Status Bar, in the warning color |
| Checks toggle and its list | Checks switch with a count in the Status Bar, and the Checks panel in the Bottom Dock |
| Interact, Reset | Header, right side ✓ M4 U18 (Interact the play toggle, Reset off outside it) |
| UI scale, State, Resources, Clean view, Screen | View popover ✓ M4 U18 (UI scale, State and Screen dropdowns that open in it, Resources, Clean view and Preview on screen checks; Escape closes it alone) |
| Zoom: Fit, 100%, "Game preview" label | Overlays in the canvas corners |
| Canvas hint line | Status Bar key caps |
| Components palette | Right Dock, top panel ✓ M4 U18 (Find a control…, the components by group in title case; added to the control chosen; on every page) |
| Layers with search, Expand All, Collapse All | Outliner in the Left Dock ✓ M4 U18 (Find a layer or ID…, Expand All, Collapse All and Layer Actions on its header; the Window row's detail its title; only the rows in sight laid out; their keys; a floating designer's window's left pane or its strip) |
| Properties: binding, Content, Size & layout, Behavior, Events, Element actions | Properties panel in the Right Dock ✓ M4 U18 (the board's form: each label beside its field, over it only where the row has no room; bold section headings that fold; Width and Height on one row; Source with the plus that makes a bind; descriptions as tooltips; Add Property and the other adding buttons with the kit's drawn plus, named by their words; Property Binding first; Element Actions its header's `…`; a panel, not a menu, under 760 points) |
| Bindings, Create bind, Window identity, Client delivery | Bindings panel under Properties ✓ M4 U18 (as the Bindings page lists them, on the other pages; on the Bindings page the panel points to it; the table is U19's) |


#### New

- The View popover (✓ M4 U18) and the zoom overlay (U19).
- The source as a second editor group with its own page switch.
- A warning mark on the Outliner row, and IDs and bind names as weak text.
- A bind chip on a bound property, and Bindings as a table.

### Tileset Editor

| Today | In the redesign |
|---|---|
| Undo, Redo | The Top Bar's Undo and Redo, which act on the active editor (the Top Bar's act on the editor last worked in ✓ M3 T2, its fix round; the editor's own buttons go with its header, M4) |
| Save beside the pages | Editor header, left of the command group: it saves the tileset file |
| Five pages | Page switch in the header |
| Make Palette, Check, Render Minimap Pictures | Header |
| OK, Cancel | Right end of the header |
| Path line and counts | Status Bar |
| Check's findings | Problems panel in the Bottom Dock |
| Find, Duplicate Tile, Add Tile, Remove Last Tile | Bar above the tile list |
| Tile list | A table: #, Model, Corners, Edges |
| Model, Walkmesh, Minimap picture, Path node, Orientation, flags, corners, edges | The form at the right of the body |
| Doors grid, Add Door, Remove Last Door | Under the form |
| Preview button | Right Dock, Preview panel, with a button to open the model in a tab |
| Textures of the model | Right Dock, Textures panel |


#### New

- An Outliner of the tileset: General, Terrains, Crossers, rules, Tiles, Groups, with counts.
- The Preview diagram, and the door sentence under it.

### Area Editor, Paint

| Today | In the redesign |
|---|---|
| Paint toggle (B) | Tool shelf, on ✓ M4 U4 |
| Scatter, Row, Fill, Erase | Header, segmented control ✓ M4 U4 |
| Radius, Density | Header number fields ✓ M4 U4 (each where its mode takes it) |
| Count All, Spacing, By Size, Flow, Turn, Scale, Height, Lock | Brush popover ✓ M4 U4 (Count all and By size in sentence case, the board's) |
| Gap (Row and Fill only) | Brush popover in those modes, with a note ✓ M4 U4 |
| "N chosen" and its tip | Palette's Brush section, "2 chosen" and the tip as a note ✓ M4 U4 |
| Show toggles while Paint is on | The Outliner's eyes ✓ M3 T13 (they work with Paint on); the header's toggles leave the row with Paint on ✓ M4 U4 (the Show popover in More meanwhile, for Start, All and None, which have no eye) |
| Snap, Turn | Brush popover, under "Placing, as with the Select tool" ✓ M4 U4 |
| Together, Onto Objects | Back in the header when Select is the active tool ✓ M4 U4 |
| Paint hints | Status Bar: Drag Paint, Shift + Wheel Brush size, Q / E Turn, Right Click / Esc Stop ✓ M3 T12 (after "Painting 2 placeables", the board's lead; Erase, Fill and Row their own) |


#### New

- The palette's Brush section, with × to take a blueprint out of the brush (today only Ctrl+click). ✓ M4 U4
- The Snap and Turn copies in the Brush popover. ✓ M4 U4
- A note in the popover on which options each mode has. ✓ M4 U4

### Area Editor, Tiles

| Today | In the redesign |
|---|---|
| Select Tiles (F10) | Tool shelf, on ✓ M4 U4 (and U5: its state's commands take the header's left group) |
| Refine Tile (T) | Header, icon button ✓ M4 U5 (on the tiles selected, grayed with none; the palette's row stays a brush) |
| Previous and Next Tile Appearance | Header, joined ‹ Appearance › control ✓ M4 U5 (the kit's `Joined::words` between the two) |
| Rotate Tile (R) | Header, Rotate button ✓ M4 U5 (Shift + click: clockwise) |
| Copy, Cut, Paste of tiles | Header, three icon buttons ✓ M4 U5 (joined; Paste grayed until tiles of the tileset are copied) |
| Delete, Eraser (X) | Header, Delete button. The Eraser stays in the palette as a brush ✓ M4 U5 |
| Tile Properties… | Header toggle, and the lower Right Dock panel ✓ M4 U5 (the toggle kept in the settings; the tile menu's Tile Properties, without its ellipsis since the review, takes Select Tiles and shows the panel) |
| Put Down, plugin tile commands | Still the tile menu (right click) ✓ M4 U5 |
| Tile Border, Grid | View controls at the right, as in Main ✓ M4 U3 (the Lighting and Overlays popovers) |
| Tiles palette: search, List and Gallery, Exact Tile | Palette panel, in the same order ✓ M4 U5 (Exact Tile always there, grayed in the List with no tile chosen) |
| Tile Properties: lights, loops, Height, Replacement texture, Defaults, Textures | Properties panel, same fields ✓ M4 U5 (the Right Dock's lower section, "Lights and Loops", in sentence case) |
| Tile Properties OK, Cancel | Gone: the step is one undo step, Cancel becomes Ctrl+Z ✓ M4 U5 (each change one step on every tile chosen; a height dragged one step for the drag) |
| Tile hints | Status Bar: Click, Drag Box, Shift + Click, Ctrl + Click, Right Click ✓ M3 T12 (and Shift + Right Click Next tile, Shift + Drag Add a box; a refusal's notice first; "2 by 3 tiles selected" at the right) |


#### New

- The heading "Lights and Loops" and the "6 tiles" subtitle in Tile Properties. ✓ M4 U5 ("Tile (4, 4)" for one tile, "none selected" with none)
- The tileset name and counts on the Tiles palette folders. ✓ M4 U5 (and the Gallery's terrain and crosser headings their tiles' counts)
- The status count "3 by 2 tiles selected" and the key cap X beside Eraser. ✓ M3 T12 (the count), M4 U5 (the key caps, the tile tools' too)

#### Not Drawn

- Variant pictures, which today appear only for a single selected tile.

## Part 3: Where Things Moved, Second Round, Continued

Every control of the screens mocked in the second round and its home in the mockups, by the names the code gives them. Left: today. Right: the mockup.


### Window at 1280 by 800

| Today | In the redesign | Done |
|---|---|---|
| Compile, Verify, Test Module with words | Icon buttons with tooltips; Quick Open at 300 px | ✓ the icons M3 T2; Quick Open at 300 px, and its icon where it would be under 240, M3 T8 |
| Left Dock: Module, Resources, Outliner | Folded to a 24 px strip of icons; its chevron replaces the 16 px pane edge | The strip ✓ M3 T4, the Outliner's icon on it ✓ M3 T13; folded by the window's width under 1500 ✓ M3 T14 (unfolded by hand, by F6 or by View until the window is 1500 wide; `hide_tree` never written by a width; the strip's chevron is the button beside its edge, T5's) |
| The ten kind toggles and All, None | Show popover, "5 of 10" | ✓ M4 U3 (under an editor 990 wide) |
| Day \| Night | One moon toggle | ✓ M4 U3 |
| Lighting, Overlays, Audio, Area Properties | Icon buttons | ✓ M4 U3 (framed, the popovers' with their chevrons) |
| Palette type strip | Combo, "Placeables" | ✓ M3 T14 (a dropdown with the type's icon under 300 px of dock); ✓ M4 U6 (Standard and Custom beside it, the name's row gone there) |
| Expand All, Collapse All | End of the Group By row | ✓ M4 U6 (the row drawn in the narrow dock whether the project offers groupings or not) |
| Palette, Preview, Properties | Three tabs of one Right Dock panel | Palette and Preview tabs at 280 ✓ M3 T5, T6 and T14; ✓ M4 U6 (Palette, Properties, Preview; Properties' detail and Open in Tab at its own top there) |
| Log, Problems, Search Results switches | Icons only | The Log's ✓ M3 T14 (under 1400, and first of the bar's yields wherever the rest does not fit beside its word); Problems and Search Results come with their panels in M4; Search Results ✓ M4 U13 (its icon alone under 1400, after the Log's and the editor's own); Problems with its panel (U20) |
| Game folder in the Status Bar | Dropped at this width | ✓ M3 T14 (under 1400) |
| Menus, file buttons, Snap, Turn, More, shelf, compass | Unchanged | Menus and file buttons ✓ M3 T2 (the Top Bar's at every width); Snap, Turn and More ✓ M4 U3; the shelf and the compass ✓ M4 U4 (the shelf's tools that a short view has no room for in its More Tools) |


#### How the Shell Yields

- Top bar: below a window width of 1300, Compile, Verify and Test Module become icons and Quick Open is capped at 300 px. If it would fall under 240 px, it becomes an icon button.
- Left Dock: below 1500, it folds to its 24 px strip. The user can unfold it, and the compact header still fits.
- Right Dock: below 1400, it narrows from 320 to 280 px and its panels become tabs. Under 300 px of panel width, the palette's type strip becomes a combo.
- Header: below an editor width of 990, it goes compact. Below 550, Snap and Turn lose their value text. Below 490, Together and Onto Objects move into More. At 1280 the editor is 974 wide. ✓ M4 U3 (the area's header, `area_view::header`, by `Yields` of its own row's width)
- Status Bar: below a window width of 1400, the panel switches become icons. Then the game folder drops, then hints drop from the end. At 1280 all six hints fit.
- Built: the Top Bar's in M3 T2 and T8; the Left Dock's, the Right Dock's and the Status Bar's in M3 T14, from one `Yields` decided once a frame. T14 adds what no board draws: below 796 the Right Dock folds to its strip too (the editor area's least, 490, beside the Left Dock's strip), and below 962 a dock unfolded by hand folds the other. The header's (E 990, 550, 490) are M4's area header.

#### New

- The strip's panel icons and the "5 of 10" count.

### Welcome

| Today | In the redesign |
|---|---|
| Heading "Moonglow Toolset" and its line | Page heading, with the version |
| New Module…, Open Module…, Open Folder… | Start section, with icons and keys |
| Recent modules | Recent Modules list, the full path as a weak second line |
| "No game data…" warning, Options… | Status section |
| Update bar: What's New…, Install Now, Later | Status section |
| Left Dock | Not drawn: today it shows only with a module open ✓ M3 T4 (with no module it holds no panel and is not drawn; Tools › Resource Browser shows it with the Resources panel alone) |
| Build actions, Save, Undo, Redo | Drawn disabled ✓ M3 T2 (`the_top_bar_s_buttons_are_named_tipped_and_enabled_as_their_commands`) |
| "No area open" page (today's no_area) | A centered empty state with Area Wizard… and Go to Start; recent areas go to Quick Open and the Module panel |


#### New

- A "Without a module" group: Open Hak…, New Hak, Open Tileset…, New Tileset…, 2DA File…, Talk Table, Resource Browser, Appearance Gallery….
- Links to the manual's chapters, and a Help group: Command Palette…, Check for Updates…, About.
- A "Game data found" line.
- A note about dropping a module on the window.
- Status Bar hints for Ctrl+N, Ctrl+O and Ctrl+P. ✓ M3 T12 (as bound: a key changed shows changed, a command without one has no hint)

#### Not Drawn

- A "last opened" time on Recent Modules: the settings keep only paths.

### Hak Editor

| Today | In the redesign |
|---|---|
| "This hak is part of the game…" warning | Notice strip under the editor header |
| "2706 resources, 183.8 MB" | Details › Contents, and the Status Bar |
| Update from Folder | Details › Built from, and More |
| Compile Models | Editor header |
| Path and description memo | Details › File and Description |
| Add Files…, Add Folder…, Remove, Extract…, Extract All…, Save, Save As…, Find | Editor header, same names |
| Undo, Redo | The Top Bar's Undo and Redo, which act on the active editor (the Top Bar's act on the editor last worked in ✓ M3 T2, its fix round; the editor's own buttons go with its header, M4) |
| OK, Cancel | Editor header, right end ✓ M4 U18 |
| By name, By type, By size | Sortable column headers |
| Name, size and "from path" in each row | Columns Name, Size and Source |
| Click, Ctrl+click, Shift+click, double-click | Status Bar hints |
| Inline preview, Close, Play or Stop | Right Dock, Preview |


#### New

- An Outliner of the hak's contents by type with counts.
- A Type column and a sort arrow.
- Model facts in Preview: nodes, faces, vertices, textures, animations, supermodel.

#### Not Drawn

- The row's right-click menu and the 2 GiB warning.

### Resources

| Today | In the redesign | Done |
|---|---|---|
| Type dropdown, Name filter, Overridden copies | Left Dock, Resources tab, top of the panel | ✓ M3 T4 (Models as text under them, where models are listed) |
| Refresh | Icon in the panel header | ✓ M3 T4 |
| "N of M resources; double-click to open" | Count in the panel; the hint in the Status Bar | Count ✓ M3 T4; the hint ✓ M3 T12 (Double Click Open in the Status Bar while the panel is in use; the count's tip is gone) |
| Export N as Files… | Icon in the panel header | ✓ M3 T4 |
| List rows: name, layer, indented overridden copies | Same rows with a kind icon | ✓ M3 T4 (cut with an ellipsis at the dock's width, whole in the row's tip) |
| Up, Down, Page, Home, End, Enter, Esc | Status Bar hints | ✓ M3 T12 (Up or Down Move in the list and Enter Open while its list has the keys, Double Click Open, Right Click More, Find's key with the pointer over it: the Resources board's) |
| Read-only tab: Copy to Module, Save As… | Editor header, with a note | M4 |
| Tab title "name (read-only)" | Kept as the document tab's name | ✓ (as it was) |
| Read-only GFF tree | The GFF view's three columns, read-only |


#### New

- Names beside numbers in the GFF view, as mg gff --names gives them.
- Both tabs open in the same shell as Main.

#### Not Drawn

- Models as text (it shows only when a model is listed), the row context menu, and the Edit menu for 2DAs.

### Journal Editor

| Today | In the redesign |
|---|---|
| Add | Header word button |
| Copy, Cut, Paste, Delete | Header icon buttons |
| Expand All, Collapse All | Header icon buttons |
| OK, Cancel | Right end of the header |
| Root, category rows, entry rows | Tree rows; an End badge marks an entry that finishes |
| Entry fields: ID, Finish Category, Text, language | Right Dock, Entry panel |
| Category fields: Name, Tag, Priority, XP, Comments | Right Dock, Category panel |
| Hints | Status Bar: Click, Double Click, Ctrl+Z, Ctrl+Shift+Z |


#### New

- The Category panel shows the parent while an entry is selected.
- A subtitle on the Entry panel tab.

#### Not Drawn

- The "Select a category or an entry" message, and the Root or no selection state.

### Faction Editor

| Today | In the redesign |
|---|---|
| Basic, Advanced | Segmented control in the header |
| Advanced grid | The matrix filling the editor, with frozen row headers |
| Basic chart | Right Dock, Reactions panel (not drawn in Basic) |
| Full Detail | Checkbox in the Reactions panel |
| Factions list | Left Dock, Outliner |
| Add Faction…, Remove Faction, Change Name… | Header word buttons |
| Global Effect | Right Dock, Faction panel |
| Attitude words and the hover sentence | Reactions panel and cell tooltips |
| OK, Cancel | Right end of the header |


#### New

- A selectable cell with a Reaction panel: slider, sentence and the Hostile, Neutral, Friendly scale.
- A read-only Parent field (today only in Add Faction).
- Frozen row headers, and the hint "Select a reaction".

#### Not Drawn

- The Add Faction and Change Faction Name windows, which stay modal.
- The Basic chart view.

### Dialogs and Wizards

| Today | In the redesign |
|---|---|
| Window title, centered | Left-aligned in a bar row ✓ M3 T11 (moved by the row; a Close where Escape closes the dialog, the pickers' too; a job's window and a plugin's question the same row with no Close) |
| Button row, left-aligned | Footer row with a rule above, buttons at the right, a setting or count at the left |
| Main button | Accent fill. The order stays: action first, Cancel last |
| Wizard steps (Creature Wizard has nine) | Step list at the left, with a check on done steps |
| Wizard page | Page-header bar over each page |
| Find and Replace Text: Find, "Replace in N" | Footer |
| Delete from Module: "Don't ask…" checkbox | Footer, at the left |
| Build Module: Counted as Used | Stays under the results |
| Adjust Location, Build Module's missing resources | Flowed into columns |
| Wording: animation speed, X rotation | Sentence case |
| Rename, Unsaved Changes, Duplicate Tags, Area Wizard, Resize Area, Recover Unsaved Work | Drawn in the same pattern |


#### New

- The step list, the page header bar and the footer row.
- The accent fill on the main button.
- Wording fixes: "6 strings, 5 times" in place of "string(s)" and "time(s)".

#### Not Drawn

- Find Instance, Plugins, NWSync, Bake and Light maker.
- A dimmed backdrop: today's dialogs do not dim, so the board has none.
- Steps for the Area Wizard, which is one page today.

#### Dialogs That Could Become Dock Panels

- Find Instance: a results list searched again and again, a Search Results tab.
- Find and Replace Text: the hits in Search Results, the form as its header.
- Build Module: Results as Problems, the options as a panel.
- Plugins: a manager kept open, a Right Dock panel or a tab.
- Hak Pak Conflict Analysis, Resources Used, Claims Table, Areas Not Yours: Bottom Dock tabs.
- Edit Areas Together: a Properties-style panel.
- Light maker, Bake Walkmesh to Transform: the area editor's Right Dock.
- Stay dialogs: wizards, confirmations, Rename, Adjust Location, the pickers, NWSync publish.

### Viewer, Creature and Colors

| Today | In the redesign |
|---|---|
| Resources list filter and search | Creatures filter on its own row, "838 creatures" |
| Nodes | Outliner, a creature's real node tree |
| Overlays | Popover drawn open: Walkmesh, Wireframe, Normals, Nodes, Shadow casters |
| Texture tab | Second group: channel switch, Mip and Zoom, with a caption line |
| Creature: Gender, Phenotype, Skin, Hair, Tattoos, Head, Body parts, Wings, Tail | Inspector, Creature section, two-column grid |
| Color pickers | Swatch and combo, with a 16 by 11 grid when open |
| PLT colors: ten layers, Reset Colors | Inspector, two columns |
| Model section | Inspector, Model section |
| Effects: search, size, Clear, rows | Bottom Dock, Effects tab |


#### New

- Status Bar hints "Double-click: Apply effect" and "Ctrl + Wheel: Zoom texture".
- The texture caption line (today the name and info sit in a body row).

#### Not Drawn

- TXI text, since none of the shown textures has one.
- A list of animations beside the transport: the Timeline has only the combo.

### Viewer at 3440 by 1440

| Today | In the redesign |
|---|---|
| Resources and Nodes | Left Dock, 320 wide, taller lists |
| 3D View, Texture, ASCII | Three editor groups side by side, scaled to fit |
| Overlays popover | Unfolded into word toggles |
| Grid, Axes, Game shaders, Shadow, Model lights | Carry their words |
| Inspector, Log | Right Dock, column 1 |
| Lighting, Effects | Right Dock, column 2 |
| Animation | Bottom Dock, Timeline alone, with a long scrub bar |


### Galleries

| Today | In the redesign |
|---|---|
| Palette: the List and Gallery switch, the size slider, fold arrows, item count, New…, More | Palette toolbar, in the same order; the slider shows with Gallery on |
| Palette: Group by, category headings with counts | Unchanged; the pictures wrap inside each heading |
| Gallery cell: picture, name on two lines, favorite star, hover enlargement with ResRef and Tag | The shared picture cell; the hover enlargement as a tooltip beside the dock |
| Tiles palette: Exact Tile, terrain and crosser headings with their tiles as pictures | Tiles palette toolbar and list, the tile's model name under each picture |
| Tile Properties: All tiles (ignore fit), Variant (N fit here) with its Size slider, the variant grid | Tile Properties panel in the Right Dock; the Turn choices and search appear with All tiles on (not drawn) ✓ M4 U5 (the panel; its pictures today's until U25's shared cell) |
| Appearance Gallery window: Placeables, Creatures, Doors; hint line with Use the Selection; Find; size slider; count | A document tab: the kind switch, Find, slider and count in its header, the hint line as a strip under it |
| Appearance Gallery: click, double click, drag, right-click Give to the Selection | Unchanged; the common ones are Status Bar hints |
| Select Portrait: three radio choices, Race, Gender or Category, the portrait grid, preview, count, OK, Cancel | A dialog by the dialog pattern; the count at the footer's left |
| Select Texture: "To draw NAME with", Find, size slider, count, Interface textures too, grid, preview, OK, Cancel | A dialog by the dialog pattern |
| Armor part gallery: "Pictures of" and the sixteen parts, two rows of part pictures | On the item's Appearance page, as today, with the shared cell |
| Creature Wizard: appearance grid with Find an appearance, portrait grid with All portraits | The wizard's Appearance and Portrait step, with the step list |


#### New

- A Preview panel and an Appearance panel beside the Appearance Gallery.
- Counts on the Tiles gallery's terrain and crosser headings.
- A well with a scroll bar around a dialog's picture grid, and a close button on the two picker dialogs (the close button ✓ M3 T11).

