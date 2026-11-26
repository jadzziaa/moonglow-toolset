---
type: Decision
title: "Instanced selection boxes in mg-render (Scene::boxes)"
description: "How Scene::boxes draws the area view's boxes on the GPU: unit-cube transform, screen-pixel quads, near-plane cut, two draws, and the traps met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T19:45:49Z }
touches: [crates/mg-render/src/line.wgsl, crates/mg-render/src/renderer.rs, "crates/mg-render/src/scene.rs:BoxCopy"]
asks:
  - how are selection boxes drawn by the renderer
  - where does the box shader get the viewport width
  - why is a WGSL vertex input not named from
  - "how do I measure a line's pixel width in a render test"
fingerprint: v2-0691d8c1e982f46d
created: 2026-11-26
author: augustaugur
---
## Decided
- BoxCopy.transform maps the unit cube 0..1 (box_corners' corner order) to the posed box: model * translate(min) * scale(max - min). Line and optional edge each (width px, gamma RGBA straight alpha); edge width is total, centered under the line (egui's BOX.0 + 2).
- Drawn in line.wgsl (vs_box_edge, vs_box_line, fs_box) so the naga/Metal/HLSL shader tests cover it; last in the scene's final pass, depth compare Always, ALPHA_BLENDING, the line pipeline's sample count and format. All edges by one instanced draw, then all lines by another; copies without an edge collapse to a point in the edge draw (order of lines kept). Empty boxes: no upload, no bind.
- Near plane cut in clip space at z = 0 (perspective_rh, wgpu depth 0..1); output w = 1 so the feather interpolates linearly on screen. Butt ends, feather 0.5 px past each end, as egui's line_segment.
- Viewport width: Frame.light_params.z (was unused); w is the height.
- Drawn.boxes and Drawn.box_draws count them (ViewKept is not per-frame stats).
## Watch out
- 'from' is a reserved word in WGSL: naga refused a vertex input named so.
- Measuring widths: sum each pixel's channel across a row (coverage), not a threshold; with a deep box seen face on so the front and back faces' edges don't overlap.
- The ignored boxes_cost test prints encode and frame time for 5,000 boxes.
