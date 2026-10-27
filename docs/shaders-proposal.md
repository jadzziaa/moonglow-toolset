---
type: Proposal
title: 'The game''s own shaders in the area view: a proposal for 2.0'
description: Drawing areas, blueprints and models with NWN:EE's own GLSL shaders (the stock ones from the install, a hak's overrides of them, and the custom shaders MTRs name) as the centerpiece of Moonglow 2.0 - what the tabled shaders branch held, what the renderer is today, the corpus now on hand, a spike in which all 102 programs reached valid wgpu shader modules, the route (glslang, SPIRV-Tools, naga with two fixups), the uniform contract, costs, milestones with their tests, and the owner's decisions (accepted 2026-10-21 - the game's shaders throughout, native glslang, on by default, a failed program falls back and is reported); M0 built - the mg-shader crate (glslang-rs, the sampler split in Rust), its corpus test (102 of 102) and Verify's shader-compile check; M1 built - the stock lit pair and its variants drawn by mg-render beside its WGSL shader (opt-in), matching it within 1 in 255 on plain lit meshes, the WGSL port's first oracle, and the time uniforms' units measured in the client; M2 built - tier A, a material's own program pair drawn in the area view, the model viewer and the previews behind a Shaders switch (off by default), see-through meshes in Moonglow's two parts, the area's wind, tangents made where a model has none, The Domains of Dread's grass, leaves, water and roof shaders moving, and the client's floor matched where its inc_lighting applies; M3 built - the Shaders switch on by default, every view within 144 fps where Moonglow's own shader is (a large stock area zoomed out 5.97 ms on, 3.97 off; a synthetic TDoD area seen whole, over the budget either way, 54 ms on, 29 off), through uniforms split in IR into a draw's, its lights', the frame's and the bones', draws as instances of storage arrays, compiles on a worker thread and a disk cache; M4 built - every mesh drawn with the program the engine chooses for it and its material's NO_DISCARD, read from the client by marking every stock program (36 kinds of mesh, Moonglow choosing as the client for each), the WGSL shader's environment map sampled as the game's (`_sm` 0.73 to 0.00 of 255 from the stock programs), a hak's inc_lighting through the load order (TDoD's area view), and the client's reference scenes compared with the switch off and on; M5 built - the special surfaces read in the client (water vswater/fswater with the tileset's 2D map, grass vsgrass/fsgrass, the skybox vsvt/fst, additive meshes as any mesh, a decal with an envmap vsvt_sm/fst_sm, particles vsparticle/fsparticle by blend) and drawn with the game's programs - the sky at the far plane, grass, additive meshes, particles - but water, kept Moonglow's because fswater without the framebuffer is farther from the client (TDoD's fountain 16.4 of 255 against 11.7); see-through game draws batched as Moonglow's own; M4's macOS failure (a cube map read with gradients) fixed; after M5, the light-dense water view brought back within the budget with the switch on (7.5 to 6.5 ms, off 5.6) by the game's programs reading the lights by their places in the frame's (a SPIR-V pass) and cheaper writing of the draws' values.
tags: [proposal, rendering, shaders, custom-content, renderer, release-2]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-27T08:33:19Z }
stale_after: 2027-04-01T00:00:00Z
sources:
  - id: viewer-0-2-0
    resource: https://github.com/jadzziaa/moonglow-viewer/releases/tag/v0.2.0
    title: Moonglow Viewer v0.2.0, on Moonglow Toolset v2.0.0 with the game's shaders (M6)
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
  - id: crates
    resource: crates.io packages shaderc-sys 0.10.1, glslang 0.9.1, glslang-sys 0.9.2+e1b562a, spirv-tools-sys 0.13.3 (their build.rs and published files), read 2026-10-21
    title: The candidate crates for native glslang and SPIRV-Tools
  - id: m0
    resource: crates/mg-shader, crates/mg-corpus-tests/tests/shaders.rs, crates/mg-module/src/doctor.rs (shader-compile)
    title: M0, the compiler
  - id: m1
    resource: crates/mg-shader/src/link.rs, crates/mg-render/src/game.rs, crates/mg-render/tests/render.rs (the_game_path_draws_opengl_s_clip_space_right, the_game_s_lit_shaders_match_moonglow_s), crates/mg-corpus-tests/tests/client_render.rs (time_uniforms_in_the_client), run 2026-10-21
    title: M1, one stock pair drawn
  - id: m2
    resource: crates/mg-render/src/game.rs, renderer.rs, model.rs (made_tangents), crates/mg-shader/src/link.rs (keep_alpha, reads_uniform), crates/mg-ui (the Shaders switch), crates/mg-module/src/doctor.rs (shader-pair), crates/mg-corpus-tests/tests/game_shaders.rs, client_render.rs (tdod_custom_shaders_look), crates/mg-ui/tests/app/drawing.rs (tdod_s_wind_shaders_move_in_the_area_view), run 2026-10-21 with MOONGLOW_HAKS=~/Projects/tdod-module/build/hak
    title: M2, tier A
  - id: m3
    resource: crates/mg-shader/src/spirv.rs (add_spare_blocks, add_draw_index), link.rs (split_uniforms), cache.rs, crates/mg-render/src/game.rs, renderer.rs, crates/mg-ui/src/settings.rs (GAME_SHADERS_BY_DEFAULT), crates/mg-ui/tests/frame_perf.rs (game_shaders_frames_at_144), crates/mg-render/tests/render.rs (an_authored_glsl_pair_draws_on_this_platform_s_gpu), run 2026-10-21 with MOONGLOW_HAKS=~/Projects/tdod-module/build/hak
    title: M3, the frame budget
  - id: m4
    resource: crates/mg-corpus-tests/tests/client_render/programs.rs (program_choice_in_the_client), client_render.rs (reference_scenes_match_the_client, the look tests off and on, tdod_custom_shaders_look), crates/mg-render/src/game.rs (stock_pair, no_discard), renderer.rs, shader.wgsl, crates/mg-render/tests/render.rs (the_game_s_lit_shaders_match_moonglow_s), crates/mg-ui/tests/app/drawing.rs (tdod_s_inc_lighting_changes_the_area_view_through_the_load_order), frame_perf.rs, run 2026-10-21 on 89.8193.37-17 with MOONGLOW_HAKS=~/Projects/tdod-module/build/hak
    title: M4, the engine's choice
  - id: m5
    resource: crates/mg-corpus-tests/tests/client_render/programs.rs (surfaces_in_the_client), client_render.rs (tdod_custom_shaders_look), crates/mg-render/src/game.rs, renderer.rs, shader.wgsl (cube_level), crates/mg-shader/src/link.rs (at_far_plane), crates/mg-render/tests/render.rs, crates/mg-ui/tests/frame_perf.rs (FRAME_AREA=water), run 2026-10-21 on 89.8193.37-17 with MOONGLOW_HAKS=~/Projects/tdod-module/build/hak
    title: M5, the special surfaces
  - id: lights
    resource: crates/mg-shader/src/spirv.rs (index_lights), crates/mg-render/src/game.rs (fill, push, upload), renderer.rs, crates/mg-render/tests/render.rs (an_authored_glsl_pair_draws_on_this_platform_s_gpu), crates/mg-corpus-tests/tests/shaders.rs, crates/mg-ui/tests/frame_perf.rs (game_shaders_frames_at_144), run 2026-10-21 with MOONGLOW_HAKS=~/Projects/tdod-module/build/hak
    title: After M5, the lights
  - id: wiki-shaders
    resource: ~/.local/opt/neverwinter/wiki/pages/NWN1/106856449-shaders-introduction-editing-a-textures-colour.md, 12027232-mtr.md
    title: 'nwn.wiki: Shaders Introduction (both programs needed), MTR (renderhint makes tangents)'
---

# The game's own shaders in the area view: a proposal for 2.0

Status: accepted on 2026-10-21 (see [the owner's decisions](#the-owners-decisions)).
M0, the compiler, is built ([M0: the compiler](#m0-the-compiler-2026-10-21)), and
M1, the stock lit programs drawn beside Moonglow's shader
([M1: one stock pair drawn](#m1-one-stock-pair-drawn-2026-10-21)), and M2, tier A
behind a **Shaders** switch ([M2: tier A](#m2-tier-a-2026-10-21)), and M3,
the switch within the frame budget and **on by default**
([M3: the frame budget](#m3-the-frame-budget-2026-10-21)), and M4, every
mesh with the program the engine would choose, settled in the client
([M4: the engine's choice](#m4-the-engines-choice-2026-10-21)), and M5,
the special surfaces each drawn the game's way or kept Moonglow's with
the reason ([M5: the special surfaces](#m5-the-special-surfaces-2026-10-21)).
After M5, the lights read by their places brought M5's light-dense view
back within the budget ([After M5: the lights](#after-m5-the-lights-2026-10-21)).
Released in Moonglow Toolset 2.0.0 (2026-10-21). M6, Moonglow Viewer on
2.0.0 with the game's shaders, is released in Moonglow Viewer 0.2.0
(2026-10-22): a Shaders switch in its window (on by default) and
`--no-game-shaders` on `mgv`'s render, turntable, gallery and sheet; its
own cache of compiled programs; the only API change to adapt was
`Scene.environment`. The plan is complete.

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

What it didn't show: nothing had been drawn. These were the parts still to
be proved (all but the last two settled in
[M1](#m1-one-stock-pair-drawn-2026-10-21)):

- vertex input locations (M0 numbers them in glslang's order of declaration);
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
   │  split combined image samplers; locations for inputs    (Rust, on the SPIR-V words)
   │  and outputs
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

**Native glslang (decided); the SPIRV-Tools pass done in Rust (M0).**
This route is proven end to end. The pure-Rust alternative is naga's
`glsl-in`, which needs a pass of Moonglow's own before it can even be
tried: gathering the loose uniforms into a block and giving the samplers
bindings. It stays a possible later swap behind the interface. The cost
is a C++ build in every package, so the first milestone (M0) proves it
on all four packages: AppImage, Flatpak (which builds offline), the
Windows installer and the macOS app.

The crate, chosen in M0 from the crates' build scripts and published
files (2026-10-21):

| Crate | Brings | Why not, or why |
|---|---|---|
| `shaderc-sys` 0.10.1 (google/shaderc-rs), `build-from-source` | shaderc, glslang, SPIRV-Tools, vendored (8 MB) | built with CMake; shaderc's API has neither glslang's relaxed Vulkan rules (`-R`) nor SPIRV-Tools' passes by name, so neither the loose uniforms nor the sampler split can be had through it |
| `spirv-tools-sys` 0.13.3 (rust-gpu) | SPIRV-Tools, vendored, built with `cc`; every optimizer pass is compiled in | its C wrapper's list of passes leaves out `SplitCombinedImageSampler`; a patched copy would be a git dependency, which the Flatpak's offline build refuses (`cargo_sources.py`) |
| **`glslang` 0.9.1 + `glslang-sys` 0.9.2** (SnowflakePowered/glslang-rs) | glslang's C++, vendored (4.5 MB), built with `cc` alone: no CMake, Python or network | **chosen**: a safe API with the relaxed rules (`VULKAN_RULES_RELAXED`); about 40 s of C++ on a cold build. Its `compile` links and generates in one call, so glslang's own I/O mapping (`--amb --aml`) never runs: Moonglow numbers the locations (below) and the bindings (in the IR, as the spike did) |

What SPIRV-Tools' optimizer did in the spike is a pass of Moonglow's on
the SPIR-V words (`mg-shader`'s `spirv.rs`, about 400 lines): each
combined image sampler becomes `NAME_image` and `NAME_sampler` (the
names `spirv-opt` gives them), each function parameter that takes one
two parameters, each load a load of both and an `OpSampledImage`. The
same pass gives every input and output without one a location, which
naga requires. No SPIRV-Tools is built.

**Licensing.** glslang (BSD/Apache) and SPIRV-Tools (Apache-2.0) are
GPL-3.0-compatible. Beamdog's shaders are never shipped. They are read
from the user's install at run time and compiled on the user's machine,
and the compiled cache stays in the user's cache folder ([the
plan](PLAN.md) §8).

## M0: the compiler (2026-10-21)

`crates/mg-shader`, after `mg-image` in [the layering](PLAN.md) §4 so that
Verify (`mg-module`) can use it; `mg-render` takes it in M1. It holds no
wgpu.

- **Assembly** (`assemble.rs`): the engine's preamble as a struct of its
  defines (`Preamble`, the game at its highest settings by default),
  then the program with each `#include "name"` (1 to 16 characters,
  after any indentation, as the engine reads it) spliced in
  verbatim through `Sources`: the resource manager's load order (a
  hak's `inc_lighting` over the game's), or texts by name. Guards are
  the shaders' own; a file nested 16 deep (one that includes itself) is
  an error at its line. A file's bytes are Latin-1, carriage returns
  dropped, and a character outside ASCII becomes one glslang refuses
  where it matters. Every line keeps the file and line it came from.
- **Compile** (`glsl.rs`): glslang to Vulkan 1.0 SPIR-V under relaxed
  rules. Its messages (`ERROR: 0:LINE: …`) are mapped back through the
  splicing to the `.shd` and line; the lines that follow an error
  (`compilation terminated`, the count) are left out. A panic in the
  bindings is caught and is that program's error.
- **SPIR-V** (`spirv.rs`): the sampler split and the locations, above.
  Every index is checked: words it doesn't understand are an error.
- **naga** (`reflect.rs`, `lib.rs`): naga 30 `spv-in`
  (`adjust_coordinate_space` on; the clip space is M1's), every
  resource bound in group 0 (uniform blocks first, then each texture
  with its sampler next to it, then anything else), validated. The
  `Program` carries the module, its `ModuleInfo` and a `Reflection`:
  the uniform block's members by name with offset, type, array length,
  stride and size; the textures by their shader names (`texUnit0`) with
  dimension and the bindings of texture and sampler; the inputs and
  outputs by name with location and type (naga keeps no output's name;
  the SPIR-V pass hands them over).
- **Measured** (release build, this machine): a program compiles in 5
  to 30 ms (the first, with glslang's start, about 30). The 102 in
  0.7 s.
- **Tests.** Unit tests on authored shaders only: an include spliced
  twice behind its guard, line numbers back through the splicing, a
  missing include at its line, the depth limit, the engine's include
  syntax, stage and program detection, a vertex program's block
  (`m_bones[64]` 4096 bytes at stride 64, no members overlapping) and
  attributes, a fragment program's three samplers split and paired with
  a `sampler2D` passed to a function, the preamble deciding what
  compiles, and truncated, binary, empty and unbalanced sources as
  errors, not panics; bad SPIR-V words likewise. The corpus test
  (`mg-corpus-tests`, `formats` group, `shaders.rs`, through
  `mg_testkit::sweep`) compiles every stock program and every program
  in the installed haks (the user's `hak/`, the Workshop's, `data/hk/`
  and `MOONGLOW_HAKS`), a folder's shader haks layered together; where
  they replace a stock file the stock programs are compiled again with
  it. Each failure is named with file, line and message. On this
  machine with `MOONGLOW_HAKS=~/Projects/tdod-module/build/hak`: 65 of
  65 stock programs, and in TDoD's folder 102 of 102 (its 37 and the 65
  stock again over its `inc_lighting`).
- **Verify** (`shader-compile`, in the Models and materials group; `mg
  checks` lists it): the custom content's programs that don't compile,
  those its materials name (`customshaderVS`/`FS`, a missing one
  included), its `.shd` programs, and where it replaces a stock file,
  the stock programs that include it. Each program's first error, at
  the custom file and line it is in; one message found in many
  programs is one finding naming them ("… and 56 more programs don't
  compile (the game draws them magenta)"). Seen on TDoD with its shader
  haks attached to a scratch copy: none; with a line of `inc_lighting`
  broken, one finding at its line, 58 programs.

- **Builds** (2026-10-21, commit `031e7dc` on a temporary branch): CI
  green on ubuntu, windows and macos
  ([run](https://github.com/jadzziaa/moonglow-toolset/actions/runs/39611319085)),
  and the packages built by the Release workflow run by hand: the
  AppImage, the Windows installer and the macOS disk image
  ([run](https://github.com/jadzziaa/moonglow-toolset/actions/runs/39611328333)).
  The Flatpak built offline on the development machine
  (`packaging/flatpak/build-flatpak.sh`, glslang compiled in the
  sandbox), and its `mg checks` lists `shader-compile`.

## M1: one stock pair drawn (2026-10-21)

`Renderer::game_shaders` (off by default; nothing in the UI turns it on
yet) draws a lit, solid mesh with the stock pair the engine would choose:
`vslit`/`fslit`, `_nm` with material maps, `_sm` reflecting an
environment map, `_sm_nm` with both; skinned meshes through the same
programs (`skinmesh`, `m_bones`). The rest draws with Moonglow's WGSL
shader as before, and so does any mesh whose pair fails to compile or
build (reported once: `Renderer::game_shader_failures`, and stderr).[^m1]

**Decided:**

- **Clip space: the engine's matrices, the depth remapped in IR.** The
  uniforms are OpenGL's, as the engine uploads them (`m_proj` with depth
  from -1 to 1, `Camera::projection_gl`), and `mg-shader` gives every
  vertex program's position `z = (z + w) / 2` as it leaves, which is what
  `glDepthRange(0, 1)` makes of it (`gl_FragCoord.z` is the same in
  both). Nothing is flipped: OpenGL's clip space has y up, as wgpu's has,
  so naga's `adjust_coordinate_space` (for SPIR-V written for Vulkan) is
  off. Uploading wgpu's projection instead would have needed no IR, but a
  shader that works with `m_proj`, `m_proj_inv` or `clipParams` (water,
  depth reconstruction) would then compute with the wrong depth range.
  The triangle test draws a mesh at OpenGL depth -0.9 (clipped without the
  remap), the right way up, the nearer of two in front.
- **Vertex inputs: the buffer layout from the reflection, by name.**
  glslang numbers a stage's inputs and outputs in the order the stage
  first uses them (in `vslit`: `vWeight` 0, `vIndex` 1, `vPos` 2…), so
  each reflected input is given an attribute at its own location from
  Moonglow's `Vertex` by the engine's name (notes §B.1): `vPos` (w filled
  as 1), `vNormal`, `vTcIn`, `vTangent` and `fHandedness` (the tangent's
  fourth component, an attribute of its own), `vCustomColor` (the model's
  `colors`), `vTcIn1`, `vTcIn2` and `vTcIn3` (the model's `tverts1` to
  `tverts3`, zeros for a set it doesn't have, as the client feeds them;
  `vTcIn2` and `vTcIn3` were zeros for every mesh up to 2.5.3); `vIndex`
  and `vWeight` from `SkinVertex`. wgpu checks only the
  scalar kind of a vertex input, but `vIndex` is a float `vec4` and no
  vertex format reads `u32`s as floats: `Program::read_input_as_uint`
  changes that argument's type and converts it in the entry point. What
  Moonglow has no data for (`vColor`, the engine's baked static light,
  with `staticLighting` 0; `vStaticLightDir`; the bones of an unskinned
  mesh) reads zeros from a buffer of its own (one instance).
- **Varyings matched by name** (`Program::link_inputs`): the fragment
  program's inputs take the locations of the vertex program's outputs of
  the same names and types. Unmatched, a same-typed pair swaps silently;
  the test has one (white and black: wgpu sees nothing wrong).
- **Two bind groups:** each program binds its own uniform block at
  binding 0, so the fragment program's resources move to group 1
  (`Program::set_group`). Each group is its block at a dynamic offset into
  one buffer of the frame's blocks, then its textures and samplers by
  name (the vertex programs declare `texUnit0` and the environment maps
  too). Bind groups are kept per pipeline and material.
- **The uniform contract** (`game.rs`, [the table](#the-uniform-contract)):
  transforms (`m_texture` the identity, not zero: `mat2(m_texture)` is
  applied to every UV), material colors linearized, `texture*Bound`,
  `fAlphaDiscardValue` 0.2, the lights as `LightGrid::reaching` chooses
  them (view-space positions, ±cutoff² as alpha), the area light, the
  attenuation constants, fog, camera and the player at its focus, the
  time. An MTR's `parameter`s fill members of their names; any name the
  contract doesn't know is zero. Samplers: `texUnit0..7` from the
  material's slots, `texUnitEnv`/`texUnitEnvCube` from its environment
  map (white where none), `texUnitNoise` from `solid_noise`, the rest
  white.
- **The preamble:** the game at its highest settings, keyholing compiled
  out (the editor has no player), `NO_DISCARD 0`.
- **Faithful to the engine where the proposal didn't say:** a skinned
  mesh draws through the same programs (the engine's skinning is a
  uniform, not a permutation); a mesh of more than 64 bones falls back
  (the engine's limit).

**Measured** (`the_game_s_lit_shaders_match_moonglow_s`, the WGSL port's
first oracle; 320², per channel of 255, over the pixels either draws;
pixels at 255 left out, which the game's post pass would tone down and
Moonglow's shader clamps as it does):

| Scene | Game draws | Mean | 99% within | Most |
|---|---|---|---|---|
| textured sphere and ground, area light, point light, fog (`vslit`/`fslit`) | 2 of 2 | 0.05 | 1 | 1 |
| tile `tcn01_a01_01` | 17 of 25 | 0.12 | 1 | 1 |
| placeable `plc_a01` | 12 of 12 | 0.11 | 1 | 1 |
| creature `c_wolf`, skinned | 24 of 24 | 0.01 | 0 | 49 (one pixel: a depth tie) |
| sphere reflecting an environment map (`_sm`) | 1 of 1 | 0.73 | 15 | 23 |
| sphere with normal and specular maps and tangents (`_nm`) | 1 of 1 | 1.11 | 11 | 31 |

The plain lit path matches the stock shaders within rounding. The two
variants differ where the environment map shows, and the oracle says why:
`inc_envmap` samples it with `texture2D(texUnitEnv, uv, bias)`, the
roughness's `clamp(r·30 − 1, 0, 10)` added to the level the derivatives
give, where Moonglow's shader samples at that level outright. With the
bias applied as GLSL applies it, `_sm` came to 0.02 and 1 (and the plain
scenes to 0.00). The port's fix is in [the deferred list](deferred.md);
the test holds the two variants to a looser tolerance until then. A
uniform zeroed in the game path (the fog color) moves the plain scenes'
mean to 9–15: the comparison sees the stock path, not a copy of
Moonglow's.

**The time uniforms** (`time_uniforms_in_the_client`, `client_render.rs`,
by hand): `worldtimerTimeOfDay` counts real milliseconds (999–1000 a
second in modules at 2 and 10 minutes an hour) and `sceneCurrentFrame` the
client's frames (54–55 a second on the off-screen display). Moonglow's
clock fills them at 1000 and 60 a second from the scene's time. The
timer's origin follows the module's minutes an hour, not its hour or day
(notes §B.4), and is open; the stock and TDoD shaders use the timer as a
phase or as seconds.

**Tests:** `mg-shader`'s on authored shaders (the depth remap's
expressions, varyings used in another order and matched, the bone
indices' type, the group); `mg-render`'s name mapping and block filling on
authored GLSL (matrices by column, arrays at their stride, ints, an MTR's
float and vector parameters by name in any case, an unknown name zero); on
the GPU, authored stand-ins for `vslit`/`fslit` (clip space, depth order,
the varying swap, the texture by name, and the same coverage as the WGSL
shader) and a program that doesn't compile (drawn as without the game's
shaders, reported once over two frames); the comparison above with the
install. The corpus still compiles 65 of 65 stock programs and TDoD's 102
of 102 with the depth remap.

What M1 leaves for later is in [the deferred list](deferred.md) ("The
game's lit shaders drawn"): see-through, additive, sky, decal and water
meshes stay WGSL; no instancing and blocks rewritten every frame (M3);
`NO_DISCARD` per material and `gl_FragCoord.y`'s direction; values the
editor has no source for; Windows and macOS not run.

[^m1]: M1: `crates/mg-shader/src/link.rs`, `crates/mg-render/src/game.rs`, the tests in `crates/mg-render/tests/render.rs` and `client_render.rs`, run 2026-10-21.

## M2: tier A (2026-10-21)

With **Shaders** on (the area view's toolbar and the model viewer: one
setting, `Settings::game_shaders`, for the renderer every 3D view shares,
thumbnails and blueprint previews among them), a mesh whose material names
a program pair draws with it, and every other mesh the game path takes
draws with M1's stock pair. Off by default: `GAME_SHADERS_BY_DEFAULT` in
`mg-ui`'s settings is the one constant the owner's decision (on once M3
meets the frame budget) changes; a builder's own choice is kept as
`Some(bool)` and survives that change.[^m2]

**What draws and moves** (The Domains of Dread's haks, `MOONGLOW_HAKS`;
pictures looked at, before and after):

| Content | Shader | Seen |
|---|---|---|
| `dd_tap_valete577` (grass placeable) | `dd_tap_wgrass_vs/fs` | sways: 2,481 pixels differ between 1.0 s and 1.7 s with the switch on, 0 off |
| `dd_tap_stron1030`'s falling water | `dd_tap_flow_vs/fs` (flow map in `texture6`, `fSlideSpeed`) | slides: 2,658 of its 4,190 pixels differ between 1 s and 2 s, 0 off |
| a stock crate under an authored MTR | `shd_roofky_vs/fs` (no TDoD material names it) | shown to a camera at 3 m in an interior area (18,808 pixels), hidden at 9 m and outdoors (0) |
| `mst04` tiles 42 and 48 in the area view | `shd_windleaf`, `shd_windplant`, `shd_windgrass` | 150 of 300 meshes read the clock; 9,029 pixels move over half a second on, none off; the tiles' grass, kept under the walkmesh by the tileset, raised by `fTilesetMeshOffsetZ` as in the game |

**Decided:**

- **A pair or neither.** A material that names `customshaderVS` and
  `customshaderFS` draws with them; one that names one alone draws with
  the stock pair the engine would choose. The wiki says both are
  needed,[^wiki-shaders] and the data agrees: 250 of TDoD's materials
  (217 armor parts in `tdod_fbr`) name `fslit_nm`, a fragment program,
  alone as `customshaderVS`; taken alone it would be compiled as a vertex
  program and drawn magenta, which the game doesn't. A program named for
  the other stage within a pair (it declares `SHADER_TYPE` of the other)
  falls back and is reported, as does a material naming a geometry
  program (`customshaderGS`: wgpu has no such stage). Verify follows:
  `shader-compile` compiles a pair's programs only, and a new
  `shader-pair` warning names, once a hak, the materials that name one
  alone (it had compiled `fslit_nm` as a vertex program and called it
  magenta).
- **What the game path takes.** A material's pair is drawn whatever
  Moonglow's own shader would do of its own (decal, ripples, water, a
  known effect); the stock pair as in M1. Both in the solid pass (a
  cut-out among them) and the see-through one; not additive meshes, the
  sky and its fade, Moonglow's markers, the debug view, skins of more than
  64 bones, or an instance drawn see-through (a blueprint about to be
  placed lays its depth first, which the game's programs don't do).
- **See-through meshes in Moonglow's two parts.** The engine draws a
  blended mesh once; Moonglow draws its nearly opaque part (alpha 0.95 and
  up) with depth, then the rest without, sorted with the other see-through
  draws, which kept plants' soft edges from hiding the ground. A game
  draw is split the same way: two pipelines of one program pair, the
  fragment program given a discard on its color's alpha as it leaves
  (`Program::keep_alpha`), the same uniform blocks for both. Each is a
  draw of its own among `Renderer`'s see-through draws.
- **`NO_DISCARD` stays 0** for every material, and `fAlphaDiscardValue`
  0.2 (read back from the client for cut-outs too, `cutouts_look`). The
  engine's choice per material is still not known (notes §M.10); 0 is the
  superset (a discard that never fires on an opaque texture changes no
  pixel), 1 would only keep early depth and double the programs.
- **Tangents made where a model has none**, from its first texture
  coordinates (`model::made_tangents`), as the engine makes them for a
  material with a render hint:[^wiki-shaders] the stock normal-mapped
  programs normalize the tangent as it comes, and TDoD's grass (a binary
  model without tangents, its MTR asking for normal maps) drew black.
  Moonglow's own shader keeps the derivative frame for those meshes (a
  flag in the draw), so its look is unchanged. A mesh without tangents
  and without a render hint (the model's or the MTR's) gets zeros on the
  game path (`PipelineKey::tangents`), as in the client, where a custom
  pair's normal map then draws it black (after 2.5.3; made tangents
  until then).
- **The clock.** `worldtimerTimeOfDay` and `sceneCurrentFrame` from the
  scene's time as M1 has them (1000 and 60 a second). The area view
  already draws its picture again every `ANIMATION_STEP` (0.04 s) while
  it is on screen, so the shaders move at that rate; the model viewer's
  clock runs while it plays, and it asks for a frame every 0.04 s while
  a drawn program reads the clock (`Drawn::timed`). A program reads it
  when its IR reads the member (`Program::reads_uniform`): glslang keeps
  every declared uniform in the block, and every stock program declares
  the timer.
- **The area's values.** `areaFlags` is the ARE's `Flags`;
  `areaGlobalWind` from its `WindPower` as (0, 0, 0), (0.707, 0.707, 0),
  (1.414, 1.414, 0) (TDoD's wind shaders' comments give those for the
  toolset's three; their lengths squared, 1 and 4, are notes §B.4's);
  `areaWeatherType` clear (the game rolls the weather from the ARE's
  chances as it runs). The model viewer's area is outdoors, without wind.
- **Programs and pipelines** are kept for the session by name and stage
  (one preamble), pipelines by pair, skin, sides and pass; Reload
  Resources forgets them. A failure is said once in the log pane (and on
  stderr), and again after a reload if it still fails.
- **Fallbacks.** `vertexalpha` and `mzlm` stay as the hand ports where
  their source isn't found (NWN Mapper ships none of `mzlm`'s: unreported)
  and with the switch off; with it on and the source there, their own
  programs draw.

**Measured against the client** (`tdod_custom_shaders_look`,
`client_render.rs`, by hand on the off-screen display; the reference
room with TDoD's fountain five meters north of the player,
`tdod_shaders`, `tdod_p_tap` and `tdod_top` linked into the scratch user
folder's `hak`):

| Region | Client | Shaders off | Shaders on |
|---|---|---|---|
| centre floor (mean RGB) | 73.5, 68.8, 64.0 | 87.9, 82.2, 76.7 | 73.4, 69.0, 64.8 |
| left floor | 36.2, 33.4, 31.8 | 39.0, 35.9, 34.2 | 36.6, 33.9, 32.6 |
| far floor | 55.0, 49.1, 46.6 | 55.2, 49.2, 46.8 | 54.0, 48.5, 46.4 |
| the fountain's 7,082 pixels (mean difference of 255) | | 13.8 | 11.7 |

The floor is the stock `tic01` tile under TDoD's `inc_lighting`: the
game path draws it as the client does, Moonglow's port of the stock
lighting can't. What is left on the fountain is its basin's water
(`ttr01_water01`, the game's shiny water: `fswater` with the framebuffer
in the client, see-through to the basin's floor; Moonglow's WGSL water,
blue) and the falling water's phase (the client's timer's origin).
Blueprint previews drawn by the stock programs match Moonglow's own
within 0.15 of 255 on average (a creature in robe and cloak, a sword, an
armoire).

**Tests:** `mg-shader` (`keep_alpha`, `reads_uniform`), `mg-render` (the
pair or neither, `custom_pair`; the area's flags, wind and MTR floats,
vectors and ints in the block; made tangents; on the GPU, authored stand-ins
for a custom pair: drawn with their parameters and moving with the clock,
still with the switch off; one program alone drawn with the stock pair; a
program of the wrong stage reported once; `vertexalpha` by its hand port
without source and by its programs with; a see-through mesh's two parts,
mutation-checked: drawn as one part, the water under the ground shows
through the plant's edge), `mg-preview`, `mg-ui` (the area view's and the
model viewer's switch, kept in the settings), `mg-module` (`shader-pair`),
and with `MOONGLOW_HAKS` the TDoD tests above (`mg-corpus-tests` `models`
group, `game_shaders.rs`; `mg-ui`'s `tdod_s_wind_shaders_move_in_the_area_view`).

What M2 leaves is in [the deferred list](deferred.md) ("Tier A of the
game's shaders").

[^m2]: M2: the files in the frontmatter's `m2` source, run 2026-10-21.
[^wiki-shaders]: nwn.wiki, "Shaders Introduction: Editing A Textures Colour" (both programs needed) and "MTR" (Renderhint: tangents generated).

## M3: the frame budget (2026-10-21)

**The default is on** (`GAME_SHADERS_BY_DEFAULT`, owner's decision 3),
by this rule: *with the switch on, every measured view that is within
the 144 fps budget with it off stays within it*. Where the switch off is
already over the budget, the switch on may cost more, and it does in one
view. In the synthetic TDoD area below, seen whole while zooming out (33,000
meshes of the densest custom tileset on screen at once), a frame takes
**54 ms on against 29 ms off**, and the GPU 17.6 ms against 5.7. That
view is 4× over the budget with Moonglow's own shader too. The owner can
reverse the default with one constant; a builder's saved choice is
kept.[^m3]

**Measured** with `frame_perf`'s `game_shaders_frames_at_144` (release
build; Ryzen 7 5800X3D and RX 9070 at 3440×1400; the view with the
pointer moving, left alone, and zooming out with the wheel). There are
two areas. The first is stock: *Tyrants of the Moonsea*'s largest,
`area092`, with 784 tiles, 61 objects and 749 lights. The second is
synthetic, The Domains of Dread's, because TDoD's own module has no
large area yet: 24 by 24 of its `mst04` tiles (every tile of the set in
turn, the wind shaders' trees, plants and grass), 400 of its
`tdod_p_tap`/`tdod_p_ham` placeables, its shader haks attached, and no
lights. The median CPU time of a frame, before M3 (M2 as committed) and
after, switch off and on (worst of the frames in brackets where it
matters):

| View | Off | On, before | On, after |
|---|---|---|---|
| area092, overview, wheel | 3.97 ms | 12.37 ms | **5.97** (6.34) |
| area092, turning, wheel | 4.21 | 10.97 | **5.01** (6.06) |
| area092, turning, pointer | 1.95 | 1.88 | 1.70 |
| area092, close, pointer | 1.74 | 2.40 | 1.88 |
| TDoD woods, close, pointer | 3.88 | 8.01 | **5.35** |
| TDoD woods, close, wheel | 4.06 | 7.43 | 4.83 |
| TDoD woods, turning, pointer | 6.05 | 4.99 | 3.56 |
| TDoD woods, overview, wheel | 29.4 | 124.9 | 54.2 (GPU 17.6; off 5.7) |
| TDoD woods, turning, wheel | 45.5 | 118.2 | 52.6 |

The renderer's frame taken apart (`area_parts`: the area at 3000×1250,
seen whole from above, 4× MSAA; the time to make and encode the draws):

| | Off | On, before | On, after |
|---|---|---|---|
| area092: encoded | 2.57 ms (1,028 draws) | 10.34 ms (3,236 game draws, each its own) | **4.01 ms** (3,236 game meshes in 1,807 draws) |
| area092: GPU | 0.96 | 1.15 | 1.00 |
| TDoD: encoded | 28.75 (27,990 draws) | 131.3 (33,170) | **38.25** (33,170 in 33,198 draws) |
| TDoD: GPU | 9.50 | 18.35 | 9.75 |
| TDoD close: encoded | 3.03 | 6.03 | 3.94 |

**What was done, step by step**, with the encoding time of area092's
overview and of TDoD's:

| Step | area092 | TDoD | Kept |
|---|---|---|---|
| M2: a draw's whole block (5–9 KB) written each frame, two bind groups a draw | 10.3 | 131 | |
| a mesh's pipelines kept by its slots; the inverses and the camera's angles once a frame | 9.4 | | yes |
| **the block split in IR** into the draw's, the frame's (written once a frame per pipeline) and the bones' | 6.6 | | yes |
| members a program never reads left out (`vslit`'s draw block 1,392 → 80 bytes), the contract's unfilled uniforms the frame's, a mesh bound once for its draws in a row | 5.1 | 60 | yes |
| the lights in a fourth dynamic block, shared by draws of one set of lights | 5.7 | 60 | no: one more dynamic offset a draw cost more than the KB it saved |
| compiles off the UI thread, the disk cache | 5.2 | 62 | yes (no change a frame) |
| **draws as instances**: a draw's values and its lights' in storage arrays its place picks, no bind group a draw | 5.7 | 68 | yes, with what follows |
| a see-through mesh's two parts sharing their values; a quicker hasher; samplers a program never reads left out of its layout | 4.7 | 56 | yes |
| what the game's draws bound kept from one see-through draw to the next | 4.0 | 39.9 | yes |
| what a material decides of a draw written once into a template | 4.0 | 38.9 | yes |

**Opening an area.** Before M3, the first frame with the switch on
compiled the area's programs on the UI thread. In a fresh renderer that
frame took 195 ms where it took 153 ms off for area092, and 5.8 s where
it took 5.2 s off for TDoD. Now no frame waits for them. A mesh draws with
Moonglow's shader until its pipeline is ready, and the area view draws
again as its animations step. Opened in a new window with the switch on,
area092's module opened in 141 ms. Its longest frame was 262 ms
(loading its models; 286 ms with the switch off), and the game's
programs drew it 7 ms after it showed. The worker's time:

| | Programs compiled / read | Pipelines | Worker's time, cache empty | Cache filled |
|---|---|---|---|---|
| area092 | 4 | 8 | 50–113 ms | 28–40 ms |
| TDoD woods | 22 | 25 | 250–277 ms | 123–146 ms |

(With a filled cache, what remains is naga reading the words and the
driver making the pipelines.) TDoD's opening is hidden in its module's:
the module opens its one area itself, in 5.9 s.

**Decided:**

- **The split is in IR, prepared in SPIR-V.** naga requires an
  expression's operands to come before it. A new global's expression,
  appended to a function, couldn't be the base of the accesses already
  there. naga's SPIR-V reader gives every function an expression for
  each global variable first, so the SPIR-V pass declares the
  variables the IR needs: two spare blocks of the uniform block's type
  (the frame's and the bones'), storage arrays of `vec4`s for the draws'
  and the lights' values, and, at each function's start, the reads of
  the draw's places and pointers to its elements. In the IR,
  `Program::split_uniforms` then gives each its type (each part laid out
  anew, std140's alignments, validated) and moves each member access to
  its part, by the contract's classification (`game::part`; mg-shader
  doesn't know the contract).
- **Instances through storage, as Moonglow's own draws.** The vertex
  program reads the order of draws at `instance_index` (four places a
  draw: its own and its lights', for each stage) and hands the fragment
  program its places as flat varyings. Each pipeline's values lie in one
  buffer, in a region at a multiple of their size, so a place is a whole
  index and group 2 is bound once a pipeline. The bones stay a dynamic
  block (group 3), bound a draw only by pipelines of skinned meshes.
  Inputs Moonglow has no data for read one zero buffer at stride 0.
- **Persistent slices** (the proposal's first answer) don't apply.
  `m_mv`, `m_mvp`, `m_normal` and the lights' view-space positions
  change whenever the camera moves, and that is the only case over the
  budget. A still view is drawn again only as its animations step.
- **One worker thread** compiles and makes the pipelines: glslang's
  compiler is the process's one. The UI's thread assembles the programs,
  since it has the assets. A picture drawn once (a thumbnail, a preview,
  `render_image`, the tests) waits for them
  (`Renderer::game_background` off), so it is never left drawn with the
  fallback. A waker asks the window for a frame when a pipeline is ready.
- **The cache** is `mg_module::build::cache_dir()`'s `shaders/`
  (`$XDG_CACHE_HOME/moonglow`, `~/Library/Caches/Moonglow`,
  `%LOCALAPPDATA%\Moonglow\cache`). An entry holds the words after
  Moonglow's passes and the inputs' and outputs' names. Its name is a
  128-bit hash of the assembled text (the preamble is its first lines),
  the stage, mg-shader's version and the passes' format, and a checksum
  closes it. An entry that is cut short, damaged or another program's is
  passed over and written anew.

**Pictures looked at.** The off and on overviews of both areas. The stock
area's are alike: its lit programs match Moonglow's within 1 of 255. TDoD's
are darker on, under its own `inc_lighting`, as M2 found against the
client. The same overviews with the whole block and with the split, and
before and after instancing, differ only where things move with the clock
(water, the swaying grass). The TDoD fountain, grass and roof pictures of
`game_shaders.rs` look as before, and the wind test's area view as well.
The oracle (`the_game_s_lit_shaders_match_moonglow_s`) still holds the
lit path within 1 of 255.

**Cross-platform.** CI has no game install. To exercise the game path
there, `an_authored_glsl_pair_draws_on_this_platform_s_gpu`
(`mg-render/tests/render.rs`) compiles an authored pair. It has loose
uniforms in the draw's, the frame's and the lights' parts, a `sampler2D`
passed to a function, varyings, an include and a zero input. The test
draws three instances of one mesh in one call and checks their pixels.
It prints the adapter it ran on past the harness's capture (`game path:
… (Vulkan, DiscreteGpu) …` here), so the Windows and macOS logs say
whether Direct3D 12 and Metal drew it or no adapter was found.

**Tests:** mg-shader (the split into four parts with members never read
left out, the scaffolding's varyings and bindings, `place_named`, the
cache read back and every damaged entry compiled again); mg-render (the
programs made while frames go on, the waker, the cache read by another
renderer, the platform test); mg-ui (the switches start from the
default; the area view tests that compare Moonglow's own pictures choose
it off); the shader corpus splits every program: 65 of 65 stock, 102 of
102 with TDoD's haks.

What M3 leaves is in [the deferred list](deferred.md) ("The frame
budget of the game's shaders").

### After M5: the lights (2026-10-21)

M5's water area broke M3's rule. Zooming out, a frame took **7.5 ms with
the switch on and 5.6 off**, so a view within the budget with the switch
off went over it with the switch on. Now it takes **6.5 ms on**, and the
rule holds again for every view measured. The default stays on. What the
game's programs compute is unchanged: the same values reach them, found
another way.[^lights]

**Where the time went** (`area_parts`, the water area seen whole; phase
timers in `Renderer::render`). Encoding took 4.36 ms on against 2.74 off,
while the GPU took less with the switch on (1.35 against 1.69 ms). Of
the 1.6 ms more, 1.4 was in the loop over the meshes and 1.3 of that in
writing the game draws' values:

- **The renderer's light sets: about 0.45 ms.** Each draw built arrays
  of its lights' positions and colors (21 lights on average), hashed the
  set with SipHash and allocated it when new.
- **The lights' blocks: 0.57 ms.** The draws had 1,468 sets of lights,
  giving 1,524 blocks of 1,040 bytes each (`numLights` and 32 lights'
  places for each array), written an element at a time.
- **The draws' own values: 0.46 ms.**
- **The upload: 0.15 ms**, for 2 MB a frame, 1.58 MB of it the lights'
  blocks. The submit took 0.1 ms more.

The sets of lights are 96% unique in the water area (2,698 draws, every
mesh near a different few of the 160 fires). area092 shares more: 3,571
draws took 2,032 blocks of 1,895 sets, with 6.9 lights on average. So the
lights a draw shares with others stay shared.

**What was done**, with the encoding time of the water area's overview
(and area092's), each step measured back to back:

| Step | Water | area092 | Kept |
|---|---|---|---|
| M5 | 4.36 | 4.00 | |
| A draw's lights written as their indices into the frame's lights, each array a vector at a time (and the bones a matrix at a time), not built per draw and written an element at a time | 3.83, all three steps | | yes |
| The light sets hashed eight bytes at a time with the quick hasher, after a check against the last draw's set | (the values' writing 1.07 to 0.99) | | yes |
| `m_mv` worked out once a draw for `m_mv`, `m_mvp` and `m_normal` | (0.69 to 0.66) | | yes |
| **The lights read by their places in the frame's** (below) | 3.80 to 3.68 | the frame 5.5 to 5.3 | yes |
| The regions, light places and frame marks in vectors indexed by number, not hash maps | 3.67 to 3.60 | 3.60 to 3.51 | yes |

(The rows in brackets were timed inside the loop with timers of their
own, which cost about 0.25 ms a frame themselves.)

**The lights by their places.** The engine declares `uniform vec4
lightPosition[MAX_NUM_LIGHTS]` and `lightColor` (32 lights, the client's
default `max-lights`). The stock programs read them an element at a time
in a loop to `numLights`. A SPIR-V pass, `index_lights`, does the
following:

- It gives the uniform block a member of the draw's lights' places,
  `mg_lightIndex`, four places to a `uvec4` (eight for 32 lights).
- It makes each read of `lightPosition[i]` or `lightColor[i]` (or of a
  component of one) a read of the frame's lights, `mg_light_table`, at
  place `mg_lightIndex[i]` times two (plus one for the color). The
  table is a storage buffer written once a frame. Its place 0 holds
  zeros, which is what the arrays held past a draw's lights.
- The arrays are then read nowhere, so the split leaves them out. A lights
  block is now 144 bytes instead of 1,040.

The pass runs in SPIR-V, before naga, for the reason M3's split needs
SPIR-V's help: naga's expressions must come before what uses them, and
the new reads come before each access. A program that reads the arrays
otherwise than an element at a time (a whole array copied), or whose
arrays aren't `vec4`s, is left as it was. The renderer fills either
layout by name. The shader corpus requires every stock program that reads
the lights to read them by index: 7 of 65, and 26 of TDoD's 102.

A smaller `MAX_NUM_LIGHTS` in the preamble, for draws with fewer lights,
was not used. The engine's loops are bounded by `numLights` when they
run, so a smaller define only shrinks the arrays, and the places do that
already, without a pipeline for each count. The preamble keeps the
client's default of 32, which is also the most lights Moonglow's light
grid gives a draw.

**Measured** (`game_shaders_frames_at_144`, release build, this machine;
the median CPU time of a frame, ms, with the GPU's in brackets where it
leads). Other sessions were building on the machine at the same time.
Each pair was run back to back under one GPU lock: the water area at load
2.2–2.5, area092 at 2.3–4.0, TDoD's woods at 2–5.

| View | Off | On, M5 | On, now |
|---|---|---|---|
| water area, overview, wheel | 5.6 | 7.3–7.5 | **6.5** |
| water area, turning, wheel | 5.9 | 7.4 | **6.5** |
| water area, close, pointer | 3.9 (6.9) | 4.4 | 4.1 (2.7) |
| water area, parts: encoded / GPU | 2.7 / 1.5 | 4.3 / 1.24 | 3.6 / 1.24 |
| water area, close parts: encoded / GPU | 0.9 / 2.8 | 1.28 / 2.48 | 1.18 / 2.55 |
| area092, overview, wheel | 4.0 | 5.7–6.1 | **5.2** |
| area092, turning, wheel | 4.3 | 4.8–5.4 | 4.4–4.6 |
| area092, parts: encoded / GPU | 2.65 / 1.1 | 4.0 / 1.0 | 3.45 / 1.0 |
| TDoD woods, overview, wheel | 28.5 | 55–66 | 59 |
| TDoD woods, close, pointer | 3.8 | 5.3 | 5.3 |

TDoD's woods have no lights. They are as they were: over the budget
either way seen whole, within it close up. Close up, the GPU takes 0.06
ms more for the read through the places (2.55 ms against 2.48), and it
leads nowhere this was measured.

**Pictures looked at.** The water area's three views with the switch on
are the same as M5's to the bit. area092's were compared at a fixed clock
(`area_parts`' last frame, saved), with the pass on and off from one
build. They are the same but for 12 values of the overview with 4×
MSAA, at most 3 apart, and two runs of one build differ by as much.
area092's views saved with `FRAME_SHOTS=1` differ from M5's by up to 64
of 255 over the sea, brighter and darker alike, and only there (its
tiles animate). Two runs of this build differ there by up to 31, where
M5's and the first steps' shots were the same from run to run. At a
fixed clock the pictures are the same, so the difference is the clock
the view was drawn at, not the lights; why it differs from run to run
was not looked into. The oracle (`the_game_s_lit_shaders_match_moonglow_s`:
stock `fslit` lit by a point light, through the table) gives the same
numbers as before, within 1 of 255.

**Tests:** mg-shader (the pass on an authored program: a component, a
constant index, a vector constant; a whole array copied left as it was;
the pass on bad words); mg-render (the fill writes each light's place plus
one; the platform test lights its three instances of one draw apart, so
CI's Direct3D 12 and Metal read through the table, mutation-checked);
the shader corpus (above).

**What is left.** In the water view the switch on still costs about 0.9
ms of encoding (3.6 against 2.7 ms off). Most of it is the draws' own
values, about 0.75 ms: the material's template copied, and `m_mv`,
`m_mvp` and `m_normal` with an inverse for each draw. The upload costs
0.09 ms and the submit 0.07 ms more. The places are not yet run on
Direct3D 12 or Metal. In [the deferred list](deferred.md).

[^m3]: M3: the files in the frontmatter's `m3` source, measured 2026-10-21.
[^lights]: After M5: the files in the frontmatter's `lights` source, measured 2026-10-21.

## M4: the engine's choice (2026-10-21)

With **Shaders** on, every mesh the game path draws uses the program pair
the engine would choose for it, with its material's `NO_DISCARD`. Both
were read from the client, not inferred.[^m4]

**How it was read** (`program_choice_in_the_client`, `client_render.rs`,
ignored). Every stock vertex and fragment program goes into the scratch
user folder's `override` with its `main` wrapped. The post passes' and the
PLT's are left out. The vertex program hands its number to the fragment
program, which paints over what it drew: red is its own number, green the
vertex program's, and blue the preamble's `NO_DISCARD`,
`FRAGMENT_LIGHTING`, `KEYHOLING_ENABLED` and the turn of `m_texture`. What
it discards stays discarded. Lit, textured fragments in a band near the
picture's foot show the uniforms instead (the module's clock, the camera's
angles, `m_texture`, `texNoiseBound`, the mean colour of the bound
environment map). The scene is an exterior `tcn01` area with every tile
turned a quarter. North of the player stand 36 cards, each a mesh of
another kind, in front of a decal board, so that a missing card shows.
Around them are the game's placeables with and without a `Reflection`,
creatures with and without an appearance `ENVMAP`, a placeable turned a
quarter, and a square lying under the band. Further runs moved the
camera (the entry direction), set a `Reflection` on the square, and used
DDS cards with all their levels (with a single level the client read
noise past it). The test then draws the same module with Moonglow and
**asserts that Moonglow chooses the client's pair and `NO_DISCARD` for
each card read** (34 of 36: the additive card is blended, and one card
straddles its own cut).

**The rules** (the table, card by card, is notes §A.4[^notes-shaders]):

- **Pairs.** A decal (TXI `decal 1`) draws with `vsvt`/`fst`: unlit,
  without fog, `globalColor` white with the mesh's alpha. A mesh without
  texture coordinates draws with `vslitnotex`, or `vslitnt_sm` when it
  reflects. Everything else draws with `vslit`. It takes `_sm` for an
  environment map: a TXI `envmaptexture` (`default` too), a
  placeables.2da `Reflection` or an appearance `ENVMAP` (`default` too).
  It takes `_nm` for a render hint, the model's or the MTR's. Vertex
  colors never choose `vslitc*`, and no content mesh draws with `vsv`,
  `vsvc` or `vsvtc`.
- **What the engine ignores.** A texture's `bumpyshinytexture` changes
  nothing. Normal and specular maps named without a render hint are not
  read (`fslit` samples none). The tileset's `EnvMap` doesn't choose
  `_sm`: it is the map bound to `texUnitEnv` for every lit mesh without
  one of its own, the one its specular light reflects, and what `default`
  means.
- **`NO_DISCARD`** (open question 10) is 0, alpha-tested, without a
  texture, with a texture whose alpha means opacity, with a mesh alpha
  below 1, or with an MTR's `transparency`. Otherwise it is 1, and the
  program's `discard`s are defined away. A TGA's alpha is read from its
  texels: a 32-bit TGA all opaque is opaque. A standard DDS's comes from
  its format: DXT1 has none (its transparent texels are drawn black),
  DXT5 always has some. A TXI `alphamean` below 1 marks a texture as
  having alpha. An environment-mapped texture's alpha is its
  reflectivity, so it gives 1.
- **The uniforms the band read.** `m_texture` turns a tile's
  `rotatetexture` meshes by the tile's own turn. A turned placeable's
  `rotatetexture` mesh keeps the identity. The module's clock is the
  IFO's starting values. `cameraYaw` is anticlockwise from north (0 facing
  north, −45 north-east) and `cameraPitch` is from straight down.
  `texNoiseBound` is 1, `staticLighting` 0 and `areaGlobalWind`
  (1.414, 1.414, 0) for a strong wind, as M2 had it.

**Decided:**

- `game::stock_pair` and `game::no_discard` hold the rules, and a unit
  test holds the table. `NO_DISCARD` is a pipeline's: `PipelineKey`
  carries it, the worker keeps programs by name, stage and `NO_DISCARD`,
  and the disk cache needs no new format (its key is the assembled
  text). A mesh whose alpha an animation lowers changes pipeline as it
  does.
- What the engine ignores is ignored in **both** paths, so Moonglow's own
  shader draws as the client does too. Water keeps its
  `bumpyshinytexture` for Moonglow's own water, which stands in for
  `fswater` (M5). The previews keep `default` and the renderer resolves
  it: the area's map, or `chrome1` without an area. DXT1's no-alpha is
  `mg-image`'s, so a preview's picture agrees.
- Decals join the game path. Additive meshes, the sky, markers and water
  stay Moonglow's.
- `m_texture` is applied in Moonglow's shader too: a `uv_turn` in the
  draw's values, and the normal map's x and y turned back on the
  model-tangent path, as `inc_standard` does. `Instance::tile` marks a
  tile's instance.
- Static placeables are drawn per mesh: the client's baking of a static
  placeable's meshes together, which seemed to share one `NO_DISCARD`, is
  recorded, not followed.
- `gl_FragCoord.y` stays unflipped. No program the game path draws reads
  it, and none of TDoD's 37 do. Texture slots 8 to 15 stay white: no
  material on this machine names one (TDoD's 18,146, the game's 7).

**The WGSL fallback's environment map** is now sampled as `inc_envmap`
samples it, with the roughness's `clamp(r·30 − 1, 0, 10)` as a bias over
the derivatives' level. The coordinates' derivatives, from the normal
map's normal, are taken at `fs_main`'s start and scaled by 2^bias in
`textureSampleGrad`, which needs no uniform control flow. Against the
stock programs (`the_game_s_lit_shaders_match_moonglow_s`, mean / 99th
percentile of 255):

| Scene | Before | After |
|---|---|---|
| `_sm`, a sphere reflecting a map | 0.73 / 15 | **0.00 / 0** |
| `_nm`, normal and specular maps, render hint | 1.11 / 11 | **0.08 / 2** |
| the plain scenes (sphere, tile, placeable, wolf) | 0.01–0.12 / 0–1 | 0.00–0.01 / 0 |
| M4's new: a decal; a sphere without coordinates; a tile turned a quarter with `rotatetexture` | | 0.00 / 0; 0.08 / 1; 0.00 / 0 |

The tolerance is now 0.5 / 1, and 0.5 / 3 for `_nm`, whose derivatives
are taken before the parallax and the bend of a normal facing away.
Pictures looked at: the reflecting sphere's seams, sharp in the stock
programs, are now sharp in Moonglow's too. A mutation check holds the
turned tile to differ from an unturned one (more than 5,000 pixels).

**A hak's include through the load order**
(`tdod_s_inc_lighting_changes_the_area_view_through_the_load_order`,
`mg-ui`, `MOONGLOW_HAKS`). The area is 4 by 4 of TDoD's `mst04` tiles in
two modules, one listing `tdod_shaders.hak` over `tdod_t_tap` and one
without. With the switch on, 259,475 of the view's 330,000 pixels differ.
The diffuse-wrap `inc_lighting` draws the ground and the cliffs darker
(mean brightness 50.6 against 63.5), and the ground's texture shows more
contrast, since the wrap lets N·L go below zero (`DARKER_SHADOWS`). With
the switch off, Moonglow's own shader draws both modules identically
(63.9). The stock-include picture with the switch on (63.5) is within 0.4
of the switch off. Pictures looked at:
`target/test-output/ui-area-tdod-lighting/`.

**Against the client** (the reference scenes and the look tests of
`client_render.rs`, by hand on the off-screen display; each of Moonglow's
pictures with the switch off and on beside the client's, the mean
difference over the picture away from the GUI, of 255):

| Scene | Switch off | Switch on |
|---|---|---|
| reference room, the sun (`reference_scenes_match_the_client`): regions' worst mean; the picture | 4.7; 5.91 | 4.7; 5.91 |
| reference room, tile lights | 0.9; 5.05 | 0.9; 5.05 |
| placeables (`placeables_look`) | 8.52 | 8.52 |
| creatures (`creatures_look`) | 9.71 | 9.71 |
| cut-outs and cards (`cutouts_look`) | 7.45 | 7.45 |
| items on the floor (`items_look`; its camera isn't fitted) | 50.57 | 50.57 |
| grass and the sky (`grass_and_sky_look`) | 14.71 | 14.71 |
| an area's grass (`grass_overrides_look`) | 18.52 | 18.52 |
| TDoD's fountain room (`tdod_custom_shaders_look`): the fountain's 7,091 pixels | 13.7 | **11.7** |
| its centre floor, mean RGB (client 73.5, 68.8, 64.0) | 88.6, 82.5, 76.5 | **73.6, 68.8, 64.0** |
| its left floor (client 36.2, 33.4, 31.8) | 39.3, 36.1, 34.1 | **36.7, 33.8, 32.2** |
| its far floor (client 55.0, 49.1, 46.6) | 55.4, 49.1, 46.5 | 54.0, 48.4, 46.0 |

On the stock scenes, on and off are within 0.01 of each other: the game's
lit programs and Moonglow's port match within rounding (the M1 oracle).
Only a few hundred pixels differ between them, by at most 14 to 58 at
edges. On TDoD's room, on is the closer, as M2 found: its `inc_lighting`
is drawn, and the centre floor now matches the client within 0.1 (M2:
73.4, 69.0, 64.8). What is left there is the fountain's water (M5). The
one region where on is farther is TDoD's far floor: 1.0 of 255 from the
client against off's 0.4. It borders a dark wall where the fitted camera
is a fraction of a pixel off (the reference test allows 6 there), so it
was left. Everywhere else on is at least as close as off. The
whole-picture means are the camera fit's as much as the shading's: a
region's mean (floor, wall, armoire) agrees within 1 to 5 of 255.

**Measured** (`frame_perf`, `game_shaders_frames_at_144`, release build,
this machine; M3 rebuilt in this session for the baseline, one area a
run, since a run of both areas came out up to 40% slower for the second):

| View | M3 off | M3 on | M4 off | M4 on |
|---|---|---|---|---|
| TDoD woods, overview, wheel | 29.4 ms (GPU 5.7) | 56.4 (GPU 17.5) | 27.8 (5.9) | **52.0** (16.6) |
| TDoD woods, turning, wheel | 46.3 | 50.4 | 43.4 | 48.7 |
| TDoD woods, close, pointer | 3.92 | 5.30 | 3.69 | 5.14 |
| TDoD woods, area parts: encoded / GPU | 29.1 / 9.57 | 38.6 / 9.74 | 26.8 / 9.86 | 37.3 / 9.47 |
| area092, overview, wheel | 3.97 (M3's session) | 5.97 | 3.98 | **6.17** (worst 6.60) |
| area092, turning, wheel | 4.21 | 5.01 | 4.28 | 5.47 |
| area092, area parts: encoded / GPU | 2.57 / 0.96 | 4.01 / 1.00 | 2.64 / 1.12 | 4.22 / 1.17 |

`NO_DISCARD` 1 goes to 2,060 of area092's 2,649 game meshes, but to only
826 of the woods' 16,605, whose textures are nearly all DXT5. Its GPU
time is 16.6 ms against 17.5 with 0 for every material, so early depth
was not what made the woods slow (notes §M.10, M3's suspicion). Moonglow's
own shader got a little quicker there (27.8 against 29.4 ms): DXT1 meshes
left the see-through draws. Every view within the budget with the switch
off stays within it on (area092's overview 6.17 ms). Opening costs more
programs: area092 compiles 8 where it compiled 4, and makes 12 pipelines
where it made 8 (87 ms of the worker's, 47 with the cache filled); TDoD
26 and 27 (341 ms, 158 filled), for the pairs drawn both ways.

**Tests:** `mg-render` holds the rules (`the_engine_s_choice_of_program_and_no_discard`),
the camera's angles, a tile's `m_texture` in the block, maps unread
without a render hint (`material_maps_shade`), and the oracle's new
scenes; `mg-preview` keeps `default`; `mg-ui` holds the hak's include
(TDoD) and the module clock in the area view; `program_choice_in_the_client`
(ignored) holds Moonglow to the client card by card.

What M4 leaves is in [the deferred list](deferred.md) ("The engine's
choice of program").

[^m4]: M4: the files in the frontmatter's `m4` source, run 2026-10-21.

## M5: the special surfaces (2026-10-21)

With **Shaders** on, the skybox, the area's grass, additive meshes and
particles draw with the programs the client draws them with. Shiny water
stays Moonglow's: the game's program needs the framebuffer, which 2.0
leaves out, and without it the water is farther from the client than
Moonglow's own.[^m5]

**How it was read** (`surfaces_in_the_client`, `client_render.rs`,
ignored): M4's marked programs, on a rural area (`ttr01`) with its grass
on (`graphics.grass` mode 2), its north row of tiles water (tile 20, all
water), a skybox, cards of water, additive and decal textures in front of
a decal board, still particles of the three blends, and a sword and a
shield on the ground. `fsparticle`'s mark carries its `blendMode` and
whether `particleSoftness` is above 0. Further runs: `black`, the board's
`fst` painting black, so what is added over it reads as its own mark (the
additive cards, a Lighten particle, one at a time: a few of them in one
place add up to white); `sky`, the camera raised once the player is in
(from the OnClientEnter it doesn't move); `waterband`, water under the
band of uniforms (the maps `fswater` is given); `unmarked`, the scene as
it looks, which Moonglow's pictures are compared with. The test holds
Moonglow's choice to the client's for every surface.

| Surface | The client | `NO_DISCARD` | Moonglow, Shaders on |
|---|---|---|---|
| shiny water (a tile's; a placeable's mesh whose TXI says `shinywater`) | `vswater`/`fswater`, the tileset's 2D `EnvMap` bound (`envMapCube` 0), not the `bumpyshinytexture` cube | 1 | its own water (chosen as the client, not drawn: below) |
| the area's grass | `vsgrass`/`fsgrass`, `FRAGMENT_LIGHTING` 0 | 0 | the same, on Moonglow's tufts (`Instance::grass`) |
| the skybox | `vsvt`/`fst` | 1 | the same, at the far plane (`Program::at_far_plane`) |
| the horizon's fade | (not marked apart) | | Moonglow's |
| an additive mesh | as any mesh: `vslit`/`fslit`, `_sm` with an envmap, `vsvt`/`fst` as a decal (inferred: that card read black over the black board, as only `fst` painted) | 1 | the same, added as Moonglow's own adds it |
| a decal with an envmap | `vsvt_sm`/`fst_sm` | 1 | the same (M4 drew `vsvt`/`fst`) |
| particles: Normal, Lighten, Punch-Through | `vsparticle`/`fsparticle`, `blendMode` 0, 1, 2; `particleSoftness` above 0 | 1, 1, 0 | the same; softness 0 |
| an item on the ground | `vslit_sm`/`fslit_sm` | 1 | the same (`chrome1`, as before; the client's map not read) |

**Against the client** (the `unmarked` run: each region's mean colour,
the mean difference per channel of 255; the fitted camera; Moonglow's
pictures off, on, and on with the game's water):

| Region | Switch off | Switch on | On, the game's water |
|---|---|---|---|
| the sky | 0.0 | 0.0 | 0.0 |
| the far water (four ttr01 water tiles) | 12.6 | 12.6 | 19.4 |
| the water card (a placeable's mesh, standing) | 55.6 | 55.6 | 38.3 |
| the grass field | 1.4 | 0.1 | 0.1 |
| the decal board | 0.2 | 0.2 | 0.2 |
| particles: Normal, Punch-Through, Lighten | 0.5, 0.2, 14.4 | 0.5, 0.2, 14.4 | |
| cards: additive, additive decal, additive envmap, decal envmap, plain | 17.0, 3.5, 6.2, 5.0, 1.0 | 17.2, 3.5, 6.2, 5.0, 1.0 | |
| the picture (away from the GUI) | 17.97 | 18.49 | 19.10 |
| TDoD's fountain room: the fountain's 7,091 pixels | 13.7 | 11.7 | 16.4 |

On is as close as off, within 0.2 of 255, for every surface's region,
and closer on the grass. The whole picture is 0.5 farther on: the grass,
whose blades Moonglow grows in other places than the engine. Over the
field its mean colour is closer (0.1 against 1.4), and per pixel a
little farther (17.6 against 17.0), as `vsgrass` lights its blades at
their corners with more contrast than Moonglow's shader. The particles'
and the cards' differences are the same by either path (Moonglow draws
the still particles of an 8×8 white texture as jagged discs and a turned
square, the client as squares: not M5's). The cards are partly hidden by the grass, which sways: their
numbers move by a few from run to run. A standing water card is the one
place `fswater` comes closer (38.3 against 55.6: Moonglow's own draws it
dark blue, the client light), on content no area has.

**Water: what `fswater` does without the framebuffer.** Its
`texFBColor` and `texFBDepth` are white, as every sampler Moonglow has
nothing for. Then `GetDepth` is the far plane everywhere and the water
infinitely deep: the main wave at its full height, the refraction's
transparency 0, and the reflection's ray finds nothing, falling back to
the surface's own specular-lit colour. The water is drawn **opaque**,
its base colour times the subsurface light plus the Fresnel reflection of
the area's map, with the client's wave pattern (its long streaks are
where the client's are). The client's is darker and see-through to what
lies under it, and its colour is mostly that refraction: TDoD's basin
shows its floor. Measured, `fswater` is farther from the client than
Moonglow's own water (the lake 19.4 against 12.6, the fountain 16.4
against 11.7). So water stays Moonglow's (`GAME_WATER`; the switch
`Renderer::game_water` draws it for comparing). A framebuffer copy (the
picture and its depth after the solid meshes, a second pass) would let it
draw as the client does; that is the uniform contract's out-of-2.0 row.

**Decided:**

- **The sky at the far plane in IR.** Moonglow draws the skybox around
  the camera, beyond the far plane, without depth. `vsvt` would have it
  clipped, so `Program::at_far_plane` sets the position's depth to
  `w·0.99999` as it leaves (after the depth range's remap), and the sky's
  pipeline writes no depth and culls nothing. The fade stays Moonglow's.
- **Passes of the game's programs.** `GamePass` gained `Additive` (one
  and one, no depth written, as Moonglow's own), `Sky`, and
  `Particles(blend)`: Normal alpha-blended, Punch-Through solid (cut at
  0.2, with depth), Lighten added by its alpha (`fsparticle` fades it by
  its alpha into the fog). Additive and sky draws are draws of their own
  among Moonglow's, in their places.
- **Particles' vertices by name** from `ParticleVertex`: `vPos`, `vTcIn`,
  and `vColor` the particle's colour (gamma, as `vsparticle` takes it). A
  game draw's geometry is a mesh or a range of the particle buffer
  (`Geometry`), drawn without indices. `blendMode` is a draw's uniform;
  `particleSoftness` is left 0 (with white for `texFBDepth` it would fade
  nothing either).
- **Grass** is marked on its instance (`Instance::grass`), chosen by
  `Traits::grass`; its vertex program reads the draw's lights (the
  vertex stage's lights part, M3's).
- **Water's map** for `fswater` is the area's (`Surface::water_material`),
  where Moonglow's own water keeps reflecting its `bumpyshinytexture`.
- **See-through game draws batched.** A see-through, additive or sky draw
  of the game's had been a draw of its own; it is now placed with the
  others of its pipeline, material and mesh where they don't meet on the
  screen, as Moonglow's own are (the batcher's key holds the pipeline),
  and the game's draws are laid out in the order they are drawn, so a
  batch's are instances in a row. Floats are written a vector at a time,
  and the light sets and pipelines looked up with the quick hasher.

**Measured** (`frame_perf`, `game_shaders_frames_at_144`, release build,
this machine; a new view, `FRAME_AREA=water`: 16 by 16 of ttr01, half of
it water, its grass, a skybox, 160 braziers, campfires and flames with
their particles and lights):

The median CPU time of a frame, ms (M4's on rebuilt in this session for
the baseline; the machine shared with another session's server and
builds, load 3 to 6, so a run's numbers move by about 5%, and each was
run twice or more):

| View | Off | M4, on | M5, on | M5, on with the game's water |
|---|---|---|---|---|
| water area, overview, wheel | 5.6 | 7.9 | **7.5** | 7.8 |
| water area, turning, wheel | 5.9 | 7.8 | 7.5 | 7.7 |
| water area, close, pointer (GPU) | 3.9 (8.0) | | 4.5 (3.0) | 4.4 (4.0) |
| water area, parts: encoded / GPU | 2.7 / 1.7 | 4.5 / 1.4 | 4.3 / 1.4 | 4.5 / 1.7 |
| area092, overview, wheel | 4.0 | 6.2 | **5.8** | |
| area092, turning, wheel | 4.4 | 5.5 | 5.1 | |
| TDoD woods, overview, wheel | 29.9 | 56.4 | 59 to 66 | |
| TDoD woods, close, pointer | 3.8 | 5.2 | 5.4 | |

- **The water area was over the budget with the switch on** (brought
  within it after M5: [the lights](#after-m5-the-lights-2026-10-21)), **and was at
  M4** (7.9 ms, off 5.6): its 160 fires' lights and 2,400 game meshes,
  more than half see-through. M5 brought it from 7.9 to 7.5 (see-through
  game draws batched: 1,347 calls to 391; floats written a vector at a
  time); what is left is a game draw's values, its lights' block (32
  lights' places, a kilobyte, for each set of lights and pipeline) above
  all, as M3 found. The surfaces M5 added cost little of it: the
  particles' game draws 0.27 ms of encoding, the grass's and the sky's
  less. Recorded in [the deferred list](deferred.md).
- **The game's water** costs the GPU 0.9 ms more close up (3,440 by
  1,400: `fswater`'s waves are sampled 3 times 3 iterations a fragment,
  its two ray marches run against white) and the CPU nothing to speak of;
  it is out for its look, not its cost.
- **area092 is quicker** (overview 6.3 to 5.8 ms, the batching alone
  measured back to back: 6.3 against 5.8) and within the budget. **TDoD's
  woods seen whole** were over the budget either way and are about 5%
  slower (the batcher's work for 30,000 see-through draws); close up they
  stay within it (5.4 ms, 5.2 at M4).
- Opening area092 compiles 14 programs (8 at M4: the sky's, the
  particles', `fst_sm`) and makes 17 pipelines, 120 ms of the worker's
  (58 with the cache filled).

**The macOS failure of M4's CI.** M4 read the reflecting cube map with
`textureSampleGrad`, which naga 30's Metal backend writes with 2D
gradients (`metal::gradient2d` of `float3`s); Metal refused the whole lit
pipeline, so every test that draws a lit mesh failed there. The cube's
level is now worked out in the shader from the direction's derivatives on
the face it points at (`cube_level`) and read with `textureSampleLevel`;
the 2D map keeps its gradients. A test holds the WGSL sources to no cube
read with gradients or a bias, another writes them for Metal with naga's
`msl-out` and finds no cube sampled with 2D gradients (both fail on M4's
line), and the stock-program oracle gained a sphere reflecting a cube map
(0.00 / 0 of 255 from `fslit_sm`). Cube maps are uploaded with one level,
so both paths read level 0 for now.

**Tests:** `mg-shader` (`at_far_plane`); `mg-render` (the engine's
choices for the new surfaces; on the GPU, authored stand-ins under the
stock names: the sky beyond the far plane, an additive mesh added, the
grass, the three particle blends with their `blendMode`, see-through game
draws as instances of one draw; each mutation-checked); the client tests
above, which hold Moonglow's choice to the client's.

What M5 leaves is in [the deferred list](deferred.md) ("The special
surfaces").

[^m5]: M5: the files in the frontmatter's `m5` source, run 2026-10-21.

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
| Time: `worldtimerTimeOfDay`, `sceneCurrentFrame`, `module*` | a clock of the view's | real milliseconds and frames (measured in M1, notes §B.4) |
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
| M6 | Moonglow Viewer and `mgv` take it (they build on `mg-render` by tag); done in Moonglow Viewer 0.2.0 | the viewer's own tests | S |

## The owner's decisions

Decided on 2026-10-21:[^decisions]

1. **Scope of 2.0: tier B**, the game's shaders for everything, with
   tier A on the way.
2. **The compiler: native glslang and SPIRV-Tools.** Which crate brings
   them is settled in M0: glslang-rs, with SPIRV-Tools' one pass done in
   Rust instead ([M0](#m0-the-compiler-2026-10-21)).
3. **On by default** once M3 meets the frame budget. The switch stays for
   comparing and for a machine where compiling fails. (Done in M3, by the
   rule written there.)
4. **A program that doesn't compile falls back** to Moonglow's own shader
   and is reported in Verify (file, line, glslang's message). No magenta.
5. **The tabled note** is in `develop` (deprecated, pointing here), and
   the `shaders` branch is deleted.

## Open

- What the client's `worldtimerTimeOfDay` counts from (its rate is real
  milliseconds; its value follows the module's minutes an hour, not its
  hour or day: notes §B.4).
- ~~How the engine chooses `_sm` and plain programs~~ and ~~how it sets
  `NO_DISCARD`~~: read from the client in M4 (notes §A.4); ~~when it uses
  `fswater`~~ read in M5. Still open: a static placeable's meshes baked
  together.
- What the toolset itself runs with: `FRAGMENT_LIGHTING`, quality,
  `MAX_NUM_LIGHTS` (notes §M.5). This only matters if Aurora's look is ever
  wanted rather than the game's.
- Whether glslang's relaxed rules accept everything else builders write.
  The corpus here is one module's. The Vault and the Workshop would widen
  it (M0's test runs over whatever haks are installed, and
  `MOONGLOW_HAKS`).

[^decisions]: The owner's answers, 2026-10-21.
[^spike]: The compile spike: `tools/shaders/spike.py`, 2026-10-21.
[^tdod-haks]: The Domains of Dread's haks, read with `mg ls` and `mg unpack`.
[^branch-note]: [The shaders branch's note](research/notes_custom_shaders.md).
[^notes-shaders]: [The shader notes](research/notes_shaders.md).
