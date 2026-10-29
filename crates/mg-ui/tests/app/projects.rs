//! Projects (nasher folders): files kept in place, changes from outside,
//! claims, builder codes, described variables and project commands.

use crate::common::*;

/// A NUI window moved in a nasher project to a folder its include leaves
/// out (a builder's report: not listed, after a restart too): said on
/// opening, and Build › Add Left-Out Files to nasher.cfg takes it in.
#[test]
fn a_nui_window_the_include_leaves_out_is_said_and_taken_in() {
    let dir = mg_testkit::scratch_dir("ui-nasher-left-out");
    let path = sample_module(&dir);
    let project = dir.join("project");
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(project.join("src/scripts/chat-commands")).unwrap();
    std::fs::write(
        project.join("nasher.cfg"),
        "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
         [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
    )
    .unwrap();
    let mut m = mg_module::Module::open(&path).unwrap();
    let location = mg_module::ModuleLocation::Project { root: project.clone(), target: "m".into() };
    m.save_as(&location).unwrap();
    let windows = project.join("src/scripts/chat-commands");
    std::fs::write(windows.join("test.jui"), br#"{"version":1}"#).unwrap();
    std::fs::write(windows.join("test.txt"), b"settings").unwrap();
    let mut h = harness(app_with(Vec::new()), 1500.0, 1000.0);
    h.state_mut().open_module(&project);
    h.run();
    let log = format!("{:?}", h.state().log.entries);
    assert!(log.contains("src/scripts/chat-commands/test.jui"), "{log}");
    assert!(h.query_by_label("NUI windows (0)").is_some());
    h.get_by_label("Build").click();
    h.run();
    h.get_by_label("Add Left-Out Files to nasher.cfg").click();
    h.run_steps(3);
    let jui = ResKey::parse("test", ResType::JUI).unwrap();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&jui), "{:?}", h.state().log.entries);
    assert!(h.query_by_label("NUI windows (1)").is_some());
    let cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
    assert!(cfg.contains("include = \"src/**/*.{jui,txt}\""), "{cfg}");
}

/// A script of a nasher project edited in the editor and saved is the
/// file it was with only the line edited changed (a builder's report: git
/// took the whole script as replaced). CRLF lines, a lone LF line, tabs,
/// a Windows-1252 byte and no newline at the end are kept as they were.
#[test]
fn a_project_s_script_edited_and_saved_changes_only_its_edited_line() {
    let dir = mg_testkit::scratch_dir("ui-nasher-script-bytes");
    let path = sample_module(&dir);
    let project = dir.join("project");
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("nasher.cfg"),
        "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
         [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
    )
    .unwrap();
    let mut m = Module::open(&path).unwrap();
    let location = mg_module::ModuleLocation::Project { root: project.clone(), target: "m".into() };
    m.save_as(&location).unwrap();
    let file = project.join("src/hello.nss");
    let before: &[u8] = b"// Caf\xe9 script\r\nvoid main()\r\n{\r\n\tint n = 1;\n\tint m = 2;\r\n}";
    std::fs::write(&file, before).unwrap();
    let other = project.join("src/other.nss");
    let untouched: &[u8] = b"void main()\n{\r\n    // \x93quoted\x94 \n}\n\n";
    std::fs::write(&other, untouched).unwrap();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&project);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let text = h.state().script_text(key).unwrap();
    let at = text[..text.find("1;").unwrap()].chars().count();
    h.state_mut().script_tools.jump = Some((key, at));
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    h.event(egui::Event::Text("7".into()));
    h.run();
    let edited = h.state().script_text(key).unwrap();
    assert!(edited.contains("int n = 7;"), "{edited:?} {text:?}");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    h.state_mut().run(mg_ui::Action::Save);
    h.run();
    let after = std::fs::read(&file).unwrap();
    let expected: &[u8] =
        b"// Caf\xe9 script\r\nvoid main()\r\n{\r\n\tint n = 7;\n\tint m = 2;\r\n}";
    assert_eq!(
        String::from_utf8_lossy(&after),
        String::from_utf8_lossy(expected),
        "{:?}",
        h.state().log.entries
    );
    assert_eq!(after, expected);
    assert_eq!(std::fs::read(&other).unwrap(), untouched, "a script not edited is as it was");
}

/// A NUI window's scripts in a nasher project: Build makes them in the
/// module, and Save writes them where the project's rules put them (a
/// builder's report: not copied to the project; Build said "written").
#[test]
fn a_nui_window_s_scripts_go_into_a_nasher_project() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-nasher-nui");
    let path = sample_module(&dir);
    let project = dir.join("project");
    let _ = std::fs::remove_dir_all(&project);
    let app = game_app_with(
        &root,
        None,
        NoDialogs { folders: vec![project.clone()], ..Default::default() },
    );
    let mut h = harness(app, 1500.0, 1000.0);
    h.state_mut().open_module(&path);
    h.run();
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Save As nasher Project…").click();
    h.run();
    assert!(project.join("nasher.cfg").is_file(), "{:?}", h.state().log.entries);
    h.state_mut().actions.push(mg_ui::Action::NewNui("zz_win".into()));
    h.run();
    h.get_by_label("Build & compile").click();
    h.run();
    let log = format!("{:?}", h.state().log.entries);
    assert!(log.contains("Built zz_win"), "{log}");
    // Said where they go, and when: into the project, with Save.
    assert!(log.contains("File › Save writes them into the nasher project's sources"), "{log}");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let mut files: Vec<String> = walk(&project.join("src"));
    files.sort();
    for name in ["zz_win_o.nss", "zz_win_e.nss"] {
        assert!(files.iter().any(|f| f.ends_with(name)), "{name} in {files:?}\n{log}");
    }
    // The project opened again: the window is in it.
    let back = mg_module::Module::open(&project).unwrap();
    let jui = ResKey::parse("zz_win", ResType::JUI).unwrap();
    assert!(
        back.contains(&jui),
        "files {files:?}; warnings {:?}",
        back.project.as_ref().unwrap().warnings
    );
}

#[test]
fn nasher_projects_save_in_place_and_never_overwrite_other_changes() {
    let dir = mg_testkit::scratch_dir("ui-nasher");
    let path = sample_module(&dir);
    let project = dir.join("project");
    let app = Moonglow::new(
        None,
        Box::new(NoDialogs {
            folders: vec![project.clone(), project.clone()],
            ..Default::default()
        }),
    );
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().open_module(&path);
    h.run();

    // File › Save As nasher Project… makes the project.
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Save As nasher Project…").click();
    h.run();
    let ifo_json = project.join("src/module.ifo.json");
    let script = project.join("src/hello.nss");
    assert!(project.join("nasher.cfg").is_file(), "{:?}", h.state().log.entries);
    assert!(ifo_json.is_file() && script.is_file());
    assert!(matches!(
        h.state().ws.as_ref().unwrap().module.location,
        Some(ModuleLocation::Project { .. })
    ));
    h.get_by_label("Build").click();
    h.run();
    h.get_by_label("Pack sample.mod");
    h.key_press(egui::Key::Escape);
    h.run();

    // Saving writes the changed resource's file alone.
    let script_before = std::fs::read(&script).unwrap();
    set_tag(h.state_mut(), "EDITED");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(std::fs::read_to_string(&ifo_json).unwrap().contains("\"EDITED\""));
    assert_eq!(std::fs::read(&script).unwrap(), script_before);

    // A file changed elsewhere (a git pull, say) is never overwritten.
    std::fs::write(
        &ifo_json,
        std::fs::read_to_string(&ifo_json).unwrap().replace("EDITED", "PULLED"),
    )
    .unwrap();
    set_tag(h.state_mut(), "MINE");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(std::fs::read_to_string(&ifo_json).unwrap().contains("\"PULLED\""));
    assert!(
        h.state().log.entries.iter().any(|(_, m)| m.contains("changed on disk")),
        "{:?}",
        h.state().log.entries
    );

    // File › Open Folder… opens the project as it is on disk.
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    h.get_by_label("Don't Save").click();
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenFolderDialog);
    h.run();
    let ws = h.state().ws.as_ref().expect("the project opens");
    assert_eq!(ws.module.info().unwrap().root.read(&ifo::MOD_TAG).as_bytes(), b"PULLED");
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("Opened nasher project")));
}

/// A nasher project that keeps its talk table as JSON (a tlk target of its
/// own, in two languages): opened, the table is the project's (not the tlk
/// folder's copy), edited, and saved with the module as nwn_tlk writes it;
/// the other language is chosen in the editor.
#[test]
fn a_new_talk_table_of_a_project_is_made_in_it() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-project-new");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("tlk")).unwrap();
    let m = mg_module::Module::open(&sample_module(&dir)).unwrap();
    let project = dir.join("project");
    let mut m = m;
    m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
        .unwrap();
    let mut app = game_app(&root, Some(user.clone()));
    app.settings.no_last_area = true;
    app.open_module(&project);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run();
    h.state_mut().talk_view.new_name = "mg_new".into();
    h.run();
    h.get_by_label_contains("Made in the project as");
    h.get_by_label("Create").click();
    h.run();
    h.run();
    let json = project.join("src/mg_new.tlk.json");
    let t = h.state().talk.as_ref().expect("the new table is open");
    assert!(matches!(&t.source, mg_module::talk::Source::Json { path, .. } if *path == json));
    assert!(!user.join("tlk/mg_new.tlk").exists(), "not in the tlk folder");
    let cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
    assert!(cfg.contains("file = \"mg_new.tlk\"") && cfg.contains("exclude = \"**/*.tlk.json\""));
    // The module names it; a line added and saved goes into its JSON.
    h.get_by_label("Add Line").click();
    h.run();
    type_into_hint(&mut h, "the line's text", "Fresh");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(std::fs::read_to_string(&json).unwrap().contains("\"text\":\"Fresh\""));
    let m = mg_module::Module::open(&project).unwrap();
    assert_eq!(m.custom_tlk().unwrap().as_deref(), Some("mg_new"));
    assert!(m.project.as_ref().unwrap().warnings.is_empty());
}

#[test]
fn a_projects_talk_table_kept_as_json_is_edited_there() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-project");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("tlk")).unwrap();
    let mut m = mg_module::Module::open(&sample_module(&dir)).unwrap();
    let mut info = m.info().unwrap();
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_proj".to_vec()));
    m.set_info(&info).unwrap();
    let project = dir.join("project");
    m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
        .unwrap();
    let mut cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
    cfg.push_str(
        "\n[target]\nname = \"tlk\"\nfile = \"mg_proj.tlk\"\n  [target.sources]\n  \
         include = \"src/tlk/*.json\"\n\n[target]\nname = \"tlk_de\"\nfile = \"de/mg_proj.tlk\"\n  \
         [target.sources]\n  include = \"src/tlk/de/*.json\"\n",
    );
    std::fs::write(project.join("nasher.cfg"), cfg).unwrap();
    std::fs::create_dir_all(project.join("src/tlk/de")).unwrap();
    let en =
        r#"{"language":0,"entries":[{"id":0,"text":"Hello"},{"id":2,"text":"","sound":"vs_x"}]}"#;
    let json = project.join("src/tlk/mg_proj.tlk.json");
    std::fs::write(&json, en).unwrap();
    let de = project.join("src/tlk/de/mg_proj.tlk.json");
    std::fs::write(&de, r#"{"language":2,"entries":[{"id":0,"text":"Hallo"}]}"#).unwrap();
    // (What `nasher install` put in the tlk folder once: not the one read.)
    let mut stale = mg_tlk::Tlk::new(Language::ENGLISH);
    stale.entries.push(mg_tlk::TlkEntry::text("Stale"));
    std::fs::write(user.join("tlk/mg_proj.tlk"), stale.to_bytes().unwrap()).unwrap();
    std::fs::write(user.join("tlk/mg_projf.tlk"), stale.to_bytes().unwrap()).unwrap();
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&project);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run();
    h.run();
    let t = h.state().talk.as_ref().expect("the project's table is open");
    assert!(matches!(&t.source, mg_module::talk::Source::Json { path, .. } if *path == json));
    assert!(t.editable());
    assert_eq!((t.len(), t.line(0).text.as_str()), (3, "Hello"));
    assert!(t.feminine.is_none(), "not the tlk folder's feminine table beside the project's");
    let game = h.state().game.as_deref().unwrap();
    assert_eq!(game.string(mg_core::StrRef(16_777_216)).as_deref(), Some("Hello"));
    // A line added and typed, saved with the module: the JSON as nwn_tlk
    // writes it, the lines there kept as they were.
    h.get_by_label("Add Line").click();
    h.run();
    type_into_hint(&mut h, "the line's text", "Greetings");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(!h.state().talk.as_ref().unwrap().is_dirty());
    let written = std::fs::read_to_string(&json).unwrap();
    assert_eq!(written, en.replace("]}", ",{\"id\":3,\"text\":\"Greetings\"}]}"));
    let game = h.state().game.as_deref().unwrap();
    assert_eq!(game.string(mg_core::StrRef(16_777_219)).as_deref(), Some("Greetings"));
    // The table's German, its file of its own.
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    (h.query_all(combo))
        .find(|n| n.accesskit_node().value().is_some_and(|v| v == "English"))
        .expect("the table's languages")
        .click();
    h.run();
    h.get_by_label("German").click();
    h.run();
    h.run();
    let t = h.state().talk.as_ref().unwrap();
    assert!(matches!(&t.source, mg_module::talk::Source::Json { path, .. } if *path == de));
    assert_eq!((t.len(), t.line(0).text.as_str()), (1, "Hallo"));
    // Changed outside Moonglow (a `git pull`): the editor reads it again;
    // with unsaved changes here, those are kept, and said.
    std::fs::write(&de, r#"{"language":2,"entries":[{"id":0,"text":"Servus"}]}"#).unwrap();
    h.state_mut().reload_resources(false);
    h.run();
    assert_eq!(h.state().talk.as_ref().unwrap().line(0).text, "Servus");
    assert!(!h.state().talk.as_ref().unwrap().is_dirty());
    h.get_by_label("Add Line").click();
    h.run();
    std::fs::write(&de, r#"{"language":2,"entries":[{"id":0,"text":"Grüß Gott"}]}"#).unwrap();
    h.state_mut().reload_resources(false);
    h.run();
    let t = h.state().talk.as_ref().unwrap();
    assert_eq!((t.len(), t.line(0).text.as_str()), (2, "Servus"), "ours kept");
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("while it has unsaved changes")));
}

/// A module folder with a shopkeeper in an area and a store of the
/// area's tagged as `[store]`'s pattern's first, another area's store with
/// its second, and the project file with `[store]` and the pattern
/// `unique_in_area`: the install, the folder, the area and its tag.
fn store_project(
    name: &str,
) -> Option<(mg_resman::GameInstall, std::path::PathBuf, ResRef, String, ResKey)> {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Shop", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let area_tag = {
        let are = m.gff(&ResKey::new(area, ResType::ARE)).unwrap().unwrap();
        String::from_utf8(are.root.string("Tag").unwrap().to_vec()).unwrap()
    };
    let read = |name: &str, t: ResType| {
        Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
    };
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let keeper = instance(&placing, ResType::UTC, &read("nw_bandit001", ResType::UTC), at, &[]);
    // A store of the area's with the pattern's first tag already.
    let mut first =
        instance(&placing, ResType::UTM, &read("nw_storethief001", ResType::UTM), at, &[]).unwrap();
    let taken = format!("{area_tag}_STORE_1");
    first.set("Tag", mg_gff::Value::String(taken.clone().into_bytes()));
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![keeper.unwrap()]));
    g.root.set("StoreList", mg_gff::Value::List(vec![first.clone()]));
    m.set_gff(git, &g).unwrap();
    // Another area with the pattern's second number: wanted unique in each
    // area (`unique_in_area`), it is free in this one.
    let spec = AreaSpec { name: "Mill".into(), ..spec };
    let mill = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let mut other = first;
    other.set("Tag", mg_gff::Value::String(format!("{area_tag}_STORE_2").into_bytes()));
    let mills = ResKey::new(mill, ResType::GIT);
    let mut g = m.gff(&mills).unwrap().unwrap();
    g.root.set("StoreList", mg_gff::Value::List(vec![other]));
    m.set_gff(mills, &g).unwrap();
    let dir = mg_testkit::scratch_dir(name).join("shop");
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(
        dir.join("moonglow.toml"),
        "[tags]\nunique_in_area = [\"*_STORE_*\"]\n\n\
         [store]\nopen_script = \"tdod_vendor_open\"\ntag = \"{area}_STORE_{n}\"\n\
         blueprint = \"nw_storethief001\"\nkeeper_variable = \"tdod_store\"\n",
    )
    .unwrap();
    Some((install, dir, area, area_tag, git))
}

/// The project file is read again when it changes with Options ›
/// General's reloading off too, and Setup Store, open then, follows the
/// rules as they are now.
#[test]
fn the_project_file_is_read_again_with_reloading_off() {
    let Some((install, dir, area, _, _)) = store_project("ui-project-file-again") else { return };
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_auto_reload = true;
    app.open_module(&dir);
    app.open_palette = false;
    mg_ui::store_wizard::open(&mut app, area, "Creature List", 0);
    // (Changed before the window's first frame, which looks at it.)
    std::fs::write(
        dir.join("moonglow.toml"),
        "[store]\nopen_script = \"tdod_vendor_two\"\nblueprint = \"nw_storethief001\"\n",
    )
    .unwrap();
    let mut h = harness(app, 1000.0, 800.0);
    h.run_steps(3);
    let said = |s: &str| h.state().log.entries.iter().any(|e| e.1.contains(s));
    assert!(said("moonglow.toml changed: its rules are read again"), "{:?}", h.state().log.entries);
    assert!(h.query_by_label_contains("the project's script tdod_vendor_two").is_some());
    assert!(h.query_by_label_contains("tdod_vendor_open").is_none());
}

/// Setup Store in a project whose moonglow.toml has `[store]`: the
/// project's script opens it (none is written), its tag follows the
/// pattern and is the only one (of its area, as `unique_in_area` wants
/// it), its blueprint is offered first, and the shopkeeper keeps its tag
/// in the project's variable.
#[test]
fn setup_store_follows_the_project_file() {
    let Some((install, dir, area, area_tag, git)) = store_project("ui-setup-store-project") else {
        return;
    };
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&dir);
    app.open_palette = false;
    assert!(app.log.entries.iter().any(|e| e.1.contains("read from moonglow.toml")));
    mg_ui::store_wizard::open(&mut app, area, "Creature List", 0);
    let mut h = harness(app, 1000.0, 800.0);
    h.run();
    let w = h.state().store_wizard.clone().unwrap();
    assert_eq!(w.store, ResRef::from_str("nw_storethief001").ok(), "the project's, chosen");
    assert!(h.query_by_label_contains("the project's script tdod_vendor_open").is_some());
    assert!(h.query_by_label("Use appraise checks").is_none());
    h.get_by_label("Next >").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    let tag = format!("{area_tag}_STORE_2");
    assert!(h.query_by_label(&format!("The store's tag: {tag}")).is_some(), "{tag}");
    h.get_all_by_label("Finish").last().unwrap().click();
    h.run();
    assert!(h.state().store_wizard.is_none(), "{:?}", h.state().log.entries);
    let said =
        |h: &Harness<'_, Moonglow>, s: &str| h.state().log.entries.iter().any(|e| e.1.contains(s));
    assert!(said(&h, "tdod_vendor_open is neither the module's nor the game's"));
    let ws = h.state_mut().ws.as_mut().unwrap();
    for t in [ResType::NSS, ResType::NCS] {
        assert!(!ws.module.keys_of(t).any(|k| k.resref.to_string().starts_with("openstore")));
    }
    let dlg = ws.doc(&ResKey::parse("store001", ResType::DLG).unwrap()).unwrap();
    let opens = dlg.root.list("ReplyList").unwrap()[0].resref("Script");
    assert_eq!(opens, ResRef::from_str("tdod_vendor_open").ok());
    let g = ws.doc(&git).unwrap();
    let stores = g.root.list("StoreList").unwrap();
    assert_eq!(stores[1].string("Tag"), Some(tag.as_bytes()));
    let c = &g.root.list("Creature List").unwrap()[0];
    let var = c.list("VarTable").unwrap().iter().find(|v| v.string("Name") == Some(b"tdod_store"));
    let var = var.expect("the shopkeeper's variable");
    assert_eq!((var.dword("Type"), var.string("Value")), (Some(3), Some(tag.as_bytes())));
}

/// Setup Store's list of stores opens scrolled to the project's
/// blueprint, chosen at first, though it is the last of the game's.
#[test]
fn setup_store_s_list_shows_the_project_s_blueprint_far_down() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    // The store the list shows last: by name, then ResRef, as it sorts.
    let mut stores: Vec<(String, ResRef)> = game
        .resman
        .list(ResType::UTM)
        .into_iter()
        .map(|r| {
            let data = game.resman.get(&ResKey::new(r, ResType::UTM)).unwrap();
            let g = Gff::read(&data).unwrap();
            let name = g.root.locstring("LocName").and_then(|l| game.locstring(l));
            (name.unwrap_or_default(), r)
        })
        .collect();
    stores.sort();
    let (name, last) = stores.last().cloned().unwrap();
    assert!(stores.len() > 20, "a list longer than the window shows");
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Shop", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let blueprint = game.resman.get(&ResKey::parse("nw_bandit001", ResType::UTC).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let keeper = instance(&placing, ResType::UTC, &Gff::read(&blueprint).unwrap().root, at, &[]);
    let key = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&key).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![keeper.unwrap()]));
    m.set_gff(key, &g).unwrap();
    let dir = mg_testkit::scratch_dir("ui-setup-store-far-down").join("shop");
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join("moonglow.toml"), format!("[store]\nblueprint = \"{last}\"\n"))
        .unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&dir);
    app.open_palette = false;
    mg_ui::store_wizard::open(&mut app, area, "Creature List", 0);
    let mut h = harness(app, 1000.0, 800.0);
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.run();
    // The row is drawn inside the list, above the buttons under it.
    let row = h.get_by_label(&format!("{name} ({last})")).rect();
    let top = h.get_by_label("Choose the store to use.").rect().bottom();
    let bottom = h.get_by_label("Next >").rect().top();
    assert!(row.top() > top && row.bottom() < bottom, "{row:?} between {top} and {bottom}");
    // Chosen once: the list is the builder's to scroll after.
    let first = format!("{} ({})", stores[0].0, stores[0].1);
    h.get_by_label(&format!("{name} ({last})")).click();
    h.run();
    assert!(h.get_by_label(&first).rect().bottom() < top, "not scrolled back");
}

/// A nasher project's files changed by another program are read again
/// while the project is open; where Moonglow has unsaved changes to one,
/// its own is kept and the window asks.
#[test]
fn a_project_s_files_changed_outside_are_read_again() {
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Outside", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    let dir = mg_testkit::scratch_dir("ui-project-outside").join("project");
    m.save_as(&ModuleLocation::Project { root: dir.clone(), target: "default".into() }).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&dir);
    let mut h = harness(app, 1100.0, 800.0);
    h.run_steps(3);
    let are = ResKey::new(area, ResType::ARE);
    let are_file = dir.join(format!("src/{area}.are.json"));
    let tag = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        String::from_utf8(ws.doc(&are).unwrap().root.string("Tag").unwrap().to_vec()).unwrap()
    };
    let first = tag(&mut h);
    let retag = |from: &str, to: &str| {
        let text = std::fs::read_to_string(&are_file).unwrap();
        let (a, b) = (format!("\"{from}\""), format!("\"{to}\""));
        assert!(text.contains(&a), "{from} in the file");
        std::fs::write(&are_file, text.replace(&a, &b)).unwrap();
    };
    let said = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state().log.entries.iter().any(|(_, m)| m.contains(what))
    };
    // Looked at (its document read), not changed: nothing to reload.
    h.state_mut().reload_project_files();
    assert!(!said(&h, "Read again"));

    // Another program changes the area, a script, adds one and deletes none.
    retag(&first, "OUTSIDE");
    std::fs::write(dir.join("src/hello.nss"), "void main() { int outside; }\n").unwrap();
    std::fs::write(dir.join("src/fresh.nss"), "void main() {}\n").unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(2);
    assert_eq!(tag(&mut h), "OUTSIDE");
    {
        let ws = h.state().ws.as_ref().unwrap();
        let script = |n: &str| ws.module.get(&ResKey::parse(n, ResType::NSS).unwrap());
        assert_eq!(script("hello"), Some(&b"void main() { int outside; }\n"[..]));
        assert!(script("fresh").is_some(), "the new file is in the module");
        assert!(!ws.is_modified(), "read again is not unsaved work");
    }
    assert!(said(&h, "Read again from the module's files"));
    assert!(h.state().outside_conflicts().is_empty());

    // Changed here (unsaved) and outside both: Moonglow's is kept, and asked.
    let set_tag = |h: &mut Harness<'_, Moonglow>, to: &str| {
        let edit = mg_edit::Edit::SetField {
            key: are,
            path: mg_edit::GffPath::root(),
            label: "Tag".into(),
            value: Some(mg_gff::Value::String(to.as_bytes().to_vec())),
        };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
        h.run_steps(2);
    };
    set_tag(&mut h, "MINE");
    retag("OUTSIDE", "THEIRS");
    h.state_mut().reload_project_files();
    h.run_steps(2);
    assert_eq!(tag(&mut h), "MINE", "kept");
    assert_eq!(h.state().outside_conflicts(), std::slice::from_ref(&are_file));
    assert!(said(&h, "has unsaved changes here"));
    // Take the Files': theirs, and the change here is gone.
    h.get_by_label("Take the Files'").click();
    h.run_steps(3);
    assert_eq!(tag(&mut h), "THEIRS");
    assert!(h.state().outside_conflicts().is_empty());

    // Again, and Keep Moonglow's: the next save writes it over the file.
    set_tag(&mut h, "MINE2");
    retag("THEIRS", "THEIRS2");
    h.state_mut().reload_project_files();
    h.run_steps(2);
    h.get_by_label("Keep Moonglow's").click();
    h.run_steps(3);
    assert_eq!(tag(&mut h), "MINE2");
    h.state_mut().run(mg_ui::Action::Save);
    h.run_steps(2);
    assert!(std::fs::read_to_string(&are_file).unwrap().contains("\"MINE2\""), "saved over it");
    h.state_mut().reload_project_files();
    assert!(h.state().outside_conflicts().is_empty());
}

/// A `git pull` on an open nasher project: a burst of files rewritten,
/// added and deleted is read on a thread of its own while the window goes
/// on drawing (nothing of it is in the module until the look is done),
/// then all of it is in, in one go; an area with an unsaved change here
/// is kept and asked about, and Take the Files' still takes it. A change
/// after that is found by the watch's looks. (How long the window's frames
/// take while it reloads: `frame_perf.rs`, `reload_frames_at_144`.)
#[test]
fn a_pull_s_burst_of_changes_is_read_off_the_window_s_thread() {
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Pull", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let areas: Vec<ResRef> =
        (0..4).map(|_| add_area(&mut m, &game, &spec, &mut rng).unwrap()).collect();
    let script = |n: usize| ResKey::parse(&format!("s{n:03}"), ResType::NSS).unwrap();
    for n in 0..120 {
        m.set(script(n), format!("void main() {{ int n{n}; }}\n").into_bytes());
    }
    let dir = mg_testkit::scratch_dir("ui-project-pull").join("project");
    m.save_as(&ModuleLocation::Project { root: dir.clone(), target: "default".into() }).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&dir);
    let mut h = harness(app, 1100.0, 800.0);
    h.run_steps(3);
    // (A first look, as the watch's start makes: nothing changed.)
    h.state_mut().look_outside();
    while h.state().looking_outside() {
        h.step();
    }
    let tag = |h: &mut Harness<'_, Moonglow>, area: ResRef| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let g = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        String::from_utf8(g.root.string("Tag").unwrap().to_vec()).unwrap()
    };
    let tags: Vec<String> = areas.iter().map(|a| tag(&mut h, *a)).collect();
    // An unsaved change to the first area, here.
    let edit = mg_edit::Edit::SetField {
        key: ResKey::new(areas[0], ResType::ARE),
        path: mg_edit::GffPath::root(),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(b"MINE".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
    h.run_steps(2);

    // The pull: every script rewritten or deleted, new ones, every area
    // retagged.
    for n in 0..120 {
        let path = dir.join(format!("src/s{n:03}.nss"));
        if n < 10 {
            std::fs::remove_file(path).unwrap();
        } else {
            std::fs::write(path, format!("void main() {{ int pulled{n}; }}\n")).unwrap();
        }
    }
    for n in 0..10 {
        std::fs::write(dir.join(format!("src/new{n}.nss")), "void main() {}\n").unwrap();
    }
    let are_file = |a: ResRef| dir.join(format!("src/{a}.are.json"));
    for (i, (a, t)) in areas.iter().zip(&tags).enumerate() {
        let text = std::fs::read_to_string(are_file(*a)).unwrap();
        assert!(text.contains(&format!("\"{t}\"")), "{t} in the file");
        let text = text.replace(&format!("\"{t}\""), &format!("\"PULLED{i}\""));
        std::fs::write(are_file(*a), text).unwrap();
    }

    // The look begins and the window's thread goes on: nothing is read yet.
    let in_module = |h: &Harness<'_, Moonglow>, k: &ResKey| {
        h.state().ws.as_ref().unwrap().module.get(k).map(<[u8]>::to_vec)
    };
    h.state_mut().look_outside();
    assert!(h.state().looking_outside());
    assert_eq!(in_module(&h, &script(50)).unwrap(), b"void main() { int n50; }\n");
    assert!(in_module(&h, &script(5)).is_some());
    let (mut frames, started) = (0, std::time::Instant::now());
    while h.state().looking_outside() {
        h.step();
        frames += 1;
        assert!(started.elapsed().as_secs() < 60, "the look ends");
    }
    assert!(frames > 0);
    // (The window that asks, laid out.)
    h.run_steps(2);
    // All of it in.
    assert_eq!(in_module(&h, &script(50)).unwrap(), b"void main() { int pulled50; }\n");
    assert!(in_module(&h, &script(5)).is_none(), "deleted");
    assert!(in_module(&h, &ResKey::parse("new3", ResType::NSS).unwrap()).is_some(), "added");
    for (i, a) in areas.iter().enumerate().skip(1) {
        assert_eq!(tag(&mut h, *a), format!("PULLED{i}"));
    }
    let said = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state().log.entries.iter().filter(|(_, m)| m.contains(what)).count()
    };
    assert_eq!(said(&h, "Read again from the module's files"), 1, "in one go");
    // The area changed here too: kept, and asked about.
    assert_eq!(tag(&mut h, areas[0]), "MINE");
    assert_eq!(h.state().outside_conflicts(), [are_file(areas[0])]);
    assert_eq!(said(&h, "has unsaved changes here"), 1);
    h.get_by_label("Take the Files'").click();
    h.run_steps(3);
    assert_eq!(tag(&mut h, areas[0]), "PULLED0");
    assert!(h.state().outside_conflicts().is_empty());

    // Later, a change is found by the watch's looks (the window's
    // every few seconds).
    std::fs::write(dir.join("src/s050.nss"), "void main() { int later; }\n").unwrap();
    let started = std::time::Instant::now();
    while in_module(&h, &script(50)).unwrap() != b"void main() { int later; }\n" {
        assert!(started.elapsed().as_secs() < 30, "the watch finds it");
        h.state_mut().reload_changed_files();
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// A module kept in one file (a `.mod`) that another program saves while
/// it is open: the watch hears it at once, without the window being asked,
/// and what changed in it is read again, resource by resource; one with
/// unsaved changes here too is asked about, and Save doesn't write over
/// it until it is answered.
#[test]
fn a_mod_saved_by_another_program_is_read_again() {
    use mg_module::new::new_module;
    let Some(root) = mg_testkit::nwn_root() else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut m = new_module(&game, "Castle", &mut fastrand::Rng::with_seed(7)).unwrap();
    let (a, b) = (
        ResKey::parse("castle_a", ResType::NSS).unwrap(),
        ResKey::parse("castle_b", ResType::NSS).unwrap(),
    );
    m.set(a, b"void main() { int a; }\n".to_vec());
    m.set(b, b"void main() { int b; }\n".to_vec());
    let path = mg_testkit::scratch_dir("ui-mod-outside").join("castle.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1100.0, 800.0);
    // (The watch begins, and its first look finds nothing.)
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_millis(600) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let in_module = |h: &Harness<'_, Moonglow>, k: &ResKey| {
        h.state().ws.as_ref().unwrap().module.get(k).map(<[u8]>::to_vec)
    };
    // Another program saves it: read again within a second or two.
    let mut theirs = Module::open(&path).unwrap();
    theirs.set(a, b"void main() { int theirs; }\n".to_vec());
    theirs.save().unwrap();
    let started = std::time::Instant::now();
    while in_module(&h, &a).unwrap() != b"void main() { int theirs; }\n" {
        assert!(started.elapsed().as_secs() < 10, "the watch hears it");
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(started.elapsed().as_secs() < 3, "{:?} after the save", started.elapsed());
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "read, not an unsaved change");
    // Changed here and there: asked about, and not written over.
    let edit =
        mg_edit::Edit::SetResource { key: b, data: Some(b"void main() { int ours; }\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Ours", vec![edit])));
    h.run_steps(2);
    let mut theirs = Module::open(&path).unwrap();
    theirs.set(b, b"void main() { int theirs, too; }\n".to_vec());
    theirs.save().unwrap();
    let started = std::time::Instant::now();
    while h.state().outside_conflicts().is_empty() {
        assert!(started.elapsed().as_secs() < 10, "the watch hears it");
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(h.state().outside_conflicts(), [path.join("castle_b.nss")]);
    assert_eq!(in_module(&h, &b).unwrap(), b"void main() { int ours; }\n");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    let on_disk = Module::open(&path).unwrap();
    assert_eq!(on_disk.get(&b).unwrap(), b"void main() { int theirs, too; }\n", "not written over");
    // Kept as Moonglow has it: then saved.
    h.get_by_label("Keep Moonglow's").click();
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    let on_disk = Module::open(&path).unwrap();
    assert_eq!(on_disk.get(&b).unwrap(), b"void main() { int ours; }\n");
}

/// A nasher project's locks stay out of its files: Lock and Save leave the
/// area's JSON as it was and keep the lock in Moonglow's folder; the area
/// changed outside is read again (no conflict) with the lock still on; and
/// the project opened again has it back.
#[test]
fn a_project_s_locks_stay_out_of_its_files() {
    use mg_gff::Value;
    use mg_module::new::{AreaSpec, add_area, new_module};
    use mg_schema::git;
    let Some(root) = mg_testkit::nwn_root() else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Locks", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let key = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&key).unwrap().unwrap();
    for (tag, x) in [("POST_A", 5.0), ("POST_B", 9.0)] {
        let mut w = git::WAYPOINT_LIST.new_item();
        w.set("Tag", Value::String(tag.as_bytes().to_vec()));
        w.set("TemplateResRef", Value::resref(ResRef::from_str("nw_waypoint001").unwrap()));
        w.set("XPosition", Value::Float(x));
        w.set("YPosition", Value::Float(5.0));
        g.root.items_mut(&git::WAYPOINT_LIST).push(w);
    }
    m.set_gff(key, &g).unwrap();
    let scratch = mg_testkit::scratch_dir("ui-project-locks");
    let dir = scratch.join("project");
    m.save_as(&ModuleLocation::Project { root: dir.clone(), target: "default".into() }).unwrap();
    let file = dir.join(format!("src/{area}.git.json"));
    let read = || std::fs::read_to_string(&file).unwrap();
    let start = |dir: &std::path::Path| {
        let mut app = Moonglow::new(Some(install.clone()), Box::new(NoDialogs::default()));
        app.settings.no_last_area = true;
        app.locks_dir = Some(scratch.join("locks"));
        app.open_module(dir);
        let mut h = harness(app, 1100.0, 800.0);
        h.run_steps(3);
        h
    };
    let locked = |h: &mut Harness<'_, Moonglow>| -> Vec<bool> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let g = ws.doc(&key).unwrap();
        g.root
            .items(&git::WAYPOINT_LIST)
            .iter()
            .map(|s| s.byte(mg_area::LOCKED) == Some(1))
            .collect()
    };
    let mut h = start(&dir);
    let written = read();
    let lock = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root().item(git::WAYPOINT_LIST.label, 0),
        label: mg_area::LOCKED.into(),
        value: Some(Value::Byte(1)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Lock", vec![lock])));
    h.run_steps(2);
    assert_eq!(locked(&mut h), [true, false]);
    h.state_mut().run(mg_ui::Action::Save);
    h.run_steps(2);
    assert_eq!(read(), written, "the file is as it was");
    let kept = mg_module::locks::file(&scratch.join("locks"), &dir);
    assert_eq!(mg_module::locks::read(&kept).len(), 1, "{:?}", h.state().log.entries);

    // Changed outside (a pull): read again, not a conflict, still locked.
    std::fs::write(&file, read().replace("POST_B", "POST_C")).unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(2);
    assert!(h.state().outside_conflicts().is_empty());
    assert_eq!(locked(&mut h), [true, false]);
    assert!(!h.state().ws.as_ref().unwrap().is_modified());

    // Opened again: the lock is back.
    drop(h);
    let mut h = start(&dir);
    assert_eq!(locked(&mut h), [true, false]);
    assert!(!h.state().ws.as_ref().unwrap().is_modified());
}

/// In a nasher project the external editor gets the project's own script
/// file, in place; a script with text typed and not saved gets a copy.
#[cfg(unix)]
#[test]
fn a_project_s_script_opens_in_place_in_the_external_editor() {
    use mg_module::ModuleLocation;
    use mg_module::new::new_module;
    let Some(root) = mg_testkit::nwn_root() else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut m = new_module(&game, "InPlace", &mut fastrand::Rng::with_seed(7)).unwrap();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(key, b"void main()\n{\n}\n".to_vec());
    let dir = mg_testkit::scratch_dir("ui-project-external").join("project");
    m.save_as(&ModuleLocation::Project { root: dir.clone(), target: "default".into() }).unwrap();
    let Some(editor) = true_program() else { return };
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.settings.external_editor = Some(editor.clone().into());
    app.settings.scripts_external = true;
    app.open_module(&dir);
    let mut h = harness(app, 1100.0, 800.0);
    h.run_steps(3);
    let said = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state().log.entries.iter().filter(|(_, m)| m.contains(what)).count()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenResource(key));
    h.run_steps(4);
    let file = dir.join("src/hello.nss");
    assert!(file.is_file(), "the project's own file");
    assert_eq!(said(&h, &format!("editing {} in {editor}", file.display())), 1);
    // Saved there by the editor: read again as a file changed outside is.
    // Written in place, as VS Code saves (VSCodium 1.143: its writes are
    // atomic only for its own settings files)...
    std::fs::write(&file, "void main()\n{\n    int n = 1;\n}\n").unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(3);
    assert!(h.state().script_text(key).unwrap().contains("int n = 1;"));
    // ...and written beside it and renamed over it, as Vim and Helix save.
    let beside = dir.join("src/.hello.nss.swp");
    std::fs::write(&beside, "void main()\n{\n    int n = 2;\n}\n").unwrap();
    std::fs::rename(&beside, &file).unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(3);
    assert!(h.state().script_text(key).unwrap().contains("int n = 2;"));
    // With text typed here and not saved: a copy, and why.
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(2);
    let tab = h.state().dock.find_tab(&Tab::Script(key)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::OpenResource(key));
    h.run_steps(4);
    assert_eq!(said(&h, "a copy is opened, not the project's file"), 1);
}

/// A table with a description beside it (`name.2da.toml`): its cells are
/// held to what the description says, values show by their names, and a
/// cell is chosen from its list or ticked from its flags, which a
/// script's constants give.
#[test]
fn a_described_table_is_chosen_from_its_lists() {
    let dir = mg_testkit::scratch_dir("ui-table-described");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/loot_inc.nss"),
        "const int LOOT_KIND_GOLD = 0x01;\nconst int LOOT_KIND_GEMS = 0x02;\n",
    )
    .unwrap();
    let file = dir.join("loot.2da");
    std::fs::write(
        &file,
        "2DA V2.0\r\n\r\n  Label Chance Kinds Rarity Place\r\n0 Chest 50 0x01 0 1\r\n1 Urn 500 \
         0x03 1 0\r\n",
    )
    .unwrap();
    // (A table of the project's own, which the description names twice:
    // the rows are its rows, and a column names one.)
    std::fs::write(
        dir.join("src/places.2da"),
        "2DA V2.0\r\n\r\n  Label\r\n0 Cellar\r\n1 Attic\r\n2 Vault\r\n",
    )
    .unwrap();
    let description = "[columns.Chance]\nkind = \"int\"\nmin = 0\nmax = 100\nnote = \"In a hundred\"\n\n\
         [columns.Kinds]\nkind = \"flags\"\nconstants = \"loot_inc\"\nprefix = \
         \"LOOT_KIND_\"\n\n[columns.Rarity]\nkind = \"list\"\nvalues = { Common = 0, Rare = 1 }\n\n\
         [columns.Place]\nkind = \"row\"\ntable = \"places\"\n";
    let description = format!("rows = \"places\"\n\n{}", description);
    std::fs::write(dir.join("loot.2da.toml"), &description).unwrap();
    let mut h = harness(app_with(Vec::new()), 1200.0, 800.0);
    h.run_steps(2);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    h.get_by_label_contains("Described by loot.2da.toml: 4 columns");
    let d = h.state().tables[0].description().expect("the description is read");
    assert_eq!(d.column("Kinds").unwrap().kind.entries().len(), 2);
    // Values by their names; the cell out of its range is the one wrong.
    h.get_by_label("LOOT_KIND_GOLD | LOOT_KIND_GEMS");
    h.get_by_label("Rare");
    h.get_by_label("1 cell is wrong").click();
    h.run_steps(3);
    assert_eq!(h.state().tables[0].selected, Some((1, 1)));
    h.get_by_label_contains("500 is above what the column takes");
    h.get_by_label("In a hundred");
    // The rows are another table's: the other's row is said with the cell.
    h.get_by_label_contains("Attic of places.2da");
    // Find looks through the names shown too.
    h.state_mut().tables[0].filter = "rare".into();
    h.run_steps(3);
    assert!(h.query_by_label("Urn").is_some() && h.query_by_label("Chest").is_none());
    h.state_mut().tables[0].filter.clear();
    h.run_steps(3);
    // A row of the project's own table is chosen by its name.
    h.state_mut().tables[0].go_to(0, 4);
    h.run_steps(3);
    h.get_by_label_contains("a row of places.2da");
    {
        let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
        let at = h
            .query_all(combo)
            .find(|n| n.accesskit_node().value().is_some_and(|v| v.contains("Attic")))
            .map(|n| n.rect().center())
            .expect("the rows' list");
        h.hover_at(at);
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
    }
    h.get_by_label_contains("Vault").click();
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(0, "Place"), Some("2"));
    // A list's cell is chosen by name: the table gets the value.
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let open = |h: &mut Harness<'_, Moonglow>, shows: &str| {
        let at = h
            .query_all(combo.clone())
            .find(|n| n.accesskit_node().value().is_some_and(|v| v == shows))
            .map(|n| n.rect().center())
            .unwrap_or_else(|| panic!("the list showing {shows}"));
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    h.get_by_label("Common").click();
    h.run_steps(3);
    open(&mut h, "Common");
    h.get_by_label("Rare  (1)").click();
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(0, "Rarity"), Some("1"));
    // Flags are ticked: each is a bit of the cell, written as the list's
    // own values are.
    h.get_by_label("LOOT_KIND_GOLD").click();
    h.run_steps(3);
    open(&mut h, "LOOT_KIND_GOLD");
    h.get_by_label("LOOT_KIND_GEMS  (0x02)").click();
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(0, "Kinds"), Some("0x03"));
    h.get_by_label("LOOT_KIND_GOLD  (0x01)").click();
    h.run_steps(3);
    h.get_by_label("LOOT_KIND_GEMS  (0x02)").click();
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(0, "Kinds"), None, "no flag: an empty cell");
    // A value no list has, typed in the cell, is a fault said as it is typed.
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    h.get_all_by_label("Rare").next().unwrap().click();
    h.run_steps(3);
    assert_eq!(h.state().tables[0].selected, Some((0, 3)));
    h.key_press(egui::Key::F2);
    h.run_steps(3);
    h.event(egui::Event::Text("7".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(0, "Rarity"), Some("17"));
    h.get_by_label("2 cells are wrong");
    // The description is read again when its file changes: with a wider
    // range, the chance of 500 is not wrong any more.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(dir.join("loot.2da.toml"), description.replace("max = 100", "max = 1000"))
        .unwrap();
    // (Looked at once a second, of the time egui gives.)
    h.run_steps(90);
    h.get_by_label("1 cell is wrong");
}

/// With the option, a project's script opens in the external editor alone:
/// no editor of Moonglow's. A script of a module in one file still opens
/// in Moonglow's too (that is how the external editor's saves come back).
#[cfg(unix)]
#[test]
fn a_project_s_script_opens_in_the_external_editor_alone() {
    use mg_module::ModuleLocation;
    use mg_module::new::new_module;
    let Some(root) = mg_testkit::nwn_root() else { return };
    let Some(editor) = true_program() else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut m = new_module(&game, "Alone", &mut fastrand::Rng::with_seed(7)).unwrap();
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    m.set(key, b"void main()\n{\n}\n".to_vec());
    let base = mg_testkit::scratch_dir("ui-external-alone");
    let (project, archive) = (base.join("project"), base.join("alone.mod"));
    m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
        .unwrap();
    for (module, tab_opens) in [(&project, false), (&archive, true)] {
        if tab_opens {
            let mut m = Module::open(&project).unwrap();
            m.save_as(&ModuleLocation::Archive(archive.clone())).unwrap();
        }
        let mut app = Moonglow::new(Some(install.clone()), Box::new(NoDialogs::default()));
        app.settings.no_last_area = true;
        app.settings.external_editor = Some(editor.clone().into());
        app.settings.scripts_external = true;
        app.settings.scripts_external_only = true;
        app.open_module(module);
        let mut h = harness(app, 1100.0, 800.0);
        h.run_steps(3);
        h.state_mut().actions.push(mg_ui::Action::OpenResource(key));
        h.run_steps(4);
        let said = h
            .state()
            .log
            .entries
            .iter()
            .filter(|(_, m)| m.contains(&format!("in {editor}")))
            .count();
        assert_eq!(said, 1, "{:?}", h.state().log.entries);
        assert_eq!(h.state().dock.find_tab(&Tab::Script(key)).is_some(), tab_opens, "{module:?}");
        if tab_opens {
            continue;
        }
        // A new script has no file yet (File › Save writes it): it opens
        // in Moonglow's editor, and the external editor gets a copy.
        h.state_mut().new_script = Some("fresh".into());
        h.run_steps(2);
        h.get_by_label("Create").click();
        h.run_steps(4);
        let fresh = ResKey::parse("fresh", ResType::NSS).unwrap();
        assert!(h.state().dock.find_tab(&Tab::Script(fresh)).is_some());
        let copy = format!("fresh.nss: editing in {editor}");
        let log = &h.state().log.entries;
        assert!(log.iter().any(|(_, m)| *m == copy), "{log:?}");
    }
}

/// Edit › Builder Code…: the code is written to the builder's own file
/// (out of version control), and the ResRefs the wizards, a copy and the
/// Area Wizard propose next begin with it (The Domains of Dread's builder
/// proposal, C1).
#[test]
fn a_builder_code_set_in_its_window_begins_the_resrefs_proposed() {
    use mg_module::palette::BlueprintKind;
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = project_folder("ui-builder-code", &[]);
    let mut app = game_app(&root, None);
    app.open_module(&dir);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    h.run();
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label("Builder Code…").click();
    h.run();
    assert!(
        h.query_by_label("None: ResRefs are made from names alone, as Aurora makes them.")
            .is_some()
    );
    type_into_hint(&mut h, "ab", "AB");
    assert!(
        h.query_by_label(
            "A new area named Market: ab_market. A copy of nw_chicken: ab_nw_chicken001."
        )
        .is_some()
    );
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert!(h.state().builder_code.is_none(), "closed");
    let user = std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap();
    assert_eq!(user, "[builder]\ncode = \"ab\"\n");
    assert!(dir.join(".moonglow/.gitignore").is_file());
    let said =
        |h: &Harness<'_, Moonglow>, s: &str| h.state().log.entries.iter().any(|e| e.1.contains(s));
    assert!(said(&h, "Your builder code is ab"), "{:?}", h.state().log.entries);
    let module = &h.state().ws.as_ref().unwrap().module;
    assert_eq!(module.project_file.user.builder_code.as_deref(), Some("ab"), "read at once");

    // A blueprint wizard's.
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Store));
    h.run();
    h.get_by_label("Merchants").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    let hint =
        |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("ab_merchants001");
    assert!(h.query_all(egui_kittest::kittest::by().predicate(hint)).next().is_some(), "the hint");
    h.get_by_label("Finish").click();
    h.run();
    assert!(module_gff(&mut h, "ab_merchants001", ResType::UTM).is_some(), "as the hint said");

    // A copy's.
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(hello));
    h.run();
    assert_eq!(h.state().copy_as.as_ref().map(|c| c.resref.as_str()), Some("ab_hello001"));
    h.state_mut().copy_as = None;

    // The Area Wizard's, which shows the ResRef where the code makes it.
    h.state_mut().actions.push(mg_ui::Action::AreaWizard);
    h.run();
    assert!(h.query_by_label("ResRef: ab_area002").is_some());
    h.get_by_label("Castle Interior").click();
    h.run();
    h.get_by_label("Small").click();
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(ws.module.contains(&ResKey::parse("ab_area002", ResType::ARE).unwrap()));

    // No code: as Aurora again, and the rest of the file kept.
    std::fs::write(dir.join(".moonglow/user.toml"), "# mine\n[builder]\ncode = \"ab\"\n").unwrap();
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label("Builder Code…").click();
    h.run();
    h.get_by_label("No Code").click();
    h.run();
    let user = std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap();
    assert_eq!(user, "# mine\n[builder]\n");
    assert_eq!(h.state().ws.as_ref().unwrap().module.project_file.naming(), Default::default());
}

/// The builder code where a new resource's name is typed (The Domains of
/// Dread's builder proposal, C1): New Script…, New Conversation… and New
/// NUI Window… start with it, the cursor where the name goes (before a
/// code that follows the name), Add Popup Text…'s conversation too, and
/// an imported story's conversation has it.
#[test]
fn the_names_typed_for_new_resources_start_with_the_builder_code() {
    let dir = project_folder("ui-code-new-names", &[]);
    mg_module::project_file::ProjectFile::set_builder_code(&dir, Some("ab")).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    let mut h = harness(app, 1000.0, 800.0);
    h.run();
    let tools = |h: &mut Harness<'_, Moonglow>, item: &str| {
        h.get_by_label("Tools").click();
        h.run();
        h.get_by_label_contains(item).click();
        h.run();
    };
    tools(&mut h, "New Script…");
    assert_eq!(h.state().new_script.as_deref(), Some("ab_"));
    typed_name(&h, "ab_").type_text("gate");
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(ws.module.contains(&ResKey::parse("ab_gate", ResType::NSS).unwrap()));
    tools(&mut h, "New Conversation…");
    assert_eq!(h.state().new_dialog.as_deref(), Some("ab_"));
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    tools(&mut h, "New NUI Window…");
    assert!(h.query_by_label("New NUI Window").is_some());
    typed_name(&h, "ab_");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    let start = ResRef::from_str("start").unwrap();
    assert_eq!(mg_ui::store_wizard::PopupText::of(h.state(), start, 0).name, "ab_");
    // A story's conversation: named after its file, with the code (never
    // twice), the next free after it.
    let ink = dir.join("Night Watch.ink");
    std::fs::write(&ink, "-> gate\n=== gate ===\nWho goes there?\n+ [A friend.] -> END\n").unwrap();
    let first = h.state_mut().import_conversation(&ink).expect("imported");
    h.run();
    assert_eq!(first.resref.to_string(), "ab_night_watch");
    let again = h.state_mut().import_conversation(&ink).expect("imported");
    assert_eq!(again.resref.to_string(), "ab_night_watc001", "the name cut for the number");
    let ab = dir.join("ab_lantern.ink");
    std::fs::write(&ab, "-> gate\n=== gate ===\nA lantern.\n+ [Light it.] -> END\n").unwrap();
    let own = h.state_mut().import_conversation(&ab).expect("imported");
    assert_eq!(own.resref.to_string(), "ab_lantern");

    // A pattern with the code after the name: the cursor before it.
    std::fs::write(dir.join("moonglow.toml"), "[resrefs]\npattern = \"{name}_{code}\"\n").unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    let mut h = harness(app, 1000.0, 800.0);
    h.run();
    tools(&mut h, "New Conversation…");
    assert_eq!(h.state().new_dialog.as_deref(), Some("_ab"));
    h.run();
    typed_name(&h, "_ab").type_text("gate");
    h.run();
    assert_eq!(h.state().new_dialog.as_deref(), Some("gate_ab"));
    h.get_by_label("Create").click();
    h.run();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(ws.module.contains(&ResKey::parse("gate_ab", ResType::DLG).unwrap()));
}

/// A ResRef typed in a wizard is taken as typed, with a button that puts
/// the builder code in where it lacks it; the Creature Wizard's, where
/// the module has it, becomes the next free from it with the code (C1).
#[test]
fn a_resref_typed_in_a_wizard_is_offered_the_builder_code() {
    use mg_module::palette::BlueprintKind;
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = project_folder("ui-code-typed", &[]);
    mg_module::project_file::ProjectFile::set_builder_code(&dir, Some("ab")).unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&dir);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 900.0);
    h.run();
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Store));
    h.run();
    h.get_by_label("Merchants").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    type_into_hint(&mut h, "ab_merchants001", "shop");
    h.run();
    h.get_by_label("Use ab_shop").click();
    h.run();
    assert_eq!(h.state().blueprint_wizard.as_ref().unwrap().resref, "ab_shop");
    assert!(h.query_by_label("Use ab_shop").is_none(), "it has the code now");
    h.get_by_label("Finish").click();
    h.run();
    assert!(module_gff(&mut h, "ab_shop", ResType::UTM).is_some());

    // The Creature Wizard: a ResRef the module has.
    let guard = ResKey::parse("guard", ResType::UTC).unwrap();
    let data = mg_gff::Gff::new(*b"UTC ").to_bytes().unwrap();
    let made = mg_edit::Edit::SetResource { key: guard, data: Some(data) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("guard", vec![made])));
    h.run();
    h.get_by_label("Wizards").click();
    h.run();
    h.get_by_label_contains("Creature Wizard…").click();
    h.run();
    let next = |h: &mut Harness<'_, Moonglow>| {
        h.get_by_label("Next >").click();
        h.run();
    };
    next(&mut h);
    h.get_by_label("Human").click();
    h.run();
    for _ in 0..4 {
        next(&mut h);
    }
    let w = h.state_mut().creature_wizard.as_mut().unwrap();
    (w.first_name, w.resref) = ("Hent".into(), "guard".into());
    h.run();
    assert!(h.query_by_label("Use ab_guard").is_some());
    next(&mut h);
    h.get_by_label("Tutorial").click();
    h.run();
    next(&mut h);
    next(&mut h);
    h.query_all_by_label("Finish")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .unwrap()
        .click();
    h.run();
    assert!(module_gff(&mut h, "ab_guard", ResType::UTC).is_some(), "the next free, with the code");
}

/// A team's own prefix, without a builder's code and by kind (`[resrefs]
/// pattern = "tdod_{kind}{name}"`): the wizards and a copy propose it for
/// everyone, and the Builder Code window says so.
#[test]
fn a_team_s_prefix_by_kind_begins_the_resrefs_proposed() {
    use mg_module::palette::BlueprintKind;
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let rules = "[resrefs]\npattern = \"tdod_{kind}{name}\"\n\
                 kinds = { store = \"m_\", script = \"s_\" }\n";
    let dir = project_folder("ui-team-prefix", &[("moonglow.toml", rules)]);
    let mut app = game_app(&root, None);
    app.open_module(&dir);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    h.run();
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label("Builder Code…").click();
    h.run();
    h.get_by_label(
        "None: the team's pattern alone. A new area named Market: tdod_market. A copy of \
         nw_chicken: tdod_nw_chick001.",
    );
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    // A blueprint wizard's: the store's kind.
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Store));
    h.run();
    h.get_by_label("Merchants").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    let hint =
        |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("tdod_m_mercha001");
    assert!(h.query_all(egui_kittest::kittest::by().predicate(hint)).next().is_some(), "the hint");
    h.get_by_label("Finish").click();
    h.run();
    assert!(module_gff(&mut h, "tdod_m_mercha001", ResType::UTM).is_some());
    // A copy's: a script's kind.
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(hello));
    h.run();
    assert_eq!(h.state().copy_as.as_ref().map(|c| c.resref.as_str()), Some("tdod_s_hello001"));
}

/// A module kept in one file keeps its project file beside it, named by
/// it (`krezk.moonglow.toml`): read when it opens, and again when it
/// changes.
#[test]
fn a_mod_s_own_project_file_is_read_and_read_again() {
    let dir = mg_testkit::scratch_dir("ui-mod-project-file");
    let folder = sample_module(&dir);
    let path = dir.join("krezk.mod");
    Module::open(&folder).unwrap().save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    std::fs::write(dir.join("krezk.moonglow.toml"), "[resrefs]\npattern = \"kz_{name}\"\n")
        .unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    assert!(logged(&h, "The project's rules are read from krezk.moonglow.toml"));
    let naming = |h: &Harness<'_, Moonglow>| {
        h.state().ws.as_ref().unwrap().module.project_file.naming().fit("market", "")
    };
    assert_eq!(naming(&h), "kz_market");
    std::fs::write(dir.join("krezk.moonglow.toml"), "[resrefs]\npattern = \"krezk_{name}\"\n")
        .unwrap();
    h.state_mut().reload_changed_files();
    h.run();
    assert!(logged(&h, "krezk.moonglow.toml changed: its rules are read again"));
    assert_eq!(naming(&h), "krezk_market");
}

/// A module kept in one file has no project to keep a code in: the menu
/// item is there, and off.
#[test]
fn a_module_in_one_file_has_no_builder_code() {
    let dir = mg_testkit::scratch_dir("ui-builder-code-mod");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Edit").click();
    h.run();
    assert!(h.get_by_label("Builder Code…").accesskit_node().is_disabled());
}

const DESCRIBED: &str = "undescribed = [\"tdod_*\"]\n\n\
    [tdod_workbench]\ntype = \"string\"\non = [\"placeable\"]\n\
    values = { forge = \"Smithing\", tanner = \"Leatherworking\" }\n\
    help = \"The craft the bench serves, within 5 m\"\n\n\
    [tdod_lit]\ntype = \"int\"\nvalues = { 1 = \"lit\" }\nhelp = \"A fire: 1, lit\"\n\n\
    [tdod_banker]\ntype = \"int\"\non = [\"creature\"]\n\n\
    [tdod_trodden]\ntype = \"int\"\nruntime = true\n";

/// The Variables window offers the project's described variables
/// (`variables.toml`) as a name is typed, sets the type of the one
/// chosen, chooses its value from its values, and says what the
/// description finds wrong without keeping OK from writing it (C2).
#[test]
fn the_variables_window_offers_the_project_s_described_variables() {
    use mg_ui::widgets::{FieldTarget, VarTableEdit};
    let dir = project_folder("ui-described-variables", &[("variables.toml", DESCRIBED)]);
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    {
        let mut m = Module::open(&dir).unwrap();
        let mut g = Gff::new(*b"GIT ");
        g.root.set("Placeable List", mg_gff::Value::List(vec![mg_gff::Struct::new(9)]));
        m.set_gff(git, &g).unwrap();
        m.save().unwrap();
    }
    let mut app = app_with(Vec::new());
    app.open_module(&dir);
    assert!(
        app.log.entries.iter().any(|e| e.1.contains("4 variables described in variables.toml"))
    );
    let path = mg_edit::GffPath::root().item("Placeable List", 0);
    app.var_edit = Some(VarTableEdit::new(FieldTarget::new(git, path, "VarTable"), &[]));
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    h.get_by_label("Add").click();
    h.run();
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Name");
    h.get(egui_kittest::kittest::by().predicate(hint)).click();
    h.run();
    // What goes on a placeable is offered, as it is typed.
    assert!(h.query_by_label("tdod_workbench").is_some() && h.query_by_label("tdod_lit").is_some());
    assert!(h.query_by_label("tdod_banker").is_none(), "a creature's");
    assert!(h.query_by_label("tdod_trodden").is_none(), "the scripts' own: never offered");
    h.get(egui_kittest::kittest::by().predicate(hint)).type_text("tdod_w");
    h.run();
    assert!(h.query_by_label("tdod_lit").is_none());
    h.get_by_label("tdod_workbench").click();
    h.run();
    h.run();
    let row =
        |h: &Harness<'_, Moonglow>, i: usize| h.state().var_edit.as_ref().unwrap().rows[i].clone();
    assert_eq!((row(&h, 0).name.as_str(), row(&h, 0).kind), ("tdod_workbench", 3), "a String");
    assert!(h.query_by_label("tdod_workbench").is_none(), "the offer closed");
    // Its value, from its values.
    h.get_by_value("(choose)").click();
    h.run();
    h.get_by_label("forge — Smithing").click();
    h.run();
    assert_eq!(row(&h, 0).value, "forge");
    // A name typed whole takes its type too; one changed after is said.
    h.get_by_label("Add").click();
    h.run();
    h.state_mut().var_edit.as_mut().unwrap().rows[1].name = "tdod_lit".into();
    h.run();
    assert_eq!(row(&h, 1).kind, 1, "an Int");
    h.state_mut().var_edit.as_mut().unwrap().rows[1].kind = 3;
    h.run();
    assert!(
        h.query_by_label(
            "tdod_lit is to be an Int (variables.toml:9): as String the scripts read nothing"
        )
        .is_some()
    );
    // One the scripts alone set, typed whole: said, its type not taken.
    h.state_mut().var_edit.as_mut().unwrap().rows[1].name = "tdod_trodden".into();
    h.run();
    assert_eq!(row(&h, 1).kind, 3, "as it was");
    assert!(
        h.query_by_label(
            "tdod_trodden is set by the scripts while the server runs (variables.toml:18): it is \
             never to be kept in a file"
        )
        .is_some()
    );
    h.state_mut().var_edit.as_mut().unwrap().rows[1].name = "tdod_wrokbench".into();
    h.run();
    assert!(
        h.query_by_label(
            "tdod_wrokbench is named as the project's variables are (tdod_*) and described \
             nowhere: tdod_workbench?"
        )
        .is_some()
    );
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert!(h.state().var_edit.is_none(), "written all the same");
    let ws = h.state_mut().ws.as_mut().unwrap();
    let doc = ws.doc(&git).unwrap();
    let vars = doc.root.list("Placeable List").unwrap()[0].list("VarTable").unwrap();
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].string("Value"), Some(&b"forge"[..]));
}

/// The Variables window says what a variable needs beside it and the
/// object lacks (`requires`, `[[rules]]`), until it is added; a name typed
/// as one with a default takes it as its value.
#[test]
fn the_variables_window_says_what_a_variable_needs_and_gives_defaults() {
    use mg_ui::widgets::{FieldTarget, VarTableEdit};
    let described = "[tdod_rental]\ntype = \"string\"\nrequires = [\"tdod_rental_door\"]\n\n\
        [tdod_rental_door]\ntype = \"string\"\n\n\
        [tdod_climb_dc]\ntype = \"int\"\ndefault = 15\n\n\
        [[rules]]\nvariable = \"tdod_kind\"\nis = \"merchant\"\nrequire = \"tdod_store\"\n";
    let dir = project_folder("ui-variables-needed", &[("variables.toml", described)]);
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    {
        let mut m = Module::open(&dir).unwrap();
        let mut g = Gff::new(*b"GIT ");
        g.root.set("Creature List", mg_gff::Value::List(vec![mg_gff::Struct::new(4)]));
        m.set_gff(git, &g).unwrap();
        m.save().unwrap();
    }
    let mut app = app_with(Vec::new());
    app.open_module(&dir);
    let path = mg_edit::GffPath::root().item("Creature List", 0);
    app.var_edit = Some(VarTableEdit::new(FieldTarget::new(git, path, "VarTable"), &[]));
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    let add = |h: &mut Harness<'_, Moonglow>, name: &str, value: Option<&str>| {
        h.get_by_label("Add").click();
        h.run();
        let rows = &mut h.state_mut().var_edit.as_mut().unwrap().rows;
        let i = rows.len() - 1;
        rows[i].name = name.into();
        if let Some(v) = value {
            rows[i].value = v.into();
        }
        h.run();
        h.run();
    };
    add(&mut h, "tdod_rental", Some("smithy"));
    let note = "tdod_rental needs tdod_rental_door beside it (variables.toml:1)";
    assert!(h.query_by_label(note).is_some());
    add(&mut h, "tdod_rental_door", Some("D_SMITHY"));
    assert!(h.query_by_label(note).is_none(), "said until it is added");
    add(&mut h, "tdod_kind", Some("merchant"));
    assert!(
        h.query_by_label(
            "tdod_kind is merchant: tdod_store is needed beside it (variables.toml:12)"
        )
        .is_some()
    );
    // A default, taken as the value of a row named so.
    add(&mut h, "tdod_climb_dc", None);
    let rows = &h.state().var_edit.as_ref().unwrap().rows;
    let dc = rows.iter().find(|r| r.name == "tdod_climb_dc").unwrap();
    assert_eq!((dc.kind, dc.value.as_str()), (1, "15"));
}

/// The Variables window's offer by the keys: Down and Up move in it,
/// Enter takes the name (not the window's OK) and Escape closes it (not
/// the window). A pattern is offered by what its names begin with, typed
/// on from there.
#[test]
fn the_variables_window_s_offer_takes_the_keys_and_offers_patterns() {
    use mg_ui::widgets::{FieldTarget, VarTableEdit};
    let described = "[tdod_workbench]\ntype = \"string\"\non = [\"placeable\"]\n\n\
        [tdod_lit]\ntype = \"int\"\n\n\
        [\"tdod_rental_*\"]\ntype = \"int\"\non = [\"placeable\"]\nhelp = \"A room's rent\"\n";
    let dir = project_folder("ui-offer-keys", &[("variables.toml", described)]);
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    {
        let mut m = Module::open(&dir).unwrap();
        let mut g = Gff::new(*b"GIT ");
        g.root.set("Placeable List", mg_gff::Value::List(vec![mg_gff::Struct::new(9)]));
        m.set_gff(git, &g).unwrap();
        m.save().unwrap();
    }
    let mut app = app_with(Vec::new());
    app.open_module(&dir);
    let path = mg_edit::GffPath::root().item("Placeable List", 0);
    app.var_edit = Some(VarTableEdit::new(FieldTarget::new(git, path, "VarTable"), &[]));
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    let row =
        |h: &Harness<'_, Moonglow>, i: usize| h.state().var_edit.as_ref().unwrap().rows[i].clone();
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Name");
    let name_field = |h: &Harness<'_, Moonglow>, _row: usize| {
        h.get_all(egui_kittest::kittest::by().predicate(hint)).last().unwrap().click();
    };
    h.get_by_label("Add").click();
    h.run();
    name_field(&h, 0);
    h.run();
    h.event(egui::Event::Text("tdod_".into()));
    h.run();
    // The names, then the pattern's beginning, with the pattern.
    assert!(h.query_by_label("tdod_workbench").is_some());
    assert!(h.query_by_label("tdod_lit").is_some());
    assert!(h.query_by_label("tdod_rental_").is_some(), "the pattern, by its beginning");
    assert!(h.query_by_label_contains("tdod_rental_*: ").is_some());
    // Down, Enter: the second, and the window stays open.
    h.key_press(egui::Key::ArrowDown);
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
    assert!(h.state().var_edit.is_some(), "Enter took the name, not the window's OK");
    assert_eq!(row(&h, 0).name, "tdod_lit");
    // A pattern taken by the keys, and typed on from.
    h.get_by_label("Add").click();
    h.run();
    name_field(&h, 1);
    h.run();
    h.event(egui::Event::Text("tdod_r".into()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
    assert_eq!(row(&h, 1).name, "tdod_rental_");
    h.event(egui::Event::Text("door".into()));
    h.run();
    assert_eq!(row(&h, 1).name, "tdod_rental_door", "typed on, at its end");
    assert_eq!(row(&h, 1).kind, 1, "an Int, as the pattern says");
    // Escape closes the offer, not the window.
    h.get_by_label("Add").click();
    h.run();
    name_field(&h, 2);
    h.run();
    h.event(egui::Event::Text("tdod_w".into()));
    h.run();
    assert!(h.query_by_label("tdod_workbench").is_some());
    h.key_press(egui::Key::Escape);
    h.run();
    h.run();
    assert!(h.state().var_edit.is_some(), "the window stays open");
    assert!(h.query_by_label("tdod_workbench").is_none(), "the offer closed");
    // With no offer, Escape is the window's.
    h.key_press(egui::Key::Escape);
    h.run();
    h.run();
    assert!(h.state().var_edit.is_none(), "closed");
}

/// Verify Module holds the module's variables to the project's
/// descriptions: a variable of another type is an error (C2).
#[test]
fn verify_holds_variables_to_their_descriptions() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = project_folder("ui-verify-described", &[("variables.toml", DESCRIBED)]);
    {
        let mut m = Module::open(&dir).unwrap();
        let mut v = mg_gff::Struct::new(0);
        v.set("Name", mg_gff::Value::String(b"tdod_lit".to_vec()));
        v.set("Type", mg_gff::Value::Dword(3));
        v.set("Value", mg_gff::Value::String(b"1".to_vec()));
        let mut p = mg_gff::Struct::new(9);
        p.set("Tag", mg_gff::Value::String(b"FIRE".to_vec()));
        p.set("VarTable", mg_gff::Value::List(vec![v]));
        let mut g = Gff::new(*b"GIT ");
        g.root.set("Placeable List", mg_gff::Value::List(vec![p]));
        m.set_gff(ResKey::parse("start", ResType::GIT).unwrap(), &g).unwrap();
        m.save().unwrap();
    }
    let mut app = game_app(&root, None);
    app.open_module(&dir);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Verify);
    h.run();
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Error
            && m.contains("tdod_lit is a String here, and variables.toml:9 says it is an Int: the scripts read 0")),
        "{log:#?}"
    );
}

fn area_name(h: &mut Harness<'_, Moonglow>) -> String {
    let are = module_gff(h, "start", ResType::ARE).unwrap();
    are.root
        .locstring("Name")
        .and_then(|l| l.text(Language::ENGLISH, Gender::Male).map(|t| t.to_string()))
        .unwrap_or_default()
}

fn logged(h: &Harness<'_, Moonglow>, s: &str) -> bool {
    h.state().log.entries.iter().any(|e| e.1.contains(s))
}

/// The Domains of Dread's builder proposal, C6: an area another builder
/// claimed is marked in the module tree, says so above it, and takes no
/// change where the project keeps others' areas read-only; where it warns
/// (the default), a save that would write a change to it asks first.
#[test]
fn an_area_another_builder_claimed_is_read_only_or_asked_about_before_a_save() {
    let (dir, app) = claimed_project(
        "ui-claims-others",
        "[claims]\nfile = \"claims.2da\"\nothers = \"read-only\"\n",
        &claims_table("cd", "claimed"),
    );
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    // The tree's mark: whose it is.
    h.get_by_label("🔒 cd");
    // Read-only: the change is refused, and said.
    h.state_mut().actions.push(rename_start("Market"));
    h.run();
    assert_eq!(area_name(&mut h), "");
    assert!(
        logged(
            &h,
            "start is claimed by cd since 2026-10-20, and the project keeps others' areas read-only"
        ),
        "{:?}",
        h.state().log.entries
    );
    assert!(!h.state().has_unsaved_work());
    // Its Properties say so above the pages.
    h.state_mut()
        .actions
        .push(mg_ui::Action::OpenTab(Tab::AreaProperties(ResRef::from_str("start").unwrap())));
    h.run();
    h.run();
    assert!(
        h.query_by_label(
            "🔒 start is claimed by cd since 2026-10-20: read-only here. Ask them to release it, \
             then claim it."
        )
        .is_some()
    );

    // Warned: the change is made, and the save asks.
    std::fs::write(dir.join("moonglow.toml"), "[claims]\nfile = \"claims.2da\"\n").unwrap();
    h.state_mut().reload_project_files();
    h.run();
    assert!(
        h.query_by_label(
            "🔒 start is claimed by cd since 2026-10-20: a save asks before it writes your \
             changes to it."
        )
        .is_some()
    );
    h.state_mut().actions.push(rename_start("Market"));
    h.run();
    assert_eq!(area_name(&mut h), "Market");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    h.run();
    h.get_by_label("Areas Not Yours");
    h.get_by_label("  •  start is claimed by cd since 2026-10-20");
    assert!(h.state().has_unsaved_work(), "not saved yet");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert!(h.state().has_unsaved_work());
    assert!(logged(&h, "Not saved: the areas not yours"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    h.run();
    h.get_by_label("Save Anyway").click();
    h.run();
    h.run();
    assert!(!h.state().has_unsaved_work(), "saved");
    let saved = Module::open(&dir).unwrap();
    let are = saved.gff(&ResKey::parse("start", ResType::ARE).unwrap()).unwrap().unwrap();
    assert_eq!(
        are.root.locstring("Name").unwrap().text(Language::ENGLISH, Gender::Male).as_deref(),
        Some("Market")
    );
    // Saved again with nothing changed in it: not asked.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(h.query_by_label("Areas Not Yours").is_none());
    // The table changed while the save waits (a pull, read again by the
    // folder's watch): the window's list follows it, and with none left
    // the save goes on.
    h.state_mut().actions.push(rename_start("Square"));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    h.run();
    h.get_by_label("  •  start is claimed by cd since 2026-10-20");
    std::fs::write(dir.join("claims.2da"), claims_table("ef", "claimed")).unwrap();
    h.state_mut().reload_project_files();
    h.run();
    h.get_by_label("  •  start is claimed by ef since 2026-10-20");
    assert!(h.state().has_unsaved_work());
    std::fs::write(dir.join("claims.2da"), claims_table("ab", "claimed")).unwrap();
    h.state_mut().reload_project_files();
    h.run();
    h.run();
    assert!(h.query_by_label("Areas Not Yours").is_none());
    assert!(logged(&h, "The claims table changed: the save writes no area not yours now"));
    assert!(!h.state().has_unsaved_work(), "saved");
}

/// A builder code chosen that holds another's claims in the team's table
/// is said in the Builder Code window, and in the log once set; the
/// builder's own code, or a free one, isn't.
#[test]
fn a_builder_code_that_holds_claims_is_said_in_its_window() {
    let (dir, app) = claimed_project(
        "ui-claims-code",
        "[claims]\nfile = \"claims.2da\"\n",
        &claims_table("cd", "claimed"),
    );
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    let said = "cd holds claims in claims.2da (start): if that is another builder's code, choose \
                your own, or the two of you share those areas and the names of new resources";
    let draft = |code: &str| mg_ui::builder_code::Draft { code: code.into(), root: dir.clone() };
    for free in ["ab", "ef"] {
        h.state_mut().builder_code = Some(draft(free));
        h.run();
        assert!(h.query_by_label_contains("holds claims").is_none(), "{free}");
    }
    h.state_mut().builder_code = Some(draft("CD"));
    h.run();
    h.get_by_label(said);
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(logged(&h, said));
    assert!(logged(&h, "Your builder code is cd"));
}

/// C6: Edit › Claims Table… shows the table whole (every area of it and
/// the module, whose each is), and takes another builder's claim over
/// (asked first), hands this builder's on, releases and claims.
#[test]
fn the_claims_table_window_shows_every_area_and_hands_claims_on() {
    let table = format!(
        "{}1   kz_gone          ****     merged   2026-10-01  KZ_GONE\n",
        claims_table("cd", "claimed")
    );
    let (dir, app) =
        claimed_project("ui-claims-table", "[claims]\nfile = \"claims.2da\"\n", &table);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label("Claims Table…").click();
    h.run();
    h.get_by_label("claims.2da: whose each area is. Your code is ab.");
    h.get_by_label("cd since 2026-10-20");
    h.get_by_label("kz_gone");
    h.get_by_label("merged, not in the module");
    let row = |dir: &std::path::Path| {
        let text = std::fs::read_to_string(dir.join("claims.2da")).unwrap();
        text.lines().nth(3).unwrap().split_whitespace().take(3).collect::<Vec<_>>().join(" ")
    };
    // A free one: claimed, and released, from the window.
    h.get_by_label("Claim").click();
    h.run();
    let gone = |dir: &std::path::Path| std::fs::read_to_string(dir.join("claims.2da")).unwrap();
    assert!(gone(&dir).lines().nth(4).unwrap().starts_with("1   kz_gone          ab "));
    h.get_by_label("Release").click();
    h.run();
    assert!(gone(&dir).lines().nth(4).unwrap().starts_with("1   kz_gone          **** "));
    // Another's: taken over once asked.
    h.get_by_label("Take Over…").click();
    h.run();
    h.get_by_label_contains("start is claimed by cd since 2026-10-20. Taken over");
    h.get_by_label("Don't Take It").click();
    h.run();
    assert_eq!(row(&dir), "0 start cd");
    h.get_by_label("Take Over…").click();
    h.run();
    h.get_by_label("Take It Over").click();
    h.run();
    assert_eq!(row(&dir), "0 start ab");
    assert!(logged(&h, "start taken over from cd by ab, in claims.2da: commit it, and tell them"));
    h.get_by_label_contains("yours: ab since");
    // Handed on: not to one's own code.
    h.get_by_label("Hand To…").click();
    h.run();
    type_into_hint(&mut h, "their code", "AB");
    h.get_by_label("That is your own code.");
    assert!(h.get_by_label("Hand Over").accesskit_node().is_disabled());
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Backspace);
    h.run();
    type_into_hint(&mut h, "their code", "ef");
    h.get_by_label("Hand Over").click();
    h.run();
    assert_eq!(row(&dir), "0 start ef");
    assert!(logged(&h, "start handed to ef by ab, in claims.2da: commit it, and tell them"));
    // The window follows the table: another's again.
    h.get_by_label_contains("ef since");
    assert!(h.query_by_label("Hand To…").is_none());
    h.get_by_label("Close").click();
    h.run();
    assert!(h.query_by_label("Claims Table").is_none());
}

/// C6: Claim This Area (the module tree's menu) writes the builder's row
/// into the claims table, which is read again at once; Release Claim lets
/// it go; without a builder code, Claim is off.
#[test]
fn claim_this_area_and_release_claim_edit_the_claims_table() {
    let (dir, app) = claimed_project(
        "ui-claims-claim",
        "[claims]\nfile = \"builders/claims.2da\"\nunclaimed = \"warn\"\n",
        "",
    );
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    assert!(logged(&h, "the claims table builders/claims.2da is not there"));
    h.get_by_label("start").click_secondary();
    h.run();
    h.get_by_label("Claim This Area").click();
    h.run();
    h.run();
    let table = std::fs::read_to_string(dir.join("builders/claims.2da")).unwrap();
    let row = table.lines().nth(3).unwrap();
    assert!(row.starts_with("0   start            ab       claimed  "), "{table}");
    assert!(logged(&h, "start claimed by ab, in builders/claims.2da: commit it"));
    h.get_by_label("✔ ab");
    // An edit of it is the builder's own: no question at the save.
    h.state_mut().actions.push(rename_start("Market"));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    h.run();
    assert!(h.query_by_label("Areas Not Yours").is_none());
    assert!(!h.state().has_unsaved_work());
    // Released from the same menu.
    h.get_by_label("start").click_secondary();
    h.run();
    h.get_by_label("Release Claim").click();
    h.run();
    h.run();
    let table = std::fs::read_to_string(dir.join("builders/claims.2da")).unwrap();
    assert!(table.lines().nth(3).unwrap().starts_with("0   start            ****     free     "));
    assert!(h.query_by_label("✔ ab").is_none());
    // Unclaimed, in a project that warns of those: said above it, and a
    // save of a change to it asks.
    h.state_mut().actions.push(rename_start("Square"));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    h.run();
    h.get_by_label("  •  start is claimed by no one");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    // No code: Claim is there, and off.
    std::fs::write(dir.join(".moonglow/user.toml"), "").unwrap();
    h.state_mut().reload_project_files();
    h.run();
    h.get_by_label("start").click_secondary();
    h.run();
    assert!(h.get_by_label("Claim This Area").accesskit_node().is_disabled());
}

/// C7: a merged area's Tag (or ResRef) about to change is asked about
/// first, with a copy offered instead; changed anyway, it is changed.
#[test]
fn a_merged_area_s_tag_and_resref_are_asked_about_before_they_change() {
    let (_dir, app) = claimed_project(
        "ui-claims-protected",
        "[claims]\nfile = \"claims.2da\"\n",
        &claims_table("ab", "merged"),
    );
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    h.get_by_label("✔ ab 📌");
    let key = ResKey::parse("start", ResType::ARE).unwrap();
    let retag = |tag: &str| {
        mg_ui::Action::Apply(mg_edit::Command::new(
            "Tag",
            vec![mg_edit::Edit::SetField {
                key,
                path: mg_edit::GffPath::root(),
                label: "Tag".into(),
                value: Some(mg_gff::Value::String(tag.as_bytes().to_vec())),
            }],
        ))
    };
    let tag = |h: &mut Harness<'_, Moonglow>| {
        let are = module_gff(h, "start", ResType::ARE).unwrap();
        String::from_utf8_lossy(are.root.string("Tag").unwrap_or_default()).into_owned()
    };
    h.state_mut().actions.push(retag("START_2"));
    h.run();
    h.run();
    assert_eq!(tag(&mut h), "", "held");
    h.get_by_label("Protected Area");
    h.get_by_label("start is merged (in the claims table).");
    assert!(!logged(&h, "not changed yet"), "the window says it, not the log");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert_eq!(tag(&mut h), "");
    // A copy instead.
    h.state_mut().actions.push(retag("START_2"));
    h.run();
    h.run();
    h.get_by_label("Make a Copy Instead…").click();
    h.run();
    assert!(h.state().copy_as.is_some(), "the Copy window");
    h.state_mut().copy_as = None;
    // Changed anyway.
    h.state_mut().actions.push(retag("START_2"));
    h.run();
    h.run();
    h.get_by_label("Change It Anyway").click();
    h.run();
    h.run();
    assert_eq!(tag(&mut h), "START_2");
    // A Name, or the Tag set to what it is: nothing asked.
    h.state_mut().actions.push(rename_start("Market"));
    h.state_mut().actions.push(retag("START_2"));
    h.run();
    h.run();
    assert!(h.query_by_label("Protected Area").is_none());
    assert_eq!(area_name(&mut h), "Market");
    // Rename…: asked before its window opens.
    h.get_by_label("start").click_secondary();
    h.run();
    h.get_by_label("Rename…").click();
    h.run();
    h.run();
    h.get_by_label("Protected Area");
    assert!(h.state().rename.is_none());
    h.get_by_label("Rename It Anyway").click();
    h.run();
    h.run();
    assert!(h.state().rename.is_some(), "the Rename window, once said");
    // The pass is the rename's: a change of the Tag meanwhile is asked.
    h.state_mut().actions.push(retag("START_3"));
    h.run();
    h.run();
    h.get_by_label("Protected Area");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert_eq!(tag(&mut h), "START_2");
    // The Rename window closed without renaming: the next Rename… asks
    // again, and so does a change of the Tag.
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    h.run();
    assert!(h.state().rename.is_none());
    h.state_mut().actions.push(mg_ui::Action::RenameDialog(key));
    h.run();
    h.run();
    h.get_by_label("Protected Area");
    assert!(h.state().rename.is_none(), "asked again");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    h.state_mut().actions.push(retag("START_3"));
    h.run();
    h.run();
    h.get_by_label("Protected Area");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    // Delete It Anyway, and its window canceled: the next Delete… asks
    // again.
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(key));
    h.run();
    h.run();
    h.get_by_label("Delete It Anyway").click();
    h.run();
    h.run();
    assert_eq!(h.state().confirm_delete, Some(vec![key]));
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    h.run();
    assert_eq!(h.state().confirm_delete, None);
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(key));
    h.run();
    h.run();
    h.get_by_label("Protected Area");
    assert_eq!(h.state().confirm_delete, None, "asked again");
    // Deleted anyway: the pass lasts until the window's Delete is done.
    h.get_by_label("Delete It Anyway").click();
    h.run();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    h.run();
    assert!(h.query_by_label("Protected Area").is_none());
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&key), "deleted");
}

/// The project's commands (C8): File › Project › Update… saves the
/// module first (asked, as on closing), asks to allow the command as it
/// reads, runs it in the project's folder with its output in the log,
/// and reads the module's files again; allowed, it runs without asking
/// until its text changes.
#[test]
fn a_project_s_command_is_allowed_once_and_run_after_a_save() {
    let (update, fails) = if cfg!(windows) {
        ("echo void main() {}> made.nss && echo said", "exit 3")
    } else {
        ("printf 'void main() {}\\n' > made.nss && echo said && echo warned 1>&2", "exit 3")
    };
    let rules = format!(
        "[commands]\nupdate = {update:?}\nsubmit = {{ run = {fails:?}, label = \"Submit for Review\" }}\n"
    );
    let dir = project_folder("ui-project-commands", &[("moonglow.toml", &rules)]);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    h.state_mut().actions.push(rename_start("Market"));
    h.run();
    assert!(h.state().has_unsaved_work());
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Project").click();
    h.run();
    h.get_by_label("Update…").click();
    h.run();
    h.run();
    // Saved first, as on closing.
    h.get_by_label("Save").click();
    h.run();
    h.run();
    assert!(!h.state().has_unsaved_work(), "saved before it runs");
    // Asked: the text shown.
    h.get_by_label("Run a Project Command?");
    h.get_by_label(update);
    assert!(!dir.join("made.nss").exists(), "not run before it is allowed");
    h.get_by_label("Allow and Run").click();
    h.run();
    h.run();
    assert!(logged(&h, "Update: done"), "{:?}", h.state().log.entries);
    assert!(logged(&h, "  said"));
    // Kept in Moonglow's settings, by the folder's canonical path, never
    // in the project's files.
    let project = std::fs::canonicalize(&dir).unwrap().display().to_string();
    assert_eq!(
        h.state().settings.allowed_commands,
        [(project, "update".to_string(), update.to_string())]
    );
    assert!(!dir.join(".moonglow").exists(), "nothing written into the project");
    // What it made is read into the module (as files changed outside are).
    let made = ResKey::parse("made", ResType::NSS).unwrap();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&made), "{:?}", h.state().log.entries);
    // Allowed: it runs again without asking.
    std::fs::remove_file(dir.join("made.nss")).unwrap();
    h.state_mut().reload_project_files();
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("update".into()));
    h.run();
    h.run();
    assert!(h.query_by_label("Run a Project Command?").is_none());
    assert!(dir.join("made.nss").exists());
    // Another, which fails: its exit code said.
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("submit".into()));
    h.run();
    h.run();
    h.get_by_label("Allow and Run").click();
    h.run();
    h.run();
    assert!(logged(&h, "Submit for Review failed: it ended with exit code 3"));
    // Its text changed: asked again.
    std::fs::write(dir.join("moonglow.toml"), rules.replace("exit 3", "exit 4")).unwrap();
    h.state_mut().reload_project_files();
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("submit".into()));
    h.run();
    h.run();
    h.get_by_label("Run a Project Command?");
    h.get_by_label("exit 4");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert!(!logged(&h, "exit code 4"), "not run");
}

/// A trust decision is never read from the checkout: a repository that
/// committed a `.moonglow/user.toml` allowing its own command is still
/// asked about it (and the log says the file's `[allowed_commands]` is not
/// read); one allowed for another folder is asked about here too.
#[test]
fn commands_allowed_by_a_committed_user_file_are_still_asked_about() {
    let rules = "[commands]\nupdate = \"echo said\"\n";
    let dir = project_folder("ui-project-commands-committed", &[("moonglow.toml", rules)]);
    mg_module::project_file::ProjectFile::write_user(
        &dir,
        "[builder]\ncode = \"ab\"\n\n[allowed_commands]\nupdate = \"echo said\"\n",
    )
    .unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    // (Allowed, as it reads, for another project's folder.)
    let elsewhere = mg_testkit::scratch_dir("ui-project-commands-elsewhere");
    let elsewhere = std::fs::canonicalize(elsewhere).unwrap().display().to_string();
    app.settings.allowed_commands.push((elsewhere, "update".into(), "echo said".into()));
    app.open_module(&dir);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    assert!(logged(&h, "[allowed_commands] is not read"), "{:?}", h.state().log.entries);
    assert_eq!(
        h.state().ws.as_ref().unwrap().module.project_file.user.builder_code.as_deref(),
        Some("ab"),
        "the builder code is still read from it"
    );
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("update".into()));
    h.run();
    h.run();
    h.get_by_label("Run a Project Command?");
    assert!(!logged(&h, "Update: done"), "not run unasked");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert!(!logged(&h, "Update: done"));
}

/// The project's commands are in the Command Palette and take keys as any
/// command does: their own (`key`) at first, those chosen in Options ›
/// Keyboard after; each runs as from File › Project (saved first, asked
/// about as it reads).
#[test]
fn a_project_s_command_is_in_the_palette_and_takes_keys() {
    let rules = "[commands]\nupdate = { run = \"echo updated\", key = \"Ctrl+Shift+U\" }\n\
                 submit = { run = \"echo submitted\", label = \"Submit for Review\" }\n";
    let dir = project_folder("ui-project-commands-keys", &[("moonglow.toml", rules)]);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    // The palette finds them, under Project, and runs one.
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
    h.run();
    type_into_hint(&mut h, "Type a command's name", "submit");
    h.get_by_label("Submit for Review…").click();
    h.run();
    h.run();
    h.get_by_label("Run a Project Command?");
    h.get_by_label("echo submitted");
    h.get_by_label("Allow and Run").click();
    h.run();
    h.run();
    assert!(logged(&h, "Submit for Review: done"), "{:?}", h.state().log.entries);
    // Its own key, shown in File › Project.
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Project").click();
    h.run();
    let menu = h.get_by_label_contains("Update…").accesskit_node().label().unwrap_or_default();
    assert!(menu.contains("Ctrl+Shift+U") || menu.contains("⇧⌘U"), "{menu}");
    h.key_press(egui::Key::Escape);
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::U);
    h.run();
    h.run();
    h.get_by_label("Run a Project Command?");
    h.get_by_label("echo updated");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    // Options › Keyboard lists them, and another key chosen there runs it.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Keyboard").click();
    h.run();
    type_into_hint(&mut h, "Find a command", "project");
    h.get_by_label("Submit for Review");
    h.state_mut().options.as_mut().unwrap().recording = Some("project:submit".to_string());
    h.run();
    h.key_press(egui::Key::F8);
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert_eq!(
        h.state().settings.key_bindings.get("project:submit"),
        Some(&vec!["F8".to_string()])
    );
    h.key_press(egui::Key::F8);
    h.run();
    h.run();
    assert_eq!(
        h.state().log.entries.iter().filter(|(_, m)| m == "Submit for Review: done").count(),
        2,
        "F8 runs it (allowed: without asking)"
    );
}

/// A command that writes none of the module's files (`writes = false`, a
/// submit) runs with the module's haks open (the open areas aren't drawn
/// again after); any other closes them while it runs, so that a checkout
/// may write over them. Seen from the command: the haks Moonglow has
/// holds open (Linux's `/proc`).
#[cfg(target_os = "linux")]
#[test]
fn a_project_s_command_that_writes_nothing_keeps_the_haks_open() {
    let root = mg_testkit::corpus!();
    let rules = "[commands]\n\
                 submit = { run = \"ls -l /proc/$PPID/fd | grep -c mg_cmd_writes.hak; true\", writes = false }\n\
                 update = \"ls -l /proc/$PPID/fd | grep -c mg_cmd_writes.hak; true\"\n";
    let dir = project_folder("ui-project-commands-writes", &[("moonglow.toml", rules)]);
    let user = dir.parent().unwrap().join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
    let table = b"2DA V2.0\n\n   Label\n0  one\n".to_vec();
    hak.add(mg_core::ResRef::from_str("mg_cmd").unwrap(), ResType::TWODA, table).unwrap();
    std::fs::write(user.join("hak/mg_cmd_writes.hak"), hak.to_bytes().unwrap()).unwrap();
    let mut m = Module::open(&dir).unwrap();
    let mut ifo = m.info().unwrap();
    let mut listed = mg_gff::Struct::new(8);
    listed.set("Mod_Hak", mg_gff::Value::String(b"mg_cmd_writes".to_vec()));
    ifo.root.set("Mod_HakList", mg_gff::Value::List(vec![listed]));
    m.set_info(&ifo).unwrap();
    m.save().unwrap();
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    let project = std::fs::canonicalize(&dir).unwrap().display().to_string();
    for (name, text) in [
        ("submit", "ls -l /proc/$PPID/fd | grep -c mg_cmd_writes.hak; true"),
        ("update", "ls -l /proc/$PPID/fd | grep -c mg_cmd_writes.hak; true"),
    ] {
        app.settings.allowed_commands.push((project.clone(), name.into(), text.into()));
    }
    app.open_module(&dir);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    let mapped = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        h.state()
            .log
            .entries
            .iter()
            .filter_map(|(_, m)| m.strip_prefix("  "))
            .map(String::from)
            .collect()
    };
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("submit".into()));
    h.run();
    h.run();
    assert!(logged(&h, "Submit: done"), "{:?}", h.state().log.entries);
    let seen = mapped(&h);
    assert!(
        seen.last().is_some_and(|n| n != "0"),
        "the hak open while it ran: {seen:?} {:?}",
        h.state().log.entries
    );
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("update".into()));
    h.run();
    h.run();
    assert!(logged(&h, "Update: done"), "{:?}", h.state().log.entries);
    assert_eq!(mapped(&h).last().map(String::as_str), Some("0"), "closed while it ran");
    // (And open again after.)
    let ws = h.state().ws.as_ref().unwrap();
    assert_eq!(ws.module.haks().unwrap(), ["mg_cmd_writes"]);
}

/// Frames until the job running is over (or a test has hung).
#[cfg(unix)]
fn until_idle(h: &mut Harness<'_, Moonglow>) {
    let started = std::time::Instant::now();
    while h.state().busy() {
        assert!(started.elapsed().as_secs() < 30, "the command never ended");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    h.run_steps(2);
}

/// A project's command's output is in a window of its own, as it comes
/// and after (File › Project › Command Output), not the log only; one that
/// reads what is typed (`input = true`) is given it from the line under
/// its output, its question shown before its line ends, a hidden line
/// kept out of the output. A command that doesn't has its input closed:
/// its question is answered with nothing at once, as before. (By `sh`:
/// `cmd /C` expands `%name%` as it reads the line, before `set /p` has
/// set it, so the same on Windows needs a script of its own.)
#[cfg(unix)]
#[test]
fn a_project_s_command_shows_its_output_and_takes_what_is_typed() {
    let ask = "printf 'Name: '; read name; printf 'Secret: '; read secret; echo \"got:$name\"; \
               echo \"kept:$secret\"";
    let closed = "read x; echo \"empty:$x\"; echo \"warned\" 1>&2; exit 2";
    let rules =
        format!("[commands]\nask = {{ run = {ask:?}, input = true }}\nclosed = {closed:?}\n");
    let dir = project_folder("ui-project-commands-input", &[("moonglow.toml", &rules)]);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.background_jobs = true;
    let project = std::fs::canonicalize(&dir).unwrap().display().to_string();
    for (name, text) in [("ask", ask), ("closed", closed)] {
        app.settings.allowed_commands.push((project.clone(), name.into(), text.into()));
    }
    app.open_module(&dir);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("ask".into()));
    h.run_steps(2);
    assert!(h.state().busy(), "running, waiting for its answer");
    // Its question shows before its line ends.
    let started = std::time::Instant::now();
    while h.query_by_label("Name: ").is_none() {
        assert!(started.elapsed().as_secs() < 20, "the question never showed");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    h.get_by_label("Send");
    h.get_by_label("End Input");
    // The line has the keys: typed, and Enter sends it.
    h.event(egui::Event::Text("Hipólito".into()));
    h.step();
    h.key_press(egui::Key::Enter);
    h.step();
    h.step();
    h.get_by_label("> Hipólito");
    // A hidden line, by Send.
    while h.query_by_label("Secret: ").is_none() {
        assert!(started.elapsed().as_secs() < 20, "the second question never showed");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    h.get_by_label("Hide").click();
    h.step();
    h.event(egui::Event::Text("swordfish".into()));
    h.step();
    h.get_by_label("Send").click();
    h.step();
    h.step();
    h.get_by_label("> (hidden)");
    until_idle(&mut h);
    assert!(logged(&h, "  got:Hipólito"), "{:?}", h.state().log.entries);
    assert!(logged(&h, "  kept:swordfish"), "the command had it");
    assert!(
        !logged(&h, "> Hipólito") && !logged(&h, "(hidden)"),
        "what was typed is not in the log: {:?}",
        h.state().log.entries
    );
    // Its output, after: File › Project › Command Output.
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Project").click();
    h.run();
    h.get_by_label("Command Output").click();
    h.run_steps(2);
    h.get_by_label("Done.");
    h.get_by_label("got:Hipólito");
    assert!(h.query_by_label("swordfish").is_none(), "a hidden line is not shown");
    // Its input closed: the question has its answer, empty, at once; its
    // errors and how it ended in the window.
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("closed".into()));
    h.run_steps(2);
    until_idle(&mut h);
    assert!(logged(&h, "  empty:"), "{:?}", h.state().log.entries);
    assert!(logged(&h, "Closed failed: it ended with exit code 2 (all it said: File › Project"));
    h.get_by_label("Failed: it ended with exit code 2.");
    h.get_by_label("empty:");
    h.get_by_label("warned");
    assert!(h.query_by_label("got:Hipólito").is_none(), "the last command's alone");
    h.get_by_label("Close").click();
    h.run();
    assert!(h.query_by_label("Command Output").is_none(), "closed");
}

/// A project's command canceled gives back what it let go: the project's
/// files are looked at again (what it wrote before it was stopped comes
/// into the module) and its haks opened again, as after one that ends.
/// (A canceled job hands nothing over: they stayed let go.)
#[cfg(unix)]
#[test]
fn a_project_s_command_canceled_reads_the_project_s_files_again() {
    let slow = "printf 'void main() {}\\n' > made.nss; sleep 30";
    let rules = format!("[commands]\nslow = {slow:?}\n");
    let dir = project_folder("ui-project-commands-cancel", &[("moonglow.toml", &rules)]);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.background_jobs = true;
    let project = std::fs::canonicalize(&dir).unwrap().display().to_string();
    app.settings.allowed_commands.push((project, "slow".into(), slow.into()));
    app.open_module(&dir);
    let mut h = harness(app, 1000.0, 700.0);
    // (The area read first, a job of its own.)
    until_idle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("slow".into()));
    h.step();
    let started = std::time::Instant::now();
    while !dir.join("made.nss").exists() {
        assert!(started.elapsed().as_secs() < 20, "the command never wrote");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    assert!(h.state().busy());
    // (The job's window shown, its fade in over.)
    h.run_steps(20);
    h.get_by_label("Slow");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run_steps(2);
    until_idle(&mut h);
    assert!(started.elapsed().as_secs() < 20, "stopped, not waited for");
    let made = ResKey::parse("made", ResType::NSS).unwrap();
    while !h.state().ws.as_ref().unwrap().module.contains(&made) {
        assert!(started.elapsed().as_secs() < 25, "the project's files never read again");
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.step();
    }
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Project").click();
    h.run();
    h.get_by_label("Command Output").click();
    h.run_steps(2);
    h.get_by_label("Canceled (stopped).");
}

/// File › Project is there and off for a project that names no commands.
#[test]
fn file_project_is_off_without_the_project_s_commands() {
    let dir = mg_testkit::scratch_dir("ui-project-commands-none");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("File").click();
    h.run();
    assert!(h.get_by_label("Project").accesskit_node().is_disabled());
}
