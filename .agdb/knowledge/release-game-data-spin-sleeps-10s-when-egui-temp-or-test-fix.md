---
type: Gotcha
title: "release_game_data waits up to 10 s for other holders: never per frame, and not with a cached Arc<GameData>"
description: "Holding extra Arc<GameData> clones, or calling release_game_data every frame, makes it spin up to 10 seconds; check crate::exclusive instead where a frame only asks."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T08:44:15Z }
touches: ["crates/mg-ui/src/lib.rs:release_game_data", "crates/mg-ui/src/lib.rs:exclusive", "crates/mg-ui/src/blueprint/mod.rs:kept_for", "crates/mg-ui/src/blueprint/mod.rs:kept_for_key", crates/mg-ui/src/tileset_view.rs, "crates/mg-ui/tests/app/talk_panels.rs:talk_window_with"]
asks:
  - Why does a test or load-order change stall for 10 seconds in release_game_data?
  - "When should blueprint::kept_for_key be used instead of blueprint::kept_for?"
  - Why did the minimap folder stay mounted after Render Minimap Pictures?
  - Can a per-frame job wait for the game data to be free?
fingerprint: v2-5bb5b58b362574e8
created: 2026-11-23
author: augustaugur
---
`Moonglow::release_game_data` spin-sleeps up to `RELEASE_WAIT` (10 seconds) waiting for `Arc::strong_count(game) == 1` before exclusive game-data mutation.

- Never pass `&Arc<GameData>` to `blueprint::kept_for`, because it clones the `Arc` into egui temp memory; use `blueprint::kept_for_key` keyed on `app.load_order` and workspace `(identity, revision)` instead.
- Tests that trigger `release_game_data` (such as `talk_panels::talk_window_with`) must own a fresh `Arc<GameData>` (`strong_count == 1`), not a shared static `Arc`.
- Never call it per frame: a job that waits for the game data across frames checks `crate::exclusive(&mut app.game)` each frame and acts once it returns `Some`. The tileset editor's minimap job does this to unmount its folder layer (`unmounting`): before 2026-11-25 the mount leaked when something still held the game data as the job ended.
- The retry where only the 3D views and haks hold the data (`!busy() && !game_data_held()`) calls `release_game_data`, which lets those go and then does not wait. Tests needing a holder without the 10 s wait use a job (`start_job_for_tests` blocked on a channel), not a bare `Arc` clone (which makes it spin the full 10 s).
- The doctor `GOLDEN` hash covers message text: rewording a finding moves it.
