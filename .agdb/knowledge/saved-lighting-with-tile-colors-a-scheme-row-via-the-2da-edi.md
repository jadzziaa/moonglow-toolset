---
type: Decision
title: "Saved lighting with tile colors, a scheme row via the 2DA editor"
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: [crates/mg-module/src/lighting.rs, crates/mg-ui/src/area_props.rs, crates/mg-ui/src/browser.rs]
fingerprint: v2-8402652a64e13df1
created: 2026-10-30
author: August
description: "Saved lighting stores an environment.2da-style tile-light palette and picks tile colors like a scheme (pick_tile_fields); Make a Lighting Scheme opens environment.2da in the 2DA editor and adds a row via Editing::add_row_of instead of writing haks."
asks:
  - "How does Make a Lighting Scheme add its row to environment.2da without writing hak files itself?"
  - "Why do my saved lighting tile colors differ between Copy From Area and Give Every Area?"
  - "Where do tile light colors come from when I copy lighting from another area?"
  - "My kittest can't open an egui submenu by clicking; how do I trigger it, and why can't I find Tile_List in Area Properties?"
---
## Decided
- Saved lighting keeps a tile-light palette shaped like an environment.2da row (four lightcolor rows each for main1, main2, source, shared by use) and gives tiles picked from it as a scheme does (mg_module::lighting::pick_tile_fields, also Scheme::tile_fields). Copy From Area picks from the source area's tiles; Give Every Area keeps the scheme-row rule.
- Make a Lighting Scheme does not write haks itself: it opens environment.2da where the builder picks (browser::table_homes / open_table, the browser's Edit menu) and adds the row with Editing::add_row_of, to be saved there. No multi-hak merge needed: the row goes into the copy chosen.
## Watch out
- Area Properties' Form root has no Tile_List (stripped each frame for speed): read the area doc from the workspace for anything about tiles.
- egui submenus in kittest open on hover: get_by_label_contains(name).hover(), not click.
