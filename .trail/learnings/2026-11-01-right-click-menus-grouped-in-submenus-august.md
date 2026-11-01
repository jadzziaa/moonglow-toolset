---
title: Right-click menus grouped in submenus
author: August
date: 2026-11-01
branch: worktree-agent-a7f53c82ace4ab7f5
cost: { minutes: 2675, tokens: 18746519 }
touches:
  - crates/mg-ui/src/palette_view.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/tree.rs
  - crates/mg-ui/tests/app/common.rs
---

## Decided
- Right-click menus group runs of related commands in `ui.menu_button` submenus of one level (the owner's ask after 3.1.4, modeled on the toolbar's Shadows and Walkmesh… held menus). Grouped: the palette's blueprint menu (Replace Selected ›, Copy ›), the area view's object menu (Position › the drops and Random Facing; Reuse › Add to Palette, Export as Blueprint…, Save as Prefab…; Test › the two Test From Here), an area's and several resources' menu in the module tree (Placeables ›, Fields ›, Export ›). Item labels are unchanged, so the manual names paths (**Copy › Copy Tag**).
- Kept at the top: Adjust Location… (as in Aurora's pmViewerArea, and seven UI tests click it as the everyday command), Set Start Location Here, Lock, the kind-specific items (each shows only for its kind, so grouping them shortens no menu a builder sees), Delete, Variables…, Properties, and the plugins' commands where they were (the plugin manual describes that place).
- Left as they were: the tile menus (short; tile_select.rs is the tile tools' file), the tree's placed-object menu, tabs, conversation rows, browser, hak, 2DA column, script list, prefabs, inventories, color buttons: under about eight distinct items each.
- egui closes the whole menu when an item of a submenu calls `ui.close()` (SubMenu propagates `should_close` to its parent), so the items' code needed no change beyond the nesting.

## Watch out
- In the accessibility tree (kittest), a submenu's button is labeled with egui's arrow: `"Copy ⏵"`, not `"Copy"`. `common::open_submenu(h, "Copy")` adds it (hover, a frame, click); `submenu_items(h, name, an_item)` lists one. `menu_items` lists the open popup's own buttons only, a submenu showing as `"Name ⏵"`.
- A submenu name must differ from every item label inside it (kittest exact-match), and from labels shown elsewhere at the same time: no "Copy" submenu in the module tree, whose object menu has a Copy item.
