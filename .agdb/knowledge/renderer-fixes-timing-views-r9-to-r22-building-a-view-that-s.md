---
type: Gotcha
title: "Renderer fixes' timing views (R9 to R22): building a view that shows a render fix"
description: "How frame_perf::render_views times the review's renderer fixes on alternate-frame arms, the traps that hid a fix (swaying tiles redraw the sun map, the bandit UTC is not of parts, set flags vs sl1 nodes, TDoD hak order), and what the views found."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T12:46:48Z }
touches: [crates/mg-ui/tests/render_views/mod.rs, crates/mg-render/src/shadow.rs]
asks:
  - Why did the R15 timing view show no difference before the fix?
  - How do I build a renderer timing view that compiles on old commits?
  - Why does a tinted TDoD area cost more to encode than untinted?
  - "Does the sun's shadow map redraw every frame in an area with trees?"
fingerprint: v2-eccf72f180fce9d2
created: 2026-11-26
author: augustaugur
---
- The views live in crates/mg-ui/tests/render_views/mod.rs, a module frame_perf pulls in with one line; it uses only APIs stable since 9 Nov (Gpu::headless, AreaModel::read, AreaScene, Renderer::render_targets), so an old tree needs the file plus the mod line. Builds: a second worktree under target/, one target dir (path-distinct packages coexist), binaries copied out and told apart by md5.
- The sun's map is drawn again whenever a caster's world matrix changes: ttr01's swaying trees and animated tiles redraw it every frame, which hid R15 (an animated creature out of the region) completely. Use tiles whose models have no animations (still_tiles). Check with Drawn::sun_map_drawn on develop.
- nw_bandit001 is appearance 269, not of parts: set Appearance_Type 6, every BodyPart_* (Belt and shoulders 0) and ArmorPart_RFoot, or only a head is drawn.
- A tileset's .set source-light flags don't mean the model has an sl1 node: filter by TileFeatures::of(model).source.
- TDoD placeables.2da ModelName is upper case; the module's hak order is tdod_top highest, tdod_p_* by name, then tdod_heads, tdod_shaders lowest.
- Found (load 9 to 43): R9 +1.5 to 2.1 ms encode before, 0.3 to 0.5 after; on develop a third arm (the second view through a renderer of its own) shows the shared renderer still 0.2 to 0.4 ms dearer, not the shadow map (cause not found). On-processor figures from schedstat sit on 1 ms ticks: do not read sub-ms deltas from them; R15 GPU +0.3 before, 0 after; R16 particles +0.55 to +0.35; R22 tinted +0.6 to 0.8 ms before and after alike (not the lookup; 288 more draws are about a third by average draw time); R12 29 copies of a DDS head, 19.3 MiB; R13 first frame 600+ ms to 110-200.
