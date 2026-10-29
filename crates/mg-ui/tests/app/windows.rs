//! Windows, panes and tabs: docking, folding, the log, and tabs opened
//! again with a module.

use crate::common::*;

/// GitHub issue 11: with the last tab of the middle closed, the palettes'
/// pane took the whole window. The middle keeps its place (a pane that
/// says no area is open), and the palettes' pane its width.
#[test]
fn the_middle_keeps_its_place_with_no_area_open() {
    let Some(mut h) = game_harness("no-area") else { return };
    h.state_mut().settings.no_last_area = true;
    h.run();
    let width = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let mut leaves = h.state().dock.iter_leaves().map(|(_, l)| l);
        leaves.find(|l| l.tabs.contains(tab)).map(|l| l.rect.width())
    };
    let has = |h: &Harness<'_, Moonglow>, tab: &Tab| h.state().dock.find_tab(tab).is_some();
    // (The module opened on its first area.)
    let area = Tab::Area(ResRef::from_str("start").unwrap());
    assert!(has(&h, &Tab::Palette) && has(&h, &area) && !has(&h, &Tab::NoArea));
    let palette = width(&h, &Tab::Palette).unwrap();

    // The middle's tabs closed (Module Properties was docked there as
    // the module opened): its place is kept.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(area.clone()));
    h.run();
    assert!(!has(&h, &Tab::NoArea), "Module Properties is in the middle still");
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    h.run();
    assert!(has(&h, &Tab::NoArea));
    h.get_by_label("No area open");
    let now = width(&h, &Tab::Palette).unwrap();
    assert!((now - palette).abs() < 3.0, "the palettes' pane: {palette} wide, then {now}");
    assert!(width(&h, &Tab::NoArea).unwrap() > 300.0);

    // An area opened takes its place (the pane lists those opened lately
    // and the module's, each a link, beside the module tree's row); the
    // palettes' pane stays.
    assert_eq!(h.query_all_by_label("start").count(), 3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(area.clone()));
    h.run();
    h.run();
    assert!(has(&h, &area) && !has(&h, &Tab::NoArea));
    let now = width(&h, &Tab::Palette).unwrap();
    assert!((now - palette).abs() < 3.0, "the palettes' pane: {palette} wide, then {now}");
    // And closed again, the same.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(area.clone()));
    h.run();
    h.run();
    assert!(has(&h, &Tab::NoArea));
    let now = width(&h, &Tab::Palette).unwrap();
    assert!((now - palette).abs() < 3.0, "the palettes' pane: {palette} wide, then {now}");
    // The area just opened is offered first, among those opened lately.
    h.get_by_label("Opened lately");
    assert_eq!(h.query_all_by_label("start").count(), 3);
    // With the resource browser docked beside the palettes (the side
    // pane's tab, not the middle's), the middle keeps its place too.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(area.clone()));
    h.run();
    h.run();
    let side = h.state().dock.find_tab(&Tab::Palette).unwrap();
    h.state_mut().dock[side.surface][side.node].append_tab(Tab::Resources);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::CloseTab(area));
    h.run();
    h.run();
    assert!(has(&h, &Tab::NoArea), "the middle is kept with Resources in the side pane");
    let now = width(&h, &Tab::Palette).unwrap();
    assert!((now - palette).abs() < 3.0, "the palettes' pane: {palette} wide, then {now}");
}

/// Without the palettes (no game), the middle is not left blank either.
#[test]
fn the_middle_says_no_area_is_open_without_the_palettes() {
    let dir = mg_testkit::scratch_dir("ui-no-area-plain");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    assert!(h.state().dock.find_tab(&Tab::NoArea).is_none());
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    h.run();
    h.run();
    h.get_by_label("No area open");
    // Closing the module brings the start page back, not this.
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::NoArea).is_none());
}

/// GitHub issue 15: the tabs by the keyboard, as in a browser. Ctrl+W
/// closes the one in front, Ctrl+Tab and Ctrl+Shift+Tab (and Ctrl+Page
/// Down and Up) go round them, Ctrl and a digit goes to one by its number
/// (9: the last), Ctrl+Shift+T opens the one closed last again.
#[test]
fn tabs_by_the_keyboard() {
    let dir = mg_testkit::scratch_dir("ui-tab-keys");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    for name in ["second", "third"] {
        let mut a = ifo::MOD_AREA_LIST.new_item();
        a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str(name).unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
        m.set(ResKey::parse(name, ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
    }
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let area = |name: &str| Tab::Area(ResRef::from_str(name).unwrap());
    for name in ["start", "second", "third"] {
        app.actions.push(mg_ui::Action::OpenTab(area(name)));
    }
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.run();
    // The middle's tabs, and the one in front.
    let tabs = |h: &Harness<'_, Moonglow>| -> (Vec<Tab>, Tab) {
        let leaf = (h.state().dock.iter_leaves())
            .find(|(p, l)| p.surface.is_main() && l.tabs.iter().any(|t| matches!(t, Tab::Area(_))))
            .map(|(_, l)| l)
            .expect("the areas' pane");
        (leaf.tabs.clone(), leaf.tabs[leaf.active.0].clone())
    };
    let front = |h: &Harness<'_, Moonglow>| tabs(h).1;
    let key = |h: &mut Harness<'_, Moonglow>, modifiers: egui::Modifiers, key: egui::Key| {
        h.key_press_modifiers(modifiers, key);
        h.run();
        h.run();
    };
    let (ctrl, shift) = (egui::Modifiers::COMMAND, egui::Modifiers::SHIFT);
    // (Module Properties is docked there first, as the module opened.)
    let all = tabs(&h).0;
    assert_eq!(all[1..], [area("start"), area("second"), area("third")]);
    assert_eq!(front(&h), area("third"));

    // Round them: on from the last is the first.
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front(&h), Tab::ModuleProperties);
    key(&mut h, ctrl, egui::Key::PageDown);
    assert_eq!(front(&h), area("start"));
    key(&mut h, ctrl | shift, egui::Key::Tab);
    key(&mut h, ctrl, egui::Key::PageUp);
    assert_eq!(front(&h), area("third"));
    // By number: the third tab, and the last.
    key(&mut h, ctrl, egui::Key::Num3);
    assert_eq!(front(&h), area("second"));
    key(&mut h, ctrl, egui::Key::Num9);
    assert_eq!(front(&h), area("third"));
    key(&mut h, ctrl, egui::Key::Num2);
    assert_eq!(front(&h), area("start"));
    // With the option (Options › General), Ctrl+Tab goes to the tab used
    // last, and again back: third was in front before start.
    h.state_mut().settings.tabs_by_recent = true;
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front(&h), area("third"));
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front(&h), area("start"));
    // (The other way round the same order: the one used longest ago.)
    key(&mut h, ctrl | shift, egui::Key::Tab);
    assert_eq!(front(&h), Tab::ModuleProperties);
    h.state_mut().settings.tabs_by_recent = false;
    key(&mut h, ctrl, egui::Key::Num2);
    assert_eq!(front(&h), area("start"));

    // Ctrl+W closes the one in front; Ctrl+Shift+T opens it again.
    key(&mut h, ctrl, egui::Key::W);
    assert!(!tabs(&h).0.contains(&area("start")));
    key(&mut h, ctrl | shift, egui::Key::T);
    assert_eq!(front(&h), area("start"));
    // Close Others (a tab's menu), and each of them opened again, the one
    // closed last first.
    let others: Vec<Tab> = tabs(&h).0.into_iter().filter(|t| *t != area("start")).collect();
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(others));
    h.run();
    assert_eq!(tabs(&h).0, [area("start")]);
    key(&mut h, ctrl | shift, egui::Key::T);
    assert_eq!(front(&h), area("third"));
    key(&mut h, ctrl | shift, egui::Key::T);
    key(&mut h, ctrl | shift, egui::Key::T);
    // (Module Properties comes back in a window of its own, as it opens.)
    assert_eq!(tabs(&h).0.len(), 3);
    assert!(h.state().dock.find_tab(&Tab::ModuleProperties).is_some());
    // Nothing more to open: the key does nothing.
    key(&mut h, ctrl | shift, egui::Key::T);
    assert_eq!(h.state().dock.iter_all_tabs().count(), 4);
}

/// GitHub issue 15, with Options › General's Ctrl+Tab by when the tabs
/// were last used: with Ctrl held, each Tab goes on through the order as
/// it was when the keys began (not back to the tab before, as one press
/// and another do), and the tab arrived at is the one used last once Ctrl
/// is let go. A window's own tabs go by when they were used too, kept
/// while the keys were elsewhere.
#[test]
fn tabs_by_when_they_were_last_used_with_ctrl_held() {
    let dir = mg_testkit::scratch_dir("ui-tab-recent");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    for name in ["second", "third"] {
        let mut a = ifo::MOD_AREA_LIST.new_item();
        a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str(name).unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
        m.set(ResKey::parse(name, ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
    }
    for name in ["one", "two", "three"] {
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    }
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.tabs_by_recent = true;
    app.open_module(&path);
    let area = |name: &str| Tab::Area(ResRef::from_str(name).unwrap());
    for name in ["start", "second", "third"] {
        app.actions.push(mg_ui::Action::OpenTab(area(name)));
    }
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.run();
    let front_of = |h: &Harness<'_, Moonglow>, of: &Tab| {
        let path = h.state().dock.find_tab(of).unwrap();
        let leaf = h.state().dock.leaf(path.node_path()).unwrap();
        leaf.tabs[leaf.active.0].clone()
    };
    let front = |h: &Harness<'_, Moonglow>| front_of(h, &area("start"));
    let ctrl = egui::Modifiers::COMMAND;
    let key = |h: &mut Harness<'_, Moonglow>, modifiers: egui::Modifiers, key: egui::Key| {
        h.key_press_modifiers(modifiers, key);
        h.run();
        h.run();
    };
    // Ctrl held down over several presses of Tab, and let go after.
    let tab = |h: &mut Harness<'_, Moonglow>| {
        for pressed in [true, false] {
            h.event(egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: ctrl,
            });
        }
        h.run();
        h.run();
    };
    let hold = |h: &mut Harness<'_, Moonglow>, held: bool| {
        h.event(egui::Event::ModifiersChanged(if held { ctrl } else { egui::Modifiers::NONE }));
        h.run();
        h.run();
    };
    // Used last: start, then second, then third, then Module Properties.
    for n in [egui::Key::Num1, egui::Key::Num4, egui::Key::Num3, egui::Key::Num2] {
        key(&mut h, ctrl, n);
    }
    assert_eq!(front(&h), area("start"));
    hold(&mut h, true);
    tab(&mut h);
    assert_eq!(front(&h), area("second"), "the one used before");
    tab(&mut h);
    assert_eq!(front(&h), area("third"), "on, not back to start");
    tab(&mut h);
    assert_eq!(front(&h), Tab::ModuleProperties);
    tab(&mut h);
    assert_eq!(front(&h), area("start"), "round the end");
    tab(&mut h);
    tab(&mut h);
    assert_eq!(front(&h), area("third"));
    hold(&mut h, false);
    // Let go: third is the one used last, start the one before it.
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front(&h), area("start"));
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front(&h), area("third"));

    // A window's own tabs brought to its front two, three, one while the
    // keys are in the middle (as a script opened from a field's Edit
    // comes to the front of the scripts' window): in the window, by when
    // its tabs were in front.
    let script = |name: &str| Tab::Script(ResKey::parse(name, ResType::NSS).unwrap());
    let window = h.state_mut().dock.add_window(vec![script("one"), script("two"), script("three")]);
    assert!(!window.is_main());
    let to_front = |h: &mut Harness<'_, Moonglow>, tab: &Tab, focused: bool| {
        let path = h.state().dock.find_tab(tab).unwrap();
        let dock = &mut h.state_mut().dock;
        let _ = dock.set_active_tab(path);
        if focused {
            dock.set_focused_node_and_surface(path.node_path());
        }
        h.run();
        h.run();
    };
    to_front(&mut h, &area("start"), true);
    for name in ["two", "three", "one"] {
        to_front(&mut h, &script(name), false);
    }
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front(&h), area("third"), "the middle's, by its own order");
    to_front(&mut h, &script("one"), true);
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front_of(&h, &script("one")), script("three"), "the window's used before one");
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front_of(&h, &script("one")), script("one"));
    // With Ctrl held, on through the window's order: three, then two.
    hold(&mut h, true);
    tab(&mut h);
    tab(&mut h);
    assert_eq!(front_of(&h, &script("one")), script("two"));
    hold(&mut h, false);
}

/// GitHub issue 10: the panes beside the middle fold away and come back
/// (View, Ctrl+Alt+1, 2, 3 and 0, and each pane's own button, on the edge
/// it faces the middle by), each as wide as it was.
#[test]
fn panels_fold_away_and_come_back() {
    let Some(mut h) = game_harness("panels") else { return };
    h.run();
    h.run();
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press_modifiers(keys, key);
        h.run();
        h.run();
    };
    let palette = |h: &Harness<'_, Moonglow>| {
        let mut leaves = h.state().dock.iter_leaves().map(|(_, l)| l);
        leaves.find(|l| l.tabs.contains(&Tab::Palette)).map(|l| l.rect.width())
    };
    let tree = |h: &Harness<'_, Moonglow>| h.query_by_label("Fold Module Tree").is_some();
    let was = palette(&h).expect("the palettes' pane");
    assert!(tree(&h));

    // The module tree: folded to a strip, and back by the strip.
    key(&mut h, egui::Key::Num1);
    assert!(!tree(&h) && h.state().settings.hide_tree);
    h.get_by_label("Show Module Tree").click();
    h.run();
    assert!(tree(&h) && !h.state().settings.hide_tree);
    // Its own button folds it away too; so does the log's.
    h.get_by_label("Fold Module Tree").click();
    h.run();
    assert!(!tree(&h) && h.state().settings.hide_tree);
    h.get_by_label("Show Module Tree").click();
    h.run();
    h.get_by_label("Fold Log").click();
    h.run();
    assert!(h.state().settings.hide_log);
    h.get_by_label("Show Log").click();
    h.run();
    assert!(!h.state().settings.hide_log && tree(&h));
    // The palettes: their pane goes, and comes back as wide.
    key(&mut h, egui::Key::Num2);
    assert!(palette(&h).is_none() && h.state().settings.hide_palettes);
    h.get_by_label("Show Palettes").click();
    h.run();
    h.run();
    let now = palette(&h).expect("back");
    assert!((now - was).abs() < 3.0, "{was} wide, then {now}");
    // Their own button folds them away as the tree's does.
    h.get_by_label("Fold Palettes").click();
    h.run();
    assert!(palette(&h).is_none() && h.state().settings.hide_palettes);
    h.get_by_label("Show Palettes").click();
    h.run();
    h.run();
    assert!(palette(&h).is_some());
    // The log.
    key(&mut h, egui::Key::Num3);
    assert!(h.state().settings.hide_log);
    h.get_by_label("Show Log").click();
    h.run();
    assert!(!h.state().settings.hide_log);
    // All of them, and all back.
    key(&mut h, egui::Key::Num0);
    let s = &h.state().settings;
    assert!(s.hide_tree && s.hide_log && s.hide_palettes && palette(&h).is_none());
    key(&mut h, egui::Key::Num0);
    let s = &h.state().settings;
    assert!(!s.hide_tree && !s.hide_log && !s.hide_palettes);
    let now = palette(&h).expect("back");
    assert!((now - was).abs() < 3.0, "{was} wide, then {now}");
    // The View menu has them, ticked.
    h.get_by_label("View").click();
    h.run();
    h.get_by_label_contains("✔ Module Tree");
    h.get_by_label("Reset Layout").click();
    h.run();
    h.run();
    assert!(palette(&h).is_some());
}

/// GitHub issue 10, as its reporter put it on 1.31.0: one button per pane
/// to fold it away, on the edge the pane faces the middle by (the module
/// tree's right, the palettes' left, the log's top), centered along it,
/// and in the same place folded, where it is all the pane leaves, in the
/// window; the palettes' tab bar has no other (egui_dock's arrow that
/// folded the pane to its bar).
#[test]
fn each_pane_folds_by_one_button_on_its_inner_edge() {
    let Some(mut h) = game_harness("pane-edges") else { return };
    h.run_steps(3);
    // A pane's button, folded or not (and that there is one).
    let button = |h: &Harness<'_, Moonglow>, name: &str| {
        let fold = format!("Fold {name}");
        let show = format!("Show {name}");
        let all: Vec<egui::Rect> = (h.query_all_by_label(&fold).chain(h.query_all_by_label(&show)))
            .map(|n| n.rect())
            .collect();
        assert_eq!(all.len(), 1, "one button for the {name}: {all:?}");
        all[0]
    };
    // The middle (the dock's panes but the palettes'), and the palettes'.
    let middle = |h: &Harness<'_, Moonglow>| {
        let leaves = h.state().dock.iter_leaves().filter(|(p, _)| p.surface.is_main());
        let rects = leaves.filter(|(_, l)| !l.tabs.contains(&Tab::Palette)).map(|(_, l)| l.rect);
        rects.reduce(|a, b| a.union(b)).expect("a middle")
    };
    let palettes = |h: &Harness<'_, Moonglow>| {
        let mut leaves = h.state().dock.iter_leaves();
        leaves.find(|(_, l)| l.tabs.contains(&Tab::Palette)).map(|(_, l)| (l.rect, l.collapsed))
    };
    let window = h.ctx.content_rect();
    let near = |a: f32, b: f32, what: &str| assert!((a - b).abs() < 1.5, "{what}: {a} and {b}");
    // (The dock's panes stand 8 points inside the middle's panel, all
    // round; the panels, as far inside the window.)
    const MARGIN: f32 = 8.0;

    // The module tree: on its right edge, toward the middle, half way
    // down it; folded, the same, at the window's left.
    let (shown, mid) = (button(&h, "Module Tree"), middle(&h));
    assert!(shown.left() > window.left() + 150.0, "right of the tree: {shown:?}");
    near(shown.right() + MARGIN, mid.left(), "beside the middle");
    let (pane, _) = palettes(&h).unwrap();
    near(shown.center().y, mid.union(pane).center().y, "half way down");
    assert!(shown.width() >= 12.0 && shown.height() > 2.0 * shown.width(), "{shown:?}");
    // (A click at the edge, where the tree is resized, is the button's.)
    let at = egui::pos2(shown.left() + 2.0, shown.center().y);
    h.hover_at(at);
    h.run();
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    assert!(h.state().settings.hide_tree, "folded by the click at its edge");
    let folded = button(&h, "Module Tree");
    assert!(h.query_by_label("Fold Module Tree").is_none());
    assert!(window.contains_rect(folded), "in the window: {folded:?}");
    near(folded.left(), window.left() + MARGIN, "at the window's edge, in its frame");
    near(folded.right() + MARGIN, middle(&h).left(), "beside the middle");
    assert_eq!(folded.size(), shown.size());
    near(folded.center().y, shown.center().y, "where it was");
    h.get_by_label("Show Module Tree").click();
    h.run_steps(2);
    assert!(!h.state().settings.hide_tree);
    assert_eq!(button(&h, "Module Tree"), shown, "back where it was");

    // The palettes: on their left edge, half way down.
    let (pane, collapsed) = palettes(&h).unwrap();
    assert!(!collapsed);
    let shown = button(&h, "Palettes");
    near(shown.left(), pane.left(), "on the palettes' left edge");
    near(shown.center().y, pane.center().y, "half way down");
    // Their tab bar's arrow is no button: a click there folds nothing.
    let arrow = egui::pos2(pane.left() + 12.0, pane.top() + 10.0);
    h.hover_at(arrow);
    h.run();
    press(&h, arrow, true, egui::Modifiers::NONE);
    press(&h, arrow, false, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(palettes(&h).map(|p| p.1), Some(false), "not folded to the tab bar");
    assert!(!h.state().settings.hide_palettes);
    h.get_by_label("Fold Palettes").click();
    h.run_steps(3);
    assert!(palettes(&h).is_none());
    let folded = button(&h, "Palettes");
    assert!(window.contains_rect(folded), "in the window: {folded:?}");
    near(folded.right(), window.right() - MARGIN, "at the window's edge, in its frame");
    near(folded.left() - MARGIN, middle(&h).right(), "beside the middle");
    assert_eq!(folded.size(), shown.size());
    near(folded.center().y, shown.center().y, "where it was");
    h.get_by_label("Show Palettes").click();
    h.run_steps(3);
    assert!(palettes(&h).is_some());
    near(button(&h, "Palettes").center().y, shown.center().y, "back where it was");

    // The log: on its top edge, half way across.
    let shown = button(&h, "Log");
    let clear = h.get_by_label("Clear Log").rect();
    assert!(shown.bottom() <= clear.top(), "over the log: {shown:?} {clear:?}");
    near(shown.top() - MARGIN, middle(&h).bottom(), "under the middle");
    near(shown.center().x, window.center().x, "half way across");
    assert!(shown.height() >= 12.0 && shown.width() > 2.0 * shown.height(), "{shown:?}");
    h.get_by_label("Fold Log").click();
    h.run_steps(2);
    assert!(h.state().settings.hide_log);
    let folded = button(&h, "Log");
    assert!(window.contains_rect(folded), "in the window: {folded:?}");
    near(folded.top() - MARGIN, middle(&h).bottom(), "under the middle");
    assert_eq!(folded.size(), shown.size());
    near(folded.center().x, shown.center().x, "where it was");
    h.get_by_label("Show Log").click();
    h.run_steps(2);
    assert!(!h.state().settings.hide_log);
    // (The others are gone: the tree's «, the palettes' » after their
    // types, the log's ⏷ in its corner and the ⏶ Log of its strip.)
    for old in ["«", "»", "⏷ Log", "⏶ Log", "⏴"] {
        assert!(h.query_by_label(old).is_none(), "{old}");
    }
}

/// GitHub issue 20: the palettes' tab dragged onto the middle of the
/// middle pane joins its tabs; the area tabs there stay where they are.
#[test]
fn the_palettes_dropped_on_the_middle_join_its_tabs() {
    let Some((mut h, area)) = area_harness("dock-palettes") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    h.run_steps(3);
    let layout = |h: &Harness<'_, Moonglow>| {
        let mut out: Vec<String> = h
            .state()
            .dock
            .iter_leaves()
            .filter(|(p, _)| p.surface.is_main())
            .map(|(p, leaf)| format!("{:?}: {:?}", p.node, leaf.tabs))
            .collect();
        out.sort();
        out
    };
    // (The tab's title: at the start of its pane's tab bar.)
    let leaf = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        h.state()
            .dock
            .iter_leaves()
            .find(|(_, l)| l.tabs.contains(tab))
            .map(|(_, l)| (l.rect, l.viewport))
            .unwrap()
    };
    let (rect, viewport) = leaf(&h, &Tab::Palette);
    let tab = egui::pos2(rect.left() + 40.0, (rect.top() + viewport.top()) / 2.0);
    let target = h
        .state()
        .dock
        .iter_leaves()
        .find(|(_, l)| l.tabs.contains(&Tab::Area(area)))
        .map(|(_, l)| l.rect)
        .unwrap();
    let to = target.center();
    h.hover_at(tab);
    press(&h, tab, true, egui::Modifiers::NONE);
    for k in 1..=10 {
        h.hover_at(tab + (to - tab) * (k as f32 / 10.0));
        h.step();
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    // One pane: the area's tab where it was, the palettes beside it.
    let after = layout(&h);
    assert_eq!(after.len(), 1, "{after:?}");
    assert!(after[0].contains("Area(") && after[0].contains("Palette"), "{after:?}");
    assert!(!after[0].contains("NoArea"), "{after:?}");
}

/// GitHub issue 21: the resource browser dropped on a side of the middle
/// pane is docked there, beside it, as the targets drawn say. (From a pane
/// of its own beside the palettes; dragged from a window of its own, the
/// window moves, here and in the report, and was not seen to dock.)
#[test]
fn the_resource_browser_dropped_on_a_side_is_docked_there() {
    let Some((mut h, area)) = area_harness("dock-resources") else { return };
    // (Docked beside the palettes, as the reporter had it.)
    {
        let dock = &mut h.state_mut().dock;
        let palettes = dock.find_tab(&Tab::Palette).unwrap();
        dock.set_focused_node_and_surface(egui_dock::NodePath {
            surface: palettes.surface,
            node: palettes.node,
        });
        dock.push_to_focused_leaf(Tab::Resources);
    }
    h.run_steps(4);
    let layout = |h: &Harness<'_, Moonglow>| {
        let mut out: Vec<String> = h
            .state()
            .dock
            .iter_leaves()
            .map(|(p, leaf)| {
                format!("{} {:?} {:?}: {:?}", p.surface.is_main(), p.node, leaf.rect, leaf.tabs)
            })
            .collect();
        out.sort();
        out
    };
    let leaf = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let found = h.state().dock.iter_leaves().find(|(_, l)| l.tabs.contains(tab));
        found.map(|(_, l)| (l.rect, l.viewport)).unwrap()
    };
    let (rect, viewport) = leaf(&h, &Tab::Resources);
    // (Its tab: the second in the bar, after the palettes'.)
    // (Its tab: the second in the bar, after the palettes'.)
    let tab = egui::pos2(rect.left() + 160.0, (rect.top() + viewport.top()) / 2.0);
    let (target, _) = leaf(&h, &Tab::Area(area));
    // The left target: the button left of the middle one.
    let to = target.center() + egui::vec2(-105.0, 0.0);
    h.hover_at(tab);
    press(&h, tab, true, egui::Modifiers::NONE);
    for k in 1..=12 {
        h.hover_at(tab + (to - tab) * (k as f32 / 12.0));
        h.step();
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    // A pane of its own, left of the area's; the palettes where they were.
    let main = |tab: &Tab| {
        let found = h.state().dock.iter_leaves().find(|(_, l)| l.tabs.contains(tab));
        found.map(|(p, l)| (p.surface.is_main(), l.tabs.len(), l.rect.center().x)).unwrap()
    };
    let (resources, area_pane) = (main(&Tab::Resources), main(&Tab::Area(area)));
    assert_eq!((resources.0, resources.1), (true, 1), "{:?}", layout(&h));
    assert!(resources.2 < area_pane.2, "left of the area: {:?}", layout(&h));
    assert_eq!(main(&Tab::Palette).1, 1, "{:?}", layout(&h));
}

/// GitHub issue 23: with the resource browser beside the palettes, the
/// area closed and then the palettes, the browser stays a side pane: the
/// middle keeps its place beside it (it took the whole window).
#[test]
fn the_resource_browser_alone_keeps_the_middle_beside_it() {
    let Some((mut h, area)) = area_harness("dock-browser-alone") else { return };
    {
        let dock = &mut h.state_mut().dock;
        let palettes = dock.find_tab(&Tab::Palette).unwrap();
        dock.set_focused_node_and_surface(egui_dock::NodePath {
            surface: palettes.surface,
            node: palettes.node,
        });
        dock.push_to_focused_leaf(Tab::Resources);
    }
    h.run_steps(3);
    for tab in [Tab::Area(area), Tab::ModuleProperties, Tab::Palette] {
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab]));
        h.run_steps(3);
    }
    let leaves: Vec<(Vec<Tab>, f32)> = h
        .state()
        .dock
        .iter_leaves()
        .filter(|(p, _)| p.surface.is_main())
        .map(|(_, l)| (l.tabs.clone(), l.rect.center().x))
        .collect();
    assert_eq!(leaves.len(), 2, "{leaves:?}");
    let x = |tab: &Tab| leaves.iter().find(|(t, _)| t.contains(tab)).map(|(_, x)| *x).unwrap();
    assert!(x(&Tab::NoArea) < x(&Tab::Resources), "the middle, then the browser: {leaves:?}");
}

/// GitHub issue 22: a tool window closed in a pane split off a side of
/// the middle opens there again, as large; and so do the palettes split
/// off another side than their right.
#[test]
fn a_tool_window_reopens_in_its_split_of_the_middle() {
    let Some((mut h, area)) = area_harness("dock-splits") else { return };
    let middle = |h: &Harness<'_, Moonglow>| {
        let dock = &h.state().dock;
        dock.iter_leaves()
            .find(|(_, l)| l.tabs.contains(&Tab::Area(area)))
            .map(|(p, l)| (p.node, l.rect))
            .unwrap()
    };
    let pane = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let dock = &h.state().dock;
        let found = dock.iter_leaves().find(|(_, l)| l.tabs.contains(tab));
        found.map(|(p, l)| (p.surface.is_main(), l.tabs.len(), l.rect)).unwrap()
    };
    // The resource browser, split off below the middle, a third as tall.
    // (The fraction: the upper pane's, the middle's.)
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.run_steps(3);
    {
        let (node, _) = middle(&h);
        let dock = &mut h.state_mut().dock;
        let window = dock.find_tab(&Tab::Resources).unwrap();
        dock.remove_tab(window);
        dock.main_surface_mut().split_below(node, 0.67, vec![Tab::Resources]);
    }
    h.run_steps(3);
    let placed = |h: &Harness<'_, Moonglow>, kind: &str| {
        let places = &h.state().settings.window_places;
        places.iter().find(|(k, _)| k == kind).map(|(_, p)| p.clone())
    };
    assert_eq!(placed(&h, "resources").as_deref(), Some("split below 330"));
    let was = pane(&h, &Tab::Resources).2;
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Resources]));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.run_steps(3);
    let (main, tabs, now) = pane(&h, &Tab::Resources);
    assert!(main && tabs == 1, "a pane of its own in the main window");
    let (_, area_rect) = middle(&h);
    assert!(now.top() >= area_rect.bottom() - 2.0, "below the middle: {now:?} {area_rect:?}");
    assert!((now.height() - was.height()).abs() < 4.0, "as tall: {was:?} {now:?}");

    // The palettes, split off left of the middle, then folded and shown.
    {
        let (node, _) = middle(&h);
        let dock = &mut h.state_mut().dock;
        let palettes = dock.find_tab(&Tab::Palette).unwrap();
        dock.remove_tab(palettes);
        // (Its node, as the dock is now, the palettes' pane gone.)
        let node = dock.find_tab(&Tab::Area(area)).map(|p| p.node).unwrap_or(node);
        // (The fraction: the left pane's, the palettes'.)
        dock.main_surface_mut().split_left(node, 0.25, vec![Tab::Palette]);
    }
    h.run_steps(3);
    assert_eq!(placed(&h, "palette").as_deref(), Some("split left 250"));
    h.state_mut().actions.push(mg_ui::Action::FoldPalettes);
    h.run_steps(3);
    // (Their strip, at the left as they were.)
    let strip = h.get_by_label("Show Palettes").rect();
    assert!(strip.right() < middle(&h).1.left(), "left of the middle: {strip:?}");
    h.get_by_label("Show Palettes").click();
    h.run_steps(3);
    let (_, _, palettes) = pane(&h, &Tab::Palette);
    let (_, area_rect) = middle(&h);
    assert!(
        palettes.right() <= area_rect.left() + 2.0,
        "left of the middle: {palettes:?} {area_rect:?}"
    );
    let share = palettes.width() / (palettes.width() + area_rect.width());
    assert!((share - 0.25).abs() < 0.03, "a quarter of the two: {share}");
}

/// GitHub issue 14: a tool window opens where one of its kind was last
/// dragged to (a tab of the main pane, here), not always in a window of
/// its own; View › Reset Layout forgets that.
#[test]
fn a_tool_window_opens_where_its_kind_was_last_docked() {
    let dir = mg_testkit::scratch_dir("ui-window-places");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let open = |h: &mut Harness<'_, Moonglow>, tab: Tab| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run();
        h.run();
    };
    let close = |h: &mut Harness<'_, Moonglow>, tab: Tab| {
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab]));
        h.run();
        h.run();
    };
    let in_main = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        h.state().dock.find_tab(tab).map(|p| p.surface.is_main())
    };
    // At first: a window of its own, over the area.
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(false));
    // Dragged into the main pane (done here on the dock itself).
    let dock = &mut h.state_mut().dock;
    let from = dock.find_tab(&Tab::Factions).unwrap();
    dock.remove_tab(from);
    dock.main_surface_mut().push_to_first_leaf(Tab::Factions);
    h.run();
    h.run();
    assert_eq!(h.state().settings.window_places, [("factions".to_string(), "middle".to_string())]);
    // Closed and opened again: there, and another kind still in a window.
    close(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), None);
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(true));
    open(&mut h, Tab::Journal);
    assert_eq!(in_main(&h, &Tab::Journal), Some(false));
    // Opened again while open, it stays in the main pane, in front.
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(true));
    // Reset Layout: a window of its own again.
    close(&mut h, Tab::Factions);
    h.get_by_label("View").click();
    h.run();
    h.get_by_label("Reset Layout").click();
    h.run();
    assert!(h.state().settings.window_places.is_empty());
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(false));
}

/// The log's lines carry the time they were said, before them, as the
/// Compiler tab's messages do (Jasperre's report); copied from the pane
/// (its lines are selectable), with it.
#[test]
fn the_log_s_lines_carry_their_time() {
    let mut app = app_with(Vec::new());
    app.log.info("First thing said");
    app.log.warn("Second thing said");
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let is_clock = |at: &str| {
        let b = at.as_bytes();
        b.len() == 8
            && b[2] == b':'
            && b[5] == b':'
            && b.iter().filter(|c| c.is_ascii_digit()).count() == 6
    };
    let log = &h.state().log;
    assert_eq!(log.times.len(), log.entries.len());
    for (i, at) in log.times.iter().enumerate() {
        assert!(is_clock(at), "HH:MM:SS: {at:?}");
        assert_eq!(log.shown(i), format!("{at} {}", log.entries[i].1));
    }
    let line = h.get_by_label_contains("Second thing said");
    let shown = line.accesskit_node().value().unwrap_or_default().to_string();
    let (at, rest) = shown.split_once(' ').unwrap();
    assert!(is_clock(at), "{shown:?}");
    assert_eq!(rest, "Second thing said");
    // Cleared, the times go with the lines.
    h.get_by_label("Clear Log").click();
    h.run();
    assert!(h.state().log.times.is_empty());
}

/// The whole log copied (its corner's Copy, View › Copy Log) and saved
/// into a text file (Save…, View › Save Log…), each line as it shows,
/// its time first (Jasperre's report: a line was copied by selecting it,
/// one at a time).
#[test]
fn the_whole_log_is_copied_and_saved() {
    let dir = mg_testkit::scratch_dir("ui-log-saved");
    let file = dir.join("said.txt");
    let dialogs = NoDialogs { save: vec![file.clone()], ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.log.clear();
    app.log.info("First thing said");
    app.log.error("Second thing said");
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let whole = h.state().log.text();
    let log = &h.state().log;
    assert_eq!(whole, format!("{}\n{}\n", log.shown(0), log.shown(1)));
    assert!(whole.contains(" First thing said\n") && whole.starts_with(&log.times[0]));
    // Copy: the clipboard has it all.
    h.get_by_label("Copy Log").click();
    h.step();
    let copied: Vec<String> = (h.output().platform_output.commands.iter())
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(copied, std::slice::from_ref(&whole));
    h.run();
    // Save…: into the file asked for, and said so (after what was saved).
    h.get_by_label("Save Log").click();
    h.run();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), whole);
    let said = &h.state().log.entries.last().unwrap().1;
    assert!(said.starts_with("Saved the log to"), "{said}");
    // View has them too.
    h.get_by_label("View").click();
    h.run();
    assert_eq!(h.get_all_by_label("Copy Log").count(), 2);
    assert_eq!(h.query_all_by_label("Save Log…").count(), 1);
}

#[test]
fn views_draw_nothing_over_anything_else() {
    // The editors, windows and wizards outside the blueprint editors, roomy
    // and narrow, on a module with a conversation.
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-overlaps");
    let path = sample_module(&dir);
    let talk = ResKey::parse("mg_talk", ResType::DLG).unwrap();
    let mut g = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "Hello there");
    let mut m = Module::open(&path).unwrap();
    m.set(talk, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    let mut found: Vec<String> = Vec::new();
    let mut look = |h: &mut Harness<'_, Moonglow>, what: &str| {
        h.run();
        let width = h.ctx.content_rect().width();
        found.extend(overlapping(h).into_iter().map(|o| format!("{what} at {width}: {o}")));
    };
    let button = |h: &Harness<'_, Moonglow>, label: &str| {
        use egui_kittest::kittest::NodeT;
        h.query_all_by_label(label)
            .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
            .map(|n| n.click())
            .is_some()
    };
    for size in [
        egui::vec2(1280.0, 800.0),
        egui::vec2(1000.0, 800.0),
        egui::vec2(800.0, 700.0),
        egui::vec2(600.0, 700.0),
    ] {
        h.set_size(size);
        use mg_ui::module_props::Page;
        for page in
            [Page::Basic, Page::Events, Page::Advanced, Page::Description, Page::CustomContent]
        {
            h.state_mut().module_page = page;
            h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
            look(&mut h, &format!("Module Properties {page:?}"));
        }
        for (tab, what) in [
            (Tab::Factions, "Factions"),
            (Tab::Journal, "Journal"),
            (Tab::Dialog(talk), "Conversation"),
            (Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap()), "Script"),
            (Tab::Resources, "Resources"),
            (Tab::Palette, "Palettes"),
            (Tab::Manual, "Manual"),
        ] {
            h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
            look(&mut h, what);
            if std::env::var("DEBUG_DOCK").is_ok() && size.x == 800.0 && what == "Script" {
                for (path, leaf) in h.state().dock.iter_leaves() {
                    println!(
                        "leaf {path:?} rect {:?} tabs {:?} active {:?}",
                        leaf.rect,
                        leaf.tabs.len(),
                        leaf.active
                    );
                }
            }
        }
        // Windows.
        let settings = h.state().settings.clone();
        h.state_mut().options = Some(mg_ui::OptionsDraft::from_settings(&settings));
        look(&mut h, "Options Folders");
        for page in
            ["Area", "General", "Script Editor", "Conversation Editor", "Sounds", "Language"]
        {
            assert!(button(&h, page), "{page}");
            look(&mut h, &format!("Options {page}"));
        }
        h.state_mut().options = None;
        h.state_mut().about = true;
        look(&mut h, "About");
        h.state_mut().about = false;
        h.state_mut().build = Some(Default::default());
        look(&mut h, "Build Module");
        h.state_mut().build = None;
        h.state_mut().find_instance = Some(Default::default());
        look(&mut h, "Find Instance");
        h.state_mut().find_instance = None;
        let hello = ResKey::parse("hello", ResType::NSS).unwrap();
        h.state_mut().actions.push(mg_ui::Action::FindReferences(hello));
        look(&mut h, "References");
        h.state_mut().actions.push(mg_ui::Action::RenameDialog(hello));
        look(&mut h, "Rename");
        h.state_mut().rename = None;
        let clip = mg_ui::area_view::ObjectClip { objects: Vec::new(), anchor: glam::Vec3::ZERO };
        h.state_mut().prefab_save = Some(("Gate".into(), clip));
        look(&mut h, "Save as Prefab");
        h.state_mut().prefab_save = None;
        h.state_mut().actions.push(mg_ui::Action::AreaWizard);
        look(&mut h, "Area Wizard");
        h.state_mut().wizard = None;
        // Wizards, page by page while Next leads on.
        for kind in mg_ui::blueprint_wizard::KINDS {
            h.state_mut().blueprint_wizard =
                Some(mg_ui::blueprint_wizard::BlueprintWizard::new(kind));
            for step in 0..8 {
                look(&mut h, &format!("{kind:?} Wizard, page {step}"));
                if !button(&h, "Next >") {
                    break;
                }
                h.run();
            }
            h.state_mut().blueprint_wizard = None;
        }
        h.state_mut().creature_wizard = Some(Default::default());
        h.run();
        for step in 0..8 {
            if step == 1 {
                assert!(button(&h, "Human"));
                h.run();
            }
            look(&mut h, &format!("Creature Wizard, page {step}"));
            if !button(&h, "Next >") {
                break;
            }
            h.run();
        }
        h.state_mut().creature_wizard = None;
    }
    assert!(found.is_empty(), "drawn over each other:\n{}", found.join("\n"));
}

#[test]
fn tabs_open_beside_the_palette_not_in_its_pane() {
    let dir = mg_testkit::scratch_dir("ui-tab-panes");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    let script = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    for tab in [Tab::Palette, Tab::Manual, script.clone(), Tab::Factions] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run();
    }
    let dock = &h.state().dock;
    let pane = |t: &Tab| dock.find_tab(t).map(|p| (p.surface, p.node));
    let palette = pane(&Tab::Palette).unwrap();
    for t in [Tab::Manual, script, Tab::Factions] {
        assert_ne!(pane(&t), Some(palette), "{t:?} opened in the palette's pane");
    }
}

#[test]
fn the_log_shrinks_to_one_line() {
    let mut h = harness(app_with(Vec::new()), 1000.0, 700.0);
    h.run();
    let line = h.ctx.global_style().text_styles[&egui::TextStyle::Monospace].size;
    let entry = |h: &Harness<'_, Moonglow>| h.get_by_label_contains("No Neverwinter Nights").rect();
    let status = h.get_by_label("No module").rect();
    // The panel's top edge, just above its first line: drag it far down.
    let top = entry(&h).top() - 3.0;
    let x = 500.0;
    h.event(egui::Event::PointerMoved(egui::pos2(x, top)));
    h.run();
    let button = egui::PointerButton::Primary;
    let modifiers = egui::Modifiers::NONE;
    h.event(egui::Event::PointerButton {
        pos: egui::pos2(x, top),
        button,
        pressed: true,
        modifiers,
    });
    h.run();
    for y in [top + 40.0, top + 120.0, status.top() - 2.0] {
        h.event(egui::Event::PointerMoved(egui::pos2(x, y)));
        h.run();
    }
    let end = egui::pos2(x, status.top() - 2.0);
    h.event(egui::Event::PointerButton { pos: end, button, pressed: false, modifiers });
    h.run();
    // About a line between the log's first line and the status bar (three
    // lines' room was the least it would take).
    let room = status.top() - entry(&h).top();
    assert!(room < 2.0 * line + 12.0, "the log keeps {room} points (a line is {line})");
}

/// A window of tabs moves by its tab bar, not by a drag on a part of its
/// body that isn't a control (Taro49's report: a click beside a control
/// moved the conversation editor); Options › General brings that back.
#[test]
fn a_window_of_tabs_moves_by_its_tab_bar_alone() {
    let dir = mg_testkit::scratch_dir("ui-window-drag");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    h.run_steps(3);
    let window = |h: &Harness<'_, Moonglow>| {
        let (_, leaf) = h.state().dock.iter_leaves().find(|(p, _)| !p.surface.is_main()).unwrap();
        leaf.rect
    };
    let drag = |h: &mut Harness<'_, Moonglow>, from: egui::Pos2| {
        let button = egui::PointerButton::Primary;
        let modifiers = egui::Modifiers::NONE;
        h.hover_at(from);
        h.run_steps(2);
        h.event(egui::Event::PointerButton { pos: from, button, pressed: true, modifiers });
        h.run_steps(2);
        for k in 1..=4 {
            h.hover_at(from + egui::vec2(15.0, 10.0) * k as f32);
            h.run_steps(2);
        }
        let to = from + egui::vec2(60.0, 40.0);
        h.event(egui::Event::PointerButton { pos: to, button, pressed: false, modifiers });
        h.run_steps(3);
        h.hover_at(egui::pos2(5.0, 780.0));
        h.run_steps(3);
    };
    // A spot of the body with nothing on it: below the relations' page,
    // left of the factions' list (seen in a picture).
    let before = window(&h);
    let spot = |r: egui::Rect| egui::pos2(r.left() + 200.0, r.center().y + 40.0);
    drag(&mut h, spot(before));
    assert_eq!(window(&h), before, "not moved by its body");
    // By the tab bar's room beside the tabs, it moves.
    drag(&mut h, before.min + egui::vec2(400.0, 12.0));
    let moved = window(&h);
    assert!(moved.min.x > before.min.x + 30.0, "moved by its tab bar: {before:?} {moved:?}");
    let before = moved;
    h.state_mut().settings.drag_windows_anywhere = true;
    h.run();
    drag(&mut h, spot(before));
    let after = window(&h);
    // (Down: the screen's right edge stops it going right.)
    assert!(
        after.min.y > before.min.y + 20.0,
        "moved by its body when asked: {before:?} {after:?}"
    );
}

#[test]
fn editors_open_in_windows_over_the_area() {
    let Some((mut h, area)) = area_harness("editor-windows") else { return };
    h.run_steps(3);
    for tab in [Tab::Factions, Tab::ModuleProperties] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
        h.run_steps(3);
        let dock = &h.state().dock;
        let path = dock.find_tab(&tab).expect("open");
        assert!(!path.surface.is_main(), "{tab:?} in a window of its own");
        // The area still shows in the main pane.
        let area_path = dock.find_tab(&Tab::Area(area)).unwrap();
        assert!(area_path.surface.is_main());
        let (_, leaf) = dock
            .iter_leaves()
            .find(|(p, _)| p.node == area_path.node && p.surface.is_main())
            .unwrap();
        assert_eq!(leaf.tabs[leaf.active.0], Tab::Area(area), "the area stays in front");
    }
    // Below the toolbar, never over it.
    let toolbar = h.get_by_label_contains("Verify").rect().bottom();
    let top = h.get_all_by_label("Factions").map(|n| n.rect().top()).fold(f32::MAX, f32::min);
    assert!(top > toolbar, "the window starts below the toolbar ({top} vs {toolbar})");
    let img = h.render().expect("render");
    let _ = img.save(mg_testkit::scratch_dir("ui-editor-windows").join("windows.png"));
}

/// A module opens with the area that was opened last in it.
#[test]
fn a_module_opens_on_the_area_opened_last_in_it() {
    let Some((mut h, area)) = area_harness("last-area") else { return };
    let path = h.state().module_path().expect("saved");
    assert_eq!(h.state().settings.last_area(&path), Some("field"));
    h.state_mut().open_module(&path);
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_none(), "opened anew");
    h.run_steps(3);
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_some(), "the area is open again");
    // Without one remembered (a module opened for the first time, or its
    // area gone), the first the tree lists, remembered from then on.
    for forgotten in [Some("gone"), None] {
        match forgotten {
            Some(gone) => h.state_mut().settings.last_areas[0].1 = gone.into(),
            None => h.state_mut().settings.last_areas.clear(),
        }
        h.state_mut().open_module(&path);
        h.run_steps(3);
        assert!(h.state().dock.find_tab(&Tab::Area(area)).is_some(), "the first area");
        assert_eq!(h.state().settings.last_area(&path), Some("field"));
    }
}

/// [`sample_module`] with a second script, and an app that keeps the open
/// tabs in a scratch folder (never Moonglow's own): the module's path, the
/// app and that folder.
fn tabs_app(name: &str) -> (std::path::PathBuf, Moonglow, std::path::PathBuf) {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    m.set(ResKey::parse("bye", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_palette = false;
    app.open_tabs_dir = Some(dir.join("open-tabs"));
    (path, app, dir.join("open-tabs"))
}

/// Opens the sample's area, its two scripts (together in one window, the
/// first in front) and the Factions; Module Properties is docked in the
/// middle as the module opened.
fn open_some_tabs(h: &mut Harness<'_, Moonglow>) {
    let script = |n| Tab::Script(ResKey::parse(n, ResType::NSS).unwrap());
    // (Steps: the module tree opens a tab's group over a few frames.)
    for tab in [Tab::Area(ResRef::from_str("start").unwrap()), Tab::Factions, script("hello")] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run_steps(8);
    }
    h.state_mut().actions.push(mg_ui::Action::OpenTab(script("bye")));
    h.run_steps(8);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(script("hello")));
    h.run_steps(8);
}

/// Each pane's tabs (a pane of the main window or not) and the one in
/// front, Module Properties' pane first.
fn panes_shown(app: &Moonglow) -> Vec<(bool, Vec<Tab>, Tab)> {
    let mut panes: Vec<(bool, Vec<Tab>, Tab)> = (app.dock.iter_leaves())
        .filter(|(_, leaf)| !leaf.tabs.is_empty())
        .map(|(p, leaf)| (p.surface.is_main(), leaf.tabs.clone(), leaf.tabs[leaf.active.0].clone()))
        .collect();
    panes.sort_by_key(|(_, tabs, _)| !tabs.contains(&Tab::ModuleProperties));
    panes
}

/// A module opened again has the tabs it had when it was closed: the
/// same, in the same order and places (the middle, a window of the
/// scripts), each pane's in front in front; kept in Moonglow's data folder
/// (a scratch one here), not in the module.
#[test]
fn a_module_opens_again_with_the_tabs_it_had() {
    let (path, app, kept) = tabs_app("ui-open-tabs");
    let resources = Module::open(&path).unwrap().len();
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().open_module(&path);
    h.run();
    open_some_tabs(&mut h);
    let before = panes_shown(h.state());
    let hello = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    let bye = Tab::Script(ResKey::parse("bye", ResType::NSS).unwrap());
    assert!(before.iter().any(|(main, tabs, front)| {
        !main && *tabs == [hello.clone(), bye.clone()] && *front == hello
    }));
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    assert!(h.state().dock.find_tab(&hello).is_none(), "closed");
    let file = mg_ui::open_tabs::file(&kept, &path);
    assert!(file.is_file(), "kept in {}", file.display());
    // (Not in the module.)
    assert_eq!(Module::open(&path).unwrap().len(), resources);
    h.state_mut().open_module(&path);
    h.run();
    assert_eq!(panes_shown(h.state()), before);
    // The tab that had the keys has them again.
    assert_eq!(
        h.state().dock.focused_leaf().map(|p| h.state().dock.leaf(p).unwrap().tabs.clone()),
        Some(vec![hello, bye])
    );
}

/// With Options › General's Open a module with the tabs it had off, a
/// module's tabs are neither kept nor opened again.
#[test]
fn a_module_s_tabs_are_not_opened_again_with_the_option_off() {
    let (path, app, kept) = tabs_app("ui-open-tabs-off");
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    let hello = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    let file = mg_ui::open_tabs::file(&kept, &path);
    for keep in [true, false] {
        h.state_mut().settings.no_reopen_tabs = !keep;
        let _ = std::fs::remove_file(&file);
        h.state_mut().open_module(&path);
        h.run();
        assert!(h.state().dock.find_tab(&hello).is_none(), "nothing kept yet");
        open_some_tabs(&mut h);
        h.state_mut().actions.push(mg_ui::Action::Close);
        h.run();
        assert_eq!(file.is_file(), keep, "kept with the option on only");
        // (Off, a record kept before is not opened either.)
        if keep {
            h.state_mut().settings.no_reopen_tabs = true;
            h.state_mut().open_module(&path);
            h.run();
            assert!(h.state().dock.find_tab(&hello).is_none(), "not opened again");
            h.state_mut().close();
        }
    }
}

/// A kept tab whose resource is gone from the module is passed over; the
/// others open again.
#[test]
fn a_kept_tab_whose_resource_is_gone_is_passed_over() {
    let (path, app, _) = tabs_app("ui-open-tabs-gone");
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().open_module(&path);
    h.run();
    open_some_tabs(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let bye = ResKey::parse("bye", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.remove(&hello);
    m.save().unwrap();
    // (Opened as File › Recent Modules opens it, within a frame.)
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
    h.run();
    let app = h.state();
    assert!(app.dock.find_tab(&Tab::Script(hello)).is_none(), "gone");
    assert!(app.dock.find_tab(&Tab::Script(bye)).is_some());
    assert!(app.dock.find_tab(&Tab::Factions).is_some());
    assert!(app.dock.find_tab(&Tab::Area(ResRef::from_str("start").unwrap())).is_some());
}

/// A tab docked in the palettes' pane opens there again, the module opened
/// again from a menu (within a frame, the pane made at the next).
#[test]
fn a_tab_kept_in_the_palettes_pane_opens_there_again() {
    let Some((mut h, area)) = area_harness("open-tabs-palettes") else { return };
    let path = h.state().module_path().expect("saved");
    let kept = mg_testkit::scratch_dir("ui-open-tabs-palettes");
    h.state_mut().open_tabs_dir = Some(kept.clone());
    let palette_pane = |app: &Moonglow| {
        let at = app.dock.find_tab(&Tab::Palette).expect("the palettes are shown");
        app.dock
            .leaf(egui_dock::NodePath { surface: at.surface, node: at.node })
            .unwrap()
            .tabs
            .clone()
    };
    {
        let app = h.state_mut();
        let at = app.dock.find_tab(&Tab::Palette).unwrap();
        app.dock.set_focused_node_and_surface(egui_dock::NodePath {
            surface: at.surface,
            node: at.node,
        });
        app.dock.push_to_focused_leaf(Tab::Resources);
    }
    h.run_steps(3);
    assert_eq!(palette_pane(h.state()), [Tab::Palette, Tab::Resources]);
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path));
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_some());
    assert_eq!(palette_pane(h.state()), [Tab::Palette, Tab::Resources], "in its pane again");
}

/// A tab's kind: its variant's name.
fn tab_kind(tab: &Tab) -> String {
    let text = format!("{tab:?}");
    text.split(['(', ' ', '{']).next().unwrap_or_default().to_string()
}

/// A new session's app (Moonglow started again) that keeps the open tabs
/// in `kept`, with the module at `path` opened in its first frame.
fn tabs_session(path: &std::path::Path, kept: &std::path::Path) -> Harness<'static, Moonglow> {
    let mut app = app_with(Vec::new());
    app.open_palette = false;
    app.open_tabs_dir = Some(kept.to_path_buf());
    let mut h = harness(app, 1400.0, 900.0);
    h.state_mut().open_module(path);
    h.run_steps(3);
    h
}

/// The side editors (a hak's, a 2DA's, a tileset's: by their files), the
/// Options (on their page), References (its search made again) and the
/// Placeable Gallery open again with the module, in the next session; a
/// side editor's whose file is gone is passed over.
#[test]
fn side_editors_and_tools_open_again_with_the_module() {
    use mg_module::hak_edit::{Hak, Source};
    use mg_ui::references::Query;
    use mg_ui::table_view::Home;
    let (path, app, kept) = tabs_app("ui-open-tabs-tools");
    let dir = path.parent().unwrap().to_path_buf();
    let hak_path = dir.join("stuff.hak");
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    let mut hak = Hak::new();
    hak.add_sources(vec![(key, Source::Bytes(SMALL_TABLE.to_vec().into()))]).unwrap();
    hak.save(Some(&hak_path)).unwrap();
    let table_path = dir.join("loose.2da");
    std::fs::write(&table_path, SMALL_TABLE).unwrap();
    let set_path = dir.join("zzz01.set");
    std::fs::write(&set_path, mg_set::edit::skeleton("zzz01").text()).unwrap();
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut h = harness(app, 1400.0, 900.0);
    h.state_mut().open_module(&path);
    h.run_steps(2);
    {
        let app = h.state_mut();
        let hak = mg_ui::hak_view::open_at(app, &hak_path).unwrap();
        mg_ui::table_view::open_in_hak(app, hak, key).unwrap();
        mg_ui::table_view::open_path(app, &table_path).unwrap();
        mg_ui::tileset_view::open_path(app, set_path.clone());
        app.find_references(Query::Resource(hello));
        app.actions.push(mg_ui::Action::OpenTab(Tab::PlaceableGallery));
        app.actions.push(mg_ui::Action::OptionsDialog);
    }
    h.run_steps(3);
    h.state_mut().options.as_mut().unwrap().page = mg_ui::OptionsPage::General;
    h.run_steps(2);
    let shown = |app: &Moonglow| -> Vec<String> {
        let mut kinds: Vec<String> = app.dock.iter_all_tabs().map(|(_, t)| tab_kind(t)).collect();
        kinds.sort();
        kinds
    };
    let before = shown(h.state());
    for kind in ["Hak", "Table", "Tileset", "References", "PlaceableGallery", "Options"] {
        assert!(before.iter().any(|k| k == kind), "{kind} open: {before:?}");
    }
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    // The next session: everything but the tileset, whose file is gone.
    std::fs::remove_file(&set_path).unwrap();
    let h = tabs_session(&path, &kept);
    let app = h.state();
    let hak = app.haks.iter().find(|d| d.hak.path.as_deref() == Some(&hak_path));
    let hak = hak.expect("the hak open again");
    assert!(app.dock.find_tab(&Tab::Hak(hak.id)).is_some());
    for home in [Home::Hak(hak.id), Home::File(table_path.clone())] {
        let table = app.tables.iter().find(|d| d.home == home).expect("the 2DA open again");
        assert!(app.dock.find_tab(&Tab::Table(table.id)).is_some(), "{home:?}");
    }
    assert!(app.tilesets.is_empty(), "the tileset's file is gone");
    assert_eq!(app.references.query, Some(Query::Resource(hello)));
    assert!(app.dock.find_tab(&Tab::References).is_some());
    assert!(app.dock.find_tab(&Tab::PlaceableGallery).is_some());
    assert!(app.dock.find_tab(&Tab::Options).is_some());
    assert_eq!(app.options.as_ref().unwrap().page, mg_ui::OptionsPage::General);
    let after = shown(app);
    assert_eq!(after.iter().filter(|k| *k == "Tileset").count(), 0);
    assert_eq!(after.len(), before.len() - 1, "{before:?} {after:?}");
}

/// A sample module with a long script and a conversation: an NPC's line
/// with two replies, the first of them with an NPC's line under it.
fn views_module(name: &str) -> (std::path::PathBuf, std::path::PathBuf, ResKey, ResKey) {
    use mg_module::dialog::{Parent, add_node, new_dialog};
    let (path, _, kept) = tabs_app(name);
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    let talk = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    let yes = add_node(&mut g, Parent::Node(mg_module::dialog::Kind::Entry, hello), "Yes.");
    add_node(&mut g, Parent::Node(mg_module::dialog::Kind::Entry, hello), "No.");
    add_node(&mut g, Parent::Node(mg_module::dialog::Kind::Reply, yes), "Good.");
    let mut m = Module::open(&path).unwrap();
    m.set_gff(talk, &g).unwrap();
    let text: String = (0..200).map(|i| format!("// line {i}\n")).collect();
    m.set(script, format!("{text}void main() {{ }}\n").into_bytes());
    m.save().unwrap();
    (path, kept, script, talk)
}

/// Each view opens again as it was: a script scrolled where it was with
/// its cursor there, a conversation with its selected line, bookmarks
/// and folded branches, Module Properties on its page.
#[test]
fn views_open_again_as_they_were() {
    use mg_module::dialog::{Kind, Parent};
    use mg_ui::dialog_view::Row;
    let (path, kept, script, talk) = views_module("ui-open-tabs-views");
    let mut h = tabs_session(&path, &kept);
    for tab in [Tab::Script(script), Tab::Dialog(talk)] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run_steps(2);
    }
    // The cursor on line 150, scrolled to it.
    let at = "// line 0\n".len() * 10 + "// line 10\n".len() * 90 + "// line 100\n".len() * 50;
    h.state_mut().script_tools.jump = Some((script, at));
    h.run_steps(3);
    let selected = Row { parent: Parent::Node(Kind::Entry, 0), pos: 1 };
    {
        let app = h.state_mut();
        app.module_page = mg_ui::module_props::Page::Events;
        let view = app.dialog_views.get_mut(&talk).unwrap();
        view.selected = Some(selected);
        view.bookmarks = vec![(Kind::Reply, 1), (Kind::Entry, 0)];
        view.collapsed.insert((Kind::Reply, 0));
    }
    h.run_steps(2);
    let editor = |h: &Harness<'_, Moonglow>| {
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .map(|n| n.rect())
            .max_by(|a, b| a.height().total_cmp(&b.height()))
            .expect("the script's editor")
    };
    let cursor = |h: &Harness<'_, Moonglow>| {
        let id = egui::Id::new(("script", script));
        let state = egui::text_edit::TextEditState::load(&h.ctx, id).unwrap();
        let r = state.cursor.char_range().unwrap();
        (usize::from(r.primary.index), usize::from(r.secondary.index))
    };
    let scrolled = editor(&h).top();
    assert_eq!(cursor(&h), (at, at));
    assert!(scrolled < -1000.0, "scrolled to line 150: {scrolled}");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let mut h = tabs_session(&path, &kept);
    h.run_steps(2);
    assert!((editor(&h).top() - scrolled).abs() < 1.0, "{} {scrolled}", editor(&h).top());
    assert_eq!(cursor(&h), (at, at));
    let app = h.state();
    assert_eq!(app.module_page, mg_ui::module_props::Page::Events);
    let view = &app.dialog_views[&talk];
    assert_eq!(view.selected, Some(selected));
    assert_eq!(view.bookmarks, [(Kind::Reply, 1), (Kind::Entry, 0)]);
    assert_eq!(view.collapsed, [(Kind::Reply, 0)].into_iter().collect());
}

/// A script only scrolled (by the wheel, its editor never clicked: no
/// cursor yet) opens again scrolled there.
#[test]
fn a_script_only_scrolled_opens_again_scrolled() {
    let (path, kept, script, _) = views_module("ui-open-tabs-scrolled");
    let mut h = tabs_session(&path, &kept);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(script)));
    h.run_steps(3);
    let editor = |h: &Harness<'_, Moonglow>| {
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .map(|n| n.rect())
            .max_by(|a, b| a.height().total_cmp(&b.height()))
            .expect("the script's editor")
    };
    let top = editor(&h).top();
    h.hover_at(egui::pos2(editor(&h).center().x, top + 100.0));
    h.run();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -600.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(30);
    let scrolled = editor(&h).top();
    assert!(scrolled < top - 300.0, "scrolled: {top} {scrolled}");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let mut h = tabs_session(&path, &kept);
    h.run_steps(2);
    assert!((editor(&h).top() - scrolled).abs() < 1.0, "{} {scrolled}", editor(&h).top());
}

/// A conversation changed since its tab was kept (outside Moonglow) opens
/// without what was kept by line number: its selection, bookmarks and
/// folded branches would be on other lines now.
#[test]
fn a_conversation_changed_since_opens_without_its_marks() {
    use mg_module::dialog::{Kind, Parent, add_node};
    use mg_ui::dialog_view::Row;
    let (path, kept, _, talk) = views_module("ui-open-tabs-dialog-changed");
    let mut h = tabs_session(&path, &kept);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(talk)));
    h.run_steps(3);
    {
        let view = h.state_mut().dialog_views.get_mut(&talk).unwrap();
        view.selected = Some(Row { parent: Parent::Root, pos: 0 });
        view.bookmarks = vec![(Kind::Entry, 0)];
        view.collapsed.insert((Kind::Reply, 0));
    }
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let mut m = Module::open(&path).unwrap();
    let mut g = m.gff(&talk).unwrap().unwrap();
    add_node(&mut g, Parent::Root, "Another start.");
    m.set_gff(talk, &g).unwrap();
    m.save().unwrap();
    let h = tabs_session(&path, &kept);
    let app = h.state();
    assert!(app.dock.find_tab(&Tab::Dialog(talk)).is_some(), "the tab opens again");
    let view = &app.dialog_views[&talk];
    assert!(view.bookmarks.is_empty() && view.collapsed.is_empty(), "let go");
    assert_eq!(view.selected, None);
}

/// A window split into two panes opens again split as it was, each pane
/// with its tabs.
#[test]
fn a_window_of_several_panes_opens_again_split() {
    let (path, app, kept) = tabs_app("ui-open-tabs-split");
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().open_module(&path);
    h.run();
    open_some_tabs(&mut h);
    let bye = Tab::Script(ResKey::parse("bye", ResType::NSS).unwrap());
    let hello = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    // The scripts' window: hello above bye, and the Factions (from their
    // own window) beside hello; egui_dock lists bye's pane first.
    {
        let dock = &mut h.state_mut().dock;
        let at = dock.find_tab(&bye).unwrap();
        let tab = dock.remove_tab(at).unwrap();
        let pane = egui_dock::NodePath { surface: at.surface, node: at.node };
        dock.split(pane, egui_dock::Split::Below, 0.4, egui_dock::Node::leaf(tab));
        let factions = dock.remove_tab(dock.find_tab(&Tab::Factions).unwrap()).unwrap();
        let at = dock.find_tab(&hello).unwrap();
        let pane = egui_dock::NodePath { surface: at.surface, node: at.node };
        dock.split(pane, egui_dock::Split::Right, 0.3, egui_dock::Node::leaf(factions));
    }
    h.run_steps(2);
    // The window's panes (their tabs) and its splits' shares, as
    // egui_dock's tree has them.
    let window = |app: &Moonglow| {
        let at = app.dock.find_tab(&hello).unwrap();
        assert!(!at.surface.is_main());
        let tree = app.dock.get_surface(at.surface).unwrap().node_tree().unwrap();
        let panes: Vec<Vec<Tab>> = tree
            .iter()
            .filter_map(|n| n.tabs().map(|t| t.to_vec()))
            .filter(|t| !t.is_empty())
            .collect();
        let shares: Vec<f32> = tree
            .iter()
            .filter_map(|n| match n {
                egui_dock::Node::Vertical(s) | egui_dock::Node::Horizontal(s) => Some(s.fraction),
                _ => None,
            })
            .collect();
        (panes, shares)
    };
    let near = |a: &[f32], b: &[f32]| {
        a.len() == b.len() && a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.01)
    };
    let (panes, shares) = window(h.state());
    let laid = [vec![bye.clone()], vec![hello.clone()], vec![Tab::Factions]];
    assert_eq!(panes, laid);
    // (The second as the window's room let it be.)
    assert!(shares.len() == 2 && (shares[0] - 0.4).abs() < 0.01, "{shares:?}");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let h = tabs_session(&path, &kept);
    let (panes, again) = window(h.state());
    assert_eq!(panes, laid, "split again");
    assert!(near(&again, &shares), "{again:?} {shares:?}");
}

/// Where a window is moved to is kept a moment after it stops, without
/// waiting for the module to close (a crash keeps it).
#[test]
fn a_window_moved_is_kept_at_once() {
    let (path, app, kept) = tabs_app("ui-open-tabs-moved");
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().open_module(&path);
    h.run();
    open_some_tabs(&mut h);
    h.run_steps(8);
    let file = mg_ui::open_tabs::file(&kept, &path);
    let rect = || -> [f32; 4] {
        let record: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        let panes = record["panes"].as_array().unwrap();
        let pane = panes.iter().find(|p| p["tabs"][0] == "script:hello.nss").unwrap();
        serde_json::from_value(pane["rect"].clone()).unwrap()
    };
    let was = rect();
    // The scripts' window dragged by its tab bar's room beside the tabs.
    let hello = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    let (_, leaf) = h.state().dock.iter_leaves().find(|(_, l)| l.tabs.contains(&hello)).unwrap();
    let from = egui::pos2(leaf.rect.right() - 40.0, leaf.rect.top() + 12.0);
    let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
    h.hover_at(from);
    h.step();
    h.event(egui::Event::PointerButton { pos: from, button, pressed: true, modifiers });
    h.step();
    for k in 1..=4 {
        h.hover_at(from - egui::vec2(20.0, -10.0) * k as f32);
        h.step();
    }
    let to = from + egui::vec2(-80.0, 40.0);
    h.event(egui::Event::PointerButton { pos: to, button, pressed: false, modifiers });
    h.step();
    assert_eq!(rect(), was, "not at once: when it stops");
    h.run_steps(6);
    let now = rect();
    let moved = [now[0] - was[0], now[1] - was[1]];
    assert!((moved[0] + 80.0).abs() < 2.0 && (moved[1] - 40.0).abs() < 2.0, "{was:?} {now:?}");
}

/// [`sample_module`]'s area with two creatures (tags A and B) placed.
fn placed_module(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let (path, _, kept) = tabs_app(name);
    let mut m = Module::open(&path).unwrap();
    set_creatures(&mut m, &[("A", 10.0), ("B", 20.0)]);
    m.save().unwrap();
    (path, kept)
}

/// The area's creatures, by tag and where each stands.
fn set_creatures(m: &mut Module, creatures: &[(&str, f32)]) {
    let list = creatures
        .iter()
        .map(|(tag, x)| {
            let mut s = mg_gff::Struct::new(4);
            s.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
            s.set("TemplateResRef", mg_gff::Value::resref(ResRef::from_str("nw_bandit").unwrap()));
            for (label, v) in [("XPosition", *x), ("YPosition", 5.0), ("ZPosition", 0.0)] {
                s.set(label, mg_gff::Value::Float(v));
            }
            s
        })
        .collect();
    let mut git = Gff::new(*b"GIT ");
    git.root.set("Creature List", mg_gff::Value::List(list));
    m.set_gff(ResKey::parse("start", ResType::GIT).unwrap(), &git).unwrap();
}

/// A placed object's Properties are found again by the object (its tag,
/// blueprint and place), not by its place in the area's list: an area
/// changed outside Moonglow since gives the same object's, or none.
#[test]
fn a_placed_object_s_properties_find_it_again() {
    let (path, kept) = placed_module("ui-open-tabs-placed");
    let area = ResRef::from_str("start").unwrap();
    let at = |i| Tab::Instance { area, path: mg_edit::GffPath::root().item("Creature List", i) };
    let mut h = tabs_session(&path, &kept);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(at(1)));
    h.run_steps(2);
    assert!(h.state().dock.find_tab(&at(1)).is_some());
    // On its Scripts page.
    let placed_in = ResKey::new(area, ResType::GIT);
    let page = |i| (placed_in, mg_edit::GffPath::root().item("Creature List", i));
    h.state_mut().blueprint_pages.insert(page(1), "Scripts");
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    // Another creature put before it: B is the third now.
    let mut m = Module::open(&path).unwrap();
    set_creatures(&mut m, &[("C", 5.0), ("A", 10.0), ("B", 20.0)]);
    m.save().unwrap();
    let mut h = tabs_session(&path, &kept);
    assert!(h.state().dock.find_tab(&at(1)).is_none(), "not A's");
    assert!(h.state().dock.find_tab(&at(2)).is_some(), "B's");
    assert_eq!(h.state().blueprint_pages.get(&page(2)), Some(&"Scripts"), "on its page");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    // B gone: no creature's.
    let mut m = Module::open(&path).unwrap();
    set_creatures(&mut m, &[("C", 5.0), ("A", 10.0)]);
    m.save().unwrap();
    let h = tabs_session(&path, &kept);
    let instances =
        h.state().dock.iter_all_tabs().filter(|(_, t)| tab_kind(t) == "Instance").count();
    assert_eq!(instances, 0);
}

/// A module opened from its recovery copy (Recover Unsaved Work, after a
/// crash) opens with the tabs it had.
#[test]
fn a_recovered_module_opens_with_its_tabs() {
    let (path, app, kept) = tabs_app("ui-open-tabs-recovered");
    let recovery = path.parent().unwrap().join("recovery");
    let hello = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    {
        let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.state_mut().recovery_dir = Some(recovery.clone());
        h.state_mut().open_module(&path);
        h.run();
        open_some_tabs(&mut h);
        let edit = mg_edit::Edit::SetField {
            key: ResKey::parse("module", ResType::IFO).unwrap(),
            path: mg_edit::GffPath::root(),
            label: "Mod_Tag".into(),
            value: Some(mg_gff::Value::String(b"RECOVERED".to_vec())),
        };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("tag", vec![edit])));
        h.run();
        h.state_mut().write_recovery().unwrap();
        // (It "crashes": never closed.)
    }
    let mut app = app_with(Vec::new());
    app.open_palette = false;
    app.open_tabs_dir = Some(kept);
    app.recovery_dir = Some(recovery);
    app.find_recoveries();
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Recover").click();
    h.run_steps(3);
    assert!(h.state().ws.as_ref().unwrap().is_modified(), "recovered");
    assert!(h.state().dock.find_tab(&hello).is_some(), "with its tabs");
    assert!(h.state().dock.find_tab(&Tab::Factions).is_some());
}

/// Escape closes a model's window while the pointer is over it.
#[test]
fn escape_closes_a_models_window() {
    let dir = mg_testkit::scratch_dir("ui-model-escape");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let model = Tab::Model(ResKey::parse("nothing", ResType::UTW).unwrap());
    app.actions.push(mg_ui::Action::OpenTab(model.clone()));
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    assert!(h.state().dock.find_tab(&model).is_some());
    // Away from the window: Escape is someone else's.
    h.hover_at(egui::pos2(2.0, 2.0));
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().dock.find_tab(&model).is_some());
    // Over the window (where it says the blueprint isn't there).
    let inside = h.get_by_label_contains("not found").rect().center();
    h.hover_at(inside);
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().dock.find_tab(&model).is_none(), "closed");
}

/// An editor's window opens as large as one of its kind was last left, and
/// Maximize (its tab's menu, or a double click on the tab) fills the
/// panes' room and puts it back.
#[test]
fn windows_remember_their_size_and_maximize() {
    let Some((mut h, area)) = area_harness("window-sizes") else { return };
    let props = Tab::AreaProperties(area);
    // The room the window's pane takes.
    let pane = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let s = h.state();
        let path = s.dock.find_tab(tab).expect("open");
        assert!(!path.surface.is_main(), "in a window of its own");
        s.dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1.rect
    };
    let open = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
        h.run_steps(8);
    };
    let close = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::CloseTab(props.clone()));
        h.run_steps(3);
    };
    open(&mut h);
    let first = pane(&h, &props);
    // Maximized, and back.
    h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(props.clone()));
    h.run_steps(8);
    let big = pane(&h, &props);
    assert!(big.area() > first.area() + 1000.0, "larger: {big:?} than {first:?}");
    // Over the whole of the window under the toolbar: the module tree and
    // the log too, not the middle alone.
    assert!(big.width() > 1100.0 * 0.95, "the window's width: {big:?}");
    assert!(big.bottom() > 800.0 * 0.95, "to the window's foot: {big:?}");
    h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(props.clone()));
    h.run_steps(8);
    let back = pane(&h, &props);
    assert!((back.size() - first.size()).length() < 2.0, "back to {back:?}, was {first:?}");
    assert!((back.min - first.min).length() < 2.0, "where it was: {back:?}, {first:?}");
    // A double click on the bar beside the tab does the same, twice; and
    // the window is still dragged by the bar.
    let bar = |h: &Harness<'_, Moonglow>| {
        let r = pane(h, &props);
        egui::pos2(r.right() - 120.0, r.top() + 10.0)
    };
    for larger in [true, false] {
        let at = bar(&h);
        h.hover_at(at);
        for _ in 0..2 {
            press(&h, at, true, egui::Modifiers::NONE);
            press(&h, at, false, egui::Modifiers::NONE);
        }
        h.run_steps(8);
        let now = pane(&h, &props);
        assert_eq!(now.area() > first.area() + 1000.0, larger, "double-clicked: {now:?}");
        // (Later: not a third and fourth click of the same.)
        h.hover_at(at + egui::vec2(0.0, 200.0));
        h.run_steps(60);
    }
    let at = bar(&h);
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    for k in 1..=5 {
        h.hover_at(at + egui::vec2(6.0 * k as f32, 4.0 * k as f32));
        h.run_steps(1);
    }
    press(&h, at + egui::vec2(30.0, 20.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    let moved = pane(&h, &props).min - first.min;
    assert!((moved - egui::vec2(30.0, 20.0)).length() < 8.0, "dragged by {moved:?}");
    // Opened again: as it first opened (maximizing isn't remembered).
    close(&mut h);
    open(&mut h);
    let again = pane(&h, &props);
    assert!((again.size() - first.size()).length() < 2.0, "opens at {again:?}, not {first:?}");
    // Left smaller (as dragging its edge leaves it): the next opens so.
    h.state_mut().settings.window_sizes = vec![("area-properties".into(), [400, 300])];
    close(&mut h);
    open(&mut h);
    let small = pane(&h, &props);
    assert!(
        (small.width() - 400.0).abs() < 20.0 && (small.height() - 300.0).abs() < 20.0,
        "{small:?} against {first:?}"
    );
    // And once more: the size doesn't creep.
    close(&mut h);
    open(&mut h);
    let third = pane(&h, &props);
    assert!((third.size() - small.size()).length() < 2.0, "then at {third:?}");
    assert_eq!(h.state().settings.window_sizes, [("area-properties".to_string(), [400, 300])]);
}

/// Maximize Window (Ctrl+Shift+M) and the button on the window's bar
/// maximize the window of the tab in front and put it back, the button
/// saying which (Maximize, Restore); a maximized window stays so as
/// Moonglow's window is resized; and a size dragged by hand, at the
/// window's corner, is the next window's of its kind.
#[test]
fn a_window_is_maximized_by_its_key_and_button_and_stays_so() {
    let Some((mut h, area)) = area_harness("window-maximize-keys") else { return };
    let props = Tab::AreaProperties(area);
    let pane = |h: &Harness<'_, Moonglow>| {
        let s = h.state();
        let path = s.dock.find_tab(&props).expect("open");
        assert!(!path.surface.is_main(), "in a window of its own");
        s.dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1.rect
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
    h.run_steps(8);
    let first = pane(&h);
    let maximized = |h: &Harness<'_, Moonglow>| pane(h).area() > first.area() + 1000.0;
    // The key, with the window's tab in front.
    h.get_by_label("Maximize").click();
    h.run_steps(8);
    assert!(maximized(&h), "by its button: {:?}", pane(&h));
    h.get_by_label("Restore");
    assert!(h.query_by_label("Maximize").is_none(), "the button says it is maximized");
    // Moonglow's window larger: the maximized one with it.
    let before = pane(&h);
    h.set_size(egui::vec2(1300.0, 950.0));
    h.run_steps(8);
    let after = pane(&h);
    assert!(after.width() > before.width() + 150.0, "{after:?} after {before:?}");
    assert!(after.bottom() > 950.0 * 0.95, "to the window's foot: {after:?}");
    // And smaller.
    h.set_size(egui::vec2(1150.0, 820.0));
    h.run_steps(8);
    let smaller = pane(&h);
    assert!(smaller.right() <= 1150.0 + 1.0 && smaller.bottom() <= 820.0 + 1.0, "{smaller:?}");
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
    // (Its tab clicked: the window's pane in front, not the palettes'.)
    let tab = pane(&h).left_top() + egui::vec2(60.0, 10.0);
    h.hover_at(tab);
    press(&h, tab, true, egui::Modifiers::NONE);
    press(&h, tab, false, egui::Modifiers::NONE);
    h.run_steps(3);
    h.hover_at(tab + egui::vec2(0.0, 200.0));
    h.run_steps(60);
    let front = h.state().front_tab();
    assert_eq!(front.as_ref(), Some(&props), "the tab in front");
    h.key_press_modifiers(keys, egui::Key::M);
    h.run_steps(8);
    assert!(!maximized(&h), "put back by the key: {:?}", pane(&h));
    h.get_by_label("Maximize");
    h.key_press_modifiers(keys, egui::Key::M);
    h.run_steps(8);
    assert!(maximized(&h), "by the key: {:?}", pane(&h));
    h.get_by_label("Restore").click();
    h.run_steps(8);
    assert!(!maximized(&h), "put back by the button");
    // Its corner dragged by hand: the next of its kind opens so.
    let corner = pane(&h).right_bottom() + egui::vec2(2.0, 2.0);
    h.hover_at(corner);
    h.run_steps(2);
    press(&h, corner, true, egui::Modifiers::NONE);
    for k in 1..=6 {
        h.hover_at(corner - egui::vec2(15.0, 10.0) * k as f32);
        h.run_steps(1);
    }
    press(&h, corner - egui::vec2(90.0, 60.0), false, egui::Modifiers::NONE);
    h.run_steps(4);
    let dragged = pane(&h);
    assert!((first.width() - dragged.width() - 90.0).abs() < 8.0, "{dragged:?} from {first:?}");
    let kept = h.state().settings.window_sizes.iter().find(|(k, _)| k == "area-properties");
    let kept = kept.map(|(_, s)| *s).expect("remembered");
    h.state_mut().actions.push(mg_ui::Action::CloseTab(props.clone()));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
    h.run_steps(8);
    let again = pane(&h);
    assert!((again.size() - dragged.size()).length() < 3.0, "opens at {again:?}, {dragged:?}");
    // (The window's size, its frame and all: a point or two over its pane.)
    let near = |a: u32, b: f32| (a as f32 - b).abs() < 4.0;
    assert!(near(kept[0], dragged.width()) && near(kept[1], dragged.height()), "{kept:?}");
    assert!(kept[0] as f32 <= first.width() - 80.0, "the dragged size kept: {kept:?}");
}

/// Escape closes what is in front: the Variables window over a Properties
/// window first (which a click on Properties doesn't bury), then the
/// Properties window itself.
#[test]
fn escape_closes_the_window_in_front() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("escape") else { return };
    h.run_steps(40);
    let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
    }
    h.run_steps(5);
    let path = mg_edit::GffPath::root().item("WaypointList", 1);
    let tab = Tab::Instance { area, path };
    assert!(h.state().dock.find_tab(&tab).is_some(), "the Properties tab");
    h.get_by_label("Advanced").click();
    h.run_steps(3);
    h.get_by_label_contains("Variables (").click();
    h.run_steps(3);
    assert!(h.state().var_edit.is_some(), "the Variables window");
    // A click in Properties, behind it: the Variables window stays in front.
    let props = h
        .state()
        .dock
        .iter_leaves()
        .find(|(p, _)| !p.surface.is_main())
        .map(|(_, leaf)| leaf.rect)
        .unwrap();
    // (Above its OK and Cancel.)
    let beside = props.left_bottom() + egui::vec2(12.0, -48.0);
    h.hover_at(beside);
    press(&h, beside, true, egui::Modifiers::NONE);
    press(&h, beside, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let top = h.ctx.memory(|m| m.areas().top_layer_id(egui::Order::Middle)).unwrap();
    assert_eq!(top.id, egui::Id::new(Some("Variables")), "in front of Properties");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(h.state().var_edit.is_none(), "Escape closed Variables");
    assert!(h.state().dock.find_tab(&tab).is_some(), "and only it");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(h.state().dock.find_tab(&tab).is_none(), "then Properties");
}

/// Properties windows and conversations open as tabs of the window one
/// of their kind is open in already, or, with their options off, each in
/// a window of its own.
#[test]
fn properties_and_conversations_open_as_tabs_of_one_window() {
    use mg_module::dialog::new_dialog;
    let dir = mg_testkit::scratch_dir("ui-properties-together");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let name = |n: &str, t: ResType| ResKey::parse(n, t).unwrap();
    let waypoints: Vec<ResKey> = ["wp_a", "wp_b", "wp_c"].map(|n| name(n, ResType::UTW)).to_vec();
    for key in &waypoints {
        m.set_gff(*key, &mg_gff::Gff::new(*b"UTW ")).unwrap();
    }
    let talks: Vec<ResKey> = ["talk_a", "talk_b", "talk_c"].map(|n| name(n, ResType::DLG)).to_vec();
    for key in &talks {
        m.set_gff(*key, &new_dialog()).unwrap();
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    let open = |h: &mut Harness<'_, Moonglow>, tab: Tab| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
        h.run_steps(4);
        let at = h.state().dock.find_tab(&tab).expect("the tab");
        (at.surface, at.node)
    };
    // Off: each in a window of its own.
    h.state_mut().settings.properties_own_windows = true;
    h.state_mut().settings.dialogs_own_windows = true;
    let first = open(&mut h, Tab::Blueprint(waypoints[0]));
    let second = open(&mut h, Tab::Blueprint(waypoints[1]));
    assert_ne!(first.0, second.0);
    let talk = open(&mut h, Tab::Dialog(talks[0]));
    assert_ne!(open(&mut h, Tab::Dialog(talks[1])).0, talk.0);
    // On: a tab of a window of its kind, and not of another kind's.
    h.state_mut().settings.properties_own_windows = false;
    h.state_mut().settings.dialogs_own_windows = false;
    let third = open(&mut h, Tab::Blueprint(waypoints[2]));
    assert!(third == first || third == second, "a tab of a Properties window");
    let talks_in: Vec<_> = [talks[0], talks[1]]
        .iter()
        .map(|k| {
            let at = h.state().dock.find_tab(&Tab::Dialog(*k)).unwrap();
            (at.surface, at.node)
        })
        .collect();
    let last = open(&mut h, Tab::Dialog(talks[2]));
    assert!(talks_in.contains(&last), "a tab of a conversation's window");
    assert!(last != first && last != second);
}

/// Scripts open as tabs of one window (the window a script is open in
/// already), or, with the option off, each in a window of its own.
#[test]
fn scripts_open_as_tabs_of_one_window() {
    let dir = mg_testkit::scratch_dir("ui-scripts-together");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let keys: Vec<ResKey> =
        ["one", "two", "three"].iter().map(|n| ResKey::parse(n, ResType::NSS).unwrap()).collect();
    for key in &keys {
        m.set(*key, b"void main()\n{\n}\n".to_vec());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    let open = |h: &mut Harness<'_, Moonglow>, key: ResKey| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        h.run_steps(4);
        let at = h.state().dock.find_tab(&Tab::Script(key)).expect("the script's tab");
        (at.surface, at.node)
    };
    let first = open(&mut h, keys[0]);
    let second = open(&mut h, keys[1]);
    assert_eq!(first, second, "a tab of the first one's window");
    h.state_mut().settings.scripts_own_windows = true;
    let third = open(&mut h, keys[2]);
    assert_ne!(third.0, first.0, "a window of its own");
}
