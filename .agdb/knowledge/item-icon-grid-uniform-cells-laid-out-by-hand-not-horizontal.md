---
type: Decision
title: "Item icon grid: uniform cells, laid out by hand, not horizontal_wrapped"
description: "Why the whole item's Appearance icon grid places cells itself and what broke skipping lines in egui's horizontal_wrapped"
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T12:52:48Z }
touches: ["crates/mg-ui/src/blueprint/item.rs:icon_grid"]
asks:
  - Why does the item icon grid not use horizontal_wrapped?
  - Why did skipping out-of-sight icon rows change the layout?
  - How is the icon cell size chosen and kept?
  - "Where is the 999-icon Appearance page's grid code?"
fingerprint: v2-5cd8fa1a99e126d5
created: 2026-11-26
author: augustaugur
---
blueprint/item.rs icon_grid (a whole item's icons) lays every icon in a cell of the largest icon drawn yet (owner's ruling 2026-11-26), kept in egui temp data at id.with("cell") as (hash of the numbers, Vec2); another base item's numbers restart it at 32x32. It allocates the whole grid's rect and draws only the lines that meet the clip rect, each icon in a new_child with a centered_and_justified layout and id_salt(n). Before, horizontal_wrapped gave icons in sight their own sizes and those out of sight the first one's, so which icons shared a line depended on the scroll position; three tries at skipping out-of-sight lines inside horizontal_wrapped each changed the layout (also: its cursor stays on the previous line until the next item is placed). part_grid (a composite item's parts) still uses horizontal_wrapped with per-number sizes. Tests: galleries a_whole_item_s_icons_keep_their_places_scrolled_through (cells evenly apart), a_whole_item_s_icons_take_the_room_of_its_base_item_s (reads the cell); screens whole_item_icons.
