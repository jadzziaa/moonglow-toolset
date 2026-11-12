//! The main window's frame (the UI redesign's shell, M3's T2): the Top
//! Bar's buttons, the Left Dock that folds to its strip, the Bottom Dock
//! the Log's switch opens and closes, the Status Bar's module and folder,
//! and the frame at a narrow width.

use crate::common::*;

/// The Top Bar's buttons, by their names: the quick actions, then the
/// build actions.
const QUICK: [&str; 5] = ["New", "Open", "Save", "Undo", "Redo"];
const BUILD: [&str; 3] = ["Compile", "Verify", "Test Module"];
/// Test Module's arrow, which opens the menu of the other ways to test.
const TEST_MENU: &str = "More Ways to Test";

/// Each Top Bar button's menu and row: the same command, which the
/// button is enabled as.
const ROWS: [(&str, &str, &str); 8] = [
    ("New", "File", "New Module…"),
    ("Open", "File", "Open Module…"),
    ("Save", "File", "Save"),
    ("Undo", "Edit", "Undo"),
    ("Redo", "Edit", "Redo"),
    ("Compile", "Build", "Compile All Scripts"),
    ("Verify", "Build", "Verify Module"),
    ("Test Module", "Build", "Test Module"),
];

/// Whether the node is said to be disabled.
fn disabled(n: &egui_kittest::Node<'_>) -> bool {
    n.accesskit_node().is_disabled()
}

/// Whether the row `row` of the menu `menu` (its words, or its words and
/// its keys; Undo and Redo name their step after them) can be chosen; the
/// menu closed after.
fn row_enabled(h: &mut Harness<'_, Moonglow>, menu: &str, row: &str) -> bool {
    h.get_by_label("Top Bar").get_by_label(menu).click();
    settle(h);
    let (label, _) = menu_row(h, row).unwrap_or_else(|| panic!("{menu} › {row}"));
    let enabled = !disabled(&h.get_by_label(&label));
    h.key_press(egui::Key::Escape);
    settle(h);
    enabled
}

/// Each button is found by its name within the Top Bar, says what it is
/// in a tooltip, and is enabled as its command is (its menu row's state),
/// with no module open and with one open and edited.
#[test]
fn the_top_bar_s_buttons_are_named_tipped_and_enabled_as_their_commands() {
    let dir = mg_testkit::scratch_dir("ui-top-bar-states");
    let path = sample_module(&dir);
    let mut h = harness(app_with(Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    let check = |h: &mut Harness<'_, Moonglow>, when: &str| {
        for (name, menu, row) in ROWS {
            let shown = !disabled(&tool(h, name));
            let as_menu = row_enabled(h, menu, row);
            assert_eq!(shown, as_menu, "{name} {when}: enabled as {menu} › {row}");
        }
    };
    check(&mut h, "with no module");
    assert!(!disabled(&tool(&h, "New")) && disabled(&tool(&h, "Save")));
    // (Test Module's arrow as its one row, Test Module, Choose Character.)
    assert!(disabled(&tool(&h, TEST_MENU)) && disabled(&tool(&h, "Test Module")));
    for name in QUICK.iter().chain(&BUILD).chain(&[TEST_MENU]) {
        let rect = tool(&h, name).rect();
        let tip = tip_at(&mut h, rect).unwrap_or_default();
        assert!(!tip.trim().is_empty(), "{name} has a tooltip");
    }
    // A tip says the button's name or what it does, with the command's key.
    let open = tool(&h, "Open").rect();
    let tip = tip_at(&mut h, open).unwrap();
    assert!(tip.starts_with("Open module (") && tip.ends_with(')'), "{tip}");
    h.state_mut().open_module(&path);
    settle(&mut h);
    check(&mut h, "with the module open");
    assert!(!disabled(&tool(&h, TEST_MENU)), "Choose Character offered with a module");
    let edit = mg_edit::Edit::SetResource {
        key: ResKey::parse("added", ResType::NSS).unwrap(),
        data: Some(b"void main() { }\n".to_vec()),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Add", vec![edit])));
    settle(&mut h);
    check(&mut h, "after an edit");
    assert!(!disabled(&tool(&h, "Undo")) && disabled(&tool(&h, "Redo")));
    // Undo's tip names the step it takes, as Edit › Undo does.
    let undo = tool(&h, "Undo").rect();
    let tip = tip_at(&mut h, undo).unwrap();
    assert!(tip.starts_with("Undo Add"), "{tip}");
}

/// Each Top Bar button runs its command: New asks for the new module's
/// name, Open opens the module chosen, Undo and Redo take the module's
/// step back and again, Save saves it, Compile and Test Module run (and
/// say, without the game, what they need), Choose Character under Test
/// Module's arrow runs Test Module, Choose Character.
#[test]
fn the_top_bar_s_buttons_run_their_commands() {
    let dir = mg_testkit::scratch_dir("ui-top-bar-runs");
    let path = sample_module(&dir);
    let other_dir = dir.join("other");
    std::fs::create_dir_all(&other_dir).unwrap();
    let other = sample_module(&other_dir);
    let mut app = app_with(vec![other.clone()]);
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let said = |h: &Harness<'_, Moonglow>, from: usize| -> Vec<String> {
        h.state().log.entries[from..].iter().map(|(_, m)| m.clone()).collect()
    };
    let step =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_owned);
    // Undo and Redo: the module's step.
    let edit = mg_edit::Edit::SetResource {
        key: ResKey::parse("added", ResType::NSS).unwrap(),
        data: Some(b"void main() { }\n".to_vec()),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Add", vec![edit])));
    settle(&mut h);
    assert_eq!(step(&h).as_deref(), Some("Add"));
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(step(&h), None, "undone by the Top Bar's Undo");
    tool(&h, "Redo").click();
    settle(&mut h);
    assert_eq!(step(&h).as_deref(), Some("Add"), "and done again by its Redo");
    // Save.
    assert!(h.state().ws.as_ref().unwrap().is_modified());
    tool(&h, "Save").click();
    settle(&mut h);
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "saved by the Top Bar's Save");
    // Compile, without the game's data: it says so.
    let from = h.state().log.entries.len();
    tool(&h, "Compile").click();
    settle(&mut h);
    let compiled = said(&h, from);
    assert!(compiled.iter().any(|m| m.starts_with("Compiling needs")), "{compiled:?}");
    // Test Module, and Choose Character under its arrow (without the
    // game: each says it needs it, after the save Test Module makes).
    let from = h.state().log.entries.len();
    tool(&h, "Test Module").click();
    settle(&mut h);
    let tested = said(&h, from);
    assert_eq!(tested.iter().filter(|m| *m == "Test Module needs the game").count(), 1);
    let arrow = tool(&h, TEST_MENU);
    assert_eq!(arrow.accesskit_node().role(), egui::accesskit::Role::Button);
    arrow.click();
    settle(&mut h);
    let from = h.state().log.entries.len();
    let row = h.get_by_label_contains("Test Module, Choose Character");
    row.click();
    settle(&mut h);
    let chosen = said(&h, from);
    assert_eq!(chosen.iter().filter(|m| *m == "Test Module needs the game").count(), 1);
    assert!(!egui::Popup::is_any_open(&h.ctx), "the menu closed");
    // New: the new module's name asked for.
    tool(&h, "New").click();
    settle(&mut h);
    assert!(matches!(h.state().wizard, Some(mg_ui::Wizard::NewModule { .. })));
    h.state_mut().wizard = None;
    settle(&mut h);
    // Open: the module chosen opened.
    tool(&h, "Open").click();
    settle(&mut h);
    assert_eq!(h.state().module_path().as_deref(), Some(other.as_path()), "opened");
}

/// Verify on the Top Bar runs Verify Module, and the Module panel's
/// header buttons run theirs: Properties opens Module Properties, New
/// Area the Area Wizard.
#[test]
fn verify_and_the_module_panel_s_buttons_run_their_commands() {
    let Some(mut h) = game_harness("top-bar-verify") else { return };
    settle(&mut h);
    // (What Verify Module from the Build menu says, the Top Bar's says.)
    let verify = |h: &mut Harness<'_, Moonglow>, by_bar: bool| {
        let from = h.state().log.entries.len();
        if by_bar {
            tool(h, "Verify").click();
        } else {
            menu_command(h, "Build", "Verify Module");
        }
        settle(h);
        let said: Vec<String> =
            h.state().log.entries[from..].iter().map(|(_, m)| m.clone()).collect();
        said
    };
    let by_menu = verify(&mut h, false);
    assert!(!by_menu.is_empty(), "Verify Module reports");
    assert_eq!(verify(&mut h, true), by_menu, "the Top Bar's Verify runs Verify Module");
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::ModuleProperties).is_none());
    let props = module_tool(&h, "Properties").rect();
    assert!(tip_at(&mut h, props).is_some_and(|t| t.contains("Module properties")));
    module_tool(&h, "Properties").click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::ModuleProperties).is_some(), "Module Properties");
    module_tool(&h, "New Area").click();
    settle(&mut h);
    assert!(
        matches!(h.state().wizard, Some(mg_ui::Wizard::NewArea(_))),
        "the Area Wizard: {:?} {:?}",
        module_tool(&h, "New Area").rect(),
        h.state().log.entries.last()
    );
}

/// Undo on the Top Bar is the side editor's the pointer came from (a
/// hak's), and, with the pointer come from none, the side editor's in
/// front of the pane with the focus (the active editor); with the focus
/// on the middle, the module's.
#[test]
fn undo_on_the_top_bar_falls_back_to_the_active_side_editor() {
    let dir = mg_testkit::scratch_dir("ui-top-bar-undo-active");
    let file = dir.join("mg_one.2da");
    std::fs::write(&file, "2DA V2.0\n\n   Label\n0  one\n").unwrap();
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(b"2DA V2.0\n\n   A\n".to_vec()) }],
    )));
    let mut h = harness(app, 1800.0, 1000.0);
    h.run();
    mg_ui::hak_view::new_hak(h.state_mut());
    h.run();
    h.state_mut().haks[0].hak.add_files(std::slice::from_ref(&file));
    settle(&mut h);
    let items = |h: &Harness<'_, Moonglow>| h.state().haks[0].hak.items().len();
    let module_step =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    assert_eq!((items(&h), module_step(&h).as_deref()), (1, Some("A table")));
    let hak = h.state().dock.find_tab(&Tab::Hak(h.state().haks[0].id)).expect("the hak's tab");
    let hak = egui_dock::NodePath { surface: hak.surface, node: hak.node };
    // The pointer over the module tree (no side editor), then straight to
    // the Top Bar's Undo, the hak's window having the focus: the hak's.
    h.state_mut().dock.set_focused_node_and_surface(hak);
    let tree = h.get_by_label("Areas (1)").rect().center();
    h.hover_at(tree);
    h.run();
    let undo = tool(&h, "Undo").rect().center();
    h.hover_at(undo);
    h.run();
    click_pos(&h, undo);
    h.run_steps(2);
    assert_eq!(items(&h), 0, "the active hak's step undone");
    assert_eq!(module_step(&h).as_deref(), Some("A table"), "not the module's");
    // The focus on the middle: the module's.
    let middle = h.state().dock.iter_leaves().find(|(p, _)| p.surface.is_main()).unwrap().0;
    h.state_mut().dock.set_focused_node_and_surface(middle);
    h.hover_at(tree);
    h.run();
    h.hover_at(undo);
    h.run();
    click_pos(&h, undo);
    h.run_steps(2);
    assert_eq!(items(&h), 0);
    assert_ne!(module_step(&h).as_deref(), Some("A table"), "the module's step undone");
}

/// The fold button's right edge, which follows the Left Dock's width.
fn fold_edge(h: &Harness<'_, Moonglow>) -> f32 {
    h.get_by_label("Left Dock").get_by_label("Fold the Left Dock").rect().right()
}

/// Drags from `from` by `by`, a frame a step.
fn drag(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, by: egui::Vec2) {
    h.hover_at(from);
    h.run();
    press(h, from, true, egui::Modifiers::NONE);
    h.run();
    for k in 1..=4 {
        h.hover_at(from + by * k as f32 / 4.0);
        h.run();
    }
    press(h, from + by, false, egui::Modifiers::NONE);
    settle(h);
}

/// The Left Dock folds to its strip by its button and by its key
/// (Ctrl+Alt+1), and unfolds by the strip's chevron, by the strip's
/// Module button and by the key, each time as wide as it was left.
#[test]
fn the_left_dock_folds_to_its_strip_and_back_as_wide() {
    let dir = mg_testkit::scratch_dir("ui-left-dock-fold");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    // Widened by its edge, half way down (the button's at the top).
    let edge = fold_edge(&h) + 3.0;
    let y = h.ctx.content_rect().center().y;
    drag(&mut h, egui::pos2(edge, y), egui::vec2(80.0, 0.0));
    let wide = fold_edge(&h);
    assert!(wide > edge + 60.0, "widened: {edge} -> {wide}");
    let folded = |h: &Harness<'_, Moonglow>| {
        let strip = h.query_by_label("Show the Left Dock").is_some();
        assert_eq!(strip, h.query_by_label("Fold the Left Dock").is_none(), "one or the other");
        assert_eq!(strip, h.state().settings.hide_tree);
        strip
    };
    // By its button, back by the chevron.
    h.get_by_label("Fold the Left Dock").click();
    settle(&mut h);
    assert!(folded(&h));
    let strip = h.get_by_label("Left Dock");
    assert!(strip.query_by_label("Module").is_some(), "the strip's Module button");
    let chevron = h.get_by_label("Show the Left Dock");
    assert_eq!(chevron.accesskit_node().data().is_expanded(), Some(false));
    // (The window's frame round it in a test: 8 points.)
    let edge = h.ctx.content_rect().left() + 8.0;
    let at = chevron.rect();
    assert!(at.left() >= edge && at.right() < edge + 26.0, "a strip at the window's edge: {at:?}");
    chevron.click();
    settle(&mut h);
    assert!(!folded(&h));
    assert_eq!(fold_edge(&h), wide, "as wide as it was");
    // By its key, back by the strip's Module button.
    h.key_press_modifiers(keys, egui::Key::Num1);
    settle(&mut h);
    assert!(folded(&h));
    h.get_by_label("Left Dock").get_by_label("Module").click();
    settle(&mut h);
    assert!(!folded(&h));
    assert_eq!(fold_edge(&h), wide);
    // By its key both ways.
    h.key_press_modifiers(keys, egui::Key::Num1);
    settle(&mut h);
    assert!(folded(&h));
    h.key_press_modifiers(keys, egui::Key::Num1);
    settle(&mut h);
    assert!(!folded(&h));
    assert_eq!(fold_edge(&h), wide);
    // Its tab and its tree.
    let tab = h.get_by_label("Left Dock").get_by_label("Module");
    assert_eq!(tab.accesskit_node().role(), egui::accesskit::Role::Tab);
    assert_eq!(tab.accesskit_node().is_selected(), Some(true));
    h.get_by_label("Areas (1)");
}

/// The Log's switch in the Status Bar opens and closes the Bottom Dock,
/// says whether it is on, and the dock comes back as high as it was left;
/// its key (Ctrl+Alt+3) does the same.
#[test]
fn the_log_switch_opens_and_closes_the_bottom_dock_as_high() {
    let mut h = harness(app_with(Vec::new()), 1000.0, 700.0);
    settle(&mut h);
    let switch = |h: &Harness<'_, Moonglow>| {
        let node = h.get_by_label("Status Bar").get_by_label("Log");
        node.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
    };
    let top = |h: &Harness<'_, Moonglow>| {
        h.query_by_label("Bottom Dock").map(|d| d.get_by_label("Log").rect().top())
    };
    assert!(switch(&h) && top(&h).is_some(), "open at first (settings from before)");
    // Raised by its edge.
    let was = top(&h).unwrap();
    drag(&mut h, egui::pos2(500.0, was - 1.0), egui::vec2(0.0, -90.0));
    let high = top(&h).unwrap();
    assert!(high < was - 60.0, "raised: {was} -> {high}");
    h.get_by_label("Status Bar").get_by_label("Log").click();
    settle(&mut h);
    assert!(!switch(&h) && top(&h).is_none() && h.state().settings.hide_log, "closed");
    h.get_by_label("Status Bar").get_by_label("Log").click();
    settle(&mut h);
    assert!(switch(&h) && !h.state().settings.hide_log);
    assert_eq!(top(&h), Some(high), "as high as it was");
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    h.key_press_modifiers(keys, egui::Key::Num3);
    settle(&mut h);
    assert!(!switch(&h) && top(&h).is_none());
    h.key_press_modifiers(keys, egui::Key::Num3);
    settle(&mut h);
    assert!(switch(&h) && top(&h) == Some(high));
    // Its tab, selected, and the log's lines under it.
    let tab = h.get_by_label("Bottom Dock").get_by_label("Log");
    assert_eq!(tab.accesskit_node().role(), egui::accesskit::Role::Tab);
    assert_eq!(tab.accesskit_node().is_selected(), Some(true));
    let line = h.get_by_label_contains("No Neverwinter Nights").rect();
    assert!(line.top() >= tab.rect().bottom(), "{line:?} under {:?}", tab.rect());
}

/// Settings kept before the shell (no field of its own) open with the
/// module tree shown and the log open, as they did; kept with the tree
/// folded or the log closed, the Left Dock folded or the Bottom Dock
/// closed.
#[test]
fn settings_from_before_open_with_the_tree_shown_and_the_log_open() {
    let dir = mg_testkit::scratch_dir("ui-shell-old-settings");
    let path = sample_module(&dir);
    let open = |settings: mg_ui::Settings| {
        let mut app = app_with(Vec::new());
        app.settings = settings;
        app.settings.no_last_area = true;
        app.open_module(&path);
        let mut h = harness(app, 1200.0, 800.0);
        settle(&mut h);
        let tree = h.query_by_label("Fold the Left Dock").is_some();
        let log = h.query_by_label("Bottom Dock").is_some();
        (tree, log)
    };
    let json = |text: &str| serde_json::from_str::<mg_ui::Settings>(text).unwrap();
    // (As the desktop app keeps them: RON, with only the fields it had.)
    let kept: mg_ui::Settings = ron::from_str("(light_theme: false, recent: [])").unwrap();
    assert_eq!(open(kept), (true, true), "as before");
    assert_eq!(open(json(r#"{"light_theme":false}"#)), (true, true), "as before");
    assert_eq!(open(json(r#"{"hide_tree":true}"#)), (false, true), "the Left Dock folded");
    assert_eq!(open(json(r#"{"hide_log":true}"#)), (true, false), "the Bottom Dock closed");
    let s: mg_ui::Settings = serde_json::from_str("{}").unwrap();
    assert!(!s.hide_tree && !s.hide_log && !s.toolbar_icons_only);
}

/// The Status Bar names the module (its file's name), says "(modified)"
/// after an edit and not after Save, gives its path in the tooltip; and
/// names the game folder, or says there is none.
#[test]
fn the_status_bar_names_the_module_and_says_when_it_is_modified() {
    let dir = mg_testkit::scratch_dir("ui-status-bar-module");
    let path = sample_module(&dir);
    let mut h = harness(app_with(Vec::new()), 1400.0, 800.0);
    settle(&mut h);
    let bar = |h: &Harness<'_, Moonglow>, name: &str| {
        h.get_by_label("Status Bar").query_by_label(name).map(|n| n.rect())
    };
    assert!(bar(&h, "No module open").is_some());
    assert!(bar(&h, "No game install").is_some());
    h.state_mut().open_module(&path);
    settle(&mut h);
    let name = bar(&h, "sample").expect("the module's name");
    let tip = tip_at(&mut h, name).unwrap_or_default();
    assert_eq!(tip, path.display().to_string(), "its path in the tip");
    let edit = mg_edit::Edit::SetResource {
        key: ResKey::parse("added", ResType::NSS).unwrap(),
        data: Some(b"void main() { }\n".to_vec()),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Add", vec![edit])));
    settle(&mut h);
    assert!(bar(&h, "sample (modified)").is_some() && bar(&h, "sample").is_none());
    tool(&h, "Save").click();
    settle(&mut h);
    assert!(bar(&h, "sample").is_some() && bar(&h, "sample (modified)").is_none());
}

/// View's rows for the docks work and say their state: Module Tree
/// ticked while the Left Dock is unfolded, Log while the Bottom Dock is
/// open; chosen, they fold and close them, and say so.
#[test]
fn the_view_menu_s_rows_fold_the_docks_and_say_so() {
    let dir = mg_testkit::scratch_dir("ui-shell-view-menu");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    settle(&mut h);
    let ticked = |h: &mut Harness<'_, Moonglow>, row: &str| {
        h.get_by_label("Top Bar").get_by_label("View").click();
        settle(h);
        let tick = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::CheckBox && n.label().as_deref() == Some(row)
        };
        let on = h.get(egui_kittest::kittest::by().predicate(tick)).accesskit_node().toggled();
        h.key_press(egui::Key::Escape);
        settle(h);
        on == Some(egui::accesskit::Toggled::True)
    };
    assert!(ticked(&mut h, "Module Tree") && ticked(&mut h, "Log"));
    menu_command(&mut h, "View", "Module Tree");
    assert!(h.state().settings.hide_tree && h.query_by_label("Show the Left Dock").is_some());
    assert!(!ticked(&mut h, "Module Tree"));
    menu_command(&mut h, "View", "Log");
    assert!(h.state().settings.hide_log && h.query_by_label("Bottom Dock").is_none());
    assert!(!ticked(&mut h, "Log"));
    let off = h.get_by_label("Status Bar").get_by_label("Log").accesskit_node().toggled();
    assert_eq!(off, Some(egui::accesskit::Toggled::False), "the switch says so too");
    menu_command(&mut h, "View", "Module Tree");
    menu_command(&mut h, "View", "Log");
    assert!(ticked(&mut h, "Module Tree") && ticked(&mut h, "Log"));
    assert!(h.query_by_label("Fold the Left Dock").is_some());
}

/// At 760 points wide no control of the Top Bar, the Status Bar or the
/// Left Dock's tab bar overlaps another or leaves the window, and the
/// build actions are icons alone (under 1300 they lose their words); at
/// 1600 they have their words. Narrower than the Top Bar holds them all
/// (600), the build actions go whole, rather than overlap: the Build
/// menu has them.
#[test]
fn at_760_wide_the_bars_controls_neither_overlap_nor_leave_the_window() {
    let dir = mg_testkit::scratch_dir("ui-shell-narrow");
    let path = sample_module(&dir);
    for (width, words) in [(760.0, Some(false)), (1600.0, Some(true)), (600.0, None)] {
        let mut app = app_with(Vec::new());
        app.settings.no_last_area = true;
        app.open_module(&path);
        let mut h = harness(app, width, 600.0);
        settle(&mut h);
        let window = h.ctx.content_rect();
        for bar in ["Top Bar", "Status Bar", "Left Dock"] {
            let group = h.get_by_label(bar);
            // (Its buttons, tabs and labels: named, or a label's words.)
            let words = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.label().or(n.value());
            let rects: Vec<(String, egui::Rect)> = group
                .query_all_by(|n| n.bounding_box().is_some() && words(n).is_some())
                .map(|n| (words(&n.accesskit_node()).unwrap_or_default(), n.rect()))
                .filter(|(_, r)| r.area() > 0.0)
                .collect();
            assert!(rects.len() >= 3, "{bar}'s controls at {width}: {rects:?}");
            for (i, (a, ra)) in rects.iter().enumerate() {
                assert!(
                    window.contains_rect(*ra),
                    "{bar} at {width}: {a} {ra:?} leaves the window"
                );
                for (b, rb) in &rects[i + 1..] {
                    let both = ra.intersect(*rb);
                    let overlap = both.is_positive() && both.area() > 1.0;
                    assert!(!overlap, "{bar} at {width}: {a} {ra:?} and {b} {rb:?} overlap");
                }
            }
        }
        for name in BUILD {
            let Some(words) = words else {
                let gone = h.get_by_label("Top Bar").query_by_label(name).is_none();
                assert!(gone, "{name} at {width}: gone");
                continue;
            };
            let w = tool(&h, name).rect().width();
            let alone = w <= mg_uikit::metrics::ICON_BUTTON + 0.5;
            assert_eq!(alone, !words, "{name} at {width}: {w} wide");
        }
        for menu in ["File", "Edit", "View", "Build", "Help"] {
            tool(&h, menu);
        }
        // (The Top Bar 30 high, the Status Bar 24, whatever the width: their
        // controls centered in them, inside the window's frame in a test,
        // 8 points.)
        let file = h.get_by_label("Top Bar").get_by_label("File").rect();
        let top = window.top() + 8.0 + mg_uikit::metrics::TOP_BAR / 2.0;
        assert!((file.center().y - top).abs() <= 1.0, "{file:?}");
        let log = h.get_by_label("Status Bar").get_by_label("Log").rect();
        let bottom = window.bottom() - 8.0 - mg_uikit::metrics::STATUS_BAR / 2.0;
        assert!((log.center().y - bottom).abs() <= 1.0, "{log:?}");
    }
}
