---
type: Decision
title: "Haks not layered: the 3D views held the game data"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: ["crates/mg-ui/src/lib.rs#release_game_data", "crates/mg-ui/src/model_view.rs#Viewport3d", "crates/mg-ui/src/renderer_check.rs#lit_check", crates/mg-ui/src/outside.rs]
created: 2026-10-29
author: August
description: "Any code mutating game data via `exclusive` (Arc::get_mut on app.game) must first call `self.release_game_data()`, which drops viewport and texture-reader Arc clones and waits up to 10 s for the strong count to hit 1."
asks:
  - "Why does my new `exclusive` call silently do nothing after an area view has been drawn?"
  - "What must I call before `exclusive` so Arc::get_mut on app.game succeeds?"
  - "Can I hold a Weak<GameData> to avoid blocking Arc::get_mut, and what should I use instead?"
  - "Why does the 3-second content watch in outside.rs use `game_data_held()` instead of `exclusive`?"
---
decided: Everything that changes the game data goes through `exclusive` (Arc::get_mut on app.game), and before it now calls `self.release_game_data()`: the viewport drops its cached GameAssets and the renderer's texture reader (`read_textures_apart(gpu, None)`), then it waits up to 10 s (not under a job) for the strong count to reach 1. A new exclusive call site needs that line first, or it silently does nothing whenever an area view has drawn.
watch out: The renderer's TextureReader threads each hold a clone of the Arc<GameAssets>; dropping the reader ends them asynchronously, so the count drops a moment later: the bounded wait is what makes the release work (the tests fail without it). The area view's first reading runs on a thread holding app.game too, and outlives a closed module's view.
watch out: Arc::get_mut also fails with a Weak alive: don't hold a Weak<GameData> to dodge this; stamp with Arc::as_ptr (as model_textures.rs does).
decided: The lit check reads the game's .shd programs into a map (92, 0.3 ms) and holds no game data while it runs, so a module opened in the first frame (command line) is layered without waiting for it.
watch out: outside.rs's 3-second content watch probes `game_data_held()` (others than the viewport), not exclusive, so the views aren't made to let go every tick while an area is open.
ruled out: Joining the reader threads in TextureReader's Drop (mg-render, which the viewer also builds on): the bounded wait in mg-ui covers it and the area-read threads alike.
