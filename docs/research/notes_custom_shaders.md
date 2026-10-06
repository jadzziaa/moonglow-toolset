---
type: Research Note
title: "Running a tileset's own shaders: where the work stands (tabled)"
description: Findings and a staged plan for a Shaders switch that draws meshes with the custom GLSL their materials name - what blocks compiling the game's shaders with naga, what the earlier glslang spike showed, and what is needed to start. Nothing is built; the work is tabled.
tags: [rendering, shaders, custom-content, plan]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-06T19:40:00Z }
sources:
  - { id: notes_shaders, resource: notes_shaders.md }
  - { id: plan, resource: ../PLAN.md }
status: draft
---

# Running a tileset's own shaders: where the work stands (tabled)

Asked for on 2026-10-06 after a builder's report (a custom tileset's lava,
which moves in the game by a shader of its own, stands still in Moonglow)
as a **Shaders** switch on the area view's toolbar. Tabled the same day,
before any code. This note keeps what was found so the work can be picked
up from here. It lives on the `shaders` branch only.

## What the request needs

Moonglow draws with its own WGSL shaders ([the plan](../PLAN.md) §5.5). A
material (MTR) can name `customshaderVS`/`customshaderFS`: GLSL `.shd`
resources. Such a shader almost always `#include`s the game's own files
(`inc_standard` and what it pulls in) and calls into them, so drawing with
it means compiling the game's shader code too, and giving it the values the
engine gives ([the shader notes](notes_shaders.md) §A.2, §B, §C): the
preamble of `#define`s, the transforms, time, lights, fog and textures.
The game's shaders are game assets: read from the user's install at run
time, never shipped.

## What was found

- **No example here.** None of the 51 Workshop materials on the development
  machine names a custom shader, and no `.shd` is in the Workshop items or
  the override. The builder's lava tileset is needed (asked: which one).
- **The stock lit fragment shader, assembled** (`tools/shaders/assemble.py
  fslit frag`, the engine's preamble and includes spliced in), is about
  6,700 lines with about 300 `uniform` declarations outside any block, many
  repeated (the includes are guarded by `#ifndef`, so the repeats go once
  the conditionals are resolved) and many inside `#if` blocks.
- **naga's GLSL front end** (in the build already through wgpu; its
  `glsl-in` feature needs one more crate, `pp-rs`, not yet fetched) takes
  Vulkan-style GLSL: uniforms in blocks, explicit bindings and locations.
  The game's GLSL 330 has none of that. So a pass of Moonglow's own is
  needed before naga: splice includes, resolve the conditionals with the
  preamble's values, gather the loose uniforms into a block, give samplers
  bindings and the stage inputs and outputs locations (attributes by the
  names in the shader notes §B.1, varyings matched between the stages by
  name).
- **The other route** is the earlier spike's (the plan §5.5, 2026-09-30):
  glslang under its relaxed Vulkan rules compiles the stock `vslit`/`fslit`
  pair, and naga accepts the SPIR-V once SPIRV-Tools splits the combined
  image samplers and the bindings are rewritten. It works, and costs native
  glslang and SPIRV-Tools in every platform's build. `glslangValidator` is
  on the development machine for comparing against.

## A staged plan

1. **Spike the pure-Rust route.** A corpus test that runs every stock
   shader (92) and a small authored test shader through the pass and naga,
   and counts what compiles. It says quickly whether the route holds or the
   glslang one is needed.
2. **Draw with them.** `mg-render`: a pipeline per shader pair and vertex
   layout; a uniform block filled by name from naga's reflection (unknown
   names zero); textures by slot. First the transforms, time, textures and
   fog, then the lights. `mg-area`: meshes whose material names a custom
   shader use it when the view's **Shaders** switch is on; a shader that
   doesn't compile, and the switch off, draw as now (the game draws magenta
   for a compile failure).
3. **Compare with the game** on the lava example
   (`client_render.rs`'s way).

## Open

- Whether naga's GLSL front end takes what the stock includes use once the
  declarations are rewritten (not tried: needs `pp-rs`).
- How much of the engine's uniform set a typical custom shader reads
  (needs examples).
- The engine's time uniforms' units and what drives them, for anything that
  scrolls (the shader notes §B.4).
