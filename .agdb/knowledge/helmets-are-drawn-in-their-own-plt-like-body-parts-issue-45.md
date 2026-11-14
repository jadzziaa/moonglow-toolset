---
type: Decision
title: Helmets are drawn in their own PLT like body parts (issue 45)
description: "A helmet with a PLT of its own name is drawn in it on every mesh, whatever the meshes name, as the client draws body parts; the mesh keeps the material (normal maps) of the name it gives; read in the client for issue 45."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T00:25:17Z }
touches: ["crates/mg-preview/src/creature.rs:helmet_textures", crates/mg-preview/src/item.rs, crates/mg-corpus-tests/tests/client_render/part_textures.rs, crates/mg-preview/tests/previews.rs, crates/mg-render/src/renderer.rs]
asks:
  - "Why is a helmet or hood drawn untextured, gray and shiny?"
  - Which texture does the game draw on a helmet whose mesh names another texture?
  - "Whose normal map does a part or helmet drawn in its own PLT get, the named texture's MTR or its own?"
  - How do I make a client reading that shows which normal map is drawn?
fingerprint: v2-4a9f45dc8154ada5
created: 2026-11-13
---
## Decided
- `creature::helmet_textures` (called from `item::item_parts`' layered branch and the worn helmet in `creature::dressed`): a helmet (`helm_NNN`) with a PLT of its own name is drawn in it on every mesh, whatever the mesh names; a mesh naming an MTR with texture0 keeps it (beside other meshes in the PLT, row $); a TGA/DDS of its name alone replaces nothing, beside the PLT it wins (rows E, &); no own PLT: each mesh keeps its name. Shares `every_mesh_in` with the body-part rule (`read_plt_fallbacks`); helmets have no fallbacks. Cached in `Remembered::helmets`.
- The material (normal/specular maps) stays the one of the name the MESH gives, not of the name it is drawn in, for helmets and parts alike: client rows 4,5,7,8 and %,+,#,= (2026-11-14). Moonglow already does this (`Renderer::slots` looks the MTR up by `mat.texture`, before renames). So TDoD's helm_006 draws with helm_003_N, as in the client.
- A mesh whose texture is nowhere: client draws it close to a white TGA of alpha 0 (env map reflected wholly), Moonglow too; Moonglow's chrome1 reflection on such a surface is darker (black tops): deferred entry, a reflection reading, not a missing-texture rule.
- `models_with_a_plt_of_their_own_name` (previews.rs, ignored; MOONGLOW_HAKS) lists models with own PLT: game 36 (35 helms + gi_cloak01), 11 change; with TDoD 195, 78 change, all helmets.
## Tried and ruled out
- Normal-map reading: front faces under ambient 0x909090 show no stripes; a 45-degree turn showed nothing; a normal into the surface (z<0) draws as straight out (the client reads x,y only). What works: `CLOSE` groups, ambient 0x303030 / diffuse 0xFFFFFF, camera (90, 4, 40) from above, creatures at y 21, stripes straight out vs tipped in x (R 252), UV box per face (`uv_box_node`, `nm:` mesh prefix), read on the sunlit tops.
## Watch out
- `helm_033.plt` is all Metal 1/2 layers: in the flat cloth/leather palettes it shows as dark metal.
- CEP is not on this machine; the reporter's hood (a cloth hood icon) is a hak's replacement of helm_033, not the stock metal helmet.
- Helmet rows need `n <= 255` (`ModelPart1` is a byte) and `mg_helm<n>` per row.
