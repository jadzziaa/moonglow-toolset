---
type: Research Note
title: 'NWN:EE model and texture formats: research notes for Moonglow Toolset'
description: NWN:EE model and texture formats for the renderer - ASCII and binary MDL, animations and supermodels, part-based creatures and PLT, textures and materials, special nodes, walkmeshes, lights, limits - with citations, open questions and where the wiki and the data disagree.
tags: [models, mdl, textures, plt, renderer]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-07T18:30:00Z }
---

# NWN:EE model and texture formats: research notes for Moonglow Toolset

Target: NWN:EE 89.8193.37-17 (Steam, Linux). Written 2026-09-30 for the renderer (areas, placeables, doors, creatures, items in the palette preview). A later writer will condense this.

**Tags:** `[V]` / `VERIFIED` / `[G]` = checked against base-game data in this session; `[S]` = from source code only; `[W:…]` = nwn.wiki (local mirror, URLs in H); `[U]` / `[inf]` = unverified or inference; `[TS]` / `[ENG]` = strings in `nwtoolset.exe` / `nwmain-linux`; `[PDF-SO]` / `[PDF-ARE]` = BioWare Door/Placeable and Area GFF PDFs.

**Scratch material:** `$SP=/tmp/claude-1000/-home-august-Projects-moonglow-toolset/42266be1-7a5b-4aaa-92bd-b9aa74640348/scratchpad`.
- Decompiled ASCII (nwnmdlcomp): `$SP/models/ascii/` has `a_ba`, `plc_a01`, `tcn01_a01_01`, `t_door09`, `c_golemerald`, `ashlw_011`, `pfe0_belt004`, `pmh0`.
- Binaries: `$SP/models/bin/`. Animation samples: `$SP/models/anim/{bin,obj,tile}`.
- Stock GLSL shaders: `$SP/models/shd/`. Sample 2DAs, PLTs and palettes: `$SP/models/data/`. Decoded PLTs: `$SP/models/png/`.
- Parsers and surveys:
  - `$SP/scripts/mdlbin.py`: binary reader, validated on all 25,597 binaries.
  - `$SP/models/anim/mdlinfo.py` and `survey.tsv`: every model's class, supermodel, animations and special nodes.
  - `$SP/research/scan_*.py`: texture, TXI and ASCII keyword scans with nwn.py.

## 0. Key takeaways for the renderer

1. **Two model formats.** The base game ships 32,832 `.mdl`: 25,597 binary (the first 4 bytes are 0) and **7,235 ASCII**. The ASCII ones include pmh0 and every other player skeleton, many tiles, GUI models and body parts. Build one data model with two loaders (A, B).
2. **Binary MDL is a 32-bit memory dump.**
   - Offsets are relative to file byte 12; the raw vertex data is relative to `12+rawOffset`.
   - The node type comes **only from the flags at node+0x6C**. Function pointers and parent pointers are garbage in EE-compiled files.
   - Orientation is a **quaternion (x,y,z,w)** in binary and **axis-angle** in ASCII.
   - Mesh alpha is **controller 128**.
   - Skin: an i16 node→bone map, 4 weights per vertex, up to 64 bones.
   - All base files parse (B).
3. **Mesh data differs by format.**
   - Binary vertices are already per-corner: all streams share one index.
   - ASCII faces use separate vertex and UV indices, so they must be de-indexed.
   - ASCII normals come from smoothing groups when `normals` is absent; tangents are needed for normal maps.
4. **Animations bind by node name** (case-insensitive). Look up an animation by name in the model, then walk up its supermodel chain.
   - Player chain: pmh0 → a_ba → a_ba_non_combat → a_ba_med_weap → a_ba_custom → a_ba_casts.
   - Races use their own animation sets, e.g. pmd0/pmg0/pmo0 → a_da.
   - `setanimationscale` scales inherited animations (exact rule unverified).
   - For which animation to play, see item 5 and C.
5. **Which animation each object plays in the editor:**

   | Object | Animation |
   |---|---|
   | Tile | `tiledefault`, then `day`/`night`, plus the enabled `animloop01..03` |
   | Placeable | by AnimationState: `default`/`open`/`close`/`dead`/`on`/`off` |
   | Door | `closed`/`opened1`/`opened2` (+ `trans`) |
   | Creature idle (MODELTYPE P/F) | `pause1` |
   | Creature idle (MODELTYPE S/L) | `cpause1` (then `creadyl` for wing/tail models) |
   | VFX | `impact` → `duration` → `cessation` |

   These match the toolset's own strings [TS] (C).
6. **Part-based creatures (D2).**
   - Build the skeleton name `p{m|f}{RACE letter}{phenotype}`.
   - For each of the 19 capart.2da slots, attach `{skeleton}_{part}{NNN}` at the listed skeleton node. The part number comes from the equipped armor, else the creature's body part.
   - The robe is a skinmesh with the supermodel of the skeleton's coat set. The cloak comes from cloakmodel.2da. Attach wings/tails at the `wings`/`tail` dummies; the helmet replaces the head.
   - The texture is the mesh's `bitmap` (often another race or part's PLT).
7. **PLT (D3).**
   - 24-byte header, then (grey, layer) byte pairs, rows bottom-up.
   - colour = `pal_<layer>.tga[row = colour index, counted from the top][column = grey]`, alpha included. The palettes are 256×176.
   - Grey 255 is transparent, or fully env-mapped.
8. **Textures (E).**
   - The MTR of the same name wins over images. Then DDS > PLT > TGA, but a TGA in a hak or override beats a DDS in the BIFs.
   - BioWare DDS: 20-byte header, DXT1 (3 channels) or DXT5 (4 channels), full mip chain, non-square allowed. Standard `DDS ` (BC1–5) is also supported.
   - **Upload every format in stored row order.** TGAs are bottom-left, and the DDS data is stored bottom-row-first too (measured).
9. **Alpha semantics (E.9).**
   - If an environment map applies (appearance/wingmodel/tailmodel/cloakmodel `ENVMAP`, placeables `Reflection`, TXI `envmaptexture`), texture alpha = 1 − reflectivity, not transparency.
   - Otherwise alpha is transparency with a 0.2 discard threshold.
   - TXI `blending additive/punchthrough`, `transparencyhint`, MTR `transparency` and "a-nodes" (`<model>a` dummies) control the transparent pass.
10. **The game's shaders ship as GLSL source** (`.shd`, 92 files). `inc_standard.shd`, `inc_material.shd` and `fs_pltgen.shd` are the reference implementation for PBR maps, legacy env-mapping and PLT colouring.

## A. MDL ASCII essentials

Sources: nwn.wiki "MDL ASCII" (https://nwn.wiki/spaces/NWN1/pages/12027273/MDL+ASCII), "Model Table of Parameters" (https://nwn.wiki/spaces/NWN1/pages/53671005/Model+Table+of+Parameters), "MDL ASCII Emitter Nodes" (https://nwn.wiki/spaces/NWN1/pages/26738875/MDL+ASCII+Emitter+Nodes), "MDL" (https://nwn.wiki/spaces/NWN1/pages/38175669/MDL), "Models" (https://nwn.wiki/spaces/NWN1/pages/38175602/Models); plus a scan of every ASCII .mdl shipped with 89.8193.37-17 (see A.7).

### A.1 Binary vs ASCII in the shipped game (measured)
- resman scan (nwn.py, base keys only): **32,832 .mdl = 25,597 binary + 7,235 ASCII**. Binary files start with 4 zero bytes; anything else is ASCII.
- ASCII ones are not rare leftovers: many EE/facelift tilesets (trs 1263, ttz 451, tss 401, tni 397, tno 242, tnp 220, twc 201, trm 200, tsw 178, tbw 134, tdt 102, dag 72, ...), GUI models (gui 318), invisible objects (invi 260), part-based body parts (pmh/pfh/pma/pmg/pfa/pfd/pfe/pfg/pfo/pmd/pme/pmo ~160-170 each), plc 113, vfx 106, vdr 36, c_* 57, weapons (wbws/wblf/wblc...). **The renderer must parse both formats** (same data model), and should expect sloppy ASCII (see A.6).
- Game compiles ASCII at load time (slow; generates normals + tangents at runtime). Console: `compileloadedmodels`, `compileloadedasciimodels`, `compilemodel <name>`, CLI `nwmain compilemodel <name>` (output to user dir `modelcompiler/`).

### A.2 File structure
```
# comment lines start with '#'
filedependancy <file>          # ignored (also spelled filedependency in shipped files)
newmodel <name>                # must be first real statement; name == resref, <=16 chars, lowercase
  setsupermodel <name> <super|NULL>
  classification <tile|character|effect|effects|door|gui|item|other>   # anything not tile/character/effect/door => "none" (0)
  ignorefog 0|1                # dynamic models only
  setanimationscale <float>    # default 1.0
  beginmodelgeom <name>
    node <type> <nodename> ... endnode      # first node: dummy named == model name, parent NULL
  endmodelgeom <name>
  newanim <animname> <modelname>
    length <sec>  transtime <sec>  animroot <nodename>
    event <time> <eventname>   (0..n)
    node <type> <nodename>  parent <p>  <controller>key ... endlist  endnode
  doneanim <animname> <modelname>
donemodel <name>
```
- Keywords are case-insensitive in practice (shipped files use `Shadow`, `Diffuse`, `Alpha`, `nDynamicType`, `affectDynamic`, `fadingLight`...). Booleans appear as `0/1` and `true/false`.
- Order matters at the top level (e.g. `classification` before `newmodel` crashes). Inside a node any order.
- Unknown keywords are skipped by the engine (safe to ignore); non-standard node types (`patch`, `pwk`, ...) are treated as **dummy**.
- `NULL` means "none" for parent/bitmap/supermodel.
- Limits: model/texture/MTR/supermodel resrefs <= 16 chars; node names <= 32 chars (truncated on compile -> duplicate names can crash); animation names <= 16 chars in practice (longer do not play; binary geometry-header name field is char[64], see B.4). Node order defines the internal "part number" used by supermodel animation matching.
- Mesh size: pre-8193.35 max 10,922 faces/mesh (int16 indices); since 1.87.8193.35 max 21,845 faces/mesh (uint16, (2^16-1)/3). Skin: max 64 bones per skin node (since 8193.21; 17 in 1.69, 128 per model in early EE), max 4 weights per vertex.

### A.3 Classification (engine effect; wiki table)
| value | render order | used for | effect |
|---|---|---|---|
| character | 1st | creatures, placeables, items, robes, cloaks | casts shadows (and attached models); body parts forced to character |
| effect(s) | 2nd | VFX | emitters render at any distance; not tab-highlighted; ignored for hit effects |
| door | 3rd | doors | door fading when camera is on the other side |
| (none/other/gui/item/unknown) | 4th | walkmeshes (pwk/dwk/wok), GUI | no special behaviour |
| tile | 5th (last) | tiles | pushed to bottom of BSP |
Shipped ASCII counts: tile 3797, character 2704, gui 323, effect 139, item 126, door 70, effects 29, other 10. Binary byte values: 0 none, 1 effect, 2 tile, 4 character, 8 door (only these occur, B.5).
Since 1.87.8193.35: models of classification other than `character` cast shadows only if not transparent.

### A.4 Node types and properties
All nodes (dummy base): `parent <name|NULL>`, `position x y z` (relative to parent), `orientation x y z angle` (axis-angle, radians), `scale s` (uniform), `wirecolor r g b` (ignored). Animatable: position, orientation, scale.

**trimesh** (and base of danglymesh/skin/animmesh):
| keyword | default | notes |
|---|---|---|
| ambient r g b | 1 1 1 (pre-1.81.8193.17 compiles 0.2) | GL ambient |
| diffuse r g b | 1 1 1 (old compiles 0.8) | multiplied with texture0; shipped files sometimes have 4 values |
| specular r g b | 0 0 0 | no effect in EE |
| shininess f | 1 | ignored in EE |
| selfillumcolor r g b | 0 0 0 | emissive; animatable; misspelt `setfillumcolor` in old files (2,388 occurrences shipped!) - accept both |
| alpha f | 1.0 | animatable |
| render 0/1 | 1 | 0 = not drawn but still casts shadow |
| shadow 0/1 | 1 | shadow volume only for trimesh (not skin); use low-poly `render 0 shadow 1` shadow meshes |
| beaming 0/1 | 0 | light-ray / "beam volume" mesh (tiles) |
| inheritcolor 0/1 | 0 | effectively unused |
| transparencyhint int | 0 | 0 = opaque pass; 1..9 sort order for transparent static meshes (uint32 in binary) |
| tilefade 0-3 | 0 | 0 never, 1 fade, 2 base(?), 3 neighbour; **4 occurs 1,877x in base tiles (undocumented)**; tiles and static placeables only (Hide Second Story) |
| rotatetexture 0/1 | 0 | tiles: undo UV rotation when tile rotated (ground seams) |
| lightmapped 0/1 | 0 | rare (t_door10); unclear |
| bitmap / texture0 name | "" | diffuse texture (alias pair). Lookup order MTR > DDS > PLT > TGA (see E) |
| texture1..texture3 name | "" | 1.69-era extra texture slots; unused by Bioware |
| materialname name | "" | EE: .mtr file (see E.8) |
| renderhint NormalAndSpecMapped / NormalTangents / None | "" | EE: generate tangents (ASCII only) and pick normal-mapped shader |
| center x y z | - | ignored |
| verts n + n lines x y z | | |
| tverts n + n lines u v 0 | | UV set 0 |
| tverts1/2/3 n ... | | EE (1.74.8159): extra UV streams -> shader attributes vTcIn1..3 |
| faces n + n lines `v1 v2 v3 smoothgroup t1 t2 t3 material` | | smoothgroup (bitmask) drives normal generation; material = surfacemat.2da id (walkmesh only) |
| colors n + n lines r g b | | vertex colours, not used by default shaders (vCustomColor) |
| normals n + lines x y z | autogen | EE (1.74.8158) reads them from ASCII |
| tangents n + lines x y z sign | autogen | EE: tangent + bitangent sign |
Face vertex vs UV indices are separate lists -> renderer must de-index (split verts) per unique (v, t) pair.

**danglymesh** = trimesh + `displacement f` (~0.5 m per unit), `tightness f`, `period f` (<59; >=60 locks), `constraints n` + n values 0-255 (per-vertex multiplier of displacement). Wind/character movement driven; frozen when paused. Shipped files also carry NWMax junk: `danglymesh 0`, `showdispl`, `displtype`, `gizmo`.

**skin** = trimesh + `weights n` + n lines `bone1 w1 [bone2 w2 [bone3 w3 [bone4 w4]]]` (1..4 pairs, sum 1.0; bone = node name in same model). Skin meshes do not cast shadows. Max 64 bones per skin node.

**animmesh** = trimesh in geometry; in animations: `sampleperiod f`, `animverts n` (list of xyz, n = frames*verts), `animtverts n` (uv0 list), optional `clipu clipv clipw cliph` (UV clip rect, seen in tno01_i69_01). Since 1.74.8158 animmeshes support tangent space (normal/spec maps). Animated meshes with no animation render static.

**light** (see wiki table; defaults):
| keyword | notes |
|---|---|
| radius f | metres; also default shadow radius; animatable (radiuskey) |
| multiplier f | intensity (default 1); animatable |
| color r g b | animatable (colorkey). **Ignored for tile main lights** (names ending `ml1`/`ml2`: colour comes from area/tile settings) |
| ambientonly 0/1 | 0 |
| ndynamictype 0/1 (old: isdynamic) | tile lights: 0 main lights, 1 animated/source lights |
| affectdynamic 0/1 | default 1 |
| shadow 0/1 | default 1 |
| lightpriority 1-5 | 1 highest (5 = unneeded tile lights) |
| fadinglight 0/1 | 1.5 s fade |
| generateflare 0/1 (always 0 in binaries even when flares exist - draw flares when lists are non-empty and flareradius > 0), flareradius f, flarepositions n / flaresizes n / flarecolorshifts n / texturenames n (lists) | lens flares (textures fxpa_lensflare, fxpa_lensring); `lensflares 0/1` also seen |
| shadowradius f, verticaldisplacement f | EE shadow tuning |

**emitter** (full list from wiki + shipped files): style `update` (Fountain / Single / Explosion (fires on `detonate` event) / Lightning), `render` (Normal / Linked / Billboard_to_Local_Z / Billboard_to_World_Z / Aligned_to_World_Z / Aligned_to_Particle_Dir / Motion_Blur), `blend` (Normal / Punch-Through / Lighten), `spawntype` (0 normal, 1 trail); geometry `xsize ysize` (cm, capped 500), `inherit inheritvel inherit_local inherit_part`, `renderorder`, `threshold`, `combinetime`, `deadspace`; particles `colorStart colorEnd` (rgb), `alphaStart alphaEnd`, `sizeStart sizeEnd sizeStart_y sizeEnd_y`, `birthrate` (lightning: segments, >9 crashes), `lifeExp`, `mass`, `spread` (radians in practice), `particleRot`, `velocity`, `randvel`, `bounce_co`, `blurlength`, `loop`, `bounce`, `m_isTinted` (typo `m_istnited` exists), `splat`, `affectedByWind`; texture `texture`, `twosidedtex`, `xgrid ygrid` (sprite sheet), `fps`, `frameStart frameEnd`, `random`, `chunkName` (model instead of sprite, e.g. plc_chunk_*); advanced `lightningDelay lightningRadius lightningSubDiv lightningScale`, `blastRadius blastLength`, `opacity`, `p2p p2p_sel (0 gravity / 1 bezier; files also have 2 + p2p_type Gravity/Bezier)`, `p2p_bezier2 p2p_bezier3`, `grav drag`; NWMax extras `update_sel render_sel blend_sel spawntype_sel`. Animatable: most numeric params via `<param>key` (birthratekey, alphaStartkey, colorStartkey, sizeStartkey, lifeExpkey, velocitykey, xsizekey ...). p2p/lightning emitters need a child **reference** node (crash otherwise). EE 1.87.8193.35: emitters can use custom shaders.

**reference**: dummy + `refmodel <mdl|fx_ref>` + `reattachable 0|1` - target nodes for p2p/lightning/beam emitters.

**aabb** (walkmesh node in tile .wok files, trimesh-like): verts/faces (face material = surfacemat.2da row), then `aabb` tree: lines `minx miny minz maxx maxy maxz leafFaceIndex(-1 for internal)`, depth-first. NWMax exports add `multimaterial N` + material names list (Dirt, Obscuring, Grass, Stone, Wood, Water, Nonwalk, Transparent, Carpet, Metal, Puddles, Swamp, Mud, Leaves, Lava, BottomlessPit, DeepWater, Door, Snow, Sand); ignore.

**camera**, **patch**, **pwk** etc.: treat as dummy.

### A.5 Animations
- `newanim <name> <model>`; `length`, `transtime` (blend-in overlap seconds), `animroot <node>` (subtree only; e.g. torch/shield arm), `event <t> <name>` (events: detonate, blur_start, blur_end (aka doneattack01/done_attack02), draw_weapon, hit, snd_footstep, snd_hitground, donefade; `cast`, `draw_arrow`, `parry` listed historically but unused).
- Animation nodes reference geometry nodes **by name** (wiki says order must match; binary data shows the compiler resolves by name against the direct supermodel, see B.19/C2.2); node types may differ (trimesh often written as dummy). Keys: `positionkey` / `orientationkey` (axis-angle per key) / `scalekey` / `alphakey` / `selfillumcolorkey` / `colorkey` / `radiuskey` / `multiplierkey` / emitter `<param>key`; each followed by lines `t v...` and closed by `endlist` (some tools write `positionkey <count>` and omit `endlist` - accept both). Bezier variants (`positionbezierkey` etc.; wiki: 9 values per line = value + 2 control points) are accepted by nwnmdlcomp (column flag 0x10) but occur **0 times** in game data and Neverblender ignores them: optional support. Single values (e.g. `position x y z`) hold for the whole animation.
- animmesh anim data: `sampleperiod`, `animverts`, `animtverts` (see above).
- Shipped junk to ignore: `centerkey`, `gizmokey`, `lockaxeskey`, `chunkykey`...

### A.6 Observed shipped-ASCII statistics (scan of all 7,235 ASCII mdls)
- node types: trimesh 130,832; dummy 73,866; light 9,103; emitter 6,367; aabb 4,419; danglymesh 2,525; skin 1,123; animmesh 806; reference 169; pwk 108 (non-standard -> dummy); a handful of malformed `node pos=[...]` lines.
- animation names found in ASCII files: animloop01/02/03, tiledefault, default, die/dead, damage (+ damagel/damager/damages), sit/sitdown, trans, opening1/opened1/closing1 (+ `2` variants), closed, open/close, open2close/close2open, on/off, on2off/off2on, day/night/day2night/night2day, impact/duration/cessation (VFX), and creature sets (cpause1, cwalk, crun, ...). (Full list: part C.)
- Only 68 shipped ASCII models carry `renderhint`/`normals`/`tangents` (e.g. wblml_b_051 uses `renderhint None`).
- Parser robustness required: mixed case, 4-component colours, `true/false` booleans, blank lines, both `filedependancy`/`filedependency`, `setfillumcolor`, keyword lines indented arbitrarily, counts that may be wrong in old files (engine: "invalid array lengths ... behave erratically").

### A.7 Scan scripts (re-runnable)
`$SP/research/scan_tex.py`, `scan_mdl.py`, `scan_kw.py`, `scan_txi.py`, `find_kw.py` (nwn.py resman, read-only; run with ~/.local/opt/neverwinter/venv/bin/python).

## B. Binary MDL layout (derived from nwnmdlcomp source, verified on all 25,597 binaries)

Source: sub-research on Neverblender + vendored nwnmdlcomp (Neverblender's build tree (`build/third_party/nwn-tools`)); "§N" below = B.N. Status tags: **[V]** verified against game data, **[S]** source only, **[U]** unverified.

### B.0 Provenance

- **The Neverblender add-on has no binary MDL reader.** It only detects binary
  files (`nvb_mdl.py:156-160 Mdl.is_binary`, 4 leading zero bytes) and shells
  out to an external decompiler (`nvb_mdl.py:195-227 Mdl.parse_mdl`,
  `build_external_decompile_cmd` 162-193). Binary walkmeshes are skipped
  (`nvb_mdl.py:257-262`). There is no `struct.unpack` of MDL data anywhere
  in `neverblender/*.py`.
- The binary layout below comes from the decompiler that the pipeline uses:
  Torlack's **nwnmdlcomp** (niv/nwn-tools @ d979787, 2018), vendored read-only at
  Neverblender's build tree (`build/third_party/nwn-tools`) and built into
  `tools/bin/nwnmdlcomp` (`tools/nwnmdlcomp-decompile.sh`). Key files, all under
  `.../nwn-tools/_NwnLib/` unless noted:
  - `NwnModel.h:84-89` file header; `NwnArray.h:65-188` CNwnPointer (null=0),
    `196-319` CNwnPointer2 (null=0xFFFFFFFF), `327-838` CNwnArray.
  - `NwnMdlGeometry.h:71-104` enums, `158-168` geometry header, `210-215`
    animation header, `277-290` model header.
  - `NwnMdlNodes.h:75-213` enums, `239-310` key/face/AABB structs,
    `370-914` node classes with offset comments.
  - `NwnMdlR2A.cpp` (offset → pointer fix-up: which pointers are model-data
    relative and which are raw-data relative), `NwnMdlSerialize.cpp` (writer
    order), `NwnMdlDecomp.cpp` (binary → ASCII), `NwnMdlGeometry.cpp`,
    `NwnMdlNodes.cpp` (defaults, function pointer constants).
  - Compiler side: `_NmcLib/NmcController.cpp:56-490` (controller names/IDs),
    `_NmcLib/NmcAttribute.cpp:111-845` (ASCII attribute table),
    `_NmcLib/NmcCoreParsers.cpp:62-120`, `_NmcLib/NmcGeometry.cpp`,
    `_NmcLib/NmcMesh.cpp:1440-1887`, `nwnmdlcomp/nwnmdlcomp.cpp:560-676`.
- Verification: my own Python reader, `$SP/scripts/mdlbin.py`, run over **every**
  MDL in the resman (keys only, no user dir). Scan scripts and outputs are
  `$SP/scripts/scan{1..5}.py`, `nodetypes_scan.py`, `skin.py`, `eeskins.py`,
  `gaps.py` and `$SP/research/scan2.txt`, `scan3.txt`, `types.txt`
  (`SP=/tmp/claude-1000/-home-august-Projects-moonglow-toolset/42266be1-7a5b-4aaa-92bd-b9aa74640348/scratchpad`).
  The whole corpus parses with 0 errors, and `12 + rawOffset + rawSize == file size` for all 25,597 binaries.

### B.1 Corpus facts [V]

| | count |
|---|---|
| `.mdl` in resman (all from `nwn_base.key` BIFs, none from ovr) | 32,832 (1.68 GiB) |
| ASCII | 7,235 (22%): e.g. `pmh0`, `amp01_*`, `a_ba_coat`, many tiles; `pmh0.mdl` is NWmax ASCII |
| binary | 25,597 |

The binaries are grouped by the geometry-header function pointer pair at file offset 0x0C (model data +0x00):

| routines[0],[1] | producer | files | node routine pointers |
|---|---|---|---|
| `0x0040BBC0, 0x0040BBD0` | BioWare 1.x compiler | 18,873 | per-type constants (table §7) |
| `0x0046AB0C, 0x0046AB1C` | nwnmdlcomp (`NwnMdlGeometry.cpp:154-155`) | 4,786 | nwnmdlcomp constants |
| `0x00000004, 0x00000000` | EE engine compiler (older EE build) | 1,935 | **garbage** (runtime values) |
| `0xCCCCCCCC` ×2 | MSVC debug build (`ctl_cg_btn_col*`) | 2 | 0xCCCCCCCC |
| `0x00050630, 0x00050850` | unknown (`pnl_lightning`) | 1 | small values |

The current EE client (37-17 `compilemodel`) writes a **64-bit runtime pointer** in routines[0..1]
(seen in Neverblender's test build (`build/e2e/compiled/zz_nvbtest.mdl`): `70 10 32 b3 ff 7f 00 00`).
**Never use routine pointers to detect a format or node type. Use node `flags` (+0x6C).** There is no PC-vs-Mac variant anywhere in the base data.

Samples (sizes): `a_ba.mdl` 5,990,990 B (bio); `plc_a01` 59,000 (bio); `tcn01_a01_01` 60,508 (bio);
`t_door09` 79,148 (EE); `ashlw_011` 11,152 (EE); `wswls_b_093` 6,768 (bio); `pfe0_belt004` 4,808 (bio);
`pmh0_chest001` 8,104 (bio); `pmh0` 9,081 (ASCII).

### B.2 Conventions [V]

- Little-endian, 32-bit layout (4-byte "pointers"). Floats are IEEE f32. `V3` = 3×f32, `V2` = 2×f32, `Q` = 4×f32.
- **Model data** base `M = 12` (file byte 12). Every "pointer" in the model section is a u32 offset from `M`.
- **Raw data** base `R = 12 + rawOffset`. Vertex streams and skin weights are offsets from `R`.
- `Ptr` (CNwnPointer, model data): 0 means null. Only the root model header lives at offset 0.
- `RPtr` (CNwnPointer2, raw data): **0xFFFFFFFF means null**, and 0 is a valid offset.
- `Array` (CNwnArray, `NwnArray.h:831-837`) is 12 bytes: `u32 offset (model data), u32 count, u32 alloc`. Ignore `alloc`. If count is 0 the offset may be garbage.
- `char[N]` fields are NUL-terminated with **garbage after the NUL** (e.g. `NULL\0med_weap`). Names may contain upper case (`T_Door09`); compare case-insensitively.
- EE-compiled raw arrays can be **unaligned** (e.g. verts at raw+394). Use unaligned reads.
- Runtime-only fields (routines, geometry/parent back-pointers, ref counts, bone constant indices, "temp mesh", pad bytes, EE `lightmapped` byte) hold garbage in EE files and must be ignored.

### B.3 File header (12 bytes, `NwnModel.h:84-89`) [V]

| off | type | field |
|---|---|---|
| 0x00 | u32 | 0 (the binary marker; an ASCII file starts with text) |
| 0x04 | u32 | rawOffset: raw data starts at file offset `12 + rawOffset` (= size of the model section) |
| 0x08 | u32 | rawSize |

nwnmdlcomp writes it at `nwnmdlcomp.cpp:605-611` and checks the 4 zero bytes at `638-676`.

### B.4 Geometry header (0x70 = 112 bytes; `NwnMdlGeometry.h:158-168`) [V]

The model header (at M+0) and every animation header start with this block.

| off | dec | type | field | notes |
|---|---|---|---|---|
| 0x00 | 0 | u32[2] | function pointers | see §1; ignore |
| 0x08 | 8 | char[64] | name | model or animation name |
| 0x48 | 72 | Ptr | root node | model: root dummy; anim: anim root node |
| 0x4C | 76 | u32 | node count | model: *includes the supermodel's nodes* (§19) |
| 0x50 | 80 | Array | runtime array 1 | always empty |
| 0x5C | 92 | Array | runtime array 2 | always empty |
| 0x68 | 104 | u32 | ref count / unknown | 0 (bio), 384 (EE garbage) |
| 0x6C | 108 | u8 | geometry type | **2 = model, 5 = animation** [V all files]. Bits: 0x01 geometry, 0x02 model, 0x04 anim; 0x80 is set at runtime when loaded binary (`NwnMdlR2A.cpp:64`) |
| 0x6D | 109 | u8[3] | pad | garbage |

### B.5 Model header (0xE8 = 232 bytes; `NwnMdlGeometry.h:277-290`) [V]

| off | dec | type | field | observed |
|---|---|---|---|---|
| 0x00 | 0 | — | geometry header | §4 |
| 0x70 | 112 | u8[2] | flags / unknown | 00 00 (bio/nmc); garbage (EE) |
| 0x72 | 114 | u8 | classification | only **0, 1 effect, 2 tile, 4 character, 8 door** occur. 0 = not set (GUI, items: 3,379 bio files) |
| 0x73 | 115 | u8 | fog | 1 in all 25,597 files. 0 means `ignorefog 1` (`NwnMdlDecomp.cpp:175-176`) |
| 0x74 | 116 | u32 | ref count | 0 |
| 0x78 | 120 | Array\<Ptr\> | animations | u32 offsets to animation headers |
| 0x84 | 132 | Ptr | supermodel pointer | 0 in files (runtime) |
| 0x88 | 136 | V3 | bbox min | usually (-5,-5,-1) |
| 0x94 | 148 | V3 | bbox max | usually (5,5,10) |
| 0xA0 | 160 | f32 | radius | 7.0 default; the compiler raises it to ≥40 when there is an emitter (`NmcGeometry.cpp:311-321`) |
| 0xA4 | 164 | f32 | animation scale | 1.0 in 25,456 files; 0.33…10 elsewhere |
| 0xA8 | 168 | char[64] | supermodel name | `NULL` if none |

nwnmdlcomp's classification strings are `effect`/`effects`=1, `tile`=2, `character`=4, `door`=8 (`NmcGeometry.cpp:976-998`).
ASCII game models also use `gui` (323), `item` (126) and `other` (10). No binary has another value, so these most likely compile to 0 [U]. Values 0x10/0x20 were **not** observed.
nwnmdlcomp bug: `ignorefog` sets `m_ucFog = (value != 0)`, the inverse of the decompiler (`NmcGeometry.cpp:959-970`).

### B.6 Animation header (0xC4 = 196 bytes; `NwnMdlGeometry.h:210-215`) [V]

| off | dec | type | field |
|---|---|---|---|
| 0x00 | 0 | — | geometry header (type 5) |
| 0x70 | 112 | f32 | length (s) |
| 0x74 | 116 | f32 | transtime (s) |
| 0x78 | 120 | char[64] | animroot (node name, e.g. `rootdummy`, `torso_g`, or the model name) |
| 0xB8 | 184 | Array | events: 36-byte elements `{f32 time; char[32] name}` (`NwnMdlGeometry.h:123-127`) |

Events seen in ASCII models: snd_footstep, hit, blur_start, blur_end, snd_hitground, detonate, cast, draw_arrow, parry.
Neverblender's default list is `nvb_def.py:38-40`. The animation's root node pointer is at +0x48.

### B.7 Node header (0x70 = 112 bytes; `NwnMdlNodes.h:370-384`) [V]

| off | dec | type | field | notes |
|---|---|---|---|---|
| 0x00 | 0 | u32[6] | function pointers | type-specific (table below); garbage in EE |
| 0x18 | 24 | u32 | inheritcolor | |
| 0x1C | 28 | i32 | part number | index in the model's node list; −1 = not in model (anim nodes) |
| 0x20 | 32 | char[32] | name | |
| 0x40 | 64 | Ptr | geometry header pointer | 0 in bio/nmc files, garbage in EE: **ignore** |
| 0x44 | 68 | Ptr | parent pointer | 0 in bio/nmc files, garbage in EE: **ignore**. Rebuild parents from the children arrays |
| 0x48 | 72 | Array\<Ptr\> | children | u32 offsets to child node headers |
| 0x54 | 84 | Array | controller keys | 12-byte entries (§8) |
| 0x60 | 96 | Array\<f32\> | controller data | |
| 0x6C | 108 | u32 | **flags / node type** | the only reliable type tag |

Content flag bits (`NwnMdlNodes.h:75-88`), all confirmed: 0x1 header, 0x2 light, 0x4 emitter, 0x8 camera, 0x10 reference, 0x20 mesh, 0x40 skin, 0x80 anim(mesh), 0x100 dangly, 0x200 aabb (0x400 "unknown" is never seen).

| flags | type | struct size | first data offset after node [V] | model nodes (bio+nmc+ee) |
|---|---|---|---|---|
| 0x001 | dummy | 0x70 (112) | 112 | 46,860 |
| 0x003 | light | 0xCC (204) | 204 | 18,016 |
| 0x005 | emitter | 0x148 (328) | 328 | 7,099 |
| 0x009 | camera | 0x70 | — | 0 (never in corpus) |
| 0x011 | reference | 0xB4 (180) | 180 | 249 |
| 0x021 | trimesh | 0x270 (624) | 624 | 210,454 |
| 0x061 | skin | 0x2D4 (724); EE variant 0x3B0 (944) | 724 / 944 | 655 |
| 0x0A1 | animmesh | 0x2A8 (680) | 680 | 487 |
| 0x121 | danglymesh | 0x288 (648) | 648 | 5,179 |
| 0x221 | aabb | 0x274 (628) | 628 | 8,875 |

"First data offset after node" means that the smallest referenced model-data offset owned by the node sits exactly `size` bytes after the node start, which confirms each struct size.

Node routine constants. BioWare (bio):

| type | routines[0..5] |
|---|---|
| dummy | 40cb70 40cbd0 40cbe0 40cbf0 40cc10 40cc20 |
| light | 40db70 40db80 40db90 40dba0 40dbc0 40dbd0 |
| emitter | 40cb70 40d9b0 40d9c0 40d9d0 40d9f0 40da00 |
| reference | 40cb70 40df30 40cbe0 40df40 40df60 40df70 |
| trimesh | 40e2e0 40e2f0 40e300 40e310 40e330 40e340 |
| skin | 40d710 40d790 40d7a0 40e310 40d7b0 40d7c0 |
| animmesh | 40d460 40d490 40d4a0 40d4b0 40d4d0 40d4e0 |
| dangly | 40e040 40e080 40e090 40e310 40e330 40e0a0 |
| aabb | 40e660 40e6e0 40e300 40e310 40e330 40e6f0 |

Mesh routines at +0x70 (bio): trimesh and aabb `40e350,40d7e0`; skin `40d7d0,40d7e0`; animmesh `40d4f0,40d500`; dangly `40e0c0,40e0b0`.
Animation header routines (bio): `40b6c0, 44e700`.
nwnmdlcomp constants: `NwnMdlNodes.cpp:80-85, 182-187, 263-268, 309-312, 359-362, 447-454, 582-588, 645-652, 707-712, 758-760`.

### B.8 Controllers [V]

Key entry, 12 bytes (`NwnMdlNodes.h:239-256`):

| off | type | field |
|---|---|---|
| 0x0 | i32 | controller type (ID, table below; the meaning depends on node flags) |
| 0x4 | i16 | rows (number of keys) |
| 0x6 | i16 | time-key offset: index (in floats) into the node's controller-data array |
| 0x8 | i16 | data offset: float index of the first value |
| 0xA | i8 | columns = values per row. 0x10 bit = Bézier keys (never in game data; see below). −1 = no values (detonate) |
| 0xB | i8 | pad (garbage) |

Data layout: `data[timeOff .. timeOff+rows)` are the key times, then
`data[dataOff .. dataOff + rows*cols)` are the values row-major.
Geometry (model) controllers always have rows=1 with time 0.0 stored (1,260,370 of 1,260,370).
Animation controllers are keyed (rows>1: 235,414; rows=1: 21,709).
Decoder: `NwnMdlDecomp.cpp:1521-1622`; encoder: `_NmcLib/NmcController.cpp:410-445`, where `bColumns = nColumns-1` because the ASCII time column is not counted.

- **Orientation = quaternion x,y,z,w** (4 columns). 26,061 of 26,064 are unit length, 19,683 are (0,0,0,1).
  ASCII is axis-angle `x y z angle`: converted at `NmcCoreParsers.cpp:108-118` and back at `NwnMdlDecomp.cpp:1707-1713`.
  Neverblender builds `Quaternion(axis, angle)` (`nvb_utils.py:583-592`).
- Scale is 1 column (uniform).
- Columns = −1 with rows=N: N times, no values (`detonatekey`, `vwp_flash_blured`). Model-level `detonate` has cols 0 or 1.
- Bezier (`*bezierkey`, 0x10) [V: `nwmain compilemodel`, 2026-10-01]: the game's compiler writes the value's own column count with 0x10 (`positionbezierkey`: 0x13) and per key the value then two handles (`t x y z h1x h1y h1z h2x h2y h2z` → 9 values), as written. nwnmdlcomp differs: it counts every value of the row (0x19, `NmcController.cpp:413-416`; decode masks `&0x0F`, `NwnMdlDecomp.cpp:1561-1564`). `orientationbezierkey` (12 values a row) is cut to 9 by the game's compiler and not converted to quaternions: unusable. The engine's curve through the handles is unknown.
  **0 occurrences in the corpus**, and none in ASCII game models either. [U] layout (value + in/out tangents?).

Controller IDs (`NwnMdlNodes.h:117-180`, names from `NmcController.cpp:56-125`). ✓ = seen in corpus (count, cols):

| node | ID | name | cols | seen |
|---|---|---|---|---|
| all | 8 | position | 3 | ✓ |
| all | 20 | orientation | 4 (quat) | ✓ |
| all | 36 | scale | 1 | ✓ (mesh 142k, emitter 4, skin 8) |
| mesh (trimesh/skin/dangly/anim/aabb) | 100 | selfillumcolor (nwnmdlcomp spells it `setfillumcolor`) | 3 | ✓ |
| mesh | **128** | alpha | 1 | ✓ 142k. **Not 132**: 132 never occurs on meshes |
| light | 76 | color | 3 | ✓ (one anim row with 4 cols) |
| light | 88 | radius | 1 | ✓ |
| light | 96 | shadowradius | 1 | ✓ 4 (fx_flame01) |
| light | 100 | verticaldisplacement | 1 | ✓ 3 (fx_flame01) |
| light | 140 | multiplier | 1 | ✓ |
| light | **144** | **unknown, EE-only** | 1 | ✓ 1,408 lights, all in EE-compiled files, value always 1.0 [U name] |
| emitter | 80 alphaEnd, 84 alphaStart, 88 birthrate, 92 bounce_co | | 1 | ✓ |
| emitter | 96 colorEnd, 108 colorStart | | 3 | ✓ |
| emitter | 120 combinetime, 124 drag, 128 fps, 132 frameEnd, 136 frameStart, 140 grav, 144 lifeExp, 148 mass, 152 p2p_bezier2, 156 p2p_bezier3, 160 particleRot, 164 randvel, 168 sizeStart, 172 sizeEnd, 176 sizeStart_y, 180 sizeEnd_y, 184 spread, 188 threshold, 192 velocity, 196 xsize, 200 ysize, 204 blurlength, 208 lightningDelay, 212 lightningRadius, 216 lightningScale | | 1 | ✓ (every emitter carries ~33 single-row controllers) |
| emitter | 228 | detonate | 0 / −1 | ✓ 26 |
| emitter | 448 alphaMid, 452 colorMid(3), 464 percentStart, 465 percentMid, 466 percentEnd, 468 sizeMid, 472 sizeMid_y | | | not in corpus; IDs from `nwmain compilemodel` [V]. nwnmdlcomp uses 464 alphaMid, 468 colorMid, 480–482 percent*, 484 sizeMid, 488 sizeMid_y (`NwnMdlNodes.h:168-174`), which the game reads as percentStart and sizeMid |

IDs overlap across node types (100 = selfillumcolor for meshes and verticaldisplacement for lights; 88, 96, 128, 140, 144 likewise), so resolve the name by (flags, ID), as `NmcGetControllerName` does (`NmcController.cpp:461-490`).
ASCII game animations also key emitter parameters that have **no known ID** (lightningsubdivkey, opacitykey, spawntypekey, xgridkey, …) [U].
**Renderer note:** most emitter parameters are controllers, not header fields.

### B.9 Mesh header (node +0x70 … +0x270; `NwnMdlNodes.h:640-686`) [V]

| off | dec | type | field | observed |
|---|---|---|---|---|
| 0x070 | 112 | u32[2] | mesh function pointers | |
| 0x078 | 120 | Array | faces (32-byte `Face`, §10) | model data |
| 0x084 | 132 | V3 | bbox min | |
| 0x090 | 144 | V3 | bbox max | |
| 0x09C | 156 | f32 | radius | |
| 0x0A0 | 160 | V3 | average (centre) | |
| 0x0AC | 172 | V3 | diffuse | |
| 0x0B8 | 184 | V3 | ambient | |
| 0x0C4 | 196 | V3 | specular | |
| 0x0D0 | 208 | f32 | shininess | |
| 0x0D4 | 212 | u32 | shadow | 0/1 |
| 0x0D8 | 216 | u32 | beaming | 0/1 |
| 0x0DC | 220 | u32 | render | 0/1 (aabb: 0) |
| 0x0E0 | 224 | u32 | transparencyhint | 0–5 |
| 0x0E4 | 228 | u32 | renderhint | 0 none given (every game file), 1 `None`, 2 `NormalAndSpecMapped`, 3 `NormalTangents` (any case; others 0) [V: `nwmain compilemodel`] |
| 0x0E8 | 232 | char[64]×4 | texture0 (bitmap), texture1, texture2, texture3 | slots 1–2 unused in corpus; slot 3 see §20 |
| 0x1E8 | 488 | u32 | tilefade | 0, 1, 2, 4 |
| 0x1EC | 492 | Array | vertex indices (strips) | count always 0 |
| 0x1F8 | 504 | Array\<u32\> | left-over faces | count always 0 |
| 0x204 | 516 | Array\<u32\> | vertex-indices count | 1 element = faces×3 (bio/nmc); EE: 0 or larger (allocated size) |
| 0x210 | 528 | Array\<RPtr\> | raw vertex indices | 1 element: raw offset of u16[faces×3], equal to the face indices (bio/nmc 100%). The game draws this list. In tiles the EE compiler wrote (most of ttf02's cliffs, 1,264 rendered meshes over eight tilesets surveyed) it has more triangles than the face array, which is its first triangles: draw the list there (`Mesh::drawn`), the faces elsewhere |
| 0x21C | 540 | u32 | "something3" offset | 0xFFFFFFFF always |
| 0x220 | 544 | u32 | "something3" count | 0 always |
| 0x224 | 548 | u8 | triangle mode | 3 (triangles); 0 when empty; ~40 bio files have garbage |
| 0x225 | 549 | u8[3] | pad | |
| 0x228 | 552 | u32 | temp mesh data | runtime |
| 0x22C | 556 | RPtr | vertices V3[vcount] | null only for empty meshes |
| 0x230 | 560 | u16 | vertex count | |
| 0x232 | 562 | u16 | texture (tvert-set) count | 0 or 1, equals the number of non-null tvert pointers |
| 0x234 | 564 | RPtr[4] | tverts0..3 V2[vcount] | only set 0 is used in the corpus |
| 0x244 | 580 | RPtr | normals V3[vcount] | always present in bio/EE |
| 0x248 | 584 | RPtr | colors u32[vcount], R = low byte, then G, B, A | bio usually present, all 0xFFFFFFFF (only 34 sampled meshes differ) |
| 0x24C | 588 | RPtr[5] | "bumpmap anim" 1–5 V3[vcount] | only water meshes (419 bio, e.g. `tcn01_a15_01:Plane452`; 163 nmc). With a renderhint, the game's compiler puts the **tangents** in slot 4 (+0x258) [V] |
| 0x260 | 608 | RPtr | "bumpmap anim" 6 f32[vcount] | same; with a renderhint, the bitangents' signs (handedness) [V]. Given `tangents` are kept, else generated |
| 0x264 | 612 | u8 | lightmapped | 0 in bio/nmc; **garbage in EE** (0xCC in debug build) |
| 0x265 | 613 | u8 | rotatetexture | 0/1 (valid in EE too) |
| 0x266 | 614 | u16 | pad | |
| 0x268 | 616 | f32 | face-normal-sum / 2 | |
| 0x26C | 620 | u32 | unknown1 | 0 |

Vertices are per corner: verts, tverts, normals and colors share one index, so faces index all streams directly.
Smoothing groups are **not stored**; the decompiler rebuilds them from normals (`NwnMdlDecomp.cpp:934-997`).
The decompiler also merges duplicate positions back into ASCII vert/tvert lists (`FindRemapVertex`/`FindRemapTVertex` 1808-1892).
Which pointers are model-relative and which raw-relative: `NwnMdlR2A.cpp:339-407`.

### B.10 Face (0x20 = 32 bytes; `NwnMdlNodes.h:264-272`) [V]

| off | type | field |
|---|---|---|
| 0x00 | V3 | plane normal |
| 0x0C | f32 | plane distance d, with `n·p + d = 0` (189,373 of 189,505 bio faces within 1e-2) |
| 0x10 | u32 | surface / material id (walkmesh: surfacemat.2da row; render meshes: mostly 0–6, sometimes ≥40) |
| 0x14 | i16[3] | adjacent faces (−1 = none; always in range) |
| 0x1A | u16[3] | vertex indices |

ASCII face line: `v1 v2 v3 smoothmask t1 t2 t3 material` (`NwnMdlDecomp.cpp:1234-1241`).

### B.11 Skin (flags 0x61; `NwnMdlNodes.h:760-771`) [V]

| off | dec | type | field | notes |
|---|---|---|---|---|
| 0x270 | 624 | Array | weights (compiler temp) | count 0 in files |
| 0x27C | 636 | RPtr | skin weights f32[4×vcount] | raw |
| 0x280 | 640 | RPtr | bone refs i16[4×vcount] | raw; −1 = unused slot; index into the bone table |
| 0x284 | 644 | Ptr | node→bone map **i16**[n] | model data. **Not float** (verified: −1/0..k int16) |
| 0x288 | 648 | u32 | n = map count | = number of nodes in this model's own geometry tree |
| 0x28C | 652 | Array\<Q\> | qbone_ref_inv, one per node | stored **w,x,y,z** (identity = (−1,0,0,0) or (1,0,0,0)); see `NmcMesh.cpp:1749-1764` |
| 0x298 | 664 | Array\<V3\> | tbone_ref_inv, one per node | |
| 0x2A4 | 676 | Array\<u32\> | bone constant indices, one per node | runtime garbage (bio/EE), 0 (nmc) |
| 0x2B0 | 688 | i16[17] | bone → node number | unused slots hold garbage |
| 0x2D2 | 722 | i16 | "pad" | **used as the 18th bone slot** (`c_halaster` Skin_Robe: 18 bones, bone 17 → node 14; nwnmdlcomp off-by-one `NmcMesh.cpp:1854`) |
| 0x2D4 | 724 | | end (1.69 / bio / nmc / some EE) | |

- Node index = **DFS pre-order index** over the model's own tree, root = 0 (`GetNthNode`, `NwnMdlDecomp.cpp:2039-2078`; `NmcGetNodeIndex`, `NmcMesh.cpp:1561-1596`).
  Verified: n2b count = walked node count, and `bnn[n2b[i]] == i`.
- Up to 4 influences per vertex (`NmcMesh.cpp:1840-1868`). Bio skins have at most 18 bones.
- **EE variant**: `c_golemerald:Body` and `c_kocrachn:*` (EE-compiled) have struct size **0x3B0 (944)**.
  0x2B0..0x3AF is 256 bytes: bone node numbers at 0x2B0, zeros after them. Probably `i16[64]` plus 128 more bytes (64-bone support, wiki: "64 bones per skin") [U].
  No header flag separating the two EE layouts was found (`tni_udoor_19..27`, also EE-compiled, use 0x2D4).
  **Robust approach:** derive the bone list by inverting the node→bone map, never from the 0x2B0 array's length, and follow pointers rather than assuming struct sizes.
  nwnmdlcomp decompiled the 0x3B0 file without error because it only indexes bnn[bone<10].
- Binary weights reference bones; ASCII weights reference node names (`NwnMdlDecomp.cpp:1320-1352`).

### B.12 AnimMesh (flags 0xA1; `NwnMdlNodes.h:808-816`) [V]

| off | dec | type | field |
|---|---|---|---|
| 0x270 | 624 | f32 | sample period |
| 0x274 | 628 | Array\<V3\> | anim verts (compiler temp; always empty) |
| 0x280 | 640 | Array\<V3\> | anim tverts (always empty) |
| 0x28C | 652 | Array\<V3\> | anim normals (always empty) |
| 0x298 | 664 | Ptr | vertex sets V3[vcount × vsets]: **model data** (nwnmdlcomp's R2A wrongly uses raw; its decompiler uses model) |
| 0x29C | 668 | Ptr | texture sets V2[vcount × tsets]: model data, directly after the vertex sets |
| 0x2A0 | 672 | u32 | vertex set count |
| 0x2A4 | 676 | u32 | texture set count |

- **Index = vertex × setCount + set** (vertex-major). Verified: set 0 equals the base verts exactly under this order.
  The ASCII `animverts N` list is set-major, with N = sets × verts (`NwnMdlDecomp.cpp:1424-1462`).
- Sets live on the **animation** node (the model node has counts 0). There are ≈ length/sampleperiod + 1 sets, not always exactly.

### B.13 Danglymesh (flags 0x121; `NwnMdlNodes.h:853-857`) [V]

| off | dec | type | field |
|---|---|---|---|
| 0x270 | 624 | Array\<f32\> | constraints [vcount] (0–255), model data |
| 0x27C | 636 | f32 | displacement |
| 0x280 | 640 | f32 | tightness |
| 0x284 | 644 | f32 | period |
| 0x288 | 648 | | end: there is **no extra vertex-data pointer** (struct size verified 648) |

### B.14 AABB walkmesh (flags 0x221; `NwnMdlNodes.h:912-914, 280-295`) [V]

- Node +0x270: Ptr to the root entry, then end at 0x274. Entries follow the node in pre-order (`NwnMdlSerialize.cpp:1374-1410`).
- Entry, 40 bytes:

| off | type | field |
|---|---|---|
| 0x00 | V3 | bbox min |
| 0x0C | V3 | bbox max |
| 0x18 | Ptr | left child (0 on leaves) |
| 0x1C | Ptr | right child (0 on leaves) |
| 0x20 | i32 | leaf face index; −1 = inner node |
| 0x24 | u32 | most-significant plane: inner nodes 1 +X, 2 +Y, 4 +Z, 8 −X, 16 −Y, 32 −Z; leaves 0 |

- Leaves = faces in 8,869 of 8,875 trees. AABB meshes have render 0 and shadow 0.
- ASCII: `aabb` followed by one line per entry, `min max leafface` (`NwnMdlDecomp.cpp:1747-1778`).

### B.15 Light (flags 0x3; `NwnMdlNodes.h:457-471`) [V]

| off | dec | type | field | observed |
|---|---|---|---|---|
| 0x70 | 112 | f32 | flare radius | |
| 0x74 | 116 | Array | unknown ("something1") | always empty |
| 0x80 | 128 | Array\<f32\> | flare sizes | |
| 0x8C | 140 | Array\<f32\> | flare positions | |
| 0x98 | 152 | Array\<V3\> | flare color shifts | |
| 0xA4 | 164 | Array\<Ptr\> | flare texture names: offsets to NUL-terminated strings (model data) | the 4 flare counts are always equal (1–5) |
| 0xB0 | 176 | u32 | light priority | 1–5 (5 = 17,153) |
| 0xB4 | 180 | u32 | ambientonly | |
| 0xB8 | 184 | u32 | dynamic type (ASCII `ndynamictype` / `isdynamic`) | |
| 0xBC | 188 | u32 | affectdynamic | |
| 0xC0 | 192 | u32 | shadow | |
| 0xC4 | 196 | u32 | generateflare | always 0, even with flares |
| 0xC8 | 200 | u32 | fadinglight | |

Color, radius and multiplier are controllers (§8). There are no separate "flare" controller IDs.

### B.16 Emitter (flags 0x5; `NwnMdlNodes.h:508-524`) [V]

| off | dec | type | field | observed values |
|---|---|---|---|---|
| 0x070 | 112 | f32 | deadspace | |
| 0x074 | 116 | f32 | blastRadius | |
| 0x078 | 120 | f32 | blastLength | |
| 0x07C | 124 | u32 | xgrid | 1,2,3,4,5,8 |
| 0x080 | 128 | u32 | ygrid | |
| 0x084 | 132 | u32 | spawntype | 0, 1, 0xFFFFFFFF (409) |
| 0x088 | 136 | char[32] | update | Fountain, Explosion, Single, Lightning |
| 0x0A8 | 168 | char[32] | render | Normal, Linked, Billboard_to_Local_Z, Billboard_to_World_Z, Aligned_to_World_Z, Aligned_to_Particle_Dir, Motion_Blur |
| 0x0C8 | 200 | char[32] | blend | Normal, Lighten |
| 0x0E8 | 232 | char[64] | texture | |
| 0x128 | 296 | char[16] | chunkname (model name) | |
| 0x138 | 312 | u32 | twosidedtex | |
| 0x13C | 316 | u32 | loop | |
| 0x140 | 320 | u16 | renderorder | 0–10, 100 |
| 0x142 | 322 | u16 | pad | |
| 0x144 | 324 | u32 | flags | |

Flags (`NwnMdlNodes.h:188-201`, all bits seen): 0x1 p2p, 0x2 p2p_sel, 0x4 affectedByWind, 0x8 m_isTinted, 0x10 bounce, 0x20 random, 0x40 inherit, 0x80 inheritvel, 0x100 inherit_local, 0x200 splat, 0x400 inherit_part. No bits above 0x400.

The brief's "num branches, control-pt smoothing, frame blending, depth texture" are KotOR fields: they **do not exist** in NWN.

### B.17 Reference (flags 0x11) [V]

| off | type | field |
|---|---|---|
| 0x70 | char[64] | refModel (e.g. `fx_ref`, `fx_arrow`) |
| 0xB0 | u32 | reattachable (0/1) |

### B.18 Camera (flags 0x9) [S]

Node header only, 0x70 bytes. Absent from the corpus.

### B.19 Hierarchy, part numbers, supermodels, animation binding

- Tree: root = geometry header +0x48, recurse through children arrays. **Parent pointers are 0/garbage.** Node order = child order (pre-order).
- Part numbers (nwnmdlcomp):
  - A node's number is its file order (`NmcGeometry.cpp:305`).
  - With a supermodel the count starts at `super.nodecount + 1` (`NmcGeometry.cpp:1024-1030`), and nodes whose names match the supermodel tree, walked child-by-name case-insensitively, take the supermodel's part number (`NmcMergeNodePartNumbers` 525-560, `NmcMergeGeometryPartNumbers` 572-590).
  - Animation nodes get the model's numbers by the same name walk, or −1 (`NmcGeometry.cpp:1113`).
  - Data: `a_ba` (super `a_ba_non_combat`) nodecount 164, 57 own nodes, max part 161. `c_goldiamond` (super `a_ba`) 198/33/193. 14,677 animation nodes have −1.
  - Across all 25,597 binaries, header node count ≠ walked count in 858 models, and every one of them has a supermodel. It is equal in all 24,710 models without a supermodel, and in 29 models with one.
- Animation geometry is a separate node tree. Its nodes are full structs of their own flags: bio anims use dummy / trimesh / danglymesh / animmesh / emitter / light; EE anims may keep aabb/reference.
  Controllers there are keyed. Mesh anim nodes carry a mesh header (animmesh sets).
- The EE engine binds animation nodes to model nodes **by part number** (measured in the game client, 2026-10-07, by the Moonglow Viewer session and seen here in its pictures: a skeleton and its supermodels written again with fresh numbers under the same names no longer pose their creatures; one stands in its rest pose, a sitting man no longer sits). `mg-render` binds so for compiled models and animations, and by name where either was read from text (their compiler gives the numbers by the names) and for a worn model (a cloak's numbers are its own). Among the game's 647 compiled models with a compiled supermodel, the root always differs in name and not in number, and 70 have other nodes where the two ways disagree (`binding_by_part_number_and_by_name` in `models.rs`): a node of the supermodel's name numbered −1 or afresh in the model (not moved in the game), and old models whose own numbers fall on nodes a later supermodel gained (`c_mohrg`'s dangles under `a_ba`'s cloak bones). (Earlier text here said by name, unverified: How the EE engine binds animation nodes to model nodes is **[U]**: the part-number scheme suggests index binding for supermodel-shared skeletons, the wiki says "order of objects must match".)
  Animations are still found by walking the supermodel chain, a model's own overriding a supermodel's of the same name.
- Neverblender:
  - Animations come from the same file, or via *Import Supermodel* (`nvb_ops_io.py:509-603 NVB_OT_mdl_superimport` → `nvb_mdl.py:551-566 Mdl.create_super`).
  - It builds a `NodeResolver` keyed by lower-cased name with a trailing `.NNN` stripped (`nvb_utils.py:17-60`, `strip_trailing_numbers` 421-423).
  - Nodes resolve by name. On duplicate names the node index in the file breaks the tie (`get_obj` 35-49; `nvb_anim.py:45-48`).
  - On export children are sorted by original import order "important for supermodels/animations" (`nvb_mdl.py:311-317`, `nvb_anim.py:98-104`). It does not auto-load supermodels.
- The skin node index space (§11) is the **model's own tree** (DFS pre-order), not part numbers.

### B.20 EE-specific findings

1. The EE-compiled layout = 1.69 layout. No version field; routines, back-pointers, pads, model flags, lightmapped and controller pad hold garbage.
2. EE raw data may contain unreferenced zero gaps: the raw index list is allocated with the "vertex-indices count", which can be bigger than faces×3, and that count can be 0 in EE (15,511 meshes).
   No tangent or extra normal stream exists in base-game EE binaries (the gaps are zeros, and no pointer references them).
   Patch 8193.36 note: "validation of compiled models upon first load to fix invalid normals and tangents". Tangents are computed at load [U whether newer compiles store them].
3. EE skin 0x3B0 variant (§11).
4. Light controller 144 (§8).
5. **materialname / renderhint in binary [V: `nwmain compilemodel` on authored probes, 2026-10-01].** No base-game binary uses them (the base game ships only 7 `.mtr`, referenced via `bitmap`).
   The game's compiler writes `materialname` into the **texture3 slot** (+0x1A8) and drops `texture3` (given alone, the slot stays empty); `renderhint` is the number at +0xE4 (table above) and makes it store tangents and handedness (+0x258, +0x260). Integers wrap as in C (`spawntype -1` → 0xFFFFFFFF, `renderorder -2` → 0xFFFE). `colors` are stored only when given.
   It agrees with the one hint from before: the debug-built EE GUI model `ctl_cg_btn_col:Plane105` has `gui_cg_color` (an MTR name) in the **texture3 slot** (+0x1A8), so EE may store `materialname` in texture slot 3.
   +0xE4 is 0 in every game file. nwnmdlcomp drops both keywords (neverblender `docs/compiling.md:106-113`).
6. ASCII EE models in the base game (e.g. `c_karandas`) use `renderhint NormalAndSpecMapped|none`, `normals N`, `tangents N` (x y z handedness), `texture1/2`.

### B.20a Particles in the client [V: `client_render.rs` `particles_look`, 2026-10-01]

Probe emitters (an overridden `plc_a01` in a scratch user directory) in the sandboxed client, measured from its screenshots:
- Gravity is `mass` × 9.8 m/s² (side-on throws: 9.80 for mass 1, 4.83 for mass 0.5); `velocity` is m/s.
- `bounce`: at the ground a particle keeps 0.8 of its speed along it and 0.8 × `bounce_co` off it (`bounce_co` 1 from 3 m: hops of 1.87, 1.2, 0.75 m; 0.5 from 1.5 m: one hop of 0.22 m, then it slides to rest about 1 m out).
- `m_isTinted`: the colour times the light at the emitter, the same for all its particles: the area's ambient and diffuse colours added as they are (0x40 + 0x80 grey → 0xC0, whatever the facing); under tile lights alone 156–162 at three emitters, where the renderer's light attenuation gives 141–144.
- The three-stop values (`colorMid`, `alphaMid`, `sizeMid`, `percentStart/Mid/End` as fractions 0–1, percents 0–100 or bytes 0–255) change nothing: the client draws start to end. The game's compiler keeps them (B.8) all the same.
- `twosidedtex` changes nothing visible: one-sided `Aligned_to_World_Z` particles show from below too.
- Not measured: wind (`affectedByWind`, needs area wind), `splat`, `deadspace`.

### B.21 Reader recipe (Moonglow)

1. If the first 4 bytes ≠ 0, parse as ASCII. Otherwise `M=12`, `R=12+u32@4`.
2. Model header at M. Walk nodes from +0x48. Switch on `flags@+0x6C` (the only type tag).
3. Read the §8 controllers into (time, values). Convert quaternions from x,y,z,w; model-level values are the rest pose.
4. Meshes: vcount@+0x230, streams via RPtr (0xFFFFFFFF = none), faces@+0x78 (32 B). Ignore the strip/raw-index arrays.
5. Skins: bones from the i16 node→bone map (inverse), weights and refs from raw, qbone (w,x,y,z) and tbone per node index.
6. Animations: array @+0x78; each has its own tree and header (§6). Bind by name.
7. Use unaligned little-endian reads and bounds-check every offset (garbage exists in EE files).

### B.22 ASCII keywords

#### 22a. Neverblender importer (what it parses)

Header, `nvb_mdl.py:47-83 read_ascii_header`: `newmodel`, `setsupermodel <model> <super>` (lower-cased), `classification`, `setanimationscale`.
- Classification must be in `nvb_def.py:199-211`: unknown, tile, character, door, effect, gui, item, other.
- It splits the file on `node ` and `newanim ` (`nvb_mdl.py:97-154`) and does not look at `beginmodelgeom`/`endmodelgeom`/`donemodel` explicitly.
- **Not parsed:** `ignorefog`, `filedependancy`, `beginwalkmeshgeom`.

Node types (`nvb_mdl.py:22-31`): dummy, patch (= dummy), reference, trimesh, animmesh, danglymesh, skin, emitter, light, aabb. No camera.
pwk/dwk are walkmesh dummies (`nvb_def.py:153-162`).

| node | keywords (file:lines) |
|---|---|
| all | `node`, `endnode`, `parent`, `position`, `orientation` (axis-angle, 4), `scale`, `wirecolor` (`nvb_node.py:55-83`) |
| reference | `refmodel`, `reattachable` (`nvb_node.py:195-204`) |
| trimesh (and subtypes) | `tilefade`, `render`, `shadow`, `beaming`, `inheritcolor`, `rotatetexture`, `transparencyhint`, `shininess` (int!), `ambient`, `diffuse`, `specular`, `selfillumcolor`/`setfillumcolor`, `alpha`, `materialname`, `renderhint`, `bitmap`, `texture<N>` (N 0–14, `texture_list=[None]*15` line 255), `verts`, `faces`, `normals`, `tangents`, `colors`, `tverts`, `tverts<N>` (`nvb_node.py:260-342`) |
| danglymesh | `period`, `tightness`, `displacement`, `constraints` (`nvb_node.py:865-881`) |
| skin | `weights N` + N lines of `name weight` pairs (`nvb_node.py:950-986`) |
| emitter | `xsize`, `ysize` + `property_dict` (`nvb_node.py:1045-1109`, parse 1127-1145): update, loop, render, blend, spawntype, renderorder, birthrate, lifeexp, mass, velocity, randvel, particlerot, spread, splat, affectedbywind, colorstart/colorend (3), alphastart/alphaend, sizestart/sizeend, sizestart_y/sizeend_y, bounce, bounce_co, blurlength, deadspace, texture, chunkname, twosidedtex, m_istinted, xgrid, ygrid, fps, framestart, frameend, random, p2p, p2p_sel, p2p_bezier2/3, src, target, combinetime, grav, drag, threshold, blastradius, blastlength, lightningdelay/radius/scale, inherit, inheritvel, inherit_local, inherit_part |
| light | `radius`, `shadow`, `multiplier`, `color`, `ambientonly`, `ndynamictype`, `isdynamic`, `affectdynamic`, `negativelight`, `lightpriority`, `fadinglight`, `lensflares`, `flareradius`, `texturenames`, `flaresizes`, `flarepositions`, `flarecolorshifts` (counts are found by scanning for numbers, `nvb_node.py:1340-1423`) |
| aabb | trimesh keys; the **`aabb` tree lines are not parsed** (regenerated on export, `nvb_node.py:1546-1604`, `nvb_aabb.py`); face field 8 = walk material |

Animation header, `nvb_anim.py:52-72`: `newanim <anim> <model>`, `length`, `transtime`, `animroot`, `event <t> <name>`.

Animation nodes, `nvb_animnode.py:67-141`:
- `node <type> <name>`, `parent`, `sampleperiod`, `faces`, `animverts N`, `animtverts N`.
- Any `<prop>` or `<prop>key` where prop is in `object_properties` (position, orientation, scale, color, radius, alpha, selfillumcolor, setfillumcolor; lines 18-25) or in the emitter property_dict.
- A key list ends at the first non-numeric line, so `endlist` works.
- **`*bezierkey` is silently ignored** (`'positionbezier'` matches no property).

Also ignored: `texindices1..3` (the `f[4..6]` tvert indices are applied to **all** UV layers, `nvb_node.py:434-445`), `lightmapped`, `center`, `gizmo`, `multimaterial`, `showdispl`, `displtype`.

Export side:
- `nvb_node.py:111-140, 788-837`, `nvb_material.py:175-205`.
- It writes `bitmap <mtr>` (default `mat_mtr_ref='bitmap'`, `nvb_def.py:336`) or `bitmap/texture1/texture2` + `renderhint NormalAndSpecMapped`.
- `tilefade` only for tile/unknown/character, `rotatetexture` only for tile.
- `ndynamictype` for lights; faces as `v v v smooth t t t mat` (`nvb_node.py:771-778`).

#### 22b. nwnmdlcomp compiler keywords (`NmcAttribute.cpp:111-845`, controllers `NmcController.cpp:56-125`, model `NmcGeometry.cpp:890-1115`)

- Model: `newmodel`, `setsupermodel`, `classification`, `ignorefog`, `setanimationscale`, `beginmodelgeom`/`endmodelgeom`, `newanim`/`doneanim`, `donemodel`, `length`, `transtime`, `animroot`, `event`.
- Header: `wirecolor` (ignored), `inheritcolor`, `parent`.
- Light: `flareradius`, `ambientonly`, `shadow`, `nDynamicType`, `isdynamic`, `affectdynamic`, `lightpriority`, `generateflare`, `fadingLight`, `flaresizes`, `flarepositions`, `flarecolorshifts`, `texturenames`.
- Emitter flags: `p2p`, `p2p_sel`, `affectedByWind`, `m_isTinted`, `bounce`, `random`, `inherit`, `inheritvel`, `inherit_local`, `splat`, `inherit_part`.
- Emitter header: `renderorder`, `xgrid`, `ygrid`, `loop`, `twosidedtex`, `spawntype`, `deadspace`, `blastRadius`, `blastLength`, `update`, `render`, `blend`, `texture`, `chunkName`. Everything else goes through controllers.
- Reference: `refModel`, `reattachable`.
- Mesh: `bmin`, `bmax`, `diffuse`, `ambient`, `specular`, `shininess`, `shadow`, `beaming`, `render`, `transparencyhint`, `tilefade`, `rotatetexture`, `lightmapped`, `texture0`=`bitmap`, `texture1..3`, `vertexindicescount`, `leftoverfaces`, `vertexindices`, `verts`, `tverts`, `tverts1..3`, `colors`, `texindices1..3`, `mirrorlist`, `faces`.
- Skin: `weights`, `qbone_ref_inv`, `tbone_ref_inv`, `boneconstantindices`.
- Anim(mesh): `sampleperiod`, `animverts`, `animtverts`. Dangly: `displacement`, `tightness`, `period`, `constraints`. AABB: `aabb`.
- **Not supported:** `materialname`, `renderhint`, `normals`, `tangents`, `selfillumcolor` (only the typo `setfillumcolor`), `*bezierkey` is accepted.

#### 22c. What the base game's 7,235 ASCII models actually contain [V]

- Top level: `filedependancy` (also misspelt `filedependency`), `newmodel`, `setsupermodel`, `classification` (tile 3797, character 2704, gui 323, effect 139, item 126, door 70, effects 29, other 10), `setanimationscale`, `beginmodelgeom`, `endmodelgeom`, `newanim`, `doneanim`, `donemodel`.
- Animation header: `length`, `transtime`, `animroot`, `event`.
- Trimesh adds: `center`, `gizmo`, `multimaterial`, `showdispl`, `displtype`, `renderhint` (57), `normals` (57), `tangents` (57), `setfillumcolor` (2387), `sampleperiod`.
- Light uses `ndynamictype` (5757) or `isdynamic` (1257), `lensflares`, `fadinglight`, and rarely `ambient_only`/`n_dynamic_type`/`affect_dynamic`/`fading_light`.
- Emitter adds `update_sel`, `render_sel`, `blend_sel`, `opacity`, `p2p_type`, `lightningsubdiv`, `spawntype_sel`, `m_istnited` (typo).
- AABB: `walkmesh`, `multimaterial` + surface names, `aabb` tree.
- Animations: `*key` lists terminated by `endlist`, plus `centerkey` and `gizmokey`.
- No `ignorefog`, `materialname`, `*bezierkey`, `texindicesN` or `lightmapped` anywhere.

### B.23 Decompiled ASCII (nwnmdlcomp, run inside `$SP`; each ≤2 s)

`$SP/models/ascii/`:
- `a_ba.mdl` (bio, 125,987 lines, animations)
- `plc_a01.mdl` (bio placeable with 8 animations and emitters)
- `tcn01_a01_01.mdl` (bio tile with lights)
- `t_door09.mdl` (EE-compiled door)
- `c_golemerald.mdl` (EE 0x3B0 skin)
- `ashlw_011.mdl` (EE)
- `pfe0_belt004.mdl`
- `pmh0.mdl` (the game's own ASCII, copied)

Binaries are in `$SP/models/bin/`. Other files in `$SP/models/ascii/` dated 08:27 were not written by this task.

## C. Animations and supermodels

### C0. Sources, tags, tools (animation sub-research)

| Tag | Meaning |
|---|---|
| `[W:Name]` | nwn.wiki page (local mirror). URLs in section C7. |
| `[G]` | Ground truth from base-game data. Resman = keys + ovr, empty temp user dir. Decoded with my parser `$SP/models/anim/mdlinfo.py` (binary and ASCII MDL: header, supermodel, anims, events, node tree, mesh flags, controller ids). |
| `[G-survey]` | Survey of all 32,831 MDLs in the resman: `$SP/models/anim/survey.tsv`. Columns: resref, format, classification, supermodel, animscale, anims (`name:len:transtime:animroot`), special nodes. |
| `[TS]` | Strings in `bin/win32/nwtoolset.exe`, the real 37-17 toolset (from `strings -n 2` and `$SP/gff/toolset_strings_a.txt`). |
| `[ENG]` | Strings in `bin/linux-x86/nwmain-linux` (`$SP/models/anim/nwmain_str.txt`). |
| `[PDF-SO]` | BioWare "Situated Object (Door and Placeable) Format" PDF. Text copy: `$SP/research/door.txt`. |
| `[PDF-ARE]` | BioWare "Area File Format" PDF. Text copy: `$SP/research/bioware_are.txt`. |
| `[CL x]` | `$G/lang/en/docs/CHANGELOG.md`, release x. |
| `[PN x]` | nwn.wiki patch-notes page for release x. |
| `[inf]` | My inference, not stated by a source. |

`$SP` is the scratchpad: `/tmp/claude-1000/-home-august-Projects-moonglow-toolset/42266be1-7a5b-4aaa-92bd-b9aa74640348/scratchpad`.

Sample models are extracted to `$SP/models/anim/{bin,obj,tile}/`:
- `bin/`: the a_ba, h_ba, j_ba and phenotype family.
- `obj/`: creatures (c_*), placeables (plc_*), doors (t_door01/02, ttr_udoor_01 with DWK), FX (fx_*, vim_*, vco_*, vdu_*, vce_*) and skies.
- `tile/`: tiles and SET files.

**Binary MDL offsets I used** (relative to file offset 12). My parser works on base-game files with these offsets.

| Structure | Field offsets |
|---|---|
| Model header | name `0x08`[64], root node `0x48`, classification byte `0x72` (1 effect, 2 tile, 4 character, 8 door), ignorefog `0x73`, animation arraydef `0x78`, radius `0xA0`, **animscale `0xA4`**, **supermodel name `0xA8`[64]** |
| Animation header | length `0x70`, transtime `0x74`, animroot `0x78`[64], events arraydef `0xB8` (36-byte entries: `float time; char name[32]`) |
| Node | **part number `0x1C`**, name `0x20`[32], children `0x48`, controller keys `0x54`, controller data `0x60`, type flags `0x6C` |
| Mesh extension (node+0x70) | shadow `+0x64`, beaming `+0x68`, render `+0x6C`, texture0 `+0x78`, tilefade `+0x178` |
| Light extension (node+0x70) | lightpriority `+0x40`, ambientonly `+0x44`, nDynamicType `+0x48`, affectdynamic `+0x4C`, shadow `+0x50`, generateflare `+0x54`, fadinglight `+0x58` |

Node type flags: dummy `0x001`, light `0x003`, emitter `0x005`, reference `0x011`, trimesh `0x021`, skin `0x061`, animmesh `0x0A1`, dangly `0x121`, aabb `0x221`.

**Controller ids seen in animations [G]:**
- All nodes: 8 position, 20 orientation, 36 scale.
- Mesh: 100 selfillumcolor, 128 alpha.
- Light: 76 color, 88 radius, 96 shadowradius, 100 verticaldisplacement, 140 multiplier.
- Emitter: 80 alphaEnd, 84 alphaStart, 88 **birthrate** (the usual on/off switch), 96 colorEnd, 108 colorStart, 168/172 sizeStart/sizeEnd. The emitter names are from the xoreos convention [inf].

---

### C1. Animation names per object type, and which one the editor plays

#### C1.1 General semantics
- An animation is a named block (`newanim NAME model`) with:
  - `length` in seconds (a length of 0 is legal and common, for example `default`);
  - `transtime`, the blend time into this animation;
  - `animroot`, the top node it drives;
  - optional `event t name` entries.
  - [W:MDL ASCII] [W:Animations]
- Types [W:Animations]:
  - **Loop**: first frame == last frame.
  - **FNF**: fire and forget.
  - **Start/End** halves: `*start`, `*lp`, `*end`, for example `custom1start/custom1lp/custom1end`.
- Animated properties:
  - All nodes: position, orientation, scale.
  - Mesh: alpha, selfillumcolor.
  - Light: color, radius, shadowradius, multiplier, verticaldisplacement.
  - Emitter: most numeric parameters (birthrate, colours, alpha, sizes...).
  - The Loop/Tinted/Bounce/Wind flags cannot be keyed.
  - [W:MDL ASCII] [W:MDL ASCII Emitter Nodes]
- Nodes in an animation block may carry a different node type than in the geometry block. For example, a trimesh is often written as a dummy in the animation [W:MDL ASCII]. Seen in t_door01 [G].
- The name limit is **16 characters in practice**; longer animations do not play [W:Animations]. No base model exceeds it [G-survey].
- Many "state" animations are 1-frame poses (length 0.0333 s, or 0). The renderer must support zero-length animations: hold the first key.

#### C1.2 Tiles (classification `tile`)
Survey [G-survey]: 13,436 tile MDLs; 2,137 have animations.

| Animation | Tiles | Length (median/max) | Meaning |
|---|---|---|---|
| `animloop01` | 1148 | 0.5 / 60 s | Optional loop 1. Played while GIT `Tile_AnimLoop1`=1 (toolset tile properties; script `SetTileAnimationLoops`). Often a 1-frame pose that sets emitter birthrate (tcn01_c09_01, ttr01_h02_03), or a real loop (tin01_l02_03: 4 s orientation keys on hanging sacks). |
| `animloop02` | 123 | 0 / 16.3 | Loop 2 (`Tile_AnimLoop2`). |
| `animloop03` | 607 | 10 / 10 | Loop 3 (`Tile_AnimLoop3`). |
| `day`, `night` | 688 each | 0.3 | Held state during day or night. |
| `day2night`, `night2day` | 688 each | 0.033 / 0.67 | Played at dusk or dawn. Example: tcn01_a08_01 keys `selfillumcolor` of `window_a08_01` (glowing window) [G]. |
| `tiledefault` | 436 | 0 / 10 | "Reset" state: turns loops, lights and emitters off. Not for animating parts [W:Animations]. |
| `tile1` | 74 | 0.333 | Only in tni02 tiles. Purpose unknown (see open questions). |
| `default` | 14 | 10 | Rare. |

- The names are **`animloop01..03`, two digits**:
  - Survey [G-survey]: tile anims use `animloop01..03`.
  - PDF-ARE: "AnimLoop01", "AnimLoop02", "AnimLoop03".
  - TS strings: `AnimLoop01|AnimLoop02|AnimLoop03|tiledefault` next to the SET keys `AnimLoop1..3`.
  - SET keys and GIT fields use a single digit: `AnimLoop1`, `Tile_AnimLoop1`.
  - The tileset tutorial's `animloop1` spelling is wrong [W:Tileset Construction Tutorial].
- Engine strings: `AnimLoop01/02/03`, `Day`, `Day2Night`, `Night2Day`, `tiledefault` [ENG].
- PDF-ARE: "An AnimLoop can only be set if the correspondingly named animation actually exists on the tile model. Otherwise the Field value is 0."
  - SET files set `AnimLoop1=1` blindly [W:Tilesets].
  - The toolset must inspect the model to enable or disable the checkboxes.
- **Animated geometry must sit under the "a-node"**: a dummy named `<tilemodel>a`, child of the root [W:Animations] [W:Changing day/night tile states]. Tile geometry is otherwise static.
  - Survey: 5,071 tiles have an a-node.
  - Day/night anims: 686 of 688 are in a-node tiles. Animloops: 1,102 of 1,148 [G-survey].
  - The a-node also forces "dynamic" render order (transparency after static geometry). It was broken in early EE and fixed in 8193.35 [W:Tileset Construction Tutorial] [CL 87.8193.35-40 "Fixed 'a' nodes in models not always being rendered dynamic"].
  - Meshes under the a-node ignore tilefade and keyholing [W:Tileset Construction Tutorial].
- Day/night behaviour:
  - The game plays Day/Night/Day2Night/Night2Day at the module dawn and dusk hours.
  - If time jumps past dawn or dusk, the last state persists [W:Animations].
  - A toolset has no clock. It should play `day` or `night` according to the area `IsNight` or day-night-cycle setting [inf]. The TS strings show that the toolset reads `IsNight` and `DayNightCycle` with the sun and moon colours.
- Toolset evidence [TS]: the tile-instance code references `AnimLoop01`, `AnimLoop02`, `AnimLoop03`, `tiledefault`, `lightcolor` RED/GREEN/BLUE, and `fx_flame01` with `sourcelight1`/`sourcelight2`.
- **Suggested editor behaviour [inf]:**
  - Base state: `tiledefault` if present.
  - Then the day or night state.
  - Overlay each enabled `animloop0N` on a loop.
- Some animloops have no keys at all. Example: tcn01_q01_01 `animloop01` has 32 nodes and 0 controllers, with a danglymesh under the a-node [G]. Treat such an animation as a no-op.

#### C1.3 Placeables (model named by `placeables.2da` ModelName; classification mostly `character`)
**AnimationState** (GIT/UTP BYTE) → required animation [PDF-SO Table 4.1.2]:

| State | Value | Animation on the model |
|---|---|---|
| default | 0 | `default` |
| open | 1 | `open` |
| closed | 2 | `close` |
| destroyed | 3 | `dead` |
| activated | 4 | `on` |
| deactivated | 5 | `off` |

- PDF-SO: "A particular animation state is only available if the model actually contains an animation of the name."
- Values seen in base UTPs [G]: 0 ×1413, 4 ×32, 3 ×4, 2 ×2, 5 ×2.
- Toolset right-click "Initial State" (for example "Activated" lights a torch) [BioWare toolset tutorial, text at `$SP/research/aurora_tut.txt` step 7].
- TS: `TNWPlaceableInstance::CreateModel` is followed by `default chrome1 open close on off off dead default`, so the toolset shows the **state pose** animation.

Full placeable set. Survey of 1,268 placeables.2da models [G-survey]; lengths are medians [G].

| Animation | Count | Length | Notes |
|---|---|---|---|
| `default` | 809 | 0 (max 33 s) | Static/idle pose. Used when static; some are long loops (flags, mills) [W:Animations]. |
| `damage` | 766 | 0.133 | Shake on damage. |
| `die` | 777 | 0.167 | Usually a `detonate` event (chunk emitters) plus sinking. |
| `dead` | 777 | 0.033 | Underground or invisible pose. |
| `on`, `off` | 144/143 | 0.033 | State poses. Emitter birthrate or light keys (plc_i05 brazier keys emitter birthrate). |
| `off2on`, `on2off` | 122/121 | 0.033 | Transitions. |
| `open`, `close` | 42/43 | 0.033 | State poses (chest lid). |
| `close2open`, `open2close` | 41 | 0.633 (max 2.97) | Transitions. |
| `use` | 8 | 0.033 | For example plc_c03 trapdoor. |

- Combinations seen:
  - die/dead+default: 635.
  - No animations: 431.
  - on/off+die/dead+default: 100.
  - open/close+die/dead+default: 38.
- Engine code inlines exactly these names: `open on2off dead close open2close damage default close2open off2on` [ENG].
- Script constants [W:Animations] [nwscript.nss]:
  - `ANIMATION_PLACEABLE_ACTIVATE` 200 (plays `off2on` then holds `on` [inf]).
  - `DEACTIVATE` 201 (`on2off`, then `off`).
  - `OPEN` 202 (`close2open`, then `open`).
  - `CLOSE` 203.
- Wiki rows `on`/`open` with length 1–50 frames are examples only. There are no `opened`/`closed` names for placeables; `opened` appears once in the whole data [G-survey].
- Placeables with inventory disable on/off/on2off/off2on/default [W:Animations].
- **Static placeables cannot animate.** They are baked into the tile static mesh, have no LOD, and cannot be scaled. Danglymesh still moves [W:Placeable].
- `placeables.2da.Static`=0 blocks "Static" for skinmesh/animmesh models, which crash when static [W:placeables.2da] [W:Animations].
- Light on placeables is **not** an animation: `placeables.2da.LightColor` spawns `fx_placeable01.mdl` at `LightOffsetX/Y/Z`, coloured from `lightcolor.2da`. Toggled with `SetPlaceableIllumination` [W:placeables.2da] [W:Area Lighting].
- A few placeables use supermodels (tnp_tree_a01 ×59, tnp_flag_c01 ×25, tnp_wndw_b14 ×16) [G-survey].
- **Suggested editor behaviour:** play the AnimationState animation on a loop. Fall back to `default`, then to the bind pose.

#### C1.4 Doors (classification `door`; generic doors from `genericdoors.2da`, tileset doors from `doortypes.2da`)
**AnimationState** [PDF-SO]:
- 0 = closed.
- 1 = opened1: opens toward the toolset wireframe arrow.
- 2 = opened2: opens the opposite way.
- Toolset right-click Initial State: Closed / Opened Forwards / Backwards [W:Door].

| Animation | Doors | Length (median) | Notes |
|---|---|---|---|
| `closed` | 206 | 0.033 | Held pose. |
| `opened1`, `opened2` | 207/206 | 0.033 | Held poses. |
| `opening1`, `opening2` | 207/206 | 1.0 | Transitions (`ANIMATION_DOOR_OPEN1` 205, `OPEN2` 206). |
| `closing1`, `closing2` | 207/206 | 0.667 | Transitions (`ANIMATION_DOOR_CLOSE` 204). |
| `trans` | 249 | 0.033 | Keys the alpha of the `sam` mesh, the area-transition highlight [G t_door01, ttr_udoor_01] [W:Animations]. |
| `die`, `dead` | 200/198 | 0.3/0.033 | `die` has a `detonate` event; `DESTROY` = 207. |
| `default` | 45 | 0.033 | "Used if there is no visible model" [W:Animations]. |
| `damage` | 30 | 0.2 | |

- TS: `TNWDoorInstance::CreateModel` is followed by `opened1 opened2 closed trans` and `opening1 opening2 closing1 closing2 closed opened1 opened2`.
- Doors fade (alpha) when the camera is on the "wrong" side, which is the `door` classification effect [W:MDL ASCII classification]. Doors with all meshes at alpha 100 do not fade [W:Door].
- **Editor behaviour:** play `closed`, `opened1` or `opened2` per AnimationState. Also play `trans` when the door is a transition (LinkedTo set) [inf].

#### C1.5 Creatures (idle for previews and area view)
The **idle loop** depends on appearance.2da `MODELTYPE`:

| MODELTYPE | Idle loop | Where it comes from |
|---|---|---|
| P (parts) and F (full) | **`pause1`** | a_ba chain, 2.0 s, transtime 0.7, animroot rootdummy [G] |
| S (simple) and L (large) | **`cpause1`** | The model itself; lengths 1.2–3.3 s (wolf 1.967, ogre 2.0, beholder 2.0, drider chief 3.3) [G] |

- TS evidence of the toolset preview order:
  - Creature: `... head_g ... pause1 | cpause1 | WEAPONSCALE | rhand | lforearm | lhand`.
  - Wings and tails (`TAILMODEL`/`WINGMODEL`): `pause1 | cpause1 | creadyl`.
  - So the toolset tries pause1, then cpause1, then (for wing and tail submodels) creadyl, and attaches equipped items at `rhand`/`lhand`/`lforearm` scaled by `WEAPONSCALE`.
- In game, weapon overlays change idle poses [W:Animations]. These are optional for a preview:
  - `plpause1` (polearm or double weapon; animroot rbicep_g).
  - `xbowr` (crossbow; animroot rforearm_g).
  - `torchl` (torch; animroot lbicep_g).
- Other idle variants: `pause2`, `pausesh`, `pausebrd`, `pausetrd`, `pausepsn`, and `cpause2` on 1 S model.
- **Full list (P/F)**: see [W:Animations] "Full Animation List". Groups: idle, walk/run (+overlays `walk_shieldl/_swordl/_swordr/_bowl`, `run_*`), WASD steering (`pauseturn`, `pausewalkl/r/fl/fr`, `runfl/fr`), emotes and talk overlays, sit/kneel/meditate/worship/get*, weapon sets `1h* 2h* 2w* pl* nw*` (readyl/r, slashl/r, stab, closeh/l, reach, parryl/r, `*slasho` overlays), dodge, damage (`damages/l/r/b`), combat steps (`cwalkf/b/l/r`, `cturnr`), knockdown/death (`kdbck*`, `kdfnt*`, `gutokd*`, `gustand*`, `deadfnt/deadbck`), ranged (`bowrdy/bowshot/xbowrdy/xbowshot/throwr`), spells (`conjure1/2`, `castout/self/up/area/point` + `*lp`, `special`), `appear/appear2/disappear/disappear2/disappearlp`, `whirlwind`, `mount1start/mount1lp`, `customNstart/lp/end` (N = 1–70), `drwright/drwleft`, `victoryfr/mg/th`, `spasm`, `hturnl/r`, `steal`, `taunt`.
- a_ba contains 165 of these [G].
- Extra animations present in data but not on the wiki page [G]:
  - `custom3start/lp/end` and `custom4start/lp/end` in a_ba.
  - `custom1start/custom1lp/custom6lp` and `mount1start/mount1lp` in a_ba_custom / a_ba_coat_cus.
  - `dismount1start/dismount1lp` and `custom3lp/4lp/5lp` in h_ba_custom.
  - `2hslasho`, `plslasho`.
- **Simple list (S/L)** [W:Animations] [G]: `cpause1 cwalk crun ctaunt cgetmid cgetmidlp chturnl chturnr creadyr creadyl ca1slashl ca1slashr ca1stab cclosel ccloseh creach cparryr cparryl cdodgelr cdodges cspasm cdamagel cdamager cdamages ccwalkf ccwalkb ccwalkl ccwalkr ccturnr ckdbck ckdbckps ckdbckdmg cguptokdb cgustandb ckdbckdie cdead cconjure1 ccastout ccastoutlp cspecial cappear cappear2 cdisappear cdisappear2 cdisappearlp`.
  - L adds `bowrdy bowshot xbowrdy xbowshot xbowr`.
  - Extras seen [G]: `ca2slashl ca2slashr ca2stab cparrys` (badger, rat: a second attack set), `cpause2`, `cpauseturn`, `ccturnl`.
  - 12 "S" appearances use placeable-like on/off anims.
- Survey of appearance.2da models [G-survey]:
  - F 520: supermodel a_ba* 395, h_ba 66, c_* 43, NULL 9, pmh0/pfe0/pmd0/pme0 7.
  - S 236: NULL 112, c_* 124.
  - L 59: NULL 37, c_* 22.
- Wings, tails, robes, cloaks and weapons are separate models with their own copies of the same animation names (c_wingbat has `2hreadyr`, ...). The engine plays the same-named animation on attached models [W:Animations "Weapon Animations"] [G]. The `bowshot` animation in wbwln_t_011 is inherited by wbwln_t_012 through setsupermodel [W:Animations].

#### C1.6 Items
- Palette and preview models are generally static.
- Survey: 73 of 2,913 item-like models have animations:
  - Weapon mirrors of creature names (`1hreadyr`, `throwr`, `bowshot`...).
  - `default`.
  - Ammo and projectiles: `impact01`, `travel01` [G-survey].
- Weapon supermodels (wblfh_t_011 ×48, wbwln_* ×62, wbwlc_* ×53, wbwsh_* ×46, wbwx* ×28) share animations.
- An item preview needs no animation. Optionally play `default` [inf].

#### C1.7 VFX models (classification `effect`)
Animation names by prefix [G-survey]:

| Animation | Used by | Semantics |
|---|---|---|
| `impact` | vim_*, vff_*, vwp_*, vps_*, vpm_*, some vdr_*/fx_* (409 models) | Played when the effect starts. On an object it lasts the effect duration. Instant ground effects last about 2 s [W:Animations]. Often only `event 0 detonate` (vim_magblue: len 1.63, two detonates) [G]. |
| `duration` | vdr_*, vps_*, vpm_*, fx_shadow_* (147) | Loops while the effect lasts (object or AOE only) [W:Animations]. |
| `cessation` | vdr_*, vce_*, vps_* (148) | Plays at removal [W:Animations]. `ProgFX_Cessation` lasts exactly 1 s [W:visualeffects.2da]. |
| `conjure01` + `fade` | vco_* (129/128) | Conjure loop during spell conjuration, then `fade` (with `detonate`) [G]. Engine strings `conjure01`, `donefade` [ENG]. |
| `cast01` | vca_*, var_* (cone and area cast visuals), beams | Cast animation. progfx type-7 beams list `cast01` as Param2 [W:progfx.2da]. |
| `travel01` | vpr_* (projectiles), ammo | Projectile flight. |
| `default` | 3 | "played... who knows when": fallback [W:Animations]. |
| `1`..`15` | fx_flame01 | Tile source-light colour index (C3.5). |
| `Blue_5m`.. `Red_20m`, `Dark_Vision`... | fx_light_clr | Vision and light colours, referenced from progfx lighting lines [G]. |

- Many emitter-only VFX have **no animation**. Examples: vdu_beam000, fx_ref, sky models [G]. The renderer must run emitters from their static parameters.
- **Suggested preview behaviour [inf]:**
  - Play `impact`, then loop `duration`, and allow `cessation` on demand.
  - vco_*: loop `conjure01`.
  - vca_*/var_*: `cast01`.
  - vpr_*: `travel01`.
  - Otherwise `default`, or no animation.

---

### C2. Supermodels, animation lookup, animroot, animation scale, events

#### C2.1 Header and lookup
- `setsupermodel <model> <super|NULL>`: the model "inherits animations; may overwrite any animation from its supermodel" [W:MDL ASCII].
- `setanimationscale <float>` scales animations to "avoid shearing when object sizes don't match the supermodel" [W:MDL ASCII]. Default 1.0, and it is always present when compiled [W:Model Table of Parameters].
- Lookup [inf, consistent with all data]: search the model's own animation list by name (case-insensitive), then its supermodel, recursively until NULL.
  - Example: pmh0 has no animations. `pause1` is found in a_ba.
  - a_ba holds 165 animations, but some exist only further up. `custom6lp` and `mount1start` are only in a_ba_custom; `disappearlp` is in a_ba_casts and a_ba.
- A missing supermodel is harmless. EE stopped the attempt to read drive N: [PN 1.81.8193.16]. There is also a "Safeguard against crashes when an animation was missing" [PN 1.83.8193.21].

#### C2.2 Node matching (important for the renderer)
- The wiki says "the structure of the two models must match, i.e. the order of the objects in both files must be the same", and that node order "defines the internal part number" [W:MDL ASCII] [W:Model Table of Parameters].
- **Ground truth [G]:** every binary node stores a **part number** (`node+0x1C`):
  - A model with a NULL supermodel has part number == own node index (a_ba_casts 26/26, c_wolf 30/30, plc_a08 17/17).
  - A model with a supermodel has part number = the **part number of the same-named node in its direct supermodel**, or **-1 (0xFFFFFFFF)** if that name does not exist there. Examples:
    - neck_g is 14 in a_ba_casts, a_ba_med_weap, a_ba_non_combat, a_ba, a_fa and c_goblinA.
    - c_dog's `Box01`/`Box02` are -1 because c_wolf has no such nodes.
    - a_ba's `head`, `lshoulder_g` and `headconjure` are -1 because a_ba_non_combat lacks them, although a_ba_casts has headconjure.
  - So the compiler resolves **by name against the direct supermodel** and caches the result as the part number.
  - Node order and count can differ (c_goblinA 29 nodes against a_ba 57; c_orcA against a_ba_non_combat).
  - The root dummy maps to the root: part number 0, and animroot may be the ancestor's model name, e.g. `animroot a_fa` or `c_Wolf`.
- **Implementation [inf]:**
  - Map animation node → instance node by case-insensitive name. Map the supermodel root name to the instance root.
  - Nodes that exist only in the child model (part number -1) are not driven by inherited animations.
  - ASCII models need the same name resolution at load time.
  - Names are ≤ 32 characters. Longer names truncate and cause mismatches [W:Model Table of Parameters] [W:Models].

#### C2.3 animroot and layering
- `animroot` is the top node the animation drives [W:Animations] [W:MDL ASCII]. Full-body animations use `rootdummy` or the model name. Partial and overlay animations use a limb:
  - `walk_shieldl`/`run_shieldl`/`walk_bowl`: `lbicep_g` (a_ba_non_combat), or `torso_g` (a_ba).
  - `walk_swordr`, `run_swordr`: `rbicep_g`.
  - `plpause1`: `rbicep_g`.
  - `xbowr`: `rforearm_g`.
  - `torchl`: `lbicep_g`.
  - Talk, listen, salute, drink, read, greeting, drwright, pausesh: `torso_g`.
  - `*slasho`: `torso_g` [G].
- "Multiple animations can be mixed... The animation with the deeper animroot takes precedence." So an overlay replaces the base animation on the subtree under its animroot [W:Animations].
- `transtime`: cross-fade time into the animation. Examples:
  - pause1 0.7; attacks 0.25–0.3; readies 0.5–0.75 [G].
  - Placeables and tiles are mostly 0.25; die/dead/damage are 0 [G].

#### C2.4 The a_ba family (verified chains [G])

| Model | Supermodel | Animations | Role |
|---|---|---|---|
| a_ba_casts | **NULL** (root, 26 nodes) | 19 | conjure1/2, cast*, victory*, appear/disappear/disappearlp, whirlwind |
| a_ba_custom | a_ba_casts | 3 | mount1start, mount1lp, custom6lp |
| a_ba_med_weap | a_ba_custom | 67 | 1h/2w/nw combat, dodge, damage, combat steps, knockdown/death, bow/xbow, throwr, *slasho, cturnr |
| a_ba_non_combat | a_ba_med_weap | 56 | walk/run and overlays, pause*, emotes, talk, sit/kneel/meditate/worship/get*, torchl, drw*, WASD set, plpause1, xbowr |
| **a_ba** | a_ba_non_combat | 165 | Full male "base" set (+2h/pl sets, custom3/4) |
| a_fa | a_ba | 49 | Female overrides: walk/run, 2h/pl, knockdown, pause1/2, taunt, sit... |
| a_da | a_ba (animscale 1.03) | 22 | Dwarf male |
| a_dfa | a_ba (1.01) | 38 | Dwarf female |
| a_ba2, a_fa2, a_da2, a_dfa2 | a_ba / a_ba2 | 20/50/22/40 | Phenotype 2 (large) |
| a_ba_coat_cus → a_ba_coat; a_fa_coat → a_ba_coat; a_da_coat, a_dfa_coat; a_ba2_coat(_cus); a_fa2_coat | chain as named | — | Supermodels for **robe** (skinmesh) models, e.g. pmh0_robe020 → a_ba_coat, pfh0_robe020 → a_fa_coat |
| h_ba_casts (NULL) ← h_ba_custom ← h_ba_med_weap ← h_ba_non_combat ← **h_ba**; h_fa_med_weap ← h_ba; h_fa_non_combat ← h_fa_med_weap; **h_fa** ← h_fa_non_combat | | | **Mounted** phenotypes 3 and 5 (h_ba 25 anims, h_fa 13) |
| j_ba | h_ba | 18 | **Jousting** phenotypes 6 and 8 (walk/run/pause1 2.967 s...) |

**Player phenotype models** (`p<gender><race><phenotype>.mdl`, a skeleton of dummies; body parts attach at `capart.2da` NODENAME) [G]:
- pmh0 → a_ba (1.0).
- pfh0 → a_fa (1.0).
- pmh2 → a_ba2.
- pfh2 → a_fa2.
- pme0 → a_ba (0.895).
- pfe0 → a_fa (0.894).
- pma0 (halfling) → a_ba (0.65).
- pfa0 → a_fa (0.64).
- pmg0 (gnome) → **a_da** (0.62).
- pfg0 → a_fa (0.64).
- pmd0 → a_da (0.66).
- pfd0 → a_dfa (0.65).
- **pmo0 (half-orc male) → a_da** (1.0).
- **pfo0 → a_dfa** (1.0). This explains why half-orcs move differently from humans.
- pmh3 → h_ba.
- pmh6 → j_ba.

Other rules [G]:
- pma0/pmd0/pmg0/pfa0/pfd0/pfg0 carry their own `sit`/`sitdown`.
- Body-part MDLs (pmh0_chest001, pmh0_head001...) have NULL supermodels. They are rigid parts parented at nodes (`capart.2da`: FOOTR→rfoot_g, CHEST→torso_g...).
- Robes and cloaks are skinmeshes with bone supermodels: pmh0_cloak_00N → pmh0; robes → a_ba / a_*_coat. Cloak models use a 17-bone skeleton plus one skinmesh (including torso_g and rootdummy) [W:Cloaks].
- Phenotype ids [W:Part-Based Models] [2da phenotype.2da]:
  - 0 Normal, 1 Skinny (unused), 2 Large.
  - 3 Normal_M, 5 Large_M (mounted).
  - 6/8 Joust.
  - Up to 20.

**Creature bases** [W:Creature BASE Models] [G]:
- Rows are "supermodel ← users". Verified by me:
  - c_dog → c_wolf.
  - c_drgred → c_drggreen.
  - c_dmsucubus (F) → a_fa.
  - c_goblinA/X → a_ba (animscale 0.58).
  - c_orcA → a_ba_non_combat (0.942).
  - c_invdrg_0N0 → c_drgblack.
- Most S/L creatures have NULL supermodel and their own ~42 c-animations.

#### C2.5 Animation scale
- Observed values [G]:
  - 1.0 for most.
  - Races: a_da 1.03, a_dfa 1.01, pme0 0.895, pma0 0.65, pmg0 0.62, pmd0 0.66.
  - Creatures: goblins 0.58, orcA 0.942, h_fa_coat 0.988.
  - Clean Models EE auto-sets 0.75 when scaling models [W:Resizing Models...].
- Implementation [inf]: when playing an animation found in an **ancestor**, multiply its **position** keys (translation, notably rootdummy height and stride) by the *instance model's* animscale. Orientation keys are unchanged.
  - This is the conventional Aurora/xoreos behaviour and matches the purpose "avoid shearing".
  - Unverified in code.
- Separate runtime speed scale: `SetObjectVisualTransform(..., OBJECT_VISUAL_TRANSFORM_ANIMATION_SPEED=40)`.
  - GFF VisualTransform structs carry `AnimationSpeed` plus Scale/Rotate/Translate [TS].
  - Cloaks and robes inherit it [PN 1.81.8193.17].
  - `PlayAnimation(n, fSpeed, fSeconds)`.
- Walk and run have engine-assumed durations. Stretching changes the speed [W:Animations]. appearance.2da `WALKDIST`/`RUNDIST` = metres per cycle (use for walk previews).

#### C2.6 Events (`event <time> <name>`)
Counts over all MDLs [G-survey]:
- `snd_footstep` 14189.
- `hit` 4960.
- `detonate` 3115: fires all `update Explosion` emitters and a wind blast (blastRadius/blastLength).
- `blur_start`/`blur_end` 2866: weapon blur trail.
- `snd_hitground` 1323.
- `cast` 278.
- `parry` 50.
- `draw_weapon` 20.
- `draw_arrow` 17.
- `1080` 2: junk.

The wiki says `cast`, `draw_arrow` and `parry` are "not in use" [W:MDL ASCII], but they are in the data. `donefade` is used for spell conjure effects only [W:MDL ASCII] [ENG]. Compiled a_ba duplicates events (each twice) [G], so deduplicate them.

**Editor needs:** `detonate` (to fire explosion emitters when previewing die/impact); the others can be ignored or turned into audio hooks.

#### C2.7 Custom and replaced animations
- `custom1..70` (`start`/`lp`/`end` suffixes; ANIMATION_LOOPING_CUSTOM21–70 added in 1.80.8193.6) [PN 1.80.8193.6].
- Walk anims unhardcoded: `walk_002`, `walk_003`... (001 = walkdead, 002 = walkinj) [PN 1.80.8193.14].
- `ReplaceObjectAnimation(obj, old, new)` since 8193.35. `EMOTE_ANIMATIONS_USE_MDL_TIMINGS` in ruleset.2da [CL 89.8193.37-13].

---

## D. Part-based creatures, PLT colouring, items, placeables, doors

Scratch copies: `$SP/models/data/` (2DA+CSV, sample MDL/PLT/TGA), `$SP/models/ascii/`, `$SP/models/gff/` (UTI/UTC as NWNT), `$SP/models/png/` (decoded PLTs/palettes). `[W:<id>]` = wiki page id (URLs in H).

### D1. appearance.2da, phenotype.2da

#### appearance.2da columns (VERIFIED header, 35 cols, 15100 rows, 838 non-blank)
`LABEL, STRING_REF, NAME, RACE, ENVMAP, BLOODCOLR, MODELTYPE, WEAPONSCALE, WING_TAIL_SCALE, HELMET_SCALE_M,
HELMET_SCALE_F, MOVERATE, WALKDIST, RUNDIST, PERSPACE, CREPERSPACE, HEIGHT, HITDIST, PREFATCKDIST, TARGETHEIGHT,
ABORTONPARRY, RACIALTYPE, HASLEGS, HASARMS, PORTRAIT, SIZECATEGORY, PERCEPTIONDIST, FOOTSTEPTYPE, SOUNDAPPTYPE,
HEADTRACK, HEAD_ARC_H, HEAD_ARC_V, HEAD_NAME, BODY_BAG, TARGETABLE` [W:38174941]

Render-relevant:
| Col | Meaning for renderer |
|---|---|
| LABEL / STRING_REF | toolset display name (TLK strref; LABEL if strref blank). NAME unused ("BASE" marks supermodel bases [W:49447555]). |
| RACE | non-P: model resref (case-insensitive, e.g. `c_DrgBlack`). P: single race letter used in part names (`D E G A H O`). |
| ENVMAP | `default` = area/SET env map (toolset: `Chrome1`, see below); `****` = no env map → texture alpha is transparency; else a TGA/DDS resref (`evmap_irrid`, `evmap_azer`, `dlag__ref01`). Ignored if a TXI sets one. Counts: default 626, **** 203. |
| MODELTYPE | see below; P=parts, S, F, L; suffix W/T allow wings/tails. |
| WEAPONSCALE | uniform scale applied to held weapon models (not S). Human 1, halfling 0.8, half-orc 1.3. |
| WING_TAIL_SCALE | scale applied to wing/tail models. |
| HELMET_SCALE_M / _F | scale of helmet model by creature gender (P only). Human 1.05/0.85, elf 0.85/0.8, dwarf 1.15/0.95, gnome 0.9/0.82, halfling 0.7/0.65, half-orc 1.2/1.2, half-elf 1/0.9. |
| HEIGHT | HEIGHT − 0.5 = default camera height; also pathing (not a render scale) [W:38174941, W:38174921]. |
| SIZECATEGORY | 1..5 tiny..huge; picks Imp_Root_{S,M,L,H}_Node VFX [W:38175069]. |
| HEAD_NAME | node used for head tracking/eye contact, default `head_g`; HEADTRACK/HEAD_ARC_H/V limits. |
| PORTRAIT | default portrait base (`po_poly` → `po_poly_{h,l,m,s,t}`); should match a portraits.2da BaseResRef [W:38174941]. |
| PERSPACE/CREPERSPACE | selection/pathing radii (useful for selection circles). |

MODELTYPE (VERIFIED counts: F 258, FWT 240, S 177, SWT 40, L 38, P 23, LWT 21, lowercase `s` 19, FT 15, FW 7) [W:38174941, W:38176272]:
- **P** parts/player: skeleton `p<g><RACE><pheno>.mdl` + part models + PLT textures; armor changes body; colors selectable.
- **S** simple: single MDL, TGA/DDS, no PLT, no visible weapons; creature anims (`cwalk`, `crun`, ...).
- **F** full: single MDL, PC-style anims (`walk`, `run`), weapons/shields visible via `rhand`/`lhand`/`lforearm` dummies; supermodel usually an a_ba variant.
- **L** limited/large: S-style anims but shows weapons/shields (same dummies) and bow/xbow anims.
- suffix **W** (wings allowed, needs `wings` dummy), **T** (tails allowed, needs `tail` dummy). `FWT/SWT/LWT` "Invisible_*" rows (569+, 849+) are invisible bodies whose tail slot carries a real creature model = scaling trick [W:53670835].
- lowercase `s` appears on object-as-creature rows (431 ObjectChair RACE=PLC_X02 ...) — treat case-insensitively.

Representative rows (VERIFIED):
| ID | LABEL | RACE | ENVMAP | MT | WEAPSCALE | HEIGHT | SIZE | HEAD_NAME | PORTRAIT |
|---|---|---|---|---|---|---|---|---|---|
| 0 | Dwarf | D | default | P | 1 | 1.5 | 3 | head_g | **** |
| 1 | Elf | E | default | P | 1 | 1.75 | 3 | head_g | **** |
| 2 | Gnome | G | default | P | 1 | 1.5 | 2 | head_g | **** |
| 3 | Halfling | A | default | P | 0.8 | 1.5 | 2 | head_g | **** |
| 4 | Half_Elf | H | default | P | 1 | 2 | 3 | head_g | **** |
| 5 | Half_Orc | O | default | P | 1.3 | 2.25 | 3 | head_g | **** |
| 6 | Human | H | default | P | 1 | 2 | 3 | head_g | **** |
| 8 | Badger | c_badger | **** | S | **** | 1 | 1 | Badger_head | po_Badger |
| 41 | Dragon_Black | c_DrgBlack | **** | S | **** | 1 | 5 | Dragon_head | po_DrgBlack |
| 86 | Goblin_A | c_goblinA | default | F | 0.8 | 1 | 2 | head_g | po_GoblinA |
| 38 | Balor | c_demon | default | L | 2.3 | 1 | 4 | **** | po_Demon |
| 163 | Succubus | c_dmsucubus | **** | FW | 1 | 1 | 3 | head_g | po_DmSucubus |
| 298 | Invisible_Human_Male | c_invsguy | default | F | 1 | 1 | 3 | **** | po_hu_m_99 |
P rows: 0–6 plus 474 Dwarf_Golem(D), 475 Dwarf_HalfOrc(O), 482–495 `<race>_mounted[_f]` (same letters; mount handled by phenotype 3/5/6/8 + tail slot).
Half-elf and human share letter H (same models) [W:38176932].

#### phenotype.2da (VERIFIED)
`Label, Name(strref), DefaultPhenoType`:
```
0 Normal 2223 0 | 1 Skinny **** 0 | 2 Large 2225 0 | 3 Normal_M 111015 0 | 4 **** 0
5 Large_M 111016 2 | 6 N_Joust_M 111017 0 | 7 **** 0 | 8 L_Joust_M 111018 2
```
- Phenotype digit(s) go straight into model names (`pmh0`, `pmh2`, `pmh12_...`); safe range 0–99; resref ≤16 chars [W:38176344]. GFF `Phenotype` is INT on UTC.
- DefaultPhenoType = fallback phenotype for any part MDL (and heads) missing for the requested phenotype (1.67) [changelog v74 txt l.1067-1068]. Only trimesh parts inherit well; skinmesh (robes, cloaks) must exist per phenotype [W:38176344, W:53670835].
- Base game only ships full part sets for phenotypes 0 and 2; phenotypes 3/5/6/8 ship only the skeleton + `robe` + `cloak_` models (VERIFIED table below) — everything else falls back 3→0, 5→2, 6→0, 8→2. `pmh1.mdl` (skinny) exists, no parts.

Skeleton → animation supermodel (VERIFIED from all 73 base models):
| skeleton | supermodel |
|---|---|
| pmh0 pme0 pma0 | a_ba |
| pmd0 pmg0 pmo0 | a_da |
| pmh2 pme2 pma2 | a_ba2 |
| pmd2 pmg2 pmo2 | a_da2 |
| pfh0 pfe0 pfa0 pfg0 | a_fa |
| pfd0 pfo0 | a_dfa |
| pfh2 pfe2 pfa2 pfg2 | a_fa2 |
| pfd2 pfo2 | a_dfa2 |
| p?? 3 / 5 (mounted) | male: h_ba; female h/e/a: h_fa; female d/g/o: h_ba |
| p?? 6 / 8 (joust) | j_ba |
Chains: a_ba → a_ba_non_combat → a_ba_med_weap → a_ba_custom → a_ba_casts; a_fa/a_da/a_dfa → a_ba; a_*2 → a_ba2 → a_ba; h_fa → h_fa_non_combat → h_fa_med_weap → h_ba_* ; `*_coat*`/`*_coat_cus` variants are separate chains (purpose not documented; presumably robe/cloak "coat" animation sets) [W:49447555].

---------------------------------------------------------------------------------------------------

### D2. Part-based (P) creature composition

#### Naming
- Skeleton: `p` + gender + RACE + phenotype → `pmh0.mdl`, `pfd2.mdl`. Gender letter from gender.2da `GENDER` col (`M`,`F`; `B/O/N` exist but unusable for parts) [W:49447501]. Toolset format strings: `%s%s%s%03d`, head `%s%s%03d`+`_HEAD` (nwtoolset.exe strings).
- Part: `<skeleton>_<mdlname><NNN>.mdl`, NNN = 3-digit part number (≥1000 → 4 digits; keep resref ≤16) [W:38174935]. `000` = no model.
- Cloak: `<skeleton>_cloak_<NNN>.mdl` (extra underscore; toolset fmt `p%c%c%s_%s_%03d`).
- Race letters: A halfling, D dwarf, E elf, G gnome, H human/half-elf, O half-orc (appearance.2da RACE).
- NWScript doc of SetCreatureBodyPart: "p<m/f><race letter><phenotype>_<body part><model number>.mdl".

#### capart.2da (VERIFIED) — part slot → part name → attach node on skeleton
Row order == `ITEM_APPR_ARMOR_MODEL_*` == `CREATURE_PART_*` (0..17; head is CREATURE_PART_HEAD=20; robe=18).
| idx | MDLNAME (lowercase in files) | NODENAME | parts 2da | UTC field | UTI field |
|---|---|---|---|---|---|
| 0 | footr | rfoot_g | parts_foot | **ArmorPart_RFoot** (sic) | ArmorPart_RFoot |
| 1 | footl | lfoot_g | parts_foot | BodyPart_LFoot | ArmorPart_LFoot |
| 2 | shinr | rshin_g | parts_shin | BodyPart_RShin | ArmorPart_RShin |
| 3 | shinl | lshin_g | parts_shin | BodyPart_LShin | ArmorPart_LShin |
| 4 | legl | lthigh_g | parts_legs | BodyPart_LThigh | ArmorPart_LThigh |
| 5 | legr | rthigh_g | parts_legs | BodyPart_RThigh | ArmorPart_RThigh |
| 6 | pelvis | pelvis_g | parts_pelvis | BodyPart_Pelvis | ArmorPart_Pelvis |
| 7 | chest | torso_g | parts_chest | BodyPart_Torso | ArmorPart_Torso |
| 8 | belt | belt_g | parts_belt | BodyPart_Belt | ArmorPart_Belt |
| 9 | neck | neck_g | parts_neck | BodyPart_Neck | ArmorPart_Neck |
| 10 | forer | rforearm_g | parts_forearm | BodyPart_RFArm | ArmorPart_RFArm |
| 11 | forel | lforearm_g | parts_forearm | BodyPart_LFArm | ArmorPart_LFArm |
| 12 | bicepr | rbicep_g | parts_bicep | BodyPart_RBicep | ArmorPart_RBicep |
| 13 | bicepl | lbicep_g | parts_bicep | BodyPart_LBicep | ArmorPart_LBicep |
| 14 | shor | rshoulder_g | parts_shoulder | BodyPart_RShoul | ArmorPart_RShoul |
| 15 | shol | lshoulder_g | parts_shoulder | BodyPart_LShoul | ArmorPart_LShoul |
| 16 | handr | rhand_g | parts_hand | BodyPart_RHand | ArmorPart_RHand |
| 17 | handl | lhand_g | parts_hand | BodyPart_LHand | ArmorPart_LHand |
| 18 | robe | root (model base) | parts_robe | – | ArmorPart_Robe |
| – | head | head_g | none (all existing models) | Appearance_Head | (helmet = separate item) |
| – | cloak_ | root (model base) | cloakmodel | – | (cloak item ModelPart1) |
| – | wings / tail models | `wings` / `tail` dummies | wingmodel/tailmodel | Wings_New / Tail_New (DWORD; legacy BYTE Wings/Tail) | – |
- The UTC right foot really is `ArmorPart_RFoot` (BioWare quirk): VERIFIED in `nw_tiefling02.utc` and nwmain/nwtoolset string tables (list `BodyPart_Neck ... BodyPart_RShin ArmorPart_RFoot`); [W:38176577] lists it too.
- Other UTC appearance fields: `Appearance_Type` (WORD, appearance.2da row), `Appearance_Head` (BYTE), `Phenotype` (INT), `Gender` (BYTE), `Race` (BYTE racialtypes row, not used for models), `Color_Skin/Hair/Tattoo1/Tattoo2` (BYTE 0..175), `Tail`/`Wings` (legacy BYTE) and `Tail_New`/`Wings_New` (DWORD), `PortraitId` (WORD) / `Portrait` (resref), `VisualTransform` (EE) [W:38176577].
- `Equip_ItemList` elements: struct ID = equip-slot bit (VERIFIED: 2 armor, 16 right hand, 32 left hand, 131072 creature armor, same bits as baseitems EquipableSlots); in UTC blueprints the element has `EquippedRes` (UTI resref), in GIT/BIC instances the whole item struct is embedded.
- UTC 1.69 files store BodyPart_* as BYTE; EE raised part limit to 999 (87.35) — storage type for >255 unverified (open Q).

#### Body-part model numbers (creature, naked)
`CREATURE_MODEL_TYPE_NONE=0, SKIN=1, TATTOO=2, UNDEAD=255` (undead only for right-arm parts); SKIN not for shoulders/pelvis/head; TATTOO not for head/feet/hands (nwscript.nss SetCreatureBodyPart). e.g. `BodyPart_Torso=1` → `pmh0_chest001` (naked), `2` → `pmh0_chest002` (tattooed). Shoulders/belt 0 = nothing. Creature body parts can also be set to armor part numbers; they show when no armor covers them [W:38176577].

#### Skeleton node tree (pmh0.mdl, ASCII in data, VERIFIED)
```
pmh0 (base; "root")                 handconjure, headconjure (children of base)
└ rootdummy (0,0.02,1.207)
  ├ torso_g ─ wings, neck_g ─ head_g ─ head
  │         ├ Lbicep_g ─ Lshoulder_g, lforearm_g ─ lforearm (shield), lhand_g ─ lhand (weapon)
  │         ├ Rbicep_g ─ Rshoulder_g, rforearm_g ─ rhand_g ─ rhand (weapon)
  │         ├ Impact
  │         └ Cloak_g ─ CL1_fg, CR1_fg, cloak_shL, cloak_shR, CL1_g─CL2_g─CL3_g─CL4_g, CM1_g..CM4_g, CR1_g..CR4_g
  ├ pelvis_g ─ tail, lthigh_g─lshin_g─lfoot_g, rthigh_g─rshin_g─rfoot_g
  └ belt_g1 ─ belt_g, FB1_g01─FB2_g01─FB4_g01, TF1_g─TF1L_g/TF1R_g/TF2_g─TF3_g01   (belt/robe flap bones)
```
- Node names are case-insensitive (`Lbicep_g` vs capart `lbicep_g`). NOTE: game file has `belt_g` under `belt_g1` under `rootdummy`, not under pelvis_g as [W:38176272] says.
- Skeleton has no geometry; all dummies are animated by the supermodel chain. Mounted skeletons (pmh3) add horse nodes (13.8 KB file).
- `headconjure` z = floating name height; `impact` = VFX/projectile target; `head` = Imp_HeadCon_Node target [W:38176272].

#### Part model structure (VERIFIED, decompiled)
```
newmodel pmh0_chest001 / setsupermodel NULL / classification character
node dummy pmh0_chest001 (parent NULL)
node trimesh pmh0_chest001g   bitmap pmh0_chest001
```
- Geometry is authored in the attach node's local space: attach the part model root at the skeleton node (capart NODENAME), inherit its animated transform. E.g. head001 z∈[-0.06,0.18] above head_g; chest z∈[-0.05,0.50] above torso_g; handr z∈[-0.16,0.01].
- **Texture = the mesh `bitmap`, not the model name**: pmh0_handr001→`pmh0_handl003`, pmh0_shor001→`pmh0_shol001`, pfd0_chest001→`pfh0_chest001`, pma0_chest005→`pmh0_chest036`, pmo0_chest010→`pmh0_forer017`, pmh2_chest001→`pmh0_chest001`. Heads use race-specific (pmd0_head001→pmd0_head001). [W:53670455] also warns the PLT is whatever BITMAP says.
- Texture lookup order DDS > PLT > TGA [W:14618045]; a PLT found → colorize (D3). Sub-meshes whose bitmap names an MTR can override PLT use [W:14618045 "Overriding Palette Colors"].
- PLT fallback when the named PLT is missing (EE, [W:38176344]; example chest 400, pheno 123, female half-orc): `pfo123_chest400.plt` (or the fallback-phenotype name if the MDL fell back) → `pfo0_chest400.plt` (pheno 0, hardcoded) → `pfh0_chest400.plt` (human, same gender) → `pmh0_chest400.plt`. Also [W:14618045]: race-specific → `pmh0_…plt`. 85.32: "PLT textures can now be phenotype-specific"; 1.67: custom P races past row 481 use their RACE letter for PLT names, else human; 37-13: "Toolset properly loads non-human PLT textures".
- MDL fallback: requested phenotype missing → DefaultPhenoType (`pfo123_chest400` → `pfo2_chest400`). No documented gender/race fallback for MDLs (missing = part invisible) [W:38176344, W:49447501 "bodypart just disappears"].

#### Base-game part inventory (VERIFIED counts of `.mdl`)
| skel | head | neck | chest | pelvis | belt | shol/shor | bicep l/r | fore l/r | hand l/r | leg l/r | shin l/r | foot l/r | robe | cloak_ |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| pmh0 | 73 | 7 | 64 | 39 | 17 | 28/26 | 18/17 | 26/25 | 12/11 | 18/18 | 23/23 | 18/18 | 13 | 7 |
| pfh0 | 33 | 9 | 64 | 40 | 17 | 27/27 | 20/18 | 27/26 | 15/14 | 19/19 | 24/24 | 19/19 | 12 | 7 |
| pmd0 | 13 | 7 | 63 | 39 | 17 | 26/26 | 18/17 | 26/25 | 14/13 | 18/18 | 23/23 | 18/18 | 11 | 7 |
| pmh2 | 41 | 6 | 62 | 38 | 16 | 26/26 | 17/16 | 25/24 | 11/10 | 17/17 | 22/22 | 17/17 | 10 | 7 |
| p??3/5 | 0 | … all 0 … | | | | | | | | | | | 10 | 7 |
| p??6/8 | 0 | … all 0 … | | | | | | | | | | | 8 | 7 |
Also stray `pmh0_hair00N`, `pmh0_beard00N`, `pmh0_face00N` (not used by capart).

#### Composition algorithm (as documented; engine internals not public)
1. Load skeleton `p{g}{race}{pheno}` (fallback pheno via DefaultPhenoType if missing? — skeletons ship for all used phenos).
2. For each capart slot 0..17: number = armor `ArmorPart_<slot>` if armor (baseitem 16) equipped in chest slot, else creature `BodyPart_<slot>`; ([W:38176577] creature parts show "if no armor part is covering it"; exact per-slot rule when armor value is 0 = open Q). Load `{skel}_{mdlname}{NNN}` (pheno fallback), attach at NODENAME.
3. Robe: if armor `ArmorPart_Robe` ≠ 0: load `{skel}_robe{NNN}`, attach at model base; hide slots whose parts_robe `HIDE*`=1 (HIDEHEAD exists, unused by BioWare) [W:53669962]. Robe = skinmesh (`node skin MHRobe`) + copies of skeleton bones (`rootdummy, pelvis_g, l/rthigh_g, l/rshin_g, torso_g, l/rbicep_g, l/rforearm_g`, bitmap NULL), `setsupermodel pmh0_robe003 pmh0` (VERIFIED) → bones animated by the same animation as the body.
4. Head: `{skel}_head{Appearance_Head:03d}` at head_g, unless a helmet is equipped and not hidden → helmet replaces head (see Helmets) [W:14618045, W:38176272, W:26738928].
5. Cloak (cloak item in cloak slot, not hidden): cloakmodel row = UTI ModelPart1 → `{skel}_cloak_{MODEL:03d}` attached at root, texture overridden to `cloak_{TEXTURE:03d}.plt`, hide wings/tail/shoulders per HIDEWING/HIDETAIL/HIDESHOL/HIDESHOR [W:53671311]. Cloak = skinmesh `Cloak_Skin` (225 weights) + 17 render-0 bones, `setsupermodel pmh0_cloak_001 pmh0` (VERIFIED); new bones impossible without supermodel support [W:111607812].
6. Wings: wingmodel row `Wings_New` → MODEL at `wings` dummy (torso_g), scale WING_TAIL_SCALE; tail: tailmodel `Tail_New` → MODEL at `tail` dummy (pelvis_g). Wing/tail models contain their own `wings`/`tail` dummy under the base and their own full animation set with PC anim names (c_wingbat: 154 anims) — play the creature's current anim name on them. PLT used if a PLT exists (default skin color) else TGA (1.67). ENVMAP from wingmodel/tailmodel ENVMAP (1.68) [changelog v74 txt l.851-852, 1069].
7. Weapons (right/left hand slots): attach to `rhand` / `lhand` dummies; shields (baseitems WeaponWield 7) to `lforearm`; scale WEAPONSCALE. F/L creatures need those dummies too [W:38176272, W:38174941].
8. VFX: Imp_HeadCon_Node→`head`, Imp_Impact_Node→`impact`, Imp_Root_*→model root; ProgFX type 12 can target any named skeleton node (only nodes in the skeleton, not in attached parts) [W:38175069, W:38176565].
- Toolset strings show the creature editor attaches the head by `%s%s%03d`+`_HEAD` at `head_g`, the helmet by `%s_%03d` + `HELMET_SCALE_M/F` at `head_g`, and wings/tails with `WING_TAIL_SCALE`, `ENVMAP`, `wings`, and plays `pause1` (else `cpause1`/`creadyl`) for the preview pose (nwtoolset.exe .data strings around `TdlgCreatureEdit::SetWings`).

#### wingmodel.2da / tailmodel.2da (VERIFIED)
`LABEL, MODEL, ENVMAP`. Wings (91 rows): 1 Demon c_wingsdm default, 2 Angel c_wingsan, 3 Bat c_wingbat, 4 Dragon c_wingdra, 5 Butterfly c_wingbut, 6 Bird c_wingbird. Tails (5002 rows; ~490 used, 5000 karandas): 1 Lizard c_tailliz, 2 Bone c_tailbone, 3 Devil c_taildevil, 4.. dragon tails, many full creature models (horses c_horse*, scaling-trick monsters). Row 0 "(None)". Toolset sorts both alphabetically since 37-13.

#### Helmets
- Layered item (ModelType 1), `helm_{ModelPart1:03d}.mdl` (35 models) + `helm_NNN.plt`, icon `ihelm_NNN.plt` [VERIFIED]. Mesh `helm_001g`, bitmap `helm_001`, z∈[-0.16,0.16], no special nodes.
- Replaces the head object [W:14618045]; head_g "replaced by helmet models" [W:38176272]; scale by HELMET_SCALE_M/F; EE `SetHiddenWhenEquipped` lets helmets (and armour/cloak/hands) be hidden "so helmets don't have to replace the head model" [changelog v74 txt l.263-267; W:26738928].
- 1.66: "Fixed placement of helmets on Human/Half-Elf females".

#### Toolset item previews (from nwtoolset.exe strings; behaviour inferred)
- Armor preview builds a **human phenotype-0** body `p%ch0` + parts `P%cH0_%s%03d` (gender char only) and head `P%cH0`+`_HEAD`+`%03d`; shows `%03d (Part %03d)` (list position + model number, 37-13).
- Cloak preview uses `pmh0_%s_%03d` (fallback `pmh0_%s_001`) + texture `%s_%03d`.
- `nwttmplt::CreateImage` uses `PMH0_%s%03d` / `pmh0_%s_%03d` (template thumbnails).
- Env map: toolset maps ENVMAP `default` → `Chrome1` (`chrome1.tga/.dds` exist) — strings `ENVMAP default Chrome1`.
- Toolset part lists: only parts_* rows with non-blank ACBONUS, sorted ascending by ACBONUS float (fraction orders within AC); list index ≠ model number [W:38174875 parts_xxx, W:14618045 palette checker]. parts_chest ACBONUS floor = armor.2da row = AC [W:49446964]. Heads: every existing `{skel}_headNNN` model [W:53670835].

#### parts_*.2da (VERIFIED)
- All non-robe: `COSTMODIFIER, ACBONUS`, 201 rows (0..200); usable rows (non-blank ACBONUS): chest 62, pelvis 38, shoulder 25, forearm 24, shin 22, belt 17, legs 17, foot 17, bicep 16, hand 10, neck 7.
- parts_robe: `COSTMODIFIER, ACBONUS, HIDEFOOTR, HIDEFOOTL, HIDESHINR, HIDESHINL, HIDELEGR, HIDELEGL, HIDEPELVIS, HIDECHEST, HIDEBELT, HIDENECK, HIDEFORER, HIDEFOREL, HIDEBICEPR, HIDEBICEPL, HIDESHOR, HIDESHOL, HIDEHANDR, HIDEHANDL, HIDEHEAD` (39 rows; usable 0,3,4,5,6,20,21,30-33,38). Row 5: hides legs/pelvis/chest/belt/shoulders only (arms visible); 30-33 hide pelvis+belt only; 38 hides thighs..belt+shoulders.
- armorparts.2da (one row of per-slot numbers) and catype.2da are unused [W:38174875].

#### Visible cloaks — cloakmodel.2da (VERIFIED)
`LABEL, MODEL, TEXTURE, ICON, HIDEWING, HIDETAIL, HIDESHOL, HIDESHOR, ENVMAP` rows 0..16; e.g. `1 Plain 1 1 1 1 0 0 1 default`, `2 Arcane 1 10 2 …`, `4 FineGreatCl 2 13 4 1 0 1 1 default`, `11 Blackgd_Cyric 6 6 11 1 0 0 0 default`. 7 cloak models × 16 textures (`cloak_001..016.plt`, 512×512). Icon = `icloak_m_{ICON:03d}.plt` (baseitems GenderSpecific=1 → `_m_`). ENVMAP blank → metal PLT layers become transparent (tattered cloak trick) [W:53671311]. Limit raised to uint16 in 87.35 [W:38174875]. Cloaks added 1.68 ("Added visible cloaks", "Enabled PLT support for visible cloaks").

---------------------------------------------------------------------------------------------------

### D3. PLT textures

#### File layout (VERIFIED on 7 files)
| off | size | value |
|---|---|---|
| 0 | 4 | `PLT ` |
| 4 | 4 | `V1  ` |
| 8 | 4 | uint32, 10 in all body/helm/cloak/armor-icon PLTs, **6 in icloak_m_001.plt** — "unused" per [W:14618045]; ignore |
| 12 | 4 | uint32 0 (unused) |
| 16 | 4 | uint32 width |
| 20 | 4 | uint32 height |
| 24 | w·h·2 | per pixel: byte0 = grey value 0..255, byte1 = layer id 0..9 |
File size = 24 + 2·w·h exactly. Sizes seen: body 256², helm 128², cloak 512², icons 64×128 / 64×64.
**Row order is bottom-up** like TGA (VERIFIED: decoded pmh0_head001 face is upright only after flipping rows; icloak_m_001 rows 0–31 = blank bottom strip of 2×3 icons).

#### Layers → colour source → palette (VERIFIED palette files; mapping [W:14618045])
| id | layer | creature/item colour | palette |
|---|---|---|---|
| 0 | Skin | UTC Color_Skin | pal_skin01.tga |
| 1 | Hair | UTC Color_Hair | pal_hair01.tga |
| 2 | Metal 1 | UTI Metal1Color | pal_armor01.tga |
| 3 | Metal 2 | UTI Metal2Color | pal_armor02.tga |
| 4 | Cloth 1 | UTI Cloth1Color | pal_cloth01.tga |
| 5 | Cloth 2 | UTI Cloth2Color | pal_cloth01.tga |
| 6 | Leather 1 | UTI Leather1Color | pal_leath01.tga |
| 7 | Leather 2 | UTI Leather2Color | pal_leath01.tga |
| 8 | Tattoo 1 | UTC Color_Tattoo1 | pal_tattoo01.tga |
| 9 | Tattoo 2 | UTC Color_Tattoo2 | pal_tattoo01.tga |
- pal_cloth01 == pal_leath01 == pal_tattoo01 byte-identical in RGB (VERIFIED). NB wiki spells `pal_armor_01`; real names `pal_armor01.tga`/`pal_armor02.tga`.
- Palettes: TGA type 2, 256×176, 32 bpp BGRA, bottom-left origin (desc 0x08) (VERIFIED all 7). **Colour index k = k-th row from the top of the displayed image = stored row 175−k** (VERIFIED by matching the 16×11 swatch grids of toolset `mvpal_*.bmp` (256×176, 16-px swatches, index = row*16+col): mean error 64 vs 164 for the flipped hypothesis). X = PLT grey value 0..255.
- Colour = `palette[layer][row = colorIndex][x = grey]` incl. alpha. [W:14618045 "Palette Function"].
- Colour indices 0..175 (176); 1.67 expanded the choices. Per-part value 255 = "no override".
- Alpha in palettes (VERIFIED): column x=255 has alpha 0 in all rows of skin/cloth/leath/tattoo/armor01 (hair: 214 in most rows) → grey 255 = transparent/fully env-mapped; rows 56–59 (chrome swatches) alpha ramps 219→49 across x; row 61 alpha 0 everywhere ("transparent" swatch); pal_armor01 has alpha < 255 in almost every row (metal = partly reflective); pal_armor02 alpha all 255. Icons use grey 255 on layer 0 for their background (VERIFIED). [W:60985308] advises avoiding 0 and 255.
- How alpha is used (game shader `inc_standard.shd`, VERIFIED): with an env map bound, `envLevel = 1 − tex.a; rgb = mix(tex.rgb, env.rgb, envLevel)` (alpha not transparency); without env map alpha is ordinary transparency. So appearance/cloakmodel/wingmodel/tailmodel `ENVMAP` decides "shiny vs see-through" for PLT alpha [W:38174941, W:53671311].
- GPU path (game `fs_pltgen.shd`, VERIFIED): PLT uploaded as texture with r=grey, g=layer; `uniform float PLTscheme[15]` maps layer → v-coordinate in a palette texture (unit1); output = `texture2D(pal, vec2(grey, PLTscheme[layer]))`. (87.35 option `graphics.experimental.generate-plt-with-shaders`; CPU path otherwise; 88.36 renders to an offscreen FBO.)
- Custom `pal_XXXyy.tga` load from haks/NWSync per module since 37-13.
- Selector images: toolset `mvpal_skin/hair/cloth/leather/tattoo/armor01.bmp` (256×176 = 16×11 swatches), older `mvpal_armor.bmp`/`mvpal_armor02.bmp` (128×128); in-game `gui_pal_skin.tga`, `gui_pal_hair01.tga`, `gui_pal_tattoo.tga` (256×256). No gui palette for armor [W:14618045].
- Decoded examples (PNG, VERIFIED): `$SP/models/png/pmh0_head001_decoded_on_magenta.png`, `ihelm_001_…`, `cloak_001_…`.

#### Which colours apply where
- Creature parts: skin/hair/tattoo from UTC; metal/cloth/leather from the **equipped armor's** colours — also on uncovered naked parts and on the head (eyepatches etc.) (Lexicon GetItemAppearance remark). Without armor: open Q (probably index 0).
- Helmet PLT: helmet UTI colours (6); skin/hair layers presumably from the wearer.
- Cloak PLT: cloak UTI colours (e.g. x2_it_mcloak001: cloth1/2=167, leather/metal=170).
- Wings/tails PLT: creature skin colour by default (1.67).
- VFX models with PLT inherit the target creature's colours (1.81.8193.15; progfx type-12 accessories) [W:14618045, W:38176565].

#### EE per-part colours (armor)
- GFF (UTI): `APart_<part>_Col_<channel>` BYTE — format string `APart_%d_Col_%d` next to `ModelPart1..3` in nwmain's item loader (nwsitem.cpp strings). part = ITEM_APPR_ARMOR_MODEL_* (0 RFOOT … 17 LHAND, 18 ROBE); channel = ITEM_APPR_ARMOR_COLOR_*: **0 LEATHER1, 1 LEATHER2, 2 CLOTH1, 3 CLOTH2, 4 METAL1, 5 METAL2** (note: differs from PLT layer order). Absent / 255 = use the item-wide colour. No base-game resource uses it (binary grep for "APart_" found nothing).
- Script index = `6 + part*6 + channel` (e.g. torso cloth1 = 50) (nwscript.nss / Lexicon GetItemAppearance; v74 notes l.282-288). Added 1.74.8150; `GetItemAppearance` per-part since 87.35.
- Verified in the engine (`engine_armor_colors.rs`): the name and BYTE type as above, read from a blueprint and written for a script-set colour; no field for a part without one.

---------------------------------------------------------------------------------------------------

### D4. Items

#### baseitems.2da (VERIFIED header, 67 cols)
Model/icon-relevant: `Name, label, InvSlotWidth, InvSlotHeight, EquipableSlots, CanRotateIcon, ModelType, ItemClass,
GenderSpecific, Part1EnvMap, Part2EnvMap, Part3EnvMap, DefaultModel, DefaultIcon, Container, WeaponWield, WeaponType,
WeaponSize, RangedWeapon, PrefAttackDist, MinRange, MaxRange, …, Category, …, RotateOnGround, …, AmmunitionType, …`.
- ModelType: 0 simple (1 model, 1 icon), 1 layered (PLT model + PLT icon), 2 composite (3 parts b/m/t), 3 armor (PLT, 18(+robe) parts) [W:38174935]. VERIFIED counts: 2 → 51 rows, 0 → 42, 1 → helmet & cloak, 3 → armor only.
- ItemClass: resref stem (case-insensitive; files are lowercase). ≤5 chars recommended (also prefixes iprp_visualfx weapon VFX, e.g. `wswls_fxfire.mdl`, `wswls_mb`, `wswls_blur`) [W:38174935].
- GenderSpecific: 1 for armor, cloak, arrow — inserts gender into names (`icloak_m_001`, `ipm_`/`ipf_`); ignored for composite.
- PartNEnvMap: **non-functional** in game (env map `default` hardcoded) [W:38174935].
- DefaultModel: model used when the specific part model is missing (typically ground model `it_bag`); armor/cloak ground models hardcoded (`gi_armor01..04`, cloak `gi_cloak01`; armor row DefaultModel `gifp`) [W:26738909]. DefaultIcon: icon if item icon missing.
- MinRange/MaxRange: toolset-only model number range; composite: numbers = model·10 + colour (011–014, 021–…); first two digits = "Model", last = "Color" (1 steel, 2 copper/bronze, 3 gold, 4 adamantine/cold iron); x0 unused; 000 reserved/unused [W:60985168, W:38174935]. Toolset availability determined by the **icon** file (TGA) existing, for composite by the bottom-part icon [W:38174935, W:60985278].
- RotateOnGround: 0 none, 1 rotate 90° about +Y, 2 about +X (ground model orientation) [W:38174935]. CanRotateIcon: description lost in mirror (inventory icon rotation?) — open Q.
- WeaponWield: 1 not wieldable, (blank) standard, 4 two-handed/pole, 5 bow, 6 crossbow, 7 shield (→ lforearm), 8 double, 9 creature, 10 sling, 11 thrown.

Representative rows (VERIFIED):
| ID | label | W×H | ModelType | ItemClass | GS | DefaultModel | DefaultIcon | Min–Max | RotateOnGround |
|---|---|---|---|---|---|---|---|---|---|
| 0 | shortsword | 1×2 | 2 | WSwSs | 0 | it_bag | iwswss | 10–100 | 1 |
| 1 | longsword | 1×4 | 2 | WSwLs | 0 | it_bag | iwswls | 10–100 | 1 |
| 14 | smallshield | 2×2 | 0 | AShSw | 0 | it_bag | iashsw | 10–100 | 1 |
| 16 | armor | 2×3 | 3 | AArCl | 1 | gifp | iit_chest | 0–100 | 0 |
| 17 | helmet | 2×2 | 1 | helm | 0 | it_bag | ihelm | 0–50 | 0 |
| 19 | amulet | 1×2 | 0 | it_neck | 0 | it_bag | iit_neck | 0–50 | 0 |
| 26 | boots | 2×2 | **2** | it_boots | 0 | it_bag | iit_boots | 0–50 | 0 |
| 49 | potions | 1×2 | **2** | it_potion | 0 | it_potion_000 | iit_potion | 10–100 | 2 |
| 52 | ring | 1×1 | 0 | it_ring | 0 | it_bag | iit_ring | 0–50 | 0 |
| 57 | towershield | 2×4 | 0 | AShTo | 0 | it_bag | iashto | 10–100 | 1 |
| 80 | cloak | 2×3 | 1 | cloak | 1 | it_bag | icloak_m_001 | 0–100 | 0 |

#### Naming per ModelType (VERIFIED on disk)
| type | model | icon | UTI fields |
|---|---|---|---|
| 0 simple | `<class>_<NNN>.mdl` (e.g. `ashsw_011`, `it_midmisc_151`) — many classes have **no** models (rings, amulets, belts, gems, books…) → DefaultModel | `i<class>_<NNN>.tga/.dds` (e.g. `iashsw_011`, `iit_ring_004`) | `ModelPart1` |
| 1 layered | helmet `helm_<NNN>.mdl` + `helm_NNN.plt`; cloak: see cloakmodel (worn model per race), no item-class model | `ihelm_<NNN>.plt`; `icloak_m_<ICON>.plt` | `ModelPart1` + 6 colours |
| 2 composite | `<class>_b_<NNN>`, `_m_`, `_t_` (bottom/middle/top), e.g. `wswls_b_011/_m_011/_t_011` (wiki's `swwss_t_001` example is wrong) | `i<class>_b_<NNN>.tga` + `_m_` + `_t_` overlaid (32×128 each for longsword) | `ModelPart1`=b, `ModelPart2`=m, `ModelPart3`=t |
| 3 armor | no item model: worn = creature parts; ground = gi_armor0N | `ip<m|f>_<part><NNN>.plt` for chest, pelvis, belt, shol, shor, robe only (364 files, all 64×128 full-frame overlays) | `ArmorPart_*` (19 incl. Robe) + 6 colours (+ per-part) |
- UTI examples (VERIFIED): nw_wswls001 ModelPart1=61, 2=11, 3=11 → `wswls_b_061, wswls_m_011, wswls_t_011` (all exist); nw_arhe001 helmet ModelPart1=1, Cloth1 19, Cloth2 16, Leather1 21, Leather2 7, Metal1 2, Metal2 2; nw_aarcl001 ArmorPart_Torso 16, LShoul 15, … colours C1 1 C2 7 L1 1 L2 7 M1 0 M2 8; nw_ashsw001 ModelPart1=11; nw_it_mpotion001 ModelPart1/2/3 = 21/23/32 (icon-only composite).
- Weapon parts (VERIFIED): each part MDL = base dummy `WSwLs_b_011` + trimesh `g_WSwLs_b_011`, bitmap `W_metal_tex` (shared TGA atlas, not PLT); all three share one origin (grip at origin, blade along +Y: b y∈[-0.03,0.23], m y∈[0.08,0.17], t y∈[-0.13,0.67]) → attach all three at `rhand`/`lhand`. ModelType-2 items cannot use PLT [W:14618045].
- Shield (VERIFIED `ashsw_011`): trimeshes + `imp_shield_1/2` dummies (arrow impact points) [W:38176272]; attaches at `lforearm`.
- Icons: TGA/DDS (PLT for layered/armor); sizes by InvSlotWidth×Height ×32 px (2×3 → 64×128, bottom 32 px unused) [W:38175245]. Toolset lists need a **TGA** icon to exist; in-game DDS preferred [W:60985278]. 37-13: toolset scales down oversized icons; "Fixed Robe Icons > 255".
- Armor icon composition: overlay the part icon PLTs in one 64×128 frame, colorized with the item colours; robe hides parts per parts_robe; robe icon shows only robe/pelvis/belt/torso/shoulders (1.61, 1.67) [changelog v74 txt l.1066, 1703]. Draw order not documented (open Q).
- Layered-item quirks (toolset shows only the PLT icon; MTR with PLT must omit texture0) [W:60985278, W:60985308].

---------------------------------------------------------------------------------------------------

### D5. Placeables and doors

#### placeables.2da (VERIFIED header)
`Label, StrRef, ModelName, LightColor, LightOffsetX, LightOffsetY, LightOffsetZ, SoundAppType, ShadowSize, BodyBag, LowGore, Reflection, Static` [W:38175210].
- ModelName: MDL resref. LightColor: lightcolor.2da row (`RED,GREEN,BLUE,LABEL,TOOLSETRED,TOOLSETGREEN,TOOLSETBLUE`, floats; toolset uses TOOLSET*) → spawn `fx_placeable01.mdl` at placeable origin + LightOffset (root node offset).
- ShadowSize: **** none, 0/1/2 → `fx_shadow_s/m/l.mdl` blob shadow (only "fast" shadow mode).
- Reflection: **** alpha = transparency; `default` = area env map; else env-map resref (`PLC_G_ENV`, `dlag__ref01`) (1.61). Static placeables ignore it (treated like tiles).
- Static: 0 greys out the Static checkbox (skinmesh placeables) (1.67). LowGore: alternate model. BodyBag: 1 for bodybags (toolset-ish).
- Placeholder rows: 16500 rows, Label `****` with ModelName `USER` (13995), `OS_RESERVED` (1023), `BD_RESERVED` (158) — skip.
- Examples: `0 Armoire PLC_A01 …SoundApp 13 Shadow 1`; `57 Brazier PLC_I05 LightColor 6 offset (0.004,0.011,0.875)`.
- UTP: `Appearance` DWORD (row), `PortraitId` WORD, `Static` BYTE, `BodyBag`, `AnimationState` (VERIFIED on tm_pl_grndwtg1x1.utp).
- Model special nodes: `<name[4:]>_ground/_hand/_head_hit/_impact`; walkmesh `.pwk` use dummies `????pwk_use01/02` [W:38176272].

#### Doors (VERIFIED headers)
- genericdoors.2da: `Label, StrRef, ModelName, BlockSight, VisibleModel, SoundAppType, Name` (Name strref required for toolset listing) — 30 used rows, e.g. `0 Wood_strong T_DOOR01`, `20 wc_generic02 dwc_gen_02` [W:38175673].
- doortypes.2da: `Label, Model, TileSet, TemplateResRef, StringRefGame, BlockSight, VisibleModel, SoundAppType` — 272 used rows, e.g. `1 Wall1Door TTR_UDoor_01 TTR01 nw_door_ttr_01 63491 1 1 1`; row 0 "Generic". Toolset also checks TileSet0/1/3 columns if present (1.69 notes) [changelog v74 txt]. VisibleModel 0 = invisible door.
- UTD: `Appearance` DWORD (doortypes row; 0 = generic), `GenericType` BYTE (genericdoors row) and EE `GenericType_New` (DWORD; engine string) — VERIFIED fields on x3_door_wood004.utd (Appearance 0, GenericType 20). Presumed rule: Appearance≠0 → doortypes.Model else genericdoors[GenericType_New|GenericType].ModelName (BioWare door spec; not in mirror — verify).
- Placement on tiles: SET `[TILE<n>DOOR<m>]` `Type` (doortypes row; 0 = generic allowed), `X,Y,Z` offset relative to the tile centre (door 0,0 = tile 5,5), `Orientation` degrees; tile `Doors=` count [W:38175683]. Door models: dummies `<model>grnd/hhit/impc`; `.dwk` walkmesh with `??_DWK_dp_closed01/02, open1_01/02, open2_01/02` [W:38176272]. Doors fade by alpha when viewed from the wrong side [W:38174806].

---------------------------------------------------------------------------------------------------

### D6. cachedmodels.2da, visualeffects.2da, portraits.2da

- cachedmodels.2da (88.36): single column `Model`; 30 rows of gib/blood chunk models (`grn_m_bone … vwp_l_chunk_red`) kept resident; clientside [W:145457156]. Toolset may ignore.
- visualeffects.2da (VERIFIED header): `Label, Type_FD, OrientWithGround, Imp_HeadCon_Node, Imp_Impact_Node, Imp_Root_S_Node, Imp_Root_M_Node, Imp_Root_L_Node, Imp_Root_H_Node, ProgFX_Impact, SoundImpact, ProgFX_Duration, SoundDuration, ProgFX_Cessation, SoundCessastion (sic), Ces_HeadCon_Node, Ces_Impact_Node, Ces_Root_S_Node … Ces_Root_H_Node, ShakeType, ShakeDelay, ShakeDuration, LowViolence, LowQuality, OrientWithObject` (10100 rows) [W:38175069]. HeadCon → `head` node (placeable `_head_hit`, door `hhit`), Impact → `impact`, Root_{S,M,L,H} by appearance SIZECATEGORY → root. OrientWithObject=1 for head-aligned accessories/eyes (e.g. 324 VFX_EYES_BLUE_HUMAN_MALE Imp_HeadCon=vfx_blueeyes) [W:38176583]. progfx type 12: Param1 node name, Param2 model [W:38176565].
- portraits.2da (VERIFIED): `BaseResRef, Sex, Race, InanimateType, Plot, LowGore`; files `po_<BaseResRef><h|l|m|s|t>` (e.g. `po_hu_m_01_m.tga`); toolset needs `_m` TGA; placeables `_m` only [W:38174997]. 87.35: DDS portraits + nearest-size fallback.

---------------------------------------------------------------------------------------------------

## E. Textures (TGA, DDS, KTX, PLT pointer, TXI, MTR, shaders)

Sources: nwn.wiki "Textures" (https://nwn.wiki/spaces/NWN1/pages/38174958/Textures), "TGA" (https://nwn.wiki/spaces/NWN1/pages/53672779/TGA), "DDS" (https://nwn.wiki/spaces/NWN1/pages/3473496/DDS), "KTX" (https://nwn.wiki/spaces/NWN1/pages/133988392/KTX), "TXI" (https://nwn.wiki/spaces/NWN1/pages/38174929/TXI), "MTR" (https://nwn.wiki/spaces/NWN1/pages/12027232/MTR), "Standard material inputs" (https://nwn.wiki/spaces/NWN1/pages/38175898/Standard+material+inputs), "Content Load Order" (https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order), "Shader Engine Support" (https://nwn.wiki/spaces/NWN1/pages/14614573/Shader+Engine+Support), "Working with Transparency" (https://nwn.wiki/spaces/NWN1/pages/68387028/Working+with+Transparency), "Dealing with transparency" (https://nwn.wiki/spaces/NWN1/pages/129236997/Dealing+with+transparency), "Environment Maps and Cubemaps" (https://nwn.wiki/spaces/NWN1/pages/38174773/Environment+Maps+and+Cubemaps), "Enhanced Lighting Engine and PBR" (https://nwn.wiki/spaces/NWN1/pages/38175899/Enhanced+Lighting+Engine+and+PBR). Ground truth: header scan of every texture in the 89.8193.37-17 base keys (nwn.py), and the game's own GLSL shaders (base_shaders.bif / ovr.bif, 92 `.shd` files, extracted to `$SP/models/shd/`).

### E.1 Inventory (base game keys)
| type | count | notes |
|---|---|---|
| tga | 28,218 | |
| dds | 6,737 | 6,736 BioWare-header DDS + **1** standard `DDS ` (solid_noise.dds, FourCC `ATI1` = BC4) |
| plt | 1,565 | all magic `PLT V1  ` |
| txi | 734 | 321 pair with tga only, 290 with both dds+tga, 54 dds only, 69 no image (procedural/font) |
| mtr | 7 | gui_cg_color, oldf_head, oldm_head, tti01_ice{clear,clear2d,floor,trim} |
| shd | 92 | GLSL source of all stock shaders (readable reference for the renderer) |
| ktx | 0 | desktop ships none (mobile only) |
| bmp | 26 | incl. toolset palettes `mvpal_{skin,hair,tattoo,cloth,leather,armor01}.bmp` (xp2patch.bif) and `mvpal_armor.bmp`, `mvpal_armor02.bmp` (editor.bif) |
- `data/txpk/*.erf` (Textures_Tpa/Tpc, Tiles_Tpa/b/c, xp1/xp2_tex_tp*) are 160-byte empty ERFs: texture packs are dead; 89.8193.37 "stripped all remaining nonfunctional texturepack code". Just ignore.

### E.2 Lookup / priority
- For a model texture reference `name` (bitmap/texture0): **`name.mtr` takes precedence** over image files (a mesh's bitmap may name an MTR). Then image: the Textures page lists load priority **KTX (1) > DDS (2) > PLT (3) > TGA (4)**; the TGA page words it as "DDS over PLT over TGA".
- Exception (Content Load Order): DDS/TGA are interleaved per location: DDS override > TGA override > DDS nwsync > TGA nwsync > DDS hak > TGA hak > DDS bif > TGA bif. I.e. a TGA in a hak beats a DDS in the base BIFs. (KTX position unknown.)
- General resman order (high->low): user portraits, install portraits, vault, development, (RIM), NWSync manifest, user hak (first listed hak wins), install hak, currentgame, savegame, module, userpatch, user override, (install override), ambient/music, install patch bifs, texture pack (dead), base key tables. For a toolset: module-> haks (in module.ifo order) -> override -> base keys.
- A `name.txi` next to the texture is always loaded alongside (MTR beats TXI where both define things; TXI blending modes prevent MTR fancy maps, per wiki).
- Names: resrefs <= 16 chars (texture, MTR, TXI). Stored in binary MDL as char[64] but only 16 are resolvable.
- Toolset-specific: item icons, palette files (pal_*.tga) and loadscreens are loaded by the Aurora toolset **as TGA**; DDS icons were cropped/unscaled before 8193.37 (now scaled).
- "Texture not found" -> engine renders white.

### E.3 TGA (measured)
| (image type, bpp, origin, alpha bits) | count |
|---|---|
| 2 uncompressed truecolour, 24, bottom-left, 0 | 17,504 |
| 2, 32, bottom-left, 8 | 8,184 |
| 10 RLE truecolour, 24, bottom-left | 1,079 |
| 2, 32, bottom-left, **0 alpha bits declared** | 789 (treat 4th byte as alpha anyway? verify) |
| 10 RLE, 32, bottom-left, 8 | 641 |
| **3 greyscale, 8 bpp** | 24 (c_allip, c_spectre, c_wraith, several pmh0_* body textures, shinywater) |
| top-left origin (desc bit 5) | 10 files |
- No colour-mapped TGAs; 5 files have a non-zero image-ID length (skip `idLength` bytes after the 18-byte header).
- Non-power-of-two TGAs exist (44): NUI/GUI images, minimaps (mi_tss13_* 153x153, 204x204), **palettes pal_*.tga 256x176**, ssky0005 900x900, a few odd textures (pmh0_neck006 12x12, tde01_statue1 88x108). Wiki says textures "must be powers of 2" - engine evidently copes (GL NPOT); support NPOT.
- Wiki: greyscale TGAs don't work in some PBR slots (convert to RGB). TGAs get no stored mipmaps (engine generates; fixed in 1.80.8193.6 for uncompressed).
- Orientation: NWN textures are stored bottom-up (TGA default); OpenGL UV (0,0) = first row in memory => bottom-left TGA rows upload directly. ~~Top-left-origin TGAs must be flipped.~~ **Measured in the client (2026-10-04, `client_minimap.rs`): the game ignores the origin bit** and takes the rows as stored, so a top-left-origin TGA whose rows are stored top first (what Krita writes) shows upside down in the game; Moonglow does the same.

### E.4 DDS
**BioWare DDS (6,736 of 6,737 shipped)** - no magic; 20-byte header then compressed data:
| off | type | field |
|---|---|---|
| 0 | u32 | width |
| 4 | u32 | height |
| 8 | u32 | channels: **3 = DXT1 (BC1, 8 B/block)**, **4 = DXT5 (BC3, 16 B/block)** |
| 12 | u32 | size of top mip level in bytes (= ceil(w/4)*ceil(h/4)*blocksize; verified for all files) |
| 16 | f32 | "alpha mean"/premultiplier: 1.0 for all ch=3 files; ch=4 mostly other values (0..1); 11 files 0.0 |
| 20 | ... | mip chain, largest first, down to 1x1 (**all 6,736 files contain the full chain**; sizes sum exactly to file size -20 in all but 2) |
- Measured: 4,429 square, 2,305 **non-square** (e.g. 512x256) - wiki's "must be square" is wrong; powers of two yes. Max dims 4 .. 2048 (5 at 2048, 83 at 1024, 2017 at 512).
- Wiki calls the 4-channel format "DXT3" but its block description (two 8-bit alpha endpoints + 3-bit indices) is DXT5/BC3; decode as **DXT5**. (xoreos also treats BioWare 4-channel DDS as DXT5.) The DXT3 section of the wiki describes a *standard* DDS sample.
- Row order (**measured**): for 5,436 base textures that exist as both .dds and .tga, compared the mean colour of the DDS's first block row with the TGA's first/last stored rows (bottom-left TGAs, same size): 381 match the TGA's **first stored row (= image bottom)**, 55 the last, 2,318 ambiguous. => BioWare DDS data is stored bottom-row-first, exactly like a bottom-left TGA. **Renderer rule: upload DDS mip data in stored order (never flip); upload TGA rows so that the bottom image row is first (flip only top-left-origin TGAs); v=0 = first stored row.** Consequence: a standard DDS authored top-down appears upside-down in NWN, which is why the wiki/NWN Crunch tell authors to y-flip DDS (crunch `-yflip`).
**Standard DDS** (`DDS ` magic, 124-byte header, since v78 "normal DDS without mangling the header"): supported per wiki BC1 (DXT1), BC2/BC3 (DXT3/DXT5), BC4 (ATI1), BC5 (ATI2/"3DC", normal maps as 2-channel since 1.82.8193.20). BC7/DX10 header: not documented -> assume unsupported. NWN Crunch recommendations: diffuse DXT5(A), normal DXN(BC5), spec/rough/height DXT5A (BC4), illumination DXT1.
- Normal maps: only R,G used (Z reconstructed), green = up (OpenGL convention).

### E.5 KTX
- KTX1 container, ETC2 (GL_COMPRESSED_RGB8_ETC2 / RGBA8_ETC2_EAC) only; RGB/RGBA only (R/RG loaded wrongly); no sRGB; KTX2 unsupported. Used by mobile; works "sort of" on desktop; default alpha mean 0.5. Load priority above DDS. Low priority for Moonglow (read-only support optional).

### E.6 PLT (pointer - details in part D)
- 24-byte header (`PLT V1  `, u32 unknown (10, or 6 in icloak_m_001), u32 0, u32 width, u32 height) then w*h*2 bytes: grey intensity, layer id 0-9; rows bottom-up like TGA (exact layout, layers, palettes: D3). Rendering in EE (stock shader `fs_pltgen.shd`/`vs_pltgen.shd`, used when the config `graphics.experimental.generate-plt-with-shaders` is on (added 1.87.8193.35; whether it is on by default now is unverified; otherwise PLT is composed on the CPU with the same math): PLT uploaded as an RG texture (R = intensity, G = layer); `PLTscheme[15]` float uniform maps layer index -> row (v coordinate) in a palette atlas texture (texUnit1); final colour = `texture2D(paletteAtlas, vec2(R, PLTscheme[layer]))`. So colour = palette[row = chosen colour index for that layer][column = intensity]. Result is baked to a normal RGBA texture (mipmapped) then used as texture0.
```glsl
vec4 c = texture2D(texUnit0, uv);            // r = grey, g = layer/255
c.g = PLTscheme[int(c.g*255.0+0.5)];          // row in palette atlas for that layer
gl_FragColor = texture2D(texUnit1, c.rg);     // palette lookup
```
- Palette textures `pal_{skin01,hair01,armor01,armor02,cloth01,leath01,tattoo01}.tga` are 256x176 (256 intensity columns x 176 colour rows). Custom `pal_XXXyy.tga` loaded from haks/NWSync since 89.8193.37. "PLT textures can now be phenotype-specific" (1.85.8193.31). Visual effects can use PLT (colours inherited from the target creature) since 1.80.8193.14.
- MTR on PLT meshes: **omit `texture0`** in the MTR (use `bitmap <plt>` + `materialname m_<plt>` in the MDL, MTR holds only texture1..5).

### E.7 TXI (texture info, same resref as texture)
Parsing: `keyword value(s)` per line; some keywords take a count then N following lines (`channelscale 4` + 4 lines, `channeltranslate 4` + 4 lines, font `upperleftcoords N`/`lowerrightcoords N` + N lines). Comments with `#`. Case-insensitive.
| keyword | values | renderer relevance | shipped count |
|---|---|---|---|
| mipmap | 0/1 (default 1) | disable mips (GUI, fxpa_*) | 478 (all 0) |
| filter | 0 nearest / 1 linear (default linear) | | 471 |
| downsamplemax / downsamplemin | 0-15 | texture quality downsampling; ignore | 571/109 |
| clamp | 0 wrap, 1 clamp U(X), 2 clamp V(Y), 3 both | sampler wrap | 15 |
| alphamean | float | hints alpha usage; since 1.83.8193.26 needed (<1.0) when custom shaders use alpha (early-Z) | 159 |
| blending | default / additive / punchthrough (/ lighten?, normal) | additive = GL ONE,ONE-style add (black transparent); punchthrough = alpha test ~0.5 cut-out; default = alpha blend | 42 (default 23, additive 13, normal 4, punchthrough 2) |
| decal | 0/1 | disables lighting (since 8193.21: truly unlit), drawn over coplanar geometry | 21 |
| envmaptexture | texture name / `default` | env map for static geometry; overrides appearance.2da ENVMAP; `default` = area's | 50 |
| bumpmaptexture shinywater + bumpyshinytexture <env> | | legacy "shiny water" (water tiles) | 22/33 |
| proceduretype | cycle / arturo / water (dead) / wave / perlin / life | texture animation | cycle 7, arturo 38, water 4 |
| numx numy fps | | `cycle`: sprite-sheet flipbook numx cols x numy rows at fps (e.g. fxpa_flame02: 4x4 @32 fps, additive) | 7 |
| distort, distortangle, distortionamplitude, speed, arturowidth, arturoheight, channelscale, channeltranslate, defaultwidth, defaultheight, waterwidth, waterheight | | arturo/water procedural distortion params (distort max 256x256 source) | ~40 |
| cube 1 + filerange 6 | | **cube map stored as 6 files `name0..name5`** (e.g. tts01__env.txi -> tts01__env0..5.tga) | 7 |
| isbumpmap, bumpmapscaling, isdiffusebumpmap, isspecularbumpmap, specularcolor, bumpintensity | | deprecated, ignored in EE | |
| temporary, candownsample, downsamplefactor, gamma, useglobalalpha, isenvironmentmapped, pltreplacement, bumpreplacementtexture | | misc; bumpreplacementtexture loads another texture instead | |
| font keys (numchars, fontheight, baselineheight, texturewidth, spacingR/B, caretindent, upperleftcoords, lowerrightcoords, isdoublebyte, dbmapping, cols, rows, codepage, ttfContrast, ttfOutlineOpacity, ttfOutlineBrightness) | | fonts only | |

### E.8 MTR (EE material, 16-char name; referenced via `materialname` or via bitmap/texture0 = mtr name)
Syntax: one directive per line, `//` comments, case-insensitive, underscores allowed (since 8193.20).
| directive | meaning |
|---|---|
| texture0..texture10 `<name>`/`null` | bind to sampler texUnitN (+ uniform textureNBound). Default shader semantics: **0 diffuse (rgb, optional alpha), 1 normal (RG, OpenGL green-up), 2 specular (R; metallic when ->1; B = self-illum if MATERIAL_READ_SELF_ILLUMINATION_FROM_SPECULAR_MAP), 3 roughness (R; or spec G with MATERIAL_READ_ROUGHNESS_FROM_SPECULAR_MAP), 4 height (white = base; parallax + occlusion), 5 self-illumination (RGB)**; 6-10 custom shaders only. MTR textures override the MDL's texture0-3. (1.74.8159 notes said up to texture14; slots 11-15 now reserved: 11 FB depth, 12 FB colour, 13 env cube, 14 env flat, 15 noise.) |
| bitmap `<name>` | alias of texture0 (texture0 wins) |
| renderhint NormalAndSpecMapped / NormalTangents / none | normal-mapped shader + tangent generation (mesh-level renderhint overrides MTR's) |
| customshadervs / customshaderfs / customshadergs `<shd>` | custom GLSL (stock names: vslit/fslit, *_nm normal mapped, *_sm env mapped ("shiny/metal"), vslit_sk* skinned, vslitc*, fst/fstc unlit, fsgrass...) |
| parameter float `<name>` v [v v v] / parameter int `<name>` v | uniforms; stock ones: Specularity, Roughness, Metallicness, DisplacementOffset, DisplacementMultiplier, CustomSpecularColor |
| transparency 1 | force transparent pass (after opaque) (1.87.8193.35) |
| twosided 1 | no backface culling |
| sample_framebuffer 1/2 | render late (before/after shadows); refraction |
| volumetric 1 | depth pre-pass of back faces |
- Base game MTRs just set `customshadervs vslit_sm / customshaderfs fslit_sm` + `parameter float Metallicness/Specularity/Roughness` (ice, old heads) to stop legacy env-map "metal" look.
- PBR inputs require `renderhint` (mdl or mtr). Parametric inputs only take effect with a custom shader specified (per wiki).

### E.9 How the stock shaders use a texture (inc_standard.shd, fslit*.shd) - behaviour the renderer should mimic
- Shader variants by `#define`s: LIGHTING, FOG, KEYHOLING, NORMAL_MAP, SPECULAR_MAP, ROUGHNESS_MAP, HEIGHT_MAP, SELF_ILLUMINATION_MAP, ENVIRONMENT_MAP, VERTEX_COLOR...; `_nm` = all PBR maps on, `_sm` = ENVIRONMENT_MAP 1, `_sk` = skinning (vIndex/vWeight vec4 attributes, `m_bones[64]` mat4).
- **Legacy env-map rule (ENVIRONMENT_MAP == 1)**: `fEnvMapLevel = 1 - texture0.a`; base colour = mix(tex.rgb, envSample.rgb, fEnvMapLevel); alpha is NOT used as transparency in that path. Without a texture: grey 0.667 fully reflective. Env map coords: sphere-map style `n.xy/(1+|n.z|)` + view offset, or cube map (texUnitEnvCube) when envmap is a cube. The env map texture comes from appearance.2da ENVMAP (creatures), wingmodel/tailmodel ENVMAP, placeables.2da Reflection (`****` none / `default` / tga name), TXI `envmaptexture` (static), area default.
- **Non-envmapped path**: colour *= texture0 (rgba); `AlphaDiscard(a)`: fragments with alpha <= fAlphaDiscardValue (0.2 default) are discarded; material alpha (`alpha` controller) multiplies FragmentColor.a (materialFrontDiffuse.a).
- Specularity/roughness/metallic when no maps: generated from env level (legacy content) or defaults (specularity 0.04 min).
- PBR path (renderhint + MTR maps): texture1 normal (tangent space: vTangent + fHandedness), 2 spec, 3 rough, 4 height (parallax), 5 illum.
- Keyholing (dissolve geometry between camera and player) and fog are per-shader features; tiles with `tilefade` get the Hide-Second-Story fade.
- Particles: fsparticle/vsparticle; shadows: vs_shadowvol/fs_shadowvol (stencil volumes) and beam volumes vs_beamvol/fs_beamvol (`beaming` meshes); grass: vsgrass/fsgrass (surfacemat grass, `GrassTextureName` in .set, default grass.tga).

### E.10 Transparency behaviour summary (for the render queue)
- Opaque pass: meshes with transparencyhint 0 and non-transparent materials. Alpha test at 0.2 (discard) always in the default path.
- Transparent: texture alpha blending (default blending), no automatic sorting of static meshes; `transparencyhint 1..9` gives manual order; TXI `blending additive` (add, never opaque, rendered last), `punchthrough` (binary cut-out ~50%), MTR `transparency 1` (after opaque), `sample_framebuffer 2` (late). 1.69 "a-node" trick: dummy named `<modelname>a` under the root: its children render as dynamic/late (still honoured; fixed in 1.87.8193.35).
- Env-mapped creature/item/placeable textures use alpha as reflectivity, not opacity (hence "shiny" legacy look); wings/tails use ENVMAP from wingmodel/tailmodel.2da (1.69 change).

## F. Special nodes, walkmeshes, lights, shadows, LOD, VFX models, limits

(Supermodel mechanics are in C2; reference nodes in A.4/B.17.)

### C3. Special nodes, lights, shadows, LOD, walkmeshes

#### C3.1 Creature nodes [W:Model Special Nodes] + counts over appearance models [G]

| Node | Parent (typical) | F / S / L presence (of 514/236/38) | Use |
|---|---|---|---|
| `headconjure` | model root (744 of 749) | 509 / 207 / 33 | ConjHeadVisual/CastHeadVisual; its z is the floating-name height. Default about 2.1 m (or 1 m) when missing. |
| `handconjure` | model root | 508 / 206 / 33 | ConjHandVisual/CastHandVisual; BODY_NODE_HAND beams. |
| `impact` | torso_g (468) or chest | 446 / 206 / 38 | Imp_Impact_Node; BODY_NODE_CHEST beam; projectile target. |
| `head` | head_g | 441 / 208 / 37 | Imp_HeadCon_Node. |
| `rhand`, `lhand` | rhand_g, lhand_g | ~444 / 12 / 35 (lhand 18) | Weapon attach (P/F/L; S ignores them). |
| `lforearm` | lforearm_g | 501 / 11 / 9 | Shield attach. |
| `rootdummy` / `*_rootdummy` | root | 513 / 142 / 30 | Full-body animroot. Root VFX use it (Imp_Root_*). |
| `monster0..9` | root | few | BODY_NODE_MONSTER_n beams; `ProjSpwnPoint monsterN`. |
| `head_g` (or appearance `HEAD_NAME`) | neck_g | | Head tracking and look-at. Helmets replace it. |
| `tail`, `wings` | pelvis_g, torso_g | | Tail and wing model attach (appearance MODELTYPE T/W). |
| `imp_shield_1/2` | shield root | | Arrow-hit points on shields. |
| cloak nodes | | | `cloak_g cl1..4_g cm1..4_g cr1..4_g cl1_fg cr1_fg cloak_shl/shr` |

- The phenotype skeleton (pmh0) nodes are:
  `rootdummy torso_g neck_g head_g head Lbicep_g Lshoulder_g lforearm_g lforearm lhand_g lhand Rbicep_g Rshoulder_g rforearm_g rhand_g rhand Impact cloak... pelvis_g tail l/rthigh_g l/rshin_g l/rfoot_g belt_g(1) FB*_g01 TF*_g handconjure headconjure wings` [G].
- The root may be named anything. c_wolf uses `c_Wolf` → `Wolf_rootdummy`. The only rule: the first node = model name, parent NULL [W:MDL ASCII].
- Engine strings: `headconjure handconjure head_g rhand lhand lforearm impact monster%d creadyl creadyr cpause1` [ENG].
- Beams, VFX targets and look-at fall back to the root when a node is missing [W:Model Special Nodes].

#### C3.2 Placeable nodes
**In the model:** `<model name minus its first 4 chars>` + suffix. Examples: plc_a08 → `A08_...`; pwc_ches_001 → `ches_001_...` [W:Model Special Nodes] [G].

| Suffix | Maps to |
|---|---|
| `_impact` | Imp_Impact_Node, BODY_NODE_CHEST |
| `_head_hit` | Imp_HeadCon_Node, text-bubble height |
| `_head` | Look-at target |
| `_hand` | BODY_NODE_HAND |
| `_ground` | Imp_Root_*, BODY_NODE_MONSTER, default |

- All are children of the root [G].
- Engine strings: `_impact _head_hit _head _ground` [ENG].
- Clickable and bounding box: a mesh is needed even if it has `render 0` (e.g. plc_u03 `Rectangle124`; pwc_ches_001 `selectme`) [W:Model Special Nodes] [G].
- **Transparency / "a" node:** a dummy `<modelname>a` between the root and meshes using TXI transparency (water), placed last in the file [W:Model Special Nodes].
  - The toolset **does not stack** placements on meshes whose name ends in "a" ("transparent" to stacking) [W:Placeables and the Toolset].
  - This matters for the toolset's object-on-object placement.

**Walkmesh (.pwk, ASCII in all 1,090 base files [G]):**
- Root name `<MODEL>_pwk`.
- One trimesh (walk and blocking geometry; commonly `<MODEL>_wg`, others `nowalk`...; all faces are surfacemat 7 = non-walkable [W:PWK]).
- Use dummies. Real naming [G-survey]:
  - `xxxx_pwk_use01` 863, `_pwk_use02` 608, `_pwk_use03/04` 12 (the engine uses only 01/02 [W:PWK]).
  - Prefixes vary: 3 to 11 characters.
- Engine matching strings: **`pwk_use`** and **`pwk_dp_use_`** (substring). Error text "Placeable Object (%s) has an invalid use node (%s)!" [ENG].
- Use nodes: a creature walks to the nearer one. use01 should be in the front arc (the toolset arrow). No use node means use at the centre, and chairs face east [W:PWK] [W:Chair Sitting].
- A 3D walkmesh taller than 20–25 units blocks line of sight [W:PWK].

#### C3.3 Door nodes
**In the model:** `<modelname>` + suffix with no underscore [W:Model Special Nodes] [G t_door01, ttr_udoor_01].

| Suffix | Maps to |
|---|---|
| `impc` | Impact, hand, chest |
| `hhit` | Head hit |
| `head` | Head |
| `grnd` | Ground |

- Engine strings: `impc hhit grnd` [ENG].
- The `sam` trimesh (bitmap NULL, alpha keyed in `trans`) is the transition highlight.
- Fire and chunk emitters are used by `die`.

**Walkmesh (.dwk; 249 base files, ASCII):** root `<MODEL>_DWK`; nodes `NN_DWK_...`.

| Node | Count in base files [G-survey] |
|---|---|
| `DWK_wg_closed` | 287 |
| `DWK_wg_open1` | 195 |
| `DWK_wg_open2` | 189 |
| `DWK_dp_closed_01` | 229 |
| `DWK_dp_closed_02` | 221 |
| `DWK_dp_open1_01` | 225 |
| `DWK_dp_open1_02` | 63 |
| `DWK_dp_open2_01` | 211 |
| `DWK_dp_open2_02` | 71 |
| `DWK_dp_closed01` (wiki spelling) | 10 |
| `DWK_dp_close_01` | 9 |

- Walkmeshes: `DWK_wg_closed|open1|open2`. Use points: `DWK_dp_{closed,open1,open2}_{01,02}`.
- **The base data uses `_01`/`_02` with an underscore**, not the wiki's `closed01`.
- Engine strings: `_DWK_wg_`, `open1`, `open2`, `closed`, `_DWK_dp_`, `_02` [ENG]. This suggests substring matching of `_DWK_dp_` + state, with `_02` marking the second point [inf].
- Only orientation, verts and faces are read for meshes, and only position for dummies [W:DWK].
- Closed door: the DWK controls the walkspace. Open door: the tile WOK plus the door's open walkmesh [W:Tileset Construction Tutorial].

#### C3.4 Tile nodes
- **Root dummy** = tile resref (the "aurorabase"). Classification `tile` is required for the walkmesh [W:Tileset Construction Tutorial].
- **Main lights `<tile>ml1`, `<tile>ml2`: node type `light`.**
  - Survey [G-survey]: ml1 12,158, ml2 11,857, all type light.
  - Case-insensitive; typically uppercase resref + lowercase suffix.
  - Engine strings `ml1 ml2 sl1 sl2` [ENG].
  - Base values [G]:
    - tcn01_a01_01ml1: pos (0,0,9), radius 14, multiplier 1, colour 0, priority 5, ambientonly 0, nDynamicType 0, affectdynamic 1, shadow 0, fading 1.
    - ml2: radius 5, colour 0.004.
    - tds01_c13_01ml1: radius 10.
  - Colour comes from the GIT `Tile_MainLight1/2` = index into **lightcolor.2da** RED/GREEN/BLUE (0–2.0 range, 32 rows; the TOOLSETRED/GREEN/BLUE columns feed the picker swatches) [PDF-ARE] [2da].
  - A value of 0 means off or missing. The toolset disables the control when the node is missing [PDF-ARE].
  - The wiki claims the engine overrides shadow 1, affectdynamic, priority 4, shadowradius 12/8 and radius 10/5 [W:Model Special Nodes]. **Unverified;** the base files carry radius 14/5 and shadow 0.
  - Area Lighting says main lights do not cast shadows [W:Area Lighting].
- **Source lights `<tile>sl1`, `<tile>sl2`:** mostly `dummy` (612/277); 77 are `light` nodes [G-survey].
  - Spawn **`fx_flame01.mdl`** at the node and play the animation named by the GIT `Tile_SrcLight1/2` value `1`..`15`. 0 = off [PDF-ARE].
  - fx_flame01 has animations `1`..`15` (0.5 s) keying the emitter colours and birthrate and the `AuroraLight01` colour (radius 7, shadowradius 15, verticaldisplacement 2, priority 4) [G].
  - The approximate colour for the UI is lightcolor.2da row = value×2 [PDF-ARE]. tilecolor.2da (16 rows) also exists [2da].
  - TS: `fx_flame01 sourcelight1 fx_flame01 sourcelight2`.
- **Other lights on tiles:** `auroralight##` (223), `l_dayn#` (158, day/night lights), `candlelt#`, `windowlt#` [G-survey].
  - They are static or animated lights (e.g. tcn01_a08_01 auroralight06: nDynamicType 1, priority 3, radius 10).
  - Their colour is **not** set by the area. Render them as normal lights.
- **a-node** `<tile>a` (C1.2).
- **AABB walkmesh node:** 12,479 tiles have exactly 1 aabb node, 956 have none, 1 has 2 [G-survey]. Only one AABB per model; parented to the root; no children; at the origin; ≤ 8 faces per vertex [W:Walkmesh Notes] [W:Tileset Construction Tutorial].
  - The WOK file holds the same AABB (ASCII `#MAXWALKMESH`, `beginwalkmeshgeom`) [G tcn01_a01_01.wok].
  - Edge tiles can have an empty WOK (ttr01_z15_01.wok, 87 bytes) [G].
  - Face material = surfacemat.2da row.
- **tilefade** per mesh [W:MDL ASCII]: 0 never, 1 fade, 2 "base", 3 neighbour.
  - Values in data [G-survey]: 0 ×206,135, 1 ×22,883, 2 ×3,087, **4 ×1,877** (undocumented; e.g. tcn01 `line161` black caps next to tilefade 2 `line160`).
  - Used by the "Hide second story tiles" option. Toolset menu Tile Fade ("Always" mode fixed in 85.8193.32) [CL 85.8193.32].
- **Shadow-only meshes** (shadow 1, render 0): 12,359 in tiles [G-survey].
- `rotatetexture 1` on ground meshes [W:MDL ASCII].
- Door placement dummies are non-functional. Door positions come from the SET `[TILEnDOORm]` [W:Tileset Construction Tutorial].
- The toolset caps lights at 2 main + 2 source per tile [W:Tileset Construction Tutorial].

#### C3.5 Sun, moon and sky
- **gidy_sun.mdl** (classification none) has two lights [G]:
  - `gidy_sun_amb`: ambientonly, radius 100000, priority 1.
  - `gidy_sun_diff`: shadow 1, radius 100000.
- It is placed at about (4000, 4500, 7000) [W:Area Lighting].
- The toolset and game recolour it with `controlpart sun gidy_sun_amb color r g b` / `gidy_sun_diff` from area Sun/Moon Ambient/Diffuse (day or night by IsNight/DayNightCycle), and use `enableshadowing`/`disableshadowing` [TS] [ENG].
- Only the sun, or the PC light when no sun, casts shadows on static geometry [W:Area Lighting].
- **Skyboxes:** skyboxes.2da gives DAWN/DAY/DUSK/NIGHT models (skyda_001, sky_001, skyd_001, skyn_001...; classification effect, 2 trimeshes, no animations) [G] [W:skyboxes.2da].
  - The models are centred on the viewer; transitions cross-fade over 1 game hour.
  - `skyfade1.mdl` (mesh `skyfade1g`, texture skyblurpoly) blends fog.
  - A skybox adds 90 m to the tile render distance (fog clip + 90) [W:skyboxes.2da] [W:Render Distance].
- Default render distances [W:Render Distance]:
  - Tiles and static placeables: fog clip distance (default 45).
  - Dynamic placeables and doors: 45 m.
  - Creatures: 35 m.

#### C3.6 Shadows
- **Stencil shadow volumes, only from trimesh nodes with `shadow 1`.** Skin and dangly meshes probably do not cast [W:Model Shadows] [W:Model Table of Parameters].
- Common pattern: the visible mesh has `shadow 0`, and a low-poly proxy has `shadow 1 render 0` (tiles: 12,359 such meshes [G]).
- Artefacts:
  - Edges shared by more than 2 faces cause shadow "tearing" [W:Model Shadows].
  - Pivot placement (the shadow volume is built relative to the node origin) causes spikes and negative shadows [W:Making Better Shadows in MAX].
- `beaming 1` renders the shadow volume as a coloured, alpha light shaft (forest light rays) [W:MDL ASCII] [W:Making Better Shadows].
- Classification:
  - `character` casts shadows.
  - EE 35: "Models of classification other than `character` only cast shadows if not transparent", and "Models with classification other than Character now all project shadows" [CL 87.8193.35-40].
  - Config `graphics.shadows.all-types-can-cast-dynamic`.
- Fast-shadow fallback: `placeables.2da.ShadowSize` ****/0/1/2 → blob VFX `fx_shadow_s/m/l.mdl` (emitter animations `duration`/`cessation`) [W:placeables.2da] [G].
- Area: SunShadows/MoonShadows flags; ShadowOpacity 0–100 [W:Area Lighting].
- Lights:
  - `shadow`, `affectdynamic`, `shadowradius` (fade over distance), `lightpriority` 1–5, `fadinglight` (1.5 s fade) [W:MDL ASCII].
  - Client limits: 3–128 lights (default 32), 0–3 shadow-casting lights [W:Area Lighting].

#### C3.7 LOD
- `.lod` text file next to `x.mdl` [PN 1.80.8193.6] [W:LOD]:
  - `x_0` / `40 x_1` / `80.5 x_2`.
  - Up to 3 levels, switched by **camera** distance.
  - Scaled by `graphics.lod.scale-factor` (tuned for 1080p). Also `graphics.lod.enabled` [ENG].
- Applies to creatures, dynamic placeables and VFX. **Not** tiles or static placeables [W:LOD] [W:Placeable].
- **No `.lod` file ships with the base game** [G: resman grep]. The toolset has `g_bEnableLOD` [TS]. The editor can ignore LOD or use level 0 [inf].
- EE fix: wielded items stay in sync with creature LOD models [CL 87.8193.35-40].

#### C3.8 Other model-level flags relevant to rendering
- `classification` [W:MDL ASCII]:
  - `character`: shadows.
  - `effect(s)`: emitters render at any distance; not highlighted by TAB; ignored for hit effects.
  - `door`: fade-through.
  - `tile`: static/BSP.
  - Anything else = none (GUI, walkmesh).
  - The toolset parser accepts `tile character effect effects door` and `ignorefog` [TS].
- `ignorefog` (header): dynamic models only [W:Model Table of Parameters].
- Danglymesh: period/tightness/displacement plus per-vertex constraints 0–255. It reacts to wind (area wind, SetAreaWind), movement and detonate blasts, and freezes when the game is paused [W:MDL ASCII] [W:NeverBlender: Danglymesh] [PN 1.81.8193.17].
- Hilite glow texture `fxpa_dot02` (hardcoded) [W:Object Hilites].

---

### C4. VFX models

#### C4.1 Prefixes (classification effect; all NULL supermodel) [G-survey + visualeffects.2da/spells.2da]

| Prefix | Count | Typical animations | Source column / meaning |
|---|---|---|---|
| `vfx_` | 140 | (few) | Mostly VFX_EYES_* (head node) and weapon glows. visualeffects Imp_HeadCon/Root_H. |
| `vco_` | 134 | conjure01, fade | spells.2da ConjHead/Hand/GrndVisual (conjuration) [W:VFX Casting Models] |
| `vim_` | 121 | impact | VFX_IMP_* (impact); also ray and beam models (vim_ray*, vim_lashfire) |
| `vdr_` | 99 | duration, cessation, impact | VFX_DUR_* |
| `vff_` | 90 | impact | VFX_FNF_* (fire and forget, ground) |
| `fx_` | 46 | various | Engine models: fx_flame01, fx_placeable01, fx_shadow_*, fx_light_clr, fx_ref (dummy reference target), fx_ref_* |
| `vwp_` | 39 | impact | VFX_COM_* chunks and blood (cached) [W:VFX Effect Caching] |
| `vpr_` | 38 | travel01 | spells.2da ProjModel projectiles (ProjType homing/accelerating/ballistic/linked/spiral/bounce) |
| `vca_` / `var_` | 33/10 | cast01 | spells.2da CastHandVisual (cones, breath) |
| `vps_` | 29 | impact, duration, cessation | vfx_persistent.2da MODEL01..03 (AOE). `_L` = MODELMINnn low-quality variants. |
| `vcm_` | 10 | impact | Monk hits (hardcoded vcm_monkquiv/stun/ki) [W:Hardcoded VFX References] |
| `vpm_` | 11 | impact, duration, cessation | VFX_DUR_* (Imp_Root_M) |
| `vce_` | 3 | cessation | vce_neutral/positive/negative, used via Imp_HeadCon_Node [W:visualeffects.2da] |
| `vdu_` | 1 (+ vdu_tex_*) | none | vdu_beam000 (lightning beam), skin overlays for progfx type 1 |
| red_/grn_/wht_ | | impact | Gib chunk models (chunkName targets) |
| tile models (tcn/ttr/tdm...) | | | visualeffects "SCENE_*" rows (tile magic) |

#### C4.2 Where VFX attach [W:visualeffects.2da] [W:Model Special Nodes]

| Column | Creature node | Placeable node | Door node |
|---|---|---|---|
| `Imp_HeadCon_Node` | `head` | `_head_hit` | `hhit` |
| `Imp_Impact_Node` | `impact` | `_impact` | `impc` |
| `Imp_Root_S/M/L/H_Node` (by creature size) | rootdummy/root | `_ground` | `grnd` |

- The `Ces_*` columns "likely don't work".
- `OrientWithGround`, `OrientWithObject` (flags and eyes).
- Spells: `ConjHeadVisual` → headconjure, `ConjHandVisual` → handconjure, `ConjGrndVisual` → root; `CastHand/Head/GrndVisual` likewise; `ProjSpwnPoint` hand/head/monsterN [2da spells.2da] [W:Model Special Nodes].
- progfx type 12 attaches a VFX model to **any named node** (Param1, e.g. `headconjure`) [W:progfx.2da].
- EffectVisualEffect(..., fScale, vTranslate, vRotate) transforms the VFX [W:Visual Effects].

#### C4.3 Emitters, beams, lights in VFX
- Emitter parameters [W:MDL ASCII Emitter Nodes]:
  - `update`: Fountain, Single, Explosion (fires on `detonate`), Lightning.
  - `render`: normal, linked, billboard_to_local_z, billboard_to_world_z, aligned_to_world_z, aligned_to_particle_dir, motion_blur.
  - `blend`: normal, punch-through, lighten.
  - `spawntype`: 0 normal, 1 trail.
  - texture grid animation (xgrid/ygrid/fps/frameStart/End/random), `chunkName` (model particles), inheritance flags, gravity and p2p (Bezier or Gravity; needs a **reference** child node), lightning parameters.
  - `birthrate` > 9 on Lightning crashes; the emitter size is capped at 500×500 cm; p2p does not work on tiles or placeables.
- Beams: progfx type 7 (Param1 model, Param2 animation `cast01`). The engine detaches the **target** of the p2p reference to the target object. Default beam models: vdu_beam000 (lightning; 3 emitters + `OmenRef##` references) and vim_ray* [W:progfx.2da] [G].
- VFX often contain a `light` node (`AuroraLight01`), e.g. 128 of the vco_* models [G-survey].
- EE fixes:
  - Emitters with custom shaders [CL 87.8193.35-40].
  - Soft particles and fog blending [PN 1.83.8193.21].
  - The toolset renders VFX emitters properly [PN 1.81.8193.17 "Toolset: Fixed VFX emitters not rendering properly (#162)"].
- `cachedmodels.2da` (client; 88.8193.36+) keeps models resident. It lists the gib and chunk models [W:cachedmodels.2da] [CL 88.8193.36-11].

---

### C5. Limits (models and animations)
The Resource Limits page [W:Resource Limits] has **no model limits** (only resource and ERF counts). Collected from elsewhere:

| Limit | Value | Source |
|---|---|---|
| Animation name | ≤ 16 characters (longer ones do not play) | W:Animations; none >16 in data [G] |
| Node name | ≤ 32 characters (binary char[32]; longer truncates and mismatches) | W:Model Table of Parameters, W:Models, [G] struct |
| Model and supermodel name | 64-byte fields in binary, but resrefs ≤ 16 | [G] struct, W:Tileset Construction Tutorial |
| Texture and bitmap name | resref ≤ 16 | W:Model Table of Parameters |
| Bones | 1.69: 17 per skinmesh; EE ≤ 8193.19: 128 per model; **8193.21+: 64 per skinmesh**, multiple skinmeshes allowed; **4 bones per vertex** | W:Models, PN 1.83.8193.21, PN 1.80.8193.7 ">17 bones" |
| Vertices per mesh | EE 35+: 65535 index limit (21,845 triangles per mesh); before: about 10,922 faces | W:Models, CL 87.8193.35 "up to 3x more vertices per mesh" |
| Custom animation slots | custom1..70 | W:Animations, PN 1.80.8193.6 |
| Tile lights | 2 main (ml1/ml2) + 2 source (sl1/sl2) per tile | W:Tileset Construction Tutorial |
| Tile anim loops | 3 | PDF-ARE |
| Walkmesh | ≤ 8 faces per vertex; one AABB per model; walkable faces point up and do not overlap; edges at exactly ±5 m | W:Walkmesh Notes, W:Tileset Construction Tutorial |
| PWK use nodes | only use01 and use02 honoured | W:PWK |
| DWK | 2 use points per state (closed/open1/open2) | W:DWK |
| LOD | 3 levels | W:LOD |
| Danglymesh period | < 60 | W:MDL ASCII |
| Lights | client max 3–128 (default 32); shadow-casting 0–3; lightpriority 1–5 | W:Area Lighting, W:MDL ASCII |
| Phenotypes | 0–20 | W:Part-Based Models |
| Body part and armour variations | 999 (was 255) | CL 87.8193.35 |
| Tilesets in use | 100 (limit removed in 89.8193.37-13) | W:Tilesets, CL 89.8193.37-13 |

---

## G. Changelog items (version: one line)

### G.1 Formats, textures, materials, general

Sources: game `lang/en/docs/CHANGELOG.md` (85.8193.32 .. 89.8193.37-17), `patchnotes/*.md`, `Neverwinter Nights Enhanced Edition (v74..v79).txt` (v74 = cp1252, contains the 1.69-era notes), and nwn.wiki release-note pages (e.g. https://nwn.wiki/spaces/NWN1/pages/60982592/1.80.8193.9, .../38176139/1.83.8193.21, .../91324440/1.87.8193.35, .../123797539/1.88.8193.36, .../149291011/1.89.8193.37).

1.69-era (v74.txt, legacy notes):
- 1.69: wings/tails use the ENVMAP column of wingmodel.2da/tailmodel.2da instead of appearance.2da.
- 1.69: wings/tails use a PLT if one exists (creature skin colour), else TGA.
- 1.69: part-based creatures past appearance row 481 use PLT names built from the RACE letter.
- 1.69: phenotype.2da DefaultPhenoType column = fallback phenotype when a model is missing; heads use it too.
- 1.69: placeables.2da `Reflection` column (`****` none, `default`, or tga name) enables placeable env mapping.
- 1.69: PLT support for visible cloaks; .set `GrassTextureName` (default grass.tga).
- 1.69: DoorTypes VisibleModel=0 doors listed in toolset.
EE:
- 1.74.8155: TXI `rotatetexture` handling fixed in new shaders.
- 1.74.8156: skinmeshes on armour parts.
- 1.74.8158: ASCII MDL `normals` and `tangents` read; animmeshes support tangent space.
- 1.74.8159/8160/8162: .mtr material files (`materialname`, texture0..N, custom shaders, parameters); extra UV streams `tverts1..3` (vTcIn1..3); vertex `colors` exposed.
- 1.74.8163: part combining re-enabled for new streams (tangents, extra UVs, colours).
- v75 (1.75): MTR may specify `renderhint` (mesh-level overrides); meshes need no texture if an MTR is set; `bitmap`/`texture0` may name an MTR; animated meshes without animations render static.
- v78 (1.78): standard DDS (with `DDS ` header) supported; KTX containers supported.
- v79 (1.79.8193): console/CLI model compiler (`compilemodel`, `compileloadedmodels`, `compileloadedasciimodels`, MODELCOMPILER alias); automatic tangent generation fixed; ASCII parser skips empty lines (wrong array counts still break).
- 1.80.8193.6/.9: `.lod` files (up to 3 LOD levels + distances); renderhint `NormalTangents`; >17 bones; mipmaps generated for uncompressed textures; human female PLTs fall back to male; `SetTextureOverride`.
- 1.80.8193.14 / 1.81.8193.15: new PBR lighting engine (specular/roughness etc.); `ReplaceObjectTexture`; VFX can use PLT (colours from target).
- 1.81.8193.17: default ambient/diffuse of newly compiled models now 1.0 (old compiles 0.2/0.8); danglymesh frozen when paused.
- 1.82.8193.20: normal maps read as 2-channel (BC5); specular ignores material transparency; material names allow underscore.
- 1.83.8193.21/.23: **max 64 bones per skinmesh**; PLT palette tweaks (less metallic); `decal` now truly unlit; old human heads + icy tiles get MTRs; DisplacementOffset parameter.
- 1.83.8193.26: tangent space computed for legacy compiled models at runtime; env maps accept `default` in TXI; texture downsampling disabled for uncompressed textures; custom-shader alpha needs TXI alphamean < 1.
- 1.85.8193.30: surfacemat supports 64 materials; static placeables no longer break animations of non-static ones with the same model.
- 1.85.8193.31: textures larger than GPU max auto-downsized; `rotatetexture` fixed for normal/displacement maps; **PLT textures can be phenotype-specific**; skinmesh bone hierarchy fixed to initial layout (dynamic body-part attach).
- 85.8193.32: shadow renderer rework (shadow/beam volumes have own shaders); debug rendering of bounding boxes, pivots, emitter/light ranges.
- 86.8193.34.1: animated skinmesh parts one frame behind fixed; animation start consistency.
- 87.8193.35: **max ~21,845 faces per mesh (3x more vertices)**; MTR `transparency/twosided/sample_framebuffer/volumetric`; PLT generation on GPU option; emitters with custom shaders; non-character classifications cast shadows only if not transparent; 'a' nodes rendered dynamic again; cubemap env maps from TXI fixed; wielded items sync with creature LOD models; SetObjectVisualTransform on HEAD/WING/TAIL/CLOAK sub-models; "Log Model Errors" logs rejected ASCII commands; `ReplaceObjectAnimation`, `SetTileAnimationLoops`.
- 88.8193.36: `cachedmodels.2da`; PLT-on-GPU via offscreen FBO; **compiled models validated on first load (invalid normals/tangents fixed)**; crash fixes for no/corrupt faces, missing bone weights, corrupt TGA; custom palettes with PLT-on-GPU; part combining respects differing materialname; flat double-sided planes normals; procedural distortion >256x256 memory fix; armor parts >255 in toolset; DDS in NUI.
- 89.8193.37 (-13..-17): `vPos` shader attribute now world space; bboxMin/bboxMax uniforms; anisotropic filtering/MSAA options; toolset shows model part ID; toolset loads non-human PLTs correctly; custom `pal_XXXyy.tga` loaded from haks/NWSync per module; no more auto-merging parts at model level for game-compiled models; texture pack code removed; keyhole-dissolvable parts rendered in transparent pass; bad tangents no longer glow with HDR bloom; tailmodel/wingmodel/skyboxes row 0 = "None"; toolset scales oversized item icons; `EMOTE_ANIMATIONS_USE_MDL_TIMINGS` in ruleset.2da.
- 89.8193.37-15: fixed some model parts appearing rotated when rendered in the toolset.

### G.2 Animation / rendering (from C6)
- **89.8193.37-15:** Fixed model parts appearing rotated when rendered in the toolset. Removed a log about duplicate FNF attack animations.
- **89.8193.37-13:**
  - `EMOTE_ANIMATIONS_USE_MDL_TIMINGS` ruleset option (custom emotes and ReplaceObjectAnimation not cut off).
  - No longer auto-merges parts at model level.
  - Fixed a shadow memleak and a crash with unenhanced lighting in some tilesets.
  - VFX lights no longer fade in too slowly.
  - Keyhole-dissolvable parts render in the transparent pass.
  - Models with bad tangents no longer glow white under HDR.
  - Toolset shows the model part id; toolset tileset-specific door palette fixes; toolset caches door 2DAs.
  - The 100 tileset limit was removed.
  - Area load keeps the tileset in memory.
  - New bboxMin/bboxMax shader uniforms.
- **88.8193.36-11:**
  - New `cachedmodels.2da`.
  - Validation of compiled models on load (fixes invalid normals and tangents).
  - Better log for bad PWK use nodes.
  - Fixed crashes: missing bone weights in skinmeshes; models with no or corrupt faces.
  - Fixed flat planes' normals; part combining now respects materialname; keyholing fixes and an option for the minimum Z offset.
  - Shadow-edge building optimised; emitter and particle optimisations.
  - Fixed VFX on placeables not fading on destroy; aura VFX after area change.
- **87.8193.35-40:**
  - Dynamic area lighting (moving sun; `nw_dynlight.nss`) and `SetAreaLightDirection`.
  - Emitters with custom shaders.
  - `ReplaceObjectAnimation()`, `SetTileAnimationLoops()`, `SetObjectVisualTransform` on sub-models (HEAD/WING/TAIL/CLOAK).
  - "Models of classification other than character only cast shadows if not transparent"; "classification other than Character now all project shadows"; `graphics.shadows.all-types-can-cast-dynamic`.
  - 3x more vertices per mesh.
  - Fixed 'a' nodes not always rendered dynamic.
  - Fixed wielded items out of sync with creature LOD models.
  - Fixed lightning emitters with linked render (ray of frost) and thin beams.
  - MTR `transparency`, `twosided`, `sample_framebuffer`, `volumetric`.
  - 8 new tile pathnodes (q–x).
  - "Log Model Errors" logs all rejected ASCII MDL commands.
- **86.8193.34.1:** Minor fixes to animation start consistency. Animated skinmesh parts no longer one frame behind.
- **85.8193.33:** Tile source lights showed red after a save and load (fixed).
- **85.8193.32:**
  - Major shadow-renderer rewrite (shader volumes, tile bounding box as the lower clip, alpha by height).
  - Debug rendering of bounding boxes, pivots, emitter and light ranges and shadow volumes.
  - Skinmesh bone hierarchy fix.
  - Toolset shadow inconsistencies fixed; toolset second-story tile fade "Always" fixed.
- **1.83.8193.21 / .23:**
  - Maximum 64 bones per skinmesh.
  - Soft particles and fog blending.
  - Safeguard against a missing animation.
  - Emitter size_y interpolation fix.
  - Skinmesh normal and tangent fixes.
  - `SetObjectVisualTransform` lerp parameters.
- **1.82.8193.20 / 1.81.8193.17:**
  - Toolset: fixed VFX emitters not rendering and some shadows.
  - Danglymesh frozen when paused.
  - Cloaks and robes inherit the visual-transform animation speed.
  - Default mesh ambient and diffuse = 1.0.
  - Texture animations consistent.
  - Skyfade covers below the horizon.
  - XP3 and DoD placeables: use nodes reworked to a "strict 13-character length", 3D PWKs.
- **1.81.8193.16:** No N:-drive read for a missing supermodel. New progfx.2da.
- **1.80.8193.14 / 1.81.8193.15:**
  - progfx unhardcoded.
  - Walk animations unhardcoded (`walk_002`+).
  - Visual effects can use PLT.
  - Tile fixes: animloops for shutdown, a-nodes for transparency (TTS02/TTF02).
  - VFX models get fog and no forced self-illumination.
- **1.80.8193.10–.13:** VFX, emitters and lens flares go through post-processing. Many tile animloop and emitter fixes. The `.lod` parser accepts a missing final newline (.13). nwhak supports `.lod`.
- **1.80.8193.6 / .9:** `.lod` files; custom animation slots 21–70.
- **1.80.8193.7:** Support for more than 17 bones. Cheaper shadows. 150+ tiles with bad emitters or animation sequences fixed; all 192 doors checked for animations and transitions.
- **1.79.8193 / 8192:**
  - Keyholing ported from Android.
  - `compileloadedmodels` / `compileloadedasciimodels` / `compilemodel`.
  - Skinmesh crash and leak fixes.
  - Uncompiled parser skips empty lines.
- **v74 (EE launch):** 1.69 MDLs binarised (horses, dragon wings). Skinned animation sync fix. Shadow-edge generation fix for complex models.

---

### G.3 Creatures, items, PLT (from D)
From `lang/en/docs/CHANGELOG.md` and `Neverwinter Nights Enhanced Edition (v74..v79).txt` (v74 file contains 1.61–1.69 history):
- 1.61 — Added robes; armor icon composition hides parts per parts_robe; wings & tails support; placeables.2da `Reflection` env-map column.
- 1.65 — custom phenotypes raised to 18.
- 1.66 — fixed helmet placement on human/half-elf females.
- 1.67 — 176-colour palettes ("expanded colour choices"); placeables.2da `Static`; custom P races (> row 481) use RACE letter for PLT, else human PLT; robe inventory icons show non-hidden robe/pelvis/belt/torso/shoulders; phenotype.2da `DefaultPhenoType` fallback (also heads); wings/tails use PLT if present (default skin colour) else TGA; hook hands/peg leg parts.
- 1.68 — visible cloaks + PLT for cloaks; wing/tail ENVMAP columns in wingmodel/tailmodel.
- 1.69 — horses (phenotypes 3/5/6/8), new armor/weapon parts; SetPhenoType limit 99; wing/tail/genericdoors row limits raised; toolset fixes for non-human cloaks on NPCs and phenotype ≥10 textures.
- 1.74 (v74 EE) — Get/SetHiddenWhenEquipped (hide helmet etc.); CopyItemAndModify/GetItemAppearance per-part colours (index formula, 255 clears); weapon colours 1–9; crash fix for broken PLTs; skinned anim sync fix.
- 1.75 — racialtypes Icon; armour part colours in chargen fixed; inventory PLT icon colour fix; layered-texture cache corruption fix; Object Visual Transforms (scale/translate/rotate creatures, items, placeables, doors).
- 1.77 — belts/shoulder pads disappearing on re-equip fixed; PLT double-free fix.
- 85.8193.32 — PLT textures can be phenotype-specific; skinmesh bone hierarchy reset for dynamically attached skinned body parts; toolset shadow fixes.
- 85.8193.33 — crash when PLT fails to load.
- 87.8193.35 — PLT generation on GPU (`graphics.experimental.generate-plt-with-shaders`); body part/armor variation limit 255→999; toolset no longer restricts part ranges; GetItemAppearance per-part; SetObjectVisualTransform can target HEAD/WING/TAIL/CLOAK sub-models; DDS portraits + fallback; "Fixed textures on body parts that don't use PLT (#471)"; HD head normal-map fix.
- 88.8193.36 — cachedmodels.2da; PLT-on-GPU uses offscreen FBO (no size limit); PLT colour-change update fix; custom palettes with GPU PLT; armor parts > 255 in toolset fixed; SetCreatureBodyPart fixes (not modifying armor; skinmesh-less parts); SetMaterialShaderUniform propagates to head/wing/tail/cloak; baseitems rows > 255 fixed.
- 89.8193.37-13 — toolset shows model part ID next to ordered ID; toolset sorts Tail/Wings lists; toolset loads non-human PLT textures; custom `pal_XXXyy.tga` from haks/NWSync; toolset scales oversized icons; robe icons > 255 fixed; c_nulltail trimesh removed; keyhole-dissolvable parts rendered with transparent pass.
- 89.8193.37-15 — "Resolved a bug that would make some model parts appear as rotated when rendered in the toolset."

---------------------------------------------------------------------------------------------------

## H. Citations

### H.1 nwn.wiki pages (local mirror `~/.local/opt/neverwinter/wiki/pages/NWN1/<id>-*.md`)

- 1.79.8193 (id 60982619): https://nwn.wiki/spaces/NWN1/pages/60982619/1.79.8193
- 1.80.8193.10 (id 38176092): https://nwn.wiki/spaces/NWN1/pages/38176092/1.80.8193.10
- 1.80.8193.13 (id 38176086): https://nwn.wiki/spaces/NWN1/pages/38176086/1.80.8193.13
- 1.80.8193.14 (id 38174859): https://nwn.wiki/spaces/NWN1/pages/38174859/1.80.8193.14
- 1.80.8193.6 (id 60982603): https://nwn.wiki/spaces/NWN1/pages/60982603/1.80.8193.6
- 1.80.8193.7 (id 60982599): https://nwn.wiki/spaces/NWN1/pages/60982599/1.80.8193.7
- 1.80.8193.9 (id 60982592): https://nwn.wiki/spaces/NWN1/pages/60982592/1.80.8193.9
- 1.81.8193.15 (id 38174976): https://nwn.wiki/spaces/NWN1/pages/38174976/1.81.8193.15
- 1.81.8193.16 (id 38174991): https://nwn.wiki/spaces/NWN1/pages/38174991/1.81.8193.16
- 1.81.8193.17 (id 38175058): https://nwn.wiki/spaces/NWN1/pages/38175058/1.81.8193.17
- 1.82.8193.20 (id 38175678): https://nwn.wiki/spaces/NWN1/pages/38175678/1.82.8193.20
- 1.83.8193.21 (id 38176139): https://nwn.wiki/spaces/NWN1/pages/38176139/1.83.8193.21
- 1.83.8193.26 (id 38176504): https://nwn.wiki/spaces/NWN1/pages/38176504/1.83.8193.26
- 1.85.8193.30 (id 48988173): https://nwn.wiki/spaces/NWN1/pages/48988173/1.85.8193.30
- 1.87.8193.35 (id 91324440): https://nwn.wiki/spaces/NWN1/pages/91324440/1.87.8193.35
- 1.88.8193.36 (id 123797539): https://nwn.wiki/spaces/NWN1/pages/123797539/1.88.8193.36
- 1.89.8193.37 (id 149291011): https://nwn.wiki/spaces/NWN1/pages/149291011/1.89.8193.37
- 2da Files (id 38174875): https://nwn.wiki/spaces/NWN1/pages/38174875/2da+Files
- A Shader for Mixing PLT Textures with Transparency and A Texture Override (id 53670455): https://nwn.wiki/spaces/NWN1/pages/53670455/A+Shader+for+Mixing+PLT+Textures+with+Transparency+and+A+Texture+Override
- Adding Doors to a Tile (id 38175683): https://nwn.wiki/spaces/NWN1/pages/38175683/Adding+Doors+to+a+Tile
- Animations (id 38175170): https://nwn.wiki/spaces/NWN1/pages/38175170/Animations
- appearance.2da (id 38174941): https://nwn.wiki/spaces/NWN1/pages/38174941/appearance.2da
- Area Lighting (id 38174907): https://nwn.wiki/spaces/NWN1/pages/38174907/Area+Lighting
- armor.2da (id 49446964): https://nwn.wiki/spaces/NWN1/pages/49446964/armor.2da
- baseitems.2da (id 38174935): https://nwn.wiki/spaces/NWN1/pages/38174935/baseitems.2da
- cachedmodels.2da (id 145457156): https://nwn.wiki/spaces/NWN1/pages/145457156/cachedmodels.2da
- Chair Sitting or Facing Direction (id 93487105): https://nwn.wiki/spaces/NWN1/pages/93487105/Chair+Sitting+or+Facing+Direction
- Changing day night tile states through tile animations e.g. glowing windows at night (id 60984838): https://nwn.wiki/spaces/NWN1/pages/60984838/Changing+day+night+tile+states+through+tile+animations+e.g.+glowing+windows+at+night
- cloakmodel.2da (id 53671311): https://nwn.wiki/spaces/NWN1/pages/53671311/cloakmodel.2da
- Cloaks - resizing dynamic cloak models (id 111607812): https://nwn.wiki/spaces/NWN1/pages/111607812/Cloaks+-+resizing+dynamic+cloak+models
- Common Issues with Tiles and Tilesets (id 72417367): https://nwn.wiki/spaces/NWN1/pages/72417367/Common+Issues+with+Tiles+and+Tilesets
- Content Load Order (id 38174823): https://nwn.wiki/spaces/NWN1/pages/38174823/Content+Load+Order
- Converting a Texture to PLT (id 60985308): https://nwn.wiki/spaces/NWN1/pages/60985308/Converting+a+Texture+to+PLT
- Creature BASE Models (id 49447555): https://nwn.wiki/spaces/NWN1/pages/49447555/Creature+BASE+Models
- Creature JSON (id 38176577): https://nwn.wiki/spaces/NWN1/pages/38176577/Creature+JSON
- Creature Size (id 38174921): https://nwn.wiki/spaces/NWN1/pages/38174921/Creature+Size
- Custom Dynamic Race (id 38176932): https://nwn.wiki/spaces/NWN1/pages/38176932/Custom+Dynamic+Race
- DDS (id 3473496): https://nwn.wiki/spaces/NWN1/pages/3473496/DDS
- Dealing with transparency (id 129236997): https://nwn.wiki/spaces/NWN1/pages/129236997/Dealing+with+transparency
- Door (id 38174806): https://nwn.wiki/spaces/NWN1/pages/38174806/Door
- DWK (id 91324443): https://nwn.wiki/spaces/NWN1/pages/91324443/DWK
- Enhanced Lighting Engine and PBR (id 38175899): https://nwn.wiki/spaces/NWN1/pages/38175899/Enhanced+Lighting+Engine+and+PBR
- Environment Maps and Cubemaps (id 38174773): https://nwn.wiki/spaces/NWN1/pages/38174773/Environment+Maps+and+Cubemaps
- genericdoors.2da (id 38175673): https://nwn.wiki/spaces/NWN1/pages/38175673/genericdoors.2da
- Hardcoded VFX References (id 38175930): https://nwn.wiki/spaces/NWN1/pages/38175930/Hardcoded+VFX+References
- Icons (id 38175245): https://nwn.wiki/spaces/NWN1/pages/38175245/Icons
- Importing Animations (id 195362863): https://nwn.wiki/spaces/NWN1/pages/195362863/Importing+Animations
- Item (id 26738909): https://nwn.wiki/spaces/NWN1/pages/26738909/Item
- KTX (id 133988392): https://nwn.wiki/spaces/NWN1/pages/133988392/KTX
- Layered PLT Items (id 60985278): https://nwn.wiki/spaces/NWN1/pages/60985278/Layered+PLT+Items
- LOD - Level of Detail (id 26738916): https://nwn.wiki/spaces/NWN1/pages/26738916/LOD+-+Level+of+Detail
- Major Changes in NWN EE (id 26738928): https://nwn.wiki/spaces/NWN1/pages/26738928/Major+Changes+in+NWN+EE
- Making Better Shadows in MAX (id 60982010): https://nwn.wiki/spaces/NWN1/pages/60982010/Making+Better+Shadows+in+MAX
- Making Item Icons in the Toolset (id 64815319): https://nwn.wiki/spaces/NWN1/pages/64815319/Making+Item+Icons+in+the+Toolset
- MDL (id 38175669): https://nwn.wiki/spaces/NWN1/pages/38175669/MDL
- MDL ASCII (id 12027273): https://nwn.wiki/spaces/NWN1/pages/12027273/MDL+ASCII
- MDL ASCII Emitter Nodes (id 26738875): https://nwn.wiki/spaces/NWN1/pages/26738875/MDL+ASCII+Emitter+Nodes
- Model Shadows (id 49447442): https://nwn.wiki/spaces/NWN1/pages/49447442/Model+Shadows
- Model Special Nodes (id 38176272): https://nwn.wiki/spaces/NWN1/pages/38176272/Model+Special+Nodes
- Model Table of Parameters (id 53671005): https://nwn.wiki/spaces/NWN1/pages/53671005/Model+Table+of+Parameters
- Models (id 38175602): https://nwn.wiki/spaces/NWN1/pages/38175602/Models
- MTR (id 12027232): https://nwn.wiki/spaces/NWN1/pages/12027232/MTR
- Neverblender Animations (id 123797566): https://nwn.wiki/spaces/NWN1/pages/123797566/Neverblender+Animations
- NeverBlender Danglymesh (id 53673556): https://nwn.wiki/spaces/NWN1/pages/53673556/NeverBlender+Danglymesh
- Object Hilites (id 38176097): https://nwn.wiki/spaces/NWN1/pages/38176097/Object+Hilites
- Part-Based Models Creatures (id 49447501): https://nwn.wiki/spaces/NWN1/pages/49447501/Part-Based+Models+Creatures
- Part-Based Models Items (id 60985168): https://nwn.wiki/spaces/NWN1/pages/60985168/Part-Based+Models+Items
- parts_robe.2da (id 53669962): https://nwn.wiki/spaces/NWN1/pages/53669962/parts_robe.2da
- Phenotype Quick Overview (id 53670835): https://nwn.wiki/spaces/NWN1/pages/53670835/Phenotype+Quick+Overview
- phenotype.2da (id 38176344): https://nwn.wiki/spaces/NWN1/pages/38176344/phenotype.2da
- Placeable (id 26738918): https://nwn.wiki/spaces/NWN1/pages/26738918/Placeable
- Placeable Walkmesh PWK (id 60985376): https://nwn.wiki/spaces/NWN1/pages/60985376/Placeable+Walkmesh+PWK
- Placeables and the Toolset (id 26738852): https://nwn.wiki/spaces/NWN1/pages/26738852/Placeables+and+the+Toolset
- placeables.2da (id 38175210): https://nwn.wiki/spaces/NWN1/pages/38175210/placeables.2da
- Player and Creature Accessories as Head-Aligned VFX (id 38176583): https://nwn.wiki/spaces/NWN1/pages/38176583/Player+and+Creature+Accessories+as+Head-Aligned+VFX
- Player and Creature Accessories Using ProgFX Type 12 (id 38176565): https://nwn.wiki/spaces/NWN1/pages/38176565/Player+and+Creature+Accessories+Using+ProgFX+Type+12
- PLT (id 14618045): https://nwn.wiki/spaces/NWN1/pages/14618045/PLT
- portraits.2da (id 38174997): https://nwn.wiki/spaces/NWN1/pages/38174997/portraits.2da
- progfx.2da (id 38175071): https://nwn.wiki/spaces/NWN1/pages/38175071/progfx.2da
- Render Distance with Fog and Skyboxes (id 38175000): https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes
- Resizing Models Easily with Clean Models EE (id 60984371): https://nwn.wiki/spaces/NWN1/pages/60984371/Resizing+Models+Easily+with+Clean+Models+EE
- Resource Limits (id 26738887): https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits
- Shader Engine Support (id 14614573): https://nwn.wiki/spaces/NWN1/pages/14614573/Shader+Engine+Support
- skyboxes.2da (id 53670635): https://nwn.wiki/spaces/NWN1/pages/53670635/skyboxes.2da
- Standard material inputs (id 38175898): https://nwn.wiki/spaces/NWN1/pages/38175898/Standard+material+inputs
- Textures (id 38174958): https://nwn.wiki/spaces/NWN1/pages/38174958/Textures
- TGA (id 53672779): https://nwn.wiki/spaces/NWN1/pages/53672779/TGA
- Tile Path Nodes (id 139689996): https://nwn.wiki/spaces/NWN1/pages/139689996/Tile+Path+Nodes
- Tileset Construction Tutorial (id 72417345): https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial
- Tilesets (id 38175063): https://nwn.wiki/spaces/NWN1/pages/38175063/Tilesets
- TXI (id 38174929): https://nwn.wiki/spaces/NWN1/pages/38174929/TXI
- VFX Casting Models (id 82247687): https://nwn.wiki/spaces/NWN1/pages/82247687/VFX+Casting+Models
- VFX Effect Caching (id 53672866): https://nwn.wiki/spaces/NWN1/pages/53672866/VFX+Effect+Caching
- Visual Effect Editing (id 136839202): https://nwn.wiki/spaces/NWN1/pages/136839202/Visual+Effect+Editing
- Visual Effects (id 26738868): https://nwn.wiki/spaces/NWN1/pages/26738868/Visual+Effects
- visualeffects.2da (id 38175069): https://nwn.wiki/spaces/NWN1/pages/38175069/visualeffects.2da
- Walkmesh Notes (id 179077218): https://nwn.wiki/spaces/NWN1/pages/179077218/Walkmesh+Notes
- Working with Transparency (id 68387028): https://nwn.wiki/spaces/NWN1/pages/68387028/Working+with+Transparency

- NeverBlender - Import (id 38176517): https://nwn.wiki/spaces/NWN1/pages/38176517/NeverBlender+-+Import
- Clean Models: EE (id 64815203): https://nwn.wiki/spaces/NWN1/pages/64815203/Clean+Models+EE
- File Format Specification (BioWare) (id 327727): https://nwn.wiki/spaces/NWN1/pages/327727/File+Format+Specification+Bioware

### H.2 Other nwn.wiki links / attachments
- https://nwn.wiki/download/attachments/327727/Bioware_Aurora_DoorPlaceableGFF.pdf

### H.3 Non-wiki sources
- Game install `$HOME/.local/share/Steam/steamapps/common/Neverwinter Nights`: `lang/en/docs/CHANGELOG.md`, `lang/en/docs/patchnotes/*.md`, `lang/en/docs/Neverwinter Nights Enhanced Edition (v74..v79).txt` (v74 is cp1252), `data/*.bif` via `nwn_base.key`; stock shaders in `base_shaders.bif`/`ovr.bif`; `bin/win32/nwtoolset.exe` and `bin/linux-x86/nwmain-linux` string tables.
- nwnmdlcomp source (Torlack; niv/nwn-tools @ d979787) vendored at Neverblender's vendored copy (`build/third_party/nwn-tools/_NwnLib/` in its build tree) (`NwnModel.h`, `NwnMdlGeometry.h`, `NwnMdlNodes.h`, `NwnMdlR2A.cpp`, `NwnMdlDecomp.cpp`, `_NmcLib/NmcController.cpp`, `NmcAttribute.cpp`, `NmcMesh.cpp`, `NmcGeometry.cpp`).
- Neverblender add-on `neverblender/` in its repository (`nvb_mdl.py`, `nvb_node.py`, `nvb_anim.py`, `nvb_animnode.py`, `nvb_def.py`, `nvb_utils.py`, `nvb_material.py`); it has no binary reader (shells out to nwnmdlcomp).
- BioWare Aurora GFF PDFs (Door/Placeable, Area) via nwn.wiki attachments (text copies `$SP/research/door.txt`, `$SP/research/bioware_are.txt`).
- NWN Lexicon via local `nwscript-docs` (GetItemAppearance, SetCreatureBodyPart); nwscript.nss constants.
- nwn.py (`~/.local/opt/neverwinter/venv`) resman used for all scans; neverwinter.nim CLI (`nwn_resman_grep/extract`, `nwn_twoda`).

## I. Open questions and wiki-vs-data discrepancies

### I.1 Textures/formats (this section's author)
1. Is `graphics.experimental.generate-plt-with-shaders` on by default in 37-17? (Irrelevant for output, same math, but CPU path may differ in filtering/mip generation.)
2. Exact GL blend functions for TXI `blending additive` / `punchthrough` / `lighten` and emitter `blend` modes (engine side, not in shaders). Assume additive = src*alpha + dst (or ONE,ONE), punchthrough = alpha test 0.5, lighten = max-ish/additive.
3. 789 TGAs are 32-bpp with descriptor alpha bits = 0: does the engine use the 4th byte as alpha? (likely yes; check a sample visually).
4. Cube map face order for `cube 1` + `filerange 6` (`name0..name5`); and single-image cube maps (DDS cube?).
5. Standard DDS: BC7 / DX10-header support not documented; KTX position in the DDS/TGA location interleave unknown.
6. Where `ENVMAP default` comes from for an area (tileset .set `EnvMap`? area setting) and which env map a toolset preview should use (the toolset uses `Chrome1` per strings [TS]).
7. 8193.37 `vPos` in world space - irrelevant unless Moonglow reuses stock shaders verbatim (it could: shaders are plain GLSL with engine uniforms; would need to emulate the uniform set).
8. How the engine treats `texture1..3` on MDL meshes vs MTR texture1..5 when both are set (MTR wins per wiki); and where materialname lives in EE binaries (B.20: maybe texture slot 3).

### I.2 Binary MDL (from B)

- materialname/renderhint storage in EE binaries (texture3 slot?)
- Semantics of the EE skin 0x3B0 variant
- Name of light controller 144
- Bezier key data layout
- Engine animation-binding rule (name vs part number)
- Binary values for `gui`/`item`/`other` classification (probably 0)
- Emitter controller IDs for lightningsubdiv/opacity etc.
- Whether current EE compiles store tangents

### I.3 Animations / nodes (from C)
#### Wiki errors found against game data
- The tile loop names are `animloop01..03`, not `animloop1`: the Tileset Construction Tutorial is wrong.
- DWK dummies are `NN_DWK_dp_closed_01` (with an underscore), not `DWK_dp_closed01`: Model Special Nodes and DWK are wrong. The engine likely matches `_DWK_dp_` + state, and `_02` for the second point.
- `tilefade 4` is used 1,877 times in base tiles but is undocumented.
- Tile source lights are not always dummies: 77 `sl` nodes are `light` nodes.
- Base main lights carry radius 14/5 (or 10) and shadow 0, while the wiki claims the engine overrides them to 10/5 with shadow 1.
- Animation events `cast`, `parry` and `draw_arrow` exist in the data, although the wiki calls them unused.
- Placeable state animation names are `open`/`close`/`on`/`off`/`dead`/`default` (PDF-SO). The wiki table suggests `open`/`close` are transitions, but in the data they are 1-frame poses and the transitions are `close2open`/`open2close`.

#### Open questions (need a game or toolset experiment)
1. Exact **animscale** application: position keys only? Root only or all nodes? Only for inherited animations?
2. Whether the toolset area view plays `day`/`night` tile states and animloops **animated in real time**, or only the first frame. The TS strings show that it references `AnimLoop01-03`, `tiledefault`, fx_flame01 and the sun `controlpart`.
3. Meaning of `tilefade` 2 ("base") and 4. Meaning of the `Tile1` animation (tni02). Use of `default` on 14 tiles.
4. Whether ml1/ml2 radius and shadow are really overridden at runtime; the wiki values are unverified.
5. The exact precedence and blending of overlapping animroots (overlay on base), and how `transtime` blends are weighted (linear?).
6. The Tile_SrcLight 0 behaviour: off, with no fx_flame01 spawned [PDF-ARE] [inf]. The exact colour mapping of values 1–15 lives only in fx_flame01's animations.
7. Whether the toolset honours `EMOTE_ANIMATIONS_USE_MDL_TIMINGS` or VisualTransform `AnimationSpeed` in previews (the TS reads the AnimationSpeed field).
8. Door `trans` playback conditions in the toolset (all doors or only transition doors).

### I.4 Creatures / items (from D)
#### Wiki vs data discrepancies found
- `belt_g` parent: data `belt_g1`→`rootdummy`; wiki says pelvis_g.
- Composite naming example `swwss_t_001` in [W:38174935] is wrong: `wswss_t_011` (class + `_b/_m/_t_` + NNN).
- Palette file names `pal_armor01/02` (wiki: `pal_armor_01/02`).
- boots/potions/keys are ModelType 2 (icon-only composites) — easy to miss.
- Toolset parts list sorting: "descending" in [W:38174875] actually means 0.00 first (ascending).

#### Open questions
1. Exact per-slot override rule when armor part number is 0 (does creature BodyPart show through, e.g. shoulders/belt?).
2. Colours for metal/cloth/leather layers on a naked creature / head with no armor (index 0?).
3. Helmet attach node: toolset strings pair helmet with `head_g`; helm_001 geometry centred on origin (z ±0.16) whereas heads start at head_g — confirm visually (head_g vs `head`).
4. Armor icon layer draw order and gender used by the toolset armor icon/preview.
5. GFF type of ArmorPart_*/BodyPart_* when > 255 (BYTE vs WORD) and exact `APart_%d_Col_%d` field type; `HiddenWhenEquipped` GFF field name (not found in strings).
6. PLT header dword @8 (10 vs 6) meaning.
7. Toolset handling of `default` ENVMAP (Chrome1 inference) and of placeholder 2DA rows (`USER`, `OS_RESERVED`).
8. CanRotateIcon semantics (wiki text lost in mirror).
9. Door model selection rule (Appearance vs GenericType/_New) — confirm with the BioWare Door GFF spec.
