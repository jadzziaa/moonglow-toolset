---
type: Decision
title: "Running the game's own shaders: the compile route and the corpus"
tags: []
status: stable
generated: { by: trail, at: 2026-10-21T00:00:00Z }
touches: [docs/deferred.md, docs/index.md, docs/log.md, docs/research/index.md, docs/research/notes_custom_shaders.md, docs/shaders-proposal.md, tools/shaders/spike.py]
fingerprint: v2-a029e69c04ecfb7e
created: 2026-10-21
author: August
description: "Accepted 2.0 plan runs the game's stock GLSL shaders via glslang, spirv-opt and naga to WGSL, proven on 102 programs; failures fall back to WGSL, and several details remain unproved."
asks:
  - "How do I convert the game's own .shd shaders to WGSL for rendering in 2.0?"
  - "Why does naga fail with 'invalid id' or BindingCollision on glslang SPIR-V output?"
  - "My custom-shader switch doesn't show tdod_shaders.hak overriding inc_lighting.shd; where should I test instead?"
  - "Why are stock .shd files extracted wrongly, picking up my overrides, and how do I avoid it?"
---
## Decided
- Proposal for 2.0 in docs/shaders-proposal.md (accepted 2026-10-21): tier B, the stock programs for every mesh with hak include overrides, tier A (MTR customshaderVS/FS) on the way; native glslang + SPIRV-Tools; on by default once the frame budget is met; a program that fails falls back to the WGSL shader and Verify reports it (no magenta). The shaders branch is deleted; its note is docs/research/notes_custom_shaders.md (deprecated).
- Route proven 2026-10-21 on 102 programs (65 stock + 37 TDoD custom): assemble (preamble + verbatim #include splicing, haks first) -> glslangValidator -V -R --amb --aml -S stage -> spirv-opt --split-combined-image-sampler -> naga 30 spv-in -> renumber every resource binding in naga IR -> validates, writes WGSL. tools/shaders/spike.py does the first two steps.
## Tried and ruled out
- naga spv-in on raw glslang output: 41 fragment shaders fail "invalid id" (combined image samplers). Split alone: BindingCollision (split pair shares a binding).
- spirv-opt --eliminate-dead-members shrinks member count, not the block span (m_bones 4 KB in every vertex program, light arrays): blocks stay 4.6-8.8 KB.
## Watch out
- Real custom shaders live in ~/Projects/tdod-module/build/hak (tdod_t_tap, tdod_p_tap, tdod_p_ham, tdod_heads) and tdod_shaders.hak overrides the stock inc_lighting.shd: a switch for custom-shader meshes only can't show that.
- Extract stock .shd with an empty --userdirectory, or the user's override wins.
- Not yet proved: vertex locations (--aml is arbitrary), varyings matched by name, clip-space flip (adjust_coordinate_space), the time uniforms' units.
