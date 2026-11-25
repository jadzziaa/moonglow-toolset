---
type: Manual Page
title: Differences from Aurora
description: Where Moonglow deliberately departs from Aurora.
tags: [manual, aurora, differences]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-25T08:44:45Z }
---

# Differences from Aurora

Moonglow aims to do what Aurora does and write what Aurora writes. Where
it differs, it is on purpose:

- **Edits apply as you make them**, each one undoable step, and **undo
  is unlimited** (Aurora's area undo has a set number of levels). A
  placed object's **Properties** in the Right Dock, and a tile's **Tile
  Properties**, have no OK or Cancel: each change is a step of its own,
  which Edit › Undo takes back. The editors that open as tabs (a
  blueprint's Properties, a placed object's opened with **Open in Tab**,
  Module and Area Properties, a conversation, the journal, the factions)
  have **OK** and **Cancel** at the right end of their header, meaning
  what Aurora's do: Cancel sets back what was changed in the tab since it
  was opened, as one step. A 2DA, a hak, a tileset, the talk table and a
  script are saved when you ask, so their OK saves and closes, and their
  Cancel closes as last saved (after asking).
- **Tabs and docks, not dialogs**: every editor is a document tab that
  stays open beside the others (or a window of its own, with Options ›
  General's **Open editors** set to As Floating Windows; areas are always
  tabs). Some of Aurora's dialogs are panels here, in the Bottom Dock:
  Find Instance's hits are in **Search Results**, Build Module's results
  in **Problems**, and the Hak Pak Conflict Analysis is a report tab.
  Moonglow's own Find and Replace Text and Verify Module list theirs
  there too, and Edit Areas Together, Custom Light and Bake Walkmesh to
  Transform are sections of the Right Dock.
- **The Welcome tab** is the start page, not a dialog at start.
- **Stores are Stores everywhere**: the area view's filters, the module
  tree, the palettes, Verify, Find Instance, the wizards and the
  options. Aurora says Merchants on some of these (its area filters,
  Paint Merchants on its palette's buttons, the Script Wizard's Start a
  Merchant) and Stores on others; the game's Merchant faction and the
  palette's Merchants category, the game's own names, are as it has
  them.
- **A `.mod` with its folder beside it** asks which to open, as Aurora
  does, and Aurora's Always open module directories is one of three
  settings of Options › General's **A .mod with its folder beside it**
  (ask, the folder, or the `.mod`, which Aurora has not). A module
  folder also opens by itself, with no `.mod` beside it (**File › Open
  Folder**), as Aurora's Open lists only `.mod` files.
- **Recovery copies** of unsaved work are offered back after a crash
  (Aurora has none).
- **Sounds** fade with distance from where the area view looks. There is
  no 3D panning, so Aurora's 2D/3D bias, listener and 3D provider options
  are not there.
- **Spell Checking**: Enhanced Edition ships no dictionary, so Aurora's
  spell check finds nothing; Moonglow has none yet (Build Module shows
  the box, switched off, as "Not yet").
- **userpatch.ini's haks** (the HD Pack and the like) and **Steam
  Workshop content** can be read as the game reads them (Options ›
  Folders, under Game Content; both off by default as in Aurora, which
  can't read them);
  Verify warns of a reference only they or your `override` folder
  satisfy.
- **Environment maps** on creatures and items are always drawn, as the
  game draws them.
- **The tile border** past an area's edge, which the game draws and
  Aurora doesn't, can be drawn in the area viewer: the header's **Tile
  border** (its Lighting popover), off by default ([Areas](04-areas.md#the-header)).
- **A press on an object with a tile or a group in hand** is the
  object's: a click selects it and a drag moves it, the brush staying
  chosen, where Aurora's brushes take every click. A press on bare ground
  places as ever, and the painting brushes (terrains, crossers,
  Raise/Lower) take every press, as Aurora's do ([Areas](04-areas.md)).
- **Doors go anywhere.** Aurora places a door only on a tile's door
  hook; Moonglow puts it on the hook within 4 m, as Aurora does, and
  with none in reach, or with **Alt** held, where it is clicked or
  dropped, turned by Q and E (Aurora's way round was to place it on a
  hook and move it with Adjust Location). A placed door dragged within
  reach of a hook snaps to it. A change of tiles (painting, Change
  Tileset, a resize) takes away only the doors standing on the hooks of
  the tiles it changes, and leaves a door placed off them where it
  stands ([Areas](04-areas.md#placing-objects)).
- **Grass** is drawn in the area viewer, which Aurora does not draw: the
  tileset's and the area's own (Area Properties › Grass, which Aurora
  hasn't), wherever the view shows grass ground, thinner far off but
  not faded (the game draws it within 30 m of its camera by default,
  fading each batch of tufts out by its distance from the camera, from
  about 16 m to nothing at about 32 m; its Grass Far Render Distance, in
  square meters, takes it farther: 3,600 is 60 m; an editor's camera is
  often farther off than the game's, and the game's fade would leave it
  no grass). It sways in the wind as far and as fast as
  the game's, measured in the game in a strong and a weak wind: how a
  swing runs between its ends is taken to be even, it sways north and
  south as measured (whether the area's wind turns it was not read), in
  no wind it stands
  still (the game's was not measured there), and grass past 180 m (drawn
  there, not in the game) stands still. Its tufts gather in clumps with bare ground between them,
  as the game's do (fitted to the game's picture of a field), as dense
  near the camera as the game's; the game's sparse grass lies a tuft at
  a time, where Moonglow's lies in twos and threes. Aurora drops an area's own grass when it
  saves the area ([Areas](04-areas.md#grass)).
- **Waypoints** are the flags of their appearance (blue, red, green,
  yellow), **sounds** the game's markers for them (a note: heard from
  where it stands, from a random position, or everywhere in the area)
  and **stores** a yellow arrow along their facing, or the game's $
  sign (Tools › Options › Area › Show stores as $ signs). All are in
  their own colors whatever the area's light, and hidden by what stands
  in front of them.
- **A placeable's illumination** (Advanced › Illuminated, the game's
  `LightState`) can be set, which Aurora doesn't show, and the area view
  draws it; and the faction
  lists don't offer the PC faction to objects (whether Aurora's do was
  not captured).
- **Locked objects** carry a field of Moonglow's (`MG_Locked`), which the
  game ignores and Aurora drops when it saves; in a nasher project the
  locks are kept in Moonglow's data folder instead.
- **An encounter's own outline color** is a field of Moonglow's
  (`MG_OutlineColor`), which the game ignores and Aurora drops when it
  saves; a nasher project's files keep it.
- **An exported archive says what it was made from** (the module's git
  commit, Moonglow's version, the module's name) after its description,
  and Import warns when the module is at another commit.
- **Blueprint previews**: Aurora draws a door's or a placeable's model
  inside its properties window; Moonglow shows it in the Right Dock's
  **Preview** panel beside the blueprint's tab (its **Preview** button
  brings the panel to the front), and from a placed object's Properties
  in the Right Dock, **Preview** opens the model viewer.
- **Renaming custom blueprints in the palette** is done in the blueprint's
  editor (in Aurora 1.89 the palette's in-place rename does nothing).
- **The Creature Template page** is not shown (Aurora 1.89 hides its tab
  too).
- **A creature's sound set and faction** are on its Basic page (the sound
  set after the gender), not Advanced: builders asked to choose them
  with the rest.
- **The Item Wizard's Magical** gives only properties the base item can
  have: Aurora's tables give ranged weapons Vampiric Regeneration and
  throwing weapons Freedom of Movement, which itemprops.2da doesn't allow
  them. Its random
  choices are Moonglow's own, made as Aurora's made items show (the same
  tables, strengths, number of properties and most an item is worth),
  not draw for draw.
- - **The renderer** draws with the game's own shaders (**Shaders**, in
  the area header's Lighting popover and in the model viewer's header, on
  by default), compiled from the game and the module's haks: each mesh
  with the program the game would choose for it, as read from the game's
  client; the sky, grass, particles and shiny water too
  ([Areas](04-areas.md)). With the switch off, Moonglow draws everything
  with its own shader, which closely approximates the game's stock
  lighting (checked against the game's screenshots). Either way a mesh is
  drawn as the game draws it: a texture's `bumpyshinytexture` reflects
  nothing, normal and specular maps show only with a render hint, a DXT1's
  transparent texels are black.

- **Static keeps a placeable's visual transform.** Aurora takes a
  placeable's visual transform off when **Static** is checked; Moonglow's
  Properties keep it in the file, so that unchecking Static brings it
  back. The game draws a static placeable without it either way, and
  Adjust Location and Verify say so. (**Make Static** in the area view
  takes it off, as Aurora does.)
- **A pasted trigger lands on the ground.** Aurora's paste sets the
  copy's height to the ground's where it is put down and keeps its
  corners' heights, which already were the ground's: the copy stands as
  high again in the air (5 m on a tileset whose ground is 5 m up), and
  an area transition so is never shown nor clickable in the game.
  Moonglow puts each corner on the ground under it, as Aurora's drag
  does; and a trigger dragged at a height other than 0 keeps its
  corners on the ground too (Aurora's drag leaves them off by that
  height).

- **Setup Area Transition** ([Blueprints](05-blueprints.md#placeables-and-doors))
  writes what Aurora's writes, but:
  - **A trigger's waypoint is used again.** Aurora makes another
    `WP_<tag>` waypoint every time a trigger is linked (three stacked
    after three runs); Moonglow uses a waypoint of that tag already
    standing inside the trigger's outline, and makes one only where there
    is none. (Two triggers of one tag in one area still get one each.)
  - **Target To Current writes the target.** Aurora loses the target's
    half unless the target's area is saved later for another reason;
    Moonglow writes it.
  - **One step, on the window's OK.** Aurora writes the current object's
    link only on its Properties' OK, so its Cancel leaves the target
    leading to an object that leads nowhere. Moonglow writes both ends and
    the waypoints on Setup Area Transition's own OK, one step to undo,
    which a Cancel of the object's Properties tab leaves whole.
  - **Lines, not questions.** Aurora asks before going on when a tag is
    still its blueprint's, to save the current area, and before replacing
    a link. Moonglow needs no save, and says the rest in the window, under
    its lists, before OK is pressed.

Moonglow has no Conversation Editor Scrap tab (the clipboard holds one
copied or cut branch) and no Sound Wizard category filter (the sound
picker opens filtered to ambient or positional sounds instead).
