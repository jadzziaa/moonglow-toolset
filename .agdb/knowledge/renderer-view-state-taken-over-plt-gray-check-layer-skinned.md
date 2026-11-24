---
type: Gotcha
title: "Renderer view state taken over, PLT gray check layer, skinned light ball, posing guard"
description: "Why a view state another view takes over must drop its target-bound bind groups, why the PLT gray check must read the drawn layer, why skinned meshes use a coarse light ball, and why the scene needs no guard around Composed::instances."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T01:14:14Z }
touches: ["crates/mg-render/src/renderer.rs:ViewState", crates/mg-render/src/views.rs, crates/mg-render/src/assets.rs, crates/mg-render/src/reach.rs, "crates/mg-area/src/grass_view.rs:ChunkWorker", crates/mg-preview/src/compose.rs]
asks:
  - "Why did water or shadows in a fifth 3D view read another view's depth?"
  - Why does a part draw its PLT when override has a true-color TGA?
  - "Why are an idle creature's lights chosen anew every frame?"
  - Is c.instances in AreaScene guarded against a panicking pose?
fingerprint: v2-4d168bf369655e99
created: 2026-11-24
author: augustaugur
---
Adversarial review fix R (2026-11-23), renderer and area scene.

- Views::take hands a fifth view the oldest view's ViewState "as it is". Everything in it must say what it is of. The cached bind groups (copy_group, sun/lights darken, the stencil volumes' copy and darken groups) bind the old view's depth and color, so ViewState::groups_of records the view (copies id) they bind, and draw_scene calls forget_groups when it differs. The sun group's SunMap id key does not cover this: the map goes with the state. Test: a_fifth_view_binds_its_own_targets (a roof in the first four views; without the fix the fifth loses its plate's shadow, Fast and High). copy_group and lights_darken_group are covered only by the same forget_groups (the test has no game shaders or light shadows).
- ResMan::texture / texture_layer choose a texture by layer class (directories, NWSync, ERFs, keys; DDS before TGA in each), while ResMan::get / get_start take the highest-priority layer. A hak (Erf) is above override (Directory) by priority but loses to it by class, so the gray check must read the start of the layer texture_layer names (assets.rs drawn_is_gray), never get_start.
- posed_box is the union of the rest box carried by every bone: any rotation makes it bigger than the rest box, and it changes every frame. As the reach cache key it missed every frame. A "rest ball unless the posed box leaves it" rule never fires for that reason. Skinned and vertex-animated meshes now use coarse_ball (centre on a 0.25 m grid, radius rounded up past the shift); static meshes keep the exact ball. Drawn::lights_chosen counts fresh choices (test a_mesh_a_skin_sways_takes_its_lights_as_chosen).
- Composed::instances wraps posed() in guard::guarded itself and latches its own failed flag (since 6a3f3746), so AreaScene calling it outside its own guard is safe. guard::fail_on only reaches GpuModel::new (model.rs), not posing, so it cannot test posing failures.
- A grass chunk that failed stays failed until it is made of other tiles: ChunkWorker keeps the MadeOf of pending and failed chunks. Do not clear failures on every AreaScene::update.
