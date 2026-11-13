---
type: Decision
title: "Model viewer shadow, faces' fronts, buckets, model-order see-through, kept pipelines"
tags: []
status: stable
generated: { by: trail, at: 2026-11-02T00:00:00Z }
touches: [crates/mg-render/src/renderer.rs, crates/mg-render/src/shadow.rs, crates/mg-render/src/volume.rs, crates/mg-render/src/game.rs, crates/mg-render/src/model.rs, crates/mg-ui/src/model_view.rs, crates/mg-ui/src/pipelines_kept.rs]
fingerprint: v2-67fd98ed36949219
created: 2026-11-02
author: August
description: "Records model viewer rendering decisions: sun shadows cast from front faces only via two volume slots, static mesh buckets, depth-sorted see-through meshes in model order, a shadow-receiving ground quad, kept pipelines, plus shadow and test pitfalls."
asks:
  - "Why does the sun cast shadows only from front faces, and how do mesh volume slots differ between sun and light shadow maps?"
  - "How are translucent meshes with alpha below 1 ordered and drawn in draw_scene, and what pass do they use?"
  - "Why did my flat ground test scene show no shadows, and what fixed the shadow::seen box collapsing for flat receivers?"
  - "How do I disable shadows or find the ground quad in the model viewer, and where is it placed?"
---
## Decided
- The sun's and moon's shadows cast from faces' fronts only: the sun map's `Casting` culls back faces (`Casting::new(device, cull)`; the lights' maps keep `None`), and a mesh has two volume slots (`VolumeSlot = [OnceLock; 2]`): front-only for the sun, both sides for a light's (a closed mesh's second slot is its first).
- Static geometry's buckets: `GpuMesh::bucket` (first mesh naming the same texture and maps, lowercased), used in `draw_scene` where `inst.tile || inst.receives_shadows` (receives_shadows is exactly "a tile or a static placeable" in mg-area's scene).
- Meshes of mesh alpha < 1 (instance opacity 1, not punch-through): `ordered` in `draw_scene`: one draw, Moonglow's `Pass::Blend` with `extra.w = 0` (no SOLID_ALPHA split; Blend's pipeline writes depth) and the game's `GamePass::Ordered`; sorted by the instance's depth, ties by push order = model order.
- The model viewer's ground: an unlit quad in the background color, receives shadows, at min(rest box z, 0) − 1 cm; `Settings::model_shadows` (Option<bool>, true by default).
- Kept pipelines: `PipelineKey::line`/`from_line`, `Renderer::game_pipelines_made` / `prepare_game_pipelines` (quiet jobs: a missing program is skipped before sending, a failure neither reported nor kept), `mg_ui::pipelines_kept` once a frame.
## Watch out
- `shadow::seen` cut the receivers' box to nothing for a flat receiver (rays' crossings a hair above its height): fixed by snapping an inversion under 1 mm. Any flat-ground test scene would have shown no shadows.
- A client reading's Moonglow side is redrawn without the client with `MG_CLIENT_PICTURES` pointing at the `target/test-output` of the worktree that ran the client (the seventh batch's: agent-a2254f…, the sixth's ribbons: agent-a60b1d…); the bucket reading draws no Moonglow side.
- frame_perf on this machine swings 3× in CPU when other agents load it (tessellate time tells: 0.06 vs 0.33 ms); compare draws counts and rerun before calling a regression.
- The thumbnail texture needed `COPY_SRC` for a test to read it back.
