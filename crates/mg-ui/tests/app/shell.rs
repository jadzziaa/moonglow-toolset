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
    let props = module_tool(&h, "Module Properties").rect();
    assert!(tip_at(&mut h, props).is_some_and(|t| t.contains("Open the module's properties")));
    module_tool(&h, "Module Properties").click();
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

/// The module's step Undo takes back, if one.
fn module_step(h: &Harness<'_, Moonglow>) -> Option<String> {
    h.state().ws.as_ref().unwrap().can_undo().map(str::to_string)
}

/// The module's step Redo takes again, if one.
fn module_redo(h: &Harness<'_, Moonglow>) -> Option<String> {
    h.state().ws.as_ref().unwrap().can_redo().map(str::to_string)
}

/// The sample module open in a 1800 by 1000 window with one step of its
/// own ("A table"), and a new hak beside it (its tab in front and
/// focused), with one step of its own ("Add files") where `hak_steps`.
/// Nothing pressed yet.
fn undo_harness(name: &str, hak_steps: bool) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(name);
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
    settle(&mut h);
    mg_ui::hak_view::new_hak(h.state_mut());
    settle(&mut h);
    if hak_steps {
        h.state_mut().haks[0].hak.add_files(std::slice::from_ref(&file));
        settle(&mut h);
    }
    let hak = hak_tab(&h);
    let at = h.state().dock.find_tab(&hak).expect("the hak's tab");
    let pane = egui_dock::NodePath { surface: at.surface, node: at.node };
    h.state_mut().dock.set_focused_node_and_surface(pane);
    settle(&mut h);
    assert_eq!(module_step(&h).as_deref(), Some("A table"));
    assert_eq!(hak_items(&h), usize::from(hak_steps));
    h
}

fn hak_tab(h: &Harness<'_, Moonglow>) -> Tab {
    Tab::Hak(h.state().haks[0].id)
}

fn hak_items(h: &Harness<'_, Moonglow>) -> usize {
    h.state().haks[0].hak.items().len()
}

/// A point inside the pane that shows `tab` (low in it, clear of its
/// buttons and rows), where a press is work in it.
fn inside(h: &Harness<'_, Moonglow>, tab: &Tab) -> egui::Pos2 {
    let at = h.state().dock.find_tab(tab).expect("the tab");
    let pane = egui_dock::NodePath { surface: at.surface, node: at.node };
    let view = h.state().dock.leaf(pane).unwrap().viewport;
    egui::pos2(view.center().x, view.bottom() - 40.0)
}

/// A point on `tab`'s tab, clear of its close button (egui_dock's tabs
/// are what can be clicked and dragged in their pane's tab bar, in their
/// order: `click_tab_close`'s way).
fn tab_title(h: &Harness<'_, Moonglow>, tab: &Tab) -> egui::Pos2 {
    let (band, index) = (h.state().dock.iter_leaves())
        .find_map(|(_, l)| {
            let index = l.tabs.iter().position(|t| t == tab)?;
            let band =
                egui::Rect::from_x_y_ranges(l.rect.x_range(), l.rect.top()..=l.viewport.top());
            Some((band, index))
        })
        .expect("the tab is open");
    let mut tabs = Vec::new();
    h.ctx.viewport(|v| {
        for (_, widgets) in v.prev_pass.widgets.layers() {
            for w in widgets.iter().filter(|w| band.contains_rect(w.interact_rect)) {
                if w.sense.senses_click() && w.sense.senses_drag() {
                    tabs.push(w.interact_rect);
                }
            }
        }
    });
    tabs.sort_by(|a, b| a.left().total_cmp(&b.left()));
    let own = tabs.get(index).expect("its tab");
    egui::pos2(own.left() + 12.0, own.center().y)
}

/// The module tree's Areas row: a place in the Left Dock.
fn tree_row(h: &Harness<'_, Moonglow>) -> egui::Pos2 {
    h.get_by_label("Areas (1)").rect().center()
}

/// Works at `at`: the pointer pressed and let go there.
fn work_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    settle(h);
    click_pos(h, at);
    settle(h);
}

/// The pointer from `from` (no press) onto the Top Bar's Undo.
fn to_top_bar(h: &mut Harness<'_, Moonglow>, from: egui::Pos2) {
    h.hover_at(from);
    settle(h);
    h.hover_at(tool(h, "Undo").rect().center());
    settle(h);
}

/// One of the three ways of asking for Undo (or Redo) with the pointer
/// on the Top Bar.
#[derive(Clone, Copy, Debug)]
enum Way {
    Button,
    Menu,
    Key,
}

/// What Undo (`redo`: Redo) is with the pointer come from `from` onto the
/// Top Bar: the button's words (its tip, without its key) where it is
/// enabled, after checking that Edit's row says the same and is enabled
/// as the button is. None: both disabled.
fn undo_said(h: &mut Harness<'_, Moonglow>, redo: bool, from: egui::Pos2) -> Option<String> {
    let name = if redo { "Redo" } else { "Undo" };
    to_top_bar(h, from);
    let rect = tool(h, name).rect();
    let enabled = !disabled(&tool(h, name));
    let tip = tip_at(h, rect).unwrap_or_default();
    let key = if redo { " (Ctrl+Shift+Z)" } else { " (Ctrl+Z)" };
    let words = tip.strip_suffix(key).unwrap_or_else(|| panic!("{name}'s tip: {tip}")).to_string();
    to_top_bar(h, from);
    h.get_by_label("Top Bar").get_by_label("Edit").click();
    settle(h);
    let (label, row_enabled) = edit_row(h, name);
    h.key_press(egui::Key::Escape);
    settle(h);
    let row_words = label.strip_suffix(key.trim_start_matches(" (").trim_end_matches(')'));
    let row_words = row_words.map(str::trim_end).unwrap_or(&label).to_string();
    assert_eq!(row_words, words, "Edit › {name} and the button say one step");
    assert_eq!(row_enabled, enabled, "Edit › {name} enabled as the button: {words}");
    enabled.then_some(words)
}

/// The open Edit menu's row `name` (Undo, Redo): the button whose words
/// begin so and go on with its step or its key, outside the bars (not an
/// editor's own Undo button, named "Undo" alone).
fn edit_row_node<'t>(h: &'t Harness<'_, Moonglow>, name: &str) -> egui_kittest::Node<'t> {
    let begins = format!("{name} ");
    let bars = ["Top Bar", "Status Bar", "Left Dock", "Bottom Dock"];
    h.query_all_by(move |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button
            && n.label().is_some_and(|l| l.starts_with(&begins))
    })
    .filter(|n| !bars.iter().any(|b| in_group(n, b)))
    .min_by_key(|n| n.accesskit_node().label().unwrap_or_default().len())
    .unwrap_or_else(|| panic!("Edit › {name}"))
}

/// The open Edit menu's row `name`: its words (and key), and whether it
/// is enabled.
fn edit_row(h: &Harness<'_, Moonglow>, name: &str) -> (String, bool) {
    let n = edit_row_node(h, name);
    (n.accesskit_node().label().unwrap_or_default(), !disabled(&n))
}

fn edit_row_at(h: &Harness<'_, Moonglow>, name: &str) -> egui::Pos2 {
    edit_row_node(h, name).rect().center()
}

/// Undo (`redo`: Redo) asked for `way`, with the pointer come from `from`
/// onto the Top Bar.
fn undo_by(h: &mut Harness<'_, Moonglow>, way: Way, redo: bool, from: egui::Pos2) {
    let name = if redo { "Redo" } else { "Undo" };
    to_top_bar(h, from);
    match way {
        Way::Button => {
            let at = tool(h, name).rect().center();
            click_pos(h, at);
        }
        Way::Menu => {
            h.get_by_label("Top Bar").get_by_label("Edit").click();
            settle(h);
            let at = edit_row_at(h, name);
            click_pos(h, at);
        }
        Way::Key => {
            let shift = if redo { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
            h.key_press_modifiers(egui::Modifiers::COMMAND | shift, egui::Key::Z);
        }
    }
    settle(h);
}

/// With the pointer come from `from` onto the Top Bar, Undo and Redo are
/// the module's (its step "A table"), however asked for: the button, Edit
/// › Undo and Ctrl+Z say it and do it, and Redo does it again; the hak's
/// `items` stay.
#[track_caller]
fn undo_is_the_module_s(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, items: Option<usize>) {
    for way in [Way::Button, Way::Menu, Way::Key] {
        assert_eq!(undo_said(h, false, from).as_deref(), Some("Undo A table"), "{way:?}");
        undo_by(h, way, false, from);
        assert_eq!(module_step(h), None, "{way:?}: the module's step undone");
        assert_eq!(items.map(|_| hak_items(h)), items, "{way:?}: the hak's stays");
        assert_eq!(undo_said(h, true, from).as_deref(), Some("Redo A table"), "{way:?}");
        undo_by(h, way, true, from);
        assert_eq!(module_step(h).as_deref(), Some("A table"), "{way:?}: done again");
        assert_eq!(module_redo(h), None);
        assert_eq!(items.map(|_| hak_items(h)), items);
    }
}

/// With the pointer come from `from` onto the Top Bar, Undo and Redo are
/// the hak's (its step "Add files"), however asked for, and the module's
/// step stays.
#[track_caller]
fn undo_is_the_hak_s(h: &mut Harness<'_, Moonglow>, from: egui::Pos2) {
    for way in [Way::Button, Way::Menu, Way::Key] {
        assert_eq!(undo_said(h, false, from).as_deref(), Some("Undo Add files"), "{way:?}");
        undo_by(h, way, false, from);
        assert_eq!(hak_items(h), 0, "{way:?}: the hak's step undone");
        assert_eq!(module_step(h).as_deref(), Some("A table"), "{way:?}: not the module's");
        assert_eq!(undo_said(h, true, from).as_deref(), Some("Redo Add files"), "{way:?}");
        undo_by(h, way, true, from);
        assert_eq!(hak_items(h), 1, "{way:?}: done again");
        assert_eq!(module_step(h).as_deref(), Some("A table"));
    }
}

/// The Top Bar's Undo (and Edit › Undo, and Ctrl+Z with the pointer on
/// the Top Bar) acts on the active editor, the one the user last worked
/// in: after a press in the module tree, the module's, even with an
/// empty hak's window in front and focused (not disabled, its tip naming
/// the module's step) and with a hak that has a step of its own (that
/// step stays); after a press in the hak's editor, the hak's, the pointer
/// come to the Top Bar over the tree without a press there (moving over a
/// view is not working in it); after a press in the tree again, the
/// module's.
#[test]
fn undo_on_the_top_bar_is_the_active_editor_s() {
    // An empty hak in front and focused, nothing pressed yet: the module's.
    let mut h = undo_harness("ui-top-bar-undo-empty-hak", false);
    let tree = tree_row(&h);
    undo_is_the_module_s(&mut h, tree, None);
    work_at(&mut h, tree);
    undo_is_the_module_s(&mut h, tree, None);
    // Worked in the empty hak: its Undo, with nothing to take back, grayed
    // (the side editors' rule: never the module's step instead), however
    // asked for; the module's step stays.
    let empty = inside(&h, &hak_tab(&h));
    work_at(&mut h, empty);
    for way in [Way::Button, Way::Menu, Way::Key] {
        assert_eq!(undo_said(&mut h, false, tree), None, "{way:?}: grayed");
        if !matches!(way, Way::Menu) {
            undo_by(&mut h, way, false, tree);
        }
        assert_eq!(module_step(&h).as_deref(), Some("A table"), "{way:?}: the module's stays");
    }
    work_at(&mut h, tree);
    undo_is_the_module_s(&mut h, tree, None);
    // A hak with a step: the module's after the tree, the hak's after it.
    let mut h = undo_harness("ui-top-bar-undo-active", true);
    let tree = tree_row(&h);
    work_at(&mut h, tree);
    undo_is_the_module_s(&mut h, tree, Some(1));
    let hak = inside(&h, &hak_tab(&h));
    work_at(&mut h, hak);
    assert_eq!((hak_items(&h), module_step(&h).as_deref()), (1, Some("A table")), "a press");
    undo_is_the_hak_s(&mut h, tree);
    // (Pressed on its tab, the same.)
    work_at(&mut h, tree);
    undo_is_the_module_s(&mut h, tree, Some(1));
    let tab = tab_title(&h, &hak_tab(&h));
    work_at(&mut h, tab);
    undo_is_the_hak_s(&mut h, tree);
    // Over the hak's editor, Ctrl+Z is the hak's; over the tree, the
    // module's (the pointer's own rule), and the Top Bar's then too.
    h.hover_at(hak);
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!((hak_items(&h), module_step(&h).as_deref()), (0, Some("A table")));
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut h);
    h.hover_at(tree);
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!((hak_items(&h), module_step(&h)), (1, None), "the module's, over the tree");
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    settle(&mut h);
    undo_is_the_module_s(&mut h, tree, Some(1));
    // Worked in the hak, then another tab put in front of its pane: not an
    // editor out of sight, the module's.
    work_at(&mut h, hak);
    undo_is_the_hak_s(&mut h, tree);
    let at = h.state().dock.find_tab(&hak_tab(&h)).unwrap();
    let pane = egui_dock::NodePath { surface: at.surface, node: at.node };
    let script = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    assert!(h.state().dock.find_tab(&script).is_none());
    h.state_mut().dock.leaf_mut(pane).unwrap().append_tab(script.clone());
    settle(&mut h);
    let leaf = h.state().dock.leaf(pane).unwrap();
    assert_eq!(leaf.tabs[leaf.active.0], script, "in front of the hak's");
    undo_is_the_module_s(&mut h, tree, Some(1));
}

/// After work in an area's view the Top Bar's Undo is the module's, with
/// a 2DA's window open beside it; after work in the 2DA's editor, the
/// 2DA's; with no side editor open at all, the module's.
#[test]
fn undo_on_the_top_bar_after_an_area_is_the_module_s() {
    let dir = mg_testkit::scratch_dir("ui-top-bar-undo-area");
    let file = dir.join("loose.2da");
    std::fs::write(&file, SMALL_TABLE).unwrap();
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(b"2DA V2.0\n\n   A\n".to_vec()) }],
    )));
    let area = Tab::Area(ResRef::from_str("start").unwrap());
    app.actions.push(mg_ui::Action::OpenTab(area.clone()));
    let mut h = harness(app, 1800.0, 1000.0);
    settle(&mut h);
    let tree = tree_row(&h);
    // No side editor at all: the module's, before and after a press.
    let in_area = inside(&h, &area);
    undo_is_the_module_s(&mut h, tree, None);
    work_at(&mut h, in_area);
    undo_is_the_module_s(&mut h, in_area, None);
    // A 2DA's window open (in front, focused): the area's work is the
    // module's still.
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    settle(&mut h);
    let table = Tab::Table(id);
    own_window(&mut h, &table, egui::pos2(900.0, 300.0), egui::vec2(600.0, 400.0));
    settle(&mut h);
    let in_area = inside(&h, &area);
    work_at(&mut h, in_area);
    undo_is_the_module_s(&mut h, in_area, None);
    // Worked in the 2DA (a row added, as its editor does): its own, and
    // the module's step stays.
    let in_table = inside(&h, &table);
    work_at(&mut h, in_table);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().edit.add_rows(1);
    settle(&mut h);
    let rows = |h: &Harness<'_, Moonglow>| table_of(h, id).len();
    let before = rows(&h);
    for way in [Way::Button, Way::Menu, Way::Key] {
        let said = undo_said(&mut h, false, tree);
        assert_eq!(said, Some(format!("Undo Add row {}", before - 1)), "{way:?}: the 2DA's");
        undo_by(&mut h, way, false, tree);
        assert_eq!(rows(&h), before - 1, "{way:?}: the 2DA's step undone");
        assert_eq!(module_step(&h).as_deref(), Some("A table"), "{way:?}");
        undo_by(&mut h, way, true, tree);
        assert_eq!(rows(&h), before, "{way:?}: and done again");
    }
    // Back in the area: the module's.
    work_at(&mut h, in_area);
    undo_is_the_module_s(&mut h, tree, None);
    // A cell clicked (the grid has the keys): the 2DA's; a press in the
    // area after it, or in the tree, the module's again.
    let cell = h.get_by_label("Bread").rect().center();
    work_at(&mut h, cell);
    let said = undo_said(&mut h, false, tree);
    assert_eq!(said, Some(format!("Undo Add row {}", before - 1)), "the grid's keys");
    work_at(&mut h, in_area);
    undo_is_the_module_s(&mut h, tree, None);
    work_at(&mut h, cell);
    work_at(&mut h, tree);
    undo_is_the_module_s(&mut h, tree, None);
}

/// The talk table's editor (a file of its own): after work in it (a line
/// added by its Add Line), the Top Bar's Undo, Edit › Undo and Ctrl+Z on
/// the Top Bar all say plain Undo (its steps have no names) and take its
/// step back, Redo gives it again, and the module's step stays; after a
/// press in the tree, they are the module's.
#[test]
fn undo_on_the_top_bar_after_the_talk_table_is_its() {
    use mg_module::talk::{Source, Table};
    let dir = mg_testkit::scratch_dir("ui-top-bar-undo-talk");
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
    settle(&mut h);
    let mut table = Table::create("words", &dir, Language::ENGLISH, false);
    table.source = Source::File(dir.join("words.tlk"));
    table.save().unwrap();
    h.state_mut().talk = Some(table);
    h.state_mut().talk_view.outside = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    settle(&mut h);
    let lines = |h: &Harness<'_, Moonglow>| h.state().talk.as_ref().unwrap().len();
    let tree = tree_row(&h);
    // Nothing worked in yet: the module's.
    undo_is_the_module_s(&mut h, tree, None);
    let before = lines(&h);
    let add = h.get_by_label("Add Line").rect().center();
    work_at(&mut h, add);
    assert_eq!(lines(&h), before + 1, "a line added");
    for way in [Way::Button, Way::Menu, Way::Key] {
        assert_eq!(undo_said(&mut h, false, tree).as_deref(), Some("Undo"), "{way:?}");
        undo_by(&mut h, way, false, tree);
        assert_eq!(lines(&h), before, "{way:?}: the talk table's step undone");
        assert_eq!(module_step(&h).as_deref(), Some("A table"), "{way:?}: not the module's");
        assert_eq!(undo_said(&mut h, true, tree).as_deref(), Some("Redo"), "{way:?}");
        undo_by(&mut h, way, true, tree);
        assert_eq!(lines(&h), before + 1, "{way:?}: given again");
    }
    work_at(&mut h, tree);
    undo_is_the_module_s(&mut h, tree, None);
    assert_eq!(lines(&h), before + 1, "the talk table's line stays");
}

/// A hak's editor in a floating window: worked in, the Top Bar's Undo is
/// the hak's; after the tree, the module's, the window still in front.
#[test]
fn undo_on_the_top_bar_with_a_floating_editor() {
    let mut h = undo_harness("ui-top-bar-undo-floating", true);
    let hak = hak_tab(&h);
    own_window(&mut h, &hak, egui::pos2(900.0, 300.0), egui::vec2(700.0, 500.0));
    settle(&mut h);
    assert!(!h.state().dock.find_tab(&hak).unwrap().surface.is_main(), "a window of its own");
    let tree = tree_row(&h);
    undo_is_the_module_s(&mut h, tree, Some(1));
    let in_hak = inside(&h, &hak);
    work_at(&mut h, in_hak);
    undo_is_the_hak_s(&mut h, tree);
    work_at(&mut h, tree);
    undo_is_the_module_s(&mut h, tree, Some(1));
}

/// The Left Dock's inner edge (the middle of the 1 px line along it):
/// its fold button stands beside it, from the line's outer side.
fn fold_edge(h: &Harness<'_, Moonglow>) -> f32 {
    h.get_by_label("Fold the Left Dock").rect().left() - 0.5
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
/// (Ctrl+Alt+1), and unfolds by the button beside the strip's edge (the
/// strip's chevron until the review of M3 T4), by the strip's Module
/// button and by the key, each time as wide as it was left.
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
    // Widened by its edge, above its button (which is half way down).
    let edge = fold_edge(&h) + 1.0;
    let y = h.ctx.content_rect().top() + 150.0;
    drag(&mut h, egui::pos2(edge, y), egui::vec2(80.0, 0.0));
    let wide = fold_edge(&h);
    assert!(wide > edge + 60.0, "widened: {edge} -> {wide}");
    let folded = |h: &Harness<'_, Moonglow>| {
        let strip = h.query_by_label("Show the Left Dock").is_some();
        assert_eq!(strip, h.query_by_label("Fold the Left Dock").is_none(), "one or the other");
        assert_eq!(strip, h.state().settings.hide_tree);
        strip
    };
    // By its button, back by the one beside the strip's edge.
    h.get_by_label("Fold the Left Dock").click();
    settle(&mut h);
    assert!(folded(&h));
    let strip = h.get_by_label("Left Dock");
    assert!(strip.query_by_label("Module").is_some(), "the strip's Module button");
    let chevron = h.get_by_label("Show the Left Dock");
    assert_eq!(chevron.accesskit_node().data().is_expanded(), Some(false));
    // (The window's frame round it in a test: 8 points; the strip 24
    // points and its line, the button beside it.)
    let edge = h.ctx.content_rect().left() + 8.0;
    let at = chevron.rect();
    let beside = at.left() - (edge + mg_uikit::shell::STRIP + 1.0);
    assert!(beside.abs() <= 1.0, "beside a strip at the window's edge: {at:?}");
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

/// Dragged as narrow as it goes, the Left Dock is 190 points wide (its
/// tabs as icons and the Module panel's four buttons), and nothing on its
/// tab bar overlaps. From there to 600 points (as far as the window lets
/// it), a point at a time, on both of its tabs: nothing of the tab bar is
/// past the dock's inner edge or overlaps, the Module panel's buttons are
/// all on it, the tabs show their words or their icons the same on every
/// frame at one width, icons below a width and words above it; and its
/// fold button stands on the edge, half way down the dock. So too in the
/// least window (420 points wide), where two fifths of the window would be
/// less than 190.
#[test]
fn the_left_dock_s_tab_bar_fits_at_its_least_width() {
    let dir = mg_testkit::scratch_dir("ui-left-dock-least");
    let path = sample_module(&dir);
    for width in [1400.0, 420.0 + TEST_FRAME] {
        let mut app = app_with(Vec::new());
        app.settings.no_last_area = true;
        app.open_module(&path);
        let mut h = harness(app, width, 700.0);
        settle(&mut h);
        // (Above the fold button, half way down.)
        let edge = fold_edge(&h) + 1.0;
        let y = h.ctx.content_rect().top() + 150.0;
        drag(&mut h, egui::pos2(edge, y), egui::vec2(-400.0, 0.0));
        // (The test's window frame, 8 points.)
        let left = h.ctx.content_rect().left() + 8.0;
        let wide = fold_edge(&h) - left;
        assert!((wide - 190.0).abs() <= 2.0, "{width}: the dock {wide} wide");
        let most = (0.4 * (width - TEST_FRAME)).clamp(190.0, 600.0) as usize;
        for panel in [mg_ui::Panel::Module, mg_ui::Panel::Resources] {
            h.state_mut().actions.push(mg_ui::Action::ShowPanel(panel));
            settle(&mut h);
            let mut icons_seen = Vec::new();
            for dock in 190..=most {
                left_dock_width(&h, dock as f32);
                let mut each = Vec::new();
                for _ in 0..2 {
                    h.run();
                    let tab = h.get_by_label("Left Dock").get_by_label("Module").rect();
                    each.push(tab.width() < 40.0);
                }
                let at = format!("{width}, {panel:?} at {dock}");
                assert!(each[0] == each[1], "{at}: words and icons by turns");
                icons_seen.push(each[0]);
                let edge = fold_edge(&h);
                assert!((edge - left - dock as f32).abs() <= 1.5, "{at}: the edge at {edge}");
                let past: Vec<String> = bar_controls(&h, "Left Dock")
                    .into_iter()
                    .filter(|(_, r)| r.right() > edge + 0.5 || r.left() < left - 0.5)
                    .map(|(n, r)| format!("{n} {r:?}"))
                    .collect();
                assert!(past.is_empty(), "{at}: past the dock's edge: {past:?}");
                let wrong: Vec<String> = bars_wrong(&h, true)
                    .into_iter()
                    .filter(|w| w.starts_with("Left Dock"))
                    .collect();
                assert!(wrong.is_empty(), "{at}: {wrong:#?}");
                if panel == mg_ui::Panel::Module {
                    for name in ["Module Properties", "New Area", "Expand All", "Collapse All"] {
                        let r = module_tool(&h, name).rect();
                        assert!(r.left() >= left - 0.5 && r.right() <= edge + 0.5, "{at}: {name}");
                    }
                }
                // The fold button: on the edge, half way down the dock.
                let fold = h.get_by_label("Fold the Left Dock").rect();
                let top = h.get_by_label("Left Dock").get_by_label("Module").rect().top();
                let bottom = h.get_by_label("Status Bar").get_by_label("Log").rect().top();
                assert!(h.ctx.content_rect().contains_rect(fold), "{at}: {fold:?}");
                assert!(fold.top() > top && fold.bottom() < bottom, "{at}: {fold:?}");
                let middle = (top + bottom) / 2.0;
                assert!((fold.center().y - middle).abs() <= 3.0, "{at}: {fold:?} {middle}");
            }
            let words = icons_seen.iter().position(|i| !i).unwrap_or(icons_seen.len());
            assert!(icons_seen[..words].iter().all(|i| *i), "{width}, {panel:?}: icons below");
            assert!(icons_seen[words..].iter().all(|i| !i), "{width}, {panel:?}: words above");
            assert!(icons_seen[0], "{width}, {panel:?}: icons at the least width");
            if most >= 300 {
                assert!(words < icons_seen.len(), "{width}, {panel:?}: words where they fit");
            }
        }
    }
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

/// View's rows for the docks work and say their state: Left Dock (the
/// row was Module Tree until the dock held the Resources panel too)
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
    assert!(ticked(&mut h, "Left Dock") && ticked(&mut h, "Log"));
    menu_command(&mut h, "View", "Left Dock");
    assert!(h.state().settings.hide_tree && h.query_by_label("Show the Left Dock").is_some());
    assert!(!ticked(&mut h, "Left Dock"));
    menu_command(&mut h, "View", "Log");
    assert!(h.state().settings.hide_log && h.query_by_label("Bottom Dock").is_none());
    assert!(!ticked(&mut h, "Log"));
    let off = h.get_by_label("Status Bar").get_by_label("Log").accesskit_node().toggled();
    assert_eq!(off, Some(egui::accesskit::Toggled::False), "the switch says so too");
    menu_command(&mut h, "View", "Left Dock");
    menu_command(&mut h, "View", "Log");
    assert!(ticked(&mut h, "Left Dock") && ticked(&mut h, "Log"));
    assert!(h.query_by_label("Fold the Left Dock").is_some());
}

/// The named controls of the bar `bar` (buttons, tabs, labels: named, or
/// a label's words) and their rectangles.
fn bar_controls(h: &Harness<'_, Moonglow>, bar: &str) -> Vec<(String, egui::Rect)> {
    let Some(group) = h.query_by_label(bar) else { return Vec::new() };
    let words = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.label().or(n.value());
    group
        .query_all_by(|n| n.bounding_box().is_some() && words(n).is_some())
        .map(|n| (words(&n.accesskit_node()).unwrap_or_default(), n.rect()))
        .filter(|(_, r)| r.area() > 0.0)
        .collect()
}

/// What is wrong with the bars' controls: two that overlap, and (where
/// `inside`) one that leaves the window.
fn bars_wrong(h: &Harness<'_, Moonglow>, inside: bool) -> Vec<String> {
    let window = h.ctx.content_rect();
    let mut wrong = Vec::new();
    for bar in ["Top Bar", "Status Bar", "Left Dock", "Bottom Dock"] {
        let rects = bar_controls(h, bar);
        for (i, (a, ra)) in rects.iter().enumerate() {
            if inside && !window.contains_rect(*ra) {
                wrong.push(format!("{bar}: {a} {ra:?} leaves the window"));
            }
            for (b, rb) in &rects[i + 1..] {
                let both = ra.intersect(*rb);
                if both.is_positive() && both.area() > 1.0 {
                    wrong.push(format!("{bar}: {a} {ra:?} and {b} {rb:?} overlap"));
                }
            }
        }
    }
    wrong
}

/// (The window's frame round the application in a test, 8 points a side:
/// a test's window this much wider than the application's.)
const TEST_FRAME: f32 = 16.0;

/// As a window narrows from 1400 to 300 points, the Top Bar loses its
/// build actions first and then its quick actions, each group whole and
/// in that order (never the build actions without the quick ones), and
/// nothing of the bars overlaps; from the least window size on, nothing
/// leaves the window either. Every one of them is in the menus.
#[test]
fn the_top_bar_drops_its_build_then_its_quick_actions_as_the_window_narrows() {
    let dir = mg_testkit::scratch_dir("ui-top-bar-sweep");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 700.0);
    settle(&mut h);
    let least = mg_ui::LEAST_WINDOW_SIZE.x + TEST_FRAME;
    // 2: everything; 1: the quick actions, no build actions; 0: neither.
    let mut was = 2;
    let mut seen = Vec::new();
    for step in (0..=55).rev() {
        let width = 300.0 + 20.0 * step as f32;
        h.set_size(egui::vec2(width, 700.0));
        settle(&mut h);
        let shown = |name: &str| h.get_by_label("Top Bar").query_by_label(name).is_some();
        let quick = QUICK.iter().map(|n| shown(n)).collect::<Vec<_>>();
        let build = BUILD.iter().chain(&[TEST_MENU]).map(|n| shown(n)).collect::<Vec<_>>();
        let whole = |all: &[bool]| all.iter().all(|s| *s) || all.iter().all(|s| !*s);
        assert!(whole(&quick) && whole(&build), "{width}: each group whole: {quick:?} {build:?}");
        let state = match (quick[0], build[0]) {
            (true, true) => 2,
            (true, false) => 1,
            (false, false) => 0,
            (false, true) => panic!("{width}: the build actions without the quick actions"),
        };
        assert!(state <= was, "{width}: {state} after {was}: the order is build, then quick");
        was = state;
        seen.push(state);
        let wrong = bars_wrong(&h, width >= least);
        assert!(wrong.is_empty(), "{width}: {wrong:#?}");
        for menu in ["File", "Edit", "Build", "Help"] {
            assert!(shown(menu), "{width}: {menu}");
        }
    }
    assert_eq!(seen.first(), Some(&2), "everything at 1400");
    assert!(seen.contains(&1), "the quick actions alone at some width: {seen:?}");
    assert_eq!(seen.last(), Some(&0), "neither at 300");
}

/// At the least window size (`LEAST_WINDOW_SIZE`, in points) the menus
/// fit at every interface size Options offers (90% to 200%): none of the
/// bars' controls leaves the window, none overlaps, with a module open
/// and with none. (A test's window is given in pixels: the least size
/// and the test's frame times the interface size.)
#[test]
fn at_the_least_window_size_the_menus_fit() {
    let dir = mg_testkit::scratch_dir("ui-least-window");
    let path = sample_module(&dir);
    let least = mg_ui::LEAST_WINDOW_SIZE + egui::Vec2::splat(TEST_FRAME);
    for scale in mg_ui::UI_SCALES {
        for module in [true, false] {
            let mut app = app_with(Vec::new());
            app.settings.no_last_area = true;
            if module {
                app.open_module(&path);
            }
            let mut h = harness(app, least.x, least.y);
            settle(&mut h);
            // The interface size chosen, then the window made the least
            // size in points at it (the harness's size brought to it).
            h.state_mut().settings.ui_scale = (scale != 100).then_some(scale);
            settle(&mut h);
            let mut raw = least;
            for _ in 0..6 {
                let content = h.ctx.content_rect().size();
                if (content - least).abs().max_elem() < 0.5 {
                    break;
                }
                raw = raw * least / content;
                h.set_size(raw);
                settle(&mut h);
            }
            let at = format!("{scale}%, module {module}");
            let zoom = h.ctx.zoom_factor();
            assert!((zoom - f32::from(scale) / 100.0).abs() < 0.001, "{at}: zoom {zoom}");
            let window = h.ctx.content_rect();
            assert!((window.width() - least.x).abs() < 1.0, "{at}: {window:?} in points");
            assert!((window.height() - least.y).abs() < 1.0, "{at}: {window:?} in points");
            let wrong = bars_wrong(&h, true);
            assert!(wrong.is_empty(), "{at}: {wrong:#?}");
            let help = h.get_by_label("Top Bar").get_by_label("Help").rect();
            let plugins = h.get_by_label("Top Bar").get_by_label("Plugins").rect();
            let right = window.right() - TEST_FRAME / 2.0;
            assert!(help.right() <= right && plugins.right() <= help.left(), "{at}: {help:?}");
        }
    }
}

/// The viewport commands of the frames run until `seen` finds what it
/// looks for (or two seconds go by): whether it did.
fn sent(h: &mut Harness<'_, Moonglow>, seen: impl Fn(&egui::ViewportCommand) -> bool) -> bool {
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(2) {
        h.step();
        let out = h.output().viewport_output.get(&egui::ViewportId::ROOT);
        if out.is_some_and(|o| o.commands.iter().any(&seen)) {
            return true;
        }
    }
    false
}

/// The window's least size is given in points as the window opens and
/// again each time the interface size changes while Moonglow runs
/// (egui-winit turns points into pixels at the new size), and a window
/// smaller than it in points at the new size is made as large.
#[test]
fn the_least_size_follows_the_interface_size() {
    use egui::ViewportCommand as C;
    let least = mg_ui::LEAST_WINDOW_SIZE;
    let min = |c: &C| matches!(c, C::MinInnerSize(s) if *s == least);
    // (The frames of the harness's making are not seen: from a size
    // set before it, the first is given as the window opens.)
    let mut h = harness(app_with(Vec::new()), 1000.0, 700.0);
    settle(&mut h);
    assert!(!sent(&mut h, min), "once, while nothing changes");
    for scale in [150, 90, 200] {
        h.state_mut().settings.ui_scale = Some(scale);
        assert!(sent(&mut h, min), "{scale}%: given again");
        settle(&mut h);
    }
    // At 200% a window of 1000 by 700 pixels is 500 by 350 points: large
    // enough. At 300% (a size set in the settings file), 333 by 233: made
    // as large as the least size.
    h.state_mut().settings.ui_scale = Some(300);
    let grown = |c: &C| matches!(c, C::InnerSize(s) if s.x >= least.x && s.y >= least.y);
    assert!(sent(&mut h, grown), "a window smaller than the least size made as large");
    h.state_mut().settings.ui_scale = None;
    assert!(sent(&mut h, min), "back at the usual size");
}

/// A long module name, modified, in the Status Bar at 420, 600 and 1000
/// points, alone and with the note of a look under way ("Reading files
/// changed outside Moonglow…"): the game folder goes first, then the name
/// and the note are cut short with an ellipsis where they do not fit (at
/// 420), never over the Log switch, each other, nor out of the window,
/// "(modified)" whole in sight at the name's end; named by their whole
/// words still, the name's path and the note's words in their tips.
#[test]
fn a_long_module_name_is_cut_short_in_the_status_bar() {
    let dir = mg_testkit::scratch_dir("ui-status-long-name");
    let path = sample_module(&dir);
    let long = dir.join("a_module_with_a_rather_long_file_name_for_the_status_bar_check.mod");
    std::fs::copy(&path, &long).unwrap();
    let name = "a_module_with_a_rather_long_file_name_for_the_status_bar_check (modified)";
    let note = "Reading files changed outside Moonglow…";
    for with_note in [false, true] {
        for width in [420.0, 600.0, 1000.0] {
            let mut app = app_with(Vec::new());
            app.settings.no_last_area = true;
            app.open_module(&long);
            app.show_looking_note(with_note.then_some(note));
            let edit = mg_edit::Edit::SetResource {
                key: ResKey::parse("added", ResType::NSS).unwrap(),
                data: Some(b"void main() { }\n".to_vec()),
            };
            app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Add", vec![edit])));
            let mut h = harness(app, width, 600.0);
            settle(&mut h);
            let at = format!("{width}, note {with_note}");
            let wrong = bars_wrong(&h, true);
            assert!(wrong.is_empty(), "{at}: {wrong:#?}");
            let item = h.get_by_label("Status Bar").get_by_label(name).rect();
            let switch = h.get_by_label("Status Bar").get_by_label("Log").rect();
            let right = h.ctx.content_rect().right() - TEST_FRAME / 2.0;
            let mut left = switch.right() + 8.0;
            if with_note {
                let shown = h.get_by_label("Status Bar").get_by_label(note).rect();
                assert!(shown.left() >= left, "{at}: the note {shown:?} clear of {switch:?}");
                left = shown.right() + 8.0;
                let words = drawn_texts(&h, shown);
                assert!(words.iter().any(|t| t.starts_with("Reading")), "{at}: {words:?}");
                let cut = words.iter().any(|t| t.ends_with('…') && t.len() < note.len());
                assert!(cut || width > 420.0, "{at}: the note cut short: {words:?}");
                assert!(!cut || width < 1000.0, "{at}: the note whole: {words:?}");
                let tip = tip_at(&mut h, shown).unwrap_or_default();
                assert_eq!(tip, note, "{at}: the note's words in its tip");
            }
            assert!(item.left() >= left, "{at}: {item:?} clear of what is before it");
            assert!(item.right() <= right, "{at}: {item:?} in the window ({right})");
            // (Drawn: the words whose end is "(modified)", inside its rect.)
            let texts = drawn_texts(&h, item);
            assert!(
                texts.iter().any(|t| t.ends_with(" (modified)") && t.len() > 12),
                "{at}: {texts:?}"
            );
            // (Cut short at 420, where it can't fit beside the switch; whole
            // at 1000, the game folder left out at 600 or both whole.)
            let cut = texts.iter().any(|t| t.contains('…'));
            assert!(cut || width > 420.0, "{at}: cut short where it does not fit: {texts:?}");
            assert!(!cut || width < 1000.0, "{at}: whole where it fits: {texts:?}");
            let tip = tip_at(&mut h, item).unwrap_or_default();
            assert_eq!(tip, long.display().to_string(), "{at}: its path in the tip");
        }
    }
}

/// The words drawn inside `rect` in the last frame, one string for each
/// line of text, the galleys that follow each other on one line joined.
fn drawn_texts(h: &Harness<'_, Moonglow>, rect: egui::Rect) -> Vec<String> {
    let mut pieces: Vec<(f32, f32, String)> = Vec::new();
    for shape in drawn(h) {
        if let egui::Shape::Text(t) = shape {
            let r = t.visual_bounding_rect();
            if rect.expand(1.0).contains_rect(r) {
                let shown: String =
                    t.galley.rows.iter().flat_map(|r| r.glyphs.iter().map(|g| g.chr)).collect();
                pieces.push((r.center().y.round(), r.left(), shown));
            }
        }
    }
    pieces.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap());
    let mut lines: Vec<(f32, String)> = Vec::new();
    for (y, _, text) in pieces {
        match lines.last_mut() {
            Some((at, line)) if (*at - y).abs() <= 2.0 => line.push_str(&text),
            _ => lines.push((y, text)),
        }
    }
    lines.into_iter().map(|(_, l)| l).collect()
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
