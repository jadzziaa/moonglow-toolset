---
type: Decision
title: Tooltips to the style guide
tags: []
status: stable
generated: { by: trail, at: 2026-11-09T00:00:00Z }
touches: [crates/mg-ui/src/icons.rs, crates/mg-ui/src/commands.rs, docs/style-guide.md]
fingerprint: v2-85a5863361856e6e
created: 2026-11-09
author: August
description: "Guide to finding every tooltip in mg-ui for a style-guide sweep: the call sites, icon helpers, command tables, tuple arrays and helpers to check, what a tokenizer finds (~870) versus hand grep, and what to skip."
asks:
  - "Where are all the tooltips in mg-ui defined so I can review them against the style guide?"
  - "Why does a tooltip on an icon button not need to repeat the button's name?"
  - "Which hover texts should I leave alone when sweeping tooltips, like the area view status line?"
  - "I changed tooltip wording and a UI test failed on get_by_label; what text format do tests expect for icon-only labels?"
---
Where mg-ui's tooltips live, for a sweep of them: on_hover_text / on_disabled_hover_text / on_hover_ui calls; icons::Tools button/toggle/toggle_word/checkbox_word/add/named (the tip is the last string argument, and with Icons only Tools::tip puts the name first, so a tip need not repeat it); commands.rs Id::text's third element (menu hints) and TOOLBAR's fourth; and many tips sit in tuple arrays iterated with for (label, tip, ..) (area_view's Arrange and Test menus, paint_mode's modes, tile_select's tile steps, palette_view's type buttons, ITEM_FLAGS, the terrain tools' tips in terrain_mode, files_already's [replace, skip], ok_cancel_saying's tips) or in helpers (transfer::scratch_tip, project_commands::tip, SetKind::from_file_tip). A tokenizer over the Rust sources that finds those calls and resolves a variable argument to its let or const found most (about 870); grep the tuple arrays by hand. Not tooltips, leave them: the area view's status line (its let status, hovered because the label truncates) and labels hovered with their own text (table and talk-table about lines). Tests read some tooltip text: icons-only labels are 'Name\nTip' (get_by_label('Palettes\nBlueprint palettes')), and blueprints.rs looks for the mixed-values tip by its words.
