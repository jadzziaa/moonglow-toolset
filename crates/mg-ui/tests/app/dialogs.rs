//! Dialogs' title rows and floating windows' bars (the redesign's M3,
//! task 11): a dialog's row across its top with its title at the left,
//! a Close where Escape closes it; moved by its row, not its body; opened
//! in the middle of the main window whatever the docks, and whole in the
//! least window. A floating editor's bar: the collapse arrow, Maximize
//! and Close Window, named and tipped, the grip in its corner; the sizes
//! kept before the theme scaled once and fitted to the window.

use crate::common::*;

/// The window named `title` (a dialog: its accessible name is its
/// title), where it is now.
fn window_rect(h: &Harness<'_, Moonglow>, title: &str) -> egui::Rect {
    h.get_by_label(title).rect()
}

/// A press at `from`, the pointer moved to `to` a step a frame (each
/// step one input event), and let go there.
fn press_drag(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run_steps(1);
    press(h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for k in 1..=6 {
        h.hover_at(from + (to - from) * (k as f32 / 6.0));
        h.run_steps(1);
    }
    press(h, to, false, egui::Modifiers::NONE);
    h.run_steps(1);
}

/// The tip `tip` shown with the pointer held at `at`, waited for (a tip
/// shows once the pointer has rested).
fn tip_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, tip: &str) {
    h.hover_at(at);
    let started = std::time::Instant::now();
    while h.query_by_label(tip).is_none() && started.elapsed().as_secs() < 10 {
        h.step();
    }
    h.get_by_label(tip);
}

/// The window named `title` once it has stopped moving and growing (a
/// dialog takes its default size over its first frames): where it is
/// three frames alike, a step a frame, within a deadline.
fn resting(h: &mut Harness<'_, Moonglow>, title: &str) -> egui::Rect {
    let started = std::time::Instant::now();
    let (mut last, mut alike) = (window_rect(h, title), 0);
    while alike < 3 {
        assert!(started.elapsed().as_secs() < 10, "{title} never rested");
        h.step();
        let now = window_rect(h, title);
        alike = if now == last { alike + 1 } else { 0 };
        last = now;
    }
    last
}

/// Help › About in a window with a module open (in a scratch folder
/// `name`) and no game: a dialog that Escape closes, so with a Close in
/// its title row.
fn about_harness(name: &str) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(name);
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1100.0, 800.0);
    h.run();
    h.state_mut().about = true;
    settle(&mut h);
    h
}

const ABOUT: &str = "About Moonglow Toolset";

/// The title row is the dialog's handle: a drag on it moves the dialog;
/// a drag on its body where no control is (its margin) does not.
#[test]
fn a_dialog_drags_by_its_title_row_and_not_by_its_body() {
    let mut h = about_harness("ui-dialogs-about-drag");
    let was = window_rect(&h, ABOUT);
    // On the row, beside the title.
    let on_row = egui::pos2(was.center().x, was.top() + 14.0);
    press_drag(&mut h, on_row, on_row + egui::vec2(60.0, 40.0));
    settle(&mut h);
    let moved = window_rect(&h, ABOUT).min - was.min;
    assert!((moved - egui::vec2(60.0, 40.0)).length() < 3.0, "dragged by its row: {moved:?}");
    // On the body's margin, by its bottom left corner: it stays.
    let now = window_rect(&h, ABOUT);
    let on_body = now.left_bottom() + egui::vec2(4.0, -4.0);
    press_drag(&mut h, on_body, on_body + egui::vec2(80.0, -60.0));
    settle(&mut h);
    let after = window_rect(&h, ABOUT);
    assert!((after.min - now.min).length() < 0.5, "not by its body: {now:?} to {after:?}");
}

/// A dialog that Escape closes has a Close at its title row's right,
/// named "Close" and tipped, the last stop of Tab after what the dialog
/// holds; the Close and Escape each close it.
#[test]
fn a_dialog_s_close_and_escape_close_it() {
    let mut h = about_harness("ui-dialogs-about-close");
    let window = window_rect(&h, ABOUT);
    let close = h.get_by_label("Close").rect();
    assert!(close.top() - window.top() < 6.0, "in the title row: {close:?} of {window:?}");
    assert!(window.right() - close.right() < 6.0, "at its right: {close:?} of {window:?}");
    // Tab: from the dialog's own button to the Close, its last stop.
    h.get_by_label("Check for Updates…").focus();
    h.run();
    assert!(h.get_by_label("Check for Updates…").is_focused(), "its own button");
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("Close").is_focused(), "the Close after it");
    // Its tip.
    tip_at(&mut h, close.center(), "Close the dialog (Esc)");
    h.get_by_label("Close").click();
    h.run();
    assert!(!h.state().about && h.query_by_label(ABOUT).is_none(), "closed by its Close");
    h.state_mut().about = true;
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().about && h.query_by_label(ABOUT).is_none(), "closed by Escape");
}

/// A dialog its own buttons answer (a wizard, a question) has no Close:
/// the New Module wizard's row holds its title alone; its Cancel and
/// Escape still close it.
#[test]
fn a_dialog_its_buttons_answer_has_no_close() {
    let mut h = harness(app_with(Vec::new()), 1100.0, 800.0);
    h.run();
    for escape in [false, true] {
        h.state_mut().actions.push(mg_ui::Action::NewModuleDialog);
        settle(&mut h);
        let window = window_rect(&h, "New Module");
        let closes = h.query_all_by_label("Close").filter(|n| window.contains_rect(n.rect()));
        assert_eq!(closes.count(), 0, "no Close on a wizard");
        if escape {
            h.key_press(egui::Key::Escape);
        } else {
            h.get_by_label("Cancel").click();
        }
        settle(&mut h);
        assert!(h.query_by_label("New Module").is_none(), "closed (by Escape: {escape})");
    }
}

/// A dialog with a Cancel of its own and a Close (Resize Area): each of
/// the Close, Escape and Cancel closes it and changes nothing.
#[test]
fn a_dialog_closes_by_its_close_by_escape_and_by_its_cancel_alike() {
    let Some((mut h, area)) = area_harness("dialog-closes") else { return };
    let size = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        (are.root.integer("Width").unwrap(), are.root.integer("Height").unwrap())
    };
    for how in ["Close", "Escape", "Cancel"] {
        h.get_by_label("Edit").click();
        h.run_steps(2);
        h.get_by_label("Resize Area…").click();
        settle(&mut h);
        h.state_mut().resize_area.as_mut().expect("open").columns = 6;
        h.run_steps(1);
        let window = window_rect(&h, "Resize Area");
        match how {
            "Escape" => h.key_press(egui::Key::Escape),
            _ => {
                let mut found = h.get_all_by_label(how).filter(|n| window.contains_rect(n.rect()));
                let one = found.next().unwrap_or_else(|| panic!("its {how}"));
                assert!(found.next().is_none(), "one {how}");
                one.click();
            }
        }
        settle(&mut h);
        assert!(h.state().resize_area.is_none(), "closed by its {how}");
        assert_eq!(size(&mut h), (4, 4), "nothing changed by its {how}");
    }
}

/// Dialogs open in the middle of the main window (the boards: "it opens
/// in the middle of the main window"), wherever the docks are, open or
/// folded: the Area Wizard over an area.
#[test]
fn a_dialog_opens_in_the_middle_of_the_main_window_whatever_the_docks() {
    let Some((mut h, _)) = area_harness("dialog-middle") else { return };
    for (tree, palettes) in [(false, false), (true, false), (false, true), (true, true)] {
        h.state_mut().settings.hide_tree = tree;
        h.state_mut().settings.hide_palettes = palettes;
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::AreaWizard);
        settle(&mut h);
        let window = window_rect(&h, "Area Wizard");
        let middle = h.ctx.content_rect().center();
        let off = window.center() - middle;
        assert!(off.length() < 2.0, "folded {tree} {palettes}: {window:?} about {middle:?}");
        h.get_by_label("Cancel").click();
        settle(&mut h);
        assert!(h.query_by_label("Area Wizard").is_none());
    }
}

/// In the least window (420 points, both docks open, the editor area
/// narrower than the wizard) the Area Wizard is wholly in the window, its
/// buttons in reach.
#[test]
fn a_dialog_stays_whole_in_the_least_window() {
    let Some((mut h, _)) = area_harness("dialog-least") else { return };
    h.set_size(egui::vec2(420.0, 700.0));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::AreaWizard);
    settle(&mut h);
    let screen = h.ctx.content_rect();
    let window = window_rect(&h, "Area Wizard");
    let area = h.state().editor_area_rect().expect("the editor area");
    assert!(area.width() < window.width(), "the editor area narrower: {area:?} {window:?}");
    assert!(screen.contains_rect(window), "{window:?} in {screen:?}");
    for button in ["Create", "Cancel"] {
        let at = h.get_by_label(button).rect();
        assert!(screen.contains_rect(at) && window.contains_rect(at), "{button} at {at:?}");
    }
    h.get_by_label("Cancel").click();
    settle(&mut h);
    assert!(h.query_by_label("Area Wizard").is_none(), "answered");
}

/// The area's Properties in a window of its own in `mode` (As Tabs keeps
/// the kind in a window as Eject would), with its window's pane.
fn properties_window(mode: Mode, name: &str) -> Option<(Harness<'static, Moonglow>, Tab)> {
    let (mut h, area) = area_harness(&mode.dir(name))?;
    put_in(h.state_mut(), mode);
    if mode.tabs() {
        h.state_mut().settings.window_places =
            vec![("area-properties".to_string(), "window".to_string())];
    }
    let props = Tab::AreaProperties(area);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
    settle(&mut h);
    let path = h.state().dock.find_tab(&props).expect("open");
    assert!(!path.surface.is_main(), "in a window of its own ({mode:?})");
    Some((h, props))
}

/// The pane of `tab`'s window: where it is, and whether it is collapsed
/// to its bar.
fn pane(h: &Harness<'_, Moonglow>, tab: &Tab) -> (egui::Rect, bool) {
    let s = h.state();
    let path = s.dock.find_tab(tab).expect("open");
    let leaf = s.dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1;
    (leaf.rect, leaf.collapsed)
}

/// The button named `name` in the bar of `tab`'s window.
fn bar_button(h: &Harness<'_, Moonglow>, tab: &Tab, name: &str) -> egui::Rect {
    let (rect, _) = pane(h, tab);
    let bar = egui::Rect::from_x_y_ranges(rect.x_range(), rect.top()..=rect.top() + 30.0);
    let found: Vec<egui::Rect> = (h.query_all_by_label(name).map(|n| n.rect()))
        .filter(|r| bar.expand(1.0).contains_rect(*r))
        .collect();
    let [one] = found[..] else { panic!("one {name} in the bar {bar:?}: {found:?}") };
    one
}

/// A floating editor's bar, in each way editors open: the collapse arrow
/// at its left folds the window to its bar and opens it again ("Collapse",
/// then "Expand"); Maximize beside Close Window at its right, by its
/// button and its key (Ctrl+Shift+M), and back; the grip in its corner
/// resizes it; Close Window closes it; each named and tipped.
fn a_floating_window_s_bar(mode: Mode) {
    let Some((mut h, props)) = properties_window(mode, "window-bar") else { return };
    let (first, _) = pane(&h, &props);
    // The buttons: the arrow at the left, Maximize then Close Window at
    // the right, 22 points square, side by side.
    let collapse = bar_button(&h, &props, "Collapse");
    let maximize = bar_button(&h, &props, "Maximize");
    let close = bar_button(&h, &props, "Close Window");
    assert!(collapse.left() - first.left() < 4.0, "the arrow at the left: {collapse:?}");
    assert!(first.right() - close.right() < 4.0, "the Close at the right: {close:?}");
    assert!((close.left() - maximize.right()).abs() < 3.0, "Maximize beside it: {maximize:?}");
    assert!((maximize.width() - 22.0).abs() < 0.5, "22 points: {maximize:?}");
    // Their tips.
    for (at, tip) in [
        (collapse, "Collapse the window to its tab bar"),
        (maximize, "Fill the main window, or double-click the bar"),
        (close, "Close the window and its tabs"),
    ] {
        tip_at(&mut h, at.center(), tip);
        h.hover_at(first.center());
        h.run();
    }
    // Collapsed to its bar, and opened again.
    h.get_by_label("Collapse").click();
    settle(&mut h);
    assert!(pane(&h, &props).1, "collapsed");
    h.get_by_label("Expand").click();
    settle(&mut h);
    assert!(!pane(&h, &props).1, "open again");
    // Maximize, by its button and by its key.
    let maximized = |h: &Harness<'_, Moonglow>| pane(h, &props).0.area() > first.area() + 1000.0;
    h.get_by_label("Maximize").click();
    settle(&mut h);
    assert!(maximized(&h), "by its button");
    h.get_by_label("Restore").click();
    settle(&mut h);
    assert!(!maximized(&h), "back");
    // (The key acts on the window of the tab in front: its tab clicked.)
    let tab = pane(&h, &props).0.left_top() + egui::vec2(60.0, 10.0);
    h.hover_at(tab);
    press(&h, tab, true, egui::Modifiers::NONE);
    press(&h, tab, false, egui::Modifiers::NONE);
    settle(&mut h);
    assert_eq!(h.state().front_tab().as_ref(), Some(&props), "the tab in front");
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::M);
    settle(&mut h);
    assert!(maximized(&h), "by its key");
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::M);
    settle(&mut h);
    assert!(!maximized(&h), "back by its key");
    // The grip, in the window's corner, in `text-off`: dragged, the
    // window smaller.
    let (now, _) = pane(&h, &props);
    let off = mg_uikit::tokens(&h.ctx).text_off;
    let corner = egui::Rect::from_min_max(now.right_bottom() - egui::vec2(14.0, 14.0), now.max);
    let grip_strokes = h.output().shapes.iter().filter(|s| match &s.shape {
        egui::Shape::LineSegment { points, stroke } => {
            stroke.color == off && points.iter().all(|p| corner.expand(2.0).contains(*p))
        }
        _ => false,
    });
    assert_eq!(grip_strokes.count(), 2, "the grip's two strokes in {corner:?}");
    let grip = now.right_bottom() - egui::vec2(6.0, 6.0);
    press_drag(&mut h, grip, grip - egui::vec2(90.0, 60.0));
    settle(&mut h);
    let (smaller, _) = pane(&h, &props);
    assert!((now.width() - smaller.width() - 90.0).abs() < 8.0, "{smaller:?} from {now:?}");
    assert!((now.height() - smaller.height() - 60.0).abs() < 8.0, "{smaller:?} from {now:?}");
    // Closed by Close Window.
    click_window_close(&mut h, &props);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&props).is_none(), "closed");
}

#[test]
fn a_floating_window_s_bar_collapses_maximizes_resizes_and_closes() {
    a_floating_window_s_bar(Mode::Floating);
}

#[test]
fn a_floating_window_s_bar_collapses_maximizes_resizes_and_closes_as_tabs() {
    a_floating_window_s_bar(Mode::Tabs);
}

/// A size kept before the theme, scaled once as the settings are read
/// (`Settings::scale_window_sizes`, run by `Moonglow::with_settings`),
/// that would pass the window's height: the window opens inside the
/// main window all the same.
#[test]
fn a_size_scaled_past_the_window_opens_inside_it() {
    let settings = mg_ui::Settings {
        window_sizes: vec![("area-properties".to_string(), [700, 900])],
        ..Default::default()
    };
    let app = Moonglow::with_settings(settings, Box::new(NoDialogs::default()));
    // (900 by 26 / 21: past an 800 point window.)
    assert_eq!(app.settings.window_sizes, [("area-properties".to_string(), [700, 1114])]);
    assert!(app.settings.window_sizes_scaled);
    let Some((mut h, area)) = area_harness("window-scaled-past") else { return };
    h.state_mut().settings.window_sizes = app.settings.window_sizes.clone();
    let props = Tab::AreaProperties(area);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
    settle(&mut h);
    let (rect, _) = pane(&h, &props);
    let screen = h.ctx.content_rect();
    assert!(screen.contains_rect(rect), "{rect:?} inside {screen:?}");
    assert!(rect.width() > 650.0, "its width kept: {rect:?}");
}

/// A job's window (an `egui::Modal`, which stops the work until it ends)
/// has the dialogs' title row with no Close: its title named once, as a
/// label across the row's 28 points at the window's top, its Cancel under
/// it.
#[test]
fn a_job_s_window_has_the_title_row() {
    let mut h = harness(app_with(Vec::new()), 1100.0, 800.0);
    h.run();
    let (tell, told) = std::sync::mpsc::channel::<()>();
    h.state_mut().start_job_for_tests("Compile All Scripts", move |p| {
        p.step(40, 312);
        _ = told.recv();
    });
    let row = |h: &Harness<'_, Moonglow>| {
        (h.query_all_by_label("Compile All Scripts").map(|n| n.rect()))
            .find(|r| (r.height() - 28.0).abs() < 0.5)
    };
    let started = std::time::Instant::now();
    while row(&h).is_none() {
        assert!(started.elapsed().as_secs() < 30, "the job's window never showed");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    let row = row(&h).unwrap();
    let cancel = h.get_by_label("Cancel").rect();
    assert!(cancel.top() > row.bottom(), "its Cancel under the row: {cancel:?} {row:?}");
    assert!(row.x_range().contains(cancel.left()), "in its width: {cancel:?} {row:?}");
    let closes = h.query_all_by_label("Close").filter(|n| row.contains_rect(n.rect())).count();
    assert_eq!(closes, 0, "no Close: its Cancel answers it");
    tell.send(()).unwrap();
    settle(&mut h);
    assert!(h.query_by_label("Cancel").is_none(), "gone with the job");
}

/// A picker (Select Portrait) has a Close in its title row, as the
/// Pickers board draws it: it closes the picker as its Cancel does,
/// nothing chosen, and the Properties window it was opened from stays.
#[test]
fn a_picker_s_close_is_its_cancel() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_close", ResType::UTC) else {
        return;
    };
    h.run();
    let portrait = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&key).unwrap().root.integer("PortraitId")
    };
    let before = portrait(&mut h);
    h.get_by_label("Portraits…").click();
    h.run();
    // (A portrait chosen, not taken: the Close drops it.)
    h.get_by_label("po_dw_f_01_").click();
    settle(&mut h);
    // (Where the picker is once its pictures are in, its Close waited for:
    // the picker grows about its middle as they come.)
    let in_picker = |h: &Harness<'_, Moonglow>| {
        let window = window_rect(h, "Select Portrait");
        h.query_all_by_label("Close").map(|n| n.rect()).find(|r| window.contains_rect(*r))
    };
    let started = std::time::Instant::now();
    while in_picker(&h).is_none() && started.elapsed().as_secs() < 10 {
        h.step();
    }
    let window = window_rect(&h, "Select Portrait");
    let close = h.query_all_by_label("Close").find(|n| window.contains_rect(n.rect()));
    let close = close.expect("a Close in the picker's title row");
    assert!(close.rect().top() - window.top() < 6.0, "in its title row");
    close.click();
    settle(&mut h);
    assert!(h.query_by_label("Select Portrait").is_none(), "closed");
    assert_eq!(portrait(&mut h), before, "nothing chosen");
    assert!(h.state().dock.find_tab(&Tab::Blueprint(key)).is_some(), "its Properties stay");
}

/// A dialog resized by its edges (Find and Replace Text) is made
/// narrower by its edge as well as wider: its title row does not hold it
/// at the width it had.
#[test]
fn a_resizable_dialog_is_made_narrower_by_its_edge() {
    let dir = mg_testkit::scratch_dir("ui-dialogs-narrower");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1100.0, 800.0);
    h.run();
    // (Ctrl+H: Edit › Find and Replace Text…)
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    assert!(h.state().text_replace.is_some(), "open");
    const TITLE: &str = "Find and Replace Text";
    let was = resting(&mut h, TITLE);
    let edge = egui::pos2(was.right() - 1.0, was.center().y);
    press_drag(&mut h, edge, edge - egui::vec2(120.0, 0.0));
    let narrower = resting(&mut h, TITLE);
    assert!(narrower.width() < was.width() - 100.0, "{narrower:?} from {was:?}");
    let edge = egui::pos2(narrower.right() - 1.0, narrower.center().y);
    press_drag(&mut h, edge, edge + egui::vec2(60.0, 0.0));
    let wider = resting(&mut h, TITLE);
    assert!(wider.width() > narrower.width() + 20.0, "{wider:?} from {narrower:?}");
}
