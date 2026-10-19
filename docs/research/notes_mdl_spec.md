---
type: Research Note
title: nwn-mdl-spec against Moonglow's model code and notes
description: Where the shared NWN:EE model specification (nwn-mdl-spec, read on engine build 89.8193.37-17) and its conformance suite disagree with mg-mdl's reader, compiler and decompiler, with the preview's skin binding and with notes_models.md - each finding, the part of Moonglow it affects, and what was done or deferred.
tags: [models, mdl, conformance, compiler, skins]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-19T20:10:00Z }
stale_after: 2027-01-19T00:00:00Z
sources:
  - id: spec
    resource: https://github.com/plenarius/nwn-mdl-spec
    title: "nwn-mdl-spec: NWN:EE model format (MDL), an evidenced specification and conformance suite (commit ac87e10; text CC-BY 4.0, data CC0)"
    last_modified: 2026-10-19T19:13:35Z
  - id: run
    resource: "runner/mdlconf.py run and stock, with the spec's Moonglow adapter built against develop at b727b33 (2026-10-19)"
    title: The conformance suite run on Moonglow
  - id: notes
    resource: notes_models.md
    title: Moonglow's model research notes
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

Run on 2026-10-19 against `develop` (b727b33, version 1.34.2) with the
spec's own adapter:[^run] **27 of 46 compiles pass, 44 of 46 decompiles;
13 of 14 stock models decompile as the spec reads them.** The result is
line for line the report the spec publishes for Moonglow (`e0ed4c4`, a
commit not in our history), so that report describes the code as it is.

## What it means for Moonglow

Moonglow uses the three parts differently, which decides what each finding
costs:

- **The reader** draws every model (areas, previews, the viewer). A rule
  that changes what a model *is* (a keyword the engine does not read, a
  default) changes the picture.
- **The compiler** (`mg_mdl::compile`) makes the binary models that
  `mg-module` packs into a hak from models kept as text, and Moonglow
  Viewer's `mgv compile`. What it gets wrong is what the game shows.
- **The decompiler** gives the text in the resource browser and
  `mg cat --text`.

### Reader: what Moonglow draws

| Finding | Rule | Moonglow | Effect |
| --- | --- | --- | --- |
| `setfillumcolor` (a misspelling in 2,387 stock ASCII files) is not read | MDL-ASCII-070 | read as `selfillumcolor` (`ctrl.rs`), and [notes_models.md](notes_models.md) says to accept both | a model whose only self-illumination is the misspelling glows in Moonglow and not in the game |
| `ambient_only`, `fading_light`, `negativelight`, `lensflares` are not read | MDL-ASCII-070 | `ambient_only` read as `ambientonly` | an ASCII light so written is ambient-only in Moonglow alone |
| a light's defaults when absent: `lightpriority` 5, `nDynamicType` 1, `affectdynamic` 1, `shadow` 1, `fadinglight` 1 | zspec_light | 0, 0, 1, 1, 0 | ASCII lights that leave them out |
| `orientation 0 0 0 a`: a rotation of a about +X (the zero axis normalised to +X) | MDL-ASCII-032 | the identity | rare |
| `PARENT` (upper case) is not read: the node leaves the tree | MDL-ASCII-031 | read | rare |
| a vertex with no valid bone is drawn at clip-space 0 (its triangles collapse) | MDL-SKIN-030 | kept in its rest pose | the stock `c_wings2` has 102 such vertices |
| a body part's skin bones are the **skeleton's tree positions** (depth-first walk), and its bind lists are the skeleton's size | MDL-SKIN-011, -012 | agreed (1.34.1, from the HD bodies); the bone was looked up by part number first, now by the walk | none for `pmh0` (text, no part numbers); a binary skeleton on a supermodel could have differed |

### Compiler: what a hak built by Moonglow holds

| Finding | Rule | Moonglow | Effect in the game |
| --- | --- | --- | --- |
| animmesh animation sets: vertex-major, the animation node's vertex count sizes the copy | MDL-MESH-062, MDL-BIN-045 | written with vertex count 0 | an animated mesh (water) does not animate |
| AABB walkmesh: vertices merged by adjacency, a tie split on -Z (32) | MDL-MESH-051, -052 | merged globally, ties split on +X (1) | the walkmesh's tree differs from the engine's |
| normals from smoothing groups (back-to-back faces, group bits) | MDL-MESH-031 | differ | shading |
| tangents: a seam of mirrored UVs clones its vertex; a material named by `bitmap` or `materialname` gives a normal-mapped hint | MDL-MESH-041, -043 | no clone; `compile_named` has no material lookup (the hak build passes none) | normal-mapped shading |
| children sorted when a child mesh has `transparencyhint` > 0 | MDL-NODE-012 | file order | draw order of see-through meshes |
| degenerate faces removed (on AABB too) | MDL-MESH-024 | kept | extra faces |
| light defaults, `ambient_only`, `setfillumcolor`, a zero orientation axis | as above | as above | as above |
| mesh `alpha` bezier key written | MDL-CTRL-020 | missing | a fading mesh |

### Decompiler

| Finding | Rule | Moonglow |
| --- | --- | --- |
| an emitter's bare static `detonate` | MDL-CTRL-012 | left out (`vwp_flash_blured`, `zspec_deton`) |
| animmesh sets over vertices split by their UVs | MDL-MESH-062 | one set fewer per frame (`zspec_animtv`) |
| a body part's skin bones named from the skeleton it is bound to | MDL-SKIN-040 | not checked here |

## Where it agrees with us

Controller ids and columns, node numbering against a supermodel (the name
walk, nodes it lacks, the node count), classification bytes, number
reading, aabb render and shadow defaults, mesh material defaults, bezier
column bytes and the three-stop emitter values: Moonglow passes these, and
several of the spec's rules cite our notes (B.2, B.8, B.19, B.22c) and the
Viewer's in-game checks.

[^spec]: nwn-mdl-spec: NWN:EE model format (MDL), an evidenced specification and conformance suite (commit ac87e10; text CC-BY 4.0, data CC0)
[^run]: The conformance suite run on Moonglow
