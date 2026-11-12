//! The Status Bar completed (the UI redesign's shell, M3's T7): the log's
//! last line beside the Log switch while the Bottom Dock is closed, a
//! running job at the right, the Bottom Dock's height kept, a new
//! installation's window, and the bar at every width from the least
//! window's.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::common::*;
use crate::shell::{bar_controls, drawn_texts};

/// The application with no module and no game, the Bottom Dock closed
/// and the log empty.
fn closed_log_app() -> Moonglow {
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.log.clear();
    app
}

/// The node named `name` in the Status Bar, if it is there.
fn in_bar<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> Option<egui_kittest::Node<'t>> {
    h.get_by_label("Status Bar").query_by_label(name)
}

/// The color the text drawn inside `rect` was painted in, the last frame.
fn text_color(h: &Harness<'_, Moonglow>, rect: egui::Rect) -> Option<egui::Color32> {
    drawn(h).into_iter().find_map(|s| match s {
        egui::Shape::Text(t) if rect.expand(1.0).contains_rect(t.visual_bounding_rect()) => {
            Some(t.fallback_color)
        }
        _ => None,
    })
}

/// How soon the window asks to be drawn again after its last frame.
fn again(h: &Harness<'_, Moonglow>) -> Duration {
    let out = &h.output().viewport_output;
    out.get(&egui::ViewportId::ROOT).map_or(Duration::MAX, |v| v.repaint_delay)
}

/// Frames until `found` says yes (a job's thread or a layout coming to
/// rest), for at most half a minute.
fn wait_for(
    h: &mut Harness<'_, Moonglow>,
    what: &str,
    found: impl Fn(&Harness<'_, Moonglow>) -> bool,
) {
    let started = Instant::now();
    while !found(h) {
        assert!(started.elapsed() < Duration::from_secs(30), "never: {what}");
        std::thread::sleep(Duration::from_millis(10));
        h.step();
    }
}

/// While the Bottom Dock is closed, the log's last line follows the Log
/// switch (its message, a button named by it); a new line takes its
/// place; opened, the log has it and the Status Bar not; closed again,
/// it is back.
#[test]
fn the_log_s_last_line_shows_while_the_log_is_closed() {
    let mut app = closed_log_app();
    app.log.info("First thing said");
    let mut h = harness(app, 1200.0, 700.0);
    settle(&mut h);
    let line = in_bar(&h, "First thing said").expect("the last line");
    assert_eq!(line.accesskit_node().role(), egui::accesskit::Role::Button);
    let switch = in_bar(&h, "Log").unwrap().rect();
    let shown = line.rect();
    assert!(shown.left() >= switch.right() + 7.5, "after the switch: {shown:?} {switch:?}");
    h.state_mut().log.warn("Second thing said");
    settle(&mut h);
    assert!(in_bar(&h, "First thing said").is_none() && in_bar(&h, "Second thing said").is_some());
    in_bar(&h, "Log").unwrap().click();
    settle(&mut h);
    assert!(!h.state().settings.hide_log && h.query_by_label("Bottom Dock").is_some());
    assert!(in_bar(&h, "Second thing said").is_none(), "not while the log shows it");
    in_bar(&h, "Log").unwrap().click();
    settle(&mut h);
    assert!(in_bar(&h, "Second thing said").is_some(), "back once closed");
}

/// The last line is in the log's color for its level, by the theme's
/// tokens: a plain line `text-3` (the board's `.sb-log`), a warning
/// `warn` and an error `error`, in both themes; it is named by the whole
/// message, its time and message in its tip.
#[test]
fn the_last_line_is_in_its_level_s_color() {
    let mut h = harness(closed_log_app(), 1200.0, 700.0);
    for light in [false, true] {
        h.state_mut().settings.light_theme = light;
        settle(&mut h);
        let t = mg_uikit::tokens(&h.ctx);
        for (level, said, token) in [
            (mg_ui::Level::Info, "A plain line", t.text_3),
            (mg_ui::Level::Warning, "A warning line", t.warn),
            (mg_ui::Level::Error, "An error line", t.error),
        ] {
            match level {
                mg_ui::Level::Info => h.state_mut().log.info(said),
                mg_ui::Level::Warning => h.state_mut().log.warn(said),
                mg_ui::Level::Error => h.state_mut().log.error(said),
            }
            // (The pointer away: a plain line under it is brighter.)
            h.hover_at(egui::pos2(1.0, 1.0));
            settle(&mut h);
            let rect = in_bar(&h, said).expect(said).rect();
            assert_eq!(text_color(&h, rect), Some(token), "{said}, light {light}");
            let tip = tip_at(&mut h, rect).unwrap_or_default();
            let log = &h.state().log;
            assert_eq!(tip, log.shown(log.entries.len() - 1), "{said}: the whole line");
        }
    }
}

/// A line longer than the room left beside the right-hand items is cut
/// short with an ellipsis, never under them, named by its whole message
/// and whole in its tip; with room, whole. A line of several lines shows
/// its first.
#[test]
fn a_long_last_line_is_cut_short_with_the_whole_line_in_its_tip() {
    let long = "Verify Module found 12 problems in the module's scripts, areas, \
                blueprints and conversations; see the log for each one of them";
    for width in [1600.0, 900.0, 640.0] {
        let mut app = closed_log_app();
        app.log.info(long);
        let mut h = harness(app, width, 600.0);
        settle(&mut h);
        let line = in_bar(&h, long).expect("the last line").rect();
        let name = in_bar(&h, "No module open").unwrap().rect();
        assert!(line.right() <= name.left() - 8.0, "{width}: {line:?} before {name:?}");
        let words = drawn_texts(&h, line);
        assert_eq!(words.len(), 1, "{width}: {words:?}");
        if width >= 1600.0 {
            assert_eq!(words[0], long, "whole where it fits");
        } else {
            assert!(words[0].ends_with('…') && words[0].starts_with("Verify"), "{words:?}");
            assert!(words[0].chars().count() < long.chars().count());
        }
        let tip = tip_at(&mut h, line).unwrap_or_default();
        assert!(tip.ends_with(long), "{width}: the whole line in its tip: {tip}");
    }
    let mut app = closed_log_app();
    app.log.error("The first line\nand the second");
    let mut h = harness(app, 1200.0, 600.0);
    settle(&mut h);
    let line = in_bar(&h, "The first line\nand the second").expect("named by it all").rect();
    assert_eq!(drawn_texts(&h, line), ["The first line"]);
}

/// Pressing the last line opens the log, as the switch does.
#[test]
fn pressing_the_last_line_opens_the_log() {
    let mut app = closed_log_app();
    app.log.info("Something said");
    let mut h = harness(app, 1200.0, 700.0);
    settle(&mut h);
    in_bar(&h, "Something said").unwrap().click();
    settle(&mut h);
    assert!(!h.state().settings.hide_log, "opened");
    // (A log line's words are its value, its time first.)
    let said = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.value().is_some_and(|v| v.contains(" Something said"))
    };
    assert!(h.query_all_by(said).count() >= 1, "the log shows it");
    assert!(in_bar(&h, "Something said").is_none());
}

/// With nothing said yet, nothing follows the switch and no room is kept
/// for it: the bar's controls are the switch and the right-hand items,
/// and at every width from 420 to 1000 the bar is as it is with the log
/// open (where it has no last line), the game folder kept as long; a line
/// said then takes the room between them alone.
#[test]
fn with_no_line_yet_nothing_is_shown_or_kept() {
    let dir = mg_testkit::scratch_dir("ui-status-no-line");
    let game = dir.join("Neverwinter Nights Enhanced Edition on the second drive of this machine");
    std::fs::create_dir_all(&game).unwrap();
    let mut app = game_app(&game, None);
    app.settings.hide_log = true;
    let mut h = harness(app, 1200.0, 700.0);
    settle(&mut h);
    h.state_mut().log.clear();
    settle(&mut h);
    let names: Vec<String> = bar_controls(&h, "Status Bar").into_iter().map(|c| c.0).collect();
    assert_eq!(
        names,
        [
            "Log",
            "No module open",
            "…/Neverwinter Nights Enhanced Edition on the second drive of this machine"
        ]
    );
    let folder = "…/Neverwinter Nights Enhanced Edition on the second drive of this machine";
    let folder_width = in_bar(&h, folder).unwrap().rect().width();
    let mut differ = Vec::new();
    let mut dropped = 0;
    for width in (420..=1000).step_by(8) {
        let width = width as f32;
        h.set_size(egui::vec2(width, 700.0));
        let mut seen = Vec::new();
        for closed in [true, false] {
            h.state_mut().settings.hide_log = closed;
            settle(&mut h);
            seen.push(bar_controls(&h, "Status Bar"));
        }
        if seen[0] != seen[1] {
            differ.push(format!("{width}: {:?} against {:?}", seen[0], seen[1]));
        }
        // The folder left out only where it has no room beside the name:
        // nothing kept for a line that is not there.
        let has = |n: &str| seen[0].iter().find(|c| c.0 == n).map(|c| c.1);
        if has(folder).is_none() {
            dropped += 1;
            let (switch, name) = (has("Log").unwrap(), has("No module open").unwrap());
            // (The room between them: the gaps, a separator and the folder.)
            let room = name.left() - switch.right() - 2.0 * 8.0;
            if room >= folder_width + 1.0 + 2.0 * 8.0 {
                differ.push(format!("{width}: the folder left out with {room} of room"));
            }
        }
    }
    assert!(differ.is_empty(), "{differ:#?}");
    assert!(dropped > 0, "the folder goes somewhere in the sweep");
    h.set_size(egui::vec2(1200.0, 700.0));
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    let at = |h: &Harness<'_, Moonglow>| {
        (in_bar(h, "Log").unwrap().rect(), in_bar(h, "No module open").unwrap().rect())
    };
    let without = at(&h);
    h.state_mut().log.info("Now something");
    settle(&mut h);
    assert_eq!(at(&h), without, "the line takes the room between them alone");
}

/// The Bottom Dock keeps the height it was given through the switch
/// (closed and opened), a restart (its settings read again) and comes
/// back at its first height after View › Reset Layout.
#[test]
fn the_bottom_dock_keeps_its_height_through_the_switch_and_a_restart() {
    let mut h = harness(app_with(Vec::new()), 1000.0, 700.0);
    settle(&mut h);
    let top = |h: &Harness<'_, Moonglow>| {
        h.query_by_label("Bottom Dock").map(|d| d.get_by_label("Log").rect().top())
    };
    let was = top(&h).expect("open (settings from before)");
    drag(&mut h, egui::pos2(500.0, was - 1.0), egui::pos2(500.0, was - 81.0));
    settle(&mut h);
    let high = top(&h).unwrap();
    assert!(high < was - 60.0, "raised: {was} -> {high}");
    let kept = h.state().settings.bottom_dock_height;
    in_bar(&h, "Log").unwrap().click();
    settle(&mut h);
    assert_eq!(top(&h), None);
    in_bar(&h, "Log").unwrap().click();
    settle(&mut h);
    assert_eq!(top(&h), Some(high), "as high again");
    assert_eq!(h.state().settings.bottom_dock_height, kept);
    // A restart: the settings as kept, a window of its own.
    let settings = h.state().settings.clone();
    let app = Moonglow::with_settings(settings, Box::new(NoDialogs::default()));
    let mut again = harness(app, 1000.0, 700.0);
    settle(&mut again);
    assert_eq!(top(&again), Some(high), "as high after a restart");
    menu_command(&mut again, "View", "Reset Layout");
    settle(&mut again);
    let first = mg_uikit::metrics::BOTTOM_DOCK.round() as u32;
    assert_eq!(again.state().settings.bottom_dock_height, Some(first));
    assert!((top(&again).unwrap() - was).abs() <= 1.0, "its first height again");
}

/// What a test job is told to do, one step at a time.
enum Step {
    /// Say how far it is (done, total).
    Progress(usize, usize),
    /// End, done.
    End,
    /// Fail (its thread panics).
    Fail,
}

/// A job titled `title` held on its thread until told: its steps sent.
fn held_job(h: &mut Harness<'_, Moonglow>, title: &str) -> mpsc::Sender<Step> {
    let (tell, told) = mpsc::channel::<Step>();
    h.state_mut().start_job_for_tests(title, move |progress| {
        for step in told {
            match step {
                Step::Progress(done, total) => progress.step(done, total),
                Step::End => return,
                Step::Fail => panic!("it broke"),
            }
        }
    });
    tell
}

/// A running job shows at the Status Bar's right, before the module's
/// name: its title and a bar, named with how far it is ("…, 3 of 10", its
/// part done the value) or "under way" where that is not known; it goes
/// when the job ends (the log's last line then saying so), or fails (the
/// failure the last line, in the error's color). A second job asked for
/// while one runs is refused, the first still shown.
#[test]
fn a_running_job_shows_in_the_status_bar_until_it_ends() {
    let mut h = harness(closed_log_app(), 1400.0, 700.0);
    settle(&mut h);
    let job = |h: &Harness<'_, Moonglow>| {
        h.get_by_label("Status Bar")
            .query_by(|n| n.role() == egui::accesskit::Role::ProgressIndicator)
            .map(|n| {
                let a = n.accesskit_node();
                (a.label().unwrap_or_default(), a.numeric_value(), n.rect())
            })
    };
    for fail in [false, true] {
        let tell = held_job(&mut h, "Count Things");
        wait_for(&mut h, "the job shown", |h| job(h).is_some());
        let (name, value, rect) = job(&h).unwrap();
        assert_eq!((name.as_str(), value), ("Count Things, under way", None));
        let module = in_bar(&h, "No module open").unwrap().rect();
        assert!(rect.right() <= module.left() - 8.0, "before the module: {rect:?} {module:?}");
        assert!(drawn_texts(&h, rect).iter().any(|w| w == "Count Things"), "its title drawn");
        // (Its moving mark, and the job's own window, ask for frames.)
        assert!(again(&h) <= Duration::from_millis(50), "{:?}", again(&h));
        tell.send(Step::Progress(3, 10)).unwrap();
        wait_for(&mut h, "3 of 10", |h| job(h).is_some_and(|j| j.0 == "Count Things, 3 of 10"));
        assert_eq!(job(&h).unwrap().1, Some(f64::from(0.3f32)));
        // One at a time: another is refused, this one still shown.
        h.state_mut().start_job_for_tests("Another Job", |_| ());
        h.step();
        assert_eq!(job(&h).unwrap().0, "Count Things, 3 of 10");
        let refused = "Another Job: waiting for Count Things to finish";
        assert!(in_bar(&h, refused).is_some(), "the refusal is the last line");
        tell.send(if fail { Step::Fail } else { Step::End }).unwrap();
        wait_for(&mut h, "the job's end", |h| !h.state().busy());
        settle(&mut h);
        assert!(job(&h).is_none(), "gone with the job");
        let said = if fail { "Count Things failed: it broke" } else { "Count Things: done" };
        let rect = in_bar(&h, said).unwrap_or_else(|| panic!("{said}")).rect();
        let t = mg_uikit::tokens(&h.ctx);
        h.hover_at(egui::pos2(1.0, 1.0));
        settle(&mut h);
        let color = if fail { t.error } else { t.text_3 };
        assert_eq!(text_color(&h, rect), Some(color), "{said}");
        // Nothing asks for frames once it is over.
        h.run_steps(2);
        assert!(again(&h) >= Duration::from_secs(1), "at rest: {:?}", again(&h));
    }
}

/// An idle window asks for no more frames with the log's last line shown
/// than with the log open.
#[test]
fn the_last_line_asks_for_no_frames() {
    let mut app = app_with(Vec::new());
    app.log.info("Said once");
    let mut h = harness(app, 1200.0, 700.0);
    settle(&mut h);
    h.run_steps(2);
    let open = again(&h);
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    h.run_steps(2);
    assert!(in_bar(&h, "Said once").is_some());
    assert!(again(&h) >= open, "{:?} against {open:?}", again(&h));
    assert!(again(&h) >= Duration::from_secs(1), "{:?}", again(&h));
}

/// A new installation's window (no settings file: the settings
/// `Settings::new_installation` gives) opens with the Bottom Dock closed,
/// the log's last line in the Status Bar; settings that exist and do not
/// name the dock open it as before.
#[test]
fn a_new_installation_opens_with_the_log_closed() {
    let dir = mg_testkit::scratch_dir("ui-new-installation");
    let file = dir.join("app.ron");
    let read = mg_ui::Settings::read_eframe(&file, "moonglow-settings");
    assert_eq!(read.settings, mg_ui::Settings::new_installation(), "no file: a new installation");
    let app = Moonglow::with_settings(read.settings, Box::new(NoDialogs::default()));
    let mut h = harness(app, 1200.0, 700.0);
    settle(&mut h);
    assert!(h.query_by_label("Bottom Dock").is_none(), "the Bottom Dock closed");
    let switch = in_bar(&h, "Log").unwrap().accesskit_node().toggled();
    assert_eq!(switch, Some(egui::accesskit::Toggled::False));
    let last = h.state().log.entries.last().map(|e| e.1.clone()).expect("a line said");
    assert!(in_bar(&h, &last).is_some(), "its last line beside the switch: {last}");
    // A file that does not name it: the log open, as an existing user's.
    let kv = std::collections::HashMap::from([(
        "moonglow-settings".to_string(),
        "(light_theme:false)".to_string(),
    )]);
    std::fs::write(&file, ron::to_string(&kv).unwrap()).unwrap();
    let read = mg_ui::Settings::read_eframe(&file, "moonglow-settings");
    let app = Moonglow::with_settings(read.settings, Box::new(NoDialogs::default()));
    let mut h = harness(app, 1200.0, 700.0);
    settle(&mut h);
    assert!(h.query_by_label("Bottom Dock").is_some(), "the log open");
}

/// The Status Bar at every width from the least window's (420) to 1600,
/// in 8 point steps (and at 1600), with a long module name (modified), a long last
/// line, a running job and a long game folder: every node of the bar
/// inside it and none over another, every text drawn inside the bar and
/// over no other; the module's name always there, everything at 1600;
/// and they yield in order as the window narrows: the last line first,
/// then the game folder, then the job's name (cut short), the module's
/// name last.
#[test]
fn the_status_bar_holds_at_every_width() {
    let dir = mg_testkit::scratch_dir("ui-status-sweep");
    let path = sample_module(&dir);
    let long = dir.join("a_module_with_a_rather_long_file_name_for_the_status_bar_check.mod");
    std::fs::copy(&path, &long).unwrap();
    let game = dir.join("Neverwinter Nights Enhanced Edition on the second drive");
    std::fs::create_dir_all(&game).unwrap();
    let mut app = game_app(&game, None);
    app.settings.no_last_area = true;
    app.settings.hide_log = true;
    app.open_module(&long);
    let edit = mg_edit::Edit::SetResource {
        key: ResKey::parse("added", ResType::NSS).unwrap(),
        data: Some(b"void main() { }\n".to_vec()),
    };
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Add", vec![edit])));
    let line = "Compiled 312 scripts of the module and its haks, with 4 warnings about \
                functions declared one way and defined another";
    let name = "a_module_with_a_rather_long_file_name_for_the_status_bar_check (modified)";
    let folder = "…/Neverwinter Nights Enhanced Edition on the second drive";
    let job_title = "Compile All Scripts";
    let mut h = harness(app, 1600.0, 600.0);
    settle(&mut h);
    h.state_mut().log.info(line);
    let tell = held_job(&mut h, job_title);
    tell.send(Step::Progress(40, 312)).unwrap();
    let job_name = "Compile All Scripts, 40 of 312";
    wait_for(&mut h, "the job shown", |h| in_bar(h, job_name).is_some());
    // (Each rect of the bar's nodes, and each text drawn in it.)
    let seen = |h: &Harness<'_, Moonglow>| {
        // (The bar: the window's width, 24 points high round the switch.)
        let window = h.ctx.content_rect();
        let switch = in_bar(h, "Log").unwrap().rect();
        let half = mg_uikit::metrics::STATUS_BAR / 2.0;
        let bar = egui::Rect::from_x_y_ranges(
            window.x_range(),
            switch.center().y - half..=switch.center().y + half,
        );
        let texts: Vec<egui::Rect> = drawn(h)
            .into_iter()
            .filter_map(|s| match s {
                egui::Shape::Text(t) => Some(t.visual_bounding_rect()),
                _ => None,
            })
            .filter(|r| r.center().y > bar.top() && r.center().y < bar.bottom())
            .collect();
        (bar, bar_controls(h, "Status Bar"), texts)
    };
    let mut full_job = 0.0;
    let mut wrong = Vec::new();
    // (1600 first, where everything shows, then from the least window up.)
    let widths = std::iter::once(1600).chain((420..1600).step_by(8));
    for width in widths {
        let width = width as f32;
        h.set_size(egui::vec2(width, 600.0));
        // (Until two frames lay the bar out alike: a job runs, so the
        // window never rests.)
        let started = Instant::now();
        let mut last = String::new();
        loop {
            h.step();
            let now = format!("{:?}", seen(&h).1);
            if now == last {
                break;
            }
            last = now;
            assert!(started.elapsed() < Duration::from_secs(30), "{width}: never at rest");
        }
        let (bar, nodes, texts) = seen(&h);
        let at = |what: &str| format!("{width}: {what}");
        let window = h.ctx.content_rect();
        for (n, r) in &nodes {
            if !bar.expand(0.5).contains_rect(*r) || !window.contains_rect(*r) {
                wrong.push(at(&format!("{n} {r:?} outside the bar {bar:?}")));
            }
        }
        for (i, (a, ra)) in nodes.iter().enumerate() {
            for (b, rb) in &nodes[i + 1..] {
                if ra.intersect(*rb).is_positive() && ra.intersect(*rb).area() > 1.0 {
                    wrong.push(at(&format!("{a} {ra:?} over {b} {rb:?}")));
                }
            }
        }
        for (i, a) in texts.iter().enumerate() {
            if !bar.expand(1.0).contains_rect(*a) {
                wrong.push(at(&format!("a text {a:?} outside the bar")));
            }
            for b in &texts[i + 1..] {
                if a.intersect(*b).is_positive() && a.intersect(*b).area() > 1.0 {
                    wrong.push(at(&format!("texts {a:?} and {b:?} overlap")));
                }
            }
        }
        let has = |n: &str| nodes.iter().find(|c| c.0 == n).map(|c| c.1);
        if has(name).is_none() {
            wrong.push(at("the module's name is gone"));
        }
        let job = has(job_name);
        if width >= 1600.0 {
            for n in ["Log", line, job_name, name, folder] {
                if has(n).is_none() {
                    wrong.push(at(&format!("{n} missing at full width")));
                }
            }
            full_job = job.map_or(0.0, |r| r.width());
        }
        // The order they yield in.
        let job_cut = job.is_none_or(|r| r.width() < full_job - 0.5);
        let name_cut =
            has(name).is_some_and(|r| drawn_texts(&h, r).iter().any(|w| w.contains('…')));
        if has(folder).is_some() && has(line).is_none() {
            wrong.push(at("the folder shown and the last line gone"));
        }
        if job_cut && has(folder).is_some() {
            wrong.push(at("the job cut short and the folder shown"));
        }
        if name_cut && (has(line).is_some() || has(folder).is_some()) {
            wrong.push(at("the name cut short before the line and the folder went"));
        }
        if name_cut && !job_cut {
            wrong.push(at("the name cut short before the job's"));
        }
    }
    tell.send(Step::End).unwrap();
    wait_for(&mut h, "the job's end", |h| !h.state().busy());
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The Bottom Dock's tab bar at every window width from the least (420)
/// to 1000, in 8 point steps, beside an area with both side docks open
/// (the dock under the editor area only, a few points wide in the least
/// window): its Log tab and its tools (Copy Log, Save Log, Clear Log)
/// never over each other nor outside the dock; the tab its icon alone
/// where its words and the tools do not fit, the tools going whole from
/// the end where even that does not, and no tab where the dock is
/// narrower than its icon; all three at 1000.
#[test]
fn the_bottom_dock_s_tab_and_tools_never_overlap() {
    let Some((mut h, _area)) = area_harness("bottom-dock-narrow") else { return };
    settle(&mut h);
    let tools = ["Copy Log", "Save Log", "Clear Log"];
    let mut wrong = Vec::new();
    let mut shown_at = Vec::new();
    for width in (420..=1000).step_by(8) {
        let width = width as f32;
        h.set_size(egui::vec2(width, 800.0));
        settle(&mut h);
        let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
        let bar = h.get_by_label("Bottom Dock");
        // (Narrower than the tab's icon, none: the bar is drawn empty.)
        let mut rects: Vec<(&str, egui::Rect)> =
            bar.query_by_label("Log").map(|t| ("Log", t.rect())).into_iter().collect();
        if rects.is_empty() && dock.width() >= 40.0 {
            wrong.push(format!("{width}: no tab in a dock {dock:?}"));
        }
        for name in tools {
            if let Some(n) = bar.query_by_label(name) {
                rects.push((name, n.rect()));
            }
        }
        // (Those shown are the first ones: they go from the end.)
        let names: Vec<&str> = rects.iter().map(|r| r.0).filter(|n| *n != "Log").collect();
        if names != tools[..names.len()] {
            wrong.push(format!("{width}: the tools shown {names:?}"));
        }
        shown_at.push((dock.width(), names.len()));
        for (i, (a, ra)) in rects.iter().enumerate() {
            if ra.left() < dock.left() - 0.5 || ra.right() > dock.right() + 0.5 {
                wrong.push(format!("{width}: {a} {ra:?} outside the dock {dock:?}"));
            }
            for (b, rb) in &rects[i + 1..] {
                if ra.intersect(*rb).is_positive() && ra.intersect(*rb).area() > 1.0 {
                    wrong.push(format!("{width}: {a} {ra:?} over {b} {rb:?}"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    assert_eq!(shown_at.last().map(|s| s.1), Some(3), "all three at 1000: {shown_at:?}");
    // (Fewer only where the dock is narrower, never more: the dock's
    // width, not the window's, which the side docks share unevenly.)
    shown_at.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(shown_at.windows(2).all(|w| w[0].1 <= w[1].1), "{shown_at:?}");
}

/// A real job, a project's command (`sleep`), shows in the Status Bar as
/// it runs, its progress not known, and goes once it is canceled from
/// its window.
#[cfg(unix)]
#[test]
fn a_project_s_command_shows_as_a_job() {
    let slow = "sleep 300";
    let rules = format!("[commands]\nslow = {slow:?}\n");
    let dir = project_folder("ui-status-project-job", &[("moonglow.toml", &rules)]);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.background_jobs = true;
    let project = std::fs::canonicalize(&dir).unwrap().display().to_string();
    app.settings.allowed_commands.push((project, "slow".into(), slow.into()));
    app.open_module(&dir);
    let mut h = harness(app, 1200.0, 700.0);
    wait_for(&mut h, "the module's own work", |h| !h.state().busy());
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("slow".into()));
    let job = |h: &Harness<'_, Moonglow>| {
        h.get_by_label("Status Bar")
            .query_by(|n| n.role() == egui::accesskit::Role::ProgressIndicator)
            .map(|n| n.accesskit_node().label().unwrap_or_default())
    };
    wait_for(&mut h, "the command shown", |h| job(h).is_some());
    let name = job(&h).unwrap();
    assert!(name.ends_with(", under way") && name.to_lowercase().contains("slow"), "{name}");
    // (The job's window shown, its fade in over.)
    h.run_steps(20);
    h.get_all_by_label("Cancel").last().unwrap().click();
    wait_for(&mut h, "the command stopped", |h| !h.state().busy());
    h.step();
    assert!(job(&h).is_none(), "gone with it");
}
