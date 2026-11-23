---
type: Gotcha
title: "Selected outlines and what stands inside them; the wide layout's Properties held while a press is under way"
description: "Why a press in the area view acts on the front hit unless that is a trigger or encounter outline, and why the wide layout's auto-hide of Properties waits for the button and egui's double-click time."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T01:58:53Z }
touches: ["crates/mg-ui/src/area_view.rs:press_target", "crates/mg-ui/src/shell.rs:hold_wide_properties", crates/mg-ui/src/tree.rs]
asks:
  - Why did dragging a waypoint inside a selected encounter move the encounter?
  - How does clicking cycle overlapping triggers and encounters?
  - Why does the Right Dock wait before showing Properties in the wide layout?
  - Why do tree and Outliner tests need run_steps between clicks on different rows?
fingerprint: v2-2ee36d67143a043d
created: 2026-11-23
author: augustaugur
---
An outline (ObjectKind::has_outline: Trigger, Encounter) lies on the ground, so pick_all puts what stands inside it first and the outline after; a rule "a selected hit under the pointer wins" (5.1.0's first cut) let a selected encounter take every drag and right-click meant for a waypoint inside it, and a click on a selected waypoint cycled to the encounter behind. AreaView::press_target: the front hit, unless the front is an outline and a selected outline is also under the pointer (then that one). Used by drag start, right-click and the double click's Properties; plain-click cycling and Shift's "add the next one" work only among the outline hits, and only when the front hit is an outline.

Wide layout (>= 2560): area_properties_shown depends on whether something editable is selected, which changes the Right Dock's width and moves the scene by half of it (a double click's second click landed 200 px off; a drag of an unselected chest moved it 9.8 m). shell::hold_wide_properties (run from frame_yields) latches Moonglow::wide_props for the active area and lets it change only with no button down and, after a press that began in an area view, once egui's input_options.max_double_click_delay has passed since the release (egui counts the double click from the release too). It schedules one repaint at the deadline; while waiting, Moonglow::under_way reports it so the tests' settle waits instead of returning on a non-zero repaint delay.

Test traps met: egui counts a double or triple click from any earlier click within the delay and within its click distance (max_click_dist, 6 points, egui 0.36.2 input_state/mod.rs:1165: rows that move under the pointer put a later row where an earlier click was), so a test clicking a folder row and then an object row needs run_steps(30) (0.5 s at 1/60) between; two folder clicks in a row can fold the second back. The Module Tree's object rows are out of the accesskit tree unless the Module Panel is tall (settings.module_panel_height); Outliner rows at the window's foot can sit under the Status Bar.
