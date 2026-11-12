//! The Left Dock's Resources panel (the redesign's M3 T4): the resource
//! browser as the dock's second tab, its header's Export and Refresh, its
//! list at the dock's widths, its keys, and the settings and open-tabs
//! records of the browser as a tab, made the panel's.

use crate::common::*;
use mg_ui::Panel;

/// The sample module with the game's data, the area not opened, in a
/// window `width` by `height`.
fn browser_harness(name: &str, width: f32, height: f32) -> Option<Harness<'static, Moonglow>> {
    let root = mg_testkit::nwn_root()?;
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, width, height);
    settle(&mut h);
    Some(h)
}

/// The Left Dock's tab `name` (Module, Resources).
fn dock_tab<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    h.get_by_label("Left Dock").get_by_label(name)
}

/// Whether the Left Dock's tab `name` is said selected.
fn selected(h: &Harness<'_, Moonglow>, name: &str) -> bool {
    dock_tab(h, name).accesskit_node().is_selected() == Some(true)
}

/// The Export button of the Resources panel's header, by its name's
/// end ("Export N as Files…").
fn export<'t>(h: &'t Harness<'_, Moonglow>) -> egui_kittest::Node<'t> {
    h.get_by_label("Left Dock").get_by(|n| n.label().is_some_and(|l| l.ends_with("as Files…")))
}

/// Tools › Resource Browser (and a key given to it) shows the Left Dock
/// on its Resources panel, unfolded where it was folded; its tab said
/// selected and the Module tab not, the header the panel's (Export and
/// Refresh, not the Module panel's buttons). Asked again, the panel stays.
#[test]
fn tools_resources_and_its_key_show_the_panel_folded_or_not() {
    let Some(mut h) = browser_harness("ui-panel-shows", 1280.0, 800.0) else { return };
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Module));
    assert!(selected(&h, "Module") && !selected(&h, "Resources"));
    let shown = |h: &Harness<'_, Moonglow>| {
        assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources));
        assert!(selected(h, "Resources") && !selected(h, "Module"));
        h.get_by_label("Filter Resources");
        h.get_by_label("Left Dock").get_by_label("Refresh");
        export(h);
        assert!(h.get_by_label("Left Dock").query_by_label("New Area").is_none());
    };
    menu_command(&mut h, "Tools", "Resource Browser");
    shown(&h);
    menu_command(&mut h, "Tools", "Resource Browser");
    shown(&h);
    // Folded: unfolded on the panel.
    h.get_by_label("Fold the Left Dock").click();
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), None);
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(Panel::Module));
    settle(&mut h);
    h.get_by_label("Fold the Left Dock").click();
    settle(&mut h);
    menu_command(&mut h, "Tools", "Resource Browser");
    shown(&h);
    // Its key (none of its own: given one in Options › Keyboard), folded
    // and not.
    let key = egui::KeyboardShortcut::new(
        egui::Modifiers::COMMAND | egui::Modifiers::ALT,
        egui::Key::F12,
    );
    h.state_mut().keymap.set_of("resources", vec![key]);
    for folded in [true, false] {
        h.state_mut().actions.push(mg_ui::Action::ShowPanel(Panel::Module));
        h.state_mut().settings.hide_tree = folded;
        settle(&mut h);
        h.key_press_modifiers(key.modifiers, key.logical_key);
        settle(&mut h);
        shown(&h);
    }
}

/// The dock's two tabs switch by a click and by the keyboard (Tab to the
/// tab, Enter or Space), each said selected; folded, its strip has both
/// panels' buttons, each unfolding the dock on its panel.
#[test]
fn the_left_dock_s_tabs_switch_by_click_keyboard_and_strip() {
    let Some(mut h) = browser_harness("ui-panel-tabs", 1280.0, 800.0) else { return };
    dock_tab(&h, "Resources").click();
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources));
    assert!(selected(&h, "Resources") && !selected(&h, "Module"));
    dock_tab(&h, "Module").click();
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Module));
    assert!(selected(&h, "Module"));
    for key in [egui::Key::Enter, egui::Key::Space] {
        let (to, panel) = match h.state().left_dock_shows() {
            Some(Panel::Module) => ("Resources", Panel::Resources),
            _ => ("Module", Panel::Module),
        };
        dock_tab(&h, to).focus();
        h.run_steps(2);
        h.key_press(key);
        settle(&mut h);
        assert_eq!(h.state().left_dock_shows(), Some(panel), "{key:?}");
        assert!(selected(&h, to));
    }
    // Folded: a button a panel on the strip.
    h.state_mut().settings.hide_tree = true;
    settle(&mut h);
    for (name, panel) in [("Resources", Panel::Resources), ("Module", Panel::Module)] {
        dock_tab(&h, name).click();
        settle(&mut h);
        assert_eq!(h.state().left_dock_shows(), Some(panel), "{name} from the strip");
        h.state_mut().settings.hide_tree = true;
        settle(&mut h);
    }
}

/// The header's Export is named by how many resources it writes, enabled
/// only with the list narrowed (not the whole game by a slip), and writes
/// them; Refresh reads the load order again (a resource the module was
/// given since is listed).
#[test]
fn the_header_s_export_and_refresh_run() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-panel-header");
    let path = sample_module(&dir);
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let mut app =
        game_app_with(&root, None, NoDialogs { folders: vec![out.clone()], ..Default::default() });
    app.settings.no_last_area = true;
    app.open_module(&path);
    show_resources(&mut app);
    let mut h = harness(app, 1280.0, 800.0);
    settle(&mut h);
    let all = export(&h).accesskit_node().label().unwrap();
    assert!(export(&h).accesskit_node().is_disabled(), "the whole list: {all}");
    h.state_mut().browser.filter = "nw_it_torch00".into();
    settle(&mut h);
    let name = export(&h).accesskit_node().label().unwrap();
    let listed = h.query_all_by_label_contains("nw_it_torch00").count();
    assert_eq!(name, format!("Export {listed} as Files…"));
    assert!(!export(&h).accesskit_node().is_disabled());
    export(&h).click();
    settle(&mut h);
    assert!(out.join("nw_it_torch001.uti").is_file());
    // Refresh: a resource the load order was given behind the browser's
    // back (its index not told) listed once the index is made again.
    let key = ResKey::parse("mg_fresh", ResType::NSS).unwrap();
    {
        let game = std::sync::Arc::get_mut(h.state_mut().game.as_mut().unwrap()).unwrap();
        let layer = game.resman.layer_mut("module").unwrap();
        layer.container.as_mem_mut().unwrap().insert(key, b"void main() { }\n".to_vec());
    }
    h.state_mut().browser.filter = "mg_fresh".into();
    settle(&mut h);
    assert!(h.query_by_label("mg_fresh.nss").is_none(), "the index as it was");
    h.get_by_label("Left Dock").get_by_label("Refresh").click();
    settle(&mut h);
    h.get_by_label("mg_fresh.nss");
}

/// The list's rows at the dock's least width, its default and 500: each
/// row's name and layer inside the dock and side by side (cut with an
/// ellipsis where both do not fit, never past the dock's edge or making
/// it wider), the list as wide as the dock; a cut cell's whole text in
/// the row's tip. (The rows: the overridden copies of portraits.2da, whose
/// layers are the longest, "hak:mg_low, overridden by hak:mg_top".)
#[test]
fn the_list_s_rows_fit_the_dock_at_190_260_and_500() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-panel-widths");
    let user = dir.join("user");
    let path = crate::resources::module_with_two_tables(&dir, &user);
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&path);
    show_resources(&mut app);
    app.browser.filter = "portraits".into();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let mut cut = None;
    for width in [190.0, 260.0, 500.0] {
        left_dock_width(&h, width);
        settle(&mut h);
        // (The dock's left edge: its first tab's.)
        let bar = h.get_by_label("Left Dock").get_by_label("Resources").rect();
        let left = h.get_by_label("Left Dock").get_by_label("Module").rect().left();
        let edge = h.get_by_label("Filter Resources").rect().right();
        assert!(edge - left < width, "{width}: the Name field inside the dock");
        let names: Vec<egui::Rect> =
            h.get_all_by_label("portraits.2da").map(|n| n.rect()).collect();
        assert!(names.len() >= 3, "{width}: the table's copies listed");
        for (i, name) in names.iter().enumerate() {
            let layer = h
                .query_all_by(|n| {
                    let said = n.label().or(n.value()).unwrap_or_default();
                    said.starts_with("hak:") || said.starts_with("key:")
                })
                .map(|n| n.rect())
                .find(|r| (r.center().y - name.center().y).abs() < 2.0)
                .expect("the row's layer");
            assert!(
                name.left() >= left - 0.5 && layer.right() <= left + width + 0.5,
                "{width}: row {i} {name:?} {layer:?}"
            );
            assert!(name.right() <= layer.left() + 0.5, "{width}: row {i} side by side");
            assert!(bar.top() < name.top());
        }
        if width == 190.0 {
            cut = names.get(1).copied();
        }
        // (The dock as wide as asked: the Name field across it.)
        assert!(edge - left > width - 60.0, "{width}: the dock {} wide", edge - left);
    }
    // A cut cell's whole text in its row's tip (back at the least width).
    left_dock_width(&h, 190.0);
    settle(&mut h);
    let row = h.get_all_by_label("portraits.2da").nth(1).unwrap().rect();
    assert!(cut.is_some());
    let tip = tip_at(&mut h, row).expect("a tip");
    assert!(tip.contains("portraits.2da from hak:mg_low, overridden by hak:mg_top"), "{tip}");
}

/// A double-click on a row (its name or its layer) and Enter at the keys'
/// row open the resource read-only in the editor area; a right-click on
/// its layer opens its menu, as on its name.
#[test]
fn double_click_and_enter_open_a_read_only_tab() {
    let Some(mut h) = browser_harness("ui-panel-open", 1280.0, 800.0) else { return };
    show_resources(h.state_mut());
    h.state_mut().browser.filter = "nw_it_torch00".into();
    settle(&mut h);
    let opened = |h: &Harness<'_, Moonglow>| -> Vec<Tab> {
        (h.state().dock.iter_all_tabs())
            .map(|(_, t)| t.clone())
            .filter(|t| matches!(t, Tab::Resource(_)))
            .collect()
    };
    let torch = |n: u8| ResKey::parse(&format!("nw_it_torch00{n}"), ResType::UTI).unwrap();
    let double_click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        double_click_now(h, at);
        settle(h);
    };
    let at = h.get_by_label(&torch(1).to_string()).rect().center();
    double_click(&mut h, at);
    assert_eq!(opened(&h), [Tab::Resource(torch(1))]);
    // On its layer: the same.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Resource(torch(1))]));
    settle(&mut h);
    let name = h.get_by_label(&torch(1).to_string()).rect();
    let layer = h
        .query_all_by_label("key:nwn_base")
        .map(|n| n.rect())
        .find(|r| (r.center().y - name.center().y).abs() < 2.0)
        .expect("its layer");
    double_click(&mut h, layer.center());
    assert_eq!(opened(&h), [Tab::Resource(torch(1))], "double-clicked on its layer");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Resource(torch(1))]));
    settle(&mut h);
    h.event(egui::Event::PointerButton {
        pos: layer.center(),
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: layer.center(),
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    apart(&h, "Open").click();
    settle(&mut h);
    assert_eq!(opened(&h), [Tab::Resource(torch(1))], "Open from its layer's menu");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Resource(torch(1))]));
    settle(&mut h);
    // Enter at the keys' row (Down from the Name: its first).
    h.get_by_label("Filter Resources").click();
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(h.state().browser.cursor, Some(0));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(opened(&h).len(), 1, "{:?}", opened(&h));
}

/// F6 steps the arrow keys into the Resources panel's list (the Left
/// Dock's pane, where the module tree's were) and on out of it; Shift+F6
/// back. With the Module panel shown, the tree again.
#[test]
fn f6_steps_into_and_out_of_the_resources_panel() {
    let Some(mut h) = browser_harness("ui-panel-f6", 1280.0, 800.0) else { return };
    show_resources(h.state_mut());
    // (Rows enough for Down to move the keys' row.)
    h.state_mut().browser.filter = "nw_it_gem00".into();
    settle(&mut h);
    let key = |h: &mut Harness<'_, Moonglow>, m: egui::Modifiers, key: egui::Key| {
        h.key_press_modifiers(m, key);
        settle(h);
    };
    let none = egui::Modifiers::NONE;
    // (Whether the keys are the list's: Down moves its row.)
    let in_list = |h: &mut Harness<'_, Moonglow>| {
        let was = h.state().browser.cursor;
        key(h, none, egui::Key::ArrowDown);
        let now = h.state().browser.cursor;
        if now != was {
            key(h, none, egui::Key::ArrowUp);
        }
        now != was && now.is_some()
    };
    for m in [none, egui::Modifiers::SHIFT] {
        let mut steps = 0;
        loop {
            key(&mut h, m, egui::Key::F6);
            steps += 1;
            if in_list(&mut h) {
                break;
            }
            assert!(steps < 6, "into the list ({m:?})");
        }
        key(&mut h, m, egui::Key::F6);
        assert!(!in_list(&mut h), "out of it ({m:?})");
    }
    // The tab gone from Resources to Module takes the keys with it.
    h.state_mut().browser.cursor = None;
    let mut steps = 0;
    while !in_list(&mut h) {
        key(&mut h, none, egui::Key::F6);
        steps += 1;
        assert!(steps < 6);
    }
    dock_tab(&h, "Module").click();
    settle(&mut h);
    let was = h.state().tree_cursor;
    key(&mut h, none, egui::Key::ArrowDown);
    assert_ne!(h.state().tree_cursor, was, "the tree's keys now");
}

/// What a plugin's command run from the Plugins menu takes as "the pane
/// last clicked in" (`plugins.pane`) is left as it was by a click in the
/// Resources panel, as by a click in the browser's tab before: the module
/// tree's, or an area's.
#[test]
fn a_click_in_the_resources_panel_leaves_the_plugins_pane() {
    let Some((mut h, area)) = area_harness("panel-plugins-pane") else { return };
    // The tree clicked: its pane.
    tree_row(&h, "Module Properties").click();
    settle(&mut h);
    assert_eq!(h.state().plugins.pane, Some(mg_ui::plugins::Pane::Tree));
    show_resources(h.state_mut());
    h.state_mut().browser.filter = "nw_it_torch00".into();
    settle(&mut h);
    h.get_by_label("nw_it_torch001.uti").click();
    settle(&mut h);
    h.get_by_label("Filter Resources").click();
    settle(&mut h);
    assert_eq!(h.state().plugins.pane, Some(mg_ui::plugins::Pane::Tree), "as it was");
    // An area's.
    h.state_mut().plugins.pane = Some(mg_ui::plugins::Pane::Area(area));
    h.get_by_label("nw_it_torch001.uti").click();
    settle(&mut h);
    assert_eq!(h.state().plugins.pane, Some(mg_ui::plugins::Pane::Area(area)), "as it was");
}

/// A settings file of an older Moonglow that kept a place and a size for
/// the resource browser's window (kind `resources`) reads with no warning
/// and no copy put aside; the app opened with it has neither, and every
/// other place and size as it was.
#[test]
fn old_settings_with_a_resources_place_open_without_it() {
    let dir = mg_testkit::scratch_dir("ui-panel-old-settings");
    let old = mg_ui::Settings {
        window_places: vec![
            ("resources".into(), "split below 330".into()),
            ("script".into(), "middle".into()),
            ("factions".into(), "palettes".into()),
        ],
        window_sizes: vec![("resources".into(), [900, 600]), ("script".into(), [1100, 700])],
        ..Default::default()
    };
    let text = old.saved_text();
    let file = dir.join("app.ron");
    let map: std::collections::HashMap<String, String> =
        [("moonglow-settings".to_string(), text)].into_iter().collect();
    std::fs::write(&file, ron::to_string(&map).unwrap()).unwrap();
    let read = mg_ui::Settings::read_eframe(&file, "moonglow-settings");
    assert!(read.unread.is_empty() && read.unreadable.is_none(), "{:?}", read.problem());
    assert!(read.kept.is_none(), "nothing put aside");
    let app = Moonglow::with_settings(read.settings, Box::new(NoDialogs::default()));
    let places = &app.settings.window_places;
    assert_eq!(
        places,
        &[("script".to_string(), "middle".to_string()), ("factions".into(), "palettes".into())]
    );
    assert_eq!(app.settings.window_sizes, [("script".to_string(), [1100, 700])]);
    let said: Vec<_> =
        (app.log.entries.iter()).filter(|(level, _)| *level != mg_ui::Level::Info).collect();
    assert!(said.is_empty(), "no warning: {said:?}");
    let names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(names.len(), 1, "no copy put aside: {names:?}");
}

/// With no module open (the Welcome page), Tools › Resource Browser shows
/// the Left Dock with the Resources panel alone (no Module tab: there is
/// no module tree), its list the game's; with no module and not asked
/// for, the dock is not drawn.
#[test]
fn the_browser_without_a_module_is_the_left_dock_s_one_panel() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let mut h = harness(game_app(&root, None), 1280.0, 800.0);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), None);
    assert!(h.query_by_label("Left Dock").is_none(), "no dock drawn");
    menu_command(&mut h, "Tools", "Resource Browser");
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources));
    assert!(selected(&h, "Resources"));
    assert!(h.get_by_label("Left Dock").query_by_label("Module").is_none(), "no Module tab");
    h.state_mut().browser.filter = "nw_it_torch001".into();
    settle(&mut h);
    h.get_by_label("nw_it_torch001.uti");
}

/// The list lays out the rows in sight alone: of the whole load order
/// (more than a hundred thousand resources, nothing typed in the Name),
/// as many rows are in the accessibility tree as the panel has room for.
#[test]
fn the_list_lays_out_only_the_rows_in_sight() {
    let Some(mut h) = browser_harness("ui-panel-rows-in-sight", 1280.0, 800.0) else { return };
    show_resources(h.state_mut());
    settle(&mut h);
    let count = |v: &str| v.contains(" of ") && v.contains(" resources");
    let all = h.get_by(|n| n.value().is_some_and(|v| count(&v)));
    let said = all.accesskit_node().value().unwrap();
    let listed: usize = said.split(' ').next().unwrap().parse().unwrap();
    assert!(listed > 100_000, "{said}");
    let top = h.get_by_label("Filter Resources").rect().bottom();
    let rows = h
        .query_all_by(|n| n.role() == egui::accesskit::Role::Button)
        .filter(|n| n.rect().top() > top && n.rect().right() < 300.0)
        .count();
    let room = (800.0 - top) / 22.0;
    assert!(rows > 10 && (rows as f32) <= room + 3.0, "{rows} rows laid out, room for {room}");
}
