---
title: "Black tile in the lit check picture: textures read apart a mesh at a time"
author: August
date: 2026-10-29
branch: worktree-agent-ab6bdccfbe724c384
cost: { minutes: 1595, tokens: 13074063 }
touches:
  - "crates/mg-render/src/renderer.rs#surfaces"
  - crates/mg-render/tests/render.rs
  - crates/mg-ui/tests/app/renderer_check.rs
---

## Found
- A tile drawn black in an area view picture where background_jobs is on is a model left out while its textures are read apart (Renderer::textures_apart; Drawn::textures_pending counts instances, not textures). Check drawn.textures_pending and drawn.meshes before suspecting shaders: in the lit check test meshes was 118 before and after the switch to Moonglow's own shaders, 142 one frame later.
- Renderer::surfaces collected surface() into Option<_>, which short-circuits: the meshes after the first waiting one never asked for their textures, so a model needed one round of reads per textured mesh (ttr01's rock tile: six, stone06 -> splotch01/bark01 -> treefol04/02 -> grassrim01 -> dirt04). Fixed by asking every mesh in one pass, then returning None if any waited.
- A texture's TXI envmaptexture is still asked a round later (it is known only once the texture is read): one extra round at most, inherent.
## Watch out
- The lit check ends in ~150 ms; a UI test that pictures the area after it (or after anything fast) with background_jobs on must step until textures_pending == 0 && game_pending == 0, or the picture races the reader threads. The lit check picture was black from its first commit (3680a48f) and drawn in some whole-group runs: no bisect target.
- To log what the reader asks and takes, an eprintln in Renderer::texture_read (r.asked.insert) and take_textures is enough; run the test with --nocapture.
