---
type: Decision
title: "Custom shaders' UV sets and tangents, read from the client"
tags: []
status: stable
generated: { by: trail, at: 2026-10-25T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/client_render.rs, crates/mg-mdl/src/ascii.rs, crates/mg-mdl/src/binary_write.rs, crates/mg-render/src/dangly.rs, crates/mg-render/src/game.rs, crates/mg-render/src/model.rs, crates/mg-render/src/renderer.rs, crates/mg-render/tests/render.rs, docs/deferred.md, docs/findings.md]
fingerprint: v2-b350bca4623d4ddc
created: 2026-10-25
author: August
description: "Moonglow's custom shaders showed washed-out walls because missing uv1..uv3 sets must be zeros, as the client does; tangents are also zero unless the model or render hint supplies them, so normal maps draw black."
asks:
  - "Why do custom-shader walls look washed out in Moonglow compared to the real client?"
  - "What does the client feed a custom shader when the model lacks uv1, uv2 or uv3 sets?"
  - "Why does a custom shader's normal map render black when the model has no tangents?"
  - "Can I use nwnmdlcomp to check tverts1..3 output, or should I compare against the client instead?"
---
## Decided
- Custom shaders' extra UV sets: Vertex holds uv1..uv3 (76 B); vTcIn2/vTcIn3 bound by name (game.rs attribute). A set the model lacks = zeros, read from the client (`custom_shader_uvs_look`, cards whose sets each cover one stripe; zero UV = the gray/blue blend at (0,0)). Heed's washed-out walls = his VS picking tverts2/3, which Moonglow fed as zeros.
- Tangents on the game path: the engine has them only from the model or a render hint (mdl or MTR); otherwise zeros (PipelineKey::tangents = !tangents_made || normal_variant). Client draws a custom pair's normal map black then; Moonglow now does too.
- Default textures are NOT a cause: `MG_UV_PAINT=bound` paints textureNBound; client and Moonglow agree on every card (unnamed maps unbound in both).
## Tried and ruled out
- nwnmdlcomp as an oracle for tverts1..3: it is the 2003 compiler, predates them. The client reading ASCII from the override is the oracle.
- Static vs dynamic placeables for custom-shader uniforms/UVs: identical in the client.
## Watch out
- headless.sh is SHARED between agents: `stop` kills the display under another agent's running client. Check `pgrep -a nwmain` for other scratch clients before stopping.
- Card regions: project the card's corners with camera.projection_gl * view (fitted_camera); the region numbers then compare client vs Moonglow without fitting.
- The client's set-0 card differs ~3/255 from Moonglow's while one-stripe cards agree within 0.5: stripe edges and mips, not the shader.
