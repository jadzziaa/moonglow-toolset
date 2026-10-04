//! UI flows driven through the accessibility tree (egui_kittest), on a small
//! module built by the test; no game install needed.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use mg_schema::{ExoString, StructExt, ifo};
use mg_ui::{Moonglow, NoDialogs, Tab};

fn sample_module(dir: &std::path::Path) -> std::path::PathBuf {
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("SAMPLE"));
    info.root.write(&ifo::MOD_XP_SCALE, 10);
    info.root.write(
        &ifo::MOD_DESCRIPTION,
        LocString::from_text(Language::ENGLISH, Gender::Male, "Line one\r\nLine two"),
    );
    let mut a = ifo::MOD_AREA_LIST.new_item();
    a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str("start").unwrap());
    info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
    m.set_info(&info).unwrap();
    m.set(ResKey::parse("start", ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    let path = dir.join("sample.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

fn app_with(open: Vec<std::path::PathBuf>) -> Moonglow {
    Moonglow::new(None, Box::new(NoDialogs { open, ..Default::default() }))
}

#[test]
fn welcome_then_open_a_module_through_the_dialog() {
    let dir = mg_testkit::scratch_dir("ui-open");
    let path = sample_module(&dir);
    let mut h =
        Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(vec![path.clone()]));
    h.run();
    h.get_by_label("Open Module…").click();
    h.run();
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()));
    // The tree lists the area and the script; Module Properties is open.
    h.get_by_label("Areas (1)");
    h.get_by_label_contains("Scripts (1)");
    h.get_by_label("Module Properties");
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("Opened")));
}

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

#[test]
fn edit_undo_save_reopen() {
    let dir = mg_testkit::scratch_dir("ui-edit");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();

    // Change the tag through the text field (commits when focus leaves).
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value("SAMPLE").find(is_input).expect("the tag's text field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("SAMPLE").find(is_input).expect("the tag's text field").type_text("RENAMED");
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    let tag_now = |app: &mut Moonglow| {
        let ws = app.ws.as_mut().unwrap();
        let g = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
        String::from_utf8_lossy(g.root.read(&ifo::MOD_TAG).as_bytes()).into_owned()
    };
    assert_eq!(tag_now(h.state_mut()), "RENAMED");
    assert!(h.state().title().ends_with(" *"), "title shows unsaved changes");

    // Undo with the keyboard.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(tag_now(h.state_mut()), "SAMPLE");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run();
    assert_eq!(tag_now(h.state_mut()), "RENAMED");

    // Save and read the file back.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    assert!(!h.state().title().ends_with(" *"));
    let reopened = Module::open(&path).unwrap();
    let info = reopened.info().unwrap();
    assert_eq!(info.root.read(&ifo::MOD_TAG).as_bytes(), b"RENAMED");
    assert!(dir.join("sample.mod.bak").is_file());
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

#[test]
fn multi_line_text_keeps_its_line_ends() {
    let dir = mg_testkit::scratch_dir("ui-crlf");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Description").click();
    h.run();
    // Shown with plain line breaks in a multi-line box.
    let is_input = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput
    };
    h.get_all_by_value("Line one\nLine two")
        .find(is_input)
        .expect("the description's text box")
        .click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("Line one\nLine two")
        .find(is_input)
        .unwrap()
        .type_text("First\nSecond\nThird");
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let g = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
    let desc: LocString = g.root.read(&ifo::MOD_DESCRIPTION);
    assert_eq!(desc.get(Language::ENGLISH, Gender::Male), Some(&b"First\r\nSecond\r\nThird"[..]));
}

fn set_tag(app: &mut Moonglow, tag: &str) {
    let key = ResKey::parse("module", ResType::IFO).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Module tag",
        vec![mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "Mod_Tag".into(),
            value: Some(mg_gff::Value::String(tag.as_bytes().to_vec())),
        }],
    )));
}

#[test]
fn closing_with_unsaved_changes_asks_first() {
    let dir = mg_testkit::scratch_dir("ui-unsaved");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    set_tag(h.state_mut(), "EDITED");
    h.run();

    // Cancel keeps the module open.
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    h.get_by_label("Save changes to sample.mod?");
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().ws.is_some() && h.state().confirm_discard.is_none());

    // File › Open Folder… asks too (it opened the folder over the work).
    h.state_mut().actions.push(mg_ui::Action::OpenFolderDialog);
    h.run();
    h.get_by_label("Save changes to sample.mod?");
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().ws.is_some() && h.state().confirm_discard.is_none());

    // Save saves, then closes.
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert!(h.state().ws.is_none());
    assert_eq!(
        Module::open(&path).unwrap().info().unwrap().root.read(&ifo::MOD_TAG).as_bytes(),
        b"EDITED"
    );

    // Don't Save discards.
    h.state_mut().open_module(&path);
    set_tag(h.state_mut(), "DISCARDED");
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    h.get_by_label("Don't Save").click();
    h.run();
    assert!(h.state().ws.is_none());
    assert_eq!(
        Module::open(&path).unwrap().info().unwrap().root.read(&ifo::MOD_TAG).as_bytes(),
        b"EDITED"
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
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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

#[test]
fn test_from_here_plays_the_module_as_it_is_now() {
    let dir = mg_testkit::scratch_dir("ui-test-from-here");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    set_tag(&mut app, "UNSAVED");
    for a in std::mem::take(&mut app.actions) {
        app.run(a);
    }
    let start = ResRef::from_str("start").unwrap();
    let bytes =
        app.test_from_here_archive(start, [5.0, 6.5, 0.25], std::f32::consts::FRAC_PI_2).unwrap();
    let erf = mg_erf::Erf::read(&bytes).unwrap();
    let e = erf.entries.iter().find(|e| e.restype == ResType::IFO).unwrap();
    let info = Gff::read(&erf.data(e).unwrap()).unwrap();
    assert_eq!(info.root.read(&ifo::MOD_TAG).as_bytes(), b"UNSAVED");
    assert_eq!(info.root.read(&ifo::MOD_ENTRY_AREA), start);
    assert_eq!((info.root.read(&ifo::MOD_ENTRY_X), info.root.read(&ifo::MOD_ENTRY_Y)), (5.0, 6.5));
    assert!((info.root.read(&ifo::MOD_ENTRY_DIR_Y) - 1.0).abs() < 1e-6);
    // Nothing was saved.
    assert!(app.ws.as_ref().unwrap().is_modified());
    assert_eq!(
        Module::open(&path).unwrap().info().unwrap().root.read(&ifo::MOD_TAG).as_bytes(),
        b"SAMPLE"
    );
}

#[test]
fn reload_resources_picks_up_new_files_in_override() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-reload");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let key = ResKey::parse("mg_reload_test", ResType::TWODA).unwrap();
    assert!(!app.game.as_deref().unwrap().resman.contains(&key));
    std::fs::write(user.join("override/mg_reload_test.2da"), "2DA V2.0\n\n   Label\n0  x\n")
        .unwrap();
    app.run(mg_ui::Action::ReloadResources);
    assert!(app.game.as_deref().unwrap().resman.contains(&key), "{:?}", app.log.entries);
    assert!(app.log.entries.iter().any(|(_, m)| m == "Reloaded override"), "{:?}", app.log.entries);
    app.run(mg_ui::Action::ReloadResources);
    assert!(app.log.entries.iter().any(|(_, m)| m == "Reload Resources: nothing changed"));
}

#[test]
fn new_module_and_area_through_the_wizards() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-new-module");
    let path = dir.join("wizard.mod");
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let app = Moonglow::new(
        Some(install),
        Box::new(NoDialogs { save: vec![path.clone()], ..Default::default() }),
    );
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();

    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value("module000").find(is_input).expect("the name field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("module000").find(is_input).unwrap().type_text("Wizard Test");
    h.run();
    h.get_by_label("Create").click();
    h.run();

    // The Area Wizard follows, as in Aurora's Module Wizard, with the first
    // tileset chosen.
    h.run();
    let Some(mg_ui::wizards::Wizard::NewArea(w)) = &h.state().wizard else {
        panic!("the Area Wizard is open")
    };
    assert_eq!(w.selected, Some(0));
    h.get_by_label("Castle Interior").click();
    h.run();
    h.get_by_label("Small").click();
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("area001", ResType::ARE).unwrap();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key), "{:?}", h.state().log.entries);
    h.get_by_label_contains("Areas (1)");
    // Open Area in the Area Viewer is on by default, as in Aurora.
    assert!(
        h.state().dock.find_tab(&Tab::Area(key.resref)).is_some(),
        "the area opens in a viewer"
    );
    assert!(h.state().dock.find_tab(&Tab::AreaProperties(key.resref)).is_none());
    let area = mg_gff::Gff::read(h.state().ws.as_ref().unwrap().module.get(&key).unwrap()).unwrap();
    assert_eq!(area.root.read(&mg_schema::are::TILESET), ResRef::from_str("tic01").unwrap());
    assert_eq!(
        (area.root.read(&mg_schema::are::WIDTH), area.root.read(&mg_schema::are::HEIGHT)),
        (4, 4)
    );

    // Save asks where (a new module), then the file has the module.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()));
    let saved = Module::open(&path).unwrap();
    let info = saved.info().unwrap();
    assert_eq!(saved.areas().unwrap(), [ResRef::from_str("area001").unwrap()]);
    assert_eq!(info.root.read(&ifo::MOD_ENTRY_AREA), ResRef::from_str("area001").unwrap());
    assert!(saved.contains(&ResKey::parse("repute", ResType::FAC).unwrap()));

    // Creating the area is one undoable step.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    assert!(!ws.module.contains(&key));
    let info = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
    assert!(info.root.items(&ifo::MOD_AREA_LIST).is_empty());
}

#[test]
fn recent_modules_reopen_from_the_welcome_page() {
    let dir = mg_testkit::scratch_dir("ui-recent");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    assert_eq!(app.settings.recent, std::slice::from_ref(&path));
    app.close();
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("sample.mod").click();
    h.run();
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()));
}

#[test]
fn options_choose_the_game_folder() {
    let dir = mg_testkit::scratch_dir("ui-options");
    let app = Moonglow::new(
        None,
        Box::new(NoDialogs { folders: vec![dir.clone()], ..Default::default() }),
    );
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    // Browse picks a folder that is not a game install.
    h.get_all_by_label("Browse…").next().unwrap().click();
    h.run();
    h.get_by_label("Not a game installation (no data/nwn_base.key).");
    h.get_by_label("OK").click();
    h.run();
    assert_eq!(h.state().settings.game_root.as_deref(), Some(dir.as_path()));
    assert_eq!(h.state().install.as_ref().map(|i| i.root.clone()), Some(dir.clone()));
    assert!(h.state().game.is_none());
    assert!(
        h.state().log.entries.iter().any(|(_, m)| m.starts_with("Could not load the game data"))
    );
}

#[test]
fn export_then_import_into_another_module() {
    let dir = mg_testkit::scratch_dir("ui-transfer");
    let source = sample_module(&dir);
    let erf = dir.join("start.erf");
    let area = ResKey::parse("start", ResType::ARE).unwrap();

    // Export the area from the sample module.
    let mut app = Moonglow::new(
        None,
        Box::new(NoDialogs {
            save: vec![erf.clone()],
            open: vec![erf.clone()],
            ..Default::default()
        }),
    );
    app.open_module(&source);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::ExportDialog(vec![area]));
    h.run();
    h.get_by_label("Export 1…").click();
    h.run();
    let bytes = std::fs::read(&erf).unwrap();
    let archive = mg_erf::Erf::read(&bytes).unwrap();
    assert!(archive.entries.iter().any(|e| e.resref == area.resref && e.restype == ResType::ARE));

    // Import it into a module without areas.
    let target = dir.join("target.mod");
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("TARGET"));
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(target.clone())).unwrap();
    h.state_mut().open_module(&target);
    h.state_mut().actions.push(mg_ui::Action::ImportDialog);
    h.run();
    h.get_by_label("Import").click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    assert!(ws.module.contains(&area));
    assert_eq!(ws.module.areas().unwrap(), [area.resref]);
    // One undoable step.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&area));
}

#[test]
fn browse_view_and_copy_a_game_resource() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-browser");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.run();
    // The module's own script is listed from the module layer.
    h.state_mut().browser.filter = "hello".into();
    h.run();
    h.get_by_label("hello.nss");
    h.get_by_label("module");

    // A game blueprint: listed from the keys, viewed read-only, copied in.
    h.state_mut().browser.filter = "nw_it_torch001".into();
    h.run();
    h.get_by_label("nw_it_torch001.uti").click_secondary();
    h.run();
    h.get_by_label("Open").click();
    h.run();
    let key = ResKey::parse("nw_it_torch001", ResType::UTI).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Resource(key)).is_some());
    h.get_by_label("LocalizedName");
    h.get_by_label("Copy to Module").click();
    h.run();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key));
    assert!(
        h.state().dock.find_tab(&Tab::Blueprint(key)).is_some(),
        "the copy opens in its editor"
    );
}

#[test]
fn module_name_in_every_language_and_variables() {
    let dir = mg_testkit::scratch_dir("ui-loc-vars");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let info = |app: &mut Moonglow| {
        let ws = app.ws.as_mut().unwrap();
        ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap().root.clone()
    };

    // String Edit: add a German name.
    h.get_by_label("…").click();
    h.run();
    h.get_by_label("Add Text").click();
    h.run();
    let edit = h.state().loc_edit.clone().expect("String Edit is open");
    assert_eq!(edit.entries.len(), 1, "the sample name is empty; one new entry");
    {
        let e = h.state_mut().loc_edit.as_mut().unwrap();
        e.entries[0].0 = Language::GERMAN;
        e.entries[0].2 = "Beispiel".into();
        e.strref = "12".into();
    }
    h.run();
    h.get_by_label("OK").click();
    h.run();
    let name: LocString = info(h.state_mut()).read(&ifo::MOD_NAME);
    assert_eq!(name.get(Language::GERMAN, Gender::Male), Some(&b"Beispiel"[..]));
    assert_eq!(name.strref.0, 12);

    // Variables on the Advanced tab.
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("Variables (0)…").click();
    h.run();
    h.get_by_label("Add").click();
    h.run();
    {
        let v = h.state_mut().var_edit.as_mut().unwrap();
        v.rows[0].name = "nLevel".into();
        v.rows[0].value = "7".into();
    }
    h.run();
    // The name's field shows a whole name (32 characters), also in a
    // window just opened.
    let name_width = h
        .get_all_by_value("nLevel")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::TextInput)
        .expect("the name's field")
        .rect()
        .width();
    assert!(name_width >= 250.0, "the name's field is {name_width} wide");
    h.get_by_label("OK").click();
    h.run();
    let vars = info(h.state_mut()).items(&ifo::VAR_TABLE).to_vec();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].get("Value"), Some(&mg_gff::Value::Int(7)));
    // Each window's OK is one undoable step.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(info(h.state_mut()).items(&ifo::VAR_TABLE).is_empty());
}

#[test]
fn talk_table_made_edited_saved_and_used_by_strings() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-table");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run();
    h.get_by_label("This module has no talk table of its own.");
    type_into_hint(&mut h, "name", "mg_talk");
    h.get_by_label("With a feminine table").click();
    h.run();
    h.get_by_label("Create").click();
    h.run();
    h.run();
    // The table is made in the tlk folder and named by the module.
    assert!(user.join("tlk/mg_talk.tlk").is_file());
    assert!(user.join("tlk/mg_talkf.tlk").is_file());
    let named = |h: &mut Harness<'_, Moonglow>| {
        h.state().ws.as_ref().unwrap().module.custom_tlk().unwrap().unwrap_or_default()
    };
    assert_eq!(named(&mut h), "mg_talk");
    assert!(h.state().game.as_deref().unwrap().custom_tlk().is_some());

    // A line, typed: one undo takes the typing back.
    h.get_by_label("Add Line").click();
    h.run();
    h.get_by_label("16777216");
    type_into_hint(&mut h, "the line's text", "Greetings");
    type_into_hint(&mut h, "the feminine text", "Greetings, lady");
    let line = |h: &Harness<'_, Moonglow>| h.state().talk.as_ref().unwrap().line(0);
    assert_eq!(line(&h).text, "Greetings");
    assert_eq!(line(&h).feminine.as_deref(), Some("Greetings, lady"));
    h.get_by_label("Undo").click();
    h.run();
    assert_eq!((line(&h).text.as_str(), line(&h).feminine.as_deref()), ("Greetings", Some("")));
    h.get_by_label("Redo").click();
    h.run();
    assert!(h.state().has_unsaved_work());

    // Saving the module saves the table, and the game data reads it.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(!h.state().talk.as_ref().unwrap().is_dirty());
    let game = h.state().game.as_deref().unwrap();
    assert_eq!(game.string(mg_core::StrRef(16_777_216)).as_deref(), Some("Greetings"));
    let f = mg_tlk::Tlk::read(&std::fs::read(user.join("tlk/mg_talkf.tlk")).unwrap()).unwrap();
    assert_eq!(f.text(mg_core::StrRef(0)).as_deref(), Some("Greetings, lady"));

    // String Edit moves a name into the table (the talk table's window
    // closed: it lies over Module Properties).
    let tab = h.state().dock.find_tab(&Tab::TalkTable).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.run();
    h.get_by_label("…").click();
    h.run();
    h.get_by_label("Add Text").click();
    h.run();
    h.state_mut().loc_edit.as_mut().unwrap().entries[0].2 = "The Keep".into();
    h.run();
    h.get_by_label("Move to Talk Table").click();
    h.run();
    let edit = h.state().loc_edit.clone().unwrap();
    assert_eq!((edit.strref.as_str(), edit.entries.len()), ("16777217", 0));
    h.get_by_label("OK").click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let info = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap().root.clone();
    let name: LocString = info.read(&ifo::MOD_NAME);
    assert_eq!((name.strref.0, name.strings.len()), (16_777_217, 0));
    let t = h.state().talk.as_ref().unwrap();
    assert_eq!((t.line(1).text.as_str(), t.is_dirty()), ("The Keep", true));
}

#[test]
fn two_da_view_shows_where_rows_come_from() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-2da-layers");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    // The game's ambientmusic.2da with a row changed and one added, in
    // override.
    let game =
        mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let key = ResKey::parse("ambientmusic", ResType::TWODA).unwrap();
    let cp = mg_core::Codepage::default();
    let t = mg_2da::TwoDa::parse(&game.get(&key).unwrap(), cp).unwrap();
    let resource = t.column("Resource").unwrap();
    let mut rows: Vec<Vec<String>> = (0..t.len())
        .map(|r| (0..t.columns().len()).map(|c| t.cell(r, c).unwrap_or("****").into()).collect())
        .collect();
    rows[1][resource] = "mus_mine".into();
    let mut added = rows[1].clone();
    added[resource] = "mus_new".into();
    rows.push(added);
    let quote = |c: &String| if c.contains(' ') { format!("\"{c}\"") } else { c.clone() };
    let mut text = format!("2DA V2.0\n\n   {}\n", t.columns().join(" "));
    for (i, r) in rows.iter().enumerate() {
        let cells: Vec<String> = r.iter().map(quote).collect();
        text += &format!("{i} {}\n", cells.join(" "));
    }
    std::fs::write(user.join("override/ambientmusic.2da"), text).unwrap();

    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, Some(user), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    h.run();
    h.get_by_label("From");
    let summary = h.query_by_label_contains("override: adds 1, changes 1");
    assert!(summary.is_some(), "the summary names what override does");
    h.get_by_label("mus_mine");
    // The Description column shows the talk table's text.
    let description = t.get(0, "Description").and_then(|s| s.parse::<u32>().ok()).unwrap();
    let text = h.state().game.as_deref().unwrap().string(mg_core::StrRef(description)).unwrap();
    assert!(h.query_by_label(&text).is_some(), "{text}");
    // Only override's rows.
    h.state_mut().browser.rows_from = Some(0);
    h.run();
    assert!(h.query_by_label("mus_new").is_some());
    assert!(h.query_by_label(&text).is_none());
}

#[test]
fn hak_built_from_a_folder_attached_edited_and_reloaded() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-hak-editor");
    let (user, content, download) = (dir.join("user"), dir.join("mg_ui_hak"), dir.join("download"));
    for d in [&user, &content, &download] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(content.join("mg_ui_test.2da"), "2DA V2.0\n\n   Label\n0  hak\n").unwrap();
    std::fs::write(content.join("mg_ui_other.2da"), "2DA V2.0\n\n   Label\n0  other\n").unwrap();
    std::fs::write(content.join("a_name_far_too_long.2da"), "x").unwrap();
    let tlk = mg_tlk::Tlk::new(Language::ENGLISH).to_bytes().unwrap();
    std::fs::write(download.join("mg_ui.tlk"), tlk).unwrap();
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    let hak = user.join("hak/mg_ui_hak.hak");
    let dialogs = NoDialogs {
        folders: vec![content.clone()],
        save: vec![hak.clone()],
        open_many: vec![vec![hak.clone(), download.join("mg_ui.tlk")]],
        open: vec![hak.clone()],
    };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();

    // Built from the folder: the name too long is left out, and said why.
    mg_ui::hak_view::build_from_folder(h.state_mut());
    h.run();
    let id = h.state().haks[0].id;
    assert!(h.state().dock.find_tab(&Tab::Hak(id)).is_some());
    h.get_by_label("mg_ui_test.2da");
    let log = |h: &Harness<'_, Moonglow>| {
        h.state().log.entries.iter().map(|e| e.1.clone()).collect::<Vec<_>>()
    };
    assert!(
        log(&h).iter().any(|m| m.contains("a_name_far_too_long.2da") && m.contains("16")),
        "{:?}",
        log(&h)
    );
    h.get_by_label("Save").click();
    h.run();
    assert!(hak.is_file(), "{:?}", log(&h));
    assert!(!h.state().haks[0].hak.is_dirty());

    // Attached with a talk table in one step; the game data has it at once.
    let tab = h.state().dock.find_tab(&Tab::Hak(id)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().haks.clear();
    h.run();
    h.get_by_label("Custom Content").click();
    h.run();
    h.get_by_label("Add Haks and Talk Table…").click();
    h.run();
    assert!(h.state().attach.is_some());
    h.get_by_label("OK").click();
    h.run();
    assert!(user.join("tlk/mg_ui.tlk").is_file());
    let key = ResKey::parse("mg_ui_test", ResType::TWODA).unwrap();
    let has = |h: &Harness<'_, Moonglow>| h.state().game.as_deref().unwrap().resman.contains(&key);
    assert!(has(&h), "{:?}", log(&h));
    assert!(h.state().game.as_deref().unwrap().custom_tlk().is_some());
    // Undone, the hak leaves the game data.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(!has(&h));
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    h.run();
    assert!(has(&h));

    // Edited and saved, the module's hak is read again.
    mg_ui::hak_view::open_hak(h.state_mut());
    h.run();
    h.get_by_label("mg_ui_test.2da").click();
    h.run();
    h.get_by_label("Remove (1)").click();
    h.run();
    assert!(h.state().haks[0].hak.is_dirty());
    h.get_by_label("Undo").click();
    h.run();
    h.get_by_label("Redo").click();
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert!(!has(&h), "{:?}", log(&h));
    let other = ResKey::parse("mg_ui_other", ResType::TWODA).unwrap();
    assert!(h.state().game.as_deref().unwrap().resman.contains(&other));
}

#[test]
fn minimap_exported_and_object_walkmeshes_shown() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-minimap");
    let path = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &path).unwrap();
    let png = dir.join("map.png");
    let dialogs = NoDialogs { save: vec![png.clone()], ..Default::default() };
    let mut app =
        Moonglow::new(Some(mg_resman::GameInstall::new(&root, None, "en")), Box::new(dialogs));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("CHESS").click_secondary();
    h.run();
    h.get_by_label("Export Minimap…").click();
    h.run();
    let data = std::fs::read(&png).expect("the minimap is saved");
    assert_eq!(&data[1..4], b"PNG");

    // The area view draws placeables' and doors' walkmeshes when asked.
    let area = ResRef::from_str("chess").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    h.run();
    h.get_by_label("👣 Object Walkmeshes").click();
    h.run();
    assert!(h.state().area_views[&area].object_walkmesh);
}

#[test]
fn the_command_palette_finds_a_command_and_runs_it() {
    let dir = mg_testkit::scratch_dir("ui-command-palette");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let open = |h: &mut Harness<'_, Moonglow>| {
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
        h.run();
        assert!(h.state().command_palette.is_some(), "Ctrl+Shift+P opens it");
    };
    let hint = "Type a command's name";

    // Every command is there until something is typed; then those whose
    // name or menu has every word.
    open(&mut h);
    h.get_by_label("Verify Module");
    type_into_hint(&mut h, hint, "mod build");
    assert!(h.query_by_label("Open Module…").is_none());
    // (By name first; then the Build menu's other commands with "mod".)
    h.get_by_label("Build Module…");
    h.get_by_label("Verify Module");
    // Enter runs the one chosen, and the palette closes.
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().build.is_some() && h.state().command_palette.is_none());
    h.state_mut().build = None;
    h.run();

    // The arrows choose among several: the second of the two editors.
    open(&mut h);
    type_into_hint(&mut h, hint, "editor");
    h.get_by_label("Faction Editor");
    h.key_press(egui::Key::ArrowDown);
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Journal).is_some());
    assert!(h.state().dock.find_tab(&Tab::Factions).is_none());

    // A command that can't be chosen now is listed, and Enter leaves it.
    open(&mut h);
    type_into_hint(&mut h, hint, "redo");
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().command_palette.is_some(), "nothing to redo: nothing done");
    // A click runs one; Escape closes without running anything.
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().command_palette.is_none());
    open(&mut h);
    type_into_hint(&mut h, hint, "about");
    h.get_by_label("About Moonglow Toolset").click();
    h.run();
    assert!(h.state().command_palette.is_none());
    h.get_by_label_contains("GNU General Public License");
    // While it is open, keys are its own: Ctrl+N starts no module.
    open(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    assert!(h.state().command_palette.is_some() && h.state().ws.is_some());
}

#[test]
fn keys_remapped_in_options_and_used() {
    use mg_ui::keys::Cmd;
    let dir = mg_testkit::scratch_dir("ui-keys");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.dialog_no_text_popup = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Keyboard").click();
    h.run();
    // Every command of the menus is listed, with or without keys; the list
    // narrows to what is typed.
    h.get_by_label("Build Module");
    type_into_hint(&mut h, "Find a command", "manual");
    h.run();
    assert!(h.query_by_label("Build Module").is_none());
    // User Manual: F1 taken away, Ctrl+M added by pressing it.
    h.get_by_label("F1 ×").click();
    h.run();
    h.state_mut().options.as_mut().unwrap().recording = Some(Cmd::Manual.id().to_string());
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::M);
    h.run();
    // A key two commands share is named.
    h.state_mut().options.as_mut().unwrap().recording = Some(Cmd::Manual.id().to_string());
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    assert!(h.query_by_label_contains("is the key of both Save and User Manual").is_some());
    // The User Manual's Ctrl+S is taken away again (the only row shown).
    h.get_by_label("Ctrl+S ×").click();
    h.run();
    // A menu command that never had a key takes one.
    h.state_mut().options.as_mut().unwrap().recording = Some("build-module".to_string());
    h.run();
    h.key_press(egui::Key::F8);
    h.run();
    h.get_by_label("OK").click();
    h.run();
    let chosen = &h.state().settings.key_bindings;
    assert_eq!(chosen.get("manual"), Some(&vec!["Ctrl+M".to_string()]), "{chosen:?}");
    assert_eq!(chosen.get("build-module"), Some(&vec!["F8".to_string()]), "{chosen:?}");
    assert_eq!(chosen.len(), 2);
    // Its key opens its window, and its menu entry shows the key.
    h.key_press(egui::Key::F8);
    h.run();
    assert!(h.state().build.is_some(), "F8 opens Build Module");
    h.state_mut().build = None;
    h.run();
    h.get_by_label("Build").click();
    h.run();
    assert!(h.query_by_label_contains("F8").is_some());
    h.key_press(egui::Key::Escape);
    h.run();

    // F1 no longer opens the manual; Ctrl+M does.
    h.key_press(egui::Key::F1);
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Manual).is_none());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::M);
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Manual).is_some());
    // The menu shows the new key.
    h.get_by_label("Help").click();
    h.run();
    assert!(h.query_by_label_contains("Ctrl+M").is_some() || cfg!(target_os = "macos"));
    h.key_press(egui::Key::Escape);
    h.run();

    // Ctrl+A adds a line in the conversation editor, with the pointer over
    // it, as in Aurora.
    let tab = h.state().dock.find_tab(&Tab::Manual).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().new_dialog = Some("keysdlg".into());
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("keysdlg", ResType::DLG).unwrap();
    let starts = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&key).unwrap().root.list("StartingList").map_or(0, <[_]>::len)
    };
    assert_eq!(starts(&mut h), 0);
    let at = h.get_by_label("Expand All").rect().center();
    h.hover_at(at);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run();
    assert_eq!(starts(&mut h), 1);
}

#[test]
fn published_to_nwsync_from_the_build_menu() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-nwsync");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    let table = b"2DA V2.0\n\n   Label\n0  synced\n".to_vec();
    w.add(ResRef::from_str("mg_synced").unwrap(), ResType::TWODA, table.clone()).unwrap();
    std::fs::write(user.join("hak/mg_sync.hak"), w.to_bytes().unwrap()).unwrap();
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.set("Mod_HakList", mg_module::attach::hak_list(&info.root, &["mg_sync".into()]));
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let install = mg_resman::GameInstall::new(&root, Some(user), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Build").click();
    h.run();
    h.get_by_label("Publish to NWSync…").click();
    h.run();
    let repo = dir.join("repo");
    h.state_mut().publish.as_mut().unwrap().folder = repo.display().to_string();
    h.run();
    h.get_by_label("Publish").click();
    // It runs in the background.
    for _ in 0..200 {
        h.run_steps(2);
        if h.state().publish.as_ref().is_some_and(|p| p.written.is_some() || p.error.is_some()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let p = h.state().publish.as_ref().unwrap();
    assert_eq!(p.error, None);
    let written = p.written.clone().expect("published");
    assert_eq!(written.files, 1);
    assert_eq!(std::fs::read_to_string(repo.join("latest")).unwrap(), written.sha1);
    let manifest = std::fs::read(repo.join("manifests").join(&written.sha1)).unwrap();
    let entries = mg_module::nwsync::read_manifest(&manifest).unwrap();
    assert_eq!(entries[0].0.to_string(), "mg_synced.2da");
    assert_eq!(entries[0].1, mg_core::sha1::sha1(&table));
    assert!(h.get_all_by_label_contains(&written.sha1).count() >= 1);
    assert_eq!(h.state().settings.nwsync_repository.as_deref(), Some(repo.as_path()));
}

#[test]
fn tileset_made_edited_saved_with_its_palette() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-tileset");
    let set_path = dir.join("zzz01.set");
    let _ = std::fs::remove_file(&set_path);
    // A game tileset's copy, to edit too.
    let game =
        mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let tic = game.get(&ResKey::parse("tic01", ResType::SET).unwrap()).unwrap().into_owned();
    let tic_path = dir.join("tic01.set");
    std::fs::write(&tic_path, &tic).unwrap();
    let dialogs = NoDialogs {
        save: vec![set_path.clone()],
        open: vec![tic_path.clone()],
        ..Default::default()
    };
    let mut app =
        Moonglow::new(Some(mg_resman::GameInstall::new(&root, None, "en")), Box::new(dialogs));
    app.open_module(&sample_module(&dir));
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1300.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    mg_ui::tileset_view::new_tileset(h.state_mut());
    h.run();
    assert!(set_path.is_file());
    let id = h.state().tilesets[0].id;
    assert!(h.state().dock.find_tab(&Tab::Tileset(id)).is_some());

    // Terrains and a crosser.
    h.get_by_label("Terrains and Crossers").click();
    h.run();
    for (name, button) in
        [("Grass", "Add Terrain"), ("Water", "Add Terrain"), ("Road", "Add Crosser")]
    {
        h.state_mut().tilesets[0].new_type = name.into();
        h.run();
        h.get_by_label(button).click();
        h.run();
    }
    let t = h.state().tilesets[0].tileset().unwrap().clone();
    assert_eq!((t.terrains.len(), t.crossers.len()), (2, 1));

    // A tile, a group; an undo and a redo.
    h.get_by_label("Tiles").click();
    h.run();
    h.get_by_label("Add Tile").click();
    h.run();
    h.get_by_label("Groups").click();
    h.run();
    h.get_by_label("Add Group").click();
    h.run();
    h.get_by_label("Undo").click();
    h.run();
    assert!(h.state().tilesets[0].tileset().unwrap().groups.is_empty());
    h.get_by_label("Redo").click();
    h.run();
    let t = h.state().tilesets[0].tileset().unwrap().clone();
    assert_eq!((t.tiles.len(), t.groups.len()), (1, 1));
    assert_eq!(t.tiles[0].corners[0].0, "Grass");

    // Saved, its palette made, checked.
    h.get_by_label("Save").click();
    h.run();
    let saved =
        mg_set::Tileset::parse(&std::fs::read(&set_path).unwrap(), mg_core::Codepage::default())
            .unwrap();
    assert_eq!(saved.groups.len(), 1);
    h.get_by_label("Make Palette").click();
    h.run();
    let pal = mg_gff::Gff::read(&std::fs::read(dir.join("zzz01palstd.itp")).unwrap()).unwrap();
    assert_eq!(pal.file_type, *b"ITP ");
    h.get_by_label("Check").click();
    h.run();
    let findings = h.state().tilesets[0].findings.clone().unwrap();
    assert!(
        findings.iter().any(|f| f.message.contains("zzz01_a01_01")),
        "the tile's model is missing: {findings:?}"
    );

    // A game tileset's copy: one value changed, one line saved.
    mg_ui::tileset_view::open(h.state_mut());
    h.run();
    let doc = h.state_mut().tilesets.iter_mut().find(|d| d.path == tic_path).unwrap();
    doc.change(|f| f.set("GENERAL", "Transition", "6"));
    assert!(doc.is_dirty());
    doc.save().unwrap();
    let after = std::fs::read(&tic_path).unwrap();
    let (a, b) =
        (String::from_utf8_lossy(&tic).into_owned(), String::from_utf8_lossy(&after).into_owned());
    assert_eq!(a.lines().zip(b.lines()).filter(|(x, y)| x != y).count(), 1);
}

#[test]
fn conversation_lines_show_their_talk_table_text() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-dialog-tlk");
    let path = sample_module(&dir);
    // A line whose text is the game's talk table's string 12 alone.
    let mut g = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "");
    let entries = g.root.list_mut("EntryList").unwrap();
    entries[0].set("Text", mg_gff::Value::LocString(LocString::from_strref(mg_core::StrRef(12))));
    let mut m = Module::open(&path).unwrap();
    let key = ResKey::parse("tlkdlg", ResType::DLG).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    h.run();
    h.get_by_label_contains("[OWNER] - Paladin").click();
    h.run();
    h.get_by_label_contains("From the talk table (string 12): Paladin");
}

#[test]
fn faction_editor_adds_and_removes_factions() {
    let dir = mg_testkit::scratch_dir("ui-factions");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    h.run();
    let factions =
        |app: &Moonglow| mg_module::factions::Factions::of_module(&app.ws.as_ref().unwrap().module);
    assert_eq!(factions(h.state()).factions.len(), 5, "no repute.fac yet: the standard five");

    h.get_by_label("Add Faction…").click();
    h.run();
    {
        let add = h.state_mut().faction_view.adding.as_mut().unwrap();
        add.name = "Guards".into();
        add.parent = 4;
    }
    h.run();
    h.get_by_label("OK").click();
    h.run();
    let f = factions(h.state());
    assert_eq!(f.factions[5].name, "Guards");
    assert_eq!(f.factions[5].parent, Some(4));
    assert_eq!(f.reputation(5, 2), f.reputation(4, 2), "Guards regard Commoners as Defenders do");

    // Removing (Guards is selected after adding) is one undoable step.
    h.get_by_label("Remove Faction").click();
    h.run();
    assert_eq!(factions(h.state()).factions.len(), 5);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(factions(h.state()).factions.len(), 6);
}

#[test]
fn journal_editor_builds_what_aurora_builds() {
    let dir = mg_testkit::scratch_dir("ui-journal");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Journal));
    h.run();
    // As Aurora was driven: two categories, two entries in the first, one
    // in the second.
    let click = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_by_label(label).click();
        h.run();
    };
    click(&mut h, "Add");
    click(&mut h, "Root");
    click(&mut h, "Add");
    click(&mut h, "Category000  [Category000]");
    click(&mut h, "Add");
    click(&mut h, "Category000  [Category000]");
    click(&mut h, "Add");
    click(&mut h, "Category001  [Category001]");
    click(&mut h, "Add");
    let key = ResKey::parse("module", ResType::JRL).unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    let ours = Gff::read(ws.module.get(&key).unwrap()).unwrap();
    let cats = ours.root.items(&mg_schema::jrl::CATEGORIES);
    assert_eq!(cats.len(), 2);
    assert_eq!(mg_module::journal::entries(&cats[0]).len(), 2);
    if let Some(capture) = mg_testkit::aurora_capture("journal/two-categories.jrl") {
        assert_eq!(ours, Gff::read(&std::fs::read(capture).unwrap()).unwrap());
    }
    // Deleting an entry is one undoable step.
    click(&mut h, "[0002] Entry002");
    click(&mut h, "Delete");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let doc = ws.doc(&key).unwrap();
    assert_eq!(
        mg_module::journal::entries(&doc.root.items(&mg_schema::jrl::CATEGORIES)[0]).len(),
        2
    );
}

fn script_cursor(h: &Harness<'_, Moonglow>, key: ResKey) -> Option<(usize, usize)> {
    let state = egui::text_edit::TextEditState::load(&h.ctx, egui::Id::new(("script", key)))?;
    let r = state.cursor.char_range()?;
    Some((r.primary.index.into(), r.secondary.index.into()))
}

#[test]
fn script_editor_completion_find_bookmarks() {
    let dir = mg_testkit::scratch_dir("ui-script-tools");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void MyHelper() {}\nvoid main()\n{\n    MyHe\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();

    // Completion (F2) after "MyHe".
    let after_prefix = text.find("MyHe\n").unwrap() + 4;
    h.state_mut().script_tools.jump = Some((key, after_prefix));
    h.run();
    assert_eq!(script_cursor(&h, key), Some((after_prefix, after_prefix)));
    h.key_press(egui::Key::F2);
    h.run();
    let c = h.state().script_tools.completion.clone().expect("completion list");
    assert_eq!(c.items[0].name, "MyHelper");
    h.key_press(egui::Key::Enter);
    h.run();
    let edited = h.state().script_text(key).unwrap();
    assert!(edited.contains("    MyHelper(\n"), "{edited:?}");

    // Find (F3) selects the next match.
    h.state_mut().script_tools.search.find = "main".into();
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.key_press(egui::Key::F3);
    h.run();
    let (a, b) = script_cursor(&h, key).unwrap();
    let start = edited.find("main").unwrap();
    assert_eq!((a.min(b), a.max(b)), (start, start + 4));

    // Bookmark (F5) on that line.
    h.key_press(egui::Key::F5);
    h.run();
    assert!(h.state().script_bookmarks(key).contains(&1));

    // Numbered bookmark 2 on that line (Ctrl+Shift+2), then back to it from
    // the top (Ctrl+2).
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Num2);
    h.run();
    assert_eq!(h.state().script_numbered_bookmarks(key), [(2, 1)]);
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Num2);
    h.run();
    h.run();
    let line1 = edited.find('\n').unwrap() + 1;
    assert_eq!(script_cursor(&h, key), Some((line1, line1)));

    // Saving the module stores the editor's text.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    let saved = Module::open(&path).unwrap();
    assert!(String::from_utf8_lossy(saved.get(&key).unwrap()).contains("MyHelper("));
}

#[test]
fn compile_errors_go_to_their_line() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-compile-errors");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main()\n{\n    int x = 1;\n    x = y + 2;\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Compile").click();
    h.run();
    let message = h.state().script_tools.messages.last().cloned().expect("a compiler message");
    assert!(message.error);
    assert_eq!(message.location, Some(("hello".to_string(), 4)));
    // The message is in the log too; the Compiler tab's comes after it.
    h.get_all_by_label(&message.text).last().unwrap().click();
    h.run();
    h.run();
    let line4 = text.match_indices('\n').nth(2).unwrap().0 + 1;
    assert_eq!(script_cursor(&h, key), Some((line4, line4)));
}

#[test]
fn conversation_editor_builds_and_links() {
    let dir = mg_testkit::scratch_dir("ui-dialog");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    // New lines' text edited in place (the Input Text popup off).
    app.settings.dialog_no_text_popup = true;
    app.open_module(&path);
    app.new_dialog = Some("capdlg".into());
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("capdlg", ResType::DLG).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Dialog(key)).is_some());

    let is_text = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput
    };
    let add_line = |h: &mut Harness<'_, Moonglow>, text: &str| {
        h.get_by_label("Add").click();
        h.run();
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).find(is_text).unwrap().click();
        h.run();
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .find(is_text)
            .unwrap()
            .type_text(text);
        h.run();
        h.key_press(egui::Key::Tab);
        h.run();
    };
    let click = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_by_label(label).click();
        h.run();
    };
    add_line(&mut h, "Hello there.");
    add_line(&mut h, "Who are you?");
    add_line(&mut h, "A traveller.");
    click(&mut h, "Root");
    add_line(&mut h, "Go away.");
    click(&mut h, "Who are you?");
    click(&mut h, "Copy");
    click(&mut h, "[OWNER] - Go away.");
    click(&mut h, "Paste As Link");

    let doc = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone()
    };
    let ours = doc(&mut h);
    let texts = |g: &Gff| -> Vec<String> {
        mg_module::dialog::outline(g)
            .iter()
            .map(|l| l.split('|').take(3).collect::<Vec<_>>().join("|"))
            .collect()
    };
    assert_eq!(
        texts(&ours),
        [
            "Entry|Hello there.|",
            "  Reply|Who are you?|",
            "    Entry|A traveller.|",
            "Entry|Go away.|",
            "  Reply|Who are you?|link"
        ]
    );
    assert_eq!(ours.root.dword("NumWords"), Some(9));
    if let Some(capture) = mg_testkit::aurora_capture("dialog/capdlg.dlg") {
        let aurora = Gff::read(&std::fs::read(capture).unwrap()).unwrap();
        assert_eq!(texts(&ours), texts(&aurora));
    }

    // Deleting "Hello there." takes its branch and the link to it; undo
    // brings both back.
    click(&mut h, "[OWNER] - Hello there.");
    click(&mut h, "Delete");
    assert_eq!(texts(&doc(&mut h)), ["Entry|Go away.|"]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(texts(&doc(&mut h)).len(), 5);
}

#[test]
fn conversation_search_bookmarks_and_test() {
    use mg_module::dialog::{Kind, Parent, add_link, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("ui-dialog-tools");
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    add_node(&mut g, Parent::Node(Kind::Reply, who), "A traveller.");
    let go = add_node(&mut g, Parent::Root, "Go away.");
    add_link(&mut g, Parent::Node(Kind::Entry, go), who);
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let click = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_by_label(label).click();
        h.run();
    };

    // Search finds the line and selects it in the tree.
    click(&mut h, "Search");
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.find = "traveller".into();
    h.run();
    click(&mut h, "Find");
    let results = h.state().dialog_views[&key].search.results.clone();
    assert_eq!(results.len(), 1);
    click(&mut h, &format!("Entry {}: A traveller.", results[0].2));
    let selected = h.state().dialog_views[&key].selected.unwrap();
    assert_eq!(selected.parent, Parent::Node(Kind::Reply, who));

    // A bookmark on it.
    click(&mut h, "Bookmark");
    assert_eq!(h.state().dialog_views[&key].bookmarks, [(Kind::Entry, results[0].2)]);

    // Replace All in this conversation.
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.find = "Go away.".into();
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.replace = "Leave.".into();
    h.run();
    click(&mut h, "Replace All");
    let doc = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
    assert!(mg_module::dialog::outline(&doc).iter().any(|l| l.starts_with("Entry|Leave.|")));

    // Test mode: the greeting said, the reply chosen, the answer said;
    // back a turn.
    click(&mut h, "Test");
    h.get_by_label("NPC: Hello there.");
    click(&mut h, "1. Who are you?");
    h.get_by_label("NPC: A traveller.");
    h.get_by_label("[END DIALOGUE]");
    assert_eq!(h.state().dialog_views[&key].test.as_ref().unwrap().len(), 2);
    click(&mut h, "<-- Back");
    assert!(h.state().dialog_views[&key].test.as_ref().unwrap().is_empty());
    click(&mut h, "Done");
    assert!(h.state().dialog_views[&key].test.is_none());
}

#[test]
fn script_wizard_writes_compiles_and_sets_scripts() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-script-wizard");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    // The new line selected (the Input Text popup off).
    app.settings.dialog_no_text_popup = true;
    app.open_module(&path);
    app.new_dialog = Some("wizdlg".into());
    // Room for the conversation's window to show its tabs' buttons.
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("wizdlg", ResType::DLG).unwrap();
    h.get_by_label("Add").click();
    h.run();
    h.run();

    let window =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::Window;
    let in_wizard = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_all_by_label("Script Wizard")
            .find(window)
            .expect("the wizard")
            .get_by_label(label)
            .click();
        h.run();
    };
    let type_in_wizard = |h: &mut Harness<'_, Moonglow>, text: &str| {
        let w = h.get_all_by_label("Script Wizard").find(window).unwrap();
        let field = w.get_all_by_role(egui::accesskit::Role::TextInput).next().expect("a field");
        field.click();
        h.run();
        let w = h.get_all_by_label("Script Wizard").find(window).unwrap();
        w.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text(text);
        h.run();
    };

    // A condition: female, carrying key_1.
    h.get_by_label("Script Wizard…").click();
    h.run();
    h.run();
    for label in ["Gender", "Item In Inventory", "Next >", "Female", "Next >"] {
        in_wizard(&mut h, label);
    }
    type_in_wizard(&mut h, "key_1");
    for label in ["Add", "Next >", "Finish"] {
        in_wizard(&mut h, label);
    }
    h.run();
    let module = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().module.clone();
    let sc = String::from_utf8(
        module(&h).get(&ResKey::parse("sc_001", ResType::NSS).unwrap()).unwrap().to_vec(),
    )
    .unwrap();
    assert!(sc.contains("if(GetGender(GetPCSpeaker()) != GENDER_FEMALE)"), "{sc}");
    assert!(sc.contains("if(!HasItem(GetPCSpeaker(), \"key_1\"))"), "{sc}");
    assert!(module(&h).contains(&ResKey::parse("sc_001", ResType::NCS).unwrap()));
    let doc = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone()
    };
    let g = doc(&mut h);
    let start = &g.root.list("StartingList").unwrap()[0];
    assert_eq!(start.resref("Active").unwrap().as_str(), Some("sc_001"));

    // An action: 50 gold.
    h.get_by_label("Actions Taken").click();
    h.run();
    h.get_by_label("Script Wizard…").click();
    h.run();
    h.run();
    for label in ["Give rewards", "Next >"] {
        in_wizard(&mut h, label);
    }
    type_in_wizard(&mut h, "50");
    for label in ["Next >", "Finish"] {
        in_wizard(&mut h, label);
    }
    h.run();
    let at = String::from_utf8(
        module(&h).get(&ResKey::parse("at_001", ResType::NSS).unwrap()).unwrap().to_vec(),
    )
    .unwrap();
    assert!(at.contains("\tGiveGoldToCreature(GetPCSpeaker(), 50);\r\n"), "{at}");
    let g = doc(&mut h);
    assert_eq!(
        g.root.list("EntryList").unwrap()[0].resref("Script").unwrap().as_str(),
        Some("at_001")
    );

    // One undo takes the script, its bytecode and the field back.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(!module(&h).contains(&ResKey::parse("at_001", ResType::NSS).unwrap()));
    assert!(!module(&h).contains(&ResKey::parse("at_001", ResType::NCS).unwrap()));
    let g = doc(&mut h);
    assert!(g.root.list("EntryList").unwrap()[0].resref("Script").unwrap().is_empty());
}

#[test]
fn model_viewer_plays_animations() {
    let root = mg_testkit::corpus!();
    mg_testkit::gpu::hold();
    if mg_render::Gpu::headless().is_none() {
        eprintln!("skipped: no GPU adapter");
        return;
    }
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    let key = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.run();
    h.get_by_label_contains(" nodes, ");
    // Choose "open" from the animation list.
    h.get_by_label("Animation").click();
    h.run();
    h.get_by_label("open").click();
    // A playing animation keeps repainting: step frames.
    h.run_steps(3);
    assert_eq!(
        h.state().model_views[&mg_ui::model_view::Source::Resource(key)].animation.as_deref(),
        Some("open")
    );
    let img = h.render().expect("render");
    assert!(img.width() > 0);
}

#[test]
fn blueprint_previews_open() {
    let root = mg_testkit::corpus!();
    mg_testkit::gpu::hold();
    if mg_render::Gpu::headless().is_none() {
        eprintln!("skipped: no GPU adapter");
        return;
    }
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    // A gnome with robe, cloak and crossbow: its skeleton, standing idle.
    let key = ResKey::parse("nw_hen_bod_05", ResType::UTC).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    // The idle animation plays from the start: step frames.
    h.run_steps(4);
    h.get_by_label_contains("pmg0: ");
    assert_eq!(
        h.state().model_views[&mg_ui::model_view::Source::Resource(key)].animation.as_deref(),
        Some("pause1")
    );
    let img = h.render().expect("render");
    assert!(img.width() > 0);
}

#[test]
fn palette_edit_copy_and_delete() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-palette");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    // The standard waypoints: the "Tavern" waypoint (nw_wp_tavern).
    let tavern = app.game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
    app.palette.tiles = false;
    app.palette.filter = "nw_wp_tavern".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label(&tavern).click_secondary();
    h.run();
    h.get_by_label("Edit Copy").click();
    h.run();
    // A copy in the module, shown in the Custom palette.
    let copy = ResKey::parse("nw_wp_tavern001", ResType::UTW).unwrap();
    let gff = h.state_mut().ws.as_mut().unwrap().doc(&copy).unwrap().clone();
    assert_eq!(gff.root.resref("TemplateResRef").unwrap().to_string(), "nw_wp_tavern001");
    assert!(h.state().palette.custom);
    // Custom palettes list blueprints under their own names.
    let name = mg_module::palette::blueprint_name(
        mg_module::palette::BlueprintKind::Waypoint,
        &gff.root,
        h.state().game.as_deref().unwrap(),
    );
    // (The copy's editor, opened in a window, closed first.)
    close_windows(&mut h);
    h.run();
    h.get_by_label(&name).click_secondary();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&copy));
    // Undo brings it back.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&copy));
}

#[test]
fn finding_in_a_palette_opens_its_categories() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-palette-find");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
    app.palette.tiles = false;
    let tavern = app.game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Seen collapsed first, as a builder opening the palette would.
    assert!(h.query_by_label(&tavern).is_none());
    h.state_mut().palette.filter = "nw_wp_tavern".into();
    h.run();
    h.get_by_label(&tavern);
}

#[test]
fn palette_finds_by_tag_keeps_favorites_and_moves_between_categories() {
    use mg_edit::{Command, Edit};
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-palette-more");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let game = app.game.as_deref().unwrap();
    let base = Gff::read(&game.resman.get_named("nw_waypoint001", ResType::UTW).unwrap()).unwrap();
    // Gate A (tag MG_SECRET_GATE) in Waypoints, Gate B in Custom 1.
    let keys: Vec<ResKey> = ["mg_gate_a", "mg_gate_b"]
        .iter()
        .map(|n| ResKey::parse(n, ResType::UTW).unwrap())
        .collect();
    let edits = keys
        .iter()
        .zip([("Gate A", "MG_SECRET_GATE", 5u8), ("Gate B", "MG_PLAIN", 0)])
        .map(|(k, (name, tag, cat))| {
            let mut g = base.clone();
            g.root.set("TemplateResRef", mg_gff::Value::resref(k.resref));
            let ls = LocString::from_text(Language::ENGLISH, Gender::Male, name);
            g.root.set("LocalizedName", mg_gff::Value::LocString(ls));
            g.root.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
            g.root.set("PaletteID", mg_gff::Value::Byte(cat));
            Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
    app.palette.tiles = false;
    app.palette.custom = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Found by its tag, a word at a time.
    h.state_mut().palette.filter = "secret gate".into();
    h.run();
    h.get_by_label("Gate A");
    assert!(h.query_by_label("Gate B").is_none());
    // Letters in order, when nothing has the words.
    h.state_mut().palette.filter = "gtb".into();
    h.run();
    h.get_by_label("No exact matches: close ones");
    h.get_by_label("Gate B");
    assert!(h.query_by_label("Gate A").is_none());
    // A favorite, shown first.
    h.state_mut().palette.filter.clear();
    h.run();
    h.get_by_label("Gate A").click_secondary();
    h.run();
    h.get_by_label("Add to Favorites").click();
    h.run();
    assert_eq!(h.state().settings.palette_favorites, ["utw:mg_gate_a"]);
    h.get_by_label("Favorites (1)");
    // Dragged onto Custom 1, it moves there (one undoable step).
    // (Finding opened the categories; they stay open.)
    if h.query_by_label("Custom 1 (1)").is_none() {
        h.get_by_label("Special").click();
        h.run();
    }
    let from = h.get_all_by_label("★ Gate A").last().unwrap().rect().center();
    let to = h.get_by_label("Custom 1 (1)").rect().center();
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(10.0, 4.0));
    h.run();
    h.hover_at(to);
    h.run();
    h.drop_at(to);
    h.run();
    h.run();
    let category = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&keys[0]).unwrap().root.integer("PaletteID")
    };
    assert_eq!(category(&mut h), Some(0));
    h.get_by_label("Custom 1 (2)");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(category(&mut h), Some(5));
}

/// A module with an edit copy of a standard blueprint, open in its editor
/// (`None` without the game).
fn blueprint_harness(
    name: &str,
    copy: &str,
    t: ResType,
) -> Option<(Harness<'static, Moonglow>, ResKey)> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let dir = mg_testkit::scratch_dir(&format!("ui-bp-{copy}"));
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named(name, t).unwrap().into_owned();
    let key = ResKey::parse(copy, t).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "copy",
        vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
    )));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    // The editor alone on screen (Module Properties, docked when the module
    // opened, and the palette would show beside its window).
    close_tab(&mut app, &Tab::ModuleProperties);
    app.open_palette = false;
    let h = Harness::builder()
        .with_size(egui::vec2(1000.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    Some((h, key))
}

fn field(h: &mut Harness<'_, Moonglow>, key: &ResKey) -> mg_gff::Struct {
    h.state_mut().ws.as_mut().unwrap().doc(key).unwrap().root.clone()
}

#[test]
fn waypoint_editor_edits_and_renames() {
    let Some((mut h, key)) = blueprint_harness("nw_waypoint001", "waypoint_copy", ResType::UTW)
    else {
        return;
    };
    h.run();
    // Tag (commits when focus leaves).
    let tag = String::from_utf8_lossy(field(&mut h, &key).string("Tag").unwrap()).into_owned();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(&tag).find(is_input).expect("the tag field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&tag).find(is_input).unwrap().type_text("WP_RENAMED");
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(field(&mut h, &key).string("Tag").unwrap(), b"WP_RENAMED");
    h.get_by_label("Advanced").click();
    h.run();
    // A map note.
    h.get_by_label("Waypoint Contains a Map Note").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("HasMapNote"), Some(1));
    // Rename: the tab follows.
    h.state_mut().actions.push(mg_ui::Action::RenameBlueprint {
        from: key,
        to: ResRef::from_str("my_waypoint").unwrap(),
    });
    h.run();
    let new = ResKey::parse("my_waypoint", ResType::UTW).unwrap();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(ws.module.contains(&new) && !ws.module.contains(&key));
    assert!(h.state().dock.find_tab(&Tab::Blueprint(new)).is_some());
    assert_eq!(field(&mut h, &new).resref("TemplateResRef").unwrap().to_string(), "my_waypoint");
}

#[test]
fn sound_editor_lists_positions_and_times() {
    let Some((mut h, key)) = blueprint_harness("animalcriesday", "sound_copy", ResType::UTS) else {
        return;
    };
    h.run();
    let before = field(&mut h, &key).list("Sounds").map_or(0, <[_]>::len);
    // Add a sound through the picker.
    h.get_by_label("Add Sounds…").click();
    h.run();
    // One the blueprint does not list yet (its own list shows the others).
    h.state_mut().picker.as_mut().unwrap().filter = "as_cv_".into();
    h.run();
    h.get_all_by_label_contains("as_cv_").next().unwrap().click();
    h.run();
    let sounds = field(&mut h, &key).list("Sounds").unwrap().len();
    assert_eq!(sounds, before + 1);
    // Positioning: a random position sets both fields.
    h.get_by_label("Positioning").click();
    h.run();
    h.get_by_label("Plays from a random position each time it is played").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!((s.integer("Positional"), s.integer("RandomPosition")), (Some(1), Some(1)));
    // Specific hours.
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("Specific Hours").click();
    h.run();
    h.get_by_label("3 PM").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!(s.integer("Times"), Some(0));
    assert_ne!(s.integer("Hours").unwrap() & (1 << 15), 0);
}

/// Types into the text field with placeholder `hint` (a live filter).
fn type_into_hint(h: &mut Harness<'_, Moonglow>, hint: &str, text: &str) {
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some(hint);
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    h.run();
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text(text);
    h.run();
}

#[test]
fn trigger_editor_sets_the_type_and_trap() {
    let Some((mut h, key)) = blueprint_harness("trackstrigger", "trigger_copy", ResType::UTT)
    else {
        return;
    };
    h.run();
    assert_eq!(field(&mut h, &key).integer("Type"), Some(0));
    // Area transition settings apply to transitions only.
    h.get_by_label("Area Transition").click();
    h.run();
    assert!(h.query_by_label_contains("apply to Area Transition triggers").is_some());
    h.get_by_label("Basic").click();
    h.run();
    // Trigger Type › Trap (the option in the combo's popup, after the tab).
    h.get_by_value("Generic").click();
    h.run();
    h.get_all_by_label("Trap").last().unwrap().click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Type"), Some(2));
    h.get_all_by_label("Trap").next().unwrap().click();
    h.run();
    let trapped = field(&mut h, &key).integer("TrapFlag").unwrap_or(0);
    h.get_by_label("Is Trapped").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("TrapFlag"), Some(1 - trapped));
}

#[test]
fn encounter_editor_lists_creatures_and_respawns() {
    let Some((mut h, key)) = blueprint_harness("nw_verminbeet", "encounter_copy", ResType::UTE)
    else {
        return;
    };
    h.run();
    let before = field(&mut h, &key).list("CreatureList").map_or(0, <[_]>::len);
    h.get_by_label("Creature List").click();
    h.run();
    // Find a badger in the standard palette and add it.
    type_into_hint(&mut h, "Find", "nw_badger");
    let badger = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button
            && n.label().is_some_and(|l| l.starts_with("Badger"))
    };
    h.get(egui_kittest::kittest::by().predicate(badger)).click();
    h.run();
    h.get_by_label("Add Creature").click();
    h.run();
    let list = field(&mut h, &key).list("CreatureList").unwrap().to_vec();
    assert_eq!(list.len(), before + 1);
    let added = list.last().unwrap();
    assert_eq!(added.resref("ResRef").unwrap().to_string(), "nw_badger");
    assert!(added.float("CR").unwrap() > 0.0);
    // Remove it again.
    h.get_by_label("Badger").click();
    h.run();
    h.get_by_label("Remove Creature").click();
    h.run();
    assert_eq!(field(&mut h, &key).list("CreatureList").unwrap().len(), before);
    // Infinite respawns. Showing the page changes nothing, though the
    // blueprint's Respawns (0) is outside the field's range.
    let respawns = field(&mut h, &key).integer("Respawns");
    h.get_by_label("Advanced").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Respawns"), respawns);
    assert_ne!(h.state().ws.as_ref().unwrap().can_undo(), Some("Respawns"));
    if field(&mut h, &key).integer("Reset") != Some(1) {
        h.get_by_label("Encounter Respawns").click();
        h.run();
    }
    // Infinite is −1; turning it off leaves one respawn.
    let infinite = field(&mut h, &key).integer("Respawns") == Some(-1);
    h.get_by_label("Infinite Respawn").click();
    h.run();
    let expected = if infinite { 1 } else { -1 };
    assert_eq!(field(&mut h, &key).integer("Respawns"), Some(expected));
}

#[test]
fn store_editor_stocks_prices_and_restricts() {
    let Some((mut h, key)) = blueprint_harness("nw_storebar01", "store_copy", ResType::UTM) else {
        return;
    };
    h.run();
    // Limited gold (a store without the field has unlimited gold).
    assert!(matches!(field(&mut h, &key).integer("StoreGold"), None | Some(-1)));
    h.get_by_label("Has Limited Gold").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("StoreGold"), Some(1000));
    // A longsword goes on the weapons page, at the first free place.
    h.get_by_label("Inventory…").click();
    h.run();
    let weapons = |s: &mg_gff::Struct| {
        s.list("StoreList")
            .unwrap()
            .iter()
            .find(|p| p.id == 4)
            .and_then(|p| p.list("ItemList"))
            .map_or(Vec::new(), <[_]>::to_vec)
    };
    let before = weapons(&field(&mut h, &key));
    type_into_hint(&mut h, "Find", "nw_wswls001");
    let sword = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button && n.label().is_some_and(|l| l == "Longsword")
    };
    h.get(egui_kittest::kittest::by().predicate(sword)).click();
    h.run();
    // The chosen item's statistics show before it is added.
    h.get_by_label("Chosen in the palette");
    h.get_by_label("Damage");
    h.get_by_label("1d8");
    h.get_by_label("19-20 / x2");
    h.get_by_label("Add Item").click();
    h.run();
    let after = weapons(&field(&mut h, &key));
    assert_eq!(after.len(), before.len() + 1);
    let added = after.last().unwrap();
    assert_eq!(added.resref("InventoryRes").unwrap().to_string(), "nw_wswls001");
    let at = (added.integer("Repos_PosX").unwrap(), added.integer("Repos_Posy").unwrap());
    let at_before: Vec<_> = before
        .iter()
        .map(|i| (i.integer("Repos_PosX").unwrap(), i.integer("Repos_Posy").unwrap()))
        .collect();
    assert!(!at_before.contains(&at), "{at:?} is taken");
    // The store will not buy torches, then will only buy them.
    h.get_by_label("Restrictions").click();
    h.run();
    type_into_hint(&mut h, "Find", "Torch");
    h.get_by_label("Torch").click();
    h.run();
    h.get_by_label("Add").click();
    h.run();
    let bases = |s: &mg_gff::Struct, list: &str| -> Vec<i64> {
        s.list(list).unwrap_or(&[]).iter().filter_map(|i| i.integer("BaseItem")).collect()
    };
    assert_eq!(bases(&field(&mut h, &key), "WillNotBuy"), [15]);
    h.get_by_label("Store will ONLY buy the following items").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!((bases(&s, "WillNotBuy"), bases(&s, "WillOnlyBuy")), (vec![], vec![15]));
    assert_eq!(s.list("WillOnlyBuy").unwrap()[0].id, 97869);
}

#[test]
fn showing_blueprint_editors_changes_nothing() {
    // Every page of every editor, on blueprints with values outside the
    // fields' ranges (Respawns 0, missing fields, generic doors).
    for (i, (name, t)) in [
        ("nw_waypoint001", ResType::UTW),
        ("animalcriesday", ResType::UTS),
        ("x0_trapavg_shuri", ResType::UTT),
        ("nw_verminbeet", ResType::UTE),
        ("nw_storebar01", ResType::UTM),
        ("x2_storethief003", ResType::UTM),
        ("nw_door_ttr_01", ResType::UTD),
        ("plc_chest1", ResType::UTP),
        ("nw_wswls001", ResType::UTI),
        ("nw_aarcl001", ResType::UTI),
        ("nw_it_mpotion001", ResType::UTI),
        ("nw_maarcl002", ResType::UTI),
        ("nw_bandit001", ResType::UTC),
        ("nw_bartender", ResType::UTC),
        ("nw_badger", ResType::UTC),
        ("db_tanarukk_do", ResType::UTC),
        ("nw_bandit004", ResType::UTC),
    ]
    .into_iter()
    .enumerate()
    {
        let Some((mut h, key)) = blueprint_harness(name, &format!("copy{i}"), t) else {
            return;
        };
        for page in mg_ui::blueprint::pages(t) {
            h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), page);
            h.run();
        }
        let ws = h.state().ws.as_ref().unwrap();
        assert_eq!(ws.can_undo(), Some("copy"), "{name}");
    }
}

#[test]
fn door_editor_sets_appearance_lock_and_transition() {
    let Some((mut h, key)) = blueprint_harness("nw_door_ttr_01", "door_copy", ResType::UTD) else {
        return;
    };
    h.run();
    // A generic door: its Generic Appearance (and the old byte field).
    let generic = {
        let game = h.state().game.as_deref().unwrap();
        let cols = mg_rules::ChoiceColumns { name: Some("Name"), label: Some("Label") };
        game.choices("genericdoors", cols).unwrap()
    };
    assert_eq!(field(&mut h, &key).integer("Appearance"), Some(0));
    h.get_by_value(&generic[0].text).click();
    h.run();
    h.get_by_label(&generic[1].text).click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!(s.integer("GenericType_New"), Some(generic[1].row as i64));
    assert_eq!(s.integer("GenericType"), Some(generic[1].row as i64));
    // Lock.
    h.get_by_label("Lock").click();
    h.run();
    h.get_by_label("Locked").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Locked"), Some(1));
    // Area transition to a waypoint.
    h.get_by_label("Area Transition").click();
    h.run();
    h.get_by_label("Waypoint").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("LinkedToFlags"), Some(2));
    // No Interrupt is Interruptable 0.
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("No Interrupt").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Interruptable"), Some(0));
}

#[test]
fn placeable_editor_fills_its_inventory() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_copy", ResType::UTP) else {
        return;
    };
    h.run();
    h.get_by_label("Inventory…").click();
    h.run();
    type_into_hint(&mut h, "Find", "nw_it_torch001");
    let torch = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button && n.label().is_some_and(|l| l == "Torch")
    };
    h.get(egui_kittest::kittest::by().predicate(torch)).click();
    h.run();
    for _ in 0..2 {
        h.get_by_label("Add Item").click();
        h.run();
    }
    // Torches are 1×3: the second goes beside the first.
    let items = field(&mut h, &key).list("ItemList").unwrap().to_vec();
    let at: Vec<_> = items
        .iter()
        .map(|i| {
            (
                i.resref("InventoryRes").unwrap().to_string(),
                i.integer("Repos_PosX").unwrap(),
                i.integer("Repos_Posy").unwrap(),
            )
        })
        .collect();
    assert_eq!(at, [("nw_it_torch001".into(), 0, 0), ("nw_it_torch001".into(), 1, 0)]);
    // An older blueprint's portrait is a resref (po_ and the portraits.2da
    // base resref); without a repute.fac, the standard factions.
    h.get_by_label("Advanced").click();
    h.run();
    assert!(h.query_by_value("PLC_A08_").is_some());
    assert!(h.query_by_value("Hostile").is_some());
    // A static placeable holds nothing: Has Inventory is disabled.
    h.get_by_label("Basic").click();
    h.run();
    h.get_by_label("Static").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Static"), Some(1));
    assert!(h.get_by_label("Has Inventory").accesskit_node().is_disabled());
}

#[test]
fn visuals_page_sets_replacements_and_misc_visuals() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_visuals", ResType::UTP) else {
        return;
    };
    h.run();
    h.get_by_label("Visuals").click();
    h.run();
    // A texture replacement, added from the empty row.
    type_into_hint(&mut h, "Texture", "plc_chest1");
    type_into_hint(&mut h, "Drawn as", "mg_gold");
    h.get_all_by_label("Add").next().unwrap().click();
    h.run();
    let tex = |h: &mut Harness<'_, Moonglow>| -> Vec<(String, String)> {
        let root = field(h, &key);
        let list = root.child("TextureReplace").and_then(|s| s.list("TextureReplaceLi"));
        list.unwrap_or(&[])
            .iter()
            .map(|e| {
                let r = |l: &str| e.resref(l).unwrap().to_string();
                (r("OldTexture"), r("NewTexture"))
            })
            .collect()
    };
    assert_eq!(tex(&mut h), [("plc_chest1".to_string(), "mg_gold".to_string())]);
    assert_eq!(field(&mut h, &key).child("TextureReplace").unwrap().id, 9, "the game's struct id");
    // The area view and model viewer draw it.
    let chest = field(&mut h, &key);
    let game = h.state().game.as_deref().unwrap();
    let preview = mg_preview::replaced(mg_preview::placeable(game, &chest).unwrap(), &chest);
    assert_eq!(preview.base.textures.get("plc_chest1").map(String::as_str), Some("mg_gold"));
    // Removed: the field goes.
    h.get_all_by_label("Remove").next().unwrap().click();
    h.run();
    assert!(field(&mut h, &key).get("TextureReplace").is_none());
    // MiscVisuals: what shows, set apart from the game's choice; the other
    // fields are left to the game.
    h.get_by_label("As the game decides").click();
    h.run();
    h.get_by_label("Name with Tab").click();
    h.run();
    let misc = field(&mut h, &key).child("MiscVisuals").cloned().unwrap();
    assert_eq!(misc.integer("UiDiscoverMask"), Some(15 & !8));
    assert_eq!(misc.id, 8);
    assert!(misc.get("VisibleDistance").is_none());
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(
        field(&mut h, &key).child("MiscVisuals").unwrap().integer("UiDiscoverMask"),
        Some(15)
    );
}

#[test]
fn classes_page_sets_domains_and_associates() {
    let Some((mut h, key)) = blueprint_harness("nw_mumcleric", "cleric_copy", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run();
    let cleric = |h: &mut Harness<'_, Moonglow>| -> mg_gff::Struct {
        let root = field(h, &key);
        root.list("ClassList")
            .unwrap()
            .iter()
            .find(|c| c.integer("Class") == Some(2))
            .unwrap()
            .clone()
    };
    // The cleric's first domain, from domains.2da (the blueprint has none:
    // the game decides).
    assert_eq!(cleric(&mut h).integer("Domain1"), None);
    let animal = {
        let game = h.state().game.as_deref().unwrap();
        let t = game.table("domains").unwrap();
        game.string(mg_core::StrRef(t.get_int(1, "Name").unwrap() as u32)).unwrap()
    };
    let current = "Not set";
    h.get_all_by_value(current).next().expect("the domain picker").click();
    h.run();
    h.get_by_label(&animal).click();
    h.run();
    assert_eq!(cleric(&mut h).integer("Domain1"), Some(1));
    // A cleric has neither a familiar nor a companion: the game ignores
    // them, and the page says so.
    assert!(h.query_by_label_contains("a familiar only for a creature with an arcane").is_some());
    assert!(h.query_by_label_contains("companion only for a creature with a divine").is_some());
}

#[test]
fn item_editor_adds_properties_and_keeps_the_cost() {
    let Some((mut h, key)) = blueprint_harness("nw_wswls001", "sword_copy", ResType::UTI) else {
        return;
    };
    let cost_of = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let value = mg_rules::ItemValue::from_gff(&s);
        let game = h.state().game.as_deref().unwrap();
        (s.integer("Cost"), Some(i64::from(game.item_cost(&value))))
    };
    h.run();
    h.get_by_label("Properties").click();
    h.run();
    let before = field(&mut h, &key).list("PropertiesList").map_or(0, <[_]>::len);
    // Far down the list: scrolled to first.
    h.get_by_label("Enhancement Bonus").scroll_to_me();
    h.run();
    h.get_by_label("Enhancement Bonus").click();
    h.run();
    h.get_by_label("Add").click();
    h.run();
    let props = field(&mut h, &key).list("PropertiesList").unwrap().to_vec();
    assert_eq!(props.len(), before + 1);
    assert_eq!(props.last().unwrap().integer("PropertyName"), Some(6));
    // The stored cost follows the properties.
    let (stored, computed) = cost_of(&mut h);
    assert_eq!(stored, computed);
    let plus_one = stored;
    // The new property is selected: its value, +1, becomes +3.
    h.get_by_value("+1").click();
    h.run();
    h.get_by_label("+3").click();
    h.run();
    assert_eq!(
        field(&mut h, &key).list("PropertiesList").unwrap().last().unwrap().integer("CostValue"),
        Some(3)
    );
    let (stored, computed) = cost_of(&mut h);
    assert_eq!(stored, computed);
    assert!(stored > plus_one);
    // One undo takes back the value and its cost.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(cost_of(&mut h).0, plus_one);
}

#[test]
fn creature_editor_levels_and_aligns() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_copy", ResType::UTC) else {
        return;
    };
    let max_hp = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let sheet = mg_rules::CreatureSheet::from_gff(&s);
        let game = h.state().game.as_deref().unwrap();
        (s.integer("MaxHitPoints"), i64::from(game.creature_stats(&sheet).max_hit_points))
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run();
    // A second class: one more level, so more hit points from Constitution,
    // stored with the class in one undo step.
    let (before, _) = max_hp(&mut h);
    h.get_by_label("Add Class").click();
    h.run();
    assert_eq!(field(&mut h, &key).list("ClassList").unwrap().len(), 2);
    let (stored, computed) = max_hp(&mut h);
    assert_eq!(stored, Some(computed));
    assert_ne!(stored, before);
    // So is the challenge rating, recalculated as Aurora does on OK.
    let s = field(&mut h, &key);
    let game = h.state().game.as_deref().unwrap();
    let item = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let mut sheet = mg_rules::CreatureSheet::from_gff(&s);
    sheet.gear_value = game.gear_value(&s, &item);
    assert!(sheet.gear_value > 0, "the bandit's gear");
    assert_eq!(s.float("ChallengeRating"), Some(game.challenge(&sheet).rating));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(field(&mut h, &key).list("ClassList").unwrap().len(), 1);
    assert_eq!(max_hp(&mut h).0, before);
    // An alignment preset sets both axes.
    h.get_by_value("Chaotic Neutral").click();
    h.run();
    h.get_by_label("Lawful Good").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!((s.integer("GoodEvil"), s.integer("LawfulChaotic")), (Some(100), Some(100)));
}

#[test]
fn creature_editor_lists() {
    let Some((mut h, key)) = blueprint_harness("db_tanarukk_do", "sorc_copy", ResType::UTC) else {
        return;
    };
    let count =
        |h: &mut Harness<'_, Moonglow>, list: &str| field(h, &key).list(list).map_or(0, <[_]>::len);
    h.run();
    // A feat.
    h.get_by_label("Feats").click();
    h.run();
    // Alertness is feat 0; the checkbox toggles it.
    let has_alertness = |h: &mut Harness<'_, Moonglow>| {
        field(h, &key).list("FeatList").unwrap().iter().any(|f| f.integer("Feat") == Some(0))
    };
    let had = has_alertness(&mut h);
    type_into_hint(&mut h, "Find", "Alertness");
    h.get_by_label("Alertness").click();
    h.run();
    assert_eq!(has_alertness(&mut h), !had);
    // A known spell for the sorcerer (level 1: Magic Missile).
    h.get_by_label("Spells").click();
    h.run();
    let known = |h: &mut Harness<'_, Moonglow>| {
        let classes = field(h, &key).list("ClassList").unwrap().to_vec();
        let sorcerer = classes.iter().find(|c| c.integer("Class") == Some(9)).unwrap();
        sorcerer.list("KnownList1").map_or(0, <[_]>::len)
    };
    let before = known(&mut h);
    type_into_hint(&mut h, "Find", "Magic Missile");
    h.get_by_label("Magic Missile").click();
    h.run();
    assert_eq!(known(&mut h), before + 1);
    // A special ability.
    h.get_by_label("Special Abilities").click();
    h.run();
    let specials = count(&mut h, "SpecAbilityList");
    type_into_hint(&mut h, "Find", "Fireball");
    h.get_all_by_label("Fireball").next().unwrap().click();
    h.run();
    assert_eq!(count(&mut h, "SpecAbilityList"), specials + 1);
    // Armor, chosen in the palette, equipped in the armor slot (the second).
    h.get_by_label("Inventory").click();
    h.run();
    type_into_hint(&mut h, "Find", "nw_aarcl001");
    h.get_by_label("Leather Armor").click();
    h.run();
    h.get_all_by_label("Equip").nth(1).unwrap().click();
    h.run();
    let equipped = field(&mut h, &key).list("Equip_ItemList").unwrap().to_vec();
    let chest = equipped.iter().find(|s| s.id == 2).expect("armor equipped");
    assert_eq!(chest.resref("EquippedRes").unwrap().to_string(), "nw_aarcl001");
}

/// The sample module with the game's data (`None` without the game).
fn game_harness(name: &str) -> Option<Harness<'static, Moonglow>> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let dir = mg_testkit::scratch_dir(&format!("ui-wizard-{name}"));
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    Some(
        Harness::builder()
            .with_size(egui::vec2(1000.0, 800.0))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app),
    )
}

fn module_gff(h: &mut Harness<'_, Moonglow>, name: &str, t: ResType) -> Option<Gff> {
    let key = ResKey::parse(name, t).unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    ws.module.gff(&key).map(|g| g.unwrap())
}

#[test]
fn store_wizard_makes_aurora_s_store() {
    use mg_module::palette::BlueprintKind;
    let Some(mut h) = game_harness("store") else { return };
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Store));
    h.run();
    h.get_by_label("Merchants").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    // The suggested name follows the category.
    assert!(h.query_all_by_value("Merchants 001").next().is_some());
    h.get_by_label("Finish").click();
    h.run();
    let made = module_gff(&mut h, "merchants001", ResType::UTM).expect("the store");
    let r = ResRef::from_str("merchants001").unwrap();
    assert_eq!(made, mg_module::blueprints::store(r, "Merchants 001", 5));
    assert!(h.state().blueprint_wizard.is_none());
}

#[test]
fn sound_wizard_steps_through_timing_positioning_and_waves() {
    use mg_module::palette::BlueprintKind;
    let Some(mut h) = game_harness("sound") else { return };
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Sound));
    h.run();
    h.get_by_label("Civilization").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_by_label("Seamlessly looping").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    // Random positions are for single shots only.
    assert!(h.get_by_label("Random Positional").accesskit_node().is_disabled());
    h.get_by_label("Positional").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_by_label("Add Sounds…").click();
    h.run();
    h.state_mut().picker.as_mut().unwrap().filter = "al_cv_firecamp1".into();
    h.run();
    h.get_all_by_label_contains("al_cv_firecamp1").next().unwrap().click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_by_label("Finish").click();
    h.run();
    let made = module_gff(&mut h, "civilization001", ResType::UTS).expect("the sound");
    let r = ResRef::from_str("civilization001").unwrap();
    let fire = ResRef::from_str("al_cv_firecamp1").unwrap();
    let expected = mg_module::blueprints::sound(
        r,
        "Civilization 001",
        13,
        mg_module::blueprints::SoundStyle::LoopingPositional,
        &[fire],
    );
    assert_eq!(made, expected);
    // Its properties open, as Aurora's Sound Wizard does by default.
    let key = ResKey::parse("civilization001", ResType::UTS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Blueprint(key)).is_some());
}

#[test]
fn item_wizard_makes_a_weapon_with_its_cost() {
    use mg_module::palette::BlueprintKind;
    let Some(mut h) = game_harness("item") else { return };
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Item));
    h.run();
    h.get_by_label("Bastard Sword").scroll_to_me();
    h.run();
    h.get_by_label("Bastard Sword").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    type_into_hint(&mut h, "Name", "Wizard Sword");
    h.get_by_label("Next >").click();
    h.run();
    for branch in ["Weapons", "Bladed"] {
        h.get_by_label(branch).click();
        h.run();
    }
    h.get_by_label("Bastard Swords").click();
    h.run();
    h.get_by_label("Finish").click();
    h.run();
    let made = module_gff(&mut h, "wizardsword", ResType::UTI).expect("the item");
    assert_eq!(made.root.integer("BaseItem"), Some(3));
    assert_eq!(made.root.integer("Cost"), Some(70));
    assert_eq!(made.root.integer("PaletteID"), Some(33));
}

/// A module with a 4 by 4 rural area holding two waypoints, at (20, 20)
/// and (30, 20), with the game's data and a GPU (`None` without either).
fn area_harness(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    area_harness_on(name, "ttr01", None)
}

/// [`area_harness`] on another tileset, with tile 5 (at column 1, row 1)
/// replaced by tile `tile` of that tileset when given.
fn area_harness_on(
    name: &str,
    tileset: &str,
    tile: Option<i32>,
) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let root = mg_testkit::nwn_root()?;
    mg_testkit::gpu::hold();
    if mg_render::Gpu::headless().is_none() {
        eprintln!("skipped: no GPU adapter");
        return None;
    }
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Area View", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str(tileset).unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    if let Some(id) = tile {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        let t = &mut are.root.list_mut("Tile_List").unwrap()[5];
        t.set("Tile_ID", mg_gff::Value::Int(id));
        t.set("Tile_Orientation", mg_gff::Value::Int(0));
        t.set("Tile_Height", mg_gff::Value::Int(0));
        m.set_gff(key, &are).unwrap();
    }
    let bp = game.resman.get(&ResKey::parse("nw_waypoint001", ResType::UTW).unwrap()).unwrap();
    let bp = Gff::read(&bp).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let waypoints = [20.0, 30.0]
        .map(|x| {
            let at = Placement { position: [x, 20.0, 0.0], rotation: 0.3 };
            instance(&placing, ResType::UTW, &bp.root, at, &[]).unwrap()
        })
        .to_vec();
    git.root.set("WaypointList", mg_gff::Value::List(waypoints));
    m.set_gff(git_key, &git).unwrap();
    let path = mg_testkit::scratch_dir(&format!("ui-area-{name}")).join("area.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    // Frames a 60th of a second apart, so that clicks can be double clicks.
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .with_step_dt(1.0 / 60.0)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    Some((h, area))
}

/// A waypoint's position in the workspace's GIT.
fn waypoint(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize) -> Option<(f32, f32, u32)> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    let w = git.root.list("WaypointList")?.get(index)?;
    Some((w.float("XPosition")?, w.float("YPosition")?, w.float("XOrientation")?.to_bits()))
}

fn screen(h: &Harness<'_, Moonglow>, area: ResRef, p: glam::Vec3) -> egui::Pos2 {
    h.state().area_views[&area].screen_pos(p).expect("in view")
}

fn press(h: &Harness<'_, Moonglow>, pos: egui::Pos2, pressed: bool, modifiers: egui::Modifiers) {
    let button = egui::PointerButton::Primary;
    h.event_modifiers(egui::Event::PointerButton { pos, button, pressed, modifiers }, modifiers);
}

#[test]
fn area_viewer_selects_moves_and_deletes() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("move") else { return };
    let view = &h.state().area_views[&area];
    let model = view.model.as_ref().expect("the area is read");
    assert_eq!((model.width, model.height, model.objects.len()), (4, 4, 2));
    assert!(model.problems.is_empty(), "{:?}", model.problems);
    let (x0, y0, turn) = waypoint(&mut h, area, 0).unwrap();

    // A click on the first waypoint's marker selects it.
    let on = screen(&h, area, Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(on);
    press(&h, on, true, egui::Modifiers::NONE);
    press(&h, on, false, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 0)]);

    // Dragged 5 m east (by its feet: the ground under the pointer moves it):
    // one command moves it, its orientation untouched.
    let on = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    let to = screen(&h, area, Vec3::new(25.0, 20.0, 0.02));
    press(&h, on, true, egui::Modifiers::NONE);
    for k in 1..=4 {
        h.hover_at(on + (to - on) * (k as f32 / 4.0));
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let (x, y, t) = waypoint(&mut h, area, 0).unwrap();
    assert!((x - 25.0).abs() < 0.05 && (y - y0).abs() < 0.05, "moved to {x}, {y}");
    assert_eq!(t, turn, "the orientation is not rewritten");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(waypoint(&mut h, area, 0).map(|w| (w.0, w.1)), Some((x0, y0)));

    // Ctrl + click adds the second; Delete deletes both.
    let second = screen(&h, area, Vec3::new(30.0, 20.0, 0.9));
    h.hover_at(second);
    press(&h, second, true, egui::Modifiers::COMMAND);
    press(&h, second, false, egui::Modifiers::COMMAND);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].selection.len(), 2);
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(waypoint(&mut h, area, 0), None, "both deleted");
    assert!(h.state().area_views[&area].selection.is_empty());
    // Undo brings them back; the second stays selectable.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(waypoint(&mut h, area, 1).is_some());
    h.hover_at(second);
    press(&h, second, true, egui::Modifiers::NONE);
    press(&h, second, false, egui::Modifiers::NONE);
    h.run_steps(2);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-move").join("area_view.png")).unwrap();
}

#[test]
fn area_viewer_places_draws_boxes_and_turns() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("place") else { return };
    let git_list = |h: &mut Harness<'_, Moonglow>, list: &str| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list(list).unwrap_or(&[]).to_vec()
    };
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
    };

    // A waypoint from the palette, placed on the ground where clicked.
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    h.run_steps(1);
    h.get_by_label_contains("Placing nw_waypoint001");
    click(&mut h, Vec3::new(15.0, 30.0, 0.0));
    h.run_steps(1);
    let waypoints = git_list(&mut h, "WaypointList");
    assert_eq!(waypoints.len(), 3);
    let (x, y) =
        (waypoints[2].float("XPosition").unwrap(), waypoints[2].float("YPosition").unwrap());
    assert!((x - 15.0).abs() < 0.1 && (y - 30.0).abs() < 0.1, "placed at {x}, {y}");
    assert_eq!(waypoints[2].float("YOrientation"), Some(1.0), "facing north, as Aurora");
    assert!(h.state().palette.selected.is_none(), "one click, one waypoint");
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 2)]);

    // A trigger, drawn: three corners, a double click on the fourth.
    h.state_mut().palette.selected = ResKey::parse("newgeneric", ResType::UTT);
    h.run_steps(1);
    for p in [[30.0, 30.0], [36.0, 30.0], [36.0, 36.0]] {
        click(&mut h, Vec3::new(p[0], p[1], 0.0));
    }
    let last = screen(&h, area, Vec3::new(30.0, 36.0, 0.0));
    h.hover_at(last);
    for _ in 0..2 {
        press(&h, last, true, egui::Modifiers::NONE);
        press(&h, last, false, egui::Modifiers::NONE);
    }
    h.run_steps(3);
    let triggers = git_list(&mut h, "TriggerList");
    assert_eq!(triggers.len(), 1, "the double click closes it");
    let points = triggers[0].list("Geometry").unwrap();
    assert_eq!(points.len(), 4);
    // It stands at its first corner; its points are relative to it.
    assert!((triggers[0].float("XPosition").unwrap() - 30.0).abs() < 0.1);
    assert_eq!(points[0].float("PointX"), Some(0.0));
    assert!((points[2].float("PointX").unwrap() - 6.0).abs() < 0.1);

    // A box around the first two waypoints selects them.
    let (a, b) = (
        screen(&h, area, Vec3::new(17.0, 17.0, 0.0)),
        screen(&h, area, Vec3::new(33.0, 23.0, 0.0)),
    );
    h.hover_at(a);
    press(&h, a, true, egui::Modifiers::NONE);
    for k in 1..=4 {
        h.hover_at(a + (b - a) * (k as f32 / 4.0));
    }
    press(&h, b, false, egui::Modifiers::NONE);
    h.run_steps(2);
    let mut selected = h.state().area_views[&area].selection.clone();
    selected.sort();
    assert_eq!(selected, [(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)]);

    // Shift + right drag turns them: one command, orientations only.
    let before = git_list(&mut h, "WaypointList");
    let (from, to) = (a, a + egui::vec2(-80.0, 0.0));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    h.hover_at(from);
    let right = |pressed, pos| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::SHIFT,
    };
    h.event(right(true, from));
    for k in 1..=4 {
        h.hover_at(from + (to - from) * (k as f32 / 4.0));
    }
    h.event(right(false, to));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let after = git_list(&mut h, "WaypointList");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Rotate"));
    for i in 0..2 {
        assert_eq!(before[i].float("XPosition"), after[i].float("XPosition"));
        assert_ne!(before[i].float("XOrientation"), after[i].float("XOrientation"), "turned");
    }
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-place").join("area_view.png")).unwrap();
}

#[test]
fn placed_objects_open_their_properties() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("properties") else { return };
    let tag_of = |h: &mut Harness<'_, Moonglow>, i: usize| -> String {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let w = &git.root.list("WaypointList").unwrap()[i];
        String::from_utf8_lossy(w.string("Tag").unwrap()).into_owned()
    };
    // A double click on the second waypoint opens its Properties.
    let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    h.run_steps(40);
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
    }
    h.run_steps(3);
    let path = mg_edit::GffPath::root().item("WaypointList", 1);
    let tab = Tab::Instance { area, path };
    assert!(h.state().dock.find_tab(&tab).is_some(), "the Properties tab");
    // Its tag, edited there, changes that waypoint only: one command.
    let tag = tag_of(&mut h, 1);
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(&tag).find(is_input).expect("the tag field").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&tag).find(is_input).unwrap().type_text("WP_PLACED");
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
    assert_eq!(tag_of(&mut h, 1), "WP_PLACED");
    assert_eq!(tag_of(&mut h, 0), tag, "the other is untouched");
    // Blueprint-only fields are not offered.
    assert!(h.query_by_label("Comments").is_none());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(tag_of(&mut h, 1), tag);
}

#[test]
fn area_properties_edit_the_area() {
    let Some((mut h, area)) = area_harness("area-props") else { return };
    let are = |h: &mut Harness<'_, Moonglow>| -> mg_gff::Struct {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().root.clone()
    };
    h.get_by_label("ℹ Area Properties").click();
    h.run_steps(2);
    assert!(h.state().dock.find_tab(&Tab::AreaProperties(area)).is_some());

    // Advanced: the tag.
    h.get_by_label("Advanced").click();
    h.run_steps(2);
    let tag = String::from_utf8_lossy(are(&mut h).string("Tag").unwrap()).into_owned();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(&tag).find(is_input).expect("the tag field").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&tag).find(is_input).unwrap().type_text("FIELD_TAG");
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(2);
    assert_eq!(are(&mut h).string("Tag"), Some(&b"FIELD_TAG"[..]));
    // A shader flag past the three, the three kept.
    let flags = are(&mut h).integer("Flags").unwrap();
    h.get_by_label("8").click();
    h.run_steps(2);
    assert_eq!(are(&mut h).integer("Flags"), Some(flags | 8));

    // Visual: a lighting scheme sets the lighting and the tiles' lights in
    // one command.
    h.get_by_label("Visual").click();
    h.run_steps(2);
    let game_row = 2; // environment.2da ExteriorDark
    let name = {
        let game = h.state().game.as_deref().unwrap();
        let t = game.table("environment").unwrap();
        let strref = t.get(game_row, "STRREF").unwrap().trim().parse::<u32>().unwrap();
        game.string(mg_core::StrRef(strref)).unwrap()
    };
    h.get_by_label(&name).click();
    h.run_steps(3);
    let a = are(&mut h);
    assert_eq!(a.integer("LightingScheme"), Some(game_row as i64));
    assert_eq!(a.integer("SunAmbientColor"), Some(0x32_3A_3C), "ExteriorDark's 60, 58, 50");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Lighting scheme"));
    // Always dark.
    h.get_by_label("Always Dark").click();
    h.run_steps(2);
    let a = are(&mut h);
    assert_eq!((a.integer("DayNightCycle"), a.integer("IsNight")), (Some(0), Some(1)));
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-props").join("visual.png")).unwrap();
}

#[test]
fn adjust_location_and_find_instance() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("adjust") else { return };
    let waypoint = |h: &mut Harness<'_, Moonglow>, i: usize| -> mg_gff::Struct {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").unwrap()[i].clone()
    };
    // Select the first waypoint; Adjust Location from its context menu.
    let at = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    h.hover_at(at);
    let right = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(right(true));
    h.event(right(false));
    h.run_steps(3);
    h.get_by_label("Adjust Location…").click();
    h.run_steps(3);
    let adjust = h.state_mut().adjust.as_mut().expect("the window");
    assert_eq!(adjust.objects, [(ObjectKind::Waypoint, 0)]);
    assert!((adjust.position.x - 20.0).abs() < 1e-3);
    // X to 12.5 and turned to face west (bearing 90): only those change.
    adjust.position.x = 12.5;
    adjust.bearing = 90.0;
    adjust.changed = [true, false, false, true];
    let before = waypoint(&mut h, 0);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let after = waypoint(&mut h, 0);
    assert_eq!(after.float("XPosition"), Some(12.5));
    assert_eq!(after.float("YPosition"), before.float("YPosition"));
    assert!((after.float("XOrientation").unwrap() + 1.0).abs() < 1e-5, "facing west");
    assert!(h.state().adjust.is_none());
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Adjust location"));

    // Find Instance: waypoints by tag, then go to one.
    let tag = String::from_utf8_lossy(waypoint(&mut h, 1).string("Tag").unwrap()).into_owned();
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Find Instance…").click();
    h.run_steps(2);
    type_into_hint(&mut h, "tag", &tag.to_lowercase());
    h.run_steps(2);
    h.get_by_label("Search").click();
    h.run_steps(2);
    let found = h.state().find_instance.as_ref().unwrap().results.clone();
    assert_eq!(found.len(), 2, "both waypoints share the blueprint's tag");
    assert!(found.iter().all(|f| f.kind == ObjectKind::Waypoint && f.area == area));
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    let second = h.get_all_by_label(&found[1].template).nth(1).expect("the second result").rect();
    h.run_steps(40);
    h.hover_at(second.center());
    for _ in 0..2 {
        press(&h, second.center(), true, egui::Modifiers::NONE);
        press(&h, second.center(), false, egui::Modifiers::NONE);
    }
    h.run_steps(4);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 1)]);
}

#[test]
fn copy_cut_and_paste_objects() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("paste") else { return };
    let waypoints = |h: &mut Harness<'_, Moonglow>| -> Vec<(f32, f32)> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root
            .list("WaypointList")
            .unwrap()
            .iter()
            .map(|w| (w.float("XPosition").unwrap(), w.float("YPosition").unwrap()))
            .collect()
    };
    // Both waypoints, copied.
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    let over = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(over);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    assert_eq!(h.state().object_clip.as_ref().map(|c| c.objects.len()), Some(2));
    // Pasted with the first at (15, 30): the second 10 m east of it.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting);
    h.hover_at(over);
    press(&h, over, true, egui::Modifiers::NONE);
    press(&h, over, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let w = waypoints(&mut h);
    assert_eq!(w.len(), 4);
    let close = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.1 && (a.1 - b.1).abs() < 0.1;
    assert!(close(w[2], (15.0, 30.0)) && close(w[3], (25.0, 30.0)), "{w:?}");
    assert_eq!(
        h.state().area_views[&area].selection,
        [(ObjectKind::Waypoint, 2), (ObjectKind::Waypoint, 3)],
        "the copies are selected"
    );
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"));
    // Cut: the copies go (into the clipboard).
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::X);
    h.run_steps(2);
    assert_eq!(waypoints(&mut h).len(), 2);
    assert_eq!(h.state().object_clip.as_ref().map(|c| c.objects.len()), Some(2));
}

#[test]
fn context_menu_sets_states_mutes_and_adds_spawn_points() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("menu") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    // A chest, a sound and an encounter, placed.
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let square = [[-2.0, -2.0, 0.0], [2.0, -2.0, 0.0], [2.0, 2.0, 0.0], [-2.0, 2.0, 0.0]];
        let mut edits = Vec::new();
        for (name, t, x, outline) in [
            ("plc_chest1", ResType::UTP, 12.0, &[][..]),
            ("animalcriesday", ResType::UTS, 28.0, &[][..]),
            ("nw_verminbeet", ResType::UTE, 20.0, &square[..]),
        ] {
            let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
            let item = instance(&placing, t, &read(name, t), at, outline).unwrap();
            let (list, _) = mg_module::instances::git_list(t).unwrap();
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: list.into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let first = |h: &mut Harness<'_, Moonglow>, list: &str| -> mg_gff::Struct {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list(list).unwrap()[0].clone()
    };
    let right_click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    // The chest: Initial State › Opened.
    right_click(&mut h, Vec3::new(12.0, 30.0, 0.3));
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Placeable, 0)]);
    h.get_by_label_contains("Initial State").hover();
    h.run_steps(3);
    h.get_by_label("Opened").click();
    h.run_steps(3);
    assert_eq!(first(&mut h, "Placeable List").integer("AnimationState"), Some(1));
    // Inventory opens its Properties at the inventory page.
    right_click(&mut h, Vec3::new(12.0, 30.0, 0.3));
    h.get_by_label("Inventory").click();
    h.run_steps(3);
    let path = mg_edit::GffPath::root().item("Placeable List", 0);
    assert!(h.state().dock.find_tab(&Tab::Instance { area, path: path.clone() }).is_some());
    assert_eq!(h.state().blueprint_pages.get(&(git_key, path)), Some(&"Inventory"));
    close_windows(&mut h);
    h.run_steps(3);
    // The sound: Mute.
    right_click(&mut h, Vec3::new(28.0, 30.0, 1.9));
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Sound, 0)]);
    h.get_by_label("Mute").click();
    h.run_steps(3);
    assert_eq!(first(&mut h, "SoundList").integer("Active"), Some(0));
    // The encounter: a spawn point where the menu was opened.
    right_click(&mut h, Vec3::new(21.0, 31.0, 0.0));
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Encounter, 0)]);
    h.get_by_label("Add Spawn Point").click();
    h.run_steps(3);
    let spawns = first(&mut h, "Encounter List").list("SpawnPointList").unwrap().to_vec();
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].id, 2);
    assert!((spawns[0].float("X").unwrap() - 21.0).abs() < 0.1);
    assert!((spawns[0].float("Y").unwrap() - 31.0).abs() < 0.1);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-menu").join("area_view.png")).unwrap();
}

#[test]
fn doors_go_on_door_hooks() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    // A castle tile with three door hooks (east one at 20, 15, facing 270°).
    let Some((mut h, area)) = area_harness_on("doors", "tic01", Some(7)) else { return };
    let doors = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("Door List").unwrap_or(&[]).to_vec()
    };
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    // Far from any hook: nothing, and a word why.
    h.state_mut().palette.selected = ResKey::parse("nw_door_normal", ResType::UTD);
    h.run_steps(1);
    click(&mut h, Vec3::new(35.0, 35.0, 0.0));
    assert!(doors(&mut h).is_empty());
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("door hooks")));
    // Near the east hook, inside the tile: on the hook, turned as it is.
    click(&mut h, Vec3::new(18.5, 15.5, 0.0));
    let placed = doors(&mut h);
    assert_eq!(placed.len(), 1);
    let d = &placed[0];
    assert_eq!((d.float("X"), d.float("Y"), d.float("Z")), (Some(20.0), Some(15.0), Some(0.0)));
    assert!((d.float("Bearing").unwrap() + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Door, 0)]);
}

#[test]
fn add_to_palette_create_waypoint_and_set() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("palette-add") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let items = |r: ResRef| {
            Gff::read(&game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?).ok().map(|g| g.root)
        };
        let placing = Placing { game, item: &items };
        let mut edits = Vec::new();
        for (name, t, x) in
            [("plc_chest1", ResType::UTP, 12.0), ("nw_bandit001", ResType::UTC, 28.0)]
        {
            let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
            let item = instance(&placing, t, &read(name, t), at, &[]).unwrap();
            let (list, _) = mg_module::instances::git_list(t).unwrap();
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: list.into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let right_click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    let git = |h: &mut Harness<'_, Moonglow>| -> mg_gff::Struct {
        h.state_mut().ws.as_mut().unwrap().doc(&git_key).unwrap().root.clone()
    };

    // Add to Palette on the chest: a new blueprint, which the chest names.
    right_click(&mut h, Vec3::new(12.0, 30.0, 0.3));
    h.get_by_label("Add to Palette").click();
    h.run_steps(3);
    let chest = git(&mut h).list("Placeable List").unwrap()[0].clone();
    let new = chest.resref("TemplateResRef").unwrap();
    assert!(new.to_string().starts_with("plc_chest"), "{new}");
    let key = ResKey::new(new, ResType::UTP);
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key), "the new blueprint");
    assert!(h.state().dock.find_tab(&Tab::Blueprint(key)).is_some(), "its editor opens");

    // Create Waypoint on the bandit, where the menu was opened.
    close_windows(&mut h);
    h.run_steps(3);
    right_click(&mut h, Vec3::new(28.0, 30.0, 0.9));
    h.get_by_label("Create Waypoint").click();
    h.run_steps(3);
    let tag = String::from_utf8_lossy(
        git(&mut h).list("Creature List").unwrap()[0].string("Tag").unwrap(),
    )
    .into_owned();
    let waypoints = git(&mut h).list("WaypointList").unwrap().to_vec();
    assert_eq!(waypoints.len(), 3);
    assert_eq!(waypoints[2].string("Tag").unwrap(), format!("WP_{tag}_01").as_bytes());

    // Create Set on the first two waypoints: Patrol_01, Patrol_02.
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    right_click(&mut h, Vec3::new(20.0, 20.0, 0.9));
    h.get_by_label("Create Set…").click();
    h.run_steps(2);
    type_into_hint(&mut h, "set name", "Patrol");
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let waypoints = git(&mut h).list("WaypointList").unwrap().to_vec();
    assert_eq!(waypoints[0].string("Tag"), Some(&b"Patrol_01"[..]));
    assert_eq!(waypoints[1].string("Tag"), Some(&b"Patrol_02"[..]));
}

#[test]
fn several_objects_edited_together() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("multi") else { return };
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    let at = screen(&h, area, Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Properties").click();
    h.run_steps(3);
    let paths = vec![
        mg_edit::GffPath::root().item("WaypointList", 0),
        mg_edit::GffPath::root().item("WaypointList", 1),
    ];
    assert!(h.state().dock.find_tab(&Tab::Instances { area, paths }).is_some());
    h.get_by_label_contains("what you change is set on each");
    h.get_by_label("Advanced").click();
    h.run_steps(2);
    h.get_by_label("Waypoint Contains a Map Note").click();
    h.run_steps(3);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap().root.clone();
    let notes: Vec<_> =
        git.list("WaypointList").unwrap().iter().map(|w| w.integer("HasMapNote")).collect();
    assert_eq!(notes, [Some(1), Some(1)], "both");
    assert_eq!(ws.can_undo(), Some("Waypoint Contains a Map Note"));
}

#[test]
fn preview_window_shows_the_chosen_blueprint() {
    let Some((mut h, _area)) = area_harness("preview") else { return };
    h.get_by_label("👁 Preview").click();
    h.run_steps(2);
    h.get_by_label("Choose a blueprint in the palette.");
    // A waypoint: its fields.
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    h.run_steps(2);
    h.get_by_label("Blueprint ResRef");
    h.get_by_label("nw_waypoint001");
    // A chest: also in 3D.
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.state_mut().palette.selected = Some(chest);
    h.run_steps(3);
    h.get_by_label("plc_chest1");
    assert!(
        h.state().model_views.contains_key(&mg_ui::model_view::Source::Resource(chest)),
        "the 3D view"
    );
    // An item: also its inventory icon.
    h.state_mut().palette.selected = ResKey::parse("nw_wswls001", ResType::UTI);
    h.run_steps(3);
    h.get_by_label("Item icon");
}

#[test]
fn placed_chest_holds_whole_items() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("held") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let data = game.resman.get(&ResKey::parse("plc_chest1", ResType::UTP).unwrap()).unwrap();
        let mut chest = Gff::read(&data).unwrap().root;
        chest.set("HasInventory", mg_gff::Value::Byte(1));
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [12.0, 30.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
        let path = mg_edit::GffPath::root().item("Placeable List", 0);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path }));
    }
    h.run_steps(3);
    h.get_by_label("Inventory…").click();
    h.run_steps(2);
    // The chest's item palette, not the Palettes pane's.
    close_tab(h.state_mut(), &Tab::Palette);
    h.run_steps(2);
    type_into_hint(&mut h, "Find", "nw_it_torch001");
    let torch = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button && n.label().is_some_and(|l| l == "Torch")
    };
    h.get(egui_kittest::kittest::by().predicate(torch)).click();
    h.run_steps(2);
    for _ in 0..2 {
        h.get_by_label("Add Item").click();
        h.run_steps(2);
    }
    let ws = h.state_mut().ws.as_mut().unwrap();
    let chest = ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[0].clone();
    let items = chest.list("ItemList").unwrap();
    let at: Vec<_> = items
        .iter()
        .map(|i| {
            (
                i.id,
                i.resref("TemplateResRef").unwrap().to_string(),
                i.integer("BaseItem").is_some(),
                i.integer("Repos_PosX").unwrap(),
                i.float("XPosition"),
            )
        })
        .collect();
    let whole = |id, x| (id, "nw_it_torch001".to_string(), true, x, Some(-1.0));
    assert_eq!(at, [whole(0, 0), whole(1, 1)], "whole items, as Aurora holds them");
    assert!(h.query_all_by_label("Torch").count() >= 2, "named by the items themselves");
}

#[test]
fn the_crosser_cursor_lies_on_raised_ground_under_the_pointer() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("crosser-cursor") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    // Corners (1, 1) and (1, 2) raised twice: ground well above the tiles'
    // own corners nearby.
    h.get_by_label("↕ Raise/Lower").click();
    h.run_steps(2);
    for corner in [(10.0, 10.0), (10.0, 20.0)] {
        for _ in 0..2 {
            let at = screen(&h, area, Vec3::new(corner.0, corner.1, 0.0));
            h.hover_at(at);
            press(&h, at, true, egui::Modifiers::NONE);
            press(&h, at, false, egui::Modifiers::NONE);
            h.run_steps(3);
        }
    }
    h.get_by_label("Road").click();
    h.run_steps(2);
    // Wherever the pointer rests on raised ground, the quarter drawn for it
    // lies on that ground.
    let rect = h.state().area_views[&area].rect;
    let mut checked = 0;
    for i in 1..12 {
        for j in 1..12 {
            let at = rect.min + rect.size() * egui::vec2(i as f32 / 12.0, j as f32 / 12.0);
            h.hover_at(at);
            h.run_steps(3);
            let view = &h.state().area_views[&area];
            let (Some(pointer), [(triangle, _)]) = (view.pointer, view.brush_cursor.as_slice())
            else {
                continue;
            };
            if pointer.z < 4.0 {
                continue;
            }
            // (Its corners may stand lower or higher on a slope or a cliff;
            // drawn at the tile corners' lattice heights, it was 10 m off.)
            let mean = triangle.iter().map(|p| p.z).sum::<f32>() / 3.0;
            assert!(
                (mean - pointer.z).abs() < 5.0,
                "the pointer at {at:?} is on the ground at {pointer}, the cursor at {triangle:?}"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no point on raised ground was found");
}

#[test]
fn area_viewer_paints_terrain() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("terrain") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    let button = |h: &mut Harness<'_, Moonglow>, p: Vec3, b: egui::PointerButton| {
        let at = screen(h, area, p);
        h.hover_at(at);
        for pressed in [true, false] {
            let m = egui::Modifiers::NONE;
            h.event(egui::Event::PointerButton { pos: at, button: b, pressed, modifiers: m });
        }
        h.run_steps(2);
    };
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| button(h, p, egui::PointerButton::Primary);
    let water = index.terrain("Water").unwrap();

    // The Tiles palette: the area's tileset, its Terrain branch open.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    h.get_by_label("Water").click();
    h.run_steps(2);
    h.get_by_label_contains("Water: click a corner");

    // Water at corner (2, 2): the four tiles around it are a pond.
    click(&mut h, Vec3::new(20.5, 19.5, 0.0));
    h.run_steps(2);
    assert_eq!(lattice(&mut h).corner(2, 2).terrain, water);
    // One command: undone, the grass is back.
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(2);
    assert_ne!(lattice(&mut h).corner(2, 2).terrain, water);

    // Shift + click with Grass on a grass corner: the four tiles around it
    // step to their next variants; the terrain stays.
    h.get_by_label("Grass").click();
    h.run_steps(2);
    let tiles = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let before = tiles(&mut h);
    let at = screen(&h, area, Vec3::new(30.0, 30.0, 0.0));
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::SHIFT);
    press(&h, at, false, egui::Modifiers::SHIFT);
    h.run_steps(2);
    let after = tiles(&mut h);
    assert_eq!(after.lattice, before.lattice);
    let moved = [(2, 2), (3, 2), (2, 3), (3, 3)]
        .iter()
        .filter(|&&(x, y)| after.tile(x, y) != before.tile(x, y))
        .count();
    assert!(moved > 0, "the tiles step to their next variants");

    // Raise and lower a corner.
    h.get_by_label("↕ Raise/Lower").click();
    h.run_steps(2);
    click(&mut h, Vec3::new(10.0, 30.0, 0.0));
    assert_eq!(lattice(&mut h).corner(1, 3).height, 1);
    button(&mut h, Vec3::new(10.0, 30.0, 5.0), egui::PointerButton::Secondary);
    assert_eq!(lattice(&mut h).corner(1, 3).height, 0);

    // A road dragged from tile (0, 0) to (2, 0), starting in the east
    // quarter of the first: on the two edges crossed.
    h.get_by_label("Road").click();
    h.run_steps(2);
    let from = screen(&h, area, Vec3::new(7.0, 5.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for x in [9.0, 12.0, 15.0, 18.0, 21.0, 24.0, 25.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 5.0, 0.0)));
        h.run_steps(1);
    }
    let to = screen(&h, area, Vec3::new(25.0, 5.0, 0.0));
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(2);
    let road = index.crosser("Road").unwrap();
    let l = lattice(&mut h);
    let edges: Vec<_> = (0..4).map(|x| l.cell(x, 0).edges).collect();
    assert_eq!(edges[0], [None, Some(road), None, None]);
    assert_eq!(edges[1], [None, Some(road), None, Some(road)]);
    assert_eq!(edges[2], [None, None, None, Some(road)]);

    // Trees beside the straight road: no tile has that, nothing changes.
    h.get_by_label("Trees").click();
    h.run_steps(2);
    let before = lattice(&mut h);
    click(&mut h, Vec3::new(20.0, 10.0, 0.0));
    assert_eq!(lattice(&mut h), before);
    h.get_by_label_contains("no tile fits there");

    // The Eraser on the road's middle tile takes the road away.
    h.get_by_label("🗑 Eraser").click();
    h.run_steps(2);
    click(&mut h, Vec3::new(15.0, 5.0, 0.0));
    let l = lattice(&mut h);
    assert!((0..4).all(|x| l.cell(x, 0).edges == [None; 4]), "{:?}", l.cell(0, 0));

    // A group (Barn 1: tiles 145, 147 / 144, 146 from the south-west),
    // turned a quarter by a right click, its first tile where clicked.
    h.get_by_label("Groups").click();
    h.run_steps(2);
    h.get_by_label("Barn 1 2x2").click();
    h.run_steps(2);
    button(&mut h, Vec3::new(25.0, 15.0, 0.0), egui::PointerButton::Secondary);
    click(&mut h, Vec3::new(25.0, 15.0, 0.0));
    let tiles = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let g = tiles(&mut h);
    let at = |x: u32, y: u32| (g.tile(x, y).tile, g.tile(x, y).orientation);
    assert_eq!([at(2, 1), at(2, 2), at(1, 1), at(1, 2)], [(145, 1), (147, 1), (144, 1), (146, 1)]);
    assert!(h.state().palette.tile_brush.is_some(), "the group stays chosen");
}

/// A screenshot of terrain mode (`target/test-output/screens/terrain.png`),
/// for looking at: `cargo test -p mg-ui --test app terrain_screen -- --ignored`.
#[test]
#[ignore]
fn terrain_screen() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("terrain-screen") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("Tiles").click();
    h.run_steps(2);
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    for (brush, at) in [
        ("Water", [10.0, 20.0]),
        ("Water", [10.0, 30.0]),
        ("Trees", [30.0, 30.0]),
        ("↕ Raise/Lower", [40.0, 30.0]),
    ] {
        h.get_by_label(brush).click();
        h.run_steps(2);
        click(&mut h, Vec3::new(at[0], at[1], 0.0));
    }
    h.get_by_label("Road").click();
    h.run_steps(2);
    let from = screen(&h, area, Vec3::new(7.0, 5.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    for x in [10.0, 15.0, 20.0, 22.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 5.0, 0.0)));
        h.run_steps(1);
    }
    let to = screen(&h, area, Vec3::new(22.0, 5.0, 0.0));
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.get_by_label("Water").click();
    h.run_steps(2);
    h.hover_at(screen(&h, area, Vec3::new(20.0, 20.0, 0.0)));
    h.run_steps(4);
    let dir = mg_testkit::scratch_dir("screens-terrain");
    let image = h.render().expect("render");
    let out = dir.parent().unwrap().join("screens");
    std::fs::create_dir_all(&out).unwrap();
    image.save(out.join("terrain.png")).unwrap();
}

#[test]
fn area_viewer_selects_tiles_and_sets_their_properties() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness_on("tiles", "tic01", None) else { return };
    let tile = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[i].clone()
    };
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    // The room in the middle: tile (1, 1), index 5.
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].tile_selection, [(1, 1)]);
    let original = tile(&mut h, 5);
    // Right click: the tile menu, Tile Properties.
    let button = egui::PointerButton::Secondary;
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(2);
    h.get_by_label("Tile Properties…").click();
    h.run_steps(2);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!(props.tiles, [5]);
    // Main light 1 to colour 9, through the colour picker.
    h.state_mut().tile_props.as_mut().unwrap().main[0] = 9;
    h.state_mut().tile_props.as_mut().unwrap().loops = [false, false, false];
    // The replacement texture: replacetexture.2da row 1.
    h.state_mut().tile_props.as_mut().unwrap().replace = Some(1);
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(2);
    let t = tile(&mut h, 5);
    assert_eq!(t.integer("Tile_ReplaceTex"), Some(1));
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    assert_eq!(model.tiles[5].replace_texture.as_deref(), Some("t_flag02"));
    if props.has_main[0] {
        assert_eq!(t.integer("Tile_MainLight1"), Some(9));
    }
    if props.has_loops[0] {
        assert_eq!(t.integer("Tile_AnimLoop1"), Some(0));
    }
    assert!(h.state().tile_props.is_none());
    // Undone in one step.
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(1);
    assert_eq!(tile(&mut h, 5), original);
}

#[test]
fn resize_and_rotate_area_from_the_edit_menu() {
    let Some((mut h, area)) = area_harness("reshape") else { return };
    let size = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        let tiles = are.root.list("Tile_List").unwrap().len();
        (are.root.integer("Width").unwrap(), are.root.integer("Height").unwrap(), tiles)
    };
    assert_eq!(size(&mut h), (4, 4, 16));
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Resize Area…").click();
    h.run_steps(2);
    let d = h.state_mut().resize_area.as_mut().expect("the window is open");
    (d.columns, d.rows) = (6, 3);
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(size(&mut h), (6, 3, 18));
    // The waypoints at (20, 20) and (30, 20) stay: inside 60 by 30.
    let waypoints = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root
            .list("WaypointList")
            .unwrap()
            .iter()
            .map(|w| (w.float("XPosition").unwrap(), w.float("YPosition").unwrap()))
            .collect::<Vec<_>>()
    };
    assert_eq!(waypoints(&mut h).len(), 2);
    // A quarter turn counter-clockwise: 3 by 6, (20, 20) to (10, 20).
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Rotate Area…").click();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(size(&mut h), (3, 6, 18));
    let w = waypoints(&mut h);
    assert!((w[0].0 - 10.0).abs() < 1e-3 && (w[0].1 - 20.0).abs() < 1e-3, "{w:?}");
    // Both undone, one step each.
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(1);
    assert_eq!(size(&mut h), (4, 4, 16));
}

#[test]
fn walkmesh_overlay_and_area_statistics() {
    let Some((mut h, area)) = area_harness("stats") else { return };
    h.get_by_label("👣 Walkmesh").click();
    h.run_steps(3);
    assert!(h.state().area_views[&area].walkmesh);
    h.get_by_label("Build").click();
    h.run_steps(2);
    h.get_by_label("Area Statistics").click();
    h.run_steps(3);
    h.get_by_label("Resources Used");
    h.get_by_label("Triangles");
    h.get_by_label("16 (4 by 4)");
    h.get_by_label("Done").click();
    h.run_steps(2);
    assert!(h.state().area_stats.is_none());
    if std::env::var_os("MOONGLOW_SCREENS").is_some() {
        let image = h.render().expect("render");
        let out = mg_testkit::scratch_dir("screens-walkmesh").parent().unwrap().join("screens");
        std::fs::create_dir_all(&out).unwrap();
        image.save(out.join("walkmesh.png")).unwrap();
    }
}

#[test]
fn tiles_copy_and_paste() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness_on("tile-paste", "tic01", None) else { return };
    let tile = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[i].clone()
    };
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    // Tile (1, 1): the room's south-west quarter, a Stone corner at its
    // north-east. Copied, pasted with its south-west tile at (0, 0).
    click(&mut h, Vec3::new(15.0, 15.0, 0.0));
    let copied = tile(&mut h, 5);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    assert!(h.state().tile_clip.is_some());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(1);
    assert!(h.state().area_views[&area].tile_pasting);
    click(&mut h, Vec3::new(5.0, 5.0, 0.0));
    h.run_steps(2);
    assert!(!h.state().area_views[&area].tile_pasting);
    let pasted = tile(&mut h, 0);
    for label in ["Tile_ID", "Tile_Orientation", "Tile_Height", "Tile_MainLight1", "Tile_SrcLight1"]
    {
        assert_eq!(pasted.get(label), copied.get(label), "{label}");
    }
    // The tiles around fit it: the area's tiles make one lattice.
    let game = h.state().game.as_deref().unwrap();
    let set = mg_area::tileset(game, ResRef::from_str("tic01").unwrap()).unwrap();
    let index = mg_tiles::TileIndex::new(&set);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
    let g = mg_area::terrain::grid(&are.root, &index).unwrap();
    let (_, bad) = mg_tiles::Lattice::from_tiles(&index, 4, 4, &g.tiles).unwrap();
    assert!(bad.is_empty(), "tiles disagree at {bad:?}");
}

#[test]
fn update_instances_remakes_placed_sounds() {
    use mg_edit::{Command, Edit, GffPath};
    let Some((mut h, area)) = area_harness("update-instances") else { return };
    // A custom sound blueprint (a copy of hawkcry) placed twice, then its
    // volume changed.
    let game = h.state().game.as_deref().unwrap();
    let data = game.resman.get(&ResKey::parse("hawkcry", ResType::UTS).unwrap()).unwrap();
    let mut bp = Gff::read(&data).unwrap();
    let key = ResKey::parse("mg_hawk", ResType::UTS).unwrap();
    bp.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
    let none = |_: ResRef| None;
    let placing = mg_module::instances::Placing { game, item: &none };
    let placed: Vec<mg_gff::Struct> = [[10.0, 10.0], [30.0, 20.0]]
        .map(|p| {
            let at = mg_module::instances::Placement { position: [p[0], p[1], 1.5], rotation: 0.0 };
            mg_module::instances::instance(&placing, ResType::UTS, &bp.root, at, &[]).unwrap()
        })
        .to_vec();
    let bytes = bp.to_bytes().unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.apply(Command::new(
        "setup",
        vec![
            Edit::SetResource { key, data: Some(bytes) },
            Edit::SetField {
                key: git,
                path: GffPath::root(),
                label: "SoundList".into(),
                value: Some(mg_gff::Value::List(placed)),
            },
            Edit::SetField {
                key,
                path: GffPath::root(),
                label: "Volume".into(),
                value: Some(mg_gff::Value::Byte(17)),
            },
        ],
    ))
    .unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::for_resource(key).unwrap()));
    h.run_steps(3);
    h.get_by_label("Advanced").click();
    h.run_steps(2);
    h.get_by_label("Update Instances").click();
    h.run_steps(3);
    // The window lists both, every area chosen.
    let draft = h.state().update_draft.clone().expect("the Update Instances window");
    assert_eq!(draft.objects.len(), 2);
    h.get_by_label("Update 2").click();
    h.run_steps(3);
    assert!(h.state().update_draft.is_none());
    let ws = h.state_mut().ws.as_mut().unwrap();
    let sounds = ws.doc(&git).unwrap().root.list("SoundList").unwrap().to_vec();
    assert_eq!(sounds.len(), 2);
    for (s, p) in sounds.iter().zip([[10.0, 10.0], [30.0, 20.0]]) {
        assert_eq!(s.integer("Volume"), Some(17), "made again from the blueprint");
        assert_eq!(
            (s.float("XPosition"), s.float("YPosition")),
            (Some(p[0]), Some(p[1])),
            "where it stood"
        );
    }
}

#[test]
fn palette_updates_the_instances_of_a_selection_and_a_category() {
    use mg_edit::{Command, Edit};
    use mg_module::instances::{Placement, Placing, instance};
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-bulk-update");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    // Two trigger blueprints, each placed once with an outline.
    let game = app.game.as_deref().unwrap();
    let base = Gff::read(&game.resman.get_named("trackstrigger", ResType::UTT).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let mut edits = Vec::new();
    let mut placed = Vec::new();
    for (i, name) in ["mg_trig_a", "mg_trig_b"].into_iter().enumerate() {
        let key = ResKey::parse(name, ResType::UTT).unwrap();
        let mut bp = base.clone();
        bp.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
        let at = Placement { position: [10.0 + 10.0 * i as f32, 10.0, 0.0], rotation: 0.0 };
        let outline = [[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 3.0, 0.0]];
        placed.push(instance(&placing, ResType::UTT, &bp.root, at, &outline).unwrap());
        // The blueprint changes after placing.
        bp.root.set("Tag", mg_gff::Value::String(format!("NEW_{i}").into_bytes()));
        edits.push(Edit::SetResource { key, data: Some(bp.to_bytes().unwrap()) });
    }
    let mut git = Gff::new(*b"GIT ");
    git.root.set("TriggerList", mg_gff::Value::List(placed));
    let git_key = ResKey::parse("start", ResType::GIT).unwrap();
    edits.push(Edit::SetResource { key: git_key, data: Some(git.to_bytes().unwrap()) });
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Trigger;
    app.palette.tiles = false;
    app.palette.custom = true;
    app.palette.filter = "mg_trig".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let triggers = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list("TriggerList").unwrap().to_vec()
    };
    let tag = |s: &mg_gff::Struct| String::from_utf8_lossy(s.string("Tag").unwrap()).into_owned();
    let before = triggers(&mut h);
    // Both chosen (Ctrl+click), the second object unticked: one updated.
    // The blueprints' name in the palette, and their category's title.
    let (a, title) = {
        let app = h.state();
        let game = app.game.as_deref().unwrap();
        let kind = mg_module::palette::BlueprintKind::Trigger;
        let gff = mg_module::palette::rebuild_custom_palette(
            &app.ws.as_ref().unwrap().module,
            game,
            kind,
        )
        .unwrap();
        let palette = mg_module::palette::Palette::read(&gff);
        fn find(
            nodes: &[mg_module::palette::PaletteNode],
        ) -> Option<&mg_module::palette::PaletteNode> {
            nodes.iter().find_map(|n| {
                if n.blueprints.iter().any(|b| b.resref.to_string() == "mg_trig_a") {
                    Some(n)
                } else {
                    find(&n.children)
                }
            })
        }
        let category = find(&palette.nodes).unwrap();
        let names: Vec<String> = category.blueprints.iter().map(|b| b.name.text(game)).collect();
        assert_eq!(names[0], names[1], "copies of one blueprint share its name");
        let title = format!("{} ({})", category.name.text(game), category.blueprints.len());
        (names[0].clone(), title)
    };
    let buttons: Vec<_> = h.get_all_by_label(&a).collect();
    assert_eq!(buttons.len(), 2);
    buttons[0].click();
    h.run();
    let buttons: Vec<_> = h.get_all_by_label(&a).collect();
    buttons[1].click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    assert_eq!(h.state().palette.chosen.len(), 1);
    h.get_all_by_label(&a).next().unwrap().click_secondary();
    h.run();
    h.get_by_label("Update Instances of 2").click();
    h.run();
    let draft = h.state().update_draft.clone().expect("the window");
    assert_eq!(draft.objects.len(), 2);
    h.get_all_by_label_contains("(mg_trig_b)").next().unwrap().click();
    h.run();
    h.get_by_label("Update 1").click();
    h.run();
    let after = triggers(&mut h);
    assert_eq!((tag(&after[0]), tag(&after[1])), ("NEW_0".to_string(), tag(&before[1])));
    // Its outline and place are kept.
    assert_eq!(after[0].get("Geometry"), before[0].get("Geometry"));
    assert_eq!(after[0].float("XPosition"), Some(10.0));
    // The whole category, from its menu.
    h.get_by_label(&title).click_secondary();
    h.run();
    h.get_by_label("Update Instances").click();
    h.run();
    h.get_by_label("Update 2").click();
    h.run();
    let after = triggers(&mut h);
    assert_eq!((tag(&after[0]), tag(&after[1])), ("NEW_0".to_string(), "NEW_1".to_string()));
    // One undo step.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(tag(&triggers(&mut h)[1]), tag(&before[1]));
}

#[test]
fn several_blueprints_are_edited_together() {
    use mg_edit::{Command, Edit};
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-edit-together");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let game = app.game.as_deref().unwrap();
    let base = Gff::read(&game.resman.get_named("plc_chest1", ResType::UTP).unwrap()).unwrap();
    let keys: Vec<ResKey> =
        ["mg_box_a", "mg_box_b"].iter().map(|n| ResKey::parse(n, ResType::UTP).unwrap()).collect();
    let edits = keys
        .iter()
        .map(|k| {
            let mut g = base.clone();
            g.root.set("TemplateResRef", mg_gff::Value::resref(k.resref));
            Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
    app.palette.tiles = false;
    app.palette.custom = true;
    app.palette.filter = "mg_box".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let name = {
        let game = h.state().game.as_deref().unwrap();
        mg_module::palette::blueprint_name(
            mg_module::palette::BlueprintKind::Placeable,
            &base.root,
            game,
        )
    };
    let buttons: Vec<_> = h.get_all_by_label(&name).collect();
    assert_eq!(buttons.len(), 2);
    buttons[0].click();
    h.run();
    h.get_all_by_label(&name).nth(1).unwrap().click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    h.get_all_by_label(&name).next().unwrap().click_secondary();
    h.run();
    h.get_by_label("Edit 2 Together").click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Blueprints(keys.clone())).is_some());
    // Lists are edited one at a time: no Inventory page.
    assert!(h.query_by_label_contains("edited one blueprint at a time").is_some());
    assert!(h.query_by_label("Inventory").is_none());
    h.get_by_label("Plot").click();
    h.run();
    for k in &keys {
        assert_eq!(field(&mut h, k).integer("Plot"), Some(1), "{k}");
    }
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Plot"));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    for k in &keys {
        assert_eq!(field(&mut h, k).integer("Plot"), Some(0), "{k}");
    }
}

#[test]
fn variable_sets_are_saved_and_added_elsewhere() {
    let Some((mut h, key)) = blueprint_harness("nw_waypoint001", "waypoint_vars", ResType::UTW)
    else {
        return;
    };
    let sets = mg_testkit::scratch_dir("ui-var-sets").join("sets");
    h.state_mut().var_set_dir = Some(sets.clone());
    h.run();
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("Variables (0)…").click();
    h.run();
    // Two variables, kept as a set.
    {
        let edit = h.state_mut().var_edit.as_mut().expect("the Variables window");
        edit.rows.push(mg_ui::widgets::VarRow::new("MG_LOOT", 1, "3"));
        edit.rows.push(mg_ui::widgets::VarRow::new("MG_SAY", 3, "Hail"));
    }
    h.run();
    h.get_by_label("Save Set…").click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::TextInput).last().unwrap().type_text("Loot");
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert!(sets.join("Loot.vars.json").is_file());
    h.get_by_label("Cancel").click();
    h.run();
    assert!(field(&mut h, &key).list("VarTable").is_none_or(|l| l.is_empty()), "not applied");
    // Added to the variables (one already named MG_LOOT takes its value).
    h.get_by_label("Variables (0)…").click();
    h.run();
    h.state_mut()
        .var_edit
        .as_mut()
        .unwrap()
        .rows
        .push(mg_ui::widgets::VarRow::new("MG_LOOT", 1, "9"));
    h.run();
    h.get_by_label("Add Set").click();
    h.run();
    h.get_by_label("Loot").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    let vars: Vec<(String, Option<i64>)> = field(&mut h, &key)
        .list("VarTable")
        .unwrap()
        .iter()
        .map(|v| {
            (String::from_utf8_lossy(v.string("Name").unwrap()).into_owned(), v.integer("Value"))
        })
        .collect();
    assert_eq!(vars, [("MG_LOOT".to_string(), Some(3)), ("MG_SAY".to_string(), None)]);
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

#[test]
fn build_module_compiles_and_reports() {
    use mg_edit::{Command, Edit, GffPath};
    let Some((mut h, area)) = area_harness("build") else { return };
    // A script that does not compile, an encounter blueprint whose creature
    // entry has a stale CR, and a waypoint naming a missing script.
    let mut ute = Gff::new(*b"UTE ");
    ute.root.set("TemplateResRef", mg_gff::Value::resref(ResRef::from_str("mg_enc").unwrap()));
    let mut entry = mg_gff::Struct::new(0);
    entry.set("ResRef", mg_gff::Value::resref(ResRef::from_str("nw_bandit001").unwrap()));
    entry.set("CR", mg_gff::Value::Float(99.0));
    entry.set("Appearance", mg_gff::Value::Int(0));
    ute.root.set("CreatureList", mg_gff::Value::List(vec![entry]));
    let git = ResKey::new(area, ResType::GIT);
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.apply(Command::new(
        "setup",
        vec![
            Edit::SetResource {
                key: ResKey::parse("mg_bad", ResType::NSS).unwrap(),
                data: Some(b"void main() { this is not nwscript }".to_vec()),
            },
            Edit::SetResource {
                key: ResKey::parse("mg_enc", ResType::UTE).unwrap(),
                data: Some(ute.to_bytes().unwrap()),
            },
            Edit::SetField {
                key: git,
                path: GffPath::root().item("WaypointList", 0),
                label: "OnUserDefined".into(),
                value: Some(mg_gff::Value::resref(ResRef::from_str("mg_nosuchscript").unwrap())),
            },
        ],
    ))
    .unwrap();
    h.get_by_label("Build").click();
    h.run_steps(2);
    h.get_by_label("Build Module…").click();
    h.run_steps(2);
    let w = h.state().build.clone().expect("the window is open");
    assert!(w.compile && w.missing && !w.unused, "Aurora's defaults");
    h.get_all_by_label("Build").last().unwrap().click();
    h.run_steps(3);
    let results: Vec<String> =
        h.state().build.as_ref().unwrap().results.iter().map(|f| f.text.clone()).collect();
    let has = |s: &str| results.iter().any(|r| r.contains(s));
    assert!(has("Error:"), "the bad script: {results:?}");
    assert!(has("mg_nosuchscript"), "{results:?}");
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(_, m)| m.contains("1 encounter creature entries")),
        "what the passes did is logged: {log:?}"
    );
    // The encounter's entry now has the bandit's CR.
    let ws = h.state_mut().ws.as_mut().unwrap();
    let enc = ws.doc(&ResKey::parse("mg_enc", ResType::UTE).unwrap()).unwrap();
    let cr = enc.root.list("CreatureList").unwrap()[0].float("CR").unwrap();
    assert!(cr < 99.0, "CR {cr}");
}

#[test]
fn script_sets_save_and_load() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_scripts", ResType::UTC)
    else {
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-script-set");
    let path = dir.join("bandit.ini");
    h.run();
    h.get_by_label("Scripts").click();
    h.run();
    // Saved in Aurora's layout: [ResRefs], Aurora's event names.
    let spawn = field(&mut h, &key).resref("ScriptSpawn").unwrap();
    h.state_mut().dialogs =
        Box::new(mg_ui::NoDialogs { save: vec![path.clone()], ..Default::default() });
    h.get_by_label("Save Script Set").click();
    h.run();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("[ResRefs]\r\nOnBlocked="), "{text}");
    assert!(text.contains(&format!("OnSpawn={spawn}\r\n")), "{text}");
    assert!(text.contains("OnSpellCast="), "Aurora's name for OnSpellCastAt: {text}");
    // A set that names only OnSpawn: loading it sets OnSpawn and clears
    // the others, in one undoable step.
    std::fs::write(&path, "[ResRefs]\r\nOnSpawn=mg_spawn\r\n").unwrap();
    h.state_mut().dialogs =
        Box::new(mg_ui::NoDialogs { open: vec![path.clone()], ..Default::default() });
    h.get_by_label("Load Script Set").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!(s.resref("ScriptSpawn"), Some(ResRef::from_str("mg_spawn").unwrap()));
    assert_eq!(s.resref("ScriptHeartbeat"), Some(ResRef::EMPTY));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(field(&mut h, &key).resref("ScriptSpawn"), Some(spawn));
}

#[test]
fn class_spell_lists_save_clear_and_load() {
    let Some((mut h, key)) = blueprint_harness("db_tanarukk_do", "sorc_lists", ResType::UTC) else {
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-spell-list");
    let path = dir.join("sorcerer.ini");
    let known = |h: &mut Harness<'_, Moonglow>| -> Vec<Vec<i64>> {
        let classes = field(h, &key).list("ClassList").unwrap().to_vec();
        let sorcerer = classes.iter().find(|c| c.integer("Class") == Some(9)).unwrap().clone();
        (0..=9)
            .map(|l| {
                sorcerer
                    .list(&format!("KnownList{l}"))
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(|s| s.integer("Spell"))
                    .collect()
            })
            .collect()
    };
    h.run();
    h.get_by_label("Spells").click();
    h.run();
    let before = known(&mut h);
    assert!(before.iter().any(|l| !l.is_empty()), "the sorcerer knows spells");
    h.state_mut().dialogs =
        Box::new(mg_ui::NoDialogs { save: vec![path.clone()], ..Default::default() });
    h.get_by_label("Save Class Spell List").click();
    h.run();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("[Spells9]\r\nSpell000="), "{text}");
    h.get_by_label("Clear Class Spell List").click();
    h.run();
    assert!(known(&mut h).iter().all(Vec::is_empty));
    h.state_mut().dialogs =
        Box::new(mg_ui::NoDialogs { open: vec![path.clone()], ..Default::default() });
    h.get_by_label("Load Class Spell List").click();
    h.run();
    assert_eq!(known(&mut h), before, "each spell back at its level");
}

#[test]
fn saving_a_script_compiles_it_with_debug_information_when_chosen() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-auto-compile");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main()\n{\n    PrintString(\"hi\");\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    // Options › Script Editor.
    app.settings.auto_compile = true;
    app.settings.debug_info = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // An edit, then Save: the script is compiled too, with its .ndb.
    h.state_mut().script_tools.jump = Some((key, text.len()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    h.get_by_label("Save").click();
    h.run();
    let module = &h.state().ws.as_ref().unwrap().module;
    let compiled = |t| module.contains(&ResKey::new(key.resref, t));
    assert!(compiled(ResType::NCS) && compiled(ResType::NDB), "{:?}", h.state().log.entries);
}

#[test]
fn build_on_save_opens_the_results_when_something_is_wrong() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-build-on-save");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    m.set(ResKey::parse("mg_bad", ResType::NSS).unwrap(), b"void main() { nope }".to_vec());
    m.save().unwrap();
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.settings.build_on_save = true; // Options › General
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let build = h.state().build.clone().expect("the Build Module window, with the problem");
    assert!(build.results.iter().any(|f| f.text.starts_with("Error:")), "{:?}", build.results);
    assert!(h.state().log.entries.iter().any(|(_, m)| m == "Finished Building Module"));
}

#[test]
fn tab_indents_selected_lines_in_the_script_editor() {
    let dir = mg_testkit::scratch_dir("ui-script-indent");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main()\n{\n}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    // Select into the second line, then Tab: both lines indented.
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowRight);
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), "\tvoid main()\n\t{\n}\n");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), "void main()\n{\n}\n");
}

#[test]
fn setup_store_makes_the_conversation_script_and_store() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
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
    // A hostile bandit at (5, 5).
    let bp =
        Gff::read(&game.resman.get(&ResKey::parse("nw_bandit001", ResType::UTC).unwrap()).unwrap())
            .unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let bandit = instance(&placing, ResType::UTC, &bp.root, at, &[]).unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![bandit]));
    m.set_gff(git, &g).unwrap();
    let path = mg_testkit::scratch_dir("ui-setup-store").join("shop.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.open_palette = false;
    mg_ui::store_wizard::open(&mut app, area, "Creature List", 0);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Aurora's defaults; then a standard store, and Finish.
    let w = h.state().store_wizard.clone().unwrap();
    assert_eq!((w.dialog.as_str(), w.script.as_str()), ("store001", "openstore001"));
    assert!(w.hostile, "the bandit's faction");
    h.get_by_label("Next >").click();
    h.run();
    type_into_hint(&mut h, "Find", "nw_storethief001");
    h.run();
    h.get_by_label_contains("(nw_storethief001)").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_all_by_label("Finish").last().unwrap().click();
    h.run();
    assert!(h.state().store_wizard.is_none(), "{:?}", h.state().log.entries);
    let ws = h.state_mut().ws.as_mut().unwrap();
    for (name, t) in
        [("store001", ResType::DLG), ("openstore001", ResType::NSS), ("openstore001", ResType::NCS)]
    {
        assert!(ws.module.contains(&ResKey::parse(name, t).unwrap()), "{name}.{t:?}");
    }
    let g = ws.doc(&git).unwrap();
    let c = &g.root.list("Creature List").unwrap()[0];
    assert_eq!(c.resref("Conversation"), Some(ResRef::from_str("store001").unwrap()));
    assert_eq!(c.integer("FactionID"), Some(3), "Merchant");
    let store = &g.root.list("StoreList").unwrap()[0];
    assert_eq!(store.resref("ResRef"), Some(ResRef::from_str("nw_storethief001").unwrap()));
    assert_eq!((store.float("XPosition"), store.float("YPosition")), (Some(5.0), Some(5.0)));
}

#[test]
fn add_popup_text_gives_a_placeable_a_one_line_conversation() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(6);
    let mut m = new_module(&game, "Barrel", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bp = game.resman.get(&ResKey::parse("x3_plc_barrel1", ResType::UTP).unwrap()).unwrap();
    let bp = Gff::read(&bp).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let barrel = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Placeable List", mg_gff::Value::List(vec![barrel]));
    m.set_gff(git, &g).unwrap();
    let path = mg_testkit::scratch_dir("ui-popup-text").join("barrel.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.popup_text = Some(mg_ui::store_wizard::PopupText {
        area,
        placeable: 0,
        text: "Just an old barrel.".into(),
        name: "mg_popup".into(),
    });
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().popup_text.is_none());
    let ws = h.state_mut().ws.as_mut().unwrap();
    let dlg = ws.doc(&ResKey::parse("mg_popup", ResType::DLG).unwrap()).unwrap().root.clone();
    assert_eq!(dlg.list("EntryList").map(<[_]>::len), Some(1));
    assert_eq!(dlg.list("ReplyList").map(<[_]>::len), Some(0));
    let g = ws.doc(&git).unwrap();
    let barrel = &g.root.list("Placeable List").unwrap()[0];
    assert_eq!(barrel.resref("Conversation"), Some(ResRef::from_str("mg_popup").unwrap()));
}

#[test]
fn the_external_script_editor_s_saves_come_back() {
    let Some(editor) =
        ["/usr/bin/true", "/bin/true"].into_iter().find(|p| std::path::Path::new(p).exists())
    else {
        eprintln!("skipped: no `true` program to stand in for an editor");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-external-editor");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main() {}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.external_editor = Some(editor.into());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("External Editor").click();
    h.run_steps(3);
    // The editor's file: the script; an edit saved there comes back.
    let file =
        std::env::temp_dir().join(format!("moonglow-{}", std::process::id())).join("hello.nss");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "void main() {}\n");
    std::fs::write(&file, "void main() { int n; }\n").unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(later).unwrap();
    h.run_steps(3);
    assert_eq!(h.state().script_text(key).unwrap(), "void main() { int n; }\n");
}

#[test]
fn the_levelup_wizard_levels_a_creature_up_as_aurora_does() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_lvl", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run();
    h.get_by_label("Levelup Wizard").click();
    h.run();
    // Fighter 1 to 5, as captured in Aurora (aurora_levelup.rs).
    h.state_mut().levelup.as_mut().expect("the wizard").slots[0].1 = 5;
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().levelup.is_none());
    let s = field(&mut h, &key);
    let level = s.list("ClassList").unwrap()[0].integer("ClassLevel");
    assert_eq!(level, Some(5));
    assert_eq!((s.integer("HitPoints"), s.integer("MaxHitPoints")), (Some(32), Some(42)));
    let feats: Vec<i64> =
        s.list("FeatList").unwrap().iter().filter_map(|f| f.integer("Feat")).collect();
    assert!(feats.ends_with(&[28, 6, 391]), "{feats:?}");
    // One undo step.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(field(&mut h, &key).list("ClassList").unwrap()[0].integer("ClassLevel"), Some(1));
}

#[test]
fn the_levelup_wizard_adds_a_class_with_its_spells_and_gear() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_wiz", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run();
    h.get_by_label("Levelup Wizard").click();
    h.run();
    // Wizard 1, as captured in Aurora on a placed bandit.
    h.state_mut().levelup.as_mut().expect("the wizard").slots.push((10, 1, 0));
    h.run();
    h.get_by_label("OK").click();
    h.run();
    let s = field(&mut h, &key);
    let wizard = &s.list("ClassList").unwrap()[1];
    let cantrips: Vec<i64> =
        wizard.list("MemorizedList0").unwrap().iter().filter_map(|e| e.integer("Spell")).collect();
    assert_eq!(cantrips, [37, 100, 144]);
    let bolts = s.list("Equip_ItemList").unwrap().iter().find(|e| e.id == 0x2000).cloned();
    assert_eq!(
        bolts.and_then(|e| e.resref("EquippedRes")),
        Some(ResRef::from_str("nw_wambo001").unwrap())
    );
    let carried: Vec<(String, i64, i64)> = s
        .list("ItemList")
        .unwrap()
        .iter()
        .map(|e| {
            let res = e.resref("InventoryRes").unwrap().to_string();
            (res, e.integer("Repos_PosX").unwrap(), e.integer("Repos_Posy").unwrap())
        })
        .collect();
    assert_eq!(carried.len(), 11, "{carried:?}");
    assert_eq!(carried[0], ("nw_wswdg001".to_string(), 0, 0));
}

#[test]
fn the_creature_wizard_makes_aurora_s_creature() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-creature-wizard");
    let path = sample_module(&dir);
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Wizards").click();
    h.run();
    h.get_by_label_contains("Creature Wizard…").click();
    h.run();
    // Each page as it comes, nothing drawn over anything else.
    let next = |h: &mut Harness<'_, Moonglow>| {
        let over = overlapping(h);
        assert!(over.is_empty(), "drawn over each other: {over:?}");
        h.get_by_label("Next >").click();
        h.run();
    };
    next(&mut h);
    h.get_by_label("Human").click();
    h.run();
    next(&mut h);
    // Fighter 1, the Human's default class.
    assert_eq!(h.state().creature_wizard.as_ref().unwrap().classes, [(4, 1)]);
    next(&mut h);
    // The page opens with the first portrait chosen.
    let first = h.state().creature_wizard.as_ref().unwrap().portrait;
    assert!(first.is_some(), "a portrait is chosen when the page opens");
    // The portraits are pictures (named after them once loaded).
    h.get_by_label("po_hu_m_01_").click();
    h.run();
    next(&mut h);
    next(&mut h); // Hostile
    let w = h.state_mut().creature_wizard.as_mut().unwrap();
    (w.first_name, w.last_name) = ("Hent".into(), "Fynolds".into());
    h.run();
    next(&mut h);
    h.get_by_label("Tutorial").click();
    h.run();
    next(&mut h); // the review
    next(&mut h);
    // The button (the last page's heading is "Finish" too).
    h.query_all_by_label("Finish")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .unwrap()
        .click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let c = ws.doc(&ResKey::parse("hent", ResType::UTC).unwrap()).unwrap().root.clone();
    assert_eq!((c.integer("HitPoints"), c.integer("MaxHitPoints")), (Some(10), Some(13)));
    let feats: Vec<i64> =
        c.list("FeatList").unwrap().iter().filter_map(|f| f.integer("Feat")).collect();
    assert_eq!(feats, [258, 46, 3, 4, 2, 32, 45, 1089, 28, 106, 10]);
    assert_eq!(c.integer("PortraitId"), Some(93));
}

#[test]
fn conversation_lines_without_speaker_names_when_chosen() {
    let dir = mg_testkit::scratch_dir("ui-dialog-names");
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_talk", ResType::DLG).unwrap();
    let mut g = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "Hello there");
    let mut m = Module::open(&path).unwrap();
    m.set(key, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.dialog_hide_names = true; // Options › Conversation Editor
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Hello there");
    assert!(h.query_by_label("[OWNER] - Hello there").is_none());
}

#[test]
fn conversation_lines_show_their_scripts_and_play_as_the_game_plays_them() {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog, node_mut, set_condition};
    let dir = mg_testkit::scratch_dir("ui-dialog-play");
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_keeper", ResType::DLG).unwrap();
    let mut g = new_dialog();
    // Two greetings: the first only for those seen before.
    let again = add_node(&mut g, Parent::Root, "Back again?");
    set_condition(&mut g, Parent::Root, 0, "c_seen");
    let hello = add_node(&mut g, Parent::Root, "Hello, stranger.");
    let n = node_mut(&mut g, Kind::Entry, hello).unwrap();
    n.set("Script", mg_gff::Value::resref(ResRef::from_str("a_hello").unwrap()));
    n.set("Quest", mg_gff::Value::String(b"q_intro".to_vec()));
    n.set("QuestEntry", mg_gff::Value::Dword(10));
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    set_condition(&mut g, Parent::Node(Kind::Entry, hello), 0, "c_curious");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Bye.");
    add_node(&mut g, Parent::Node(Kind::Reply, who), "I am the keeper.");
    let _ = again;
    let mut m = Module::open(&path).unwrap();
    m.set(key, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // The lines name their scripts and journal updates.
    h.get_by_label("if c_seen");
    h.get_by_label("do a_hello  journal q_intro 10");
    h.get_by_label("Scripts").click();
    h.run();
    assert!(h.state().settings.dialog_hide_scripts);
    assert!(h.query_by_label("if c_seen").is_none());
    // Played through: the first greeting whose condition passes.
    h.get_by_label("Test").click();
    h.run();
    h.get_by_label("NPC: Back again?");
    h.get_by_label("if c_seen: TRUE").click();
    h.run();
    h.get_by_label("(not said) Back again?");
    h.get_by_label("NPC: Hello, stranger.");
    h.get_by_label("do a_hello  journal q_intro 10");
    // A reply whose condition fails is hidden.
    h.get_by_label("if c_curious: TRUE").click();
    h.run();
    h.get_by_label("(hidden) Who are you?");
    h.get_by_label("if c_curious: FALSE").click();
    h.run();
    h.get_by_label("2. Bye.");
    h.get_by_label("1. Who are you?").click();
    h.run();
    h.get_by_label("NPC: I am the keeper.");
    h.get_by_label("You: Who are you?");
    h.get_by_label("[END DIALOGUE]");
}

#[test]
fn conversations_export_and_import() {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog, nodes, text};
    let dir = mg_testkit::scratch_dir("ui-dialog-io");
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_guard", ResType::DLG).unwrap();
    let mut g = new_dialog();
    let halt = add_node(&mut g, Parent::Root, "Halt!");
    add_node(&mut g, Parent::Node(Kind::Entry, halt), "Sorry.");
    let mut m = Module::open(&path).unwrap();
    m.set(key, g.to_bytes().unwrap());
    m.save().unwrap();
    // A Twine export, then the CSV edited and read back.
    let twee = dir.join("guard.twee");
    let csv = dir.join("guard.csv");
    let mut app = Moonglow::new(
        None,
        Box::new(NoDialogs { save: vec![csv.clone(), twee.clone()], ..Default::default() }),
    );
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    for format in ["Twine (Twee)…", "CSV…"] {
        h.get_by_label("Export").click();
        h.run();
        h.get_by_label(format).click();
        h.run();
    }
    assert!(std::fs::read_to_string(&twee).unwrap().contains("[[Sorry.->END]]"));
    let edited = std::fs::read_to_string(&csv).unwrap().replace("Halt!", "Stop right there!");
    std::fs::write(&csv, edited).unwrap();
    h.state_mut().dialogs = Box::new(NoDialogs { open: vec![csv.clone()], ..Default::default() });
    h.get_by_label("Import Lines…").click();
    h.run();
    let line = |h: &mut Harness<'_, Moonglow>, k: ResKey| {
        let g = h.state_mut().ws.as_mut().unwrap().doc(&k).unwrap().clone();
        text(&nodes(&g, Kind::Entry)[0])
    };
    assert_eq!(line(&mut h, key), "Stop right there!");
    // An Ink story, as a new conversation named after its file.
    let ink = dir.join("Night Watch.ink");
    std::fs::write(&ink, "-> gate\n=== gate ===\nWho goes there?\n+ [A friend.] -> END\n").unwrap();
    let new = h.state_mut().import_conversation(&ink).expect("imported");
    h.run();
    assert_eq!(new.resref.to_string(), "night_watch");
    assert!(h.state().dock.find_tab(&Tab::Dialog(new)).is_some());
    assert_eq!(line(&mut h, new), "Who goes there?");
}

#[test]
fn saving_keeps_the_module_as_it_was_as_a_backup() {
    let dir = mg_testkit::scratch_dir("ui-backup");
    let path = sample_module(&dir);
    let before = std::fs::read(&path).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let key = ResKey::parse("mg_new", ResType::NSS).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "add",
        vec![mg_edit::Edit::SetResource { key, data: Some(b"void main() {}".to_vec()) }],
    )));
    app.actions.push(mg_ui::Action::Save);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Aurora's name: <module>.BackupMod, the module before the save.
    let backup = path.with_extension("BackupMod");
    assert_eq!(std::fs::read(&backup).unwrap(), before);
    assert_ne!(std::fs::read(&path).unwrap(), before);
    // Off in Options › General: none.
    std::fs::remove_file(&backup).unwrap();
    h.state_mut().settings.no_backups = true;
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(!backup.exists());
}

#[test]
fn a_script_named_as_the_game_s_own_is_warned_about() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-standard-warning");
    let path = sample_module(&dir);
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    let add = |name: &str| {
        mg_ui::Action::Apply(mg_edit::Command::new(
            "add",
            vec![mg_edit::Edit::SetResource {
                key: ResKey::parse(name, ResType::NSS).unwrap(),
                data: Some(b"void main() {}".to_vec()),
            }],
        ))
    };
    app.actions.push(add("nw_c2_default9"));
    app.actions.push(add("mg_mine"));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let warned: Vec<&String> = h
        .state()
        .log
        .entries
        .iter()
        .filter(|(l, _)| *l == mg_ui::Level::Warning)
        .map(|(_, m)| m)
        .collect();
    assert!(
        warned.iter().any(|m| m.contains("nw_c2_default9.nss replaces the game's own")),
        "{warned:?}"
    );
    assert!(!warned.iter().any(|m| m.contains("mg_mine")), "{warned:?}");
}

/// However a resource arrives, it goes in through the same gate: an import
/// warns of what it shadows as an editor's change does (the first few, and
/// how many more).
#[test]
fn an_import_warns_of_what_it_shadows() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-import-warning");
    let path = sample_module(&dir);
    let erf = dir.join("scripts.erf");
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs { open: vec![erf.clone()], ..Default::default() }),
    );
    app.open_module(&path);
    // Twelve scripts named as the game's own, and one of the module's.
    let mut names = app.game.as_deref().unwrap().resman.list(ResType::NSS);
    names.sort();
    names.truncate(12);
    let mut w = mg_erf::ErfWriter::new(*b"ERF ");
    for n in &names {
        w.add(*n, ResType::NSS, b"void main() {}".to_vec()).unwrap();
    }
    w.add(ResRef::from_str("mg_mine").unwrap(), ResType::NSS, b"void main() {}".to_vec()).unwrap();
    std::fs::write(&erf, w.to_bytes().unwrap()).unwrap();
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::ImportDialog);
    h.run();
    h.get_by_label("Import").click();
    h.run();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(names.iter().all(|n| ws.module.contains(&ResKey::new(*n, ResType::NSS))));
    let warned: Vec<&String> = h
        .state()
        .log
        .entries
        .iter()
        .filter(|(l, _)| *l == mg_ui::Level::Warning)
        .map(|(_, m)| m)
        .collect();
    let shadows = warned.iter().filter(|m| m.contains("replaces the game's own")).count();
    assert_eq!(shadows, 8, "{warned:?}");
    assert!(warned.iter().any(|m| m.contains("and 4 more like these")), "{warned:?}");
    assert!(!warned.iter().any(|m| m.contains("mg_mine")), "{warned:?}");
}

/// A look at Options › Area's marks: a door's orientation arrow and an
/// encounter's spawn point posts. Writes
/// `target/test-output/ui-area-marks/marks.png`; run by hand.
#[test]
#[ignore]
fn area_marks_screen() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("marks") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let game = h.state().game.as_deref().unwrap();
    let read = |r: &str, t: ResType| {
        Gff::read(&game.resman.get(&ResKey::parse(r, t).unwrap()).unwrap()).unwrap()
    };
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let door = instance(
        &placing,
        ResType::UTD,
        &read("nw_door_normal", ResType::UTD).root,
        Placement { position: [15.0, 15.0, 0.0], rotation: 0.6 },
        &[],
    )
    .unwrap();
    let outline = [[-3.0, -3.0, 0.0], [3.0, -3.0, 0.0], [3.0, 3.0, 0.0], [-3.0, 3.0, 0.0]];
    let mut enc = instance(
        &placing,
        ResType::UTE,
        &read("nw_giantevil", ResType::UTE).root,
        Placement { position: [25.0, 25.0, 0.0], rotation: 0.0 },
        &outline,
    )
    .unwrap();
    let spawn = |x: f32, y: f32| {
        let mut s = mg_gff::Struct::new(2);
        for (l, v) in [("X", x), ("Y", y), ("Z", 0.0), ("Orientation", 0.0)] {
            s.set(l, mg_gff::Value::Float(v));
        }
        s
    };
    enc.set("SpawnPointList", mg_gff::Value::List(vec![spawn(24.0, 24.0), spawn(26.0, 26.0)]));
    let edits = vec![
        mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Door List".into(),
            index: 0,
            item: door,
        },
        mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Encounter List".into(),
            index: 0,
            item: enc,
        },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("place", edits)));
    h.run_steps(5);
    let dir = mg_testkit::scratch_dir("ui-area-marks");
    h.render().expect("render").save(dir.join("marks.png")).unwrap();
}

#[test]
fn creature_names_can_be_random() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_named", ResType::UTC) else {
        return;
    };
    h.run();
    let name = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let l = s.locstring("FirstName").cloned().unwrap_or_default();
        l.strings.first().map(|(_, b)| String::from_utf8_lossy(b).into_owned())
    };
    let before = name(&mut h);
    h.get_all_by_label("🎲").next().unwrap().click();
    h.run();
    let after = name(&mut h).expect("a name");
    assert_ne!(Some(after.clone()), before);
    assert!((4..=13).contains(&after.len()), "{after}");
}

#[test]
fn speaker_tags_come_from_the_module_s_creatures() {
    use egui_kittest::kittest::Queryable;
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(8);
    let mut m = new_module(&game, "Speakers", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bp = game.resman.get(&ResKey::parse("nw_bandit001", ResType::UTC).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let mut bandit =
        instance(&placing, ResType::UTC, &Gff::read(&bp).unwrap().root, at, &[]).unwrap();
    bandit.set("Tag", mg_gff::Value::String(b"MG_SPEAKER".to_vec()));
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![bandit]));
    m.set_gff(git, &g).unwrap();
    let key = ResKey::parse("mg_talk", ResType::DLG).unwrap();
    let mut d = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut d, mg_module::dialog::Parent::Root, "Hello there");
    m.set(key, d.to_bytes().unwrap());
    let path = mg_testkit::scratch_dir("ui-speaker-tags").join("speakers.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("[OWNER] - Hello there").click();
    h.run();
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    h.get_by_label("MG_SPEAKER").click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let dlg = ws.doc(&key).unwrap().root.clone();
    assert_eq!(dlg.list("EntryList").unwrap()[0].string("Speaker"), Some(&b"MG_SPEAKER"[..]));
}

#[test]
fn text_is_edited_in_the_chosen_language() {
    use mg_core::{Gender, Language};
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_german", ResType::UTC)
    else {
        return;
    };
    // Options › Language: German (the UI thread's; this test's own).
    mg_ui::set_edit_language(Language::GERMAN);
    h.run();
    let english = |h: &mut Harness<'_, Moonglow>| {
        field(h, &key)
            .locstring("FirstName")
            .and_then(|l| l.text(Language::ENGLISH, Gender::Male).map(std::borrow::Cow::into_owned))
    };
    let before = english(&mut h);
    assert!(h.query_by_label_contains("(German)").is_some());
    h.get_all_by_label("🎲").next().unwrap().click();
    h.run();
    mg_ui::set_edit_language(Language::ENGLISH);
    let name = field(&mut h, &key).locstring("FirstName").cloned().unwrap();
    assert!(name.text(Language::GERMAN, Gender::Male).is_some(), "{name:?}");
    assert_eq!(english(&mut h), before);
}

#[test]
fn the_spells_page_warns_about_invalid_assignments() {
    use mg_gff::{Struct, Value};
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_wizard", ResType::UTC)
    else {
        return;
    };
    // Wizard 1 with Bull's Strength (level 2) prepared.
    let mut spell = Struct::new(3);
    spell.set("Spell", Value::Word(9));
    spell.set("SpellMetaMagic", Value::Byte(0));
    spell.set("SpellFlags", Value::Byte(1));
    let mut wizard = Struct::new(2);
    wizard.set("Class", Value::Int(10));
    wizard.set("ClassLevel", Value::Short(1));
    wizard.set("MemorizedList2", Value::List(vec![spell]));
    let set = |label: &str, value: Value| mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(value),
    };
    let edits = vec![set("ClassList", Value::List(vec![wizard])), set("Int", Value::Byte(18))];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("wizard", edits)));
    h.run();
    h.get_by_label("Spells").click();
    h.run();
    let warning = "This creature has spells assigned to it that are too high for its current \
                   Wizard level.";
    assert!(h.query_by_label(warning).is_some(), "Aurora's warning, without its question");
    h.get_by_label("Never warn again").click();
    h.run();
    assert!(h.state().settings.no_spell_warning);
    assert!(h.query_by_label(warning).is_none());
}

#[test]
fn equipping_without_the_feat_asks_to_add_it() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_unskilled", ResType::UTC)
    else {
        return;
    };
    let edit = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: "FeatList".into(),
        value: Some(mg_gff::Value::List(Vec::new())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("no feats", vec![edit])));
    h.run();
    h.get_by_label("Inventory").click();
    h.run();
    assert!(h.query_by_label_contains("The game may unequip items").is_some(), "the notice");
    type_into_hint(&mut h, "Find", "nw_waxbt001");
    h.get_by_label("Battleaxe").click();
    h.run();
    let primary = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let list = s.list("Equip_ItemList").unwrap_or(&[]).to_vec();
        list.iter().find(|s| s.id == 0x10).map(|s| s.resref("EquippedRes").unwrap().to_string())
    };
    let feats = |h: &mut Harness<'_, Moonglow>| -> Vec<i64> {
        let s = field(h, &key);
        s.list("FeatList").unwrap_or(&[]).iter().filter_map(|f| f.integer("Feat")).collect()
    };
    let before = primary(&mut h);
    // The Primary Weapon slot's Equip (the fifth): No leaves it as it was.
    h.get_all_by_label("Equip").nth(4).unwrap().click();
    h.run();
    assert!(h.query_by_label_contains("Weapon Proficiency (martial)").is_some());
    h.get_by_label("No").click();
    h.run();
    assert_eq!(primary(&mut h), before);
    assert!(feats(&mut h).is_empty());
    // Yes adds the first feat and equips it, one undoable step.
    h.get_all_by_label("Equip").nth(4).unwrap().click();
    h.run();
    h.get_by_label("Yes").click();
    h.run();
    assert_eq!(primary(&mut h).as_deref(), Some("nw_waxbt001"));
    assert_eq!(feats(&mut h), [45]);
    // The slot shows the axe's icon.
    let image = egui::accesskit::Role::Image;
    assert!(h.get_all_by_label("Battleaxe").any(|n| n.accesskit_node().role() == image));
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-inventory-icons").join("inventory.png"));
    }
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert!(feats(&mut h).is_empty());
    assert_eq!(primary(&mut h), before);
}

#[test]
fn conversation_options_popup_link_directions_drag_and_backup() {
    let dir = mg_testkit::scratch_dir("ui-dialog-options");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.conversation_backups = dir.join("backups");
    app.new_dialog = Some("optdlg".into());
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("optdlg", ResType::DLG).unwrap();
    let outline = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let g = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
        mg_module::dialog::outline(&g)
            .iter()
            .map(|l| l.split('|').take(3).collect::<Vec<_>>().join("|"))
            .collect()
    };
    let click = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_by_label(label).click();
        h.run();
    };
    // The popup's text field (its text is also a text run).
    fn input<'h>(h: &'h mut Harness<'_, Moonglow>) -> egui_kittest::Node<'h> {
        let role = egui::accesskit::Role::MultilineTextInput;
        h.get_all_by_value("<< Enter text here >>")
            .find(|n| n.accesskit_node().role() == role)
            .expect("the Input Text field")
    }
    // Add asks for the text (the placeholder selected, typed over); the
    // parent stays selected.
    let add = |h: &mut Harness<'_, Moonglow>, text: &str| {
        click(h, "Add");
        input(h).type_text(text);
        h.run();
        click(h, "OK");
    };
    add(&mut h, "Hello");
    assert!(h.query_by_label("Enter what the NPC says next:").is_none());
    click(&mut h, "Add");
    assert!(h.query_by_label("Enter what the NPC says next:").is_some());
    click(&mut h, "Cancel");
    add(&mut h, "Second");
    click(&mut h, "[OWNER] - Hello");
    click(&mut h, "Add");
    assert!(h.query_by_label("Enter what the player says next:").is_some());
    input(&mut h).type_text("Hi");
    h.run();
    click(&mut h, "OK");
    assert_eq!(outline(&mut h), ["Entry|Hello|", "  Reply|Hi|", "Entry|Second|"]);

    // Paste As Link, Aurora's default: the selected line links to the
    // copied one.
    click(&mut h, "[OWNER] - Second");
    click(&mut h, "Copy");
    click(&mut h, "Hi [END DIALOGUE]");
    click(&mut h, "Paste As Link");
    assert_eq!(
        outline(&mut h),
        ["Entry|Hello|", "  Reply|Hi|", "    Entry|Second|link", "Entry|Second|"]
    );
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    // Link Source To Destination: the copied line links to the selected.
    h.state_mut().settings.dialog_paste_source_to_dest = true;
    click(&mut h, "Hi [END DIALOGUE]");
    click(&mut h, "Paste As Link");
    assert_eq!(
        outline(&mut h),
        ["Entry|Hello|", "  Reply|Hi|", "Entry|Second|", "  Reply|Hi|link"]
    );
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();

    // A drag moves Hi under Second; with Ctrl, Hi links to Second.
    let drag = |h: &mut Harness<'_, Moonglow>, from: &str, to: &str, ctrl: bool| {
        let a = h.get_by_label(from).rect().center();
        let b = h.get_by_label(to).rect().center();
        h.hover_at(a);
        h.run();
        h.drag_at(a);
        h.run();
        h.hover_at(a + egui::vec2(10.0, 4.0));
        h.run();
        h.hover_at(b);
        h.run();
        if ctrl {
            h.event(egui::Event::ModifiersChanged(egui::Modifiers::CTRL));
        }
        h.drop_at(b);
        h.run();
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run();
    };
    drag(&mut h, "Hi [END DIALOGUE]", "[OWNER] - Second", true);
    assert_eq!(
        outline(&mut h),
        ["Entry|Hello|", "  Reply|Hi|", "    Entry|Second|link", "Entry|Second|"]
    );
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    drag(&mut h, "Hi [END DIALOGUE]", "[OWNER] - Second", false);
    assert_eq!(outline(&mut h), ["Entry|Hello|", "Entry|Second|", "  Reply|Hi|"]);

    // The backup: the open, changed conversation as optdlg.bak, once.
    let written = h.state_mut().backup_conversations();
    let bak = dir.join("backups").join("sample").join("optdlg.bak");
    assert_eq!(written, std::slice::from_ref(&bak));
    let g = Gff::read(&std::fs::read(&bak).unwrap()).unwrap();
    assert_eq!(mg_module::dialog::outline(&g).len(), 3);
    assert!(h.state_mut().backup_conversations().is_empty(), "unchanged since");
}

#[test]
fn sound_blueprints_play_their_sounds() {
    use mg_ui::audio::{Channel, Silence};
    let Some((mut h, key)) = blueprint_harness("animalcriesday", "sound_play", ResType::UTS) else {
        return;
    };
    let speaker = std::rc::Rc::new(std::cell::RefCell::new(Silence::default()));
    h.state_mut().speaker = Box::new(speaker.clone());
    h.run();
    let s = field(&mut h, &key);
    let first = s.list("Sounds").unwrap()[0].resref("Sound").unwrap();
    let volume = s.integer("Volume").unwrap() as f32 / 127.0;
    // Play: the first sound (none selected), at the blueprint's volume.
    h.get_by_label("Play").click();
    h.run();
    assert_eq!(speaker.borrow().channels.get(&Channel::Preview), Some(&(first, volume, false)));
    h.get_by_label("Stop").click();
    h.run();
    assert!(speaker.borrow().channels.is_empty());
    // A sound that is not there goes to the log.
    assert!(!h.state_mut().play_sound(
        Channel::Preview,
        ResRef::from_str("no_such").unwrap(),
        1.0,
        false
    ));
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("no_such not found")));
}

#[test]
fn the_area_view_plays_the_area_s_sounds() {
    use mg_ui::audio::{Channel, Silence};
    let Some((mut h, area)) = area_harness("sounds") else { return };
    let speaker = std::rc::Rc::new(std::cell::RefCell::new(Silence::default()));
    h.state_mut().speaker = Box::new(speaker.clone());
    // The first rows of ambientsound.2da and ambientmusic.2da with a sound.
    let first = |table: &str| -> (i32, ResRef) {
        let t = h.state().game.as_deref().unwrap().table(table).unwrap();
        (0..t.len())
            .find_map(|r| {
                let name = ResRef::from_str(t.get(r, "Resource")?).ok()?;
                (!name.is_empty()).then_some((r as i32, name))
            })
            .unwrap()
    };
    let (sound_row, sound) = first("ambientsound");
    let (music_row, music) = first("ambientmusic");
    // A bell 8 m east of where the view looks, full volume within 5 m,
    // nothing past 18 m; the area's ambient sound and music by day.
    let target = h.state().area_views[&area].orbit.as_ref().unwrap().target;
    let key = ResKey::new(area, ResType::GIT);
    let mut git = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
    let mut bell = mg_gff::Struct::new(6);
    let mut entry = mg_gff::Struct::new(0);
    entry.set("Sound", mg_gff::Value::resref(ResRef::from_str("as_cv_bell1").unwrap()));
    bell.set("Sounds", mg_gff::Value::List(vec![entry]));
    for (label, v) in [("XPosition", target.x + 8.0), ("YPosition", target.y), ("ZPosition", 0.0)] {
        bell.set(label, mg_gff::Value::Float(v));
    }
    bell.set("MinDistance", mg_gff::Value::Float(5.0));
    bell.set("MaxDistance", mg_gff::Value::Float(18.0));
    for (label, v) in [("Active", 1), ("Positional", 1), ("Continuous", 1), ("Volume", 127)] {
        bell.set(label, mg_gff::Value::Byte(v));
    }
    git.root.set("SoundList", mg_gff::Value::List(vec![bell]));
    let props = git.root.child_mut("AreaProperties").unwrap();
    props.set("AmbientSndDay", mg_gff::Value::Int(sound_row));
    props.set("AmbientSndDayVol", mg_gff::Value::Byte(64));
    props.set("MusicDay", mg_gff::Value::Int(music_row));
    let edit = mg_edit::Edit::SetResource { key, data: git.to_bytes().ok() };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("sounds", vec![edit])));
    h.state_mut().area_views.get_mut(&area).unwrap().night = false;
    h.run_steps(3);
    // Aurora's defaults: the placed sound only, at 1 − (8 − 5) / 13.
    let playing = |c: Channel| speaker.borrow().channels.get(&c).cloned();
    let (name, gain, looped) = playing(Channel::Placed(0)).expect("the bell");
    assert_eq!(name.to_string(), "as_cv_bell1");
    assert!((gain - 10.0 / 13.0).abs() < 1e-3, "{gain}");
    assert!(!looped);
    assert!(playing(Channel::Ambient).is_none() && playing(Channel::Music).is_none());
    // Ambient sound and music on: looped, at the area's and Aurora's volumes.
    h.state_mut().settings.ambient_sound = true;
    h.state_mut().settings.ambient_music = true;
    h.run_steps(2);
    let (a, a_volume, a_looped) = playing(Channel::Ambient).expect("the ambient sound");
    assert_eq!((a, a_looped), (sound, true));
    assert!((a_volume - 64.0 / 127.0).abs() < 1e-4);
    let (m, m_volume, _) = playing(Channel::Music).expect("the music");
    assert_eq!(m, music);
    assert!((m_volume - 92.0 / 127.0).abs() < 1e-4);
    // Placed sounds off; then the view closes and everything stops.
    h.state_mut().settings.no_placed_sounds = true;
    h.run_steps(2);
    assert!(playing(Channel::Placed(0)).is_none());
    let tab = h.state().dock.find_tab(&Tab::Area(area)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.run_steps(2);
    assert!(speaker.borrow().channels.is_empty(), "{:?}", speaker.borrow().channels);
}

#[test]
fn a_creature_s_sound_set_plays_a_sample() {
    use mg_ui::audio::{Channel, Silence};
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_voice", ResType::UTC) else {
        return;
    };
    let speaker = std::rc::Rc::new(std::cell::RefCell::new(Silence::default()));
    h.state_mut().speaker = Box::new(speaker.clone());
    h.run();
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("▶").click();
    h.run();
    let played = speaker.borrow().channels.get(&Channel::Preview).cloned();
    assert!(played.is_some(), "{:?}", h.state().log.entries);
    // Female sound sets only: a male one is no longer offered.
    let name = |h: &Harness<'_, Moonglow>, row: usize| -> Option<String> {
        let game = h.state().game.as_deref().unwrap();
        let strref = game.table("soundset").unwrap().get_int(row, "STRREF")?;
        game.string(mg_core::StrRef(strref as u32))
    };
    let rows = |h: &Harness<'_, Moonglow>, gender: i32| -> Vec<String> {
        let t = h.state().game.as_deref().unwrap().table("soundset").unwrap();
        (0..t.len())
            .filter(|&r| t.get_int(r, "GENDER") == Some(gender))
            .filter_map(|r| name(h, r))
            .collect()
    };
    let (male, female) = (rows(&h, 0), rows(&h, 1));
    let male_only = male.iter().find(|m| !female.contains(m)).unwrap().clone();
    let set = field(&mut h, &key).integer("SoundSetFile").unwrap() as usize;
    let current = name(&h, set).unwrap();
    h.get_by_value("Both").click();
    h.run();
    h.get_by_label("Female").click();
    h.run();
    h.get_by_value(&current).click();
    h.run();
    assert!(h.query_by_label(&male_only).is_none(), "{male_only} offered");
    assert!(female.iter().any(|f| h.query_all_by_label(f).next().is_some()));
}

#[test]
fn print_opens_a_highlighted_page_of_the_script() {
    let dir = mg_testkit::scratch_dir("ui-script-print");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main() { int a = 1 < 2; }\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.print_dir = dir.join("print");
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Print…").click();
    h.run();
    let page = std::fs::read_to_string(dir.join("print").join("hello.html")).unwrap();
    assert!(page.contains(" &lt; ") && page.contains("window.print()"), "{page}");
    assert!(page.contains(">int</span>"), "keywords are coloured spans");
}

#[test]
fn portraits_are_chosen_from_their_pictures() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_face", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Portraits…").click();
    h.run();
    assert!(h.query_by_label("Select Portrait").is_some());
    let count = |h: &Harness<'_, Moonglow>| -> usize {
        let n = h.get_by_label_contains(" portraits");
        let a = n.accesskit_node();
        let text = a.label().or_else(|| a.value()).map(|t| t.to_string()).unwrap();
        text.split(' ').next().unwrap().parse().unwrap()
    };
    let creatures = count(&h);
    // The first creature portrait is portraits.2da row 1, a dwarf woman:
    // its picture is loaded (its name is the picture's).
    h.get_by_label("po_dw_f_01_").click();
    h.run();
    // Placeables and doors have their own.
    h.get_by_label("Placeable Objects and Doors").click();
    h.run();
    let things = count(&h);
    assert!(creatures > 100 && things > 100 && creatures != things, "{creatures} {things}");
    assert!(h.query_by_label("po_dw_f_02_").is_none());
    h.get_by_label("Characters and Creatures").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("PortraitId"), Some(1));
    assert!(h.query_by_label("Select Portrait").is_none());
    // A look at the window (ignored test `screens`-style output).
    h.get_by_label("Portraits…").click();
    h.run();
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-portraits").join("select.png"));
    }
}

#[test]
fn items_show_their_icons_and_choose_appearances_by_icon() {
    let Some((mut h, key)) = blueprint_harness("nw_it_gem001", "gem_icons", ResType::UTI) else {
        return;
    };
    h.run();
    h.get_by_label("Appearance").click();
    h.run();
    h.get_by_label("Icon");
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-item-icons").join("gem.png"));
    }
    // The grid offers each model by its icon; a click takes it.
    let before = field(&mut h, &key).integer("ModelPart1").unwrap();
    let other = h
        .get_all_by_label_contains("Appearance ")
        .filter_map(|n| n.accesskit_node().label().map(|l| l.to_string()))
        .find(|l| l != &format!("Appearance {before}"))
        .expect("another appearance");
    h.get_by_label(&other).click();
    h.run();
    let n: i64 = other.trim_start_matches("Appearance ").parse().unwrap();
    assert_eq!(field(&mut h, &key).integer("ModelPart1"), Some(n));
    // A look at a weapon's and an armor's icons.
    let dir = mg_testkit::scratch_dir("ui-item-icons");
    for (bp, name) in [("nw_wswls001", "sword"), ("nw_aarcl001", "armor"), ("nw_arhe001", "helm")] {
        let Some((mut h, _)) = blueprint_harness(bp, &format!("icon_{name}"), ResType::UTI) else {
            return;
        };
        h.run();
        h.get_by_label("Appearance").click();
        h.run();
        h.get_by_label("Icon");
        if let Ok(img) = h.render() {
            let _ = img.save(dir.join(format!("{name}.png")));
        }
    }
}

#[test]
fn colours_are_chosen_from_palette_swatches() {
    let Some((mut h, key)) = blueprint_harness("nw_arhe001", "helm_colours", ResType::UTI) else {
        return;
    };
    h.run();
    h.get_by_label("Appearance").click();
    h.run();
    let cloth = field(&mut h, &key).integer("Cloth1Color").unwrap();
    let combo = egui::accesskit::Role::ComboBox;
    h.get_all_by_value(&cloth.to_string())
        .find(|n| n.accesskit_node().role() == combo)
        .expect("the Cloth 1 colour")
        .click();
    h.run();
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-colours").join("swatches.png"));
    }
    let pick = if cloth == 5 { 6 } else { 5 };
    h.get_by_label(&format!("Cloth 1 {pick}")).click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Cloth1Color"), Some(pick));
}

#[test]
fn an_armor_s_parts_take_colors_of_their_own_and_keep_them() {
    use mg_gff::Value;
    let Some((mut h, key)) = blueprint_harness("nw_aarcl001", "part_colors", ResType::UTI) else {
        return;
    };
    // A color the armor came with (the left hand's Metal 2), as the game
    // writes it.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "part color",
        vec![mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "APart_17_Col_5".into(),
            value: Some(Value::Byte(171)),
        }],
    )));
    h.run();
    h.get_by_label("Appearance").click();
    h.run();
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-part-colors").join("page.png"));
    }
    // The torso (shown first) has none of its own: each channel shows the
    // armor's. Its Cloth 1 is given one.
    let combo = egui::accesskit::Role::ComboBox;
    h.get_all_by_value("Default")
        .find(|n| n.accesskit_node().role() == combo)
        .expect("the torso's Cloth 1")
        .click();
    h.run();
    h.get_by_label("Torso Cloth 1 33").click();
    h.run();
    assert_eq!(field(&mut h, &key).get("APart_7_Col_2"), Some(&Value::Byte(33)));

    // Another edit, and a save: both parts' colors are in the saved file.
    let cloth = field(&mut h, &key).integer("Cloth1Color").unwrap();
    // (Its colors are in the right column; part numbers, some the same,
    // in the left.)
    h.get_all_by_value(&cloth.to_string())
        .find(|n| n.accesskit_node().role() == combo && n.rect().min.x > 590.0)
        .expect("the armor's Cloth 1")
        .click();
    h.run();
    let pick = if cloth == 5 { 6 } else { 5 };
    h.get_by_label(&format!("Cloth 1 {pick}")).click();
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-output/ui-bp-part_colors/sample.mod");
    let saved = mg_module::Module::open(&path).unwrap();
    let uti = saved.gff(&key).unwrap().unwrap().root;
    assert_eq!(uti.integer("Cloth1Color"), Some(pick));
    assert_eq!(uti.get("APart_7_Col_2"), Some(&Value::Byte(33)));
    assert_eq!(uti.get("APart_17_Col_5"), Some(&Value::Byte(171)));

    // Reset: the part takes the armor's color again, and the field goes.
    h.get_by_label("Reset Torso Cloth 1").click();
    h.run();
    assert_eq!(field(&mut h, &key).get("APart_7_Col_2"), None);
    assert_eq!(field(&mut h, &key).get("APart_17_Col_5"), Some(&Value::Byte(171)));
}

#[test]
fn sound_play_styles_set_aurora_s_fields_and_priority() {
    let Some((mut h, key)) = blueprint_harness("animalcriesday", "sound_styles", ResType::UTS)
    else {
        return;
    };
    h.run();
    let int = |h: &mut Harness<'_, Moonglow>, l: &str| field(h, &key).integer(l);
    h.get_by_label("Advanced").click();
    h.run();
    // Four sounds: no seamless looping (Aurora greys it).
    assert!(h.get_by_label("Seamlessly looping").accesskit_node().is_disabled());
    h.get_by_label("Once").click();
    h.run();
    assert_eq!((int(&mut h, "Looping"), int(&mut h, "Continuous")), (Some(0), Some(0)));
    assert_eq!(int(&mut h, "Priority"), Some(20), "a positional single shot");
    h.get_by_label("Positioning").click();
    h.run();
    h.get_by_label("Plays everywhere in area").click();
    h.run();
    assert_eq!(int(&mut h, "Priority"), Some(19), "an area-wide single shot");
}

#[test]
fn creature_items_can_be_dropable_and_pickpocketable() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_flags", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Inventory").click();
    h.run();
    let flag = |h: &mut Harness<'_, Moonglow>, label: &str| {
        field(h, &key).list("Equip_ItemList").unwrap()[0].integer(label)
    };
    assert_eq!(flag(&mut h, "Dropable"), None, "unset, as the game's blueprints leave it");
    h.get_by_label_contains("Selected Item: none");
    // Select the armor (its name, not its icon), as in Aurora's inventory.
    let image = egui::accesskit::Role::Image;
    h.get_all_by_label("Padded Armor")
        .find(|n| n.accesskit_node().role() != image)
        .unwrap()
        .click();
    h.run();
    h.get_by_label("Selected Item: Padded Armor");
    h.get_by_label("Dropable").click();
    h.run();
    assert_eq!(flag(&mut h, "Dropable"), Some(1));
    h.get_by_label("Dropable").click();
    h.run();
    assert_eq!(flag(&mut h, "Dropable"), None, "cleared: left out again");
    h.get_by_label("Pickpocketable").click();
    h.run();
    assert_eq!(flag(&mut h, "Pickpocketable"), Some(1));
}

#[test]
fn unsaved_work_survives_a_crash() {
    let dir = mg_testkit::scratch_dir("ui-recovery");
    let path = sample_module(&dir);
    let recovery = dir.join("recovery");
    let tag = |app: &mut Moonglow| -> Vec<u8> {
        let key = ResKey::parse("module", ResType::IFO).unwrap();
        app.ws.as_mut().unwrap().doc(&key).unwrap().root.string("Mod_Tag").unwrap().to_vec()
    };
    // A session edits the module, its copy is written, then it "crashes".
    {
        let mut app = app_with(Vec::new());
        app.recovery_dir = Some(recovery.clone());
        app.open_module(&path);
        let edit = mg_edit::Edit::SetField {
            key: ResKey::parse("module", ResType::IFO).unwrap(),
            path: mg_edit::GffPath::root(),
            label: "Mod_Tag".into(),
            value: Some(mg_gff::Value::String(b"RECOVERED".to_vec())),
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("tag", vec![edit])));
        let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run();
        let copy = h.state_mut().write_recovery().unwrap();
        assert!(copy.starts_with(&recovery) && copy.is_file());
    }
    // The next session offers it back; Recover opens it as the module.
    let mut app = app_with(Vec::new());
    app.recovery_dir = Some(recovery.clone());
    app.find_recoveries();
    assert_eq!(app.recoveries.len(), 1);
    assert_eq!(app.recoveries[0].note.module.as_deref(), Some(path.as_path()));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Recover Unsaved Work");
    h.get_by_label("Recover").click();
    h.run();
    assert_eq!(tag(h.state_mut()), b"RECOVERED");
    assert!(h.state().ws.as_ref().unwrap().is_modified(), "not saved yet");
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()));
    // Saving writes it to the module and removes the copy.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(mg_ui::recovery::find(&recovery).is_empty());
    let saved = Module::open(&path).unwrap();
    let ifo = saved.gff(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap().unwrap();
    assert_eq!(ifo.root.string("Mod_Tag"), Some(&b"RECOVERED"[..]));
}

#[test]
fn closing_without_saving_leaves_no_copy() {
    let dir = mg_testkit::scratch_dir("ui-recovery-close");
    let path = sample_module(&dir);
    let recovery = dir.join("recovery");
    let mut app = app_with(Vec::new());
    app.recovery_dir = Some(recovery.clone());
    app.open_module(&path);
    app.write_recovery().unwrap();
    assert_eq!(mg_ui::recovery::find(&recovery).len(), 1);
    app.close();
    assert!(mg_ui::recovery::find(&recovery).is_empty());
}

#[test]
fn premium_campaigns_find_their_talk_tables_in_the_install() {
    // Tyrants of the Moonsea names `tyrants`, which the game ships in
    // `data/tlk/` beside the campaign's haks in `data/hk/`.
    let Some(root) = mg_testkit::nwn_root() else { return };
    let path = root.join("data/nwm/Neverwinter Nights - Tyrants of the Moonsea.nwm");
    if !path.is_file() {
        eprintln!("skipped: Tyrants of the Moonsea is not installed");
        return;
    }
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    let loaded = app.log.entries.iter().any(|(_, e)| e == "Custom talk table tyrants loaded");
    assert!(loaded, "{:?}", app.log.entries);
    assert!(app.game.as_deref().unwrap().custom_tlk().is_some());
}

#[test]
fn the_user_manual_opens_from_help_and_follows_its_links() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(Vec::new()));
    h.run();
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label_contains("User Manual").click();
    h.run();
    // The contents, with the chapters beside them.
    h.get_by_label("Moonglow Toolset: User Manual");
    // Chapters in the list are buttons; links in the text are not.
    let listed =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::Button;
    assert!(h.get_all_by_label("Troubleshooting").any(|n| listed(&n)));
    // A link in the text opens its chapter in place.
    h.get_all_by_label("Getting started").find(|n| !listed(n)).unwrap().click();
    h.run();
    h.get_by_label("What you need");
    // The chapter list picks another.
    h.get_all_by_label("Areas").find(|n| listed(n)).unwrap().click();
    h.run();
    h.get_by_label("Placing objects");
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label("About Moonglow Toolset").click();
    h.run();
    h.get_by_label(&format!("Version {}", env!("CARGO_PKG_VERSION")));
}

/// Widgets drawn over each other: pairs of named widgets in one layer (a
/// window, the main panels) whose visible rectangles (clipped by their scroll
/// areas, as egui records them for the frame) overlap by more than a quarter
/// of the smaller, as when a row of widgets is squeezed into a narrow
/// column. Their names, for the failure message.
fn overlapping(h: &Harness<'_, Moonglow>) -> Vec<String> {
    use egui_kittest::kittest::{AccessKitNode, NodeT};
    use std::collections::HashMap;
    // The named widgets (role and name), by egui id.
    fn names(n: AccessKitNode<'_>, out: &mut HashMap<u64, String>) {
        let role = n.role();
        let name = n.label().or_else(|| n.value()).unwrap_or_default();
        let part = matches!(
            role,
            egui::accesskit::Role::Unknown | egui::accesskit::Role::GenericContainer
        );
        // Widgets, not the windows and groups that hold them.
        let leaf = n.children().next().is_none();
        if leaf && !name.trim().is_empty() && !part {
            out.insert(n.locate().0.0, format!("{role:?} {name:?}"));
        }
        for k in n.children() {
            names(k, out);
        }
    }
    let mut named = HashMap::new();
    names(h.root().accesskit_node(), &mut named);
    let mut out = Vec::new();
    h.ctx.viewport(|v| {
        for (_, widgets) in v.prev_pass.widgets.layers() {
            let shown: Vec<(egui::Rect, &String)> = widgets
                .iter()
                .filter(|w| w.interact_rect.area() > 1.0)
                .filter_map(|w| Some((w.interact_rect, named.get(&w.id.value())?)))
                .collect();
            for (i, (a, name_a)) in shown.iter().enumerate() {
                for (b, name_b) in &shown[i + 1..] {
                    let both = a.intersect(*b);
                    let smaller = a.area().min(b.area());
                    if both.is_positive() && both.area() > 0.25 * smaller {
                        out.push(format!("{name_a} over {name_b}"));
                    }
                }
            }
        }
    });
    out
}

#[test]
fn blueprint_editor_pages_draw_nothing_over_anything_else() {
    // Every page of every editor, roomy and narrow (a tab beside the
    // palette and tree).
    let editors = [
        ("nw_bandit001", ResType::UTC),
        ("nw_arhe001", ResType::UTI),
        ("plc_chest1", ResType::UTP),
        ("nw_door_ttr_01", ResType::UTD),
        ("trackstrigger", ResType::UTT),
        ("nw_verminbeet", ResType::UTE),
        ("nw_storebar01", ResType::UTM),
        ("animalcriesday", ResType::UTS),
        ("nw_waypoint001", ResType::UTW),
    ];
    let mut found = Vec::new();
    for (name, t) in editors {
        let copy = format!("over_{}", t.extension().unwrap_or("x"));
        let Some((mut h, key)) = blueprint_harness(name, &copy, t) else { return };
        if t == ResType::UTC {
            // A human of parts (the bandit is one model), for the body
            // parts on the Appearance page.
            h.run();
            h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
                "appearance",
                vec![mg_edit::Edit::SetField {
                    key,
                    path: mg_edit::GffPath::root(),
                    label: "Appearance_Type".into(),
                    value: Some(mg_gff::Value::Word(6)),
                }],
            )));
        }
        for size in [
            egui::vec2(1280.0, 800.0),
            egui::vec2(1000.0, 800.0),
            egui::vec2(800.0, 700.0),
            egui::vec2(600.0, 700.0),
        ] {
            h.set_size(size);
            for page in mg_ui::blueprint::pages(t) {
                h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), page);
                h.run();
                for o in overlapping(&h) {
                    found.push(format!("{t:?} {page} at {}: {o}", size.x));
                }
            }
        }
    }
    assert!(found.is_empty(), "drawn over each other:\n{}", found.join("\n"));
}

#[test]
fn views_draw_nothing_over_anything_else() {
    // The editors, windows and wizards outside the blueprint editors, roomy
    // and narrow, on a module with a conversation.
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-overlaps");
    let path = sample_module(&dir);
    let talk = ResKey::parse("mg_talk", ResType::DLG).unwrap();
    let mut g = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "Hello there");
    let mut m = Module::open(&path).unwrap();
    m.set(talk, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let mut found: Vec<String> = Vec::new();
    let mut look = |h: &mut Harness<'_, Moonglow>, what: &str| {
        h.run();
        let width = h.ctx.content_rect().width();
        found.extend(overlapping(h).into_iter().map(|o| format!("{what} at {width}: {o}")));
    };
    let button = |h: &Harness<'_, Moonglow>, label: &str| {
        use egui_kittest::kittest::NodeT;
        h.query_all_by_label(label)
            .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
            .map(|n| n.click())
            .is_some()
    };
    for size in [
        egui::vec2(1280.0, 800.0),
        egui::vec2(1000.0, 800.0),
        egui::vec2(800.0, 700.0),
        egui::vec2(600.0, 700.0),
    ] {
        h.set_size(size);
        use mg_ui::module_props::Page;
        for page in
            [Page::Basic, Page::Events, Page::Advanced, Page::Description, Page::CustomContent]
        {
            h.state_mut().module_page = page;
            h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
            look(&mut h, &format!("Module Properties {page:?}"));
        }
        for (tab, what) in [
            (Tab::Factions, "Factions"),
            (Tab::Journal, "Journal"),
            (Tab::Dialog(talk), "Conversation"),
            (Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap()), "Script"),
            (Tab::Resources, "Resources"),
            (Tab::Palette, "Palettes"),
            (Tab::Manual, "Manual"),
        ] {
            h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
            look(&mut h, what);
            if std::env::var("DEBUG_DOCK").is_ok() && size.x == 800.0 && what == "Script" {
                for (path, leaf) in h.state().dock.iter_leaves() {
                    println!(
                        "leaf {path:?} rect {:?} tabs {:?} active {:?}",
                        leaf.rect,
                        leaf.tabs.len(),
                        leaf.active
                    );
                }
            }
        }
        // Windows.
        let settings = h.state().settings.clone();
        h.state_mut().options = Some(mg_ui::OptionsDraft::from_settings(&settings));
        look(&mut h, "Options Folders");
        for page in
            ["Area", "General", "Script Editor", "Conversation Editor", "Sounds", "Language"]
        {
            assert!(button(&h, page), "{page}");
            look(&mut h, &format!("Options {page}"));
        }
        h.state_mut().options = None;
        h.state_mut().about = true;
        look(&mut h, "About");
        h.state_mut().about = false;
        h.state_mut().build = Some(Default::default());
        look(&mut h, "Build Module");
        h.state_mut().build = None;
        h.state_mut().find_instance = Some(Default::default());
        look(&mut h, "Find Instance");
        h.state_mut().find_instance = None;
        let hello = ResKey::parse("hello", ResType::NSS).unwrap();
        h.state_mut().actions.push(mg_ui::Action::FindReferences(hello));
        look(&mut h, "References");
        h.state_mut().actions.push(mg_ui::Action::RenameDialog(hello));
        look(&mut h, "Rename");
        h.state_mut().rename = None;
        let clip = mg_ui::area_view::ObjectClip { objects: Vec::new(), anchor: glam::Vec3::ZERO };
        h.state_mut().prefab_save = Some(("Gate".into(), clip));
        look(&mut h, "Save as Prefab");
        h.state_mut().prefab_save = None;
        h.state_mut().actions.push(mg_ui::Action::AreaWizard);
        look(&mut h, "Area Wizard");
        h.state_mut().wizard = None;
        // Wizards, page by page while Next leads on.
        for kind in mg_ui::blueprint_wizard::KINDS {
            h.state_mut().blueprint_wizard =
                Some(mg_ui::blueprint_wizard::BlueprintWizard::new(kind));
            for step in 0..8 {
                look(&mut h, &format!("{kind:?} Wizard, page {step}"));
                if !button(&h, "Next >") {
                    break;
                }
                h.run();
            }
            h.state_mut().blueprint_wizard = None;
        }
        h.state_mut().creature_wizard = Some(Default::default());
        h.run();
        for step in 0..8 {
            if step == 1 {
                assert!(button(&h, "Human"));
                h.run();
            }
            look(&mut h, &format!("Creature Wizard, page {step}"));
            if !button(&h, "Next >") {
                break;
            }
            h.run();
        }
        h.state_mut().creature_wizard = None;
    }
    assert!(found.is_empty(), "drawn over each other:\n{}", found.join("\n"));
}

#[test]
fn tabs_open_beside_the_palette_not_in_its_pane() {
    let dir = mg_testkit::scratch_dir("ui-tab-panes");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let script = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    for tab in [Tab::Palette, Tab::Manual, script.clone(), Tab::Factions] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run();
    }
    let dock = &h.state().dock;
    let pane = |t: &Tab| dock.find_tab(t).map(|p| (p.surface, p.node));
    let palette = pane(&Tab::Palette).unwrap();
    for t in [Tab::Manual, script, Tab::Factions] {
        assert_ne!(pane(&t), Some(palette), "{t:?} opened in the palette's pane");
    }
}

#[test]
fn the_log_shrinks_to_one_line() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(Vec::new()));
    h.run();
    let line = h.ctx.global_style().text_styles[&egui::TextStyle::Monospace].size;
    let entry = |h: &Harness<'_, Moonglow>| h.get_by_label_contains("No Neverwinter Nights").rect();
    let status = h.get_by_label("No module").rect();
    // The panel's top edge, just above its first line: drag it far down.
    let top = entry(&h).top() - 3.0;
    let x = 500.0;
    h.event(egui::Event::PointerMoved(egui::pos2(x, top)));
    h.run();
    let button = egui::PointerButton::Primary;
    let modifiers = egui::Modifiers::NONE;
    h.event(egui::Event::PointerButton {
        pos: egui::pos2(x, top),
        button,
        pressed: true,
        modifiers,
    });
    h.run();
    for y in [top + 40.0, top + 120.0, status.top() - 2.0] {
        h.event(egui::Event::PointerMoved(egui::pos2(x, y)));
        h.run();
    }
    let end = egui::pos2(x, status.top() - 2.0);
    h.event(egui::Event::PointerButton { pos: end, button, pressed: false, modifiers });
    h.run();
    // About a line between the log's first line and the status bar (three
    // lines' room was the least it would take).
    let room = status.top() - entry(&h).top();
    assert!(room < 2.0 * line + 12.0, "the log keeps {room} points (a line is {line})");
}

#[test]
fn a_preview_follows_its_blueprint_s_editor() {
    let root = mg_testkit::corpus!();
    mg_testkit::gpu::hold();
    if mg_render::Gpu::headless().is_none() {
        eprintln!("skipped: no GPU adapter");
        return;
    }
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let dir = mg_testkit::scratch_dir("ui-preview-live");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    // A human of parts, in the module, its preview open beside its editor.
    let game = app.game.as_deref().unwrap();
    let data = game.resman.get_named("nw_bandit001", ResType::UTC).unwrap().into_owned();
    let key = ResKey::parse("live_look", ResType::UTC).unwrap();
    let set = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(value),
    };
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "copy",
        vec![
            mg_edit::Edit::SetResource { key, data: Some(data) },
            set("Appearance_Type", mg_gff::Value::Word(6)),
            set("Appearance_Head", mg_gff::Value::Byte(1)),
        ],
    )));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    let head = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let p = h.state().model_views[&mg_ui::model_view::Source::Resource(key)]
            .preview()
            .expect("a preview");
        p.parts.iter().map(|p| p.model.clone()).filter(|m| m.contains("_head")).collect()
    };
    assert_eq!(head(&h), ["pmh0_head001"]);
    // The editor changes the head: the open preview shows the new one.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Head",
        vec![set("Appearance_Head", mg_gff::Value::Byte(3))],
    )));
    h.run_steps(4);
    assert_eq!(head(&h), ["pmh0_head003"]);
}

#[test]
fn blueprints_drag_from_the_module_tree_into_the_area() {
    let Some((mut h, area)) = area_harness("drag-tree") else { return };
    // A waypoint blueprint of the module's own (the game's Tavern's copy).
    let key = ResKey::parse("mg_tree_wp", ResType::UTW).unwrap();
    let tavern = ResKey::parse("nw_wp_tavern", ResType::UTW).unwrap();
    let data = h.state().game.as_deref().unwrap().resman.get(&tavern).unwrap().into_owned();
    h.state_mut().ws.as_mut().unwrap().module.set(key, data);
    h.run_steps(2);
    let count = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").map_or(0, <[mg_gff::Struct]>::len)
    };
    let before = count(&mut h);
    h.get_by_label_contains("Waypoints (").click();
    h.run_steps(2);
    let from = h.get_by_label("mg_tree_wp.utw").rect().center();
    let to = screen(&h, area, glam::Vec3::new(25.0, 25.0, 0.0));
    let modifiers = egui::Modifiers::NONE;
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    press(&h, from, true, modifiers);
    h.run_steps(1);
    for t in [0.1, 0.4, 0.7, 1.0] {
        h.event(egui::Event::PointerMoved(from + (to - from) * t));
        h.run_steps(1);
    }
    // Before it is let go, its ghost shows where it would go; Shift + Q
    // turns it a quarter anticlockwise.
    let ghost = h.state().area_views[&area].ghost_shown.clone().expect("a ghost");
    assert_eq!(ghost.kind, mg_area::ObjectKind::Waypoint);
    let at = ghost.position;
    assert!((at.x - 25.0).abs() < 0.5 && (at.y - 25.0).abs() < 0.5, "the ghost is at {at}");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Q);
    h.run_steps(1);
    let turned = h.state().area_views[&area].ghost_shown.as_ref().unwrap().rotation;
    assert!((turned - ghost.rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    press(&h, to, false, modifiers);
    h.run_steps(3);
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    let placed = model
        .objects
        .iter()
        .find(|o| o.kind == mg_area::ObjectKind::Waypoint && o.index == before)
        .expect("the waypoint is placed");
    let d = (placed.rotation - turned).rem_euclid(std::f32::consts::TAU);
    assert!(d < 1e-3 || d > std::f32::consts::TAU - 1e-3, "placed at {}", placed.rotation);
    assert_eq!(count(&mut h), before + 1, "{:?}", h.state().log.entries);
    let (x, y, _) = waypoint(&mut h, area, before).unwrap();
    assert!((x - 25.0).abs() < 0.5 && (y - 25.0).abs() < 0.5, "dropped at {x}, {y}");
    // Placed: no ghost.
    assert!(h.state().area_views[&area].ghost_shown.is_none());
}

#[test]
fn a_placeable_dragged_over_the_area_shows_its_model_where_it_would_go() {
    let Some((mut h, area)) = area_harness("drag-ghost") else { return };
    let key = ResKey::parse("mg_ghost_chest", ResType::UTP).unwrap();
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let data = h.state().game.as_deref().unwrap().resman.get(&chest).unwrap().into_owned();
    h.state_mut().ws.as_mut().unwrap().module.set(key, data);
    h.run_steps(2);
    h.get_by_label_contains("Placeables (").click();
    h.run_steps(2);
    let from = h.get_by_label("mg_ghost_chest.utp").rect().center();
    let to = screen(&h, area, glam::Vec3::new(20.0, 22.0, 0.0));
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for t in [0.1, 0.5, 1.0] {
        h.event(egui::Event::PointerMoved(from + (to - from) * t));
        h.run_steps(1);
    }
    let ghost = h.state().area_views[&area].ghost_shown.clone().expect("a ghost");
    assert!(ghost.preview.is_some(), "the chest has a model to show");
    assert!((ghost.position.x - 20.0).abs() < 0.5 && (ghost.position.y - 22.0).abs() < 0.5);
    assert!(ghost.rotation.abs() < 1e-4);
    // E twice turns it clockwise by 15° each (the selection stays as it is).
    h.key_press(egui::Key::E);
    h.run_steps(1);
    h.key_press(egui::Key::E);
    h.run_steps(1);
    let turned = h.state().area_views[&area].ghost_shown.as_ref().unwrap().rotation;
    assert!((turned + 30f32.to_radians()).abs() < 1e-4, "turned to {}", turned.to_degrees());
    // Moved off the view (onto the module tree), it goes.
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(2);
    assert!(h.state().area_views[&area].ghost_shown.is_none());
    // Back over the view and let go: placed as the ghost was turned.
    h.event(egui::Event::PointerMoved(to));
    h.run_steps(2);
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    // (The last placeable: the copy keeps plc_chest1 as its template.)
    let placed = model
        .objects
        .iter()
        .filter(|o| o.kind == mg_area::ObjectKind::Placeable)
        .max_by_key(|o| o.index)
        .expect("the chest is placed");
    assert!((placed.rotation - turned).abs() < 1e-3, "placed at {}", placed.rotation.to_degrees());
}

#[test]
fn blueprints_drag_from_the_palette_into_the_area() {
    let Some((mut h, area)) = area_harness("drag-place") else { return };
    // The standard Tavern waypoint, in the palette beside the area.
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    h.state_mut().palette.kind = mg_module::palette::BlueprintKind::Waypoint;
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.filter = "nw_wp_tavern".into();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    let count = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").map_or(0, <[mg_gff::Struct]>::len)
    };
    assert_eq!(count(&mut h), 2);
    // Chosen in the palette (a click), then pressed on, dragged over the
    // area, let go there.
    h.get_by_label(&tavern).click();
    h.run_steps(1);
    let from = h.get_by_label(&tavern).rect().center();
    let to = screen(&h, area, glam::Vec3::new(25.0, 25.0, 0.0));
    let modifiers = egui::Modifiers::NONE;
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    press(&h, from, true, modifiers);
    h.run_steps(1);
    for t in [0.1, 0.4, 0.7, 1.0] {
        h.event(egui::Event::PointerMoved(from + (to - from) * t));
        h.run_steps(1);
    }
    press(&h, to, false, modifiers);
    h.run_steps(3);
    assert_eq!(count(&mut h), 3, "{:?}", h.state().log.entries);
    // Remembered in the palette's Recent.
    assert_eq!(
        h.state().settings.palette_recent.first().map(String::as_str),
        Some("utw:nw_wp_tavern")
    );
    let (x, y, _) = waypoint(&mut h, area, 2).unwrap();
    assert!((x - 25.0).abs() < 0.5 && (y - 25.0).abs() < 0.5, "dropped at {x}, {y}");
    // What was dropped moves at once, like anything else: dragged, not
    // placed again.
    let at = screen(&h, area, glam::Vec3::new(x, y, 0.0));
    let to = screen(&h, area, glam::Vec3::new(30.0, 25.0, 0.0));
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    press(&h, at, true, modifiers);
    h.run_steps(1);
    for t in [0.2, 0.6, 1.0] {
        h.event(egui::Event::PointerMoved(at + (to - at) * t));
        h.run_steps(1);
    }
    press(&h, to, false, modifiers);
    h.run_steps(3);
    assert_eq!(count(&mut h), 3, "moved, not placed again");
    let (x, _, _) = waypoint(&mut h, area, 2).unwrap();
    assert!((x - 30.0).abs() < 0.5, "moved to {x}");
}

#[test]
fn placed_objects_preview_as_placed() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("instance-preview") else { return };
    // A human of parts, placed in the area.
    let git = ResKey::new(area, ResType::GIT);
    let entry = {
        let game = h.state().game.as_deref().unwrap();
        let data = game.resman.get_named("nw_bandit001", ResType::UTC).unwrap();
        let mut bp = Gff::read(&data).unwrap().root;
        bp.set("Appearance_Type", mg_gff::Value::Word(6));
        bp.set("Appearance_Head", mg_gff::Value::Byte(1));
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [25.0, 25.0, 0.0], rotation: 0.0 };
        instance(&placing, ResType::UTC, &bp, at, &[]).unwrap()
    };
    let path = mg_edit::GffPath::root().item("Creature List", 0);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "place",
        vec![mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Creature List".into(),
            index: 0,
            item: entry,
        }],
    )));
    h.state_mut()
        .actions
        .push(mg_ui::Action::OpenTab(Tab::InstanceModel { area, path: path.clone() }));
    h.run_steps(4);
    let source = mg_ui::model_view::Source::Instance { area, path: path.clone() };
    let head = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let p = h.state().model_views[&source].preview().expect("a preview of the placed creature");
        p.parts.iter().map(|p| p.model.clone()).filter(|m| m.contains("_head")).collect()
    };
    assert_eq!(head(&h), ["pmh0_head001"]);
    // Its Properties change its head (both fields, as the editor sets them:
    // a placed creature has EE's wide one too): the preview follows.
    let set = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
        key: git,
        path: path.clone(),
        label: label.into(),
        value: Some(value),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Head",
        vec![
            set("Appearance_Head", mg_gff::Value::Byte(3)),
            set("xAppearance_Head", mg_gff::Value::Word(3)),
        ],
    )));
    h.run_steps(4);
    assert_eq!(head(&h), ["pmh0_head003"]);
}

#[test]
fn enter_does_what_a_dialog_s_main_button_does() {
    let root = mg_testkit::corpus!();
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    // Enter on Cancel presses Cancel, not Create.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    h.get_by_label("Cancel").focus();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().ws.is_none() && h.state().wizard.is_none(), "cancelled");
    // A name typed, then Enter: the module, as Create makes it.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    h.get_all_by_value("module000").find(is_input).expect("the name field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("module000").find(is_input).unwrap().type_text("moonglowTEST");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().ws.is_some(), "{:?}", h.state().log.entries);
    assert!(h.state().log.entries.iter().any(|(_, m)| m == "Created module moonglowTEST"));
}

#[test]
fn dialogs_open_with_their_main_field_ready_to_type_in() {
    let root = mg_testkit::corpus!();
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // New Module: typed straight away, no click, the name is replaced...
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("Typed".into()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().log.entries.iter().any(|(_, m)| m == "Created module Typed"));
    // ...and the Area Wizard that follows takes its name the same way.
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("Glade".into()));
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    assert!(h.get_all_by_value("Glade").any(|n| is_input(&n)), "the area's name typed in");
}

#[test]
fn the_start_location_shows_on_raised_ground() {
    // Medieval Rural 2 builds its ground 5 m up; a new module's start
    // location is stored at height 0.
    let Some((mut h, area)) = area_harness_on("start-raised", "trm02", None) else { return };
    h.run_steps(3);
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let stored = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let r = ws.doc(&ifo).unwrap().root.clone();
        (r.read(&ifo::MOD_ENTRY_AREA), r.read(&ifo::MOD_ENTRY_Z))
    };
    assert_eq!(stored(&mut h), (area, 0.0));
    h.get_by_label("🏃 Go to Start Location").click();
    h.run_steps(2);
    let target = h.state().area_views[&area].orbit.unwrap().target;
    assert!((target.z - 5.0).abs() < 0.5, "looks at the ground, not under it: {target}");
    assert_eq!(stored(&mut h), (area, 0.0), "the module's start is as it was");
}

/// Closes a tab, wherever it is.
fn close_tab(app: &mut Moonglow, tab: &Tab) {
    if let Some(path) = app.dock.find_tab(tab) {
        app.dock.remove_tab(path);
    }
}

/// Closes the editors' windows (the tabs not in the main dock), back to the
/// area.
fn close_windows(h: &mut Harness<'_, Moonglow>) {
    let app = h.state_mut();
    let open: Vec<Tab> = app
        .dock
        .iter_all_tabs()
        .filter(|(p, _)| !p.surface.is_main())
        .map(|(_, t)| t.clone())
        .collect();
    for t in open {
        close_tab(app, &t);
    }
}

#[test]
fn editors_open_in_windows_over_the_area() {
    let Some((mut h, area)) = area_harness("editor-windows") else { return };
    h.run_steps(3);
    for tab in [Tab::Factions, Tab::ModuleProperties] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
        h.run_steps(3);
        let dock = &h.state().dock;
        let path = dock.find_tab(&tab).expect("open");
        assert!(!path.surface.is_main(), "{tab:?} in a window of its own");
        // The area still shows in the main pane.
        let area_path = dock.find_tab(&Tab::Area(area)).unwrap();
        assert!(area_path.surface.is_main());
        let (_, leaf) = dock
            .iter_leaves()
            .find(|(p, _)| p.node == area_path.node && p.surface.is_main())
            .unwrap();
        assert_eq!(leaf.tabs[leaf.active.0], Tab::Area(area), "the area stays in front");
    }
    // Below the toolbar, never over it.
    let toolbar = h.get_by_label_contains("Verify").rect().bottom();
    let top = h.get_all_by_label("Factions").map(|n| n.rect().top()).fold(f32::MAX, f32::min);
    assert!(top > toolbar, "the window starts below the toolbar ({top} vs {toolbar})");
    let img = h.render().expect("render");
    let _ = img.save(mg_testkit::scratch_dir("ui-editor-windows").join("windows.png"));
}

#[test]
fn w_a_s_d_drive_the_camera_like_the_arrows() {
    let Some((mut h, area)) = area_harness("wasd") else { return };
    h.run_steps(3);
    let target = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().target;
    let centre = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(centre);
    h.run_steps(1);
    // Held for a few frames, with modifiers.
    let hold = |h: &mut Harness<'_, Moonglow>, key: egui::Key, modifiers: egui::Modifiers| {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        // The modifiers held down all the while, as a hand holds them.
        h.event(egui::Event::ModifiersChanged(modifiers));
        h.event(event(true));
        h.run_steps(4);
        h.event(event(false));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(1);
    };
    let none = egui::Modifiers::NONE;
    let start = target(&h);
    hold(&mut h, egui::Key::ArrowUp, none);
    let by_arrow = target(&h) - start;
    assert!(by_arrow.length() > 0.1, "the arrow moves the view");
    let start = target(&h);
    hold(&mut h, egui::Key::W, none);
    let by_w = target(&h) - start;
    assert!((by_w - by_arrow).length() < 0.05 * by_arrow.length(), "{by_w} like {by_arrow}");
    // Ctrl+W is not a move.
    let start = target(&h);
    hold(&mut h, egui::Key::W, egui::Modifiers::COMMAND);
    assert_eq!(target(&h), start);
    // The pointer gone to the module tree: the keys still drive the view.
    h.hover_at(egui::pos2(60.0, 300.0));
    h.run_steps(2);
    let start = target(&h);
    hold(&mut h, egui::Key::W, none);
    let off_view = target(&h) - start;
    assert!((off_view - by_w).length() < 0.05 * by_w.length(), "{off_view} like {by_w}");
}

#[test]
fn a_right_drag_turns_the_camera_and_w_a_s_d_fly_while_held() {
    let Some((mut h, area)) = area_harness("right-drag") else { return };
    h.run_steps(3);
    let orbit = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap();
    let selection = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].selection.clone();
    let before = orbit(&h);
    let at = screen(&h, area, glam::Vec3::new(20.0, 10.0, 0.0));
    let button = egui::PointerButton::Secondary;
    let modifiers = egui::Modifiers::NONE;
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: at, button, pressed: true, modifiers });
    h.run_steps(1);
    for dx in [20.0, 40.0, 60.0] {
        h.event(egui::Event::PointerMoved(at + egui::vec2(dx, 0.0)));
        h.run_steps(1);
    }
    let turned = orbit(&h);
    assert!((turned.yaw - before.yaw).abs() > 0.3, "turned: {} → {}", before.yaw, turned.yaw);
    // W, with the button still held: the camera moves.
    let key = |pressed| egui::Event::Key {
        key: egui::Key::W,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    };
    h.event(key(true));
    h.run_steps(4);
    h.event(key(false));
    h.run_steps(1);
    assert!((orbit(&h).target - turned.target).length() > 0.1, "moved while turning");
    let end = at + egui::vec2(60.0, 0.0);
    h.event(egui::Event::PointerButton { pos: end, button, pressed: false, modifiers });
    h.run_steps(2);
    assert!(selection(&h).is_empty(), "nothing selected or turned");
    assert!(h.query_by_label("Properties").is_none(), "no context menu after a drag");
}

#[test]
fn placement_tools_turn_ground_arrange_lock_and_make_prefabs() {
    use glam::Vec3;
    use mg_area::ObjectKind::Waypoint;
    let Some((mut h, area)) = area_harness("placement") else { return };
    let prefabs = mg_testkit::scratch_dir("ui-prefabs");
    h.state_mut().prefab_dir = Some(prefabs.clone());
    h.run_steps(3);
    let waypoints = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&ResKey::new(area, ResType::GIT))
            .unwrap()
            .root
            .list("WaypointList")
            .unwrap()
            .to_vec()
    };
    let facing = |s: &mg_gff::Struct| {
        s.float("YOrientation").unwrap().atan2(s.float("XOrientation").unwrap())
    };
    let right_click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    let a = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    let both = vec![(Waypoint, 0), (Waypoint, 1)];
    h.state_mut().area_views.get_mut(&area).unwrap().selection = both.clone();
    h.hover_at(a);
    h.run_steps(1);

    // Q turns the selection 15° to the left, Shift+E 90° to the right.
    let before: Vec<f32> = waypoints(&mut h).iter().map(facing).collect();
    h.key_press(egui::Key::Q);
    h.run_steps(2);
    let after: Vec<f32> = waypoints(&mut h).iter().map(facing).collect();
    for (b, a) in before.iter().zip(&after) {
        assert!((a - b - 15f32.to_radians()).abs() < 1e-4, "{b} → {a}");
    }
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(2);
    let turned: Vec<f32> = waypoints(&mut h).iter().map(facing).collect();
    assert!((after[0] - turned[0] - 90f32.to_radians()).abs() < 1e-4);

    // G drops a raised object to the ground.
    let git = ResKey::new(area, ResType::GIT);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Raise",
        vec![mg_edit::Edit::SetField {
            key: git,
            path: mg_edit::GffPath::root().item("WaypointList", 0),
            label: "ZPosition".into(),
            value: Some(mg_gff::Value::Float(3.0)),
        }],
    )));
    h.run_steps(2);
    h.key_press(egui::Key::G);
    h.run_steps(2);
    let ground = waypoints(&mut h)[0].float("ZPosition").unwrap();
    assert!(ground.abs() < 1.0, "on the ground: {ground}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Drop to Ground"));

    // Arrange › Mirror West–East swaps them.
    right_click(&mut h, a);
    h.get_by_label_contains("Arrange").click();
    h.run_steps(2);
    h.get_by_label("Mirror West–East").click();
    h.run_steps(3);
    let xs: Vec<f32> = waypoints(&mut h).iter().map(|s| s.float("XPosition").unwrap()).collect();
    assert!((xs[0] - 30.0).abs() < 1e-3 && (xs[1] - 20.0).abs() < 1e-3, "{xs:?}");

    // Save as Prefab, then place it again elsewhere.
    let b = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    right_click(&mut h, b);
    h.get_by_label("Save as Prefab…").click();
    h.run_steps(3);
    // The name field has the focus; Enter saves.
    h.event(egui::Event::Text("Two Posts".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(2);
    assert!(prefabs.join("Two Posts.prefab.json").is_file(), "{:?}", h.state().log.entries);
    h.state_mut().actions.push(mg_ui::Action::PlacePrefab("Two Posts".into()));
    h.run_steps(2);
    assert!(h.state().area_views[&area].pasting);
    let c = screen(&h, area, Vec3::new(25.0, 32.0, 0.02));
    h.hover_at(c);
    h.run_steps(2);
    press(&h, c, true, egui::Modifiers::NONE);
    press(&h, c, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(waypoints(&mut h).len(), 4, "{:?}", h.state().log.entries);

    // Lock: clicks pass locked objects by; Unlock All frees them.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = both;
    right_click(&mut h, b);
    h.get_by_label("Lock").click();
    h.run_steps(3);
    let locked = waypoints(&mut h);
    assert!(locked[..2].iter().all(|s| s.byte(mg_area::LOCKED) == Some(1)));
    press(&h, b, true, egui::Modifiers::NONE);
    press(&h, b, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert!(h.state().area_views[&area].selection.is_empty(), "a locked object isn't picked");
    let empty = screen(&h, area, Vec3::new(10.0, 35.0, 0.0));
    right_click(&mut h, empty);
    h.get_by_label("Unlock All (2)").click();
    h.run_steps(3);
    assert!(waypoints(&mut h).iter().all(|s| s.get(mg_area::LOCKED).is_none()));

    // With a 1 m grid, a dragged object lands on whole meters.
    h.state_mut().settings.snap_grid = Some(100);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(Waypoint, 1)];
    let from = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    let to = from + egui::vec2(37.0, 23.0);
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=4 {
        h.hover_at(from + (to - from) * (k as f32 / 4.0));
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let moved = &waypoints(&mut h)[1];
    let (x, y) = (moved.float("XPosition").unwrap(), moved.float("YPosition").unwrap());
    assert!((x, y) != (20.0, 20.0), "it moved");
    assert!((x - x.round()).abs() < 1e-4 && (y - y.round()).abs() < 1e-4, "on the grid: {x}, {y}");
}

#[test]
fn script_navigation_definition_references_rename_and_live_errors() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-script-nav");
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("NAV"));
    m.set_info(&info).unwrap();
    let inc = ResKey::parse("inc_util", ResType::NSS).unwrap();
    let main = ResKey::parse("main_a", ResType::NSS).unwrap();
    m.set(inc, b"// Doubles a number.\nint Twice(int n) { return n * 2; }\n".to_vec());
    m.set(main, b"#include \"inc_util\"\nvoid main()\n{\n    int x = Twice(2);\n}\n".to_vec());
    let path = dir.join("nav.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(main)));
    h.run_steps(3);

    // Definition: Twice is in the include.
    let text = "#include \"inc_util\"\nvoid main()\n{\n    int x = Twice(2);\n}\n";
    let at = text.find("Twice").unwrap() + 2;
    let d = h.state_mut().declaration_at(main, at).expect("a declaration");
    assert_eq!((d.at.file.as_str(), d.doc.as_str()), ("inc_util", "Doubles a number."));
    h.state_mut().go_to_declaration(&d);
    h.run_steps(3);
    assert!(h.state().dock.find_tab(&Tab::Script(inc)).is_some(), "the include opens");

    // References: its definition and its use.
    h.state_mut().show_references(&d);
    let results: Vec<String> = h
        .state()
        .script_tools
        .search
        .results
        .iter()
        .map(|(k, l, _)| format!("{}:{l}", k.resref))
        .collect();
    assert_eq!(results, ["inc_util:1", "main_a:3"]);

    // Rename: both scripts, one undoable step.
    h.state_mut().rename_symbol(&d, "Double");
    h.run_steps(3);
    let text_of = |h: &Harness<'_, Moonglow>, k: ResKey| {
        String::from_utf8(h.state().ws.as_ref().unwrap().module.get(&k).unwrap().to_vec()).unwrap()
    };
    assert!(text_of(&h, inc).contains("int Double(int n)"));
    assert!(text_of(&h, main).contains("int x = Double(2);"));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Rename Twice to Double"));

    // Errors as you type: checked once typing pauses, the line shown.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(main)));
    h.run_steps(3);
    h.state_mut().scripts_mut_for_test(
        main,
        "#include \"inc_util\"\nvoid main()\n{\n    int x = Double(2) +;\n}\n",
    );
    h.run_steps(1);
    std::thread::sleep(std::time::Duration::from_millis(600));
    h.run_steps(2);
    let error = h.state().script_error_for_test(main);
    assert_eq!(error.as_ref().map(|e| e.0), Some(Some(3)), "{error:?}");
    h.get_by_label_contains("⚠ line 4:");
}

#[test]
fn the_palette_opens_with_a_module_as_wide_as_the_tree() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-palette-opens");
    let path = sample_module(&dir);
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let palette_width = |h: &Harness<'_, Moonglow>| {
        let path = h.state().dock.find_tab(&Tab::Palette).expect("the palette is open");
        let pane = h.state().dock.iter_leaves().find(|(p, _)| p.node == path.node).unwrap().1;
        pane.rect.width()
    };
    // The tree's default width, 240 points (give or take the separators).
    let w = palette_width(&h);
    assert!((w - 240.0).abs() <= 12.0, "the palette is {w} points wide");
    // On the area's tiles.
    assert!(h.state().palette.tiles);
    // With every tab closed, the palette comes back as the only pane.
    close_tab(h.state_mut(), &Tab::ModuleProperties);
    close_tab(h.state_mut(), &Tab::Palette);
    h.run();
    h.get_by_label("📦 Palettes").click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Palette).is_some());
}

#[test]
fn the_area_wizard_opens_at_the_top_of_its_tilesets() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-area-wizard-top");
    let path = sample_module(&dir);
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    app.open_palette = false;
    let first = mg_module::new::tilesets(app.game.as_deref().unwrap())[0].name.clone();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let shown = |h: &Harness<'_, Moonglow>| {
        let top = h.get_by_label("Tileset").rect().bottom();
        let bottom = h.get_by_label("Size").rect().top();
        let r = h.get_by_label(&first).rect();
        r.top() >= top && r.bottom() <= bottom
    };
    h.state_mut().actions.push(mg_ui::Action::AreaWizard);
    h.run_steps(3);
    assert!(shown(&h), "{first} is at the top of the list");
    // Scrolled to the end, closed and opened again: at the top again.
    let over = h.get_by_label(&first).rect().center();
    h.hover_at(over);
    for _ in 0..20 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -400.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
    }
    h.run_steps(5);
    assert!(!shown(&h), "the list scrolled");
    h.state_mut().wizard = None;
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::AreaWizard);
    h.run_steps(3);
    assert!(shown(&h), "{first} is at the top of the list again");
}

#[test]
fn a_terrain_brush_paints_every_corner_it_is_dragged_across() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("terrain-drag") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let water = index.terrain("Water").unwrap();
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    h.get_by_label("Water").click();
    h.run_steps(2);
    let before = lattice(&mut h);
    // Dragged from corner (1, 2) to corner (3, 2), 10 m apart each, then
    // run back to (2, 2): (3, 2) is let go.
    let from = screen(&h, area, Vec3::new(10.0, 20.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for x in [12.0, 16.0, 20.0, 24.0, 28.0, 30.0, 27.0, 23.0, 20.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 20.0, 0.0)));
        h.run_steps(1);
    }
    // Nothing is painted before the button is let go.
    assert_eq!(lattice(&mut h), before);
    let to = screen(&h, area, Vec3::new(20.0, 20.0, 0.0));
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let after = lattice(&mut h);
    for x in 1..=2 {
        assert_eq!(after.corner(x, 2).terrain, water, "corner ({x}, 2)");
    }
    for x in [0, 3] {
        assert_eq!(after.corner(x, 2).terrain, before.corner(x, 2).terrain, "corner ({x}, 2)");
    }
    // The drag is one command: one undo takes all of it back.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), before);
}

#[test]
fn cursors_that_only_choose_tiles_again_are_blue() {
    use glam::Vec3;
    use mg_ui::terrain_mode::CYCLE;
    let Some((mut h, area)) = area_harness("cycle-cursor") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    let colors = |h: &Harness<'_, Moonglow>| -> Vec<egui::Color32> {
        h.state().area_views[&area].brush_cursor.iter().map(|(_, c)| *c).collect()
    };
    // A crosser over grass: a drag from there lays it, green.
    let green = egui::Color32::from_rgb(80, 220, 80);
    h.get_by_label("Road").click();
    h.run_steps(2);
    let at = screen(&h, area, Vec3::new(13.0, 15.0, 0.0));
    h.hover_at(at);
    h.run_steps(3);
    assert_eq!(colors(&h), [green]);
    press(&h, at, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for x in [16.0, 19.0, 22.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 15.0, 0.0)));
        h.run_steps(1);
    }
    h.run_steps(2);
    assert!(!colors(&h).contains(&CYCLE), "{:?}", colors(&h));
    let to = screen(&h, area, Vec3::new(22.0, 15.0, 0.0));
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    // Over the road it laid, a click only chooses the tile again: blue.
    h.hover_at(screen(&h, area, Vec3::new(19.0, 15.0, 0.0)));
    h.run_steps(3);
    assert_eq!(colors(&h), [CYCLE]);
    // The Eraser erases, and with Shift steps the tile through those that
    // fit: blue then.
    h.get_by_label("🗑 Eraser").click();
    h.run_steps(2);
    h.hover_at(at);
    h.run_steps(3);
    assert!(!colors(&h).contains(&CYCLE), "{:?}", colors(&h));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    h.run_steps(3);
    assert_eq!(colors(&h), [CYCLE]);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(1);
    // A terrain brush on a corner of its own terrain adds none, it chooses
    // the tiles again: blue; elsewhere it paints. (Corner (3, 3), clear of
    // the road.)
    h.get_by_label("Water").click();
    h.run_steps(2);
    let corner = screen(&h, area, Vec3::new(30.0, 30.0, 0.0));
    h.hover_at(corner);
    h.run_steps(3);
    assert_eq!(colors(&h), [egui::Color32::from_rgb(80, 220, 80)], "grass: it paints");
    for pressed in [true, false] {
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.event(egui::Event::PointerButton { pos: corner, button, pressed, modifiers });
    }
    h.run_steps(3);
    assert_eq!(colors(&h), [CYCLE], "water on water");
}

#[test]
fn the_eraser_erases_every_tile_it_is_dragged_across() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("eraser-drag") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    let drag = |h: &mut Harness<'_, Moonglow>, path: &[Vec3]| {
        let from = screen(h, area, path[0]);
        h.hover_at(from);
        press(h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for &p in &path[1..] {
            h.hover_at(screen(h, area, p));
            h.run_steps(1);
        }
        press(h, screen(h, area, path[path.len() - 1]), false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    h.get_by_label("Road").click();
    h.run_steps(2);
    // Two roads, along the first row and the third.
    for y in [5.0, 25.0] {
        let path: Vec<Vec3> =
            [7.0, 12.0, 18.0, 24.0, 25.0].iter().map(|&x| Vec3::new(x, y, 0.0)).collect();
        drag(&mut h, &path);
    }
    let roads = lattice(&mut h);
    let road = |l: &mg_tiles::Lattice, y: u32| l.cell(1, y).edges.iter().any(Option::is_some);
    assert!(road(&roads, 0) && road(&roads, 2), "two roads drawn");

    // One Eraser drag down across both: each is erased.
    h.get_by_label("🗑 Eraser").click();
    h.run_steps(2);
    let path: Vec<Vec3> =
        [5.0, 10.0, 15.0, 20.0, 25.0].iter().map(|&y| Vec3::new(15.0, y, 0.0)).collect();
    drag(&mut h, &path);
    let l = lattice(&mut h);
    assert!(!road(&l, 0) && !road(&l, 2), "both roads erased");

    // As one command.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), roads);
}

#[test]
fn a_right_click_with_a_crosser_on_its_own_tile_erases_it() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("crosser-right-click") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    let right_click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(2);
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    h.get_by_label("Road").click();
    h.run_steps(2);
    // A road along the first row.
    let from = screen(&h, area, Vec3::new(7.0, 5.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for x in [12.0, 18.0, 24.0, 25.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 5.0, 0.0)));
        h.run_steps(1);
    }
    press(&h, screen(&h, area, Vec3::new(25.0, 5.0, 0.0)), false, egui::Modifiers::NONE);
    h.run_steps(2);
    let road = lattice(&mut h);
    assert!(road.cell(1, 0).edges.iter().any(Option::is_some), "a road drawn");

    // On grass, the brush still chosen: nothing is erased.
    right_click(&mut h, Vec3::new(15.0, 25.0, 0.0));
    assert_eq!(lattice(&mut h), road);
    // On the road's east quarter of tile (1, 0): the tile is erased, as the
    // Eraser's click there, and the brush stays.
    right_click(&mut h, Vec3::new(18.0, 5.0, 0.0));
    assert!((0..4).all(|x| lattice(&mut h).cell(x, 0).edges == [None; 4]));
    assert!(h.state().palette.tile_brush.is_some());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), road);

    // A stream across the road (along the second row, the road turned up
    // the second column): a right click with Road on their tile takes the
    // road, and the stream stays.
    let drag = |h: &mut Harness<'_, Moonglow>, path: &[Vec3]| {
        let from = screen(h, area, path[0]);
        h.hover_at(from);
        press(h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for &p in &path[1..] {
            h.hover_at(screen(h, area, p));
            h.run_steps(1);
        }
        press(h, screen(h, area, path[path.len() - 1]), false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    right_click(&mut h, Vec3::new(18.0, 5.0, 0.0));
    h.get_by_label("Stream").click();
    h.run_steps(2);
    let along: Vec<Vec3> =
        [3.0, 8.0, 14.0, 20.0, 26.0, 32.0, 37.0].iter().map(|&x| Vec3::new(x, 15.0, 0.0)).collect();
    drag(&mut h, &along);
    h.get_by_label("Road").click();
    h.run_steps(2);
    let up: Vec<Vec3> =
        [3.0, 8.0, 14.0, 20.0, 26.0, 32.0, 37.0].iter().map(|&y| Vec3::new(15.0, y, 0.0)).collect();
    drag(&mut h, &up);
    let (road_c, stream_c) = (index.crosser("Road").unwrap(), index.crosser("Stream").unwrap());
    let has = |l: &mg_tiles::Lattice, c| l.cell(1, 1).edges.contains(&Some(c));
    let crossing = lattice(&mut h);
    assert!(has(&crossing, road_c) && has(&crossing, stream_c), "{:?}", crossing.cell(1, 1));
    right_click(&mut h, Vec3::new(15.0, 18.0, 0.0));
    let l = lattice(&mut h);
    assert!(!has(&l, road_c) && has(&l, stream_c), "{:?}", l.cell(1, 1));
}

#[test]
fn the_eraser_and_raise_lower_head_the_terrain_brushes() {
    let Some((mut h, _)) = area_harness("tools-first") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    // Rural lists them Eraser, Grass, Raise/Lower, Road…
    let top = |h: &Harness<'_, Moonglow>, label: &str| h.get_by_label(label).rect().top();
    let (eraser, raise) = (top(&h, "🗑 Eraser"), top(&h, "↕ Raise/Lower"));
    assert!(eraser < raise);
    for other in ["Grass", "Road", "Water"] {
        assert!(raise < top(&h, other), "Raise/Lower is above {other}");
    }
}

#[test]
fn refine_tile_steps_a_tile_through_those_that_fit_and_paints_nothing() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("refine-tile") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    // Among the tools at the top: after the Eraser, before Raise/Lower.
    let top = |h: &Harness<'_, Moonglow>, label: &str| h.get_by_label(label).rect().top();
    let refine = top(&h, "🔁 Refine Tile");
    assert!(top(&h, "🗑 Eraser") < refine && refine < top(&h, "↕ Raise/Lower"));
    h.get_by_label("🔁 Refine Tile").click();
    h.run_steps(2);
    let before = grid(&mut h);
    let next = mg_tiles::paint::next_fit(&index, &before.lattice.cell(1, 1), before.tile(1, 1))
        .expect("Rural's grass has other tiles");
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    h.run_steps(3);
    let cursor: Vec<_> = h.state().area_views[&area].brush_cursor.iter().map(|(_, c)| *c).collect();
    assert_eq!(cursor, [mg_ui::terrain_mode::CYCLE]);
    for pressed in [true, false] {
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
    }
    h.run_steps(3);
    let after = grid(&mut h);
    assert_eq!(after.tile(1, 1), next, "the next tile that fits");
    assert_eq!(after.lattice, before.lattice, "no terrain painted");
    for (x, y) in [(0, 1), (2, 1), (1, 0), (1, 2)] {
        assert_eq!(after.tile(x, y), before.tile(x, y), "({x}, {y}) is as it was");
    }
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(grid(&mut h).tile(1, 1), before.tile(1, 1));
}

#[test]
fn a_terrain_drag_with_shift_fills_its_rectangle() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("terrain-fill") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let water = index.terrain("Water").unwrap();
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    h.get_by_label("Water").click();
    h.run_steps(2);
    let before = lattice(&mut h);
    // Corner (1, 1) diagonally to (3, 2), Shift held.
    let from = screen(&h, area, Vec3::new(10.0, 10.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    for t in [0.2, 0.4, 0.6, 0.8, 1.0] {
        let p = Vec3::new(10.0, 10.0, 0.0).lerp(Vec3::new(30.0, 20.0, 0.0), t);
        h.hover_at(screen(&h, area, p));
        h.run_steps(1);
    }
    let to = screen(&h, area, Vec3::new(30.0, 20.0, 0.0));
    press(&h, to, false, egui::Modifiers::SHIFT);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let after = lattice(&mut h);
    for y in 1..=2 {
        for x in 1..=3 {
            assert_eq!(after.corner(x, y).terrain, water, "corner ({x}, {y})");
        }
    }
    for (x, y) in [(0, 1), (4, 2), (2, 0), (2, 3)] {
        assert_eq!(after.corner(x, y).terrain, before.corner(x, y).terrain, "({x}, {y})");
    }
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), before);
}

#[test]
fn a_crosser_drag_with_shift_follows_its_rectangle_s_outline() {
    use glam::Vec3;
    use mg_tiles::{EAST, NORTH, SOUTH, TileIndex, WEST};
    let Some((mut h, area)) = area_harness("crosser-outline") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let road = index.crosser("Road").unwrap();
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    h.get_by_label("Road").click();
    h.run_steps(2);
    let before = lattice(&mut h);
    // Tile (0, 0) diagonally to tile (2, 2), Shift held.
    let from = screen(&h, area, Vec3::new(5.0, 5.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    for t in [0.25, 0.5, 0.75, 1.0] {
        let p = Vec3::new(5.0, 5.0, 0.0).lerp(Vec3::new(25.0, 25.0, 0.0), t);
        h.hover_at(screen(&h, area, p));
        h.run_steps(1);
    }
    let to = screen(&h, area, Vec3::new(25.0, 25.0, 0.0));
    press(&h, to, false, egui::Modifiers::SHIFT);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let l = lattice(&mut h);
    let edges = |x, y| l.cell(x, y).edges;
    let on = |list: &[usize]| {
        let mut e = [None; 4];
        for &i in list {
            e[i] = Some(road);
        }
        e
    };
    assert_eq!(edges(0, 0), on(&[EAST, NORTH]), "{:?}", h.state().log.entries);
    assert_eq!(edges(1, 0), on(&[EAST, WEST]));
    assert_eq!(edges(2, 0), on(&[WEST, NORTH]));
    assert_eq!(edges(2, 1), on(&[SOUTH, NORTH]));
    assert_eq!(edges(2, 2), on(&[SOUTH, WEST]));
    assert_eq!(edges(1, 2), on(&[EAST, WEST]));
    assert_eq!(edges(0, 2), on(&[EAST, SOUTH]));
    assert_eq!(edges(0, 1), on(&[NORTH, SOUTH]));
    assert_eq!(edges(1, 1), [None; 4], "nothing inside");
    assert_eq!(edges(3, 3), before.cell(3, 3).edges);
}

#[test]
fn painting_with_shift_held_the_camera_still_zooms_and_turns() {
    let Some((mut h, area)) = area_harness("shift-camera") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    h.get_by_label("Road").click();
    h.run_steps(2);
    let orbit = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap();
    let at = screen(&h, area, glam::Vec3::new(20.0, 10.0, 0.0));
    h.hover_at(at);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    h.run_steps(1);
    // The wheel (which egui turns sideways with Shift) zooms.
    let before = orbit(&h);
    for _ in 0..5 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 60.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::SHIFT,
        });
        h.run_steps(1);
    }
    h.run_steps(10);
    let zoomed = orbit(&h);
    assert!(zoomed.distance < before.distance - 0.5, "{} → {}", before.distance, zoomed.distance);
    // A right drag turns it.
    let button = egui::PointerButton::Secondary;
    let modifiers = egui::Modifiers::SHIFT;
    h.event(egui::Event::PointerButton { pos: at, button, pressed: true, modifiers });
    h.run_steps(1);
    for dx in [20.0, 40.0, 60.0] {
        h.event(egui::Event::PointerMoved(at + egui::vec2(dx, 0.0)));
        h.run_steps(1);
    }
    let pos = at + egui::vec2(60.0, 0.0);
    h.event(egui::Event::PointerButton { pos, button, pressed: false, modifiers });
    h.run_steps(1);
    let turned = orbit(&h);
    assert!((turned.yaw - zoomed.yaw).abs() > 0.3, "turned: {} → {}", zoomed.yaw, turned.yaw);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(1);
}

#[test]
fn a_tile_brush_previews_the_tiles_its_click_makes() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("tile-preview") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    // What the preview shows is what the click puts down.
    let check = |h: &mut Harness<'_, Moonglow>, at: Vec3| {
        let pos = screen(h, area, at);
        h.hover_at(pos);
        h.run_steps(3);
        let shown = h.state().area_views[&area].tile_preview.clone();
        assert!(!shown.is_empty(), "a preview at {at}");
        for pressed in [true, false] {
            let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
            h.event(egui::Event::PointerButton { pos, button, pressed, modifiers });
        }
        h.run_steps(3);
        let g = grid(h);
        for t in &shown {
            let placed = g.tile(t.column, t.row);
            assert_eq!(
                (i64::from(placed.tile), placed.orientation, placed.height),
                (t.id, t.orientation, t.height),
                "tile ({}, {})",
                t.column,
                t.row
            );
        }
    };
    // Water on a corner (its four tiles, chosen at random among those that
    // fit), and a barn.
    h.get_by_label("Water").click();
    h.run_steps(2);
    check(&mut h, Vec3::new(20.0, 20.0, 0.0));
    h.get_by_label("Groups").click();
    h.run_steps(2);
    h.get_by_label("Barn 1 2x2").click();
    h.run_steps(2);
    check(&mut h, Vec3::new(5.0, 5.0, 0.0));
}

#[test]
fn an_area_is_renamed_from_its_tab() {
    let Some((mut h, area)) = area_harness("rename-tab") else { return };
    h.run_steps(3);
    // Right click on the area's tab: Rename….
    h.get_by_label(&area.to_string()).click_secondary();
    h.run_steps(2);
    h.get_by_label("Rename…").click();
    h.run_steps(2);
    // (The dialog's own typing is another test's.)
    h.state_mut().rename.as_mut().expect("the Rename dialog").to = "meadow".into();
    h.run();
    h.get_by_label("Rename").click();
    h.run_steps(3);
    let meadow = ResRef::from_str("meadow").unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    for t in [ResType::ARE, ResType::GIT] {
        assert!(ws.module.contains(&ResKey::new(meadow, t)), "{t:?} renamed");
        assert!(!ws.module.contains(&ResKey::new(area, t)));
    }
    // The module's area list names it, and its tab and view follow.
    let ifo = ws.module.info().unwrap();
    let areas: Vec<_> = ifo
        .root
        .list("Mod_Area_list")
        .unwrap()
        .iter()
        .filter_map(|a| a.resref("Area_Name"))
        .collect();
    assert_eq!(areas, [meadow]);
    assert!(h.state().dock.find_tab(&Tab::Area(meadow)).is_some());
    assert!(h.state().area_views.contains_key(&meadow));
}

#[test]
fn a_store_s_preview_lists_what_it_sells() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-store-preview");
    let path = sample_module(&dir);
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    app.open_palette = false;
    // The thieves' store of Hordes of the Underdark (its jewelry page has
    // rings and amulets), as the palette's double click opens it.
    let key = ResKey::parse("x2_storethief001", ResType::UTM).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Model(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    assert!(h.query_by_label_contains("no preview").is_none());
    h.get_by_label("Blueprint ResRef");
    h.get_by_label_contains("Rings & Amulets (");
}

#[test]
fn a_drag_previews_the_tiles_letting_go_paints() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("drag-preview") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    h.get_by_label("🗻 Tiles").click();
    h.run_steps(2);
    // Dragged along `path` (metres): before it is let go, the preview; let
    // go, those very tiles.
    let check = |h: &mut Harness<'_, Moonglow>, path: &[Vec3]| {
        let before = grid(h);
        let from = screen(h, area, path[0]);
        h.hover_at(from);
        press(h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for p in &path[1..] {
            h.hover_at(screen(h, area, *p));
            h.run_steps(1);
        }
        h.run_steps(2);
        let shown = h.state().area_views[&area].tile_preview.clone();
        assert!(!shown.is_empty(), "a preview while dragging");
        assert_eq!(grid(h).tiles, before.tiles, "nothing painted yet");
        let to = screen(h, area, *path.last().unwrap());
        press(h, to, false, egui::Modifiers::NONE);
        h.run_steps(3);
        let g = grid(h);
        for t in &shown {
            let placed = g.tile(t.column, t.row);
            assert_eq!(
                (i64::from(placed.tile), placed.orientation, placed.height),
                (t.id, t.orientation, t.height),
                "tile ({}, {})",
                t.column,
                t.row
            );
        }
    };
    h.get_by_label("Water").click();
    h.run_steps(2);
    check(&mut h, &[10.0, 15.0, 20.0, 25.0, 30.0].map(|x| Vec3::new(x, 20.0, 0.0)));
    h.get_by_label("Road").click();
    h.run_steps(2);
    check(&mut h, &[7.0, 12.0, 18.0, 24.0, 25.0].map(|x| Vec3::new(x, 5.0, 0.0)));
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

/// The plugin host's fixture plugins, as an installed plugins folder.
fn plugin_fixtures() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mg-plugin/tests/fixtures")
}

/// The sample module with a guard whose tag is in lower case, and the
/// fixture plugins installed (none enabled).
fn plugin_harness(name: &str, game: bool) -> Option<(Harness<'static, Moonglow>, ResKey)> {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut app = if game {
        let root = mg_testkit::nwn_root()?;
        Moonglow::new(
            Some(mg_resman::GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        )
    } else {
        app_with(Vec::new())
    };
    app.open_module(&path);
    app.open_palette = false;
    let key = ResKey::parse("guard", ResType::UTC).unwrap();
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("Tag", mg_gff::Value::String(b"gate_guard".to_vec()));
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A guard",
        vec![mg_edit::Edit::SetResource { key, data: Some(guard.to_bytes().unwrap()) }],
    )));
    app.plugin_dir = Some(plugin_fixtures());
    app.load_plugins();
    let h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    Some((h, key))
}

fn tag_of(h: &mut Harness<'_, Moonglow>, key: &ResKey) -> String {
    String::from_utf8(field(h, key).string("Tag").unwrap().to_vec()).unwrap()
}

#[test]
fn a_plugin_is_off_until_enabled_and_its_command_is_one_undoable_step() {
    let Some((mut h, key)) = plugin_harness("ui-plugins", false) else { return };
    h.run();
    // Installed, none enabled: the Plugins menu has only its manager.
    assert_eq!(h.state().plugins.installed.len(), 4);
    assert!(h.state().plugin_commands().is_empty());
    h.get_by_label("Plugins").click();
    h.run();
    assert!(h.query_by_label("Fix Creature Tags").is_none());
    h.get_by_label("Manage Plugins…").click();
    h.run();
    // The window says what each adds; enabling one is a tick.
    h.get_by_label_contains("cannot reach your files");
    h.get_by_label_contains("Plugins are experimental: the plugin API (0.1)");
    h.get_by_label("Command: Fix Creature Tags");
    h.get_by_label("Check: Creature tags are upper case");
    // (The last of the four installed: below the list's fold.)
    h.get_by_label("Tag conventions 1.0.0").scroll_to_me();
    h.run();
    h.get_by_label("Tag conventions 1.0.0").click();
    h.run();
    assert_eq!(h.state().settings.plugins_enabled, ["example.tag-conventions"]);
    h.state_mut().plugins.window = false;
    h.run();

    // Its commands are in the Plugins menu; one runs as one undoable step.
    h.get_by_label("Plugins").click();
    h.run();
    h.get_by_label("Promote the Guards");
    h.get_by_label("Fix Creature Tags").click();
    h.run();
    assert_eq!(tag_of(&mut h, &key), "GATE_GUARD");
    let log: Vec<String> = h.state().log.entries.iter().map(|(_, m)| m.clone()).collect();
    assert!(log.contains(&"Tag conventions: 1 creature tags changed".to_string()), "{log:#?}");
    assert!(
        log.iter().any(|m| m.contains("Fix Creature Tags changed 1 resource (Edit › Undo")),
        "{log:#?}"
    );
    assert_eq!(
        h.state().ws.as_ref().unwrap().can_undo(),
        Some("Upper-case creature tags"),
        "named by the plugin"
    );
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(tag_of(&mut h, &key), "gate_guard");
    // Run again with nothing to do: it says so, and adds no step.
    h.state_mut().run_plugin_command("example.tag-conventions", "fix-tags");
    h.run();
    h.state_mut().run_plugin_command("example.tag-conventions", "fix-tags");
    h.run();
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("changed nothing")));

    // The Command Palette finds it, under its plugin's name.
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
    h.run();
    type_into_hint(&mut h, "Type a command's name", "tag conv");
    h.get_by_label("Promote the Guards");
    h.key_press(egui::Key::Escape);
    h.run();
    // And it takes a key like any command: listed with the plugins'.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Keyboard").click();
    h.run();
    type_into_hint(&mut h, "Find a command", "plugins");
    h.get_by_label("Tag conventions: Fix Creature Tags");
    h.state_mut().options.as_mut().unwrap().recording =
        Some("example.tag-conventions/fix-tags".to_string());
    h.run();
    h.key_press(egui::Key::F6);
    h.run();
    h.get_by_label("OK").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(tag_of(&mut h, &key), "gate_guard");
    h.key_press(egui::Key::F6);
    h.run();
    assert_eq!(tag_of(&mut h, &key), "GATE_GUARD", "F6 runs the plugin's command");

    // Disabled again: its commands are gone.
    h.state_mut().enable_plugin("example.tag-conventions", false);
    h.run();
    assert!(h.state().plugin_commands().is_empty());
    // A plugin whose code fails changes nothing and says why.
    h.state_mut().enable_plugin("test.hostile", true);
    h.state_mut().run_plugin_command("test.hostile", "fails");
    h.run();
    assert!(
        h.state()
            .log
            .entries
            .iter()
            .any(|(l, m)| { *l == mg_ui::Level::Error && m.contains("something went wrong") })
    );
    assert_eq!(tag_of(&mut h, &key), "GATE_GUARD");
}

/// A plugin's questions show in its job's window, one after another, and
/// its work goes on with the answers.
#[test]
fn a_plugin_asks_in_its_job_s_window() {
    let Some((mut h, key)) = plugin_harness("ui-plugin-forms", false) else { return };
    h.state_mut().enable_plugin("test.forms", true);
    h.state_mut().background_jobs = true;
    h.run();
    h.state_mut().run_plugin_command("test.forms", "rename");
    // Frames until something with this label shows (the job's thread asks).
    let wait_for = |h: &mut Harness<'_, Moonglow>, label: &str| {
        let started = std::time::Instant::now();
        while h.query_by_label(label).is_none() {
            h.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(5));
            assert!(started.elapsed() < std::time::Duration::from_secs(20), "no {label:?}");
        }
        // (The window settles where it stands before it is clicked.)
        h.run_steps(3);
    };
    // The form, as the plugin described it, with its defaults.
    wait_for(&mut h, "Prefix Tags");
    h.get_by_label("Forms: Prefix Tags");
    h.get_by_label("At most");
    assert!(h.get_all_by_value("NPC_").next().is_some(), "the prefix's default");
    h.get_by_label("OK").click();
    // Then its question, then its message.
    wait_for(&mut h, "Prefix up to 10 creatures?");
    h.get_by_label("Yes").click();
    wait_for(&mut h, "Done.");
    h.get_by_label("OK").click();
    let started = std::time::Instant::now();
    while h.state().busy() {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "the job never ended");
    }
    h.run_steps(2);
    assert_eq!(tag_of(&mut h, &key), "NPC_GATE_GUARD");

    // Canceled at the form: nothing changes.
    h.state_mut().run_plugin_command("test.forms", "rename");
    wait_for(&mut h, "Prefix Tags");
    h.get_by_label("Cancel").click();
    let started = std::time::Instant::now();
    while h.state().busy() {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "the job never ended");
    }
    h.run_steps(2);
    assert_eq!(tag_of(&mut h, &key), "NPC_GATE_GUARD");
}

#[test]
fn the_plugin_console_runs_what_is_typed() {
    let Some((mut h, key)) = plugin_harness("ui-plugin-console", false) else { return };
    h.run();
    h.state_mut().plugins.window = true;
    h.state_mut().plugins.console =
        "ctx.edit:set(\"guard.utc\", \"Tag\", \"TYPED\")\nreturn ctx.module:gff(\"guard.utc\").Tag"
            .into();
    h.run();
    h.get_by_label("Console").click();
    h.run();
    h.get_by_label("Run").click();
    h.run();
    assert_eq!(tag_of(&mut h, &key), "TYPED");
    h.get_by_label("\"TYPED\"");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Plugin console"));
    // A fault shows under the console, and changes nothing.
    h.state_mut().plugins.console = "return nothing.here".into();
    h.run();
    h.get_by_label("Run").click();
    h.run();
    // (Under the console, and in the log.)
    assert_eq!(h.get_all_by_label_contains("attempt to index nil").count(), 2);
    assert_eq!(tag_of(&mut h, &key), "TYPED");
}

/// A plugin's checks run with Verify Module (which needs the game).
#[test]
fn a_plugin_s_checks_run_with_verify() {
    let Some((mut h, _)) = plugin_harness("ui-plugin-verify", true) else {
        eprintln!("skipped: no game install");
        return;
    };
    h.state_mut().enable_plugin("example.tag-conventions", true);
    h.state_mut().enable_plugin("test.hostile", true);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Verify);
    h.run();
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Warning
            && m.contains("guard.utc › Tag: tag \"gate_guard\" is not upper case")),
        "{log:#?}"
    );
    // A check that breaks the rules is an error of its own, not a crash.
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Error && m.contains("a check only reads")),
        "{log:#?}"
    );
}

/// Plugins › Install Plugin from File…: a plugin's archive goes into the
/// plugins folder, off; installing it again asks before it replaces; what
/// is no plugin's archive changes nothing.
#[test]
fn a_plugin_installs_from_a_file_and_is_off() {
    let dir = mg_testkit::scratch_dir("ui-plugin-install");
    let plugins = dir.join("plugins");
    // The fixture plugin's folder, to pack as it is and as a later version.
    let work = dir.join("work/tag-conventions");
    std::fs::create_dir_all(&work).unwrap();
    for file in ["plugin.cfg", "main.luau", "rules.luau"] {
        std::fs::copy(plugin_fixtures().join("tag-conventions").join(file), work.join(file))
            .unwrap();
    }
    let pack = |to: &str| {
        let plugin = mg_plugin::Plugin::load(&work).unwrap();
        let path = dir.join(to);
        std::fs::write(&path, mg_plugin::pack(&plugin).unwrap()).unwrap();
        path
    };
    let first = pack("tag-conventions-1.0.0.zip");
    let manifest = std::fs::read_to_string(work.join("plugin.cfg")).unwrap();
    std::fs::write(work.join("plugin.cfg"), manifest.replace("1.0.0", "1.1.0")).unwrap();
    let second = pack("tag-conventions-1.1.0.zip");
    let junk = dir.join("junk.zip");
    std::fs::write(&junk, "not an archive").unwrap();

    let mut app = app_with(vec![first]);
    app.plugin_dir = Some(plugins.clone());
    // (Enabled once, and removed since: installing does not bring that back.)
    app.settings.plugins_enabled = vec!["example.tag-conventions".to_string()];
    app.load_plugins();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Plugins").click();
    h.run();
    h.get_by_label("Install Plugin from File…").click();
    h.run();
    // (In the window, and in the log.)
    let said = |h: &Harness<'_, Moonglow>, text: &str| h.get_all_by_label(text).count() == 2;
    assert!(said(
        &h,
        "Installed Tag conventions 1.0.0 from tag-conventions-1.0.0.zip: it is off until you \
         enable it"
    ));
    assert_eq!(h.state().plugins.installed.len(), 1);
    assert!(!h.state().plugin_enabled("example.tag-conventions"));
    assert!(plugins.join("example.tag-conventions/main.luau").is_file());
    h.get_by_label("Tag conventions 1.0.0").click();
    h.run();
    assert!(h.state().plugin_enabled("example.tag-conventions"));

    // Again, from the window: asked first, and Cancel leaves it.
    let install = |h: &mut Harness<'_, Moonglow>, file: &std::path::Path| {
        h.state_mut().dialogs =
            Box::new(NoDialogs { open: vec![file.to_path_buf()], ..Default::default() });
        h.get_by_label("Install from File…").click();
        h.run();
        h.run();
    };
    install(&mut h, &second);
    h.get_by_label("Replace Tag conventions?");
    h.get_by_label(
        "Tag conventions 1.0.0 is installed. Replace it with 1.1.0 from \
         tag-conventions-1.1.0.zip?",
    );
    h.get_by_label_contains("It is enabled, and stays enabled.");
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().plugins.replace.is_none());
    h.get_by_label("Tag conventions 1.0.0");
    install(&mut h, &second);
    h.get_by_label("Replace").click();
    h.run();
    h.get_by_label("Tag conventions 1.1.0");
    assert!(said(
        &h,
        "Installed Tag conventions 1.1.0 from tag-conventions-1.1.0.zip: it stays enabled"
    ));
    assert!(h.state().plugin_enabled("example.tag-conventions"));

    // What is no plugin's archive says why, and changes nothing.
    install(&mut h, &junk);
    assert!(said(&h, "junk.zip was not installed: it is not a zip archive"));
    assert!(h.state().log.entries.iter().any(|(l, m)| *l == mg_ui::Level::Error
        && m == "junk.zip was not installed: it is not a zip archive"));
    assert_eq!(h.state().plugins.installed.len(), 1);
    h.get_by_label("Tag conventions 1.1.0");

    // Remove: what Install from File put there, after a question. A
    // plugin copied in by hand has no Remove: Moonglow deletes only what
    // it installed.
    let by_hand = plugins.join("by-hand");
    std::fs::create_dir_all(&by_hand).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        std::fs::copy(plugin_fixtures().join("forms").join(file), by_hand.join(file)).unwrap();
    }
    h.get_by_label("Reload").click();
    h.run();
    let disabled: Vec<bool> =
        h.get_all_by_label("Remove…").map(|n| n.accesskit_node().is_disabled()).collect();
    assert_eq!(disabled, [true, false], "by-hand, then the one installed");
    let remove = |h: &mut Harness<'_, Moonglow>| {
        let button =
            h.get_all_by_label("Remove…").find(|n| !n.accesskit_node().is_disabled()).unwrap();
        button.scroll_to_me();
        h.run();
        h.get_all_by_label("Remove…").find(|n| !n.accesskit_node().is_disabled()).unwrap().click();
        h.run();
        h.run();
    };
    remove(&mut h);
    h.get_by_label("Remove Tag conventions 1.1.0?");
    h.get_by_label("Cancel").click();
    h.run();
    assert!(plugins.join("example.tag-conventions").is_dir());
    remove(&mut h);
    h.get_by_label("Remove").click();
    h.run();
    assert!(!plugins.join("example.tag-conventions").exists());
    assert!(said(&h, "Removed Tag conventions 1.1.0"));
    assert!(!h.state().plugin_enabled("example.tag-conventions"));
    assert_eq!(h.state().plugins.installed.len(), 1);
    assert!(by_hand.join("main.luau").is_file());
}

/// Started with `--no-plugins`: none is loaded, whatever is installed and
/// was enabled, and Manage Plugins says so.
#[test]
fn started_without_plugins_none_is_loaded() {
    let mut app = app_with(Vec::new());
    app.plugin_dir = Some(plugin_fixtures());
    app.settings.plugins_enabled = vec!["example.tag-conventions".to_string()];
    app.no_plugins = true;
    app.load_plugins();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    assert!(h.state().plugins.installed.is_empty());
    assert!(h.state().plugin_commands().is_empty());
    h.get_by_label("Plugins").click();
    h.run();
    assert!(h.query_by_label("Fix Creature Tags").is_none());
    h.get_by_label("Manage Plugins…").click();
    h.run();
    h.get_by_label("Started with --no-plugins: none are loaded.");
    assert!(h.query_by_label_contains("No plugins are installed").is_none());
    // (What was enabled stays enabled for the next start.)
    assert_eq!(h.state().settings.plugins_enabled, ["example.tag-conventions"]);
}

/// Edit › Edit Areas Together…: areas ticked in the chooser (narrowed by
/// its filters) open in one Area Properties, where a change is set on each:
/// a field, a flag (each area keeping its other flags), and the variables
/// added or changed (each keeping its own).
#[test]
fn several_areas_are_edited_together() {
    let dir = mg_testkit::scratch_dir("ui-areas-together");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let key = |name: &str, t| ResKey::parse(name, t).unwrap();
    let int_var = |name: &str, value: i32| {
        let mut s = mg_gff::Struct::new(0);
        s.set("Name", mg_gff::Value::String(name.as_bytes().to_vec()));
        s.set("Type", mg_gff::Value::Dword(1));
        s.set("Value", mg_gff::Value::Int(value));
        s
    };
    // Two caves (underground) and an inn; the caves' flags and variables
    // differ.
    for (name, title, flags, vars) in [
        ("cave1", "Wolf Cave", 0x7u32, vec![int_var("nDepth", 1)]),
        ("cave2", "Bear Cave", 0x3 | 0x100, vec![int_var("nBears", 4)]),
        ("inn", "Apple Inn", 0x1, Vec::new()),
    ] {
        let mut are = Gff::new(*b"ARE ");
        let title = LocString::from_text(Language::ENGLISH, Gender::Male, title);
        are.root.set("Name", mg_gff::Value::LocString(title));
        are.root.set("Flags", mg_gff::Value::Dword(flags));
        are.root.set("DayNightCycle", mg_gff::Value::Byte(1));
        are.root.set("IsNight", mg_gff::Value::Byte(0));
        are.root.set("VarTable", mg_gff::Value::List(vars));
        m.set(key(name, ResType::ARE), are.to_bytes().unwrap());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let are = |h: &mut Harness<'_, Moonglow>, name: &str| -> mg_gff::Struct {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&ResKey::parse(name, ResType::ARE).unwrap()).unwrap().root.clone()
    };

    // The chooser: the underground areas, ticked.
    h.get_by_label("Edit").click();
    h.run();
    h.get_by_label("Edit Areas Together…").click();
    h.run();
    h.get_by_label_contains("Apple Inn");
    h.get_by_value("Above or under ground").click();
    h.run();
    h.get_all_by_label("Underground").last().unwrap().click();
    h.run();
    assert!(h.query_by_label_contains("Apple Inn").is_none(), "the inn is above ground");
    assert!(h.query_by_label("Edit 0 Together").is_some());
    h.get_by_label("Tick Shown").click();
    h.run();
    h.get_by_label("Edit 2 Together").click();
    h.run_steps(3);
    let caves: Vec<ResRef> = ["cave1", "cave2"].map(|n| ResRef::from_str(n).unwrap()).to_vec();
    assert!(h.state().dock.find_tab(&Tab::AreasProperties(caves.clone())).is_some());
    assert!(h.query_by_label("Edit Areas Together").is_none(), "the chooser closed");

    // Visual: always dark, on both caves and not on the inn.
    h.get_by_label("Always Dark").click();
    h.run_steps(2);
    for (name, night) in [("cave1", 1), ("cave2", 1), ("inn", 0)] {
        let a = are(&mut h, name);
        assert_eq!(
            (a.integer("IsNight"), a.integer("DayNightCycle")),
            (Some(night), Some(1 - night))
        );
    }
    // Advanced: no tag or ResRef for several; a flag is cleared on each,
    // which keeps its other flags.
    h.get_all_by_label("Advanced").last().unwrap().click();
    h.run_steps(2);
    assert!(h.query_by_label("ResRef").is_none());
    h.get_by_label("Names, tags and comments are edited one area at a time.");
    h.get_by_label("Above ground").click();
    h.run_steps(2);
    let flags = |h: &mut Harness<'_, Moonglow>, name: &str| are(h, name).integer("Flags");
    assert_eq!(flags(&mut h, "cave1"), Some(0x5));
    assert_eq!(flags(&mut h, "cave2"), Some(0x1 | 0x100));
    assert_eq!(flags(&mut h, "inn"), Some(0x1));
    // Variables: one added is added to each, which keeps its own.
    h.get_by_label("Variables (1)…").click();
    h.run();
    h.get_by_label("Add").click();
    h.run();
    {
        let row = h.state_mut().var_edit.as_mut().unwrap().rows.last_mut().unwrap();
        row.name = "nMusic".into();
        row.value = "42".into();
    }
    h.run();
    h.get_by_label("OK").click();
    h.run_steps(2);
    let names = |h: &mut Harness<'_, Moonglow>, name: &str| -> Vec<String> {
        let a = are(h, name);
        let vars = a.list("VarTable").unwrap_or(&[]);
        vars.iter()
            .map(|v| String::from_utf8_lossy(v.string("Name").unwrap()).into_owned())
            .collect()
    };
    assert_eq!(names(&mut h, "cave1"), ["nDepth", "nMusic"]);
    assert_eq!(names(&mut h, "cave2"), ["nBears", "nMusic"]);
    assert!(names(&mut h, "inn").is_empty());
    // One undo takes the variable from both.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert_eq!(names(&mut h, "cave2"), ["nBears"]);
    assert_eq!(names(&mut h, "cave1"), ["nDepth"]);
}

/// Export as Files and Copy to Scratch Folder: a script goes out with its
/// compiled script, as it is now in the toolset (saved or not), into the
/// folder chosen; the scratch folder is asked for once, then kept.
#[test]
fn resources_are_exported_as_files() {
    let dir = mg_testkit::scratch_dir("ui-export-files");
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let ncs = ResKey::parse("hello", ResType::NCS).unwrap();
    m.set(ncs, b"NCS old".to_vec());
    m.save().unwrap();
    let (chosen, scratch) = (dir.join("out"), dir.join("scratch"));
    // (The folders the dialog gives, the last first.)
    let folders = vec![scratch.clone(), chosen.clone()];
    let mut app = Moonglow::new(None, Box::new(NoDialogs { folders, ..Default::default() }));
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Compiled again in the toolset, not saved: what is exported.
    let edit = mg_edit::Edit::SetResource { key: ncs, data: Some(b"NCS new".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Compile", vec![edit])));
    h.run();
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    let export = |h: &mut Harness<'_, Moonglow>, scratch: bool| {
        let keys = vec![script];
        h.state_mut().actions.push(mg_ui::Action::ExportFiles {
            keys,
            dependencies: false,
            scratch,
        });
        h.run();
    };
    export(&mut h, false);
    assert_eq!(h.state().settings.scratch_dir, None, "Export as Files isn't the scratch folder");
    export(&mut h, true);
    for folder in [&chosen, &scratch] {
        assert_eq!(std::fs::read(folder.join("hello.ncs")).unwrap(), b"NCS new", "{folder:?}");
        assert_eq!(std::fs::read(folder.join("hello.nss")).unwrap(), b"void main() { }\n");
        assert_eq!(std::fs::read_dir(folder).unwrap().count(), 2);
    }
    let log = &h.state().log.entries;
    assert!(log.iter().any(|(_, m)| m.contains("hello.nss, hello.ncs")), "{log:?}");
    // The scratch folder is kept: not asked for again (the dialog has no
    // more folders to give).
    assert_eq!(h.state().settings.scratch_dir.as_deref(), Some(scratch.as_path()));
    std::fs::remove_file(scratch.join("hello.ncs")).unwrap();
    export(&mut h, true);
    assert!(scratch.join("hello.ncs").exists());
    // The Export window offers files too, for the resources ticked.
    h.state_mut().actions.push(mg_ui::Action::ExportDialog(vec![script]));
    h.run();
    h.get_by_label("Export 1 as Files…");
}

/// The script editor's To Scratch: the script saved and compiled, then it
/// and its compiled script copied into the scratch folder; one that
/// doesn't compile is not copied.
#[test]
fn a_script_goes_to_the_scratch_folder_compiled() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-script-scratch");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let open = |module: &std::path::Path, scratch: &std::path::Path| {
        let install = mg_resman::GameInstall::new(&root, None, "en");
        let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
        app.settings.scratch_dir = Some(scratch.into());
        app.open_module(module);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run();
        h
    };
    let scratch = dir.join("scratch");
    let mut h = open(&path, &scratch);
    h.get_by_label("To Scratch").click();
    h.run_steps(3);
    assert_eq!(std::fs::read(scratch.join("hello.nss")).unwrap(), b"void main() { }\n");
    let ncs = std::fs::read(scratch.join("hello.ncs")).unwrap();
    assert!(ncs.starts_with(b"NCS V1.0"), "a compiled script");
    // A script that doesn't compile is not copied.
    let broken_path = sample_module(&dir.join("broken"));
    let mut m = Module::open(&broken_path).unwrap();
    m.set(key, b"void main() { x = 1; }\n".to_vec());
    m.save().unwrap();
    let scratch = dir.join("scratch2");
    let mut h = open(&broken_path, &scratch);
    h.get_by_label("To Scratch").click();
    h.run_steps(3);
    assert!(h.state().script_tools.messages.last().is_some_and(|m| m.error));
    assert!(!scratch.join("hello.nss").exists());
}

/// The area view's To Scratch: the area as it is now (its .are, .git and
/// .gic) copied into the scratch folder.
#[test]
fn an_area_goes_to_the_scratch_folder() {
    let Some((mut h, area)) = area_harness("area-scratch") else { return };
    // No scratch folder yet, and none chosen when asked: nothing is copied.
    h.get_by_label("To Scratch").click();
    h.run_steps(2);
    assert_eq!(h.state().settings.scratch_dir, None);
    let scratch = mg_testkit::scratch_dir("ui-area-scratch").join("scratch");
    h.state_mut().settings.scratch_dir = Some(scratch.clone());
    h.run_steps(2);
    h.get_by_label("To Scratch").click();
    h.run_steps(3);
    let mut names: Vec<String> = std::fs::read_dir(&scratch)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["are", "gic", "git"].map(|e| format!("{area}.{e}")));
    let ws = h.state_mut().ws.as_mut().unwrap();
    let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().to_bytes().unwrap();
    assert_eq!(std::fs::read(scratch.join(format!("{area}.are"))).unwrap(), are);
}

/// The raw fields view names what a number stands for: an area's music,
/// ambient sounds and audio environment are shown by name (their rows of
/// ambientmusic.2da, ambientsound.2da and soundeax.2da), each in a list to
/// choose another by name.
#[test]
fn raw_fields_name_their_2da_rows() {
    let Some((mut h, area)) = area_harness("gff-rows") else { return };
    let git = ResKey::new(area, ResType::GIT);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Gff(git)));
    h.run_steps(3);
    // AreaProperties is the GIT's first struct.
    h.get_all_by_label_contains(" fields").next().expect("AreaProperties").click();
    h.run_steps(3);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let props = ws.doc(&git).unwrap().root.child("AreaProperties").unwrap().clone();
    let shown: Vec<String> = h
        .query_all_by_role(egui::accesskit::Role::ComboBox)
        .filter_map(|n| n.accesskit_node().value())
        .collect();
    let game = h.state().game.clone().unwrap();
    for field in
        ["AmbientSndDay", "AmbientSndNight", "EnvAudio", "MusicBattle", "MusicDay", "MusicNight"]
    {
        let table = mg_rules::rows::row_table("GIT ", &["AreaProperties"], field).unwrap();
        let name = game.row_name(&table, props.integer(field).unwrap()).expect(field);
        assert!(shown.contains(&name), "{field} is shown as {name:?}: {shown:?}");
    }
}

/// The Creature Wizard offers the racial types that have a name (not
/// racialtypes.2da's DELETED and INVALID_RACE rows), and a monster's race,
/// whose portraits are of no gender, has portraits to choose from.
#[test]
fn the_creature_wizard_offers_real_races_and_monsters_portraits() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-creature-wizard-races");
    let path = sample_module(&dir);
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    app.open_module(&path);
    app.open_palette = false;
    app.creature_wizard = Some(Default::default());
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let next = |h: &mut Harness<'_, Moonglow>| {
        h.get_by_label("Next >").click();
        h.run();
    };
    next(&mut h);
    assert!(h.query_by_label("DELETED").is_none() && h.query_by_label("INVALID_RACE").is_none());
    h.get_by_label("Outsider").click();
    h.run();
    next(&mut h);
    next(&mut h);
    // Appearance and Portrait: one is chosen, and Next goes on.
    let w = h.state().creature_wizard.as_ref().unwrap();
    assert_eq!(w.race, Some(20));
    assert!(w.portrait.is_some() && !w.no_portraits, "an outsider has portraits");
    next(&mut h);
    assert_eq!(h.state().creature_wizard.as_ref().unwrap().page, 4);
}

/// Options › General › Interface size: the whole interface larger, and
/// back to the usual size when set back.
#[test]
fn the_interface_size_is_set_in_options() {
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(Vec::new()));
    h.run();
    assert_eq!(h.ctx.zoom_factor(), 1.0);
    h.state_mut().settings.ui_scale = Some(150);
    h.run();
    assert_eq!(h.ctx.zoom_factor(), 1.5);
    h.state_mut().settings.ui_scale = None;
    h.run();
    assert_eq!(h.ctx.zoom_factor(), 1.0);
}

/// Escape closes a model's window while the pointer is over it.
#[test]
fn escape_closes_a_models_window() {
    let dir = mg_testkit::scratch_dir("ui-model-escape");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let model = Tab::Model(ResKey::parse("nothing", ResType::UTW).unwrap());
    app.actions.push(mg_ui::Action::OpenTab(model.clone()));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    assert!(h.state().dock.find_tab(&model).is_some());
    // Away from the window: Escape is someone else's.
    h.hover_at(egui::pos2(2.0, 2.0));
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().dock.find_tab(&model).is_some());
    // Over the window (where it says the blueprint isn't there).
    let inside = h.get_by_label_contains("not found").rect().center();
    h.hover_at(inside);
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().dock.find_tab(&model).is_none(), "closed");
}
