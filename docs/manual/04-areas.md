---
type: Manual Page
title: Areas
description: Areas - the viewer's toolbar, minimaps and camera, selecting, arranging and placing objects, painting terrain, tiles, Area Properties, area sounds and making tilesets.
tags: [manual, areas, terrain, tiles]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-03T23:47:13Z }
---

# Areas

Double-click an area in the module tree (or right-click it › **View
Area**) to open it in the **area viewer**, which draws its tiles and
objects with the area's lighting, fog and sky, by day or night. Areas open
looking straight down at their middle, north up, as in Aurora.

## The toolbar

- **Creatures, Doors, … Waypoints, Start**: show or hide each kind of
  object and the start location marker; **All** and **None**.
- **Night**, **Fog**, **Grid**: show the area at night, with its fog, and
  the tile grid. The fog is drawn as the game's camera (at most 20 m from
  the player) would see it from where the view looks, so a view from
  farther off isn't lost in it.
- **💡 Lighting**: the area's lighting. Switched off, everything is evenly
  lit, whatever the area's colors and lights: for working in a dark area.
- **🔊 Sounds**, **Ambient**, **Music**: play the placed sound objects, the
  area's ambient sound and its music, heard from where the camera looks.
- **◎ Sound Ranges**: two circles around each placed sound that is heard
  from where it stands: at full volume inside the inner one (its minimum
  distance), not at all outside the outer one (its maximum). The
  selection's are brighter; a sound heard everywhere in the area has none.
- **Walkmesh**: the ground's walkmesh over the area (Aurora's Render AABB
  Nodes): walkable faces green, the others red.
- **Object Walkmeshes**: where placeables (their `.pwk`, orange) and doors
  (their `.dwk`, blue, for the state they're placed in: closed or open)
  keep creatures out. The selection's are brighter.
- **Select Tiles**: select tiles rather than objects (Aurora's Select
  Terrain).
- **Area Properties**, **Reorient Camera** (north up again), **Go to Start
  Location**.
- **To Scratch**: copy the area as it is now (its `.are`, `.git` and
  `.gic`) into the scratch folder (chosen the first time; Tools › Options
  › Folders changes it).

## Minimaps

**Export Minimap…** (an area's right-click menu in the module tree) saves
the area's map as a PNG, as the game's map draws it: each tile's picture
(the tileset's `ImageMap2D`) turned as the tile is, north up, unsaved
tiles included. A tile without a picture is black, as in the game. `mg
minimap` does the same from the command line. Each picture keeps its own
size unless `--size` sets one; the game's are mostly 16 pixels.

## The camera

Aurora's bindings, plus a few of Moonglow's (change the keys in Tools ›
Options › Keyboard):

| Do | To |
| --- | --- |
| Ctrl + drag | move the camera over the area |
| Right drag, or middle drag | turn the camera (W A S D move it meanwhile) |
| Shift + middle drag | move the camera |
| Wheel | zoom (slowly with Shift or Ctrl; when painting tiles, Shift is the brush's) |
| Arrow keys, W A S D, or numpad 4, 6, 8, 2 | move the camera |
| Numpad 7, 9 | turn |
| Numpad 1, 3 | tilt |
| Z, C | move the camera up, down |
| Ctrl + Shift + wheel, or Ctrl + Shift + middle drag | move the camera up and down, finely |
| Numpad 5 | look straight down at the whole area (and back to the ground) |
| F10 | select tiles or objects (Aurora's) |
| Double-click an object in Find Instance | go to it |

The keys drive the area view the pointer was in last, also while the
pointer is over the palette or the module tree (not while you type in a
field).

## Selecting and arranging objects

| Do | To |
| --- | --- |
| Click | select an object |
| Ctrl + click | add it to the selection, or take it out |
| Drag on the ground | select the objects in the box |
| Drag the selection | move it over the ground |
| Shift + right drag | turn the selected objects |
| Q, E | turn them 15° left or right (or by the snapping angle); with Shift, 90° |
| G | drop them to the ground |
| Alt + drag | raise or lower them (not creatures, which stand on the ground) |
| Delete | delete them |
| Ctrl+C, Ctrl+X, Ctrl+V | copy, cut, paste (in this area or another) |
| Double-click | the object's Properties |

Every move, turn and deletion is one undoable step. The view's corner
shows the pointer's position in the area, to the centimeter.

**Snapping** (the toolbar's **Snap** and **Turn**):
- **Snap** moves objects to a grid of 0.25 to 5 m as you drag, place or
  paste them.
- **Turn** turns them in steps of 5° to 90°.

With several selected, the first selected snaps and the others keep their
places and turns around it.

**Right-click** the selection for its menu:

- **Properties**: the object's properties as placed in the area (an
  instance can differ from its blueprint). With several objects of one
  kind selected, one window edits them together: a field changed there
  changes in all of them.
- **Adjust Location…**: an exact position and bearing, and the visual
  transform (scale and offset) that EE added.
- **Drop to Ground** (G): puts raised objects back on the ground under
  them.
- **Arrange** (several selected; the first selected leads):
  - **Line Up West–East** or **South–North**: on a line through the first.
  - **Space Evenly**: evenly spaced between the two farthest apart.
  - **Face Alike**: facing as the first does.
  - **Mirror West–East** or **South–North**: mirrored about the
    selection's middle, facings too.

  Objects keep their height above the ground.
- **Lock**: locked objects can't be selected by a click or a box, so you
  can work around them. **Unlock All** (on any part of the area) frees
  them. Moonglow marks a locked object with a field of its own
  (`MG_Locked`), which the game ignores. Aurora drops it when it saves,
  unlocking the object.
- **Save as Prefab…**: the selected objects, kept under a name to place
  again (see below).
- Doors: **Reverse Door**, **Initial State** (open or closed, locked).
  Placeables: **Initial State** (open, active…). Sounds: **Mute** or
  **Turn On**. Triggers and encounters: **Redraw Polygon**; encounters:
  **Add Spawn Point** (at the point you right-clicked).
- **Conversation**, **Inventory** (creatures, placeables with an
  inventory, merchants), **Variables…**.
- **Add to Palette**: the instance as a new custom blueprint.
- **Test From Here**: starts the game at the point you right-clicked,
  facing the way the camera looks, with the module as it is now (see
  [Build, verify and test](09-build-and-test.md)).
- **Create Waypoint**: a waypoint where the object stands. **Create Set…**
  (waypoints): name the selected waypoints `<name>_01`, `<name>_02`…
- **Levelup Wizard…** (creatures), **Setup Store…** (a creature or
  placeable as a shopkeeper: its conversation, a store and the script that
  opens it), **Add Popup Text…** (a placeable that speaks a line when
  clicked).

## Placing objects

Choose a blueprint in the palette and click in the area to place it, or
drag it onto the area from the palette or the module tree. Until it is
placed, a see-through copy in a blue box follows the pointer where it
would go (on the snapping grid; a door on the nearest hook). **Q** and
**E** turn it as they turn a selection (Shift + Q and E by 90°), and it is
placed facing that way; a door faces as its hook does. Shift + click
places it and keeps it chosen for another; a right click or Escape lets
it go. Doors go on a tile's door hook when you click near one. Triggers
and encounters are drawn point by point: click each corner, double-click
to close the outline.

**Edit › Find Instance…** lists the placed objects across the module by
kind, area, blueprint and tag; double-click one to go to it.

**Prefabs** are groups of placed objects saved under a name (**Save as
Prefab…** on the selection), such as a camp, a market stall or a furnished
room.
- **Placing one:** choose it under **Edit › Prefabs**. It follows the
  pointer like a paste. A click places it, with the objects in their
  places around each other and at their heights above the ground.
- **Any area, any module:** prefabs are kept in Moonglow's data folder, in
  `prefabs` (see [Troubleshooting](13-troubleshooting.md)). Copy the
  `.prefab.json` files to share them.
- **Blueprints:** an object keeps everything it had when it was saved,
  and works without the blueprint it came from.

## Painting terrain

Choose **Tiles** in the palette pane to paint the area with its tileset,
as Aurora's Terrain tab does:

- **Terrain brushes** (grass, water, cliffs… whatever the tileset has)
  and **Raise/Lower** act on the tile corner nearest the pointer; the
  right button lowers. Drag to mark every corner the pointer passes
  (yellow; running the drag back unmarks them), or Shift + drag to mark
  the whole rectangle from where the drag began. Letting go paints them
  all, and one undo takes the whole drag back. The cursor shows the four
  tiles a stroke changes.
- **Crossers** (roads, streams, walls) are dragged: they follow the
  pointer through the tiles it passes. A drag straight across a tile may
  wander up to 2.5 m off its middle. To turn within a tile, head for the
  side it should leave by. Running the drag back over its path lets go of
  what it passed. Shift + drag lays it round the outline of the rectangle
  from the tile where the drag began (straight along a rectangle one tile
  wide). Right-click a quarter the crosser already crosses (the
  cursor is blue there) to take that crosser off the tile, as the Eraser
  would, leaving other crossers (a road goes, the stream it crosses
  stays) and the brush chosen.
- **Groups** (buildings, big features) are placed whole; right-click to
  turn one before placing it. It stays chosen, to place another.
- The **Eraser** takes the crossers off a tile; Shift + click steps the
  tile through the other tiles that fit there. Dragged, it marks the
  tiles it passes, as a terrain brush marks corners (run back, Shift for
  a rectangle), and erases them when you let go.
- **Refine Tile** (Moonglow's own) steps the tile you click through the
  other tiles that fit there, whatever it holds, and never paints.
- The Eraser, Refine Tile and Raise/Lower head the Terrain list, whatever
  the tileset's order.

Under the pointer, the area shows the tiles a click would make, slightly
see-through, in place of those they replace: a feature or group, a
terrain's or a raised corner's tiles, the Eraser's or Refine Tile's
choice. While you drag a terrain, a crosser or the Eraser, it shows what letting go
now would paint. The click or drag puts down exactly the tiles shown:
where several fit, the one shown.

The cursor is green where a click paints and red where the tileset
refuses it. It is blue where a click only chooses tiles again: a crosser
over a quarter it already crosses, the Eraser with Shift, Refine Tile, and
a corner of the brush's own terrain (with Shift, the next tiles that fit).

Each stroke is one undoable step. Moonglow paints as Aurora does: the
same strokes give the same tiles, heights and crossers.

## Tiles

With **Select Tiles** on, a click selects a tile, Ctrl + click adds one,
and a drag selects a box of them. **Delete** takes the selected tiles'
crossers away. **Shift + right click** steps the tile under the pointer
through the tiles that fit. Ctrl+C and Ctrl+V copy and paste tiles.

A right click opens the tile menu with **Tile Properties**:
- the tiles' main and source light colors and their animation loops, as
  the tile's model has them (**Defaults** puts back the lighting
  scheme's);
- the **Replacement Texture**: what a texture named `replace_tex` in the
  tile's model is drawn with (a `replacetexture.2da` row; the game keeps
  it, though Aurora has no field for it).

**Edit › Resize Area…** grows or shrinks the area at its north and east
edges; **Edit › Rotate Area…** turns it by 90° steps, objects and all.
**Build › Area Statistics** counts the area's tiles and objects and the
memory its models take.

## Area Properties

Right-click the area in the module tree › **Properties**, or **Area
Properties** on the viewer's toolbar:

- **Basic**: name, tag, the tileset (fixed once made), the lighting
  scheme, the load screen, Variables.
- **Visual**: day and night (or always one), the sun and moon colors,
  fog colors and amounts, shadows, the skybox, lightning, rain and snow
  chances, as Aurora's Customize Environment sets them.
- **Audio**: day and night ambient sounds and their volumes, music, battle
  music, the environment's sound effects.
- **Events**, **Advanced** (flags such as interior, underground, natural,
  no rest, PvP; **Shader Flags**, the area flags past those three, which
  custom shaders read from their `areaFlags` uniform), **Comments**.

### Several areas together

**Edit › Edit Areas Together…** (or an area's right-click menu in the
module tree) lists the module's areas to tick. Narrow the list by a name
or ResRef, by tileset, and by kind (interior or exterior, above or under
ground, natural or artificial); **Tick Shown** ticks what the filters
leave. **Edit N Together** opens one Area Properties for the areas
ticked, shown as the first of them:

- A change is set on each area, as one undoable step: the lighting scheme
  (each area's tiles get its lights) and environment, fog, weather, day
  and night, the ambient sounds and music, the event scripts, the check
  modifiers, player vs. player, the loading screen and No Rest.
- Interior, natural, underground and the shader flags change alone: each
  area keeps its other flags.
- **Variables**: the variables you add or change are set on each area,
  those you delete are deleted from each, and each keeps its others.
- Names, tags and comments are an area's own, and aren't offered.

To give every underground area the same music: Underground in the
chooser, Tick Shown, Edit Together, then Audio. From the command line,
`mg areas` does the same (see [Command-line tools](11-command-line.md)).

## Area sounds

With **🔊 Sounds** on, the placed sound objects play as you move around:
full volume within a sound's minimum distance, fading to nothing at its
maximum, each repeating, looping or playing once as set. **Ambient** and
**Music** play the area's ambient sound and music (by day or night, as the
view shows). Options › Sounds sets which play when an area opens, and the
music's volume.

## Making tilesets

**Tools › Tilesets** edits a tileset's `.set` file: **New Tileset…**
starts one, **Open Tileset…** opens one (a copy of a game tileset, or one
being made beside its models). An edit changes only the lines it is
about, so comments, order and spacing stay. **Undo**, **Redo** and
**Save** are in the editor's toolbar; saving the module saves open
tilesets too.
- **General:** the name, the name players see, the height step, the
  border, default and floor terrains, interior, height transitions, and
  grass.
- **Terrains and Crossers:** the lists, and **Add Terrain** and **Add
  Crosser** by name.
- **Tiles:** every tile with its corners and edges; **Find** by model or
  terrain. Per tile: model, walkmesh, minimap picture, path node,
  orientation, corner terrains and heights, edge crossers, lights and
  animation loops. **Add Tile** and **Duplicate Tile** add at the end, and
  **Remove Last Tile** takes the last away: areas store tiles by number,
  so tiles in the middle stay where they are. **Preview** shows the
  model when the game data has it.
- **Groups:** each group's name, size and tiles, the bottom row first.

The toolbar also has:
- **Make Palette** writes `<tileset>palstd.itp` beside the `.set`, the
  palette the painter offers: one-tile groups as Features, the others as
  Groups, the terrains and crossers, the eraser and (with height
  transitions) raise and lower. For the game's own tilesets it holds
  every group their palettes place. `mg tileset-palette` does the same.
- **Check** runs Verify's tileset checks: counts that disagree with the
  sections, tiles whose models are missing, groups that won't paint.
- **Render Minimap Pictures** draws each tile from above with Moonglow's
  renderer and saves it as its minimap picture (32 pixels, a TGA named
  by the tile's `ImageMap2D`, which tiles without one get as
  `mi_<model>`) beside the `.set`. Models and textures beside the `.set`
  are used before the game's.

To use the tileset, put the `.set`, palette, models, walkmeshes, textures
and minimap pictures in a hak (Tools › Haks › Build Hak from Folder…).
