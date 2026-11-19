---
type: Gotcha
title: "Renderer on wgpu's OpenGL back end: three causes and their guards"
description: "Why mg-render failed 37-40 of its 93 picture tests on WGPU_BACKEND=gl and what keeps each cause out: depth read as float, storage elements laid out by std430, one sample a pixel on GL (Gpu::samples)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T12:47:35Z }
touches: ["crates/mg-render/src/gpu.rs:samples", "crates/mg-render/src/framebuffer.rs:DEPTH_READ", crates/mg-shader/src/layout.rs, "crates/mg-shader/src/link.rs:split_module", "crates/mg-render/tests/render.rs:moonglow_s_own_shaders_are_written_in_glsl_as_opengl_writes_them"]
asks:
  - Why does the renderer draw black or nothing on OpenGL?
  - Why is MSAA off on the GL backend?
  - Why are draw values padded to 16 bytes in split_uniforms?
  - Why do the shaders bind depth as float instead of texture_depth_2d?
fingerprint: v2-52ca85bf0c14962e
created: 2026-11-19
author: augustaugur
---
Measured 2026-11-19 on Mesa radeonsi (iGPU, the one EGL gives), wgpu/naga 30.0.1. Finding: moonglow-client docs/research/opengl-fallback.md.

1. Depth read by textureLoad: naga's GLSL writer makes a depth texture a shadow sampler and refuses texelFetch ("WGSL `textureLoad` from depth textures is not supported in GLSL"). The four readers (framebuffer copies, shadow darkening, volume depth copy, watch) declare `framebuffer::depth_read_type` and bind `framebuffer::DEPTH_READ` (float, not filterable). Shadow maps keep texture_depth_2d: they are sampled with comparison, which GLSL has. Guard: render.rs writes every `wgsl_sources()` module in GLSL (no GPU needed).
2. naga's GLSL writes storage buffers as `layout(std430)` with no offsets or strides, so the driver lays them out itself. split_module lays the draw's and lights' elements out by std430 (layout::relayout for arrays and nested structs) and pads with u32 `mg_unused` words to the 16-byte span; split_uniforms refuses any program whose blocks differ (layout::module_differs, uniforms by std140). 7 of 65 stock programs had elements that differed (fsc, fsc_ls, fsfbgam, fsfblvls, fsfbshrp, fsfbssao, fsparticle: tails only). Guard: mg-shader unit tests and the corpus shader test (splits by mg_render::uniform_part too and writes GLSL).
3. wgpu-hal 30's GL back end never makes GL_TEXTURE_2D_MULTISAMPLE: a multisampled texture that is also TEXTURE_BINDING gets glTexStorage2DMultisample on GL_TEXTURE_2D, GL_INVALID_ENUM, silently (only MESA_DEBUG=1 shows it); everything drawn into it is lost. Gpu::samples(wanted) returns 1 on GL; Renderer::new, Targets::new, watch::Sampler::new ask it; a caller's own pipeline into the targets must too (Targets::samples). Only a run with WGPU_BACKEND=gl catches this one coming back. If wgpu-hal gains multisampled textures (C1 upstream), Gpu::samples is the one place to change.
