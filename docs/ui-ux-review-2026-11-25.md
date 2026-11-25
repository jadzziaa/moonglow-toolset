---
type: Report
title: 'UI/UX waves 1 to 3 and their review: findings, rulings and fixes'
description: 'Compact reference (2026-11-25, branch ui-ux-fixes) to the three UI/UX waves (0e3c6195, 6da9e560, 62e1c507): what each answers, what the review of them found, the owner''s five rulings, and what was fixed on top in the working tree.'
tags: [review, ui, ux, accessibility, contrast, focus, copy, style-guide]
status: stable
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-25T10:35:01Z }
sources:
  - id: wave1
    resource: git show 0e3c6195
    title: Fix Wave 1 high-severity data-loss, contrast, focus, and UX bugs
  - id: wave2
    resource: git show 6da9e560
    title: Fix Wave 2 shell, floating window, area mode, and editor workflow bugs
  - id: wave3
    resource: git show 62e1c507
    title: Polish Wave 3 interface copy, style-guide consistency, and UI rough edges
  - id: debuginfo
    resource: git show d8e3fb21
    title: Strip dependency debuginfo and move builder_files lookup out of target
  - id: fixes
    resource: git diff 62e1c507 (working tree of branch ui-ux-fixes, 2026-11-25)
    title: The review's fixes on top of Wave 3, uncommitted when this was written
  - id: review
    resource: the UI/UX review's report to the owner, 2026-11-25 (not kept in the repository)
    title: 'UI/UX review, waves 1 and 2: read-only at 6da9e560'
  - id: owner
    resource: human:august
    title: The owner's five rulings on the review (2026-11-25)
---

# UI/UX waves 1 to 3 and their review: findings, rulings and fixes

Branch `ui-ux-fixes`, on top of `b127d58b`. The manual chapters were brought in line in the same change ([log](log.md), 2026-11-25). The fixes on top were not yet committed when this was written, so they are cited by what they do, not by a hash.[^fixes]

## What each wave answers

| Wave | Commit | Answers |
|---|---|---|
| 1 | `0e3c6195` | High-severity data loss: Adjust Location over several objects wrote the edited object's whole visual transform onto each (now only the fields changed); a script tab closed with unsaved text (now asks Save, Don't Save or Cancel); recovery's Discard deleted at once (now Confirm Discard or Keep). Tileset editor: Make Palette and Render Minimap Pictures ask before overwriting, and the minimaps render a tile a frame with a progress window and Cancel. Contrast: syntax colors on the code selection (`code_select`), Quick Open's unavailable rows. Focus rings on kit buttons, icon buttons and Problems rows. The update offer's Later renamed Install on Exit. Gallery rim drawn outside the tile.[^wave1] |
| 2 | `6da9e560` | Shell and floating windows: Maximize and Restore only for tabs in a floating window (a docked tab has none), the Status Bar's Escape hint for a floating window, Preview hints in a floating window. Area mode: Paint with nothing paintable chosen, Select Tiles' Escape (clear the selection, then leave Select Tiles), the hint "Esc: Clear selection / Select objects", handle chips kept inside the view, the mixed-markers cue past 64 others. Editor workflow: the journal's Add with an entry chosen and Delete keeping a choice, the 2DA grid's header following its rows, OK as the primary button, the page selector's scroll cue.[^wave2] |
| 3 | `62e1c507` | Interface copy and the style guide: tick to check across the UI (Check All, Check Shown, Uncheck All, Check a Set…, "N checked"), title case for menu items and groups (Appearance Gallery, Manage Plugins, NUI Windows, Journal and Factions, Without a Module, All Tiles (Ignore Fit)), plurals instead of "(s)" (Choose N Areas…, objects, times, ranks), "…" only where a dialog follows, Counterclockwise, tips rewritten (NUI, plugins' time limit, Search, light maker, settings tidy), the prefab list renamed at once, the compass's N.[^wave3] |
| — | `d8e3fb21` | Dependencies built without debug info in dev and test; builder files looked up in `~/.local/share/moonglow-builder-files` first, the old path as a fallback.[^debuginfo] |

## The review's findings

The review of waves 1 and 2 (read-only at `6da9e560`: one blocker, seven to fix, five for the owner's ruling) and of Wave 3's copy. Each was fixed in the working tree unless a ruling below says otherwise. The review also found sound: Adjust Location's data-loss fix, recovery's Discard confirmation (5.2.0's recovery files still read), the focus rings on accent fills, brighter dark conversation colors, the overwrite questions, the journal's Add and Delete, the 2DA header, and `d8e3fb21`.[^review][^fixes]

| Area | Finding | Kind |
|---|---|---|
| Floating windows | A floating window's tab no longer maximized on a double click, and its menu always said Maximize: the TabViewer read `app.dock` while the dock is swapped out (empty) during drawing | Wave 2 regression |
| Scripts | A script closed with nothing unsaved lost its bookmarks, Undo history, codepage and External Editor link when opened again (the buffer was dropped on every close) | Wave 1 regression |
| Scripts | Open question: whether Edit › Undo follows a script's own history in time (a deleted comment said it could not be known) | question |
| Side editors | Cancel's tip said "Close without saving uncommitted changes", but it asks first | copy |
| Options | Closing-cancels tip listed editor kinds in a run-on; Show file extensions tip too long | copy |
| Area view | Paint on with nothing paintable chosen swallowed all input (no select, drag, Delete, copy, paste); a door, waypoint, appearance, light or start location in hand was still placed by a click, with its ghost | Wave 2 over-reach (ruling 5) |
| Area view | Select Tiles: Escape during a box drag also cleared the selection or left the mode | gap |
| Area view | Adjust Location's multi-object fix tested on its preview only, not on the path that writes (OK, Apply) | test owed |
| Area view | Mixed-markers cue said "more than 64" where 66 or more objects were selected | off by one |
| Area view | Paint's hint gave no reason where a non-placeable was chosen; Rotate Tile's and Delete's tips ("{k}, {c}: clockwise", "counter-clockwise", empty "Eraser ()") | copy |
| Area view | Leftover "tick" in area and header tips (Edit Areas Together, Creature shadows, Static) | copy |
| Kit | Slider thumb fix took no effect (the kit's thumbs are rectangles, the fix restyled circles) and left a ring in egui's text color, near white in dark, darker in light | look |
| Kit | Color swatch button: hover and open no longer showed on its border, and the new focus ring, drawn outside, could be clipped in a tight grid | look |
| Kit | A focused selected icon button, or a focused toggle that is on, showed two rings | focus |
| Kit | Page selector's scroll cue a dimmed `--bar` on a list whose ground is `--panel` | look |
| Kit | Gallery hover rim in light `--raised-hover`: 1.45:1 on panel, 1.64:1 on inset | contrast |
| Kit | Compass N made white by Wave 3, invisible on the light disc | Wave 3 regression |
| Kit | Slider value text `--text-2` | look |
| Tileset | Make Palette set (or answered) to skip logged nothing | gap |
| Tileset | Cancelling the minimap overwrite question had already renamed the tiles' `ImageMap2D` | Wave 1 data change |
| Tileset | The minimap job's folder stayed mounted where something held the game data as the job ended | leak |
| Copy | Import warning ran on with a semicolon; tick in Export Changed, creature feats, 2DA merge and Advanced Search; NUI, plugins, Search and light maker tips over-long or unclear | Wave 3 defects |
| Prefabs | The prefab list kept the old name for a frame after a rename | Wave 3 gap |

## The owner's rulings (2026-11-25)

1. **Closing a script tab with unsaved text asks Save, Don't Save or Cancel.** Reverses the design of 2026-10-29, where a script's × kept its text for the module's Save and recovery ([log](log.md), noted there).[^owner]
2. **Pane sizes are kept for every window kind**, not scripts alone. Reverses the 5.2.0 W5 ruling (`KEPT_WINDOW_KINDS`, removed; [log](log.md) 2026-11-23, noted there).[^owner]
3. **The gallery tile's rim is drawn outside the tile.** Reverses [the redesign](ui-redesign-proposal.md) §8.4, which said inside (amended there).[^owner]
4. **The switch's light track off and the slider thumb's light ring stay as the proposal has them**, though both are under 3:1 ([the redesign](ui-redesign-proposal.md) §4.2, noted there).[^owner]
5. **Paint with nothing paintable chosen swallows only the placing click**, not all input.[^owner]

Later the same day, on ruling 1: a script tab closed with unsaved text and answered **Save** (and the header's OK, the same action) keeps the editor's state (bookmarks, Undo history, codepage, External Editor link) for the session, as a clean close does; **Don't Save** still drops it.[^owner]

Later again, on the header's Cancel: "header Cancel on a clean script: keep its bookmarks and history". A clean script closed by its header's Cancel keeps the editor's state as its × does; it had dropped it (`side_doc::may_cancel`, `scripts::a_clean_script_closed_with_the_header_s_cancel_keeps_its_bookmarks_and_history`).[^owner]

## Fixed on top of Wave 3

| Where | What |
|---|---|
| Shell | `Moonglow::window_tabs_maximized`, a snapshot taken before the dock swap, drives a floating tab's double click and its menu's Maximize or Restore. Pane sizes kept for every kind (ruling 2), and a size let go of as the pointer moves is kept (the pane lags the pointer a frame; `keep_window_size` keeps saving for three passes after the drag). |
| Scripts | Close asks (ruling 1); a clean close, the question's Save and the header's OK keep the editor's state for the session, Don't Save drops it (`scripts::a_script_closed_with_save_keeps_its_bookmarks_and_history`); Edit › Undo off with nothing typed. |
| Side editors, Options | Cancel's tip "Close the window, asking about unsaved changes first"; the two Options tips. |
| Area view | Adjust Location's write path over several objects tested (`area_input_fixes::adjust_location_on_several_keeps_each_object_s_own_values`). Paint (ruling 5) through one rule, `clicks_place` and `to_place`; Escape or a right click turns Paint off, a double click on the ground goes to Select Tiles. Select Tiles' Escape during a box drops the box alone. Mixed-markers cue from the limit ("more than 65 selected: mixed markers omitted"). Paint hint, Rotate Tile and Delete tips, tick to check. |
| Kit | Slider thumb: no ring in dark, 1 px `#8d949e` in light; value text `--text`. Swatch button raised with its color inset 3 px, lit hovered and open, a 1 px `--accent` border inside on open or focus. One ring per control. Scroll cue band in the selector's own ground, chevron `--text-3`. Gallery hover rim `--text-off` in light (2.81:1, 3.19:1), outside the tile (ruling 3). Compass N `--text`. [The redesign](ui-redesign-proposal.md) amended. |
| Tileset | Skip logs "Wrote no palette into … (1 existing skipped: …palstd.itp)"; the minimap question's Cancel changes nothing (names given only when rendering goes ahead, one Undo step); the folder is unmounted once the game data is free. |
| Copy | Import warning, Export Changed, creature feats, 2DA merge, Advanced Search, NUI, plugins, Search, light maker; the prefab list follows a rename at once. |

## The second round

An independent review of the fixes above found these, fixed in the working tree:

| Where | What |
|---|---|
| Pane sizes | The "dragged" slot was shared by every window of a kind, so a window drawn after another overwrote its mark and kept its own untouched size. It is now the window's own, keyed by its edges. |
| Paint | The swallowed click came before the paste and outline dispatch and ate their clicks. It now sits after them, only for a click that would go to placing and on a writable view. |
| Alt + drag | With Paint on and no placeable chosen it moved the camera; Paint counts only with placeables chosen (`paint_mode::active`), otherwise it is Select Objects. |
| Escape | During a paste, an outline drawn anew or a tile paste with Paint on, Escape turned Paint off; it now drops the inner thing and Paint stays on. |
| Minimap job | A failed start lost the folder mounted before (kept now until the new one is mounted); the unmount retry can finish where only the 3D views and haks hold the game data; tiles are named only once the job has started, and the log says why nothing was rendered. |
| Scripts | A closed script's kept editor state no longer counts as open: Find and Replace In Open Scripts, Edit › Undo and the script in front go by tabs (`script_open`). It is dropped when the script is deleted from the module. |
| Copy | Plugins, search and doctor tips reworded ("checking it again takes the transform off"). |

[^wave1]: `git show 0e3c6195`
[^wave2]: `git show 6da9e560`
[^wave3]: `git show 62e1c507`
[^debuginfo]: `git show d8e3fb21`
[^fixes]: `git diff 62e1c507`, branch `ui-ux-fixes`, 2026-11-25
[^owner]: The owner's rulings, 2026-11-25
[^review]: The UI/UX review of waves 1 and 2, 2026-11-25
