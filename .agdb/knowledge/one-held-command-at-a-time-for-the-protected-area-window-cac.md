---
type: Gotcha
title: "One held command at a time for the Protected Area window; caches keyed by held values, not addresses"
description: "Why plugin runs and other protected changes are refused while a plugin run or import is held for the Protected Area window, and why window caches must hold what they key by (adversarial review fix U2)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T00:55:50Z }
touches: ["crates/mg-ui/src/plugins.rs:held_for_protected", "crates/mg-ui/src/claims.rs:gate", "crates/mg-ui/src/minimap.rs:of", "crates/mg-ui/src/outside.rs:forget_module", "crates/mg-ui/src/recovery.rs:forget_own_recovery"]
asks:
  - Why does a plugin command say it is not run while the Protected Area window waits?
  - Can two plugin runs be held for the Protected Area prompt at once?
  - Why does the minimap cache keep the area in its stamp?
  - Why does close_keeping not reset all of OutsideState?
fingerprint: v2-0fe19b8649e288f7
created: 2026-11-24
author: augustaugur
---
* The Protected Area window (claims::protected_window) is not modal, and next_plugin_event starts queued event jobs whenever no job runs. So with one plugin run held (plugins.pending_apply, its haks already written) a second run could finish and be held too, replacing pending_apply and protected_ask: the first run's haks stayed written past Cancel. Two plugins handling ModuleOpened back to back are enough. Now: run_plugin refuses while held_for_protected() (pending_apply or pending_import), next_plugin_event waits without popping the queue, and claims::gate refuses a new protected change while one is held (instead of replacing protected_ask). Quit also calls cancel_pending_protected.
* minimap::of serves Setup Area Transition's one slot for whichever Target area is shown: the area must be in the stamp check and the same-load check, or the old area's map is kept (B1).
* OutsideState::forget_module keeps repaint, walk and the tileset watch: resetting all of it made reload_changed_tilesets rebuild the tileset watch before reload_timer set repaint again, so outside changes stopped waking the window.
* Undo back to clean removes only the copy this window wrote (forget_own_recovery), not a crashed session's copy the builder put off with Later; closing or saving still removes both.
* Caches keyed by an Arc's address (search ticked_fits, dialog larger_text) must hold the Arc and use ptr_eq, or a new allocation at the same address takes the old answer.
