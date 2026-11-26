//! Timing a window's views against their resting views, as
//! `frame_perf`'s `against` does, in a test of its own that drives the
//! application through `egui_kittest` and times it without the
//! accessibility tree (`nui_perf`, `editor_perf`).

use std::time::{Duration, Instant};

use mg_ui::Moonglow;

/// A frame at 144 a second (as `frame_perf`'s).
pub(crate) const BUDGET: Duration = Duration::from_micros(6_944);
/// How much more interface time a view may take than its rest before it
/// is a cost to find (as `frame_perf`'s `UI_LINE`).
pub(crate) const UI_LINE: Duration = Duration::from_micros(70);

fn micros(d: Duration) -> f64 {
    d.as_secs_f64() * 1e6
}

/// `count` frames of the window as the desktop app runs them (no
/// accessibility tree, the clock a 144th of a second a frame), `each`
/// giving each one's events: the median and the worst of the interface's
/// pass with its tessellation, and how soon the last frame asked for
/// another.
pub(crate) fn frames(
    h: &mut egui_kittest::Harness<'_, Moonglow>,
    count: usize,
    mut each: impl FnMut(usize) -> Vec<egui::Event>,
) -> (Duration, Duration, Duration) {
    let ctx = h.ctx.clone();
    ctx.disable_accesskit();
    let mut clock = ctx.input(|i| i.time);
    let (mut took, mut again) = (Vec::new(), Duration::MAX);
    for n in 0..count {
        clock += 1.0 / 144.0;
        let mut input = h.input().clone();
        input.time = Some(clock);
        input.predicted_dt = 1.0 / 144.0;
        input.events = each(n);
        let app = h.state_mut();
        let t = Instant::now();
        // (As the harness's steps wrap the application, so that what the
        // steps opened is what is timed.)
        let full = ctx.run_ui(input, |ui| {
            ui.scope_builder(egui::UiBuilder::new(), |ui| {
                egui::Frame::central_panel(ui.style())
                    .outer_margin(8.0)
                    .inner_margin(0.0)
                    .show(ui, |ui| app.ui(ui));
            });
        });
        let _ = ctx.tessellate(full.shapes, full.pixels_per_point);
        took.push(t.elapsed());
        again = full
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |v| v.repaint_delay);
    }
    ctx.enable_accesskit();
    took.sort();
    (took[took.len() / 2], *took.last().unwrap(), again)
}

/// The views timed against their rests, and what they print.
pub(crate) struct Views {
    only: Option<String>,
    pub(crate) count: usize,
    pub(crate) lines: Vec<String>,
}

impl Views {
    /// The groups the variable `view` names, `count`'s frames of each
    /// (60 where it is not set).
    pub(crate) fn new(view: &str, count: &str) -> Views {
        Views {
            only: std::env::var(view).ok(),
            count: std::env::var(count).ok().and_then(|n| n.parse().ok()).unwrap_or(60),
            lines: Vec::new(),
        }
    }

    /// Whether the group `what` is timed (the variable's part of a name).
    pub(crate) fn wanted(&self, what: &str) -> bool {
        self.only.as_ref().is_none_or(|o| what.contains(o.as_str()))
    }

    /// Idle frames of what is shown now, timed twice (the two the noise).
    pub(crate) fn rest(
        &mut self,
        h: &mut egui_kittest::Harness<'_, Moonglow>,
    ) -> (Duration, Duration) {
        let a = frames(h, self.count, |_| Vec::new()).0;
        let b = frames(h, self.count, |_| Vec::new()).0;
        (a, b)
    }

    /// Frames of the view shown now (`each` their events) against `rest`:
    /// the line, with its verdict.
    pub(crate) fn against(
        &mut self,
        h: &mut egui_kittest::Harness<'_, Moonglow>,
        what: &str,
        rest: (Duration, Duration),
        each: impl FnMut(usize) -> Vec<egui::Event>,
    ) {
        let (median, worst, again) = frames(h, self.count, each);
        let at_rest = rest.0.min(rest.1);
        let by = micros(median) - micros(at_rest);
        let noise = (micros(rest.0) - micros(rest.1)).abs();
        let verdict = if median > BUDGET || by > micros(UI_LINE) { "find" } else { "within" };
        let again = match again {
            Duration::ZERO => "at once".to_owned(),
            Duration::MAX => "never".to_owned(),
            d => format!("in {:.0} ms", d.as_secs_f64() * 1000.0),
        };
        let line = format!(
            "{what}: ui {:.3} ms (worst {:.3}, the next asked for {again}), at rest {:.3} \
             (its two runs {noise:.0} µs apart): {by:+.0} µs, {verdict}",
            median.as_secs_f64() * 1000.0,
            worst.as_secs_f64() * 1000.0,
            at_rest.as_secs_f64() * 1000.0,
        );
        println!("  {line}");
        self.lines.push(line);
    }
}
