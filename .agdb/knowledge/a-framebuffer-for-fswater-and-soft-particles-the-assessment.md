---
type: Decision
title: "A framebuffer for fswater and soft particles: the assessment"
tags: []
status: stable
generated: { by: trail, at: 2026-10-27T00:00:00Z }
touches: [crates/mg-render/src/framebuffer.rs, crates/mg-render/src/renderer.rs, crates/mg-render/src/game.rs, crates/mg-shader/src/assemble.rs, crates/mg-ui/tests/frame_perf.rs, docs/framebuffer-proposal.md]
fingerprint: v2-45d55763927e35e4
created: 2026-10-27
author: August
description: "Assessment recommends adding a framebuffer copy pass (resolved MSAA colour plus depth into R32Float) for fswater and soft particles, gated on comparing against surfaces_in_the_client; a +4 ms TDoD cost came from VRAM exhaustion, not the copy."
asks:
  - "Which shaders in the game actually read texFBColor or texFBDepth, and do any of TDoD's programs need a framebuffer copy?"
  - "Why does the framebuffer copy cost an extra 4 ms in TDoD's woods, and is the pass cut or decompression to blame?"
  - "Water renders with flipped or mismatched screen coordinates when reading the framebuffer; how do I fix gl_FragCoord handling in fswater?"
  - "How do I compare fswater with the copy against surfaces_in_the_client before turning on GAME_WATER, given client pictures get deleted?"
---
## Decided
- Proposal docs/framebuffer-proposal.md (draft, for the owner): do the copy (M), gated on comparing fswater+copy with surfaces_in_the_client before GAME_WATER flips. Spike on branch framebuffer-assessment (039c999b), off by default, not for develop as is.
- Only fswater (inc_water -> inc_framebuffer) and fsparticle (particleSoftness > 0) read texFBColor/texFBDepth among drawn programs; none of TDoD's 37; TDoD's two sample_framebuffer 2 MTRs (flow) read no FB (the flag = draw late).
- Copy design: cut the pass after the game's solid block, one full-screen draw resolving MSAA colour (mean) + depth sample 0 into R32Float (unfilterable: NonFiltering sampler + Float{filterable:false} for texFBDepth); stored depth already = GL's (link.rs (z+w)/2), clipParams filled.
- gl_FragCoord: fswater mixes FragCoord-derived and projection-derived screen coords; consistent fix = copy rows flipped (GL) + FragCoord.y -> height - y in FB-reading programs (glslang refuses #define gl_*: spike rewrote text; do it in IR).
## Tried and ruled out
- Blaming the pass cut or decompression for a +4 ms copy in TDoD's woods: the cut alone is free, the copy at frame end costs the same, RADV_DEBUG=nodcc,nohiz the same. Cause: frame_perf's TDoD area fills the 16 GB card (16.8 of 17.1 GB, 2 GB GTT); textures made after the frame's textures land in system memory. Made first: 0.0-0.2 ms.
## Watch out
- Watch VRAM (/sys/class/drm/card1/device/mem_info_vram_used) when timing anything allocated late in TDoD runs.
- Client pictures of surfaces_in_the_client are not kept (scratch_dir empties them): comparing needs a client run.
- inc_framebuffer assumes a 45 deg fov; Moonglow's camera is 40.
