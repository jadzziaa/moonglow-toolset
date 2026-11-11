---
type: Checklist
title: 'Where every control goes: today''s screens and their homes in the UI redesign'
description: The checklist implementation of the UI redesign works from - every control of today's Moonglow Toolset and Moonglow Viewer screens, grouped by screen as the three "Where Things Moved" boards group them, with its home in the redesign, what is new and what the mockups do not draw.
tags: [ui-redesign, mapping, checklist, toolset, viewer]
generated: { by: claude-code/claude-sonnet-5-5, at: 2026-11-11T07:06:55Z }
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

Implementation ticks a row when its control has reached the home named here and its test (or picture, compared with the board) says so.

## Part 1: Where Things Moved (the first round)

Every control of today's screens and its home in the mockups, by the names the code gives them. Left: today. Right: the mockup.


### Main Toolbar and Window

| Today | In the redesign |
|---|---|
| New, Open, Save, Undo, Redo | Top Bar, icon buttons after the menus |
| Properties | Module panel header button (Module Properties) |
| New Area | Module panel header button |
| Start | Area view, camera button under the compass; also in the View menu |
| Resources | Left Dock tab beside Module |
| Palettes | Right Dock, Palette panel |
| Preview | Right Dock, Preview panel |
| Compile, Verify | Top Bar, right end |
| Test Module (F9) | Top Bar, right end, with Choose Character under its arrow |
| The eight menus | Top Bar, unchanged |
| Module tree: Filter, groups, Expand All, Collapse All | Module panel |
| Log panel | Bottom Dock tab, opened by the Log switch in the Status Bar |
| Status line: module path, game folder | Status Bar, right |


### Area Editor

| Today | In the redesign |
|---|---|
| Show Creatures, Doors, Encounters, Items, Placeables, Sounds, Stores, Triggers, Waypoints, Start | Header, a group of icon toggles; the same eyes in the Outliner |
| All, None | The arrow at the end of that group |
| Snap, Turn | Header combos (magnet and rotate icons) |
| Together, Onto Objects | Header icon toggles beside Turn |
| Night | Header, Day \| Night |
| Lighting, Shadows and their kinds and quality, Fog, Fade Geometry | Lighting popover |
| Shaders, Grass, Tile Border, Animations | Lighting popover |
| Grid, Minimap, Ground walkmesh, Object walkmeshes, Sound Ranges | Overlays popover, with a master switch |
| Sounds, Ambient, Music | Audio popover |
| Area Properties | Header, right end |
| To Scratch, Save as Prefab… | Header, More button |
| Select Tiles (F10), Paint (B) | Tool shelf at the left of the 3D view |
| Paint's second strip (Scatter, Row, Fill, Erase, Radius, Density and the rest) | Takes the header's left group while Paint is the active tool (not drawn) |
| Reorient Camera, Go to Start Location | Buttons under the compass |
| "11 by 11 tiles, 192 objects" | Status Bar, right |
| Hint line | Status Bar, as key caps |
| The strip's right-click menu (which buttons to show) | The header's right-click menu |
| Palette: type picker, Standard and Custom, search, List and Gallery, New…, More, Group By | Palette panel, in the same order |
| A placed object's properties window | Properties panel under the Palette, its pages as an icon rail, with Open in Tab |


#### At 3440 by 1440

- The three popovers unfold into the header row. Day, Night, Lighting, Shadows, Fog, Grid, Minimap, Walkmesh and Sound Ranges carry their words. Fade Geometry is a combo. Animations, Grass, Tile Border, Shaders and the three Audio switches are icon toggles.
- The Right Dock stands two columns wide (Palette, Properties), a script is open in a second editor group, and the Log is open in the Bottom Dock.

### Script Editor

| Today | In the redesign |
|---|---|
| Save, Compile | Header, icon and word |
| Save As…, Print…, Open…, External Editor | Header icons |
| Find…, Replace… | Header icons, their own group |
| Bookmark | Header icon |
| Definition, References, Rename Symbol…, Used By | Header icons |
| To Scratch | Header, More button |
| Codepage label | Header, right end |
| "modified" label | The dot on the document tab |
| Live error line under the toolbar | Status Bar message, and a wavy underline on the line |
| Functions, Variables, Constants, Templates and the filter | Right Dock, Reference panel |
| Help tab (bottom) | Right Dock, Help under Reference |
| Compiler, Bookmarks, Search Results tabs (bottom) | Bottom Dock tabs, with switches in the Status Bar |
| Key hints | Status Bar, as key caps |


### Conversation Editor

| Today | In the redesign |
|---|---|
| Add, Copy, Cut, Paste, Paste As Link, Delete | Header, left (Add with its word) |
| Move the line up, down | Header, arrow icons |
| Expand All, Collapse All | Header icons |
| Comments, Actions, Quests, Animations, Sounds (highlight) | Header, a group of icon toggles |
| Scripts | Header, word toggle |
| Export and its formats, Import Lines… | Header, right |
| Test | Header, right end; also in the Node panel header |
| Data and Search tabs (bottom pane) | Right Dock, Node panel tabs |
| Bookmarks tab | Outliner, under the Module panel |
| Bookmark button | Node panel header |
| Speaker tag, Text with its languages button and Token… | Node panel, Text section |
| Text Appears When…, Actions Taken (script, Script Wizard…, parameters, preview) | Node panel, a section each |
| Other Actions (animation, sound, journal), Comments | Node panel, a section each |
| Current File (Normal end, Aborted, Stop camera zoom in, word count) | Right Dock, Conversation panel under Node |
| Row context menu, drag to move, Ctrl+drag to link | Unchanged; the common ones are named in the Status Bar hints |


### Creature Blueprint

| Today | In the redesign |
|---|---|
| The floating blueprint window | A document tab; its header carries the name, the resref, OK and Cancel |
| The wrapped row of 14 page tabs | A page list at the left, in the same order; the pages follow one another on one scroll |
| Basic: First name, Last name (languages button, Random name), Tag | Identity section |
| Race, Appearance with Gallery…, Phenotype, Gender, Preview | Race and Appearance section |
| Challenge rating, Category | Palette section |
| Portrait picture, its combo, Portraits… | Portrait section |
| Conversation with its picker, Edit, No interrupt | Conversation section |
| Description with its language button | Description section, full width |
| Preview window: info line, Animation, Pause, Frame, Lights, Shadow, Shaders, Export Files… | Right Dock, Preview panel, one header row over the 3D view |
| The Appearance page's own preview | The same Preview panel |
| OK, Cancel | Editor header, right end, meaning unchanged |


#### At 3440 by 1440

- All 14 pages are in view at once, in five columns: Basic, Statistics, Appearance, Classes and Skills head the columns and the other pages stack under them. The page list keeps the real order and marks the pages in view.

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

| Today | In the redesign |
|---|---|
| Ten pages: Folders, Area, General, Script Editor, Conversation Editor, Sounds, Verify, Language, Keyboard, Updates | Page list at the left, styled like Creature's, with icons |
| OK, Cancel | Editor header, right end |
| Enter for OK, Esc for Cancel | Status Bar hints: Enter OK, Tab Next field, Shift+Tab Previous field (no Esc, it does nothing docked) |
| Editors, Saving and Exporting, Opening Modules, Testing | Column 1, in the code's order |
| Windows and Tabs, Module Tree and Palettes, Appearance | Column 2 |
| Questions, Warnings, Troubleshooting | Column 3 |
| Toolbar buttons | Retired: the Top Bar's buttons are icons with tooltips |
| Open Properties as tabs of one window, Closing a window cancels its changes, Move windows by dragging anywhere | Editors section, under the switch |
| Right Dock | None on this screen |


#### New

- Open editors: As Tabs | As Floating Windows, with a note that areas always open as tabs and moved kinds open where you last put one, until View › Reset Layout.
- The page icons in the page list.
- The General page bar, as on Creature.

#### Findings for the Design Notes

- Today a kind of editor moved to a window is never stored, so it would be lost once As Tabs is the default.
- Today Esc cancels Options only when it floats.

### Floating Editors

| Today | In the redesign |
|---|---|
| A script in a window of its own (egui_dock surface) | A 1040 by 640 window over the main window, with a border, radius and shadow |
| The tab row as the window's bar, with the collapse arrow | Bar row: collapse arrow, tabs with ×, Maximize, Close |
| Double-click and right-click on the empty bar | Unchanged: Maximize or Restore |
| Tab menu: Eject, Close, Rename…, Show in Module Tree, Close Others, Close Tabs to the Right | Unchanged, not drawn |
| Script header, Functions, Variables, Constants, Templates | Script board's header, and Reference as a right pane inside the window |
| Help tab of the script's bottom pane | Under Reference, as on the Script board |
| Compiler, Bookmarks, Search Results | Bottom pane inside the window |
| Resize corner | A grip at the bottom right |
| Main window behind | Area editor with Module Properties and an area as tabs; the Right Dock keeps Palette and Properties |


#### New

- The docks and Status Bar follow the active docked editor, and a floating editor brings its own panels.

### 2DA Table Editor

| Today | In the redesign |
|---|---|
| Save, Add Row with its count | Header, left |
| Undo, Redo | The Top Bar's Undo and Redo, which act on the active editor |
| Empty Row, Delete Row, Add Column… | Header, icon buttons |
| Find, Go to, "N cells are wrong" | Header |
| Values as names, Strings as text | Header, right, icon toggles |
| Save As File…, Take Rows…, Compact, Show Hidden Columns | More menu |
| OK, Cancel | Header, right end |
| Info line: rows, columns, where it is kept | Status Bar |
| "Described by …" line | Note in the Cell panel |
| Cell inspector under the grid (value, list or flags choice, kind words) | Right Dock, Cell panel |
| Find References to Row N and its results | Right Dock, References panel |
| Column right-click menu | Stays on the column header. Hide Column is also the Outliner's eye |
| Keys | Status Bar hints |


#### New

- An Outliner of the columns with kind icon, kind words and a show or hide eye.
- The selection count ("6 cells") in the Status Bar.

#### Not Drawn

- Talk Table, Choose String…, New Line…, Choose Model… and the row chooser, which appear in the Cell panel only for those column kinds.
- The read-only resource view with Copy to Module and Save As….

### Talk Table Editor

| Today | In the redesign |
|---|---|
| Find, Go to, Add Line, Find References | Header |
| Undo, Redo | The Top Bar's Undo and Redo, which act on the active editor |
| Remove Last Line, Save | Header, icon buttons |
| Only lines with text | Header, icon toggle |
| Export CSV…, Export JSON… | Export menu |
| Import CSV…, Import JSON…, Open File…, New File…, Module's Table | More menu |
| OK and Cancel (Close when read-only) | Close in the header |
| Info line | Talk Table panel and Status Bar |
| Line editor under the list | Right Dock, String panel |
| Draggable splitter | Gone, no longer needed |
| The page for a module with no talk table | Right Dock, Talk Table panel |


#### New

- A header row on the list.
- dialog.tlk opened read-only and numbered by its own StrRefs.
- The Talk Table panel as a home for the no-table page.

#### Not Drawn

- The Language dropdown (nasher projects only), which would sit in the Talk Table panel.

### NUI Designer

| Today | In the redesign |
|---|---|
| Design, Bindings, Events & variants pages | Page switch in the design group: Design, Bindings, Events & Variants, Images |
| Advanced › JUI source, Binding JSON | Page switch in the source group |
| Advanced › Images | Images, in the design group's page switch |
| Advanced › Opener Script | Icon button in the source group's header |
| Event Script… | Button in the source group's header |
| Editor, Preview only, Split source | View popover. Split source is the second editor group |
| Build & compile | Primary icon button, with a tooltip |
| Export… | Icon button |
| Import…, How to open it… | More menu |
| OK, Cancel | Editor header, right end |
| "Not built", "Build failed" | Status Bar, in the warning color |
| Checks toggle and its list | Checks switch with a count in the Status Bar, and the Checks panel in the Bottom Dock |
| Interact, Reset | Header, right side |
| UI scale, State, Resources, Clean view, Screen | View popover |
| Zoom: Fit, 100%, "Game preview" label | Overlays in the canvas corners |
| Canvas hint line | Status Bar key caps |
| Components palette | Right Dock, top panel |
| Layers with search, Expand All, Collapse All | Outliner in the Left Dock |
| Properties: binding, Content, Size & layout, Behavior, Events, Element actions | Properties panel in the Right Dock |
| Bindings, Create bind, Window identity, Client delivery | Bindings panel under Properties |


#### New

- The View popover and the zoom overlay.
- The source as a second editor group with its own page switch.
- A warning mark on the Outliner row, and IDs and bind names as weak text.
- A bind chip on a bound property, and Bindings as a table.

### Tileset Editor

| Today | In the redesign |
|---|---|
| Undo, Redo | The Top Bar's Undo and Redo, which act on the active editor |
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
| Paint toggle (B) | Tool shelf, on |
| Scatter, Row, Fill, Erase | Header, segmented control |
| Radius, Density | Header number fields |
| Count All, Spacing, By Size, Flow, Turn, Scale, Height, Lock | Brush popover |
| Gap (Row and Fill only) | Brush popover in those modes, with a note |
| "N chosen" and its tip | Palette's Brush section, "2 chosen" and the tip as a note |
| Show toggles while Paint is on | The Outliner's eyes |
| Snap, Turn | Brush popover, under "Placing, as with the Select tool" |
| Together, Onto Objects | Back in the header when Select is the active tool |
| Paint hints | Status Bar: Drag Paint, Shift + Wheel Brush size, Q / E Turn, Right Click / Esc Stop |


#### New

- The palette's Brush section, with × to take a blueprint out of the brush (today only Ctrl+click).
- The Snap and Turn copies in the Brush popover.
- A note in the popover on which options each mode has.

### Area Editor, Tiles

| Today | In the redesign |
|---|---|
| Select Tiles (F10) | Tool shelf, on |
| Refine Tile (T) | Header, icon button |
| Previous and Next Tile Appearance | Header, joined ‹ Appearance › control |
| Rotate Tile (R) | Header, Rotate button |
| Copy, Cut, Paste of tiles | Header, three icon buttons |
| Delete, Eraser (X) | Header, Delete button. The Eraser stays in the palette as a brush |
| Tile Properties… | Header toggle, and the lower Right Dock panel |
| Put Down, plugin tile commands | Still the tile menu (right click) |
| Tile Border, Grid | View controls at the right, as in Main |
| Tiles palette: search, List and Gallery, Exact Tile | Palette panel, in the same order |
| Tile Properties: lights, loops, Height, Replacement texture, Defaults, Textures | Properties panel, same fields |
| Tile Properties OK, Cancel | Gone: the step is one undo step, Cancel becomes Ctrl+Z |
| Tile hints | Status Bar: Click, Drag Box, Shift + Click, Ctrl + Click, Right Click |


#### New

- The heading "Lights and Loops" and the "6 tiles" subtitle in Tile Properties.
- The tileset name and counts on the Tiles palette folders.
- The status count "3 by 2 tiles selected" and the key cap X beside Eraser.

#### Not Drawn

- Variant pictures, which today appear only for a single selected tile.

## Part 3: Where Things Moved, Second Round, Continued

Every control of the screens mocked in the second round and its home in the mockups, by the names the code gives them. Left: today. Right: the mockup.


### Window at 1280 by 800

| Today | In the redesign |
|---|---|
| Compile, Verify, Test Module with words | Icon buttons with tooltips; Quick Open at 300 px |
| Left Dock: Module, Resources, Outliner | Folded to a 24 px strip of icons; its chevron replaces the 16 px pane edge |
| The ten kind toggles and All, None | Show popover, "5 of 10" |
| Day \| Night | One moon toggle |
| Lighting, Overlays, Audio, Area Properties | Icon buttons |
| Palette type strip | Combo, "Placeables" |
| Expand All, Collapse All | End of the Group By row |
| Palette, Preview, Properties | Three tabs of one Right Dock panel |
| Log, Problems, Search Results switches | Icons only |
| Game folder in the Status Bar | Dropped at this width |
| Menus, file buttons, Snap, Turn, More, shelf, compass | Unchanged |


#### How the Shell Yields

- Top bar: below a window width of 1300, Compile, Verify and Test Module become icons and Quick Open is capped at 300 px. If it would fall under 240 px, it becomes an icon button.
- Left Dock: below 1500, it folds to its 24 px strip. The user can unfold it, and the compact header still fits.
- Right Dock: below 1400, it narrows from 320 to 280 px and its panels become tabs. Under 300 px of panel width, the palette's type strip becomes a combo.
- Header: below an editor width of 990, it goes compact. Below 550, Snap and Turn lose their value text. Below 490, Together and Onto Objects move into More. At 1280 the editor is 974 wide.
- Status Bar: below a window width of 1400, the panel switches become icons. Then the game folder drops, then hints drop from the end. At 1280 all six hints fit.

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
| Left Dock | Not drawn: today it shows only with a module open |
| Build actions, Save, Undo, Redo | Drawn disabled |
| "No area open" page (today's no_area) | A centered empty state with Area Wizard… and Go to Start; recent areas go to Quick Open and the Module panel |


#### New

- A "Without a module" group: Open Hak…, New Hak, Open Tileset…, New Tileset…, 2DA File…, Talk Table, Resource Browser, Appearance Gallery….
- Links to the manual's chapters, and a Help group: Command Palette…, Check for Updates…, About.
- A "Game data found" line.
- A note about dropping a module on the window.
- Status Bar hints for Ctrl+N, Ctrl+O and Ctrl+P.

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
| Undo, Redo | The Top Bar's Undo and Redo, which act on the active editor |
| OK, Cancel | Editor header, right end |
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

| Today | In the redesign |
|---|---|
| Type dropdown, Name filter, Overridden copies | Left Dock, Resources tab, top of the panel |
| Refresh | Icon in the panel header |
| "N of M resources; double-click to open" | Count in the panel; the hint in the Status Bar |
| Export N as Files… | Icon in the panel header |
| List rows: name, layer, indented overridden copies | Same rows with a kind icon |
| Up, Down, Page, Home, End, Enter, Esc | Status Bar hints |
| Read-only tab: Copy to Module, Save As… | Editor header, with a note |
| Tab title "name (read-only)" | Kept as the document tab's name |
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
| Window title, centered | Left-aligned in a bar row |
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
| Tile Properties: All tiles (ignore fit), Variant (N fit here) with its Size slider, the variant grid | Tile Properties panel in the Right Dock; the Turn choices and search appear with All tiles on (not drawn) |
| Appearance Gallery window: Placeables, Creatures, Doors; hint line with Use the Selection; Find; size slider; count | A document tab: the kind switch, Find, slider and count in its header, the hint line as a strip under it |
| Appearance Gallery: click, double click, drag, right-click Give to the Selection | Unchanged; the common ones are Status Bar hints |
| Select Portrait: three radio choices, Race, Gender or Category, the portrait grid, preview, count, OK, Cancel | A dialog by the dialog pattern; the count at the footer's left |
| Select Texture: "To draw NAME with", Find, size slider, count, Interface textures too, grid, preview, OK, Cancel | A dialog by the dialog pattern |
| Armor part gallery: "Pictures of" and the sixteen parts, two rows of part pictures | On the item's Appearance page, as today, with the shared cell |
| Creature Wizard: appearance grid with Find an appearance, portrait grid with All portraits | The wizard's Appearance and Portrait step, with the step list |


#### New

- A Preview panel and an Appearance panel beside the Appearance Gallery.
- Counts on the Tiles gallery's terrain and crosser headings.
- A well with a scroll bar around a dialog's picture grid, and a close button on the two picker dialogs.

