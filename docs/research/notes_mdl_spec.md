---
type: Research Note
title: nwn-mdl-spec against Moonglow's model code and notes
description: Where the shared NWN:EE model specification (nwn-mdl-spec, read on engine build 89.8193.37-17) and its conformance suite disagreed with mg-mdl's reader, compiler and decompiler, with the preview's skin binding and with notes_models.md - each finding, the part of Moonglow it touched, what was changed (the suite from 27 to 46 of 46 compiles) and what is left.
tags: [models, mdl, conformance, compiler, skins]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-19T20:39:09Z }
stale_after: 2027-01-19T00:00:00Z
sources:
  - id: spec
    resource: https://github.com/plenarius/nwn-mdl-spec
    title: 'nwn-mdl-spec: NWN:EE model format (MDL), an evidenced specification and conformance suite (commit ac87e10; text CC-BY 4.0, data CC0)'
    last_modified: 2026-10-19T19:13:35Z
  - id: run
    resource: runner/mdlconf.py run and stock, with the spec's Moonglow adapter (its compile given the probe folder's .mtr files through compile_named_with), against develop on 2026-10-19
    title: The conformance suite run on Moonglow
  - id: client
    resource: placeables_look in crates/mg-corpus-tests/tests/client_render.rs, with MG_OVERRIDE
    title: The game client, on a panel lit by setfillumcolor and by selfillumcolor
---

# nwn-mdl-spec against Moonglow's model code and notes

[nwn-mdl-spec](https://github.com/plenarius/nwn-mdl-spec) is a shared
description of the game's model files: rules with stable ids
(`MDL-SKIN-012`), each with its evidence (a function in the Linux engine
build 89.8193.37-17, or the engine's own compile of a probe model), and a
conformance suite of 46 probes with the engine's compiles (goldens).[^spec]
It names Moonglow's `mg-mdl` among the implementations and ships an adapter
for it; its reconciliation table started from our `ctrl.rs`, `keywords.rs`
and [notes_models.md](notes_models.md).

## The suite run on Moonglow

| | Compile | Decompile | Stock models |
| --- | --- | --- | --- |
| 1.34.2 (as the spec's published report, `e0ed4c4`) | 27 of 46 | 44 of 46 | 13 of 14 |
| After the changes below | **46 of 46** | **45 of 46** | **14 of 14** |

Run with the spec's runner and adapter;[^run] two of the compiles need the
probe's `.mtr` files, which the adapter passes once it calls
`compile_named_with` (the spec's adapter calls `compile_named`, which looks
up no materials).

## How Moonglow uses each part

- **The reader** draws every model (areas, previews, the viewer).
- **The compiler** (`mg_mdl::compile`) makes the binary models that the hak
  window, Build and `mg pack --compile-models` pack from models kept as
  text, and Moonglow Viewer's `mgv compile`.
- **The decompiler** gives the text in the resource browser and
  `mg cat --text`.

## What changed

### The reader (what Moonglow draws)

- **`setfillumcolor` gives no self-illumination** (MDL-ASCII-070). Our
  notes said to accept it; the game does not read it, seen in the client
  on a test panel.[^client] 28 meshes of the game's ASCII models glowed in
  Moonglow alone (the lanterns of `tni01` tiles). The linter notes it.
- **A light's values when left out** are the game's: priority 5, dynamic
  type 1, affect dynamic 1, shadow 1, fading 1; `ambient_only`,
  `fading_light`, `n_dynamic_type` and `affect_dynamic` are not read (the
  linter notes them). Four of the game's lights have no priority: drawn at
  the highest in Moonglow before.
- **`orientation 0 0 0 a`** turns `a` about +X (MDL-ASCII-032); 206 of the
  game's nodes have such a line, with angles under a millionth.
- **`parent` is read in lower case only** (MDL-ASCII-010); the linter
  warns about `PARENT`.
- **Vertices merge as the game merges them** (MDL-MESH-010, -011): by
  their values (position, normal, UVs, colour, tangent, a face's
  `mirrorlist`, a skin's weights, a dangly constraint), not by their
  index in the text; a walkmesh only through a neighbouring face
  (MDL-MESH-012). A face left with two corners on one vertex is dropped
  (MDL-MESH-020), except on a walkmesh, whose tree numbers its faces.
- **Normals from smoothing groups** as the game computes them
  (MDL-MESH-031, -032). The game's own ASCII models look the same: the
  most a picture of the kobold `c_koba` changed was 4 of 255.
- **Key lists in the geometry** (`alphabezierkey`) are controllers at their
  first value (MDL-CTRL-023).
- **A body part's skin bones** are the skeleton's tree positions
  (MDL-SKIN-011), looked up by the walk, not by part number.

### The compiler (what a hak built by Moonglow holds)

- **Animated meshes animate**: the animation node carries its vertex count
  (MDL-BIN-045), and every set follows the merged vertices (MDL-MESH-062).
  Before, an animated mesh compiled by Moonglow did not move in the game.
- **Walkmesh trees** take the game's planes (MDL-MESH-052: from the
  children's centres, signed, a tie -Z).
- **Tangents** as the game makes them (MDL-MESH-043): seams of mirrored UVs
  get copied vertices; only `NormalAndSpecMapped` and `NormalTangents`
  ask for them; and a mesh with no hint takes its material's
  (MDL-MESH-041), from the `.mtr` its `materialname` or `bitmap` names
  (`compile_named_with`; the hak build finds them among the files, the
  haks and the game).
- **Children of see-through meshes** are sorted as the game sorts them
  (MDL-NODE-012): libstdc++'s `std::sort`, step for step
  (`mg_mdl::stdsort`), since past 16 children it is not stable.

### The decompiler

- An emitter's bare `detonate` is written (MDL-CTRL-012).

## What is left

| Finding | Rule | Moonglow |
| --- | --- | --- |
| an animated mesh's frames over vertices split by their UVs (`zspec_animtv`) | MDL-MESH-062 | written over the welded vertices: the game reads them back to the same frames, the suite compares the text |
| a node whose `parent` line is missing or not read is in no tree | MDL-ASCII-031 | hangs from the root (only `c_quiver`'s malformed `node pos=[…]` lines in the game's models) |
| a vertex with no valid bone collapses to clip-space 0 | MDL-SKIN-030 | kept in its rest pose (the stock `c_wings2` has 102) |
| face records' normals and adjacency over the source faces after a deletion | MDL-MESH-021, -022 | from the faces kept (SHOULD; nothing seen in game) |
| a body part's skin bones named from its skeleton when decompiled | MDL-SKIN-040 | not checked |
| the suite in our own tests | | its probes and goldens are CC0 and could run in the corpus tests |

## Where it agreed with us from the start

Controller ids and columns, node numbering against a supermodel,
classification bytes, number reading, aabb render and shadow defaults,
mesh material defaults, bezier column bytes and the three-stop emitter
values; several of the spec's rules cite our notes (B.2, B.8, B.19, B.22c)
and the Viewer's in-game checks.

[^spec]: nwn-mdl-spec: NWN:EE model format (MDL), an evidenced specification and conformance suite (commit ac87e10; text CC-BY 4.0, data CC0)
[^run]: The conformance suite run on Moonglow
[^client]: The game client, on a panel lit by setfillumcolor and by selfillumcolor
