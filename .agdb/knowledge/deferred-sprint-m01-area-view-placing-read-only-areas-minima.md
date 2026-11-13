---
type: Decision
title: "Deferred sprint m01: area view - placing, read-only areas, minimap, opening frames"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [crates/mg-module/src/instances.rs, crates/mg-ui/src/area_view.rs, crates/mg-ui/src/claims.rs, crates/mg-ui/src/minimap.rs, crates/mg-ui/src/blueprint/situated.rs, crates/mg-ui/tests/frame_perf.rs]
fingerprint: v2-21990a4064eb9a75
created: 2026-10-29
author: August
description: "Records area-view decisions: closure-based blueprint placing via with_placing, read-only areas via claims::read_only, a CPU minimap, thread-based Reload Resources, plus ruled-out frame-perf theories and test pitfalls."
asks:
  - "How do I place objects from blueprints in the area view, and why does with_placing take a closure?"
  - "How does the area view become read-only for a claimed area, and which handles and inputs get disabled?"
  - "My frame_perf test shows every frame of an area opening redrawing fully; is that a real rendering problem?"
  - "Tests with add_enabled_ui(true, ..) in a Grid or toolbar run past max_steps; why does it keep requesting frames and how do I avoid it?"
---
## Decided
- Placed objects from blueprints go through `mg_module::instances::with_placing(game, module, |placing, read| ..)` (closure-taking: `Placing` holds `&dyn Fn`, a struct-returning helper fights lifetimes); in the window `area_view::placing(app, ..)` flushes the workspace first. The paint stroke keeps its own per-stroke blueprint cache inside the closure.
- A read-only area (`claims::read_only(app, area)`, `Access::ReadOnly` only; `Warn` areas stay editable) sets `AreaView::read_only` each frame in `area_view::ui`: `turn_ring` false hides every handle, `terrain_mode::active` returns None, input() gates drops/brush/paste/drag starts/keys; the start location (the IFO's, not the area's) still moves. Forms reuse the game-blueprint "viewed" path: `blueprint::look_only` + `read_only_look`, OK/Cancel become Close (`ok_cancel(.., viewed)`).
- The minimap is CPU (`mg_module::minimap`, `crate::minimap::Map` keyed by a hash of the ARE's tileset, size and tiles' id/turn): no mg-render change. `AreaView::is_over` excludes `map_rect`, so the coordinates/ghost don't show under the map; the map's `ui.interact` is registered after the view's image, so it takes the clicks.
- Reload Resources reads on the thread too: refresh's thread condition is `view.scene.is_none()` (was also `model.is_none()`), `Reading::again` lets show() drop the tile tools' hold.

## Ruled out
- "The first frame looks its MTRs up, 19-30 ms": instrumented `Renderer::slots`: 298 slots in 0.6-1.1 ms. And "every frame of an area's opening redraws whole": an artifact of `frame_perf`'s harness, whose kittest default step is 0.25 s (every frame past ANIMATION_STEP); with `with_step_dt(1/144)` one frame in six is drawn. An edit's re-read in the woods costs ~1 ms beyond its redraw.

## Watch out
- `ui.add_enabled_ui(true, ..)` around a widget in a Grid (the Variables button) or the area toolbar kept frames coming (scroll_area repaint): three "draw nothing over anything else" tests ran past max_steps. Wrap only when disabled; call the widget directly otherwise.
- Glyph 🗺 is the main toolbar's New Area (icons::a_glyph_is_one_command_s); the Minimap is 🗾.
- A `.mod` beside-place has no claims: a read-only test needs a project folder (`read_only_area_harness` in app/common.rs saves the area module as a folder with moonglow.toml and claims.2da).
- `mg_testkit::scratch_dir(name)` empties the folder each call: a second picture saved with the same name in one test wipes the first.
