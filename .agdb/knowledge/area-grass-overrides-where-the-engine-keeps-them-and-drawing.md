---
type: Decision
title: "Area grass overrides: where the engine keeps them, and drawing grass"
tags: []
status: stable
generated: { by: trail, at: 2026-10-21T00:00:00Z }
touches: [crates/mg-module/src/grass.rs, crates/mg-area/src/grass.rs, crates/mg-area/src/scene.rs, crates/mg-ui/src/area_grass.rs, crates/mg-ui/src/area_view.rs, crates/mg-testkit/src/engine.rs, crates/mg-corpus-tests/tests/engine_grass.rs, crates/mg-corpus-tests/tests/client_render.rs]
fingerprint: v2-8834f489ec356a99
created: 2026-10-21
author: August
description: "Area grass overrides are stored in the GIT's AreaProperties (GrassDefDisabled, GrassOverrides), not the ARE root; the note covers reading engine values via console save, how tufts are drawn, live preview, and pitfalls."
asks:
  - "Where does the engine read SetAreaGrassOverride and SetAreaDefaultGrassDisabled state from, the GIT or the ARE file?"
  - "My grass overrides in the ARE root are ignored when the module loads; why, and where should they go?"
  - "How can I read back an engine value that no script can return, such as an area's grass override?"
  - "How do I compare rendered grass against the client, and why does a top-down camera with pitch 1 look wrong?"
---
## Decided
- SetAreaGrassOverride/SetAreaDefaultGrassDisabled state lives in the GIT's `AreaProperties` (`GrassDefDisabled` BYTE, `GrassOverrides` struct 101 › `GrassOvrList` struct-0 items), and the engine reads it from a module's GIT at load; the same fields in the ARE root are dropped (`engine_grass`). The format layer is `mg_module::grass` (so `mg areas` can use it); `mg_area::grass` re-exports it and adds mg-edit edits and the tuft geometry.
- An engine value no script can read back: start the server with a console and type `save 2 NAME`, then read the area's GIT out of `saves/000002 - NAME/<module>.sav` (`mg_testkit::engine::save_game`). The engine writes overrides from a hash map: compare them by material, not order.
- Grass is drawn as tufts per walkmesh face, count = face area × density (capped at 2000 a tile), one upright quad of a random texture quarter, both windings, normals up. Near the client's camera its mean color matches within 3 for default, sparse, dense, tall and red (`grass_overrides_look`).
- Live preview: the Grass page puts what a drag/picker is trying into egui temp data with the pass number (`area_grass::tried`); the area view regrows from it, so undo gets one step at release.

## Tried and ruled out
- A top-down client picture: the client clamps SetCameraFacing's pitch, so pitch 1 looks like 60. Use the fitted (90, 12, 45) camera and compare regions.
- Shrinking the grass reach (45 to 30 m) for dense grass: the GPU cost is fragments near the camera, not tiles in reach.

## Watch out
- The server ignores a console command typed while the module is still being set up: retry `save` until it prints "Saving".
- `ui.ctx().cumulative_pass_nr()` inside a `ctx.data_mut` closure deadlocks (egui's RwLock): read it first.
- In UI tests a page button named "Grass" collides with the Tiles palette's Grass terrain (`get_all_by_label(..).last()`); a ComboBox's selected text is not its accessible label, a `menu_button` is.
- The client grows grass only within ~24 m of its camera and gathers tufts in patches; Moonglow draws whole tiles within 30 m of the focus, evenly (deferred).
