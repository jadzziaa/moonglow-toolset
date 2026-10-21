---
title: "The game's lights read by index: the water view within the frame budget"
author: August
date: 2026-10-21
branch: shaders-2
cost: { minutes: 168, tokens: 927366 }
touches:
  - crates/mg-shader/src/spirv.rs
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-corpus-tests/tests/shaders.rs
  - crates/mg-render/tests/render.rs
  - docs/shaders-proposal.md
---

## Decided
- Measured first with phase timers in Renderer::render (Instant per phase, env-gated) plus GameShaders counters: water view encode 4.36 on vs 2.74 off, GPU lower on (1.35 vs 1.69) -> CPU. push 1.3 ms: renderer light-set building+SipHash+Box ~0.45, lights blocks 0.57 (1,524 blocks x 1,040 B, element-at-a-time put_one), draw values 0.46; upload 0.15 (2 MB, 1.58 of it lights). Water sets 96% unique (mean 20.9 lights); area092 shares (3,571 draws -> 2,032 blocks) so the dedupe stays.
- Stage 1 (CPU only): DrawValues carries frame_lights + light_index (indices) instead of per-draw position/color Vecs; put_array copies vec4/mat4 arrays a vector/matrix at a time; Quick::write 8 bytes at a time (M5's "quick hasher slower" was byte-at-a-time) + last-set check; mv computed once per draw. Water encode 4.36 -> 3.83.
- Stage 2: SPIR-V pass index_lights (before add_spare_blocks): appends member mg_lightIndex (uvec4[ceil(N/4)]) to the default uniform block, rewrites OpAccessChain(block, K_lightPosition|K_lightColor, i, rest..) into: bitcast i to uint if signed, q=i>>2, r=i&3, load block.mg_lightIndex[q][r], t=2*l+which, AccessChain(mg_light_table, 0, t, rest..). Result pointer type is unchanged (Uniform+BufferBlock storage like add_draw_index's). Table place 0 = zeros (old arrays were zero past numLights), indices written +1. Lights block 1040 -> 144 B. Whole-array use or non-vec4 arrays -> module left as is; renderer fills both layouts by name. Cache FORMAT 3.
- Dense Vec indexing for regions/light_places/filled (ids count from 1, sets from 0 per frame): -0.07..0.09 ms.
- Result: water overview wheel 7.5 -> 6.5 ms on (off 5.6), area092 5.7-6.1 -> 5.2 (off 4.0). Default stays on.
## Tried and ruled out
- Smaller MAX_NUM_LIGHTS per draw: loops are bounded by numLights at runtime, a smaller define only shrinks arrays (the index does that) and needs pipeline variants.
- Sharing lights blocks across pipelines: 1,468 sets for 1,524 blocks, nothing to share.
## Watch out
- Building a SPIR-V type map keyed by args[0] over all declarations is WRONG: OpConstantComposite's args[0] is its TYPE, so a vec4 constant overwrote the vec4 type entry and the pass silently bailed on every stock program (authored tests had no vec4 constant). Key types by opcodes 19..=39 only. The corpus test now asserts no stock program still reads lightPosition/lightColor directly (mutation-checked).
- A result-type map over function instructions must skip no-result ops (OpStore's args[1] is a value, OpDecorate's a decoration number).
- New types a modified struct member uses must be declared before the struct: hoist uint/uvec4/float/vec4 to the first declaration.
- Per-draw Instant::now() timers cost ~0.25 ms on 2,700 draws; use frame-level phases for A/B.
- FRAME_SHOTS of areas with animated tiles (area092's sea) differ run to run: the view's clock counts opening frames. Compare at a fixed clock (area_parts render_image) instead.
- Other agents' builds give bimodal runs (a whole renderer config at +0.7 ms, sort/batch phase doubled): alternate A/B several times; run GPU work under the shared flock.
