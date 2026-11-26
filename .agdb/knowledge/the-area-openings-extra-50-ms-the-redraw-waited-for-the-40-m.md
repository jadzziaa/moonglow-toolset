---
type: Gotcha
title: "The area opening's extra 50 ms: the redraw waited for the 40 ms animation step"
description: "Why an area's opening to its last texture was ~50 ms longer since 3.0.0, how it was taken apart (old tags, a misleading bisect) and the fix; plus the compression-as-read and MSAA facts found beside it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T00:39:50Z }
touches: [crates/mg-ui/src/area_view.rs, "crates/mg-render/src/renderer.rs:textures_still_read", "crates/mg-render/src/texture.rs:compress_read", "crates/mg-render/src/gpu.rs:sample_counts", crates/mg-image/src/encode.rs]
asks:
  - Why does an area take longer to show its last texture than in 2.13.3?
  - Why is the picture not redrawn right after textures finish loading?
  - "Which textures does Moonglow compress to BC as they are read, and why not maps?"
  - Why is 2x MSAA missing on some adapters?
fingerprint: v2-28bde099177d0acb
created: 2026-11-26
author: augustaugur
---
Found 2026-11-25 (branch texture-memory).
- Cause: after an area was first shown, the picture was redrawn only at the animation's 40 ms steps (ANIMATION_STEP), though every texture had been read long before; area092 whole after 174-177 ms vs v2.13.3's 102-110. Fix: the area view draws again at once when Renderer::textures_still_read() == 0 and the last picture had textures pending (take_textures used to run only inside a render). The model viewer reads textures in the frame (textures_apart false), so it never waited; a read_all there was dead code and was taken out. Compression adds ~10 ms to area092's opening (medians 122 vs 111), no longer frames. Test an_area_is_drawn_again_as_soon_as_its_textures_are_read.
- Ruled out: the harness's step dt (2.13.3 patched to 1/144 still 103-113), the test's drag_selection (175 without it), compression (same with FRAME_UNCOMPRESSED=1).
- A bisect over develop's first-parent merges named b6404480 ("wave 5d"), which touches no renderer/area-view code: a timing coincidence; dd74c1b8 gave 110 and 156 in two runs (bimodal). Don't trust one-run bisects of the opening; time 2-3 runs each.
- Old tags: git archive <tag> | tar -x -C target/old/<tag>, own CARGO_TARGET_DIR, frame_perf patched (scripts in target/texmem/, not committed).
- Compression as read (texture::compress_read): only RGBA8 in a material's slot 0 (Role::Picture), sides multiples of 4, smaller side >= 64, not TXI procedural; BC1 if all alpha 255 else BC3, Effort::Quick. mg_image::encode makes only BC1/BC3 (no BC4/5/7), so normal/spec/roughness maps stay as stored (none read as RGBA8 in area092 or TDoD woods anyway). Encoder Best was 72-192 ms/MP single-thread; Quick + rayon block rows 4-6 ms/MP.
- MSAA: wgpu allows 2x only with Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES and every target format (color, Depth32Float, the R8 mask, the stencil format) supporting it; without it 1 and 4. OpenGL: Gpu::samples gives 1.
