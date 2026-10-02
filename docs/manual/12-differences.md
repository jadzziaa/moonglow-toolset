# Differences from Aurora

Moonglow aims to do what Aurora does and write what Aurora writes. Where
it differs, it is on purpose:

- **Edits apply as you make them.** Property windows are tabs with no
  OK or Cancel; every change is one undoable step, and **undo is
  unlimited** (Aurora's area undo has a set number of levels).
- **Tabs and windows, not dialogs**: every editor is a tab or a window
  that stays open beside the others; areas stay in the main pane.
- **The Welcome tab** is the start page, rather than a dialog at start.
- **Module folders** and archives open the same way; there is no "always
  open module directories" option.
- **Recovery copies** of unsaved work, offered back after a crash, are
  Moonglow's own (Aurora has none).
- **Sounds** fade with the distance from where the area view looks;
  there is no 3D panning, so Aurora's 2D/3D bias, listener and 3D
  provider options are not there.
- **Spell Checking**: Enhanced Edition ships no dictionary, so Aurora's
  spell check finds nothing; Moonglow leaves it out.
- **Environment maps** on creatures and items are always drawn, as the
  game draws them.
- **Grass** is not drawn in the area viewer, as in Aurora (the game's
  grass does not show at the distances the editor views from).
- **Waypoints and merchants** are yellow arrows along their facing, sounds
  boxes.
- **Locked objects** carry a field of Moonglow's (`MG_Locked`), which the
  game ignores and Aurora drops when it saves.
- **Blueprint previews** in the door and placeable windows are a
  **Preview** button that opens the model viewer.
- **Renaming custom blueprints in the palette** is done in the blueprint's
  editor (in Aurora 1.89 the palette's in-place rename does nothing).
- **The Creature Template tab** is not shown (Aurora 1.89 hides it too),
  nor the Item Wizard's Magical, Item Level and Quality (disabled in
  Aurora).
- **The renderer** is Moonglow's own: it approximates the game's lighting
  closely (checked against the game's screenshots) but is not the game's
  shaders.

Not done yet: dragging items between inventories (use the palette and
Remove), the Conversation Editor's Scrap tab (the clipboard holds one
copied or cut branch), and the Sound Wizard's category filter (the sound
picker opens filtered to ambient or positional sounds instead).
