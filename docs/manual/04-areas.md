# Areas

Double-click an area in the module tree (or right-click it › **View
Area**) to open it in the **area viewer**: its tiles and objects drawn
with the area's lighting, fog and sky, by day or night. Areas open looking
straight down at their middle, north up, as in Aurora.

## The toolbar

- **Creatures, Doors, … Waypoints, Start**: show or hide each kind of
  object and the start location marker; **All** and **None**.
- **Night**, **Fog**, **Grid**: show the area at night, with its fog, and
  the tile grid.
- **🔊 Sounds**, **Ambient**, **Music**: play the placed sound objects, the
  area's ambient sound and its music, heard from where the camera looks.
- **Walkmesh**: the ground's walkmesh over the area (walkable faces green).
- **Select Tiles**: select tiles rather than objects (Aurora's Select
  Terrain).
- **Area Properties**, **Reorient Camera** (north up again), **Go to Start
  Location**.

## The camera

Aurora's bindings, plus a few of Moonglow's:

| Do | To |
| --- | --- |
| Ctrl + drag | move the camera over the area |
| Right drag, or middle drag | turn the camera (W A S D move it meanwhile) |
| Shift + middle drag | move the camera |
| Wheel | zoom (with Shift or Ctrl: slowly) |
| Arrow keys, W A S D, or numpad 4, 6, 8, 2 | move the camera |
| Numpad 7, 9 | turn |
| Numpad 1, 3 | tilt |
| Numpad 5 | look straight down at the whole area |
| Double-click an object in Find Instance | go to it |

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

Every move, turn and deletion is one undoable step. The pointer's
position in the area shows in the view's corner, to the centimeter.

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

Choose a blueprint in the palette and click in the area to place it.
Shift + click places it and keeps it
chosen for another; a right click or Escape lets it go. Doors go on a
tile's door hook when you click near one. Triggers and encounters are
drawn point by point: click each corner, double-click to close the
outline.

**Edit › Find Instance…** lists the placed objects across the module by
kind, area, blueprint and tag; double-click one to go to it.

**Prefabs** are groups of placed objects saved under a name (**Save as
Prefab…** on the selection), such as a camp, a market stall or a furnished
room.
- **Placing one:** choose it under **Edit › Prefabs**. It follows the
  pointer like a paste, and a click places it, with the objects in their
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
  and **Raise/Lower** act on the tile corner nearest the pointer; the right
  button lowers. The cursor shows the four tiles a stroke changes, and turns
  red where the tileset does not allow it.
- **Crossers** (roads, streams, walls) are dragged: they follow the
  pointer through the tiles it passes.
- **Groups** (buildings, big features) are placed whole; right-click to
  turn one before placing it.
- The **Eraser** takes the crossers off a tile; with Shift + click it
  steps the tile through the other tiles that fit there.

Each stroke is one undoable step. Moonglow paints as Aurora does: the
same strokes give the same tiles, heights and crossers.

## Tiles

With **Select Tiles** on, a click selects a tile, Ctrl + click adds one,
and a drag selects a box of them. **Delete** takes the selected tiles'
crossers away. **Shift + right click** steps the tile under the pointer
through the tiles that fit; a right click opens the tile menu with **Tile
Properties**: the tiles' main and source light colors and their
animation loops, as the tile's model has them (**Defaults** puts back the
lighting scheme's), and the **Replacement Texture**: what a texture named
`replace_tex` in the tile's model is drawn with (a `replacetexture.2da`
row; the game keeps it, though Aurora has no field for it). Ctrl+C and Ctrl+V copy and paste tiles.

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

## Area sounds

With **🔊 Sounds** on, the placed sound objects play as you move around:
full volume within a sound's minimum distance, fading to nothing at its
maximum, each repeating, looping or playing once as it is set. **Ambient**
and **Music** play the area's ambient sound and music (by day or night, as
the view shows). Options › Sounds sets which play when an area opens, and
the music's volume.
