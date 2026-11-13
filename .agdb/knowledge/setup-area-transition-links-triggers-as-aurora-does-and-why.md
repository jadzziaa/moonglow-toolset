---
type: Decision
title: "Setup Area Transition links triggers as Aurora does, and why the game hid Moonglow's transitions"
tags: []
status: stable
generated: { by: trail, at: 2026-11-12T00:00:00Z }
touches: [crates/mg-ui/src/area_view.rs, crates/mg-ui/src/jobs.rs, crates/mg-ui/src/open_tabs.rs, crates/mg-ui/tests/app/area_view.rs, docs/log.md, docs/manual/04-areas.md, crates/mg-ui/src/terrain_mode.rs, crates/mg-ui/tests/app/main.rs, crates/mg-ui/tests/app/tile_edges.rs, crates/mg-ui/tests/app/tiles.rs]
fingerprint: v2-2dbf0e1b7124f1ef
created: 2026-11-12
author: August
trail_id: l-713ec182
description: "Moonglow's Setup Area Transition window matches Aurora by linking triggers to a new WP_<tag> waypoint, and the game client only shows or clicks transition triggers whose points run counterclockwise and sit on the ground."
asks:
  - "How does Moonglow's Setup Area Transition link a trigger or door, and does it match what Aurora writes?"
  - "Why is my transition trigger invisible or not clickable in the game client even though it is linked correctly?"
  - "What does the transition-hidden Verify check flag, and why doesn't it use a height tolerance or check every corner?"
  - "Why does a transition fire on click and not when walking across it, and how do I test this with the client?"
---
## Decided
- Aurora's Setup Area Transition (measured 2026-11-12, captures `~/.local/share/moonglow-oracle/captures/transition-triggers/` 00 to 48, notes in `docs/findings.md` "Setup Area Transition with triggers"): whatever links to a trigger links to a new waypoint `WP_<trigger tag>` (LinkedToFlags 2) at the center of the bounding box of the trigger's points, Z the middle of its PointZ range, facing north; whatever links to a door links to the door's tag (1). Moonglow's window does the same as ONE Command (`mg_edit::transition`, window in `crates/mg-ui/src/blueprint/situated.rs`), compared field by field in `crates/mg-corpus-tests/tests/aurora_transitions.rs`.
- A destination is the object (area, kind, index), never the tag: two triggers of one tag are two rows, and the current object is left out by identity.
- Deliberate differences from Aurora (manual, Differences): a `WP_<tag>` already inside the trigger's box is used again; Target To Current writes the target; everything on the window's OK; Aurora's four questions are lines.
- The game client draws a transition trigger under the pointer, and takes a click on it, only when its Geometry points run counterclockwise and lie on the ground (ZPosition + PointZ). Held by the hand-run tests in `crates/mg-corpus-tests/tests/client_render/transition_outlines.rs`. Cursor and HighlightHeight do not matter. Tab shows no transition.
- Triggers are saved in Aurora's order (`mg_module::instances::outline_order`: counterclockwise, from the smallest X then Y, position at the first click) and put back on the ground on a move, paste or redraw (`mg_area::edit` `changes`; `outline_point_z`). Redraw Polygon sets ZPosition 0, as Aurora.
- Verify `transition-hidden`: clockwise is a fact; the height half flags only an outline wholly more than 1 m off the walkable ground and says "may".

## Tried and ruled out
- The reporter's sequence as written (link by hand, redraw on the same corners): the two saved modules are byte-identical. A redraw only changes the point order (if drawn the other way) and the heights.
- A height tolerance for Verify near the ground (0.05 m): it flagged four transitions in the game's own modules, three of which the client shows. How far off the ground the client still shows a transition depends on the place (0.1 m hides on flat ground, 0.6 m above still shows on one Daggerford slope); what decides it was not found. The drawn tile surface is within 0.04 m of the walkmesh there, so that is not it.
- Checking every corner against the ground: flagged 17 of the game's own transitions. A lone corner off the ground leaves the outline shown in part and clickable.
- Changing the Trigger wizard's Cursor 0 and HighlightHeight 3.0: Aurora's own wizard capture writes the same.
- Re-grounding triggers when the terrain changes under them: Aurora leaves them, and it was left out (the owner's call).

## Watch out
- One driver on the off-screen display at a time, and client tests with `--test-threads=1`: two clients on one display spoil each other's pictures.
- A control picture first: an Aurora-painted trigger moved to other ground is itself hidden until its points are on that ground.
- A stock transition fires on a click, not on walking across it; "it still works when crossed" means the module runs a script.
- Aurora on the off-screen display would not place a palette door (doors were written by `tools/aurora/capture_transition_triggers.py` and then linked in Aurora); a double click closes a trigger's outline; Ctrl+O on the open module crashed Aurora.
- Aurora's Target To Current writes the target's half only when the target's area is saved later: compare against the later capture.
- Moonglow writes no `.gic` entries for placed objects (Aurora keeps one per object): a lead for October's "List index out of bounds" in Aurora's dialog, not followed.
