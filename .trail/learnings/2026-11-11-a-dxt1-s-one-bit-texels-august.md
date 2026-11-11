---
title: "A DXT1's one-bit texels in the client: opaque black"
author: August
date: 2026-11-11
branch: develop
cost: { minutes: 29, tokens: 145218 }
touches:
  - apps/mg/tests/texture.rs
  - crates/mg-corpus-tests/tests/client_render/batch6.rs
  - crates/mg-image/src/lib.rs
  - crates/mg-render/src/texture.rs
  - docs/findings.md
  - docs/log.md
  - docs/research/notes_shaders.md
  - apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock
  - apps/mg/target/tmp/moonglow-serve/a1098591-0667e805/.mg-lock
  - apps/mg/target/tmp/moonglow-serve/a1098591-f068852f/.mg-lock
---

## Decided
- Read in the client (`batch6_bioware_dxt1_one_bit`, batch6.rs): a DXT1's index-3 texels in the three-color mode (c0 <= c1) are opaque black with alpha 1 to the lit and env-mapped programs (decals, additive, MTR transparency, particles, icons not read), under BioWare's header and the standard one; a TXI `alphamean 0.5` or `blending punchthrough`, a mesh `alpha 0.5` (evenly half there) and an `envmaptexture` change nothing. The game's tdt01_wood04 draws like its 24-bit TGA with those texels black.
- The owner's ruling "handle BioWare DXT1 as the game does": `mg texture convert`/`info` (`source_rgba`, opaque) were already right; unchanged. A standard DXT1 keeps its bit in the converter (NWN2 cut-outs) though the game draws it black: outside the ruling, left.
- The gap was the renderer's game-shader path: `Bc1RgbaUnorm` gives index 3 alpha 0, so the game's programs cut or reflected through it. `mg_render::texture::one_bit_texels` sends such a texture up decoded (its stored levels, alpha 255). Moonglow's own shader was already right.
## Tried and ruled out
- Taking the alphamean card alone as evidence: a hole there fits both "texel alpha 1" and "the client ignores alphamean on DXT1". The mesh-alpha and env-map cards told them apart (alpha 1).
## Watch out
- `ps -C nwmain-linux` exits 1 when nothing runs: never chain a PID logger after it with `&&` (one run's sandbox PIDs went unrecorded so).
- Hole/black/opaque need a bright board behind the cards (a yellow 24-bit TGA card 0.3 m north); tic01's walls alone don't tell a hole from black.
- Cube map faces with such texels go up decoded too (`upload_cube_as_stored` returns None for them).
- Counts: the game 4 DDS at any level (wood04/05 a quarter of texels; tts02_gate03, tts02_tree01 a few in smaller levels); TDoD 38 entries, a few scattered texels each.
