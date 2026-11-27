---
type: Fact
title: "AnimationState reaches the area picture only through Preview::idle; a re-dress restarts particles"
description: "Read while designing animated objects: scene.rs never reads AreaObject::state, the picture follows AnimationState through mg_preview::object to Preview::idle, walk.rs alone reads state, and the scene's particle key restarts an object's sims when its look changes."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T07:36:46Z }
touches: [crates/mg-area/src/scene.rs, crates/mg-area/src/walk.rs, crates/mg-preview/src/object.rs, crates/mg-area/src/values.rs]
asks:
  - does the area scene pose a door or placeable from AnimationState
  - "why does a door's walkmesh say open while its picture is closed"
  - "why do an object's particles start again after its equipment changes"
  - where would per-object animation state for a game client live
fingerprint: v2-4b2b5b908bad6b0c
created: 2026-11-27
author: augustaugur
---
- `AreaScene::scene_hiding` draws each object with `c.instances(c.idle, object_time(..), transform)`; it never reads `AreaObject::state` (scene.rs's `posed.state` is a tile's mesh state).
- The picture follows `AnimationState` only through the look: `mg_preview::object::placeable` maps 1 open, 2 close, 3 dead, 4 on, 5 off, else default; `object::door` maps 1 opened1, 2 opened2, else closed, into `Preview::idle`, copied to `Composed::idle`.
- `AreaObject::state` is read only by `walk.rs` (door walkmesh surfaces). `values::Object::read` and `Look::read` read the field separately and `AreaObject::from_values` keeps both copies unsynced: a values-built door can have an open walkmesh and a closed picture. `Look::placeable` sets AnimationState only on a scratch struct; there is no `Look::door`.
- Particle sims are keyed by `source_key(object_place, Arc::as_ptr(shown), source)`; `carry_over` matches the same look only, so a re-dress gives a new sim that runs 20 s ahead (`PARTICLES_AHEAD`). That run-ahead would also spend a visual effect's impact burst before its first frame.
- Decided in the proposal (draft, docs/animated-objects-proposal.md): run-time play state is held by the caller beside the scene (`mg_area::Live`, like particle sims), keyed by a new `ObjectKey`, never on `AreaModel` (the values, hashed by area_values.rs) and not in `Composed`/`Shown` (shared by Arc among objects of one look).
