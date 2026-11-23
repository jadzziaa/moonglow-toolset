---
type: Fact
title: "egui zoom: set_zoom_factor takes at the next pass, so egui-winit's monitor_size lags a zoom change"
description: "Reading viewport().monitor_size with ctx.zoom_factor() in the frame after set_zoom_factor mismatches (egui-winit measured at the old zoom); wait a frame of unchanged zoom. Also: eframe persists egui's zoom_factor, so 5.1's Ctrl+Plus zoom survives into 5.2."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T18:20:01Z }
asks:
  - Why does the largest interface size swing between frames?
  - "When does egui's set_zoom_factor take effect?"
  - Why did an old zoom survive after the interface size setting was unset?
  - How is the interface size kept within the screen?
created: 2026-11-23
author: augustaugur
---
egui 0.36 set_zoom_factor stores new_zoom_factor, applied at the next begin_pass. egui-winit's update_viewport_info runs before that pass with ctx.zoom_factor() still the old value, so monitor_size (points) is at the old zoom while the frame reads the new one. Moonglow::ui (lib.rs, interface size block) measures commands::largest_fitting_ui_scale at screen_zoom: the zoom of the last pass of the previous frame (cumulative_frame_nr), and in the very first pass at the current zoom. Two wrong tries: comparing with this pass's zoom swung 3.0, 2.0, 1.75, 2.5; waiting for the zoom to hold a frame missed the first frames (300% applied, least size sent past the screen) and a discarded pass within one frame applies the pending zoom, so the pass before is not the measure's zoom either. Kittest tests must h.step() and set monitor_size = pixels / h.ctx.zoom_factor() before each step to mimic egui-winit. eframe persists Memory with Options::zoom_factor: commands::ui_scale_kept_from_zoom maps it to the nearest UI_SCALES once (zoom_kept_seen) when ui_scale is None. ui_scale(app) clamps to 90..=300 (Some(0) panicked in egui layout). shell::keep_least_size compares with commands::ui_scale(app).
