---
type: Decision
title: "Test Module's Steam start follows Read Steam Workshop content (5.1.0 T1)"
description: "Why a test of a Steam copy starts through Steam only where Moonglow reads the Workshop, how test_without_steam was retired, and what the game binaries show about bare starts."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T01:04:43Z }
touches: ["crates/mg-ui/src/test_module.rs:command", "crates/mg-ui/src/test_module.rs:steam_line", "crates/mg-ui/src/settings.rs:test_through_steam", crates/mg-ui/src/options.rs]
asks:
  - why does the game look different when tested from Moonglow than from Aurora
  - when does Test Module set SteamAppId
  - where did the option Start a Steam copy of the game through Steam for a test go
  - does nwmain start without Steam or relaunch through Steam
fingerprint: v2-d7450368ca2a6775
created: 2026-11-23
author: augustaugur
---
A builder (2026-11-22) saw other textures (zcn01, tin01, placeables), glossy materials, darker light and less fog in a test started from Moonglow, while both toolsets' views agreed. Cause (likely): 5.0.0 set SteamAppId/SteamGameId=704450 for a Steam copy by default (Options > General "Start a Steam copy of the game through Steam for a test", field `test_without_steam`), so the game loaded the builder's Workshop subscriptions; Aurora starts it bare, and Moonglow's view reads the Workshop only with Read Steam Workshop content (off by default).

Owner's ruling ("steam test: a"): one switch. `Settings::test_through_steam()` is `read_workshop`. The General row is gone; `test_without_steam` retired the codebase's way (field removed; serde ignores unknown fields, so old files read with nothing unread; the toolbar_icons_only old-file test filters it from the kept fields too). A builder with Workshop on keeps Steam starts even if they had unticked the old option (the one case lost).

A bare start now also `env_remove`s both variables (Moonglow's own environment may carry them). `steam_copy` matches `steamapps` case-insensitively (older Windows libraries: SteamApps). `steam_line` gives the log line, only for a Steam copy.

Why bare starts work: the Linux, Windows and macOS binaries load the Steam library dynamically (`steamapi_Init` pointer, `steam_api64` by name) and have no `RestartAppIfNecessary` string (looked at with `strings` only; no nm/objdump), the same binary carries GOG Galaxy DLLs, and run-client.sh runs the Steam copy bare with Steam hidden. Workshop content in the client comes through Steam's UGC API (SteamWorkshopDiscoveryRepository), hence none without Steam. Not known without a run: whether a bare start on Windows/macOS still reaches a running Steam through the root's steam_appid.txt (the client starts in bin/<platform>).
