---
type: Gotcha
title: "A selected outline's turning ring gives way to an object on it; break tests must touch the restored file"
description: "Why AreaView::ring_at skips an outline's ring over a non-outline object, how the ring tests place objects on a ring, and why a break script must bump the restored file's mtime."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T03:48:21Z }
touches: ["crates/mg-ui/src/area_view.rs:ring_at", crates/mg-ui/tests/app/area_view.rs]
asks:
  - "why can't I click a chest near a selected encounter's ring"
  - how do I put an object exactly on a turning ring in a test
  - cargo test still runs the broken code after restoring a file
  - why does Shift-click on a selected object never reach the add branch in tests
fingerprint: v2-4f5f4d59395f6836
created: 2026-11-23
author: augustaugur
---
Ring give-way (5.1.0 review F1 round 2, owner's ruling): AreaView::ring_at skips a ring whose owner is an outline (Trigger, Encounter) where the front pick under the pointer is not an outline, so a chest standing in a selected encounter's ring band is clicked and dragged as itself (the same rule as press_target). Over bare ground and other outlines the ring still wins; a selected waypoint's or chest's ring is unchanged. ring_at is the single source for the click guard, handle_under (drag and hover chip) and move_grip; the pick is made lazily, only when an outline's ring is under the pointer.

Tests: put objects exactly on a ring with view.ring_places()[0].0 (pivot) plus a direction times view.handle_radius_at(pivot) (a ring's radius is handle_radius at its pivot); screen() of that point at the pivot's height lands on the object's foot and on the ring.

Shift handles (axis arrows, tilt rings, scale handle) cover a small selected object at the default camera distance, so a Shift+click on it never reaches the click branch; to test Shift on a selected object, bring the camera close (orbit distance 2.5 m) and pick a point on the object far from view.handle_points(true).

Break-script trap: restoring a source file from a copy made before the break (shutil.copy/move) gives it an mtime older than the broken build, and cargo keeps the broken library; the next "fixed" run tests broken code. Touch the file after restoring (os.utime).

Double click on a selected overlapping outline: AreaView::cycled_from records the outline a plain click cycled away from; the double click's second click goes back to it, so its Properties are the outline double-clicked. Any click on an object in the view takes the field; egui 0.36.2 counts a double click only within max_click_dist of the earlier click, so a click elsewhere cannot meet a stale entry (a timed stamp was tried and dropped).
