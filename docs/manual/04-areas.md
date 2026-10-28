---
type: Manual Page
title: Areas
description: Areas - the viewer's toolbar, minimaps and camera, selecting, arranging and placing objects, Setup Store, duplicate tags, claimed and protected areas, painting placeables, painting terrain, tiles, Area Properties, area sounds and making tilesets.
tags: [manual, areas, terrain, tiles]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-28T09:51:25Z }
---

# Areas

Double-click an area in the module tree (or right-click it › **View
Area**) to open it in the **area viewer**, which draws its tiles and
objects with the area's lighting, fog and sky, by day or night. Areas open
looking straight down at their middle, north up, as in Aurora.

## The toolbar

Each button shows an icon and its name. With **Tools › Options ›
General › Toolbar buttons: Icons only** the buttons that have an icon
show it alone, and the toolbar takes about half the rows it took: rest
the pointer on an icon and its tip names it (and gives its key, as
Select Tiles' and Paint's do). All, None, Together, To Scratch and the
dropdowns keep their words.

- **Creatures, Doors, … Waypoints, Start**: show or hide each kind of
  object and the start location marker; **All** and **None**. What you
  hide is hidden in every area view, as in Aurora: the other open ones,
  those you open later, and the next time Moonglow starts (an area that
  looks empty may have its kinds hidden: **All** brings them back).
- **The start location** (where a player's character enters the module)
  is marked as in Aurora: a blue ring on the ground with a red arrow the
  way the character faces. To set it, right-click the ground where it
  should be and choose **Set Start Location Here**: it faces the way the
  view looks. Click the marker to select it (a yellow box, as round an
  object; the game's invisible object, which has nothing to draw, has
  the box its model holds undrawn, 0.6 m across, as Aurora outlines it). Drag it by its ring or its arrow to move it; selected, the
  arrow's tip has a round handle that turns it (both follow the snapping grid and
  angle; Escape lets go; an object standing on the marker is taken first). **Go to Start Location** brings the view to it; with the start in another area, it opens that area there. **Start** on the main toolbar (View › Go to Start Location) does the same with no area open: move the start location to where you are working, and it is a bookmark.
- **Fade Geometry**: draws the tiles without the parts that fade in the
  game to show a character behind them (roofs, upper walls), to see and
  place what is under them (Aurora's Environment › Fade Geometry):
  **Never**, **Object Mode Only** (while you work on objects: not with
  Select Tiles on or a terrain brush chosen) or **Always**. The choice
  is kept for the areas opened after. This list, **Snap** and **Turn**
  take the arrow keys once clicked or reached with Tab, as every list
  of choices does (Up and Down choose; the camera stays where it is
  meanwhile, and Escape gives the keys back to the view); so do the
  lists of Resize Area, Find Instance and Edit Areas Together.
- **Night**, **Fog**, **Grid**: show the area at night, with its fog, and
  the tile grid. The grid is faint from above; as the view comes down to
  look along the ground from the area's edge it turns Aurora's red, to
  count tiles by and line areas up, and under raised ground it is drawn
  at the area's foot as well. The fog is drawn as the game's camera (at most 20 m from
  the player) would see it from where the view looks, so a view from
  farther off isn't lost in it.
- **▶ Animations**: creatures and other placed objects play their
  animations, and those with particles show them (a campfire's flames,
  magic sparks, a portal: placeables that are only an effect show
  nothing otherwise), as do the tiles: a chimney smokes with its
  animation loop on in Tile Properties, and a torch on a wall burns where
  its source light has a color (Tile Properties). Switched off, they hold
  still (tiles' water and flames go on), in every area view; it stays as
  you leave it.
- **🌿 Grass**: the grass the area grows, as the game grows it on the
  tiles' walkmesh faces: the tileset's, and the area's own (Area
  Properties › Grass). It is drawn wherever the view shows grass ground,
  out to the far edge of the view, as dense as the game's within 45 m of
  the camera and thinner further off (dense grass from 45 m, the
  tilesets' own past 180 m). The game draws it within 30 m of its camera
  by default (its `Grass Render Distance` setting, up to about 170 m).
  From high above a field a tuft is a pixel or two, and the grass shows
  little. Switched off, in every area view; it stays as you leave it.
- **🎨 Shaders**: draws with the game's own shaders, compiled on your
  machine from the game and the module's haks, as the game draws: a
  material's custom shaders (its MTR's `customshaderVS` and
  `customshaderFS`: grass and leaves that sway in the wind, water that
  flows, roofs that show only to a low camera indoors), and for the rest
  the game's own programs, each mesh with the one the game chooses for
  it (a decal unlit, a reflecting mesh's, a normal-mapped one's where
  its model or material has a render hint), with a hak's changes to them
  (a hak that replaces `inc_lighting` lights every surface its way). A
  custom shader gets what the game gives it: the MTR's `parameter` lines
  as its uniforms, every texture coordinate set of the model (`tverts`,
  `tverts1`, `tverts2`, `tverts3`, its `vTcIn`…`vTcIn3`; zeros for a set
  the model doesn't have), and tangents only where the model has them or
  a render hint (the model's or the MTR's): without, a normal map draws
  black, as in the game. (A uniform a script sets in the game, with
  `SetMaterialShaderUniformInt` and the like, keeps the MTR's value
  here.) What moves by
  itself moves while the view is open. The wind is the area's (Area
  Properties › Wind Power), the indoor and outdoor flags its own. Off,
  Moonglow draws with its own shader, which looks like the game's stock
  lighting and moves no custom shader's mesh (two that builders' tilesets
  use, `vertexalpha` and NWN Mapper's `mzlm`, it draws its own way). A
  program that doesn't compile is drawn with Moonglow's shader and named
  in the log, once (the game draws it magenta; Verify names its file and
  line). A material that names one custom program alone is drawn with
  the game's lit shaders, as the game draws it. The sky, the area's
  grass, additive meshes and particles are drawn with the game's programs
  too (the grass a touch darker and yellower, lit at its corners as the
  game lights it). So is shiny water: the game's water shader shows what
  lies under the water and mirrors what is around it, through a copy of
  the picture drawn so far, as in the game; with the switch off it is
  Moonglow's own water. Particles fade where they meet the ground or a
  wall, as the game's do. On by default. The programs are compiled
  while the view goes on, the first time an area needs them (a fraction
  of a second each), and kept in Moonglow's cache folder for the next
  time: until they are ready, what they draw is drawn with Moonglow's own
  shader, so an area may change its look a moment after it opens. It is
  one switch for every 3D view (the model viewer has it too) and it stays
  as you leave it: switched off, it stays off.
- **💡 Lighting**: the area's lighting: the sun or moon colors of Area
  Properties, the tiles' main and source lights (Tile Properties), the
  light of each placeable whose appearance has one (placeables.2da's
  `LightColor`, unless its **Illuminated** is off) and the light nodes of
  every object's models (a builder's lamps and braziers, a held torch),
  as the game lights them: each with its color and reach, a light node's
  `multiplier` lengthening its reach, and each surface lit by the 32
  most important lights that reach it (by their priority, then their
  nearness), as the game's client is by default. Compared with Aurora an
  interior looks darker and its colors deeper: Aurora's lighting isn't
  the game's, Moonglow's is checked against the game's own pictures.
  Switched off, everything is evenly lit, whatever the area's colors and
  lights: for working in a dark area. Areas opened later open as you left
  it.
- **◑ Shadows**: the shadows the game draws, where the area has them
  (Area Properties' **Shadows**, sun and moon, and its **Shadow
  Opacity**), so that you can set the opacity by eye. A click turns them
  all off, and on again as they were; hold the button a moment, or
  right-click it, for the kinds to choose from:
  - **Sun and Moon Shadows**: the sun's by day and the moon's at night,
    falling away from the sun (to the south-west, as the game's sun stands
    for every area), cast by the tiles and the **static** placeables (a
    model's meshes flagged `shadow`, including shadow-only meshes that
    aren't drawn).
  - **Creature Shadows**: creatures, dynamic placeables and doors cast the
    sun's and moon's shadows too, posed as they stand, as the game's
    Creature Shadow Detail at Best has them (its default; at Fast or Off,
    none of them casts).
  - **Light Shadows**: where the area has no sun's or moon's shadows at
    the time (an interior, or night without Moon Shadows), each creature
    casts one from the nearest light that casts shadows, as in the game: a
    light with `shadow 1` and a radius of 10 m or more (a placeable's
    light, a lamp post; the tiles' torches, 7 m, cast none), from a little
    above it (its `verticaldisplacement`: a placeable's light 3 m).

  All three are on at first. Every shadow darkens the tiles and the
  static placeables, not dynamic ones or creatures, as in the game.
  Opacity 100 is as dark as the game makes them; like the game, Moonglow
  fades them toward the top of the picture (from a camera looking
  straight down, not at all), so they look as dark as in the game from
  the same angle; a light's shadow is lighter than the sun's at the same
  opacity, as the game's. Grass stays bright in a shadow, and with **Fade
  Geometry** on the roofs it leaves out cast none. Switched off (or with
  **Lighting** off) there are none; areas opened later open as you left
  them. Changes to the shadow fields in Area Properties show at once.
- **🔊 Sounds**, **Ambient**, **Music**: play the placed sound objects, the
  area's ambient sound and its music, heard from where the camera looks.
- **◎ Sound Ranges**: two circles around each placed sound that is heard
  from where it stands: at full volume inside the inner one (its minimum
  distance), not at all outside the outer one (its maximum). The
  selection's are brighter; a sound heard everywhere in the area has none.
  A sound that plays from a random place has a rectangle too: how far
  east and north of itself it may play. Areas opened later open as you
  left it.
- **Walkmesh…**: a click shows the ground's walkmesh over the area
  (Aurora's Render AABB Nodes): walkable faces green, the others red; a
  click again hides what is shown. Hold the button, or right-click it,
  for the two to choose from: **Ground Walkmesh** and **Object
  Walkmeshes**, where placeables (their `.pwk`, orange) and doors (their
  `.dwk`, blue, for the state they're placed in: closed or open) keep
  creatures out. The selection's are brighter.
- **Select Tiles** (F10): select tiles rather than objects (Aurora's
  Select Terrain); the Tiles palette has it too, at the top of its tools.
  Without the toolbar: a double click on the ground selects its tile
  (Select Tiles comes on), and a click on an object while tiles are
  selected selects it (Select Tiles goes off). Turning it on puts down
  the terrain brush, and choosing a brush turns it off, but for Next
  and Previous Tile Appearance and Rotate Tile with tiles selected,
  which step those tiles and leave Select Tiles on (see [Painting
  terrain](#painting-terrain)).
- **✏ Paint** (B): paint the placeables chosen in the palette onto the
  ground (see [Painting placeables](#painting-placeables)).
- **Area Properties**, **Reorient Camera** (north up again), **Go to Start
  Location**.
- **To Scratch**: copy the area as it is now (its `.are`, `.git` and
  `.gic`) into the scratch folder (chosen the first time; Tools › Options
  › Folders changes it), with the module's own blueprints, scripts
  (compiled first where they changed) and conversations it uses, as
  Export's **Include the resources they use** takes them.

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
| Ctrl + drag | move the camera over the area, the ground following the pointer (a closed hand shows it held; a tileset brush paints nothing with it) |
| Right drag, or middle drag | turn the camera (W A S D move it meanwhile) |
| Alt + middle drag, or Alt + drag begun off the selection | move the camera (Alt + drag on the selection raises or lowers it) |
| Wheel | zoom (slowly with Shift, or with Ctrl unless the pointer is on a selected object that takes a scale; when painting tiles, Shift is the brush's) |
| Sideways scroll (a tilted wheel, a second wheel, two fingers sideways on a touchpad) | move the camera left and right, as the screen has it |
| Arrow keys, W A S D, or numpad 4, 6, 8, 2 | move the camera |
| Numpad 7, 9 | turn |
| Numpad 1, 3 | tilt |
| Z, C | move the camera up, down |
| Ctrl + Shift + wheel, or Ctrl + Shift + middle drag | move the camera up and down, finely |
| Numpad 5 | look straight down at the whole area (and back to the ground) |
| F10 | select tiles or objects (Aurora's) |
| X, T, N, Shift+N, R, Shift+R | the tile tools: the Eraser, Refine Tile, Next and Previous Tile Appearance, Rotate Tile and clockwise ([Painting terrain](#painting-terrain)) |
| Double-click an object in Find Instance | go to it |

The keys drive the area view the pointer was in last, also while the
pointer is over the palette or the module tree (not while you type in a
field). The letters and digits among them don't while a tree goes to its
lines by them: the module tree under the pointer, a conversation's tree
while it has the keys ([Conversations](06-conversations.md)).

## Selecting and arranging objects

| Do | To |
| --- | --- |
| Click | select an object (the one whose model is under the pointer: a click through an arch reaches what is behind it) |
| Ctrl + click | add it to the selection, or take it out |
| Drag on the ground | select the objects in the box |
| Drag the selection | move it over the ground |
| Alt + right drag | turn the selected objects |
| Drag the ring around them | turn them as the pointer goes round |
| Shift + drag a tilt ring | tilt their models about X (red) or Y (green) |
| Shift + drag the square handle | scale their models: away from the object larger, toward it smaller |
| Ctrl + wheel, the pointer on one of them | scale their models, as in Aurora: up larger, down smaller |
| Shift + drag an arrow | move them along one axis alone: east (red), north (green) or up (blue) |
| Q, E | turn them 15° left or right (or by the snapping angle); with Shift, 90° |
| G | drop them to the ground |
| Shift+G | drop them onto the placeable below |
| Alt + drag | raise or lower them (not creatures, which stand on the ground) |
| Delete | delete them |
| Escape | select nothing (it drops a drag, a brush or a paste first) |
| Ctrl+C, Ctrl+X, Ctrl+V | copy, cut, paste (in this area or another) |
| Double-click | the object's Properties |

Every move, turn and deletion is one undoable step. The view's corner
shows the pointer's position in the area, to the centimeter, and the
tile there: its model's name (`ttr01_a01_01`), its column and row, and its
height and turn when it has them. With one tile selected (Select Tiles),
it names that tile too.

**The turning ring**: an orange ring lies around the
selected object, with a mark on it the way the object faces. Take the
ring anywhere along it (near it is enough) and lead it round: the object
turns with the pointer, its facing shown in degrees; with several
selected, each has its ring (and with Shift its tilt rings and arrows),
any of which leads them all: each turns about itself by the same angle. With
**Together** on (beside Turn on the toolbar), they turn about their
middle as one instead, keeping their places among themselves, like a
prefab turned whole; Q and E turn them so too. A trigger or an
encounter turns by its outline: alone about the outline's middle (an
encounter's spawn points go round with it), and with the others when
they turn together. **Turn** on the
toolbar snaps it. **Tools › Options › Area** switches the ring off.

**The tilt rings**: hold **Shift** and two upright rings take the turning
ring's place, red and green. Lead the red one round to tip the model
about its X axis, the green one to lean it about its Y axis; the angle
shows in degrees, and **Turn** snaps it. This is the rotation of EE's
visual transform (the one **Adjust Location…** has as numbers): the red
ring sets its **Y Rotation** and the green one its **Z Rotation**, which
is how the game reads them (see Adjust Location below). It is the
model that tilts, not the object: what it blocks stays as it was.
Creatures, items, doors and placeables that aren't **Static** have the
rings; the game draws a static placeable without its visual transform,
and its right-click menu has **Make Dynamic** to clear Static. A dynamic one's
has **Make Static**, which also takes its visual transform away, as
Aurora does. (The game draws static placeables, and finds paths around
them, more cheaply: keep scenery that needn't tilt static.) A ring
seen edge-on (from straight above, both are lines over the object) is
led along its line.

**The scale handle**: with **Shift** held, a white square stands on a
stalk up and to the right of the selected object. Pull it away from the
object to make the model larger and push it toward the object to make it
smaller: twice as far from the object's feet, twice the size. The size
shows beside the pointer as you drag; with **Snap** on it goes in steps
of 0.05. The model grows from its feet, so it stays on the ground, and
every way alike (Adjust Location's Scale is the same number). Like the
tilt, it is the model that changes, for the objects a tilt works on;
several selected objects each scale about their own feet. Escape drops
the drag.

**Ctrl + wheel** scales the selected objects too, as Aurora's does: with
the pointer on one of them, turn the wheel up to make their models
larger and down to make them smaller, about a twentieth a notch. With
the pointer anywhere else, Ctrl + wheel zooms the view slowly, so that
the wheel turned while Ctrl still moves the view (Ctrl + drag) leaves
the selection's size alone. The
size shows beside the pointer and in the status line; it becomes one
step to undo when Ctrl is let go or the wheel rests (Escape, or Undo
meanwhile, drops it). It works on what a scale works on: creatures,
items, doors and placeables that are not static. A static placeable
takes no scale (the game ignores one): clear **Static** in its
properties first; the status line says so, and the wheel zooms the view
slowly, as it does with nothing selected that takes a scale.

**The arrows**: with **Shift** held, three arrows stand out from the
selected object: red to the east, green to the north and blue upward.
Take an arrow's head and lead it along: the object moves along that axis
alone, however the pointer strays, with its place on the axis shown in
meters. East and north keep it as high above the ground as it was, and
**Snap** snaps along the axis; up raises or lowers it, as Alt + drag
does. They move the object itself, so they work for static placeables
too; creatures stand on the ground and have no blue arrow. An arrow
pointing at the camera (the blue one, from straight above) is led up and
down the screen.

A click on a ring or an arrow selects the object under it, if there is
one; the selection stays otherwise.

**Lights**: right-click the ground and choose **Add Light Here**, then
a color. Moonglow puts one of the game's invisible light placeables
there ("Light, White" and the six others of placeables.2da), a little
above the ground: it lights what is around it in the game and needs no
custom content. It shows as a small sun in its color; select, move and
raise it as any placeable. **Light Color** on its right-click menu gives
it another of the colors. Selected, a ring in its color shows how far
it reaches: where the game's light ends, 20 m out, the light fading
all the way there. These are the game's fixed lights: seven colors, one
reach. A hak that adds rows of the same kind (a `LightColor`
on the model `dag_invisible`) adds to the list.

**An encounter's spawn points**: with the encounter selected, the tip of
each spawn point's arrow is a handle. Lead it round the point and what
spawns there faces that way (**Turn** snaps it). The square at the foot
of its post is another: lead it over the ground and the point moves, the
encounter staying where it is.

What you are dragging stays in hand while another button turns the
camera (hold the middle button too and swing the view), until you let go
of the button that holds it. **Escape** drops the drag: nothing moves.

**Snapping** (the toolbar's **Snap** and **Turn**):
- **Snap** moves objects to a grid of 0.25 to 5 m as you drag, place or
  paste them.
- **Turn** turns them in steps of 5° to 90°, or of any number of degrees
  typed beside **Other** in its list.

With several selected, the first selected snaps and the others keep their
places and turns around it.

**Right-click** the selection for its menu (Properties last, as in
Aurora):

- **Adjust Location…**: an exact position and bearing, and the visual
  transform that EE added (scale, rotation and offset of the model) for
  creatures, doors, items and placeables that aren't Static. **OK**
  sets what you changed and closes the window, **Apply** sets it and
  keeps the window open; each is one step to undo. The rotations are
  named as Aurora names them and the game reads them, which is not by
  the axis each turns about: **X Rotation** turns the model about its
  upright axis (as Bearing does), **Y Rotation** tips it forward or
  back about its own X axis (east at Bearing 0), and **Z Rotation**
  leans it to one side about its own Y axis; the game makes them in
  that order, X outermost. **Scale** scales the model every way alike
  (the game reads one number, `ScaleX`, and Moonglow writes the three
  alike, as Aurora does; a file whose `ScaleY` or `ScaleZ` differs,
  set by hand or by another tool, keeps them until Scale is changed,
  and the window says so), and the offsets move it along the
  object's own axes, turned by its Bearing but not by the rotations.
  The area view draws all of this as the game client does. A static
  placeable is offered none, as in Aurora: the game draws a static
  placeable without its visual transform, whatever its file has (the
  server keeps it, but players don't see it). The window says so;
  untick **Static** in its Properties to transform it. Ticking Static
  (in Properties, or Make Static) takes the transform off, as Aurora
  does, so a static placeable with one in its file was made elsewhere:
  the window says so, and Verify warns of it (`static-transform`), with
  a **Take the transform off** button (since Moonglow 2.13.0).
- **Random Facing**: turns each selected object to face a way of its
  own, at random (by the snap angle, if one is set): for trees and rocks
  placed in numbers.
- **Drop to Ground** (G): puts raised objects back on the ground under
  them. A trigger or an encounter comes down (or up) as a whole, until
  its lowest corner is on the ground.
- **Drop onto Object** (Shift+G): sets each selected object on the
  placeable or door below its center (a skull on an altar, a book on a
  table), by that object's model, up to 2 m above it; with nothing there,
  on the ground. Not creatures, which the game stands on the ground.
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
  unlocking the object. A nasher project's files don't carry it: Moonglow
  keeps the project's locks in its own folder (see
  [nasher projects](03-modules.md#nasher-projects-version-control)), so
  locking never shows in a diff.
- **Save as Prefab…**: the selected objects, kept under a name to place
  again (see below): a group of placeables to copy around, in this area
  or another.
- Doors: **Reverse Door**, **Initial State** (open or closed, locked).
  Placeables: **Initial State** (open, active…). Sounds: **Mute** or
  **Turn On**. Triggers and encounters: **Redraw Polygon**; encounters:
  **Add Spawn Point** (at the point you right-clicked).
- **Conversation**, **Inventory** (creatures, placeables with an
  inventory, stores).
- **Add to Palette**: the instance as a new custom blueprint (its ResRef
  after [your builder code](03-modules.md#your-builder-code), where you
  have one).
- **Export as Blueprint…**: the instance written into an archive (`.erf`)
  as a blueprint of its own, made and named as Add to Palette makes one
  (with the items it holds, each an item blueprint), but not added to the
  module; with the module's resources it uses (its scripts, its
  conversation), for another module's **File › Import…**. The module
  tree's menu of a placed object has it too.
- **Test From Here**: starts the game at the point you right-clicked,
  facing the way the camera looks, with the module as it is now (see
  [Build, verify and test](09-build-and-test.md)). **Test From Here,
  Choose Character** does the same through the game's character
  selection.
- **Create Waypoint**: a waypoint where the object stands. **Create Set…**
  (waypoints): name the selected waypoints `<name>_01`, `<name>_02`…
- **Levelup Wizard…** (creatures), **Setup Store…** (a creature or
  placeable as a shopkeeper: its conversation, a store and the script that
  opens it; see [Setup Store](#setup-store)), **Add Popup Text…** (a
  placeable that speaks a line when clicked).
- **Variables…**: the object's local variables (with the project's
  [described variables](03-modules.md#described-variables) offered).
- **Properties**, the last item: the object's properties as placed in
  the area (an instance can differ from its blueprint). With several
  objects of one kind selected, one window edits them together: a field
  changed there changes in all of them. It shows the first one's values,
  with **≠** before a field where the others have another (so too for
  blueprints edited together from the palette): every field set on each
  has the mark (names in every language, descriptions, sliders, colors,
  appearances, the conversation, the variables, each of an area's flags),
  and the pointer resting on it says what it means. A list the Visuals
  page sets whole (texture and animation replacements, shader
  parameters) says so under its heading where they differ. Inventories, classes,
  skills, feats, spells and the other lists of an object's own are
  edited one object at a time, so those pages aren't offered there (and
  a placeable's **Inventory…** is grayed).

### Setup Store

**Setup Store…** asks what Aurora's Store Setup Wizard asks: the
shopkeeper's lines, the store blueprint, and a friendlier faction for a
hostile shopkeeper. **Finish** makes, as one step to undo, the
conversation (`store001`), the script that opens the store
(`openstore001`, which calls the game's `OpenStore`, or
`gplotAppraiseOpenStore` with appraise checks), the store placed where
the shopkeeper stands with its blueprint's tag, and the shopkeeper's
conversation.

A server with its own way of opening stores (its prices, its
reputation, its logs) says so in the [project
file](03-modules.md#the-project-file-moonglowtoml)'s `[store]`:

- `open_script`: the reply that opens the store runs this script, and
  no `openstoreNNN` is written (the wizard's page says so; the log warns
  when neither the module nor the game has the script);
- `tag`: the store's tag, `{area}` the area's tag and `{n}` the first
  number no other object of the module has (`{area}_STORE_{n}`); without
  a pattern the blueprint's tag, with a number where another object has
  it. Either way no two stores share a tag, and the last page says it;
- `blueprint`: the store blueprint chosen at first;
- `keeper_variable`: a String variable set on the shopkeeper to the
  store's tag, where the project's script reads it.

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
to close the outline. For a rectangle, click one corner, press Alt (the
rectangle to the pointer shows), then click the corner opposite: its
sides run east and north, and it is placed at once (the same in
**Redraw Polygon**). Alt may be held for the click or let go before it:
on Linux, many window managers keep a click with Alt held for
themselves, to move the window, and there it is let go first. Escape
drops the outline, to begin again point by point. Turn the rectangle
after, by its ring, if it is to lie another way.

**Edit › Find Instance…** lists the placed objects across the module by
kind, area, blueprint and tag; double-click one to go to it.

**Prefabs** are groups of placed objects saved under a name, such as a
camp, a market stall or a furnished room: the way to copy a group of
placeables around.
- **Saving one:** select the objects (a click, Ctrl+click for more, or a
  box dragged around them), then **Save N as Prefab…** on the area's
  toolbar (it shows when several are selected), **Save as Prefab…** on
  the selection's right-click menu, or **Save Selection as Prefab…** in
  the palette's **Prefabs**.
- **Placing one:** choose it in the palette's **🗐 Prefabs** (beside the
  blueprint types), or under **Edit › Prefabs**. It follows the pointer
  like a paste. A click places it, with the objects in their places
  around each other and at their heights above the ground.
- **What one holds** shows while the pointer rests on it in the
  palette's Prefabs: how many objects of each kind, and their tags.
- **Renaming one:** right-click it there and choose **Rename…**.
- **Deleting one:** right-click it and choose **Delete**. A prefab isn't
  the module's, so Edit › Undo doesn't bring it back: **Undo Delete**,
  above the list, does (for the one deleted last, until Moonglow is
  closed).
- **Copy and paste** do the same without a name: Ctrl+C on a selection,
  Ctrl+V in this area or another. (Copying leaves a line of text on the
  system's clipboard, "Moonglow Toolset: 3 objects": Windows gives Ctrl+V
  nothing to do with an empty clipboard.) A paste that puts a tag on a
  second object says so: see [Duplicate tags](#duplicate-tags).
- **Any area, any module:** prefabs are kept in Moonglow's data folder, in
  `prefabs` (see [Troubleshooting](13-troubleshooting.md)). Copy the
  `.prefab.json` files to share them.
- **Blueprints:** an object keeps everything it had when it was saved,
  and works without the blueprint it came from.

## Duplicate tags

A copy keeps its original's tag, and a script that looks an object up by
its tag (`GetObjectByTag`) finds only one of them: in the game, the copy
placed later. A copied start waypoint, rental door or store then takes
its original's place, or the original its copy's, with no error.

With **Options › General › Show duplicate tags warning after a paste
or a copy** on, a paste (Ctrl+V, a prefab placed) that puts a tag on
more than one object of the module (its areas and the objects placed in
them) is said in the log and the **Duplicate Tags** window:
"KREZK_RENTAL_DOOR is now on two objects". They say it of the tags a
builder set: a tag that isn't the object's blueprint's own (as the
module, its haks or the game have the blueprint; one whose blueprint
isn't found counts as set), and those the project wants unique. Every
kind of object is weighed so: a creature, a waypoint, a trigger or a
store pasted with its blueprint's tag asks nothing, and any of them with
a tag typed in asks. A spawn system's waypoints, whose tags are typed
in, ask; the creatures placed from their blueprints don't. A static
placeable (scenery) never asks.

The option is **off by default**: Aurora has no such warning, and
builders put one tag on many objects on purpose (a spawn system's
waypoints, generic triggers). **Don't warn again** in the window turns
it off (Options › General turns it back on). A project that wants the
warning says so in its project file (below), which wins over the
option: its `paste_warning` is followed whatever the option says, and
the window then offers no Don't warn again. Where the project file
doesn't set `paste_warning`, the option on warns as above, and off warns
only of the tags the project wants unique (`unique`, `unique_in_area`),
the team's own rule; without those, nothing.

- **Retag** gives each pasted object with such a tag one of its own, the
  tag with a number that no object has (`LAMP_2`; `STORE_7` becomes
  `STORE_8`), as one step to undo.
- **Keep Tags** leaves them (objects that no script looks up).

An area copied with **Copy…** in the module tree has every tag of its
original's objects: with the option on, the window lists the area's own
tag (an area has no blueprint), the tags a builder set and those the
project wants unique.

The [project file](03-modules.md#the-project-file-moonglowtoml) says
which to warn of (`[tags] paste_warning`: `"custom"`, as above;
`"any"` for every tag but a static placeable's, an area copy's
then listing only its own tag and the unique ones and counting the
rest; `"unique"` for only the tags it names in `unique` and
`unique_in_area`; or `"off"`), and Verify Module warns of a tag it wants
unique that is on two objects.

A tag a script looks for in its own area only (a store's, which a
vendor's script finds as the nearest of that tag) may be on one object
of each area: the project names those in `[tags] unique_in_area` (since
Moonglow 2.2.0). A paste then counts such a tag in the area it was
pasted into, whatever `paste_warning` says ("KZ_SQUARE_STORE_1 is now on
two objects in kz_square (the project wants \*_STORE_\* unique in each
area)"), a copy of a whole area is quiet about them, and Verify Module
warns of one that is twice in one area.

## Painting placeables

Trees, rocks, bushes, rubble: **✏ Paint** on the area's toolbar (or **B**
with the pointer over the area) turns the placeable brush on, and the
placeables chosen in the palette are painted onto the ground rather than
placed one a click. Choose them in the **Placeables** palette, the
game's or the module's: a click on one, **Ctrl + click** on more (Ctrl +
click again takes one out). The brush picks among them at random. A strip
of its settings shows above the view while it is on; they are kept for
the next time.

- **Scatter** paints in a green circle on the ground. Press to lay a dab,
  drag to go on laying them. Hold the button still and it keeps laying
  until the circle is full. **Shift + wheel** sizes the circle (the
  **Radius**).
- **Density** is how many placeables a tile's ground (10 by 10 m) holds
  once painted. The brush adds placeables until the circle has its share,
  counting those of the chosen blueprints already there. So going over a
  painted place again fills its gaps and adds no more.
- **Spacing** is the least distance between two placeables: those the
  brush puts down, and any placeable already in the area.
- **Flow** is how much of what the circle lacks each dab adds: at 100% a
  click fills the circle at once; lower, it fills as you go over it.
- **Turn**: each placeable faces the way **Q** and **E** turned the brush,
  give or take half this (360°, the default, is any way at all; 0° all
  alike). With the snapping angle on, facings snap to it.
- **Scale**: each placeable's scale is picked between the two. Static
  placeables take none: the game draws them at their own size.
- **Height** raises them above the ground (or sinks them, below 0): roots
  into a slope, rocks half buried.
- Placeables go only on **walkable ground** (the walkmesh's walkable
  faces, as **Walkmesh…** shows them green), at the level nearest the
  pointer's: not into water, onto roofs or inside walls.
- **Row** lays pieces end to end instead: drag from where the row
  begins to where it ends (floor tiles, a fence, a line of benches). Each
  piece faces along the row; **Q** and **E** turn the pieces on it
  (Shift + Q by 90°, for a piece that is long the other way). A piece
  takes up its model's length along the row. **Gap** adds room between
  pieces, or overlaps them when below 0: a model's box takes in whatever
  sticks out of it, so a little overlap may be what makes floors meet.
  The row begins on the snapping grid, and its direction snaps to the
  snapping angle. It follows the ground (walkable or not, so a walkway
  can cross water), raised by **Height**. Of several placeables chosen,
  each piece is a random pick.
- **Fill** covers a rectangle instead, in rows (a floor): drag from one
  corner to the opposite one, whichever way. **Q** and **E** turn the
  grid, and the rectangle's sides with it (by the snapping angle, if on;
  Shift + Q by 90°), each piece facing along it. A row lays its pieces end
  to end, each by its own length and **Gap**; the next row begins past the
  deepest piece of the one before, and Gap. So pieces of different sizes
  pack as they are. The first piece's corner is where you pressed (on the
  snapping grid), and pieces go on until the rectangle is covered, so the
  last row and piece may reach a little past it. It follows the ground as
  Row does, raised by **Height**.

Before a press, the dab a click would lay (with Row or Fill, the first piece)
shows see-through under the pointer, and a click lays exactly that. What
a stroke lays shows see-through until you let go; then it goes into the area as one step,
**Paint Placeables**, which one Undo takes away. Escape or a right click
during a stroke drops it. Otherwise it turns Paint off, and the
placeables stay chosen.

## Painting terrain

Choose **Tiles** in the palette pane to paint the area with its tileset,
as Aurora's Terrain tab does:

- **Terrain brushes** (grass, water, cliffs… whatever the tileset has)
  and **Raise/Lower** act on the tile corner nearest the pointer;
  Raise/Lower's right button lowers, and a terrain's right click opens
  the [tile menu](#tiles) for the tile under the pointer, the brush kept
  (Aurora's right click puts the brush down: here Escape does, or the
  menu's first item, **Put Down**). Drag to mark every corner the pointer passes
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
  turn one before placing it. It stays chosen, to place another. Over
  part of another group or feature it is refused, as Aurora refuses it:
  the cursor is red there, and a click flashes the other's tiles red and
  says so on the status line (erase that one first). With **Allow
  editing tiles of groups and features** on (Tools › Options › Area), it
  takes the other away whole instead.
- **A group or a feature is one thing.** Its tiles are the tileset's
  group's, as the area holds them (the area's file doesn't say which
  tiles a group placed: Moonglow knows them as the tileset's group
  standing whole, each tile at its place, turned and raised alike). Next
  and Previous Tile Appearance, Rotate Tile, Refine Tile, the Eraser's
  Shift + click, a terrain's Shift + click and Shift + right click leave
  its tiles as they are, and a terrain painted where it would change one
  of them is refused; the status line (the log, from the area's own
  menu) says why and how (the Eraser takes
  all of it away), and its tiles flash red. Painting beside it, where
  its tiles still meet the terrain, is as ever. With **Allow editing
  tiles of groups and features** on, each of its tiles can be changed
  alone; the status line warns (⚠) when a change leaves a group part
  changed (Undo puts it back).
- The **Eraser** takes the crossers off a tile, and on a tile of a group
  or a feature takes the whole of it away (its tiles become ground that
  fits there); Shift + click steps the
  tile through the other tiles that fit there. Dragged, it marks the
  tiles it passes, as a terrain brush marks corners (run back, Shift for
  a rectangle), and erases them when you let go.
- **Refine Tile** (Moonglow's own) steps the tile you click through the
  other tiles that fit there, whatever it holds, and never paints. It
  goes in Aurora's order, as Shift + right click does: each tile's turns,
  then the next tile's, so the tile's look and its rotation change
  together (it isn't random: click on to come round again).
  Dragged, it does that to the first tile, and every tile it passes of the
  same kind (the same terrain at its corners and the same crossers, so
  that tile fits there) becomes that tile too, turned to fit: a whole run
  of wall made alike in one stroke. Tiles of another kind are left as
  they were, and the area's line says how many.
- The Tiles palette has the blueprint palettes' tools: a **search** (a
  brush's name, or a tile's model: a group or feature is found by the
  models of its tiles too, and shows as itself, one picture in the
  Gallery), **Expand All** and **Collapse All**,
  how many there are, and **List** (☰) or **Gallery** (⊞), the same switch
  as the blueprint palettes', with the pictures' size beside it.
- In the **Gallery**, each terrain and crosser is a heading that opens and
  closes, with every tile it can make under it as pictures: each variant,
  and each transition to another terrain (none of a group's). A tile is
  seen from the south-west, as an area opens, or from the corner that
  shows much more of it (a chasm's wall, which faces into the chasm). Its name
  still chooses the brush. Click a picture to paint with that tile itself:
  a click places it, a drag places it in every tile the pointer passes,
  and a right click turns it a quarter, as a group. Drag a picture into
  the area to place it where you drop it. The tiles around fit themselves
  to it (a tile with a bit of road brings the road into its neighbors).
  Double-click a picture for the tile's view: its model to turn and zoom,
  and what the tileset says of it (each corner's terrain and height, each
  edge's crosser, its lights, loops, doors and sounds), with **Paint with
  it**. Chosen from its picture, **Refine Tile** chosen next (it then
  reads **Refine Tile →** and the tile's model) makes the tiles you click
  or drag over that tile, wherever it fits as they stand, and leaves the
  rest. Click the picture again, or another brush, to let it go.
- **📌 Exact tile**, beside List and Gallery (shown with the Gallery, or
  a tile chosen from it), puts a tile chosen from the pictures in
  **alone**: a click (or a drag, in every tile it passes, or a picture
  dropped) puts exactly that tile there, turned as the right click
  turns it, at the height of the tile it replaces, and nothing else
  changes: no neighbor, corner terrain, height or crosser around it is
  adjusted, so it may not meet them (a wall's end in open ground, the
  lip of a castle by itself, as builders otherwise did by hand in the
  area's `Tile_List`). The preview and the cursor show that one tile.
  Its lights are chosen afresh, as a painted tile's, and the doors its
  hooks call for come and go as a painted tile's do (as the plugin
  API's `set_tile` puts a tile). Over a tile of a group or a feature it
  is refused, as any stroke is (the group flashes red), unless **Allow
  editing tiles of groups and features** is on, when the status line
  warns of the group left part changed. Each click, or drag, is one step
  to undo. It is for a terrain's and a crosser's tiles: the **Features**
  and **Groups** pictures are placed whole as ever, switch or not. The
  switch is off at first and remembered; with it off,
  painting is as above. A tile placed so is a tile like any other after:
  a terrain painted beside it chooses it again where its corners change.
  To raise or lower it, use Tile Properties' **Height**; to put in a
  tile the Gallery doesn't show (a piece of a group), its **All tiles**
  ([Tiles](#tiles)).
- In the Gallery, **Features** and **Groups** are pictures too: each as
  it stands once placed in open ground, its tiles together with the tiles
  around that its placement changes (a castle's gate, a single tile, in
  the castle its corners call for; a boat in its water). Click one to
  place it (a right click in the area turns it), or drag it into the
  area.
- **Next Tile Appearance** and **Previous Tile Appearance** (Moonglow's
  own) give the tile you click another tile of the same kind: the same
  terrain at its corners and the same crossers, so it still fits, turned
  as the old one was where it fits so. Its lights are chosen afresh, as a
  painted tile's.
- **Rotate Tile** (Moonglow's own) turns the tile you click a quarter
  counter-clockwise (Shift + click: clockwise), the same tile, keeping its
  lights and loops. A tile turns only where it still fits its neighbors:
  open ground turns every way, a tile with a corner of another terrain or
  a crosser at one side fits only as it lies (turned, it would no longer
  meet its neighbors' terrain), so the area's line says so and the tile
  flashes red; a tile whose opposite sides match turns a half. To turn the
  terrain itself, paint it. A group's or a feature's tile turns only with
  that option on (above), where it still meets its neighbors.
- **With tiles selected** (Select Tiles on), a click on **Next Tile
  Appearance**, **Previous Tile Appearance** or **Rotate Tile** in the
  palette steps the selected tiles at once, each on its own, as the tile
  menu's items do ([Tiles](#tiles)): one step to undo for all of them.
  The tool isn't chosen: Select Tiles stays on and the tiles stay
  selected, so another click steps them again (Next Tile Appearance
  clicked three times: three appearances on), and a click in the area still
  selects. Shift + click on Rotate Tile turns them clockwise. A tile
  that can't step (no other appearance, a tile that doesn't turn, a
  group's tile) is left as it is; when none of them can, the status
  line says why (at the start of the line, where a narrow view doesn't
  cut it off) and they (a group's, the whole group) flash red.
  **Refine Tile** clicked there refines the selected tiles as its drag
  over them would (the first the next tile that fits, the others of its
  kind made that tile too; with a tile chosen in the Gallery, that tile
  wherever it fits), and the **Eraser** deletes them, as Delete does:
  each one step to undo, the selection kept. With no tile selected, the
  tools are chosen as above, and you click the tiles.
- **Keys** (Tools › Options › Keyboard changes them): **X** the Eraser,
  **T** Refine Tile, **N** Next Tile Appearance, **Shift+N** Previous
  Tile Appearance, **R** Rotate Tile and **Shift+R** Rotate Tile
  clockwise, with the pointer over the area view. With tiles selected
  they work on them, as the palette's clicks do; with none, the key
  chooses the tool, and pressed again puts it down. Their tips in the
  palette name them.
- Select Tiles, the Eraser, Refine Tile, the three tools above and
  Raise/Lower head the Terrain list, whatever the tileset's order. A
  right click with the Eraser, Refine Tile or the three opens the tile
  menu.

Under the pointer, the area shows the tiles a click would make, slightly
see-through (their nearer surfaces over their farther ones, a bridge
over its water), in place of those they replace: a feature or group, a
terrain's or a raised corner's tiles, the Eraser's or Refine Tile's
choice. While you drag a terrain, a crosser or the Eraser, it shows what letting go
now would paint. The click or drag puts down exactly the tiles shown:
where several fit, the one shown.

The cursor is green where a click paints and red where the tileset
refuses it. Click there all the same and the tiles in the way flash red
for a moment, as in Aurora: those that have no tile fitting what the
stroke would make of them, or belong to a group it would break. It is blue where a click only chooses tiles again: a crosser
over a quarter it already crosses, the Eraser with Shift, Refine Tile, the
appearance and rotate tools (red where the tile has no other appearance,
or doesn't turn), and a corner of the brush's own terrain (with Shift,
the next tiles that fit).

Each stroke is one undoable step. Moonglow paints as Aurora does: the
same strokes give the same tiles, heights and crossers.

## Tiles

With **Select Tiles** on, a click selects a tile, Ctrl + click adds one,
and a drag selects a box of them. **Delete** takes the selected tiles'
crossers away, and a group or a feature with a tile among them whole, as
the Eraser does (Aurora's Delete takes the tile alone, leaving the rest
of the group: Moonglow does so with **Allow editing tiles of groups and
features** on). **Shift + right click** steps the tile under the pointer
through every tile that fits, each of its turns and then the next tile's
(Aurora's order, as Refine Tile steps). Ctrl+C and Ctrl+V copy and paste
tiles.

A right click on a tile opens the **tile menu**: **Next Tile
Appearance**, **Previous Tile Appearance** and **Rotate Tile**, as the
Tiles palette's tools of those names do (each one step to undo), then
**Delete**, and last **Tile Properties**. In Select Tiles they act on
the selected tiles (each on its own), as the Tiles palette's three do
when clicked with tiles selected; with a terrain brush or a tile tool
chosen, on the tile clicked. Without either, a right click on the ground
where no object is starts the area's menu with the tile's items (Tile
(x, y), the three, Tile Properties), as Aurora's menu has Tile
Properties for a tile; on an object, the object's menu.

The status line says why a step was refused, in Select Tiles as with a
brush.

With one tile chosen, Tile Properties shows the **variants**: every tile
that fits there, as pictures, the one there now framed (not for a
group's or a feature's tile, which changes only with its group, unless
the option above is on). Click another and
it is put there at once (one step to undo), the window staying open on
it. Tile Properties is a window like the editors': resize it, maximize it
or dock it beside the area, and the pictures fill it. The **Size** slider
beside the variants makes the pictures larger (up to 224 points), to tell
tiles much alike apart; it is remembered. Under the pointer, a picture
shown smaller shows at its largest beside it. The same model turned
another way is listed again, its picture turned so (seen from the
south-west) and the turn written under it.

Above them are the tiles' **Main Light 1** and **2** and **Source Light
1** and **2** colors and their **Animation Loops 1** to **3**, as Aurora
shows them: a control is there for what the tile's model has (a light
node named `<model>ml1` or `ml2`, a node `<model>sl1` or `sl2` where a
source light's flame burns, an animation `animloop01` to `03`), and
greyed out, a loop unchecked, for what it hasn't. The tileset's own
`MainLight1`…`AnimLoop3` flags don't tell (the game's tilesets have them
for every tile). With several tiles chosen, a control is there for what
any of them has, and it shows the first tile's value; a control you set
goes to every tile that has it, and the ones you don't touch keep each
tile's own value. Each tile is given only what its own model has: a
value already in the area for a light the model hasn't is kept as it
is. **Defaults** puts back the lighting scheme's colors and the loops.

Every change in Tile Properties shows in the area **as you make it**: a
light, a loop, the Height (each click of - or +, and each step of a
height dragged), Defaults. While the window stays open, all of them are
**one step to undo**, however many you make. **OK** keeps them and
closes the window. **Cancel** takes them back, and the tiles and what
stands on them are as they were when the window opened (nothing is left
for Redo). Closed any other way with changes (its tab's or window's ✕,
Escape), it asks, as a side editor's tab does: **Save** keeps them,
**Don't Save** takes them back, **Cancel** leaves the window open
(Aurora's ✕ discards them without asking).

A tile put in from the window's pictures (a variant, a tile of All
tiles, or the tiles turned by its Turn, below) is a step of its own,
made at once, and Cancel doesn't take it out: Undo does. The window's
lights, loops and height go on it, and stay its own step, above the
tile's: Cancel takes them off the new tile, and the tile stays. If
another change is made in the module while the window is open (an
object moved, a tile painted), Cancel still puts back what the window
changed, in a step of its own, **Cancel Tile Properties**, where those
tiles still have the window's values; what the other change did stays.
**Undo** (Ctrl+Z) while the window is open takes its step back, and the
window reads its tiles again: it shows them as they are now, and what
you change in it after is written from there, that change alone (Redo
brings its step back, which Cancel still takes back). So it does after
any other change to its tiles (a brush, a paste, a plugin).

**Height** (Moonglow's own) is the tiles' `Tile_Height`, in the
tileset's height steps (the meters are written beside it): **-** and
**+** lower and raise them a step, and any number of steps can be typed
or dragged, below 0 too. It is set on the tiles alone: no neighbor,
corner or terrain around them changes, so a tile raised alone stands
above the tiles around with gaps between (what the builder asked for,
to build up a single tile), and the area view draws it, its walkmesh
and what stands on it there. What stands on a tile (an object whose
position is over its square) rises or falls with it, as far. The game
stands a tile where its `Tile_Height` says, whatever its neighbors'
heights: tested from 3 steps below 0 to 1000 above (5 km on Rural's
5 m steps), the walkmesh with it; Moonglow takes from -1024 to 1024.
With several tiles chosen of different heights it shows **Mixed** until
one is set, which they all take (**-** and **+** step from the first's).
It changes at once, with the lights in the window's one step, as
above. A tile of a group or a feature changes
height only with the whole group chosen (in Select Tiles, all its
tiles), so that the group stays whole; otherwise the control is greyed
out and says why (unless **Allow editing tiles of groups and
features** is on). The minimap shows a tile's picture whatever its
height, as the game's does.

**All tiles (ignore fit)** (Moonglow's own; off each time the window
opens) shows every tile of the tileset in place of the variants, in the
tileset's order, groups' pieces too, with **Turn** (0°, ↺90°, ↺180°,
↺270°) and the same **Size** slider; only the rows in sight are drawn,
however many tiles the tileset has (a hak's of 1,872 tiles as readily as
the game's). Its **search** narrows the pictures to the tiles whose
model's name or number in the tileset has what you type (`a04`, `117`),
the count saying how many of all; Escape in it clears it. Choosing a Turn turns the tiles
chosen at once: each keeps its own tile and its lights and loops, and
lies as the Turn says, put in as it is, whether it fits its neighbors
or not (none of them changes); each Turn chosen is one step to undo.
The pictures show the tiles turned so. Click one and it is put in the
tiles chosen as it is, turned so, at the height they have (and the
window's **Height**, if it is set), whether it fits its neighbors or
not: none of them changes (as **📌 Exact tile** puts one in). It is one
step to undo, the window stays open on it, and its lights are chosen
afresh (but for those set in the window), its doors come and go, and
what stands on it moves with a new height, as above. Off, the variants
are only the tiles that fit, as ever, and Next and Previous Tile
Appearance and Rotate Tile only ever give a tile that fits. Not for a
tile of a group or a feature, unless the option above is on. A piece
of a group put in alone is taken, after, for what is left of a group
(see above: only the Eraser or the option changes it).


A tile's replacement texture (`Tile_ReplaceTex`, a `replacetexture.2da`
row for a texture named `replace_tex` in the tile's model) isn't in the
window: it is BioWare's unused field, no tile of the game's tilesets has
such a texture, and whether the game draws it wasn't seen. One already in
an area is kept, and shown among the raw fields of the area's `.are`
(its row's menu in the module tree), where it can be edited.

**Edit › Resize Area…** grows or shrinks the area at its north and east
edges, as Aurora does, or at the two you choose there (east and south,
south and west, west and north): what is in the area keeps its place
against the two edges that stay; **Edit › Rotate Area…** turns it by 90° steps, objects and all.
**Build › Area Statistics** counts the area's tiles and objects and the
memory its models take.

## Claimed and protected areas

In a project that keeps a [claims table](03-modules.md#claimed-areas),
an area another builder claimed says so above its view (and its
Properties), with who and since when: its changes are either refused
(the project keeps others' areas read-only) or asked about before a save
writes them. An unclaimed one says so too where the project warns of
those, with **Claim This Area** beside it. The area's tab menu and its
menu in the module tree have **Claim This Area** or **Release Claim**,
which write the table. A merged (or protected) area says its Tag and
ResRef are kept; changing either, or deleting it, is
[asked about first](03-modules.md#protected-areas), with a copy offered
instead.

## Area Properties

Right-click the area in the module tree › **Properties**, or **Area
Properties** on the viewer's toolbar:

- **Basic**: name, the tileset (fixed once made) and the size.
- **Visual**: the lighting scheme, day and night (or always one), the sun and moon colors,
  fog colors and amounts, shadows (the sun's and the moon's, and how dark
  they are: the view draws them, see **Shadows** above), the skybox, lightning, rain and snow
  chances, as Aurora's Customize Environment sets them. The schemes are
  listed in as many columns as the page is wide; once one is clicked
  (or reached with Tab), the arrow keys choose another: Up and Down down
  the columns, Left and Right to the column beside, Home and End the
  first and last. Under them:
  - **Copy From Area** gives this area what another's Visual page holds
    (its lighting, fog, shadows, sky and weather), as one undoable step.
  - **Saved Lighting** keeps this page's settings under a name (**Save
    This Lighting…**) and gives lighting kept so to this area: yours
    from any module, as small files in Moonglow's data folder
    (`lighting`), easy to share. A tile's own lights aren't part of it
    (a scheme picks those).
  With several areas edited together, both give each of them the
  lighting.
- **Grass**: the grass the area grows, by surface material (see
  [Grass](#grass)).
- **Audio**: day and night ambient sounds and their volumes, music, battle
  music, the environment's sound effects.
- **Events**, **Advanced** (the tag, the loading screen, Variables; flags
  such as interior, underground, natural, no rest, PvP; **Shader Flags**, the area flags past those three, which
  custom shaders read from their `areaFlags` uniform: they do nothing
  with the stock shaders, and each bit means what a hak's shader makes
  of it), **Comments**.
- An area's **Variables** are kept in its GIT, where the game reads them
  (a script's `GetLocalInt` on the area). Before 1.34.3 Moonglow wrote
  them into its ARE, which the game does not read, so they read 0:
  opening the area's Variables moves them to the GIT (one step to undo;
  a variable of the same name already there keeps its value), and
  **Verify Module** lists the areas that still hold some in their ARE.

### Grass

The game grows grass on the walkmesh faces of an area's tiles, by their
surface material (a row of `surfacemat.2da`). The tileset's grass (its
`[GRASS]` section: a texture, density, height and colors) grows on
**Grass** faces (material 3). An area can turn it off and grow grass of
its own on any material: on Grass in place of the tileset's, or on dirt,
sand, snow, even the wood floor of an inn. These are what the game's
`SetAreaGrassOverride` and `SetAreaDefaultGrassDisabled` set while it
runs; the Grass page sets them from the start, and the area view grows
them as you change them (while a value is dragged or a color picked, as
it goes; each change is one step to undo).

- **Grow the tileset's grass on Grass faces**: off, the tileset's grass
  is gone (`SetAreaDefaultGrassDisabled(oArea, TRUE)`). Under it, what
  the tileset grows. A tileset without grass (an interior) says so.
- **The Area's Own Grass**: a box for each material with grass of the
  area's own, and **Add Grass to…** for another (it starts as the
  tileset's grass). In each: the **Surface Material** (one already
  growing the area's grass isn't offered twice), the **Texture** (a
  ResRef; the game's are four tufts, two by two: `grass`, `ttz_grass`),
  the **Density** (tufts to a square meter of the faces), the **Height**
  (meters), the **Ambient Color** and **Diffuse Color** (the light they
  take, as the tileset's are), and **Remove**.

**Where it is kept.** In the area's GIT (`area.git`), in its
`AreaProperties` struct, where the game reads it when the module loads
(the same fields in the ARE are not read; Moonglow's engine test
`engine_grass` saves a game on the server and reads them back):

| Field | Type | |
|---|---|---|
| `GrassDefDisabled` | BYTE | 1: the tileset's grass is off |
| `GrassOverrides` | struct (id 101) | holds `GrassOvrList` |
| `GrassOvrList` | list of structs (id 0) | one for each material |
| `GrassMaterialId` | BYTE | the surfacemat.2da row |
| `GrassTexture` | CExoString | the texture's ResRef |
| `GrassDensity`, `GrassHeight` | FLOAT | as on the page |
| `GrassAmbientX`, `Y`, `Z` | FLOAT | the ambient color's red, green and blue, 0 to 1 |
| `GrassDiffuseX`, `Y`, `Z` | FLOAT | the diffuse color's |

They are no local variables: a script can't read them. A script that
changes grass while the game runs calls `SetAreaGrassOverride`,
`RemoveAreaGrassOverride` or `SetAreaDefaultGrassDisabled`; what it sets
lasts in a saved game (which keeps it in the same fields), and the module
starts again with what the GIT holds. A server that keeps grass settings of
its own (in its database, say) applies them in the area's OnEnter or the
module's OnModuleLoad, over what the GIT holds.

From the command line, `mg areas` sets them on many areas at once:
`--grass Dirt:grass:3:0.5:0.5,0.6,0.3` (the material by number or
surfacemat.2da label, the texture, density, height, then the ambient and
the diffuse colors, which may be left out), `--remove-grass Dirt` and
`--tileset-grass no`.

Moonglow draws the grass closely enough to judge an override by: a
tuft's size, the density and the colors match the game near its camera.
The game's tufts gather in patches, Moonglow's are spread evenly.

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
- **Give Every Area This Page's Settings** sets what the page shows (the
  first area's) on all the others, whether changed here or not: to make
  the areas alike in what they already have. On the Visual page, an area
  whose lighting scheme was another also gets its tiles' lights from the
  first's scheme, as choosing the scheme gives them (with the game's
  data, which the scheme's colors come from); an area of the same scheme
  keeps its tiles' own.
- **Loading Screen** (Advanced, and a door's or trigger's Area
  Transition page) shows the chosen screen's picture under the list, and
  each screen's when the pointer rests on its name in the list.
- Interior, natural, underground and the shader flags change alone: each
  area keeps its other flags.
- **Variables**: the variables you add or change are set on each area,
  those you delete are deleted from each, and each keeps its others.
- **Grass**: turning the tileset's grass off or on is set on each area;
  a material's grass, added, changed or removed, is that material's in
  each area (one an area has for it already changes in place). Give
  Every Area This Page's Settings gives them the first area's grass as
  it is.
- Names, tags and comments are an area's own, and aren't offered.

To give every underground area the same music: Underground in the
chooser, Tick Shown, Edit Together, then Audio. From the command line,
`mg areas` does the same (see [Command-line tools](11-command-line.md)).

## Static placeables, many at once

The game merges an area's **static** placeables into its tiles when the
area loads, so they cost less to load and draw than dynamic ones. An
area's right-click menu in the module tree has **Make Placeables
Static**, and the **Areas** group's has **Make Placeables Static in Every
Area**: each sets Static on the placeables that lose nothing by it, as
one undoable step, and the log says how many it made static and how many
it left, and why. Left dynamic are placeables that:

- are **Useable**;
- are tilted, scaled or moved by a visual transform (a static placeable
  has none);
- have a script, a conversation, a trap or an inventory, or an animation
  switched on (a lit torch): a static placeable plays no animations and
  can't be destroyed;
- have local variables: scripts set and read them (a lore stone, a
  lamp, a workbench), and what they do to the placeable a static one
  doesn't show. The server keeps a static placeable as an object, found
  by its tag with its variables, but made Plot, and players see it as
  part of the tiles: lit or put out, opened, destroyed, it stays as it
  loaded. The log names each kept for its variables;
- or the [project file](03-modules.md#the-project-file-moonglowtoml)
  says so (`[make_static]`): its tags or blueprints by name, and which
  variables count (all, unless it names them).

One a script destroys or animates by its tag, with no variables, can't
be told from the rest: make it dynamic again from its menu in the area
view. **Make
Placeables Dynamic**, on the same menus, clears Static on every
placeable of the area or the module. From the command line, `mg areas
--static-placeables` and `--dynamic-placeables` do the same.

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
**Save** are in the editor's toolbar (Undo's and Redo's keys are the
tileset's too with the pointer over the editor); saving the module saves
open tilesets too, and closing one with unsaved changes asks first.
- **General:** the name, the name players see, the height step, the
  border, default and floor terrains, interior, height transitions, and
  grass.
- **Terrains and Crossers:** the lists, and **Add Terrain** and **Add
  Crosser** by name. **Rename…** beside one renames it, and every place
  the file names it follows: the tiles' corners (a crosser: their
  edges), the general border, default and floor terrains and the
  painting rules (one step to undo). A name another terrain or crosser
  has, one with a space, or one longer than 16 characters (a palette
  names it as a ResRef) is refused, saying why. Areas name tiles by
  number, so they need nothing; when the file is saved, the palette
  beside it (`<tileset>palstd.itp`) follows the names changed since the
  last save, the rest of it as it was.
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
