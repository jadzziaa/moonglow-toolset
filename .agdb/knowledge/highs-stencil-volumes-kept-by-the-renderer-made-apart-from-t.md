---
type: Decision
title: "High's stencil volumes: kept by the renderer, made apart from the frame"
description: "Why the meshes' shadow volumes live in the renderer's VolumeMeshes slab (not GpuMesh), how the area view makes them on threads beside the frames, and the per-caster lookup cost that a hash map added and the slab with a per-mesh hint removed."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T20:59:40Z }
touches: ["crates/mg-render/src/volume.rs:VolumeMeshes", "crates/mg-render/src/volume.rs:Slab", "crates/mg-render/src/renderer.rs:volumes_apart", "crates/mg-ui/src/area_view.rs:let_go_of_unseen", "crates/mg-ui/tests/frame_perf.rs:sound_ranges_frames_at_144"]
asks:
  - Why does a caster cast no High shadow for the first frames after an area opens?
  - "Where are the stencil shadow volumes kept, and when are they freed?"
  - Why does GpuMesh have a volume_at hint instead of the volume itself?
  - "Why was area084's first frame with High shadows 35 ms?"
fingerprint: v2-f314aba738c37743
created: 2026-11-25
author: augustaugur
---
Decided (2026-11-25, branch shadows-first-frame):
- area084's first High frame spent 19.7 ms making the 846 casting meshes' volume meshes (volume_mesh + buffers) and 1.6 ms on the stencil target and the volumes' pipelines (left in the frame: once a session). Measured with a timer around ShadowVolumes::lay's casters, not a guess.
- The volumes are the renderer's (volume::VolumeMeshes inside ShadowVolumes), let go of with their model (GpuModel::life, an Arc<()> whose Weak the renderer holds) or when none of a model's has cast for 30 s of the kept clock (High turned to Fast). Swept at each frame's beginning (render_targets, beside let_go_of_stale_textures) and by Renderer::let_go_of_gone_models, which area_view::let_go_of_unseen calls when a closed tab drops its scene: no frame may be drawn after that.
- Renderer::volumes_apart (set by the area view as textures_apart is, from background) makes a missing volume on two 'moonglow shadow volumes' threads; its caster casts nothing until then (Drawn::volumes_pending, which the area view treats as pending work: redraw at the next step). Off (pictures drawn once, tests, the client comparisons) the frame makes them, also for a key already asked apart (the late result is dropped). The Maker joins its threads on drop (the validation-layer exit crash note). Renderer::hold_volumes holds the threads, per renderer, for the test.
- A closed mesh's both-sides volume is its front one (Kept::Front), as before: no doubled memory for lights.
Tried and dropped:
- A Fast HashMap keyed by (model id, node, both) looked up per caster: the volumes' part of a steady High frame of area084 (4,764 casters) went from 0.08-0.09 ms to 0.14-0.22 ms. Packing the key into one u64 and avoiding an Arc clone per caster helped little; what fixed it: a slab whose place each GpuMesh remembers (volume_at, an AtomicU32 hint checked against the key, so another renderer's or a reused place falls back to the map), and Counted holding places, not Arcs. Back to 0.08-0.09 ms, timed in turn with the old build.
Watch out:
- frame_perf's harness clock is 1/144 s a step: waiting for the 2 s let-go of an unseen view takes 3*144 steps, not wall time.
- The opening's whole condition in frame_perf now waits for volumes_pending too; it prints how many frames casters waited (6 frames, 29 ms in area084).
