---
title: "The fifth client batch: programs patched in the override, joined players, NWSync"
author: August
date: 2026-10-30
branch: worktree-agent-ad9b9d7bc6b94d49e
touches:
  - crates/mg-corpus-tests/tests/client_render/batch5.rs
  - crates/mg-render/src/shader.wgsl
  - crates/mg-render/src/model.rs
  - crates/mg-render/src/shadow.rs
  - crates/mg-preview/src/compose.rs
---

## Decided
- A game program (`fswater`, `fst`, `vsgrass`/`fsgrass`, a custom pair) patched in a `Reading`'s scratch `override` is read by the client AND by Moonglow's game path (the readings open `GameData` with that user directory): paint one part of what a program computes (a term, a uniform, a varying, the clock) and compare both pictures directly. Moonglow's own shader (`moonglow.png`) doesn't read it: a control.
- The readings' `moonglow_beside` draws without fog (`View { fog: false }`); the client fogs the far ground and water even at a fog amount of 0 (start 30 m before the clip distance). Compare far things with `v.fog = true`.
- A player joined to a server (`mg_testkit::joined`, `MG_CLIENT_CDKEY`): the character list's Play is at (615, 615) of the 1024×768 window, the NWSync Downloader's Download at (435, 596); click by move, move, press, release, then move the pointer off the game at once (a click left over lands in the area as an attack). Scratchpad scripts did it (`play.sh`, `click.sh`, `watch.sh`).

## Tried and ruled out
- `AssignCommand(pc, ActionMoveToLocation/JumpToLocation(...))` from the entry script moved no player in tic01's reference room; an encounter about the start spawns when the player enters the area.
- After an NWSync download the client ends itself; started again it joins and leaves within the second (server log: Joined, Left), its own logs silent.

## Watch out
- A `DelayCommand`'s arguments are evaluated when it is called: `DelayCommand(8.0, Log(... GetPosition(pc) ...))` logs the position at entry. Wrap in a function.
- `nwmain compilemodel` (the client run with it) compiles no skin mesh; nwnmdlcomp keeps no model `renderhint`.
- In the client a model's mesh `alpha 0.5` writes depth and a model's see-through meshes draw in node order; a ribbon behind one is hidden.
- `hint_wants_tangents` must be case-insensitive: tests set `Mesh::renderhint` by hand in mixed case.
