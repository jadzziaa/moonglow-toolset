---
title: "Saved lighting with tile colors, a scheme row via the 2DA editor"
author: August
date: 2026-10-30
branch: worktree-agent-a02be1f76ba28c11f
cost: { minutes: 60, tokens: 13321697 }
touches:
  - crates/mg-module/src/lighting.rs
  - crates/mg-ui/src/area_props.rs
  - crates/mg-ui/src/browser.rs
---

## Decided
- Saved lighting keeps a tile-light palette shaped like an environment.2da row (four lightcolor rows each for main1, main2, source, shared by use) and gives tiles picked from it as a scheme does (mg_module::lighting::pick_tile_fields, also Scheme::tile_fields). Copy From Area picks from the source area's tiles; Give Every Area keeps the scheme-row rule.
- Make a Lighting Scheme does not write haks itself: it opens environment.2da where the builder picks (browser::table_homes / open_table, the browser's Edit menu) and adds the row with Editing::add_row_of, to be saved there. No multi-hak merge needed: the row goes into the copy chosen.
## Watch out
- Area Properties' Form root has no Tile_List (stripped each frame for speed): read the area doc from the workspace for anything about tiles.
- egui submenus in kittest open on hover: get_by_label_contains(name).hover(), not click.
