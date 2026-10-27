---
title: "A framebuffer for fswater and soft particles: the assessment"
author: August
date: 2026-10-27
branch: framebuffer-assessment
cost: { minutes: 1196, tokens: 11113748 }
touches:
  - crates/mg-render/src/framebuffer.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/src/game.rs
  - crates/mg-shader/src/assemble.rs
  - crates/mg-ui/tests/frame_perf.rs
  - docs/framebuffer-proposal.md
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
