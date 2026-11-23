---
type: Gotcha
title: release_game_data spin-sleeps 10s when egui temp or test fixtures retain Arc<GameData>
description: Holding extra Arc<GameData> clones in egui temp cache or test fixtures causes release_game_data to spin-sleep 10 seconds.
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T23:34:33Z }
touches: ["crates/mg-ui/src/lib.rs:release_game_data", "crates/mg-ui/src/blueprint/mod.rs:kept_for", "crates/mg-ui/src/blueprint/mod.rs:kept_for_key", crates/mg-ui/src/blueprint_wizard.rs, "crates/mg-ui/tests/app/talk_panels.rs:talk_window_with"]
asks:
  - Why does a test or load-order change stall for 10 seconds in release_game_data?
  - "When should blueprint::kept_for_key be used instead of blueprint::kept_for?"
  - Why did talk_panels tests hang for 10 seconds after using shared game data?
fingerprint: v2-eca1096cb1bacd0b
created: 2026-11-23
author: augustaugur
---
`Moonglow::release_game_data` spin-sleeps up to `RELEASE_WAIT` (10 seconds) waiting for `Arc::strong_count(game) == 1` before exclusive game-data mutation. Never pass `&Arc<GameData>` to `blueprint::kept_for`, because it clones the `Arc` into egui temp memory; use `blueprint::kept_for_key` keyed on `app.load_order` and workspace `(identity, revision)` instead. Similarly, tests that trigger `release_game_data` (such as `talk_panels::talk_window_with`) must own a fresh `Arc<GameData>` (`strong_count == 1`), not a shared static `Arc`.
