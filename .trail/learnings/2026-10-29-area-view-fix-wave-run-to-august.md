---
title: "Area view fix wave: run-to-run pictures, grass fade, start location"
author: August
date: 2026-10-29
branch: worktree-agent-a83a02f01b46ebb44
cost: { minutes: 60, tokens: 13331154 }
touches:
  - crates/mg-corpus-tests/examples/render_area.rs
  - crates/mg-area/src/grass.rs
  - crates/mg-ui/src/area_view.rs
  - crates/mg-render/src/game.rs
---

Decided: run-to-run picture differences are looked for with render_area run twice in two processes per area (a scratch script over mg ls MODULE's .are names, sha1 then PIL diff). Moonglow's own shader: every area of Tyrants of the Moonsea (140), Prelude and Chapter 1 (118) the same to the bit. MG_SHADERS=1 (the game's shaders, the app's default): 8 of Tyrants' 140 differ by a few scattered pixels (area085 the easiest, 1 to 366 px, up to 15/255), a different few each run. Ruled out: the scene (MG_LIGHTS=1 lists identical), the pipelines' naga modules and bind group layouts (Renderer::capture_game_modules dumped with Debug, identical across runs), and anything within one process (the same scene rendered again by the same or a new Renderer is identical). So the cause is set once a process in the game shaders' draw path: look at std HashMap orders in mg-render/src/game.rs (pipelines, value_ids, the worker's programs) and the order the worker finishes pipelines. Grass levels: a fade must start AT a level's distance, not before it: grass_covers_the_whole_view fails (184 m point, 34 of 1600 px changed) when the tilesets' grass thins from 135 m. Watch out: a right click on the start marker must not take the selection away from objects selected (placement_tools_turn_ground_arrange_lock_and_make_prefabs right-clicks a waypoint standing on the default start location at 20,20, where pick() is None). Worktree shell guard: python heredocs whose text contains '.git' (area.git) or 'm²' are refused; write the script to the scratchpad and run it.
