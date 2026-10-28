---
title: "Blueprint editors: edit together, marks, view, Select Texture"
author: August
date: 2026-10-28
branch: worktree-agent-a79690dba2bcc8985
cost: { minutes: 996, tokens: 9364216 }
touches:
  - docs/deferred.md
  - docs/log.md
  - docs/manual/03-modules.md
  - docs/manual/04-areas.md
  - docs/manual/05-blueprints.md
  - crates/mg-ui/src/area_props.rs
  - crates/mg-ui/src/blueprint/creature.rs
  - crates/mg-ui/src/blueprint/door.rs
  - crates/mg-ui/src/blueprint/item.rs
  - crates/mg-ui/src/blueprint/mod.rs
---

## Decided
- Several objects edited together (blueprints or placed objects) leave out `LIST_PAGES`; the guard drops a command that reaches some targets but not all, by (document, path prefix) (`blueprint::partial`): placed objects share one GIT key, so keys alone can't tell them apart.
- The "≠" intent: every field the form sets on each target gets the mark. `Form::mark_mixed` paints it in the gutter left of a grid cell and gives it an accesskit Label node and a tip; `mark_mixed_inline` lays it out for rows of radios that start at the page's edge (a painted mark there is clipped); `mark_mixed_bit` for one bit of `Flags`; `mixed_line` under a Visuals section whose list is set whole.
- A viewed game blueprint: after the form draws, a focused widget inside its scroll area is made to surrender focus, unless it is a search (`widgets::is_search_field`: `SearchField` marks itself, plain Find fields call `widgets::searching`). Disabling the UI was ruled out: ScrollArea's wheel checks `ui.is_enabled()`, so lists would stop scrolling.
- Select Texture leaves out the interface's pictures by name (`INTERFACE_PREFIXES`, `mi??NN_` map tiles, `i<ItemClass>_` icons from baseitems.2da): about 24,000 of 29,567, a tick shows them. The game's list is kept in `ui.data` by the game data's pointer (14 ms to 1 ms on opening again).
- Drags that move by the camera's distance use `Camera::per_point(height)` (what is dragged follows the pointer).

## Watch out
- Escape is read for dialogs once a frame before anything is drawn (`dialogs_begin`); a widget that takes Escape for itself (the Variables window's offer) must call `widgets::keep_escape`, or the window's Cancel closes it. Consume Up/Down/Enter before the TextEdit is added, or the field (and the window's Enter = OK) sees them.
- In kittest, a Label's text is its node's `value`, not `label`: count marks with `query_all_by_value("≠")`. `get_by_label` still finds labels by text.
- A radio row edited together does nothing when the option clicked is the first object's value (`set != want` is the first's): a test wanting the others changed clicks the other option.
- plc_chest1 already has Appearance 7 and Interruptable 0: a test making a second chest differ must pick other values.
- The chest's model has emitters (fxpa_cloud02, fxpa_smoke01): the Visuals page lists them now and is taller; tests reaching MiscVisuals scroll to it first.
