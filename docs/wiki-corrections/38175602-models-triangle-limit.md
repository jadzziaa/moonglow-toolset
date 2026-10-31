---
type: Wiki Edit
title: "Models: a mesh's limit is 65,535 vertices, not 21,845 triangles"
description: Ready-to-paste edit for the nwn.wiki Models page, whose Model Triangle Limit gives 65535 / 3 = 21845 triangles a mesh; the EE client draws a mesh of 29,768 triangles whole, the limits being 65,535 vertices and the binary format's 32,767 faces.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-31T08:10:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/38175602/Models
sources:
  - id: client
    resource: crates/mg-corpus-tests/tests/client_render.rs
    title: "placeables_look with a 2 by 3 m wall of 123 by 123 vertices (15,129 vertices, 29,768 faces) in one mesh, as text and compiled, beside the same wall in 19,602 faces (2026-10-31)"
  - id: spec
    resource: ~/.local/share/moonglow-oracle/nwn-mdl-spec/spec/binary.md
    title: "nwn-mdl-spec: the mesh header's u2 vertex count, the face's s2 adjacent faces, the index list counts' u4"
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/38175602/Models
    title: "nwn.wiki: Models"
---

# Models

https://nwn.wiki/spaces/NWN1/pages/38175602/Models, edit written against the local mirror's version.

## Edit summary

The per-mesh limit is 65,535 vertices (and 32,767 faces in a compiled model), not 21,845 triangles.

## Change 1

**Find** ("Model Triangle Limit", its first paragraph):

> In NWN:EE 1.87.8193.35 the vertex (polygon) limit per mesh is now 65535 / 3 so 21845 triangles per individual mesh (each "node" section). This is (((2^16) -1) / 3). As per before anything larger must be split into multiple meshes.

**Replace with:**

> In NWN:EE 1.87.8193.35 the limit per mesh is 65,535 vertices (each "node" section): vertex indices are 16 bits. Triangles that share vertices don't count against it more than once, so a mesh may have more than 65535 / 3 = 21845 triangles: a mesh of 29,768 triangles on 15,129 vertices is drawn whole. A compiled (binary) model also holds at most 32,767 faces per mesh, as each face names its neighbours in a signed 16-bit number. Anything larger must be split into multiple meshes. (Vertices here are the game's: a vertex with two texture coordinates or normals, at a UV seam or a sharp edge, counts twice.)

## How it was checked

Moonglow's `placeables_look` (`crates/mg-corpus-tests/tests/client_render.rs`) shows three armoires in the EE client, their model replaced from the user folder's `override`. A 2 by 3 metre wall of 123 by 123 vertices (15,129 vertices, 29,768 triangles in one `trimesh`), as text and compiled to binary by Moonglow, was drawn pixel for pixel as the same wall of 100 by 100 vertices (19,602 triangles): none of it was cut away. The binary format's limits are nwn-mdl-spec's (the mesh header's `u2` vertex count, each face's `s2` adjacent faces, the index list's `u4` count).
