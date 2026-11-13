---
type: Decision
title: "Long row lists and Add to Palette's 002"
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: ["crates/mg-ui/src/widgets.rs#RowChoice", "crates/mg-module/src/palette_add.rs#new_resref_named"]
created: 2026-10-30
author: August
description: "widgets::row_choice uses ScrollArea::show_rows for long lists and sorts only on Key or MouseWheel events, cutting 50,000 rows from 93 ms to 0.3 ms per frame; the cause of Add to Palette's 002 numbering is still unresolved."
asks:
  - "Why was my combo box with 50,000 rows taking about 93 ms per frame, and what fixed it?"
  - "How does row_choice keep the selected row centered in a long list when scroll_to_me can't reach unlaid-out rows?"
  - "My test calls scroll_to_me on a row in a long list and can't find it. What should I do instead?"
  - "Why does Add to Palette give 002 for unnumbered templates, and is it caused by the bare template being in the module?"
---
decided: widgets::row_choice lays out a long list (more than FILTERED_CHOICES) with ScrollArea::show_rows, the chosen row put in the middle by a scroll offset (scroll_to_me can't reach a row that isn't laid out); the rows found (sorted by mg_rules::name_order, filtered) are kept in egui temp data by (choices ptr, len, filter) while open.
watch out: a combo box keeps the keyboard after its click, so the arrow-key branch ran every frame and sorted the whole list each frame (by_name on a copy): the order is made only when a Key or MouseWheel event came. That, not the layout alone, was most of the 93 ms a frame for 50,000 rows (release); 0.3 ms after. Tests that scroll_to_me a row of a long list out of sight now find it with the Filter (type_into_hint 'Filter'; Escape clears it).
ruled out: Add to Palette's 002 for unnumbered templates (wave1 add-palette.mod, pal2-addpal.mod) is not the bare template's being in the module: placement-doors.mod had mgp_utp.utp too and gave mgp_utp001. The captures differ in what was added (a creature and its items vs a placeable and its items); an Aurora capture must settle it.
