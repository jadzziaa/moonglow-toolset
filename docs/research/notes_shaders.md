# NWN:EE rendering and lighting pipeline: research notes (game 89.8193.37-17)

Scope: what a from-scratch area renderer for Moonglow Toolset needs to match NWN:EE visuals.
Evidence tags: **[SRC]** read in the effective stock `.shd` source; **[ENG]** strings in `nwmain-linux` / `nwtoolset.exe`;
**[WIKI]** nwn.wiki (local mirror, see section L); **[CL]** game changelog / patch notes; **[DATA]** extracted 2DA/MDL/TXI/ARE;
**[INF]** my inference, not verified.

Working copies (scratch, all read-only extracts):
- shaders: `$SP/shaders/*.shd` (92 files, extracted with `nwn_resman_extract` + empty `--userdirectory` = effective highest-priority versions)
- 2DAs/models: `$SP/res/` (lightcolor, environment, skyboxes, weathertypes .2da; gidy_sun, fx_flame01, fx_placeable01, skyfade1 .mdl; ASCII in `$SP/res/ascii/`)
- all base TXI: `$SP/txi/` (734), all base MTR: `$SP/mtr/` (7), tilesets .set: `$SP/set/`, sample tiles: `$SP/tile/`
- `$SP` = `/tmp/claude-1000/-home-august-Projects-moonglow-toolset/42266be1-7a5b-4aaa-92bd-b9aa74640348/scratchpad`

---------------------------------------------------------------------------------------------------

## A) Shader inventory and selection

### A.1 Where shaders live, and which copy wins
- Resource type **`.shd` = restype 2069**. GLSL text. No other shader restypes; no textures in `base_shaders.bif`, which holds **only** 76 `.shd`.
- `data/nwn_retail.key` → `data/ovr.bif` (93 resources, added in 89.8193.37-13) holds 33 `.shd`. These **override** base copies. That matters because the retail `inc_standard` includes retail-only files.
  - Retail-only (new): `fsfbbloom, vsfbbloom, inc_debug, inc_displacement, inc_envmap, inc_fog, inc_framebuffer, inc_postpr_debug, inc_postpr_ssao, inc_postpr_toon, inc_random, inc_scriptable, inc_target, inc_tonemap, inc_transform, inc_uniforms`
  - Retail overrides base: `fs_cg_btn_col, fs_shadowplane, fsfbssao, fsparticle, fstc_sm, fswater, inc_common, inc_config, inc_keyhole, inc_lighting, inc_material, inc_postpr, inc_postpr_dyn_c, inc_standard, inc_water, vsgrass, vsparticle`
  - All notes below use the effective (retail-if-present) versions.
- Other `ovr.bif` content relevant to rendering: `skyboxes.2da` (updated), `nw_dynlight*.nss` / `nw_inc_dynlight.nss` (the scripted moving-sun system).
- Resman order applies: custom shaders in haks, override or NWSync can replace stock ones. The file name limit is 16 characters.

### A.2 Engine-injected preamble ([ENG], identical in `nwmain` and `nwtoolset.exe`)
```
#version 330 core            (mobile: #version 300 es + precision highp float)
#define mediump / lowp / highp   (empty)
#define MAX_NUM_LIGHTS %d        // = client "max lights" setting (3..128; default 32; user has 64)
#define MAX_NUM_BONES %d         // 64 (CL 8193.21: "Maximum number of bones of a single skinmesh is now 64")
#define GAMMA_CORRECTION %d      // "enhanced lighting" (graphics.lighting.enhanced)
#define FRAGMENT_LIGHTING %d     // per-pixel lighting (enhanced)
#define SHADER_QUALITY_MODE %d   // 0/1/2 = low/medium/high (graphics.general.shader-quality)
#define KEYHOLING_ENABLED %d
#define SHADER_DEBUG_MODE %d     // 0..18, see inc_common
#define BUILD_VERSION %s         // 8193
#define BUILD_REVISION %s        // 37
#define NO_DISCARD %d            // per-material variant: 1 = opaque (keeps early-Z), 0 = alpha-test/keyhole
#define POSTPROCESSING_TYPES_ENABLED %d  // bitmask, see H
#define varying out|in ; #define attribute in ; #define texture2D texture ; #define textureCube texture
FS only: #define gl_FragColor compat_glFragColor / out vec4 compat_glFragColor;
```
- `#include "name"` is a verbatim text splice ([WIKI], [ENG] `#include "%16[^"]"%n`). The stock includes use `#ifndef INC_X` guards.
- Compile error → the object renders with `vs_invalid`/`fs_invalid`, which is solid magenta `vec4(1,0,1,1)` ([SRC], [WIKI]).
- Shader types: `SHADER_TYPE 1` = vertex, `2` = fragment. Each `.shd` sets it; the default is 2 in `inc_common`.

### A.3 Stock program pairs the engine hard-codes ([ENG] name table in nwmain, in this order)
| VS | FS | Used for | Permutation (VS/FS defines) |
|---|---|---|---|
| `vs_invalid` | `fs_invalid` | compile failure fallback | magenta |
| `vsv` | `fs` | untextured, unlit geometry (fog on), colour from `globalColor` | NO_TEXTURE_COORDS / NO_TEXTURE, FOG 1 |
| `vsvt` | `fst` | textured unlit: old GUI panels, **skyboxes** ([WIKI]) | FOG 0, LIGHTING 0 |
| `vsvt_sm` | `fst_sm` | textured unlit + env map | ENVIRONMENT_MAP 1 |
| `vsvtc` | `fstc` | textured + vertex colour, unlit, no fog | VERTEX_COLOR 1 |
| `vsvc` | `fsc` | vertex colour only, fog | NO_TEXTURE, VERTEX_COLOR |
| `vslitnotex` | `fslit` | lit, no UVs | NO_TEXTURE_COORDS |
| `vslitnt_sm` | `fslit_sm` | lit, no UVs, env map | |
| **`vslit`** | **`fslit`** | **default lit textured mesh (tiles, placeables, creatures)** | LIGHTING 1, FOG 1, KEYHOLING 1 |
| `vslit_nm` | `fslit_nm` | lit + normal/spec/rough/height/self-illum maps (renderhint) | NORMAL_MAP, SPECULAR_MAP, ROUGHNESS_MAP, HEIGHT_MAP, SELF_ILLUMINATION_MAP = 1 |
| `vslit_sm` | `fslit_sm` | lit + environment map (TXI envmaptexture / appearance ENVMAP / bumpyshiny) | ENVIRONMENT_MAP 1 |
| `vslit_sm_nm` | `fslit_sm_nm` | lit + env + all PBR maps | all 1 |
| `vsglu` | `fsglu` | debug lines/solids (GL utility) | outputs `RevertColorSpace(globalColor)` |

Other programs loaded by name elsewhere ([ENG]):
- `vsgrass` + `fsgrass`: walkmesh grass.
- `vsparticle` + `fsparticle`: emitter particles. Chunk emitters do not use this pair ([WIKI]).
- `vswater` + `fswater`: "fancy/shiny water", chosen when the texture's TXI has `bumpmaptexture shinywater` and `graphics.water.shiny` is on. [INF] from [WIKI]+[DATA]: 22 base water TXIs carry this, and `fswater` sits beside `vswater` in the string table.
- `vs_shadowvol` + `fs_shadowvol` (stencil shadow volumes), `vs_shadowplane` + `fs_shadowplane` (shadow darkening plane), `vs_beamvol` + `fs_beamvol` (light-shaft "beaming" volumes).
- `vs_pltgen` + `fs_pltgen`: PLT → RGBA on the GPU (`graphics.experimental.generate-plt-with-shaders`).
- `vsfbpostpr` + `fsfbpostpr`: the single post-process pass, which always runs ([CL] .35).
- `vsFBSSAO` + `fsFBSSAO`, `vsFBBLOOM` + `fsFBBLOOM`: extra multi-pass post effects.
- `fs_cg_btn_col` (with `vsvt`): only through `gui_cg_color.mtr`, the chargen colour button.

Legacy / unreferenced, safe to ignore:
- `vslitc*`: byte-identical defines to `vslit*`.
- `vslit_sk`, `vslit_sk_nm`: same as `vslit_sm` / `vslit_sm_nm`. Skinning is a runtime uniform (`skinmesh`), not a permutation.
- `vswaterc`, `fsc_ls`, `fsc_sm`, `fstc_sm`.
- The old full-screen passes `fsfbdof/fsfbgam/fsfblvls/fsfbshrp/fsfbvib` and their `vsfb*`, superseded by `fsfbpostpr` + `inc_postpr_*`.

Toolset (per coordinator, from `nwtoolset.exe`): uses `vslit, vslit_nm, vslit_sm, vslit_sm_nm, vslitnotex, vslitnt_sm, vsv, vsvc, vsvt, vsvtc, vsvt_sm, vsglu, vsgrass, vsparticle, vs_beamvol, vs_invalid, vs_pltgen, vs_shadowplane, vs_shadowvol, vsfbpostpr` plus the matching `fs*`, **and "vso"**.
- **`vso.shd` does not exist in resman** (checked with `nwn_resman_grep -p vso`). It is dead or legacy, or falls back to `vs_invalid`.
- The toolset does NOT use: `vswater/fswater` (no fancy water), SSAO, bloom, DoF, sharpen, toon, dynamic contrast, `fs_cg_btn_col`, or the legacy `fsfb*`.
- The toolset does run `fsfbpostpr` (tonemap + gamma).
- The toolset has "Use Area Lighting" (`miUseAreaLighting`) and "Compute Static Lighting" (`actComputeStaticLighting`), and sets the sun through `controlpart %s gidy_sun_amb color %f %f %f` / `gidy_sun_diff` ([ENG]).

Naming key ([WIKI] + [SRC]):
- `v` = vertex position only
- `t` = texture coordinates
- `c` = vertex colour (VERTEX_COLOR)
- `lit` = LIGHTING
- `nt` / `notex` = no UVs
- `_sm` = **sphere / environment mapped** (ENVIRONMENT_MAP), not shadow and not specular
- `_nm` = normal map plus the other PBR maps
- `_sk` = skinned (legacy name only)
- `_ls` = long-distance fog
- `fb` = framebuffer post-process

### A.4 How a mesh's program is chosen
1. **MTR** (`materialname` in the MDL, or an MTR whose name matches `bitmap`/`texture0`) with `customshaderVS` + `customshaderFS`: both must be given ([WIKI]). `customShaderGS` also exists ([ENG]).
2. Otherwise the engine picks a stock pair from mesh traits:
   - lit or not
   - has UVs or not
   - environment map present (TXI `envmaptexture` / `bumpyshinytexture` / `cube`; appearance.2da `ENVMAP`; model envmap) → `_sm`
   - `renderhint NormalAndSpecMapped|NormalTangents` (MDL or MTR) → `_nm`, and tangents + handedness are generated for ASCII models
   - water TXI → `fswater`
   - TXI `decal 1` → unlit ([CL] .21: "decal property actually disable lighting")
   - GUI → `vsvt/fst`
   - [INF] the exact decision tree is engine code; the list above covers what shows in the data.
3. The old TXI way of naming shaders is deprecated ([WIKI] TXI/MTR).
4. Engine-side variant keys beyond the file: `NO_DISCARD` (per material: alpha-tested, keyholed, transparent), plus the global quality and lighting defines. Base-game MTRs (7 in total):
   - ice (`tti01_*`): `vslit_sm/fslit_sm` with Metallicness 0.001, Specularity 0.0179
   - old heads: Specularity 0.04, Roughness 0.65
   - `gui_cg_color`

### A.5 Permutation defines (complete list from [SRC])
Set in the `.shd` or MTR-selected file:
`SHADER_TYPE, LIGHTING, FOG, KEYHOLING, NO_TEXTURE, NO_TEXTURE_COORDS, NORMAL_MAP, SPECULAR_MAP, ROUGHNESS_MAP, HEIGHT_MAP, SELF_ILLUMINATION_MAP, ENVIRONMENT_MAP, VERTEX_COLOR, POSITION_VIEW, POSITION_WORLD, VERTEX_NORMAL, FRAGMENT_NORMAL, FORCE_VERTEX_LIGHTING, LIGHT_FORCE_INDIRECTIONAL, FORCE_ALLOW_DISCARD, POSTPROCESSING, MATERIAL_READ_ROUGHNESS_FROM_SPECULAR_MAP, MATERIAL_READ_SELF_ILLUMINATION_FROM_SPECULAR_MAP, MATERIAL_ROUGHNESS_VALUE / MATERIAL_SPECULARITY_VALUE / MATERIAL_METALLICNESS_VALUE / MATERIAL_SPECULAR_COLOR_VALUE (overrides), LIGHT_SUBSURFACE_SCATTERING(_WRAP), LIGHT_TRANSLUCENCE_VALUE, OCCLUSION_MULTIPLIER, OCCLUSION_MODIFIER, ALPHA_DISCARD_VALUE`.

Derived in `inc_common` / `inc_config`:
- `NORMAL_MAP` ⇒ `FRAGMENT_NORMAL=1`
- any of FOG / KEYHOLING / HEIGHT_MAP / ENVIRONMENT_MAP / LIGHTING ⇒ `POSITION_VIEW=1` and `VERTEX_NORMAL=1`
- `KEYHOLING_ENABLED==0` ⇒ `KEYHOLING=0`
- `FORCE_VERTEX_LIGHTING` ⇒ `FRAGMENT_LIGHTING=0`
- `SHADER_QUALITY_MODE==0` ⇒ `FRAGMENT_LIGHTING=0`
- **`SPECULAR_LIGHT = LIGHTING && (FRAGMENT_LIGHTING || FRAGMENT_NORMAL) && !POSTPROCESSING`**
- `NO_DISCARD==1 && !FORCE_ALLOW_DISCARD` ⇒ `#define discard` (empty)

`inc_config` constants:
| Define | Value |
|---|---|
| `COLOR_CORRECTION_TYPE` | **4** ("legacy balanced") |
| `SPECULAR_DISTRIBUTION_MODEL` | **1 (GGX)** |
| `LIGHT_SOFTENING_ENABLED` | quality>1 |
| `LIGHT_SOFTENING_VALUE` | 0.25 |
| `SPECULAR_GEOMETRIC_SHADOWING` | 2 (Schlick) at quality>1, else 0 |
| `SPECULAR_FRESNEL` | 1 (full, per light) at quality 2; 2 (simplified) at quality 1; 0 at quality 0 |

---------------------------------------------------------------------------------------------------

## B) Vertex attributes and uniforms

### B.1 Vertex attributes ([ENG] bind list; [SRC] usage)
| Attribute | Type | Source / meaning |
|---|---|---|
| `vPos` | vec4 | position. [WIKI]: since 1.89.8193.37 static geometry has position and orientation **baked into the vertex data in world space** ([CL] .37-13 "Baked static geometry position and orientation in to GPU vertex data"), so `m_mv` for such meshes is effectively the view matrix [INF] |
| `vTcIn` | vec2 | UV set 0 (`tverts`) |
| `vTcIn1..3` | vec2 | `tverts1..3` (since 1.74.8159). Not used by stock shaders. **Base tiles have no second UV set and no lightmaps** (checked tno01_a01_01, ttr01_a01_01) |
| `vColor` | vec4 | **engine-generated baked static lighting per vertex** (used when `staticLighting==1`, vertex-lit path). Not the MDL `colors` |
| `vCustomColor` | vec4 | MDL trimesh `colors` (RGB). Stock shaders ignore it |
| `vNormal` | vec3 | normal |
| `vTangent` | vec3 | tangent (renderhint / compiled TSB) |
| `fHandedness` | float | bitangent sign |
| `vIndex` | vec4 | 4 bone indices (float → int) |
| `vWeight` | vec4 | 4 bone weights (skipped when ≤0) |
| `vStaticLightDir` | vec3 | direction of the brightest static light at the vertex (object space; `mat3(m_mv)`-transformed). Normal-mapped static lighting ([CL] 1.74.8160) |
| `fProjectionFrontal` | float | shadow/beam volume vertex flag: >0 = cap / front vertex (offset 0.1), else extruded |

### B.2 Uniforms: transforms and screen (`inc_common`, `inc_transform`)
| Uniform | Meaning |
|---|---|
| `m_mvp` | used when `POSITION_VIEW==0` |
| `m_mv` | model→view (view-space lighting) |
| `m_proj`, `m_proj_inv`, `m_view`, `m_view_inv`, `m_vp_inv` | as named. **Lighting is done in VIEW space** |
| `m_m` | model→world (shadow/beam volume shaders) |
| `m_texture` | UV transform; **only `mat2(m_texture)` is used** (rotation/flip, no translation) → `rotatetexture` tile-UV compensation. Normal map XY rotated by `transpose(mat2(m_texture))` |
| `m_normal`, `m_vp` | exist, unused by stock shaders |
| `skinmesh` (int) | 1 = skinned |
| `m_bones[MAX_NUM_BONES]` | mat4. Mobile uses `m_boneRotations` / `m_bonePositions` (quat + pos). Skinning: `pos = Σ w_i · (m_bones[idx_i] · vPos)`, same for normal/tangent via `mat3`, 4 influences |
| `screenParams` = (w, h) | |
| `pixelParams` = (1/w, 1/h, aspect) | |
| `clipParams` = (near, far, 1 − near/far) | linear depth from the depth buffer: `z = near / (1 − d·(1 − near/far))` |
| `subvariant` (int) | multi-pass index |
| `bboxMin/bboxMax` | object-space mesh bounds (.37) |
| `userinputMousePosition` (ivec2), `userinputMouseButtons` | |

### B.3 Uniforms: lighting and material (`inc_lighting`, `inc_material`, `inc_envmap`)
| Uniform | Type | Meaning |
|---|---|---|
| `numLights` | int | lights in the per-draw arrays (≤ MAX_NUM_LIGHTS) |
| `lightPosition[i]` | vec4 | **view-space** light position (the shader does `lightPosition[i].xyz − vPosView`; the wiki's "world" is wrong) |
| `lightColor[i]` | vec4 | (GAMMA_CORRECTION=1) rgb = linear light colour, at most 1; **a = cutoff distance², negative ⇒ ambient-only light** (values: E.6) |
| `lightAmbient[i]`, `lightDiffuse[i]`, `lightQuadraticAtten[i]` | | legacy path (GAMMA_CORRECTION=0): a light is ambient-only when `lightAmbient.rgb ≠ 0` |
| `lightMaxIntensityInv` | float | 1 / lin(`graphics.lighting.max-intensity`) (E.6) |
| `lightFalloffFactor` | float | from `intensity-at-range` and the cutoff multiplier (E.6) |
| `lightCutoffRangeMultiplier` | | uploaded, not read by stock shaders ([ENG]) |
| `staticLighting` | int | 1 if the mesh has baked per-vertex static light (`vColor`, `vStaticLightDir`) |
| `lightAreaAmbient` | vec3 | sun/moon ambient |
| `lightAreaDiffuse` | vec3 | sun/moon diffuse |
| `lightAreaDiffuseDirection` | vec3 | direction **to** the sun/moon, view space [INF from use in N·L] |
| `materialFrontAmbient` | vec4 | MDL `ambient` (default 1,1,1; pre-.17 compiled models 0.2) |
| `materialFrontDiffuse` | vec4 | MDL `diffuse` (default 1,1,1; old 0.8) with **alpha = mesh `alpha`** |
| `materialFrontEmissive` | vec4 | MDL `selfillumcolor` (animatable). Debug modes `RevertColorSpace` these ⇒ they are uploaded **linear** |
| `materialFrontSpecular`, `materialFrontShininess` | | uploaded, unused |
| `frontLightModelProductSceneColor` | vec4 | GUI minimum light: `TotalLight = max(TotalLight, min(sceneColor·diffuse, 1))` |
| `globalColor` | vec4 | flat colour for fs/fst/fstc/glu/beams |
| `fAlphaDiscardValue` | float | alpha-test threshold. `discard` if `ALPHA_DISCARD_VALUE ≥ 0 && a ≤ value` |

`fAlphaDiscardValue` values ([WIKI]):
- default 0.2 (20%, the legacy minimum opacity)
- 0 when discards are off
- `sample_framebuffer`/`transparency` materials set it to 0
- TXI `blending punchthrough` gives a ≈0.5 cutoff [WIKI]

MTR-settable material uniforms (`parameter float/int NAME v…`; a float parameter with 4 values maps to vec4):
- `Specularity`, `Roughness`, `Metallicness` (a value > 0 overrides the map/derivation)
- `CustomSpecularColor` (vec4; black = derive)
- `DisplacementOffset`, `DisplacementMultiplier`
- any custom uniform name

Environment map uniforms:
- `texUnitEnv` (sampler2D), `texUnitEnvCube` (samplerCube)
- `envMapCube` (1 = cube), `texEnvBound`, `texEnvCubeBound`

### B.4 Uniforms: fog, keyhole, world, time (`inc_fog`, `inc_keyhole`, `inc_uniforms`, `inc_target`)
Fog:
- `fogParams` = (enabled, start, end, 1/(end − start))
- `fogColor` (vec4, **gamma space**)

Keyhole:
- `keyholeCanDissolve` (per object)
- `keyholeDiameterMin`, `keyholeDiameterMax` (fractions of the smaller screen dimension; settings 0.4 / 0.9)
- `keyholeMinZOffset` (setting 1.0 m)

Player and camera:
- `playerPosition`, `playerCameraDist`, `playerInCutscene`, `playerOrientation`
- `cameraPosition`, `cameraFocus`, `cameraPitch`, `cameraYaw`, `cameraViewAngle`, `cameraDialogState`

Time:
- `sceneCurrentFrame`
- `worldtimerTimeOfDay` (ms-ish client ticks)
- `moduleYear/Month/Day/Hour`, `moduleDawnHour`, `moduleDuskHour`, `moduleMinutesPerHour`
- `moduleTimeIntoTransition`, `moduleTransitionTime`

Area and weather:
- `areaFlags` (1 interior, 2 underground, 4 natural)
- `areaWeatherType` (0 clear, 1 rain, 2 snow), `areaWeatherDensity`
- `areaGlobalWind` (vec3, magnitude²: 1 medium, 4 strong)

Wind sources (`MAX_WINDS=32`):
- `windPointSourcesCount`
- `windPointSourcesPosition/Radius/Duration/Intensity/TimeRemaining[32]`

Targeting overlay:
- `targetShape`, `targetShapeSizeX/Y`, `targetShapeColour`, `targetWorldCoords`, `targetObjectType`, …
- Draws the spell AoE outline onto ground fragments in any `POSITION_VIEW` shader.

Script-driven:
- `scriptableFloat1..16`, `scriptableInt1..16`, `scriptableVec1..16` (per player; `inc_scriptable`)

### B.5 Uniforms: post / framebuffer
- `texFBColor`, `texFBDepth`
- `Gamma`, `Vibrance`, `DynamicContrastMidpoint`, `DynamicContrastIntensity`
- `DOFAmount`, `DOFDeadZone`, `DOFVignette`, `DOFNeverBlurPC`, `DOFFocusType`
- `AOIntensity` (0.5); `AORadius`, `AOColor` unused
- `ToonDepthEdgeThreshold`, `ToonColorEdgeThreshold`, `ToonEdgeHardness`
- `BloomThreshold`, `BloomSoftness`, `BloomMultiplier`, `BloomDownscaleOffset`, `BloomDownscaleSteps`
- Particles: `blendMode` (0 normal, 1 additive, 2 punchthrough), `particleSoftness`
- PLT: `PLTscheme[15]`
- Volumes: `projectionSource` (xyz = dir or pos; **w<0 ⇒ directional**, else w = radius), `projectionWorldZClip`

---------------------------------------------------------------------------------------------------

## C) Texture slots

| Unit | Sampler | Default meaning ([SRC] `inc_material`) | Colour space in shader |
|---|---|---|---|
| 0 | `texUnit0` | diffuse / albedo (+ alpha: opacity, or **1 − envmap strength** when ENVIRONMENT_MAP) | `ApplyColorSpace` (pow 2.2) |
| 1 | `texUnit1` | normal map, **RG only**, z reconstructed, OpenGL green-up; BC5-friendly ([CL] .20) | raw |
| 2 | `texUnit2` | specularity (R). Optionally roughness (G) and self-illumination (B) via defines | raw |
| 3 | `texUnit3` | roughness (R) | raw |
| 4 | `texUnit4` | height (R). White = surface, darker = deeper; parallax + occlusion | raw |
| 5 | `texUnit5` | self-illumination (RGB) | pow 2.2 |
| 6–10 | `texUnit6..10` | free for custom shaders (MTR `texture6..10`) | — |
| 11/12 | `texFBColor` / `texFBDepth` | framebuffer colour/depth. **Sources disagree on which is 11 and which is 12**: Shader Engine Support says 11 = colour, 12 = depth; the MTR page says the reverse | colour is gamma-space |
| 13 | `texUnitEnvCube` | cube env map | pow 2.2 |
| 14 | `texUnitEnv` | 2D (sphere-ish) env map | pow 2.2 |
| 15 | `texUnitNoise` | noise: **`solid_noise.dds`** (bd_misc.bif, 1024², BC4/ATI1, 11 mips) [ENG]+[DATA] | raw |

- `textureNBound` (int) exists per unit 0–10. Stock code checks 0–5 (a "not bound" slot may hold stale data [WIKI]).
- MTR `textureN` overrides MDL `bitmap`/`textureN`. `null` skips a slot. An MTR takes precedence over TGA/DDS/PLT of the same name.
- For PLT, omit `texture0` in the MTR.

Env map sources:
- TXI `envmaptexture X` / `bumpyshinytexture X` (on the base texture's TXI)
- appearance/wingmodel/tailmodel.2da `ENVMAP`
- tileset `.set` `[GENERAL] EnvMap=` (area default, e.g. `tno01.set: EnvMap=ttr01__ref01`)
- fallback **`chrome1`** (TGA 64×64, and a legacy BioWare-format DDS) ([ENG], [CL] .26)
- Env maps are always bound for SPECULAR_LIGHT shaders; the code never checks `texEnvBound`.

Cubemaps:
- The TXI has `cube 1` + `filerange 6` → six files `name0..name5` (e.g. `tno01__env0..5.tga/.dds`, 64² TGA).
- The face order is not documented [INF: GL order +X,−X,+Y,−Y,+Z,−Z; CL .26 "Fixed GL CubeMap load order"].

Texture formats:
- TGA
- DDS: the BioWare legacy variant (header: w, h, bpp, size, alphamean) plus standard DDS since v78; BC4/BC5 since .14
- KTX (v78)
- PLT (layered; GPU generation via `fs_pltgen`, see J)

TXI keywords seen in base data ([DATA] tally):
- `downsamplemax/min`, `mipmap`, `filter`, `alphamean` (159)
- `envmaptexture` (50), `proceduretype` (arturo 38, cycle 7, water 4), `blending` (default/additive/punchthrough/normal)
- `bumpmaptexture shinywater` (22), `bumpyshinytexture` (33), `decal` (21)
- `cube` (7), `numx/numy/fps` (flipbooks), `clamp`, `distort` etc.

---------------------------------------------------------------------------------------------------

## D) Lighting equations (as implemented in `inc_lighting` / `inc_material` / `inc_standard`)

Notation:
- All vectors are **view space**. `N` = shading normal (fragment normal or vertex normal).
- `V = −normalize(posView)` (the shader uses `vViewToSurface_n = normalize(posView)`).
- `L` = unit vector surface→light.
- `lin(x) = sign(x)·|x|^2.2`, `gam(x) = x^(1/2.2)`. Only when `GAMMA_CORRECTION==1`; otherwise both are the identity.

### D.1 Frame of a lit fragment (`fslit*`, FRAGMENT_LIGHTING=1 path)
```
FragmentColor = (1,1,1,1)
a = clamp(1 * materialFrontDiffuse.a, 0, 1)
tex = texture0 bound ? texture(texUnit0, uv) : white
   (ENVIRONMENT_MAP: envLevel = 1 - tex.a; with fragment lighting tex.rgb = mix(tex.rgb, 0.6667, envLevel); unbound → grey 0.6667, envLevel=1)
tex = lin(tex)
FragmentColor *= tex   (non-env: also alpha; then AlphaDiscard(a))   (env: rgb only)
N = normalize(frontFacing ? vN : -vN)                // two-sided
if normal map:  n.xy = tex1.rg*2-1; n.z = sqrt(max(1-dot(n.xy,n.xy),0)); n.xy = transpose(mat2(m_texture))*n.xy; N = TSB * n
   TSB = [T=normalize(vTangent), B=cross(Nsurf,T)*(handedness>=0?+1:-1), Nsurf]
if height map:  uv = parallax(uv) (D.6); AO from height (D.6)
SetupSpecularity(albedo = FragmentColor.rgb * materialFrontDiffuse.rgb)   // D.3
ComputeLighting(FragmentColor, N)                                        // D.2
ApplyTargetGroundHighlighting; ApplyKeyhole                              // I
FragmentColor.rgb = gam(FragmentColor.rgb)
ApplyFog (in gamma space)                                                // F
```

### D.2 `ComputeLighting(Color, N)`
```
Ambient = lightAreaAmbient;  Diffuse = 0;  Specular = 0;  Static = 0 (vertex-lit only)
if SPECULAR_LIGHT:
   NdotV = dot(N, V); if NdotV<0 { N = normalize(N - NdotV*viewDir); NdotV = 0 }   // bend back-facing normals
   r = roughness; α² := r²  (fRoughness_sq); k := r (geometric term, SPECULAR_GEOMETRIC_SHADOWING_TERM 0)
Diffuse(area):  AddDirectional(lightAreaDiffuse, L = lightAreaDiffuseDirection, att = 1)
for i < numLights:
   GAMMA path:  d² = |lightPos_i - P|²;  R² = |lightColor_i.a|;  if d² > R²: skip
                f = d²/R²;  att = (1 - f) / (lightMaxIntensityInv + lightFalloffFactor * f)
                if lightColor_i.a < 0 → Ambient += rgb*att   (ambient-only light; no N·L)
                else AddDirectional(rgb, L = normalize(lightPos-P), att)
   Legacy path: att = 1 / (1 + lightQuadraticAtten_i * d²), ambient if lightAmbient_i != 0

AddDirectional(C, L, att):
   NdotL = dot(N, L)
   [quality 2 && alpha<1: translucence  Diffuse += C*att*(-NdotL)*(1-alpha)  when NdotL<0]   ("backlit transparent", CL .35)
   if light softening (quality 2, no fragment normal):  accept NdotL > -0.25;
        att *= 2*smoothstep(0, 2, NdotL*0.8 + 0.2); NdotL = max(NdotL,0)
   else: require NdotL > 0; att *= NdotL
   [normal/height mapped, quality 2: att *= clamp(dot(Nsurface, L) * fSurfaceFade, 0, 1); fSurfaceFade = 10 (or AO-driven)]
   Diffuse += C*att            (vertex-lit + normal map: static light goes to Static instead)
   if SPECULAR_LIGHT:
      VdotH = sqrt(dot(V, L)*0.5 + 0.5);  NdotH = clamp((NdotL + NdotV)*0.5 / VdotH, 0, 1)
      Specular += C*att * GetSpecularIntensity(NdotH, VdotH, NdotL)
```
GGX (`SPECULAR_DISTRIBUTION_MODEL 1`):
```
den = NdotH²·(α² − 1) + 1            // α² = r²  → roughness value is used directly as GGX α
per-light:  1/den² · [q2: 1/(NdotL·(1−k)+k)] · [SPECULAR_FRESNEL 1: F(VdotH)]
after loop: Specular *= (α²·0.25) · [q2: 1/(NdotV·(1−k)+k)]
F(c) = mix(spec0, 1, (1−c)^5)   (quality 2; (1−c)^4 at quality ≤1)
```
This is standard Cook-Torrance GGX + Schlick-G with k = r, **without the 1/π**. Diffuse is also plain Lambert without 1/π. Blinn-Phong (shininess `2/(α²+0.001) − 2`) and Beckmann exist but are compiled out.

After the loop:
```
Ambient *= materialFrontAmbient.rgb
Diffuse  = (Diffuse + Static) * materialFrontDiffuse.rgb          (grass: × 1/π, see J)
if SPECULAR_LIGHT:
   Env = Ambient + Diffuse                                        // "naive GI": env reflection is lit by local irradiance
   Specular *= base modifier (above)
   envSpec = SPECULAR_FRESNEL!=0 ? mix(F(NdotV), spec0, sqrt(r)) : spec0
   if SPECULAR_FRESNEL != 1: Specular *= envSpec                  // quality 1/0: fresnel only via env term
   lod = clamp(r*30 − 1, 0, 10)   (cube: clamp to 0 → no roughness blur on cubemaps)
   envSample = lin(texture(env, coords, lod))                     // D.5
   Specular += Env * envSample * [AO] * envSpec
if HEIGHT_MAP: Ambient *= AO
Total = materialFrontEmissive.rgb + (1 − envSpec) * (Ambient + Diffuse)
Total = max(Total, min(frontLightModelProductSceneColor.rgb * materialFrontDiffuse.rgb, 1))
[SELF_ILLUM from spec-map B: Total = mix(Total, 1, tex2.b)]
if !GAMMA_CORRECTION: Total = clamp(Total, 0, 1)                   // legacy LDR; enhanced = unclamped (HDR)
Color.rgb *= Total
[SELF_ILLUMINATION_MAP: Color.rgb += lin(tex5.rgb)]
if SPECULAR_LIGHT: if 0.001<a<1: Specular /= max(a, 0.1)           // spec ignores transparency (CL .20)
                   Color.rgb += SpecularColor * Specular
```

### D.3 Material value generation (`SetupSpecularity`, fragment)
Constants: `specMin=0.04`, `roughMin=0.125` (GGX), `roughMax=0.55`, legacy env factors ×8 / max 0.98 / ×2.5.

- **Specularity (spec0)**:
  - MTR `Specularity>0`
  - else texture2.r if bound
  - else if ENVIRONMENT_MAP: `mix(0.04, 0.98, min(envLevel·8, 1))`
  - else `0.04`
- **Roughness**:
  - MTR `Roughness>0`
  - else texture3.r (or texture2.g with the define)
  - else if spec map: `mix(0.125, 0.55, (1−spec)²)` [and with env: `min(that, mix(0.55, 0.125, min(envLevel·2.5, 1)))`]
  - else if env: `mix(0.55, 0.125, min(envLevel·2.5, 1))`
  - else **0.55**
- **Metallicness**:
  - MTR `Metallicness>0`
  - else with a spec map: `m = spec²; m = clamp(m/(0.04 + 0.96·m), 0, 1)`
  - else `clamp(10·spec − 0.4, 0, 1)` (spec 0.04 → 0, ≥0.14 → 1)
- **Specular colour**:
  - `CustomSpecularColor` (lin) if non-black
  - else (gamma mode) `mix(vec3(min(1, maxChannel(albedo)/lin(0.5))), albedo, metal)`; lin(0.5) = 0.2176, so non-metals get white spec scaled by albedo brightness
  - height-mapped: `mix(1, albedo, metal)`
  - legacy: `albedo`
- The wiki's metals guidance: specularity 0.9–1.0 with the colour from the base texture. Non-metals 0.02–0.05 (water 0.02, glass 0.04, skin 0.028, ice 0.018) [WIKI].

### D.4 Vertex-lit path (FRAGMENT_LIGHTING=0: quality 0, or "enhanced lighting" off)
- VS computes `ComputeLighting(white, vertexNormal)` → `VertexColor` (no specular, since SPECULAR_LIGHT=0).
  - Baked static lights come in as `StaticLight = lin(vColor.rgb)` when `staticLighting==1`.
  - Dynamic lights are still looped per vertex.
- FS: `FragmentColor = tex * VertexColor`.
  - With a self-illumination map: `rgb *= rgb * (VertexColor + selfIllum.r)` (note: squares the texture; stock quirk).
- With a normal map (FRAGMENT_NORMAL=1) lighting runs per fragment anyway. Static light is reapplied as a directional light: colour `lin(vColor)`, direction `vStaticLightDir`.
- ENVIRONMENT_MAP without SPECULAR_LIGHT: `tex.rgb = mix(tex.rgb, envSample, envLevel)` (the classic 1.69 chrome look).

### D.5 Environment coordinates
- **Cube**: `dir = mat3(m_view_inv) * reflect(viewDir, N)` (world space). Sampled with LOD 0 only.
- **2D ("sphere") map**, view-space formula:
  ```
  e  = N.xy/(1+|N.z|) + 0.5 * posView.xy/(−posView.z + 0.2)
  uv = 0.5*e + vec2(0.5, 0.25)
  ```
  Computed per vertex unless FRAGMENT_NORMAL.

### D.6 Height map (`inc_displacement`, `inc_material`)
- **Parallax**: iterative search. Iterations = `16` (quality ≤1) or `32` (quality 2), × `DisplacementMultiplier` (default 1), scaled by view angle, distance and screen height/1080.
  - Base step `0.05·(view.xy in tangent space)·((1−v.z)/((0.5v.z+0.5)²)+1)`.
  - Start offset `DisplacementOffset`. Height read as `1 − h`.
- **AO**: `occ = clamp(h_blurred(mip bias 7 − log2(depth)) − h, 0, 0.9)`; `AO = (1−occ)²`.
  - `fSurfaceFade = 1/mix(1, 0.1, AO²)`.
  - AO multiplies Ambient and the env-specular term.

### D.7 Quality-mode differences (summary)
| | q0 | q1 | q2 |
|---|---|---|---|
| lighting | vertex only | per-pixel if enhanced | per-pixel |
| Fresnel | none | pow4, env-only | pow5, per light |
| geometric shadowing | none | none | Schlick |
| light softening / translucence | off | off | on |
| parallax iterations | 16 | 16 | 32 |
| water | simple | refraction | + SSR + subsurface |

---------------------------------------------------------------------------------------------------

## E) Light sources: how area, tile and MDL data become shader lights

### E.1 Area sun and moon (ARE)
ARE fields ([DATA] Chapter1.nwm AREs; confirmed against environment.2da):
- `SunAmbientColor`, `SunDiffuseColor`, `SunFogColor`, `MoonAmbientColor`, `MoonDiffuseColor`, `MoonFogColor`: **DWORD 0x00BBGGRR** (BGR; e.g. `MoonDiffuseColor 0x628A87` = RGB 135,138,98 = environment.2da InteriorNormal DARK_DIFF). nwscript takes 0xRRGGBB.
- `SunFogAmount`/`MoonFogAmount`: BYTE, 0–15 seen.
- `SunShadows`/`MoonShadows`: BYTE.
- `ShadowOpacity`: BYTE 0–100 (environment.2da `SHADOW_ALPHA` 0–0.6).
- `FogClipDist`: FLOAT, default 45.
- `DayNightCycle`, `IsNight`: BYTE.
- `LightingScheme`: environment.2da row, toolset preset only.
- `SkyBox`: skyboxes.2da row.

Mapping to shaders:
- EE sends sun/moon as dedicated uniforms `lightAreaAmbient`, `lightAreaDiffuse`, `lightAreaDiffuseDirection` ([CL] .21 "Separated out area wide lighting from area point lights").
- Selection:
  - `DayNightCycle=1`: day → Sun set, night → Moon set, with dawn/dusk transitions.
  - Otherwise `IsNight` picks Sun ("Always Bright") or Moon ("Always Dark").
- Scripts can fade colours and direction (`SetAreaLightColor`, `SetAreaLightDirection`; `nw_dynlight` moving sun, .35).
- The legacy/toolset form is `gidy_sun.mdl` ([DATA] ASCII):
  - light `gidy_sun_amb`: ambientonly, radius 100000, colour .31/.35/.43, priority 1
  - light `gidy_sun_diff`: shadow 1, radius 100000, colour 1/1/1
  - the engine and toolset overwrite both colours via `controlpart sun gidy_sun_amb|gidy_sun_diff color r g b` ([ENG])
  - [WIKI] places it at (4000, 4500, 7000); the legacy `fsfblvls` detects "area light" as `lightPosition.z > 6999.9`
  - The default direction is normalize(4000, 4500, 7000) = (0.4332, 0.4874, 0.7581) for both sun and moon: `GetAreaLightDirection` in a new area returns it (engine test `engine_area_light.rs`).
- [INF] colours reach the shader as ARE/255, linearised (pow 2.2) in enhanced mode. Evidence: all other colour uniforms are linear, the fog colour is explicitly "always gamma".

### E.2 Tile main lights (`Tile_MainLight1/2`, BYTE)
- Index 0–31 into **lightcolor.2da** (`TILE_MAIN_LIGHT_COLOR_*` 0..31; 0 = Black = off).
- Applied to the tile model's light nodes `<tile>ml1` / `<tile>ml2` (case-insensitive). No node ⇒ the toolset greys out the control. The MDL `color` and `radius` are ignored (radius 10 for ml1, 5 for ml2, verified in E.6); the multiplier is kept [INF].
- Base tiles checked:
  - `tno01_a01_01`: ml1 radius 14, ml2 radius 5
  - `ttr01_a01_01`: ml1 radius 14
  - all: `isdynamic 0`, `shadow 0`, `affectdynamic 1`, `lightpriority 5`, `fadinglight 1`, `multiplier 1`
- [WIKI] the engine forces ml1 radius 10 / shadowradius 12 and ml2 radius 5 / shadowradius 8, priority 4, shadow 1. The radii are verified (E.6); the rest is not.
- lightcolor.2da `RED/GREEN/BLUE` go up to 2.4 (e.g. 3 BrightWhite = 2.0; 18 PaleBlue = 1.4/1.4/2.4); above 1 they lengthen the light's range, not its brightness (E.6). `TOOLSETRED/GREEN/BLUE` (≤1) are UI swatch colours: the toolset reads both sets, the game reads RGB ([ENG]).
- Enhanced mode: they are ordinary point lights in the per-draw array. Static (non-dynamic) tile lights are not recomputed per frame ([CL] .15).
- Vertex mode: they are **baked per vertex** into `vColor` + `vStaticLightDir` by `ComputeStaticLighting` ([ENG]; [CL] "Toolset: Update static lighting after changing tile light properties").

### E.3 Tile source lights (`Tile_SrcLight1/2`, BYTE 0–15)
- Spawn **`fx_flame01.mdl`** at the dummy `<tile>sl1`/`sl2` and play the animation named by the value (1..15). 0 = none.
- The light colour comes from fx_flame01's per-animation `AuroraLight01` colorkey. [DATA]: anims 2..15 = lightcolor.2da row `2n`; anim 1 = (2,2,2) rather than row 2 = 1.2. `TILE_SOURCE_LIGHT_COLOR_*` n ↔ lightcolor row 2n ([WIKI]).
- fx_flame01 light: radius 7, shadowradius 15 (key duplicated), shadow 1, isdynamic 0, priority 4, flare 30.
- Also emitters Flame/REAL_Flame01. Editing lightcolor.2da does not change source-light colours ([WIKI]).

### E.4 Placeable lights
- placeables.2da `LightColor` (index into lightcolor.2da) + `LightOffsetX/Y/Z` ([ENG] `AddPlaceableObjectLight`) → spawns `fx_placeable01.mdl`: radius 10, shadowradius 15, verticaldisplacement 3, priority 4, shadow 1, flare.
- `SetPlaceableIllumination` toggles it.

### E.5 MDL light-node fields ([WIKI] MDL ASCII)
| Field | Meaning |
|---|---|
| `radius` | range in metres; the cutoff is range × `graphics.lighting.cutoff-range-multiplier` (2.0) × the colour's intensity (E.6) |
| `multiplier` | intensity. Not verified in EE (Moonglow multiplies the colour, so it lengthens the range above 1) |
| `color` | light colour |
| `ambientonly` | → negative `lightColor.a`, no N·L |
| `nDynamicType` / `isdynamic` | 0 = static (bakeable), 1 = dynamic |
| `affectdynamic` | affects dynamic objects (creatures) |
| `shadow` | can cast shadows |
| `shadowradius` | default = radius; shadow fades to transparent at 50% |
| `lightpriority` | 1–5 (1 sun/moon, 2 torches/light spells, 3 spells, 4 tile lights, 5 others); used when trimming to MAX_NUM_LIGHTS |
| `fadinglight` | 1.5 s fade in/out |
| `verticaldisplacement` | moves the shadow-projection origin |
| `generateflare`, `flare*` | lens flares |

Player light: always one, from progfx.2da (darkvision / low-light / default).

### E.6 Uploaded light values (read back from the client)
Verified by `client_render.rs` `light_uniforms_match_the_client`: a debug copy of `inc_standard.shd` in the user `override` folder paints the uniforms into the image. Game 89.8193.37-17, default settings (`max-intensity 1.5`, `intensity-at-range 0.2`, `cutoff-range-multiplier 2.0`).
```
lightMaxIntensityInv = 1 / lin(1.5)                              = 0.4098
lightFalloffFactor   = m² · (1/lin(0.2) − 1/lin(1.5)),  m = 2   = 136.33
   → the denominator is 1/lin(0.2) where d = cutoff/m (the light's radius); lin(x) = x^2.2
per light, colour c (MDL colour × multiplier, or lightcolor.2da RED/GREEN/BLUE):
   intensity    = max(1, max channel of c)
   lightColor   = lin(c / intensity)                  // at most 1 per channel
   lightColor.a = ±(radius · m · intensity)²          // brighter colours reach further instead
tile main lights: radius 10 (ml1) / 5 (ml2), whatever the MDL says (tic01 ml1 has 14)
source lights: fx_flame01's radius 7, colour from the animation's colour key
area: lightAreaAmbient/Diffuse = lin(ARE colour), 0x00BBGGRR
```
Examples: White 1.2 → colour 1, cutoff 24 m; BrightWhite 2.0 → 1, 40 m; DimWhite 0.6 → 0.325, 20 m; Yellow (1.9, 1.7, 0.06) → (1, 0.783, 0.0005), 38 m.

The earlier reconstruction (att(0) = 1.5, k ≈ 12.33, colours uploaded as-is) made tile lights ~10× too bright.

---------------------------------------------------------------------------------------------------

## F) Fog
- **Linear, view-depth based**: `f = clamp((−posView.z − fogStart) / (fogEnd − fogStart), 0, 1)`.
  - Computed per vertex (unclamped) and interpolated. The depth is planar, not radial.
  - Applied **after** gamma revert: `rgb = mix(rgb, fogColor.rgb, f)`.
  - Alpha is untouched, except additive particles and beams, where `a *= 1 − f`.
- Uniforms: `fogParams`, `fogColor` (gamma space: ARE Sun/MoonFogColor as-is [INF]). The legacy `fogMode` (exp/exp2) is gone.
- `fogEnd` ≈ `FogClipDist`, which is also the far clip / render distance for tiles and static placeables.
  - A skybox adds **+90 m** to the render distance ([WIKI]).
  - Dynamic placeables and doors: 45 m; creatures: 35 m.
- `fogStart` comes from `Sun/MoonFogAmount` (0–15; higher = fog closer). **The exact formula is unknown.** Console `fogstart/fogend` exist.
- Skybox:
  - drawn with `vsvt/fst` (FOG 0 → never fogged), centred on the player
  - **`skyfade1.mdl`** (texture `skyblurpoly.tga`, colour = how much fog, alpha = how much sky shows) blends fog colour over the horizon and below
  - skyboxes.2da DAWN/DAY/DUSK/NIGHT models cross-fade over 1 hour from the dawn/dusk hour
- Water SSR, SSAO and bloom fade with fog. The dynamic-contrast midpoint mixes toward the fog colour.

---------------------------------------------------------------------------------------------------

## G) Shadows
- **Stencil shadow volumes** plus a darkening **shadow plane**; no shadow maps ([WIKI] Model Shadows; [SRC]).
- **Volumes** (`vs_shadowvol`; the FS is a debug colour only):
  - Extrusion is per vertex on the GPU. Direction = light dir (`projectionSource.w<0`, directional/sun) or from the point light.
  - Front/cap vertices (`fProjectionFrontal>0`) move 0.1 m. Others extrude to the world Z plane `projectionWorldZClip` (tile bbox bottom, [CL] .31) for directional lights, or to the light radius for point lights.
  - Sun shadows never render above `max(camera.z, focus.z)`.
  - Clamped to the near/far planes (frustum clipping in the VS, [CL] .31).
  - Volumes come from `shadow 1` trimeshes (a `render 0` low-poly shadow mesh is common). Edges must be 2-manifold, or you get "shadow tearing".
- **Plane** (`vs_shadowplane/fs_shadowplane`):
  - Covers the stencil-marked pixels. `VertexColor = lin(vColor)` supplies the colour/alpha [INF: black with alpha = ShadowOpacity/100].
  - Alpha fades toward the top of the screen (fog end / far clip, from camera pitch) and with fog: `a = clamp(min(fade, a·(1−fog)), 0, 1)`.
- **Which lights cast**:
  - Area sun/moon when `SunShadows`/`MoonShadows`; the sun is the only caster onto static geometry (tiles, static placeables) via precomputed static projections (`ComputeStaticProjections`).
  - Up to `graphics.shadows.max-casting-lights` (≤3) dynamic lights with `shadow 1` and a big enough radius (≥10 m for PC lights [WIKI]).
  - Shadow opacity fades over `shadowRadius` and with the height between the plane and the source ([CL] .31).
- Casters:
  - Classification `character` casts dynamic shadows.
  - Other classifications cast only if not transparent ([CL] .35), plus `graphics.shadows.all-types-can-cast-dynamic`.
- **Beaming** (`beaming 1` meshes, e.g. forest sunbeams): the same extrusion (`vs_beamvol`), drawn as translucent volumes in `globalColor` with fog alpha.
- A renderer can skip all of this initially. The legacy look is "blob-ish hard stencil shadows darkened by a constant alpha".

---------------------------------------------------------------------------------------------------

## H) Gamma, sRGB, tonemap and post chain
- **No GL sRGB formats or FBOs**; conversion is manual pow 2.2 in shaders (`ApplyColorSpace`/`RevertColorSpace`, sign-preserving).
  - Enhanced mode: diffuse, self-illumination and env textures are linearised. Normal/spec/rough/height maps stay raw.
  - Lighting is computed linear and HDR (unclamped). The output is reverted to gamma **before** fog, and written in gamma space.
  - Legacy mode: no conversion, and total light is clamped to [0,1].
- The framebuffer can hold values > 1 (bloom reads up to 10). HDR framebuffers when bloom is on (`g_nHDRFramebuffers`) [INF: RGBA16F].
- **Post pass** (`fsfbpostpr`, always runs [CL] .35). Order:
  1. Sharpen
  2. DoF
  3. Toon
  4. Dynamic contrast ("high contrast")
  5. Vibrance
  6. Gamma
  7. **ApplyColorTonemap**
  8. debug

  Bitmask: `SHARPEN 0x1, DOF 0x2, DYNAMIC_CONTRAST 0x4, VIBRANCE 0x8, GAMMA 0x10, TOON 0x20`.
  - **Gamma**: `rgb = pow(rgb, Gamma/2.2)`. The default 2.2 is the identity; the bit is probably set only when ≠ 2.2. A higher value darkens (exponent > 1).
  - **Tonemap**, `COLOR_CORRECTION_TYPE 4` → `ColorClamp`, the default "legacy balanced" branch:
    ```
    if max(rgb) = M > 1:
       if a<1: {a *= M; s = 1/M; if a>1 {s *= a; a = 1}; rgb *= s; M *= s}   // overflow into opacity
       rgb = M − (M − rgb)·(1 − ((M−1)/M)²)                                  // desaturate toward white; framebuffer clamps
    ```
    Type 2 (ACES, 2.51/0.03/2.43/0.59/0.14) exists but is not selected.
  - **Dynamic contrast**:
    ```
    VS: mid = luma(DynamicContrastMidpoint * gam(0.5*areaAmbient + 0.5*areaDiffuse*(0.5 + ...) + Σ light terms)), blended toward the fog colour
    FS: rgb *= 1 + (luma − mid) * DynamicContrastIntensity * max(0, 1 − bright²) / max(bright, 0.2)
    ```
    Defaults: intensity 0.15, midpoint 0.4.
  - **Vibrance** (0.7): pushes low-saturation pixels, keeps luma.
  - **Sharpen**: luma-edge darkening + cheap AA + a slight brighten, depth- and fog-scaled.
  - **DoF**: focus from camera pitch/player/mouse.
  - **Toon**: Sobel on depth and colour.
- **SSAO** (separate `fsFBSSAO`, half resolution, [CL] .36):
  - 3 subvariants: compute → two diagonal blurs. The multiplier `1 + delta`, with occlusion ×0.15 darkness and "exposure highlight" ×0.075, × `AOIntensity`.
  - Fades with fog; skipped for depth < 0.135.
  - [INF] composited multiplicatively.
- **Bloom** (`fsFBBLOOM`, subvariants threshold/downscale/upscale/apply):
  - threshold on max channel; normalised geometric sum; alpha = blend factor
  - "prevent darkening" max
- Settings (defaults, [ENG] + user settings.tml): `graphics.gamma 2.2`, `fbo.ssao/sharpen/vibrance/high-contrast/hdr-bloom` on, `dof/toon` off, `anti-aliasing` (MSAA), `anisotropic-filtering`.
- **For a toolset renderer**: render linear, apply `gam()` at output, fog in gamma space, clamp. Optionally add ColorClamp for > 1 values. Skip the other post effects.

---------------------------------------------------------------------------------------------------

## I) Keyholing (camera-to-player cutaway)
Enabled by `KEYHOLING_ENABLED`, the per-object `keyholeCanDissolve==1` (engine decides: tiles and blocking objects), and `NO_DISCARD==0`.
```
VS: camOffset = −posView.z − playerCameraDist + (90 − cameraPitch)/45        (only when keyholeCanDissolve)
FS (if camOffset < 0, i.e. fragment is in front of the player):
  Rk² = sqr(clamp(−(Dmax−Dmin)·Dmin·12.5/posView.z + Dmin, Dmin, Dmax))   // Dmin=0.4, Dmax=0.9 (settings)
  c   = (2·fragCoord/screen − 1), aspect-corrected so the shorter axis spans [−1, 1]
  if |c|² < Rk²:
     noise = simplex3D(worldPos)*0.5+0.5
     lvl = (1 − |c|²/Rk² − 0.35·noise + 0.5/camOffset) · clamp(worldPos.z − keyholeMinZOffset − player.z, 0, 1)   // minZ = 1.0
     lvl ≥ 0.3 → discard ;  0 < lvl < 0.3 → rgb *= 1 − 0.8·(lvl/0.3)²  (dark rim)
```
- In stock VS: `vslit*`, `vsgrass`, `vswater`. Keyholed parts render in the transparent pass ([CL] .37).
- Separate from the MDL `tilefade` (0 none, 1 fade, 2 base, 3 neighbour), which is engine-side mesh hiding.
- A toolset camera does not need either.

---------------------------------------------------------------------------------------------------

## J) Special surfaces

### Water (`vswater` + `fswater` via `inc_water`)
Triggered by a texture TXI with `bumpmaptexture shinywater` (+ `bumpyshinytexture <tileset>__env` cubemap, + `proceduretype arturo distort 1` for the legacy look). Forced settings: `FRAGMENT_LIGHTING 1`, `ENVIRONMENT_MAP 1`, `POSITION_WORLD 1`, spec colour white, metal 0.
- `WaterBaseColor = lin(texture(texUnit0, (0.5,0.5), bias 20))`: the smallest mip, i.e. average colour + alpha = opacity.
- specularity 0.02; roughness `0.00025 + 0.00025·dist` (GGX).
- **Waves**:
  - noise texture `texUnitNoise` (solid_noise) sampled at several scales/directions
  - main wave from `areaGlobalWind` (force = |wind|² + 0.05; default dir (0.707, 0.707))
  - local ripples from `windPointSources*`
  - time `fTick = worldtimerTimeOfDay·0.001`
  - normals by finite differences (offset 0.05 m); amplitude suppressed on non-horizontal surfaces
- **Quality ≥1**: screen-space refraction (IOR 1.333, 5 steps, 4 m range) from `texFBColor/texFBDepth`. Depth-based transparency: `exp2(−depth·0.25·a/(1−a))`.
- **Quality 2**: plus screen-space reflection (10 steps), a subsurface term (`LIGHT_SUBSURFACE_SCATTERING`, wrap 1.0), Fresnel mix.
- **Quality 0**: plain lit texture.
- Keyhole + fog applied.
- Toolset: water renders with the ordinary shaders (no `fswater`). Legacy TXI procedural distortion (arturo, 256×256 limit) is CPU-side.

### Grass (`vsgrass/fsgrass`)
- `FORCE_VERTEX_LIGHTING`, `LIGHT_FORCE_INDIRECTIONAL`: lights add `C·att` with no N·L, then diffuse × 1/π. No specular. FS = texture × vertex light. Fog + keyhole.
- Engine-generated quads on walkmesh faces with material 3 (grass), sorted by distance, fading in, render distance `graphics.grass.render-distance` (900).
- Wind sway is CPU-side [INF].
- Tileset `.set [GRASS]`:
  - `Grass=1`
  - `GrassTextureName` (default `grass`)
  - `Density` (e.g. 3–5)
  - `Height` (0.5–0.8 m)
  - `AmbientRed/Green/Blue`, `DiffuseRed/Green/Blue` (0–1) → grass `materialFrontAmbient/Diffuse` [INF; CL .26 "Fixed setting ambient material for grass"]
- Scripts: `SetAreaGrassOverride`.

### Environment maps
See C and D.5.
- Legacy content: diffuse alpha encodes `1 − reflectivity`. It drives specularity and roughness in enhanced mode; in vertex mode it lerps.
- All SPECULAR_LIGHT materials reflect the bound env map (area/tileset default or chrome1), scaled by the surface's own irradiance (Ambient + Diffuse).

### Self-illumination
- MDL `selfillumcolor` → `materialFrontEmissive`: added to total light before multiplying by albedo, i.e. it tints the texture.
- `texture5` map: added after lighting (linearised).
- Or `texture2.b` with `MATERIAL_READ_SELF_ILLUMINATION_FROM_SPECULAR_MAP`: mixes total light toward 1.

### Transparency
- Mesh `alpha` → `materialFrontDiffuse.a`. Texture alpha multiplies unless an env map is used.
- Alpha test uses `fAlphaDiscardValue` (≈0.2 default).
- TXI `blending`: default (alpha), `additive`, `punchthrough` (binary ≈0.5).
- TXI `alphamean` < 1 marks a texture as having alpha, enabling a discard variant ([CL] .26).
- `transparencyhint 0–9` orders static transparent meshes. The `'a'`-suffixed dummy makes children dynamic (sorted).
- MTR `transparency 1` / `twosided 1` / `sample_framebuffer 1|2` / `volumetric 1` (.35).
- No automatic per-triangle sorting.
- Two-sided lighting always flips the normal for back faces (culling is separate).

### Texture animation
- TXI `proceduretype cycle` + `numx numy fps` (flipbook, e.g. fxpa_flame02: 4×4 at 32 fps, additive, decal).
- `arturo`/`water`/`wave`/`life`/`perlin` + `distort`, `channelscale/translate`, `speed`.
- **These are CPU procedural textures** (no stock shader support; `m_texture` has no translation; [CL] mentions memory corruption above 256×256 in procedural distortion).
- Toggle `graphics.texture-animations.enabled`.
- MDL `animmesh` (animverts/animtverts) and emitters animate on the CPU.

### Tile specifics
- `rotatetexture 1` (ground meshes): the engine counter-rotates UVs when the tile is rotated, through `m_texture` (and the normal map XY).
- `tilefade`: see I.
- Animation dummy `<tile>a` holds animated meshes (static geometry otherwise). `Tile_AnimLoop1..3` toggle `animloop01..03` (day/night windows etc. [WIKI]).
- Static tile geometry is combined into buckets and baked into world-space vertex data ([CL] .37).

### Vertex colours
- MDL `colors` → `vCustomColor`, unused by stock shaders.
- `vColor` = engine-baked static lighting (vertex-lit mode only).

### PLT (`fs_pltgen`)
```
p = tex(PLT, uv)                      // r = grey intensity, g = layer index/255
p.g = PLTscheme[int(p.g*255+0.5)]     // layer → palette row v-coordinate (15 entries)
rgba = texture(palette, p.rg)
```
The result is a normal RGBA texture fed to `texUnit0`. The game can do this on the CPU or GPU.

### Particles (`fsparticle`)
- `VertexColor × tex`; alpha discard.
- Soft particles: `a *= sqr(clamp((sceneDepth − fragDepth)·particleSoftness, 0, 1))`.
- Additive: `a *= 1 − fog`; others: normal fog.

---------------------------------------------------------------------------------------------------

## K) Changelog items relevant to the renderer (version → item)
Sources: CHANGELOG.md / patchnotes (85.32 → 37-17), `Neverwinter Nights Enhanced Edition (v74..v79).txt`, wiki patch pages 1.74.8154 → 1.85.8193.31.

- **1.74.8155**: TXI `rotatetexture` fixed in the "new shaders" (so shader-based rendering exists from early EE).
- **1.74.8156–8158**: SSAO re-enabled. Experimental normal/specular maps; normals read from ASCII MDL; animeshes support TSB.
- **1.74.8159/8160**:
  - MTR format introduced (customshaderVS/FS, texture0–14, parameters)
  - `colors` → `vCustomColor`, `tverts1–3` → `vTcIn1–3`
  - **per-vertex static lighting with a "brightest light direction" stream** for normal mapping
- **v75 (1.75)**: dynamic contrast, SSAO and DoF shaders (Zarathustra217). MTR renderhint; `SetMaterialShaderUniform*`.
- **v76**: Sharpen post effect; SSAO 25% cheaper.
- **v78**: console/INI `Gamma` (FBO shader, 2.2 default). Standard DDS headers; KTX; 256 MB texture memory.
- **v79 / 1.79.8193**: keyholing ported from Android (settings min-height, min/max radius); shader `#include`; tangent generation fixes.
- **1.80.8193.6–.9**: `.lod` model LODs; `SetTextureOverride`; texture cache; renderhint `NormalTangents`; mipmaps for non-compressed textures fixed.
- **1.80.8193.10–.13**: FB passes get view/projection matrices; no alpha or depth test in FB passes; post passes use nearest filtering. Light priority selection ("enhanced light managing").
- **1.80.8193.14 / 1.81.8193.15**: **new enhanced lighting engine**:
  - PBR (specular, roughness, Fresnel, gamma correction)
  - tone mapping / overbright
  - per-pixel lighting
  - up to 32 dynamic lights (was effectively 6)
  - tunable attenuation/falloff
  - new water (dynamic reflections, wind waves); grass sorted and optimised
  - VFX models get fog and no self-illumination
  - BC4/BC5 DDS
  - "General Shader/Lighting Quality" setting; `BUILD_VERSION/REVISION` defines
  - static lights no longer recalculated in full-dynamic mode
- **1.81.8193.16/.17, 1.82.8193.20**:
  - smoother shadow fade; light range fix (no flicking)
  - water always uses the env map; skyfade covers the area underside
  - grass fade-in; texture animations consistent
  - **normal maps read as two-channel**
  - **specular ignores material transparency**; refined occlusion
  - shared material uniforms reset per draw
  - weapons/shields/cloaks ambient/diffuse set to 1.0
- **1.83.8193.21/.23**:
  - water screen-space refraction and reflection; soft particles + fog on particles
  - improved colour overflow for transparent objects (ColorClamp alpha)
  - unified shader setup (vs/fslit_nm/sm cover all PBR)
  - `DisplacementOffset` uniform; alpha follows Fresnel at high quality
  - debug render modes; all shaders get colour/depth FB access
  - bones = 64
  - `decal` disables lighting; darkness/negative lights priority fix
  - ice and old-head MTRs
- **1.83.8193.26**:
  - post shaders combined into one pass
  - **early-Z via NO_DISCARD** (custom alpha shaders need TXI `alphamean` < 1)
  - cubemap load order fixed
  - "default" envmap in TXI; chrome fallback
  - gamma checks any channel
  - `TILE_SOURCE_LIGHT_COLOR_BLACK` really removes the light
  - no downsampling of uncompressed textures; grass ambient material fixed
- **1.84.8193.29 / 1.85.8193.30**: fallback envmap global; water refraction fixes; TSB generation for opposing handedness and for animated/dangly meshes; draw buckets (additive-blend fixes).
- **1.85.8193.31/.32**:
  - shadows: 50–90% less CPU, own shaders (`vs_shadowvol`, `vs_beamvol`), VS frustum clipping
  - tile-bbox lower clip; shadow alpha by vertical distance
  - `shadowfliporder` console
  - debug outputs (bboxes, light ranges, shadow volumes)
  - auto-downsize over-large textures
  - subsurface light tweak; `rotatetexture` vs normal/displacement maps fixed; unlit+envmap fixed
- **1.86.8193.34**: rough surfaces' milky sheen fixed (heightmaps).
- **1.87.8193.35**:
  - **OpenGL 3.3**; toon shader; dynamic area lighting (`nw_dynlight`, `SetAreaLightDirection`)
  - GPU PLT; emitters with custom shaders; `SetShaderUniform*` scriptable uniforms; wireframe debug
  - nicer water waves; **transparent surfaces backlit**
  - non-character models cast shadows only if opaque; 3× more vertices per mesh
  - MTR `transparency/twosided/sample_framebuffer/volumetric`
  - `a`-nodes always dynamic; displacement at steep angles fixed; cubemap envmaps from TXI fixed; grass normals fixed
- **1.88.8193.36**:
  - **HDR bloom**; SSAO at half resolution; renderbuffers
  - GPU PLT uses an offscreen FB; compiled-model normal/tangent validation
  - keyhole min-Z setting; `subvariant` uniform
  - weather-density uniform fix; displacement warping fix; procedural-distortion > 256² memory fix
- **1.89.8193.37-13**:
  - MSAA and anisotropic filtering built in; `bboxMin/Max` uniforms
  - "Disable Gui Lighting" / "Enhanced Light Managing" always on
  - shadow corruption with dynamic area light fixed
  - crash with static (unenhanced) lighting fixed; VFX light fade fix
  - **keyhole parts render in the transparent pass**; clip uniforms updated (SSAO beyond fog fixed)
  - bad tangents no longer glow with bloom
  - **static geometry baked into world-space vertex data**
- **37-15**: toolset fix, model parts rotated in the toolset.
- **37-16/17**: no renderer items found.

---------------------------------------------------------------------------------------------------

## L) Wiki citations (local mirror `~/.local/opt/neverwinter/wiki/pages/NWN1/`)
- Enhanced Lighting Engine and PBR: https://nwn.wiki/spaces/NWN1/pages/38175899/Enhanced+Lighting+Engine+and+PBR (its linear-colour and per-fragment sections are "forthcoming")
- Reflectivity of Common Substances: https://nwn.wiki/spaces/NWN1/pages/38176311/Reflectivity+of+Common+Substances
- Shaders: https://nwn.wiki/spaces/NWN1/pages/60981936/Shaders
- Shader Engine Support: https://nwn.wiki/spaces/NWN1/pages/14614573/Shader+Engine+Support
- Shaders and Area Flags: https://nwn.wiki/spaces/NWN1/pages/65470710/Shaders+and+Area+Flags
- Per player shaders: https://nwn.wiki/spaces/NWN1/pages/65470748/Per+player+shaders
- MTR: https://nwn.wiki/spaces/NWN1/pages/12027232/MTR
- Standard material inputs: https://nwn.wiki/spaces/NWN1/pages/38175898/Standard+material+inputs
- Environment Maps and Cubemaps: https://nwn.wiki/spaces/NWN1/pages/38174773/Environment+Maps+and+Cubemaps
- Area Lighting: https://nwn.wiki/spaces/NWN1/pages/38174907/Area+Lighting
- Render Distance with Fog and Skyboxes: https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes
- TXI: https://nwn.wiki/spaces/NWN1/pages/38174929/TXI
- Model Shadows: https://nwn.wiki/spaces/NWN1/pages/49447442/Model+Shadows
- Changing day/night tile states through tile animations: https://nwn.wiki/spaces/NWN1/pages/60984838/Changing+day+night+tile+states+through+tile+animations+e.g.+glowing+windows+at+night
- Texture Tiling: https://nwn.wiki/spaces/NWN1/pages/38174960/Texture+Tiling (stub)
- Working with Transparency: https://nwn.wiki/spaces/NWN1/pages/68387028/Working+with+Transparency
- Dealing with transparency: https://nwn.wiki/spaces/NWN1/pages/129236997/Dealing+with+transparency
- skyboxes.2da: https://nwn.wiki/spaces/NWN1/pages/53670635/skyboxes.2da
- environment.2da: https://nwn.wiki/spaces/NWN1/pages/53670277/environment.2da (toolset-only presets)
- Also used:
  - MDL ASCII: https://nwn.wiki/spaces/NWN1/pages/12027273/MDL+ASCII
  - Model Special Nodes (ml1/ml2/sl1/sl2): https://nwn.wiki/spaces/NWN1/pages/38176272/Model+Special+Nodes
  - Model Table of Parameters: https://nwn.wiki/spaces/NWN1/pages/53671005/Model+Table+of+Parameters
  - Tilesets (.set GRASS/EnvMap): https://nwn.wiki/spaces/NWN1/pages/38175063/Tilesets
  - Console Commands: https://nwn.wiki/spaces/NWN1/pages/38175598/Console+Commands
  - Patch pages: 1.80.8193.14 https://nwn.wiki/spaces/NWN1/pages/38174859/1.80.8193.14 ; 1.83.8193.21 https://nwn.wiki/spaces/NWN1/pages/38176139/1.83.8193.21 ; 1.74.8159/8160
- `nwn-wiki search` for lightcolor/fog/keyhole/shadow/gamma/cubemap found no further pages beyond those above.

---------------------------------------------------------------------------------------------------

## M) Open questions and unknowns
1. **Fog amount → fogStart/fogEnd**:
   - What is the formula from `Sun/MoonFogAmount` (0–15) and `FogClipDist`?
   - Is fogEnd exactly FogClipDist, and does a skybox add +90 to fogEnd or only to the far clip?
   - The wiki's "0–200" range conflicts with the 0–15 values seen in AREs and environment.2da.
   - Needs an in-game measurement, or reading the toolset DFM trackbar max.
2. ~~Uniform values from settings~~: answered in E.6. Still open: the role of the MDL `multiplier` (Moonglow multiplies the colour; every light tested had 1).
3. **Colour space of uploaded colours**: lights and ARE colours are linearised (E.6). Still open: MDL material ambient/diffuse/selfillum (tiles read back as 1, 1).
4. ~~Default `lightAreaDiffuseDirection`~~: (4000, 4500, 7000) normalised, sun and moon (E.1). Still open: how dawn/dusk colours are interpolated (1 h?).
5. **Toolset rendering mode**: does nwtoolset run with GAMMA_CORRECTION/FRAGMENT_LIGHTING on, or vertex static lighting (it has "Compute Static Lighting" and `controlpart gidy_sun`)? What MAX_NUM_LIGHTS and shader quality does it use? Which set (day/night) does its area view show?
6. **Texture slots 11 vs 12** (FB colour vs depth): the wiki pages contradict each other. They are named samplers, so only custom shaders care.
7. **Cubemap face order** for `filerange 6` (`name0..5`) and the axis convention (NWN is Z-up; `m_view_inv` reflection in world space).
8. **Tile main-light overrides**: radius 10/5 is forced (E.6). Not checked: shadowradius 12/8, shadow 1, priority 4.
9. **Shadow plane colour/alpha source** (`vColor` of the plane = ShadowOpacity?). How are static sun projections built? Only relevant if shadows are implemented.
10. **SSAO composite blend mode**; the framebuffer format (RGBA8 vs 16F) with and without bloom; how `NO_DISCARD` is chosen per material.
11. `vso` is referenced by nwtoolset.exe but absent from resman. What does the toolset do with it?
12. **Engine selection logic** for `_sm` vs plain (e.g. does an area/tileset default envmap force `_sm` on every tile, or only when a TXI or appearance asks?) and for `fswater` (TXI `bumpmaptexture shinywater` + setting).
13. **Procedural TXI textures** (arturo/cycle/water): exact CPU algorithms; the wiki has only descriptions.
