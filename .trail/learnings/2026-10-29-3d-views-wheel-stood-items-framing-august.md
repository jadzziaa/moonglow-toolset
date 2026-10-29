---
title: "3D views' wheel, stood items, framing, export files"
author: August
date: 2026-10-29
branch: worktree-agent-a73fcfbda82916d8c
cost: { minutes: 1687, tokens: 12750546 }
touches:
  - "crates/mg-ui/src/area_view.rs#Wheel"
  - crates/mg-ui/src/model_view.rs
  - crates/mg-preview/src/compose.rs
  - crates/mg-preview/src/files.rs
---

egui 0.36 turns Ctrl + wheel into zoom_delta and spreads it over the frames after, keeping the wheel's modifiers from the scroll's start: after Ctrl is let go, zoom_delta still moves while i.modifiers.command is false. area_view::Wheel therefore reads zoom_delta only while Ctrl is held, and a pinch from the frame's Event::Zoom events alone (reading zoom_delta without Ctrl zoomed the area view after a Ctrl+wheel scale with nothing selected: ctrl_and_the_wheel_scale_what_is_selected caught it). Choosing which side of a model faces the viewer: Composed::facing sums vertex normals and cannot tell a sword's flat from its edge (the blade is a part, not the base, and low-poly blades have as many edge vertices as flat ones: 0.048 against 0.071 m2 by area, but within 2% by normals); Composed::seen_area sums the triangles' projected areas over every part. Tight framing of pictures: mg_preview::compose::framed_on is the viewer's OrbitCamera::frame_tight; the gallery frames on Composed::points_in of the idle stance plus particle vertices, the box corners only when there is nothing. The game's own plc_a01 names fxpa_cloud02, which the game lacks: Gatherer::missing reports it (the json test allows it).
