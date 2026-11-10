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

/// A rename that leaves the scripts' strings says which still spell the
/// old name, from the references the window keeps (`UsageIndex`, read
/// before the rename and again for the scripts it changed): a script
/// added after the index was read is among them.
#[test]
fn a_rename_says_which_strings_still_spell_the_old_name() {
    let dir = mg_testkit::scratch_dir("ui-references-still-spelled");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    app.run(mg_ui::Action::FindReferences(hello));
    let caller = ResKey::parse("caller", ResType::NSS).unwrap();
    let src = b"void main() { ExecuteScript(\"hello\", OBJECT_SELF); }\n".to_vec();
    let edit = mg_edit::Edit::SetResource { key: caller, data: Some(src) };
    app.run(mg_ui::Action::Apply(mg_edit::Command::new("Add", vec![edit])));
    app.rename_resource(hello, ResRef::from_str("greet").unwrap(), false);
    let said = |s: &str| app.log.entries.iter().any(|e| e.1.contains(s));
    assert!(said("A string still spelled “hello” in scripts (caller.nss line 1)"), "{:?}", app.log);
}

/// A rename takes the compiled code of the scripts it rewrote and
/// compiles them again. Whether a failure was one of theirs was told by
/// the script's name being somewhere in the error's message: a script
/// whose compile failed in another include (the message names that file)
/// lost its compiled code without a word, and an unrelated script whose
/// name holds a rewritten one's was said to be the rename's (the review
/// of November 2026, D9). The failures are told by the script compiled.
#[test]
fn a_rename_says_which_of_its_scripts_no_longer_compile() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-rename-recompile");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let script = |name: &str| ResKey::parse(name, ResType::NSS).unwrap();
    m.set(script("inc_old"), b"void Old() {}\n".to_vec());
    m.set(script("inc_utils"), b"void Utils() { int x = ; }\n".to_vec());
    let main = b"#include \"inc_old\"\n#include \"inc_utils\"\nvoid main() { Old(); }\n";
    m.set(script("quest"), main.to_vec());
    // (Compiled once, before inc_utils was broken: the rename takes it.)
    m.set(ResKey::parse("quest", ResType::NCS).unwrap(), b"NCS V1.0B".to_vec());
    // Another script that never compiled, its name holding the first's.
    m.set(script("quest_other"), b"void main() { nothing here }\n".to_vec());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.rename_resource(script("inc_old"), ResRef::from_str("inc_new").unwrap(), false);
    let module = &app.ws.as_ref().unwrap().module;
    assert!(module.contains(&script("inc_new")), "{:?}", app.log.entries);
    assert!(!module.contains(&ResKey::parse("quest", ResType::NCS).unwrap()));
    let errors: Vec<&str> = (app.log.entries.iter())
        .filter(|(level, _)| matches!(level, mg_ui::Level::Error))
        .map(|(_, m)| m.as_str())
        .collect();
    assert!(
        errors.iter().any(|m| m.contains("quest.nss") && m.contains("inc_utils")),
        "{errors:?}"
    );
    assert!(!errors.iter().any(|m| m.contains("quest_other")), "not the rename's: {errors:?}");
    let said = |s: &str| app.log.entries.iter().any(|e| e.1.contains(s));
    assert!(said("Some scripts the rename changed aren't compiled"));
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

/// A talk-table line a script names by number (`GetStringByStrRef`), a
/// palette's category and a hak's 2DA: listed, the script's at its line.
#[test]
fn find_references_finds_a_line_in_scripts_and_the_haks_2das() {
    use mg_schema::{StructExt, ifo};
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-strref-haks");
    let user = user_folder(&dir, &["hak"]);
    let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
    let classes = b"2DA V2.0\n\n   Label Name\n0  Kz 16777300\n1  Other 5\n".to_vec();
    hak.add(ResRef::from_str("classes").unwrap(), ResType::TWODA, classes).unwrap();
    std::fs::write(user.join("hak/kz_classes.hak"), hak.to_bytes().unwrap()).unwrap();
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    let mut item = ifo::MOD_HAK_LIST.new_item();
    item.write(&ifo::mod_hak_list::MOD_HAK, mg_schema::ExoString::from("kz_classes"));
    info.root.items_mut(&ifo::MOD_HAK_LIST).push(item);
    m.set_info(&info).unwrap();
    let script = b"void main()\n{\n    SpeakStringByStrRef(16777300);\n}\n".to_vec();
    m.set(ResKey::parse("kz_speak", ResType::NSS).unwrap(), script);
    m.save().unwrap();
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::FindStrRef(16_777_300));
    h.run_steps(3);
    let places: Vec<String> = h.state().references.usages.iter().map(|u| u.place.clone()).collect();
    assert_eq!(places, ["kz_speak.nss › line 3"]);
    h.get_by_label("In the haks' 2DAs (1):");
    h.get_by_label("kz_classes › classes.2da › row 0, Name");
    // The script's opens at its line.
    h.get_by_label("kz_speak.nss › line 3").click();
    h.run_steps(3);
    let key = ResKey::parse("kz_speak", ResType::NSS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Script(key)).is_some(), "the script opens");
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
    let mut h = window_harness(app);
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

/// Find and Replace in the tags, by a regular expression: ticked beside
/// the players' text, its groups in what replaces it; a tag it makes
/// another object's is said as a paste's is (the Duplicate Tags window,
/// with the option on); an expression that isn't one is said, and not
/// looked for.
#[test]
fn find_and_replace_in_tags_by_a_regular_expression() {
    use mg_edit::{Command, Edit};
    use mg_module::text::TextKind;
    let dir = mg_testkit::scratch_dir("ui-replace-tags");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.settings.warn_duplicate_tags = true;
    let key = ResKey::parse("start", ResType::GIT).unwrap();
    let mut g = Gff::new(*b"GIT ");
    let waypoint = |tag: &str| {
        let mut w = mg_gff::Struct::new(5);
        w.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
        w
    };
    let list = vec![waypoint("WP_GATE_1"), waypoint("WP_WELL_1"), waypoint("WP_INN")];
    g.root.set("WaypointList", mg_gff::Value::List(list));
    let edit = Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) };
    app.ws.as_mut().unwrap().apply(Command::new("setup", vec![edit])).unwrap();
    let mut h = window_harness(app);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    h.run();
    {
        let d = h.state_mut().text_replace.as_mut().unwrap();
        assert_eq!(d.kinds, TextKind::PLAYERS, "the players' text, unless ticked");
        d.find = "^wp_(gate|well)_1$".into();
        d.with = "WP_${1}_EXIT".into();
    }
    h.run();
    h.get_by_label("Regular expression").click();
    h.get_by_label("Tags").click();
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("2 string(s), 2 time(s)");
    h.get_by_label("Replace in 2").click();
    h.run();
    let tags = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let g = ws.doc(&key).unwrap();
        let list = g.root.list("WaypointList").unwrap();
        list.iter().map(|w| String::from_utf8_lossy(w.string("Tag").unwrap()).into()).collect()
    };
    assert_eq!(tags(&mut h), ["WP_GATE_EXIT", "WP_WELL_EXIT", "WP_INN"], "case kept, as typed");
    assert!(h.state().tag_clash.is_none(), "each its own");
    // One made another's: said.
    {
        let d = h.state_mut().text_replace.as_mut().unwrap();
        d.find = "_(gate|well)_EXIT$".into();
        d.with = "_INN".into();
    }
    h.run();
    // (By its action: the window, centered, moves as its list shrinks.)
    h.get_by_label("Find").click_accesskit();
    h.run();
    h.get_by_label("Replace in 2").click_accesskit();
    h.run();
    let clash = h.state().tag_clash.clone().expect("the Duplicate Tags window");
    let said: Vec<String> = clash.clashes.iter().map(|c| c.sentence()).collect();
    assert_eq!(said, ["WP_INN is now on three objects"]);
    assert!(clash.what.starts_with("“Replace"), "{}", clash.what);
    h.get_by_label("Keep Tags").click();
    h.run();
    // Not a regular expression: said, and Find can't be clicked.
    h.state_mut().text_replace.as_mut().unwrap().find = "wp_(".into();
    h.run();
    assert!(h.query_by_label_contains("Not a regular expression").is_some());
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
    let mut h = window_harness(app);
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
    // (The module changed under what was found: found again first.)
    h.get_by_label("Find").click();
    h.run();
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

/// What Advanced Search found is a list of places (`start.git /Creature
/// List[1]`). With an object before one deleted since, Change wrote to
/// whatever stood at the place by then, another creature (the review of
/// November 2026, D2): in a module changed since they were found, the
/// hits are found again and nothing is changed, opened or edited together
/// until the builder has seen the new list.
#[test]
fn advanced_search_finds_again_before_it_acts_in_a_changed_module() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_gff::{Struct, Value};
    use mg_ui::search::{Act, Condition, Kind};
    let dir = mg_testkit::scratch_dir("ui-advanced-search-stale");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let creature = |tag: &str, quest: bool| {
        let mut c = Struct::new(4);
        c.set("Tag", Value::String(tag.as_bytes().to_vec()));
        let mut v = Struct::new(0);
        v.set("Name", Value::String(b"nQuest".to_vec()));
        v.set("Type", Value::Dword(1));
        v.set("Value", Value::Int(1));
        c.set("VarTable", Value::List(if quest { vec![v] } else { Vec::new() }));
        c
    };
    let mut git = Gff::new(*b"GIT ");
    let creatures =
        vec![creature("FIRST", false), creature("ASKS", true), creature("PLAIN", false)];
    git.root.set("Creature List", Value::List(creatures));
    let start = ResKey::parse("start", ResType::GIT).unwrap();
    let set = Edit::SetResource { key: start, data: Some(git.to_bytes().unwrap()) };
    app.ws.as_mut().unwrap().apply(Command::new("setup", vec![set])).unwrap();
    let tags = |app: &mut Moonglow| -> Vec<(String, bool)> {
        let ws = app.ws.as_mut().unwrap();
        ws.flush().unwrap();
        let g = ws.module.gff(&start).unwrap().unwrap();
        (g.root.list("Creature List").unwrap().iter())
            .map(|c| {
                let tag = String::from_utf8_lossy(c.string("Tag").unwrap()).into_owned();
                let marked = c.list("VarTable").is_some_and(|v| {
                    v.iter().any(|v| v.string("Name") == Some(b"nMarked".as_slice()))
                });
                (tag, marked)
            })
            .collect()
    };
    let mut d = mg_ui::search::AdvancedSearch {
        first: Condition { kind: Kind::Variable, name: "nQuest".into(), ..Default::default() },
        ..Default::default()
    };
    app.advanced_search(&mut d);
    assert_eq!(d.hits.len(), 1, "{:?}", d.hits);
    let object = |h: &mg_module::search::Hit| GffPath(h.path.0.iter().take(1).cloned().collect());
    assert_eq!(object(&d.hits[0]), GffPath::root().item("Creature List", 1));
    d.act.act = Act::SetVariable;
    d.act.name = "nMarked".into();
    d.act.value = "1".into();
    // The first creature goes: at the place of the one found is PLAIN, which
    // was never found.
    let remove = Edit::remove_item(start, GffPath::root(), "Creature List", 0);
    app.ws.as_mut().unwrap().apply(Command::new("Delete", vec![remove])).unwrap();
    app.change_found(&mut d);
    assert_eq!(tags(&mut app), [("ASKS".to_string(), false), ("PLAIN".to_string(), false)]);
    assert!(d.note.as_deref().is_some_and(|n| n.contains("Found again")), "{:?}", d.note);
    assert_eq!(object(&d.hits[0]), GffPath::root().item("Creature List", 0), "found where it is");
    // Asked again, with the list as it is now: the creature that was found.
    app.change_found(&mut d);
    assert_eq!(tags(&mut app), [("ASKS".to_string(), true), ("PLAIN".to_string(), false)]);
    assert!(d.note.is_none());

    // In the window: a hit clicked after an Undo is found again, not
    // opened where it was.
    app.advanced_search = d;
    let mut h = window_harness(app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    let was = h.state().dock.iter_all_tabs().count();
    h.get_by_label_contains("creature ASKS").click();
    h.run_steps(2);
    assert_eq!(h.state().dock.iter_all_tabs().count(), was, "nothing opened");
    h.get_by_label_contains("The module changed since this was found");
    assert_eq!(
        object(&h.state().advanced_search.hits[0]),
        GffPath::root().item("Creature List", 1),
        "found again, where Undo put it back"
    );
}

/// Advanced Search's list lays out the rows in sight, the others taking
/// a row's room each (every hit was laid out every frame: 8,785 rows of
/// widgets for a world's variables; the review of November 2026, D23):
/// the first rows are there, the last is not until the list is scrolled
/// to it, and a row scrolled to is ticked and clicked as any other.
#[test]
fn advanced_search_lays_out_the_rows_in_sight() {
    use mg_edit::{Command, Edit};
    use mg_gff::{Struct, Value};
    use mg_ui::search::{Condition, Kind};
    let dir = mg_testkit::scratch_dir("ui-advanced-search-rows");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let creatures: Vec<Struct> = (0..400)
        .map(|i| {
            let mut c = Struct::new(4);
            c.set("Tag", Value::String(format!("T{i:03}").into_bytes()));
            let mut v = Struct::new(0);
            v.set("Name", Value::String(b"nQuest".to_vec()));
            v.set("Type", Value::Dword(1));
            v.set("Value", Value::Int(1));
            c.set("VarTable", Value::List(vec![v]));
            c
        })
        .collect();
    let mut git = Gff::new(*b"GIT ");
    git.root.set("Creature List", Value::List(creatures));
    let start = ResKey::parse("start", ResType::GIT).unwrap();
    let set = Edit::SetResource { key: start, data: Some(git.to_bytes().unwrap()) };
    app.ws.as_mut().unwrap().apply(Command::new("setup", vec![set])).unwrap();
    let mut h = window_harness(app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    h.run_steps(2);
    h.state_mut().advanced_search.first =
        Condition { kind: Kind::Variable, name: "nQuest".into(), ..Default::default() };
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("400 found:");
    h.get_by_label_contains("creature T000");
    assert!(h.query_by_label_contains("creature T399").is_none(), "out of sight: not laid out");
    let laid_out = h.query_all_by_label_contains("creature T").count();
    assert!((5..200).contains(&laid_out), "{laid_out} rows laid out");
    // Scrolled to its end: the last rows, where the first were.
    let over = h.get_by_label_contains("creature T003").rect().center();
    h.hover_at(over);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -1_000_000.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    let deadline = std::time::Instant::now() + SETTLE_TIME;
    while h.query_by_label_contains("creature T399").is_none() {
        assert!(std::time::Instant::now() < deadline, "the list scrolls to its end");
        h.step();
    }
    assert!(h.query_by_label_contains("creature T000").is_none());
    h.get_by_label_contains("creature T399").click();
    let area = ResRef::from_str("start").unwrap();
    while h.state().dock.find_tab(&Tab::Area(area)).is_none() {
        assert!(std::time::Instant::now() < deadline, "its place opens");
        h.step();
    }
}

/// Every search wrote the field looked for over the change's name, which
/// is a variable's too: after a search of appearances, a variable typed
/// as nBoss read Appearance_Type once the change had run (it ends with a
/// search), and the next change set a variable of that name on everything
/// found (the review of November 2026, D10). The field is offered where
/// a field is what is set, and when another thing is looked for.
#[test]
fn advanced_search_keeps_the_name_typed_for_a_change() {
    use mg_edit::{Command, Edit};
    use mg_gff::{Struct, Value};
    use mg_ui::search::{Act, AdvancedSearch, Condition, Kind};
    let dir = mg_testkit::scratch_dir("ui-advanced-search-name");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut c = Struct::new(4);
    c.set("Tag", Value::String(b"OGRE".to_vec()));
    c.set("Appearance_Type", Value::Word(6));
    let mut git = Gff::new(*b"GIT ");
    git.root.set("Creature List", Value::List(vec![c]));
    let start = ResKey::parse("start", ResType::GIT).unwrap();
    let set = Edit::SetResource { key: start, data: Some(git.to_bytes().unwrap()) };
    app.ws.as_mut().unwrap().apply(Command::new("setup", vec![set])).unwrap();
    let mut d = AdvancedSearch {
        first: Condition { kind: Kind::Appearance, row: Some(6), ..Default::default() },
        ..Default::default()
    };
    d.act.act = Act::SetVariable;
    d.act.name = "nBoss".into();
    d.act.value = "1".into();
    app.advanced_search(&mut d);
    assert_eq!(d.hits.len(), 1, "{:?}", d.hits);
    assert_eq!(d.act.name, "nBoss", "after the search");
    app.change_found(&mut d);
    assert_eq!(d.act.name, "nBoss", "after the change, which searches again");
    let ws = app.ws.as_mut().unwrap();
    ws.flush().unwrap();
    let g = ws.module.gff(&start).unwrap().unwrap();
    let vars = g.root.list("Creature List").unwrap()[0].list("VarTable").unwrap().to_vec();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].string("Name"), Some(b"nBoss".as_slice()));
    // A field to set is the one looked for, once another thing is asked.
    d.act.act = Act::SetField;
    d.first.row = Some(7);
    app.advanced_search(&mut d);
    assert_eq!(d.act.name, "Appearance_Type");
    // (And what is typed over it stays through the same question.)
    d.act.name = "Tag".into();
    app.advanced_search(&mut d);
    assert_eq!(d.act.name, "Tag");
}

/// Advanced Search's changes to lists, through Edit Together's rules
/// (`docs/lists-together-proposal.md`): a feat added to the fighters
/// found (where they lack it), taken from them, a class added at a level,
/// and their only class kept, said; each one step.
#[test]
fn advanced_search_adds_and_removes_list_entries_on_what_it_found() {
    use mg_edit::{Command, Edit};
    use mg_gff::{Struct, Value};
    use mg_ui::search::{Act, Condition, Kind};
    let dir = mg_testkit::scratch_dir("ui-advanced-search-lists");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let fighter = |tag: &str, feat: bool| {
        let mut class = Struct::new(2);
        class.set("Class", Value::Int(4));
        class.set("ClassLevel", Value::Short(5));
        let mut c = Struct::new(4);
        c.set("Tag", Value::String(tag.as_bytes().to_vec()));
        c.set("ClassList", Value::List(vec![class]));
        let mut f = Struct::new(1);
        f.set("Feat", Value::Word(0));
        c.set("FeatList", Value::List(if feat { vec![f] } else { Vec::new() }));
        c
    };
    let mut git = Gff::new(*b"GIT ");
    let creatures = vec![fighter("GUARD", true), fighter("CAPTAIN", false)];
    git.root.set("Creature List", Value::List(creatures));
    let start = ResKey::parse("start", ResType::GIT).unwrap();
    let set = Edit::SetResource { key: start, data: Some(git.to_bytes().unwrap()) };
    app.ws.as_mut().unwrap().apply(Command::new("setup", vec![set])).unwrap();
    let mut h = window_harness(app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    h.run_steps(2);
    let list = |h: &mut Harness<'_, Moonglow>, label: &str, field: &str| -> Vec<Vec<i64>> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        let g = ws.module.gff(&start).unwrap().unwrap();
        let all = g.root.list("Creature List").unwrap().to_vec();
        all.iter()
            .map(|c| c.list(label).unwrap_or(&[]).iter().filter_map(|e| e.integer(field)).collect())
            .collect()
    };
    h.state_mut().advanced_search.first =
        Condition { kind: Kind::Class, row: Some(4), ..Default::default() };
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("2 found:");
    h.get_by_label("Change the 2 ticked").click();
    h.run();
    let act = |h: &mut Harness<'_, Moonglow>, act: Act, row: usize| {
        // (The kind of row chosen first: a new kind starts with none.)
        h.state_mut().advanced_search.act.act = act;
        h.run();
        h.state_mut().advanced_search.act.entry.row = Some(row);
        h.run();
        h.get_by_label("Change 2 Found").click();
        h.run_steps(3);
    };
    // Feat 0: added to the captain, who lacks it; the guard has it.
    act(&mut h, Act::AddFeat, 0);
    assert_eq!(list(&mut h, "FeatList", "Feat"), [vec![0], vec![0]]);
    assert!(h.query_by_label_contains("has it already").is_some());
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    // (Found again in the module as Undo left it.)
    h.get_by_label("Find").click();
    h.run();
    assert_eq!(list(&mut h, "FeatList", "Feat"), [vec![0], vec![]], "one step");
    act(&mut h, Act::RemoveFeat, 0);
    assert_eq!(list(&mut h, "FeatList", "Feat"), [Vec::<i64>::new(), vec![]]);
    // A rogue at level 3 for both; the fighter class, their only one, kept.
    h.state_mut().advanced_search.act.entry.levels.0 = 3;
    act(&mut h, Act::AddClass, 8);
    assert_eq!(list(&mut h, "ClassList", "Class"), [vec![4, 8], vec![4, 8]]);
    assert_eq!(list(&mut h, "ClassList", "ClassLevel"), [vec![5, 3], vec![5, 3]]);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    // (Found again in the module as Undo left it.)
    h.get_by_label("Find").click();
    h.run();
    act(&mut h, Act::RemoveClass, 4);
    assert_eq!(list(&mut h, "ClassList", "Class"), [vec![4], vec![4]]);
    assert!(h.query_by_label_contains("its only class").is_some());
}

/// Advanced Search with a condition ticked Not (fighters without a feat),
/// then any of its conditions (fighters, or what has the feat): the row
/// says Not, Or, And as it is, and the window asks all or any.
#[test]
fn advanced_search_leaves_out_a_condition_and_finds_any_of_them() {
    use mg_edit::{Command, Edit};
    use mg_gff::{Struct, Value};
    use mg_ui::search::{Condition, Kind};
    let dir = mg_testkit::scratch_dir("ui-advanced-search-any-not");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    // A fighter with a feat, one without, and a wizard with it.
    let creature = |tag: &str, class: i32, feat: bool| {
        let mut c = Struct::new(4);
        c.set("Tag", Value::String(tag.as_bytes().to_vec()));
        let mut cl = Struct::new(2);
        cl.set("Class", Value::Int(class));
        cl.set("ClassLevel", Value::Short(5));
        c.set("ClassList", Value::List(vec![cl]));
        let mut f = Struct::new(1);
        f.set("Feat", Value::Word(0));
        c.set("FeatList", Value::List(if feat { vec![f] } else { Vec::new() }));
        c
    };
    let mut git = Gff::new(*b"GIT ");
    let creatures =
        vec![creature("GUARD", 4, true), creature("CAPTAIN", 4, false), creature("MAGE", 10, true)];
    git.root.set("Creature List", Value::List(creatures));
    let start = ResKey::parse("start", ResType::GIT).unwrap();
    let set = Edit::SetResource { key: start, data: Some(git.to_bytes().unwrap()) };
    app.ws.as_mut().unwrap().apply(Command::new("setup", vec![set])).unwrap();
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    h.run_steps(2);
    h.state_mut().advanced_search.first =
        Condition { kind: Kind::Class, row: Some(4), ..Default::default() };
    h.run();
    // (One condition: nothing to choose between all and any.)
    assert!(h.query_by_value("meeting all of the conditions").is_none());
    h.get_by_label("And…").click();
    h.run();
    h.state_mut().advanced_search.and[0].row = Some(0);
    h.run();
    h.get_by_value("meeting all of the conditions");
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("1 found:");
    h.get_by_label("Class: row 4, level 5 · Feat: row 0");

    // The feat ticked Not: the fighter without it.
    h.get_by_label("Not").click();
    h.run();
    assert!(h.state().advanced_search.and[0].not);
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("1 found:");
    h.get_by_label("start › creature CAPTAIN");
    h.get_by_label("Class: row 4, level 5");

    // Any of them: the fighters, and what has the feat (the guard twice:
    // his class, his feat).
    h.state_mut().advanced_search.and[0].not = false;
    h.run();
    h.get_by_value("meeting all of the conditions").click();
    h.run();
    h.get_all_by_label("meeting any of the conditions").last().unwrap().click();
    h.run();
    assert!(h.state().advanced_search.any);
    h.get_by_value("meeting any of the conditions");
    h.get_by_label("Or…");
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("4 found:");
    h.get_by_label("start › creature MAGE");
    // Any of them, less one ticked Not: fighters or mages, without the
    // feat.
    let not_feat = Condition { kind: Kind::Feat, row: Some(0), not: true, ..Default::default() };
    {
        let d = &mut h.state_mut().advanced_search;
        d.and =
            vec![Condition { kind: Kind::Class, row: Some(10), ..Default::default() }, not_feat];
    }
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("1 found:");
    h.get_by_label("start › creature CAPTAIN");
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
    let mut h = window_harness(app);
    settle(&mut h);
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

/// A spell by the class that knows it (As class), and the game's own
/// blueprints searched with the module's: listed after them, said to be
/// read-only, never ticked (so neither changed nor edited together), and
/// opened as their copy to look at.
#[test]
fn advanced_search_finds_a_spell_by_its_class_and_the_game_s_blueprints() {
    use mg_edit::{Command, Edit};
    use mg_gff::{Struct, Value};
    use mg_ui::search::{Condition, Kind};
    let dir = mg_testkit::scratch_dir("ui-advanced-search-spell-class");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    // A fighter (4) who knows spell 107, as a blueprint of the module's and
    // of the game's.
    let fighter = |tag: &str| {
        let mut known = Struct::new(3);
        known.set("Spell", Value::Word(107));
        let mut class = Struct::new(2);
        class.set("Class", Value::Int(4));
        class.set("ClassLevel", Value::Short(5));
        class.set("KnownList1", Value::List(vec![known]));
        let mut utc = Gff::new(*b"UTC ");
        utc.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
        utc.root.set("ClassList", Value::List(vec![class]));
        utc
    };
    let guard = ResKey::parse("guard", ResType::UTC).unwrap();
    let set = Edit::SetResource { key: guard, data: Some(fighter("GUARD").to_bytes().unwrap()) };
    app.ws.as_mut().unwrap().apply(Command::new("setup", vec![set])).unwrap();
    let mut mem = mg_resman::MemContainer::new();
    mem.insert(
        ResKey::parse("classes", ResType::TWODA).unwrap(),
        &b"2DA V2.0\n\n Label Name\n0 Barbarian ****\n1 Bard ****\n2 Cleric ****\n3 Druid ****\n4 Fighter 1\n"[..],
    );
    let bandit = ResKey::parse("nw_bandit", ResType::UTC).unwrap();
    mem.insert(bandit, &fighter("BANDIT").to_bytes().unwrap()[..]);
    let mut rm = mg_resman::ResMan::new();
    rm.add(mg_resman::priority::KEY, "base", mg_resman::LayerClass::Key, mem);
    let mut tlk = mg_tlk::Tlk::new(Language::ENGLISH);
    tlk.entries.push(mg_tlk::TlkEntry::text("Bad Strref"));
    tlk.entries.push(mg_tlk::TlkEntry::text("Fighter"));
    app.game = Some(std::sync::Arc::new(mg_rules::GameData::new(rm, tlk)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    h.run_steps(2);
    h.state_mut().advanced_search.first =
        Condition { kind: Kind::Spell, row: Some(107), ..Default::default() };
    h.run();
    // The class it is known as, chosen from the game's classes.
    h.get_by_label("As class");
    h.state_mut().advanced_search.first.class = Some(4);
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("1 found:");
    h.get_by_label("Known spell: row 107 (Fighter)");
    h.state_mut().advanced_search.first.class = Some(2);
    h.run();
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("Nothing in the module holds it.");

    // The game's blueprints too: the bandit, after the guard, read-only.
    h.state_mut().advanced_search.first.class = Some(4);
    h.get_by_label("The haks' and the game's blueprints too").click();
    h.run();
    assert!(h.state().advanced_search.haks_and_game);
    h.get_by_label("Find").click();
    h.run();
    h.get_by_label("2 found:");
    h.get_by_label("(1 of them the haks' or the game's, read-only)");
    h.get_by_label("(base)");
    let d = &h.state().advanced_search;
    assert_eq!(d.hits[1].layer.as_deref(), Some("base"));
    assert_eq!(d.ticked, [true, false], "the game's is not ticked");
    assert_eq!(d.chosen().len(), 1);
    h.get_by_label("Change the 1 ticked");
    // Tick All leaves it out; a tick set by hand is not taken.
    h.state_mut().advanced_search.ticked = vec![true, true];
    h.run();
    assert_eq!(h.state().advanced_search.chosen().len(), 1);
    // A click opens its copy, to look at.
    h.get_by_label("nw_bandit.utc").click();
    h.run_steps(3);
    let copy = Tab::ResourceCopy { key: bandit, layer: "base".into() };
    assert!(h.state().dock.find_tab(&copy).is_some());
}

/// Advanced Search's Value list of an item property keeps the cost
/// table's order, as the item editor's Cost Parameter does (+2 before
/// +10), where it went by name past 12 rows.
#[test]
fn advanced_search_lists_a_property_s_values_in_the_cost_table_s_order() {
    let Some(mut h) = game_harness("advanced-search-values") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    settle(&mut h);
    {
        let d = &mut h.state_mut().advanced_search;
        d.first.kind = mg_ui::search::Kind::ItemProperty;
        // (itempropdef.2da's Enhancement Bonus: iprp_bonuscost's +1 to +20.)
        d.first.row = Some(6);
    }
    settle(&mut h);
    h.get_by_label("Value");
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let any = |n: &egui_kittest::Node<'_>| n.accesskit_node().value() == Some("Any".into());
    let lists: Vec<_> = h.query_all(combo).filter(any).map(|n| n.rect().center()).collect();
    // (Subtype has none: the Value list is the first "Any".)
    click_pos(&h, lists[0]);
    settle(&mut h);
    let at = |label: &str| h.query_by_label(label).map(|n| n.rect().top());
    let (two, ten) = (at("+2"), at("+10"));
    assert!(two.is_some(), "the list open, with +2");
    assert!(ten.is_none_or(|ten| two.unwrap() < ten), "+2 before +10: {two:?} {ten:?}");
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

/// Find References and the Delete window say where things are as they
/// did when GFF paths were text: each place and the path its link's hover
/// shows (`start.git /Creature List[10]/ScriptHeartbeat`, `caller.nss
/// line 1`, a sound set's `entry 3`, a 2DA's `row 0, Name`), the Delete
/// window's in the order of that text; and a conversation's place opens
/// at its row.
#[test]
fn the_references_window_says_where_things_are() {
    use mg_gff::{Struct, Value};
    let dir = mg_testkit::scratch_dir("ui-reference-places");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let rr = |s: &str| Value::resref(ResRef::from_str(s).unwrap());
    let with = |fields: Vec<(&str, Value)>| {
        let mut s = Struct::new(0);
        for (label, v) in fields {
            s.set(label, v);
        }
        s
    };
    let guard =
        || with(vec![("Tag", Value::String(b"GUARD".to_vec())), ("ScriptHeartbeat", rr("hello"))]);
    let mut git = Gff::new(*b"GIT ");
    git.root.set("Creature List", Value::List((0..12).map(|_| guard()).collect()));
    m.set_gff(ResKey::parse("start", ResType::GIT).unwrap(), &git).unwrap();
    let mut dlg = Gff::new(*b"DLG ");
    let text = |s: &str| Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, s));
    let link =
        || Value::List(vec![with(vec![("Index", Value::Dword(0)), ("Active", rr("hello"))])]);
    let entry =
        with(vec![("Text", text("Halt!")), ("Script", rr("hello")), ("RepliesList", link())]);
    dlg.root.set("EntryList", Value::List(vec![entry]));
    let reply = with(vec![("Text", text("A friend.")), ("EntriesList", Value::List(Vec::new()))]);
    dlg.root.set("ReplyList", Value::List(vec![reply]));
    dlg.root.set("StartingList", link());
    m.set_gff(ResKey::parse("talk", ResType::DLG).unwrap(), &dlg).unwrap();
    let caller = b"#include \"hello\"\nvoid main()\n{\n    SpeakStringByStrRef(16777220);\n}\n";
    m.set(ResKey::parse("caller", ResType::NSS).unwrap(), caller.to_vec());
    let mut item = Gff::new(*b"UTI ");
    let named = LocString { strref: mg_core::StrRef(16_777_220), strings: Vec::new() };
    item.root.set("LocalizedName", Value::LocString(named));
    m.set_gff(ResKey::parse("sword", ResType::UTI).unwrap(), &item).unwrap();
    let classes = b"2DA V2.0\n\n   Label Name\n0  Kz 16777220\n1  Other 5\n".to_vec();
    m.set(ResKey::parse("classes", ResType::TWODA).unwrap(), classes);
    let entries = (0..4)
        .map(|i| mg_ssf::SsfEntry {
            sound: Vec::new(),
            strref: mg_core::StrRef(if i == 3 { 16_777_220 } else { u32::MAX }),
        })
        .collect();
    let ssf = mg_ssf::Ssf { entries };
    m.set(ResKey::parse("guard", ResType::SSF).unwrap(), ssf.to_bytes().unwrap());
    m.save().unwrap();

    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    let said = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        (h.state().references.usages.iter())
            .map(|u| format!("{}\t{} {}", u.place, u.from, u.path))
            .collect()
    };
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::FindReferences(hello));
    h.run_steps(3);
    let mut expected = vec!["caller.nss › line 1\tcaller.nss line 1".to_string()];
    for i in 0..12 {
        let at = format!("start.git /Creature List[{i}]/ScriptHeartbeat");
        expected.push(format!("start › creature GUARD › OnHeartbeat\t{at}"));
    }
    expected.extend([
        "talk › NPC line 1 “Halt!” › action\ttalk.dlg /EntryList[0]/Script".to_string(),
        "talk › NPC line 1 “Halt!” (link) › condition\ttalk.dlg /EntryList[0]/RepliesList[0]/Active"
            .to_string(),
        "talk › start 1 › condition\ttalk.dlg /StartingList[0]/Active".to_string(),
    ]);
    assert_eq!(said(&h), expected);
    h.get_by_label("Used in 16 places:");
    // A conversation's place opens it at the row: a link's condition at
    // the link, a line's action at the link to the line.
    h.get_by_label("talk › NPC line 1 “Halt!” (link) › condition").click();
    h.run_steps(3);
    let talk = ResKey::parse("talk", ResType::DLG).unwrap();
    let selected =
        |h: &Harness<'_, Moonglow>| format!("{:?}", h.state().dialog_views[&talk].selected);
    assert_eq!(selected(&h), "Some(Row { parent: Node(Entry, 0), pos: 0 })");
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Dialog(talk)));
    h.state_mut().actions.push(mg_ui::Action::FindReferences(hello));
    h.run_steps(3);
    h.get_by_label("talk › NPC line 1 “Halt!” › action").click();
    h.run_steps(3);
    assert_eq!(selected(&h), "Some(Row { parent: Root, pos: 0 })");

    // A talk-table line: a localized string, a script's line, a 2DA's
    // cell and a sound set's entry.
    h.state_mut().actions.push(mg_ui::Action::FindStrRef(16_777_220));
    h.run_steps(3);
    assert_eq!(
        said(&h),
        [
            "caller.nss › line 4\tcaller.nss line 4",
            "classes.2da › row 0, Name\tclasses.2da row 0, Name",
            "guard.ssf › entry 3\tguard.ssf entry 3",
            "sword.uti › LocalizedName\tsword.uti /LocalizedName",
        ]
    );

    // The Delete window's, in the order of their text.
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(hello));
    h.run_steps(3);
    h.get_by_label("16 places in the module name it, left as they are:");
    let named = h.state().references.named.clone().expect("what names it");
    let order: Vec<String> =
        named.usages.iter().map(|u| format!("{} {}", u.from, u.path)).collect();
    let mut by_text = order.clone();
    by_text.sort();
    assert_eq!(order, by_text);
    assert_eq!(order[1], "start.git /Creature List[0]/ScriptHeartbeat");
    assert_eq!(order[2], "start.git /Creature List[10]/ScriptHeartbeat");
}
