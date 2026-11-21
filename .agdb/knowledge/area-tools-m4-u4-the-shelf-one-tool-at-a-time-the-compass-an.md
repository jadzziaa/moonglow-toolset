---
type: Decision
title: "Area tools (M4 U4): the shelf, one tool at a time, the compass and Paint's header"
description: "Where the area view's tools live since U4 (shelf 'Area tools', compass group 'Compass', Paint's options in the header with a Brush popover), the one function every tool choice goes through, and the test pitfalls met (scope under the view, segmented group's first-frame bounds, a floating window covering the shelf)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-15T07:47:44Z }
touches: [crates/mg-ui/src/area_view/tools.rs, "crates/mg-ui/src/paint_mode.rs:header_items", crates/mg-uikit/src/overlay.rs, "crates/mg-ui/tests/app/common.rs:area_tool"]
asks:
  - "Where did Select Tiles, Paint, Reorient Camera and Go to Start Location go after U4?"
  - Why does turning on Paint or F10 put the tile brush down?
  - Why did the area header go compact after adding something under the view?
  - "Why is the Brush popover's Turn field named Turn spread?"
fingerprint: v2-7ad05ddb2b7a76f3
created: 2026-11-15
---
- **Shelf**: `area_view::tools::shelf`, a kit `ToolShelf` named **"Area tools"** (the board's "Tools" collides with the Top Bar's Tools menu in `get_by_label`). Tools: Select Objects, Select Tiles, Paint, Terrain. Every way of taking a tool calls `tools::choose`/`toggle` (shelf, B and F10 in `area_view::input`; F10 moved out of `camera_input`, which has no `app`): Paint drops the tile brush and tile mode, Select Tiles drops Paint and the brush, a tile brush chosen in the tiles palette (`terrain_mode`, `tile_view`'s Paint with It) sets `app.paint = false`. `tools::tool_of` says which is on (Paint > Tiles > Terrain > Select).
- **Terrain** (decision 8): no mode of its own; on while `terrain_mode::active` (any tiles palette brush); a click shows the Palette on Tiles; on, a click puts the brush down.
- **Compass**: `tools::corner`, kit `overlay::{paint_compass, ViewButton, corner_column}`, group **"Compass"**; `AreaView::corner_rect` is left out of the view's input (`is_over`) like `map_rect`. The minimap moved to the bottom right and is left out where it would meet the compass column. Needle = camera heading (`north_of`), not a projected ground direction (perspective foreshortens that).
- **Reorient Camera** is `Id::ReorientCamera` + `keys::Cmd::ReorientCamera` (General, Ctrl+Alt+R, in `GIVES_WAY` like Quick Open), pushed into `Id::all()` (no menu).
- **Paint's header**: `paint_mode::header_items` (mode segmented "How the brush paints", Radius, Density, Brush popover with `Popover::min_width(286)` and `title_note(mode)`), the Show popover `in_more` after them. Fields are named by `accesskit set_label` and their words painted (no Label node), so `get_by_label("Gap")` finds the field; the spread field is **"Turn spread"** (the popover also holds the Turn combo). The palette's section is group **"Brush blueprints"** ("Brush" is the button and the popover).
- Pitfalls: (1) `ui.scope` after the view's image starts at the cursor under it and grew the tab's content by the item spacing: the tab (in a ScrollArea) got a scroll bar, 8 px narrower, and the header went compact. Put overlay widgets in `scope_builder(max_rect(...))`. (2) A kit `segmented` group that came onto the header this frame has NaN accesskit bounds for one frame: run a step before `header_holds`. `header_holds` scrolls a group in a long More by its first part. (3) A floating editor window held to the screen's height can cover the shelf: take shelf tools before opening one (accesskit clicks don't reach under a window with Cancel anyway).
- Review (U4): the tiles palette and the Tile window's Paint with It go through `tools::choose_in(app, area, tool)` (the view is out of the map while they run); F10 is handled after the area's body is drawn, over the whole editor rect (header, shelf, view), not in `input`; a read-only area resets tile mode every frame, so F10 there is a no-op as before. The coordinates' line is truncated before the minimap (`readout_rect`). The faster area084 wheel views in U4's timing were the tip's camera zooming out during its Paint view (strip moving the view), not U4.
