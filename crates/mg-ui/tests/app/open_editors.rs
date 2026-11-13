//! Options › General › Open editors (M3's T10): editors open as tabs of
//! the focused editor group (a new installation's As Tabs) or as windows
//! of their own (an existing user's As Floating Windows); a kind moved
//! keeps its place, a window among them in As Tabs, until View › Reset
//! Layout; Properties gather into one window in As Floating Windows alone;
//! plugin panels follow the switch (ruling R3); and Escape is the Cancel
//! of a docked tab with OK and Cancel (ruling R1), with each of its limits.
//! Each test runs in both modes ([`MODES`]).

use crate::common::*;

/// The sample module with two waypoint blueprints (`wp_a`, `wp_b`) and
/// three more scripts (`s01` to `s03`).
fn module(dir: &std::path::Path) -> std::path::PathBuf {
    let path = sample_module(dir);
    let mut m = Module::open(&path).unwrap();
    for name in ["wp_a", "wp_b"] {
        let mut g = Gff::new(*b"UTW ");
        g.root.set("Tag", mg_gff::Value::String(name.as_bytes().to_vec()));
        m.set_gff(ResKey::parse(name, ResType::UTW).unwrap(), &g).unwrap();
    }
    for n in 1..=3 {
        let key = ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap();
        m.set(key, b"void main() { }\n".to_vec());
    }
    m.save().unwrap();
    path
}

/// The window in `mode` with [`module`] open, Module Properties docked
/// as the module opened and the area `start` open beside it, `places`
/// kept for kinds.
fn window(mode: Mode, name: &str, places: &[(&str, &str)]) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = module(&dir);
    let mut app = app_in(mode, Vec::new());
    app.settings.no_last_area = true;
    app.settings.window_places =
        places.iter().map(|(k, p)| (k.to_string(), p.to_string())).collect();
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(start()));
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    h
}

fn start() -> Tab {
    Tab::Area(ResRef::from_str("start").unwrap())
}

fn script(n: usize) -> Tab {
    Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap())
}

fn waypoint(name: &str) -> Tab {
    Tab::Blueprint(ResKey::parse(name, ResType::UTW).unwrap())
}

fn open(h: &mut Harness<'_, Moonglow>, tab: &Tab) {
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(h);
}

/// Whether `tab` is open in the main window (an editor group), `None`
/// where it is not open.
fn in_main(h: &Harness<'_, Moonglow>, tab: &Tab) -> Option<bool> {
    h.state().dock.find_tab(tab).map(|p| p.surface.is_main())
}

/// The pane `tab` is in.
fn pane(h: &Harness<'_, Moonglow>, tab: &Tab) -> egui_dock::NodePath {
    let p = h.state().dock.find_tab(tab).unwrap();
    egui_dock::NodePath { surface: p.surface, node: p.node }
}

fn places(h: &Harness<'_, Moonglow>) -> Vec<(String, String)> {
    h.state().settings.window_places.clone()
}

fn reset_layout(h: &mut Harness<'_, Moonglow>) {
    menu_command(h, "View", "Reset Layout");
    settle(h);
}

/// Frames until `done` says so, for ten seconds at most, failing with
/// `what`.
fn until(h: &mut Harness<'_, Moonglow>, what: &str, done: impl Fn(&Harness<'_, Moonglow>) -> bool) {
    let started = std::time::Instant::now();
    while !done(h) {
        assert!(started.elapsed().as_secs() < 10, "never: {what}");
        h.step();
    }
}

/// A script, a Properties (a waypoint's blueprint), the Faction Editor
/// and the user manual open as tabs of the focused editor group in As
/// Tabs, as windows of their own in As Floating Windows; areas as tabs in
/// both; Tile Properties' kin, the model views, in windows in both. In As
/// Tabs, "the focused group" is the one last clicked: a script opens
/// beside another in the group Split Editor made, and beside the area once
/// the area's group is clicked.
#[test]
fn editors_open_as_tabs_or_as_windows_as_the_switch_says() {
    for mode in MODES {
        let mut h = window(mode, "ui-open-editors", &[]);
        assert_eq!(in_main(&h, &start()), Some(true), "{mode:?}: the area");
        for tab in [script(1), waypoint("wp_a"), Tab::Factions, Tab::Manual] {
            open(&mut h, &tab);
            assert_eq!(in_main(&h, &tab), Some(mode.tabs()), "{mode:?}: {tab:?}");
        }
        let model = Tab::Model(ResKey::parse("hello", ResType::NSS).unwrap());
        open(&mut h, &model);
        assert_eq!(in_main(&h, &model), Some(false), "{mode:?}: a model's view, a window");
        if !mode.tabs() {
            continue;
        }
        // The focused group: the one Split Editor made, then the area's.
        // (Split Editor moved a script: scripts are kept going to its
        // group, `split right 500`; other kinds go where the focus is.)
        click_tab(&mut h, &script(1));
        settle(&mut h);
        h.get_by_label("Split Editor").click();
        settle(&mut h);
        assert_ne!(pane(&h, &script(1)), pane(&h, &start()), "a group of its own");
        open(&mut h, &Tab::Journal);
        assert_eq!(pane(&h, &Tab::Journal), pane(&h, &script(1)), "into the focused group");
        click_tab(&mut h, &start());
        settle(&mut h);
        open(&mut h, &waypoint("wp_b"));
        assert_eq!(pane(&h, &waypoint("wp_b")), pane(&h, &start()), "into the area's, focused");
        open(&mut h, &script(2));
        assert_eq!(pane(&h, &script(2)), pane(&h, &script(1)), "scripts where one was moved");
        assert_eq!(places(&h), [("script".to_string(), "split right 500".to_string())]);
    }
}

/// Moved to a window (the tab menu's Eject) in As Tabs, a script's kind
/// is kept going to a window: the next script opens in one, until View ›
/// Reset Layout. In As Floating Windows, the same for a script docked:
/// the next opens docked; ejected again, its place is forgotten (a window
/// is where one opens there), and Reset Layout forgets a docked place.
#[test]
fn a_kind_moved_keeps_its_place_until_reset_layout() {
    let eject = |h: &mut Harness<'_, Moonglow>, name: &str| {
        doc_tab(h, name).click_secondary();
        until(h, "the tab's menu", |h| h.query_by_label("Eject").is_some());
        h.get_by_label("Eject").click();
        settle(h);
    };
    let windowed = vec![("script".to_string(), "window".to_string())];
    let middle = vec![("script".to_string(), "middle".to_string())];
    for mode in MODES {
        let mut h = window(mode, "ui-open-moved", &[]);
        open(&mut h, &script(1));
        if mode.tabs() {
            assert_eq!(in_main(&h, &script(1)), Some(true));
            eject(&mut h, "s01.nss");
            assert_eq!(in_main(&h, &script(1)), Some(false), "ejected");
            assert_eq!(places(&h), windowed, "kept: a window");
            open(&mut h, &script(2));
            assert_eq!(in_main(&h, &script(2)), Some(false), "the next in a window");
            reset_layout(&mut h);
            assert!(places(&h).is_empty(), "forgotten");
            open(&mut h, &script(3));
            assert_eq!(in_main(&h, &script(3)), Some(true), "a tab again");
        } else {
            assert_eq!(in_main(&h, &script(1)), Some(false));
            // Docked into the area's group (as a drag does).
            let middle_pane = pane(&h, &start());
            let dock = &mut h.state_mut().dock;
            let at = dock.find_tab(&script(1)).unwrap();
            dock.remove_tab(at);
            dock.set_focused_node_and_surface(middle_pane);
            dock.push_to_focused_leaf(script(1));
            settle(&mut h);
            assert_eq!(places(&h), middle, "kept: docked");
            open(&mut h, &script(2));
            assert_eq!(in_main(&h, &script(2)), Some(true), "the next docked");
            eject(&mut h, "s02.nss");
            assert!(places(&h).is_empty(), "a window: where one opens anyway");
            open(&mut h, &script(3));
            assert_eq!(in_main(&h, &script(3)), Some(false));
            h.state_mut().settings.window_places = middle.clone();
            reset_layout(&mut h);
            assert!(places(&h).is_empty(), "forgotten");
        }
        assert_eq!(h.state().settings.open_editors, mode.settings().open_editors, "kept");
    }
}

/// In As Tabs, editors live among the groups, and a place is measured
/// from the middle (the areas' group), which moves as groups are split
/// and emptied: a script that did not move keeps no place for its kind
/// when the area beside it is split off or its group closed; one moved by
/// Split Editor does (`split right 500`), and so does one dragged.
#[test]
fn in_tabs_a_kind_s_place_is_kept_for_its_own_moves_alone() {
    let mut h = window(Mode::Tabs, "ui-open-own-moves", &[]);
    open(&mut h, &script(1));
    assert_eq!(pane(&h, &script(1)), pane(&h, &start()));
    // The area split off: the script now left of the middle, not moved.
    click_tab(&mut h, &start());
    settle(&mut h);
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    assert_ne!(pane(&h, &script(1)), pane(&h, &start()));
    assert!(places(&h).is_empty(), "the middle moved, not the script: {:?}", places(&h));
    // The area's group closed: the script's group is the middle now.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![start()]));
    settle(&mut h);
    assert!(places(&h).is_empty(), "{:?}", places(&h));
    // Moved by Split Editor: kept.
    open(&mut h, &script(2));
    click_tab(&mut h, &script(2));
    settle(&mut h);
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    assert_eq!(places(&h), [("script".to_string(), "split right 500".to_string())]);
    // Moved by hand (a drag, here on the dock, noted as the drag notes it)
    // back into the first group: kept there.
    let first = pane(&h, &script(1));
    h.state_mut().moved_by_hand(&script(2));
    let dock = &mut h.state_mut().dock;
    let at = dock.find_tab(&script(2)).unwrap();
    dock.remove_tab(at);
    dock.set_focused_node_and_surface(first);
    dock.push_to_focused_leaf(script(2));
    settle(&mut h);
    assert_eq!(places(&h), [("script".to_string(), "middle".to_string())]);
}

/// A tab dragged by the pointer from one group onto another's tab bar, in
/// As Tabs: a move by hand, its kind's place kept (the drop seen in the
/// frame the button is let go).
#[test]
fn in_tabs_a_tab_dragged_to_another_group_keeps_its_kind_s_place() {
    let mut h = window(Mode::Tabs, "ui-open-dragged", &[]);
    open(&mut h, &script(1));
    open(&mut h, &Tab::Journal);
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    assert_ne!(pane(&h, &Tab::Journal), pane(&h, &script(1)));
    assert_eq!(places(&h), [("journal".to_string(), "split right 500".to_string())]);
    let from = h.state().tab_rect(&script(1)).unwrap().center();
    let onto = h.state().tab_rect(&Tab::Journal).unwrap();
    let to = egui::pos2(onto.right() + 40.0, onto.center().y);
    drag(&mut h, from, to);
    settle(&mut h);
    assert_eq!(pane(&h, &script(1)), pane(&h, &Tab::Journal), "dropped in that group");
    let script_place = ("script".to_string(), "split right 500".to_string());
    assert!(places(&h).contains(&script_place), "{:?}", places(&h));
}

/// Properties open as tabs of one window (Options' "Open Properties as
/// tabs of one window") in As Floating Windows; in As Tabs they are tabs
/// of the focused group, the option for windows alone.
#[test]
fn properties_gather_into_one_window_in_floating_mode_only() {
    for mode in MODES {
        let mut h = window(mode, "ui-open-gather", &[]);
        let (a, b) = (waypoint("wp_a"), waypoint("wp_b"));
        open(&mut h, &a);
        open(&mut h, &b);
        assert_eq!(pane(&h, &a), pane(&h, &b), "{mode:?}: together");
        assert_eq!(in_main(&h, &a), Some(mode.tabs()), "{mode:?}");
        if mode.tabs() {
            assert_eq!(pane(&h, &a), pane(&h, &start()), "the focused group, the area's");
        }
    }
}

/// A plugin's panel (Plugins › Panels) opens as a tab in As Tabs, as a
/// window of its own in As Floating Windows (ruling R3).
#[test]
fn a_plugin_panel_follows_the_switch() {
    for mode in MODES {
        let Some((mut h, _)) = plugin_harness(&mode.dir("ui-open-plugin-panel"), false) else {
            return;
        };
        h.state_mut().settings = mode.settings();
        let examples =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples");
        h.state_mut().plugin_dir = Some(examples);
        h.state_mut().load_plugins();
        h.state_mut().enable_plugin("example.module-overview", true);
        settle(&mut h);
        h.get_by_label("Plugins").click();
        h.run();
        h.get_by_label("Panels").click();
        h.run();
        h.get_by_label("Module Overview").click();
        settle(&mut h);
        let tab =
            Tab::PluginPanel { plugin: "example.module-overview".into(), panel: "overview".into() };
        assert_eq!(in_main(&h, &tab), Some(mode.tabs()), "{mode:?}");
        h.get_by_label("The module");
    }
}

// Escape on a docked tab with OK and Cancel (ruling R1).

fn tag(h: &mut Harness<'_, Moonglow>) -> String {
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    let root = h.state_mut().ws.as_mut().unwrap().doc(&info).unwrap().root.clone();
    String::from_utf8_lossy(root.string("Mod_Tag").unwrap_or_default()).into_owned()
}

/// Module Properties (docked as the module opened, in both modes, and
/// opened there again: its kind kept in the area's group) in front of its
/// group, its tag field clicked and typed in ("SAMPLEX").
fn edit_tag(h: &mut Harness<'_, Moonglow>) {
    if h.state().dock.find_tab(&Tab::ModuleProperties).is_none() {
        open(h, &Tab::ModuleProperties);
    }
    click_tab(h, &Tab::ModuleProperties);
    settle(h);
    let row = h.get_all_by_label("Tag").last().unwrap().rect().center().y;
    let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some("SAMPLE")
    };
    let at = (h.query_all(egui_kittest::kittest::by().predicate(is)))
        .map(|n| n.rect().center())
        .find(|c| (c.y - row).abs() < 8.0)
        .expect("the tag's field");
    h.hover_at(at);
    press(h, at, true, egui::Modifiers::NONE);
    press(h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("X".into()));
    h.run_steps(2);
}

/// A point of Module Properties' body away from its fields (under its
/// pages' list).
fn in_module_properties(h: &Harness<'_, Moonglow>) -> egui::Pos2 {
    let (_, leaf) = (h.state().dock.iter_leaves())
        .find(|(_, l)| l.tabs.contains(&Tab::ModuleProperties))
        .unwrap();
    egui::pos2(leaf.viewport.left() + 40.0, leaf.viewport.bottom() - 80.0)
}

/// A point of the Left Dock (the module tree, below its rows).
fn in_left_dock(h: &Harness<'_, Moonglow>) -> egui::Pos2 {
    let area = h.state().editor_area_rect().unwrap();
    egui::pos2(area.left() - 60.0, area.bottom() - 40.0)
}

fn escape(h: &mut Harness<'_, Moonglow>) {
    h.key_press(egui::Key::Escape);
    settle(h);
}

fn open_tab(h: &Harness<'_, Moonglow>, tab: &Tab) -> bool {
    h.state().dock.find_tab(tab).is_some()
}

/// Escape cancels Module Properties docked in front of its group: with
/// the pointer over it, and with the keyboard in it (typed in, the
/// pointer then on the Left Dock), what was typed set back and the tab
/// closed, as a floating Properties window's Escape does. Not with the
/// pointer over another group, nor with the keyboard in another group.
#[test]
fn escape_cancels_a_docked_tab_with_ok_and_cancel() {
    for mode in MODES {
        let places = [("script", "split right 500"), ("module-properties", "middle")];
        let mut h = window(mode, "ui-open-escape", &places);
        // The pointer over it, typed in.
        edit_tag(&mut h);
        assert_eq!(in_main(&h, &Tab::ModuleProperties), Some(true));
        escape(&mut h);
        assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: closed");
        assert_eq!(tag(&mut h), "SAMPLE", "{mode:?}: set back");
        // The keyboard in it, the pointer on the Left Dock.
        edit_tag(&mut h);
        h.hover_at(in_left_dock(&h));
        h.run_steps(2);
        escape(&mut h);
        assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: closed by the keyboard's");
        assert_eq!(tag(&mut h), "SAMPLE");
        // A script in a group of its own. Typed in Module Properties, the
        // pointer over the script: the keyboard's, Module Properties'.
        open(&mut h, &script(1));
        edit_tag(&mut h);
        let other = h.state().tab_rect(&script(1)).unwrap().center() + egui::vec2(0.0, 120.0);
        h.hover_at(other);
        h.run_steps(2);
        escape(&mut h);
        assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the field's tab's");
        assert_eq!(tag(&mut h), "SAMPLE");
        assert!(open_tab(&h, &script(1)), "the script stays");
        // Typed in, then a click in the script (its text has the keyboard):
        // not Module Properties', nor with the pointer back over it.
        edit_tag(&mut h);
        let is_text = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::MultilineTextInput
                && n.value().is_some_and(|v| v.contains("void main"))
        };
        let text = h.get(egui_kittest::kittest::by().predicate(is_text)).rect().center();
        h.hover_at(text);
        h.run_steps(1);
        press(&h, text, true, egui::Modifiers::NONE);
        press(&h, text, false, egui::Modifiers::NONE);
        h.run_steps(2);
        assert!(h.ctx.memory(|m| m.focused().is_some()), "the script's text has the keyboard");
        let back = in_module_properties(&h);
        h.hover_at(back);
        h.run_steps(2);
        escape(&mut h);
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the keyboard in another group");
        // Nothing with the keyboard, the pointer over another group.
        let dock = in_left_dock(&h);
        click_away(&mut h, dock);
        h.hover_at(other);
        h.run_steps(2);
        escape(&mut h);
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the pointer on another group");
        click_tab(&mut h, &script(1));
        settle(&mut h);
        h.hover_at(in_left_dock(&h));
        h.run_steps(2);
        escape(&mut h);
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the keyboard in another");
        // Nor a tab without OK and Cancel: the script stays.
        h.hover_at(other);
        h.run_steps(2);
        escape(&mut h);
        assert!(open_tab(&h, &script(1)), "{mode:?}: a script's tab is not closed");
    }
}

/// Escape gives way: to an open menu (it closes the menu, the tab stays;
/// a second Escape is the tab's), to Quick Open, to the module tree's
/// Filter (it clears what was typed), to the log holding the arrow keys
/// (it hands them back), to a drag in progress (a button held), and to a window in front with OK and Cancel (a waypoint's
/// Properties, floating: the window's, the docked tab's at the next).
#[test]
fn escape_on_a_docked_tab_gives_way() {
    for mode in MODES {
        let places = [("waypoint", "window"), ("module-properties", "middle")];
        let mut h = window(mode, "ui-open-escape-way", &places);
        edit_tag(&mut h);
        let over = in_module_properties(&h);
        // An open menu.
        h.get_by_label("View").click();
        h.run();
        assert!(h.query_by_label("Reset Layout").is_some(), "the menu open");
        escape(&mut h);
        assert!(h.query_by_label("Reset Layout").is_none(), "{mode:?}: the menu closed");
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: and only it");
        // Quick Open.
        h.hover_at(over);
        h.run_steps(2);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::P);
        settle(&mut h);
        assert!(h.state().quick_open.is_open(), "Quick Open open");
        escape(&mut h);
        assert!(!h.state().quick_open.is_open());
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: Quick Open's Escape");
        // The module tree's Filter.
        type_into_tree_filter(&mut h, "zz");
        h.hover_at(over);
        h.run_steps(2);
        escape(&mut h);
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the Filter's Escape");
        assert!(h.query_all_by_value("zz").next().is_none(), "the Filter cleared");
        // The log's own Escape: clicked in, it has the arrow keys, and
        // Escape hands them back (drawn before the editor area).
        h.state_mut().settings.hide_log = false;
        settle(&mut h);
        let log = h.state().bottom_dock_rect().unwrap();
        let in_log = egui::pos2(log.center().x, log.bottom() - 20.0);
        click_away(&mut h, in_log);
        h.hover_at(over);
        h.run_steps(2);
        escape(&mut h);
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the log's Escape");
        // (Module Properties shorter above the log now.)
        let over = in_module_properties(&h);
        // A drag: the button held over it.
        h.hover_at(over);
        press(&h, over, true, egui::Modifiers::NONE);
        h.run_steps(2);
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        press(&h, over, false, egui::Modifiers::NONE);
        settle(&mut h);
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: during a drag");
        // A window in front with OK and Cancel: a waypoint's Properties.
        let wp = waypoint("wp_a");
        open(&mut h, &wp);
        assert_eq!(in_main(&h, &wp), Some(false), "Properties in a window");
        h.hover_at(in_left_dock(&h));
        h.run_steps(2);
        escape(&mut h);
        assert!(!open_tab(&h, &wp), "{mode:?}: the window's Escape");
        assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the window's alone");
        // Then the docked tab's (the keyboard in its group).
        escape(&mut h);
        assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: then the tab's");
        assert_eq!(tag(&mut h), "SAMPLE");
    }
}

/// Docked, Options closes at Escape (its draft dropped, as its Cancel); a
/// model's view docked (its kind kept in a group) does not: a tab without
/// OK and Cancel.
#[test]
fn escape_closes_docked_options_but_not_a_docked_model_view() {
    for mode in MODES {
        let places = [("options", "middle"), ("model", "middle")];
        let mut h = window(mode, "ui-open-escape-options", &places);
        h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
        settle(&mut h);
        assert_eq!(in_main(&h, &Tab::Options), Some(true), "{mode:?}: docked");
        h.state_mut().options.as_mut().unwrap().light_theme = true;
        let (_, leaf) =
            (h.state().dock.iter_leaves()).find(|(_, l)| l.tabs.contains(&Tab::Options)).unwrap();
        let over = leaf.viewport.center();
        h.hover_at(over);
        h.run_steps(2);
        escape(&mut h);
        assert!(!open_tab(&h, &Tab::Options), "{mode:?}: closed");
        assert!(!h.state().settings.light_theme, "its draft dropped");
        // Waiting for a key on its Keyboard page: Escape is the page's.
        h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
        settle(&mut h);
        h.state_mut().options.as_mut().unwrap().recording = Some("manual".to_string());
        h.hover_at(over);
        h.run_steps(2);
        escape(&mut h);
        assert!(open_tab(&h, &Tab::Options), "{mode:?}: the recorder's Escape");
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Options]));
        settle(&mut h);
        let model = Tab::Model(ResKey::parse("hello", ResType::NSS).unwrap());
        open(&mut h, &model);
        assert_eq!(in_main(&h, &model), Some(true), "{mode:?}: docked");
        h.hover_at(over);
        h.run_steps(2);
        escape(&mut h);
        assert!(open_tab(&h, &model), "{mode:?}: a docked model's view stays");
    }
}

/// Tile Properties docked (its kind kept beside the area) with a change
/// not applied asks before it closes at Escape, as its X does; Cancel
/// keeps it. (The game's data and a GPU, else skipped.)
#[test]
fn escape_on_docked_tile_properties_with_changes_asks() {
    use glam::Vec3;
    for mode in MODES {
        let Some((mut h, area)) = area_harness(&mode.dir("ui-open-escape-tiles")) else { return };
        let mut settings = mode.settings();
        settings.window_places = vec![("tile-properties".into(), "split right 400".into())];
        settings.hide_log = h.state().settings.hide_log;
        h.state_mut().settings = settings;
        crate::tiles::tile_properties_of(&mut h, area, &[Vec3::new(15.0, 15.0, 0.0)]);
        settle(&mut h);
        assert_eq!(in_main(&h, &Tab::TileProperties), Some(true), "{mode:?}: docked");
        let p = h.state_mut().tile_props.as_mut().expect("open");
        p.main[0] = (p.main[0] + 5) % 32;
        settle(&mut h);
        let (_, leaf) = (h.state().dock.iter_leaves())
            .find(|(_, l)| l.tabs.contains(&Tab::TileProperties))
            .unwrap();
        let over = leaf.viewport.center();
        h.hover_at(over);
        h.run_steps(2);
        escape(&mut h);
        h.get_by_label("Save changes to Tile Properties?");
        assert!(open_tab(&h, &Tab::TileProperties), "{mode:?}: asked, still open");
        h.get_by_label("Don't Save").click();
        settle(&mut h);
        assert!(!open_tab(&h, &Tab::TileProperties));
    }
}
