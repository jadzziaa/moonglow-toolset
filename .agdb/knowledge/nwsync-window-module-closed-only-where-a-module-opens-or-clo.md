---
type: Decision
title: "NWSync window: module_closed only where a module opens or closes; remount through game_free_for; a thread left runs to its end"
description: "Why layer_open_module must not reset the NWSync Content window, how a Sync's remount waits for held game data, and what happens to a look/Sync/prune when its module closes."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T07:29:58Z }
touches: ["crates/mg-ui/src/lib.rs:layer_open_module", "crates/mg-ui/src/nwsync_content_view.rs:remount", "crates/mg-ui/src/nwsync_content_view.rs:module_closed", "crates/mg-module/src/bake.rs:PENDING_PRIORITY"]
asks:
  - why was my NWSync Sync canceled after an edit
  - why does the game data keep the old NWSync manifest after a Sync
  - what happens to a running NWSync look or clean-up when the module closes
  - why must the bake pending rows layer be above the NWSync layer
fingerprint: v2-94d422017745baa0
created: 2026-11-27
author: augustaugur
---
Review of PR #53, 2026-11-27. layer_open_module runs from make_waiting_game_changes whenever GameWaits.layers was set (refresh_module_layer after edits and before reads_game jobs, update_module_layer, sync_haks, bake rows, Save), not only on module open: it must replace only nwsync_content.mounted (None, then Some from Stacked.nwsync). NwSyncContent::module_closed (cancels a Sync, closes the window, clears survey/synced/error) is called in use_module BEFORE game_free_for (so the held-at-open path gets it too) and in close. remount() after a Sync calls game_free_for(|w| w.layers = true): when a job holds the game data, layer_open_module later mounts the newer manifest (it used to call release_game_data then exclusive and silently return). A thread under way at module_closed is kept as NwSyncContent.left until its done is set: running() is true meanwhile (no new Sync, Moonglow::under_way waits), its result is discarded by poll, and poll looks once it ends if the window is open with no survey. Look and prune can't be stopped (no stop in mg_nwsync::survey/prune; deferred.md). Mount of 220,000 entries is about 32 ms in release (nwsync_mount_is_timed), so it stays on the UI thread. bake::PENDING_PRIORITY = HAK_OVER_NWSYNC + 1 (42): table_under builds the pending table from the first placeables.2da under it, which may be NWSync's (40) or a newer hak's (41); at 32 the rows were hidden.
