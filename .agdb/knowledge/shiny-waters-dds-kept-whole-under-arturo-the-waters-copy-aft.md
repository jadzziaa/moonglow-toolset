---
type: Fact
title: "Shiny water's DDS kept whole under arturo; the water's copy after see-through meshes"
description: "The fountain/shallow-pool gap was the rippling procedure's gray (eba64fd5) wiping shiny water's clarity alpha; the client copies the framebuffer once after see-through meshes, solid readers get the frame before"
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T06:49:33Z }
asks:
  - Why did a shallow pool hide its floor and shadow in Moonglow but not in the client?
  - "What do the client's texFBColor and texFBDepth copies hold, and when are they made?"
  - Why is there no second pair of framebuffer copies for the frame before?
  - Why does procedure_picture skip textures with bumpmaptexture shinywater?
created: 2026-11-26
author: augustaugur
---
fswater reads the smallest mip's alpha as the water's clarity (transparency exp2(-depth*0.25*a/(1-a)), 0 at a=1). procedure_picture (renderer.rs, eba64fd5) turned every arturo DDS into the opaque LOST_RIPPLE_GRAY, so ttr01_water01 and TDoD's fountain water went opaque: the pool darkened 0.002 under a shadow where the client's darkens 0.26. The client draws shiny water's DDS whole (read with patched fswater in user/override: client_render.rs MG_FOUNTAIN_OVERRIDE, shadows.rs MG_SEE_THROUGH_OVERRIDE), so shiny water is excluded; after it the pool darkens 0.264. Copies (copies::reading_copy_order, MG_COPY_ORDER=color|depth|particles): one copy a frame after alpha/hint/DXT5 see-through meshes, before water and sample_framebuffer meshes; solid and transparency-1 readers get the previous frame's final picture (additives in it); particles get depth without water. Moonglow now marks see-through game draws that read the copy as late, sorts them after other see-through solid parts and copies at the first late or Fringe/Additive draw. The frame-before pair is not built: no stock or TDoD program reads it that way, 40 MB at 3440x1440 per view, and one-shot pictures have no frame before (deferred.md Settled).
