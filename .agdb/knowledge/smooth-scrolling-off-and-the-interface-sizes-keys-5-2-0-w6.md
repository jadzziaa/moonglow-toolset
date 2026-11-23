---
type: Decision
title: "Smooth scrolling off and the interface size's keys (5.2.0 W6)"
description: "How smooth scrolling is turned off without patching egui 0.36 (an egui Plugin input_hook splits notched wheel events into sub-8-point Point events), why the style's scroll animation lands only in the next frame, and why the zoom keys are commands with egui's zoom_with_keyboard off."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T12:29:24Z }
touches: [crates/mg-uikit/src/scrolling.rs, crates/mg-ui/src/commands.rs, crates/mg-ui/src/keys.rs, crates/mg-ui/src/lib.rs]
asks:
  - "How do I turn off egui's mouse wheel smoothing?"
  - "Why does a scroll_to with ScrollAnimation::none still take a frame to land?"
  - "Why are Ctrl+Plus and Ctrl+Minus Moonglow commands and not egui's zoom?"
  - How does Moonglow tell a mouse wheel from a touchpad?
fingerprint: v2-a951f1deb8e2f660
created: 2026-11-23
author: augustaugur
---
egui 0.36 smooths a wheel in WheelState::on_wheel_event/after_events (input_state/wheel_state.rs): an event is added to the frame unsmoothed only when the wheel is InTouch (between TouchPhase Start and End) or it is MouseWheelUnit::Point shorter than 8 points; Line and Page always go through unprocessed_wheel_delta and an exponential (90% in 0.1 s). No option turns it off and InputState::wheel is private, so it cannot be fixed after begin_pass. Bracketing with Start/End does not work: End zeroes the frame's delta, and InTouch keeps modifiers sticky.

What works (mg_uikit::scrolling): an egui::Plugin registered on the Context (ctx.add_plugin, once; its input_hook runs in Context::begin_pass before the input is read, so kittest harnesses exercise it too, unlike eframe's App::raw_input_hook) rewrites each notch into N Point events of at most 5 points summing to line_scroll_speed * lines (or the screen height for Page). A notch = Line of at least one whole line or Page, phase Move, without the zoom modifier. Point events (Wayland and macOS touchpads) and fractional Lines (Windows precision touchpads and X11's smooth valuators, also high-resolution wheels) are left to egui: their limit. Ctrl+wheel is left alone (egui turns it into zoom_delta, smoothed; see the 3D views' wheel note). Code reading raw MouseWheel events (widgets::wheel_pick) only uses the sum's sign, so N events still make one step. The 3D views read smooth_scroll_delta for their wheel zoom: off, it zooms the same total at once.

Style::scroll_animation = ScrollAnimation::none() still lands in the frame after the request: ScrollArea sets offset_target at the end of the frame that asks, and applies it when it next begins. Tests count "the frame that asks and the next". Trap: egui_kittest's Harness sets ScrollAnimation::none() on its context, so a test of an animated scroll passes broken; smooth_scrolling(ctx, true) therefore leaves the context's own animation (egui's default in the app, kittest's none in tests, so existing tests are unchanged) and sets egui's default back only when turned on after off; a test of the animation sets the default on h.ctx first. Also a scroll area applies a wheel delta as it ends a frame: the content shows it the frame after.

The zoom keys: egui's Options::zoom_with_keyboard (on by default, run at end_pass) owned Ctrl +/-/0 and changed the zoom without the setting (and fought ui_scale when set). Now it is turned off each frame in Moonglow::ui and the keys are commands (zoom-in: Ctrl+Plus, Ctrl+=, Ctrl+Shift+Plus because keymaps match modifiers exactly and US keyboards give Shift with +; zoom-out Ctrl+Minus; reset-zoom Ctrl+0, free: script bookmarks are Ctrl+1..9) stepping UI_SCALES and writing settings.ui_scale and the open Options draft's row. View › Interface Size is placed before Hide All Panels and Reset Layout because options_and_keys' menu-key test relies on Reset Layout being View's last row. The sizes are Item::UiScales (drawn ticks), not nine Ids, so the palette and Options › Keyboard get three commands only.
