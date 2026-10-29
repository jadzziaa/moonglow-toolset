---
type: Manual Page
title: Differences from Aurora
description: Where Moonglow deliberately departs from Aurora.
tags: [manual, aurora, differences]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-29T21:10:00Z }
---

# Differences from Aurora

Moonglow aims to do what Aurora does and write what Aurora writes. Where
it differs, it is on purpose:

- **Edits apply as you make them**, each one undoable step, and **undo
  is unlimited** (Aurora's area undo has a set number of levels). The
  editors' windows (a blueprint's Properties, a placed object's, Module
  and Area Properties, a conversation, the journal, the factions) still
  have **OK** and **Cancel** as Aurora's do: Cancel sets back what was
  changed in the window since it was opened, as one step. A 2DA, a hak,
  a tileset, the talk table and a script are saved when you ask, so
  their OK saves and closes, and their Cancel closes as last saved
  (after asking).
- **Tabs and windows, not dialogs**: every editor is a tab or a window
  that stays open beside the others; areas stay in the main pane.
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
  Folder…**), as Aurora's Open lists only `.mod` files.
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
  General, both off by default as in Aurora, which can't read them);
  Verify warns of a reference only they or your `override` folder
  satisfy.
- **Environment maps** on creatures and items are always drawn, as the
  game draws them.
- **Grass** is drawn in the area viewer, which Aurora does not draw: the
  tileset's and the area's own (Area Properties › Grass, which Aurora
  hasn't), wherever the view shows grass ground, thinner far off (the
  game draws it within 30 m of its camera by default, fading it in at
  about 24 m). Its tufts are spread evenly, where the game's gather in
  patches. Aurora drops an area's own grass when it saves the area
  ([Areas](04-areas.md#grass)).
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
- **An exported archive says what it was made from** (the module's git
  commit, Moonglow's version, the module's name) after its description,
  and Import warns when the module is at another commit now.
- **Blueprint previews** in the door and placeable windows are a
  **Preview** button that opens the model viewer.
- **Renaming custom blueprints in the palette** is done in the blueprint's
  editor (in Aurora 1.89 the palette's in-place rename does nothing).
- **The Creature Template tab** is not shown (Aurora 1.89 hides it too).
- **The Item Wizard's Magical** gives only properties the base item can
  have: Aurora's tables give ranged weapons Vampiric Regeneration and
  throwing weapons Freedom of Movement, which itemprops.2da doesn't allow
  them. Gems and gauntlets aren't offered Magical (Aurora offers it, and
  gives them no properties but makes them unidentified). Its random
  choices are Moonglow's own, made as Aurora's made items show (the same
  tables, strengths, number of properties and most an item is worth),
  not draw for draw.
- **The renderer** draws with the game's own shaders (the area view's
  and the model viewer's **🎨 Shaders**, on by default), compiled from the
  game and the module's haks: each mesh with the program the game would
  choose for it, as read from the game's client. Water, particles and
  the sky are drawn by Moonglow's own shader, which closely approximates
  the game's stock lighting (checked against the game's screenshots) and
  draws everything with the switch off. Either way a mesh is drawn as the
  game draws it: a texture's `bumpyshinytexture` reflects nothing, normal
  and specular maps show only with a render hint, a DXT1's transparent
  texels are black.

- **Static keeps a placeable's visual transform.** Aurora takes a
  placeable's visual transform off when **Static** is ticked; Moonglow's
  Properties keep it in the file, so that unticking Static brings it
  back. The game draws a static placeable without it either way, and
  Adjust Location and Verify say so. (**Make Static** in the area view
  takes it off, as Aurora does.)

Not done yet: the Conversation Editor's Scrap tab (the clipboard holds one
copied or cut branch), and the Sound Wizard's category filter (the sound
picker opens filtered to ambient or positional sounds instead).
