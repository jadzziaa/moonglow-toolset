---
title: "Long row lists and Add to Palette's 002"
author: August
date: 2026-10-30
branch: worktree-agent-a16a1ce7fc4c0cfd0
cost: { minutes: 60, tokens: 13347662 }
touches:
  - "crates/mg-ui/src/widgets.rs#RowChoice"
  - "crates/mg-module/src/palette_add.rs#new_resref_named"
---

decided: widgets::row_choice lays out a long list (more than FILTERED_CHOICES) with ScrollArea::show_rows, the chosen row put in the middle by a scroll offset (scroll_to_me can't reach a row that isn't laid out); the rows found (sorted by mg_rules::name_order, filtered) are kept in egui temp data by (choices ptr, len, filter) while open.
watch out: a combo box keeps the keyboard after its click, so the arrow-key branch ran every frame and sorted the whole list each frame (by_name on a copy): the order is made only when a Key or MouseWheel event came. That, not the layout alone, was most of the 93 ms a frame for 50,000 rows (release); 0.3 ms after. Tests that scroll_to_me a row of a long list out of sight now find it with the Filter (type_into_hint 'Filter'; Escape clears it).
ruled out: Add to Palette's 002 for unnumbered templates (wave1 add-palette.mod, pal2-addpal.mod) is not the bare template's being in the module: placement-doors.mod had mgp_utp.utp too and gave mgp_utp001. The captures differ in what was added (a creature and its items vs a placeable and its items); an Aurora capture must settle it.
