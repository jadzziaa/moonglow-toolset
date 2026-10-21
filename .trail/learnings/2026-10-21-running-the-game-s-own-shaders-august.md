---
title: "Running the game's own shaders: the compile route and the corpus"
author: August
date: 2026-10-21
branch: develop
cost: { minutes: 1, tokens: 955 }
touches:
  - docs/deferred.md
  - docs/index.md
  - docs/log.md
  - docs/research/index.md
  - docs/research/notes_custom_shaders.md
  - docs/shaders-proposal.md
  - tools/shaders/spike.py
---

## Decided
- Proposal for 2.0 in docs/shaders-proposal.md (draft, owner's decisions open): tier A (MTR customshaderVS/FS) on the way to tier B (stock programs for every mesh, hak include overrides included), WGSL shader kept as fallback.
- Route proven 2026-10-21 on 102 programs (65 stock + 37 TDoD custom): assemble (preamble + verbatim #include splicing, haks first) -> glslangValidator -V -R --amb --aml -S stage -> spirv-opt --split-combined-image-sampler -> naga 30 spv-in -> renumber every resource binding in naga IR -> validates, writes WGSL. tools/shaders/spike.py does the first two steps.
## Tried and ruled out
- naga spv-in on raw glslang output: 41 fragment shaders fail "invalid id" (combined image samplers). Split alone: BindingCollision (split pair shares a binding).
- spirv-opt --eliminate-dead-members shrinks member count, not the block span (m_bones 4 KB in every vertex program, light arrays): blocks stay 4.6-8.8 KB.
## Watch out
- Real custom shaders live in ~/Projects/tdod-module/build/hak (tdod_t_tap, tdod_p_tap, tdod_p_ham, tdod_heads) and tdod_shaders.hak overrides the stock inc_lighting.shd: a switch for custom-shader meshes only can't show that.
- Extract stock .shd with an empty --userdirectory, or the user's override wins.
- Not yet proved: vertex locations (--aml is arbitrary), varyings matched by name, clip-space flip (adjust_coordinate_space), the time uniforms' units.
