---
title: "The seventh client batch: the client's code, the clock shim, buckets, front faces, NWSync reload"
author: August
date: 2026-11-02
branch: worktree-agent-a2254f9b3951dcc98
cost: { minutes: 2964, tokens: 20237256 }
touches:
  - crates/mg-corpus-tests/tests/client_render/batch7.rs
  - crates/mg-preview/src/usage.rs
  - crates/mg-testkit/src/joined.rs
  - tools/nwclient/timeshift.c
  - tools/nwclient/run-server-and-client.sh
  - docs/deferred.md
---

## Decided
- The seventh client batch's readings are in `crates/mg-corpus-tests/tests/client_render/batch7.rs`; what each saw is in its `Seen` paragraph, `docs/findings.md` and the deferred entries. Fixes inside mg-render were written into the entries as sized rules (another package owned it).
- Reading the client's code is the owner's last resort (2026-11-02), only after every other way (pictures, patched shader includes, scripts, shims, runs designed to tell causes apart) has been tried and failed. When it is needed: the client binary (`nwmain-linux`) is not stripped, and `nm -C` and `objdump -d -C` on it settle rules a screen can't (the world timer's start: `CWorldTimer`; the creation screen's body parts: `CCharacterAppearancePanel::PostAttachmentInitialize`; why a joined client leaves: `CServerExoAppInternal::PlayerListChange`, `CAutoConnectPanel::Bail`). Disassemble once into `target/` (170 MB) and grep it.
- `tools/nwclient/timeshift.c` (LD_PRELOAD; `MG_SHIFT_REAL`, `MG_SHIFT_MONO` seconds) moves the client's clocks; `run-client.sh` hands a library to the client alone with `MG_CLIENT_PRELOAD`. The world timer follows `CLOCK_MONOTONIC` in two-hour steps; moving the boot clock moves it.
- Joined servers keep their module loaded (`reload-when-empty = false` in `mg_testkit::joined`): reloaded, a module is advertised without its NWSync manifest, and a client back from its download leaves.
- In static geometry (tiles, static placeables) the first mesh of a texture gives the rest its `NO_DISCARD` and opacity. The client casts the sun's shadow from a face's front only.

## Watch out
- The client rewrites its `settings.tml` as it ends, dropping debug flags (`game.log.debug.network`): put it back before each start (`MG_CLIENT_SETTINGS` in `run-server-and-client.sh`, `client-settings.tml` in the joined scratch).
- A client started with `+connect` exits 0 whenever it returns to the main menu.
- A GameData opened before override files are written doesn't see them: write the client's debug `inc_standard` after opening the game Moonglow draws with (`tdod_fountain`), or Moonglow draws the debug band too (at the top: wgpu's `gl_FragCoord.y` counts from the top).
- `r.place` with facing 0 leaves a model's axes as they are (its -X faces west).
- The worktree guard refuses heredocs and `python3 - <<EOF`: write edit scripts to `target/` and run them.

## Tried and ruled out
- Sight: a tile out of the camera's view still casts its shadow in the client.
- The fountain's falling-water phase: matched to the client's clock, the difference is unchanged (10.57 of 255); the basin's water differs.
