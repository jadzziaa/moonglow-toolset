---
type: Fact
title: "Soft texels' depth writes: where the woods' cost is, and selective writing ruled out"
description: "In-process A/B (2026-11-26 afternoon, cheap GPU state) of 62a1d436's soft-part depth writes in TDoD's woods: the cost is the soft draws' own writes (+0.35-0.45 ms turned), not the order or the draws after; writing only in front of water/particles (screen squares) cost more (+0.7-0.9) and still wrote 90% of soft parts."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T21:09:07Z }
touches: [crates/mg-render/src/renderer.rs, crates/mg-render/src/game.rs]
asks:
  - Why not write soft-part depth only where it stands in front of shiny water or particles?
  - Where does the GPU cost of soft see-through texels writing depth come from?
  - Does the draw order or lost early-Z after the soft parts cost the woods anything?
  - Was the +2.5 ms woods cost of soft-texel depth writes reproduced?
fingerprint: v2-81377d44646539d5
created: 2026-11-26
author: augustaugur
---
Measured with frame_perf's FRAME_ALT (even frames base, odd frames MG_VAR variant; patches in the main checkout's target/softw-ab/variants-v1.patch and -v3.patch, never committed), woods turned / overview, 4x MSAA, Shaders on, 160 frames, load 5-50, GPU in the cheap state all afternoon (sclk 900-1600 MHz):
- Fringe pass whole (nofringe): 4.2 ms of the turned view's ~9.7 GPU, 7-8 ms of encoding.
- Depth writes on soft parts (writes vs nowrite, new order): +0.35 turned, +0.3 overview.
- Order (soft parts before the copy, oldorder): 0. Draws after the soft parts (skipafter in both frames): the delta unchanged, so lost early-Z for later draws is not the cost; the soft draws' own writes are.
- Depth bias off on the writing pipelines: no change. radv_gfx12_hiz_wa=disabled: no change.
- Selective writing (SoftCover: 32x32 screen squares holding the farthest late/additive/particle draw; branch soft-water-2 commit "WIP: soft parts write depth only in front of what follows"): woods has 214 late water draws over ~26% of the squares, mostly under the ground, so 14,000-15,000 of 16,762 soft parts still write; cost +0.7-0.9 turned against develop. With water readers skipped (1,118 write) +1.4: mixing writing and non-writing soft pipelines in one back-to-front run costs more than writing all (pipeline changes 7450 all, 7756 cover, 8023 skip-water; cause beyond that unexplained).
- Depth pre-pass of soft parts rejected without a build: it repeats the 4.2 ms pass's draws.
- The morning's +2.5 ms (base 12.4 against 9.6) was not reproduced, by the old fp-softab and softab.sh either (+0.2-0.5 all afternoon).
Watch out: softab.sh writes target/retime/softab/<suite>.<var>.<round>.log and overwrites the earlier logs.
