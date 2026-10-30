---
title: "Aurora's second batch: what its dialogs really do"
author: August
date: 2026-10-30
branch: worktree-agent-a2d3e99f52f49774c
cost: { minutes: 1902, tokens: 12930437 }
touches:
  - crates/mg-module/src/palette_add.rs
  - crates/mg-ui/src/creature_wizard.rs
  - crates/mg-ui/src/blueprint_wizard.rs
  - tools/aurora/capture_wave2.py
  - tools/aurora/xdrive.py
---

decided: Add to Palette names by kind (captures/wave2/add-palette.mod). Creatures and items: past the bare template where it exists (w2guard -> w2guard002, the 002 that puzzled wave 1). Placeables, doors, triggers, sounds, waypoints: the template minus one reserved prefix (nw_, plc_, xN_) when that name is free (nw_waypoint001 -> waypoint001, x3_plc_rubble1 -> plc_rubble1 -> rubble1), else Edit Copy's numbering. Encounters and stores were not measured.
decided: the Creature Wizard's portrait page is Aurora's Select Portrait with Race and Gender ticked: Sex equal to the gender only, table order, module portraits included; Moonglow keeps no-gender rows only when the gender has none (a builder asked for monsters).
watch out: Aurora's Select Property Parameters hides Chance of Appearing; there is nothing to type past. Setup Area Transition's Target To Current wrote nothing (twice); a placed probe trigger crashed the dialog (List index out of bounds).
watch out: driving Aurora: menus open on the second click after a close; a dialog that sinks behind the main window needs xdrive.py raise; the 'Inaccessable Objects' popup can't be closed, move it off with Xlib (window.configure(x=...,y=...)); multi-select tiles with a drag in Select Terrain; xdrive.py click now takes --ctrl and --shift.
watch out: the worktree shell guard refuses a command naming *.git files (field.git) and loops; scripts in the scratchpad with an alias for the extension.
