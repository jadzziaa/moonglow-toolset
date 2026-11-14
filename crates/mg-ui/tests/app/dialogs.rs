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

/// The window sizes a dialog must hold in: the least window
/// (`LEAST_WINDOW_SIZE`, 420 by 320) and a short one.
const SHORT_WINDOWS: [egui::Vec2; 2] = [mg_ui::LEAST_WINDOW_SIZE, egui::vec2(800.0, 450.0)];

/// The dialog `title` held in the window whatever its size: wholly inside
/// it, its title row in sight, and each of `buttons` (its last ones)
/// brought into sight where its body scrolls, inside the dialog and the
/// window; then the last of them clicked, which answers it.
fn reachable(h: &mut Harness<'_, Moonglow>, title: &str, buttons: &[&str], at: egui::Vec2) {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, at);
    let window = resting(h, title);
    assert!(screen.contains_rect(window.shrink(0.5)), "{title} at {at:?}: {window:?}");
    let row = egui::Rect::from_min_size(window.min, egui::vec2(window.width(), 28.0));
    assert!(screen.contains_rect(row.shrink(0.5)), "{title} at {at:?}: its row {row:?}");
    // (Its buttons found within it: a window behind it has an OK and a
    // Cancel too.)
    let mine = |h: &Harness<'_, Moonglow>, button: &str| -> Option<egui::Rect> {
        let window = h.get_by_label(title);
        window.query_all_by_label(button).map(|n| n.rect()).next()
    };
    for button in buttons {
        let window = h.get_by_label(title);
        let node = window.query_all_by_label(button).next();
        node.unwrap_or_else(|| panic!("{title} at {at:?}: its {button}")).scroll_to_me();
        h.run();
        let window = resting(h, title);
        let r = mine(h, button).unwrap_or_else(|| panic!("{title} at {at:?}: its {button}"));
        let seen = window.intersect(screen);
        assert!(seen.contains_rect(r.shrink(0.5)), "{title} at {at:?}: {button} {r:?} {seen:?}");
    }
    let last = buttons.last().unwrap();
    resting(h, title);
    h.get_by_label(title).query_all_by_label(last).next().unwrap().click();
    settle(h);
    assert!(h.query_by_label(title).is_none(), "{title} at {at:?}: answered by its {last}");
}

/// The Area Wizard (the tallest dialog over an area) in the least window,
/// 420 by 320, and in a short one, 800 by 450: in the window, its title
/// row in sight, its body scrolling to Create and Cancel.
#[test]
fn a_tall_dialog_stays_whole_and_scrolls_in_a_short_window() {
    let Some((mut h, _)) = area_harness("dialog-least") else { return };
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    for at in SHORT_WINDOWS {
        h.set_size(at);
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::AreaWizard);
        settle(&mut h);
        if at == mg_ui::LEAST_WINDOW_SIZE {
            let window = resting(&mut h, "Area Wizard");
            let area = h.state().editor_area_rect().expect("the editor area");
            assert!(area.width() < window.width(), "the editor area narrower: {area:?} {window:?}");
        }
        reachable(&mut h, "Area Wizard", &["Create", "Cancel"], at);
    }
}

/// New Module (over the Welcome page) and a picker (Select Portrait, as
/// wide as most of the window) at the same sizes.
#[test]
fn new_module_and_a_picker_stay_whole_in_a_short_window() {
    for at in SHORT_WINDOWS {
        let mut h = harness(app_with(Vec::new()), at.x, at.y);
        h.run();
        h.state_mut().actions.push(mg_ui::Action::NewModuleDialog);
        settle(&mut h);
        reachable(&mut h, "New Module", &["Create", "Cancel"], at);
    }
    let Some((mut h, _)) = blueprint_harness("nw_bandit001", "bandit_short", ResType::UTC) else {
        return;
    };
    for at in SHORT_WINDOWS {
        h.set_size(at);
        settle(&mut h);
        let portraits = h.get_by_label("Portraits…");
        portraits.scroll_to_me();
        h.run();
        h.get_by_label("Portraits…").click();
        settle(&mut h);
        reachable(&mut h, "Select Portrait", &["OK", "Cancel"], at);
    }
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
    // Tab and Shift+Tab about Maximize: each stop a named control (the
    // bar behind the buttons and the body's drag catcher take none).
    for m in [egui::Modifiers::SHIFT, egui::Modifiers::NONE] {
        h.get_by_label("Maximize").focus();
        h.run();
        for i in 0..3 {
            h.key_press_modifiers(m, egui::Key::Tab);
            h.run();
            focused(&h, &format!("{m:?} Tab {}", i + 1));
        }
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
    let t = mg_uikit::tokens(&h.ctx);
    let corner = egui::Rect::from_min_max(now.right_bottom() - egui::vec2(14.0, 14.0), now.max);
    let grip_strokes = |h: &Harness<'_, Moonglow>, color: egui::Color32| {
        (h.output().shapes.iter())
            .filter(|s| match &s.shape {
                egui::Shape::LineSegment { points, stroke } => {
                    stroke.color == color && points.iter().all(|p| corner.expand(2.0).contains(*p))
                }
                _ => false,
            })
            .count()
    };
    assert_eq!(grip_strokes(&h, t.text_off), 2, "the grip's two strokes in {corner:?}");
    // Lit where egui resizes by the corner (its grab radius, 10 points),
    // not past it.
    let window_corner = now.max + egui::vec2(1.0, 1.0);
    for (off, lit) in [(13.0, false), (4.0, true)] {
        h.hover_at(window_corner - egui::vec2(off, off));
        h.run_steps(1); // (One step: the hover's frame.)
        let color = if lit { t.text_2 } else { t.text_off };
        assert_eq!(grip_strokes(&h, color), 2, "{off} points in: lit {lit}");
    }
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
    // (Under the Top Bar, where windows open: the editor area's top down
    // to the window's foot.)
    let screen = h.ctx.content_rect();
    let top = h.state().editor_area_rect().expect("the editor area").top();
    let room = egui::Rect::from_x_y_ranges(screen.x_range(), top..=screen.bottom());
    assert!(room.expand(1.0).contains_rect(rect), "{rect:?} inside {room:?}");
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
    // (Where the picker is once it rests: its first frame is egui's
    // sizing pass, laid out at a default place, and it grows about its
    // middle as its pictures come.)
    let window = resting(&mut h, "Select Portrait");
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

/// The node with the keyboard's focus: its name and where it is drawn,
/// failing where none has it (a widget with no node: nothing a screen
/// reader can say, nothing drawn).
fn focused(h: &Harness<'_, Moonglow>, after: &str) -> (String, egui::Rect) {
    let found: Vec<(String, egui::Rect)> = h
        .query_all_by(|n| n.is_focused())
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
        .collect();
    let [(name, rect)] = &found[..] else { panic!("after {after}: focused {found:?}") };
    assert!(!name.is_empty(), "after {after}: a stop with no name at {rect:?}");
    assert!(rect.is_positive(), "after {after}: {name} not drawn ({rect:?})");
    (name.clone(), *rect)
}

/// Tab through a dialog, forward and backward: every stop is a control
/// with a name, drawn where it is (none of the catchers behind the body
/// or the bar takes the keyboard).
#[test]
fn every_stop_of_tab_in_a_dialog_is_a_named_control() {
    let mut h = about_harness("ui-dialogs-about-tab");
    for (key, back) in [("Tab", false), ("Shift+Tab", true)] {
        h.get_by_label("Check for Updates…").focus();
        h.run();
        focused(&h, "focus");
        for i in 0..4 {
            let m = if back { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
            h.key_press_modifiers(m, egui::Key::Tab);
            h.run();
            focused(&h, &format!("{key} {}", i + 1));
        }
    }
}

/// A dialog that folds (Find and Replace Text, which egui's own arrow
/// folded) has the arrow at its title row's left, "Collapse": clicked, the
/// dialog is its row alone and the arrow is "Expand"; clicked again, the
/// dialog is as it was, its size kept; and so by the keyboard (the arrow
/// focused, Enter). A dialog its buttons answer (New Module) has none.
#[test]
fn a_dialog_folds_to_its_title_row_and_back() {
    let dir = mg_testkit::scratch_dir("ui-dialogs-fold");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1100.0, 800.0);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    const TITLE: &str = "Find and Replace Text";
    let open = resting(&mut h, TITLE);
    let arrow = |h: &Harness<'_, Moonglow>, name: &str| -> Option<egui::Rect> {
        h.get_by_label(TITLE).query_all_by_label(name).map(|n| n.rect()).next()
    };
    let at = arrow(&h, "Collapse").expect("the arrow at its row's left");
    assert!(at.left() - open.left() < 6.0 && at.top() - open.top() < 6.0, "{at:?} {open:?}");
    tip_at(&mut h, at.center(), "Collapse the dialog to its title row");
    for keyboard in [false, true] {
        let press = |h: &mut Harness<'_, Moonglow>, name: &str| {
            let node = h.get_by_label(TITLE).query_all_by_label(name).next().unwrap();
            if keyboard {
                node.focus();
                h.run();
                h.key_press(egui::Key::Enter);
            } else {
                node.click();
            }
            settle(h);
        };
        press(&mut h, "Collapse");
        let folded = resting(&mut h, TITLE);
        assert!(folded.height() < 32.0, "its row alone (keyboard {keyboard}): {folded:?}");
        assert!(arrow(&h, "Collapse").is_none() && arrow(&h, "Expand").is_some());
        assert!(h.query_by_label("Find what").is_none(), "no body while folded");
        press(&mut h, "Expand");
        let back = resting(&mut h, TITLE);
        assert!((back.size() - open.size()).length() < 1.0, "{back:?} as {open:?}");
        assert!(arrow(&h, "Collapse").is_some(), "open again");
    }
    // A dialog its buttons answer folds not.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::NewModuleDialog);
    settle(&mut h);
    let wizard = h.get_by_label("New Module");
    assert!(wizard.query_all_by_label("Collapse").next().is_none(), "no arrow on a wizard");
}

/// The bar's buttons and tabs of the pane of `tab`'s window, by the
/// accessibility tree: each node's name and where it is, within the bar.
fn window_bar_nodes(
    h: &Harness<'_, Moonglow>,
    tab: &Tab,
) -> (egui::Rect, Vec<(String, egui::Rect)>) {
    let (pane, _) = pane(h, tab);
    let s = h.state();
    let leaf = s.dock.iter_leaves().find(|(_, l)| l.tabs.contains(tab)).unwrap().1;
    let bar = egui::Rect::from_x_y_ranges(pane.x_range(), pane.top()..=leaf.viewport.top());
    let mut out = Vec::new();
    for role in [egui::accesskit::Role::Tab, egui::accesskit::Role::Button] {
        for n in h.get_all_by_role(role) {
            let r = n.rect();
            if r.intersects(bar.shrink(1.0))
                && r.top() >= bar.top() - 1.0
                && r.bottom() <= bar.bottom() + 1.0
            {
                out.push((n.accesskit_node().label().unwrap_or_default(), r));
            }
        }
    }
    (bar, out)
}

/// What takes the pointer in a window's bar's row as a tab does (a click
/// and a drag), where it does: each tab cut where the bar cuts it. (On
/// the windows' layers: the main surface's tabs lie under it.)
fn bar_tab_hits(h: &Harness<'_, Moonglow>, bar: egui::Rect) -> Vec<egui::Rect> {
    let mut out = Vec::new();
    h.ctx.viewport(|v| {
        for (layer, widgets) in v.prev_pass.widgets.layers() {
            if layer.order != egui::Order::Middle {
                continue;
            }
            for w in widgets.iter() {
                let r = w.interact_rect;
                if w.sense.senses_click()
                    && w.sense.senses_drag()
                    && r.is_positive()
                    && (r.center().y - bar.center().y).abs() < 2.0
                    && r.height() < bar.height() + 1.0
                    && bar.intersects(r)
                {
                    out.push(r);
                }
            }
        }
    });
    out
}

/// A floating window's bar at every width it can have, from its least to
/// 900 points, with 1, 3 and 8 tabs: the collapse arrow, Maximize and
/// Close Window always there and inside the bar; every tab and every
/// tab's close ending before Maximize; no two buttons on one point.
#[test]
fn a_window_s_bar_holds_at_every_width() {
    let Some((mut h, area)) = area_harness("window-bar-sweep") else { return };
    let props = Tab::AreaProperties(area);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
    settle(&mut h);
    let more = [
        Tab::Factions,
        Tab::Journal,
        Tab::Manual,
        Tab::References,
        Tab::AdvancedSearch,
        Tab::ModuleProperties,
        Tab::PlaceableGallery,
    ];
    let mut shown = 1;
    for count in [1usize, 3, 8] {
        while shown < count {
            let tab = more[shown - 1].clone();
            let s = h.state_mut();
            let path = s.dock.find_tab(&props).expect("open");
            s.dock.set_focused_node_and_surface(path.node_path());
            s.dock.push_to_focused_leaf(tab);
            shown += 1;
        }
        settle(&mut h);
        let surface = {
            let path = h.state().dock.find_tab(&props).unwrap();
            assert!(!path.surface.is_main());
            path.surface
        };
        // (From under its least width: it is held there.)
        let mut width = 40.0;
        while width <= 900.0 {
            let s = h.state_mut();
            // (Its bar below the main surface's tab bar: nothing of that
            // one lies under it.)
            let state = s.dock.get_window_state_mut(surface).unwrap();
            state.set_position(egui::pos2(200.0, 200.0)).set_size(egui::vec2(width, 500.0));
            settle(&mut h);
            let (bar, nodes) = window_bar_nodes(&h, &props);
            let one = |name: &str| -> egui::Rect {
                let found: Vec<_> =
                    nodes.iter().filter(|(n, _)| n == name).map(|(_, r)| *r).collect();
                let [r] = found[..] else {
                    panic!("{count} at {width}: one {name} in {bar:?}, {found:?}")
                };
                assert!(
                    bar.expand(0.5).contains_rect(r),
                    "{count} at {width}: {name} {r:?} in {bar:?}"
                );
                r
            };
            let (collapse, maximize, close) =
                (one("Collapse"), one("Maximize"), one("Close Window"));
            // The tabs, as they take the pointer, between the arrow and
            // Maximize.
            for r in bar_tab_hits(&h, bar) {
                assert!(
                    r.left() >= collapse.right() - 0.5 && r.right() <= maximize.left() + 0.5,
                    "{count} at {width}: a tab at {r:?}, Collapse at {collapse:?}, Maximize at {maximize:?}"
                );
            }
            let buttons: Vec<&(String, egui::Rect)> = nodes
                .iter()
                .filter(|(n, _)| {
                    n.starts_with("Close ")
                        || ["Collapse", "Maximize", "Close Window"].contains(&n.as_str())
                })
                .collect();
            for (name, r) in &buttons {
                if name.starts_with("Close ") && name.as_str() != "Close Window" {
                    assert!(r.right() <= maximize.left() + 0.5, "{count} at {width}: {name} {r:?}");
                }
            }
            for (i, (a, ra)) in buttons.iter().enumerate() {
                for (b, rb) in &buttons[i + 1..] {
                    let both = ra.intersect(*rb);
                    assert!(!both.is_positive(), "{count} at {width}: {a} {ra:?} over {b} {rb:?}");
                }
            }
            let _ = close;
            width += 60.0;
        }
    }
}

/// The bar of the pane `tab` is in (not its window's first pane's).
fn leaf_bar(h: &Harness<'_, Moonglow>, tab: &Tab) -> egui::Rect {
    let s = h.state();
    let leaf = s.dock.iter_leaves().find(|(_, l)| l.tabs.contains(tab)).expect("open").1;
    egui::Rect::from_x_y_ranges(leaf.rect.x_range(), leaf.rect.top()..=leaf.viewport.top())
}

/// The button named `name` in the bar of `tab`'s pane.
fn in_leaf_bar(h: &Harness<'_, Moonglow>, tab: &Tab, name: &str) -> egui::Rect {
    let bar = leaf_bar(h, tab);
    let found: Vec<egui::Rect> = (h.query_all_by_label(name).map(|n| n.rect()))
        .filter(|r| bar.expand(1.0).contains_rect(*r))
        .collect();
    let [one] = found[..] else { panic!("one {name} in {bar:?}: {found:?}") };
    one
}

/// A click at `at` with `modifiers` held from before the press to after
/// the release (Shift and a click), a frame each.
fn click_holding(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, modifiers: egui::Modifiers) {
    h.input_mut().events.push(egui::Event::ModifiersChanged(modifiers));
    h.hover_at(at);
    h.run_steps(1);
    for pressed in [true, false] {
        let button = egui::PointerButton::Primary;
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers,
        });
        h.run_steps(1);
    }
    h.input_mut().events.push(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(1);
}

/// A window split in panes: each pane's Close is "Close Pane", tipped so,
/// and closes its pane's tabs alone, with Shift held too (Shift changes
/// nothing on a window's buttons); a tab of it that asks before closing
/// (a table with unsaved changes) asks; with one pane left its Close is
/// "Close Window".
#[test]
fn a_window_s_panes_close_one_at_a_time() {
    let Some((mut h, _)) = area_harness("window-panes") else { return };
    let dir = mg_testkit::scratch_dir("ui-window-panes");
    let file = dir.join("loose.2da");
    std::fs::write(&file, "2DA V2.0\n\n Label Cost\n0 Bread 1\n").unwrap();
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    settle(&mut h);
    let table = Tab::Table(id);
    let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
    doc.edit.set(0, 0, Some("Loaf".into())).unwrap();
    // Split: the table, Factions at its right, the Journal under Factions.
    {
        let s = h.state_mut();
        let path = s.dock.find_tab(&table).expect("open");
        assert!(!path.surface.is_main(), "in a window");
        let state = s.dock.get_window_state_mut(path.surface).unwrap();
        state.set_position(egui::pos2(200.0, 120.0)).set_size(egui::vec2(800.0, 560.0));
        let [_, right] = s.dock[path.surface].split_right(path.node, 0.5, vec![Tab::Factions]);
        s.dock[path.surface].split_below(right, 0.5, vec![Tab::Journal]);
    }
    settle(&mut h);
    for tab in [&table, &Tab::Factions, &Tab::Journal] {
        in_leaf_bar(&h, tab, "Close Pane");
    }
    let at = in_leaf_bar(&h, &Tab::Factions, "Close Pane").center();
    tip_at(&mut h, at, "Close this pane's tabs");
    // Factions' pane alone.
    let at = in_leaf_bar(&h, &Tab::Factions, "Close Pane").center();
    click_holding(&mut h, at, egui::Modifiers::NONE);
    settle(&mut h);
    let open = |h: &Harness<'_, Moonglow>, t: &Tab| h.state().dock.find_tab(t).is_some();
    assert!(!open(&h, &Tab::Factions) && open(&h, &Tab::Journal) && open(&h, &table));
    // Shift held: the Journal's pane alone too.
    let at = in_leaf_bar(&h, &Tab::Journal, "Close Pane").center();
    click_holding(&mut h, at, egui::Modifiers::SHIFT);
    settle(&mut h);
    assert!(!open(&h, &Tab::Journal) && open(&h, &table), "Shift closes the pane alone");
    // The table, unsaved: its window's Close asks.
    let at = in_leaf_bar(&h, &table, "Close Window").center();
    click_holding(&mut h, at, egui::Modifiers::NONE);
    settle(&mut h);
    h.get_by_label("Save changes to loose.2da?");
    h.get_by_label("Don't Save").click();
    settle(&mut h);
    assert!(!open(&h, &table), "closed once answered");
}
