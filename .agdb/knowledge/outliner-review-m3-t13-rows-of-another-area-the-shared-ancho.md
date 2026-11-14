---
type: Decision
title: "Outliner review (M3 T13): rows of another area, the shared Anchor, the tree stops following, the kit indent"
description: "What T13's review fixed and decided: per-area row state in the Outliner, tree::Anchor for Shift+click, the owner's ruling that the tree stops following while the Outliner shows, and an old kit TreeGroup indent bug."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T04:27:03Z }
touches: [crates/mg-ui/src/outliner.rs, "crates/mg-ui/src/tree.rs:follow_selection", "crates/mg-ui/src/tree.rs:Anchor", "crates/mg-uikit/src/tree.rs:TreeGroup", crates/mg-ui/tests/app/tree.rs]
asks:
  - "Delete in the Outliner deleted an object in another area: why can't that happen now?"
  - Why does the module tree no longer scroll to the object selected in the area view?
  - "Object rows jump 14 points left when their group's row scrolls out of sight: what was it?"
  - "A UI test clicks a tree row after the mouse wheel and hits another row: what to wait for?"
fingerprint: v2-8095ef532a0d8d8c
created: 2026-11-14
---
## Decided
- The Outliner keeps no row state across areas: `Outliner::listed_area` drops the keys' row, the anchor, a kind to open and the type-ahead when another area is listed; Delete acts only on a row it lists (hovered, else the keys' row if listed). A `TreeAt` carries its area: never strip it with `_` and pair the index with the area in front.
- Shift+click anchors are `tree::Anchor` (area, object, per-kind counts then), shared by the tree and the Outliner: valid only in its area and while no object of the area came or went (indices shift on delete, paste, Undo, outside change). The Outliner also drops it when the selection changes elsewhere.
- Owner's ruling 2026-11-14: "while the Outliner shows, the tree stops following the selection". `follow_selection` still marks `tree_object` and moves a `TreeAt::Object` cursor, but sets neither `tree_object_pending` nor `tree_reveal` when `Panel::OUTLINER.available && !hide_tree`. Explicit asks (Show in Module Tree, tab chosen, clicks in the tree) still reveal. Tests that need following use `common::outliner_away` (Module Properties docked in front).
- Going to or selecting an object of a hidden kind (tree or Outliner) shows the kind: `tree::show_kinds_of` in `act_on_object` and the tree's Ctrl/Shift path.

## Watch out
- Kit `TreeGroup`: with its row out of sight the row allocates zero width, and egui's `show_body_indented` expands the body to the header's right, pulling the body left over its indent. The body response is made at least `indent` wide. Pre-existing since T3.
- The mouse wheel's scroll goes on for frames: after wheeling, wait until a row's rect rests before clicking it (kittest clicks at the rect read before).
- "Waypoints (0)" matches both the module tree's blueprint group and an emptied area kind: wait on the GIT's own count instead, or an Undo can be pushed before the Delete is applied.
- In the least window the Outliner folds to its header (dock shorter than 180 + 116); `outliner::panel` returns after the header when no row fits.
