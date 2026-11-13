---
type: Decision
title: Driving Aurora on the oracle for checks
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [tools/aurora/xdrive.py, tools/aurora/capture_wave1.py]
fingerprint: v2-f67df2d642cc0cfa
created: 2026-10-29
author: August
description: "Practical tips for driving Aurora 89.8193.37 under the oracle harness (headless.sh, run-aurora.sh, xdrive.py): opening late modules, flaky menus, hidden dialogs, palette behavior and palcus.itp rewrites, typing into fields, and shell-guard limits."
asks:
  - "How do I open a module in Aurora that I copied in after the welcome screen already loaded?"
  - "A dialog vanished after I clicked and I can't find it in Aurora under xdrive.py; how do I bring it back?"
  - "Which actions actually rewrite palcus.itp, and why did Ctrl+S leave the file unchanged?"
  - "My menu clicks in the automated Aurora session do nothing every second time; what is going wrong and how do I avoid it?"
---
Driving Aurora 89.8193.37 under the oracle harness (headless.sh, run-aurora.sh, xdrive.py), learned while checking 28 deferred entries: (1) The welcome screen's module list is built at start: a module copied in later is opened with Ctrl+O and its name typed into Module Name. (2) Menus open on every other click of the menu bar when one was just closed; take a shot to see one is open before clicking an item. (3) A dialog can end up behind the main window (a click on its status bar instead of Cancel): xdrive.py raise TITLE brings it back. (4) Palette right-click menus: New/Find/Find Next/Refresh Palette on a category, Edit/Edit Copy/New/Delete/Update Instances/Export/... on a blueprint; Refresh Palette and Save As are what rewrite palcus.itp (Ctrl+S with nothing changed writes nothing). (5) The custom palette shows palcus.itp's own names; the module's skeleton is read only by Select Category and Refresh Palette. (6) Its ini's LanguageID doesn't switch Aurora's language (still English). (7) Fade Geometry shows on the rural tiles' trees (tilefade 1) seen from above, not on an interior's walls. (8) Add to Palette of a placed creature re-points it and its items at the new blueprints: do Update Instances tests on another creature. (9) Typing into a field: click, End, Shift+Home, BackSpace, type, Tab (fill.py in the wave's scratch); limits show after Tab. (10) The worktree shell guard refuses heredocs and loops with variables: write scripts in the scratchpad.
