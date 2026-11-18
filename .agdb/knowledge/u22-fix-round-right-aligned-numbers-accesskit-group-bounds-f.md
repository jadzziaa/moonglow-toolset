---
type: Gotcha
title: "U22 fix round: right-aligned numbers, AccessKit group bounds, floating windows placed clear of the strips, journal whole writes"
description: egui gotchas met in the U22 fix round of the journal and faction editors and how each was solved.
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-18T21:21:09Z }
touches: ["crates/mg-ui/src/blueprint/mod.rs:number_at_right", crates/mg-ui/src/faction_view.rs, "crates/mg-ui/src/lib.rs:open_window", "crates/mg-ui/src/blueprint/follow.rs:followed_journal"]
asks:
  - "why can't a DragValue be right-aligned in egui"
  - Every egui node should have a rect panic on a group
  - why did the floating faction editor fold its Outliner
  - how is typed journal text kept when a plugin writes the journal whole
fingerprint: v2-16b658ec37bfc02e
created: 2026-11-18
---
## Watch out
- egui 0.36's `DragValue` grows its value atom to the width given, so its text is centered and no layout right-aligns it. `blueprint::Form::number_at_right` paints the number itself right-aligned (the DragValue's own text made transparent) while it is not being typed in; typing is told by `has_focus(ui.next_auto_id())`.
- A group named with `name_group` has no AccessKit bounds of its own, and kittest's `Node::rect()` panics "Every egui node should have a rect". `faction_view` gives the MATRIX group its bounds with `ctx.accesskit_node_builder(id, |b| b.set_bounds(..))`.
- `DragValue` sets its value while typing (no pending text). The Reaction panel's value field sits in `ui.push_id((pair, names), ..)` so a pair changing under it (the factions written whole) ends the typing instead of carrying the keys to another pair.
- Shrinking `room` in `Moonglow::open_window` to clear a folded dock's strip made the floating editor too narrow and folded its own Outliner: the window is instead placed (and only narrowed to) a `clear` rect inset by the strips plus egui's grab margin.
- The Add Faction window's checkbox and the Faction panel's are both "Global effect": a test must pick the one outside `right_dock_rect`.
- The journal written whole (`Edit::SetResource`, a plugin's `ctx.edit:write_gff`) is followed by identity in `follow::followed_journal`; an entry not found drops its typed text. A test runs the plugin command after typing, then waits `!busy` (`settle` panics while a job keeps repainting).
