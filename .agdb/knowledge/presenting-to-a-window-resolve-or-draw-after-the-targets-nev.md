---
type: Decision
title: "Presenting to a window: resolve or draw after the targets, never resolving the scene into the surface"
description: "Why mg-render's present path (present::Presenter, Screen) puts the picture into a surface texture after render_targets, by a no-draw resolve pass or a texel-copy draw, and not by making the surface the scene's resolve target or by copy_texture_to_texture."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T13:11:23Z }
touches: [crates/mg-render/src/present.rs, crates/mg-render/examples/window.rs]
asks:
  - How does the renderer present to a window surface?
  - Why not resolve the scene straight into the surface texture?
  - Why does presenting not use copy_texture_to_texture?
  - How are sRGB surfaces handled when presenting?
fingerprint: v2-b7a8441418df92fa
created: 2026-11-19
author: augustaugur
---
Decided 2026-11-19 (upstream request A6 from Moonglow Client).

- The scene keeps resolving into Targets.color; the present is a separate last step. Resolving the scene straight into the surface would leave color_view stale: the watch reads targets.color_view (watch.rs look), callers draw overlays into resolve_view after render_targets (Moonglow Viewer's overlay.rs), and draw_scene resolves at the end of several passes.
- Way::Resolve (multisampled targets of the surface's format): a render pass with no draws, msaa_view loaded (StoreOp::Store kept, so later readers still see it) and resolve_target = the surface view; it does resolve at the end with no draws (checked on Vulkan by the pixel test).
- Way::Draw (one sample, as on GL where Gpu::samples is 1, or another format): a full-screen triangle that textureLoads color_view, alpha forced to 1. Not copy_texture_to_texture: a surface promises only RENDER_ATTACHMENT, and the formats may differ (BGRA vs RGBA).
- sRGB: the renderer writes gamma-space values, so Screen picks a non-sRGB surface format first; an sRGB-only surface gets the values decoded in the draw (round trip within 1 step of 255, tested).
- Screen::targets_format is the surface's own format when it is 8-bit RGBA/BGRA, so the 4-sample case resolves straight in.
- Tests use a stand-in texture with RENDER_ATTACHMENT | COPY_SRC only; Screen's surface-free part (Fit) is tested headless. On X11 (off-screen display) both Vulkan and GL surfaces offer Bgra8Unorm.
