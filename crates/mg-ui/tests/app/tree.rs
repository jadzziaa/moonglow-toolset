//! The module tree: its filter, keys, menus and what it lists.

use crate::common::*;

#[test]
fn filtering_the_tree_opens_its_groups() {
    let dir = mg_testkit::scratch_dir("ui-tree-filter");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Scripts start closed.
    assert!(h.query_by_label("hello").is_none());
    // The tree's filter, the first text field.
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text("hel");
    h.run();
    h.get_by_label("hello");
}

/// The tree's Filter finds a row with every word typed, in any order (as
/// every list's search does: "sword iron" finds Iron Sword), and Find's
/// keys go to it as Options › Keyboard has them, not Ctrl+F alone.
#[test]
fn the_tree_s_filter_finds_every_word_and_takes_find_s_keys() {
    let dir = mg_testkit::scratch_dir("ui-tree-filter-words");
    let mut m = Module::new();
    for name in ["iron_sword", "sword_shield", "longsword_iron"] {
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    let path = dir.join("words.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.key_bindings.insert("find".into(), vec!["Ctrl+Shift+F".into()]);
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label_contains("Scripts (").click();
    h.run_steps(2);
    // Ctrl+F is no longer Find's: the keys stay in the rows.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run_steps(2);
    assert!(!h.ctx.egui_wants_keyboard_input());
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::F);
    h.run_steps(2);
    assert!(h.ctx.egui_wants_keyboard_input(), "Find's keys go to the Filter");
    filter_type(&mut h, "SWORD iron");
    h.run_steps(2);
    assert!(h.query_by_label("iron_sword").is_some());
    assert!(h.query_by_label("longsword_iron").is_some());
    assert!(h.query_by_label("sword_shield").is_none(), "every word, not one of them");
    h.get_by_label_contains("Scripts (2)");
}

/// Edit on a row's right-click menu opens it, as a double click does.
#[test]
fn a_tree_row_s_menu_opens_it_in_its_editor() {
    let dir = mg_testkit::scratch_dir("ui-tree-edit");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Script(script)).is_none());
    h.get_by_label_contains("Scripts (1)").click();
    h.run();
    h.get_by_label("hello").click_secondary();
    h.run();
    // (The menu bar has an Edit too; the row's menu is drawn after it.)
    h.get_all_by_label("Edit").last().unwrap().click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Script(script)).is_some());
}

#[test]
fn resources_open_in_their_editors() {
    let dir = mg_testkit::scratch_dir("ui-tabs");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::for_resource(script).unwrap()));
    h.run();
    // Tab titles are drawn without accessible labels; check the dock and the
    // editor's content.
    assert!(h.state().dock.find_tab(&Tab::Script(script)).is_some());
    h.get_by_label("Compile");
    let area = ResKey::parse("start", ResType::ARE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::for_resource(area).unwrap()));
    h.run();
    // The area viewer, with its object filters.
    assert!(h.state().dock.find_tab(&Tab::Area(area.resref)).is_some());
    h.get_by_label("⛲ Placeables");
}

/// GitHub issue 12: the module tree by the keyboard. A click in it gives
/// it the arrow keys: Up and Down go from row to row, Left and Right
/// close and open a group, Enter opens the row's resource.
#[test]
fn the_module_tree_by_the_keyboard() {
    use mg_ui::TreeAt;
    let dir = mg_testkit::scratch_dir("ui-tree-keys");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let start = ResKey::parse("start", ResType::ARE).unwrap();
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run();
        h.run();
    };
    let at = |h: &Harness<'_, Moonglow>| h.state().tree_cursor;
    // Before a click the keys are not the tree's.
    h.get_by_label("start").hover();
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(at(&h), None);
    // F6 gives them to the tree without one, at its first row; again, on
    // to the middle (no palettes here), and Shift+F6 back.
    key(&mut h, egui::Key::F6);
    assert_eq!(at(&h), Some(TreeAt::Group("Areas")));
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(at(&h), Some(TreeAt::Resource(start)));
    key(&mut h, egui::Key::F6);
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(at(&h), Some(TreeAt::Resource(start)), "the keys are the middle's");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F6);
    h.run();
    h.run();
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(at(&h), Some(TreeAt::Group("Areas")));
    h.state_mut().tree_cursor = None;
    key(&mut h, egui::Key::F6);
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(at(&h), None, "handed on again");
    h.get_by_label("start").click();
    h.run();
    assert_eq!(at(&h), Some(TreeAt::Resource(start)));
    // Up to its group; Left closes it, Right opens it and goes in.
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(at(&h), Some(TreeAt::Group("Areas")));
    key(&mut h, egui::Key::ArrowLeft);
    assert!(h.query_by_label("start").is_none(), "the group is closed");
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(at(&h), Some(TreeAt::Group("Conversations")), "past the closed group's rows");
    key(&mut h, egui::Key::Home);
    key(&mut h, egui::Key::ArrowRight);
    assert!(h.query_by_label("start").is_some(), "open again");
    key(&mut h, egui::Key::ArrowRight);
    assert_eq!(at(&h), Some(TreeAt::Resource(start)));
    // Enter opens it.
    let area = Tab::Area(ResRef::from_str("start").unwrap());
    assert!(h.state().dock.find_tab(&area).is_none());
    key(&mut h, egui::Key::Enter);
    assert!(h.state().dock.find_tab(&area).is_some());
    // A letter goes to the next row that begins with it.
    key(&mut h, egui::Key::C);
    assert_eq!(at(&h), Some(TreeAt::Group("Conversations")));
    key(&mut h, egui::Key::S);
    assert_eq!(at(&h), Some(TreeAt::Group("Scripts")));
    key(&mut h, egui::Key::A);
    key(&mut h, egui::Key::S);
    assert_eq!(at(&h), Some(TreeAt::Resource(start)), "the area named start, after Areas");
    // F2 and Delete at the cursor ask, as the row's menu does.
    h.get_by_label("start").hover();
    key(&mut h, egui::Key::F2);
    assert!(h.state().rename.is_some(), "Rename… of the cursor's resource");
    h.state_mut().rename = None;
    h.run();
    key(&mut h, egui::Key::Delete);
    assert_eq!(h.state().confirm_delete, Some(start));
    h.state_mut().confirm_delete = None;
    h.run();
    // Ctrl+F goes to the Filter, and Down from it into the rows.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run();
    h.run();
    assert!(h.ctx.egui_wants_keyboard_input(), "the Filter has the keyboard");
    key(&mut h, egui::Key::ArrowDown);
    assert!(!h.ctx.egui_wants_keyboard_input());
    assert_eq!(at(&h), Some(TreeAt::Group("Areas")));
    // In the Filter, Escape clears what is typed; again, back to the rows
    // where the cursor was.
    key(&mut h, egui::Key::ArrowDown);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run();
    h.run();
    h.event(egui::Event::Text("sta".into()));
    h.run();
    h.run();
    key(&mut h, egui::Key::Escape);
    assert!(h.ctx.egui_wants_keyboard_input(), "cleared, still in the Filter");
    h.get_by_label("Conversations (0)");
    key(&mut h, egui::Key::Escape);
    assert!(!h.ctx.egui_wants_keyboard_input());
    assert_eq!(at(&h), Some(TreeAt::Resource(start)));
    // Right opens an area out to what is placed in it, and Left closes it
    // again before it goes to the area's group.
    key(&mut h, egui::Key::ArrowRight);
    h.run();
    // (One is the tree's own Expand All.)
    assert_eq!(h.query_all_by_label("⏷").count(), 2, "the area is opened out");
    key(&mut h, egui::Key::ArrowLeft);
    h.run();
    assert_eq!(at(&h), Some(TreeAt::Resource(start)), "closed, the cursor stays");
    assert_eq!(h.query_all_by_label("⏷").count(), 1);
    key(&mut h, egui::Key::ArrowLeft);
    assert_eq!(at(&h), Some(TreeAt::Group("Areas")));
    // The keys are the tree's with the pointer off it too (over the
    // middle, where no area is open).
    h.hover_at(egui::pos2(700.0, 400.0));
    h.run();
    key(&mut h, egui::Key::Home);
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(at(&h), Some(TreeAt::Resource(start)), "Down, with the pointer elsewhere");
    // End is the last row; Escape hands the keys back.
    key(&mut h, egui::Key::End);
    assert!(matches!(at(&h), Some(TreeAt::Group(_))));
    key(&mut h, egui::Key::Escape);
    assert_eq!(at(&h), None);
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(at(&h), None);
}

/// Expand All and Collapse All open and close every group of the module
/// tree at once.
#[test]
fn the_module_tree_s_groups_open_and_close_together() {
    let Some((mut h, area)) = area_harness("fold-tree") else { return };
    h.run_steps(2);
    let listed = |h: &Harness<'_, Moonglow>| h.query_all_by_label(&area.to_string()).count();
    let before = listed(&h);
    assert!(before >= 1, "Areas is open to begin with");
    h.get_all_by_label("⏶").next().unwrap().click();
    h.run_steps(4);
    assert_eq!(listed(&h), before - 1, "the area's row is gone from the tree");
    h.get_all_by_label("⏷").next().unwrap().click();
    h.run_steps(4);
    assert_eq!(listed(&h), before, "and back");
}

/// The module tree's Delete… takes an area out of the module with its
/// objects and its entry in the area list, but not the area the module
/// starts in.
#[test]
fn an_area_is_deleted_from_the_module_tree() {
    let Some((mut h, area)) = area_harness("delete-area") else { return };
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let (are, git) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let listed = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let has = ws.module.contains(&are) && ws.module.contains(&git);
        (has, ws.doc(&ifo).unwrap().root.list("Mod_Area_list").unwrap().len())
    };
    assert_eq!(listed(&mut h), (true, 1));
    // The start location is in it: it stays.
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(are));
    h.run_steps(3);
    assert!(h.query_by_label_contains("start location is in this area").is_some());
    h.get_by_label("OK").click();
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::DeleteResource(are));
    h.run_steps(2);
    assert_eq!(listed(&mut h), (true, 1), "not deleted");
    // With the start elsewhere, it goes, asked first.
    let edit = mg_edit::Edit::SetField {
        key: ifo,
        path: mg_edit::GffPath::root(),
        label: "Mod_Entry_Area".into(),
        value: Some(mg_gff::Value::resref(ResRef::from_str("elsewhere").unwrap())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(are));
    h.run_steps(3);
    assert!(h.query_by_label_contains("everything placed in it").is_some());
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert_eq!(listed(&mut h), (false, 0));
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_none(), "its view is closed");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(listed(&mut h), (true, 1), "one undo");
}

/// An area opens out, in the module tree, to what is placed in it, kind
/// by kind; a click on an object goes to it in the area's view.
#[test]
fn the_module_tree_lists_what_is_placed_in_an_area() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-contents") else { return };
    assert!(h.query_by_label("Waypoints (2)").is_none());
    h.get_by_label("⏵").click();
    h.run_steps(3);
    // Every kind, with how many; the empty ones too.
    // (The tree's own Creatures group, of blueprints, reads the same.)
    assert_eq!(h.query_all_by_label("Creatures (0)").count(), 2);
    h.get_by_label("Waypoints (2)").click();
    h.run_steps(3);
    // Its two waypoints, by tag.
    let rows: Vec<_> = h.query_all_by_label("NW_WAYPOINT001").map(|n| n.rect().center()).collect();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(h.state().area_views[&area].selection.is_empty());
    let second = rows[1];
    h.hover_at(second);
    press(&h, second, true, egui::Modifiers::NONE);
    press(&h, second, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 1)]);
    // What is placed later is listed too.
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let mut copy = ws.doc(&git).unwrap().root.list("WaypointList").unwrap()[0].clone();
    // (Standing 1.5 m above the ground, as on a table.)
    let z = copy.float("ZPosition").unwrap();
    copy.set("ZPosition", mg_gff::Value::Float(z + 1.5));
    let edit = mg_edit::Edit::InsertItem {
        key: git,
        path: mg_edit::GffPath::root(),
        list: "WaypointList".into(),
        index: 2,
        item: copy,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.run_steps(4);
    h.get_by_label("Waypoints (3)");
    // A right click on one: copied to paste in an area, or deleted (undone
    // as any change).
    let rows: Vec<_> = h.query_all_by_label("NW_WAYPOINT001").map(|n| n.rect().center()).collect();
    assert_eq!(rows.len(), 3);
    let menu = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2, what: &str| {
        h.hover_at(at);
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(3);
        h.get_by_label(what).click();
        h.run_steps(4);
    };
    menu(&mut h, rows[2], "Copy");
    assert_eq!(h.state().object_clip.as_ref().map(|c| c.objects.len()), Some(1));
    // As high above the ground as it stood, as Copy in the area's view
    // keeps it: it was pasted on the ground.
    let lift = h.state().object_clip.as_ref().unwrap().objects[0].2;
    assert!((lift - 1.5).abs() < 0.05, "lift {lift}");
    menu(&mut h, rows[2], "Delete");
    h.get_by_label("Waypoints (2)");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Delete"));
    // The tree's Filter narrows the objects to those it matches.
    let ws = h.state_mut().ws.as_mut().unwrap();
    let path = mg_edit::GffPath::root().item("WaypointList", 1);
    let edit = mg_edit::Edit::SetField {
        key: git,
        path,
        label: "LocalizedName".into(),
        value: Some(mg_gff::Value::LocString(LocString::from_text(
            Language::ENGLISH,
            Gender::Male,
            "Ferry Landing",
        ))),
    };
    let _ = ws;
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Name", vec![edit])));
    h.run_steps(4);
    // (A waypoint shows its tag; its name is found too.)
    assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), 2);
    let filter = h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap();
    filter.click();
    h.run_steps(2);
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text("ferry");
    h.run_steps(4);
    h.get_by_label("Waypoints (1 of 2)");
    assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), 1);
}

/// Options › General › Show areas by name: the tree lists areas by their
/// names (an unnamed one by its ResRef), in the names' order; the filter
/// finds a name or a ResRef, and a renamed area shows its new name at once.
#[test]
fn areas_are_listed_by_name_when_asked() {
    let dir = mg_testkit::scratch_dir("ui-area-names");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let named = |name: &str| {
        let mut are = Gff::new(*b"ARE ");
        let name = LocString::from_text(Language::ENGLISH, Gender::Male, name);
        are.root.set("Name", mg_gff::Value::LocString(name));
        are.to_bytes().unwrap()
    };
    let docks = ResKey::parse("area467", ResType::ARE).unwrap();
    m.set(docks, named("The Docks"));
    m.set(ResKey::parse("area900", ResType::ARE).unwrap(), named("Apple Inn"));
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // By ResRef, as Aurora lists them.
    h.get_by_label("area467");
    assert!(h.query_by_label("The Docks").is_none());
    h.state_mut().settings.area_names = true;
    h.run();
    assert!(h.query_by_label("area467").is_none());
    let y = |h: &Harness<'_, Moonglow>, label: &str| h.get_by_label(label).rect().top();
    assert!(y(&h, "Apple Inn") < y(&h, "The Docks"), "in the names' order");
    // The area without a name keeps its ResRef.
    h.get_by_label("start");
    // Renamed (as Area Properties does): the new name, in its place.
    let edit = mg_edit::Edit::SetField {
        key: docks,
        path: mg_edit::GffPath::root(),
        label: "Name".into(),
        value: Some(mg_gff::Value::LocString(LocString::from_text(
            Language::ENGLISH,
            Gender::Male,
            "A Quay",
        ))),
    };
    h.state_mut().ws.as_mut().unwrap().apply(mg_edit::Command::new("Rename", vec![edit])).unwrap();
    h.run();
    assert!(y(&h, "A Quay") < y(&h, "Apple Inn"));
    // The filter finds the name.
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text("apple");
    h.run();
    h.get_by_label("Apple Inn");
    assert!(h.query_by_label("A Quay").is_none());
}

/// The tree's filter finds a name whatever the case of its letters, those
/// past ASCII too (an area named in German, typed with its capital).
#[test]
fn the_tree_s_filter_ignores_the_case_of_any_letter() {
    let dir = mg_testkit::scratch_dir("ui-tree-filter-case");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let mut are = Gff::new(*b"ARE ");
    // (Its bytes in Windows-1252, as the file holds them.)
    let name = LocString::from_text(Language::ENGLISH, Gender::Male, b"\xDCbelwald".to_vec());
    are.root.set("Name", mg_gff::Value::LocString(name));
    m.set(ResKey::parse("area001", ResType::ARE).unwrap(), are.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().settings.area_names = true;
    h.run();
    h.get_by_label("Übelwald");
    for typed in ["Übel", "ÜBEL", "übel"] {
        h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().click();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text(typed);
        h.run();
        assert!(h.query_by_label("Übelwald").is_some(), "{typed}");
        assert!(h.query_by_label("start").is_none(), "{typed}");
    }
}

/// List areas and blueprints by name: a blueprint is listed by its name
/// (a creature by its first and last names), with its ResRef in
/// parentheses when asked for; one without a name keeps its ResRef.
#[test]
fn blueprints_are_listed_by_name_when_asked() {
    let dir = mg_testkit::scratch_dir("ui-blueprint-names");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let text = |t: &str| {
        mg_gff::Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, t))
    };
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("FirstName", text("Hent"));
    guard.root.set("LastName", text("Fynolds"));
    m.set(ResKey::parse("creature007", ResType::UTC).unwrap(), guard.to_bytes().unwrap());
    m.set(ResKey::parse("nameless", ResType::UTC).unwrap(), Gff::new(*b"UTC ").to_bytes().unwrap());
    let mut chest = Gff::new(*b"UTP ");
    chest.root.set("LocName", text("Old Chest"));
    m.set(ResKey::parse("plc001", ResType::UTP).unwrap(), chest.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Open the groups (the filter opens every group with a match).
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text("e");
    h.run();
    h.get_by_label("creature007.utc");
    h.state_mut().settings.area_names = true;
    h.run();
    h.get_by_label("Hent Fynolds");
    h.get_by_label("Old Chest");
    h.get_by_label("nameless.utc");
    assert!(h.query_by_label("creature007.utc").is_none());
    h.state_mut().settings.name_resrefs = true;
    h.run();
    h.get_by_label("Hent Fynolds (creature007)");
    h.get_by_label("Old Chest (plc001)");
}

/// The module tree leaves the middle its room: a very long name is cut
/// short in it rather than widen it over the area and the palettes (a
/// builder's module opened with the tree over the whole window).
#[test]
fn the_module_tree_never_takes_the_window() {
    let Some((mut h, area)) = area_harness("tree-width") else { return };
    // Areas by name, and a name far longer than the pane.
    h.state_mut().settings.area_names = true;
    let name = "An Area With A Name That Goes On And On ".repeat(8);
    let edit = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::ARE),
        path: mg_edit::GffPath::root(),
        label: "Name".into(),
        value: Some(mg_gff::Value::LocString(mg_core::LocString::from_text(
            mg_core::Language(0),
            mg_core::Gender::Male,
            name.trim(),
        ))),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Name", vec![edit])));
    h.run_steps(6);
    // The area's view and the palettes are on screen, with room, and the
    // tree takes at most two fifths of the window.
    let s = h.state();
    let pane = |tab: &Tab| {
        let path = s.dock.find_tab(tab).expect("open");
        let at = |p: &egui_dock::NodePath| p.surface == path.surface && p.node == path.node;
        s.dock.iter_leaves().find(|(p, _)| at(p)).unwrap().1.rect
    };
    let (view, palette) = (pane(&Tab::Area(area)), pane(&Tab::Palette));
    assert!(view.left() <= 1100.0 * 0.4 + 20.0, "the tree ends by {}", view.left());
    assert!(view.width() > 150.0, "the area's view: {view:?}");
    assert!(palette.width() > 150.0 && palette.right() <= 1100.0, "the palettes: {palette:?}");
    // A split dragged all the way over (or remembered so) is brought back:
    // neither side is left with nothing.
    for (_, node) in h.state_mut().dock.iter_all_nodes_mut() {
        if let egui_dock::Node::Horizontal(split) = node {
            split.fraction = 0.999;
        }
    }
    h.run_steps(3);
    let s = h.state();
    let path = s.dock.find_tab(&Tab::Palette).unwrap();
    let at = |p: &egui_dock::NodePath| p.surface == path.surface && p.node == path.node;
    let palette = s.dock.iter_leaves().find(|(p, _)| at(p)).unwrap().1.rect;
    assert!(palette.width() > 60.0, "the palettes keep some room: {palette:?}");
}

/// The Delete key on a row of the module tree asks as its menu's Delete…
/// does.
#[test]
fn delete_on_a_tree_row_asks_first() {
    let Some((mut h, key)) = plugin_harness("ui-tree-delete-key", false) else { return };
    h.run();
    // The guard's row (its type opened out), under the pointer.
    h.get_all_by_label("⏷").next().unwrap().click();
    h.run_steps(3);
    let found = h.query_all_by_label_contains("guard").count();
    assert!(found > 0, "the guard's row");
    let row = h.query_all_by_label_contains("guard").next().unwrap().rect().center();
    h.hover_at(row);
    h.run_steps(2);
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert!(h.query_by_label("Delete").is_some(), "asked");
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert!(!h.state_mut().ws.as_mut().unwrap().module.contains(&key));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(h.state_mut().ws.as_mut().unwrap().module.contains(&key), "undone");
}

/// An area opened, or its tab chosen, brings the area's row into view in
/// the module tree, as the one in hand; the tab's Show in Module Tree opens it out
/// to what is placed in it as well.
#[test]
fn the_module_tree_goes_to_the_area_of_the_tab_chosen() {
    let Some((mut h, area)) = area_harness("tree-reveal") else { return };
    // The area opened is the one marked (a double click, View Area).
    assert_eq!(h.state().tree_area, Some(area));
    h.state_mut().tree_area = None;
    assert!(h.query_by_label("Waypoints (2)").is_none());
    // (As a click on the area's tab asks.)
    h.state_mut().tree_reveal = Some((ResKey::new(area, ResType::ARE), false));
    h.run_steps(2);
    assert_eq!(h.state().tree_area, Some(area));
    assert!(h.state().tree_reveal.is_none(), "done once");
    assert!(h.query_by_label("Waypoints (2)").is_none(), "not opened out");
    // Show in Module Tree: opened out too.
    h.state_mut().tree_reveal = Some((ResKey::new(area, ResType::ARE), true));
    h.run_steps(3);
    assert!(h.query_by_label("Waypoints (2)").is_some());
}

/// Every kind of tab chosen moves the module tree to what it shows, as an
/// area's does: a script opened marks its row, opens the Scripts group and
/// brings the row into view; Ctrl+Tab to another script, and a click on
/// the area's tab, move the mark with them; the keys stay with the tab.
/// With Options › General's Show the Tab Chosen in the Module Tree off,
/// the tree stays and marks the area in front only, as Aurora's does.
#[test]
fn every_kind_of_tab_chosen_moves_the_module_tree() {
    let scripts = |m: &mut mg_module::Module, _: &mg_rules::GameData, _: ResRef| {
        for n in 0..40 {
            let key = ResKey::parse(&format!("script_{n:02}"), ResType::NSS).unwrap();
            m.set(key, b"void main()\n{\n}\n".to_vec());
        }
    };
    let Some((mut h, area)) = area_harness_with("tree-follows", "ttr01", None, 4, None, &scripts)
    else {
        return;
    };
    let script = |n: u32| ResKey::parse(&format!("script_{n:02}"), ResType::NSS).unwrap();
    let area_key = ResKey::new(area, ResType::ARE);
    let marked = |h: &Harness<'_, Moonglow>| h.state().tree_marked;
    assert_eq!(marked(&h), Some(area_key), "the area opened");
    // (The area's tab first in the middle, for a click on it below.)
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    h.run_steps(3);
    assert!(h.query_by_label("script_38").is_none(), "the Scripts group is closed");
    let cursor = h.state().tree_cursor;
    // A script opened: its row marked, its group opened, brought into
    // view (the row beside it drawn too: rows out of sight are not).
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(script(39))));
    // (The group opens over a few frames, then the row is gone to.)
    h.run_steps(12);
    assert_eq!(marked(&h), Some(script(39)));
    assert!(h.query_by_label("script_38").is_some(), "brought into view");
    assert_eq!(h.state().tree_cursor, cursor, "the keys stay with the tab");
    // Another, in the same window; Ctrl+Tab back to the first.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(script(0))));
    h.run_steps(4);
    assert_eq!(marked(&h), Some(script(0)));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Tab);
    h.run_steps(3);
    assert_eq!(marked(&h), Some(script(39)), "Ctrl+Tab");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::PageDown);
    h.run_steps(3);
    assert_eq!(marked(&h), Some(script(0)), "Ctrl+PageDown");
    // A click on the area's tab (the first of the middle's, in its bar).
    let area_tab = |h: &mut Harness<'_, Moonglow>| {
        let s = h.state();
        let path = s.dock.find_tab(&Tab::Area(area)).expect("open");
        let leaf = s.dock.leaf(path.node_path()).unwrap();
        assert_eq!(leaf.tabs.iter().position(|t| *t == Tab::Area(area)), Some(0));
        let at = egui::pos2(leaf.rect.left() + 30.0, (leaf.rect.top() + leaf.viewport.top()) / 2.0);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    area_tab(&mut h);
    assert_eq!(marked(&h), Some(area_key));
    // The tab's Show in Module Tree, for a script too.
    h.state_mut().tree_reveal = Some((script(20), true));
    h.run_steps(3);
    assert_eq!(marked(&h), Some(script(20)));
    // Off: the tree stays where it is; the area in front alone is marked.
    h.state_mut().settings.tree_stays = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(script(5))));
    h.run_steps(4);
    assert_eq!(marked(&h), Some(script(20)), "not moved");
    assert!(h.state().tree_reveal.is_none());
    area_tab(&mut h);
    assert_eq!(marked(&h), Some(area_key), "the area in front marked");
}

/// A placed object's row in the module tree opens its variables from its
/// menu, as Aurora's list does; the window lists them by name, and writes
/// them back in the order they were stored (a builder's asks).
#[test]
fn a_tree_object_s_menu_opens_its_variables_listed_by_name() {
    use mg_gff::{Struct, Value};
    let Some((mut h, area)) = area_harness("tree-variables") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let var = |name: &str, value: i32| {
        let mut s = Struct::new(0);
        s.set("Name", Value::String(name.as_bytes().to_vec()));
        s.set("Type", Value::Dword(1));
        s.set("Value", Value::Int(value));
        s
    };
    let stored = vec![var("zeta", 1), var("Alpha", 2), var("mid", 3)];
    let edits = (0..2)
        .map(|i| mg_edit::Edit::SetField {
            key: git,
            path: mg_edit::GffPath::root().item("WaypointList", i),
            label: "VarTable".into(),
            value: Some(Value::List(stored.clone())),
        })
        .collect();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    h.run_steps(6);
    h.query_all_by_label_contains("WAYPOINT").next().expect("a waypoint's row").click_secondary();
    h.run_steps(3);
    h.get_by_label("Variables…").click();
    h.run_steps(3);
    let edit = h.state().var_edit.clone().expect("the Variables window");
    assert_eq!(edit.target.key, git);
    let names: Vec<&str> = edit.rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Alpha", "mid", "zeta"], "by name");
    assert_eq!(edit.list(), stored, "written back as stored");
    assert!(h.query_by_label("Variables").is_some(), "the window shows");
}

#[test]
fn the_module_tree_shows_the_object_selected_in_the_area() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-follows") else { return };
    h.state_mut().settings.tree_keeps_areas_closed = true;
    h.run_steps(3);
    assert!(h.query_by_label("Waypoints (2)").is_none());
    // The area is not opened out in the tree: it stays closed.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 1)];
    h.run_steps(6);
    assert_eq!(h.state().tree_object, Some((area, ObjectKind::Waypoint, 1)));
    assert!(!h.state().tree_object_pending, "nothing to go to");
    assert!(h.query_by_label("Waypoints (2)").is_none(), "the area stays closed");
    // Opened out (its arrow): the kinds are listed, folded, and opening
    // it does not go to the object selected before.
    h.get_by_label("⏵").click();
    h.run_steps(6);
    assert!(h.query_by_label("Waypoints (2)").is_some());
    let folded = h.query_all_by_label_contains("WAYPOINT").count();
    // Another object selected: its kind's list opens to it.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    h.run_steps(6);
    assert_eq!(h.state().tree_object, Some((area, ObjectKind::Waypoint, 0)));
    assert!(!h.state().tree_object_pending, "gone to");
    let rows = h.query_all_by_label_contains("WAYPOINT").count();
    assert!(rows >= folded + 2, "both waypoints' rows are there: {folded} → {rows}");
    // Nothing selected: the tree stays as it is.
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    h.run_steps(3);
    assert_eq!(h.state().tree_object, None);
    assert!(h.query_by_label("Waypoints (2)").is_some());
    // With Options › General's choice as it is at first, as Aurora: the
    // area closed again, a selection opens it out and goes to the object.
    // (The arrow on the area's row, before its name.)
    let row = h.get_by_label("field").rect();
    let arrow = h
        .get_all_by_label("⏷")
        .find(|n| {
            n.rect().right() <= row.left() && (n.rect().center().y - row.center().y).abs() < 4.0
        })
        .unwrap();
    arrow.click();
    h.run_steps(6);
    assert!(h.query_by_label("Waypoints (2)").is_none());
    h.state_mut().settings.tree_keeps_areas_closed = false;
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 1)];
    h.run_steps(8);
    assert!(h.query_by_label("Waypoints (2)").is_some(), "opened out");
    assert_eq!(h.state().tree_object, Some((area, ObjectKind::Waypoint, 1)));
    assert!(!h.state().tree_object_pending, "gone to");
}

/// What is placed in an area is listed sorted, waypoints by tag (they
/// often share a name), as Aurora lists them (builders' reports). A row
/// clicked stays where it is, so that a second click makes a double click
/// (Properties); an object selected in the area whose row is in view does
/// not move the tree, and one out of view is scrolled to, not the area's
/// row above it.
#[test]
fn the_module_tree_sorts_objects_and_stays_still_under_a_click() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-sorted") else { return };
    // Forty waypoints, tagged WP_39 to WP_00 in the area's order, each
    // named alike.
    let git = ResKey::new(area, ResType::GIT);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let first = ws.doc(&git).unwrap().root.list("WaypointList").unwrap()[0].clone();
    let list: Vec<mg_gff::Struct> = (0..40)
        .map(|i| {
            let mut w = first.clone();
            w.set("Tag", mg_gff::Value::String(format!("WP_{:02}", 39 - i).into_bytes()));
            w.set("XPosition", mg_gff::Value::Float(1.0 + i as f32));
            w
        })
        .collect();
    let edit = mg_edit::Edit::SetField {
        key: git,
        path: mg_edit::GffPath::root(),
        label: "WaypointList".into(),
        value: Some(mg_gff::Value::List(list)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.run_steps(4);
    h.get_by_label("⏵").click();
    h.run_steps(3);
    h.get_by_label("Waypoints (40)").click();
    h.run_steps(6);
    // Sorted by tag: WP_00 (the last in the area's list) first.
    let y = |h: &Harness<'_, Moonglow>, tag: &str| h.get_by_label(tag).rect().center().y;
    assert!(y(&h, "WP_00") < y(&h, "WP_01") && y(&h, "WP_01") < y(&h, "WP_02"));
    // A click on a row: the object is selected in the area, and the row
    // stays under the pointer.
    let row = h.get_by_label("WP_03").rect();
    let at = row.center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(6);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 36)]);
    assert_eq!(h.get_by_label("WP_03").rect(), row, "the tree stays still");
    // A double click there: its Properties.
    h.run_steps(30);
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        h.step();
        press(&h, at, false, egui::Modifiers::NONE);
        h.step();
    }
    h.run_steps(4);
    let path = mg_edit::GffPath::root().item("WaypointList", 36);
    let tab = h.state().dock.find_tab(&Tab::Instance { area, path });
    h.state_mut().dock.remove_tab(tab.expect("its Properties"));
    h.run_steps(3);
    // Another selected in the area, its row in view: the tree stays still.
    let row = h.get_by_label("WP_01").rect();
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 38)];
    h.run_steps(6);
    assert_eq!(h.get_by_label("WP_01").rect(), row);
    // One row marked, the one selected last: not WP_03's too, clicked
    // before (a builder's report).
    let marked: Vec<String> = h
        .query_all_by(|n| {
            n.label().is_some_and(|l| l.starts_with("WP_"))
                && n.toggled() == Some(egui::accesskit::Toggled::True)
        })
        .filter_map(|n| n.accesskit_node().label().map(|l| l.to_string()))
        .collect();
    assert_eq!(marked, ["WP_01"]);
    // One far down: its row is brought into view.
    assert!(h.query_by_label("WP_39").is_none(), "out of view");
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    h.run_steps(8);
    assert!(h.query_by_label("WP_39").is_some(), "scrolled to");
    assert!(!h.state().tree_object_pending);
}

/// A builder's report: a name typed in the module tree went letter by
/// letter, each to the next row beginning with it ("shopthing": an "s",
/// then an "h", then an "o"...), and only with the pointer over the tree.
/// Once the tree is clicked they go to it wherever the pointer is; typed
/// in quick succession the letters go
/// by all of them, as in Aurora's tree and the conversation tree; the same
/// letter again goes to the next row beginning with it, and a pause
/// starts over.
#[test]
fn the_module_tree_s_letters_typed_quickly() {
    use mg_ui::TreeAt;
    let dir = mg_testkit::scratch_dir("ui-tree-type-ahead");
    let mut m = Module::new();
    for name in ["sewer", "shop", "shopkeep", "shopthing", "tavern"] {
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    let path = dir.join("typing.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_step_dt(0.25)
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    if h.query_by_label("sewer").is_none() {
        h.get_by_label_contains("Scripts (").click();
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
    }
    h.get_by_label("sewer").click();
    h.run();
    // (Clicked, the tree has the keys wherever the pointer goes: the
    // builder asked for that, as Aurora's tree has them.)
    h.hover_at(egui::pos2(700.0, 400.0));
    h.run();
    let at = |h: &Harness<'_, Moonglow>| h.state().tree_cursor;
    let script = |name: &str| Some(TreeAt::Resource(ResKey::parse(name, ResType::NSS).unwrap()));
    // (Typed as a keyboard types, a frame apart: a quarter of a second.)
    let typed = |h: &mut Harness<'_, Moonglow>, t: &str| {
        for c in t.chars() {
            h.event(egui::Event::Text(c.to_string()));
            h.step();
        }
        h.run();
    };
    typed(&mut h, "shopt");
    assert_eq!(at(&h), script("shopthing"), "by all the letters");
    h.run_steps(5);
    typed(&mut h, "t");
    assert_eq!(at(&h), script("tavern"), "after a pause, t alone");
    h.run_steps(5);
    typed(&mut h, "SHOP");
    assert_eq!(at(&h), script("shop"), "the first beginning with them, in either case");
    typed(&mut h, "k");
    assert_eq!(at(&h), script("shopkeep"), "on with them: shopk");
    h.run_steps(5);
    typed(&mut h, "s");
    assert_eq!(at(&h), script("shopthing"));
    typed(&mut h, "s");
    assert_eq!(at(&h), Some(TreeAt::Group("Sounds")), "the same letter again: the next");
}

/// The module tree's keys at an area opened out: letters typed go to its
/// kinds and its placed objects as their rows show them; Delete at a
/// placed object the keys are at deletes it as its row's menu does
/// (undone as any change), and the keys go on to the row after it; F2
/// there does nothing (its menu has no Rename, nor has Aurora's). Down
/// from the Filter goes to the first row it finds, not the first group.
#[test]
fn the_module_tree_s_keys_at_placed_objects() {
    use mg_area::ObjectKind::Creature;
    use mg_ui::TreeAt;
    let dir = mg_testkit::scratch_dir("ui-tree-object-keys");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut git = Gff::new(*b"GIT ");
    for tag in ["GOBLIN", "BANDIT"] {
        let mut c = mg_schema::git::CREATURE_LIST.new_item();
        c.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
        git.root.items_mut(&mg_schema::git::CREATURE_LIST).push(c);
    }
    let git_key = ResKey::parse("start", ResType::GIT).unwrap();
    m.set_gff(git_key, &git).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_step_dt(0.25)
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let start = ResRef::from_str("start").unwrap();
    let at = |h: &Harness<'_, Moonglow>| h.state().tree_cursor;
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(3);
    };
    let typed = |h: &mut Harness<'_, Moonglow>, t: &str| {
        for c in t.chars() {
            h.event(egui::Event::Text(c.to_string()));
            h.step();
        }
        h.run();
        // (A pause: the next letters start over.)
        h.run_steps(5);
    };
    h.get_by_label("start").click();
    h.run();
    // (The keys are the tree's wherever the pointer is: off its rows, so
    // that F2 and Delete are for the keys' row, not the one under it.)
    h.hover_at(egui::pos2(700.0, 400.0));
    key(&mut h, egui::Key::ArrowRight);
    h.get_by_label("Creatures (2)");
    typed(&mut h, "c");
    assert_eq!(at(&h), Some(TreeAt::Kind(start, Creature)), "a kind, by its name");
    key(&mut h, egui::Key::ArrowRight);
    typed(&mut h, "bandit");
    assert_eq!(at(&h), Some(TreeAt::Object(start, Creature, 1)), "an object, as its row shows");
    typed(&mut h, "g");
    assert_eq!(at(&h), Some(TreeAt::Object(start, Creature, 0)), "the next beginning with it");
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(at(&h), Some(TreeAt::Object(start, Creature, 1)));
    // F2 at a placed object: nothing to rename (not the area's Rename…).
    key(&mut h, egui::Key::F2);
    assert!(h.state().rename.is_none());
    // Delete: the object deleted, as its menu's Delete does; the keys on
    // the row after it (Goblin, still the first of the area's list).
    key(&mut h, egui::Key::Delete);
    assert!(h.state().confirm_delete.is_none(), "not the area's Delete…");
    let creatures = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let list = git.root.list("Creature List").unwrap_or(&[]);
        list.iter()
            .map(|c| String::from_utf8_lossy(c.string("Tag").unwrap()).into_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(creatures(&mut h), ["GOBLIN"]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Delete"));
    assert_eq!(at(&h), Some(TreeAt::Object(start, Creature, 0)));
    h.get_by_label("Creatures (1)");
    // Down from the Filter: to the first row it finds (the area is listed
    // for its object, which is found).
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run_steps(2);
    assert!(h.ctx.egui_wants_keyboard_input(), "the Filter has the keyboard");
    filter_type(&mut h, "gob");
    key(&mut h, egui::Key::ArrowDown);
    assert!(!h.ctx.egui_wants_keyboard_input());
    assert_eq!(at(&h), Some(TreeAt::Object(start, Creature, 0)));
    // A resource it finds by its name: that one, past the groups before.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run_steps(2);
    key(&mut h, egui::Key::Escape);
    filter_type(&mut h, "hel");
    key(&mut h, egui::Key::ArrowDown);
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    assert_eq!(at(&h), Some(TreeAt::Resource(hello)));
}
