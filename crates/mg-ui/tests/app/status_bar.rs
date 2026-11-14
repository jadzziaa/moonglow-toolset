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
/// and the log empty; the Status Bar with no hints (the Welcome page's
/// are `status_hints.rs`'s), so that the last line has the middle alone.
fn closed_log_app() -> Moonglow {
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.log.clear();
    app.status_for_tests(Some((Vec::new(), Vec::new())));
    app
}

/// The most of the bar the last line takes beside an editor that is no
/// area (`shell::LAST_LINE_CAP`: the boards' `.sb-log` `max-width`, the
/// Creature board's 160).
const LINE_CAP: f32 = 160.0;

/// The node named `name` in the Status Bar, if it is there.
fn in_bar<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> Option<egui_kittest::Node<'t>> {
    h.get_by_label("Status Bar").query_by_label(name)
}

/// How the last line's button is named: "Last log line: " and the line.
const LAST: &str = "Last log line: ";

/// The last line reading `text` in the Status Bar, if it is there: its
/// rectangle.
fn last_line(h: &Harness<'_, Moonglow>, text: &str) -> Option<egui::Rect> {
    let name = format!("{LAST}{text}");
    in_bar(h, &name).map(|n| n.rect())
}

/// Clicks the last line reading `text`.
fn click_last_line(h: &Harness<'_, Moonglow>, text: &str) {
    let name = format!("{LAST}{text}");
    in_bar(h, &name).expect("the last line").click();
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
    let name = format!("{LAST}First thing said");
    let line = in_bar(&h, &name).expect("the last line");
    assert_eq!(line.accesskit_node().role(), egui::accesskit::Role::Button);
    let switch = in_bar(&h, "Log").unwrap().rect();
    let shown = line.rect();
    assert!(shown.left() >= switch.right() + 7.5, "after the switch: {shown:?} {switch:?}");
    h.state_mut().log.warn("Second thing said");
    settle(&mut h);
    assert!(last_line(&h, "First thing said").is_none());
    assert!(last_line(&h, "Second thing said").is_some());
    in_bar(&h, "Log").unwrap().click();
    settle(&mut h);
    assert!(!h.state().settings.hide_log && h.query_by_label("Bottom Dock").is_some());
    assert!(last_line(&h, "Second thing said").is_none(), "not while the log shows it");
    in_bar(&h, "Log").unwrap().click();
    settle(&mut h);
    assert!(last_line(&h, "Second thing said").is_some(), "back once closed");
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
            let rect = last_line(&h, said).expect(said);
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
        let line = last_line(&h, long).expect("the last line");
        let name = in_bar(&h, "No module open").unwrap().rect();
        assert!(line.right() <= name.left() - 8.0, "{width}: {line:?} before {name:?}");
        let words = drawn_texts(&h, line);
        assert_eq!(words.len(), 1, "{width}: {words:?}");
        // (Cut short at every width: at its cap where the bar has room,
        // the boards' `.sb-log`, shorter where it has less.)
        assert!(words[0].ends_with('…') && words[0].starts_with("Verify"), "{words:?}");
        assert!(words[0].chars().count() < long.chars().count());
        assert!(line.width() <= LINE_CAP + 0.5, "{width}: held to its cap: {line:?}");
        if width >= 1600.0 {
            assert!(line.width() > LINE_CAP - 12.0, "at its cap where there is room: {line:?}");
        }
        let tip = tip_at(&mut h, line).unwrap_or_default();
        assert!(tip.ends_with(long), "{width}: the whole line in its tip: {tip}");
    }
    let mut app = closed_log_app();
    app.log.error("The first line\nand the second");
    let mut h = harness(app, 1200.0, 600.0);
    settle(&mut h);
    let line = last_line(&h, "The first line").expect("named by its first line");
    assert_eq!(drawn_texts(&h, line), ["The first line"]);
    let tip = tip_at(&mut h, line).unwrap_or_default();
    assert!(tip.ends_with("The first line\nand the second"), "its lines in the tip: {tip}");
}

/// Pressing the last line opens the log, as the switch does.
#[test]
fn pressing_the_last_line_opens_the_log() {
    let mut app = closed_log_app();
    app.log.info("Something said");
    let mut h = harness(app, 1200.0, 700.0);
    settle(&mut h);
    click_last_line(&h, "Something said");
    settle(&mut h);
    assert!(!h.state().settings.hide_log, "opened");
    // (A log line's words are its value, its time first.)
    let said = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.value().is_some_and(|v| v.contains(" Something said"))
    };
    assert!(h.query_all_by(said).count() >= 1, "the log shows it");
    assert!(last_line(&h, "Something said").is_none());
}

/// The last line is the last entry whose first line is not blank: a last
/// entry of spaces (a project command's blank output line), an empty one
/// or one starting with a new line leaves the one before shown; with only
/// blank entries, no line, no separator and no room, as with none at all.
#[test]
fn a_blank_last_entry_shows_the_line_before() {
    let mut h = harness(closed_log_app(), 1200.0, 700.0);
    settle(&mut h);
    let controls = |h: &Harness<'_, Moonglow>| bar_controls(h, "Status Bar");
    let bare = controls(&h);
    let separators = |h: &Harness<'_, Moonglow>| {
        let bar = in_bar(h, "Log").unwrap().rect();
        let name = in_bar(h, "No module open").unwrap().rect();
        // (A separator: a 1 by 14 rectangle of `line-soft` between them.)
        let soft = mg_uikit::tokens(&h.ctx).line_soft;
        drawn(h)
            .into_iter()
            .filter(|s| match s {
                egui::Shape::Rect(r) => {
                    r.fill == soft
                        && r.rect.width() <= 1.5
                        && r.rect.left() > bar.right()
                        && r.rect.right() < name.left()
                        && (r.rect.center().y - bar.center().y).abs() < 6.0
                }
                _ => false,
            })
            .count()
    };
    for blank in ["   ", "", "\nthe rest of it", "  \t  "] {
        h.state_mut().log.clear();
        h.state_mut().log.info("The message before");
        h.state_mut().log.info(blank);
        settle(&mut h);
        assert!(last_line(&h, "The message before").is_some(), "{blank:?}: the one before");
        // Only blank entries: nothing shown, no separator, no room.
        h.state_mut().log.clear();
        h.state_mut().log.info(blank);
        h.state_mut().log.info("  ");
        settle(&mut h);
        assert_eq!(controls(&h), bare, "{blank:?}: as with no line");
        assert_eq!(separators(&h), 0, "{blank:?}: no separator");
    }
}

/// A message of megabytes, on one line or many: the last line shows its
/// first line cut to 300 characters with an ellipsis, its button's name
/// as bounded; its tip (made only while it is hovered: not in a frame
/// where it is not) its first lines cut likewise and "… more in the log".
#[test]
fn a_huge_last_line_is_bounded() {
    let mut h = harness(closed_log_app(), 1600.0, 700.0);
    settle(&mut h);
    let one_line = "x".repeat(4_000_000);
    let many_lines = "y".repeat(100) + &"\nline".repeat(400_000);
    for (what, text) in [("one line", one_line), ("many lines", many_lines)] {
        h.state_mut().log.clear();
        h.state_mut().log.info(text.clone());
        h.hover_at(egui::pos2(1.0, 1.0));
        settle(&mut h);
        let node = h
            .get_by_label("Status Bar")
            .query_by(|n| n.label().is_some_and(|l| l.starts_with(LAST)))
            .expect(what);
        let name = node.accesskit_node().label().unwrap_or_default();
        let rect = node.rect();
        assert!(name.chars().count() <= LAST.len() + 301, "{what}: {} characters", name.len());
        let first = text.lines().next().unwrap();
        assert!(name.starts_with(&format!("{LAST}{}", &first[..50])), "{what}");
        if first.chars().count() > 300 {
            assert!(name.ends_with('…'), "{what}: cut, and says so");
        }
        // Frames with the pointer away make no tip.
        let made = h.state().last_line_tips();
        h.run_steps(3);
        assert_eq!(h.state().last_line_tips(), made, "{what}: no tip made unhovered");
        let tip = tip_at(&mut h, rect).unwrap_or_default();
        assert!(h.state().last_line_tips() > made, "{what}: made while hovered");
        assert!(tip.len() < 4_000, "{what}: the tip bounded: {} bytes", tip.len());
        assert!(tip.ends_with("… more in the log"), "{what}: {}", &tip[tip.len() - 40..]);
    }
}

/// A message that reads "Log" (or any control's word) does not take the
/// switch's name: the last line is "Last log line: Log", the switch
/// still found alone by "Log".
#[test]
fn a_last_line_reading_log_is_not_the_switch() {
    let mut app = closed_log_app();
    app.log.info("Log");
    let mut h = harness(app, 1200.0, 700.0);
    settle(&mut h);
    assert!(last_line(&h, "Log").is_some());
    let switch = in_bar(&h, "Log").expect("the switch, found alone by its name");
    assert_eq!(switch.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
}

/// A long game folder's name, for [`with_no_line_yet_nothing_is_shown_or_kept`]:
/// with [`MODULE`]'s name, wide enough to go for want of room in windows
/// over 1400 (226 bytes: a folder's name holds 255).
const FOLDER: &str = "Neverwinter Nights Enhanced Edition on the second drive of this machine, \
                      in the folder kept for the games installed before the last move of them all \
                      to the larger drive, with the haks and the overrides of the older servers too";
/// A long module's name, for [`with_no_line_yet_nothing_is_shown_or_kept`].
const MODULE: &str = "a_module_with_a_rather_long_file_name_for_the_status_bar_check";

/// With nothing said yet, nothing follows the switch and no room is kept
/// for it: the bar's controls are the switch and the right-hand items,
/// and at every width from 1400 to 2200 the bar is as it is with the log
/// open (where it has no last line), the game folder (a long one, beside a
/// long module's name) kept as long; a line said then takes the room
/// between them alone. (Under 1400 the folder goes whatever the room, M3
/// T14: the sweep was 420 to 1000 with a shorter folder and no module
/// before.)
#[test]
fn with_no_line_yet_nothing_is_shown_or_kept() {
    let dir = mg_testkit::scratch_dir("ui-status-no-line");
    let game = dir.join(FOLDER);
    std::fs::create_dir_all(&game).unwrap();
    let module = dir.join(format!("{MODULE}.mod"));
    std::fs::copy(sample_module(&dir), &module).unwrap();
    let mut app = game_app(&game, None);
    app.settings.hide_log = true;
    app.settings.no_last_area = true;
    app.open_module(&module);
    // (No hints: the Welcome page's are `status_hints.rs`'s.)
    app.status_for_tests(Some((Vec::new(), Vec::new())));
    let mut h = harness(app, 2400.0, 700.0);
    settle(&mut h);
    h.state_mut().log.clear();
    settle(&mut h);
    let names: Vec<String> = bar_controls(&h, "Status Bar").into_iter().map(|c| c.0).collect();
    assert_eq!(names, ["Log", MODULE, format!("…/{FOLDER}").as_str()]);
    let folder = &format!("…/{FOLDER}");
    let folder_width = in_bar(&h, folder).unwrap().rect().width();
    let mut differ = Vec::new();
    let mut dropped = 0;
    for width in (1400..=2200).step_by(8) {
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
            let (switch, name) = (has("Log").unwrap(), has(MODULE).unwrap());
            // (The room between them: the gaps, a separator and the folder.)
            let room = name.left() - switch.right() - 2.0 * 8.0;
            if room >= folder_width + 1.0 + 2.0 * 8.0 {
                differ.push(format!("{width}: the folder left out with {room} of room"));
            }
        }
    }
    assert!(differ.is_empty(), "{differ:#?}");
    assert!(dropped > 0, "the folder goes somewhere in the sweep");
    assert!(dropped < 100, "and is shown somewhere in it: dropped at {dropped} widths");
    // (Wide enough for the folder beside the line too: where it is not,
    // the line said makes the folder go first, as the bar yields.)
    h.set_size(egui::vec2(2400.0, 700.0));
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    let at = |h: &Harness<'_, Moonglow>| {
        (in_bar(h, "Log").unwrap().rect(), in_bar(h, MODULE).unwrap().rect())
    };
    let without = at(&h);
    h.state_mut().log.info("Now something");
    settle(&mut h);
    assert_eq!(at(&h), without, "the line takes the room between them alone");
}

/// The Bottom Dock keeps the height it was given through the switch
/// (closed and opened) and a restart (its settings read again); View ›
/// Reset Layout gives the shell's own layout: the Bottom Dock closed (as
/// a new installation's), the side docks unfolded, and opened again the
/// dock is at its first height.
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
    again.state_mut().settings.hide_tree = true;
    settle(&mut again);
    menu_command(&mut again, "View", "Reset Layout");
    settle(&mut again);
    // (Not chosen: its first height, `metrics::BOTTOM_DOCK`, below.)
    assert_eq!(again.state().settings.bottom_dock_height, None);
    assert_eq!(top(&again), None, "closed, as the shell has it at first");
    assert!(!again.state().settings.hide_tree, "the Left Dock unfolded");
    in_bar(&again, "Log").unwrap().click();
    settle(&mut again);
    assert!((top(&again).unwrap() - was).abs() <= 1.0, "opened, its first height");
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
        // (One frame: the refusal is said when it is asked for, not later.)
        h.step();
        assert_eq!(job(&h).unwrap().0, "Count Things, 3 of 10");
        let refused = "Another Job: waiting for Count Things to finish";
        assert!(last_line(&h, refused).is_some(), "the refusal is the last line");
        tell.send(if fail { Step::Fail } else { Step::End }).unwrap();
        wait_for(&mut h, "the job's end", |h| !h.state().busy());
        settle(&mut h);
        assert!(job(&h).is_none(), "gone with the job");
        let said = if fail { "Count Things failed: it broke" } else { "Count Things: done" };
        let rect = last_line(&h, said).unwrap_or_else(|| panic!("{said}"));
        let t = mg_uikit::tokens(&h.ctx);
        h.hover_at(egui::pos2(1.0, 1.0));
        settle(&mut h);
        let color = if fail { t.error } else { t.text_3 };
        assert_eq!(text_color(&h, rect), Some(color), "{said}");
        // Nothing asks for frames once it is over.
        settle(&mut h);
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
    let open = again(&h);
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    assert!(last_line(&h, "Said once").is_some());
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
    assert!(last_line(&h, &last).is_some(), "its last line beside the switch: {last}");
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
/// in 8 point steps, then to 2560 in 16, and at 3440 (the wide boards'),
/// with a
/// long module name (modified), a long last line, the Main board's six
/// hints and its two counts, a running job and a long game folder: every
/// node of the bar inside it and none over another, every text drawn
/// inside the bar and over no other; the module's name always there,
/// everything at 2560 and 3440 (the long texts are wider than the
/// board's); and they yield in the one order
/// `shell::status_bar` builds, as the window narrows: the Bottom Dock's
/// switches show their icons alone first (always under 1400, M3 T14); then
/// the game folder goes (always under 1400; never shown beside a hint
/// dropped or the line cut); then
/// the hints drop whole from the end (those shown always the first ones);
/// then the last line is cut short from its cap (`shell::LAST_LINE_CAP`,
/// the boards' `.sb-log`) down to its least room
/// (`shell::LAST_LINE_LEAST`, 48 points), then goes; then the counts go
/// from the end; then the job's name is cut short; the module's name
/// last. Each stage is seen at some width.
#[test]
fn the_status_bar_holds_at_every_width() {
    use mg_uikit::status::Hint;
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
    // The Main board's hints and counts (in place of an area's, which
    // need the game: `status_hints.rs` has them).
    let hints = vec![
        Hint::keys(["Drag"], "Move"),
        Hint::keys(["Alt", "Right Drag"], "Turn"),
        Hint::keys(["Shift"], "Handles"),
        Hint::keys(["Alt", "Drag"], "Raise"),
        Hint::new("Turn by a step").key("Q").key("E"),
        Hint::keys(["G"], "Drop to ground"),
    ];
    let hint_names: Vec<String> = hints.iter().map(Hint::spoken).collect();
    let count_names = ["11 by 11 tiles", "192 objects"];
    app.status_for_tests(Some((hints, count_names.map(String::from).to_vec())));
    let said = "Compiled 312 scripts of the module and its haks, with 4 warnings about \
                functions declared one way and defined another";
    let line = &format!("{LAST}{said}");
    let name = "a_module_with_a_rather_long_file_name_for_the_status_bar_check (modified)";
    let folder = "…/Neverwinter Nights Enhanced Edition on the second drive";
    let job_title = "Compile All Scripts";
    let mut h = harness(app, 1600.0, 600.0);
    settle(&mut h);
    h.state_mut().log.info(said);
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
    // (The line's least room, a point to spare for the rounding of its
    // galley; its cap as it is drawn at 3440, held to the cap of the
    // editor in front, Module Properties: 160, the boards' `.sb-log`.)
    let least = 48.0 - 1.0;
    let mut cap = 0.0;
    let mut full_job = 0.0;
    let mut wrong = Vec::new();
    // (Widths where each stage was seen.)
    let mut stages = std::collections::BTreeMap::<&str, usize>::new();
    // (3440 first, where everything shows, then from the least window up:
    // by 8 to 1600, by 16 to 2560.)
    let widths =
        std::iter::once(3440).chain((420..1600).step_by(8)).chain((1600..=2560).step_by(16));
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
                // (The hints' group holds its hints.)
                if a == "Hints" || b == "Hints" {
                    continue;
                }
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
        if width >= 2560.0 {
            let all = ["Log", line.as_str(), job_name, name, folder];
            let all = all.iter().copied().chain(hint_names.iter().map(String::as_str));
            for n in all.chain(count_names) {
                if has(n).is_none() {
                    wrong.push(at(&format!("{n} missing at full width")));
                }
            }
            if width >= 3440.0 {
                full_job = job.map_or(0.0, |r| r.width());
                cap = has(line).map_or(0.0, |r| r.width() - 1.0);
                assert!((150.0..=161.0).contains(&cap), "the line at its cap: {cap}");
            }
        }
        // Those shown are the first ones, in order.
        let hints_shown = hint_names.iter().take_while(|n| has(n).is_some()).count();
        if hint_names.iter().filter(|n| has(n).is_some()).count() != hints_shown {
            wrong.push(at("a hint shown after one dropped"));
        }
        let counts_shown = count_names.iter().take_while(|n| has(n).is_some()).count();
        if count_names.iter().filter(|n| has(n).is_some()).count() != counts_shown {
            wrong.push(at("a count shown after one gone"));
        }
        let all_hints = hints_shown == hint_names.len();
        let line_w = has(line).map(|r| r.width());
        let line_full = line_w.is_some_and(|w| w >= cap);
        let job_cut = job.is_none_or(|r| r.width() < full_job - 0.5);
        let name_cut =
            has(name).is_some_and(|r| drawn_texts(&h, r).iter().any(|w| w.contains('…')));
        // The order they yield in: first the switches show their icons
        // alone (always under 1400, M3 T14, and wherever the rest does not
        // fit beside their words), then the folder goes (always under
        // 1400), then the middle's.
        let icons = has("Log").is_some_and(|r| r.width() < 30.0);
        if width < 1400.0 && (!icons || has(folder).is_some()) {
            wrong.push(at("under 1400 the switches' words or the folder shown"));
        }
        if !icons && has(folder).is_none() {
            wrong.push(at("the folder gone beside the switches' words"));
        }
        if has(folder).is_some() && !(all_hints && line_full) {
            wrong.push(at("the folder shown beside a hint dropped or the line cut"));
        }
        if hints_shown > 0 && !line_full {
            wrong.push(at("a hint shown beside the line cut or gone"));
        }
        if line_w.is_some_and(|w| w < least) {
            wrong.push(at("the line under its least room"));
        }
        if counts_shown < count_names.len() && (line_w.is_some() || hints_shown > 0) {
            wrong.push(at("a count gone before the line and the hints"));
        }
        if job_cut && counts_shown > 0 {
            wrong.push(at("the job cut short and a count shown"));
        }
        if name_cut && !job_cut {
            wrong.push(at("the name cut short before the job's"));
        }
        // (Which stage this width is at.)
        let stage = if has(folder).is_some() && !icons {
            "everything"
        } else if has(folder).is_some() {
            "the switches as icons"
        } else if all_hints {
            "the folder gone"
        } else if hints_shown > 0 {
            "hints dropped"
        } else if line_full {
            "no hints, the line whole"
        } else if line_w.is_some() {
            "the line cut"
        } else if counts_shown == count_names.len() {
            "the line gone"
        } else if counts_shown > 0 {
            "a count gone"
        } else if !job_cut {
            "the counts gone"
        } else if !name_cut {
            "the job cut"
        } else {
            "the name cut"
        };
        *stages.entry(stage).or_default() += 1;
    }
    tell.send(Step::End).unwrap();
    wait_for(&mut h, "the job's end", |h| !h.state().busy());
    assert!(wrong.is_empty(), "{wrong:#?}");
    for stage in [
        "everything",
        "the switches as icons",
        "the folder gone",
        "hints dropped",
        "the line cut",
        "the line gone",
        "the counts gone",
        "the job cut",
    ] {
        assert!(stages.contains_key(stage), "never {stage}: {stages:?}");
    }
}

/// The Bottom Dock's tab bar at every window width from the least (420)
/// to 1000, in 4 point steps, beside an area with both side docks open
/// (the dock under the editor area only, 24 points wide in the least
/// window): its Log tab and its tools (Copy Log, Save Log, Clear Log)
/// never over each other nor outside the dock; and, as the dock narrows,
/// each stage in turn: the tab's words with all three tools; the tab its
/// icon alone with the tools going one by one from the end; the icon
/// alone with no tools; nothing under the icon's width. The tab is "Log"
/// whatever its form; each tool not shown is in the View menu.
#[test]
fn the_bottom_dock_s_tab_and_tools_never_overlap() {
    let Some((mut h, _area)) = area_harness("bottom-dock-narrow") else { return };
    settle(&mut h);
    let tools = ["Copy Log", "Save Log", "Clear Log"];
    // (The stages in order, the narrowest first: the tab's form and how
    // many tools.)
    let stages = [("none", 0), ("icon", 0), ("icon", 1), ("icon", 2), ("icon", 3), ("words", 3)];
    let mut wrong = Vec::new();
    let mut seen = Vec::new();
    for width in (420..=1000).step_by(4) {
        let width = width as f32;
        h.set_size(egui::vec2(width, 800.0));
        settle(&mut h);
        let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
        let bar = h.get_by_label("Bottom Dock");
        let tab = bar.query_by_label("Log").map(|t| t.rect());
        // The tab's form, by what it draws: its icon alone, or its words.
        let form = match tab {
            None => "none",
            Some(r) if drawn_texts(&h, r).iter().any(|w| w == "Log") => "words",
            Some(r) if icon_drawn(&h, mg_uikit::Icon::Log, r) => "icon",
            Some(r) => {
                wrong.push(format!("{width}: a tab {r:?} with neither its words nor its icon"));
                "none"
            }
        };
        let mut rects: Vec<(&str, egui::Rect)> = tab.map(|r| ("Log", r)).into_iter().collect();
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
        match stages.iter().position(|s| *s == (form, names.len())) {
            Some(stage) => seen.push((dock.width(), stage)),
            None => wrong.push(format!("{width}: {form} with {} tools", names.len())),
        }
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
        // The tools not shown are in the View menu.
        if names.len() < tools.len() {
            h.get_by_label("Top Bar").get_by_label("View").click();
            settle(&mut h);
            for (tool, row) in
                [("Copy Log", "Copy Log"), ("Save Log", "Save Log…"), ("Clear Log", "Clear Log")]
            {
                if !names.contains(&tool) && menu_row(&h, row).is_none() {
                    wrong.push(format!("{width}: {row} not in the View menu"));
                }
            }
            h.key_press(egui::Key::Escape);
            settle(&mut h);
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    // (In order of the dock's width, not the window's, which the side
    // docks share unevenly: a stage never comes back once passed.)
    seen.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(seen.windows(2).all(|w| w[0].1 <= w[1].1), "{seen:?}");
    let reached: Vec<usize> = seen.iter().map(|s| s.1).collect();
    for (i, stage) in stages.iter().enumerate() {
        // (The icon with all three tools is a few points of the dock's
        // width at most: not every sweep lands on it.)
        if *stage != ("icon", 3) {
            assert!(reached.contains(&i), "{stage:?} never seen: {seen:?}");
        }
    }
    // Nothing under the icon tab's width; the icon alone from there.
    for (w, stage) in &seen {
        if *stage == 0 {
            assert!(*w < 34.0, "no tab in a dock {w} wide");
        }
    }
}

/// Kills the process whose number is in `file` (written by the command
/// it ran: `echo $$ > FILE; exec sleep …`), if it is still that command,
/// when dropped: a test's command does not outlive it, failed or not.
struct KillOnDrop(std::path::PathBuf);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let Ok(pid) = std::fs::read_to_string(&self.0) else { return };
        let pid = pid.trim();
        let cmd = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        if String::from_utf8_lossy(&cmd).starts_with("sleep") {
            _ = std::process::Command::new("kill").args(["-9", pid]).status();
        }
    }
}

/// A real job, a project's command (`sleep`), shows in the Status Bar as
/// it runs, its progress not known, and goes once it is canceled from
/// its window. (The command's own process is killed when the test ends,
/// whether or not it got as far as Cancel.)
#[cfg(unix)]
#[test]
fn a_project_s_command_shows_as_a_job() {
    let scratch = mg_testkit::scratch_dir("ui-status-project-job-pid");
    let pid = scratch.join("job.pid");
    let slow = format!("echo $$ > '{}'; exec sleep 300", pid.display());
    let rules = format!("[commands]\nslow = {slow:?}\n");
    let dir = project_folder("ui-status-project-job", &[("moonglow.toml", &rules)]);
    let _kill = KillOnDrop(pid.clone());
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.background_jobs = true;
    let project = std::fs::canonicalize(&dir).unwrap().display().to_string();
    app.settings.allowed_commands.push((project, "slow".into(), slow.clone()));
    app.open_module(&dir);
    let mut h = harness(app, 1200.0, 700.0);
    wait_for(&mut h, "the module's own work", |h| !h.state().busy());
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("slow".into()));
    let job = |h: &Harness<'_, Moonglow>| {
        h.get_by_label("Status Bar")
            .query_by(|n| n.role() == egui::accesskit::Role::ProgressIndicator)
            .map(|n| n.accesskit_node().label().unwrap_or_default())
    };
    wait_for(&mut h, "the command shown", |h| job(h).is_some() && pid.exists());
    let name = job(&h).unwrap();
    assert!(name.ends_with(", under way") && name.to_lowercase().contains("slow"), "{name}");
    // Its window's Cancel, pressed until the job ends (a press while the
    // window fades in is not taken).
    let started = Instant::now();
    while h.state().busy() {
        assert!(started.elapsed() < Duration::from_secs(60), "the command never stopped");
        if let Some(cancel) = h.get_all_by_label("Cancel").last() {
            cancel.click();
        }
        std::thread::sleep(Duration::from_millis(10));
        h.step();
    }
    settle(&mut h);
    assert!(job(&h).is_none(), "gone with it");
}

/// A first start with no game found, the log closed as a new
/// installation's: the Status Bar's last line is the log's warning that
/// no installation was found (as the manual's troubleshooting says), the
/// folder "No game install".
#[test]
fn a_first_start_without_the_game_says_so_in_the_last_line() {
    let mut app = app_with(Vec::new());
    app.settings.hide_log = mg_ui::Settings::new_installation().hide_log;
    let mut h = harness(app, 1600.0, 700.0);
    settle(&mut h);
    let said = h.state().log.entries.last().map(|e| e.1.clone()).unwrap_or_default();
    assert!(said.starts_with("No Neverwinter Nights installation found"), "{said:?}");
    assert!(last_line(&h, &said).is_some(), "shown: {said}");
    assert!(in_bar(&h, "No game install").is_some());
}
