---
title: "The game's shaders within the frame budget (M3): split blocks, instances, worker, cache"
author: August
date: 2026-10-21
branch: shaders-2
touches:
  - crates/mg-shader/src/spirv.rs
  - crates/mg-shader/src/link.rs
  - crates/mg-shader/src/cache.rs
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/tests/frame_perf.rs
  - docs/shaders-proposal.md
---

## Decided
- Uniform split happens in naga IR but is PREPARED in SPIR-V: naga's validator forbids forward expression refs, so new globals' expressions can't be appended. naga spv-in creates a GlobalVariable expression for EVERY global at each function's start (make_expression_storage), so spirv.rs declares spare blocks (mg_frame, mg_bones) and storage arrays (mg_draws, mg_lights of vec4; vs mg_order of uvec4) plus, at each function's start, OpLoads of the draw's places and OpAccessChains to its elements. link.rs split_uniforms then retypes those globals and rewrites AccessIndex(block, k) to the part's base.
- Instancing like the WGSL path: vs reads mg_order[instance_index] (vs draw, fs draw, vs lights, fs lights), passes fs places as flat varyings mg_draw/mg_light. Each pipeline's values in a region of one storage buffer at a multiple of its stride (whole-index places, group 2 bound once per pipeline). Bones = dynamic uniform group 3, per draw only for skinned pipelines (skin_slot) — every vs has a bones block.
- GAME_SHADERS_BY_DEFAULT = true by the rule "on stays within budget wherever off is". area092 zoom-out 5.97 ms on / 3.97 off (was 12.37). TDoD synthetic woods seen whole 54 on / 29 off (both over) -> deferred.
- One worker thread for glslang+pipelines; Renderer::game_background (area view, model view) draws WGSL until ready; one-shot pictures/tests wait. ProgramCache in cache_dir()/shaders, key = FNV-128 of assembled text+stage+crate version+FORMAT (bump FORMAT when passes change).
## Tried and ruled out
- Lights in a 4th dynamic uniform block shared per light set: 5.1 -> 5.7 ms (one more dynamic offset per draw costs more than the KB saved). As a storage array (no per-draw bind) it pays.
- Persistent per-draw slices: m_mv/m_mvp/m_normal/view-space lights change whenever the camera moves, the only over-budget case.
## Watch out
- glslang keeps every DECLARED uniform: vertex blocks held the lights. Leave unread members out of the split (vslit draw block 1392 -> 80 B).
- glslang may already declare an InstanceIndex builtin input: reuse it ("present more than once").
- Zero-input vertex buffer must be array_stride 0 once instance ranges > 1.
- The big wins after instancing: see-through mesh parts sharing values (no copy), keeping bound state across one-by-one see-through draws (TDoD 56 -> 40 ms), not rebinding the vs group when the vs samples no texture (drop unused samplers via ModuleInfo GlobalUse).
- Harness::run never settles while the model view asks for frames during compiles: pin tests to settings.game_shaders = Some(false) unless they test the switch.
- eu-stack profiling: filter stacks with "Renderer>::render&&GameShaders" (report.py) or idle worker threads swamp the samples.
