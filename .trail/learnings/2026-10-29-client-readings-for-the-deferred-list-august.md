---
title: "Client readings for the deferred list (Wave 1, client batch)"
author: August
date: 2026-10-29
branch: worktree-agent-ae062067e430eeeec
cost: { minutes: 330, tokens: 13501182 }
touches:
  - crates/mg-area/src/scene.rs
  - docs/deferred.md
  - docs/findings.md
  - docs/research/notes_shaders.md
  - crates/mg-corpus-tests/tests/client_render.rs
  - crates/mg-corpus-tests/tests/client_render/readings.rs
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/lib.rs
  - docs/log.md
  - docs/manual/05-blueprints.md
---

## Decided
- Small client probes live in `crates/mg-corpus-tests/tests/client_render/readings.rs`: `Reading::new(name, tileset, light, overrides)` builds a 4x4 module (files put in the scratch `override` first, the game opened with them), `place()` adds a placeable by appearance row, `shoot()` runs the client, `shoot_beside()` also renders Moonglow's area view from the fitted camera (`moonglow.png`, `moonglow_on.png`; `MG_MAGENTA=1` paints where nothing is drawn). `client_after()` runs the client without waiting for `MG_READY` (+LoadNewModule).
- The client's GIT `SunDirectionX/Y/Z` is uploaded as `lightAreaDiffuseDirection` exactly (normalized, towards the light): read back by the debug `inc_standard` (blocks 96-98) and turned to world space with the fitted camera's view. The area view now takes it (`Lighting::read_with`).
- `Tile_ReplaceTex` v draws `replacetexture.2da` row v-1; absent = row 0; 0 or past the table crashes the client (`CNWCTile::SetReplaceTexture`).
- The game's inventory shows a woman's armor with `ipf_` icons and each part in its own colors; a custom cloak base item lies as its DefaultModel (gi_cloak01 is base 80's alone).
## Tried and ruled out
- A fixed origin for `worldtimerTimeOfDay`: 16 runs in 15 minutes agreed (17,697,400 at entry, taken mod the module's day) and the 2026-10-21 values matched, but an hour later the count was two hours more. Committed as a rule in mg-render and reverted the same day. The mod-day part holds.
- A placeable walkmesh as ground: a static slab's walkable grass-material pwk grew no grass, and the server stood a player inside it on the tile.
## Watch out
- The off-screen display here is `:2` (`headless.sh start` prints it); `:1` is the desktop kwin's Xwayland init display. run-client.sh only refuses `:0`.
- `SetCameraFacing` at once on entry is ignored but for the reference (90, 12, 45): give others a second later (`enter()` does).
- The `+TestNewModule` player is the first pregenerated character, a woman (Aluvian Darkstar): use `SetGender` to compare.
- Items made with `CopyItemAndModify` in the PC's inventory leave copies until the script ends (DestroyObject is deferred): make them in a chest and `ActionTakeItem` them, or they land on later pages.
- `bwrap --unshare-net` has a working loopback: a server and the client in one sandbox would work.
- `common::client_screenshot` now returns at once when the client exits (a crash on the module) instead of waiting 180 s.
- The worktree's Bash guard refuses heredocs, `python -c` with computed text and names containing "git" (a test filter like `reading_git_sun`): use scratchpad scripts and other filters.
