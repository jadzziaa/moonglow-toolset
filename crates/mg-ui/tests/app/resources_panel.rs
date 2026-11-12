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

/// The same settings as JSON (serde's other format; no settings file is
/// read as JSON today, the desktop app's is RON): read and given to the
/// app as it starts, the browser's place and size are dropped there too,
/// as `Moonglow::with_settings` runs the migration after any reading.
#[test]
fn json_settings_with_a_resources_place_open_without_it() {
    let json = r#"{
        "window_places": [["resources", "split below 330"], ["script", "middle"]],
        "window_sizes": [["resources", [900, 600]], ["dialog", [900, 680]]]
    }"#;
    let read: mg_ui::Settings = serde_json::from_str(json).unwrap();
    assert_eq!(read.window_places.len(), 2, "read as it is");
    let app = Moonglow::with_settings(read, Box::new(NoDialogs::default()));
    assert_eq!(app.settings.window_places, [("script".to_string(), "middle".to_string())]);
    assert_eq!(app.settings.window_sizes, [("dialog".to_string(), [900, 680])]);
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
    // (A kind kept in the palettes' pane, place `palettes`, is dropped
    // too since the palettes are the Right Dock's panel, M3 T5.)
    assert_eq!(places, &[("script".to_string(), "middle".to_string())]);
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

/// Chapter1 open (a tree long enough to scroll) with the game's data, in
/// a window of the narrow board's size; the area not opened.
fn chapter1_harness() -> Option<Harness<'static, Moonglow>> {
    let root = mg_testkit::nwn_root()?;
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = harness(app, 1280.0, 800.0);
    settle(&mut h);
    Some(h)
}

/// What a list shows: each row in sight, its words and its top.
type Place = dyn Fn(&Harness<'_, Moonglow>) -> Vec<(String, i32)>;

/// The mouse wheel turned `by` points down over `at`, and the frames run
/// until what `place` reads stays still (the scroll is animated).
fn wheel(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, by: f32, place: &Place) {
    h.hover_at(at);
    h.run();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -by),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    still(h, place);
}

/// Frames until what `place` reads is the same four frames in a row.
fn still(h: &mut Harness<'_, Moonglow>, place: &Place) {
    let started = std::time::Instant::now();
    let (mut last, mut same) = (None, 0);
    while same < 4 {
        h.run_steps(1);
        let now = place(h);
        same = if last.as_ref() == Some(&now) { same + 1 } else { 0 };
        last = Some(now);
        assert!(started.elapsed() < SETTLE_TIME, "never at rest");
    }
}

/// The module tree's rows in sight: each one's words and its top.
fn tree_place(h: &Harness<'_, Moonglow>) -> Vec<(String, i32)> {
    let Some(rows) = h.query_by_label("Module Tree Rows") else { return Vec::new() };
    let mut out: Vec<(String, i32)> = rows
        .query_all_by(|n| n.label().is_some() && n.bounding_box().is_some())
        .filter(|n| n.rect().top() > 60.0 && n.rect().bottom() < 780.0)
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect().top().round() as i32))
        .collect();
    out.sort();
    out
}

/// The browser's rows in sight (named by their resources, `name.ext`,
/// inside the Left Dock and under its Name field): each one's name and
/// its top.
fn list_place(h: &Harness<'_, Moonglow>) -> Vec<(String, i32)> {
    let Some(field) = h.query_by_label("Filter Resources") else { return Vec::new() };
    let (top, right) = (field.rect().bottom() + 30.0, field.rect().right() + 10.0);
    let mut out: Vec<(String, i32)> = h
        .query_all_by(|n| {
            n.role() != egui::accesskit::Role::Label && n.label().is_some_and(|l| l.contains('.'))
        })
        .filter(|n| n.rect().top() > top && n.rect().right() < right && n.rect().bottom() < 780.0)
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect().top().round() as i32))
        .collect();
    out.sort();
    out
}

/// Whether the row named `name` is laid out and in sight between `top`
/// and the Status Bar.
fn in_sight(h: &Harness<'_, Moonglow>, name: &str, top: f32) -> bool {
    h.query_all_by_label(name).any(|n| n.rect().top() >= top && n.rect().bottom() <= 780.0)
}

/// The module tree and the browser's list each keep their own scroll
/// place (the review of M3 T4: both were egui's scroll area of the
/// default id in the same panel, so each took the other's place when its
/// tab came back): Chapter1's tree scrolled with a script chosen, the
/// list scrolled far down with a row chosen, and the tabs switched both
/// ways three times: each list is where it was left, its keys' row in
/// sight.
#[test]
fn the_tree_and_the_list_keep_their_own_scroll_places() {
    let Some(mut h) = chapter1_harness() else { return };
    // The tree: the Scripts group opened, scrolled, a script chosen.
    let scripts = |h: &Harness<'_, Moonglow>| {
        let rows = h.get_by_label("Module Tree Rows");
        let found = rows.query_all_by(|n| n.label().is_some_and(|l| l.starts_with("Scripts (")));
        found.min_by(|a, b| a.rect().left().total_cmp(&b.rect().left())).unwrap().rect()
    };
    click_pos(&h, scripts(&h).center());
    settle(&mut h);
    wheel(&mut h, egui::pos2(100.0, 700.0), 400.0, &tree_place);
    let script = tree_place(&h)
        .into_iter()
        .filter(|(l, t)| !l.contains('(') && (300..600).contains(t))
        .map(|(l, _)| l)
        .next()
        .expect("a script in sight");
    tree_row(&h, &script).click();
    settle(&mut h);
    still(&mut h, &tree_place);
    let tree_was = tree_place(&h);
    assert!(!tree_was.is_empty());
    // The list: scrolled far down, a row chosen.
    dock_tab(&h, "Resources").click();
    settle(&mut h);
    wheel(&mut h, egui::pos2(100.0, 600.0), 5000.0, &list_place);
    let rows = list_place(&h);
    let (row, _) = rows.iter().find(|(_, t)| (400..600).contains(t)).expect("a row in sight");
    let row = row.clone();
    h.get_by_label(&row).click();
    settle(&mut h);
    still(&mut h, &list_place);
    let list_was = list_place(&h);
    assert!(h.state().browser.cursor.is_some_and(|c| c > 100), "far down the list");
    let field = h.get_by_label("Filter Resources").rect().bottom();
    for round in 0..3 {
        dock_tab(&h, "Module").click();
        settle(&mut h);
        still(&mut h, &tree_place);
        assert_eq!(tree_place(&h), tree_was, "round {round}: the tree where it was");
        assert!(in_sight(&h, &script, 60.0), "round {round}: its keys' row {script} in sight");
        dock_tab(&h, "Resources").click();
        settle(&mut h);
        still(&mut h, &list_place);
        assert_eq!(list_place(&h), list_was, "round {round}: the list where it was");
        assert!(in_sight(&h, &row, field), "round {round}: its keys' row {row} in sight");
    }
}

/// The Left Dock's tabs have one form (their words, or their icons alone)
/// and one place at each width of the dock, whichever panel is in front
/// (the review of M3 T4: from 198 to 240 points, the Module panel's four
/// tools gave icons and the Resources panel's two gave words, so a click
/// on a tab changed the bar under the pointer): from 190 to 300 points.
/// At 220, a double-click on the other tab ends on it, the tab still
/// under the pointer.
#[test]
fn the_dock_s_tabs_keep_their_form_and_place_whichever_panel_is_in_front() {
    let Some(mut h) = browser_harness("ui-panel-tabs-form", 1280.0, 800.0) else { return };
    let tabs =
        |h: &Harness<'_, Moonglow>| (dock_tab(h, "Module").rect(), dock_tab(h, "Resources").rect());
    for width in 190..=300 {
        left_dock_width(&h, width as f32);
        let mut each = Vec::new();
        for panel in [Panel::Module, Panel::Resources] {
            h.state_mut().actions.push(mg_ui::Action::ShowPanel(panel));
            settle(&mut h);
            assert_eq!(h.state().left_dock_shows(), Some(panel));
            each.push(tabs(&h));
        }
        assert_eq!(each[0], each[1], "{width}: the tabs as they were on the other panel");
    }
    left_dock_width(&h, 220.0);
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(Panel::Module));
    settle(&mut h);
    for (name, panel) in [("Resources", Panel::Resources), ("Module", Panel::Module)] {
        // Two clicks a frame apart, then the two of a double-click in one.
        let at = dock_tab(&h, name).rect().center();
        click_pos(&h, at);
        h.run();
        click_pos(&h, at);
        settle(&mut h);
        assert_eq!(h.state().left_dock_shows(), Some(panel), "{name}: two clicks");
        assert!(dock_tab(&h, name).rect().contains(at), "{name}: still under the pointer");
        let other = if panel == Panel::Module { Panel::Resources } else { Panel::Module };
        h.state_mut().actions.push(mg_ui::Action::ShowPanel(other));
        settle(&mut h);
        double_click_now(&mut h, at);
        settle(&mut h);
        assert_eq!(h.state().left_dock_shows(), Some(panel), "{name}: a double-click");
        assert!(dock_tab(&h, name).rect().contains(at), "{name}: still under the pointer");
    }
}

/// The Left Dock's right edge: its panel's, as egui keeps it under the
/// panel's id (`"tree"`).
fn dock_right(h: &Harness<'_, Moonglow>) -> f32 {
    let id = egui::Id::new("tree");
    let state = h.ctx.data_mut(|d| d.get_persisted::<egui::containers::panel::PanelState>(id));
    state.expect("the dock drawn").outer_rect.right()
}

/// A press on a list's scroll bar at the fold button's height, dragged.
fn drag_bar(h: &mut Harness<'_, Moonglow>, by: f32) {
    let fold = h.get_by_label("Fold the Left Dock").rect();
    // (The scroll bar runs along the list's right, a few points in from
    // the dock's edge.)
    let at = egui::pos2(dock_right(h) - 6.0, fold.center().y);
    drag(h, at, at + egui::vec2(0.0, by));
    settle(h);
}

/// The fold button stands beside the Left Dock's inner edge, on the
/// editor area's side, never over the dock's lists or their scroll bars
/// (the review of M3 T4: across the edge it covered the browser's scroll
/// bar but 2 points of it, and a press there folded the dock): a press on
/// a list's scroll bar at the button's height, dragged, scrolls the list
/// and leaves the dock open: the browser's list, the module tree's Areas
/// well (every group open), and the tree itself (scrolled past the well).
/// The edge still drags above and below the button.
#[test]
fn a_press_on_a_list_s_scroll_bar_by_the_fold_button_scrolls_it() {
    let Some(mut h) = chapter1_harness() else { return };
    show_resources(h.state_mut());
    settle(&mut h);
    let fold = h.get_by_label("Fold the Left Dock").rect();
    assert!(fold.left() >= dock_right(&h) - 0.01, "beside the dock: {fold:?} {}", dock_right(&h));
    // The browser's list.
    let before = list_place(&h);
    drag_bar(&mut h, 120.0);
    still(&mut h, &list_place);
    assert!(!h.state().settings.hide_tree, "the dock left open");
    assert_ne!(list_place(&h), before, "the list scrolled");
    // The tree's Areas well, at the button's height with every group open.
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(Panel::Module));
    settle(&mut h);
    h.get_by_label("Left Dock").get_by_label("Expand All").click();
    settle(&mut h);
    still(&mut h, &tree_place);
    let before = tree_place(&h);
    drag_bar(&mut h, 60.0);
    still(&mut h, &tree_place);
    assert!(!h.state().settings.hide_tree, "the dock left open");
    assert_ne!(tree_place(&h), before, "the well scrolled");
    // The tree, scrolled to its foot (past the well), its own bar.
    wheel(&mut h, egui::pos2(100.0, 770.0), 50_000.0, &tree_place);
    let before = tree_place(&h);
    drag_bar(&mut h, -120.0);
    still(&mut h, &tree_place);
    assert!(!h.state().settings.hide_tree, "the dock left open");
    assert_ne!(tree_place(&h), before, "the tree scrolled");
    // The edge drags above and below the button.
    for y in [fold.top() - 12.0, fold.bottom() + 12.0] {
        let was = dock_right(&h);
        drag(&mut h, egui::pos2(was - 1.0, y), egui::pos2(was + 29.0, y));
        settle(&mut h);
        assert!(
            (dock_right(&h) - was - 30.0).abs() < 2.0,
            "dragged at {y}: {was} {}",
            dock_right(&h)
        );
    }
    assert!(!h.state().settings.hide_tree);
}

/// The keyboard reaches the fold button after the Left Dock's own
/// controls, before the editor area's (the review of M3 T4: drawn after
/// the editor area, it came after the whole of it); Enter there folds the
/// dock, and the button that unfolds it is reached the same way.
#[test]
fn tab_reaches_the_fold_button_after_the_dock_s_controls() {
    let Some(mut h) = browser_harness("ui-panel-fold-tab", 1280.0, 800.0) else { return };
    let right = dock_right(&h);
    // From the Module tab, Tab by Tab: each control in the dock until the
    // fold button.
    dock_tab(&h, "Module").focus();
    h.run_steps(2);
    let fold_is_focused = |h: &Harness<'_, Moonglow>| {
        h.query_by_label("Fold the Left Dock").is_some_and(|n| n.accesskit_node().is_focused())
    };
    let mut steps = 0;
    while !fold_is_focused(&h) {
        h.key_press(egui::Key::Tab);
        h.run_steps(2);
        steps += 1;
        assert!(steps < 120, "never reached the fold button");
        let focused = h.ctx.memory(|m| m.focused()).and_then(|id| h.ctx.read_response(id));
        // (The dock's own: its controls, and the edge it is resized by,
        // which egui's panel makes after them.)
        if let Some(r) = focused.filter(|_| !fold_is_focused(&h)) {
            assert!(r.rect.left() <= right, "{steps}: left the dock first: {:?}", r.rect);
        }
    }
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().settings.hide_tree, "Enter folds it");
    // Folded: from the strip's last button, the one that unfolds it.
    h.get_by_label("Left Dock").get_by_label("Resources").focus();
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(2);
    let show = h.get_by_label("Show the Left Dock");
    assert!(show.accesskit_node().is_focused(), "after the strip's buttons");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(!h.state().settings.hide_tree, "Enter unfolds it");
}

/// Left and Right on a focused tab of the Left Dock go to the other tab
/// (the panel shown with it) and leave the tree's keys alone.
#[test]
fn the_arrow_keys_on_a_dock_tab_choose_the_other_tab() {
    let Some(mut h) = browser_harness("ui-panel-tab-arrows", 1280.0, 800.0) else { return };
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Module));
    let cursor = h.state().tree_cursor;
    dock_tab(&h, "Module").focus();
    h.run_steps(2);
    for (key, panel, name) in [
        (egui::Key::ArrowRight, Panel::Resources, "Resources"),
        (egui::Key::ArrowRight, Panel::Module, "Module"),
        (egui::Key::ArrowLeft, Panel::Resources, "Resources"),
    ] {
        h.key_press(key);
        settle(&mut h);
        assert_eq!(h.state().left_dock_shows(), Some(panel), "{key:?}");
        assert!(selected(&h, name) && dock_tab(&h, name).accesskit_node().is_focused());
        assert_eq!(h.state().tree_cursor, cursor, "the tree's keys left alone");
    }
}

/// With no module open, the Left Dock showing the Resources panel alone
/// folds and unfolds by View › Left Dock and its key (Ctrl+Alt+1), and F6
/// gives the arrow keys to its list (the review of M3 T4: the row was
/// disabled and the keys did nothing, while the fold button's tip named
/// the key).
#[test]
fn without_a_module_the_view_row_its_key_and_f6_reach_the_resources_panel() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let mut h = harness(game_app(&root, None), 1280.0, 800.0);
    settle(&mut h);
    menu_command(&mut h, "Tools", "Resource Browser");
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources));
    h.get_by_label("Top Bar").get_by_label("View").click();
    settle(&mut h);
    let row = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::CheckBox && n.label().as_deref() == Some("Left Dock")
    };
    let node = h.get(egui_kittest::kittest::by().predicate(row));
    assert!(!node.accesskit_node().is_disabled(), "View › Left Dock enabled");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    menu_command(&mut h, "View", "Left Dock");
    assert!(h.state().settings.hide_tree, "folded by the row");
    menu_command(&mut h, "View", "Left Dock");
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources), "and back");
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    h.key_press_modifiers(keys, egui::Key::Num1);
    settle(&mut h);
    assert!(h.state().settings.hide_tree, "folded by its key");
    h.key_press_modifiers(keys, egui::Key::Num1);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources));
    // F6: the list's keys (its first row).
    h.state_mut().browser.filter = "nw_it_gem00".into();
    settle(&mut h);
    assert_eq!(h.state().browser.cursor, None);
    let mut steps = 0;
    while h.state().browser.cursor.is_none() {
        h.key_press(egui::Key::F6);
        settle(&mut h);
        steps += 1;
        assert!(steps < 5, "F6 never reached the list");
    }
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(h.state().browser.cursor, Some(1), "the arrows the list's");
}

/// View › Module Tree is View › Left Dock now that the dock holds the
/// Resources panel too (its tip says so), with its id and so its key as
/// before: with Resources in front it folds the dock, and again unfolds
/// it on Resources; a key given it in Options › Keyboard (kept by its id)
/// does the same; the Command Palette finds it by its new name; Tools ›
/// Resource Browser still shows the panel.
#[test]
fn view_left_dock_folds_the_dock_on_whichever_panel() {
    let Some(mut h) = browser_harness("ui-panel-view-row", 1280.0, 800.0) else { return };
    show_resources(h.state_mut());
    settle(&mut h);
    assert_eq!(menu_row_tip(&mut h), "Show the Left Dock, or fold it away");
    menu_command(&mut h, "View", "Left Dock");
    assert!(h.state().settings.hide_tree);
    menu_command(&mut h, "View", "Left Dock");
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources), "back on Resources");
    // A key of the user's own, kept under the row's id.
    let key = egui::KeyboardShortcut::new(
        egui::Modifiers::COMMAND | egui::Modifiers::ALT,
        egui::Key::F11,
    );
    h.state_mut().keymap.set_of("view-tree", vec![key]);
    h.key_press_modifiers(key.modifiers, key.logical_key);
    settle(&mut h);
    assert!(h.state().settings.hide_tree, "the user's key folds it");
    h.key_press_modifiers(key.modifiers, key.logical_key);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources));
    // The Command Palette.
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
    h.run();
    type_into_hint(&mut h, "Type a command's name", "left dock");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().settings.hide_tree, "run from the Command Palette");
    menu_command(&mut h, "Tools", "Resource Browser");
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Resources));
}

/// The tip of View › Left Dock (the View menu opened, its row hovered).
fn menu_row_tip(h: &mut Harness<'_, Moonglow>) -> String {
    h.get_by_label("Top Bar").get_by_label("View").click();
    settle(h);
    let rect = menu_row(h, "Left Dock").expect("the row").1;
    let tip = tip_at(h, rect).expect("a tip");
    h.key_press(egui::Key::Escape);
    settle(h);
    tip
}

/// Export's tip says how many resources it writes and what it does with
/// them, or, disabled, what to do first (the review of M3 T4: the count
/// was only in its accessible name): the whole list, "Narrow the list";
/// a list narrowed, the count; one resource, said singular; nothing
/// listed, so.
#[test]
fn export_s_tip_says_how_many_it_writes() {
    let Some(mut h) = browser_harness("ui-panel-export-tip", 1280.0, 800.0) else { return };
    show_resources(h.state_mut());
    settle(&mut h);
    let tip = |h: &mut Harness<'_, Moonglow>| {
        let r = export(h).rect();
        tip_at(h, r).unwrap_or_default()
    };
    assert!(export(&h).accesskit_node().is_disabled());
    assert_eq!(tip(&mut h), "Narrow the list to export it");
    h.state_mut().browser.filter = "nw_it_gem00".into();
    settle(&mut h);
    let name = export(&h).accesskit_node().label().unwrap();
    let n: usize = name.split(' ').nth(1).unwrap().parse().unwrap();
    assert!(n > 1, "{name}");
    assert_eq!(tip(&mut h), format!("Write the {n} listed resources into a folder as files"));
    h.state_mut().browser.filter = "nw_it_torch001".into();
    settle(&mut h);
    assert_eq!(export(&h).accesskit_node().label().as_deref(), Some("Export 1 as Files…"));
    assert_eq!(tip(&mut h), "Write the 1 listed resource into a folder as a file");
    h.state_mut().browser.filter = "zzzqqqxxx".into();
    settle(&mut h);
    assert!(export(&h).accesskit_node().is_disabled());
    assert_eq!(tip(&mut h), "Nothing is listed to export");
}

/// The words each row of the browser's list paints inside the Left Dock,
/// under its Name field, by the row's top: the name first (monospace),
/// then the layer where there is one.
fn painted_rows(h: &Harness<'_, Moonglow>) -> Vec<Vec<String>> {
    let field = h.get_by_label("Filter Resources").rect();
    let right = dock_right(h);
    let mut texts: Vec<(i32, f32, String)> = drawn(h)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => Some(t),
            _ => None,
        })
        .filter(|t| t.pos.y > field.bottom() + 30.0 && t.pos.x < right && t.pos.y < 760.0)
        .map(|t| {
            (
                (t.pos.y + t.galley.size().y / 2.0).round() as i32,
                t.pos.x,
                t.galley.text().to_string(),
            )
        })
        .collect();
    texts.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap());
    let mut rows: Vec<(i32, Vec<String>)> = Vec::new();
    for (y, _, text) in texts {
        match rows.last_mut() {
            Some((at, words)) if (*at - y).abs() <= 2 => words.push(text),
            _ => rows.push((y, vec![text])),
        }
    }
    // (The rows: each starts with its name, which keeps its extension.)
    rows.into_iter().map(|(_, w)| w).filter(|w| w[0].contains('.')).collect()
}

/// At the dock's least width the rows can be told apart (the review of
/// M3 T4: four rows read "nw_ch_srch…", the layer keeping 45% of the
/// room): the name comes first, cut in its middle where it must be, its
/// extension kept, and the layer is left out where less than a stub of
/// it would be (the row's tip has both); at 260 the names are whole, and
/// the layer is shown in what they leave.
#[test]
fn at_the_least_width_the_rows_are_told_apart_by_their_names() {
    let Some(mut h) = browser_harness("ui-panel-row-names", 1280.0, 800.0) else { return };
    show_resources(h.state_mut());
    h.state_mut().browser.filter = "nw_ch_s".into();
    left_dock_width(&h, 190.0);
    settle(&mut h);
    let rows = painted_rows(&h);
    assert!(rows.len() > 8, "{rows:?}");
    let names: Vec<&String> = rows.iter().map(|r| &r[0]).collect();
    let mut apart = names.clone();
    apart.sort();
    apart.dedup();
    assert_eq!(apart.len(), names.len(), "every row's name its own: {names:?}");
    for row in &rows {
        assert!(row[0].ends_with(".ncs") || row[0].ends_with(".nss"), "{row:?}: its extension");
        assert!(row.len() == 1 || row[1].chars().count() >= 5, "{row:?}: no layer, or a stub");
    }
    assert!(names.iter().all(|n| !n.ends_with('…')), "never at the end: {names:?}");
    // The row's tip: its whole name and layer.
    let first = h.get_all_by_label_contains("nw_ch_s").next().unwrap();
    let (whole, rect) = (first.accesskit_node().label().unwrap(), first.rect());
    let tip = tip_at(&mut h, rect).unwrap_or_default();
    assert!(tip.contains(&format!("{whole} from key:")), "{tip}");
    // At 260: the names whole, each row's layer beside it.
    left_dock_width(&h, 260.0);
    settle(&mut h);
    let rows = painted_rows(&h);
    for row in &rows {
        assert!(!row[0].contains('…'), "{row:?}: the name whole");
        assert_eq!(row.len(), 2, "{row:?}: its layer");
    }
}

/// A name longer than its row at the least width (a script of sixteen
/// letters, the longest a name has, overridden by the module's copy and
/// listed set in under it) is cut in its middle, its start, end and
/// extension in sight; the module's own copy, not set in, is whole.
#[test]
fn a_name_longer_than_its_row_is_cut_in_its_middle() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let game = game_data(&root);
    let long = (game.resman.every_copy().into_iter())
        .map(|(key, _)| key)
        .find(|k| k.restype == ResType::NSS && k.resref.to_string().len() == 16)
        .expect("a script of sixteen letters");
    drop(game);
    let dir = mg_testkit::scratch_dir("ui-panel-long-name");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    m.set(long, b"void main() { }\n".to_vec());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&path);
    show_resources(&mut app);
    app.browser.filter = long.resref.to_string();
    app.browser.restype = Some(ResType::NSS);
    let mut h = harness(app, 1280.0, 800.0);
    left_dock_width(&h, 190.0);
    settle(&mut h);
    let rows = painted_rows(&h);
    let whole = long.to_string();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0][0], whole, "the module's copy whole: {rows:?}");
    let cut = &rows[1][0];
    let (head, tail) = cut.split_once('…').expect("the copy's name cut");
    assert!(whole.starts_with(head) && whole.ends_with(tail) && tail.ends_with(".nss"), "{cut}");
    assert!(head.len() >= 4 && tail.len() >= 6, "{cut}: its start and end in sight");
}

/// A window of the editor area moved over the Left Dock's edge is over
/// its fold button too, which is over the editor area beside the dock but
/// under the windows (a layer of its own just over the window's panels):
/// a click there is the window's, and the dock stays open.
#[test]
fn a_window_over_the_fold_button_takes_its_clicks() {
    let Some(mut h) = browser_harness("ui-panel-fold-under-window", 1280.0, 800.0) else {
        return;
    };
    let fold = h.get_by_label("Fold the Left Dock").rect();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    {
        let dock = &mut h.state_mut().dock;
        let found = dock.find_tab(&Tab::Factions).unwrap();
        let tab = dock.remove_tab(found).unwrap();
        let surface = dock.add_window(vec![tab]);
        let at = egui::pos2(fold.left() - 100.0, fold.top() - 150.0);
        let window = dock.get_window_state_mut(surface).unwrap();
        window.set_position(at).set_size(egui::vec2(400.0, 400.0));
    }
    settle(&mut h);
    let layer = h.ctx.layer_id_at(fold.center()).expect("a layer there");
    assert_eq!(layer.order, egui::Order::Middle, "the window's, over the button");
    click_pos(&h, fold.center());
    settle(&mut h);
    assert!(!h.state().settings.hide_tree, "the click the window's");
    // Without the window, the click is the button's.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Factions]));
    settle(&mut h);
    click_pos(&h, fold.center());
    settle(&mut h);
    assert!(h.state().settings.hide_tree, "the button's");
}
