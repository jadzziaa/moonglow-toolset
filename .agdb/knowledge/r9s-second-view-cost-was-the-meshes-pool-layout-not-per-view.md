---
type: Gotcha
title: "R9's second-view cost was the meshes' pool layout, not per-view state"
description: "Why a second view through a shared renderer seemed 0.2-0.4 ms dearer than through its own (the arms' meshes in different pool chunks, Drawn::chunks), how it was found (phase timers, callgrind on finish), and the frame bind group now kept per view."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T20:22:40Z }
touches: ["crates/mg-render/src/renderer.rs:ViewState", crates/mg-render/src/pool.rs, crates/mg-ui/tests/render_views/mod.rs]
asks:
  - "Why does drawing a second view through the area's renderer cost more than a renderer of its own?"
  - Why do two timing arms of the same area differ in encoding time?
  - "What does Drawn::chunks tell me in a timing view?"
  - How can I profile wgpu encoding without perf on this machine?
fingerprint: v2-c5519c9196ec412a
created: 2026-11-26
author: augustaugur
---
- Arms that each make their own AreaScene one after another get different places in the GPU-wide meshes' pool (pool.rs): the first area made there (in the pool's first, small chunks) bound chunks' buffers most, but where the meshes go is what decides it, not simply order: with a scene made before them the three arms bound 185 / 3 / 625. The game's solid draws are sorted by (pipeline, material, first draw), not by chunk, so the three arms made in turn bound chunks' buffers 2,106, 237 and 3 times a frame: +0.27 ms in wgpu's CommandEncoder::finish (wgpu 30 encodes the passes there, not at pass time). That was R9's whole "shared renderer" cost; the earlier note's "something of a view's still redone" was wrong.
- Check Drawn::chunks per arm first. The R9 view now shares one scene across arms (Pane::sharing) and alternate() warns when the arms' chunks differ. R22's arms differ too (95 vs 15 alone, 2,296 vs 3 after the other views), its arms having different content.
- The only true per-view redo was the frame bind group (0.02 ms): now ViewState::frame_group, keyed by Renderer::frame_buffers (bumped when the light, bone, draw or order buffer grows) and the mask's id; Drawn::frame_group_made; test two_views_in_turn_keep_their_frame_groups.
- Profiling without perf (perf_event_paranoid 2 refuses samply): temporary Instant marks in draw_scene into a pub array, medians per phase; then valgrind --tool=callgrind --toggle-collect='*command_encoder_finish*' on the frame_perf binary with one arm kept, FRAME_COUNT=10, and diff callgrind_annotate's self costs between the arms. About a minute a run.
- Not fixed (deferred.md): sorting a material's draws by chunk would break game::sort's run-to-run order for coplanar meshes; pool placement is its own design.
