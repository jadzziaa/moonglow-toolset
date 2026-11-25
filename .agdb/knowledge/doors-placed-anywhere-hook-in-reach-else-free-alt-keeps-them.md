---
type: Decision
title: "Doors placed anywhere: hook in reach, else free; Alt keeps them off hooks (5.3)"
description: "How the area view decides whether a door goes on a door hook or freely (door_at, DoorAt), why Alt is the modifier, the Alt-alone menu trap it hit, and the rule tile changes use to tell hook doors from free ones."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T00:18:02Z }
touches: ["crates/mg-ui/src/area_view.rs:door_at", "crates/mg-ui/src/area_view.rs:door_drag_hook", "crates/mg-ui/src/commands.rs:menu_bar", "crates/mg-edit/src/terrain.rs:door_edits", crates/mg-ui/tests/app/free_doors.rs]
asks:
  - "Why does a door snap to a hook, and how do I place one off it?"
  - Why Alt and not a grid no-snap modifier for free doors?
  - Why did letting go of Alt after a drag open the File menu?
  - "Which doors does a tile change, resize or Change Tileset delete?"
fingerprint: v2-ef2d745e19275e76
created: 2026-11-25
author: augustaugur
---
## Decided
- One decision function, `area_view::door_at(view, pos, off)`, gives `DoorAt::Hook(hook)` (a hook within DOOR_REACH, 4 m, of `ground_at(pos, 0.0)`, the pointer's tile's first) or `DoorAt::Free { at: placed_at(pos), held_off }`. The ghost (`ghost()`, which stores it in `view.ghost_door`), the brush click, the palette drop, the lit hook line and the Status Bar all ask it, so they cannot disagree. A free door is placed with `view.ghost_turn` (Q/E); the Q/E gate for the ghost only skips a door on a hook.
- A placed door dragged alone (`Drag::Move`, not axis-locked, one selected) snaps onto a hook within reach in `AreaView::dragged()` (`door_drag_hook`), so preview and commit agree; Q/E turning under way is overridden by the hook's bearing there. Before 5.3 placed doors dragged freely (the brief thought they snapped: they did not; only the palette ghost did).
- The modifier is Alt (`hooks_off`, read each frame into `view.hooks_off`). No modifier suspended grid snapping anywhere in the area view (checked: `snapped`, `put_down_at`, the Move snap in `dragged` are unconditional), so none could be reused. Ctrl is out: a press with Ctrl is the camera's and Ctrl held during a primary drag pans (`camera_input`). Shift is "place more" / add to selection. Alt is free at the brush click and the drop; for a drag it must be pressed after the drag starts, since Alt at the press makes `Drag::Lift`. Linux window managers that take Alt + click: press Alt after the button.
- Tile changes keep free doors by the existing rule, unchanged: `door_edits` (painting, plugins), `reshape::resize` (cut groups) and `retile` take away only doors whose X/Y is within 0.05 m of a TYPED hook (door type not 0) of a changed/cut/old tile. Tests: `a_tile_changed_takes_its_hooks_doors_and_leaves_free_ones` (mg-edit), `a_cut_group_takes_its_hooks_doors_and_leaves_free_ones` (mg-area). Verify (mg-module doctor) has no door-on-hook check, so a free door is never flagged.
- Aurora places doors on hooks only: the divergence is recorded in docs/parity/checklists.md's door row.

## Watch out
- `commands::menu_bar` opens the File menu on "Alt alone" (down then up with nothing between). Alt pressed during a drag and let go after it counted as Alt alone, opening the menu, which then ate letter keys (Q did nothing). Fixed: a pointer button down or released while Alt is held spoils Alt alone (`any_down() || any_released()`).
- In kittest, Alt sent with `ModifiersChanged` on the second hover step of a drag landed before `drag_started` fired, so the press became a Lift; the test waits for `door_drag_hooked()` to be Some before sending Alt.
- `common` already has a `says(h, area, ..)`; free_doors.rs uses its own `bar_says`.
