---
type: Fact
title: "A cut-out is the TXI of the mesh's bitmap; the hint and MTR transparency change nothing"
description: "Which TXI makes a mesh a punch-through cut-out in the NWN client (the bitmap's, an MTR's own name's), that transparencyhint and an MTR's transparency don't make it blend, and how Moonglow matches it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T12:46:22Z }
touches: [crates/mg-render/src/renderer.rs, crates/mg-corpus-tests/tests/client_render/soft.rs, crates/mg-render/tests/render.rs]
asks:
  - Why is a punchthrough texture with transparencyhint 1 drawn solid?
  - "Does an MTR's texture's TXI blending punchthrough make the mesh a cut-out?"
  - "Where does the client cut a punch-through texture's alpha?"
  - Why does TestAssets follow an MTR to its texture?
fingerprint: v2-ecade10e1dda767b
created: 2026-11-27
author: augustaugur
---
Read 2026-11-27 with client_render `soft::reading_punch_cut` (ten cards; the test asserts the client's solid/blended per card and Moonglow's against it).
- Cut at alpha 0.2 for every kind (50/255 cut, 52/255 kept): fAlphaDiscardValue, as before.
- Cut-out (solid where kept) iff the TXI of the name the mesh binds says `blending punchthrough`: bitmap naming a texture -> that texture's TXI (materialname too); bitmap naming an MTR -> `<mtr name>.txi`, NOT the MTR's texture0's TXI. transparencyhint 1 and MTR transparency 1 do not make a cut-out blend.
- My prediction before the run: solid for hint and MTR transparency alike; the MTR card (texture TXI) proved blended, which led to the four extra MTR cards.
## Moonglow
- renderer.rs Renderer::surface overrides `blending` when the bitmap names the MTR (slots.mtr equal to the slot-0 name, not renamed, not a materialname): Punchthrough iff assets.txi(that name) says so, else Default (Additive kept). It reads slots.mtr rather than calling assets.material per surface (a first version did; that pair's woods opening frame was 10 ms longer, one run each). Turned, no cost; the final pair (frame_perf game_shaders_frames_at_144, one quiet hold): the woods opening's longest frame 21.0 -> 24.4 ms off, 27.4 -> 33.4 on, shown at 375 ms both: noisy, a TXI lookup per MTR-named surface remains. materialname with a texture0 other than the bitmap is unread; Moonglow keeps texture0's TXI. The draw loop's `see_through` no longer counts the hint for a cut-out. Every other punch site (blended_by, ordered, extra.w 3) follows surface.blending.
- Game assets (assets.rs image_of) read texture0's TXI for an MTR name; that is why the override is in surface(), not in the texture entry.
- render.rs TestAssets::texture now follows an MTR (texture0) for a name of no texture, as ResMan does; without it an MTR-named bitmap drew untextured (solid) and the test passed wrongly.
- Not read: whether a hinted cut-out casts shadows (shadow::mesh_casts excludes hint > 0; the cards were shadow 0).
