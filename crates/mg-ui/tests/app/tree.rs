//! The module tree: its filter, keys, menus and what it lists.

use crate::common::*;

#[test]
fn filtering_the_tree_opens_its_groups() {
    let dir = mg_testkit::scratch_dir("ui-tree-filter");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = window_harness(app);
    h.run();
    // Scripts start closed.
    assert!(h.query_by_label("hello").is_none());
    // The tree's filter, the first text field.
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap()
        .click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap()
        .type_text("hel");
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
    let mut h = harness(app, 1200.0, 800.0);
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
    let mut h = window_harness(app);
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
    let mut h = window_harness(app);
    h.run();
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::for_resource(script).unwrap()));
    h.run();
    // Tab titles are drawn without accessible labels; check the dock and the
    // editor's content.
    assert!(h.state().dock.find_tab(&Tab::Script(script)).is_some());
    apart(&h, "Compile");
    let area = ResKey::parse("start", ResType::ARE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::for_resource(area).unwrap()));
    h.run();
    // The area viewer, with its object filters.
    assert!(h.state().dock.find_tab(&Tab::Area(area.resref)).is_some());
    area_tool(&h, "Show Placeables");
}

/// The log is among Next Pane's panes (the module tree and the palettes'
/// entry, its first item): F6 or a click in it gives it the keys, which
/// scroll it, a line, a page, to its ends; Escape hands them back. A thin
/// outline shows it has them (`target/test-output/ui-log-keys/`).
#[test]
fn the_log_by_the_keyboard() {
    // (The pictures are to look at, taken where there is a GPU.)
    let picture = |h: &mut Harness<'_, Moonglow>, dir: &std::path::Path, name: &str| {
        if adapter() {
            mg_testkit::gpu::hold();
            h.render().expect("render").save(dir.join(name)).unwrap();
        }
    };
    let dir = mg_testkit::scratch_dir("ui-log-keys");
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&sample_module(&dir));
    app.log.clear();
    for i in 0..200 {
        app.log.info(format!("Line {i:03} of the log"));
    }
    let mut h = harness(app, 1000.0, 700.0);
    h.run_steps(3);
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(2);
    };
    let shown = |h: &Harness<'_, Moonglow>, i: usize| {
        h.query_by_label_contains(&format!("Line {i:03} of the log")).is_some()
    };
    assert!(shown(&h, 199) && !shown(&h, 0), "at its end");
    key(&mut h, egui::Key::Home);
    assert!(!shown(&h, 0), "not the log's yet");
    // (From the middle's: the palettes' where their pane is, then the
    // log's.)
    key(&mut h, egui::Key::F6);
    if palette_shown(h.state()) {
        key(&mut h, egui::Key::F6);
    }
    picture(&mut h, &dir, "log-keys.png");
    key(&mut h, egui::Key::Home);
    assert!(shown(&h, 0) && !shown(&h, 199), "at its start");
    let first_shown = |h: &Harness<'_, Moonglow>| (0..200).find(|&i| shown(h, i)).unwrap();
    let mut at = Vec::new();
    for _ in 0..6 {
        key(&mut h, egui::Key::ArrowDown);
        at.push(first_shown(&h));
    }
    assert!(at.windows(2).all(|w| w[1] == w[0] || w[1] == w[0] + 1), "a line at a time: {at:?}");
    picture(&mut h, &dir, "log-keys-lines.png");
    assert!(at[5] >= 4, "{at:?}");
    let lines = at[5];
    key(&mut h, egui::Key::PageDown);
    let page = first_shown(&h);
    assert!(page > lines + 3, "a page: {page}");
    picture(&mut h, &dir, "log-keys-page.png");
    key(&mut h, egui::Key::PageUp);
    assert_eq!(first_shown(&h), lines);
    key(&mut h, egui::Key::End);
    assert!(shown(&h, 199), "at its end again");
    // Escape hands the keys back; a click in it gives them again.
    key(&mut h, egui::Key::Escape);
    key(&mut h, egui::Key::Home);
    assert!(!shown(&h, 0), "the keys aren't the log's");
    h.get_by_label_contains("Line 199 of the log").click();
    h.run_steps(2);
    key(&mut h, egui::Key::Home);
    assert!(shown(&h, 0), "the log's from the click");
    picture(&mut h, &dir, "log-keys-clicked.png");
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
    let mut h = harness(app, 1200.0, 800.0);
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
    // F6 gives them to the next pane from the middle's: the log (no
    // palettes here), then the tree without one, at its first row; again,
    // on to the middle, and Shift+F6 back.
    key(&mut h, egui::Key::F6);
    assert_eq!(at(&h), None, "the log's");
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
    tree_row(&h, "start").hover();
    key(&mut h, egui::Key::F2);
    assert!(h.state().rename.is_some(), "Rename… of the cursor's resource");
    h.state_mut().rename = None;
    h.run();
    key(&mut h, egui::Key::Delete);
    assert_eq!(h.state().confirm_delete, Some(vec![start]));
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
    assert_eq!(opened_out(&h), 1, "the area is opened out");
    key(&mut h, egui::Key::ArrowLeft);
    h.run();
    assert_eq!(at(&h), Some(TreeAt::Resource(start)), "closed, the cursor stays");
    assert_eq!(opened_out(&h), 0);
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
    module_tool(&h, "Collapse All").click();
    h.run_steps(4);
    assert_eq!(listed(&h), before - 1, "the area's row is gone from the tree");
    // (Each group's row says whether it is open, for a screen reader.)
    let areas_open = |h: &Harness<'_, Moonglow>| {
        let row = h.get_by_label_contains("Areas (");
        row.accesskit_node().data().is_expanded()
    };
    assert_eq!(areas_open(&h), Some(false), "Areas said closed");
    module_tool(&h, "Expand All").click();
    h.run_steps(4);
    assert_eq!(listed(&h), before, "and back");
    assert_eq!(areas_open(&h), Some(true), "Areas said open");
    // By their keys (Options › Keyboard), once the tree has the keys:
    // Collapse All closes an area opened out to its objects too.
    let ctrl_shift = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
    let arrows = |h: &Harness<'_, Moonglow>| opened_out(h);
    let closed = arrows(&h);
    h.get_all_by_label(&area.to_string()).next().unwrap().click();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowRight);
    h.run_steps(3);
    settle(&mut h);
    assert_eq!(arrows(&h), closed + 1, "the area opened out");
    h.key_press_modifiers(ctrl_shift, egui::Key::ArrowLeft);
    h.run_steps(4);
    assert_eq!(listed(&h), before - 1, "every group closed by Collapse All's key");
    h.key_press_modifiers(ctrl_shift, egui::Key::ArrowRight);
    h.run_steps(4);
    assert_eq!(listed(&h), before, "opened by Expand All's");
    assert_eq!(arrows(&h), closed, "the area closed with the rest, and not opened out again");
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

/// The Delete window says what in the module names what it deletes, left
/// as it is: a creature's script, a string in another script, another
/// area's door leading into an area; what goes with them is not said.
/// Those chosen in the tree are deleted together, as one step, but the
/// start area.
#[test]
fn deleting_says_what_names_it_and_takes_those_chosen_together() {
    let dir = mg_testkit::scratch_dir("ui-delete-named");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let key = |name: &str, t| ResKey::parse(name, t).unwrap();
    let mut info = m.gff(&key("module", ResType::IFO)).unwrap().unwrap();
    let mut listed = mg_schema::ifo::MOD_AREA_LIST.new_item();
    listed.write(&mg_schema::ifo::mod_area_list::AREA_NAME, ResRef::from_str("yard").unwrap());
    info.root.items_mut(&mg_schema::ifo::MOD_AREA_LIST).push(listed);
    info.root.write(&mg_schema::ifo::MOD_ENTRY_AREA, ResRef::from_str("start").unwrap());
    m.set_info(&info).unwrap();
    // start: a creature that spawns with `hello`, and a door to the
    // yard's gate.
    let mut git = Gff::new(*b"GIT ");
    let mut c = mg_schema::git::CREATURE_LIST.new_item();
    c.write(&mg_schema::git::creature_list::SCRIPT_SPAWN, ResRef::from_str("hello").unwrap());
    c.set("Tag", mg_gff::Value::String(b"GREETER".to_vec()));
    git.root.items_mut(&mg_schema::git::CREATURE_LIST).push(c);
    let mut door = mg_gff::Struct::new(8);
    door.set("Tag", mg_gff::Value::String(b"TO_YARD".to_vec()));
    door.set("LinkedTo", mg_gff::Value::String(b"YARD_GATE".to_vec()));
    git.root.set("Door List", mg_gff::Value::List(vec![door]));
    m.set_gff(key("start", ResType::GIT), &git).unwrap();
    // yard: its gate, and a waypoint its own door leads to.
    m.set(key("yard", ResType::ARE), Gff::new(*b"ARE ").to_bytes().unwrap());
    let mut yard = Gff::new(*b"GIT ");
    let mut gate = mg_gff::Struct::new(8);
    gate.set("Tag", mg_gff::Value::String(b"YARD_GATE".to_vec()));
    gate.set("LinkedTo", mg_gff::Value::String(b"YARD_SPOT".to_vec()));
    yard.root.set("Door List", mg_gff::Value::List(vec![gate]));
    let mut spot = mg_gff::Struct::new(5);
    spot.set("Tag", mg_gff::Value::String(b"YARD_SPOT".to_vec()));
    yard.root.set("WaypointList", mg_gff::Value::List(vec![spot]));
    m.set_gff(key("yard", ResType::GIT), &yard).unwrap();
    let caller = key("caller", ResType::NSS);
    m.set(caller, b"void main() {\n  ExecuteScript(\"hello\", OBJECT_SELF);\n}\n".to_vec());
    m.set(key("hello", ResType::NCS), b"NCS V1.0".to_vec());
    m.save().unwrap();

    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    let hello = key("hello", ResType::NSS);
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(hello));
    h.run_steps(3);
    h.get_by_label("Delete hello.nss and its compiled script?");
    h.get_by_label("1 place in the module names it, left as they are:");
    h.get_by_label("  start › creature GREETER › OnSpawn");
    h.get_by_label("Spelled out in scripts (1), which may mean it:");
    h.get_by_label_contains("caller.nss line 2: ExecuteScript(\"hello\", OBJECT_SELF);");
    // Find References from it: the window gives way to the tab.
    h.get_by_label("Find References").click();
    h.run_steps(2);
    assert_eq!(h.state().confirm_delete, None);
    assert!(h.state().dock.find_tab(&Tab::References).is_some());
    // The yard: the door into it from the start, not its own door to
    // its own waypoint, nor its entry in the module's area list.
    let yard_are = key("yard", ResType::ARE);
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(yard_are));
    h.run_steps(3);
    h.get_by_label("Delete the area yard and everything placed in it?");
    h.get_by_label("1 place in the module names it, left as they are:");
    h.get_by_label("  start › door TO_YARD › LinkedTo");
    h.get_by_label("Cancel").click();
    h.run_steps(2);
    // Several chosen: the start area stays; what names the script from
    // the caller, deleted with it, is not said.
    let chosen = vec![key("start", ResType::ARE), hello, caller];
    h.state_mut().actions.push(mg_ui::Action::DeleteChosenDialog(chosen));
    h.run_steps(3);
    h.get_by_label_contains("Delete these 2 resources");
    h.get_by_label("Not deleted: start.are: the module's start location is in it.");
    h.get_by_label("  start › creature GREETER › OnSpawn");
    // (The References tab, opened above, says its own.)
    assert!(h.query_by_label_contains("which may mean").is_none());
    h.get_by_label("Delete").click();
    h.run_steps(3);
    let module = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().module.clone();
    let gone = [hello, key("hello", ResType::NCS), caller];
    assert!(gone.iter().all(|k| !module(&h).contains(k)), "{:?}", h.state().log.entries);
    assert!(module(&h).contains(&key("start", ResType::ARE)));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Delete 2 resources"));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert!(gone.iter().all(|k| module(&h).contains(k)), "one undo brings them back");
}

/// An area opens out, in the module tree, to what is placed in it, kind
/// by kind; a click on an object goes to it in the area's view.
#[test]
fn the_module_tree_lists_what_is_placed_in_an_area() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-contents") else { return };
    tall_tree(&mut h);
    assert!(h.query_by_label("Waypoints (2)").is_none());
    h.get_by_label_contains("Placed Objects in ").click();
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
    let filter = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap();
    filter.click();
    h.run_steps(2);
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap()
        .type_text("ferry");
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
    let mut h = window_harness(app);
    h.run();
    // By ResRef, as Aurora lists them.
    tree_row(&h, "area467");
    assert!(h.query_by_label("The Docks").is_none());
    h.state_mut().settings.area_names = true;
    h.run();
    assert!(h.query_by_label("area467").is_none());
    let y = |h: &Harness<'_, Moonglow>, label: &str| tree_row(h, label).rect().top();
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
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap()
        .click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap()
        .type_text("apple");
    h.run();
    tree_row(&h, "Apple Inn");
    // (The area's tab, open, keeps its name: the tree's rows are looked
    // in.)
    assert!(h.get_by_label("Module Tree Rows").query_by_label("A Quay").is_none());
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
    let mut h = window_harness(app);
    h.run();
    h.state_mut().settings.area_names = true;
    h.run();
    tree_row(&h, "Übelwald");
    for typed in ["Übel", "ÜBEL", "übel"] {
        h.get_all_by_role(egui::accesskit::Role::TextInput)
            .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
            .unwrap()
            .click();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.get_all_by_role(egui::accesskit::Role::TextInput)
            .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
            .unwrap()
            .type_text(typed);
        h.run();
        assert!(h.get_by_label("Module Tree Rows").query_by_label("Übelwald").is_some(), "{typed}");
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
    let mut h = window_harness(app);
    h.run();
    // Open the groups (the filter opens every group with a match).
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap()
        .click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap()
        .type_text("e");
    h.run();
    h.get_by_label("creature007.utc");
    h.state_mut().settings.area_names = true;
    h.run();
    h.get_by_label("Hent Fynolds");
    h.get_by_label("nameless.utc");
    assert!(h.query_by_label("creature007.utc").is_none());
    // (The placeables' group is below the pane's edge: scrolled to.)
    let tree = h.get_by_label("Hent Fynolds").rect().center();
    wheel_to(&mut h, tree, "Old Chest");
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
    let view = (s.dock.iter_leaves())
        .find(|(_, l)| l.tabs.contains(&Tab::Area(area)))
        .map(|(_, l)| l.rect)
        .expect("the area's pane");
    let palette = right_dock_rect(&h);
    let window = AREA_HARNESS_WIDTH;
    assert!(view.left() <= window * 0.4 + 20.0, "the tree ends by {}", view.left());
    assert!(view.width() > 150.0, "the area's view: {view:?}");
    assert!(palette.width() > 150.0 && palette.right() <= window, "the palettes: {palette:?}");
    assert!(view.right() <= palette.left(), "the view beside the palettes: {view:?} {palette:?}");
    // The Right Dock dragged (or remembered) wider than the window is held
    // to two fifths of it: the area's view keeps its room.
    right_dock_width(&mut h, 5000.0);
    h.run_steps(3);
    let palette = right_dock_rect(&h);
    assert!(palette.width() <= window * 0.4 + 2.0, "the palettes: {palette:?}");
    let view = (h.state().dock.iter_leaves())
        .find(|(_, l)| l.tabs.contains(&Tab::Area(area)))
        .map(|(_, l)| l.rect)
        .unwrap();
    assert!(view.width() > 150.0, "the area's view keeps some room: {view:?}");
}

/// The Delete key on a row of the module tree asks as its menu's Delete…
/// does.
#[test]
fn delete_on_a_tree_row_asks_first() {
    let Some((mut h, key)) = plugin_harness("ui-tree-delete-key", false) else { return };
    h.run();
    // The guard's row (its type opened out), under the pointer.
    module_tool(&h, "Expand All").click();
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
    let Some((mut h, area)) = area_harness("tree-shows-selected") else { return };
    tall_tree(&mut h);
    // (The tree follows the selection where the Outliner is not shown.)
    outliner_away(&mut h);
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
    h.get_by_label_contains("Placed Objects in ").click();
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
    let row = tree_row(&h, "field").rect();
    let arrow = h.get_by_label("Placed Objects in field");
    let r = arrow.rect();
    assert!(r.right() <= row.left() && (r.center().y - row.center().y).abs() < 4.0, "{r:?}");
    assert_eq!(arrow.accesskit_node().data().is_expanded(), Some(true), "open before the click");
    arrow.click();
    h.run_steps(6);
    assert!(h.query_by_label("Waypoints (2)").is_none());
    let arrow = h.get_by_label("Placed Objects in field");
    assert_eq!(arrow.accesskit_node().data().is_expanded(), Some(false), "closed by it");
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
    tall_tree(&mut h);
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
    h.get_by_label_contains("Placed Objects in ").click();
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
    // One far down, with the Outliner shown (the area in front): the tree
    // stays where it is, the row marked though out of view (the owner's
    // ruling of 2026-11-14: the Outliner follows the selection).
    assert!(h.query_by_label("WP_39").is_none(), "out of view");
    let still = h.get_by_label("WP_01").rect();
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    h.run_steps(8);
    assert!(h.query_by_label("WP_39").is_none(), "not scrolled to");
    assert_eq!(h.get_by_label("WP_01").rect(), still, "the tree as it was");
    assert_eq!(h.state().tree_object, Some((area, ObjectKind::Waypoint, 0)), "marked");
    // Without the Outliner (another tab in front): brought into view.
    outliner_away(&mut h);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 1)];
    let started = std::time::Instant::now();
    while h.query_by_label("WP_38").is_none() {
        assert!(started.elapsed().as_secs() < 10, "never: scrolled to");
        h.step();
    }
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
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), crate::common::docks_as_set(app));
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
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), crate::common::docks_as_set(app));
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

/// Several resources chosen in the module tree: a click chooses one, Shift
/// and a click those from it to another, as listed, Ctrl and a click adds
/// one or takes it away; they are marked, and a right click on one of them
/// acts on them all (Export…, Export as Files…; Edit Together for
/// blueprints of one kind). A click alone chooses one again, whose menu is
/// its own.
#[test]
fn several_resources_are_chosen_in_the_tree_and_exported_together() {
    let dir = mg_testkit::scratch_dir("ui-tree-several");
    let mut m = Module::new();
    for name in ["s1", "s2", "s3", "s4", "s5"] {
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    let path = dir.join("several.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let out = dir.join("out");
    let _ = std::fs::remove_dir_all(&out);
    let dialogs = NoDialogs { folders: vec![out.clone()], ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.get_by_label_contains("Scripts (").click();
    h.run_steps(3);
    let s = |name: &str| ResKey::parse(name, ResType::NSS).unwrap();
    let chosen = |h: &Harness<'_, Moonglow>| -> Vec<ResKey> {
        h.state().tree_selected.iter().copied().collect()
    };
    h.get_by_label("s1").click();
    h.run_steps(2);
    assert_eq!(chosen(&h), [s("s1")]);
    h.get_by_label("s3").click_modifiers(egui::Modifiers::SHIFT);
    h.run_steps(2);
    assert_eq!(chosen(&h), [s("s1"), s("s2"), s("s3")], "from the one clicked to this one");
    h.get_by_label("s5").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    h.get_by_label("s2").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    assert_eq!(chosen(&h), [s("s1"), s("s3"), s("s5")], "s5 added, s2 taken away");
    // Their menu: for them all.
    h.get_by_label("s3").click_secondary();
    h.run_steps(2);
    h.get_by_label("3 resources chosen");
    assert!(h.query_by_label("Rename…").is_none(), "not one's own menu");
    open_submenu(&mut h, "Export");
    h.get_by_label("Export as Files…").click();
    h.run_steps(3);
    let mut written: Vec<String> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    written.sort();
    assert_eq!(written, ["s1.nss", "s3.nss", "s5.nss"]);
    h.get_by_label("s5").click_secondary();
    h.run_steps(2);
    open_submenu(&mut h, "Export");
    h.get_by_label("Export…").click();
    h.run_steps(2);
    let export = h.state().export.as_ref().expect("the Export window");
    let wanted: std::collections::BTreeSet<ResKey> = [s("s1"), s("s3"), s("s5")].into();
    assert_eq!(export.selected, wanted);
    h.state_mut().export = None;
    h.run();
    // A click alone: that one, and its own menu.
    h.get_by_label("s4").click();
    h.run_steps(2);
    assert_eq!(chosen(&h), [s("s4")]);
    h.get_by_label("s4").click_secondary();
    h.run_steps(2);
    assert!(h.query_by_label("Rename…").is_some());
}

/// The module tree follows Module Properties' tab (its row marked) and a
/// tab of several (Properties of two blueprints: both marked, as chosen
/// together); a row gone to from below is brought to the middle of the
/// pane, not left at its foot; and a tab whose row the Filter hides
/// chosen while it has something typed: the row is listed under it, the
/// Filter as it was, until what is typed changes.
#[test]
fn the_module_tree_follows_module_properties_tabs_of_several_and_the_filter() {
    let dir = mg_testkit::scratch_dir("ui-tree-follows-more");
    let mut m = Module::new();
    for n in 0..80 {
        let key = ResKey::parse(&format!("script_{n:02}"), ResType::NSS).unwrap();
        m.set(key, b"void main()\n{\n}\n".to_vec());
    }
    let utc = |name: &str| ResKey::parse(name, ResType::UTC).unwrap();
    for name in ["c1", "c2"] {
        m.set(utc(name), Gff::new(*b"UTC ").to_bytes().unwrap());
    }
    let path = dir.join("follows.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1100.0, 760.0);
    h.run_steps(3);
    let open = |h: &mut Harness<'_, Moonglow>, tab: Tab| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run_steps(12);
    };
    let script = |n: u32| ResKey::parse(&format!("script_{n:02}"), ResType::NSS).unwrap();
    // (Module Properties' tab was opened with the module: away first.)
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    h.run_steps(3);
    open(&mut h, Tab::Script(script(40)));
    assert_eq!(h.state().tree_marked, Some(script(40)));
    // In the middle of the pane: the rows after it in sight too.
    let row = h.get_by_label("script_40").rect();
    assert!(h.query_by_label("script_45").is_some(), "the rows after it are in sight");
    let above = h.query_by_label("script_35").map(|n| n.rect().top());
    assert!(above.is_some_and(|t| t < row.top()), "and those before it");
    // Module Properties: its row marked.
    open(&mut h, Tab::ModuleProperties);
    let module = ResKey::parse("module", ResType::IFO).unwrap();
    assert_eq!(h.state().tree_marked, Some(module));
    // Two blueprints' Properties: both chosen, the first marked.
    open(&mut h, Tab::Blueprints(vec![utc("c1"), utc("c2")]));
    assert_eq!(h.state().tree_marked, Some(utc("c1")));
    let both: std::collections::BTreeSet<ResKey> = [utc("c1"), utc("c2")].into();
    assert_eq!(h.state().tree_selected, both);
    // The Filter typed, then a script it hides chosen: listed under it.
    // (The windows closed, which lie over the tree's Filter; it is found
    // by what it says while empty.)
    let tabs = vec![
        Tab::Script(script(40)),
        Tab::ModuleProperties,
        Tab::Blueprints(vec![utc("c1"), utc("c2")]),
    ];
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(tabs));
    h.run_steps(3);
    type_into_tree_filter(&mut h, "script_0");
    h.run_steps(3);
    assert!(h.query_by_label("script_50").is_none(), "hidden by the Filter");
    open(&mut h, Tab::Script(script(50)));
    assert!(h.query_by_label("script_50").is_some(), "listed under the Filter");
    assert!(h.query_by_label("script_03").is_some(), "with what it finds");
    assert!(h.query_by_label("script_60").is_none());
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    assert!(h.get_all_by_value("script_0").any(|n| is_input(&n)), "the Filter stays");
    // Something else typed: the Filter's own again.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Script(script(50))));
    h.run_steps(3);
    h.get_all_by_value("script_0").find(is_input).unwrap().click();
    h.run_steps(2);
    h.key_press(egui::Key::End);
    filter_type(&mut h, "1");
    h.run_steps(3);
    assert!(h.query_by_label("script_01").is_some());
    assert!(h.query_by_label("script_50").is_none());
}

/// Placed objects chosen together in the module tree (Ctrl and a click):
/// the area's view selects them, they are marked, and a right click on one
/// of them copies or deletes them all (one step to undo). The Filter finds
/// what is placed in an area not opened out (it reads every area's).
#[test]
fn placed_objects_are_chosen_together_and_found_in_areas_not_opened_out() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-objects-several") else { return };
    tall_tree(&mut h);
    assert!(h.query_by_label("NW_WAYPOINT001").is_none(), "not opened out");
    // The Filter: what is placed there, without opening the area out.
    let filter = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap();
    filter.click();
    h.run_steps(2);
    filter_type(&mut h, "nw_waypoint001");
    // (Its kind's list opening.)
    h.run_steps(30);
    assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), 2, "found in the area closed");
    h.get_by_label("Waypoints (2 of 2)");
    h.get_by_label("Clear Filter").click();
    h.run_steps(4);
    assert!(h.query_by_label("NW_WAYPOINT001").is_none(), "closed again");
    // Opened out (its waypoints open still, as the Filter opened them):
    // one clicked, another with Ctrl.
    h.get_by_label_contains("Placed Objects in ").click();
    h.run_steps(30);
    let rows: Vec<egui::Pos2> =
        h.query_all_by_label("NW_WAYPOINT001").map(|n| n.rect().center()).collect();
    assert_eq!(rows.len(), 2);
    h.hover_at(rows[0]);
    press(&h, rows[0], true, egui::Modifiers::NONE);
    press(&h, rows[0], false, egui::Modifiers::NONE);
    h.run_steps(40);
    let second = h.query_all_by_label("NW_WAYPOINT001").nth(1).unwrap();
    second.click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(3);
    let both = vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    let mut selection = h.state().area_views[&area].selection.clone();
    selection.sort();
    assert_eq!(selection, both, "the area's view selects them");
    assert_eq!(h.state().tree_objects.as_ref().map(|(a, s)| (*a, s.len())), Some((area, 2)));
    // Their menu: copied together, then deleted together.
    h.query_all_by_label("NW_WAYPOINT001").nth(1).unwrap().click_secondary();
    h.run_steps(3);
    h.get_by_label("2 objects chosen");
    assert!(h.query_by_label("Variables…").is_none(), "one object's alone");
    h.get_by_label("Copy").click();
    h.run_steps(3);
    assert_eq!(h.state().object_clip.as_ref().map(|c| c.objects.len()), Some(2));
    h.query_all_by_label("NW_WAYPOINT001").next().unwrap().click_secondary();
    h.run_steps(3);
    h.get_by_label("Delete").click();
    h.run_steps(4);
    // (The kind's, and the Waypoints group of blueprints.)
    assert_eq!(h.query_all_by_label("Waypoints (0)").count(), 2);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Delete"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(4);
    h.get_by_label("Waypoints (2)");
}

/// A placed object copied from the module tree with its area's view not
/// open is as high above the ground as it stood (its area's ground read
/// for it), as one copied with the view open is.
#[test]
fn a_placed_object_copied_with_its_area_closed_keeps_its_height() {
    let Some((mut h, area)) = area_harness("tree-copy-closed") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let z = ws.doc(&git).unwrap().root.list("WaypointList").unwrap()[0].float("ZPosition").unwrap();
    let path = mg_edit::GffPath::root().item("WaypointList", 0);
    let edit = mg_edit::Edit::SetField {
        key: git,
        path,
        label: "ZPosition".into(),
        value: Some(mg_gff::Value::Float(z + 2.0)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Lift", vec![edit])));
    h.run_steps(3);
    // The area's tab closed, and its view let go.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Area(area)));
    h.run_steps(3);
    h.state_mut().area_views.remove(&area);
    h.get_by_label_contains("Placed Objects in ").click();
    h.run_steps(3);
    h.get_by_label("Waypoints (2)").click();
    h.run_steps(3);
    let lifted = h.query_all_by_label("NW_WAYPOINT001").next().unwrap();
    lifted.click_secondary();
    h.run_steps(3);
    h.get_by_label("Copy").click();
    h.run_steps(3);
    assert!(!h.state().area_views.contains_key(&area), "no view open");
    let clip = h.state().object_clip.as_ref().expect("copied");
    let lifts: Vec<f32> = clip.objects.iter().map(|o| o.2).collect();
    // (Listed by tag, alike: the first listed is the first placed.)
    assert!((lifts[0] - 2.0).abs() < 0.05, "lift {lifts:?}");
}

/// Options › General › Hide empty groups leaves the module tree's groups
/// with nothing in them out, and an area's kinds with nothing placed
/// (Carcerian's ask on Discord, 2026-11-06); off, they are listed as
/// Aurora lists them.
#[test]
fn empty_groups_are_hidden_when_asked() {
    let Some((mut h, _area)) = area_harness("tree-hide-empty") else { return };
    h.get_by_label_contains("Placed Objects in ").click();
    h.run_steps(3);
    let empty = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let rows = h.query_all_by_label_contains(" (0)");
        let mut labels: Vec<String> = rows.filter_map(|n| n.accesskit_node().label()).collect();
        labels.sort();
        labels.dedup();
        labels
    };
    let listed = empty(&h);
    assert!(listed.contains(&"Doors (0)".to_string()), "an area's kind: {listed:?}");
    assert!(listed.contains(&"Conversations (0)".to_string()), "a group: {listed:?}");
    h.state_mut().settings.tree_hides_empty = true;
    h.run_steps(3);
    assert_eq!(empty(&h), Vec::<String>::new());
    assert!(h.query_by_label("Waypoints (2)").is_some(), "what has something stays");
    assert!(h.query_by_label_contains("Areas (").is_some());
    h.state_mut().settings.tree_hides_empty = false;
    h.run_steps(3);
    assert_eq!(empty(&h), listed);
}

/// Adjust Location… on a placed object's row of the module tree selects
/// it in the area's view and opens Adjust Location for it, as the area
/// view's menu does (Carcerian's ask on Discord, 2026-11-06).
#[test]
fn adjust_location_from_a_placed_object_s_row() {
    use mg_area::ObjectKind::Waypoint;
    let Some((mut h, area)) = area_harness("tree-adjust-location") else { return };
    tall_tree(&mut h);
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    h.get_by_label_contains("Placed Objects in ").click();
    h.run_steps(3);
    h.get_by_label("Waypoints (2)").click();
    h.run_steps(3);
    // The second waypoint (at 30, 20).
    h.query_all_by_label("NW_WAYPOINT001").nth(1).unwrap().click_secondary();
    h.run_steps(3);
    h.get_by_label("Adjust Location…").click();
    h.run_steps(4);
    assert_eq!(h.state().area_views[&area].selection, [(Waypoint, 1)]);
    let adjust = h.state().adjust.clone().expect("Adjust Location open");
    assert_eq!((adjust.area, adjust.objects), (area, vec![(Waypoint, 1)]));
    assert!((adjust.position.x - 30.0).abs() < 1e-3, "{:?}", adjust.position);
    assert!(h.query_by_label("Adjust Location").is_some(), "its window");
}

/// Export as Blueprint… on a placed object (the module tree's menu, and
/// the area view's): written into an archive as a blueprint of its own,
/// named as Add to Palette names one, without where it stood, and not
/// added to the module (GitHub issue 7).
#[test]
fn a_placed_object_is_exported_as_a_blueprint() {
    let Some((mut h, area)) = area_harness("tree-export-object") else { return };
    tall_tree(&mut h);
    let erf = mg_testkit::scratch_dir("ui-tree-export-object").join("waypoint.erf");
    let _ = std::fs::remove_file(&erf);
    h.state_mut().dialogs = Box::new(NoDialogs { save: vec![erf.clone()], ..Default::default() });
    let before = h.state().ws.as_ref().unwrap().module.keys().count();
    h.get_by_label_contains("Placed Objects in ").click();
    h.run_steps(3);
    h.get_by_label("Waypoints (2)").click();
    h.run_steps(3);
    h.query_all_by_label("NW_WAYPOINT001").next().unwrap().click_secondary();
    h.run_steps(3);
    h.get_by_label("Export as Blueprint…").click();
    h.run_steps(3);
    let bytes = std::fs::read(&erf).expect("the archive written");
    let archive = mg_erf::Erf::read(&bytes).unwrap();
    let blueprints: Vec<_> = archive.entries.iter().filter(|e| e.restype == ResType::UTW).collect();
    assert_eq!(blueprints.len(), 1, "{:?}", archive.entries);
    let entry = blueprints[0];
    let key = ResKey::new(entry.resref, entry.restype);
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&key), "not added to the module");
    assert_ne!(entry.resref.to_string(), "nw_waypoint001", "a blueprint of its own");
    let gff = Gff::read(&archive.data(entry).unwrap()).unwrap();
    assert!(gff.root.float("XPosition").is_none(), "where it stood left out");
    assert_eq!(gff.root.resref("TemplateResRef"), Some(entry.resref));
    assert_eq!(h.state().ws.as_ref().unwrap().module.keys().count(), before);
    let said = &h.state().log.entries.last().unwrap().1;
    assert!(said.starts_with(&format!("Exported {key}")), "{said}");
    let _ = area;
}

/// An ARE the module's area list leaves out (the game doesn't load it) is
/// marked in the tree, and its menu's Add to the Module's Area List lists
/// it, one undoable step, after which the mark is gone.
#[test]
fn an_area_its_list_leaves_out_is_marked_and_listed_from_its_menu() {
    let dir = mg_testkit::scratch_dir("ui-tree-unlisted-area");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    m.set(ResKey::parse("lost", ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = window_harness(app);
    h.run();
    assert_eq!(h.get_all_by_label("(not in the area list)").count(), 1);
    tree_row(&h, "lost").click_secondary();
    h.run();
    h.get_by_label("Add to the Module's Area List").click();
    h.run();
    assert!(h.query_by_label("(not in the area list)").is_none());
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    let listed: Vec<String> = ws.module.areas().unwrap().iter().map(|a| a.to_string()).collect();
    assert_eq!(listed, ["start", "lost"]);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(h.get_all_by_label("(not in the area list)").count(), 1);
}

/// A builder's report: closing an area, or a Properties window opened from
/// the module tree, scrolled the tree to its top. The tab come to the front
/// because another closed was not chosen: the tree stays where it was.
#[test]
fn the_module_tree_stays_where_it_was_when_a_tab_closes() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-tab-closed") else { return };
    tall_tree(&mut h);
    // Forty waypoints, for a tree that scrolls.
    let git = ResKey::new(area, ResType::GIT);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let first = ws.doc(&git).unwrap().root.list("WaypointList").unwrap()[0].clone();
    let list: Vec<mg_gff::Struct> = (0..40)
        .map(|i| {
            let mut w = first.clone();
            w.set("Tag", mg_gff::Value::String(format!("WP_{i:02}").into_bytes()));
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
    h.get_by_label_contains("Placed Objects in ").click();
    h.run_steps(3);
    h.get_by_label("Waypoints (40)").click();
    h.run_steps(6);
    // Scrolled down to the last (End over the Areas group's well: with the
    // Outliner shown, the tree no longer scrolls to an object selected in
    // the view, the owner's ruling of 2026-11-14).
    h.hover_at(h.get_by_label("WP_00").rect().center());
    h.key_press(egui::Key::End);
    let started = std::time::Instant::now();
    while h.query_by_label("WP_39").is_none() {
        assert!(started.elapsed().as_secs() < 10, "never: scrolled to");
        h.step();
    }
    // (And selected in the view: its row marked, the tree still.)
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 39)];
    let started = std::time::Instant::now();
    while h.state().tree_object != Some((area, ObjectKind::Waypoint, 39)) {
        assert!(started.elapsed().as_secs() < 10, "never: marked");
        h.step();
    }
    let row = h.get_by_label("WP_39").rect();
    // (How high the Outliner is, which the tree gains as it goes.)
    let outliner = h.get_by_label(mg_ui::OUTLINER).get_by_label("Collapse All").rect();
    let gained = h.ctx.content_rect().bottom() - 24.0 - (outliner.center().y - 13.0);
    // The area closed: Module Properties (opened with the module) in front.
    assert!(h.state().dock.find_tab(&Tab::ModuleProperties).is_some());
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Area(area)));
    h.run_steps(8);
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_none());
    // (The Outliner gone with the area, the tree is taller by its height:
    // the last row stays in sight, as low as it was or lower by no more
    // than that, the tree's end in sight; scrolled to the top, it would be
    // out of sight, 40 rows further.)
    let now = h.query_by_label("WP_39").map(|n| n.rect()).expect("the area closed: in sight");
    let moved = now.top() - row.top();
    assert!((0.0..=gained + 1.0).contains(&moved), "{row:?} → {now:?}, {gained} gained");
    // A placed object's Properties, opened from its row, then closed: Module
    // Properties in front again.
    let path = mg_edit::GffPath::root().item("WaypointList", 39);
    let tab = Tab::Instance { area, path };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(8);
    let row = h.get_by_label("WP_39").rect();
    h.state_mut().actions.push(mg_ui::Action::CloseTab(tab));
    h.run_steps(8);
    assert_eq!(h.query_by_label("WP_39").map(|n| n.rect()), Some(row), "Properties closed");
    // (A tab chosen still moves it: `every_kind_of_tab_chosen_moves_the_module_tree`.)
}

/// A builder's report: with the Filter typed, the kinds of an area it
/// found objects in were held open: a click on one's arrow closed it for a
/// frame. Now the Filter opens them once, as it is typed, and they close.
#[test]
fn the_kinds_the_filter_opens_close_again() {
    let Some((mut h, _)) = area_harness("tree-filter-kinds") else { return };
    let filter = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap();
    filter.click();
    h.run_steps(2);
    filter_type(&mut h, "nw_waypoint001");
    h.run_steps(30);
    assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), 2, "opened by the Filter");
    h.get_by_label("Waypoints (2 of 2)").click();
    h.run_steps(10);
    assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), 0, "closed, and stays closed");
    h.get_by_label("Waypoints (2 of 2)").click();
    h.run_steps(10);
    assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), 2, "opened again");
    h.get_by_label("Waypoints (2 of 2)").click();
    h.run_steps(10);
    // The Filter cleared and typed again: opened once more.
    h.get_by_label("Clear Filter").click();
    h.run_steps(4);
    let filter = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap();
    filter.click();
    h.run_steps(2);
    filter_type(&mut h, "nw_waypoint001");
    h.run_steps(30);
    assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), 2, "typed again");
}

/// El Grillo's report after the kinds' fix: with the Filter typed, an
/// area it found objects in was held opened out (its own arrow undone the
/// next frame). Now it closes, and stays closed while the Filter has the
/// same; its arrow opens it again; Collapse All closes it too; and typed
/// anew, the Filter opens it out once more.
#[test]
fn an_area_the_filter_opens_out_closes_again() {
    let Some((mut h, _)) = area_harness("tree-filter-area") else { return };
    let typed = |h: &mut Harness<'_, Moonglow>, text: &str| {
        let filter = h
            .get_all_by_role(egui::accesskit::Role::TextInput)
            .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
            .unwrap();
        filter.click();
        h.run_steps(2);
        filter_type(h, text);
        h.run_steps(30);
    };
    let kinds = |h: &Harness<'_, Moonglow>| h.query_all_by_label("Waypoints (2 of 2)").count();
    typed(&mut h, "nw_waypoint001");
    assert_eq!(kinds(&h), 1, "opened out by the Filter");
    // (The area's arrow, said open, just above its kinds and left of them.)
    let under = h.get_by_label("Waypoints (2 of 2)").rect();
    let arrow = h.get_by_label_contains("Placed Objects in ");
    assert_eq!(arrow.accesskit_node().data().is_expanded(), Some(true));
    assert!(arrow.rect().left() < under.left() && arrow.rect().top() < under.top());
    arrow.click();
    h.run_steps(10);
    assert_eq!(kinds(&h), 0, "closed, and stays closed");
    h.get_by_label_contains("Placed Objects in ").click();
    h.run_steps(10);
    assert_eq!(kinds(&h), 1, "opened out again");
    // Collapse All, then the Filter cleared and typed again: opened out once more.
    // (The tree's, on the Module panel's header.)
    module_tool(&h, "Collapse All").click();
    h.run_steps(10);
    assert_eq!(kinds(&h), 0, "Collapse All");
    h.get_by_label("Clear Filter").click();
    h.run_steps(4);
    typed(&mut h, "nw_waypoint001");
    assert_eq!(kinds(&h), 1, "typed again");
}

/// What the Filter finds in an area is kept between frames, and follows
/// what is placed there: a placed object deleted is no longer listed under
/// it, the area found for its objects alone goes with the last of them,
/// and both are back with Undo.
#[test]
fn the_filter_follows_what_is_placed_as_it_changes() {
    let Some((mut h, _)) = area_harness("tree-filter-follows") else { return };
    let filter = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap();
    filter.click();
    h.run_steps(2);
    filter_type(&mut h, "nw_waypoint001");
    settle(&mut h);
    h.run_steps(30);
    h.get_by_label("Areas (1)");
    h.get_by_label("Waypoints (2 of 2)");
    let delete_one = |h: &mut Harness<'_, Moonglow>| {
        h.query_all_by_label("NW_WAYPOINT001").next().unwrap().click_secondary();
        h.run_steps(3);
        h.get_by_label("Delete").click();
        settle(h);
        h.run_steps(4);
    };
    delete_one(&mut h);
    h.get_by_label("Waypoints (1 of 1)");
    assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), 1);
    delete_one(&mut h);
    h.get_by_label("Areas (0)");
    assert!(h.query_by_label("NW_WAYPOINT001").is_none(), "the area is found no more");
    for (left, kinds) in [(1, "Waypoints (1 of 1)"), (2, "Waypoints (2 of 2)")] {
        h.state_mut().actions.push(mg_ui::Action::Undo);
        settle(&mut h);
        h.run_steps(30);
        h.get_by_label("Areas (1)");
        h.get_by_label(kinds);
        assert_eq!(h.query_all_by_label("NW_WAYPOINT001").count(), left);
    }
}

/// Resources chosen together in the module tree are chosen no more once
/// they are deleted or renamed (a plugin's command run on the tree's
/// selection was handed them: the review's S26), with the tree shown or
/// folded away; those left stay chosen.
#[test]
fn resources_deleted_or_renamed_are_chosen_no_more() {
    let dir = mg_testkit::scratch_dir("ui-tree-selection-pruned");
    let path = dir.join("m.mod");
    let key = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for n in ["one", "two", "three", "four"] {
        m.set(key(n), b"void main() {}\n".to_vec());
    }
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    let chosen = |h: &Harness<'_, Moonglow>| h.state().tree_selected.iter().copied().collect();
    let gone = |k: ResKey| mg_edit::Edit::SetResource { key: k, data: None };
    for hidden in [false, true] {
        h.state_mut().settings.hide_tree = hidden;
        h.state_mut().tree_selected = [key("one"), key("two"), key("three")].into();
        h.run();
        let all: Vec<ResKey> = chosen(&h);
        assert_eq!(all.len(), 3, "all in the module: all chosen");
        let delete = mg_edit::Command::new("Delete", vec![gone(key("one")), gone(key("two"))]);
        h.state_mut().actions.push(mg_ui::Action::Apply(delete));
        h.run();
        h.run();
        let left: Vec<ResKey> = chosen(&h);
        assert_eq!(left, [key("three")], "tree folded away: {hidden}");
        // Renamed: the old name is no resource.
        let renamed = mg_edit::Edit::SetResource {
            key: key("third"),
            data: Some(b"void main() {}\n".to_vec()),
        };
        let rename = mg_edit::Command::new("Rename", vec![gone(key("three")), renamed]);
        h.state_mut().actions.push(mg_ui::Action::Apply(rename));
        h.run();
        h.run();
        let left: Vec<ResKey> = chosen(&h);
        assert!(left.is_empty(), "{left:?}");
        // (Back, for the other way round.)
        h.state_mut().actions.push(mg_ui::Action::Undo);
        h.run();
        h.state_mut().actions.push(mg_ui::Action::Undo);
        h.run();
    }
}

/// A plugin whose command is for every kind the module tree lists
/// ("Each Kind"), in a folder of installed plugins of its own.
fn each_kind_plugin(name: &str) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let plugin = dir.join("each");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.cfg"),
        "[plugin]\nid = \"test.each\"\nname = \"Each\"\nversion = \"1\"\napi = \"0.5\"\n\
         license = \"MIT\"\n\n[command]\nid = \"each\"\ntitle = \"Each Kind\"\nfor = \"area, \
         dlg, nss, jui, jrl, fac, creature, door, encounter, item, placeable, sound, store, \
         trigger, waypoint\"\n",
    )
    .unwrap();
    std::fs::write(
        plugin.join("main.luau"),
        "local mg = require(\"@moonglow\")\nmg.command(\"each\", function(ctx)\n  local on = {}\n  \
         for _, s in ipairs(ctx.selection) do table.insert(on, s.resource) end\n  \
         ctx.log:info(\"each: \" .. table.concat(on, \",\"))\nend)\n",
    )
    .unwrap();
    dir
}

/// Delete… asks first; Don't ask when nothing else names them makes it
/// Options › General › Questions' switch: what nothing names is then
/// deleted at once, and what something names is still asked about (a
/// builder's ask: a question always given the same answer).
#[test]
fn delete_is_answered_once_for_what_nothing_names() {
    let dir = mg_testkit::scratch_dir("ui-delete-always");
    let path = dir.join("m.mod");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for n in ["one", "two"] {
        m.set(ResKey::parse(n, ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    }
    m.set(
        ResKey::parse("caller", ResType::NSS).unwrap(),
        b"void main() { ExecuteScript(\"two\", OBJECT_SELF); }\n".to_vec(),
    );
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    let has = |h: &Harness<'_, Moonglow>, n: &str| {
        h.state().ws.as_ref().unwrap().module.contains(&ResKey::parse(n, ResType::NSS).unwrap())
    };
    let one = ResKey::parse("one", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(one));
    h.run_steps(3);
    h.get_by_label("Don't ask when nothing else names them").click();
    h.run_steps(2);
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert!(!has(&h, "one"));
    assert!(h.state().settings.delete_without_asking);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    // Not asked again where nothing names it.
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(one));
    h.run_steps(4);
    assert!(h.state().confirm_delete.is_none(), "not asked");
    assert!(!has(&h, "one"), "deleted at once");
    // Asked about what something names, saying what.
    let two = ResKey::parse("two", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(two));
    h.run_steps(4);
    assert!(h.state().confirm_delete.is_some(), "asked");
    h.get_by_label_contains("Spelled out in scripts (1)");
    assert!(has(&h, "two"));
}

/// A right click on the row `label` (the pointer brought to it first, as
/// a builder's is), and the frames that open its menu.
fn right_click(h: &mut Harness<'_, Moonglow>, label: &str) {
    h.get_by_label(label).hover();
    h.run_steps(1);
    h.get_by_label(label).click_secondary();
    h.run_steps(3);
}

/// Several resources of every kind the module tree lists, chosen as a
/// builder chooses them (a click, Shift and a click, Ctrl and a click),
/// have the menu of several on a right click on one of them: Delete… for
/// them all, Export, and the plugins' commands for their kind; the Delete
/// key on one of them asks about them all too. Several of different
/// kinds have Delete… and Export. (El Grillo's report, 3.6.0: the menu
/// of several scripts or conversations had no Delete…, which releases
/// before 3.0.0 left out.)
#[test]
fn several_of_every_kind_have_delete_and_export_in_their_menu() {
    let dir = mg_testkit::scratch_dir("ui-tree-several-kinds");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    // Each group's three (the journal and the factions: one each).
    let mut groups: Vec<(String, Vec<ResKey>)> = Vec::new();
    let mut add = |group: &str, t: ResType, names: &[&str], bytes: &dyn Fn() -> Vec<u8>| {
        let keys: Vec<ResKey> = names.iter().map(|n| ResKey::parse(n, t).unwrap()).collect();
        for k in &keys {
            m.set(*k, bytes());
        }
        groups.push((group.to_string(), keys));
    };
    let gff = |sig: &[u8; 4]| Gff::new(*sig).to_bytes().unwrap();
    add("Areas", ResType::ARE, &["area_1", "area_2", "area_3"], &|| gff(b"ARE "));
    add("Conversations", ResType::DLG, &["talk_1", "talk_2", "talk_3"], &|| gff(b"DLG "));
    add("Scripts", ResType::NSS, &["code_1", "code_2", "code_3"], &|| b"void main() {}\n".to_vec());
    add("NUI windows", ResType::JUI, &["nui_1", "nui_2", "nui_3"], &|| b"{}".to_vec());
    for kind in mg_rules::ObjectKind::ALL {
        let t = kind.restype();
        let sig: [u8; 4] = format!("{:<4}", t.extension().unwrap().to_ascii_uppercase())
            .into_bytes()
            .try_into()
            .unwrap();
        let names: Vec<String> = (1..=3).map(|i| format!("{}_{i}", kind.name())).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        add(kind.label(), t, &names, &|| gff(&sig));
    }
    m.set(ResKey::parse("module", ResType::JRL).unwrap(), gff(b"JRL "));
    m.set(ResKey::parse("repute", ResType::FAC).unwrap(), gff(b"FAC "));
    groups.push((
        "Journal and factions".into(),
        vec![
            ResKey::parse("module", ResType::JRL).unwrap(),
            ResKey::parse("repute", ResType::FAC).unwrap(),
        ],
    ));
    for k in m.keys_of(ResType::ARE).copied().collect::<Vec<_>>() {
        m.set(ResKey::new(k.resref, ResType::GIT), gff(b"GIT "));
    }
    let path = dir.join("kinds.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.plugin_dir = Some(each_kind_plugin("ui-tree-several-kinds-plugins"));
    app.load_plugins();
    app.enable_plugin("test.each", true);
    let mut h = harness(app, 1400.0, 900.0);
    h.run();
    let label = |k: &ResKey| match k.restype {
        ResType::ARE | ResType::DLG | ResType::NSS => k.resref.to_string(),
        _ => k.to_string(),
    };
    let chosen = |h: &Harness<'_, Moonglow>| -> Vec<ResKey> {
        h.state().tree_selected.iter().copied().collect()
    };
    for (group, keys) in &groups {
        let header = format!("{group} ({})", keys.len());
        let rows: Vec<String> = keys.iter().map(label).collect();
        // (The areas' group is open from the start.)
        if h.query_by_label(&rows[0]).is_none() {
            h.get_by_label(&header).click();
            h.run_steps(3);
        }
        // A click on the first, Shift and a click on the last (all three),
        // Ctrl and a click on the middle one (taken away again).
        h.get_by_label(&rows[0]).click();
        h.run_steps(2);
        let last = rows.len() - 1;
        h.get_by_label(&rows[last]).click_modifiers(egui::Modifiers::SHIFT);
        h.run_steps(2);
        assert_eq!(chosen(&h), *keys, "{group}: from the one clicked to this one");
        let wanted: Vec<ResKey> = if keys.len() > 2 {
            h.get_by_label(&rows[1]).click_modifiers(egui::Modifiers::COMMAND);
            h.run_steps(2);
            vec![keys[0], keys[2]]
        } else {
            keys.clone()
        };
        assert_eq!(chosen(&h), wanted, "{group}");
        right_click(&mut h, &rows[last]);
        h.get_by_label(&format!("{} resources chosen", wanted.len()));
        h.get_by_label("Export");
        // (A plugin's command where they are of one kind.)
        let one_kind = keys.iter().all(|k| k.restype == keys[0].restype);
        assert_eq!(h.query_by_label("Each Kind").is_some(), one_kind, "{group}");
        h.get_by_label("Delete…").click();
        h.run_steps(3);
        assert_eq!(h.state().confirm_delete.as_ref(), Some(&wanted), "{group}");
        h.get_all_by_label("Cancel").last().unwrap().click();
        h.run_steps(2);
        // The Delete key on one of them: the same question.
        h.get_by_label(&rows[0]).hover();
        h.run_steps(1);
        h.key_press(egui::Key::Delete);
        h.run_steps(3);
        assert_eq!(h.state().confirm_delete.as_ref(), Some(&wanted), "{group}: the Delete key");
        h.get_all_by_label("Cancel").last().unwrap().click();
        h.run_steps(2);
        // The plugin's command runs on them all.
        if one_kind {
            right_click(&mut h, &rows[0]);
            h.get_by_label("Each Kind").click();
            h.run_steps(4);
            let said = (h.state().log.entries.iter().rev())
                .find_map(|(_, m)| m.strip_prefix("Each: each: ").map(str::to_string));
            let names: Vec<String> = wanted.iter().map(ToString::to_string).collect();
            assert_eq!(said, Some(names.join(",")), "{group}");
        }
        h.get_by_label(&header).click();
        h.run_steps(3);
    }
    // A conversation and a script together: Delete… and Export for both.
    h.get_by_label("Conversations (3)").click();
    h.run_steps(3);
    h.get_by_label("Scripts (3)").click();
    h.run_steps(3);
    h.get_by_label("talk_2").click();
    h.run_steps(2);
    h.get_by_label("code_3").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    right_click(&mut h, "code_3");
    h.get_by_label("2 resources chosen");
    h.get_by_label("Export");
    assert!(h.query_by_label("Each Kind").is_none(), "of two kinds");
    h.get_by_label("Delete…").click();
    h.run_steps(3);
    let both = vec![ResKey::parse("code_3", ResType::NSS).unwrap(), groups[1].1[1]];
    let mut asked = h.state().confirm_delete.clone().unwrap();
    asked.sort();
    assert_eq!(asked, both);
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run_steps(2);
}

/// Several chosen in the module tree stay chosen when a conversation
/// opened before is clicked in for the first time (a tab opened comes to
/// the front without the focus, and the click that gives it the focus
/// chose no other tab): a right click on one of them still has the menu
/// of several. A tab of another chosen after does take the tree to it.
#[test]
fn several_chosen_stay_when_an_editor_opened_before_is_clicked_in() {
    let dir = mg_testkit::scratch_dir("ui-tree-several-editor");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for name in ["talk_1", "talk_2", "talk_3", "talk_4"] {
        let g = mg_module::dialog::new_dialog();
        m.set(ResKey::parse(name, ResType::DLG).unwrap(), g.to_bytes().unwrap());
    }
    let path = dir.join("editor.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 900.0);
    h.run();
    let k = |name: &str| ResKey::parse(name, ResType::DLG).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenResource(k("talk_1")));
    settle(&mut h);
    let tab = Tab::Dialog(k("talk_1"));
    assert!(h.state().dock.find_tab(&tab).is_some());
    let chosen = |h: &Harness<'_, Moonglow>| -> Vec<ResKey> {
        h.state().tree_selected.iter().copied().collect()
    };
    assert_eq!(chosen(&h), [k("talk_1")], "the tree goes to the tab opened");
    h.get_by_label("talk_2").click();
    h.run_steps(2);
    h.get_by_label("talk_3").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    // A click in the editor's pane (its tab's own body, right of the tree).
    let body = h.state().dock.find_tab(&tab).and_then(|p| h.state().dock.leaf(p.node_path()).ok());
    let at = body.map(|leaf| leaf.rect.center()).expect("the editor's pane");
    click_pos(&h, at);
    h.run_steps(4);
    assert_eq!(chosen(&h), [k("talk_2"), k("talk_3")], "kept");
    right_click(&mut h, "talk_3");
    h.get_by_label("2 resources chosen");
    h.get_by_label("Delete…");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    // Another conversation opened: the tree goes to it.
    h.state_mut().actions.push(mg_ui::Action::OpenResource(k("talk_4")));
    settle(&mut h);
    assert_eq!(chosen(&h), [k("talk_4")]);
}

/// Conversations chosen in the module tree go out together from its
/// Export menu, each a file of its own named after it in a folder asked
/// for, in each of the four formats, exactly as the conversation
/// editor's Export writes one (a change not saved yet included); files of
/// their names there are asked about once, all named; one that can't be
/// read is named in the log, and the rest are written. Not offered for
/// a script, nor for a conversation chosen with a script. (dansen9191's
/// request.)
#[test]
fn several_conversations_are_exported_as_ink_twine_text_and_csv() {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("ui-tree-export-stories");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let k = |name: &str| ResKey::parse(name, ResType::DLG).unwrap();
    for (name, line) in [("gate", "Halt!"), ("inn", "A room?"), ("well", "Splash.")] {
        let mut g = new_dialog();
        let top = add_node(&mut g, Parent::Root, line);
        add_node(&mut g, Parent::Node(Kind::Entry, top), "Goodbye.");
        m.set(k(name), g.to_bytes().unwrap());
    }
    m.set(k("broken"), b"not a conversation".to_vec());
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    let path = dir.join("stories.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 900.0);
    h.run();
    // The inn's editor open, with a change not saved.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(k("inn"))));
    h.run_steps(3);
    let mut changed = h.state_mut().ws.as_mut().unwrap().doc(&k("inn")).unwrap().clone();
    add_node(&mut changed, Parent::Root, "Not saved yet.");
    let set = mg_edit::Edit::SetResource { key: k("inn"), data: Some(changed.to_bytes().unwrap()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Line", vec![set])));
    h.run_steps(3);
    // (The conversations' group opened with the inn's tab.)
    if h.query_by_label("gate").is_none() {
        h.get_by_label("Conversations (4)").click();
        h.run_steps(3);
    }
    h.get_by_label("Scripts (1)").click();
    h.run_steps(3);
    h.get_by_label("gate").click();
    h.run_steps(2);
    h.get_by_label("well").click_modifiers(egui::Modifiers::SHIFT);
    h.run_steps(2);
    let three = [k("gate"), k("inn"), k("well")];
    assert_eq!(h.state().tree_selected.iter().copied().collect::<Vec<_>>(), three);
    let export =
        |h: &mut Harness<'_, Moonglow>, row: &str, item: &str, folder: &std::path::Path| {
            let dialogs = NoDialogs { folders: vec![folder.to_path_buf()], ..Default::default() };
            h.state_mut().dialogs = Box::new(dialogs);
            right_click(h, row);
            open_submenu(h, "Export");
            h.get_by_label(item).click();
            h.run_steps(3);
        };
    let read = |p: std::path::PathBuf| std::fs::read(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
    // The editor's Export of the inn, in each format, for the files to be
    // compared with.
    let single = dir.join("single");
    std::fs::create_dir_all(&single).unwrap();
    for (word, menu, ext) in [
        ("Ink", "Ink…", "ink"),
        ("Twine", "Twine (Twee)…", "twee"),
        ("Text", "Plain text…", "txt"),
        ("CSV", "CSV…", "csv"),
    ] {
        let out = dir.join(format!("out-{ext}"));
        let _ = std::fs::remove_dir_all(&out);
        export(&mut h, "inn", &format!("Export as {word}…"), &out);
        let mut written: Vec<String> = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        written.sort();
        let wanted: Vec<String> = ["gate", "inn", "well"].map(|n| format!("{n}.{ext}")).into();
        assert_eq!(written, wanted, "{word}");
        let said = h
            .state()
            .log
            .entries
            .iter()
            .rev()
            .map(|(_, m)| m.as_str())
            .find(|m| m.starts_with("Wrote"));
        let where_ = out.display().to_string();
        assert_eq!(
            said,
            Some(
                format!("Wrote 3 conversations as {word} to {where_}: {}", wanted.join(", "))
                    .as_str()
            )
        );
        let one = single.join(format!("inn.{ext}"));
        h.state_mut().dialogs =
            Box::new(NoDialogs { save: vec![one.clone()], ..Default::default() });
        h.get_by_label("Export").click();
        h.run_steps(2);
        h.get_by_label(menu).click();
        h.run_steps(3);
        assert_eq!(
            read(out.join(format!("inn.{ext}"))),
            read(one),
            "{word}: as the editor writes it"
        );
    }
    let ink = dir.join("out-ink");
    let inn = String::from_utf8(read(ink.join("inn.ink"))).unwrap();
    assert!(inn.contains("Not saved yet."), "{inn}");
    // Again into the same folder: asked first, the three named; Cancel
    // leaves them.
    std::fs::write(ink.join("gate.ink"), "kept").unwrap();
    export(&mut h, "gate", "Export as Ink…", &ink);
    h.get_by_label_contains("already has 3 of the 3 files");
    for name in ["gate.ink", "inn.ink", "well.ink"] {
        h.get_by_label(name);
    }
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run_steps(3);
    assert_eq!(read(ink.join("gate.ink")), b"kept");
    export(&mut h, "gate", "Export as Ink…", &ink);
    h.get_by_label("Replace 3").click();
    h.run_steps(3);
    assert!(h.state().dialog_export_ask.is_none());
    assert_ne!(read(ink.join("gate.ink")), b"kept", "overwritten");
    // One that can't be read: named, with why; the rest written.
    h.get_by_label("broken").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    let out = dir.join("out-broken");
    let _ = std::fs::remove_dir_all(&out);
    export(&mut h, "broken", "Export as Ink…", &out);
    assert!(out.join("gate.ink").exists() && out.join("well.ink").exists());
    assert!(!out.join("broken.ink").exists());
    let said = h
        .state()
        .log
        .entries
        .iter()
        .rev()
        .map(|(_, m)| m.clone())
        .find(|m| m.starts_with("Not written"));
    assert!(
        said.as_deref().is_some_and(|s| s.starts_with("Not written: broken.dlg: ")),
        "{said:?}"
    );
    // One conversation alone: offered; a script, and a conversation with a
    // script: not.
    h.get_by_label("well").click();
    h.run_steps(2);
    right_click(&mut h, "well");
    open_submenu(&mut h, "Export");
    h.get_by_label("Export as Ink…");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    h.get_by_label("hello").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    for row in ["well", "hello"] {
        right_click(&mut h, row);
        h.get_by_label("2 resources chosen");
        open_submenu(&mut h, "Export");
        h.get_by_label("Export as Files…");
        assert!(h.query_by_label("Export as Ink…").is_none(), "a script among them");
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
    }
    h.get_by_label("hello").click();
    h.run_steps(2);
    right_click(&mut h, "hello");
    open_submenu(&mut h, "Export");
    h.get_by_label("Export as Files…");
    assert!(h.query_by_label("Export as Twine…").is_none(), "a script");
}

/// How many area rows of the module tree are opened out to what is placed
/// in them: their arrow ("Placed Objects in" the area) is said open, and
/// points down.
fn opened_out(h: &Harness<'_, Moonglow>) -> usize {
    let open = |n: &egui_kittest::Node<'_>| {
        let said = n.accesskit_node().data().is_expanded() == Some(true);
        let down = icon_drawn(h, mg_uikit::Icon::ChevronDown, n.rect());
        assert_eq!(said, down, "an area's arrow says what it shows");
        said
    };
    h.query_all_by_label_contains("Placed Objects in ").filter(open).count()
}

/// Frames until `done`, for ten seconds at most (an area's view asks for
/// frames as it draws).
fn until_tree(
    h: &mut Harness<'_, Moonglow>,
    what: &str,
    done: impl Fn(&Harness<'_, Moonglow>) -> bool,
) {
    let started = std::time::Instant::now();
    while !done(h) {
        assert!(started.elapsed().as_secs() < 10, "never: {what}");
        h.step();
    }
}

/// The module tree's rows (`Module Tree Rows`) whose names contain `words`.
fn tree_rows_with<'t>(h: &'t Harness<'_, Moonglow>, words: &'t str) -> Vec<egui_kittest::Node<'t>> {
    h.get_by_label("Module Tree Rows").query_all_by_label_contains(words).collect()
}

/// A placed object of a read-only area (another builder's) is not
/// deleted from the module tree, by its Delete key or its menu's Delete:
/// the claims' gate refuses it and says why, and what was selected,
/// marked and the keys' row stay as they were.
#[test]
fn a_read_only_area_s_object_is_not_deleted_from_the_tree() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = read_only_area_harness("tree-read-only-delete") else { return };
    // (The tree its old height: the area's kinds and rows in the well's sight.)
    tall_tree(&mut h);
    let chests = |h: &mut Harness<'_, Moonglow>| {
        let git = ResKey::new(area, ResType::GIT);
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git).unwrap().root.list("Placeable List").map_or(0, |l| l.len())
    };
    h.get_by_label_contains("Placed Objects in ").click();
    until_tree(&mut h, "its kinds", |h| !tree_rows_with(h, "Placeables (1)").is_empty());
    tree_rows_with(&h, "Placeables (1)")[0].click();
    until_tree(&mut h, "the chest's row", |h| !tree_rows_with(h, "Chest").is_empty());
    tree_rows_with(&h, "Chest")[0].click();
    let chosen = [(ObjectKind::Placeable, 0)];
    until_tree(&mut h, "selected", |h| h.state().area_views[&area].selection == chosen);
    let marked = |h: &Harness<'_, Moonglow>| {
        tree_rows_with(h, "Chest")
            .first()
            .is_some_and(|n| n.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True))
    };
    let refused = |h: &Harness<'_, Moonglow>, since: usize| {
        h.state().log.entries.iter().skip(since).any(|e| e.1.contains("read-only"))
    };
    // Its key, the keys the tree's (the row clicked), the pointer away.
    let cursor = h.state().tree_cursor;
    h.hover_at(egui::pos2(600.0, 20.0));
    let said = h.state().log.entries.len();
    h.key_press(egui::Key::Delete);
    until_tree(&mut h, "refused by the key", |h| refused(h, said));
    assert_eq!(chests(&mut h), 1, "not deleted");
    assert_eq!(h.state().area_views[&area].selection, chosen, "still selected");
    assert_eq!(h.state().tree_cursor, cursor, "the keys' row as it was");
    assert!(marked(&h), "its row marked");
    // Its menu's Delete.
    let row = tree_rows_with(&h, "Chest")[0].rect().center();
    h.hover_at(row);
    for pressed in [true, false] {
        let (button, modifiers) = (egui::PointerButton::Secondary, egui::Modifiers::NONE);
        h.event(egui::Event::PointerButton { pos: row, button, pressed, modifiers });
        // (One step for each input event.)
        h.step();
    }
    let said = h.state().log.entries.len();
    h.get_all_by_label("Delete").last().unwrap().click();
    until_tree(&mut h, "refused by the menu", |h| refused(h, said));
    assert_eq!(chests(&mut h), 1, "not deleted");
    assert_eq!(h.state().area_views[&area].selection, chosen, "still selected");
    assert!(marked(&h), "its row marked");
}

/// Going to or selecting an object of a kind hidden in the view, from the
/// module tree (a click, a Ctrl+click), shows its kind, in every view, as
/// its Show toggle would, and the object stays selected (one rule with
/// the Outliner's).
#[test]
fn an_object_of_a_hidden_kind_gone_to_from_the_tree_shows_its_kind() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-hidden-kind") else { return };
    // (The tree its old height: the area's kinds and rows in the well's sight.)
    tall_tree(&mut h);
    let shown =
        |h: &Harness<'_, Moonglow>| h.state().area_views[&area].show[ObjectKind::Waypoint.index()];
    h.get_by_label_contains("Placed Objects in ").click();
    until_tree(&mut h, "its kinds", |h| !tree_rows_with(h, "Waypoints (2)").is_empty());
    tree_rows_with(&h, "Waypoints (2)")[0].click();
    until_tree(&mut h, "its rows", |h| tree_rows_with(h, "NW_WAYPOINT001").len() == 2);
    area_tool(&h, "Show Waypoints").click();
    until_tree(&mut h, "hidden", |h| !shown(h));
    tree_rows_with(&h, "NW_WAYPOINT001")[0].click();
    until_tree(&mut h, "shown and selected", |h| {
        shown(h) && h.state().area_views[&area].selection.len() == 1
    });
    // With Ctrl.
    area_tool(&h, "Show Waypoints").click();
    until_tree(&mut h, "hidden again", |h| {
        !shown(h) && h.state().area_views[&area].selection.is_empty()
    });
    tree_rows_with(&h, "NW_WAYPOINT001")[1].click_modifiers(egui::Modifiers::COMMAND);
    until_tree(&mut h, "shown and chosen", |h| {
        shown(h) && h.state().area_views[&area].selection.len() == 1
    });
}

/// At the window's size at first (the Outliner under the tree, the Areas
/// group's well five rows high), a builder still reaches a placed
/// object's row in the module tree and does there what the tree does:
/// the well scrolled by the wheel to the area's last kind and its rows; a
/// click selects, Ctrl+click chooses another with it; the menu of one of
/// them deletes both (Undo brings them back); one's menu has Export as
/// Blueprint…; and by the keys, from the kind's row, Down and Enter go to
/// an object.
#[test]
fn the_module_tree_s_placed_objects_at_the_size_at_first() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-default-size") else { return };
    let w = |i: usize| (ObjectKind::Waypoint, i);
    h.get_by_label_contains("Placed Objects in ").click();
    until_tree(&mut h, "its kinds", |h| !tree_rows_with(h, "Waypoints (2)").is_empty());
    // (The last kind under the well's foot: the wheel over the well until
    // its row ends above the next group's row, which is under the well.)
    let in_well = |h: &Harness<'_, Moonglow>, words: &str| {
        let rows = h.get_by_label("Module Tree Rows");
        let next = rows.get_by_label("Conversations (0)").rect().top();
        tree_rows_with(h, words).first().is_some_and(|n| n.rect().bottom() <= next + 0.5)
    };
    let wheel = |h: &mut Harness<'_, Moonglow>| {
        // (Over the well: between the Areas group's row and the next's.)
        let rows = h.get_by_label("Module Tree Rows");
        let (top, foot) =
            (rows.get_by_label("Areas (1)").rect(), rows.get_by_label("Conversations (0)").rect());
        let at = egui::pos2(top.center().x, (top.bottom() + foot.top()) / 2.0);
        h.hover_at(at);
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -40.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        // (One step for the wheel's event.)
        h.step();
    };
    // (The wheel's scroll goes on a while after it: until a row rests.)
    let rest = |h: &mut Harness<'_, Moonglow>, words: &str| {
        let at = |h: &Harness<'_, Moonglow>| tree_rows_with(h, words).first().map(|n| n.rect());
        let (mut was, mut same) = (at(h), 0);
        let started = std::time::Instant::now();
        while same < 3 {
            assert!(started.elapsed().as_secs() < 10, "never at rest: {words}");
            h.step();
            let now = at(h);
            same = if now == was { same + 1 } else { 0 };
            was = now;
        }
    };
    let started = std::time::Instant::now();
    while !in_well(&h, "Waypoints (2)") {
        assert!(started.elapsed().as_secs() < 10, "never: Waypoints in the well's sight");
        wheel(&mut h);
        rest(&mut h, "Waypoints (2)");
    }
    tree_rows_with(&h, "Waypoints (2)")[0].click();
    // (Both waypoints' rows brought into the well's sight by the wheel.)
    let into_sight = |h: &mut Harness<'_, Moonglow>| {
        let seen = |h: &Harness<'_, Moonglow>| {
            let rows = tree_rows_with(h, "NW_WAYPOINT001");
            let next = h.get_by_label("Module Tree Rows").get_by_label("Conversations (0)");
            rows.len() == 2 && rows.iter().all(|n| n.rect().bottom() <= next.rect().top() + 0.5)
        };
        let started = std::time::Instant::now();
        loop {
            rest(h, "NW_WAYPOINT001");
            if seen(h) {
                break;
            }
            assert!(started.elapsed().as_secs() < 10, "never: both rows in the well's sight");
            wheel(h);
        }
    };
    into_sight(&mut h);
    // A click, then Ctrl+click.
    tree_rows_with(&h, "NW_WAYPOINT001")[0].click();
    until_tree(&mut h, "one selected", |h| h.state().area_views[&area].selection.len() == 1);
    tree_rows_with(&h, "NW_WAYPOINT001")[1].click_modifiers(egui::Modifiers::COMMAND);
    until_tree(&mut h, "both chosen", |h| {
        let mut s = h.state().area_views[&area].selection.clone();
        s.sort();
        s == [w(0), w(1)]
    });
    // The menu of one of them: Delete, on both; Undo.
    let row = tree_rows_with(&h, "NW_WAYPOINT001")[1].rect().center();
    let menu = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            let (button, modifiers) = (egui::PointerButton::Secondary, egui::Modifiers::NONE);
            h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
            // (One step for each input event.)
            h.step();
        }
    };
    menu(&mut h, row);
    h.get_by_label("2 objects chosen");
    h.get_all_by_label("Delete").last().unwrap().click();
    let placed = |h: &Harness<'_, Moonglow>| {
        let git = ResKey::new(area, ResType::GIT);
        let ws = h.state().ws.as_ref().unwrap();
        ws.loaded(&git).and_then(|g| g.root.list("WaypointList").map(|l| l.len()))
    };
    until_tree(&mut h, "both deleted", |h| placed(h) == Some(0));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    until_tree(&mut h, "back", |h| placed(h) == Some(2));
    // One alone: Export as Blueprint… on its menu.
    into_sight(&mut h);
    tree_rows_with(&h, "NW_WAYPOINT001")[0].click();
    until_tree(&mut h, "one selected", |h| h.state().area_views[&area].selection.len() == 1);
    into_sight(&mut h);
    let row = tree_rows_with(&h, "NW_WAYPOINT001")[0].rect().center();
    menu(&mut h, row);
    assert!(h.query_by_label("Export as Blueprint…").is_some(), "on one's menu");
    h.key_press(egui::Key::Escape);
    // (One step for the key's event.)
    h.step();
    // The keys: from the kind's row, Down and Enter.
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    tree_rows_with(&h, "Waypoints (2)")[0].click();
    until_tree(&mut h, "the tree's keys", |h| {
        mg_ui::palette_view::arrow_keys_are(&h.ctx) == "Tree"
    });
    h.hover_at(egui::pos2(600.0, 20.0));
    // (Its kind closed again by the click: open it by Right.)
    if tree_rows_with(&h, "NW_WAYPOINT001").is_empty() {
        h.key_press(egui::Key::ArrowRight);
        // (One step for the key's event.)
        h.step();
    }
    h.key_press(egui::Key::ArrowDown);
    // (One step for the key's event.)
    h.step();
    h.key_press(egui::Key::Enter);
    until_tree(&mut h, "gone to", |h| h.state().area_views[&area].selection.len() == 1);
}
