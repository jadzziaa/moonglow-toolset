---
type: Decision
title: "Game data changes that find it held wait in GameWaits, made once it is free"
description: "When exclusive(&mut app.game) fails, game-data changes go through Moonglow::game_free_for, which keeps them in GameWaits and make_waiting_game_changes applies them each frame once free; what step 1 of issues #49/#50 found."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T15:34:40Z }
touches: ["crates/mg-ui/src/lib.rs:game_free_for", "crates/mg-ui/src/lib.rs:make_waiting_game_changes", "crates/mg-ui/src/lib.rs:layer_open_module", "crates/mg-ui/src/lib.rs:GameWaits", crates/mg-ui/tests/app/game_data_holders.rs]
asks:
  - "How do I change the game data's layers when something may still hold it?"
  - Why does the log say the game data is still in use and when is the change made?
  - "What caused the 10 second Save stall and the previous module's haks staying loaded (issues 49 and 50)?"
  - Can RELEASE_WAIT be shortened now that game data changes are retried?
fingerprint: v2-66249a691bf894be
created: 2026-11-25
author: augustaugur
---
decided (2026-11-25, br54-issues): a new site that changes app.game calls `self.game_free_for(|w| w.<flag> = true)` instead of `release_game_data(); exclusive(..)`. It releases and returns true when the data is free; otherwise it marks GameWaits (layers, tlk, reload, changed), logs a warning once and returns false. While anything waits it does NOT call release_game_data again, so a stuck holder costs one RELEASE_WAIT, not one per site (New Module had spun 3 x 10 s for one holder). make_waiting_game_changes runs each frame after poll_job (minimap pattern: release only when !busy && !game_data_held, else frame_after 250 ms) and computes the change at apply time: layer_open_module (remove MODULE-priority and hak layers, flush, ws.module.stack_onto; no module: bake::layer_pending(None)), then reload_resources, reload_custom_tlk, game_data_changed (forgets drawn textures, pictures, area views reload). Status Bar: looking_note says 'Waiting for the game data to be free…'. Pending is kept OUT of under_way(): a permanent holder would make every settle() panic.
ruled out: queueing release_haks (its project command needs the haks closed before it runs; it warns instead), and replaying the old Module (A is gone by the time the data frees; B is in ws).
found (step 1): on 5.3.0 no GPU-free surface leaves a holder (game_data_holders::only_the_window_holds_the_game_data_once_it_lets_go). Putting back 4.2.1's `kept_for(.., app.game, ..)` in the Placeable Wizard's Category step reproduces #49 exactly (Save 10 s, 'still shared by 1 other'): the likely 4.2.1 cause, fixed in 5.2.0 by kept_for_key. Viewport-side holders (texture readers, picture readers, area first read) are covered only by the GPU tests in game_data_holders::gpu.
RELEASE_WAIT: could be shortened for Save/close/open, but start_job_on's refresh_module_layer (the job clones the data right after), build-on-save's run(), release_haks and the readers right after use_module (load_custom_tlk, plugins_module_opened) want the change done before they go on.
watch out: tests that need a holder use a job (start_job_for_tests on a channel) and call close()/open_module() directly: run() refuses Close/Open while a job runs.
