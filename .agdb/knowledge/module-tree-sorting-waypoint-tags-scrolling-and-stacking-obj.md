---
type: Decision
title: "Module tree: sorting, waypoint tags, scrolling, and stacking objects"
tags: []
status: stable
generated: { by: trail, at: 2026-10-16T00:00:00Z }
touches: [crates/mg-ui/src/area_view.rs, crates/mg-ui/src/keys.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/tree.rs, crates/mg-ui/tests/app.rs, docs/deferred.md, docs/manual/02-main-window.md, docs/manual/04-areas.md, docs/manual/09-build-and-test.md]
created: 2026-10-16
author: August
description: "Design decisions for the module tree in tree.rs: objects sorted by row text with waypoint tags, scrolling only as needed, stacking via Drop onto Object (Shift+G) ray casts, plus egui scroll and test pitfalls."
asks:
  - "How does the module tree order an area's objects, and how do waypoints appear in it?"
  - "Why does the module tree jump to the top of an area when I open its tab, and how was it fixed?"
  - "How do I stack an object on top of another, and why isn't it a placement modifier key?"
  - "My UI test's double click registers as a triple click, or a freshly opened tree header won't scroll to the row. What's wrong?"
---
## Decided
- Module tree (crates/mg-ui/src/tree.rs): an area's objects are listed sorted by their row text; waypoints show their tag (name in the tooltip), as Aurora does. `KindContents.order` holds GIT indices in display order, so the keys, `goes` and `sought` logic keep working on GIT indices.
- Tree scrolling never re-centers: every reveal uses `scroll_to_rect(.., None)` (only as far as needed). A click on an object row sets `tree_object` itself, so the tree does not jump between the two clicks of a double click.
- Stacking is a command, not a placement mode: Drop onto Object (Shift+G) casts a ray down from 2 m above each selected object against placeables' and doors' models (`scene.ray_hit`), else the ground.
- A per-object "lock, not selectable" already exists (right-click › Lock, field `MG_Locked`).

## Tried and ruled out
- Stacking while placing via a modifier: Shift (place another), Ctrl (camera) and Alt (rectangle) are all taken during placement.

## Watch out
- The jump to the top of an area's tree came from `Action::OpenTab(Tab::Area)` setting `tree_reveal`, which scrolled the area's row (far above its objects) to the center. When the area is opened out and has a marked object, scroll to the object row instead.
- An egui CollapsingHeader opened this frame does not lay out its body until its animation starts, so keep `tree_object_pending` until the row has really been scrolled to.
- A UI test's double click right after a single click (within 0.3 s) becomes a triple click: wait ~30 steps first.
