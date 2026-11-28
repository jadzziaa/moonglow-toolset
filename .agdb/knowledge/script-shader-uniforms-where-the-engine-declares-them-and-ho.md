---
type: Fact
title: "Script shader uniforms: where the engine declares them and how Moonglow feeds them"
description: "SetShaderUniformFloat/Int/Vec map to inc_scriptable.shd's loose uniforms scriptableFloat1..16, scriptableInt1..16, scriptableVec1..16 (index 0 = name 1); Moonglow feeds them from Environment::script in the frame block, and a client passes them via mg_area::View::script_uniforms."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-28T07:57:44Z }
touches: ["crates/mg-render/src/scene.rs:ScriptUniforms", "crates/mg-render/src/game.rs:scriptable", "crates/mg-area/src/scene.rs:environment"]
asks:
  - Where does the game declare the uniforms SetShaderUniformFloat sets?
  - Is SHADER_UNIFORM_1 scriptableFloat0 or scriptableFloat1?
  - "How does Moonglow Client hand the server's shader uniforms (message 56) to the renderer?"
  - Why did a custom shader reading scriptableFloat1 draw zero in Moonglow?
fingerprint: v2-cdf9f95172c01d73
created: 2026-11-28
author: augustaugur
---
- The engine source: the install's base data `inc_scriptable.shd` (read with `mg cat --no-user-dir inc_scriptable.shd`; no .shd is loose in the install) declares 16 of each as LOOSE uniforms: `uniform float scriptableFloat1;` ... `uniform int scriptableInt1;` ... `uniform vec4 scriptableVec1;` ... up to 16. No base-game program includes it; nwn.wiki "Shader Engine Support" says they are passed to all shaders the player runs (added 1.87.8193.35).
- nwscript.nss: `SHADER_UNIFORM_1 = 0` ... `SHADER_UNIFORM_16 = 15`, so a script index n is the uniform numbered n+1. Per player; not kept over a rejoin. The server sends them as messages 56.1 (float), 56.2 (int), 56.3 (vec) (Moonglow Client E10).
- Moonglow: `mg_render::ScriptUniforms` (floats/ints/vecs, setters by index, zero default) in `Environment::script`; `game.rs` `scriptable()` keys the names (only 1..16, no leading zero) into `Key::ScriptableFloat/Int/Vec`, all `Part::Frame`, so both stages read them. Before 2026-11-27 only the vectors were fed (`Environment::scriptable`, for the TDoD wallpaper) and floats/ints were zero via `unfilled()`.
- `mg_area::View::script_uniforms`: `Some` replaces everything (the wallpaper too: a real server sends that itself); `None` = the editor's (zero but the GIT's `tdod_wallpaper` in `scriptableVec1`).
- Client comparison: `script_uniforms_in_the_client` in client_render (ignored), run 2026-11-28 on the off-screen display: client and Moonglow match to the unit; uniforms no script set read zero in the client (the default confirmed). Measure only the cards' upper half: tni01's candles stand before some cards' feet and darken a whole-card mean.
