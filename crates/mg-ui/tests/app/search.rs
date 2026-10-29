//! Find References, Rename, Find and Replace and Advanced Search.

use crate::common::*;

#[test]
fn find_references_then_rename_everywhere() {
    let dir = mg_testkit::scratch_dir("ui-references");
    let path = sample_module(&dir);
    // The sample's area places a creature that spawns with `hello`, and a
    // script runs `hello` by name.
    let mut m = Module::open(&path).unwrap();
    let mut git = Gff::new(*b"GIT ");
    let mut c = mg_schema::git::CREATURE_LIST.new_item();
    c.write(&mg_schema::git::creature_list::SCRIPT_SPAWN, ResRef::from_str("hello").unwrap());
    c.set("Tag", mg_gff::Value::String(b"GREETER".to_vec()));
    git.root.items_mut(&mg_schema::git::CREATURE_LIST).push(c);
    m.set_gff(ResKey::parse("start", ResType::GIT).unwrap(), &git).unwrap();
    let caller = ResKey::parse("caller", ResType::NSS).unwrap();
    m.set(caller, b"void main() { ExecuteScript(\"hello\", OBJECT_SELF); }\n".to_vec());
    m.save().unwrap();

    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::FindReferences(hello));
    h.run();
    h.run();
    h.get_by_label("Used in 1 place:");
    h.get_by_label("Spelled out in scripts (1):");
    // A place opens its area with the object selected.
    h.get_by_label("start › creature GREETER › OnSpawn").click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Area(ResRef::from_str("start").unwrap())).is_some());
    // The area opens in the main window, not over the References window.
    let area = h.state().dock.find_tab(&Tab::Area(ResRef::from_str("start").unwrap())).unwrap();
    assert!(area.surface.is_main());
    h.run();

    // Rename, script strings included.
    h.get_by_label("Rename…").click();
    h.run();
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("hello").find(is_input).expect("the name field").type_text("greet");
    h.run();
    h.get_by_label("Also change the string spelling “hello” in scripts").click();
    h.run();
    h.get_by_label("Rename").click();
    h.run();
    let module = &h.state().ws.as_ref().unwrap().module;
    let greet = ResKey::parse("greet", ResType::NSS).unwrap();
    assert!(module.contains(&greet) && !module.contains(&hello), "{:?}", h.state().log.entries);
    let g = module.gff(&ResKey::parse("start", ResType::GIT).unwrap()).unwrap().unwrap();
    assert_eq!(
        g.root.items(&mg_schema::git::CREATURE_LIST)[0]
            .read(&mg_schema::git::creature_list::SCRIPT_SPAWN),
        ResRef::from_str("greet").unwrap()
    );
    assert!(String::from_utf8_lossy(module.get(&caller).unwrap()).contains("\"greet\""));
    // The References tab follows the new name; one undo takes it all back.
    h.run();
    h.get_by_label("Used in 1 place:");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    let module = &h.state().ws.as_ref().unwrap().module;
    assert!(module.contains(&hello) && !module.contains(&greet));
    assert!(String::from_utf8_lossy(module.get(&caller).unwrap()).contains("\"hello\""));
}

/// The first Find References in a module reads its references on a
/// thread of its own, as the desktop app runs (the window draws, and says
/// it is reading); the next ones read only what changed, at once; the
/// Rename window counts from the same references once they are read.
#[test]
fn the_first_find_references_reads_the_module_on_a_thread_of_its_own() {
    let dir = mg_testkit::scratch_dir("ui-references-thread");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    // (More scripts than a frame reads where they are asked.)
    for i in 0..200 {
        let k = ResKey::parse(&format!("s{i:03}"), ResType::NSS).unwrap();
        m.set(k, format!("void main() {{ ExecuteScript(\"hello\"); }} // {i}\n").into_bytes());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.background_jobs = true;
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::FindReferences(hello));
    // (Frame by frame: the frame that starts the read says so, however
    // soon it is done.)
    let (started, mut said) = (std::time::Instant::now(), false);
    while h.query_by_label("Spelled out in scripts (200):").is_none() {
        assert!(started.elapsed().as_secs() < 30, "read in time");
        h.run_steps(1);
        said |= h.query_by_label("Reading the module's references…").is_some();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(said, "it said it was reading");
    // One script changed: read where it is asked, at once.
    let one = ResKey::parse("s000", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key: one, data: Some(b"void main() {}\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Edit", vec![edit])));
    h.run_steps(2);
    h.get_by_label("Spelled out in scripts (199):");
    assert!(h.query_by_label("Reading the module's references…").is_none());
    // Rename… counts the strings from them.
    h.get_by_label("Rename…").click();
    h.run_steps(2);
    h.get_by_label("Also change the 199 strings spelling “hello” in scripts");
}

/// Find References for a talk-table line: the module's localized strings
/// that name its StrRef, from the talk-table editor's button's action and
/// from a number typed in the References window's field.
#[test]
fn find_references_finds_where_a_talk_table_line_is_named() {
    let dir = mg_testkit::scratch_dir("ui-strref-refs");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let mut item = mg_gff::Gff::new(*b"UTI ");
    let named = mg_core::LocString { strref: mg_core::StrRef(16_777_220), strings: Vec::new() };
    item.root.set("LocalizedName", mg_gff::Value::LocString(named));
    let key = ResKey::parse("sword", ResType::UTI).unwrap();
    let edit = mg_edit::Edit::SetResource { key, data: Some(item.to_bytes().unwrap()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Item", vec![edit])));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::FindStrRef(16_777_220));
    h.run_steps(3);
    h.get_by_label("Talk-table line 16777220");
    let found = h.state().references.usages.clone();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].from, key);
    // Another line: nothing names it.
    h.state_mut().actions.push(mg_ui::Action::FindStrRef(16_777_221));
    h.run_steps(3);
    assert!(h.state().references.usages.is_empty());
}

#[test]
fn find_and_replace_text_across_the_module() {
    use mg_edit::{Command, Edit};
    let dir = mg_testkit::scratch_dir("ui-replace-text");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let keys: Vec<ResKey> =
        ["camp_a", "camp_b"].iter().map(|n| ResKey::parse(n, ResType::UTW).unwrap()).collect();
    let edits = keys
        .iter()
        .map(|k| {
            let mut g = Gff::new(*b"UTW ");
            let name = LocString::from_text(Language::ENGLISH, Gender::Male, "Orc camp");
            g.root.set("LocalizedName", mg_gff::Value::LocString(name));
            Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    h.run();
    assert!(h.state().text_replace.is_some(), "Ctrl+H opens it");
    {
        let d = h.state_mut().text_replace.as_mut().unwrap();
        d.find = "orc".into();
        d.with = "Goblin".into();
        d.options.whole_word = true;
    }
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("2 string(s), 2 time(s)");
    // The second unticked: one replaced.
    h.state_mut().text_replace.as_mut().unwrap().hits[1].1 = false;
    h.run();
    h.get_by_label("Replace in 1").click();
    h.run();
    let name = |h: &mut Harness<'_, Moonglow>, k: &ResKey| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let g = ws.doc(k).unwrap();
        let ls = g.root.locstring("LocalizedName").unwrap();
        ls.text(Language::ENGLISH, Gender::Male).unwrap().into_owned()
    };
    assert_eq!(name(&mut h, &keys[0]), "Goblin camp");
    assert_eq!(name(&mut h, &keys[1]), "Orc camp");
    // Found again: what's left.
    h.get_by_label("1 string(s), 1 time(s)");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(name(&mut h, &keys[0]), "Orc camp");
}

/// Advanced Search with conditions together (fighters with a feat, a
/// class by its levels), then what it found changed in one step (a
/// variable set on the ticked ones, undone as one), opened in Edit
/// Together, and its areas given to Edit Areas Together.
#[test]
fn advanced_search_changes_what_it_found_and_takes_conditions_together() {
    use mg_edit::{Command, Edit};
    use mg_gff::{Struct, Value};
    use mg_ui::search::{Condition, Kind};
    let dir = mg_testkit::scratch_dir("ui-advanced-search-act");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    // Two fighters placed in the area, of levels 5 and 9; the first has
    // a feat.
    let fighter = |tag: &str, level: i16, feat: bool| {
        let mut class = Struct::new(2);
        class.set("Class", Value::Int(4));
        class.set("ClassLevel", Value::Short(level));
        let mut c = Struct::new(4);
        c.set("Tag", Value::String(tag.as_bytes().to_vec()));
        c.set("ScriptSpawn", Value::resref(ResRef::from_str("x2_def_spawn").unwrap()));
        c.set("ClassList", Value::List(vec![class]));
        let mut f = Struct::new(1);
        f.set("Feat", Value::Word(0));
        c.set("FeatList", Value::List(if feat { vec![f] } else { Vec::new() }));
        c
    };
    let mut git = Gff::new(*b"GIT ");
    let creatures = vec![fighter("GUARD", 5, true), fighter("CAPTAIN", 9, false)];
    git.root.set("Creature List", Value::List(creatures));
    let start = ResKey::parse("start", ResType::GIT).unwrap();
    let set = Edit::SetResource { key: start, data: Some(git.to_bytes().unwrap()) };
    app.ws.as_mut().unwrap().apply(Command::new("setup", vec![set])).unwrap();
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    h.run_steps(2);
    let creature = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        let g = ws.module.gff(&start).unwrap().unwrap();
        g.root.list("Creature List").unwrap()[i].clone()
    };
    let var = |c: &Struct| c.list("VarTable").and_then(|v| v.first().map(|v| v.integer("Value")));

    // Fighters, then fighters with the feat.
    h.state_mut().advanced_search.first =
        Condition { kind: Kind::Class, row: Some(4), ..Default::default() };
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("2 found:");
    h.get_by_label("And…").click();
    h.run();
    h.state_mut().advanced_search.and[0].row = Some(0);
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("1 found:");
    h.get_by_label("Class: row 4, level 5 · Feat: row 0");
    h.get_by_label("Remove").click();
    h.run();
    // A class by its levels.
    h.state_mut().advanced_search.first.levels = (6, 20);
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("1 found:");
    h.get_by_label("Class: row 4, level 9");
    h.state_mut().advanced_search.first.levels = (1, 60);
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("2 found:");

    // A variable set on the ticked one, in one step.
    h.state_mut().advanced_search.ticked[1] = false;
    h.run();
    h.get_by_label("Change the 1 ticked").click();
    h.run();
    type_into_hint(&mut h, "its name, e.g. nQuestState", "nTest");
    type_into_hint(&mut h, "the value", "5");
    h.get_by_label("Change 1 Found").click();
    h.run();
    assert_eq!(var(&creature(&mut h, 0)), Some(Some(5)));
    assert_eq!(var(&creature(&mut h, 1)), None, "not ticked");
    // (Found again, everything ticked.)
    h.get_by_label("2 found:");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert_eq!(var(&creature(&mut h, 0)), None, "undone as one");
    // An event's script, on both.
    {
        let a = &mut h.state_mut().advanced_search.act;
        a.act = mg_ui::search::Act::SetScript;
        a.value = String::new();
    }
    h.run();
    h.state_mut().advanced_search.act.event = Some("ScriptSpawn".into());
    h.run();
    type_into_hint(&mut h, "its name (empty: none)", "my_spawn");
    h.get_by_label("Change 2 Found").click();
    h.run();
    for i in 0..2 {
        assert_eq!(
            creature(&mut h, i).resref("ScriptSpawn"),
            Some(ResRef::from_str("my_spawn").unwrap())
        );
    }

    // Edit Together, and their area chosen in Edit Areas Together.
    h.get_by_label("Edit the 2 ticked together").click();
    h.run();
    h.get_by_label("Edit 2 creature(s) in start Together").click();
    h.run_steps(2);
    let paths = vec![
        mg_edit::GffPath::root().item("Creature List", 0),
        mg_edit::GffPath::root().item("Creature List", 1),
    ];
    let area = ResRef::from_str("start").unwrap();
    let both = Tab::Instances { area, paths };
    assert!(h.state().dock.find_tab(&both).is_some());
    // (Closed: in this small screen it lies over the search's.)
    h.state_mut().actions.push(mg_ui::Action::CloseTab(both));
    h.run_steps(2);
    h.get_by_label("Choose 1 Area(s)…").click();
    h.run_steps(2);
    // (The chooser's button: one area ticked.)
    h.get_by_label("Edit 1 Together");
}

#[test]
fn advanced_search_finds_objects_by_what_they_hold() {
    use mg_edit::{Command, Edit};
    use mg_gff::{Struct, Value};
    let dir = mg_testkit::scratch_dir("ui-advanced-search");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    // An amulet with a variable, and a fighter placed in the area who
    // carries one of his own.
    let var = |name: &str| {
        let mut v = Struct::new(0);
        v.set("Name", Value::String(name.as_bytes().to_vec()));
        v.set("Type", Value::Dword(1));
        v.set("Value", Value::Int(3));
        v
    };
    let amulet = ResKey::parse("amulet", ResType::UTI).unwrap();
    let mut uti = Gff::new(*b"UTI ");
    uti.root.set("Tag", Value::String(b"AMULET".to_vec()));
    uti.root.set("VarTable", Value::List(vec![var("sDeidad")]));
    let mut own = Struct::new(0);
    own.set("Tag", Value::String(b"OLD_AMULET".to_vec()));
    own.set("VarTable", Value::List(vec![var("sDeidadOld")]));
    let mut class = Struct::new(2);
    class.set("Class", Value::Int(4));
    class.set("ClassLevel", Value::Short(5));
    let mut guard = Struct::new(4);
    guard.set("Tag", Value::String(b"GUARD".to_vec()));
    guard.set("ClassList", Value::List(vec![class]));
    guard.set("ItemList", Value::List(vec![own]));
    let mut git = Gff::new(*b"GIT ");
    git.root.set("Creature List", Value::List(vec![guard]));
    let set = |key: ResKey, g: &Gff| Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) };
    let start = ResKey::parse("start", ResType::GIT).unwrap();
    let edits = vec![set(amulet, &uti), set(start, &git)];
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    // The game's classes, to name them.
    let mut mem = mg_resman::MemContainer::new();
    mem.insert(
        ResKey::parse("classes", ResType::TWODA).unwrap(),
        &b"2DA V2.0\n\n Label Name\n0 Barbarian ****\n1 Bard ****\n2 Cleric ****\n3 Druid ****\n4 Fighter 1\n"[..],
    );
    let mut rm = mg_resman::ResMan::new();
    rm.add(mg_resman::priority::KEY, "mem", mg_resman::LayerClass::Key, mem);
    let mut tlk = mg_tlk::Tlk::new(Language::ENGLISH);
    tlk.entries.push(mg_tlk::TlkEntry::text("Bad Strref"));
    tlk.entries.push(mg_tlk::TlkEntry::text("Fighter"));
    app.game = Some(std::sync::Arc::new(mg_rules::GameData::new(rm, tlk)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Advanced Search…").click();
    h.run_steps(2);
    assert!(
        h.state().dock.find_tab(&Tab::AdvancedSearch).is_some(),
        "Edit › Advanced Search… opens its tab"
    );
    // Variables by a wildcard name: the amulet, and the one the guard
    // carries.
    type_into_hint(&mut h, "e.g. *sDeidad* (* for any characters)", "*sdeidad*");
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("2 found:");
    h.get_by_label("start › creature GUARD › item OLD_AMULET");
    h.get_by_label("sDeidad (int) = 3");
    // A click opens the object.
    h.get_by_label("amulet.uti").click();
    h.run_steps(3);
    assert!(h.state().dock.find_tab(&Tab::Blueprint(amulet)).is_some());
    // The search's tab, brought back, has kept what it found (the
    // amulet's window closed: in this small screen it lies over the
    // search's).
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Blueprint(amulet)));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    h.run_steps(2);
    h.get_by_label("2 found:");
    // Creatures by class, named as the game names them.
    {
        let d = &mut h.state_mut().advanced_search;
        d.first.kind = mg_ui::search::Kind::Class;
        d.first.row = Some(4);
    }
    h.run();
    h.get_by_label("Find again for what's asked now.");
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("1 found:");
    h.get_by_label("Class: Fighter, level 5");
    // Nothing chosen: nothing to find yet.
    h.state_mut().advanced_search.first.row = None;
    h.run();
    h.get_by_label("Choose a class");
}

/// The references list names an area as the rest of the window does:
/// by its name where areas are shown by name.
#[test]
fn references_name_areas_as_the_window_does() {
    let Some((mut h, area)) = area_harness("refs-area-names") else { return };
    let waypoint = ResKey::parse("nw_waypoint001", ResType::UTW).unwrap();
    h.state_mut().settings.area_names = false;
    h.state_mut().actions.push(mg_ui::Action::FindReferences(waypoint));
    h.run_steps(4);
    let by_resref = h.query_all_by_label_contains(&format!("{area} ›")).count();
    assert!(by_resref > 0, "its waypoints, by the area's resref");
    h.state_mut().settings.area_names = true;
    h.run_steps(3);
    assert_eq!(h.query_all_by_label_contains(&format!("{area} ›")).count(), 0);
    assert_eq!(h.query_all_by_label_contains("Field ›").count(), by_resref);
}
