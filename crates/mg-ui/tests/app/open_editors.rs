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
        until(&mut h, "the Plugins menu", |h| h.query_by_label("Panels").is_some());
        h.get_by_label("Panels").click();
        until(&mut h, "its Panels", |h| h.query_by_label("Module Overview").is_some());
        h.get_by_label("Module Overview").click();
        settle(&mut h);
        let tab =
            Tab::PluginPanel { plugin: "example.module-overview".into(), panel: "overview".into() };
        assert_eq!(in_main(&h, &tab), Some(mode.tabs()), "{mode:?}");
        h.get_by_label("The module");
    }
}

// Escape on a docked tab with OK and Cancel (ruling R1). The keyboard
// decides first (the tab or window of the widget with the focus, else of
// the last press); the pointer only where the keyboard is in nothing
// that could take the Escape. Each body runs in both modes, a test each.

fn tag(h: &mut Harness<'_, Moonglow>) -> String {
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    let root = h.state_mut().ws.as_mut().unwrap().doc(&info).unwrap().root.clone();
    String::from_utf8_lossy(root.string("Mod_Tag").unwrap_or_default()).into_owned()
}

/// A waypoint blueprint's tag, as the workspace holds it.
fn wp_tag(h: &mut Harness<'_, Moonglow>, name: &str) -> String {
    let key = ResKey::parse(name, ResType::UTW).unwrap();
    let root = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().root.clone();
    String::from_utf8_lossy(root.string("Tag").unwrap_or_default()).into_owned()
}

/// The pointer moved to `at`: one step for the move.
fn hover(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    h.step();
}

/// A click at `at`: a step for the move, the press and the release each.
fn click_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    hover(h, at);
    press(h, at, true, egui::Modifiers::NONE);
    h.step();
    press(h, at, false, egui::Modifiers::NONE);
    h.step();
}

/// The text field showing `value` on the row of a label `label`.
fn field_at(h: &Harness<'_, Moonglow>, label: &str, value: &str) -> egui::Pos2 {
    let rows: Vec<f32> = h.get_all_by_label(label).map(|n| n.rect().center().y).collect();
    let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some(value)
    };
    (h.query_all(egui_kittest::kittest::by().predicate(is)))
        .map(|n| n.rect().center())
        .find(|c| rows.iter().any(|y| (c.y - y).abs() < 8.0))
        .unwrap_or_else(|| panic!("the {label} field showing {value}"))
}

/// Types "X" at the end of the field at `at`, clicked first; waits until
/// the field shows it.
fn type_x(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, value: &str) {
    click_at(h, at);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("X".into()));
    let typed = format!("{value}X");
    until(h, "the field typed in", |h| h.query_all_by_value(&typed).next().is_some());
}

/// Module Properties (docked as the module opened, in both modes) brought
/// in front of its group, opened again where it was closed, and its tag
/// field typed in (an X at its end).
fn edit_tag(h: &mut Harness<'_, Moonglow>) {
    if h.state().dock.find_tab(&Tab::ModuleProperties).is_none() {
        open(h, &Tab::ModuleProperties);
    }
    click_tab(h, &Tab::ModuleProperties);
    settle(h);
    let now = tag(h);
    let at = field_at(h, "Tag", &now);
    type_x(h, at, &now);
}

/// A point of `tab`'s body that nothing lies over (no window), away from
/// its fields and its OK and Cancel where it can be: its right side, from
/// the middle down, then up.
fn visible_in(h: &Harness<'_, Moonglow>, tab: &Tab) -> egui::Pos2 {
    let (_, leaf) = (h.state().dock.iter_leaves())
        .find(|(_, l)| l.tabs.contains(tab))
        .unwrap_or_else(|| panic!("{tab:?} open"));
    let v = leaf.viewport;
    let main = |p: egui::Pos2| h.ctx.layer_id_at(p).is_some_and(|l| l.order != egui::Order::Middle);
    let rows = [0.5, 0.6, 0.4, 0.7, 0.3, 0.8, 0.2];
    rows.iter()
        .flat_map(|y| (0..8).map(move |j| (*y, j)))
        .map(|(y, j)| {
            egui::pos2(v.right() - 30.0 - j as f32 * v.width() / 9.0, v.top() + y * v.height())
        })
        .find(|p| v.contains(*p) && main(*p))
        .expect("a part of it in sight")
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

fn last_log(h: &Harness<'_, Moonglow>) -> String {
    h.state().log.entries.last().map(|e| e.1.clone()).unwrap_or_default()
}

/// Escape cancels Module Properties docked in front of its group: typed
/// in with the pointer over it, and typed in with the pointer on the Left
/// Dock, what was typed set back and the tab closed, as a floating
/// Properties window's Escape does. The log says so (and that Undo brings
/// the changes back, which it does), shown in the Status Bar while the
/// log is closed; nothing is said where nothing was changed. A waypoint's
/// Properties opened where editors open (a tab in As Tabs, a window in As
/// Floating Windows) is cancelled the same way. A script's text with the
/// keyboard, in a group of its own: Escape is neither Module Properties'
/// nor the script's (no OK and Cancel).
#[test]
fn escape_cancels_a_docked_tab_with_ok_and_cancel() {
    cancels(Mode::Floating);
}

#[test]
fn escape_cancels_a_docked_tab_with_ok_and_cancel_as_tabs() {
    cancels(Mode::Tabs);
}

fn cancels(mode: Mode) {
    // (Module Properties opened again in the area's group in As Floating
    // Windows too; As Tabs places it so by itself.)
    let places: &[(&str, &str)] = match mode {
        Mode::Floating => &[("script", "split right 500"), ("module-properties", "middle")],
        Mode::Tabs => &[("script", "split right 500")],
    };
    let mut h = window(mode, "ui-open-escape", places);
    // The log closed, its last line in the Status Bar.
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    // Typed in, the pointer over it.
    edit_tag(&mut h);
    assert_eq!(in_main(&h, &Tab::ModuleProperties), Some(true));
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: closed");
    assert_eq!(tag(&mut h), "SAMPLE", "{mode:?}: set back");
    let said = "Escape canceled Module Properties: its changes are set back. \
                Edit › Undo brings them back.";
    assert_eq!(last_log(&h), said);
    let status = format!("Last log line: {said}");
    until(&mut h, "the line in the Status Bar", |h| h.query_by_label(&status).is_some());
    // One Undo: the changes back.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(tag(&mut h), "SAMPLEX", "{mode:?}: Undo brings them back");
    // Nothing changed: closed, nothing said.
    open(&mut h, &Tab::ModuleProperties);
    let lines = h.state().log.entries.len();
    let over = visible_in(&h, &Tab::ModuleProperties);
    click_at(&mut h, over);
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: closed unchanged");
    assert_eq!(h.state().log.entries.len(), lines, "nothing said");
    // Typed in, the pointer on the Left Dock: the keyboard's.
    edit_tag(&mut h);
    let at = in_left_dock(&h);
    hover(&mut h, at);
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: closed by the keyboard's");
    assert_eq!(tag(&mut h), "SAMPLEX");
    // A waypoint's Properties opened where editors open: Escape its
    // Cancel, a tab or a window.
    let wp = waypoint("wp_a");
    open(&mut h, &wp);
    assert_eq!(in_main(&h, &wp), Some(mode.tabs()), "{mode:?}: where editors open");
    let at = field_at(&h, "Tag", "wp_a");
    type_x(&mut h, at, "wp_a");
    escape(&mut h);
    assert!(!open_tab(&h, &wp), "{mode:?}: the waypoint's Properties canceled");
    assert_eq!(wp_tag(&mut h, "wp_a"), "wp_a", "set back");
    assert!(last_log(&h).ends_with("its changes are set back. Edit › Undo brings them back."));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(wp_tag(&mut h, "wp_a"), "wp_aX", "{mode:?}: Undo brings it back");
    // A script in a group of its own, its text clicked (it has the
    // keyboard): neither Module Properties' with the pointer over it, nor
    // the script's. (Module Properties opened first, in the area's group:
    // opened after, As Tabs puts it in the script's, which has the focus.)
    open(&mut h, &Tab::ModuleProperties);
    open(&mut h, &script(1));
    edit_tag(&mut h);
    let is_text = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::MultilineTextInput
            && n.value().is_some_and(|v| v.contains("void main"))
    };
    let text = h.get(egui_kittest::kittest::by().predicate(is_text)).rect().center();
    click_at(&mut h, text);
    assert!(h.ctx.memory(|m| m.focused().is_some()), "the script's text has the keyboard");
    let at = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, at);
    escape(&mut h);
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the keyboard in another group");
    assert!(open_tab(&h, &script(1)), "{mode:?}: a script's tab is not closed");
}

/// Where Escape goes when the keyboard and the pointer are in different
/// places (Module Properties in the area's group, a waypoint's Properties
/// in a group at its right):
/// - the keyboard in a docked tab with OK and Cancel: that tab's, over the
///   one under the pointer;
/// - the keyboard moved without a press (a field focused by the keys) is
///   where it is, not where the last press was;
/// - the keyboard in a floating window (a script's: its text with the
///   keys, or its tab pressed last): no docked tab under the pointer takes
///   it, as before ruling R1;
/// - the keyboard in nothing that takes Escape (the last press on the
///   Left Dock, no field with the keys): the tab under the pointer.
#[test]
fn escape_follows_the_keyboard_then_the_pointer() {
    follows(Mode::Floating);
}

#[test]
fn escape_follows_the_keyboard_then_the_pointer_as_tabs() {
    follows(Mode::Tabs);
}

fn follows(mode: Mode) {
    let places =
        [("module-properties", "middle"), ("waypoint", "split right 500"), ("script", "window")];
    let mut h = window(mode, "ui-open-escape-keys", &places);
    let wp = waypoint("wp_a");
    open(&mut h, &wp);
    assert_eq!(in_main(&h, &wp), Some(true));
    assert_ne!(pane(&h, &wp), pane(&h, &Tab::ModuleProperties), "two groups");
    // The keyboard in Module Properties, the pointer over the waypoint's.
    edit_tag(&mut h);
    let at = visible_in(&h, &wp);
    hover(&mut h, at);
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the keyboard's tab");
    assert!(open_tab(&h, &wp), "{mode:?}: not the hovered one");
    // The last press in Module Properties, then the waypoint's tag field
    // given the keys without a press: the waypoint's, the pointer over
    // Module Properties.
    open(&mut h, &Tab::ModuleProperties);
    let at = visible_in(&h, &Tab::ModuleProperties);
    click_at(&mut h, at);
    let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some("wp_a")
    };
    h.get(egui_kittest::kittest::by().predicate(is)).focus();
    until(&mut h, "the waypoint's field with the keys", |h| {
        h.ctx.memory(|m| m.focused().is_some())
    });
    let at = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, at);
    escape(&mut h);
    assert!(!open_tab(&h, &wp), "{mode:?}: where the keys are");
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: not the last press's");
    // A floating script's window with the keyboard, the pointer over a part
    // of Module Properties in sight: neither.
    let s = script(1);
    open(&mut h, &s);
    assert_eq!(in_main(&h, &s), Some(false), "a window");
    let is_text = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::MultilineTextInput
            && n.value().is_some_and(|v| v.contains("void main"))
    };
    let text = h.get(egui_kittest::kittest::by().predicate(is_text)).rect().center();
    click_at(&mut h, text);
    let at = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, at);
    escape(&mut h);
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the window's Escape, not the tab's");
    assert!(open_tab(&h, &s), "{mode:?}: the script stays");
    // The last press on the window's tab (no field with the keys): the
    // window's still, the pointer over Module Properties.
    let title = h.state().tab_rect(&s).unwrap().center();
    click_at(&mut h, title);
    assert!(h.ctx.memory(|m| m.focused().is_none()), "no field with the keys");
    let at = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, at);
    escape(&mut h);
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the window's, its tab pressed");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![s]));
    settle(&mut h);
    // The keyboard in nothing that takes it: the pointer's tab.
    open(&mut h, &wp);
    let at = in_left_dock(&h);
    click_at(&mut h, at);
    let at = visible_in(&h, &wp);
    hover(&mut h, at);
    escape(&mut h);
    assert!(!open_tab(&h, &wp), "{mode:?}: the pointer's tab");
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: and only it");
}

/// Escape gives way: to an open menu (it closes the menu, the tab stays;
/// a second Escape, the pointer over the tab, is the tab's), to Quick Open, to the module tree's
/// Filter (it clears what was typed), to the log holding the arrow keys
/// (it hands them back), to a drag in progress (a button held), and to a
/// window in front with OK and Cancel (a waypoint's Properties, floating:
/// the window's, the docked tab's at the next).
#[test]
fn escape_on_a_docked_tab_gives_way() {
    gives_way(Mode::Floating);
}

#[test]
fn escape_on_a_docked_tab_gives_way_as_tabs() {
    gives_way(Mode::Tabs);
}

fn gives_way(mode: Mode) {
    let places = [("waypoint", "window"), ("module-properties", "middle")];
    let mut h = window(mode, "ui-open-escape-way", &places);
    edit_tag(&mut h);
    // An open menu: its Escape, then the tab's.
    h.get_by_label("View").click();
    until(&mut h, "the menu open", |h| h.query_by_label("Reset Layout").is_some());
    escape(&mut h);
    assert!(h.query_by_label("Reset Layout").is_none(), "{mode:?}: the menu closed");
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: and only it");
    // (The last press on the menu's name: the pointer's tab.)
    let at = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, at);
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the second Escape the tab's");
    assert_eq!(tag(&mut h), "SAMPLE");
    // Quick Open.
    edit_tag(&mut h);
    let over = visible_in(&h, &Tab::ModuleProperties);
    click_at(&mut h, over);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::P);
    settle(&mut h);
    assert!(h.state().quick_open.is_open(), "Quick Open open");
    escape(&mut h);
    assert!(!h.state().quick_open.is_open());
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: Quick Open's Escape");
    // The module tree's Filter.
    type_into_tree_filter(&mut h, "zz");
    hover(&mut h, over);
    escape(&mut h);
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the Filter's Escape");
    assert!(h.query_all_by_value("zz").next().is_none(), "the Filter cleared");
    // The log's own Escape: clicked in, it has the arrow keys, and Escape
    // hands them back (drawn before the editor area).
    h.state_mut().settings.hide_log = false;
    settle(&mut h);
    let log = h.state().bottom_dock_rect().unwrap();
    click_at(&mut h, egui::pos2(log.center().x, log.bottom() - 20.0));
    let at = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, at);
    escape(&mut h);
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the log's Escape");
    // A drag: the button held over it.
    let over = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, over);
    press(&h, over, true, egui::Modifiers::NONE);
    h.step();
    h.key_press(egui::Key::Escape);
    // (A step for the key, and one more for an Escape seen to act.)
    h.step();
    h.step();
    press(&h, over, false, egui::Modifiers::NONE);
    settle(&mut h);
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: during a drag");
    // A window in front with OK and Cancel: a waypoint's Properties.
    let wp = waypoint("wp_a");
    open(&mut h, &wp);
    assert_eq!(in_main(&h, &wp), Some(false), "Properties in a window");
    let at = field_at(&h, "Tag", "wp_a");
    click_at(&mut h, at);
    let at = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, at);
    escape(&mut h);
    assert!(!open_tab(&h, &wp), "{mode:?}: the window's Escape");
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the window's alone");
    // Then the docked tab's (the pointer over it, the keyboard in nothing).
    let at = in_left_dock(&h);
    click_at(&mut h, at);
    let at = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, at);
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: then the tab's");
    // The window in front again, the keyboard in Module Properties (typed
    // in): the tab's, not the window's.
    open(&mut h, &Tab::ModuleProperties);
    open(&mut h, &wp);
    // (Its window set at the right, off Module Properties' fields.)
    let path = h.state().dock.find_tab(&wp).unwrap();
    let w = h.state_mut().dock.get_window_state_mut(path.surface).unwrap();
    w.set_position(egui::pos2(950.0, 420.0)).set_size(egui::vec2(420.0, 420.0));
    settle(&mut h);
    edit_tag(&mut h);
    assert_eq!(in_main(&h, &wp), Some(false), "the window over the area");
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the keyboard's tab");
    assert!(open_tab(&h, &wp), "{mode:?}: not the window in front");
}

/// Docked, Options closes at Escape (its draft dropped, as its Cancel, the
/// log saying so with no Undo); waiting for a key on its Keyboard page,
/// Escape is the page's; a model's view docked (its kind kept in a group)
/// does not close: a tab without OK and Cancel.
#[test]
fn escape_closes_docked_options_but_not_a_docked_model_view() {
    options_and_model(Mode::Floating);
}

#[test]
fn escape_closes_docked_options_but_not_a_docked_model_view_as_tabs() {
    options_and_model(Mode::Tabs);
}

fn options_and_model(mode: Mode) {
    let places = [("options", "middle"), ("model", "middle")];
    let mut h = window(mode, "ui-open-escape-options", &places);
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    assert_eq!(in_main(&h, &Tab::Options), Some(true), "{mode:?}: docked");
    h.state_mut().options.as_mut().unwrap().light_theme = true;
    let over = visible_in(&h, &Tab::Options);
    hover(&mut h, over);
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::Options), "{mode:?}: closed");
    assert!(!h.state().settings.light_theme, "its draft dropped");
    assert_eq!(last_log(&h), "Escape canceled Options: its changes are set back.");
    // Waiting for a key on its Keyboard page: Escape is the page's.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    h.state_mut().options.as_mut().unwrap().recording = Some("manual".to_string());
    hover(&mut h, over);
    escape(&mut h);
    assert!(open_tab(&h, &Tab::Options), "{mode:?}: the recorder's Escape");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Options]));
    settle(&mut h);
    let model = Tab::Model(ResKey::parse("hello", ResType::NSS).unwrap());
    open(&mut h, &model);
    assert_eq!(in_main(&h, &model), Some(true), "{mode:?}: docked");
    let at = visible_in(&h, &model);
    hover(&mut h, at);
    escape(&mut h);
    assert!(open_tab(&h, &model), "{mode:?}: a docked model's view stays");
}

/// Tile Properties docked (its kind kept beside the area) with a change
/// not applied asks before it closes at Escape, as its X does; Don't Save
/// closes it. (The game's data and a GPU, else skipped.)
#[test]
fn escape_on_docked_tile_properties_with_changes_asks() {
    tile_properties_ask(Mode::Floating);
}

#[test]
fn escape_on_docked_tile_properties_with_changes_asks_as_tabs() {
    tile_properties_ask(Mode::Tabs);
}

fn tile_properties_ask(mode: Mode) {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness(&mode.dir("ui-open-escape-tiles")) else { return };
    put_in(h.state_mut(), mode);
    h.state_mut().settings.window_places =
        vec![("tile-properties".into(), "split right 400".into())];
    crate::tiles::tile_properties_of(&mut h, area, &[Vec3::new(15.0, 15.0, 0.0)]);
    settle(&mut h);
    assert_eq!(in_main(&h, &Tab::TileProperties), Some(true), "{mode:?}: docked");
    let p = h.state_mut().tile_props.as_mut().expect("open");
    p.main[0] = (p.main[0] + 5) % 32;
    settle(&mut h);
    let at = visible_in(&h, &Tab::TileProperties);
    click_at(&mut h, at);
    escape(&mut h);
    h.get_by_label("Save changes to Tile Properties?");
    assert!(open_tab(&h, &Tab::TileProperties), "{mode:?}: asked, still open");
    h.get_by_label("Don't Save").click();
    settle(&mut h);
    assert!(!open_tab(&h, &Tab::TileProperties));
}

/// Options › General's Open editors is a radio group named "Open editors"
/// (a screen reader says the label with the choices), its choices in it;
/// the arrow keys on the chosen one choose the other.
#[test]
fn the_switch_is_a_named_radio_group_the_arrows_choose_in() {
    let mut h = harness(app_with(Vec::new()), 1100.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    h.state_mut().options.as_mut().unwrap().page = mg_ui::OptionsPage::General;
    settle(&mut h);
    let group = h.get_by_role_and_label(egui::accesskit::Role::RadioGroup, "Open editors");
    group.get_by_label("As Tabs");
    group.get_by_label("As Floating Windows").focus();
    settle(&mut h);
    h.key_press(egui::Key::ArrowLeft);
    settle(&mut h);
    let draft = h.state().options.as_ref().unwrap().open_editors;
    assert_eq!(draft, mg_ui::OpenEditors::Tabs, "Left: As Tabs");
    assert!(h.get_by_label("As Tabs").is_focused());
}

/// What the Status Bar says (`Moonglow::status_said`).
fn said(h: &Harness<'_, Moonglow>) -> Vec<String> {
    h.state().status_said().to_vec()
}

/// Whether the Status Bar shows an Esc hint for a tab's Cancel.
fn esc_hinted(h: &Harness<'_, Moonglow>) -> bool {
    said(h).iter().any(|s| s.starts_with("Esc: Cancel"))
}

/// The Status Bar's Esc hint (M3's T12) is shown for a docked tab with OK
/// and Cancel only where an Escape would in truth be its Cancel (T10's
/// rule): Module Properties in front, pressed in away from its fields
/// (Esc, Tab and Shift + Tab shown), kept with the pointer moved to the
/// Left Dock (the last press's tab; and Escape closes it); not while a
/// field has the keyboard (which tab's an Escape is shows only at the
/// key); after a press on the Left Dock, not with the pointer there, and
/// with the pointer over the tab, its; not while a menu is open; not
/// where the last press was in a floating window (the window's Escape:
/// the window closes, the tab stays); not while the module tree has the
/// keys (its Escape first).
#[test]
fn the_esc_hint_shows_where_the_escape_is_the_tab_s() {
    esc_hint(Mode::Floating);
}

#[test]
fn the_esc_hint_shows_where_the_escape_is_the_tab_s_as_tabs() {
    esc_hint(Mode::Tabs);
}

fn esc_hint(mode: Mode) {
    let places = [("module-properties", "middle"), ("waypoint", "window")];
    let mut h = window(mode, "ui-open-esc-hint", &places);
    if !open_tab(&h, &Tab::ModuleProperties) {
        open(&mut h, &Tab::ModuleProperties);
    }
    click_tab(&mut h, &Tab::ModuleProperties);
    settle(&mut h);
    let over = visible_in(&h, &Tab::ModuleProperties);
    click_at(&mut h, over);
    until(&mut h, "the Esc hint", esc_hinted);
    let now = said(&h);
    for w in ["Esc: Cancel", "Tab: Next field", "Shift, Tab: Previous field"] {
        assert!(now.iter().any(|s| s == w), "{mode:?}: {w} in {now:?}");
    }
    assert!(!now.iter().any(|s| s == "Enter: OK"), "{mode:?}: Enter is Options' alone");
    // The pointer on the Left Dock, the last press in the tab: still its
    // (the keyboard's), and Escape cancels it there.
    let dock = in_left_dock(&h);
    hover(&mut h, dock);
    h.step();
    assert!(esc_hinted(&h), "{mode:?}: the last press's tab: {:?}", said(&h));
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the hint was true");
    // A field with the keyboard: no hint.
    edit_tag(&mut h);
    until(&mut h, "no Esc hint, a field typed in", |h| !esc_hinted(h));
    // A press on the Left Dock: not with the pointer there; over the tab,
    // its.
    let dock = in_left_dock(&h);
    click_at(&mut h, dock);
    until(&mut h, "no Esc hint, the pointer on the dock", |h| !esc_hinted(h));
    hover(&mut h, over);
    until(&mut h, "the Esc hint, the pointer over the tab", esc_hinted);
    // A menu open: none.
    h.get_by_label("View").click();
    until(&mut h, "the menu open", |h| h.query_by_label("Reset Layout").is_some());
    until(&mut h, "no Esc hint, a menu open", |h| !esc_hinted(h));
    escape(&mut h);
    // The last press in a floating window with OK and Cancel (its bar):
    // none, and Escape is the window's.
    let wp = waypoint("wp_a");
    open(&mut h, &wp);
    assert_eq!(in_main(&h, &wp), Some(false), "a window");
    let title = h.state().tab_rect(&wp).unwrap().center();
    click_at(&mut h, title);
    hover(&mut h, over);
    until(&mut h, "no Esc hint, the window's", |h| !esc_hinted(h));
    escape(&mut h);
    assert!(!open_tab(&h, &wp), "{mode:?}: the window's Escape");
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the tab stays");
    // The module tree with the keys (F6 to it): its Escape first (it lets
    // the row go), none.
    // (The last press on the Left Dock, which takes no Escape: without
    // the tree's keys, the pointer's tab would take it.)
    let dock = in_left_dock(&h);
    click_at(&mut h, dock);
    let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
    for _ in 0..4 {
        if keys(&h) == "Tree" {
            break;
        }
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    assert_eq!(keys(&h), "Tree", "{mode:?}: the tree's keys");
    let over = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, over);
    until(&mut h, "no Esc hint, the tree's keys", |h| !esc_hinted(h));
    escape(&mut h);
    assert!(open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the tree's Escape, as hinted");
}

/// Where an Escape would cancel a docked tab other than the active
/// editor (Module Properties in a group of its own beside the area, the
/// pointer over it, the area's group focused), the hint names it, first:
/// "Esc: Cancel Module Properties"; and Escape cancels it.
#[test]
fn the_esc_hint_names_another_tab_than_the_active_editor() {
    other_tab(Mode::Floating);
}

#[test]
fn the_esc_hint_names_another_tab_than_the_active_editor_as_tabs() {
    other_tab(Mode::Tabs);
}

fn other_tab(mode: Mode) {
    let places = [("module-properties", "split right 500")];
    let mut h = window(mode, "ui-open-esc-other", &places);
    // (Opened again where its kind's place puts it, beside the area.)
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::ModuleProperties]));
    settle(&mut h);
    open(&mut h, &Tab::ModuleProperties);
    assert_ne!(pane(&h, &start()), pane(&h, &Tab::ModuleProperties), "{mode:?}: two groups");
    click_tab(&mut h, &start());
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&start()), "{mode:?}: the area's group");
    let over = visible_in(&h, &Tab::ModuleProperties);
    hover(&mut h, over);
    let named = "Esc: Cancel Module Properties";
    until(&mut h, "the other tab's Esc", |h| said(h).first().is_some_and(|s| s == named));
    escape(&mut h);
    assert!(!open_tab(&h, &Tab::ModuleProperties), "{mode:?}: the hint was true");
}

/// Docked Options shows Enter for its OK while the pointer is over it and
/// nothing has the keyboard (where its Enter is OK, and pressed it is),
/// and not with the pointer elsewhere, nor while its Keyboard page waits
/// for a key; its Esc, Tab and Shift + Tab as the Options board has them.
#[test]
fn docked_options_hints_enter_where_it_is_ok() {
    let mut h = window(Mode::Tabs, "ui-open-enter-hint", &[("options", "middle")]);
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    assert_eq!(in_main(&h, &Tab::Options), Some(true), "docked");
    let over = visible_in(&h, &Tab::Options);
    click_at(&mut h, over);
    until(&mut h, "Enter and Esc", |h| said(h).iter().any(|s| s == "Enter: OK") && esc_hinted(h));
    let now = said(&h);
    let board = ["Enter: OK", "Esc: Cancel", "Tab: Next field", "Shift, Tab: Previous field"];
    assert_eq!(now, board, "the Options board's, with Esc");
    let away = in_left_dock(&h);
    hover(&mut h, away);
    until(&mut h, "no Enter, the pointer away", |h| !said(h).iter().any(|s| s == "Enter: OK"));
    hover(&mut h, over);
    until(&mut h, "Enter again", |h| said(h).iter().any(|s| s == "Enter: OK"));
    // Waiting for a key on its Keyboard page: no Enter (nor Esc).
    h.state_mut().options.as_mut().unwrap().recording = Some("manual".to_string());
    until(&mut h, "no Enter, a key awaited", |h| {
        !said(h).iter().any(|s| s == "Enter: OK") && !esc_hinted(h)
    });
    h.state_mut().options.as_mut().unwrap().recording = None;
    until(&mut h, "Enter once more", |h| said(h).iter().any(|s| s == "Enter: OK"));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(!open_tab(&h, &Tab::Options), "Enter was its OK");
}
