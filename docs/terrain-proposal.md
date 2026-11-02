---
type: Proposal
title: 'Sculpted terrain in Moonglow: a design'
description: A builder's free height field sculpted over an area's tiles inside Moonglow, with NWN Mapper as the measure - what a builder sculpts and paints (heights, texture layers, the walkmesh's surfaces; placeables kept as objects, the game's own lighting), how it becomes what the game reads (a model and walkmesh per tile, a tileset of the area's own, a terrain shader in a hak, the ARE retiled), the game's limits with numbers, where it lives (a mode of the area view, not a plugin), what stays out, six milestones with sizes, the risks, the readings to take in the engine and client before building, and the owner's questions.
tags: [proposal, terrain, tilesets, areas, walkmesh, rendering, haks]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-02T11:49:07Z }
sources:
  - id: owner
    resource: human:august (the deferred sprint's large entry 9, 2026-11-02)
    title: 'Sculpted terrain: design first, then build'
  - id: deferred
    resource: deferred.md
    title: 'Deferred: "A builder''s terrain tool (NWN Mapper) as the measure"'
  - id: mapper
    resource: https://github.com/Dafena/nwn_mapper_ffeom
    title: NWN Mapper FFEOM (Godot 4.7; README, nwn_area.gd, nwn_terrain.gd, nwn_exporter.gd, main.gd, export_shaders/; read 2026-11-02, no license file)
  - id: tileset-import
    resource: plugins/examples/tileset-import/main.luau
    title: The plugin that puts a Mapper export into a hak and makes an area of it
  - id: tilesets
    resource: research/notes_tilesets.md
    title: The tileset system (SET, ARE tiles, walkmeshes, path nodes, minimaps)
  - id: border
    resource: research/notes_tile_border.md
    title: What the client draws past an area's edge
  - id: shaders
    resource: research/notes_shaders.md
    title: The game's shaders (texture slots, vertex attributes)
  - id: frames
    resource: research/notes_frames.md
    title: Frame times of large areas
  - id: wiki-limits
    resource: https://nwn.wiki/spaces/NWN1/pages/26738887/Resource+Limits
    title: 'nwn.wiki: Resource Limits (with the local correction on 2 GB haks)'
  - id: wiki-hak
    resource: https://nwn.wiki/spaces/NWN1/pages/53670235/HAK
    title: 'nwn.wiki: HAK (16-character names, NWSync''s 15 MB a file)'
  - id: wiki-render
    resource: https://nwn.wiki/spaces/NWN1/pages/38175000/Render+Distance+with+Fog+and+Skyboxes
    title: 'nwn.wiki: Render Distance (tiles drawn to the fog clip distance)'
  - id: wiki-tutorial
    resource: https://nwn.wiki/spaces/NWN1/pages/72417345/Tileset+Construction+Tutorial
    title: 'nwn.wiki: Tileset Construction Tutorial (walkmesh rules)'
  - id: wiki-walkmesh
    resource: https://nwn.wiki/spaces/NWN1/pages/179077218/Walkmesh+Notes
    title: 'nwn.wiki: Walkmesh Notes'
  - id: wiki-tilesets
    resource: https://nwn.wiki/spaces/NWN1/pages/38175063/Tilesets
    title: 'nwn.wiki: Tilesets (the tileset limit, 16-character names)'
  - id: measured
    resource: the install (89.8193.37) read with mg cat, 2026-11-02
    title: 'Stock tiles measured: ttr01_a01_01, ttr01_a01_02, ttr01_a07_01, trm02_a01_01, tcm02_a01_01, ttf02_a01_01 and ttr01_grass02'
  - id: lights
    resource: lights-proposal.md
    title: 'Lights placed on the fly (way B: a generated hak, off by default)'
  - id: inventory
    resource: crates/mg-ui/src/terrain_mode.rs, crates/mg-area/src/{walk,scene,pick}.rs, crates/mg-render/src/{renderer,model,scene}.rs, crates/mg-mdl/src/{compile,binary_write,walkmesh}.rs, crates/mg-set/src/edit.rs, crates/mg-image/src/lib.rs, crates/mg-edit/src/lib.rs (read 2026-11-02 at 12581870)
    title: What Moonglow has to build on
---

# Sculpted terrain in Moonglow: a design

NWN's ground is made of tiles: 10 m squares chosen from a tileset, each a
model made by hand, stepping up and down by the tileset's `Transition`. A
builder who wants a hillside of their own has to make a tileset. The
builder dafena's **NWN Mapper** does that for them. It sculpts a free height
field, paints texture layers and a walkmesh, bakes placeables and a
lightmap in, and writes the result out as a tileset's group: a model and
walkmesh per tile, textures, materials and a patched `.set`.[^mapper] A
Moonglow plugin already takes such an export into a hak and makes an area
of it.[^tileset-import] What the tool itself does is out of a plugin's
reach: a plugin has no view to draw in and no brush.[^deferred]

The owner's word is design first, then build.[^owner] This is the design.
Nothing of it is built. Each decision below gives its evidence and a
recommendation. The [questions for the owner](#the-owners-questions) are
at the end.

**In short.** A builder turns on sculpting for an area. From then on its
ground is a height field at 0.625 m (16 quads a tile), painted with up to
five texture layers and the game's surface materials. Moonglow draws it
live in the area view. Build Terrain writes it into a hak the builder
names:

- a model and a walkmesh per tile, each holding only its own 10 m square;
- a tileset of the area's own (a copy of the tileset it started from, plus
  its sculpted tiles in a group);
- the layers blended in the game by a terrain shader Moonglow writes.

Placeables stay objects and the game lights the ground as it lights any
tile. The first milestone is small and has no brush: a heightmap picture
in, a walkable hilly area in the game out.

## NWN Mapper, as it is

NWN Mapper FFEOM is a Godot 4.7 application. Its repository was created on
2026-09-25. It has no license file, so its code can't be taken into
Moonglow (GPL-3.0): only what it does is described here, read from its
README and sources on 2026-11-02.[^mapper] The repository's owner, Dafena,
appears to be the builder dafena whose request led to the
[lights proposal](lights-proposal.md).[^lights]

| What | NWN Mapper |
|---|---|
| Area | 2 to 20 tiles a side (4 × 4 by default) |
| Height field | 33 samples a tile edge (32 quads, 0.3125 m), float; 1.0 stored is 15 m |
| Brushes | raise, lower, smooth, flatten (to the height under the pointer), set height, stamp (an image); smoothstep falloff by radius in meters |
| Texture layers | 9, weights at 64 texels a tile (0.156 m) |
| Walkmesh | 16 quads a tile (512 faces), its own height field (it can be unlocked from the ground and sculpted apart); a surface material a texel at 64 a tile; faces split along material edges |
| Free meshes | the ground's and the walkmesh's vertices can be edited by hand |
| Placeables | from the game's placeables.2da, a `.mdl` or a `.glb`; **baked into the tiles** as meshes, their `.pwk` added to the walkmesh as non-walkable faces |
| Lights | the game's lights parsed; local lights dynamic, baked or both |
| Texturing in the game | **baked**: the nine layers blended into one albedo (and normal, specular, roughness) a block of tiles, the block as large as fits 4096 px |
| Lightmap | baked a block (512 px on the long side by default), multiplied in by its own `mzlm_vs`/`mzlm_fs` shader pair |
| Output | loose files: ASCII `.mdl` and `.wok` a tile, TGA textures, `.mtr`s, a 32 px minimap a tile, a copy of the chosen `.set` with tiles and a group appended, and a palette. No hak, no `.are` or `.git`: "pack the hak yourself" |
| Its own files | an area saved as a Godot `.res` resource |

Four things in that table matter for the design:

- **Render geometry is packed across tile files.** A block's mesh is
  re-centered on one tile and written into that tile's model, filling it
  to 40 MB of text before going on to the next. The other tiles hold only
  their walkmesh and lights. The game draws a tile out to the fog clip
  distance (45 m by default) and no farther.[^wiki-render] A block mesh
  that lives in one tile's model would then appear and vanish with that
  one tile, whatever part of the block is in view. That is
  [reading R2](#readings-to-take-before-building).
- **Every sculpted tile declares Default terrain at height 0** on all four
  corners, and path node `A` (one region, all four exits joined). The
  game's long-distance pathing would think a cliff inside a tile can be
  walked across.[^tilesets]
- **The `.set` is a patched copy of the one chosen,** under the same
  name. Put into a hak as `ttr01.set`, it would replace the game's Rural
  tileset for every area of every module that uses the hak.
- **Its lightmap shader is in the repository now** (`export_shaders/`,
  stock lighting times the lightmap, fog last). Moonglow's notes said
  "NWN Mapper ships none of `mzlm`'s"
  (`crates/mg-render/src/renderer.rs`, [the shaders
  proposal](shaders-proposal.md)). With its `.shd` files in a hak and the
  Shaders switch on, Moonglow would already draw an export with Mapper's
  own program. Whether the exporter copies them out with the rest was not
  read.

## 1. What a builder sculpts

### A height field over the area's tiles

A sculpted area holds one height field over its whole extent, sampled on a
regular grid. Tiles are cut out of it only when the game's files are
written, so a brush never sees a tile edge.

**The resolution** decides the triangles, the hak's size and the frame
time. Stock tiles, for comparison: ttr01_a01_01 (Rural grass) has 355
faces in 20-odd meshes, ttf02_a01_01 (a forest) 990.[^measured]

| Quads a tile | Spacing | Triangles a tile | 20 × 20 area | 32 × 32 area | Samples, 32 × 32 (f32) |
|---|---|---|---|---|---|
| 8 | 1.25 m | 128 | 51 k | 131 k | 66 k (0.26 MB) |
| **16** | **0.625 m** | **512** | **205 k** | **524 k** | **263 k (1.05 MB)** |
| 32 (Mapper) | 0.3125 m | 2,048 | 819 k | 2.1 M | 1.05 M (4.2 MB) |

**Recommendation:** 16 quads a tile, the area's choice of 8 or 32 kept
open in the format. At 16 a sculpted tile has about the triangles of a
stock tile, which the game and Moonglow already draw in the thousands.
Sharper features than 0.625 m (a wall, a ledge) belong to placeables.

Heights are meters, stored as 32-bit floats. They may go below the area's
zero (a pit, a riverbed) within what the engine accepts (R6).

### Brushes

Raise, lower, smooth, flatten and noise, as the owner listed, plus set
height (flatten to a typed height). Each has a radius in meters, a
strength and a falloff (smoothstep, as Mapper and most terrain tools use).
Flatten takes the height under the pointer when the stroke starts. Noise
adds seeded fractal noise (a few octaves, a scale in meters), so the same
seed gives the same ground. Two things that come from outside:

- **Heightmap import and export** (a 16-bit grayscale PNG and a height
  range). Builders who use World Machine, Gaea or L3DT, and Mapper if it
  writes one, come in this way. It is also the first milestone's only
  input.
- **Start from the area's tiles:** the field sampled from the walkmeshes
  of the tiles the area has (`mg_area::walk::Ground::height`). An
  existing area is converted with its shape kept, and a Mapper export
  made into an area by the plugin can be taken back into a height field
  the same way.

Mapper's free-vertex editing and stamp-from-image are left out (§4).

### Texture layers

The decision here is where the layers are blended: baked into textures
when the files are written (Mapper's way), or in the game, by a shader
that blends tiled layer textures by a weight map. The game's own ground
sets the sharpness to match. ttr01's grass is a 512 × 512 texture repeated
twice across a tile: about 100 pixels a meter.[^measured]

| | Baked albedo (Mapper) | Blended in the game |
|---|---|---|
| Texture data a tile | 1024² (stock sharpness): ≈ 700 KB in BC1 with mips; 256²: ≈ 43 KB, 4× blurrier than the ground beside it | a weight map, 32² RGBA: 4 KB; the layer textures are the game's or a hak's, shared, often already installed |
| 20 × 20 / 32 × 32 area | ≈ 280 / 700 MB at 1024²; ≈ 17 / 44 MB at 256² (BC1). As 24-bit TGA, which is all Mapper writes, a 4096² block is 48 MB, over NWSync's 15 MB a file[^wiki-hak] | ≈ 1.6 / 4 MB of weights |
| Sharpness | fixed when baked; blurred under the camera | the layers' own, as sharp as stock ground |
| What the hak needs | textures, MTRs | MTRs, the weight maps, one `.shd` pair |
| In Moonglow | any path | the game-shader path draws it as it draws The Domains of Dread's 37 shaders; the WGSL fallback needs a port (its `vertexalpha` port already blends by a mask) |
| Risk | download size; a BC1 encoder Moonglow lacks | a custom shader every player's client compiles (a failure draws magenta); a change to the engine's includes |

**Recommendation:** blend in the game, with a terrain shader Moonglow
writes. Its text is Moonglow's own. It may do what Mapper's does
(including the stock `inc_standard` so the game's lighting, fog and
keyholing stay the game's), but it is not a copy: Mapper's repository has
no license. A bake to one texture stays a later option (M6), for servers
that allow no custom shaders, and the minimaps are a bake either way.

**How many layers** is set by the texture slots a custom shader can read:
an MTR binds `texture0` to `texture10` for it.[^shaders] With a normal map
each:

| Layers | Slots |
|---|---|
| base + 4 painted, albedo and normal each, one RGBA weight map | 5 + 5 + 1 = **11** |
| 4 painted, no base | 4 + 4 + 1 = 9 |
| 8 painted, albedo only, two weight maps | 8 + 2 = 10 |
| 9 (Mapper's) with normals | 19: does not fit |

**Recommendation: a base layer and four painted over it,** each with its
albedo and normal map. The base shows where the four weights leave room,
so a new area is all base. More layers in an area, with each block of
tiles choosing its own five, is possible later, but block edges would
show where two blocks' sets differ. Mapper's nine fit only because it
bakes. Moonglow's renderer binds slots 0 to 7 today and leaves 8 to 15
white, so it needs 8 to 10 bound (R11 confirms the client binds them).

### A walkmesh painted with the game's surface materials

The walkmesh is cut from the same height field at **8 quads a tile**
(128 faces, every second sample). Its faces carry the material painted
under them: any row of `surfacemat.2da` (Dirt, Grass, Stone, Water,
Nonwalk…; 64 rows since 1.84, hak-overridable since 1.85).[^tilesets]
Painting is per walkmesh cell (1.25 m, two faces), with the same brush.
A switch marks faces steeper than an angle as non-walkable.

Why coarser than the ground:

- The wiki's walkmesh rules ask for few faces ("100 is a good limit") and
  at most eight faces at a vertex.[^wiki-tutorial] A regular grid gives
  six.
- Stock walkmeshes are small: 2.6 to 8.6 KB of text for the six tiles
  measured.[^measured]
- Mapper's 512 a tile is four times 128. How the server's pathing copes
  with either is R5.

The walkmesh's vertices lie on the ground (every second sample). Between
them the ground can stand a few centimeters off the walkmesh, where feet
would float or sink; R4 measures it.

Other rules the generator keeps, each a test:[^wiki-walkmesh]

- edge vertices exactly at ±5.0, so neighbors meet (an unsnapped edge is
  a gap);
- no vertical faces;
- no duplicate or degenerate faces, no unused vertices;
- one `aabb` node under the tile's base.

Grass needs nothing more: the game grows it on faces of material 3 with
the tileset's `[GRASS]` settings, and Moonglow's view already grows it on
walkmesh faces the same way (`mg_area::grass`).

### Placeables: kept as objects

Mapper bakes placeables into the tiles. **Recommendation: keep them as
objects in the area's `.git`**, placed with Moonglow's tools as now, for
these reasons:

- A baked placeable can't be moved, scripted, made usable or replaced
  without rebuilding the tiles.
- Static placeables already block with their `.pwk` in the engine
  (`engine_placeable_walkmesh.rs`).
- Baking adds texture and model bytes to every tile it touches.

What sculpting adds for objects:

- **Keep objects on the ground:** objects whose feet were on the ground a
  stroke changed move with it, in the same step of Undo.
- Moonglow's Drop to Ground (`drop_to_ground`) works as now.

### Lighting: the game's own

Mapper bakes a lightmap and multiplies it into everything, sun and moon
and dynamic lights alike. The game's own lighting already does sun and
moon (with day and night), tile main lights, source lights and shadows,
and Moonglow matches the client's.

**Recommendation: the game's lighting, no bake.** Each sculpted tile gets
`ml1` and `ml2` light nodes where the source tileset's flat tile has them,
raised to the ground's height. The area's lighting scheme and Tile
Properties then light it as they light a stock tile (R8).

Later (M6), an ambient-occlusion bake can go into the tiles' vertex colors
(MDL `colors`, which reach a custom shader as `vCustomColor` and need no
texture slot).[^shaders] Occlusion does not depend on the time of day,
which a sun-baked lightmap would get wrong at night.

## 2. How it becomes what the game reads

### What Build Terrain writes

For an area of W × H tiles sculpted from source tileset S, into the hak
the builder named:

| File | Count | What |
|---|---|---|
| `<set>.set` | 1 | the area's own tileset: S's `.set` copied whole (its tiles keep their numbers), then one `[TILEn]` a sculpted tile, then one `[GROUPn]` holding them (§ below) |
| `<set>_edge.2da` | 1 | S's edge table copied, so the client draws a border (see [Tile edges](#tile-edges-and-the-area-border)) |
| `<set>palstd.itp` | 1 | the palette (`mg_module::tileset::palette`), so Aurora and Moonglow can paint the area's stock tiles |
| `<set>_NNNN.mdl` | W·H | binary model: one trimesh (the ground, 16 quads), its `aabb` walk node, `ml1`/`ml2` |
| `<set>_NNNN.wok` | W·H | the walkmesh (8 quads, materials, AABB tree) |
| `mi_<set>_NNNN.tga` | W·H | 32 × 32 minimap picture |
| `<set>_bNN.mtr`, `<set>_wNN.tga` | one a block | a block's material (the five layers' textures, its weight map, the shader) and its weight map, 32 texels a tile |
| `<set>_ter.shd` ×2 | 1 pair | Moonglow's terrain shader (vertex and fragment), the same in every terrain hak of a module |

Names fit the game's 16 characters[^wiki-hak]: a `<set>` of up to 7
characters gives `mi_` + 7 + `_` + 4 digits = 15. The builder chooses
`<set>`; the default is made from the builder code and the area.

**Per-tile models, not Mapper's packing.** Each model holds exactly its
own square, centered as the game expects (origin at the tile's center,
corners at ±5 m).[^tilesets] The tile is then culled, faded and shadowed
as one.

**Normals from the whole field.** `mg-mdl`'s compiler makes normals from
each mesh's own faces (`compile.rs`). Two per-tile meshes would then shade
differently along their shared edge. Moonglow builds each tile's `Model`
in memory, with normals and tangents taken from the whole field, and
writes the binary model with `binary_write::write` directly.

**Blocks.** Tiles are grouped in blocks of 4 × 4 that share a material and
one weight map (128 × 128 RGBA, 64 KB). A 32 × 32 area has 64 blocks. The
area view draws the same blocks as its live meshes, so the view and the
game read the same textures.

### How the area names its tiles

- The ARE's `Tileset` becomes `<set>`.
- Each cell's `Tile_ID` is its sculpted tile's index in `<set>.set`, with
  `Tile_Orientation` 0 and `Tile_Height` 0. Heights are in the model;
  Transition doesn't apply to them.
- The lights are the scheme's colors, chosen as the painter chooses them
  for new tiles.
- The `.git` is untouched, but for objects that Keep on Ground moved.

The `[GROUPn]` of the area's tiles is the area's size. Tiles in a group
are never chosen by terrain painting,[^tilesets] so painting stock tiles
elsewhere can't put a sculpted tile in by chance. Its `Tile0` names the
group in the palette.

**The corners each sculpted tile declares** are the lattice's, not
geometry. Inside a wholly sculpted area all corners are S's Default at
height 0, as Mapper writes them, which keeps Aurora's and Moonglow's
checks content. Where a sculpted cell borders stock tiles (M5), its
corners copy the stock neighbor's, so the lattice stays consistent.

**Path nodes.** Path nodes are computed from the walkmesh: the type among
A to x whose regions and exits match which of the tile's edges its
walkable faces join. A tile cut by a cliff gets the type that keeps its
two sides apart. Visibility nodes stay unset, so the game takes the path
node for them.[^tilesets]

### Unique tiles or shared ones

A sculpted tile is unique by nature. Two tiles whose bytes are equal (a
flat plain, one material, one layer) share one model by a hash of their
content (M6). An open sea floor or a plain then costs one tile, not a
hundred.

**One tileset per area,** not one per module and not S patched under its
own name:

- **Patching S by name** replaces a base or a hak's tileset for every area
  of every module that loads the hak.
- **One tileset for the module** grows with every area. Tile numbers can
  never be reused: the ARE stores indices, so renumbering breaks every
  area.[^tilesets] Rebuilding one area would leave the others' tiles
  where they are and its own old ones as holes.
- **One for the area** is rewritten whole when the area is rebuilt, and
  only that area names it.

The limit of 100 tilesets in use in a module was lifted in
89.8193.37-13[^tilesets] (the wiki's Tilesets page still gives
100[^wiki-tilesets]). A module with many sculpted areas then needs a
server and clients at that version or later (R12).

### The game's limits

| Limit | Value | Source | For a sculpted area |
|---|---|---|---|
| Area size | 2 to 32 tiles a side in the wizard; whether the engine takes more is open | [^tilesets] | at most 1,024 tiles |
| Resource names | 16 characters; 17 can crash the game | [^wiki-hak] | the naming above |
| Hak size | 2 GB: a resource starting past 2³¹ bytes is missing to the game, and a lower hak's copy doesn't take over | [^wiki-limits] | far off (below) |
| Resources a hak | no limit since 1.80.8193.14 (16,384 before) | [^wiki-limits] | 3 a tile + a few: about 3,100 for 32 × 32 |
| A file in NWSync | 15 MB recommended | [^wiki-hak] | the largest file is a weight map (64 KB) |
| Tilesets in use | none since 89.8193.37-13 (100 before) | [^tilesets] | one an area |
| Tiles a tileset | none documented; tcm02 has 1,872 | [^tilesets] | S's count + up to 1,024 |
| Faces at a walkmesh vertex | 8 | [^wiki-tutorial] | 6 on the grid |
| Texture slots for a custom shader | `texture0`–`texture10` | [^shaders] | 11, all used |
| Tile draw distance | the fog clip distance (45 m usual), +90 m with a skybox | [^wiki-render] | per tile |

**Hak size**, estimated (M1 measures it). Stock binary tile models are 36
to 125 KB.[^measured] A sculpted tile at 16 quads has one 289-vertex
ground mesh and its walk node, about 50 KB. Its `.wok` (81 vertices, 128
faces, a 255-node tree) is about 20 KB as text. Its minimap is 3 KB and
its share of a weight map 4 KB. That makes about **75 KB a tile**:

| Area | Tiles | Hak (uncompressed) |
|---|---|---|
| 8 × 8 | 64 | ≈ 5 MB |
| 20 × 20 | 400 | ≈ 30 MB |
| 32 × 32 | 1,024 | ≈ 77 MB |

NWSync stores files compressed. A smooth field and its walkmesh compress
well, but by how much was not measured. At 8 quads a tile the model
halves; at 32 (Mapper's) it is four times the size.

### Tile edges and the area border

- **Between sculpted tiles** the edges are the same samples of one field,
  so they meet exactly: positions, normals (from the whole field) and
  walkmesh vertices on ±5.0.
- **Past the area's edge** the client draws the tileset's edge tiles five
  deep, darkened. It looks each one up in `<tileset>_edge.2da` by the
  border cell's corner terrains and crosser.[^border] Those corners are
  Default at 0, so the edge tiles stand at height 0, while the sculpted
  edge may be at 8 m. The result is a step or a gap at the border.

Options:

1. Pin the field's outer ring to height 0 (a slope down to the border
   over a width the builder sets).
2. Have the builder hide the border (`SetAreaTileBorderDisabled`, a
   script, read at the client's next load of the area).
3. Generate edge tiles of our own. Edge tiles are looked up by corner
   terrain, not by place, so they can't follow a sculpted edge without
   one terrain name per border position.

**Recommendation:** option 1 by default, a **Border** setting in the
sculpt panel (Meet the tile border, on; or Leave the edge as sculpted),
and the manual names `SetAreaTileBorderDisabled` for the second. R7 looks
at both.

### The minimap

Each tile's `ImageMap2D` picture (32 × 32, as EE's tilesets and Mapper
use) is computed without the GPU. The layers' mean colors are blended by
the weights and shaded by the slope toward the sun, so `mg` can write
them headless. It is not a render from above: `tileset_view`'s
`render_minimaps` is GUI-only, and a hillshade reads better on a map than
a lit render. `client_minimap.rs` already compares the client's map
quarter by quarter (R10).

### Readings to take before building

The game client is read only through `tools/nwclient/run-client.sh` on the
off-screen display, and the server through `mg_testkit::engine`. None is
taken in this package. Most are taken in M1 on its own test area: a 4 × 4
hill and a pit written by the pipeline.

| # | Reading | How | Decides |
|---|---|---|---|
| R1 | Seams between per-tile meshes: no cracks or lighting steps at tile edges, close and at a distance | client screenshot, `client_render` reading beside Moonglow's view | normals from the whole field |
| R2 | A tile's geometry past its square (Mapper's packing) is culled with its own tile at the fog clip distance | client screenshots walking away from a two-tile mesh | per-tile models; a note for Mapper's exports |
| R3 | `GetGroundHeight` across the area equals the field, and creatures stand on it | engine test (`engine_terrain.rs` style), points on a grid | walkmesh resolution |
| R4 | How far feet float or sink between walkmesh vertices on a slope (8 quads walk under 16 quads ground) | client screenshot of a creature on a slope | walkmesh resolution |
| R5 | Pathing over 128 and 512 faces a tile: `ActionMoveToLocation` across 32 tiles, its time, and the computed path nodes respected at a cliff | engine test (`engine_placeable_walkmesh.rs` style) | walkmesh resolution, path nodes |
| R6 | Walkable faces below 0 and far above (−10 m, +50 m): heights, camera, creatures | engine test and client screenshot | height range |
| R7 | The border at a sculpted edge, pinned and not; and with `SetAreaTileBorderDisabled` | client screenshot | Border setting |
| R8 | A sculpted tile lit as a stock tile beside it (main lights, sun, moon, day and night) | client screenshot beside a stock ttr01 tile | the light nodes |
| R9 | Sun shadows from a hill onto neighboring tiles (tile shadow volumes are clipped to the tile's box since 85.8193.32) | client screenshot, Fast and High | whether hills cast across tiles |
| R10 | The minimap pictures on the client's map | `client_minimap.rs` | the minimap |
| R11 | The terrain shader compiles in the client (no magenta), reads `texture8`–`texture10`, and its weights look right up close and at fog distance | client screenshot; `mg verify`'s `shader-compile` | the layer design |
| R12 | Load time and memory of an area of 1,024 unique tiles, and of a module of several sculpted tilesets | client log, timed load; server log | area size, tileset per area |
| R13 | An area of the generated tileset (S's copy plus sculpted tiles) opens in Aurora with the hak, and Aurora saves it unchanged | Aurora oracle (`tools/aurora`) | Aurora compatibility |
| R14 | The source resource (below) in a `.mod`: the server ignores it, Aurora keeps it when it saves | engine run; Aurora oracle | where the source lives |

## 3. Where it lives in Moonglow

### A mode of the area view

Sculpting is a mode of the area view, next to the tile brushes. It is not
a plugin. The plugin API can't reach it, and it shouldn't be stretched to:

- A plugin's panel is read-only and runs within 250 ms when edits or the
  selection change; one that draws or runs on a clock is out by design
  ([plugin proposal](plugin-proposal.md#api-06-exporters-importers-wizards-and-panels-2026-11-02)).
- A brush needs the pointer every frame, a mesh rebuilt and uploaded as
  it moves, and a few megabytes of field and weights at hand. In Luau the
  meshing alone would be slow.[^deferred]

Plugins might later read and write the height field (a generator, a
noise of their own), but that is beyond this design.

**What the builder sees:**

- **Turning it on:** Area Properties › **Terrain**, "Sculpt this area…".
  It asks:
  - the hak to write into (the module's haks or a new one), with what it
    writes and that players must have it;
  - the tileset's name;
  - the source tileset (the area's own by default);
  - where to start: flat, a heightmap, or the area's tiles as they are.

  Off by default, as Options › Areas › **Sculpted terrain**, the owner's
  rule for generated haks in [lights way B](lights-proposal.md) ("to
  prevent unwittingly touching PW haks").
- **The palette's Sculpt tab** for a sculpted area:
  - brushes (Raise, Lower, Smooth, Flatten, Set Height, Noise; Paint
    Layer; Paint Surface);
  - the brush's radius, strength and falloff;
  - the five layers, each a texture chosen from the load order (with its
    normal map found by name, or chosen) and a tiling in meters;
  - the surface materials;
  - the Border setting;
  - **Build Terrain**.
- **Brush input** as the tile brushes take it (`terrain_mode::input`):
  - left drag applies the brush, right drag its opposite (lower, erase
    paint), Shift smooths;
  - a stroke is committed on release;
  - Ctrl stays the camera's, Escape puts the brush down;
  - a circle on the ground shows the radius.
- **The walkmesh overlay** shows the painted materials and the
  non-walkable faces.

**Undo.** A stroke is one command. `Edit::SetResource` replaces a whole
resource, which for a field of 5 MB would fill the undo history in a few
dozen strokes. A stroke instead records the blocks it changed, before and
after, compressed: a new edit kind in `mg-edit` for a region of an area's
terrain, applied and inverted like the others.

**Saving. The source lives in the module,** as one resource for each
sculpted area, named after the area. Its contents:

- a small header (format version, the area's size, quads a tile, the
  source tileset, `<set>`, the hak, the layers' textures);
- the field (f32), the weights (u8, 32 a tile) and the materials (u8,
  8 a tile), each compressed with zstd.

A 32 × 32 area is about 5 MB before compression. The module is the
server's and the builders'; players never download it. The NUI Creator
is the precedent: it keeps its authoring settings in the module as
`<name>.txt` marked `moonglow.nui/1`. Which resource type the game
ignores and Aurora keeps is R14.

The **hak is output**. Build Terrain writes it:

- from the area's menu or the Sculpt tab;
- and Build › Build Module offers it when an area's source is newer than
  its hak;
- rewriting only the tiles whose blocks changed.

Saving the module never writes a hak by itself.

**What Moonglow already has** (read at 12581870):[^inventory]

| Need | What exists | What's missing |
|---|---|---|
| Pointer, strokes, preview | `terrain_mode::input`, `TerrainDrag`, `preview` | sub-tile positions |
| Picking the ground | `Ray::from_screen`, `ground_at`, `Ground::hit` (every face, no tree) | a ray against the field itself (march and refine: cheap) |
| Live meshes | `GpuModel::new` uploads any in-memory model (the marker arrow does) | a block's mesh replaced in place, a block at a time |
| Drawing the layers | the game-shader path (custom shaders a material names); the WGSL fallback's `vertexalpha` port | slots 8–10 bound; the terrain shader's WGSL port |
| Undo | `mg_edit` commands, `Moonglow::apply` | the region edit |
| Models | `binary_write::write`, `ascii_write`, the compiler's `aabb_tree` | none for models |
| Walkmeshes | read (`walkmesh.rs`); `to_ascii` writes AABB trees | a `.wok` writer (no model header) |
| Materials, textures | MTR read; `write_tga` 24-bit | an MTR writer; 32-bit TGA (the weights' alpha); a BC1 encoder only for M6's bake |
| Tilesets | `mg_set::edit::SetFile` (`add_tile`, `add_group`, untouched bytes kept); `palette` | an `_edge.2da` copy (a 2DA write: exists) |
| Haks | `hak_edit::write_into`, `attach::hak_list`, `sync_haks` | none |
| Seeing a rebuilt tile | `tile_cache` and `Ground` keyed by model name | dropping the rebuilt names without a full `reload` |
| Walkmesh overlay | `walkmesh_overlay`: an egui mesh projected on the CPU, face by face, each frame | a GPU overlay: 131 k faces at 32 × 32 is too many for the CPU path |
| Engine and client readings | `mg_testkit::engine` (`GetGroundHeight`, `GetSurfaceMaterial`, saved games), `client_render`'s readings, `client_minimap` | the M1 test area |

## 4. What stays out, and why

- **Placeables baked into tiles** (Mapper does): they stop being
  editable, and the client already batches static placeables. Kept as
  objects (§1).
- **A sun-baked lightmap:** wrong at night and under the area's changing
  lighting, it duplicates what the game lights, and it costs a texture a
  block. Ambient occlusion in vertex colors (M6) gives most of the depth
  for none of that.
- **Overhangs, caves, arches, bridges:** a height field has one height a
  point. Placeables do these, as in the game's own tilesets.
- **Walkable surfaces over each other:** the engine forbids walkable
  faces above one another in a tile.[^wiki-tutorial]
- **Water planes:** the game's water is tile geometry with its own
  material and `fswater`. A water level cut into the sculpted tiles is a
  natural next design, not this one.
- **Editing vertices by hand, and walkmeshes unlocked from the ground**
  (Mapper has both): free meshes break the grid's guarantees (six faces a
  vertex, edges on ±5, normals from one field) that make the output safe.
- **Level of detail, and triplanar texturing of cliffs:** the game draws
  each tile whole and its shader would have to do both; steep faces
  stretch the layers as they do on stock cliffs.
- **Areas larger than 32 × 32:** the wizard's limit, and the engine's is
  unknown (R12).
- **Reading Mapper's own `.res`:** a Godot binary resource, readable only
  by Godot's code for its version. Its exports come in through the
  plugin, and its heights through "start from the area's tiles" or a
  heightmap PNG if Mapper writes one.
- **Sculpting from a plugin** (above).

## 5. Build order

Sizes as the deferred list uses them (S, M, L). Each milestone is usable
on its own, has its tests at the lowest tier that catches its
regressions, and its manual pages.

**M1 — From a heightmap to the game (S–M).** No brush, no layers, one
material.

- `mg terrain build MODULE AREA --heights hills.png --range 0:20
  --hak NAME [--from ttr01]`, and the same as Area › **Terrain from
  Heightmap…** in the window.
- It writes the hak: the tileset of the area's own (S copied, tiles,
  group, palette, edge table), the models and `.wok`s, and the minimaps.
- Only the tileset's ground texture with the stock shader: no custom
  shader yet, one surface material.
- It attaches the hak, retiles the area and saves the source.
- New pieces: the `.wok` writer, the per-tile meshing with whole-field
  normals, the generated `.set`, path node `A`, and invalidating rebuilt
  tile names in the view.
- Tests:
  - unit tests of the meshing: edges equal, ±5.0, six faces a vertex,
    no vertical or degenerate faces;
  - the generated `.set` read back by `mg-set` and nwn.py;
  - `mg verify` clean on the hak;
  - engine tests R3, R5 (at 128 faces) and R6;
  - client readings R1, R2, R4, R7, R8, R9, R10 and R12 (by hand on the
    off-screen display, recorded);
  - Aurora R13.

  A builder with a heightmap from any tool gets a walkable hilly area.

**M2 — The Sculpt mode (M).**

- The Sculpt tab and its brushes, live block meshes, the ray against the
  field, the region edit and its Undo.
- The source in the module (R14 first), Keep Objects on the Ground,
  heightmap import and export, and Start from the Area's Tiles.
- Build Terrain in the window, and Build Module's offer.
- UI tests: a stroke, Undo, Redo, save and reopen, Build. `screens.rs`
  pictures of the tab. `frame_perf` with a brush moving over a 32 × 32
  sculpted area.

**M3 — Surfaces (M).**

- Surface materials painted, the steepness switch, and the GPU walkmesh
  overlay.
- Path nodes computed from the walkmesh (R5 at a cliff).
- Grass on Grass faces, checked in the view and the client.
- The engine's `GetSurfaceMaterial` at painted points.

**M4 — Texture layers (M–L).**

- The base and four layers, and Moonglow's terrain shader written into the
  hak.
- The renderer binds slots 8–10; the shader is drawn by the game-shader
  path, with its WGSL port as the fallback.
- Weight maps as 32-bit TGA, and minimaps from the layers.
- R11 in the client. The view against the client on the M1 test area
  (the shaders proposal's comparison, its tolerances).
- `mg verify`'s `shader-compile` on the written pair.

**M5 — Sculpted cells in a tiled area (M).**

- Sculpting only some cells of an area, with stock tiles and groups
  around and among them (a village of ttr01's houses on a sculpted
  hillside).
- The field pinned at the stock neighbors' corner heights along the
  region's edge, and the sculpted cells declaring the lattice's corners.
- Tile painting refuses to paint over sculpted cells, except with a
  "Make Tiles Again" that drops them from the source.
- Pictures in the client where a sculpted cell meets ttr01's grass, road
  and water edges.

**M6 — Finishing (S–M).**

- Ambient occlusion in vertex colors.
- Identical tiles shared by hash.
- The hak tidied of tiles no area uses.
- The bake option, one albedo a block, for servers without custom
  shaders. It needs a BC1 encoder (the performance audit rates one M).
- Taking a Mapper export's heights back through the plugin.

M1 is the hard constraint end to end (the game's files, the engine, the
client) before any UI is built on it. If a reading there goes against the
design (R2 or R5 above all), the change is made before M2.

## 6. Risks

- **The frame budget (144 fps, 6.94 ms).**
  - Drawing: a 32 × 32 sculpted area at 16 quads is 524 k triangles in 64
    block draws in the view (the game draws 1,024 tiles). The stock area
    area092 draws 1,028 times in 0.94 ms of the GPU, and its redrawn
    frame is 4.1 ms.[^frames]
  - Strokes: a brush of 10 m radius touches up to 4 blocks (40 m each),
    about 17 k vertices to rebuild and upload (under 1 MB) a frame of the
    stroke. That should be well under a millisecond of meshing, but it was
    not measured.
  - Already over: areas with The Domains of Dread's dense placeables are
    over the budget without sculpting (13–21 ms; the deferred sprint's
    entry 10).[^frames]
  - M2's `frame_perf` case holds the brush to the budget on the stock
    machine.
- **Hak size.** About 75 KB a tile, 77 MB for a full 32 × 32 area,
  uncompressed: the size of a modest tileset hak for one area. The
  numbers are estimates until M1. The levers are coarser quads, shared
  identical tiles and pinned borders. The bake would multiply it (§1).
- **Multiplayer download.** Every player downloads the hak. A server with
  ten sculpted areas ships about the base game's Rural tileset ten times
  over. NWSync's per-file limit is no issue (the largest file is 64 KB).
  The Build Terrain dialog says the size before writing.
- **Translation into the game's look.**
  - The view and the game read the same shader and textures, and the
    shader includes the game's own lighting.
  - The view is held to the client by R1, R8 and R11, as the shaders
    proposal holds the stock programs.
  - What can still differ: the client's shadow volumes on a hill (R9),
    how weights filter at a distance, and drivers' handling of a custom
    shader. A shader that fails to compile in some client draws the
    ground magenta, which the bake option (M6) is the fallback for.
- **Aurora.** Areas of a generated tileset should open in Aurora like any
  hak tileset (R13). Aurora doesn't know the source resource. If Aurora
  drops it on save, a team that mixes Aurora and Moonglow loses the
  sculpt (R14 decides the resource type; the manual warns).
  Tileset-specific doors in doortypes.2da are matched by tileset name, so
  `<set>` lists none of S's in Aurora's palette.
- **Copying the source tileset.** `<set>.set` repeats S's text, as
  Mapper's copy does. Game data is never committed to the repository, but
  this copy is made on the builder's machine into their own hak, as
  tileset haks do. If S later gains tiles, the copy keeps the old count:
  its numbers stay right, and the new tiles aren't offered.
- **Builder workflows with Mapper.**
  - Mapper's exports are readable (the plugin imports them) and its
    heights can be taken back.
  - Its project files are not (§4).
  - A builder moving between the two tools would export a heightmap PNG
    if Mapper writes one. Asking its author for that, and for a license,
    is cheaper than reading Godot's format.
- **Licensing.** Moonglow is GPL-3.0. The terrain shader is Moonglow's
  text, copied into every builder's hak. Unless its file says otherwise,
  a hak carrying it carries GPL code (question 7).

## The owner's questions

1. **Layers blended in the game by Moonglow's terrain shader, or baked
   into textures (Mapper's way)?** Recommended: blended in the game,
   default and only way until M6; the bake later as an option for servers
   without custom shaders. Baked at stock sharpness, a 32 × 32 area is
   about 700 MB of textures; blended, 4 MB of weights.
2. **Resolution:** 16 quads a tile for the ground and 8 for the walkmesh
   (Mapper: 32 and 16)? Recommended: yes, with 8 or 32 for the ground
   kept possible in the format. R4 and R5 can move the walkmesh's.
3. **Layers:** a base and four painted, each with its normal map (11 of
   the 11 slots)? Recommended: yes. Mapper's nine don't fit a shader.
4. **Where the source lives:** in the module, a resource an area
   (recommended; the NUI Creator's precedent, never sent to players), or
   in files beside the module or in the project folder?
5. **The tileset:** one of the area's own, S copied whole and the
   sculpted tiles after it (recommended), rather than one for the module
   or S patched under its own name as Mapper does?
6. **Off by default,** behind Options › Areas › Sculpted terrain, with
   the hak named by the builder, as lights way B? Recommended: yes.
7. **The shader's license:** may the terrain shader file be offered under
   a permissive license (MIT) beside Moonglow's GPL, so a server's hak
   carries no GPL terms? Recommended: yes. It is the owner's code to
   license.
8. **Lighting:** the game's own, with ambient occlusion in vertex colors
   later, and no sun-baked lightmap? Recommended: yes.
9. **Mapper:** ask dafena for a license on the repository and for a
   heightmap and weight-map export, rather than reading its `.res`?
   Recommended: yes. It also settles whether its exports ship
   `mzlm_*.shd`.
10. **The first milestone:** the pipeline from a heightmap with no brush
    (M1), taking the client and engine readings, before the Sculpt mode?
    Recommended: yes.
11. **Area size:** keep 32 × 32 as the limit? Recommended: yes, until R12
    shows the engine and the client's load time take more.

[^owner]: human:august, 2026-11-02: the deferred sprint's large entry 9, design first, then build.
[^deferred]: The deferred list, "A builder's terrain tool (NWN Mapper) as the measure".
[^mapper]: github.com/Dafena/nwn_mapper_ffeom, read 2026-11-02 (README, nwn_area.gd, nwn_terrain.gd, nwn_exporter.gd, main.gd, export_shaders/); no license file.
[^tileset-import]: docs/plugins/examples/tileset-import, tried on the builder's earlier export of 149 files.
[^tilesets]: research/notes_tilesets.md: §A.6–A.7 (tiles, groups, ImageMap2D), §B.1 (geometry), §C.2 (Tile_List), §D (walkmeshes, path nodes), §G (EE changes, the tileset limit lifted in 89.8193.37-13).
[^border]: research/notes_tile_border.md.
[^shaders]: research/notes_shaders.md §C (texture slots 0–10 for custom shaders) and the vertex attributes (`vCustomColor`).
[^frames]: research/notes_frames.md (area092; The Domains of Dread's woods).
[^wiki-limits]: nwn.wiki, Resource Limits, with the local correction on 2 GB haks (`engine_big_hak.rs`).
[^wiki-hak]: nwn.wiki, HAK.
[^wiki-render]: nwn.wiki, Render Distance with Fog and Skyboxes.
[^wiki-tutorial]: nwn.wiki, Tileset Construction Tutorial, walkmesh rules 1, 2, 3 and 6.
[^wiki-walkmesh]: nwn.wiki, Walkmesh Notes.
[^wiki-tilesets]: nwn.wiki, Tilesets, "Tileset Limit".
[^measured]: The install (89.8193.37) read with `mg cat`: binary tile models of 36 to 125 KB, walkmeshes of 2.6 to 8.6 KB; ttr01_a01_01 355 faces, ttf02_a01_01 990; ttr01_grass02 a 512² texture spanning 2 units of UV across a tile.
[^lights]: lights-proposal.md: the request relayed from the builder dafena; way B's generated hak, off by default by the owner's word.
[^inventory]: Read at develop 12581870: crates/mg-ui/src/terrain_mode.rs and area_view.rs, crates/mg-area/src/walk.rs, scene.rs and pick.rs, crates/mg-render/src/renderer.rs, model.rs and scene.rs, crates/mg-mdl/src/compile.rs, binary_write.rs and walkmesh.rs, crates/mg-set/src/edit.rs, crates/mg-image/src/lib.rs, crates/mg-edit/src/lib.rs.
