//! Smooth scrolling, on or off for a whole window ([`smooth_scrolling`]).
//!
//! egui eases two kinds of scrolling over a few frames: a mouse wheel's
//! notch (its `WheelState` spreads a notch's lines over about a tenth of a
//! second, a smoothing no option of egui 0.36 turns off) and a jump made in
//! code (`scroll_to_rect`, `scroll_to_me`: `Style::scroll_animation`).
//! Off, both land at once: the style's animation is none, and a plugin of
//! the context ([`egui::Plugin::input_hook`], run before the frame reads its
//! input) rewrites each notch's wheel event into the form egui applies
//! whole, in the frame it arrives. That form is a run of point deltas each
//! shorter than egui's 8 points, which egui takes as a touchpad's and adds
//! to the frame's scroll as they are; together they move as far as the
//! notch would have.
//!
//! What is a notch: a wheel event in whole lines (or pages). A touchpad's
//! scroll, in points (Wayland, macOS) or in fractions of a line (Windows'
//! and X11's emulated wheel, a fine-step wheel), is left to egui, with its
//! momentum, as is any wheel with Ctrl (or Cmd) held, which egui makes a
//! zoom (`InputState::zoom_delta`). A fine-step stream is left to egui
//! whole: once a fraction of a line has come, whole lines too until the
//! wheel has rested for [`FINE_REST`] (a fine-step wheel sends 1.0 lines
//! now and then, which given at once among smoothed fractions would
//! jerk). Smoothing it all is the safe side: a fraction cannot be given
//! unsmoothed without egui's smoothing turned off for the touchpads too.

/// Turns smooth scrolling on (egui's own, the default) or off for every
/// view of `ctx`, from the next frame. Cheap to call every frame: it
/// changes the style only when it differs, and the plugin is added once.
///
/// On, the style's scroll animation is left as the context has it (egui's
/// own, or kittest's none, which tests rely on), and set back to egui's
/// own only when smooth scrolling is turned on again after being off.
pub fn smooth_scrolling(ctx: &egui::Context, on: bool) {
    let was_off = !is_smooth(ctx);
    let now = ctx.global_style().scroll_animation;
    if !on && !same(&now, &none()) {
        ctx.all_styles_mut(|s| s.scroll_animation = none());
    } else if on && was_off {
        let smooth = egui::style::ScrollAnimation::default();
        ctx.all_styles_mut(|s| s.scroll_animation = smooth);
    }
    match ctx.plugin_opt::<WheelSteps>() {
        Some(plugin) => plugin.lock().whole = !on,
        None => ctx.add_plugin(WheelSteps { whole: !on, fine_at: None }),
    }
}

/// Whether `ctx`'s scrolling is smooth (egui's), as [`smooth_scrolling`]
/// last set it: `true` where it never was.
pub fn is_smooth(ctx: &egui::Context) -> bool {
    ctx.plugin_opt::<WheelSteps>().is_none_or(|p| !p.lock().whole)
}

/// A scroll animation that lands at once.
fn none() -> egui::style::ScrollAnimation {
    egui::style::ScrollAnimation::none()
}

fn same(a: &egui::style::ScrollAnimation, b: &egui::style::ScrollAnimation) -> bool {
    a.points_per_second == b.points_per_second && a.duration == b.duration
}

/// The longest point delta egui adds to a frame's scroll unsmoothed is
/// under 8 points (`WheelState::on_wheel_event`); a step of a notch is
/// kept well below it, so that Alt's turn of a sideways step into a
/// vertical one (x + y) stays below it too.
const STEP: f32 = 5.0;

/// How long a wheel rests after a fraction of a line before its whole
/// lines are notches again (seconds).
const FINE_REST: f64 = 0.5;

/// The context's plugin that gives a notch's scroll whole, while smooth
/// scrolling is off.
struct WheelSteps {
    /// Smooth scrolling is off: notches are rewritten.
    whole: bool,
    /// When the last fraction of a line came (the input's time): a
    /// fine-step stream, left to egui until it rests.
    fine_at: Option<f64>,
}

impl egui::Plugin for WheelSteps {
    fn debug_name(&self) -> &'static str {
        "mg-uikit wheel steps"
    }

    fn input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        let now = input.time.unwrap_or_else(|| ctx.input(|i| i.time));
        if input.events.iter().any(is_fraction) {
            self.fine_at = Some(now);
        }
        let fine = self.fine_at.is_some_and(|at| now - at < FINE_REST);
        if !self.whole || fine || !input.events.iter().any(is_notch_of(ctx)) {
            return;
        }
        let (line, zoom) =
            ctx.options(|o| (o.input_options.line_scroll_speed, o.input_options.zoom_modifier));
        let page = input.screen_rect.unwrap_or_else(|| ctx.viewport_rect()).height();
        let events = std::mem::take(&mut input.events);
        for event in events {
            match event {
                egui::Event::MouseWheel { unit, delta, phase, modifiers }
                    if is_notch(unit, delta, phase) && !modifiers.matches_any(zoom) =>
                {
                    let points = match unit {
                        egui::MouseWheelUnit::Page => delta * page,
                        _ => delta * line,
                    };
                    let n = (points.length() / STEP).ceil().max(1.0);
                    let step = points / n;
                    for _ in 0..n as usize {
                        input.events.push(egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: step,
                            phase,
                            modifiers,
                        });
                    }
                }
                other => input.events.push(other),
            }
        }
    }
}

/// Whether an event is a notch's (see the module's words), Ctrl's zoom not
/// counted out yet.
fn is_notch_of(ctx: &egui::Context) -> impl Fn(&egui::Event) -> bool {
    let zoom = ctx.options(|o| o.input_options.zoom_modifier);
    move |e| match e {
        egui::Event::MouseWheel { unit, delta, phase, modifiers } => {
            is_notch(*unit, *delta, *phase) && !modifiers.matches_any(zoom)
        }
        _ => false,
    }
}

fn is_notch(unit: egui::MouseWheelUnit, delta: egui::Vec2, phase: egui::TouchPhase) -> bool {
    phase == egui::TouchPhase::Move
        && delta != egui::Vec2::ZERO
        && match unit {
            egui::MouseWheelUnit::Point => false,
            egui::MouseWheelUnit::Line => whole(delta.x) && whole(delta.y),
            egui::MouseWheelUnit::Page => true,
        }
}

/// A number of whole lines (a notch's 1, a fast wheel's 3; not 0.5 or 1.5).
fn whole(lines: f32) -> bool {
    (lines - lines.round()).abs() < 0.001
}

/// A wheel's event in a fraction of a line: a fine-step wheel's or a
/// touchpad's.
fn is_fraction(e: &egui::Event) -> bool {
    matches!(e, egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta, .. }
        if !whole(delta.x) || !whole(delta.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notch(
        unit: egui::MouseWheelUnit,
        delta: egui::Vec2,
        modifiers: egui::Modifiers,
    ) -> egui::Event {
        egui::Event::MouseWheel { unit, delta, phase: egui::TouchPhase::Move, modifiers }
    }

    /// One frame of `ctx` at `time` with `events`: the frame's scroll.
    fn frame(ctx: &egui::Context, time: f64, events: Vec<egui::Event>) -> (egui::Vec2, f32) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            time: Some(time),
            predicted_dt: 1.0 / 60.0,
            events,
            ..Default::default()
        };
        let mut seen = (egui::Vec2::ZERO, 1.0);
        let mut out = ctx.run_ui(input, |ui| {
            seen = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
        });
        out.textures_delta.clear();
        seen
    }

    /// The scroll of a notch down, frame by frame for `frames` frames
    /// (the first with the notch).
    fn scrolled(on: bool, event: egui::Event, frames: usize) -> Vec<f32> {
        let ctx = egui::Context::default();
        frame(&ctx, 0.0, Vec::new());
        smooth_scrolling(&ctx, on);
        let mut out = Vec::new();
        for f in 0..frames {
            let events = if f == 0 { vec![event.clone()] } else { Vec::new() };
            let (d, _) = frame(&ctx, (f + 1) as f64 / 60.0, events);
            out.push(d.y + d.x);
        }
        out
    }

    /// Off, a wheel's notch (a line, 40 points) moves the content its
    /// whole way in the frame it arrives, and nothing after; on (egui's
    /// own), over several frames to the same sum.
    #[test]
    fn a_notch_scrolls_whole_at_once_when_smooth_scrolling_is_off() {
        let line = egui::InputOptions::default().line_scroll_speed;
        let down = notch(egui::MouseWheelUnit::Line, egui::vec2(0.0, -1.0), egui::Modifiers::NONE);
        let off = scrolled(false, down.clone(), 30);
        assert!((off[0] + line).abs() < 0.01, "the whole notch at once: {off:?}");
        assert!(off[1..].iter().all(|d| *d == 0.0), "nothing after: {off:?}");
        let on = scrolled(true, down, 30);
        assert!((on[0] + line).abs() > 1.0, "smoothed, not whole at once: {on:?}");
        assert!(on[1] != 0.0, "and goes on: {on:?}");
        let sum: f32 = on.iter().sum();
        assert!((sum + line).abs() < 1.0, "to the same sum: {on:?}");
        // Shift's sideways scroll too, whole.
        let shift =
            notch(egui::MouseWheelUnit::Line, egui::vec2(0.0, -1.0), egui::Modifiers::SHIFT);
        let ctx = egui::Context::default();
        frame(&ctx, 0.0, Vec::new());
        smooth_scrolling(&ctx, false);
        let (d, _) = frame(&ctx, 1.0 / 60.0, vec![shift]);
        assert!((d.x + line).abs() < 0.01 && d.y == 0.0, "sideways, whole: {d:?}");
        // Three notches in one frame: three lines.
        let ctx = egui::Context::default();
        frame(&ctx, 0.0, Vec::new());
        smooth_scrolling(&ctx, false);
        let three = notch(egui::MouseWheelUnit::Line, egui::vec2(0.0, 3.0), egui::Modifiers::NONE);
        let (d, _) = frame(&ctx, 1.0 / 60.0, vec![three]);
        assert!((d.y - 3.0 * line).abs() < 0.01, "{d:?}");
    }

    /// A touchpad's scroll (points, a fraction of a line) and Ctrl's zoom
    /// are egui's own with smooth scrolling off too.
    #[test]
    fn a_touchpad_and_ctrl_and_the_wheel_are_left_to_egui() {
        let line = egui::InputOptions::default().line_scroll_speed;
        // (Points of 20: egui's own smoothing, as with smooth scrolling on.)
        for unit_delta in [
            (egui::MouseWheelUnit::Point, egui::vec2(0.0, -20.0)),
            (egui::MouseWheelUnit::Line, egui::vec2(0.0, -0.5)),
        ] {
            let event = notch(unit_delta.0, unit_delta.1, egui::Modifiers::NONE);
            assert_eq!(
                scrolled(false, event.clone(), 10),
                scrolled(true, event, 10),
                "{unit_delta:?}"
            );
        }
        let zoom =
            notch(egui::MouseWheelUnit::Line, egui::vec2(0.0, 1.0), egui::Modifiers::COMMAND);
        let zooms = |on: bool| {
            let ctx = egui::Context::default();
            frame(&ctx, 0.0, Vec::new());
            smooth_scrolling(&ctx, on);
            (0..10)
                .map(|f| {
                    frame(
                        &ctx,
                        (f + 1) as f64 / 60.0,
                        if f == 0 { vec![zoom.clone()] } else { Vec::new() },
                    )
                    .1
                })
                .collect::<Vec<f32>>()
        };
        assert_eq!(zooms(false), zooms(true));
        assert!(zooms(false)[0] > 1.0, "a zoom: {:?} ({line})", zooms(false));
    }

    /// Off, a fine-step wheel (fractions of a line, and a whole line now
    /// and then) is egui's own throughout, as with smooth scrolling on: no
    /// whole line given at once among smoothed fractions. Rested, a
    /// notch is whole again.
    #[test]
    fn a_fine_step_wheel_is_left_to_egui_whole() {
        let line =
            |d: f32| notch(egui::MouseWheelUnit::Line, egui::vec2(0.0, d), egui::Modifiers::NONE);
        let stream: [f32; 6] = [0.4, 0.6, 1.0, 0.5, 1.0, 0.3];
        let run = |on: bool| {
            let ctx = egui::Context::default();
            frame(&ctx, 0.0, Vec::new());
            smooth_scrolling(&ctx, on);
            let mut out = Vec::new();
            for f in 0..40 {
                let events = stream.get(f).map(|d| vec![line(-*d)]).unwrap_or_default();
                out.push(frame(&ctx, (f + 1) as f64 / 60.0, events).0.y);
            }
            // (Rested: a notch is whole at once.)
            let (d, _) = frame(&ctx, 2.0, vec![line(-1.0)]);
            (out, d.y)
        };
        let (off, rested) = run(false);
        assert_eq!(off, run(true).0, "smoothed alike");
        let line_speed = egui::InputOptions::default().line_scroll_speed;
        assert!((rested + line_speed).abs() < 0.01, "a notch after a rest: {rested}");
        // (1.5 lines is a fraction too, not a notch.)
        assert!(!is_notch(
            egui::MouseWheelUnit::Line,
            egui::vec2(0.0, 1.5),
            egui::TouchPhase::Move
        ));
    }

    /// Off, the style's scroll animation is none; on again, egui's. On from
    /// the start, the context's own is left as it is (kittest's none).
    #[test]
    fn the_scroll_animation_follows_the_switch() {
        let ctx = egui::Context::default();
        let own = egui::style::ScrollAnimation::duration(0.5);
        ctx.all_styles_mut(|s| s.scroll_animation = own);
        smooth_scrolling(&ctx, true);
        assert!(same(&ctx.global_style().scroll_animation, &own), "the context's own");
        assert!(is_smooth(&ctx));
        smooth_scrolling(&ctx, false);
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            assert!(same(&ctx.style_of(theme).scroll_animation, &none()), "{theme:?}");
        }
        assert!(!is_smooth(&ctx));
        smooth_scrolling(&ctx, true);
        let default = egui::style::ScrollAnimation::default();
        assert!(same(&ctx.global_style().scroll_animation, &default));
        assert!(is_smooth(&ctx));
    }
}
