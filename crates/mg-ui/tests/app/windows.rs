//! Windows, panes and tabs: docking, folding, the log, and tabs opened
//! again with a module.

use crate::common::*;

/// GitHub issue 11: with the last tab of the middle closed, the palettes'
/// pane took the whole window. The middle keeps its place (a pane that
/// says no area is open), and the Right Dock (the palettes, a panel since
/// M3 T5) its width. (The issue's other half, a tool's tab docked beside
/// the palettes in their pane, is gone with the pane: ruling R5.)
#[test]
fn the_middle_keeps_its_place_with_no_area_open() {
    let Some(mut h) = game_harness("no-area") else { return };
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    h.state_mut().settings.no_last_area = true;
    h.run();
    let width = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let mut leaves = h.state().dock.iter_leaves().map(|(_, l)| l);
        leaves.find(|l| l.tabs.contains(tab)).map(|l| l.rect.width())
    };
    let has = |h: &Harness<'_, Moonglow>, tab: &Tab| h.state().dock.find_tab(tab).is_some();
    let dock = |h: &Harness<'_, Moonglow>| right_dock_rect(h).width();
    // (The module opened on its first area.)
    let area = Tab::Area(ResRef::from_str("start").unwrap());
    assert!(palette_shown(h.state()) && has(&h, &area) && !has(&h, &Tab::NoArea));
    let palette = dock(&h);

    // The middle's tabs closed (Module Properties was docked there as
    // the module opened): its place is kept.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(area.clone()));
    h.run();
    assert!(!has(&h, &Tab::NoArea), "Module Properties is in the middle still");
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    h.run();
    assert!(has(&h, &Tab::NoArea));
    h.get_by_label("No area open");
    let now = dock(&h);
    assert!((now - palette).abs() < 3.0, "the Right Dock: {palette} wide, then {now}");
    assert!(width(&h, &Tab::NoArea).unwrap() > 300.0);

    // An area opened takes its place (the pane lists those opened lately
    // and the module's, each a link, beside the module tree's row); the
    // Right Dock stays.
    assert_eq!(h.query_all_by_label("start").count(), 3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(area.clone()));
    h.run();
    h.run();
    assert!(has(&h, &area) && !has(&h, &Tab::NoArea));
    let now = dock(&h);
    assert!((now - palette).abs() < 3.0, "the Right Dock: {palette} wide, then {now}");
    // And closed again, the same.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(area.clone()));
    h.run();
    h.run();
    assert!(has(&h, &Tab::NoArea));
    let now = dock(&h);
    assert!((now - palette).abs() < 3.0, "the Right Dock: {palette} wide, then {now}");
    // The area just opened is offered first, among those opened lately.
    h.get_by_label("Opened lately");
    assert_eq!(h.query_all_by_label("start").count(), 3);
}

/// Without the palettes (no game), the middle is not left blank either.
#[test]
fn the_middle_says_no_area_is_open_without_the_palettes() {
    let dir = mg_testkit::scratch_dir("ui-no-area-plain");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = window_harness(app);
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

/// GitHub issue 15: the tabs by the keyboard, as in a browser, in the
/// editor group with the focus (here the only one; two groups:
/// `doc_tabs::the_tab_keys_act_on_the_focused_group`). Ctrl+W closes the
/// one in front, Ctrl+Tab and Ctrl+Shift+Tab (and Ctrl+Page Down and Up)
/// go round them, Ctrl and a digit goes to one by its number (9: the
/// last), Ctrl+Shift+T opens the one closed last again.
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

/// GitHub issue 10: the docks fold away and come back (View, Ctrl+Alt+1,
/// 2, 3 and 0, and each one's own control: the Left and Right Docks' fold
/// buttons at their tab bars' ends and, folded, the chevrons at their
/// strips' tops; the Log's switch in the Status Bar), each as wide as it
/// was.
#[test]
fn panels_fold_away_and_come_back() {
    let Some(mut h) = game_harness("panels") else { return };
    // (A window wide enough that no width folds a dock: the test reads the
    // docks as the builder left them, M3 T14.)
    wide_window(&mut h);
    h.run();
    h.run();
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press_modifiers(keys, key);
        h.run();
        h.run();
    };
    // (The Right Dock's width, while it is unfolded.)
    let palette = |h: &Harness<'_, Moonglow>| {
        h.query_by_label("Fold the Right Dock").is_some().then(|| right_dock_rect(h).width())
    };
    let tree = |h: &Harness<'_, Moonglow>| h.query_by_label("Fold the Left Dock").is_some();
    let log_switch = |h: &Harness<'_, Moonglow>| {
        let node = h.get_by_label("Status Bar").get_by_label("Log");
        node.click();
    };
    let was = palette(&h).expect("the Right Dock");
    assert!(tree(&h));

    // The Left Dock: folded to a strip, and back by the strip's chevron.
    key(&mut h, egui::Key::Num1);
    assert!(!tree(&h) && h.state().settings.hide_tree);
    h.get_by_label("Show the Left Dock").click();
    h.run();
    assert!(tree(&h) && !h.state().settings.hide_tree);
    // Its own button folds it away too; the Log's switch closes the log.
    h.get_by_label("Fold the Left Dock").click();
    h.run();
    assert!(!tree(&h) && h.state().settings.hide_tree);
    h.get_by_label("Show the Left Dock").click();
    h.run();
    log_switch(&h);
    h.run();
    assert!(h.state().settings.hide_log);
    log_switch(&h);
    h.run();
    assert!(!h.state().settings.hide_log && tree(&h));
    // The Right Dock (the palettes): folded to a strip, and back as wide
    // by the button beside the strip's edge.
    key(&mut h, egui::Key::Num2);
    assert!(palette(&h).is_none() && h.state().settings.hide_palettes);
    h.get_by_label("Show the Right Dock").click();
    h.run();
    h.run();
    let now = palette(&h).expect("back");
    assert!((now - was).abs() < 3.0, "{was} wide, then {now}");
    // Its own button folds it away as the Left Dock's does.
    h.get_by_label("Fold the Right Dock").click();
    h.run();
    assert!(palette(&h).is_none() && h.state().settings.hide_palettes);
    h.get_by_label("Show the Right Dock").click();
    h.run();
    h.run();
    assert!(palette(&h).is_some());
    // The log.
    key(&mut h, egui::Key::Num3);
    assert!(h.state().settings.hide_log);
    log_switch(&h);
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
    // (Its row Left Dock, as the dock's tab bar is named: the check box.)
    let row = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::CheckBox && n.label().as_deref() == Some("Left Dock")
    };
    let ticked = h.get(egui_kittest::kittest::by().predicate(row)).accesskit_node().toggled();
    assert_eq!(ticked, Some(egui::accesskit::Toggled::True), "Left Dock ticked");
    // (And Right Dock, the palettes' row, so named since M3 T5.)
    let right = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::CheckBox && n.label().as_deref() == Some("Right Dock")
    };
    let ticked = h.get(egui_kittest::kittest::by().predicate(right)).accesskit_node().toggled();
    assert_eq!(ticked, Some(egui::accesskit::Toggled::True), "Right Dock ticked");
    h.get_by_label("Reset Layout").click();
    h.run();
    h.run();
    assert!(palette(&h).is_some());
}

/// Tab reaches the Right Dock's fold button on its tab bar, after the
/// tabs and before the panel's body (M3 T14's review: the button is at the
/// bar's end, inside the dock), then goes on into the body; Shift+Tab from
/// the body comes back to it. With each tab in
/// front: the palettes' types, search and list; the Preview's body (a
/// creature chosen: its model viewer's controls), drawn in the dock in its
/// place. Enter on it folds the dock.
#[test]
fn tab_reaches_the_right_dock_s_fold_button_after_its_controls() {
    let Some((mut h, _)) = area_harness("right-dock-tab-order") else { return };
    // (1600 wide: no width folds a dock.)
    wide_window(&mut h);
    h.state_mut().palette.selected = ResKey::parse("nw_bandit001", ResType::UTC);
    for (name, panel) in [("Palette", mg_ui::Panel::Palette), ("Preview", mg_ui::Panel::Preview)] {
        h.state_mut().actions.push(mg_ui::Action::ShowPanel(panel));
        settle(&mut h);
        let dock = right_dock_rect(&h);
        // (Under the tab bar: the panel's body.)
        let body = h.get_by_label("Right Dock").get_by_label(name).rect().bottom();
        h.get_by_label("Right Dock").get_by_label(name).focus();
        settle(&mut h);
        let fold_is_focused = |h: &Harness<'_, Moonglow>| {
            h.query_by_label("Fold the Right Dock").is_some_and(|n| n.accesskit_node().is_focused())
        };
        let focused = |h: &Harness<'_, Moonglow>| {
            h.ctx.memory(|m| m.focused()).and_then(|id| h.ctx.read_response(id))
        };
        // From the panel's tab: the tabs, then the fold button, nothing of
        // the body before it.
        let mut steps = 0;
        while !fold_is_focused(&h) {
            h.key_press(egui::Key::Tab);
            settle(&mut h);
            steps += 1;
            assert!(steps < 4, "{name}: the fold button after the tabs");
            if let Some(r) = focused(&h).filter(|_| !fold_is_focused(&h)) {
                assert!(r.rect.bottom() <= body + 0.5, "{name}: the body first: {:?}", r.rect);
            }
        }
        // On from it into the body; Shift+Tab back to it.
        h.key_press(egui::Key::Tab);
        settle(&mut h);
        let next = focused(&h).expect("a control after it");
        assert!(
            next.rect.top() >= body - 0.5 && dock.contains_rect(next.rect),
            "{name}: into the body: {next:?}"
        );
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
        settle(&mut h);
        assert!(fold_is_focused(&h), "{name}: Shift+Tab back to it");
    }
    h.get_by_label("Fold the Right Dock").focus();
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().settings.hide_palettes, "folded by Enter on the button");
}

/// GitHub issue 10, as its reporter put it on 1.31.0, kept by the shell:
/// one control per dock or pane to fold it away or close it. Since M3
/// T14's review, where the boards put it: the open side dock's fold button
/// on its tab bar, inside the dock (the Left Dock's at the bar's end
/// toward the editor area, the Right Dock's at the bar's end by the
/// window's edge; no board draws one; beside the dock's edge it lay over
/// the editor area's view, taking its clicks), and folded, the chevron at
/// its strip's top (`Narrow.png`'s rail), the strip's one control that
/// unfolds it, with its panels' buttons under it. Each is named and tipped
/// by what it does, and a click of the pointer where it is drawn folds or
/// unfolds the dock (nothing over it takes the click); the dock's edge
/// drags, high and low. The Bottom Dock's is the Log's switch in the
/// Status Bar, which says whether it is on and stays where it is.
#[test]
fn each_side_dock_folds_by_one_button_on_its_tab_bar() {
    let Some(mut h) = game_harness("pane-edges") else { return };
    // (1600 wide: no width folds a dock, and the Right Dock's edge drags.)
    wide_window(&mut h);
    // A dock's or pane's button, folded or not (and that there is one).
    let button = |h: &Harness<'_, Moonglow>, name: &str| {
        let fold = format!("Fold {name}");
        let show = format!("Show {name}");
        let all: Vec<egui::Rect> = (h.query_all_by_label(&fold).chain(h.query_all_by_label(&show)))
            .map(|n| n.rect())
            .collect();
        assert_eq!(all.len(), 1, "one button for {name}: {all:?}");
        all[0]
    };
    // The middle (the editor area's panes).
    let middle = |h: &Harness<'_, Moonglow>| {
        let leaves = h.state().dock.iter_leaves().filter(|(p, _)| p.surface.is_main());
        let rects = leaves.map(|(_, l)| l.rect);
        rects.reduce(|a, b| a.union(b)).expect("a middle")
    };
    let window = h.ctx.content_rect();
    let near = |a: f32, b: f32, what: &str| assert!((a - b).abs() < 1.5, "{what}: {a} and {b}");
    // (The editor area's panes reach the docks' edges: flush, as the
    // boards draw it, since M3 T14.)
    const MARGIN: f32 = 0.0;
    for (dock, name, rect) in [
        ("Left Dock", "the Left Dock", left_dock_rect as fn(&Harness<'_, Moonglow>) -> egui::Rect),
        (mg_ui::RIGHT_DOCK, "the Right Dock", right_dock_rect),
    ] {
        // Open: on the dock's tab bar, inside the dock, over nothing of
        // the editor area.
        let shown = button(&h, name);
        let tab = if dock == "Left Dock" { "Module" } else { "Palette" };
        let bar = h.get_by_label(dock).get_by_label(tab).rect();
        let area = middle(&h);
        assert!(rect(&h).contains_rect(shown), "{name}: inside the dock {shown:?}");
        assert!(bar.y_range().contains(shown.center().y), "{name}: on its tab bar {shown:?}");
        // (The Left Dock's at its bar's end toward the editor area, the
        // Right Dock's at its bar's end by the window's edge: each at the
        // dock's right, within the tools' few points of room.)
        let end = rect(&h).right() - shown.right();
        assert!((0.0..=6.0).contains(&end), "{name}: at its bar's right end, {end} short");
        assert!(!shown.intersects(area), "{name}: over nothing of the editor area");
        // (The pointer's click where the button is drawn: a widget over
        // it would take it.)
        click_pos(&h, shown.center());
        h.run_steps(2);
        let folded = match dock {
            "Left Dock" => h.state().settings.hide_tree,
            _ => h.state().settings.hide_palettes,
        };
        assert!(folded, "{name}: folded by its button");
        // Folded: the chevron at the strip's top, in the strip's group,
        // over its panels' buttons.
        let chevron = button(&h, name);
        let strip = h.get_by_label(dock);
        let first_panel =
            strip.get_by_label(if dock == "Left Dock" { "Module" } else { "Palette" });
        assert!(strip.query_by_label(&format!("Show {name}")).is_some(), "{name}: on the strip");
        assert!(chevron.bottom() <= first_panel.rect().top(), "{name}: at the strip's top");
        assert!(window.contains_rect(chevron), "{name}: in the window {chevron:?}");
        click_pos(&h, chevron.center());
        h.run_steps(2);
        let folded = match dock {
            "Left Dock" => h.state().settings.hide_tree,
            _ => h.state().settings.hide_palettes,
        };
        assert!(!folded, "{name}: unfolded by its chevron");
        assert_eq!(button(&h, name), shown, "{name}: back where it was");
    }
    // The Right Dock's edge drags (wider by a drag to the left), high and
    // low; and the Left Dock's.
    let dock = right_dock_rect(&h);
    for y in [dock.top() + 150.0, dock.bottom() - 150.0] {
        let edge = right_dock_rect(&h).left();
        let from = egui::pos2(edge + 1.0, y);
        drag(&mut h, from, from - egui::vec2(30.0, 0.0));
        settle(&mut h);
        let now = right_dock_rect(&h).left();
        assert!((edge - now - 30.0).abs() < 2.0, "dragged at {y}: {edge} {now}");
    }
    let dock = left_dock_rect(&h);
    for y in [dock.top() + 150.0, dock.bottom() - 150.0] {
        let edge = left_dock_rect(&h).right();
        let from = egui::pos2(edge - 1.0, y);
        drag(&mut h, from, from + egui::vec2(30.0, 0.0));
        settle(&mut h);
        let now = left_dock_rect(&h).right();
        assert!((now - edge - 30.0).abs() < 2.0, "dragged at {y}: {edge} {now}");
    }
    assert!(!h.state().settings.hide_palettes && !h.state().settings.hide_tree);

    // The Bottom Dock: under the middle, its tab bar over the log's lines;
    // its one control the Log's switch in the Status Bar, on while it is
    // open, in the same place closed.
    let switch = |h: &Harness<'_, Moonglow>| {
        let node = h.get_by_label("Status Bar").get_by_label("Log");
        (node.rect(), node.accesskit_node().toggled())
    };
    let on = Some(egui::accesskit::Toggled::True);
    let (at, state) = switch(&h);
    assert_eq!(state, on);
    // (The Log's tab: the tab bar's height.)
    let bar = h.get_by_label("Bottom Dock").get_by_label("Log").rect();
    let clear = h.get_by_label("Bottom Dock").get_by_label("Clear Log").rect();
    let on_bar = bar.y_range().contains(clear.center().y) && clear.left() > bar.right();
    assert!(on_bar, "the log's buttons on its tab bar: {bar:?} {clear:?}");
    near(bar.top() - MARGIN, middle(&h).bottom(), "under the middle");
    assert!(at.top() > bar.bottom() && window.contains_rect(at), "under the dock: {at:?}");
    h.get_by_label("Status Bar").get_by_label("Log").click();
    h.run_steps(2);
    assert!(h.state().settings.hide_log);
    assert!(h.query_by_label("Bottom Dock").is_none() && h.query_by_label("Clear Log").is_none());
    let (closed, state) = switch(&h);
    assert_eq!(state, Some(egui::accesskit::Toggled::False));
    assert_eq!(closed, at, "where it was");
    h.get_by_label("Status Bar").get_by_label("Log").click();
    h.run_steps(2);
    assert!(!h.state().settings.hide_log);
    // The docks' and panes' controls are the only ones that fold or show
    // them (the tree's and the palettes' arrows after their types, and the
    // log's arrow buttons, are gone): every button named for one, by
    // itself or with a verb, is one of the three.
    let panes =
        ["Module Tree", "Palettes", "Log", "the Left Dock", "the Right Dock", "the Bottom Dock"];
    let verbs = ["", "Fold ", "Show ", "Hide ", "Close ", "Collapse ", "Expand "];
    let mut named: Vec<String> = h
        .query_all_by(|n| n.role() == egui::accesskit::Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| panes.iter().any(|p| verbs.iter().any(|v| *l == format!("{v}{p}"))))
        .collect();
    named.sort();
    assert_eq!(named, ["Fold the Left Dock", "Fold the Right Dock", "Log"]);
}

/// The palettes are the Right Dock's Palette panel, fixed there (the
/// owner's ruling R5 of 2026-11-11), not a tab: Tools › Palettes shows the
/// panel in the Right Dock and puts no tab in the editor area, so there is
/// no tab of theirs to drag into the middle (GitHub issue 20's drop,
/// retired with `the_palettes_dropped_on_the_middle_join_its_tabs`), nor a
/// pane of theirs for tabs to open in
/// (`tabs_open_beside_the_palette_not_in_its_pane`, retired). The editor
/// area holds the editors alone: the area's pane is the main window's
/// only one, and tabs opened go to it or to windows; the module's open
/// tabs are kept without a tab or a pane of the palettes. (The kit's
/// tabs, the dock's, are not egui_dock's and cannot be dragged at all.)
#[test]
fn the_palette_is_a_panel_not_a_tab() {
    let Some((mut h, area)) = area_harness("palette-panel") else { return };
    let path = h.state().module_path().expect("saved");
    let kept = mg_testkit::scratch_dir("ui-palette-panel-tabs");
    h.state_mut().open_tabs_dir = Some(kept.clone());
    fold_palette(h.state_mut());
    settle(&mut h);
    assert!(palette_types(&h).is_none(), "folded");
    let leaves = |h: &Harness<'_, Moonglow>| -> Vec<Vec<Tab>> {
        h.state().dock.iter_leaves().map(|(_, l)| l.tabs.clone()).collect()
    };
    let before = leaves(&h);
    menu_command(&mut h, "Tools", "Palettes");
    settle(&mut h);
    assert!(palette_shown(h.state()));
    assert_eq!(leaves(&h), before, "the editor area's panes and tabs as they were");
    // The palette drawn in the Right Dock, beside the editor area.
    let dock = right_dock_rect(&h);
    let types = palette_types(&h).expect("the palette's types").rect();
    assert!(dock.contains_rect(types), "the palette in the Right Dock: {types:?} {dock:?}");
    let editor = (h.state().dock.iter_leaves())
        .find(|(_, l)| l.tabs.contains(&Tab::Area(area)))
        .map(|(_, l)| l.rect)
        .unwrap();
    assert!(editor.right() <= dock.left(), "the editor area beside it: {editor:?} {dock:?}");
    // Tools opened beside it go to the editor area or windows: one pane
    // of the main window, the area's.
    let script = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    for tab in [Tab::Manual, script, Tab::Factions] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        settle(&mut h);
    }
    let main: Vec<Vec<Tab>> = (h.state().dock.iter_leaves())
        .filter(|(p, _)| p.surface.is_main())
        .map(|(_, l)| l.tabs.clone())
        .collect();
    assert_eq!(main.len(), 1, "{main:?}");
    assert!(main[0].contains(&Tab::Area(area)), "{main:?}");
    // The open tabs kept with the module: the area and the tools, no tab
    // or pane of the palettes.
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    let file = mg_ui::open_tabs::file(&kept, &path);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains(&format!("area:{area}")) && text.contains("factions"), "{text}");
    assert!(!text.contains("\"palettes\""), "no palettes' pane: {text}");
    assert!(!text.contains("\"palette\""), "no palettes' tab: {text}");
}

/// GitHub issue 21: a tool's tab dropped on a side of the middle pane is
/// docked there, beside it, as the targets drawn say. (From a pane of its
/// own beside the middle, as the reporter's was beside the palettes in
/// their pane, gone since the palettes are the Right Dock's panel;
/// dragged from a window of its own, the window moves, here and in the
/// report, and was not seen to dock.) The report's tab was the resource
/// browser's, a panel since (ruling R5); the Faction Editor's is dragged
/// in its place.
#[test]
fn a_tool_tab_dropped_on_a_side_is_docked_there() {
    let Some((mut h, area)) = area_harness("dock-tool-side") else { return };
    // (In a pane of its own split off the middle's right.)
    {
        let dock = &mut h.state_mut().dock;
        let node = dock.find_tab(&Tab::Area(area)).unwrap().node;
        dock.main_surface_mut().split_right(node, 0.7, vec![Tab::Factions]);
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
    let (rect, viewport) = leaf(&h, &Tab::Factions);
    // (Its tab: the first in its pane's bar, after egui_dock's arrow.)
    let tab = egui::pos2(rect.left() + 60.0, (rect.top() + viewport.top()) / 2.0);
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
    let (tool, area_pane) = (main(&Tab::Factions), main(&Tab::Area(area)));
    assert_eq!((tool.0, tool.1), (true, 1), "{:?}", layout(&h));
    assert!(tool.2 < area_pane.2, "left of the area: {:?}", layout(&h));
    // The Right Dock follows the pane with the focus: the tool's brings
    // none; the area's the palettes, as they were.
    click_tab(&mut h, &Tab::Factions);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Factions));
    assert!(h.state().right_dock_rect().is_none(), "no Right Dock beside the tool");
    click_tab(&mut h, &Tab::Area(area));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Area(area)));
    assert!(palette_shown(h.state()), "the palettes where they were");
}

/// The resource browser is the Left Dock's Resources panel, fixed there
/// (the owner's ruling R5 of 2026-11-11), not a tab: Tools › Resource
/// Browser shows the panel and puts no tab in the editor area, so there
/// is no tab of it to drag to a side or into the middle (GitHub issue 21's
/// drop, retired with `the_resource_browser_dropped_on_a_side_is_docked_there`).
/// The editor area keeps the middle as it was, its panes and their tabs.
#[test]
fn the_resource_browser_is_a_panel_not_a_tab() {
    let Some((mut h, area)) = area_harness("browser-panel") else { return };
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    settle(&mut h);
    let leaves = |h: &Harness<'_, Moonglow>| -> Vec<(Vec<Tab>, egui::Rect)> {
        let dock = &h.state().dock;
        dock.iter_leaves().map(|(_, l)| (l.tabs.clone(), l.rect)).collect()
    };
    let before = leaves(&h);
    let tabs = h.state().dock.iter_all_tabs().count();
    menu_command(&mut h, "Tools", "Resource Browser");
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources));
    h.get_by_label("Filter Resources");
    assert_eq!(h.state().dock.iter_all_tabs().count(), tabs, "no tab of its own");
    let after = leaves(&h);
    assert_eq!(
        after.iter().map(|l| &l.0).collect::<Vec<_>>(),
        before.iter().map(|l| &l.0).collect::<Vec<_>>(),
        "the editor area's panes and tabs as they were"
    );
    // (The area's pane where it was: the Left Dock as wide on either tab.)
    let middle = |ls: &[(Vec<Tab>, egui::Rect)]| {
        ls.iter().find(|l| l.0.contains(&Tab::Area(area))).map(|l| l.1).unwrap()
    };
    assert_eq!(middle(&after), middle(&before));
}

/// GitHub issue 23 (retired with
/// `the_resource_browser_alone_keeps_the_middle_beside_it`: the browser
/// is no pane of the editor area to be left alone in it): with the
/// Resources panel shown and the area and Module Properties closed (the
/// palettes are the Right Dock's panel), the middle keeps the whole editor
/// area, a pane that says no area is open; the Left Dock beside it as wide
/// as it was.
#[test]
fn the_middle_keeps_the_editor_area_whatever_the_left_dock_shows() {
    let Some((mut h, area)) = area_harness("browser-panel-middle") else { return };
    // (A window wide enough that no width folds a dock: the test reads the
    // docks as the builder left them, M3 T14.)
    wide_window(&mut h);
    show_resources(h.state_mut());
    settle(&mut h);
    // (The dock's inner edge.)
    let edge = |h: &Harness<'_, Moonglow>| left_dock_rect(h).right();
    let dock = edge(&h);
    for tab in [Tab::Area(area), Tab::ModuleProperties] {
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab]));
        settle(&mut h);
    }
    let leaves: Vec<(Vec<Tab>, egui::Rect)> = (h.state().dock.iter_leaves())
        .filter(|(p, _)| p.surface.is_main())
        .map(|(_, l)| (l.tabs.clone(), l.rect))
        .collect();
    assert_eq!(leaves.len(), 1, "{leaves:?}");
    assert_eq!(leaves[0].0, [Tab::NoArea], "{leaves:?}");
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources));
    assert_eq!(edge(&h), dock, "the Left Dock as wide");
    // (The middle from the dock's edge, the editor area's margin between.)
    let gap = leaves[0].1.left() - dock;
    assert!((0.0..16.0).contains(&gap), "{:?} from {dock}", leaves[0].1);
}

/// GitHub issue 22: a tool window closed in a pane split off a side of
/// the middle (an editor group of its own) opens there again, as large;
/// while that group is there, others of its kind join it
/// (`doc_tabs::a_kind_kept_beside_the_middle_opens_in_that_group`). (The palettes, which could be
/// split off another side than their right too, are the Right Dock's
/// panel since M3 T5, fixed there: ruling R5.)
#[test]
fn a_tool_window_reopens_in_its_split_of_the_middle() {
    reopens_in_its_split(Mode::Floating);
}

/// [`a_tool_window_reopens_in_its_split_of_the_middle`] where editors open
/// as tabs (a new installation's): the manual, a tab of the area's group,
/// dragged off below it (a move by hand, which As Tabs keeps).
#[test]
fn a_tool_reopens_in_its_split_of_the_middle_as_tabs() {
    reopens_in_its_split(Mode::Tabs);
}

fn reopens_in_its_split(mode: Mode) {
    let Some((mut h, area)) = area_harness(&mode.dir("dock-splits")) else { return };
    put_in(h.state_mut(), mode);
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
    // The user manual, split off below the middle, a third as tall.
    // (The fraction: the upper pane's, the middle's.) (The resource
    // browser was this tool until it was the Left Dock's panel.)
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Manual));
    h.run_steps(3);
    assert_eq!(pane(&h, &Tab::Manual).0, mode.tabs(), "where editors open");
    // (Moved by hand: noted as a drag notes it.)
    h.state_mut().moved_by_hand(&Tab::Manual);
    {
        let (node, _) = middle(&h);
        let dock = &mut h.state_mut().dock;
        let window = dock.find_tab(&Tab::Manual).unwrap();
        dock.remove_tab(window);
        dock.main_surface_mut().split_below(node, 0.67, vec![Tab::Manual]);
    }
    h.run_steps(3);
    let placed = |h: &Harness<'_, Moonglow>, kind: &str| {
        let places = &h.state().settings.window_places;
        places.iter().find(|(k, _)| k == kind).map(|(_, p)| p.clone())
    };
    assert_eq!(placed(&h, "manual").as_deref(), Some("split below 330"));
    let was = pane(&h, &Tab::Manual).2;
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Manual]));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Manual));
    h.run_steps(3);
    let (main, tabs, now) = pane(&h, &Tab::Manual);
    assert!(main && tabs == 1, "a pane of its own in the main window");
    let (_, area_rect) = middle(&h);
    assert!(now.top() >= area_rect.bottom() - 2.0, "below the middle: {now:?} {area_rect:?}");
    assert!((now.height() - was.height()).abs() < 4.0, "as tall: {was:?} {now:?}");
}

/// GitHub issue 14: a tool window opens where one of its kind was last
/// dragged to (a tab of the main pane, here), not always in a window of
/// its own; View › Reset Layout forgets that.
#[test]
fn a_tool_window_opens_where_its_kind_was_last_docked() {
    kind_last_put(Mode::Floating);
}

/// [`a_tool_window_opens_where_its_kind_was_last_docked`] where editors
/// open as tabs (a new installation's): a kind moved out to a window of
/// its own opens in one until View › Reset Layout.
#[test]
fn a_tool_opens_in_a_window_where_its_kind_was_last_put_as_tabs() {
    kind_last_put(Mode::Tabs);
}

fn kind_last_put(mode: Mode) {
    let dir = mg_testkit::scratch_dir(&mode.dir("ui-window-places"));
    let path = sample_module(&dir);
    let mut app = app_in(mode, Vec::new());
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
    // At first: where editors open (a window of its own over the area, or
    // a tab).
    let at_first = mode.tabs();
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(at_first));
    // Moved (done here on the dock itself): dragged into the main pane, or
    // out to a window of its own (as Eject or a drag out does).
    let dock = &mut h.state_mut().dock;
    let from = dock.find_tab(&Tab::Factions).unwrap();
    dock.remove_tab(from);
    match mode.tabs() {
        true => _ = dock.add_window(vec![Tab::Factions]),
        false => dock.main_surface_mut().push_to_first_leaf(Tab::Factions),
    }
    h.run();
    h.run();
    let kept = if mode.tabs() { "window" } else { "middle" };
    assert_eq!(h.state().settings.window_places, [("factions".to_string(), kept.to_string())]);
    // Closed and opened again: there, and another kind still as at first.
    close(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), None);
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(!at_first));
    open(&mut h, Tab::Journal);
    assert_eq!(in_main(&h, &Tab::Journal), Some(at_first));
    // Opened again while open, it stays where it is, in front.
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(!at_first));
    // Reset Layout: where editors open again.
    close(&mut h, Tab::Factions);
    h.get_by_label("View").click();
    h.run();
    h.get_by_label("Reset Layout").click();
    h.run();
    assert!(h.state().settings.window_places.is_empty());
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(at_first));
}

/// The log's lines carry the time they were said, before them, as the
/// Compiler tab's messages do (Jasperre's report); copied from the pane
/// (its lines are selectable), with it.
#[test]
fn the_log_s_lines_carry_their_time() {
    let mut app = app_with(Vec::new());
    app.log.info("First thing said");
    app.log.warn("Second thing said");
    let mut h = window_harness(app);
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

/// The whole log copied (its tab bar's Copy, View › Copy Log) and saved
/// into a text file (its Save, View › Save Log…), each line as it shows,
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
    let mut h = window_harness(app);
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
    // (A window's button: not the toolbars' or the palette's types, whose
    // Sounds was clicked for Options' Sounds page.)
    let button = |h: &Harness<'_, Moonglow>, label: &str| {
        use egui_kittest::kittest::NodeT;
        h.query_all_by_label(label)
            .filter(|n| !in_toolbar(n) && !in_group(n, "Palette types"))
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
        // The Right Dock's Palette panel (the palettes, a pane before), at
        // its least width too (the palettes' pane dragged to about 135 px
        // spilled a blueprint type's list toolbar under its fold button).
        show_palette(h.state_mut());
        look(&mut h, "Palettes");
        right_dock_width(&mut h, 280.0);
        look(&mut h, "Palettes at 280");
        h.state_mut().palette.tiles = false;
        look(&mut h, "Palettes on a blueprint type at 280");
        // The Left Dock's Resources panel (the browser, a tab before), and
        // the Module panel again.
        show_resources(h.state_mut());
        look(&mut h, "Resources");
        h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Module));
        look(&mut h, "Module");
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
fn the_log_shrinks_to_one_line() {
    let mut h = harness(app_with(Vec::new()), 1000.0, 700.0);
    h.run();
    let line = h.ctx.global_style().text_styles[&egui::TextStyle::Monospace].size;
    let entry = |h: &Harness<'_, Moonglow>| h.get_by_label_contains("No Neverwinter Nights").rect();
    let status = h.get_by_label("Status Bar").get_by_label("No module open").rect();
    // The Bottom Dock's top edge, just above its tab bar: drag it far
    // down.
    let top = h.get_by_label("Bottom Dock").get_by_label("Log").rect().top() - 1.0;
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
    // About a line between the log's first line and the Status Bar, under
    // its tab bar (three lines' room was the least it would take).
    let room = status.top() - entry(&h).top();
    assert!(room < 2.0 * line + 12.0, "the log keeps {room} points (a line is {line})");
    let bar = h.get_by_label("Bottom Dock").get_by_label("Log").rect();
    assert!(bar.bottom() <= entry(&h).top(), "its tab bar over its line: {bar:?}");
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

/// Editors open in windows of their own over the area, which stays in
/// front of the main pane, below the Top Bar: as Options › General › Open
/// editors has it for an existing user's settings, As Floating Windows
/// (`open_editors::editors_open_as_tabs_or_as_windows_as_the_switch_says`
/// has both).
#[test]
fn editors_open_in_windows_over_the_area() {
    let Some((mut h, area)) = area_harness("editor-windows") else { return };
    assert_eq!(h.state().settings.open_editors, mg_ui::OpenEditors::Floating);
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
    // (The Right Dock beside the tabs, as Tools › Palettes shows it: with
    // no game data its Palette panel says so. The tabs come back with
    // the dock shown, as they did with the palettes' pane.)
    show_palette(&mut app);
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
    let mut h = window_harness(app);
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

/// The talk table opens again on the line chosen when the module was
/// closed.
#[test]
fn the_talk_table_opens_again_on_its_line() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-open-tabs-talk-line");
    let user = user_folder(&dir, &["tlk"]);
    let mut tlk = mg_tlk::Tlk::new(Language::ENGLISH);
    for text in ["Zero", "One", "Two", "Three"] {
        tlk.entries.push(mg_tlk::TlkEntry::text(text));
    }
    std::fs::write(user.join("tlk/mg_lines.tlk"), tlk.to_bytes().unwrap()).unwrap();
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_lines".to_vec()));
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let kept = dir.join("open-tabs");
    let session = || {
        let mut app = game_app(&root, Some(user.clone()));
        app.settings.no_last_area = true;
        // (The Right Dock shown with the module, beside the tabs.)
        app.open_tabs_dir = Some(kept.clone());
        app.open_module(&path);
        harness(app, 1100.0, 800.0)
    };
    let mut h = session();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run_steps(3);
    assert_eq!(h.state().talk.as_ref().map(|t| t.len()), Some(4), "the module's table");
    h.state_mut().talk_view.selected = Some(2);
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let mut h = session();
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_some(), "open again");
    assert_eq!(h.state().talk_view.selected, Some(2), "on its line");
    // (No area in front, the Right Dock is not drawn; Tools › Palettes
    // shows it beside the tabs.)
    show_palette(h.state_mut());
    settle(&mut h);
    assert!(palette_shown(h.state()), "the Right Dock beside it");
    assert_eq!(h.state().talk_view.selected, Some(2), "still on its line");
}

/// Tile Properties kept with a module, still waiting for its area's view
/// when the module is closed again (the area never drawn: here, with no
/// game data), is kept again, not lost.
#[test]
fn tile_properties_waiting_for_its_area_is_kept_again() {
    let (path, app, kept) = tabs_app("ui-open-tabs-tiles-waiting");
    let mut h = window_harness(app);
    h.state_mut().open_module(&path);
    h.run();
    open_some_tabs(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    let file = mg_ui::open_tabs::file(&kept, &path);
    let mut record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let pane = serde_json::json!({ "place": "window", "tabs": ["tile-properties:start:5"] });
    record["panes"].as_array_mut().unwrap().push(pane);
    std::fs::write(&file, record.to_string()).unwrap();
    for session in 0..2 {
        h.state_mut().open_module(&path);
        h.run_steps(3);
        assert!(h.state().dock.find_tab(&Tab::TileProperties).is_none(), "waiting");
        assert!(h.state().dock.find_tab(&Tab::Factions).is_some(), "the others open");
        h.state_mut().actions.push(mg_ui::Action::Close);
        h.run();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("tile-properties:start:5"), "kept again ({session}): {text}");
    }
}

/// With Options › General's Open a module with the tabs it had off, a
/// module's tabs are neither kept nor opened again.
#[test]
fn a_module_s_tabs_are_not_opened_again_with_the_option_off() {
    let (path, app, kept) = tabs_app("ui-open-tabs-off");
    let mut h = window_harness(app);
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
    let mut h = window_harness(app);
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

/// A module's open tabs kept with a tab in the palettes' pane (place
/// `palettes`, before the palettes were the Right Dock's panel) open where
/// their kind opens (the Faction Editor: a window of its own), the module
/// opened again from a menu; and the palettes' own tab in a record
/// (`palette`, which no Moonglow wrote) puts the Palette panel in front of
/// the Right Dock, the builder's fold of the dock standing (folded, it
/// stays folded on the Palette; M3 T14's review: until then the record
/// unfolded it, past the builder's own fold). Kept again, the record has
/// neither. (In place of `a_tab_kept_in_the_palettes_pane_opens_there_again`:
/// the pane is gone.)
#[test]
fn a_tab_kept_in_the_palettes_pane_opens_where_its_kind_opens() {
    let Some((mut h, area)) = area_harness("open-tabs-palettes") else { return };
    let path = h.state().module_path().expect("saved");
    let kept = mg_testkit::scratch_dir("ui-open-tabs-palettes");
    h.state_mut().open_tabs_dir = Some(kept.clone());
    // (Folded by the builder, with the Preview in front, so that the
    // record's `palette` is seen to leave the fold and choose the panel.)
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Preview));
    settle(&mut h);
    fold_palette(h.state_mut());
    settle(&mut h);
    assert!(!palette_shown(h.state()));
    let file = mg_ui::open_tabs::file(&kept, &path);
    let record = format!(
        r#"{{"panes":[
        {{"place":"middle","tabs":["area:{area}"]}},
        {{"place":"palettes","tabs":["palette","factions"],"active":"factions"}}
    ]}}"#
    );
    std::fs::write(&file, record).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
    settle(&mut h);
    let app = h.state();
    assert!(app.dock.find_tab(&Tab::Area(area)).is_some());
    let factions = app.dock.find_tab(&Tab::Factions).expect("the Faction Editor open");
    assert!(!factions.surface.is_main(), "in a window of its own, where its kind opens");
    assert!(app.settings.hide_palettes, "the builder's fold stands");
    assert!(!palette_shown(app), "folded");
    let strip = h.get_by_label(mg_ui::RIGHT_DOCK);
    strip.get_by_label("Palette");
    // (Unfolded as the builder left it, View › Right Dock's: on the
    // panel the record put in front, not the Preview it had.)
    menu_command(&mut h, "View", "Right Dock");
    settle(&mut h);
    let app = h.state();
    assert!(palette_shown(app), "unfolded: the Palette panel in front, the record's");
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("factions"), "{text}");
    assert!(!text.contains("\"palettes\""), "no palettes' pane: {text}");
    assert!(!text.contains("\"palette\""), "no palettes' tab: {text}");
    // The palettes' tab in a record whose tab in front is not an area's
    // (the Faction Editor, docked in the middle): the dock unfolded on the
    // Palette but not asked for by name, so not drawn beside the Faction
    // Editor; the area's tab in front, it is, on the Palette.
    fold_palette(h.state_mut());
    settle(&mut h);
    h.state_mut().settings.window_places = vec![("factions".into(), "middle".into())];
    let record = format!(
        r#"{{"panes":[
        {{"place":"middle","tabs":["area:{area}","factions"],"active":"factions"}},
        {{"place":"palettes","tabs":["palette"]}}
    ],"front":"factions"}}"#
    );
    std::fs::write(&file, record).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path));
    settle(&mut h);
    let factions = h.state().dock.find_tab(&Tab::Factions).expect("the Faction Editor open");
    assert!(factions.surface.is_main(), "docked in the middle");
    assert_eq!(h.state().active_editor(), Some(&Tab::Factions));
    assert!(h.state().right_dock_rect().is_none(), "no Right Dock beside the Faction Editor");
    assert!(h.query_by_label("Show the Right Dock").is_none(), "nor its strip");
    assert!(h.state().settings.hide_palettes, "the builder's fold stands");
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Palette));
    settle(&mut h);
    click_tab(&mut h, &Tab::Area(area));
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Palette), "beside the area");
}

/// A module's open tabs kept by an older Moonglow with the resource
/// browser's tab open (`resources`, before the browser was the Left
/// Dock's Resources panel): the module opens with the dock on that panel
/// and the record's other tabs where they were; kept again, the record
/// has no such tab. A record with that tab alone shows the panel too. The
/// record never unfolds the dock: folded by the width or by the builder,
/// it stays folded, on the panel.
#[test]
fn an_old_record_with_the_resource_browser_shows_the_panel() {
    let (path, app, kept) = tabs_app("ui-open-tabs-resources");
    drop(app);
    std::fs::create_dir_all(&kept).unwrap();
    let file = mg_ui::open_tabs::file(&kept, &path);
    let write = |tabs: &str| {
        let record = format!(r#"{{"panes":[{{"place":"middle","tabs":[{tabs}]}}]}}"#);
        std::fs::write(&file, record).unwrap();
    };
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    write(r#""script:hello.nss","resources""#);
    let mut h = tabs_session(&path, &kept);
    // (A window wide enough that no width folds a dock: the test reads the
    // docks as the builder left them, M3 T14.)
    wide_window(&mut h);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources));
    assert!(h.state().dock.find_tab(&Tab::Script(hello)).is_some(), "its other tab");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("script:hello.nss"), "{text}");
    assert!(!text.contains("\"resources\""), "never kept again: {text}");
    // The tab alone.
    write(r#""resources""#);
    let mut h = tabs_session(&path, &kept);
    wide_window(&mut h);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources));
    drop(h);
    // The record puts the panel in front and writes no fold of the dock
    // (M3 T14's second review): in a window that folds it by the width
    // (1400), it stays its strip; in a wide one with the builder's fold,
    // the fold stands, and View › Left Dock unfolds it on the panel.
    write(r#""resources""#);
    let mut h = tabs_session(&path, &kept);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), None, "still folded by the width");
    h.get_by_label("Show the Left Dock");
    drop(h);
    write(r#""resources""#);
    let mut app = app_with(Vec::new());
    show_palette(&mut app);
    app.open_tabs_dir = Some(kept.to_path_buf());
    app.settings.hide_tree = true;
    let mut h = harness(app, 1600.0, 900.0);
    h.state_mut().open_module(&path);
    settle(&mut h);
    assert!(h.state().settings.hide_tree, "the builder's fold stands");
    assert_eq!(h.state().left_dock_shows(), None, "folded");
    menu_command(&mut h, "View", "Left Dock");
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources), "on the panel");
}

/// An older record with the resource browser's tab in front of a pane
/// split off the middle (and the record's tab in front): the panel shown,
/// the split's other tab in its split and the middle's tab in the middle,
/// as before; kept again, the record has no such tab and nothing of it.
#[test]
fn an_old_record_with_the_browser_in_front_of_a_split_shows_the_panel() {
    let (path, app, kept) = tabs_app("ui-open-tabs-resources-split");
    drop(app);
    std::fs::create_dir_all(&kept).unwrap();
    let file = mg_ui::open_tabs::file(&kept, &path);
    let record = r#"{"panes":[
        {"place":"middle","tabs":["script:hello.nss"]},
        {"place":"split left 300","tabs":["resources","script:bye.nss"],"active":"resources"}
    ],"front":"resources"}"#;
    std::fs::write(&file, record).unwrap();
    let mut h = tabs_session(&path, &kept);
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources));
    let script = |n: &str| Tab::Script(ResKey::parse(n, ResType::NSS).unwrap());
    let pane_of = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let found = h.state().dock.iter_leaves().find(|(_, l)| l.tabs.contains(tab));
        found.map(|(p, l)| (p.surface.is_main(), l.rect.center().x, l.tabs.clone()))
    };
    let (main, x, tabs) = pane_of(&h, &script("bye")).expect("the split's other tab");
    assert!(main && tabs == [script("bye")], "in a pane of its own: {tabs:?}");
    let (_, middle, _) = pane_of(&h, &script("hello")).expect("the middle's tab");
    assert!(x < middle, "split off the middle's left");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("script:bye.nss") && text.contains("script:hello.nss"), "{text}");
    assert!(!text.contains("\"resources\""), "never kept again: {text}");
}

/// An older record with the resource browser's tab in a window of its own
/// with a stored size, beside another tab: the panel shown, the other tab
/// in its window as before, the window's size kept for none but it; kept
/// again, nothing of the browser is written, in the record or the
/// settings' window places and sizes.
#[test]
fn an_old_record_with_the_browser_in_a_window_shows_the_panel() {
    let (path, app, kept) = tabs_app("ui-open-tabs-resources-window");
    drop(app);
    std::fs::create_dir_all(&kept).unwrap();
    let file = mg_ui::open_tabs::file(&kept, &path);
    let record = r#"{"panes":[
        {"place":"middle","tabs":["script:hello.nss"]},
        {"place":"window","tabs":["resources","script:bye.nss"],"active":"resources",
         "rect":[200.0,150.0,700.0,500.0]},
        {"place":"window","tabs":["resources"],"rect":[300.0,200.0,920.0,700.0]}
    ]}"#;
    std::fs::write(&file, record).unwrap();
    let mut h = tabs_session(&path, &kept);
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources));
    let script = |n: &str| Tab::Script(ResKey::parse(n, ResType::NSS).unwrap());
    let windows: Vec<Vec<Tab>> = (h.state().dock.iter_leaves())
        .filter(|(p, _)| !p.surface.is_main())
        .map(|(_, l)| l.tabs.clone())
        .collect();
    assert_eq!(windows, [vec![script("bye")]], "the other tab's window, and no empty one");
    assert!(h.state().dock.find_tab(&script("hello")).is_some());
    let s = &h.state().settings;
    assert!(!s.window_sizes.iter().any(|(k, _)| k == "resources"), "{:?}", s.window_sizes);
    assert!(!s.window_places.iter().any(|(k, _)| k == "resources"), "{:?}", s.window_places);
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    let s = h.state().settings.clone();
    drop(h);
    assert!(!s.window_sizes.iter().any(|(k, _)| k == "resources"), "{:?}", s.window_sizes);
    assert!(!s.window_places.iter().any(|(k, _)| k == "resources"), "{:?}", s.window_places);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("script:bye.nss"), "{text}");
    assert!(!text.contains("\"resources\""), "never kept again: {text}");
    assert_eq!(text.matches("\"window\"").count(), 1, "one window kept: {text}");
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
    // (The Right Dock beside the tabs, as Tools › Palettes shows it: with
    // no game data its Palette panel says so. The tabs come back with
    // the dock shown, as they did with the palettes' pane.)
    show_palette(&mut app);
    app.open_tabs_dir = Some(kept.to_path_buf());
    let mut h = harness(app, 1400.0, 900.0);
    h.state_mut().open_module(path);
    h.run_steps(3);
    assert!(h.state().right_dock_rect().is_some(), "the Right Dock beside the tabs");
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

/// What the side editors, a model view and Advanced Search showed opens
/// again with them: a hak's selected resources, a 2DA's selected cell, a
/// tileset's page and tile, a model view's camera, Advanced Search's
/// conditions (two, of two kinds, kept to placeables).
#[test]
fn what_side_editors_a_model_and_a_search_showed_opens_again() {
    use mg_module::hak_edit::{Hak, Source};
    let (path, app, kept) = tabs_app("ui-open-tabs-views-more");
    let dir = path.parent().unwrap().to_path_buf();
    let hak_path = dir.join("stuff.hak");
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    let other = ResKey::parse("other", ResType::TWODA).unwrap();
    let mut hak = Hak::new();
    let bytes = || Source::Bytes(SMALL_TABLE.to_vec().into());
    hak.add_sources(vec![(key, bytes()), (other, bytes())]).unwrap();
    hak.save(Some(&hak_path)).unwrap();
    let table_path = dir.join("loose.2da");
    std::fs::write(&table_path, SMALL_TABLE).unwrap();
    let set_path = dir.join("zzz01.set");
    std::fs::write(&set_path, mg_set::edit::skeleton("zzz01").text()).unwrap();
    let model = ResKey::parse("mg_box", ResType::MDL).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(model, b"not a model".to_vec());
    m.save().unwrap();
    let mut h = harness(app, 1400.0, 900.0);
    h.state_mut().open_module(&path);
    h.run_steps(2);
    {
        let app = h.state_mut();
        let hak = mg_ui::hak_view::open_at(app, &hak_path).unwrap();
        app.haks.iter_mut().find(|d| d.id == hak).unwrap().selected = [other].into();
        let table = mg_ui::table_view::open_path(app, &table_path).unwrap();
        app.tables.iter_mut().find(|d| d.id == table).unwrap().go_to(1, 1);
        mg_ui::tileset_view::open_path(app, set_path.clone());
        let set = app.tilesets.last_mut().unwrap();
        set.change(|f| _ = mg_set::edit::add_tile(f, "zzz01_a01_01", ""));
        set.change(|f| _ = mg_set::edit::add_tile(f, "zzz01_a02_01", ""));
        set.save().unwrap();
        set.page = mg_ui::tileset_view::Page::Tiles;
        set.tile = Some(1);
        let s = &mut app.advanced_search;
        s.first.name = "mg_quest*".into();
        s.and.push(mg_ui::search::Condition {
            kind: mg_ui::search::Kind::Class,
            row: Some(4),
            levels: (3, 5),
            ..Default::default()
        });
        s.object = Some(ResType::UTP);
        app.actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
        app.actions.push(mg_ui::Action::OpenTab(Tab::Model(model)));
    }
    h.run_steps(4);
    let orbit = mg_ui::area_view::Orbit {
        target: glam::Vec3::new(1.0, 2.0, 3.0),
        yaw: 0.5,
        pitch: 0.25,
        distance: 7.0,
    };
    let source = mg_ui::model_view::Source::Resource(model);
    h.state_mut().model_views.get_mut(&source).expect("the model's view").orbit = orbit;
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let mut h = tabs_session(&path, &kept);
    let app = h.state();
    let hak = app.haks.iter().find(|d| d.hak.path.as_deref() == Some(&hak_path)).unwrap();
    assert_eq!(hak.selected.iter().copied().collect::<Vec<_>>(), [other]);
    let table =
        app.tables.iter().find(|d| d.home == mg_ui::table_view::Home::File(table_path.clone()));
    assert_eq!(table.unwrap().selected, Some((1, 1)));
    let set = app.tilesets.iter().find(|d| d.path == set_path).unwrap();
    assert_eq!((set.page, set.tile), (mg_ui::tileset_view::Page::Tiles, Some(1)));
    let s = &app.advanced_search;
    assert_eq!(s.first.name, "mg_quest*");
    assert_eq!(s.and.len(), 1);
    assert_eq!(
        (s.and[0].kind, s.and[0].row, s.and[0].levels),
        (mg_ui::search::Kind::Class, Some(4), (3, 5))
    );
    assert_eq!(s.object, Some(ResType::UTP));
    // The model's camera, once its tab is drawn.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Model(model)));
    h.run_steps(3);
    let view = h.state().model_views.get(&source).expect("the model's view again");
    assert_eq!(view.orbit, orbit);
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
    // Line 150 bookmarked (F5), and numbered 3 (Ctrl+Shift+3).
    h.ctx.memory_mut(|m| m.request_focus(egui::Id::new(("script", script))));
    h.run();
    h.key_press(egui::Key::F5);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Num3);
    h.run_steps(2);
    let marks = |h: &Harness<'_, Moonglow>| {
        let app = h.state();
        (app.script_bookmarks(script), app.script_numbered_bookmarks(script))
    };
    let marked = marks(&h);
    assert_eq!(marked.0, [150], "bookmarked: {marked:?}");
    assert_eq!(marked.1.len(), 1, "numbered: {marked:?}");
    assert_eq!(cursor(&h), (at, at), "the keys left the cursor where it was");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let mut h = tabs_session(&path, &kept);
    h.run_steps(2);
    assert!((editor(&h).top() - scrolled).abs() < 1.0, "{} {scrolled}", editor(&h).top());
    assert_eq!(cursor(&h), (at, at));
    assert_eq!(marks(&h), marked, "its bookmarks as they were");
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
    let mut h = window_harness(app);
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
    let mut h = window_harness(app);
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

/// A session kept by Moonglow 3.1.0 or before, which wrote a placed
/// object's path its own way (`Creature List#1`), opens with its tabs as
/// before, the object found again by what was kept of it, and is written
/// anew in the one form paths are written in (`/Creature List[1]`).
#[test]
fn a_session_kept_as_3_1_0_kept_it_opens_and_is_written_anew() {
    let (path, kept) = placed_module("ui-open-tabs-old-paths");
    let area = ResRef::from_str("start").unwrap();
    let at = |i| Tab::Instance { area, path: mg_edit::GffPath::root().item("Creature List", i) };
    let mut h = tabs_session(&path, &kept);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(at(1)));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let file = mg_ui::open_tabs::file(&kept, &path);
    let new = std::fs::read_to_string(&file).unwrap();
    assert!(new.contains("instance:start:/Creature List[1]"), "{new}");
    // As 3.1.0 wrote it (its tab, its view and its object's key).
    let old = new.replace("/Creature List[1]", "Creature List#1");
    assert!(!old.contains("List["), "{old}");
    std::fs::write(&file, &old).unwrap();
    // Another creature put before it, as above: found by what was kept.
    let mut m = Module::open(&path).unwrap();
    set_creatures(&mut m, &[("C", 5.0), ("A", 10.0), ("B", 20.0)]);
    m.save().unwrap();
    let mut h = tabs_session(&path, &kept);
    assert!(h.state().dock.find_tab(&at(2)).is_some(), "B's, found again");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    let again = std::fs::read_to_string(&file).unwrap();
    assert!(again.contains("instance:start:/Creature List[2]"), "{again}");
    assert!(!again.contains("List#"), "written anew: {again}");
}

/// A module opened from its recovery copy (Recover Unsaved Work, after a
/// crash) opens with the tabs it had.
#[test]
fn a_recovered_module_opens_with_its_tabs() {
    let (path, app, kept) = tabs_app("ui-open-tabs-recovered");
    let recovery = path.parent().unwrap().join("recovery");
    let hello = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    {
        let mut h = window_harness(app);
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
    // (The Right Dock beside the tabs, as Tools › Palettes shows it: with
    // no game data its Palette panel says so. The tabs come back with
    // the dock shown, as they did with the palettes' pane.)
    show_palette(&mut app);
    app.open_tabs_dir = Some(kept);
    app.recovery_dir = Some(recovery);
    app.find_recoveries();
    let mut h = window_harness(app);
    h.run();
    h.get_by_label("Recover").click();
    h.run_steps(3);
    assert!(h.state().ws.as_ref().unwrap().is_modified(), "recovered");
    assert!(h.state().dock.find_tab(&hello).is_some(), "with its tabs");
    assert!(h.state().dock.find_tab(&Tab::Factions).is_some());
    assert!(h.state().right_dock_rect().is_some(), "the Right Dock beside them");
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
    // (The area's harness 1160 wide: 200 more.)
    h.set_size(egui::vec2(AREA_HARNESS_WIDTH + 200.0, 950.0));
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

/// El Grillo's report: Module Properties, docked in the main pane as a
/// module opens, "ejected" (its tab dragged out into a window of its own)
/// was maximized neither by a double click on the window's bar nor by its
/// Maximize button. Moonglow followed a window by the tab it opened it
/// for, and a window egui_dock makes of a tab dragged out had nothing to
/// follow. Dragged out (egui_dock's `detach_tab`, as its drag does), its
/// window is maximized by its button and put back, and by a double click
/// on its bar; and a window Moonglow opened (Area Properties) still is.
#[test]
fn a_tab_dragged_out_into_its_own_window_is_maximized() {
    let Some((mut h, area)) = area_harness("window-maximize-dragged-out") else { return };
    let (area_props, module_props) = (Tab::AreaProperties(area), Tab::ModuleProperties);
    let pane = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let s = h.state();
        let path = s.dock.find_tab(tab).expect("open");
        assert!(!path.surface.is_main(), "in a window");
        s.dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1.rect
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(area_props.clone()));
    h.run_steps(8);
    let path = h.state().dock.find_tab(&module_props).expect("docked as the module opened");
    assert!(path.surface.is_main(), "in the main pane");
    // Dragged out, to the right of Area Properties' window (as far as the
    // screen lets it).
    let beside = pane(&h, &area_props).right() + 20.0;
    let rect = egui::Rect::from_min_size(egui::pos2(beside, 120.0), egui::vec2(340.0, 300.0));
    h.state_mut().dock.detach_tab(path, rect);
    h.run_steps(8);
    // Each window's own Maximize button: the one at its bar's right end.
    let button = |h: &Harness<'_, Moonglow>, tab: &Tab, label: &str| {
        let bar = pane(h, tab);
        (h.get_all_by_label(label))
            .filter(|n| (n.rect().center().x - bar.right()).abs() < 60.0)
            .filter(|n| n.rect().center().y >= bar.top())
            .min_by(|a, b| {
                let d = |n: &egui_kittest::Node<'_>| (n.rect().center().y - bar.top()).abs();
                d(a).total_cmp(&d(b))
            })
            .map(|n| n.rect().center())
            .unwrap_or_else(|| panic!("{label} on {tab:?}'s window at {bar:?}"))
    };
    let click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2, times: usize| {
        h.hover_at(at);
        for _ in 0..times {
            press(h, at, true, egui::Modifiers::NONE);
            press(h, at, false, egui::Modifiers::NONE);
        }
        h.run_steps(8);
        // (Later: not a third click of the same.)
        h.hover_at(at + egui::vec2(0.0, 200.0));
        h.run_steps(30);
    };
    let maximized_by = |h: &mut Harness<'_, Moonglow>, tab: &Tab| {
        let before = pane(h, tab);
        let at = button(h, tab, "Maximize");
        click(h, at, 1);
        let big = pane(h, tab);
        assert!(big.area() > before.area() + 1000.0, "{tab:?} maximized: {big:?} from {before:?}");
        let at = button(h, tab, "Restore");
        click(h, at, 1);
        let back = pane(h, tab);
        assert!(
            (back.size() - before.size()).length() < 3.0 && (back.min - before.min).length() < 3.0,
            "{tab:?} put back: {back:?}, {before:?}"
        );
    };
    // The one dragged out (in front, as the last window made): its button,
    // then a double click on its bar, between its tab and the button (the
    // window is narrow).
    maximized_by(&mut h, &module_props);
    let before = pane(&h, &module_props);
    let at = button(&h, &module_props, "Maximize") - egui::vec2(30.0, 0.0);
    click(&mut h, at, 2);
    let big = pane(&h, &module_props);
    assert!(big.area() > before.area() + 1000.0, "by a double click: {big:?} from {before:?}");
    let at = button(&h, &module_props, "Restore") - egui::vec2(30.0, 0.0);
    click(&mut h, at, 2);
    assert!((pane(&h, &module_props).size() - before.size()).length() < 3.0, "and back");
    // Area Properties' window, which Moonglow opened (it covers the other
    // as it comes to the front).
    maximized_by(&mut h, &area_props);
}

/// Escape closes what is in front: the Variables window over a Properties
/// window first (which a click on Properties doesn't bury), then the
/// Properties window itself.
#[test]
fn escape_closes_the_window_in_front() {
    escape_in_front(Mode::Floating);
}

/// [`escape_closes_the_window_in_front`] where editors open as tabs (a new
/// installation's): the Properties a tab over the area, Escape its Cancel
/// there too (ruling R1), once the Variables window over it is closed.
#[test]
fn escape_closes_the_window_in_front_as_tabs() {
    escape_in_front(Mode::Tabs);
}

fn escape_in_front(mode: Mode) {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness(&mode.dir("escape")) else { return };
    put_in(h.state_mut(), mode);
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
    assert_eq!(h.state().dock.find_tab(&tab).unwrap().surface.is_main(), mode.tabs());
    let props = h
        .state()
        .dock
        .iter_leaves()
        .find(|(_, l)| l.tabs.contains(&tab))
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
/// a window of its own. (With editors opening As Floating Windows, an
/// existing user's settings; As Tabs they are tabs of the focused group:
/// `open_editors::properties_gather_into_one_window_in_floating_mode_only`.)
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
/// already), or, with the option off, each in a window of its own. (With
/// editors opening As Floating Windows, an existing user's settings.)
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

/// The lower halves of the script, conversation and journal editors
/// (the compiler's pane, the selection's fields) are dragged up only so
/// far: three fifths of the script editor, seven tenths of the others,
/// the text or the lines above keeping their room.
#[test]
fn the_editors_lower_halves_are_held_to_a_share_of_the_editor() {
    let dir = mg_testkit::scratch_dir("ui-lower-halves");
    let path = sample_module(&dir);
    let script = ResKey::parse("held", ResType::NSS).unwrap();
    let talk = ResKey::parse("held", ResType::DLG).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(script, b"void main()\n{\n}\n".to_vec());
    let mut g = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "Hello there.");
    m.set_gff(talk, &g).unwrap();
    m.save().unwrap();
    for (tab, panel, share) in [
        (Tab::Script(script), egui::Id::new(("script-info", script)), 0.6),
        (Tab::Dialog(talk), egui::Id::new(("dlg-data", talk)), 0.7),
        (Tab::Journal, egui::Id::new("journal-fields"), 0.7),
    ] {
        let mut app = app_with(Vec::new());
        app.settings.no_last_area = true;
        app.open_module(&path);
        app.actions.push(mg_ui::Action::OpenTab(tab.clone()));
        let mut h = harness(app, 1200.0, 900.0);
        h.run_steps(4);
        let rect = |h: &Harness<'_, Moonglow>| {
            egui::containers::panel::PanelState::load(&h.ctx, panel)
                .unwrap_or_else(|| panic!("{tab:?}: no lower half"))
                .outer_rect
        };
        // (Dragged by its top edge.)
        let drag = |h: &mut Harness<'_, Moonglow>, by: f32| {
            let r = rect(h);
            let from = egui::pos2(r.center().x, r.top() + 2.0);
            h.hover_at(from);
            h.run_steps(1);
            press(h, from, true, egui::Modifiers::NONE);
            h.run_steps(1);
            for k in 1..=6 {
                h.hover_at(from + egui::vec2(0.0, by * k as f32 / 6.0));
                h.run_steps(1);
            }
            press(h, from + egui::vec2(0.0, by), false, egui::Modifiers::NONE);
            h.run_steps(3);
        };
        let before = rect(&h).height();
        drag(&mut h, -60.0);
        let dragged = rect(&h).height();
        assert!((dragged - before - 60.0).abs() < 3.0, "{tab:?}: dragged {before} to {dragged}");
        drag(&mut h, -2000.0);
        let highest = rect(&h).height();
        let editor = h
            .state()
            .dock
            .iter_leaves()
            .map(|(_, l)| l)
            .find(|l| l.tabs.contains(&tab))
            .map(|l| l.rect.height())
            .unwrap_or_else(|| panic!("{tab:?}: not open"));
        assert!(highest > dragged + 50.0, "{tab:?}: taller still, {dragged} to {highest}");
        assert!(highest <= editor * share + 1.0, "{tab:?}: {highest} of {editor}, held to {share}");
        drag(&mut h, -300.0);
        assert!((rect(&h).height() - highest).abs() < 1.0, "{tab:?}: no higher");
    }
}

/// GitHub issue 38: a properties window's foot (its OK and Cancel, and
/// its right edge) went up and down every frame at some sizes, until the
/// window was resized. Each window with OK and Cancel, at sizes and
/// display scales where they moved, is drawn a few frames: OK stays where
/// it is.
#[test]
fn a_properties_window_s_buttons_stay_put() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-buttons-put");
    let path = sample_module(&dir);
    // (A waypoint's blueprint, as the reporter's waypoint was.)
    let mut m = Module::open(&path).unwrap();
    let post = ResKey::parse("post", ResType::UTW).unwrap();
    m.set(post, Gff::new(*b"UTW ").to_bytes().unwrap());
    let talk = ResKey::parse("talk", ResType::DLG).unwrap();
    m.set(talk, Gff::new(*b"DLG ").to_bytes().unwrap());
    m.save().unwrap();
    let area = ResRef::from_str("start").unwrap();
    let mut moving = Vec::new();
    const SIZES: [(f32, f32); 6] = [
        (400.0, 434.8),
        (676.8, 530.7),
        (728.7, 380.0),
        (728.7, 654.0),
        (746.0, 475.9),
        (746.0, 612.9),
    ];
    for scale in [1.0, 1.25, 1.5] {
        let mut app = game_app(&root, None);
        app.open_module(&path);
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 800.0))
            .with_pixels_per_point(scale)
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(8);
        // (The editors of files of their own: a talk table and a new hak.)
        let mut words = mg_module::talk::Table::create("words", &dir, Language::ENGLISH, false);
        words.source = mg_module::talk::Source::File(dir.join("words.tlk"));
        h.state_mut().talk = Some(words);
        h.state_mut().talk_view.outside = true;
        mg_ui::hak_view::new_hak(h.state_mut());
        h.run_steps(4);
        let hak = Tab::Hak(h.state().haks[0].id);
        let file = dir.join("costs.2da");
        std::fs::write(&file, "2DA V2.0\n\n\tLabel\tCost\n0\tBread\t1\n").unwrap();
        let table = Tab::Table(mg_ui::table_view::open_path(h.state_mut(), &file).unwrap());
        let set = dir.join(format!("zzz{}.set", (scale * 100.0) as u32));
        h.state_mut().dialogs = Box::new(NoDialogs { save: vec![set], ..Default::default() });
        mg_ui::tileset_view::new_tileset(h.state_mut());
        h.run_steps(4);
        let tileset = Tab::Tileset(h.state().tilesets[0].id);
        let are = ResKey::new(area, ResType::ARE);
        for tab in [
            Tab::AreaProperties(area),
            Tab::ModuleProperties,
            Tab::Blueprint(post),
            Tab::Gff(are),
            Tab::Factions,
            Tab::Journal,
            Tab::Dialog(talk),
            Tab::TalkTable,
            hak,
            table,
            tileset,
        ] {
            h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
            h.run_steps(8);
            // (A window of its own, as a builder has it.)
            let path = h.state().dock.find_tab(&tab).unwrap();
            if h.state_mut().dock.get_window_state_mut(path.surface).is_none() {
                let rect =
                    egui::Rect::from_min_size(egui::pos2(60.0, 40.0), egui::vec2(600.0, 500.0));
                h.state_mut().dock.detach_tab(path, rect);
                h.run_steps(8);
            }
            // (Sizes where every one of them moved at 125%, from a sweep
            // of 504 sizes at scales 100% to 200%: 124 moved at 125%, none
            // at another scale.)
            for (w, ht) in SIZES {
                let size = egui::vec2(w, ht);
                let path = h.state().dock.find_tab(&tab).unwrap();
                let state = h.state_mut().dock.get_window_state_mut(path.surface).unwrap();
                state.set_size(size);
                h.run_steps(3);
                let mut at = Vec::new();
                for _ in 0..4 {
                    h.run_steps(1);
                    at.push(h.get_all_by_label("OK").last().unwrap().rect());
                }
                if at.windows(2).any(|p| p[0] != p[1]) {
                    moving.push(format!("{tab:?} x{scale} at {size:?}: {at:?}"));
                }
            }
            close_tab(h.state_mut(), &tab);
            h.run_steps(3);
        }
    }
    assert!(moving.is_empty(), "{} sizes:\n{}", moving.len(), moving.join("\n"));
}

/// The Variables window and String Edit kept their object by its place
/// in its list and wrote there at OK, whatever stood there by then: an
/// object deleted before theirs (or put back by Undo) while they were
/// open had the next object's variables replaced (the review of November
/// 2026, S3 and B6). They follow their object as its Properties window
/// does, and close when it is gone.
#[test]
fn the_variables_window_follows_its_object_through_the_list() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_gff::{Struct, Value};
    use mg_ui::widgets::{FieldTarget, LocStringEdit, VarTableEdit};
    let dir = mg_testkit::scratch_dir("ui-variables-follow");
    let path = sample_module(&dir);
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    let mut m = Module::open(&path).unwrap();
    let mut g = Gff::new(*b"GIT ");
    let placed = |tag: &str| {
        let mut s = Struct::new(9);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s.set("VarTable", Value::List(Vec::new()));
        s
    };
    g.root.set("Placeable List", Value::List(["A", "B", "C", "D"].map(placed).to_vec()));
    m.set(git, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = window_harness(app);
    h.run();
    let at = |i: usize| GffPath::root().item("Placeable List", i);
    // Variables on C, String Edit on D.
    h.state_mut().var_edit = Some(VarTableEdit::new(FieldTarget::new(git, at(2), "VarTable"), &[]));
    h.state_mut().loc_edit = Some(LocStringEdit::new(
        FieldTarget::new(git, at(3), "LocName"),
        "Name",
        &LocString::default(),
    ));
    h.run();
    let held = |h: &Harness<'_, Moonglow>| {
        let app = h.state();
        (
            app.var_edit.as_ref().map(|w| w.target.path.clone()),
            app.loc_edit.as_ref().map(|w| w.target.path.clone()),
        )
    };
    let remove = |i: usize| {
        let e = Edit::RemoveItem {
            key: git,
            path: GffPath::root(),
            list: "Placeable List".into(),
            index: i,
        };
        mg_ui::Action::Apply(Command::new("Delete", vec![e]))
    };
    // A deleted: C is the second now, D the third.
    h.state_mut().actions.push(remove(0));
    h.run();
    assert_eq!(held(&h), (Some(at(1)), Some(at(2))));
    // Undo puts A back: they are where they were.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(held(&h), (Some(at(2)), Some(at(3))));
    // C itself deleted: its window closes; D's follows.
    h.state_mut().actions.push(remove(2));
    h.run();
    assert_eq!(held(&h), (None, Some(at(2))));
}

/// Windows that hold a place in the module stayed open when it was
/// closed, and their OK was applied to the next module opened, where it
/// has a resource of that name (the review of November 2026, B7 and S3).
#[test]
fn closing_a_module_closes_the_windows_that_held_its_places() {
    use mg_edit::GffPath;
    use mg_ui::widgets::{FieldTarget, LocStringEdit, VarTableEdit};
    let dir = mg_testkit::scratch_dir("ui-close-windows");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = window_harness(app);
    h.run();
    let info = Module::info_key();
    h.state_mut().var_edit =
        Some(VarTableEdit::new(FieldTarget::new(info, GffPath::root(), "VarTable"), &[]));
    h.state_mut().loc_edit = Some(LocStringEdit::new(
        FieldTarget::new(info, GffPath::root(), "Mod_Name"),
        "Name",
        &LocString::default(),
    ));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    let app = h.state();
    assert!(app.ws.is_none());
    assert!(app.var_edit.is_none() && app.loc_edit.is_none());
}

/// Which tabs show a resource was written three times (Rename, Delete,
/// files gone outside Moonglow) beside `Tab::tree_keys`, and the lists
/// differed: an area deleted outside left its objects' windows open, and
/// none of them knew an Edit Together tab, which kept a blueprint deleted
/// or renamed (the review of November 2026, D30). All three ask the tab.
#[test]
fn tabs_follow_a_rename_and_close_with_what_they_show() {
    use mg_edit::{Command, Edit, GffPath};
    let dir = project_folder("ui-tabs-of-resources", &[]);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    let blueprint = |name: &str| ResKey::parse(name, ResType::UTP).unwrap();
    let (a, b, c) = (blueprint("mg_a"), blueprint("mg_b"), blueprint("mg_c"));
    let made = [a, b]
        .map(|key| Edit::SetResource { key, data: Some(Gff::new(*b"UTP ").to_bytes().unwrap()) });
    app.ws.as_mut().unwrap().apply(Command::new("Blueprints", made.to_vec())).unwrap();
    let area = ResRef::from_str("start").unwrap();
    let object = Tab::Instance { area, path: GffPath::root().item("Placeable List", 0) };
    let has = |app: &Moonglow, tab: &Tab| app.dock.find_tab(tab).is_some();
    // Renamed: an Edit Together tab that holds it holds it by its new name.
    app.run(mg_ui::Action::OpenTab(Tab::Blueprints(vec![a, b])));
    app.rename_resource(a, c.resref, false);
    assert!(has(&app, &Tab::Blueprints(vec![c, b])), "{:?}", app.log.entries);
    // Deleted: the tab that holds it closes, as its own tab would.
    app.run(mg_ui::Action::OpenTab(Tab::Blueprint(b)));
    app.run(mg_ui::Action::DeleteResources(vec![b]));
    assert!(!has(&app, &Tab::Blueprints(vec![c, b])) && !has(&app, &Tab::Blueprint(b)));
    // An area's file gone outside Moonglow: its tabs close, its objects'
    // windows with them.
    app.run(mg_ui::Action::OpenTab(object.clone()));
    app.run(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
    app.run(mg_ui::Action::OpenTab(Tab::Journal));
    assert!(has(&app, &object));
    std::fs::remove_file(dir.join("start.are")).unwrap();
    app.reload_project_files();
    assert!(!app.ws.as_ref().unwrap().module.contains(&ResKey::new(area, ResType::ARE)));
    assert!(!has(&app, &object), "an object's window of an area that is gone");
    assert!(!has(&app, &Tab::AreaProperties(area)));
    assert!(has(&app, &Tab::Journal), "the journal's stays");
}

/// The other windows that hold a placed object by its place in its list
/// (the review's B6): the Appearance Gallery opened for one object, the
/// Levelup Wizard, Setup Store and Add Popup Text. Each gave another
/// object what was meant for its own once one before it was deleted
/// behind the window; they follow it now, and close when it is gone.
#[test]
fn the_wizards_on_a_placed_object_follow_it_through_its_list() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_gff::{Struct, Value};
    let dir = mg_testkit::scratch_dir("ui-wizards-follow");
    let path = sample_module(&dir);
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    let area = git.resref;
    let mut m = Module::open(&path).unwrap();
    let mut g = Gff::new(*b"GIT ");
    let placed = |tag: &str| {
        let mut s = Struct::new(9);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s
    };
    g.root.set("Placeable List", Value::List(["A", "B", "C", "D"].map(placed).to_vec()));
    g.root.set("Creature List", Value::List(["a", "b", "c", "d"].map(placed).to_vec()));
    m.set(git, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = window_harness(app);
    h.run();
    let at = |list: &str, i: usize| GffPath::root().item(list, i);
    {
        let app = h.state_mut();
        // The gallery for placeable C, Add Popup Text on D; the Levelup
        // Wizard on creature c, Setup Store on d.
        let mut gallery = mg_ui::appearance_gallery::Gallery::default();
        gallery.target = Some((git, at("Placeable List", 2)));
        app.placeable_gallery = Some(gallery);
        app.popup_text = Some(mg_ui::store_wizard::PopupText::of(app, area, 3));
        app.levelup = Some(mg_ui::levelup_view::LevelupWizard::new(
            git,
            at("Creature List", 2),
            &Struct::new(0),
        ));
        mg_ui::store_wizard::open(app, area, "Creature List", 3);
        assert!(app.store_wizard.is_some());
    }
    h.run();
    type Held = (Option<GffPath>, Option<usize>, Option<GffPath>, Option<usize>);
    let held = |h: &Harness<'_, Moonglow>| -> Held {
        let app = h.state();
        (
            app.placeable_gallery.as_ref().and_then(|g| g.target.clone()).map(|t| t.1),
            app.popup_text.as_ref().map(|w| w.placeable),
            app.levelup.as_ref().map(|w| w.path.clone()),
            app.store_wizard.as_ref().map(|w| w.creature),
        )
    };
    let remove = |list: &str, i: usize| {
        let e = Edit::RemoveItem { key: git, path: GffPath::root(), list: list.into(), index: i };
        mg_ui::Action::Apply(Command::new("Delete", vec![e]))
    };
    // The first of each list deleted: each window's object is one up.
    h.state_mut().actions.push(remove("Placeable List", 0));
    h.state_mut().actions.push(remove("Creature List", 0));
    h.run();
    let up = (Some(at("Placeable List", 1)), Some(2), Some(at("Creature List", 1)), Some(2));
    assert_eq!(held(&h), up);
    // Undo puts the creature back, then the placeable.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    let first = (Some(at("Placeable List", 2)), Some(3), Some(at("Creature List", 2)), Some(3));
    assert_eq!(held(&h), first);
    // The objects themselves deleted, last first: the gallery is no longer
    // one object's, and the wizards close.
    for (list, i) in
        [("Placeable List", 3), ("Placeable List", 2), ("Creature List", 3), ("Creature List", 2)]
    {
        h.state_mut().actions.push(remove(list, i));
        h.run();
    }
    assert_eq!(held(&h), (None, None, None, None));
    assert!(h.state().placeable_gallery.is_some(), "the gallery itself stays");
}

/// A module's open-tabs record from before the theme (no `rects_scaled`):
/// its window opens as much taller as its kind's windows are now (a
/// script's 8%), once; the record written again says so, and read again
/// the window is as tall, not taller; and the window moved, its kind's
/// remembered size is the scaled height, not the record's old one.
#[test]
fn an_old_record_s_window_opens_taller_once() {
    let (path, app, kept) = tabs_app("ui-open-tabs-scaled");
    drop(app);
    std::fs::create_dir_all(&kept).unwrap();
    let file = mg_ui::open_tabs::file(&kept, &path);
    let record = r#"{"panes":[
        {"place":"middle","tabs":["module-properties"]},
        {"place":"window","tabs":["script:hello.nss"],"rect":[300.0,120.0,700.0,500.0]}
    ]}"#;
    std::fs::write(&file, record).unwrap();
    let hello = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    let window_height = |h: &Harness<'_, Moonglow>| {
        let path = h.state().dock.find_tab(&hello).expect("open");
        assert!(!path.surface.is_main(), "in its window");
        let leaf = h.state().dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1;
        // (The window's height: its pane's and the frame round it.)
        leaf.rect.height() + 2.0
    };
    let mut h = tabs_session(&path, &kept);
    settle(&mut h);
    let first = window_height(&h);
    assert!((first - 540.0).abs() < 2.0, "500 made 540: {first}");
    // Moved by its bar: its kind's size is the scaled one.
    let bar = {
        let path = h.state().dock.find_tab(&hello).unwrap();
        let leaf = h.state().dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1;
        egui::pos2(leaf.rect.right() - 80.0, leaf.rect.top() + 10.0)
    };
    h.hover_at(bar);
    h.run_steps(1);
    press(&h, bar, true, egui::Modifiers::NONE);
    for k in 1..=5 {
        h.hover_at(bar + egui::vec2(8.0 * k as f32, 6.0 * k as f32));
        h.run_steps(1); // (One step a pointer move.)
    }
    press(&h, bar + egui::vec2(40.0, 30.0), false, egui::Modifiers::NONE);
    settle(&mut h);
    let size = h.state().settings.window_sizes.iter().find(|(k, _)| k == "script").map(|(_, s)| *s);
    let size = size.expect("the script's size kept as it moved");
    assert!((size[1] as f32 - 540.0).abs() < 2.0, "the scaled height kept: {size:?}");
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    drop(h);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"rects_scaled\": true"), "{text}");
    let mut h = tabs_session(&path, &kept);
    settle(&mut h);
    let again = window_height(&h);
    assert!((again - first).abs() < 2.0, "not taller again: {again} after {first}");
}
