---
title: "The game's lit shaders drawn (M1): clip space, inputs by name, the oracle"
author: August
date: 2026-10-21
branch: shaders-2
touches:
  - crates/mg-shader/src/link.rs
  - crates/mg-render/src/game.rs
  - crates/mg-render/src/renderer.rs
  - crates/mg-render/tests/render.rs
  - crates/mg-corpus-tests/tests/client_render.rs
  - crates/mg-corpus-tests/tests/common/mod.rs
  - docs/shaders-proposal.md
  - docs/research/notes_shaders.md
  - docs/deferred.md
---

## Decided
- Clip space: upload the engine's OpenGL matrices (Camera::projection_gl) and remap depth in IR as each vertex program's position leaves (z = (z+w)/2, mg-shader link.rs gl_depth_to_wgpu). No y flip: naga's adjust_coordinate_space is OFF (it is for Vulkan-authored SPIR-V; GL and wgpu clip space are both y-up).
- Vertex inputs: build the wgpu VertexBufferLayout from the reflected locations by the engine's names (game.rs attribute()); fHandedness = tangent+12 as its own Float32 attribute; vIndex converted u32->float in IR (Program::read_input_as_uint) because wgpu only checks scalar KIND and no format turns u32 into floats; unknown inputs (vColor...) from a zero buffer with Instance step.
- Fragment resources moved to bind group 1 (both stages bind their own block at binding 0); each block at a dynamic offset into one per-frame buffer.
- Game path = Opaque pass only (Renderer::game_shaders, off). Draws are never instanced (GLSL has no instance index for the order buffer).
## Tried and ruled out
- Uploading wgpu-style 0..1 projection as m_proj: works for vslit but shaders using m_proj_inv/clipParams would get the wrong depth range.
- Assuming glslang numbers by declaration: it numbers each stage's varyings in the order main FIRST USES them; a test must use them in different orders or it proves nothing (it once passed without link_inputs).
- Explaining the _sm/_nm oracle gap by per-vertex env coords: at SHADER_QUALITY_MODE 2 the stock shader computes them per fragment. The real cause: texture2D(env, uv, lodbias) is a BIAS added to the derivative level before clamping; shader.wgsl uses it as an absolute level. Experiment with the bias: _sm 0.73/15 -> 0.02/1 (mean/p99 of 255). Fix deferred (changes the app's look).
## Watch out
- TestAssets::solid textures have alpha -> Blend pass -> never the game path. Use opaque (into_texture(false)) textures.
- Background must be masked with an exact colour (magenta); a 0.1 clear is not exactly 26.
- Mutation-check the oracle: zeroing fogColor in the game path moves the mean to 9-15, proving the stock path draws.
- Client time test: winshot can return a stale frame (same sceneCurrentFrame twice): drop repeats. worldtimerTimeOfDay = real ms; its origin follows Mod_MinPerHour, not hour/day (unexplained). Extracting all TDoD haks into the scratchpad filled /tmp's task space: grep the existing scratchpad extracts instead.
