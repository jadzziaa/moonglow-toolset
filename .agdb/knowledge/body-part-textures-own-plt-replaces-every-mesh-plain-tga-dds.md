---
type: Decision
title: "Body part textures: own PLT replaces every mesh, plain TGA/DDS beats PLT"
tags: []
status: stable
generated: { by: trail, at: 2026-11-11T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/client_render.rs, crates/mg-corpus-tests/tests/client_render/programs.rs, crates/mg-corpus-tests/tests/models.rs, crates/mg-corpus-tests/tests/textures.rs, crates/mg-image/src/tga.rs, crates/mg-image/src/dds.rs, crates/mg-corpus-tests/tests/client_render/part_textures.rs, crates/mg-preview/src/creature.rs, crates/mg-preview/src/lib.rs, crates/mg-preview/src/usage.rs, crates/mg-preview/tests/previews.rs, crates/mg-render/src/assets.rs]
fingerprint: v2-b31305c0712fe558
created: 2026-11-11
author: August
description: "The client draws a body part's own-named PLT on every mesh of the part, while a color DDS/TGA of the same name beats the PLT and gray 8-bit TGAs lose; MTRs with texture0 are kept."
asks:
  - "Why does my part's PLT texture show up on all meshes, including hair or gem meshes, instead of just one?"
  - "Which wins in the client when a PLT and a DDS or TGA share the same name, and does load order matter?"
  - "My custom body part texture looks gray-green or flat in the client; what palette or TGA format should I use?"
  - "When does a mesh keep its MTR material instead of being replaced by the PLT fallback in read_plt_fallbacks?"
---
## Decided
- Client reading `part_textures_look` (crates/mg-corpus-tests/tests/client_render/part_textures.rs, 32 rows, groups via MG_GROUPS; pictures in target/test-output/part_textures/): if a part has a PLT of its own name (or the first fallback with a PLT: pheno 0 -> human same gender -> human man; heads stop at their race's pheno 0), EVERY mesh is drawn with that name, whatever it names (non-part TGA/PLT, hair/gem second mesh, bitmap NULL). An own TGA/DDS alone replaces nothing.
- Of a PLT and a DDS/true-color TGA of one name the client draws the DDS/TGA wherever each is in the load order (override, hak, either way round), on parts and items (helmet). A gray 8-bit TGA (type 3) loses to the PLT: the base game's 18 PLT+TGA pairs are all gray (pmd0_head001). Implemented in creature::read_plt_fallbacks and mg_render assets texture_kept/takes_colors via mg_image::tga::is_gray.
- Materials (rows G-L, V): a mesh whose bitmap or materialname names an MTR WITH texture0 keeps the MTR (read_plt_fallbacks skips it via Lookup::has_material_texture); an MTR without texture0 is replaced by the PLT. The replacement name is drawn WITHOUT its own MTR in the client (row J); Moonglow still reads it through the MTR (deferred: renderer renames, ReplaceObjectTexture would change too; no content hits it).
- Robes are parts like any (M, N). A luminance (one-channel) DDS loses to the PLT like a gray TGA (mg_image::dds::is_gray); a gray DXT1 wins. A luminance DDS alone: client garbage colors (deferred).
- TDoD (150 haks): 2,520 parts change, all hak parts, mostly cloth parts whose meshes name `cmh0_`/`qmh0_` PLTs; the client draws them as the new rule does (part_textures_tdod_look, MOONGLOW_HAKS, MG_TDOD_SET=1|2).
- The old "a mesh naming no part's texture keeps it" half had never been read in the client (no stock body part names one; only cloaks).
## Tried and ruled out
- 24-bit flat palettes in override: the client draws no flat color (gray-green). Use 32-bit palettes with alpha 255.
- "Key TGA loses, override TGA wins": wrong; it is grayscale vs color (row A/B: a gray TGA in override loses too).
## Watch out
- Placement.rotation PI faces the creature to the player; 270deg shows profiles.
- MG_MOONGLOW_ONLY=1 MG_SIDE=after redraws Moonglow's side beside the kept client.png without the client.
- A mesh whose texture is missing: client light gray, Moonglow black (filed in deferred.md).
- To compare a rule over real content: a throwaway cfg(test) module inside mg-preview with HEAD's function copied (`git show HEAD:...`) can reach pub(crate) plt_fallbacks/Lookup; a temporary env switch gives the "before" picture. Remove both after.
