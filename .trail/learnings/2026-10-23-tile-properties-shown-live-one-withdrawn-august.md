---
title: "Tile Properties shown live: one withdrawn-and-rewritten step"
author: August
date: 2026-10-23
branch: tile-props-live
touches:
  - crates/mg-area/src/edit.rs
  - crates/mg-area/src/lib.rs
  - crates/mg-edit/src/lib.rs
  - crates/mg-ui/src/terrain_mode.rs
  - crates/mg-ui/src/tile_select.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/04-areas.md
---

## Decided
- Tile Properties writes as it changes (`tile_select::write_draft`, called at the end of `ui` every frame; nothing when `Draft` equals `written`). The window's step is ONE command "Tile Properties": while `ws.revision() == props.live` it is the last command, and each change takes it back with `Workspace::withdraw` (mg-edit, new: undo that pushes nothing to redo) and writes the whole draft again from the module as it is. That is the amend-equivalent: the step stays minimal (no inverse growing per drag frame) and Cancel is one withdraw, byte for byte.
- Only the controls set are written: `touched` (picker click, checkbox change, Defaults) or a value differing from `opened`. 2.4.0's OK wrote every shown value (the first tile's) into all selected tiles.
- Picks (variant, All tiles, Turn) are applied synchronously (`terrain_mode::tile_command_made` returns the Command; `app.apply` at once) in `put_in`: withdraw the window's step first, apply the pick, re-open the window from the ARE (`open_properties` now reads the workspace, not `view.model`), `carry_draft` the set values, and `write_draft` puts the window's step back ON TOP. So history is [.., pick, Tile Properties]; Cancel takes off the lights/height and leaves the pick. The old deferred `chosen` re-read is gone.
- Cancel when the window's step is not on top (another command after it, or the user's Ctrl+Z): `take_back` writes "Cancel Tile Properties", setting each window-set field back to `base` (tile structs at opening) only where the tile still holds the window's value; heights with their object moves.
- Turn in All tiles mode: `Pick::Turn` = set_exact each cell with its own tile id, the new orientation, its height; `kept` = its struct so lights stay; label "Turn tile (ignoring fit)".
- `mg_area::edit::tile_height_edits` reads the objects from `files.git_root` itself (`AreaObject::placed`, no game or previews): the area view's model lags a frame (or never refreshes when hidden), and absolute positions from a stale model moved objects wrong on the second step.

## Watch out
- `mg_testkit::scratch_dir` empties the folder: two tests sharing one name lose each other's pictures.
- A Tile Properties window floating over the area hides it in pictures: dock `set_position` is clamped to the dock rect; move the orbit target east instead (`shots_beside` in app.rs).
- A grass tile turned looks the same: turn a cliff corner (ttr01 tile 0) to see it.
- The worktree guard refuses `flock ... cargo test` written inline; a script in target/ that runs it is accepted.
