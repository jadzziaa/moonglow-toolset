---
type: Decision
title: Helmets are drawn in their own PLT like body parts (issue 45)
description: "A helmet with a PLT of its own name is drawn in it on every mesh, whatever the meshes name, as the client draws body parts; read in the client for issue 45 (the game's hood helm_033 names 'helm', which is nowhere)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-13T23:54:16Z }
touches: ["crates/mg-preview/src/creature.rs:helmet_textures", crates/mg-preview/src/item.rs, crates/mg-corpus-tests/tests/client_render/part_textures.rs, crates/mg-preview/tests/previews.rs]
asks:
  - "Why is a helmet or hood drawn untextured, gray and shiny?"
  - Which texture does the game draw on a helmet whose mesh names another texture?
  - "Do shields, weapons or other items take their own PLT like body parts?"
  - "How do I read a helmet's textures in the game client?"
fingerprint: v2-bda7581578a68fa2
created: 2026-11-13
---
## Decided
- `creature::helmet_textures` (called from `item::item_parts`' layered branch and the worn helmet in `creature::dressed`): a helmet (`helm_NNN`) with a PLT of its own name is drawn in it on every mesh, whatever the mesh names; a mesh naming an MTR with texture0 keeps it; a TGA/DDS of its name alone replaces nothing; no own PLT: each mesh keeps its name. Shares `every_mesh_in` with the body-part rule (`read_plt_fallbacks`); helmets have no fallbacks (no race/pheno names). Cached in `Remembered::helmets`.
- Client reading (2026-11-13): `part_textures_look` groups `WXYZ36` and `012` (authored helmets 245-254, `helm_248.plt` the "other helmet"; rows 3 and 6 the game's helm_033 and helm_006), and `part_textures_tdod_look` with `MG_TDOD_SET=3` (TDoD helms 17, 36, 71, 76, 96, 33). All drawn by the client as the rule draws them.
- The game's `helm_033` and `helm_035` name `helm` (nowhere); nine game helms name another helmet's PLT (006->003, 008->002, ...). Over the game + TDoD's 151 haks, only helmets and body parts have a PLT of their own name: no shield, weapon or other item, so the rule is not applied to them.
## Watch out
- `helm_033.plt` is all Metal 1/2 layers: in the reading's flat cloth/leather palettes it shows as dark metal, not the flat orange.
- CEP is not on this machine; the reporter's hood may be CEP's `helm_033` (its icon is a cloth hood), but the stock model shows the same fault.
- A mesh whose texture is nowhere is still drawn darker than the client's light gray (the deferred "missing texture drawn black" entry): row Z shows it.
- Helmet rows in the reading need `n <= 255` (`ModelPart1` is a byte) and `mg_helm<n>` per row.
