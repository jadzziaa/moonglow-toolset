---
type: Proposal
title: 'The game''s own shaders in the area view: a proposal for 2.0'
description: Drawing areas, blueprints and models with NWN:EE's own GLSL shaders (the stock ones from the install, a hak's overrides of them, and the custom shaders MTRs name) as the centerpiece of Moonglow 2.0 - what the tabled shaders branch held, what the renderer is today, the corpus now on hand, a spike in which all 102 programs reached valid wgpu shader modules, the route (glslang, SPIRV-Tools, naga with two fixups), the uniform contract, costs, milestones with their tests, and the owner's decisions (accepted 2026-10-21 - the game's shaders throughout, native glslang, on by default, a failed program falls back and is reported).
tags: [proposal, rendering, shaders, custom-content, renderer, release-2]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-21T13:11:00Z }
stale_after: 2027-04-01T00:00:00Z
sources:
  - id: request
    resource: human:august (2026-10-21)
    title: The request - examine the old shaders branch, an angle of attack for proper shader support as 2.0's centerpiece
  - id: decisions
    resource: human:august (2026-10-21)
    title: The owner's answers to the five decisions
  - id: branch-note
    resource: research/notes_custom_shaders.md
    title: The shaders branch's note (2026-10-06, tabled)
  - id: notes-shaders
    resource: research/notes_shaders.md
    title: NWN:EE rendering and lighting pipeline, read from the stock shaders
  - id: plan
    resource: PLAN.md
    title: The architecture, §5.5 Renderer
  - id: tdod-haks
    resource: ~/Projects/tdod-module/build/hak (tdod_t_tap, tdod_p_tap, tdod_p_ham, tdod_heads, tdod_shaders)
    title: The Domains of Dread's haks - 37 custom shaders and an override of inc_lighting
  - id: spike
    resource: tools/shaders/spike.py and tools/shaders/nagacheck, run 2026-10-21 (game 89.8193.37, glslang 16.6.0, SPIRV-Tools spirv-opt, naga 30.0.1)
    title: The compile spike
---

# The game's own shaders in the area view: a proposal for 2.0

Status: accepted on 2026-10-21 (see [the owner's decisions](#the-owners-decisions)); nothing is built yet.

Moonglow draws with one WGSL shader of its own that approximates EE's
lighting ([the plan](PLAN.md) §5.5). This proposes that 2.0 draws with
**the game's own shaders**, read from the user's install and haks and
compiled on the user's machine: the stock programs, a hak's overrides of
them, and the custom shaders that materials name. Moonglow's own shader
stays as the fallback. A spike on 2026-10-21 took every stock program and
every custom shader on this machine, 102 in all, to a valid wgpu shader
module.[^spike]

## Why it is the 2.0 story

- **Custom shaders move nothing in Moonglow today.** Wind-swayed grass,
  leaves and plants, flow-mapped water and lava, roofs that rise and
  sink with the camera, and no-keyhole variants all stand still or draw
  flat. Two shaders are hand-ported by name (`vertexalpha`, `mzlm`, in
  [the deferred list](deferred.md)), and each further shader would need its
  own port.
- **A hak can change every lit mesh in the game.** `tdod_shaders.hak`
  replaces the stock `inc_lighting.shd` with a diffuse-wrap version, so
  every lit surface in The Domains of Dread is shaded differently from the
  stock game.[^tdod-haks] Moonglow's port of the lighting cannot show this,
  and neither can a switch that applies only to meshes with a custom shader
  (the design in the tabled note[^branch-note]).
- **Builders could check their shaders before starting the game.** A
  shader that doesn't compile shows as magenta in the client. Moonglow
  could name the file and line in Verify.
- **Moonglow's own shader gets an oracle.** With the real `fslit` on the
  same scene, the WGSL port can be compared frame by frame without
  starting the client.

## What was there

**The `shaders` branch** is one commit: the note that tabled the work on
2026-10-06, before any code.[^branch-note] It had found no example shader
on this machine. It weighed naga's GLSL front end (pure Rust, but it needs
a rewriting pass first and was never tried) against the 2026-09-30 spike's
glslang route. Its plan was three steps: count what compiles, draw with
it, compare with the game.

**The renderer today** (`crates/mg-render`, about 6,700 lines):

- One lit WGSL shader (`shader.wgsl`, 659 lines). It ports `vslit`/`fslit`,
  `inc_standard`, `inc_lighting` and `inc_material`. It has no
  permutations: variants are branches on per-draw flags.
- 24 pipelines are built up front, keyed by (pass, skinned, two-sided).
  All of them share one layout:
  - group 0, per frame: the frame uniform, a storage buffer of lights
    (at most 32 per draw), bones, per-draw uniforms, and the instance
    order;
  - group 1, per material: 12 bindings (texture slots 0–7, the
    environment map and cube, and two samplers).
- Instancing is by the batcher. Everything is drawn in one render pass,
  ordered sky, sky fade, opaque, blend, fringe, additive, then particles
  and lines. The target is `Rgba8Unorm` with 4× MSAA, gamma is applied in
  the shader, and there is no post-processing.
- MTRs are resolved in `Renderer::slots`. `customshaderfs` is matched by
  prefix against `EFFECT_SHADERS` (`vertexalpha`, `mzlm`), and their
  effects become bits in the per-draw uniform. No `.shd` is compiled
  anywhere. `Assets::shader` already reads one, but only to look for a
  `#define`.
- `mg-image`'s MTR parser already reads `customshadervs`, `customshaderfs`
  and `customshadergs`, all 16 texture slots and the parameters.

**The corpus.** The game has 92 stock `.shd` files, 65 of them programs and
the rest includes. On this machine, The Domains of Dread's haks hold 37
custom shaders and one stock-include override:[^tdod-haks]

| Hak | Shaders |
|---|---|
| `tdod_t_tap` | `shd_flowmap`, `shd_mixlite{kn,ky}`, `shd_nokeyhole`, `shd_roof{kn,ky}`, `shd_tooltip`, `shd_wind{grass,leaf,plant}` (vs and fs each, 20) |
| `tdod_p_tap` | `dd_tap_{flow,nokey,wgrass,wleaf,wplant}` (10) |
| `tdod_p_ham` | `dd_ham_{wgrass,wleaf,wplant}` (6) |
| `tdod_heads` | `fs_plt_hair` |
| `tdod_shaders` | `inc_lighting` (overrides the stock include) |

These shaders follow the shape the tabled note expected. Each is a stock
shader copied and edited: it `#include`s `inc_standard` and calls the
stock machinery (skinning, `SetupLights`, `ComputeLighting`, fog,
keyholing). It adds its own uniforms (`fSwaySpeed`, `fSwayStrength`,
`fTilesetMeshOffsetZ`, `fRoofOffsetZ`, `fSlideSpeed`, `vOverrideColor`…),
which the engine fills from the MTR's `parameter` lines. Running one of
them therefore means running the game's includes too.

## The spike: what compiles (2026-10-21)

`tools/shaders/spike.py` assembles each program the way the engine does
(notes §A.2[^notes-shaders]): it writes the engine preamble, then splices
the `#include`s verbatim from a search path in load order (haks first,
then the install). It compiles the result with glslang to Vulkan SPIR-V
under relaxed rules. `tools/shaders/nagacheck` then parses the SPIR-V
with naga 30.0.1, the version wgpu uses, validates it and writes WGSL.

| Step | Result over 102 programs (65 stock, 37 TDoD, with TDoD's `inc_lighting`) |
|---|---|
| `glslangValidator -V -R --amb --aml -S <stage>` | 102 compile |
| naga `spv-in`, as compiled | 61 accepted; 41 fail with "invalid id" (the fragment shaders that sample textures: combined image samplers) |
| after `spirv-opt --split-combined-image-sampler` | all parse; the same 41 fail validation with a binding collision (each split pair shares its binding) |
| after renumbering every resource's binding in naga's IR | **102 validate and write WGSL** |

What it shows:

- Each program has **one uniform block** (glslang gathers the loose
  uniforms into `gl_DefaultUniformBlock`). Its members keep the engine's
  names (`m_mv`, `lightPosition`, `fogParams`, `worldtimerTimeOfDay`,
  `fSwaySpeed`…), so values can be filled by name from naga's reflection.
  The same goes for the samplers (`texUnit0`…, `texUnitEnv`), the vertex
  inputs (`vPos`, `vNormal`, `vTcIn`, `vIndex`, `vWeight`…) and the
  varyings.
- **The blocks are large.** They span 4.6 to 8.8 KB (mean 5.1 KB, across
  90 blocks). Dead-member elimination (`spirv-opt -O
  --eliminate-dead-members`) cuts the member count (`fslit`: 92 to 36) but
  not the span, which the arrays set. Every vertex program reserves 4 KB
  for `m_bones` (64 bones), because `skinmesh` is a run-time branch, and
  the fragment programs carry the light and wind-source arrays.

What it doesn't show: **nothing has been drawn yet.** These are the parts
still to be proved:

- vertex input locations (`--aml` numbers them arbitrarily);
- matching varyings between the stages;
- the coordinate space (the spike set naga's `adjust_coordinate_space`
  without checking it);
- the preamble's other permutations (the spike used `NO_DISCARD 0` and
  `KEYHOLING_ENABLED 1` only);
- the cost per frame.

## The route

```
.shd sources (haks, override, install; load order)
   │  assemble: engine preamble + #include splicing         (Rust, from assemble.py/spike.py)
   ▼
GLSL 330 ──glslang -V -R (relaxed Vulkan rules)──► SPIR-V
   │  split combined image samplers                          (SPIRV-Tools optimizer pass)
   ▼
naga::front::spv ──► naga::Module
   │  fixups in IR, by name:
   │    · resource bindings renumbered: one group, samplers and textures paired by name
   │    · vertex inputs given the locations of Moonglow's Vertex / SkinVertex
   │    · fragment inputs matched to the vertex outputs by name
   ▼
wgpu::ShaderSource::Naga ──► pipeline (cached)
```

One Rust interface takes the assembled source and the preamble and returns
a naga `Module` with its reflection. The compiler behind it can be swapped
later.

**Native glslang and SPIRV-Tools (decided).** This route is proven
end to end. The pure-Rust alternative is naga's `glsl-in`, which needs a
pass of Moonglow's own before it can even be tried: gathering the loose
uniforms into a block and giving the samplers bindings. It stays a
possible later swap behind the interface. The cost is a C++ build in
every package, so the first milestone (M0) proves it on all four
packages: AppImage, Flatpak (which builds offline), the Windows installer
and the macOS app. Which crate to use is not settled. `shaderc-sys`
(google/shaderc-rs) has a `build-from-source` feature, built with CMake.
`glslang` (SnowflakePowered/glslang-rs) needs checking: whether it vendors
its C++ sources, and whether SPIRV-Tools' optimizer comes with it. If the
optimizer has to be left out, the sampler split can be done another way:
in the GLSL pass (only two include functions take a `sampler2D`
parameter), or on the SPIR-V words in Rust.

**Licensing.** glslang (BSD/Apache) and SPIRV-Tools (Apache-2.0) are
GPL-3.0-compatible. Beamdog's shaders are never shipped. They are read
from the user's install at run time and compiled on the user's machine,
and the compiled cache stays in the user's cache folder ([the
plan](PLAN.md) §8).

## Two tiers

- **Tier A, custom shaders.** A mesh whose MTR names
  `customshaderVS`/`FS` draws with that pair. Every other mesh draws as
  now. This replaces the hand-ported `vertexalpha` and `mzlm`, which stay
  as the fallback where the source isn't found. It is the first milestone
  that draws, and on its own it would be a 1.x feature.
- **Tier B, the game's shaders throughout (the 2.0 claim).** With
  **Shaders** on, every mesh draws with the stock program the engine would
  choose for it (`vslit_sm_nm`/`fslit_sm_nm` and the rest; selection as in
  notes §A.4, where open question 12 must be settled), compiled from the
  includes as the load order resolves them. A hak's `inc_lighting`
  therefore changes the view as it changes the game. Moonglow's WGSL
  shader is the fallback for a program still compiling, for one that
  doesn't compile, and with the switch off.

Decided: A is built on the way to B. Both need the same compiler,
pipelines and uniform contract, and a custom shader is just a stock
shader with edits.

## The uniform contract

Values are filled by name from the reflected block, and names the table
doesn't know are zero. The table in notes §B[^notes-shaders] is the
starting point. By group:

| Group | Source in Moonglow | Notes |
|---|---|---|
| Transforms: `m_mv`, `m_mvp`, `m_proj`, `m_view`, `m_vp_inv`, `m_texture`, `m_bones`, `skinmesh` | the scene and camera, skinning | clip-space convention decided by the first triangle test (M1) |
| Material: `materialFront*`, `Specularity`, `Roughness`, `Metallicness`, `CustomSpecularColor`, `fAlphaDiscardValue`, `texture*Bound`, `envMapCube` | what `Renderer::slots` resolves now | colors linearized as notes §M.3 found |
| Lights: `lightPosition`, `lightColor`, `numLights`, `lightArea*`, `lightMaxIntensityInv`, `lightFalloffFactor`, `staticLighting` | `LightGrid::reaching`, the frame uniform | as read back from the client (notes §E.6) |
| Fog: `fogParams`, `fogColor` | the frame uniform | `fogColor` in gamma space |
| Time: `worldtimerTimeOfDay`, `sceneCurrentFrame`, `module*` | a clock of the view's | **units unknown**: measured in M1 |
| Area: `areaFlags`, `areaGlobalWind`, `areaWeather*`, `bboxMin/Max` | the ARE | wind drives the sway shaders |
| Camera and player: `camera*`, `player*`, `keyhole*` | the editor camera; the player at its focus, keyholing off | the editor has no player |
| Wind sources, targeting, `scriptable*` | zero | no in-editor source |
| An MTR's `parameter` lines (`fSwaySpeed`…) | the MTR, by name | the same fill path covers them |
| Samplers `texUnit0..15`, `texUnitEnv`, `texUnitEnvCube`, `texUnitNoise` | the slots as now, by name | |
| `texFBColor`, `texFBDepth`, post-processing | **out of 2.0** | one pass, no post; the material draws without its framebuffer read |

## Costs and their answers

- **Per-draw uniforms.** One block of about 5–9 KB per draw, and the
  current batcher's instancing doesn't apply to it. Re-uploading it for
  3,000 draws would be about 40 MB a frame, far over the 144 fps budget.
  The answer is in two steps:
  - Keep each draw's block in a persistent slice (a dynamic offset into
    one buffer), rewritten only when the draw changes. Tiles and still
    placeables don't change between frames.
  - If that isn't enough, split the block in IR: the frame-constant
    members (projection, fog, time, camera, module, area) go to a second
    binding written once a frame, leaving the draw's own members (`m_mv`,
    material, its lights, bones), which come to about 1 KB without bones.

  Measured in M3 with `frame_perf`.
- **Permutations and compile time.** A pipeline is keyed by (vs, fs,
  preamble defines, pass blending, culling, vertex layout). glslang takes
  tens to hundreds of milliseconds on a 6,700-line program, so an area
  with a hundred programs would cost seconds. The answers:
  - pipelines are built lazily, off the UI thread;
  - a draw uses the WGSL fallback until its program is ready;
  - compiled modules are cached on disk in the user's cache folder,
    keyed by a hash of the assembled source and the preamble.
- **A program that fails.** It draws with the fallback, and Verify reports
  it (`mg verify`: the file, line and glslang's message). It is not drawn
  magenta, as the client draws it (decision 4).

## Milestones

| | What | Proves / tests | Size |
|---|---|---|---|
| M0 | Compiler crate (new, or in `mg-render`): assemble, compile, split, IR fixups, reflection; native build in all four packages through CI on a temporary branch | corpus test: every stock program and every installed hak's compiles (102/102 the baseline), each failure named; CI and packages green | M |
| M1 | One stock pair drawn: `vslit`/`fslit` on a lit mesh, bound by name; clip space, vertex locations and varyings settled; the time uniforms' units measured in the client (a patched `inc_standard` in the scratch override reads them back on two frames a known time apart, as `light_uniforms_match_the_client` does) | render test: the stock `fslit` against the WGSL shader on the same scene within tolerance (the WGSL port's first oracle) | M |
| M2 | Tier A: MTR custom shaders in the area view, models and previews; MTR parameters; the Shaders switch; Verify's check | TDoD's windgrass sways and flowmap slides; `client_render.rs` comparison with the client on them | M–L |
| M3 | Persistent per-draw blocks, the lazy and cached pipelines, off-thread compiles | `frame_perf` at 144 fps on a large TDoD area; open-area time | M |
| M4 | Tier B: the engine's program choice for every mesh (notes §A.4, open question 12 settled in the client), hak include overrides through the load order | screens of TDoD with and without its `inc_lighting`; reference scenes against the client | L |
| M5 | The rest: water (`fswater`), grass and particles through their stock programs, or kept on WGSL with a reason recorded | per surface, the client comparison | M each |
| M6 | Moonglow Viewer and `mgv` take it (they build on `mg-render` by tag) | the viewer's own tests | S |

## The owner's decisions

Decided on 2026-10-21:[^decisions]

1. **Scope of 2.0: tier B**, the game's shaders for everything, with
   tier A on the way.
2. **The compiler: native glslang and SPIRV-Tools.** Which crate brings
   them is settled in M0.
3. **On by default** once M3 meets the frame budget. The switch stays for
   comparing and for a machine where compiling fails.
4. **A program that doesn't compile falls back** to Moonglow's own shader
   and is reported in Verify (file, line, glslang's message). No magenta.
5. **The tabled note** is in `develop` (deprecated, pointing here), and
   the `shaders` branch is deleted.

## Open

- The time uniforms' units and what drives them (M1).
- How the engine chooses `_sm` and plain programs, and when it uses
  `fswater` (notes §M.12; needed for M4).
- How the engine sets `NO_DISCARD` per material (notes §M.10).
- What the toolset itself runs with: `FRAGMENT_LIGHTING`, quality,
  `MAX_NUM_LIGHTS` (notes §M.5). This only matters if Aurora's look is ever
  wanted rather than the game's.
- Whether glslang's relaxed rules accept everything else builders write.
  The corpus here is one module's. The Vault and the Workshop would widen
  it (M0's test runs over whatever haks are installed).

[^decisions]: The owner's answers, 2026-10-21.
[^spike]: The compile spike: `tools/shaders/spike.py`, 2026-10-21.
[^tdod-haks]: The Domains of Dread's haks, read with `mg ls` and `mg unpack`.
[^branch-note]: [The shaders branch's note](research/notes_custom_shaders.md).
[^notes-shaders]: [The shader notes](research/notes_shaders.md).
