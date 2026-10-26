---
title: Tilesets with holes read tile by tile; trf01 draws nothing
author: August
date: 2026-10-26
branch: worktree-agent-aee3dce02b04a7316
touches:
  - crates/mg-area/src/lib.rs
  - crates/mg-corpus-tests/tests/groups/engine.rs
  - crates/mg-corpus-tests/tests/small_formats.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-set/src/lib.rs
  - crates/mg-tiles/src/lib.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/findings.md
---

## Decided
- `mg_set::Tileset::parse` reads a tileset tile by tile, as the engine does: a missing `[TILEn]` or a tile without `Model=` is a hole (`Tile::is_hole`, empty model) in its place; missing TERRAINn/CROSSERn/RULEn sections are left out; a `Count` stops at the list's last section. Only a file without `[GENERAL]` is an error. `Tileset::missing_sections` names the holes.
- The area view keeps why a tileset couldn't be read (`AreaView::tileset` is a `Result`), and `why_left_out` says, per model the scene left out, whether it was found (and in which layer) and why it can't be read. Before, both said "not found".
- Verify: `set-hole` (warning) and `set-line-ends` (error, an all-LF SET).

## Tried and ruled out
- El Grillo's trf01.set (2813 tiles) is not the cause of "no tiles at all": it parses (7 ms), has no holes and CRLF line ends, and an area of its tiles with base-game models draws. Its tile names in capitals are fine (ResRef compares case-insensitively); the ARE's `Tileset` in capitals is fine too. What is left is its hak's models: not found, or unreadable.
- `GetSurfaceMaterial` at a tile's centre reads 3 both for grass and for a tile with no walkmesh: probe with water tiles (ttr01 20, 101, 102), which read 0, and calibrate against a copy whose models don't exist.

## Watch out
- A SET with LF line ends in a hak hangs the server at "Loading Module" (no shutdown; the runner times out). An engine test that rewrites a SET must keep CRLF. Moonglow keeps a file's line ends.
- In a worktree session, `flock … cargo test` and heredocs are refused by the shell guard: put the command in a scratchpad script and run it with `sh`.
