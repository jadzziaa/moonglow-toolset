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
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
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

/// Advanced Controls shows the Build Module window's options without
/// growing it to the screen's height: its buttons stay in reach.
#[test]
fn the_build_window_s_advanced_controls_keep_its_buttons_in_reach() {
    let dir = mg_testkit::scratch_dir("ui-build-advanced");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.build = Some(Default::default());
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 1000.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Advanced Controls").click();
    h.run();
    h.get_by_label("Creature CR");
    let done = h.get_by_label("Done").rect();
    assert!(done.bottom() < 800.0, "the buttons are in the window: {done:?}");
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

#[test]
fn multi_line_text_keeps_its_line_ends() {
    let dir = mg_testkit::scratch_dir("ui-crlf");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
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

/// The override folder is watched off the window's thread: a file new to
/// it is the game data's within a few seconds, without being asked for,
/// and nothing is read again while nothing changed.
#[test]
fn a_file_new_to_the_override_is_seen_by_the_watch() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-reload-watch");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let key = ResKey::parse("mg_watch_test", ResType::TWODA).unwrap();
    // (The watch begins; what it says at its start finds nothing changed.)
    app.reload_changed_content();
    std::thread::sleep(std::time::Duration::from_millis(300));
    app.reload_changed_content();
    assert!(!app.log.entries.iter().any(|(_, m)| m.starts_with("Reloaded")));
    std::fs::write(user.join("override/mg_watch_test.2da"), "2DA V2.0\n\n   Label\n0  x\n")
        .unwrap();
    let had = |app: &Moonglow| app.game.as_deref().unwrap().resman.contains(&key);
    for _ in 0..80 {
        if had(&app) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        app.reload_changed_content();
    }
    assert!(had(&app), "{:?}", app.log.entries);
    assert!(app.log.entries.iter().any(|(_, m)| m == "Reloaded override"));
}

/// Saving a module that was never saved asks where first, and then does
/// what a save does once: the build before saving is not run for the
/// question and again for the answer, and not at all if it is called off.
#[test]
fn a_first_save_builds_once_and_not_when_called_off() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-first-save");
    let path = dir.join("fresh.mod");
    let install = mg_resman::GameInstall::new(&root, None, "en");
    // The first Save As is called off, the second answered.
    let app = Moonglow::new(
        Some(install),
        Box::new(NoDialogs { save: vec![path.clone()], ..Default::default() }),
    );
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::NewModule("Fresh".into()));
    h.run();
    h.state_mut().wizard = None;
    h.state_mut().settings.build_on_save = true;
    h.run();
    let builds = |h: &Harness<'_, Moonglow>| {
        h.state().log.entries.iter().filter(|(_, m)| m == "Building Module...").count()
    };
    let dialogs = std::mem::replace(&mut h.state_mut().dialogs, Box::new(NoDialogs::default()));
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert_eq!(h.state().module_path(), None);
    assert_eq!(builds(&h), 0, "called off: nothing was done");
    h.state_mut().dialogs = dialogs;
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()));
    assert_eq!(builds(&h), 1, "{:?}", h.state().log.entries);
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
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
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
    assert_eq!(h.state().install.as_ref().map(|i| i.root.clone()), Some(dir));
    assert!(h.state().game.is_none());
    assert!(
        h.state().log.entries.iter().any(|(_, m)| m.starts_with("Could not load the game data"))
    );
}

/// Options › OK leaves the module open, its unsaved work with it, and asks
/// nothing: only another game or user folder has it read again, and then
/// it is opened again.
#[test]
fn options_leave_the_module_open() {
    let dir = mg_testkit::scratch_dir("ui-options-open");
    let path = sample_module(&dir);
    let other = dir.join("elsewhere");
    std::fs::create_dir_all(&other).unwrap();
    let dialogs = NoDialogs { folders: vec![other.clone()], ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Unsaved work: the module's tag changed.
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let edit = mg_edit::Edit::SetField {
        key: ifo,
        path: mg_edit::GffPath::root(),
        label: "Mod_Tag".into(),
        value: Some(mg_gff::Value::String(b"CHANGED".to_vec())),
    };
    let command = mg_edit::Command::new("Tag", vec![edit]);
    h.state_mut().actions.push(mg_ui::Action::Apply(command));
    h.run();
    assert!(h.state().has_unsaved_work());

    // A switch changed, OK: applied, the module and its change still there.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("General").click();
    h.run();
    h.get_by_label("Light theme").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().settings.light_theme);
    assert!(h.state().confirm_discard.is_none(), "nothing to ask");
    assert!(h.state().has_unsaved_work(), "the change is kept");
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tag"));

    // Another game folder: unsaved work is asked about first.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_all_by_label("Browse…").next().unwrap().click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().confirm_discard.is_some(), "asked before the module is read again");
    assert_eq!(h.state().settings.game_root, None, "not applied until answered");
    // Saved, the same options reopen the module where it was.
    let ask = h.state_mut().confirm_discard.take().unwrap();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    h.state_mut().actions.push(ask);
    h.run();
    assert_eq!(h.state().settings.game_root.as_deref(), Some(other.as_path()));
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()), "opened again");
    assert!(!h.state().has_unsaved_work());
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

/// Saving writes the custom palettes with their blueprints' names as the
/// game reads them: the bytes the names have in the module (an accent in
/// the game's codepage, a color token's bytes), not UTF-8, which the DM's
/// Creator in the game showed as "HipÃ³lito".
#[test]
fn a_saved_module_s_custom_palette_has_names_in_the_game_s_bytes() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-palette-bytes");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let name: &[u8] = b"Hip\xf3lito <c\xfe\x81\x01>rojo</c>";
    let key = ResKey::parse("mg_book", ResType::UTI).unwrap();
    {
        let game = app.game.as_deref().unwrap();
        let torch = game.resman.get_named("nw_it_torch001", ResType::UTI).unwrap();
        let mut g = Gff::read(&torch).unwrap();
        let named = mg_core::LocString::from_text(Language::ENGLISH, mg_core::Gender::Male, name);
        g.root.set("LocalizedName", mg_gff::Value::LocString(named));
        g.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
        let data = g.to_bytes().unwrap();
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "book",
            vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
        )));
    }
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    // The file on disk, read apart from the toolset's own palette reader.
    let saved = Module::open(&path).unwrap();
    let palette = saved.gff(&ResKey::parse("itempalcus", ResType::ITP).unwrap()).unwrap().unwrap();
    fn leaves(list: &[mg_gff::Struct], out: &mut Vec<(String, Vec<u8>)>) {
        for node in list {
            if let Some(r) = node.resref("RESREF") {
                out.push((r.to_string(), node.string("NAME").unwrap_or_default().to_vec()));
            }
            leaves(node.list("LIST").unwrap_or(&[]), out);
        }
    }
    let mut found = Vec::new();
    leaves(palette.root.list("MAIN").unwrap(), &mut found);
    let book = found.iter().find(|(r, _)| r == "mg_book").expect("the book in the palette");
    assert_eq!(book.1, name, "{:?}", String::from_utf8_lossy(&book.1));

    // A module saved by 1.19.3 has the names as UTF-8 in its palette:
    // opened and saved, with nothing changed, it has them right.
    let mut broken = Module::open(&path).unwrap();
    let palcus = ResKey::parse("itempalcus", ResType::ITP).unwrap();
    let mut palette = broken.gff(&palcus).unwrap().unwrap();
    fn spoil(list: &mut [mg_gff::Struct]) {
        for node in list {
            if node.resref("RESREF").is_some()
                && let Some(name) = node.string("NAME").map(<[u8]>::to_vec)
            {
                // (As 1.19.3 wrote them: the text, read as Windows-1252, in UTF-8.)
                let text = mg_core::Codepage::WINDOWS_1252.decode(&name).into_owned();
                node.set("NAME", mg_gff::Value::String(text.into_bytes()));
            }
            if let Some(mg_gff::Value::List(children)) = node.get_mut("LIST") {
                spoil(children);
            }
        }
    }
    if let Some(mg_gff::Value::List(main)) = palette.root.get_mut("MAIN") {
        spoil(main);
    }
    broken.set_gff(palcus, &palette).unwrap();
    broken.save().unwrap();
    let mut found = Vec::new();
    let spoiled = Module::open(&path).unwrap().gff(&palcus).unwrap().unwrap();
    leaves(spoiled.root.list("MAIN").unwrap(), &mut found);
    assert_ne!(found.iter().find(|(r, _)| r == "mg_book").unwrap().1, name, "spoiled");
    h.state_mut().open_module(&path);
    h.run();
    assert!(!h.state().ws.as_ref().unwrap().is_modified());
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let mut found = Vec::new();
    let repaired = Module::open(&path).unwrap().gff(&palcus).unwrap().unwrap();
    leaves(repaired.root.list("MAIN").unwrap(), &mut found);
    let book = found.iter().find(|(r, _)| r == "mg_book").expect("the book in the palette");
    assert_eq!(book.1, name, "opened and saved: {:?}", String::from_utf8_lossy(&book.1));
}

/// A module that is a folder is written as a `.mod` beside the folder too
/// when it is saved, as Aurora saves a module directory; not with
/// Options › General's switch off.
#[test]
fn a_folder_module_is_saved_as_a_mod_beside_it() {
    let dir = mg_testkit::scratch_dir("ui-folder-mod");
    let path = sample_module(&dir);
    let folder = dir.join("unpacked");
    let beside = dir.join("unpacked.mod");
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    {
        let ws = app.ws.as_mut().unwrap();
        ws.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    }
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    assert!(!h.state().settings.no_mod_beside_folder, "on unless switched off");
    h.state_mut().settings.no_mod_beside_folder = true;
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(folder.join("module.ifo").is_file() || folder.join("module.ifo.json").is_file());
    assert!(!beside.exists(), "not with the option off");
    h.state_mut().settings.no_mod_beside_folder = false;
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let packed = mg_module::Module::open(&beside).expect("the .mod beside the folder");
    let ours = h.state().ws.as_ref().unwrap();
    assert_eq!(packed.keys().count(), ours.module.keys().count());
    assert!(matches!(ours.module.location, Some(ModuleLocation::Folder(_))), "still the folder's");
    // Saved again: the one there is kept as its backup.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(dir.join("unpacked.BackupMod").is_file());
}

#[test]
fn browse_view_and_copy_a_game_resource() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-browser");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let dialogs = NoDialogs { folders: vec![out.clone()], ..Default::default() };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resources));
    h.run();
    // Those listed go out as loose files.
    h.state_mut().browser.filter = "nw_it_torch00".into();
    h.run();
    h.get_by_label_contains("as Files…").click();
    h.run();
    assert!(out.join("nw_it_torch001.uti").is_file());
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
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
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

/// The Talk Table editor makes and opens a `.tlk` file on its own, with no
/// module open.
#[test]
fn a_talk_table_file_is_made_and_opened_on_its_own() {
    let dir = mg_testkit::scratch_dir("ui-talk-file");
    let file = dir.join("strings.tlk");
    let dialogs =
        NoDialogs { open: vec![file.clone()], save: vec![file.clone()], ..Default::default() };
    let mut h = Harness::builder().with_size(egui::vec2(1000.0, 800.0)).build_ui_state(
        |ui, app: &mut Moonglow| app.ui(ui),
        Moonglow::new(None, Box::new(dialogs)),
    );
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run();
    h.get_by_label("New File…").click();
    h.run();
    assert!(file.is_file());
    h.get_by_label("Add Line").click();
    h.run();
    type_into_hint(&mut h, "the line's text", "On its own");
    h.get_by_label("Save").click();
    h.run();
    let read = mg_tlk::Tlk::read(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(read.text(mg_core::StrRef(0)).as_deref(), Some("On its own"));
    // Opened again from the file.
    h.state_mut().talk = None;
    h.state_mut().talk_view = Default::default();
    h.run();
    h.get_by_label("Open File…").click();
    h.run();
    h.get_by_label("16777216");
    assert_eq!(h.state().talk.as_ref().unwrap().line(0).text, "On its own");
    // Go to takes a StrRef or a line's number: that line is chosen and
    // the list goes to it. A long text scrolls in its box: the sound's row
    // under it stays in the window.
    let long = (1..=60).map(|n| format!("Line {n}")).collect::<Vec<_>>().join("\n");
    for row in 1..120 {
        let line = mg_module::talk::Line {
            text: if row == 100 { long.clone() } else { format!("Text {row}") },
            feminine: None,
            sound: String::new(),
            sound_length: 0.0,
        };
        h.state_mut().talk.as_mut().unwrap().add_line(&line).unwrap();
    }
    h.run();
    assert!(h.query_by_label("16777316").is_none(), "far down the list");
    type_into_hint(&mut h, "StrRef or line", "16777316");
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
    assert_eq!(h.state().talk_view.selected, Some(100));
    h.get_by_label("16777316");
    let sound = h.get_by_label("Sound").rect();
    assert!(
        sound.bottom() < 800.0 && sound.top() > 400.0,
        "the sound's row is in sight: {sound:?}"
    );
    // A line's number goes there too; past the end, it says what there is.
    h.state_mut().talk_view.go_to.clear();
    h.run();
    type_into_hint(&mut h, "StrRef or line", "3");
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().talk_view.selected, Some(3));
    h.state_mut().talk_view.go_to.clear();
    h.run();
    type_into_hint(&mut h, "StrRef or line", "5000");
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().talk_view.selected, Some(3), "nowhere to go");
    assert!(h.query_by_label_contains("The table has 120 lines").is_some());
}

/// A talk table the module names that is there but cannot be read is tried
/// once and said once, not every frame the editor is drawn.
#[test]
fn a_talk_table_that_cannot_be_read_is_tried_once() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-bad");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("tlk")).unwrap();
    std::fs::write(user.join("tlk/mg_bad.tlk"), b"not a talk table").unwrap();
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_bad".to_vec()));
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let install = mg_resman::GameInstall::new(&root, Some(user), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run_steps(8);
    let said: Vec<&String> = h
        .state()
        .log
        .entries
        .iter()
        .map(|(_, m)| m)
        .filter(|m| m.starts_with("Talk table mg_bad"))
        .collect();
    assert_eq!(said.len(), 1, "{said:?}");
}

#[test]
fn talk_table_made_edited_saved_and_used_by_strings() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-table");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    // (Export CSV and Import CSV are answered with the same file.)
    let csv = dir.join("lines.csv");
    let dialogs =
        NoDialogs { open: vec![csv.clone(); 2], save: vec![csv.clone(); 2], ..Default::default() };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
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
    // Ctrl+Z, with the pointer over the editor, is the table's too.
    let over = h.get_by_label("Add Line").rect().center();
    h.hover_at(over);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(line(&h).feminine.as_deref(), Some(""), "the table's undo");
    h.get_by_label("Redo").click();
    h.run();
    assert_eq!(line(&h).feminine.as_deref(), Some("Greetings, lady"));

    // The lines go out as CSV; changed in a spreadsheet, they come back.
    h.get_by_label("Export CSV…").click();
    h.run();
    let sheet = std::fs::read_to_string(&csv).unwrap();
    assert!(sheet.contains("16777216,Greetings,\"Greetings, lady\""), "{sheet}");
    std::fs::write(&csv, sheet.replace("16777216,Greetings,", "16777216,Well met,")).unwrap();
    h.get_by_label("Import CSV…").click();
    h.run();
    assert_eq!(line(&h).text, "Well met");
    h.get_by_label("Undo").click();
    h.run();
    assert_eq!(line(&h).text, "Greetings", "the import is one undo");

    // As JSON too (nwn_tlk's, as a nasher repository keeps a table): the
    // lines with text, by their numbers in the table.
    h.get_by_label("Export JSON…").click();
    h.run();
    let json = std::fs::read_to_string(&csv).unwrap();
    let read: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(read["entries"][0], serde_json::json!({"id": 0, "text": "Greetings"}));
    let other = r#"{"language":0,"entries":[{"id":0,"text":"Hail"},{"id":40,"text":"Far"}]}"#;
    std::fs::write(&csv, other).unwrap();
    h.get_by_label("Import JSON…").click();
    h.run();
    assert_eq!((line(&h).text.as_str(), h.state().talk.as_ref().unwrap().len()), ("Hail", 41));
    // Only Lines with Text leaves the empty ones between out of the list;
    // the selected line stays selected when they come back.
    h.get_by_label("16777220");
    h.get_by_label("Only lines with text").click();
    h.run();
    assert!(h.query_by_label("16777220").is_none(), "an empty line is not listed");
    h.get_by_label("16777256").click();
    h.run();
    h.get_by_label("Only lines with text").click();
    h.run();
    h.run();
    assert_eq!(h.state().talk_view.selected, Some(40));
    h.get_by_label("16777256");
    h.get_by_label("16777255");
    h.get_by_label("Undo").click();
    h.run();
    assert_eq!((line(&h).text.as_str(), h.state().talk.as_ref().unwrap().len()), ("Greetings", 1));

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

/// Module Properties › Custom Content: the module's haks in a list of
/// their own, numbered from the one that overrides the others. Rows are
/// chosen (Ctrl and Shift for more) and moved by the keys, the arrows
/// under the list or a drag, and removed, one undo each; a hak not in the
/// hak folders is flagged.
#[test]
fn the_hak_list_is_reordered_by_keys_buttons_and_a_drag() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-hak-list");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    for name in ["alpha", "bravo", "charlie"] {
        let empty = mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap();
        std::fs::write(user.join(format!("hak/{name}.hak")), empty).unwrap();
    }
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, Some(user), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    let set = |h: &mut Harness<'_, Moonglow>, names: &[&str]| {
        let items = names
            .iter()
            .map(|n| {
                let mut item = ifo::MOD_HAK_LIST.new_item();
                item.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from(*n));
                item
            })
            .collect();
        let edit = mg_edit::Edit::SetField {
            key: info,
            path: mg_edit::GffPath::root(),
            label: ifo::MOD_HAK_LIST.label.to_string(),
            value: Some(mg_gff::Value::List(items)),
        };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Haks", vec![edit])));
        h.run();
    };
    let haks = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let root = ws.doc(&info).unwrap().root.clone();
        root.items(&ifo::MOD_HAK_LIST)
            .iter()
            .map(|i| {
                String::from_utf8_lossy(i.read(&ifo::mod_hak_list::MOD_HAK).as_bytes()).into_owned()
            })
            .collect()
    };
    let chosen = |h: &Harness<'_, Moonglow>| -> Vec<usize> {
        h.state().hak_list.selected.iter().copied().collect()
    };
    h.get_by_label("Custom Content").click();
    h.run();
    set(&mut h, &["alpha", "bravo", "charlie", "gone"]);
    h.get_by_label("Hak Files (4)");
    h.get_by_label("Top overrides lower");
    assert_eq!(h.query_all_by_label("not found").count(), 1, "the one not in the hak folder");

    // A click chooses a row; Alt + Up moves it up, one undo back.
    h.get_by_label("bravo").click();
    h.run();
    assert_eq!(chosen(&h), [1]);
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowUp);
    h.run();
    assert_eq!(haks(&mut h), ["bravo", "alpha", "charlie", "gone"]);
    assert_eq!(chosen(&h), [0], "what moved stays chosen");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(haks(&mut h), ["alpha", "bravo", "charlie", "gone"]);

    // Down chooses the next; with Shift, more; the arrow under the list
    // moves what is chosen down together.
    h.get_by_label("alpha").click();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(chosen(&h), [1]);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    h.run();
    assert_eq!(chosen(&h), [1, 2]);
    h.get_by_label("⏷").click();
    h.run();
    assert_eq!(haks(&mut h), ["alpha", "gone", "bravo", "charlie"]);
    assert_eq!(chosen(&h), [2, 3]);
    // (At the end: no further.)
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowDown);
    h.run();
    assert_eq!(haks(&mut h), ["alpha", "gone", "bravo", "charlie"]);

    // A row dragged by its name to before another: it is among the two
    // chosen, which go together.
    let from = h.get_by_label("charlie").rect().center();
    let to = h.get_by_label("alpha").rect().center() - egui::vec2(0.0, 6.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let drag = |h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2| {
        h.hover_at(from);
        h.run_steps(1);
        h.event(button(from, true));
        for k in 1..=6 {
            h.hover_at(from + (to - from) * (k as f32 / 6.0));
            h.run_steps(1);
        }
        h.event(button(to, false));
        h.run();
    };
    drag(&mut h, from, to);
    assert_eq!(haks(&mut h), ["bravo", "charlie", "alpha", "gone"]);
    assert_eq!(chosen(&h), [0, 1]);
    // One not among those chosen is dragged alone: to the end.
    let from = h.get_by_label("alpha").rect().center();
    let to = h.get_by_label("gone").rect().center() + egui::vec2(0.0, 6.0);
    drag(&mut h, from, to);
    assert_eq!(haks(&mut h), ["bravo", "charlie", "gone", "alpha"]);
    assert_eq!(chosen(&h), [3]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(haks(&mut h), ["bravo", "charlie", "alpha", "gone"], "one undo for a drag");

    // Ctrl + click chooses another too; Delete removes them, one undo.
    h.get_by_label("alpha").click();
    h.run();
    h.get_by_label("gone").click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    assert_eq!(chosen(&h), [2, 3]);
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(haks(&mut h), ["bravo", "charlie"]);
    assert_eq!(h.query_all_by_label("not found").count(), 0);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(haks(&mut h), ["bravo", "charlie", "alpha", "gone"]);
    // Remove under the list does the same for the rows chosen.
    h.get_by_label("bravo").click();
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    assert_eq!(haks(&mut h), ["charlie", "alpha", "gone"]);
}

/// Files added to a hak that has some of them already are asked about
/// first, as Aurora's hak editor asks: replaced, left out, or nothing
/// added at all.
#[test]
fn files_a_hak_already_has_are_asked_about_before_they_are_replaced() {
    let dir = mg_testkit::scratch_dir("ui-hak-add");
    let table = |text: &str| format!("2DA V2.0\n\n   Label\n0  {text}\n");
    let (old, new) = (dir.join("old"), dir.join("new"));
    for d in [&old, &new] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(old.join("mg_one.2da"), table("old")).unwrap();
    std::fs::write(new.join("mg_one.2da"), table("new one")).unwrap();
    std::fs::write(new.join("mg_two.2da"), table("two")).unwrap();
    let chosen = vec![new.join("mg_one.2da"), new.join("mg_two.2da")];
    // (Add Files… is answered three times, the last first.)
    let dialogs = NoDialogs {
        open_many: vec![chosen.clone(), chosen.clone(), chosen, vec![old.join("mg_one.2da")]],
        ..Default::default()
    };
    let app = Moonglow::new(None, Box::new(dialogs));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    mg_ui::hak_view::new_hak(h.state_mut());
    h.run();
    let sizes = |h: &Harness<'_, Moonglow>| -> Vec<(String, u64)> {
        let mut items: Vec<(String, u64)> =
            h.state().haks[0].hak.items().iter().map(|i| (i.key.to_string(), i.size)).collect();
        items.sort();
        items
    };
    let (was, now) = (table("old").len() as u64, table("new one").len() as u64);
    // Nothing of the name there yet: added without a word.
    h.get_by_label("Add Files…").click();
    h.run();
    assert_eq!(sizes(&h), [("mg_one.2da".to_string(), was)]);
    assert!(h.query_by_label("Files Already in the Hak").is_none());
    // One of the two chosen is there: asked. Cancel adds nothing.
    h.get_by_label("Add Files…").click();
    h.run();
    h.get_by_label("Files Already in the Hak");
    assert!(h.query_by_label_contains("already has 1 of the 2 files").is_some());
    assert_eq!(sizes(&h).len(), 1, "nothing added before the answer");
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!(sizes(&h), [("mg_one.2da".to_string(), was)]);
    // Skip Those: the other is added, the hak's own stays.
    h.get_by_label("Add Files…").click();
    h.run();
    h.get_by_label("Skip Those").click();
    h.run();
    let two = table("two").len() as u64;
    assert_eq!(sizes(&h), [("mg_one.2da".to_string(), was), ("mg_two.2da".to_string(), two)]);
    // Replace: the chosen files take their place.
    h.get_by_label("Add Files…").click();
    h.run();
    assert!(h.query_by_label_contains("already has 2 of the 2 files").is_some());
    h.get_by_label("Replace 2").click();
    h.run();
    assert_eq!(sizes(&h), [("mg_one.2da".to_string(), now), ("mg_two.2da".to_string(), two)]);
}

/// The hak editor's Compile Models compiles the hak's models kept as
/// text, one step to undo; a model whose supermodel is nowhere stays as
/// text and is named in the log, and a compiled model shows as text.
#[test]
fn the_hak_editor_compiles_the_models_kept_as_text() {
    let dir = mg_testkit::scratch_dir("ui-hak-models");
    let base = "newmodel base\nsetsupermodel base NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom base\nnode dummy base\n  parent NULL\nendnode\n\
node dummy arm\n  parent base\nendnode\nendmodelgeom base\ndonemodel base\n";
    let orphan = base.replace("base", "orphan").replace("orphan NULL", "orphan mg_nowhere_zz");
    std::fs::write(dir.join("base.mdl"), base).unwrap();
    std::fs::write(dir.join("orphan.mdl"), &orphan).unwrap();
    let dialogs = NoDialogs {
        open_many: vec![vec![dir.join("base.mdl"), dir.join("orphan.mdl")]],
        ..Default::default()
    };
    let app = Moonglow::new(None, Box::new(dialogs));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    mg_ui::hak_view::new_hak(h.state_mut());
    h.run();
    h.get_by_label("Add Files…").click();
    h.run();
    let model = |h: &Harness<'_, Moonglow>, name: &str| {
        h.state().haks[0].hak.data(ResKey::parse(name, ResType::MDL).unwrap()).unwrap()
    };
    assert!(!mg_mdl::is_binary(&model(&h, "base")));
    h.get_by_label("Compile Models").click();
    h.run();
    assert!(mg_mdl::is_binary(&model(&h, "base")), "compiled");
    assert_eq!(model(&h, "orphan"), orphan.as_bytes(), "left as text");
    let said = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state().log.entries.iter().any(|(_, m)| m.contains(what))
    };
    assert!(
        said(&h, "1 compiled") && said(&h, "Left as text: orphan.mdl"),
        "{:?}",
        h.state().log.entries
    );
    // One step to undo, named for what it was.
    assert_eq!(h.state().haks[0].hak.undo_label(), Some("Compile Models"));
    h.state_mut().haks[0].hak.undo();
    assert!(!mg_mdl::is_binary(&model(&h, "base")));
    // As the window runs it: on a thread of its own (no module is open),
    // the hak changed when the work is handed back.
    h.state_mut().background_jobs = true;
    h.run();
    h.get_by_label("Compile Models").click();
    let started = std::time::Instant::now();
    while !mg_mdl::is_binary(&model(&h, "base")) {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "not compiled");
    }
    h.run_steps(2);
    assert!(!h.state().busy(), "the job is done");
    assert_eq!(model(&h, "orphan"), orphan.as_bytes());
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
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
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
    // Shift + click selects from the one clicked last to the one clicked.
    h.get_by_label("mg_ui_other.2da").click();
    h.run();
    h.get_by_label("mg_ui_test.2da").click_modifiers(egui::Modifiers::SHIFT);
    h.run();
    assert_eq!(h.state().haks[0].selected.len(), 2);
    h.get_by_label("mg_ui_test.2da").click();
    h.run();
    assert_eq!(h.state().haks[0].selected.len(), 1);
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

    // Opened again, the hak knows the folder it was built from: Update
    // from Folder has what the folder has now (the removed table back, a
    // new file with it).
    assert_eq!(h.state().haks[0].folder.as_deref(), Some(content.as_path()));
    std::fs::write(content.join("mg_ui_new.2da"), "2DA V2.0\n\n   Label\n0  a longer one\n")
        .unwrap();
    h.get_by_label("Update from Folder").click();
    h.run();
    let names = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        h.state().haks[0].hak.items().iter().map(|i| i.key.to_string()).collect()
    };
    let mut have = names(&h);
    have.sort();
    assert_eq!(have, ["mg_ui_new.2da", "mg_ui_other.2da", "mg_ui_test.2da"]);
    // By size, the largest is listed first.
    h.state_mut().haks[0].sort = mg_ui::hak_view::Sort::Size;
    h.run();
    let first = |h: &Harness<'_, Moonglow>| {
        let rows: Vec<_> = h.query_all_by_label_contains(".2da").collect();
        rows.iter()
            .map(|n| (n.rect().top(), n.accesskit_node().label()))
            .fold((f32::MAX, None), |best, row| if row.0 < best.0 { row } else { best })
    };
    assert_eq!(first(&h).1.as_deref(), Some("mg_ui_new.2da"));
    // View (or a double click) shows a resource under the list.
    assert!(h.query_by_label_contains("a longer one").is_none());
    h.get_by_label("mg_ui_new.2da").click_secondary();
    h.run();
    // (The row's View: the menu bar has a View too.)
    h.get_all_by_label("View").last().unwrap().click();
    h.run();
    assert!(h.query_by_label_contains("a longer one").is_some(), "its text is shown");
    h.get_by_label("Close").click();
    h.run();
    assert!(h.query_by_label_contains("a longer one").is_none());
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
    // (The conversation editor's: the module tree and the palettes have one too.)
    let at = h.get_all_by_label("Expand All").last().unwrap().rect().center();
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

/// A tileset group's name is typed like any other field: kept while it is
/// typed, set when focus leaves.
#[test]
fn a_tileset_group_s_name_is_typed() {
    let dir = mg_testkit::scratch_dir("ui-tileset-group");
    let set_path = dir.join("zzz02.set");
    let dialogs = NoDialogs { save: vec![set_path.clone()], ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.open_module(&sample_module(&dir));
    app.open_palette = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1300.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    mg_ui::tileset_view::new_tileset(h.state_mut());
    h.run();
    h.get_by_label("Terrains and Crossers").click();
    h.run();
    h.state_mut().tilesets[0].new_type = "Grass".into();
    h.run();
    h.get_by_label("Add Terrain").click();
    h.run();
    h.get_by_label("Tiles").click();
    h.run();
    h.get_by_label("Add Tile").click();
    h.run();
    h.get_by_label("Groups").click();
    h.run();
    h.get_by_label("Add Group").click();
    h.run();
    let group =
        |h: &Harness<'_, Moonglow>| h.state().tilesets[0].tileset().unwrap().groups[0].clone();
    let before = group(&h).name;
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(&before).find(is_input).expect("the group's name field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&before).find(is_input).expect("the group's name field").type_text("Bridge");
    h.run();
    h.run();
    assert_eq!(group(&h).name, before, "not before focus leaves");
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(group(&h).name, "Bridge");
    // What Check found is of the file as it was: an undo or a redo, like
    // any change, clears it.
    for step in [mg_ui::tileset_view::TilesetDoc::undo, mg_ui::tileset_view::TilesetDoc::redo] {
        h.state_mut().tilesets[0].findings = Some(Vec::new());
        step(&mut h.state_mut().tilesets[0]);
        assert!(h.state().tilesets[0].findings.is_none());
    }
    assert_eq!(group(&h).name, "Bridge");
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

    // Search finds the line and selects it in the tree. Ctrl+F, with the
    // pointer over the editor, opens the Search pane; F3 finds.
    assert!(h.query_by_label("Find What").is_none());
    let over = h.get_by_label("Test").rect().center();
    h.hover_at(over);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run();
    h.get_by_label("Find What");
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.find = "traveller".into();
    h.run();
    // (The Find What field has the keyboard: let it go, for F3.)
    h.key_press(egui::Key::Escape);
    h.run();
    h.hover_at(over);
    h.key_press(egui::Key::F3);
    h.run();
    let results = h.state().dialog_views[&key].search.results.clone();
    assert_eq!(results.len(), 1, "F3 finds");
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
    // View opens the game's own to look at: a change made there isn't kept
    // and the module doesn't get the blueprint.
    let stock = ResKey::parse("nw_wp_tavern", ResType::UTW).unwrap();
    // (The palette's View: the menu bar has a View too.)
    h.get_all_by_label("View").last().unwrap().click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Blueprint(stock)).is_some());
    assert!(h.query_by_label_contains("the game's blueprint").is_some());
    let set = mg_edit::Edit::SetField {
        key: stock,
        path: mg_edit::GffPath::root(),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(b"MINE".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![set])));
    h.run();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(!ws.module.contains(&stock) && !ws.is_modified(), "nothing of it is the module's");
    close_windows(&mut h);
    h.run();
    // Edit Copy asks for the copy's ResRef and Tag, offering a free one.
    h.get_by_label(&tavern).click_secondary();
    h.run();
    h.get_by_label("Edit Copy…").click();
    h.run();
    let draft = h.state().copy_as.clone().expect("the Copy window");
    assert_eq!(draft.resref, "nw_wp_tavern001");
    let tag = draft.tag.expect("a waypoint has a tag");
    h.state_mut().copy_as.as_mut().unwrap().tag = Some("MY_TAVERN".into());
    h.get_by_label("Create Copy").click();
    h.run();
    // A copy in the module, shown in the Custom palette.
    let copy = ResKey::parse("nw_wp_tavern001", ResType::UTW).unwrap();
    let gff = h.state_mut().ws.as_mut().unwrap().doc(&copy).unwrap().clone();
    assert_eq!(gff.root.resref("TemplateResRef").unwrap().to_string(), "nw_wp_tavern001");
    assert_eq!(gff.root.string("Tag"), Some(&b"MY_TAVERN"[..]), "not {tag}");
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

/// A creature's statistics have Aurora's arrows after their numbers: a
/// click is one more or one less, one change each, within the range.
#[test]
fn a_creature_s_statistics_have_arrows_to_step_by_one() {
    let Some((mut h, key)) = blueprint_harness("nw_chicken", "stat_arrows", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Statistics").click();
    h.run();
    let strength = |h: &mut Harness<'_, Moonglow>| field(h, &key).integer("Str").unwrap();
    let was = strength(&mut h);
    // (The first pair on the page is Strength's.)
    let up = h.query_all_by_label("⏶").next().expect("an arrow up").rect().center();
    let down = h.query_all_by_label("⏷").next().expect("an arrow down").rect().center();
    let click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
    };
    click(&mut h, up);
    click(&mut h, up);
    assert_eq!(strength(&mut h), was + 2);
    click(&mut h, down);
    assert_eq!(strength(&mut h), was + 1);
    // One change each.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(strength(&mut h), was + 2);
}

/// A waypoint's editor in a module whose hak has (or, `table` off, has
/// not) an `encoding.2da` with the wiki's Turkish letters: byte 0xF0 is
/// "ğ", 0xFD "ı", 0xFE "ş". The waypoint's tag is `Da\xf0`.
fn turkish_harness(name: &str, table: bool) -> Option<(Harness<'static, Moonglow>, ResKey)> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let dir = mg_testkit::scratch_dir(name);
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut rows = String::from("2DA V2.0\n\n     Codepoint\n");
    let own = mg_core::Codepage::WINDOWS_1252.chars().unwrap();
    for (b, c) in own.iter().enumerate() {
        let c = match b {
            0xF0 => 'ğ',
            0xFD => 'ı',
            0xFE => 'ş',
            _ => *c,
        };
        rows.push_str(&format!("{b} 0x{:x}\n", c as u32));
    }
    let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
    let key = ResKey::parse(if table { "encoding" } else { "mg_other" }, ResType::TWODA).unwrap();
    hak.add(key.resref, key.restype, rows.into_bytes()).unwrap();
    std::fs::write(user.join("hak/mg_turkish.hak"), hak.to_bytes().unwrap()).unwrap();

    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    let mut item = ifo::MOD_HAK_LIST.new_item();
    item.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from("mg_turkish"));
    info.root.items_mut(&ifo::MOD_HAK_LIST).push(item);
    m.set_info(&info).unwrap();
    m.save().unwrap();

    let install = mg_resman::GameInstall::new(&root, Some(user), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let game = app.game.as_deref().unwrap();
    assert_eq!(game.codepage().is_table(), table);
    let data = game.resman.get_named("nw_waypoint001", ResType::UTW).unwrap();
    let mut utw = Gff::read(&data).unwrap();
    utw.root.set("Tag", mg_gff::Value::String(b"Da\xf0".to_vec()));
    let key = ResKey::parse("mg_dag", ResType::UTW).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "copy",
        vec![mg_edit::Edit::SetResource { key, data: Some(utw.to_bytes().unwrap()) }],
    )));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    close_tab(&mut app, &Tab::ModuleProperties);
    app.open_palette = false;
    let h = Harness::builder()
        .with_size(egui::vec2(1000.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    Some((h, key))
}

/// A module whose hak has an `encoding.2da` (EE 1.87: the character each
/// byte of the game's text stands for) is read and written by it: the
/// bytes of the files are the game's, the letters on screen the table's.
#[test]
fn a_hak_s_encoding_table_reads_and_writes_the_module_s_text() {
    let Some((mut h, key)) = turkish_harness("ui-encoding-table", true) else { return };
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    // The tag's byte 0xF0 is the table's letter.
    h.get_all_by_value("Dağ").find(is_input).expect("the tag as the table reads it").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("Dağ").find(is_input).unwrap().type_text("Işık dağı é");
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    // Typed letters are the table's bytes; the rest are as ever.
    assert_eq!(field(&mut h, &key).string("Tag").unwrap(), b"I\xfe\xfdk da\xf0\xfd \xe9");
    // A name, too (a localized string), read back by the same table.
    let name = field(&mut h, &key).locstring("LocalizedName").unwrap().clone();
    let mut named = name;
    named.set(Language::ENGLISH, Gender::Male, b"Da\xf0 yolu".to_vec());
    let game = h.state().game.clone().unwrap();
    assert_eq!(game.locstring(&named).as_deref(), Some("Dağ yolu"));
    // A letter the table gave up ("ð", whose byte is "ğ" now) has none.
    assert!(game.codepage().encode("ð").is_none());
    drop(game);

    // The hak taken off the module: the game's own codepage again.
    let edit = mg_edit::Edit::SetField {
        key: ResKey::parse("module", ResType::IFO).unwrap(),
        path: mg_edit::GffPath::root(),
        label: ifo::MOD_HAK_LIST.label.to_string(),
        value: Some(mg_gff::Value::List(Vec::new())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Haks", vec![edit])));
    h.run();
    assert!(!h.state().game.as_deref().unwrap().codepage().is_table());
    assert!(h.query_all_by_value("Iþýk daðý é").any(|n| is_input(&n)));
}

/// Without the table all is as it was: Windows-1252.
#[test]
fn text_without_an_encoding_table_is_windows_1252() {
    let Some((mut h, key)) = turkish_harness("ui-encoding-none", false) else { return };
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value("Dað").find(is_input).expect("the tag in Windows-1252").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("Dað").find(is_input).unwrap().type_text("Épée ğ");
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    // (A letter Windows-1252 lacks is a "?", as before.)
    assert_eq!(field(&mut h, &key).string("Tag").unwrap(), b"\xc9p\xe9e ?");
}

/// Export Files, on a placeable's picture: the files the game draws it
/// with, written into a folder (a chest of the game's own: all of them).
#[test]
fn a_placeable_s_model_files_are_exported() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_files", ResType::UTP) else {
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-export-model-files");
    h.state_mut().dialogs =
        Box::new(NoDialogs { folders: vec![dir.clone()], ..Default::default() });
    h.run();
    // (The button of the model's picture: there is none without a GPU.)
    let source = mg_ui::model_view::Source::Resource(key);
    mg_ui::model_view::export_files(h.state_mut(), &source);
    let mut names: Vec<String> = (std::fs::read_dir(&dir).unwrap().flatten())
        .map(|e| e.file_name().into_string().unwrap())
        .collect();
    names.sort();
    let has = |ext: &str| names.iter().any(|n| n.ends_with(ext));
    assert!(has(".mdl") && has(".pwk"), "{names:?}");
    assert!(has(".dds") || has(".tga"), "{names:?}");
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    let said = log.iter().find(|m| m.starts_with("Exported ")).expect("said in the log");
    assert!(said.contains(&format!("{} files", names.len())) && said.contains(".mdl ("), "{said}");
    // (Every file is the game's own here: none left out.)
    assert!(!said.contains("left out"), "{said}");

    // Again into the same folder: it asks first. One file changed and one
    // gone: Skip Those writes the one gone and keeps the other.
    let model = names.iter().find(|n| n.ends_with(".mdl")).unwrap();
    let walk = names.iter().find(|n| n.ends_with(".pwk")).unwrap();
    std::fs::write(dir.join(model), b"mine").unwrap();
    std::fs::remove_file(dir.join(walk)).unwrap();
    h.state_mut().dialogs =
        Box::new(NoDialogs { folders: vec![dir.clone()], ..Default::default() });
    mg_ui::model_view::export_files(h.state_mut(), &source);
    h.run();
    assert_eq!(h.state().model_export.as_ref().unwrap().existing.len(), names.len() - 1);
    assert_eq!(std::fs::read(dir.join(model)).unwrap(), b"mine", "nothing written yet");
    h.get_by_label("Skip Those").click();
    h.run();
    assert!(h.state().model_export.is_none());
    assert_eq!(std::fs::read(dir.join(model)).unwrap(), b"mine");
    assert!(dir.join(walk).is_file());
    // Replace writes them all; Cancel none.
    for (button, replaced) in [("Cancel", false), ("Replace", true)] {
        let again = NoDialogs { folders: vec![dir.clone()], ..Default::default() };
        h.state_mut().dialogs = Box::new(again);
        mg_ui::model_view::export_files(h.state_mut(), &source);
        h.run();
        h.get_by_label_contains(button).click();
        h.run();
        assert!(h.state().model_export.is_none());
        assert_eq!(std::fs::read(dir.join(model)).unwrap() != b"mine", replaced, "{button}");
    }
}

/// GitHub issue 11: with the last tab of the middle closed, the palettes'
/// pane took the whole window. The middle keeps its place (a pane that
/// says no area is open), and the palettes' pane its width.
#[test]
fn the_middle_keeps_its_place_with_no_area_open() {
    let Some(mut h) = game_harness("no-area") else { return };
    h.state_mut().settings.no_last_area = true;
    h.run();
    let width = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let mut leaves = h.state().dock.iter_leaves().map(|(_, l)| l);
        leaves.find(|l| l.tabs.contains(tab)).map(|l| l.rect.width())
    };
    let has = |h: &Harness<'_, Moonglow>, tab: &Tab| h.state().dock.find_tab(tab).is_some();
    // (The module opened on its first area.)
    let area = Tab::Area(ResRef::from_str("start").unwrap());
    assert!(has(&h, &Tab::Palette) && has(&h, &area) && !has(&h, &Tab::NoArea));
    let palette = width(&h, &Tab::Palette).unwrap();

    // The middle's tabs closed (Module Properties was docked there as
    // the module opened): its place is kept.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(area.clone()));
    h.run();
    assert!(!has(&h, &Tab::NoArea), "Module Properties is in the middle still");
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    h.run();
    assert!(has(&h, &Tab::NoArea));
    h.get_by_label("No area open");
    let now = width(&h, &Tab::Palette).unwrap();
    assert!((now - palette).abs() < 3.0, "the palettes' pane: {palette} wide, then {now}");
    assert!(width(&h, &Tab::NoArea).unwrap() > 300.0);

    // An area opened takes its place (the pane lists those opened lately
    // and the module's, each a link, beside the module tree's row); the
    // palettes' pane stays.
    assert_eq!(h.query_all_by_label("start").count(), 3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(area.clone()));
    h.run();
    h.run();
    assert!(has(&h, &area) && !has(&h, &Tab::NoArea));
    let now = width(&h, &Tab::Palette).unwrap();
    assert!((now - palette).abs() < 3.0, "the palettes' pane: {palette} wide, then {now}");
    // And closed again, the same.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(area));
    h.run();
    h.run();
    assert!(has(&h, &Tab::NoArea));
    let now = width(&h, &Tab::Palette).unwrap();
    assert!((now - palette).abs() < 3.0, "the palettes' pane: {palette} wide, then {now}");
    // The area just opened is offered first, among those opened lately.
    h.get_by_label("Opened lately");
    assert_eq!(h.query_all_by_label("start").count(), 3);
}

/// Without the palettes (no game), the middle is not left blank either.
#[test]
fn the_middle_says_no_area_is_open_without_the_palettes() {
    let dir = mg_testkit::scratch_dir("ui-no-area-plain");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    assert!(h.state().dock.find_tab(&Tab::NoArea).is_none());
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    h.run();
    h.run();
    h.get_by_label("No area open");
    // Closing the module brings the start page back, not this.
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::NoArea).is_none());
}

/// GitHub issue 15: the tabs by the keyboard, as in a browser. Ctrl+W
/// closes the one in front, Ctrl+Tab and Ctrl+Shift+Tab (and Ctrl+Page
/// Down and Up) go round them, Ctrl and a digit goes to one by its number
/// (9: the last), Ctrl+Shift+T opens the one closed last again.
#[test]
fn tabs_by_the_keyboard() {
    let dir = mg_testkit::scratch_dir("ui-tab-keys");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    for name in ["second", "third"] {
        let mut a = ifo::MOD_AREA_LIST.new_item();
        a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str(name).unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
        m.set(ResKey::parse(name, ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
    }
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let area = |name: &str| Tab::Area(ResRef::from_str(name).unwrap());
    for name in ["start", "second", "third"] {
        app.actions.push(mg_ui::Action::OpenTab(area(name)));
    }
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.run();
    // The middle's tabs, and the one in front.
    let tabs = |h: &Harness<'_, Moonglow>| -> (Vec<Tab>, Tab) {
        let leaf = (h.state().dock.iter_leaves())
            .find(|(p, l)| p.surface.is_main() && l.tabs.iter().any(|t| matches!(t, Tab::Area(_))))
            .map(|(_, l)| l)
            .expect("the areas' pane");
        (leaf.tabs.clone(), leaf.tabs[leaf.active.0].clone())
    };
    let front = |h: &Harness<'_, Moonglow>| tabs(h).1;
    let key = |h: &mut Harness<'_, Moonglow>, modifiers: egui::Modifiers, key: egui::Key| {
        h.key_press_modifiers(modifiers, key);
        h.run();
        h.run();
    };
    let (ctrl, shift) = (egui::Modifiers::COMMAND, egui::Modifiers::SHIFT);
    // (Module Properties is docked there first, as the module opened.)
    let all = tabs(&h).0;
    assert_eq!(all[1..], [area("start"), area("second"), area("third")]);
    assert_eq!(front(&h), area("third"));

    // Round them: on from the last is the first.
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front(&h), Tab::ModuleProperties);
    key(&mut h, ctrl, egui::Key::PageDown);
    assert_eq!(front(&h), area("start"));
    key(&mut h, ctrl | shift, egui::Key::Tab);
    key(&mut h, ctrl, egui::Key::PageUp);
    assert_eq!(front(&h), area("third"));
    // By number: the third tab, and the last.
    key(&mut h, ctrl, egui::Key::Num3);
    assert_eq!(front(&h), area("second"));
    key(&mut h, ctrl, egui::Key::Num9);
    assert_eq!(front(&h), area("third"));
    key(&mut h, ctrl, egui::Key::Num2);
    assert_eq!(front(&h), area("start"));

    // Ctrl+W closes the one in front; Ctrl+Shift+T opens it again.
    key(&mut h, ctrl, egui::Key::W);
    assert!(!tabs(&h).0.contains(&area("start")));
    key(&mut h, ctrl | shift, egui::Key::T);
    assert_eq!(front(&h), area("start"));
    // Close Others (a tab's menu), and each of them opened again, the one
    // closed last first.
    let others: Vec<Tab> = tabs(&h).0.into_iter().filter(|t| *t != area("start")).collect();
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(others));
    h.run();
    assert_eq!(tabs(&h).0, [area("start")]);
    key(&mut h, ctrl | shift, egui::Key::T);
    assert_eq!(front(&h), area("third"));
    key(&mut h, ctrl | shift, egui::Key::T);
    key(&mut h, ctrl | shift, egui::Key::T);
    // (Module Properties comes back in a window of its own, as it opens.)
    assert_eq!(tabs(&h).0.len(), 3);
    assert!(h.state().dock.find_tab(&Tab::ModuleProperties).is_some());
    // Nothing more to open: the key does nothing.
    key(&mut h, ctrl | shift, egui::Key::T);
    assert_eq!(h.state().dock.iter_all_tabs().count(), 4);
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
    // End is the last row; Escape hands the keys back.
    key(&mut h, egui::Key::End);
    assert!(matches!(at(&h), Some(TreeAt::Group(_))));
    key(&mut h, egui::Key::Escape);
    assert_eq!(at(&h), None);
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(at(&h), None);
}

/// GitHub issue 10: the panes beside the middle fold away and come back
/// (View, Ctrl+Alt+1, 2, 3 and 0, and the strip a folded pane leaves at
/// its edge), each as wide as it was.
#[test]
fn panels_fold_away_and_come_back() {
    let Some(mut h) = game_harness("panels") else { return };
    h.run();
    h.run();
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press_modifiers(keys, key);
        h.run();
        h.run();
    };
    let palette = |h: &Harness<'_, Moonglow>| {
        let mut leaves = h.state().dock.iter_leaves().map(|(_, l)| l);
        leaves.find(|l| l.tabs.contains(&Tab::Palette)).map(|l| l.rect.width())
    };
    let tree = |h: &Harness<'_, Moonglow>| h.query_by_label("Filter").is_some();
    let was = palette(&h).expect("the palettes' pane");
    assert!(tree(&h));

    // The module tree: folded to a strip, and back by the strip.
    key(&mut h, egui::Key::Num1);
    assert!(!tree(&h) && h.state().settings.hide_tree);
    h.get_by_label("⏵").click();
    h.run();
    assert!(tree(&h) && !h.state().settings.hide_tree);
    // The palettes: their pane goes, and comes back as wide.
    key(&mut h, egui::Key::Num2);
    assert!(palette(&h).is_none() && h.state().settings.hide_palettes);
    h.get_by_label("⏴").click();
    h.run();
    h.run();
    let now = palette(&h).expect("back");
    assert!((now - was).abs() < 3.0, "{was} wide, then {now}");
    // The log.
    key(&mut h, egui::Key::Num3);
    assert!(h.state().settings.hide_log);
    h.get_by_label("⏶ Log").click();
    h.run();
    assert!(!h.state().settings.hide_log);
    // All of them, and all back.
    key(&mut h, egui::Key::Num0);
    let s = &h.state().settings;
    assert!(s.hide_tree && s.hide_log && s.hide_palettes && palette(&h).is_none());
    key(&mut h, egui::Key::Num0);
    let s = &h.state().settings;
    assert!(!s.hide_tree && !s.hide_log && !s.hide_palettes);
    let now = palette(&h).expect("back");
    assert!((now - was).abs() < 3.0, "{was} wide, then {now}");
    // The View menu has them, ticked.
    h.get_by_label("View").click();
    h.run();
    h.get_by_label_contains("✔ Module Tree");
    h.get_by_label("Reset Layout").click();
    h.run();
    h.run();
    assert!(palette(&h).is_some());
}

/// GitHub issue 14: a tool window opens where one of its kind was last
/// dragged to (a tab of the main pane, here), not always in a window of
/// its own; View › Reset Layout forgets that.
#[test]
fn a_tool_window_opens_where_its_kind_was_last_docked() {
    let dir = mg_testkit::scratch_dir("ui-window-places");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let open = |h: &mut Harness<'_, Moonglow>, tab: Tab| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run();
        h.run();
    };
    let close = |h: &mut Harness<'_, Moonglow>, tab: Tab| {
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab]));
        h.run();
        h.run();
    };
    let in_main = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        h.state().dock.find_tab(tab).map(|p| p.surface.is_main())
    };
    // At first: a window of its own, over the area.
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(false));
    // Dragged into the main pane (done here on the dock itself).
    let dock = &mut h.state_mut().dock;
    let from = dock.find_tab(&Tab::Factions).unwrap();
    dock.remove_tab(from);
    dock.main_surface_mut().push_to_first_leaf(Tab::Factions);
    h.run();
    h.run();
    assert_eq!(h.state().settings.window_places, [("factions".to_string(), "middle".to_string())]);
    // Closed and opened again: there, and another kind still in a window.
    close(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), None);
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(true));
    open(&mut h, Tab::Journal);
    assert_eq!(in_main(&h, &Tab::Journal), Some(false));
    // Opened again while open, it stays in the main pane, in front.
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(true));
    // Reset Layout: a window of its own again.
    close(&mut h, Tab::Factions);
    h.get_by_label("View").click();
    h.run();
    h.get_by_label("Reset Layout").click();
    h.run();
    assert!(h.state().settings.window_places.is_empty());
    open(&mut h, Tab::Factions);
    assert_eq!(in_main(&h, &Tab::Factions), Some(false));
}

/// GitHub issue 9: the menus by the keyboard. F10 opens the first (and
/// closes them), Left and Right go from menu to menu, Up and Down from row
/// to row past those that can't be chosen, Enter chooses, Right opens a
/// submenu and Left closes it; and with a menu open, the pointer over
/// another's name opens that one.
#[test]
fn menus_by_the_keyboard() {
    let dir = mg_testkit::scratch_dir("ui-menu-keys");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run();
        h.run();
        h.run();
    };
    let shown = |h: &Harness<'_, Moonglow>, label: &str| {
        h.query_all_by_label_contains(label).next().is_some()
    };
    // F10: File, open; F10 again: closed.
    assert!(!shown(&h, "Save As"));
    key(&mut h, egui::Key::F10);
    assert!(shown(&h, "Save As"));
    key(&mut h, egui::Key::F10);
    assert!(!shown(&h, "Save As"));
    // Right twice from File: View (round the ends the other way: Help).
    key(&mut h, egui::Key::F10);
    key(&mut h, egui::Key::ArrowLeft);
    assert!(shown(&h, "About Moonglow") && !shown(&h, "Save As"));
    key(&mut h, egui::Key::ArrowRight);
    key(&mut h, egui::Key::ArrowRight);
    key(&mut h, egui::Key::ArrowRight);
    assert!(shown(&h, "Hide All Panels"));
    // Its first row is marked; Down passes the palettes (no game here:
    // they can't be chosen) to the log, and Enter folds it away.
    key(&mut h, egui::Key::ArrowDown);
    assert!(!h.state().settings.hide_log);
    key(&mut h, egui::Key::Enter);
    assert!(h.state().settings.hide_log && !h.state().settings.hide_tree);
    assert!(!shown(&h, "Hide All Panels"), "chosen, the menus close");
    // Up from the first row is the last: Reset Layout brings the log back.
    key(&mut h, egui::Key::F10);
    key(&mut h, egui::Key::ArrowRight);
    key(&mut h, egui::Key::ArrowRight);
    key(&mut h, egui::Key::ArrowUp);
    key(&mut h, egui::Key::Space);
    assert!(!h.state().settings.hide_log);

    // A submenu: Tools, up to Options and three more to Tilesets; Right
    // opens it, Left closes it, and the menu stays.
    key(&mut h, egui::Key::F10);
    for _ in 0..4 {
        key(&mut h, egui::Key::ArrowRight);
    }
    assert!(shown(&h, "Faction Editor") && !shown(&h, "Open Tileset"));
    for _ in 0..4 {
        key(&mut h, egui::Key::ArrowUp);
    }
    key(&mut h, egui::Key::ArrowRight);
    assert!(shown(&h, "Open Tileset"), "the submenu is open");
    key(&mut h, egui::Key::ArrowLeft);
    key(&mut h, egui::Key::ArrowLeft);
    assert!(shown(&h, "Rotate Area") || shown(&h, "Area Wizard"), "Left again: the menu before");
    key(&mut h, egui::Key::Escape);

    // Alt and a menu's letter opens it; a letter then goes to the next
    // row that begins with it (L: the Log), which Enter chooses.
    let alt = |h: &mut Harness<'_, Moonglow>, k: egui::Key| {
        h.key_press_modifiers(egui::Modifiers::ALT, k);
        h.run();
        h.run();
        h.run();
    };
    alt(&mut h, egui::Key::V);
    assert!(shown(&h, "Hide All Panels") && !shown(&h, "Save As"));
    key(&mut h, egui::Key::L);
    key(&mut h, egui::Key::Enter);
    assert!(h.state().settings.hide_log, "the row the letter went to");
    alt(&mut h, egui::Key::V);
    key(&mut h, egui::Key::R);
    key(&mut h, egui::Key::Enter);
    assert!(!h.state().settings.hide_log, "Reset Layout");
    // Alt and another menu's letter, with one open: that one.
    alt(&mut h, egui::Key::V);
    alt(&mut h, egui::Key::H);
    assert!(shown(&h, "About Moonglow") && !shown(&h, "Hide All Panels"));
    key(&mut h, egui::Key::Escape);
    assert!(!shown(&h, "About Moonglow"));
    // Escape in a submenu closes the submenu, and the menu stays.
    alt(&mut h, egui::Key::T);
    for _ in 0..4 {
        key(&mut h, egui::Key::ArrowUp);
    }
    key(&mut h, egui::Key::ArrowRight);
    assert!(shown(&h, "Open Tileset"));
    key(&mut h, egui::Key::Escape);
    assert!(!shown(&h, "Open Tileset"), "the submenu closed");
    assert!(shown(&h, "Faction Editor"), "one level");
    key(&mut h, egui::Key::Escape);
    assert!(!shown(&h, "Faction Editor"));

    // By the mouse: with File open, the pointer over Edit opens Edit.
    h.get_by_label("File").click();
    h.run();
    assert!(shown(&h, "Save As"));
    h.get_by_label("Edit").hover();
    h.run();
    h.run();
    h.run();
    assert!(shown(&h, "Module Properties") && !shown(&h, "Save As"));
}

/// GitHub issue 16: the palette's header. The types are one row of icons
/// (a dropdown in a pane too narrow for it), the one shown named under
/// it; the search says what it looks in and has a button to clear it; the
/// list's toolbar counts its blueprints and keeps the rarer commands
/// under "more".
#[test]
fn the_palette_s_header() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-palette-header");
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1800.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    // The row of icons: a click on the door shows the doors, named.
    // (All of it inside the palettes' pane, at the width it has at first.)
    let pane = {
        let mut leaves = h.state().dock.iter_leaves().map(|(_, l)| l);
        leaves.find(|l| l.tabs.contains(&Tab::Palette)).expect("the palettes' pane").rect
    };
    let last = h.get_by_label("🗐").rect();
    assert!(last.right() <= pane.right() && last.left() > pane.left(), "{last:?} in {pane:?}");
    h.get_by_label("🚪").click();
    h.run_steps(2);
    assert_eq!(h.state().palette.kind, mg_module::palette::BlueprintKind::Door);
    h.get_by_label("Doors");
    // How many the list has; a search that finds none, and cleared.
    let count = |h: &Harness<'_, Moonglow>| -> usize {
        let counts = h.query_all_by_label_contains(" item").filter_map(|n| {
            let node = n.accesskit_node();
            let text = node.value().or(node.label()).unwrap_or_default();
            text.split(' ').next()?.parse().ok()
        });
        { counts }.next().expect("the count")
    };
    let all = count(&h);
    assert!(all > 10, "{all} doors");
    assert!(h.query_by_label("×").is_none());
    type_into_hint(&mut h, "Find", "zzzzqq");
    h.run_steps(2);
    assert!(count(&h) < 5);
    h.get_by_label("×").click();
    h.run_steps(2);
    assert!(h.state().palette.filter.is_empty());
    assert_eq!(count(&h), all);
    // The rarer commands are under "more".
    assert!(h.query_by_label("Categories…").is_none());
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…");
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

/// The palette's types' icons, as its dropdown shows the one chosen.
const PALETTE_TYPES: [&str; 11] = ["🗻", "👤", "🚪", "⚔", "🗡", "⛲", "🔉", "💰", "⚡", "📍", "🗐"];

/// Chooses a type in the palette's header: its icon in the row of them,
/// or, in a pane too narrow for the row, its name in the dropdown.
fn palette_type(h: &mut Harness<'_, Moonglow>, glyph: &str, name: &str) {
    if h.query_by_label(glyph).is_some() {
        h.get_by_label(glyph).click();
        return;
    }
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let is_type = |n: &egui_kittest::Node<'_>| {
        let value = n.accesskit_node().value().unwrap_or_default();
        PALETTE_TYPES.iter().any(|glyph| value.starts_with(glyph))
    };
    h.query_all(combo).find(is_type).expect("the palette's type dropdown").click();
    h.run();
    // (The last of them are below the dropdown's edge until scrolled to.)
    h.get_by_label(&format!("{glyph} {name}")).scroll_to_me();
    h.run();
    h.get_by_label(&format!("{glyph} {name}")).click();
    h.run();
}

/// Types into the text field with placeholder `hint` (a live filter).
fn type_into_hint(h: &mut Harness<'_, Moonglow>, hint: &str, text: &str) {
    // ("Find" is a list's search: the palette's says what it looks in.)
    let palette = "🔍 Search name, tag, resref";
    let has = |h: &Harness<'_, Moonglow>, hint: &str| {
        let by = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some(hint);
        h.query_all(egui_kittest::kittest::by().predicate(by)).next().is_some()
    };
    let hint = if hint == "Find" && !has(h, hint) { palette } else { hint };
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some(hint);
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    h.run();
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text(text);
    h.run();
}

/// A dropdown with a Filter field stays open while the field is clicked
/// and typed in, and closes when a choice is made.
#[test]
fn a_dropdown_s_filter_field_can_be_typed_in() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_filter", ResType::UTP) else {
        return;
    };
    h.run();
    let before = field(&mut h, &key).integer("Appearance").unwrap();
    // The Appearance Type dropdown: it shows "Chest".
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let appearance = |h: &Harness<'_, Moonglow>| {
        h.query_all(combo.clone())
            .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("Chest")))
            .map(|n| n.rect().center())
            .expect("the Appearance Type dropdown")
    };
    let at = appearance(&h);
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run();
    // Its Filter field, clicked and typed in: the list narrows, still open.
    type_into_hint(&mut h, "Filter", "Armoire");
    let found = h.query_all_by_label_contains("Armoire").count();
    assert!(found > 0, "the dropdown is still open, and filtered");
    assert!(h.query_by_label("Altar").is_none(), "what doesn't match is left out");
    // A choice closes it and sets the appearance.
    h.get_all_by_label_contains("Armoire").next().unwrap().click();
    h.run();
    assert_ne!(field(&mut h, &key).integer("Appearance"), Some(before));
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
    assert!(h.query(egui_kittest::kittest::by().predicate(hint)).is_none(), "closed");
}

/// Hovering a skill shows what the game says of it (Aurora's F1).
#[test]
fn a_skill_s_description_shows_on_hover() {
    let Some((mut h, _)) = blueprint_harness("nw_bartender", "skill_help", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Skills").click();
    h.run();
    assert!(h.query_by_label_contains("opposed test").is_none());
    let hide = h.get_by_label("Hide").rect().center();
    h.hover_at(hide);
    // (A tooltip waits a moment.)
    h.run_steps(60);
    assert!(
        h.query_by_label_contains("Spot check").is_some(),
        "the game's description of Hide is shown"
    );
}

/// Palette › Categories…: a category of the module's own, which its
/// blueprints can then be given; one with blueprints in it isn't removed.
#[test]
fn a_module_gets_palette_categories_of_its_own() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_cat", ResType::UTP) else {
        return;
    };
    h.run();
    let skeleton = BlueprintKind::Placeable.skeleton_key();
    let own = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&skeleton).ok().map(mg_module::palette::Palette::read)
    };
    assert!(own(&mut h).is_none(), "the game's categories, to begin with");
    {
        let app = h.state_mut();
        // (The chest's editor, a window, would lie over the palette.)
        close_tab(app, &Tab::Blueprint(key));
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
        app.palette.kind = BlueprintKind::Placeable;
        app.palette.tiles = false;
        app.palette.custom = true;
    }
    h.run();
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…").click();
    h.run();
    h.get_by_label("The game's categories. A change gives the module categories of its own.");
    // Nothing to keep yet.
    type_into_hint(&mut h, "a category's or group's name", "Ruins");
    h.get_by_label("Add Category").click();
    h.run();
    let ruins = h.state().palette_categories.as_ref().unwrap().selected.clone().expect("chosen");
    // With a row chosen and the pointer resting on it, the wheel still
    // scrolls the list.
    let top_of = |h: &Harness<'_, Moonglow>, label: &str| {
        h.query_all_by_label_contains("  (")
            .find(|n| n.accesskit_node().label().is_some_and(|l| l == label))
            .map(|n| n.rect().top())
    };
    let (third, third_rect) = {
        let mut rows: Vec<(String, egui::Rect)> = h
            .query_all_by_label_contains("  (")
            .filter_map(|n| Some((n.accesskit_node().label()?, n.rect())))
            .collect();
        rows.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        rows[2].clone()
    };
    let chosen = h.state().palette_categories.as_ref().unwrap().selected.clone();
    h.hover_at(third_rect.center());
    h.event(egui::Event::PointerButton {
        pos: third_rect.center(),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: third_rect.center(),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    // (Long enough for a tooltip, had the row one.)
    h.run_steps(90);
    assert_ne!(h.state().palette_categories.as_ref().unwrap().selected, chosen, "chosen");
    let was = top_of(&h, &third).unwrap();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -120.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(30);
    let now = top_of(&h, &third).unwrap();
    assert!(now < was - 40.0, "the list scrolled: {was} to {now}");
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 400.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(30);
    // (The category added last is chosen again, for what follows.)
    h.state_mut().palette_categories.as_mut().unwrap().selected = chosen;
    h.run();

    // A row dragged onto the upper half of the first goes before it.
    let rows = |h: &Harness<'_, Moonglow>| -> Vec<(String, egui::Rect)> {
        let mut rows: Vec<(String, egui::Rect)> = h
            .query_all_by_label_contains("  (")
            .filter_map(|n| Some((n.accesskit_node().label()?, n.rect())))
            .collect();
        rows.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        rows
    };
    let before = rows(&h);
    // (The second of the list: the new category is at its end, out of
    // sight until the list is scrolled.)
    let (moved, dragged) = before[1].clone();
    let first = before[0].1;
    let button = egui::PointerButton::Primary;
    let modifiers = egui::Modifiers::NONE;
    let (from, to) = (dragged.center(), first.center() - egui::vec2(0.0, first.height() * 0.3));
    h.hover_at(from);
    h.run();
    h.event(egui::Event::PointerButton { pos: from, button, pressed: true, modifiers });
    h.run_steps(2);
    // (Held near the list's edge, a dragged row keeps it scrolling: frames
    // are counted, not waited out.)
    for k in 1..=4 {
        h.hover_at(from + (to - from) * (k as f32 / 4.0));
        h.run_steps(2);
    }
    h.event(egui::Event::PointerButton { pos: to, button, pressed: false, modifiers });
    h.run_steps(3);

    // Dragged to the list's lower edge and held there, a row scrolls the
    // list: what was out of sight below comes up.
    let seen = |h: &Harness<'_, Moonglow>| top_of(h, &third).unwrap();
    let (top, grab) = {
        let r = rows(&h);
        (seen(&h), r[1].1.center())
    };
    let edge = egui::pos2(grab.x, grab.y + 400.0);
    h.hover_at(grab);
    h.run_steps(2);
    h.event(egui::Event::PointerButton { pos: grab, button, pressed: true, modifiers });
    h.run_steps(2);
    for k in 1..=4 {
        h.hover_at(grab + (edge - grab) * (k as f32 / 4.0));
        h.run_steps(2);
    }
    h.run_steps(20);
    assert!(seen(&h) < top - 60.0, "the list scrolled as the row was held at its edge");
    // Still held, away from the edges, the wheel scrolls the list too.
    let middle = egui::pos2(grab.x, grab.y + 150.0);
    h.hover_at(middle);
    h.run_steps(3);
    let scrolled = seen(&h);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 200.0),
        phase: egui::TouchPhase::Move,
        modifiers,
    });
    h.run_steps(20);
    assert!(seen(&h) > scrolled + 40.0, "the wheel scrolls the list while a row is held");
    // (Let go where nothing takes it: the order stays.)
    h.key_press(egui::Key::Escape);
    h.event(egui::Event::PointerButton { pos: middle, button, pressed: false, modifiers });
    h.run_steps(3);
    let after = rows(&h);
    assert_eq!(after[0].0, moved, "moved to the top");
    assert_eq!(after[1].0, before[0].0);
    let first_category = moved.split("  (").next().unwrap().to_string();
    h.get_by_label("OK").click();
    h.run();
    let palette = own(&mut h).expect("the module has its own skeleton now");
    let game = h.state().game.clone().unwrap();
    let categories = palette.categories(&game);
    let id = categories.iter().find(|(_, name)| name == "Ruins").expect("the new category").0;
    assert_eq!(categories[0].1, first_category, "the Category lists follow the order given");
    assert!(categories.iter().any(|(_, name)| name.ends_with("Custom 1")), "the game's stay");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Palette categories"));

    // The chest, given the category: listed under it in the custom palette.
    let edit = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: "PaletteID".into(),
        value: Some(mg_gff::Value::Byte(id)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Category", vec![edit])));
    h.run();
    assert!(h.query_by_label("Ruins (1)").is_some(), "the custom palette has the category");

    // With a blueprint in it, it isn't removed.
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…").click();
    h.run();
    h.get_by_label("This module's own categories, kept in the module.");
    {
        let w = h.state_mut().palette_categories.as_mut().unwrap();
        w.selected = Some(ruins);
    }
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    let w = h.state().palette_categories.as_ref().unwrap();
    assert!(
        w.error.as_deref().is_some_and(|e| e.starts_with("1 of the module's blueprint is in it"))
    );
    h.get_by_label("Cancel").click();
    h.run();
    assert!(own(&mut h).unwrap().ids().contains(&id));
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

    // Another item, to tell the rows apart: a potion.
    let potion = {
        let game = h.state().game.as_deref().unwrap();
        let key = ResKey::parse("nw_it_mpotion001", ResType::UTI).unwrap();
        let uti = Gff::read(&game.resman.get(&key).unwrap()).unwrap();
        game.locstring(uti.root.locstring("LocalizedName").unwrap()).unwrap()
    };
    let typed =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value("nw_it_torch001").find(typed).unwrap().click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("nw_it_torch001").find(typed).unwrap().type_text("nw_it_mpotion001");
    h.run();
    // (The palette's row is the leftmost of that name; the list's rows are
    // right of it, top to bottom.)
    let named = |h: &Harness<'_, Moonglow>, name: &str| -> Vec<egui::Rect> {
        let mut rects: Vec<egui::Rect> = h
            .get_all_by_label(name)
            .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
            .map(|n| n.rect())
            .collect();
        rects.sort_by(|a, b| a.left().total_cmp(&b.left()).then(a.top().total_cmp(&b.top())));
        rects
    };
    let at = named(&h, &potion)[0].center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run();
    h.get_by_label("Add Item").click();
    h.run();
    let held = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let items = field(h, &key).list("ItemList").unwrap().to_vec();
        items.iter().map(|i| i.resref("InventoryRes").unwrap().to_string()).collect()
    };
    assert_eq!(held(&mut h), ["nw_it_torch001", "nw_it_torch001", "nw_it_mpotion001"]);
    // Copy, from its row's menu; Paste, from the list's heading: another.
    let row = *named(&h, &potion).last().unwrap();
    h.hover_at(row.center());
    h.event(egui::Event::PointerButton {
        pos: row.center(),
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: row.center(),
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    h.get_by_label("Copy").click();
    h.run();
    assert_eq!(h.state().item_clip.len(), 1);
    h.get_by_label("Paste (1)").click();
    h.run();
    assert_eq!(held(&mut h).len(), 4);
    assert_eq!(held(&mut h)[3], "nw_it_mpotion001", "pasted");
    // The last row dragged onto the first: it takes that place (one step).
    let last = *named(&h, &potion).last().unwrap();
    let first =
        named(&h, "Torch").into_iter().find(|r| r.left() > 400.0).expect("the list's torch");
    let (from, to) = (last.center(), first.center());
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.run_steps(1);
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let order = held(&mut h);
    assert_eq!(order, ["nw_it_mpotion001", "nw_it_torch001", "nw_it_torch001", "nw_it_mpotion001"]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(held(&mut h)[0], "nw_it_torch001", "one undo puts it back");
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
    // (Its checkbox, the first of that name; one it has is listed beside
    // the feats too, under Assigned.)
    assert_eq!(h.query_all_by_label("Alertness").count(), 1 + usize::from(had));
    h.get_all_by_label("Alertness").next().unwrap().click();
    h.run();
    assert_eq!(has_alertness(&mut h), !had);
    assert_eq!(h.query_all_by_label("Alertness").count(), 1 + usize::from(!had));
    h.get_by_label("Assigned");
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
    // Ready as added (the game reads the flags as yes or no: the one
    // switch); switched off, its uses are spent.
    let flags = |h: &mut Harness<'_, Moonglow>| {
        let list = field(h, &key).list("SpecAbilityList").unwrap().to_vec();
        list.last().unwrap().integer("SpellFlags")
    };
    assert_eq!(flags(&mut h), Some(1));
    assert!(h.query_by_label("Unlimited").is_none() && h.query_by_label("Spontaneous").is_none());
    let ready = egui_kittest::kittest::by().role(egui::accesskit::Role::CheckBox);
    h.query_all(ready.clone()).last().unwrap().click();
    h.run();
    assert_eq!(flags(&mut h), Some(0));
    h.query_all(ready).last().unwrap().click();
    h.run();
    assert_eq!(flags(&mut h), Some(1));
    // Armor, chosen in the palette, equipped in the armor slot (the second).
    h.get_by_label("Inventory").click();
    h.run();
    type_into_hint(&mut h, "Find", "nw_aarcl001");
    h.get_by_label("Leather Armor").click();
    h.run();
    // (The Equip under the palette comes first, then each slot's.)
    h.get_all_by_label("Equip").nth(2).unwrap().click();
    h.run();
    let worn = |h: &mut Harness<'_, Moonglow>, slot: u32| {
        let equipped = field(h, &key).list("Equip_ItemList").unwrap().to_vec();
        equipped.iter().find(|s| s.id == slot).map(|s| s.resref("EquippedRes").unwrap().to_string())
    };
    let packed = |h: &mut Harness<'_, Moonglow>| {
        let items = field(h, &key).list("ItemList").unwrap_or(&[]).to_vec();
        items.iter().map(|s| s.resref("InventoryRes").unwrap().to_string()).collect::<Vec<_>>()
    };
    assert_eq!(worn(&mut h, 2).as_deref(), Some("nw_aarcl001"));
    // To Backpack, on the equipped item selected: unequipped and packed, in
    // one step.
    let had = packed(&mut h).len();
    h.get_all_by_label("Leather Armor").last().unwrap().click();
    h.run();
    h.get_by_label("To Backpack").click();
    h.run();
    assert_eq!(worn(&mut h, 2), None);
    assert_eq!(packed(&mut h).last().map(String::as_str), Some("nw_aarcl001"));
    assert_eq!(packed(&mut h).len(), had + 1);
    // The Equip under the palette puts the chosen item in the first free
    // slot it goes in: the armor's, again.
    h.get_all_by_label("Equip").next().unwrap().click();
    h.run();
    assert_eq!(worn(&mut h, 2).as_deref(), Some("nw_aarcl001"), "in the armor slot");
    // Taken, a second can't be equipped: said in the log, nothing changed.
    let said = h.state().log.entries.len();
    h.get_all_by_label("Equip").next().unwrap().click();
    h.run();
    assert!(h.state().log.entries[said..].iter().any(|e| e.1.contains("is taken")));

    // Dragged and dropped. Where things are: the palette's row is the
    // topmost "Leather Armor" left of the equipment, the armor slot's name the rightmost
    // "Armor", the backpack's rows those under its heading.
    let drag = |h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2| {
        h.hover_at(from);
        h.run_steps(1);
        press(h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for k in 1..=5 {
            h.hover_at(from + (to - from) * (k as f32 / 5.0));
            h.run_steps(1);
        }
        press(h, to, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    let places = |h: &Harness<'_, Moonglow>| {
        let rects = |label: &str| -> Vec<egui::Rect> {
            h.get_all_by_label(label).map(|n| n.rect()).collect()
        };
        let slot = rects("Armor").into_iter().max_by(|a, b| a.left().total_cmp(&b.left())).unwrap();
        let heading = h.get_by_label("Backpack").rect();
        let armors = rects("Leather Armor");
        // (The palette's row: left of the equipment, above what it says of
        // the item chosen.)
        let palette = armors
            .iter()
            .copied()
            .filter(|r| r.right() < slot.left())
            .min_by(|a, b| a.top().total_cmp(&b.top()))
            .unwrap();
        // (Its name, right of its icon.)
        let worn = armors
            .iter()
            .copied()
            .filter(|r| r.left() > slot.left() && (r.center().y - slot.center().y).abs() < 12.0)
            .max_by(|a, b| a.left().total_cmp(&b.left()));
        // (Their names, wider than their icons.)
        let packed: Vec<egui::Rect> = armors
            .iter()
            .copied()
            .filter(|r| r.top() > heading.bottom() && r.width() > 80.0)
            .collect();
        (slot, heading, palette, worn, packed)
    };
    // (A window tall enough to show the equipment and the backpack under it.)
    h.set_size(egui::vec2(1000.0, 1600.0));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(Tab::Blueprint(key)));
    h.run_steps(10);
    // The equipped armor, onto the backpack: off, and packed.
    let had = packed(&mut h).len();
    let (_, heading, _, on, _) = places(&h);
    let backpack = heading.left_bottom() + egui::vec2(80.0, 40.0);
    drag(&mut h, on.expect("the armor worn").center(), backpack);
    assert_eq!(worn(&mut h, 2), None, "dragged off");
    assert_eq!(packed(&mut h).len(), had + 1);
    // A backpack row, onto the armor slot: worn again, out of the backpack.
    let (slot, _, _, _, rows) = places(&h);
    let row = rows.iter().copied().max_by(|a, b| a.width().total_cmp(&b.width())).unwrap();
    drag(&mut h, row.center(), slot.center());
    assert_eq!(worn(&mut h, 2).as_deref(), Some("nw_aarcl001"), "dropped on its slot");
    assert_eq!(packed(&mut h).len(), had);
    // The palette's row, onto the backpack: one more in it.
    let (_, heading, palette, _, _) = places(&h);
    drag(&mut h, palette.center(), heading.left_bottom() + egui::vec2(80.0, 40.0));
    assert_eq!(packed(&mut h).len(), had + 1, "added from the palette");
    // And onto a slot it doesn't go in (the helmet's): refused, in the log.
    let (_, _, palette, _, _) = places(&h);
    let helmet = h
        .get_all_by_label("Helmet")
        .map(|n| n.rect())
        .max_by(|a, b| a.left().total_cmp(&b.left()))
        .unwrap();
    let said = h.state().log.entries.len();
    drag(&mut h, palette.center(), helmet.center());
    assert_eq!(worn(&mut h, 1), None);
    assert!(h.state().log.entries[said..].iter().any(|e| e.1.contains("does not go in that slot")));
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

    // A second of the same name, with a ResRef and Tag of the builder's
    // own; the ResRef the module has already is refused.
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
    h.state_mut().blueprint_wizard.as_mut().unwrap().resref = "wizardsword".into();
    h.run();
    assert!(h.query_by_label_contains("has a wizardsword.uti already").is_some());
    let w = h.state_mut().blueprint_wizard.as_mut().unwrap();
    (w.resref, w.tag) = ("it_sword_ws2".into(), "IT_WS_TWO".into());
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    // (The categories stay open from the first time.)
    for branch in ["Weapons", "Bladed"] {
        if h.query_by_label("Bastard Swords").is_none() {
            h.get_by_label(branch).click();
            h.run();
        }
    }
    h.get_by_label("Bastard Swords").click();
    h.run();
    h.get_by_label("Finish").click();
    h.run();
    let second = module_gff(&mut h, "it_sword_ws2", ResType::UTI).expect("the second item");
    assert_eq!(second.root.string("Tag"), Some(&b"IT_WS_TWO"[..]));
    assert_eq!(
        second.root.resref("TemplateResRef").map(|r| r.to_string()).as_deref(),
        Some("it_sword_ws2")
    );
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
    // (A look test on a builder's content: a user folder with it in its
    // override, and an area with room for its groups.)
    let user = std::env::var_os("MG_TEST_USER").map(std::path::PathBuf::from);
    let side = std::env::var("MG_TEST_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
    let install = mg_resman::GameInstall::new(&root, user, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Area View", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str(tileset).unwrap(),
        width: side,
        height: side,
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

    // Alt + right drag turns them: one command, orientations only. With
    // Shift (which shows their handles) a right drag turns the view, as
    // without it.
    let before = git_list(&mut h, "WaypointList");
    let (from, to) = (a, a + egui::vec2(-80.0, 0.0));
    let right_drag = |h: &mut Harness<'_, Moonglow>, modifiers: egui::Modifiers| {
        h.event(egui::Event::ModifiersChanged(modifiers));
        h.hover_at(from);
        let right = |pressed, pos| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers,
        };
        h.event(right(true, from));
        for k in 1..=4 {
            h.hover_at(from + (to - from) * (k as f32 / 4.0));
            h.run_steps(1);
        }
        h.event(right(false, to));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };
    let yaw = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().yaw;
    let (undo, was) = (h.state().ws.as_ref().unwrap().can_undo().map(str::to_string), yaw(&h));
    right_drag(&mut h, egui::Modifiers::SHIFT);
    assert!((yaw(&h) - was).abs() > 0.1, "Shift + right drag turns the view");
    let now = h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    assert_eq!(now, undo, "and nothing else");
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw = was;
    h.run_steps(2);
    right_drag(&mut h, egui::Modifiers::ALT);
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
    // (Where that is now: the toolbar's lines wrap as its readout changes.)
    let over = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
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
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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
    h.get_by_label_contains("no tile fits the tiles shown in red");
    // The tiles in the way are shown in red for a moment.
    let refused = h.state().area_views[&area].refused.clone().expect("the tiles that refused");
    assert!(!refused.0.is_empty());

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

/// Tile Properties of one tile shows the tiles that fit there as
/// pictures; a click puts that one there, in a step of its own.
#[test]
fn tile_properties_choose_a_variant_by_picture() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-variants") else { return };
    let tile = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[i].clone()
    };
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(2);
    h.get_by_label("Tile Properties…").click();
    h.run_steps(2);
    // Its variants: the tiles that fit there, the one there now among
    // them; another chosen is put there at once, the window staying.
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!(props.tiles, [5]);
    let (cell, variants) = props.variants.clone().expect("one tile: its variants");
    assert_eq!(cell, (1, 1));
    let now = props.variant.unwrap();
    assert!(variants.len() > 1 && variants.iter().any(|(p, _)| *p == now), "{variants:?}");
    h.render().unwrap().save(mg_testkit::scratch_dir("ui-tile-variants").join("a.png")).unwrap();
    let other = variants.iter().find(|(p, _)| p.tile != now.tile).unwrap();
    h.get_by_label_contains(&format!("Variant ({} fit here)", variants.len()));
    let before_undo = h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    // (A picture has no name of its own: the one over its model's name.)
    let turn = ["", " ↺90°", " ↺180°", " ↺270°"][usize::from(other.0.orientation % 4)];
    let picture = h
        .query_all_by_label(&format!("{}{turn}", other.1))
        .next()
        .map(|n| n.rect().center_top() - egui::vec2(0.0, 40.0));
    if let Some(at) = picture {
        h.hover_at(at);
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        h.run_steps(4);
        // (Which picture that was is not told apart here: one of them.)
        let id = tile(&mut h, 5).integer("Tile_ID").unwrap();
        assert!(variants.iter().any(|(p, _)| i64::from(p.tile) == id), "tile {id}");
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tile variant"));
        let after = h.state().tile_props.clone().expect("the window stays");
        assert_eq!(after.variant.map(|p| i64::from(p.tile)), Some(id));
        assert_ne!(after.variant, Some(now), "another than the one that was there");
        assert!(!after.chosen, "taken up: the window is the new tile's");
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.state_mut().tile_props = Some(props);
        h.run_steps(2);
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo().map(str::to_string), before_undo);
    } else {
        panic!("no variant pictures");
    }
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
    // At the south and west edges: two columns come at the west, a row
    // goes at the south, and the waypoints keep their places against the
    // north and east edges: (20, 20) to (40, 10).
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Resize Area…").click();
    h.run_steps(2);
    let d = h.state_mut().resize_area.as_mut().expect("the window is open");
    (d.columns, d.rows, d.edges) = (6, 3, 2);
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(size(&mut h), (6, 3, 18));
    let w = waypoints(&mut h);
    assert!((w[0].0 - 40.0).abs() < 1e-3 && (w[0].1 - 10.0).abs() < 1e-3, "{w:?}");
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(1);
    assert_eq!(size(&mut h), (4, 4, 16));
    let w = waypoints(&mut h);
    assert!((w[0].0 - 20.0).abs() < 1e-3 && (w[0].1 - 20.0).abs() < 1e-3, "{w:?}");
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
    // The log says which were updated: the one ticked, by its area, kind,
    // tag and blueprint; not the one left out.
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    assert!(log.iter().any(|l| l == "Updated 1 object(s) from 2 blueprint(s):"), "{log:?}");
    let listed: Vec<&String> = log.iter().filter(|l| l.starts_with("  ")).collect();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert!(listed[0].contains("trigger") && listed[0].contains("(mg_trig_a)"), "{listed:?}");
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
    h.state_mut().dialogs = Box::new(mg_ui::NoDialogs { open: vec![path], ..Default::default() });
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
    h.state_mut().dialogs = Box::new(mg_ui::NoDialogs { open: vec![path], ..Default::default() });
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
    h.state_mut().dialogs = Box::new(NoDialogs { open: vec![csv], ..Default::default() });
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
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("[OWNER] - Hello there").click();
    h.run();
    // (The speaker's: not the palette's dropdown of types.)
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let speaker = |n: &egui_kittest::Node<'_>| {
        let value = n.accesskit_node().value().unwrap_or_default();
        !PALETTE_TYPES.iter().any(|glyph| value.starts_with(glyph))
    };
    h.query_all(combo).find(speaker).expect("the speaker's dropdown").click();
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
    // The Primary Weapon slot's Equip (the sixth: the palette's Equip is first): No leaves it as it was.
    h.get_all_by_label("Equip").nth(5).unwrap().click();
    h.run();
    assert!(h.query_by_label_contains("Weapon Proficiency (martial)").is_some());
    h.get_by_label("No").click();
    h.run();
    assert_eq!(primary(&mut h), before);
    assert!(feats(&mut h).is_empty());
    // Yes adds the first feat and equips it, one undoable step.
    h.get_all_by_label("Equip").nth(5).unwrap().click();
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
    // The same from the line's right-click menu, which selects it.
    click(&mut h, "[OWNER] - Second");
    h.get_by_label("Hi [END DIALOGUE]").click_secondary();
    h.run();
    h.get_all_by_label("Paste As Link").last().unwrap().click();
    h.run();
    h.run();
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
    // (An area's sounds come up over its first three seconds.)
    h.run_steps(240);
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
        let text = a.label().or_else(|| a.value()).unwrap();
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
        .filter_map(|n| n.accesskit_node().label())
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

/// A palette's row is one line however narrow the pane: a name too long
/// for it is cut short, not wrapped onto a second line (which made the
/// list hard to read), and the pointer over it shows the whole.
#[test]
fn a_palette_row_too_long_for_the_pane_stays_one_line() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, _)) = area_harness("palette-one-line") else { return };
    h.state_mut().palette.kind = BlueprintKind::Waypoint;
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.custom = false;
    // (ResRefs beside the names: longer than the pane is wide.)
    h.state_mut().settings.name_resrefs = true;
    h.run_steps(3);
    h.get_by_label_contains("(13)").click();
    h.run_steps(4);
    let rows: Vec<egui::Rect> =
        h.query_all_by_label_contains("Beholder AI Exit").map(|n| n.rect()).collect();
    assert_eq!(rows.len(), 2, "both waypoints of the name are listed");
    let line = h.get_by_label_contains("Waypoints (13)").rect().height();
    for r in &rows {
        assert!(r.height() <= line + 2.0, "one line: {r:?} against {line}");
    }
    // One under the other, a line apart.
    assert!((rows[1].top() - rows[0].top()) < line * 1.5, "{rows:?}");
}

/// The arrow keys move through the palette's tree after a click in it, as
/// in Aurora: Up and Down through its rows, Left to the category a
/// blueprint is in and then closing it, Right opening it and going in.
/// They do not move the camera meanwhile, until the pointer is back in
/// the area's view.
#[test]
fn the_arrow_keys_move_through_the_palette_after_a_click_in_it() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, area)) = area_harness("palette-arrows") else { return };
    h.state_mut().palette.kind = BlueprintKind::Waypoint;
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.custom = false;
    h.run_steps(3);
    let target = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().target;
    let selected = |h: &Harness<'_, Moonglow>| h.state().palette.selected;
    let press = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(4);
    };
    // The pointer in the area's view: the arrows are its camera's.
    let centre = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(centre);
    h.run_steps(2);
    // The standard waypoints are one category: clicked, it opens, and the
    // arrow keys are the palette's.
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    assert!(h.query_by_label(&tavern).is_none());
    h.get_by_label_contains("(13)").click();
    h.run_steps(4);
    h.get_by_label(&tavern);
    assert_eq!(selected(&h), None);
    let start = target(&h);
    // Down: into the category, a blueprint at a time, each selected.
    press(&mut h, egui::Key::ArrowDown);
    let first = selected(&h).expect("the first waypoint");
    press(&mut h, egui::Key::ArrowDown);
    let second = selected(&h).expect("the second");
    assert_ne!(first, second);
    press(&mut h, egui::Key::ArrowUp);
    assert_eq!(selected(&h), Some(first));
    // Enter opens the blueprint, as a double click does.
    let tabs: Vec<Tab> = h.state().dock.iter_all_tabs().map(|(_, t)| t.clone()).collect();
    press(&mut h, egui::Key::Enter);
    let opened: Vec<Tab> = h
        .state()
        .dock
        .iter_all_tabs()
        .map(|(_, t)| t.clone())
        .filter(|t| !tabs.contains(t))
        .collect();
    assert_eq!(opened.len(), 1, "{opened:?}");
    let at = h.state().dock.find_tab(&opened[0]).unwrap();
    h.state_mut().dock.remove_tab(at);
    h.run_steps(3);
    // Right does nothing on a blueprint. Left: to its category (the
    // blueprint stays in hand), then the category closes.
    press(&mut h, egui::Key::ArrowRight);
    assert_eq!(selected(&h), Some(first));
    press(&mut h, egui::Key::ArrowLeft);
    assert!(h.state().palette.at_category());
    assert_eq!(selected(&h), Some(first));
    h.get_by_label(&tavern);
    press(&mut h, egui::Key::ArrowLeft);
    assert!(h.query_by_label(&tavern).is_none(), "closed");
    // Enter on a category opens and closes it.
    press(&mut h, egui::Key::Enter);
    h.get_by_label(&tavern);
    press(&mut h, egui::Key::Enter);
    assert!(h.query_by_label(&tavern).is_none(), "closed by Enter");
    // Right: it opens, then the cursor goes in.
    press(&mut h, egui::Key::ArrowRight);
    h.get_by_label(&tavern);
    h.state_mut().palette.selected = None;
    press(&mut h, egui::Key::ArrowRight);
    assert_eq!(selected(&h), Some(first));
    // A blueprint clicked, then Left (as a builder described Aurora's): to
    // its category.
    h.get_by_label(&tavern).click();
    h.run_steps(3);
    let tavern_key = ResKey::parse("nw_wp_tavern", ResType::UTW);
    assert_eq!(selected(&h), tavern_key);
    assert!(!h.state().palette.at_category());
    press(&mut h, egui::Key::ArrowLeft);
    assert!(h.state().palette.at_category());
    assert_eq!(selected(&h), tavern_key, "still in hand");
    // With the pointer elsewhere (the log, under the area), the keys
    // leave the palette alone.
    let palette_at = h.get_by_label(&tavern).rect().center();
    h.hover_at(egui::pos2(300.0, 700.0));
    h.run_steps(2);
    press(&mut h, egui::Key::ArrowDown);
    assert!(h.state().palette.at_category(), "the pointer is not over the palette");
    h.hover_at(palette_at);
    h.run_steps(2);
    // None of that moved the camera, nor does an arrow held down.
    let hold = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        h.event(event(true));
        h.run_steps(4);
        h.event(event(false));
        h.run_steps(1);
    };
    hold(&mut h, egui::Key::ArrowUp);
    assert_eq!(target(&h), start, "the arrows are the palette's");
    // W still does (the pointer was in this area's view last).
    hold(&mut h, egui::Key::W);
    let moved = target(&h);
    assert!((moved - start).length() > 0.1, "W moves the view all along");
    // The pointer back in the area's view: the arrows are the camera's.
    h.hover_at(centre);
    h.run_steps(2);
    let was = selected(&h);
    hold(&mut h, egui::Key::ArrowUp);
    assert!((target(&h) - moved).length() > 0.1, "the arrow moves the view again");
    assert_eq!(selected(&h), was, "and the palette stays");
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

/// An object held with the left button stays in hand while the middle
/// button swings the camera, and after that button is let go.
#[test]
fn a_held_object_stays_in_hand_through_a_camera_swing() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("held-swing") else { return };
    let orbit = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap();
    let (x0, y0, _) = waypoint(&mut h, area, 0).unwrap();
    let none = egui::Modifiers::NONE;
    let button = |h: &Harness<'_, Moonglow>, pos, button, pressed| {
        h.event(egui::Event::PointerButton { pos, button, pressed, modifiers: none });
    };
    let (left, middle) = (egui::PointerButton::Primary, egui::PointerButton::Middle);
    let mut at = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    button(&h, at, left, true);
    h.run_steps(1);
    for _ in 0..4 {
        at += egui::vec2(6.0, 0.0);
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
    }
    // The middle button too: a hard swing, to and fro.
    let before = orbit(&h);
    button(&h, at, middle, true);
    h.run_steps(1);
    for step in [egui::vec2(90.0, -40.0), egui::vec2(120.0, -60.0), egui::vec2(-150.0, 70.0)] {
        at += step;
        h.event(egui::Event::PointerMoved(at));
        h.run_steps(1);
    }
    assert!((orbit(&h).yaw - before.yaw).abs() > 0.3, "the camera swung");
    button(&h, at, middle, false);
    h.run_steps(2);
    assert_eq!(waypoint(&mut h, area, 0).map(|w| (w.0, w.1)), Some((x0, y0)), "still held");
    // Still in hand: it goes on following the pointer, and is put down
    // under it when the left button is let go.
    let swung = orbit(&h);
    at += egui::vec2(-30.0, 20.0);
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    assert_eq!(orbit(&h).yaw, swung.yaw, "the camera is let go");
    button(&h, at, left, false);
    h.run_steps(3);
    let (x, y, _) = waypoint(&mut h, area, 0).unwrap();
    let view = &h.state().area_views[&area];
    let model = view.model.as_ref().unwrap();
    let z = model.objects.iter().find(|o| o.index == 0).unwrap().position.z;
    let shown = view.screen_pos(Vec3::new(x, y, z)).expect("in view");
    assert!((shown - at).length() < 3.0, "put down at {shown:?}, the pointer at {at:?}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(waypoint(&mut h, area, 0).map(|w| (w.0, w.1)), Some((x0, y0)), "one move");
}

/// The ring around a selected object: taken anywhere on it (or just off
/// it) and led round, it turns the object with the pointer, as one Rotate.
#[test]
fn the_turning_ring_spins_the_selection() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("turn-ring") else { return };
    let facing = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let w = &git.root.list("WaypointList").unwrap()[0];
        w.float("YOrientation").unwrap().atan2(w.float("XOrientation").unwrap())
    };
    let none = egui::Modifiers::NONE;
    let on = screen(&h, area, Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(on);
    press(&h, on, true, none);
    press(&h, on, false, none);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 0)]);

    // A place on the ring, away from where the waypoint faces.
    let before = facing(&mut h);
    let view = &h.state().area_views[&area];
    let pivot =
        view.model.as_ref().unwrap().objects.iter().find(|o| o.index == 0).unwrap().position;
    let eye = view.orbit.unwrap().camera().eye;
    let radius = ((eye - pivot).length() * 0.0765).max(0.5);
    // (A little outside its line.)
    let ring = |a: f32| pivot + Vec3::new(a.cos(), a.sin(), 0.0) * radius * 1.06;
    let from = before + 2.0;
    let handle = screen(&h, area, ring(from));
    // A click on it keeps the selection.
    h.hover_at(handle);
    press(&h, handle, true, none);
    press(&h, handle, false, none);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Waypoint, 0)]);

    // Led a quarter of the way round: the waypoint faces there.
    let quarter = std::f32::consts::FRAC_PI_2;
    press(&h, handle, true, none);
    let mut at = handle;
    for k in 1..=6 {
        at = screen(&h, area, ring(from + quarter * k as f32 / 6.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-turn-ring").join("turn_ring.png")).unwrap();
    press(&h, at, false, none);
    h.run_steps(3);
    let after = facing(&mut h);
    let turned = (after - before).rem_euclid(std::f32::consts::TAU);
    assert!((turned - quarter).abs() < 0.03, "turned {turned} from {before} to {after}");
    assert_eq!(waypoint(&mut h, area, 0).map(|w| (w.0, w.1)), Some((20.0, 20.0)), "in place");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Rotate"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!((facing(&mut h) - before).abs() < 1e-4, "one undo");

    // Switched off in Options › Area: the same drag turns nothing.
    h.state_mut().settings.no_turn_ring = true;
    h.run_steps(2);
    h.hover_at(handle);
    press(&h, handle, true, none);
    for k in 1..=6 {
        at = screen(&h, area, ring(from + quarter * k as f32 / 6.0));
        h.hover_at(at);
    }
    press(&h, at, false, none);
    h.run_steps(3);
    assert!((facing(&mut h) - before).abs() < 1e-4, "no ring, no turn");
}

/// With Shift held, the rings for tilting a model: one led round turns
/// the visual transform about its axis, as one Tilt. A static placeable
/// has none.
#[test]
fn a_tilt_ring_tilts_a_placeable_s_model_while_shift_is_held() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("tilt-ring") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        // Two chests: the first static, the second (put before it) not.
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut edits = Vec::new();
        for (x, y, fixed) in [(28.0, 30.0, 1), (12.0, 20.0, 0)] {
            let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
            let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
            item.set("Static", mg_gff::Value::Byte(fixed));
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: "Placeable List".into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let rotate = |h: &mut Harness<'_, Moonglow>, index: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let chest = &git.root.list("Placeable List").unwrap()[index];
        mg_area::VisualTransform::read(chest).map(|v| v.rotate)
    };
    let look_at = |h: &mut Harness<'_, Moonglow>, index: usize, target: Vec3| {
        // From the east and above: the ring about X faces the camera.
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(ObjectKind::Placeable, index)];
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.yaw, o.pitch, o.distance) = (target, 0.3, 0.5, 14.0);
    };
    // Leads the ring about X a twelfth of the way round, Shift held.
    let lead = |h: &mut Harness<'_, Moonglow>, pivot: Vec3, shot: bool| {
        let eye = h.state().area_views[&area].orbit.unwrap().camera().eye;
        let radius = ((eye - pivot).length() * 0.0765).max(0.5);
        let ring = |a: f32| pivot + Vec3::new(0.0, a.cos(), a.sin()) * radius;
        let shift = egui::Modifiers::SHIFT;
        h.event(egui::Event::ModifiersChanged(shift));
        let mut at = screen(h, area, ring(1.0));
        h.hover_at(at);
        h.run_steps(1);
        let left = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: shift,
        };
        h.event(left(at, true));
        for k in 1..=6 {
            at = screen(h, area, ring(1.0 + std::f32::consts::FRAC_PI_6 * k as f32 / 6.0));
            h.hover_at(at);
        }
        h.run_steps(1);
        if shot {
            let img = h.render().expect("render");
            img.save(mg_testkit::scratch_dir("ui-area-tilt-ring").join("tilt_ring.png")).unwrap();
        }
        h.event(left(at, false));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };

    let pivot = Vec3::new(12.0, 20.0, 0.0);
    look_at(&mut h, 0, pivot);
    h.run_steps(2);
    assert_eq!(rotate(&mut h, 0), None, "no visual transform yet");
    lead(&mut h, pivot, true);
    let tilted = rotate(&mut h, 0).expect("a visual transform");
    assert!((tilted.x - 30.0).abs() < 1.0, "tilted {tilted:?}");
    assert_eq!((tilted.y, tilted.z), (0.0, 0.0), "about X alone");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tilt"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(rotate(&mut h, 0), None, "one undo");

    // The static chest has no rings: the same drag tilts nothing.
    let pivot = Vec3::new(28.0, 30.0, 0.0);
    look_at(&mut h, 1, pivot);
    h.run_steps(2);
    lead(&mut h, pivot, false);
    assert_eq!(rotate(&mut h, 1), None, "a static placeable is not tilted");

    // (Without rings, the drag was the usual one: it moved the chest.)
    if h.state().ws.as_ref().unwrap().can_undo() == Some("Move") {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(2);
    }

    // Its menu makes it dynamic; then it tilts.
    let at = screen(&h, area, pivot + Vec3::Z * 0.3);
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
    assert!(h.query_by_label("Make Static").is_none(), "it is static");
    h.get_by_label("Make Dynamic").click();
    h.run_steps(3);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Make Dynamic"));
    look_at(&mut h, 1, pivot);
    h.run_steps(2);
    lead(&mut h, pivot, false);
    let tilted = rotate(&mut h, 1).expect("a visual transform");
    assert!((tilted.x - 30.0).abs() < 1.0, "tilted {tilted:?}");

    // The arrow over it, Shift held: led up a meter, the chest rises one.
    let height = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        git.root.list("Placeable List").unwrap()[1].float("Z").unwrap()
    };
    let z0 = height(&mut h);
    let eye = h.state().area_views[&area].orbit.unwrap().camera().eye;
    let radius = ((eye - pivot).length() * 0.0765).max(0.5);
    let shift = egui::Modifiers::SHIFT;
    let left = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: shift,
    };
    h.event(egui::Event::ModifiersChanged(shift));
    let head = pivot + Vec3::Z * radius * 1.45;
    let mut at = screen(&h, area, head);
    h.hover_at(at);
    h.run_steps(1);
    h.event(left(at, true));
    for k in 1..=5 {
        at = screen(&h, area, head + Vec3::Z * (k as f32 / 5.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-tilt-ring").join("height_arrow.png")).unwrap();
    h.event(left(at, false));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let z = height(&mut h);
    assert!((z - z0 - 1.0).abs() < 0.03, "raised from {z0} to {z}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Raise"));

    // The green arrow, north of it: led 2 m along, the chest goes north
    // and nowhere else.
    let place = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let chest = &git.root.list("Placeable List").unwrap()[1];
        (chest.float("X").unwrap(), chest.float("Y").unwrap(), chest.float("Z").unwrap())
    };
    let (x0, y0, z0) = place(&mut h);
    let stands = Vec3::new(x0, y0, z0);
    h.event(egui::Event::ModifiersChanged(shift));
    let head = stands + Vec3::Y * radius * 1.45;
    let mut at = screen(&h, area, head);
    h.hover_at(at);
    h.run_steps(2);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-shift-tools").join("shift_tools.png")).unwrap();
    h.event(left(at, true));
    for k in 1..=5 {
        at = screen(&h, area, head + Vec3::Y * (2.0 * k as f32 / 5.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-tilt-ring").join("axis_arrow.png")).unwrap();
    h.event(left(at, false));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let (x, y, z) = place(&mut h);
    assert!((y - y0 - 2.0).abs() < 0.05, "north from {y0} to {y}");
    assert_eq!(x, x0, "not east or west");
    assert!((z - z0).abs() < 1e-3, "as high above the ground as before: {z0} to {z}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move"));
    let pivot = Vec3::new(x, y, 0.0);

    // Dynamic now, its menu makes it static again: the tilt goes.
    let at = screen(&h, area, pivot + Vec3::Z * 1.3);
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
    assert!(h.query_by_label("Make Dynamic").is_none(), "it is dynamic");
    h.get_by_label("Make Static").click();
    h.run_steps(3);
    assert_eq!(rotate(&mut h, 1), None, "a static placeable has no visual transform");
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&git_key).unwrap();
    let chest = &git.root.list("Placeable List").unwrap()[1];
    assert_eq!(chest.integer("Static"), Some(1));
}

/// A module opens with the area that was opened last in it.
#[test]
fn a_module_opens_on_the_area_opened_last_in_it() {
    let Some((mut h, area)) = area_harness("last-area") else { return };
    let path = h.state().module_path().expect("saved");
    assert_eq!(h.state().settings.last_area(&path), Some("field"));
    h.state_mut().open_module(&path);
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_none(), "opened anew");
    h.run_steps(3);
    assert!(h.state().dock.find_tab(&Tab::Area(area)).is_some(), "the area is open again");
    // Without one remembered (a module opened for the first time, or its
    // area gone), the first the tree lists, remembered from then on.
    for forgotten in [Some("gone"), None] {
        match forgotten {
            Some(gone) => h.state_mut().settings.last_areas[0].1 = gone.into(),
            None => h.state_mut().settings.last_areas.clear(),
        }
        h.state_mut().open_module(&path);
        h.run_steps(3);
        assert!(h.state().dock.find_tab(&Tab::Area(area)).is_some(), "the first area");
        assert_eq!(h.state().settings.last_area(&path), Some("field"));
    }
}

/// Make Placeables Static: those that lose nothing by it, as one command.
#[test]
fn an_area_s_placeables_are_made_static_together() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("static-placeables") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        // Three chests: plain, Useable, and tilted.
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut edits = Vec::new();
        for (index, useable) in [(0, 0), (1, 1), (2, 0)] {
            let at = Placement { position: [10.0 + 5.0 * index as f32, 20.0, 0.0], rotation: 0.0 };
            let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
            for (label, value) in [("Static", 0), ("Useable", useable), ("HasInventory", 0)] {
                item.set(label, mg_gff::Value::Byte(value));
            }
            // (The blueprint's scripts would keep it dynamic.)
            let scripts: Vec<String> = item
                .fields
                .iter()
                .filter(|f| matches!(f.value, mg_gff::Value::ResRef(_)))
                .map(|f| f.label.to_string())
                .filter(|l| l.starts_with("On"))
                .collect();
            for label in scripts {
                item.set(&label, mg_gff::Value::resref(ResRef::EMPTY));
            }
            item.set("Conversation", mg_gff::Value::resref(ResRef::EMPTY));
            item.set("AnimationState", mg_gff::Value::Byte(0));
            item.set("TrapFlag", mg_gff::Value::Byte(0));
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: "Placeable List".into(),
                index,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let s = ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[2].clone();
        let game = h.state().game.as_deref().unwrap();
        let o = mg_area::AreaObject::read(game, mg_area::ObjectKind::Placeable, 2, &s);
        let tilt = mg_area::VisualTransform { rotate: glam::Vec3::X * 20.0, ..Default::default() };
        let edits = mg_area::edit::visual_transform_edits(git_key, &o, &s, tilt);
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tilt", edits)));
    }
    h.run_steps(3);
    let fixed = |h: &mut Harness<'_, Moonglow>| -> Vec<i64> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let list = git.root.list("Placeable List").unwrap();
        list.iter().map(|p| p.integer("Static").unwrap()).collect()
    };
    assert_eq!(fixed(&mut h), [0, 0, 0]);
    h.state_mut().actions.push(mg_ui::Action::StaticPlaceables(vec![area]));
    h.run_steps(3);
    assert_eq!(fixed(&mut h), [1, 0, 0], "the Useable and the tilted ones stay dynamic");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Make Placeables Static"));
    let said = &h.state().log.entries.last().unwrap().1;
    assert!(said.starts_with("Made 1 placeable static in 1 area"), "{said}");
    assert!(said.contains("1 Useable, 1 with a visual transform"), "{said}");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(fixed(&mut h), [0, 0, 0], "one undo");

    // And back, every one at once.
    h.state_mut().actions.push(mg_ui::Action::StaticPlaceables(vec![area]));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::DynamicPlaceables(vec![area]));
    h.run_steps(3);
    assert_eq!(fixed(&mut h), [0, 0, 0]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Make Placeables Dynamic"));
    assert_eq!(h.state().log.entries.last().unwrap().1, "Made 1 placeable dynamic in 1 area");
}

/// From straight above, where a tilt ring is seen edge-on and the up arrow
/// end on, both are still led, by the pointer's travel on screen. Adjust
/// Location offers a static placeable no visual transform. The Lighting
/// switch is kept in the settings.
#[test]
fn rings_and_arrows_are_led_from_straight_above() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("from-above") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut edits = Vec::new();
        for (x, y, fixed) in [(28.0, 30.0, 1), (12.0, 30.0, 0)] {
            let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
            let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
            item.set("Static", mg_gff::Value::Byte(fixed));
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: "Placeable List".into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let chest = |h: &mut Harness<'_, Moonglow>, index: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[index].clone()
    };
    let adjust = |h: &mut Harness<'_, Moonglow>, at: Vec3| -> bool {
        let at = screen(h, area, at + Vec3::Z * 0.3);
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
        h.get_by_label("Adjust Location…").click();
        h.run_steps(3);
        let offered = h.query_by_label("Visual Transforms").is_some();
        h.get_by_label("Cancel").click();
        h.run_steps(2);
        offered
    };
    assert!(!adjust(&mut h, Vec3::new(28.0, 30.0, 0.0)), "a static placeable has none");
    let pivot = Vec3::new(12.0, 30.0, 0.0);
    assert!(adjust(&mut h, pivot), "a dynamic one has");
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Placeable, 0)]);

    // Shift + the arrow or ring at `from`, led to `to`.
    let lead = |h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2| {
        let shift = egui::Modifiers::SHIFT;
        let left = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: shift,
        };
        h.event(egui::Event::ModifiersChanged(shift));
        h.hover_at(from);
        h.run_steps(1);
        h.event(left(from, true));
        for k in 1..=5 {
            h.hover_at(from + (to - from) * (k as f32 / 5.0));
        }
        h.run_steps(1);
        h.event(left(to, false));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };
    // The camera looks straight down, as the view opens.
    let eye = h.state().area_views[&area].orbit.unwrap().camera().eye;
    let radius = ((eye - pivot).length() * 0.0765).max(0.5);

    // The up arrow, end on: up the screen raises the chest.
    let head = screen(&h, area, pivot + Vec3::Z * radius * 1.45);
    lead(&mut h, head, head + egui::vec2(0.0, -40.0));
    let z = chest(&mut h, 0).float("Z").unwrap();
    assert!(z > 0.05, "raised to {z}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Raise"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);

    // The ring about X, edge-on (a line north and south over the chest):
    // led along that line, it tilts the model.
    let ring = |a: f32| pivot + Vec3::new(0.0, a.cos(), a.sin()) * radius;
    let (from, to) = (screen(&h, area, ring(1.05)), screen(&h, area, ring(0.52)));
    lead(&mut h, from, to);
    let tilted = mg_area::VisualTransform::read(&chest(&mut h, 0)).expect("a visual transform");
    assert!((-40.0..-10.0).contains(&tilted.rotate.x), "tilted {:?}", tilted.rotate);
    assert_eq!((tilted.rotate.y, tilted.rotate.z), (0.0, 0.0));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tilt"));

    // Lighting, switched off, stays off for views opened later.
    assert!(!h.state().settings.unlit_areas);
    h.get_by_label("💡 Lighting").click();
    h.run_steps(2);
    assert!(h.state().settings.unlit_areas);
}

/// A selected encounter's spawn points: the tip of a point's arrow, led
/// round, turns the way what spawns there faces.
#[test]
fn a_spawn_point_s_arrow_turns_its_facing() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("spawn-facing") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("nw_giantevil", ResType::UTE).unwrap();
        let blueprint = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let outline = [[-3.0, -3.0, 0.0], [3.0, -3.0, 0.0], [3.0, 3.0, 0.0], [-3.0, 3.0, 0.0]];
        let at = Placement { position: [12.0, 30.0, 0.0], rotation: 0.0 };
        let mut enc = instance(&placing, ResType::UTE, &blueprint, at, &outline).unwrap();
        let mut point = mg_gff::Struct::new(2);
        for (label, v) in [("X", 12.0), ("Y", 30.0), ("Z", 0.0), ("Orientation", 0.0)] {
            point.set(label, mg_gff::Value::Float(v));
        }
        enc.set("SpawnPointList", mg_gff::Value::List(vec![point]));
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Encounter List".into(),
            index: 0,
            item: enc,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    h.run_steps(3);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Encounter, 0)];
    h.run_steps(2);
    let facing = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let enc = &git.root.list("Encounter List").unwrap()[0];
        enc.list("SpawnPointList").unwrap()[0].float("Orientation").unwrap()
    };
    let at = Vec3::new(12.0, 30.0, 0.1);
    let tip = |a: f32| at + Vec3::new(a.cos(), a.sin(), 0.0);
    let none = egui::Modifiers::NONE;
    let from = screen(&h, area, tip(0.0));
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, none);
    let mut to = from;
    for k in 1..=6 {
        to = screen(&h, area, tip(std::f32::consts::FRAC_PI_2 * k as f32 / 6.0));
        h.hover_at(to);
    }
    h.run_steps(1);
    press(&h, to, false, none);
    h.run_steps(3);
    let turned = facing(&mut h);
    assert!((turned - std::f32::consts::FRAC_PI_2).abs() < 0.03, "faces {turned}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Turn Spawn Point"));
    assert_eq!(h.state().area_views[&area].selection, [(ObjectKind::Encounter, 0)]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(facing(&mut h), 0.0, "one undo");
    // The foot of its post, led over the ground, moves the point alone.
    let place = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let enc = &git.root.list("Encounter List").unwrap()[0];
        let p = &enc.list("SpawnPointList").unwrap()[0];
        let outline = enc.float("XPosition").unwrap();
        (p.float("X").unwrap(), p.float("Y").unwrap(), outline)
    };
    let foot = screen(&h, area, Vec3::new(12.0, 30.0, 0.0));
    h.hover_at(foot);
    h.run_steps(1);
    press(&h, foot, true, none);
    let mut to = foot;
    for k in 1..=6 {
        to = screen(&h, area, Vec3::new(12.0 + 0.4 * k as f32, 30.0 - 0.2 * k as f32, 0.0));
        h.hover_at(to);
        h.run_steps(1);
    }
    press(&h, to, false, none);
    h.run_steps(3);
    let (x, y, encounter) = place(&mut h);
    assert!((x - 14.4).abs() < 0.2 && (y - 28.8).abs() < 0.2, "moved to {x}, {y}");
    assert_eq!(encounter, 12.0, "the encounter stays");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move Spawn Point"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(place(&mut h), (12.0, 30.0, 12.0), "one undo");
    // G on an encounter raised off the ground brings it down, its lowest
    // corner onto the ground there.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Raise",
        vec![mg_edit::Edit::SetField {
            key: git_key,
            path: mg_edit::GffPath::root().item("Encounter List", 0),
            label: "ZPosition".into(),
            value: Some(mg_gff::Value::Float(3.0)),
        }],
    )));
    h.run_steps(2);
    let inside = screen(&h, area, Vec3::new(10.5, 31.5, 0.0));
    h.hover_at(inside);
    h.run_steps(1);
    h.key_press(egui::Key::G);
    h.run_steps(2);
    let z = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        git.root.list("Encounter List").unwrap()[0].float("ZPosition").unwrap()
    };
    assert!(z.abs() < 0.5, "on the ground: {z}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Drop to Ground"));
}

/// The palette's hover says something of a blueprint that has no model to
/// show: an encounter's creatures, here.
#[test]
fn the_palette_s_hover_tells_of_blueprints_without_a_model() {
    let Some(mut h) = game_harness("palette-about") else { return };
    {
        let app = h.state_mut();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
        app.palette.kind = mg_module::palette::BlueprintKind::Encounter;
        app.palette.tiles = false;
        app.palette.custom = false;
        app.palette.filter = "nw_verminbeet".into();
    }
    h.run();
    // The one blueprint found: the row under the search's.
    let rows: Vec<_> = h.query_all_by_role(egui::accesskit::Role::Button).collect();
    let row = rows
        .iter()
        .filter(|n| n.accesskit_node().label().is_some_and(|l| l.contains("Beetle")))
        .map(|n| n.rect().center())
        .next_back()
        .expect("the encounter's row");
    assert!(h.query_by_label_contains("Spawns ").is_none());
    h.hover_at(row);
    // (A tooltip waits a moment.)
    h.run_steps(60);
    assert!(h.query_by_label_contains("Spawns ").is_some(), "its creatures are named");
    assert!(h.query_by_label_contains(" creatures; ").is_some());
}

/// The area's menu sets the module's start location where it was opened.
#[test]
fn the_start_location_is_set_from_the_area_s_menu() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("set-start") else { return };
    let entry = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let info = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap().root.clone();
        let f = |label: &str| info.float(label).unwrap();
        (info.resref("Mod_Entry_Area").unwrap(), f("Mod_Entry_X"), f("Mod_Entry_Y"))
    };
    let (_, x0, y0) = entry(&mut h);
    let at = screen(&h, area, Vec3::new(12.0, 31.0, 0.0));
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
    h.get_by_label("Set Start Location Here").click();
    h.run_steps(3);
    let (in_area, x, y) = entry(&mut h);
    assert_eq!(in_area, area);
    assert!((x - 12.0).abs() < 0.1 && (y - 31.0).abs() < 0.1, "at {x}, {y}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Set start location"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    let (_, x, y) = entry(&mut h);
    assert_eq!((x, y), (x0, y0), "one undo");
}

/// The start location's marker is dragged by its ring and turned by its
/// arrow's tip.
#[test]
fn the_start_location_is_dragged_and_turned_in_the_view() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("drag-start") else { return };
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    {
        let set = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
            key: ifo,
            path: mg_edit::GffPath::root(),
            label: label.into(),
            value: Some(value),
        };
        let edits = vec![
            set("Mod_Entry_Area", mg_gff::Value::resref(area)),
            set("Mod_Entry_X", mg_gff::Value::Float(12.0)),
            set("Mod_Entry_Y", mg_gff::Value::Float(31.0)),
            set("Mod_Entry_Dir_X", mg_gff::Value::Float(1.0)),
            set("Mod_Entry_Dir_Y", mg_gff::Value::Float(0.0)),
        ];
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
    let entry = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let r = ws.doc(&ifo).unwrap().root.clone();
        let f = |label: &str| r.float(label).unwrap();
        (f("Mod_Entry_X"), f("Mod_Entry_Y"), f("Mod_Entry_Dir_Y").atan2(f("Mod_Entry_Dir_X")))
    };
    let none = egui::Modifiers::NONE;
    let drag = |h: &mut Harness<'_, Moonglow>, path: &[Vec3]| {
        let from = screen(h, area, path[0]);
        h.hover_at(from);
        h.run_steps(1);
        press(h, from, true, none);
        let mut to = from;
        for p in &path[1..] {
            to = screen(h, area, *p);
            h.hover_at(to);
            h.run_steps(1);
        }
        press(h, to, false, none);
        h.run_steps(3);
    };
    // By the ring's west side, four meters east and north.
    let west = Vec3::new(11.0, 31.0, 0.0);
    let steps: Vec<Vec3> = (0..=4).map(|k| west + Vec3::new(k as f32, k as f32, 0.0)).collect();
    drag(&mut h, &steps);
    let (x, y, facing) = entry(&mut h);
    assert!((x - 16.0).abs() < 0.15 && (y - 35.0).abs() < 0.15, "moved to {x}, {y}");
    assert!(facing.abs() < 1e-3, "faces as it did: {facing}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Move Start Location"));
    assert!(h.state().area_views[&area].start_selected, "held, it is selected");
    // A click on the ground lets it go; one on its ring selects it.
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let pos = screen(h, area, p);
        h.hover_at(pos);
        h.run_steps(1);
        press(h, pos, true, none);
        press(h, pos, false, none);
        h.run_steps(2);
    };
    click(&mut h, Vec3::new(x + 6.0, y, 0.0));
    assert!(!h.state().area_views[&area].start_selected);
    click(&mut h, Vec3::new(x - 1.0, y, 0.0));
    assert!(h.state().area_views[&area].start_selected, "selected by a click");
    assert_eq!(entry(&mut h).0, x, "a click moves nothing");
    // By the arrow's tip, a quarter turn to the north (it is selected).
    let middle = Vec3::new(x, y, 0.0);
    let tip = |a: f32| middle + Vec3::new(a.cos(), a.sin(), 0.0) * 0.75;
    let round: Vec<Vec3> =
        (0..=6).map(|k| tip(std::f32::consts::FRAC_PI_2 * k as f32 / 6.0)).collect();
    drag(&mut h, &round);
    let (x1, y1, facing) = entry(&mut h);
    assert_eq!((x1, y1), (x, y), "turned where it is");
    assert!((facing - std::f32::consts::FRAC_PI_2).abs() < 0.05, "faces {facing}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Turn Start Location"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(entry(&mut h).2.abs() < 1e-3, "one undo");
    // With the marker hidden, there is nothing to take hold of.
    h.state_mut().area_views.get_mut(&area).unwrap().show_start = false;
    h.run_steps(2);
    let steps: Vec<Vec3> = (0..=4).map(|k| Vec3::new(15.0 + k as f32, 35.0, 0.0)).collect();
    drag(&mut h, &steps);
    assert_eq!(entry(&mut h).0, x, "not moved");
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
    h.get_all_by_label("Collapse All").next().unwrap().click();
    h.run_steps(4);
    assert_eq!(listed(&h), before - 1, "the area's row is gone from the tree");
    h.get_all_by_label("Expand All").next().unwrap().click();
    h.run_steps(4);
    assert_eq!(listed(&h), before, "and back");
}

/// Edit beside a script's name makes a script that is nowhere (as Aurora
/// does) and opens it; the module's and the game's open as they are.
#[test]
fn edit_beside_a_script_s_name_creates_a_missing_script() {
    let Some((mut h, _)) = area_harness("edit-script") else { return };
    let name = ResRef::from_str("fresh_one").unwrap();
    let key = ResKey::new(name, ResType::NSS);
    let has = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().module.contains(&key);
    assert!(!has(&h));
    h.state_mut().actions.push(mg_ui::Action::EditScript { name, condition: false });
    h.run_steps(3);
    assert!(has(&h), "made in the module");
    assert!(h.state().dock.find_tab(&Tab::Script(key)).is_some(), "and opened");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("New script fresh_one.nss"));
    // Again: opened, not made anew.
    h.state_mut().actions.push(mg_ui::Action::EditScript { name, condition: false });
    h.run_steps(3);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("New script fresh_one.nss"));
    // A condition starts as one.
    let name = ResRef::from_str("fresh_check").unwrap();
    h.state_mut().actions.push(mg_ui::Action::EditScript { name, condition: true });
    h.run_steps(3);
    let ws = h.state().ws.as_ref().unwrap();
    let text = ws.module.get(&ResKey::new(name, ResType::NSS)).unwrap();
    assert!(text.starts_with(b"int StartingConditional()"));
    // The game's own script is shown, not copied into the module.
    let name = ResRef::from_str("nw_c2_default1").unwrap();
    h.state_mut().actions.push(mg_ui::Action::EditScript { name, condition: false });
    h.run_steps(3);
    let key = ResKey::new(name, ResType::NSS);
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&key));
    assert!(h.state().dock.find_tab(&Tab::Resource(key)).is_some());
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

/// Options › Script Editor › Open scripts in the external editor: a
/// script opened from the module tree goes to the external editor too.
#[cfg(unix)]
#[test]
fn a_script_opened_from_the_tree_goes_to_the_external_editor() {
    let Some((mut h, _)) = area_harness("script-external") else { return };
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key, data: Some(b"void main() { }\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.state_mut().settings.external_editor = Some("/bin/true".into());
    h.run_steps(3);
    // (As a double click on it in the module tree does.)
    let open = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::OpenResource(key));
        h.run_steps(4);
    };
    let said = |h: &Harness<'_, Moonglow>| {
        h.state().log.entries.iter().filter(|e| e.1.contains("editing in /bin/true")).count()
    };
    // Without the option: Moonglow's editor alone.
    open(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Script(key)).is_some());
    assert_eq!(said(&h), 0);
    // With it: the external editor is started as well.
    let tab = h.state().dock.find_tab(&Tab::Script(key)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().settings.scripts_external = true;
    h.run_steps(2);
    open(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Script(key)).is_some());
    assert_eq!(said(&h), 1, "{:?}", h.state().log.entries.last());
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
    // Its two waypoints, by name.
    let rows: Vec<_> = h.query_all_by_label("Waypoint").map(|n| n.rect().center()).collect();
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
    let copy = ws.doc(&git).unwrap().root.list("WaypointList").unwrap()[0].clone();
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
    let rows: Vec<_> = h.query_all_by_label("Waypoint").map(|n| n.rect().center()).collect();
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
    h.get_by_label("Ferry Landing");
    let filter = h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap();
    filter.click();
    h.run_steps(2);
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text("ferry");
    h.run_steps(4);
    h.get_by_label("Waypoints (1 of 2)");
    h.get_by_label("Ferry Landing");
    assert_eq!(h.query_all_by_label("Waypoint").count(), 0);
}

/// Prefabs are found where builders look: a button on the area's toolbar
/// when several objects are selected, and a Prefabs section in the palette
/// that lists them, places them and says how to make one.
#[test]
fn prefabs_are_offered_on_the_toolbar_and_in_the_palette() {
    use mg_area::ObjectKind::Waypoint;
    let Some((mut h, area)) = area_harness("prefab-found") else { return };
    let prefabs = mg_testkit::scratch_dir("ui-prefab-found");
    h.state_mut().prefab_dir = Some(prefabs.clone());
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    // The palette's Prefabs: none yet, and how to make one.
    palette_type(&mut h, "🗐", "Prefabs");
    h.run_steps(2);
    assert!(h.query_by_label_contains("No prefabs yet").is_some());
    assert!(h.query_by_label_contains("Save 2 as Prefab").is_none());

    // Two objects selected: the toolbar offers to save them.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(Waypoint, 0), (Waypoint, 1)];
    h.run_steps(2);
    h.get_by_label("Save 2 as Prefab…").click();
    h.run_steps(2);
    let (_, clip) = h.state().prefab_save.clone().expect("the Save as Prefab window");
    assert_eq!(clip.objects.len(), 2);
    h.state_mut().prefab_save = None;
    // The palette's button does the same.
    h.get_by_label("Save Selection as Prefab…").click();
    h.run_steps(2);
    assert!(h.state().prefab_save.is_some());
    h.state_mut().prefab_save = None;
    h.state_mut().save_prefab("Camp", &clip).unwrap();
    h.run_steps(2);

    // Saved, it is listed in the palette; a click takes it up to place.
    assert!(h.query_by_label_contains("No prefabs yet").is_none());
    h.get_by_label("Camp").click();
    h.run_steps(3);
    assert!(h.state().area_views[&area].pasting, "it follows the pointer, to be placed");
    assert!(h.state().log.entries.iter().any(|e| e.1.contains("Prefab Camp: click in the area")));
    // Over the area, its objects show see-through where they would go.
    let over = screen(&h, area, glam::Vec3::new(12.0, 30.0, 0.0));
    h.hover_at(over);
    h.run_steps(3);
    assert_eq!(h.state().area_views[&area].pasted_shown, 2);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-prefab-ghost").join("prefab_ghost.png")).unwrap();
    // What it holds shows while the pointer rests on it.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    let holds = mg_ui::prefabs::summary(Some(&prefabs), "Camp").unwrap();
    assert!(holds.starts_with("2 "), "{holds}");
    // Renamed from its menu.
    h.get_by_label("Camp").click_secondary();
    h.run_steps(3);
    h.get_by_label("Rename…").click();
    h.run_steps(3);
    let field = h.get_by(|n| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some("Camp")
    });
    field.type_text(" Site");
    h.run_steps(2);
    h.get_by_label("Rename").click();
    h.run_steps(3);
    assert!(prefabs.join("Camp Site.prefab.json").is_file(), "{:?}", h.state().log.entries);
    assert!(!prefabs.join("Camp.prefab.json").exists());
    // Deleted from the palette, and put back by Undo Delete.
    h.get_by_label("Camp Site").click_secondary();
    h.run_steps(3);
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert!(!prefabs.join("Camp Site.prefab.json").exists());
    assert!(h.query_by_label_contains("No prefabs yet").is_some());
    h.get_by_label("Undo Delete").click();
    h.run_steps(3);
    assert!(prefabs.join("Camp Site.prefab.json").is_file());
    assert!(h.query_by_label("Camp Site").is_some());
    assert!(h.query_by_label("Undo Delete").is_none());
}

/// Every selected object has its turning ring: the ring of any of them,
/// led round, turns them all.
#[test]
fn each_selected_object_has_a_ring_and_any_turns_them_all() {
    use glam::Vec3;
    use mg_area::ObjectKind::Waypoint;
    let Some((mut h, area)) = area_harness("rings-all") else { return };
    let facings = |h: &mut Harness<'_, Moonglow>| -> Vec<f32> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let list = git.root.list("WaypointList").unwrap();
        let facing = |w: &mg_gff::Struct| {
            w.float("YOrientation").unwrap().atan2(w.float("XOrientation").unwrap())
        };
        list.iter().map(facing).collect()
    };
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(Waypoint, 0), (Waypoint, 1)];
    h.run_steps(2);
    let before = facings(&mut h);
    // The ring around the second waypoint (the first selected is the
    // other), taken away from where it faces.
    let view = &h.state().area_views[&area];
    let second = view.model.as_ref().unwrap().objects.iter().find(|o| o.index == 1).unwrap();
    let pivot = second.position;
    let eye = view.orbit.unwrap().camera().eye;
    let radius = ((eye - pivot).length() * 0.0765).max(0.5);
    let ring = |a: f32| pivot + Vec3::new(a.cos(), a.sin(), 0.0) * radius;
    let from = before[1] + 2.0;
    let quarter = std::f32::consts::FRAC_PI_2;
    let none = egui::Modifiers::NONE;
    let mut at = screen(&h, area, ring(from));
    h.hover_at(at);
    press(&h, at, true, none);
    for k in 1..=6 {
        at = screen(&h, area, ring(from + quarter * k as f32 / 6.0));
        h.hover_at(at);
    }
    h.run_steps(1);
    press(&h, at, false, none);
    h.run_steps(3);
    let after = facings(&mut h);
    for (was, now) in before.iter().zip(&after) {
        let turned = (now - was).rem_euclid(std::f32::consts::TAU);
        assert!((turned - quarter).abs() < 0.03, "turned {turned}: {before:?} to {after:?}");
    }
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Rotate"));
}

/// The wheel over a window that lies on the area view is the window's:
/// the area's camera stays where it is.
#[test]
fn the_wheel_over_a_window_leaves_the_area_s_camera_be() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, area)) = area_harness("wheel-window") else { return };
    {
        let app = h.state_mut();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
        app.palette.kind = BlueprintKind::Placeable;
        app.palette.tiles = false;
    }
    h.run_steps(3);
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…").click();
    h.run_steps(3);
    let distance = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().distance;
    let before = distance(&h);
    // (A row of the window, which lies over the area's view.)
    let view = h.state().area_views[&area].rect;
    let row = h
        .query_all_by_label_contains("  (category")
        .map(|n| n.rect().center())
        .find(|p| view.contains(*p))
        .expect("a row over the view");
    h.hover_at(row);
    h.run_steps(3);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -3.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(20);
    assert_eq!(distance(&h), before, "the area isn't zoomed from under the window");
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

    // Together: the two turn about their middle as one, each keeping
    // where the other is, seen from the way it faces.
    let seen = |w: &[mg_gff::Struct]| {
        let at = |s: &mg_gff::Struct| {
            glam::Vec2::new(s.float("XPosition").unwrap(), s.float("YPosition").unwrap())
        };
        let to = at(&w[1]) - at(&w[0]);
        (to.y.atan2(to.x) - facing(&w[0])).rem_euclid(std::f32::consts::TAU)
    };
    let was = waypoints(&mut h);
    h.state_mut().settings.turn_together = true;
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(2);
    let now = waypoints(&mut h);
    assert!((facing(&was[0]) - facing(&now[0]) - 90f32.to_radians()).abs() < 1e-4, "turned");
    let moved = (was[0].float("XPosition").unwrap() - now[0].float("XPosition").unwrap()).abs()
        + (was[0].float("YPosition").unwrap() - now[0].float("YPosition").unwrap()).abs();
    assert!(moved > 0.1, "led round the middle");
    assert!((seen(&was) - seen(&now)).abs() < 1e-3, "{} → {}", seen(&was), seen(&now));
    h.state_mut().settings.turn_together = false;
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);

    // A trigger turns by its outline: alone about the outline's middle,
    // and Together with the rest about theirs.
    let git = ResKey::new(area, ResType::GIT);
    let mut trigger = mg_gff::Struct::new(1);
    for (label, v) in [("XPosition", 22.0), ("YPosition", 20.0), ("ZPosition", 0.0)] {
        trigger.set(label, mg_gff::Value::Float(v));
    }
    let corner = |x: f32, y: f32| {
        let mut p = mg_gff::Struct::new(3);
        p.set("PointX", mg_gff::Value::Float(x));
        p.set("PointY", mg_gff::Value::Float(y));
        p.set("PointZ", mg_gff::Value::Float(0.0));
        p
    };
    let corners = vec![corner(0.0, 0.0), corner(4.0, 0.0), corner(4.0, 2.0), corner(0.0, 2.0)];
    trigger.set("Geometry", mg_gff::Value::List(corners));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Trigger",
        vec![mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "TriggerList".into(),
            index: 0,
            item: trigger,
        }],
    )));
    h.run_steps(3);
    // The outline's corners in the area, from the GIT.
    let outline = |h: &mut Harness<'_, Moonglow>| -> Vec<glam::Vec2> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let t = ws.doc(&git).unwrap().root.list("TriggerList").unwrap()[0].clone();
        let at = glam::Vec2::new(t.float("XPosition").unwrap(), t.float("YPosition").unwrap());
        t.list("Geometry")
            .unwrap()
            .iter()
            .map(|p| at + glam::Vec2::new(p.float("PointX").unwrap(), p.float("PointY").unwrap()))
            .collect()
    };
    let near = |a: glam::Vec2, b: glam::Vec2| a.distance(b) < 1e-3;
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Trigger, 0)];
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(2);
    // (22..26 by 20..22, a quarter turn to the right about 24, 21.)
    let now = outline(&mut h);
    assert!(near(now[0], glam::Vec2::new(23.0, 23.0)), "{now:?}");
    assert!(near(now[2], glam::Vec2::new(25.0, 19.0)), "{now:?}");
    let alone = now;
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Trigger, 0), (Waypoint, 0)];
    h.state_mut().settings.turn_together = true;
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::E);
    h.run_steps(2);
    let together = outline(&mut h);
    let side = |o: &[glam::Vec2]| o[1] - o[0];
    assert!(near(side(&together), -side(&alone).perp()), "{alone:?} → {together:?}");
    h.state_mut().settings.turn_together = false;
    for _ in 0..3 {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(2);
    }
    h.state_mut().area_views.get_mut(&area).unwrap().selection = both.clone();
    h.run_steps(2);

    // G drops a raised object to the ground.
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
    palette_type(&mut h, "🗻", "Tiles");
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
    // With Ctrl held the press is the camera's (Ctrl + drag moves the
    // view): the brush paints nothing, dragged or let go where it was.
    let ctrl = egui::Modifiers::COMMAND;
    press(&h, from, true, ctrl);
    h.run_steps(1);
    for x in [12.0, 16.0, 20.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 20.0, 0.0)));
        h.run_steps(1);
    }
    let at = h.state().area_views[&area].screen_pos(Vec3::new(20.0, 20.0, 0.0)).unwrap_or(to);
    press(&h, at, false, ctrl);
    h.run_steps(3);
    let spot = screen(&h, area, Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(spot);
    press(&h, spot, true, ctrl);
    press(&h, spot, false, ctrl);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), before, "a Ctrl + drag and a Ctrl + click paint nothing");
    // And the next plain click paints as before.
    press(&h, spot, true, egui::Modifiers::NONE);
    press(&h, spot, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_ne!(lattice(&mut h), before);
}

#[test]
fn cursors_that_only_choose_tiles_again_are_blue() {
    use glam::Vec3;
    use mg_ui::terrain_mode::CYCLE;
    let Some((mut h, area)) = area_harness("cycle-cursor") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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
    // (Previous Variant: a step back from it is where it was.)
    let back = mg_tiles::paint::step_fit(&index, &before.lattice.cell(1, 1), next, true);
    assert_eq!(back, Some(before.tile(1, 1)));
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
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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
    palette_type(&mut h, "🗻", "Tiles");
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

/// Build Module without the game's data says that it is the game's data
/// that is missing (there is a module).
#[test]
fn a_build_without_game_data_says_so() {
    let dir = mg_testkit::scratch_dir("ui-build-no-game");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Build").click();
    h.run_steps(2);
    h.get_by_label("Build Module…").click();
    h.run_steps(2);
    h.get_all_by_label("Build").last().unwrap().click();
    h.run_steps(3);
    let results: Vec<String> =
        h.state().build.as_ref().unwrap().results.iter().map(|f| f.text.clone()).collect();
    assert_eq!(results.len(), 1, "{results:?}");
    assert!(results[0].starts_with("No game data"), "{results:?}");
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
    h.get_by_label_contains("Plugins are experimental: the plugin API (0.2)");
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
    assert!(h.state().dock.find_tab(&Tab::AreasProperties(caves)).is_some());
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

/// A compiled model of the load order goes out as the text it compiles
/// from: Save As Text… for one, and Export as Files with Models as text
/// ticked for those listed; without it, as it is.
#[test]
fn compiled_models_are_saved_as_text() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-models-as-text");
    let (plain, text) = (dir.join("plain"), dir.join("text"));
    for d in [&plain, &text] {
        std::fs::create_dir_all(d).unwrap();
    }
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let dialogs = NoDialogs {
        save: vec![dir.join("one.mdl")],
        folders: vec![text.clone(), plain.clone()],
        ..Default::default()
    };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    let key = ResKey::parse("plc_a01", ResType::MDL).unwrap();
    let is_text = |p: &std::path::Path| {
        let data = std::fs::read(p).unwrap();
        !mg_mdl::is_binary(&data) && String::from_utf8_lossy(&data).contains("newmodel")
    };
    app.run(mg_ui::Action::SaveModelText(key));
    assert!(is_text(&dir.join("one.mdl")));
    app.run(mg_ui::Action::SaveResources(vec![key]));
    assert!(!is_text(&plain.join("plc_a01.mdl")), "as it is, unless asked");
    app.browser.models_as_text = true;
    app.run(mg_ui::Action::SaveResources(vec![key]));
    assert!(is_text(&text.join("plc_a01.mdl")));
    assert!(app.log.entries.iter().any(|(_, m)| m.contains("1 compiled models as text")));
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
        // (The module opens on its Properties, not on an area.)
        app.settings.no_last_area = true;
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

/// A script changed since it was compiled goes out, from the module tree,
/// with what it compiles to now, not with the older compiled script.
#[test]
fn a_stale_compiled_script_is_compiled_before_it_goes_out() {
    let Some((mut h, _)) = area_harness("stale-ncs") else { return };
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let ncs = ResKey::parse("hello", ResType::NCS).unwrap();
    let scratch = mg_testkit::scratch_dir("ui-stale-ncs").join("scratch");
    h.state_mut().settings.scratch_dir = Some(scratch.clone());
    let set = |h: &mut Harness<'_, Moonglow>, k: ResKey, data: &[u8]| {
        let edit = mg_edit::Edit::SetResource { key: k, data: Some(data.to_vec()) };
        let command = mg_edit::Command::new("Setup", vec![edit]);
        h.state_mut().actions.push(mg_ui::Action::Apply(command));
        h.run_steps(2);
    };
    // The script, and a compiled script that isn't its own.
    set(&mut h, key, b"void main() { SpeakString(\"new\"); }\n");
    set(&mut h, ncs, b"NCS V1.0 old");
    let out = |h: &mut Harness<'_, Moonglow>| {
        let export =
            mg_ui::Action::ExportFiles { keys: vec![key], dependencies: false, scratch: true };
        h.state_mut().actions.push(export);
        h.run_steps(3);
        std::fs::read(scratch.join("hello.ncs")).unwrap()
    };
    let written = out(&mut h);
    assert_ne!(written, b"NCS V1.0 old", "compiled first");
    assert!(written.starts_with(b"NCS V1.0"));
    let said = |h: &Harness<'_, Moonglow>, text: &str| {
        h.state().log.entries.iter().any(|e| e.1.contains(text))
    };
    assert!(said(&h, "Compiled first (changed since last compiled): hello"));
    let module = h.state().ws.as_ref().unwrap().module.get(&ncs).map(<[u8]>::to_vec);
    assert_eq!(module, Some(written.clone()), "and kept in the module");
    // Again: it is current, and nothing is compiled.
    let entries = h.state().log.entries.len();
    assert_eq!(out(&mut h), written);
    let later = &h.state().log.entries[entries..];
    assert!(!later.iter().any(|e| e.1.contains("Compiled first")), "{later:?}");
    // One that no longer compiles goes out with what it has, and is named.
    set(&mut h, key, b"void main() { x = 1; }\n");
    assert_eq!(out(&mut h), written);
    assert!(said(&h, "No longer compiles (the .ncs written is older than the script): hello"));
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

/// Options › General › Light theme.
#[test]
fn the_theme_is_set_in_options() {
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(Vec::new()));
    h.run();
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    h.state_mut().settings.light_theme = true;
    h.run();
    assert_eq!(h.ctx.theme(), egui::Theme::Light);
}

/// An editor's window opens as large as one of its kind was last left, and
/// Maximize (its tab's menu, or a double click on the tab) fills the
/// panes' room and puts it back.
#[test]
fn windows_remember_their_size_and_maximize() {
    let Some((mut h, area)) = area_harness("window-sizes") else { return };
    let props = Tab::AreaProperties(area);
    // The room the window's pane takes.
    let pane = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let s = h.state();
        let path = s.dock.find_tab(tab).expect("open");
        assert!(!path.surface.is_main(), "in a window of its own");
        s.dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1.rect
    };
    let open = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
        h.run_steps(8);
    };
    let close = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::CloseTab(props.clone()));
        h.run_steps(3);
    };
    open(&mut h);
    let first = pane(&h, &props);
    // Maximized, and back.
    h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(props.clone()));
    h.run_steps(8);
    let big = pane(&h, &props);
    assert!(big.area() > first.area() + 1000.0, "larger: {big:?} than {first:?}");
    // Over the whole of the window under the toolbar: the module tree and
    // the log too, not the middle alone.
    assert!(big.width() > 1100.0 * 0.95, "the window's width: {big:?}");
    assert!(big.bottom() > 800.0 * 0.95, "to the window's foot: {big:?}");
    h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(props.clone()));
    h.run_steps(8);
    let back = pane(&h, &props);
    assert!((back.size() - first.size()).length() < 2.0, "back to {back:?}, was {first:?}");
    assert!((back.min - first.min).length() < 2.0, "where it was: {back:?}, {first:?}");
    // A double click on the bar beside the tab does the same, twice; and
    // the window is still dragged by the bar.
    let bar = |h: &Harness<'_, Moonglow>| {
        let r = pane(h, &props);
        egui::pos2(r.right() - 120.0, r.top() + 10.0)
    };
    for larger in [true, false] {
        let at = bar(&h);
        h.hover_at(at);
        for _ in 0..2 {
            press(&h, at, true, egui::Modifiers::NONE);
            press(&h, at, false, egui::Modifiers::NONE);
        }
        h.run_steps(8);
        let now = pane(&h, &props);
        assert_eq!(now.area() > first.area() + 1000.0, larger, "double-clicked: {now:?}");
        // (Later: not a third and fourth click of the same.)
        h.hover_at(at + egui::vec2(0.0, 200.0));
        h.run_steps(60);
    }
    let at = bar(&h);
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    for k in 1..=5 {
        h.hover_at(at + egui::vec2(6.0 * k as f32, 4.0 * k as f32));
        h.run_steps(1);
    }
    press(&h, at + egui::vec2(30.0, 20.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    let moved = pane(&h, &props).min - first.min;
    assert!((moved - egui::vec2(30.0, 20.0)).length() < 8.0, "dragged by {moved:?}");
    // Opened again: as it first opened (maximizing isn't remembered).
    close(&mut h);
    open(&mut h);
    let again = pane(&h, &props);
    assert!((again.size() - first.size()).length() < 2.0, "opens at {again:?}, not {first:?}");
    // Left smaller (as dragging its edge leaves it): the next opens so.
    h.state_mut().settings.window_sizes = vec![("area-properties".into(), [400, 300])];
    close(&mut h);
    open(&mut h);
    let small = pane(&h, &props);
    assert!(
        (small.width() - 400.0).abs() < 20.0 && (small.height() - 300.0).abs() < 20.0,
        "{small:?} against {first:?}"
    );
    // And once more: the size doesn't creep.
    close(&mut h);
    open(&mut h);
    let third = pane(&h, &props);
    assert!((third.size() - small.size()).length() < 2.0, "then at {third:?}");
    assert_eq!(h.state().settings.window_sizes, [("area-properties".to_string(), [400, 300])]);
}

/// A module folder's files changed by another program (a builder keeps
/// the scripts open in an editor of their own, as with Aurora) are read
/// again while the module is open, and a save never writes over them:
/// what Moonglow did not change itself stays the other program's.
#[test]
fn a_module_folder_s_files_changed_outside_are_read_again() {
    use mg_module::ModuleLocation;
    let dir = mg_testkit::scratch_dir("ui-folder-outside");
    let folder = dir.join("module");
    let mut m = Module::open(&sample_module(&dir)).unwrap();
    m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.no_mod_beside_folder = true;
    app.open_module(&folder);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let script = |h: &Harness<'_, Moonglow>, name: &str| {
        let ws = h.state().ws.as_ref().unwrap();
        ws.module.get(&ResKey::parse(name, ResType::NSS).unwrap()).map(<[u8]>::to_vec)
    };
    let said = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state().log.entries.iter().any(|(_, m)| m.contains(what))
    };
    let file = |name: &str| std::fs::read(folder.join(name)).ok();
    h.state_mut().reload_project_files();
    assert!(!said(&h, "Read again"));

    // Another editor saves a script and makes a new one. A save in
    // Moonglow before it has looked leaves both as they are.
    std::fs::write(folder.join("hello.nss"), "void main() { int outside; }\n").unwrap();
    std::fs::write(folder.join("fresh.nss"), "void main() {}\n").unwrap();
    h.state_mut().ws.as_mut().unwrap().mark_modified();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert_eq!(file("hello.nss").unwrap(), b"void main() { int outside; }\n");
    assert!(file("fresh.nss").is_some(), "a file new to the folder is not removed");
    // Looked at: both are the module's now.
    h.state_mut().reload_project_files();
    h.run_steps(2);
    assert_eq!(script(&h, "hello").unwrap(), b"void main() { int outside; }\n");
    assert!(script(&h, "fresh").is_some());
    assert!(said(&h, "Read again from the module's files"));
    assert!(h.state().outside_conflicts().is_empty());

    // Changed in Moonglow and outside both: Moonglow's is kept, the save
    // refuses to write over the file, and the window asks.
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "edit",
        vec![mg_edit::Edit::SetResource {
            key,
            data: Some(b"void main() { int ours; }\n".to_vec()),
        }],
    )));
    h.run_steps(2);
    std::fs::write(folder.join("hello.nss"), "void main() { int theirs; }\n").unwrap();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert_eq!(file("hello.nss").unwrap(), b"void main() { int theirs; }\n", "not written over");
    assert!(said(&h, "changed on disk"));
    h.state_mut().reload_project_files();
    h.run_steps(2);
    assert_eq!(h.state().outside_conflicts(), [folder.join("hello.nss")]);
    assert_eq!(script(&h, "hello").unwrap(), b"void main() { int ours; }\n");

    // A file deleted outside leaves the module.
    std::fs::remove_file(folder.join("fresh.nss")).unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(2);
    assert!(script(&h, "fresh").is_none());
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
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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

/// The view's readout names the tile under the pointer: its model, as
/// Aurora's status bar does.
#[test]
fn the_area_s_readout_names_the_tile() {
    let Some((h, area)) = area_harness("tile-name") else { return };
    let view = &h.state().area_views[&area];
    let named = view.tile_named(1, 2).expect("a tile there");
    assert!(named.starts_with("ttr01_") && named.contains(" at 1, 2"), "{named}");
    assert_eq!(view.tile_named(40, 40), None, "outside the area");
}

/// A number field dragged left and right changes its number (a builder
/// found the drag did nothing: the value was read again from the file each
/// frame of it).
#[test]
fn a_number_field_is_dragged_to_a_new_value() {
    let Some((mut h, key)) = blueprint_harness("nw_door_ttr_01", "door_drag", ResType::UTD) else {
        return;
    };
    h.run();
    h.get_by_label("Lock").click();
    h.run();
    let was = field(&mut h, &key).integer("OpenLockDC").unwrap();
    let spin =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::SpinButton;
    let at = h.get_all_by_value(&was.to_string()).find(spin).expect("the DC field").rect().center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    for k in 1..=8 {
        h.hover_at(at + egui::vec2(5.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    press(&h, at + egui::vec2(40.0, 0.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    let now = field(&mut h, &key).integer("OpenLockDC").unwrap();
    assert!(now > was, "dragged from {was} to {now}");
    // One command for the whole drag.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(field(&mut h, &key).integer("OpenLockDC"), Some(was));
}

/// A drag begun in a Properties window lying over the area's view moves
/// nothing in the area.
#[test]
fn a_drag_in_a_window_over_the_area_moves_nothing_there() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("drag-over") else { return };
    h.run_steps(40);
    let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
    }
    h.run_steps(5);
    let before = (waypoint(&mut h, area, 0), waypoint(&mut h, area, 1));
    // Where the window covers a waypoint: a drag from there.
    let covered = h
        .state()
        .dock
        .iter_leaves()
        .find(|(p, _)| !p.surface.is_main())
        .map(|(_, leaf)| leaf.rect)
        .expect("the Properties window");
    let feet = [Vec3::new(20.0, 20.0, 0.02), Vec3::new(30.0, 20.0, 0.02)];
    let from = feet.map(|p| screen(&h, area, p)).into_iter().find(|p| covered.contains(*p));
    let from = from.unwrap_or_else(|| panic!("no waypoint under {covered:?}"));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=6 {
        h.hover_at(from + egui::vec2(8.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    press(&h, from + egui::vec2(48.0, 0.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!((waypoint(&mut h, area, 0), waypoint(&mut h, area, 1)), before);
}

/// Escape closes what is in front: the Variables window over a Properties
/// window first (which a click on Properties doesn't bury), then the
/// Properties window itself.
#[test]
fn escape_closes_the_window_in_front() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("escape") else { return };
    h.run_steps(40);
    let at = screen(&h, area, Vec3::new(30.0, 20.0, 0.02));
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
    }
    h.run_steps(5);
    let path = mg_edit::GffPath::root().item("WaypointList", 1);
    let tab = Tab::Instance { area, path };
    assert!(h.state().dock.find_tab(&tab).is_some(), "the Properties tab");
    h.get_by_label("Advanced").click();
    h.run_steps(3);
    h.get_by_label_contains("Variables (").click();
    h.run_steps(3);
    assert!(h.state().var_edit.is_some(), "the Variables window");
    // A click in Properties, behind it: the Variables window stays in front.
    let props = h
        .state()
        .dock
        .iter_leaves()
        .find(|(p, _)| !p.surface.is_main())
        .map(|(_, leaf)| leaf.rect)
        .unwrap();
    let beside = props.left_bottom() + egui::vec2(12.0, -12.0);
    h.hover_at(beside);
    press(&h, beside, true, egui::Modifiers::NONE);
    press(&h, beside, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let top = h.ctx.memory(|m| m.areas().top_layer_id(egui::Order::Middle)).unwrap();
    assert_eq!(top.id, egui::Id::new(Some("Variables")), "in front of Properties");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(h.state().var_edit.is_none(), "Escape closed Variables");
    assert!(h.state().dock.find_tab(&tab).is_some(), "and only it");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(h.state().dock.find_tab(&tab).is_none(), "then Properties");
}

/// Copy… in the module tree copies an area with what is placed in it,
/// under the ResRef and Tag given, and the module lists it.
#[test]
fn an_area_is_copied_with_its_objects() {
    let Some((mut h, area)) = area_harness("copy-area") else { return };
    let key = ResKey::new(area, ResType::ARE);
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(key));
    h.run_steps(3);
    let draft = h.state_mut().copy_as.as_mut().expect("the Copy window");
    draft.resref = "start_two".into();
    draft.tag = Some("StartTwo".into());
    h.run_steps(2);
    h.get_by_label("Create Copy").click();
    h.run_steps(3);
    let new = ResRef::from_str("start_two").unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let are = ws.doc(&ResKey::new(new, ResType::ARE)).unwrap().root.clone();
    assert_eq!(are.resref("ResRef"), Some(new));
    assert_eq!(are.string("Tag"), Some(&b"StartTwo"[..]));
    let placed = |ws: &mut mg_edit::Workspace, a: ResRef| {
        let git = ws.doc(&ResKey::new(a, ResType::GIT)).unwrap();
        git.root.list("WaypointList").map_or(0, <[mg_gff::Struct]>::len)
    };
    assert_eq!(placed(ws, new), placed(ws, area));
    assert!(placed(ws, new) > 0);
    let ifo = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
    let listed: Vec<ResRef> = ifo
        .root
        .list("Mod_Area_list")
        .unwrap()
        .iter()
        .filter_map(|a| a.resref("Area_Name"))
        .collect();
    assert_eq!(listed, [area, new]);
    // A name the module has is refused.
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(key));
    h.run_steps(3);
    h.state_mut().copy_as.as_mut().unwrap().resref = "start_two".into();
    h.run_steps(2);
    assert!(h.query_by_label_contains("has a start_two already").is_some());
    // One undo takes the copy away.
    h.state_mut().copy_as = None;
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    let ws = h.state().ws.as_ref().unwrap();
    assert!(!ws.module.contains(&ResKey::new(new, ResType::ARE)));
    assert!(!ws.module.contains(&ResKey::new(new, ResType::GIT)));
}

/// A right click on the ground offers a light: one of the game's
/// invisible light placeables is put there, a little above the ground,
/// and its menu gives it another color.
#[test]
fn a_light_is_added_from_the_area_s_menu_and_recolored() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("add-light") else { return };
    h.set_size(egui::vec2(1400.0, 1000.0));
    h.run_steps(3);
    let git_key = ResKey::new(area, ResType::GIT);
    let placed = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        git.root.list("Placeable List").map(<[_]>::to_vec).unwrap_or_default()
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
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    right_click(&mut h, at);
    h.get_by_label_contains("Add Light Here").hover();
    h.run_steps(3);
    h.get_by_label("White").click();
    h.run_steps(3);
    let lights = placed(&mut h);
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].integer("Appearance"), Some(15112), "Light, White");
    let (x, z) = (lights[0].float("X").unwrap(), lights[0].float("Z").unwrap());
    assert!((x - 15.0).abs() < 0.5 && (z - 1.5).abs() < 0.3, "at {x}, up {z}");
    assert_eq!(lights[0].integer("Static"), Some(1));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add Light, White"));
    assert_eq!(h.state().area_views[&area].selection, [(mg_area::ObjectKind::Placeable, 0)]);
    h.render().unwrap().save(mg_testkit::scratch_dir("ui-add-light").join("a.png")).unwrap();
    // Its own menu: another color.
    let on_it = screen(&h, area, Vec3::new(x, lights[0].float("Y").unwrap(), z + 0.5));
    right_click(&mut h, on_it);
    h.get_by_label_contains("Light Color").hover();
    h.run_steps(3);
    h.get_by_label("Red").click();
    h.run_steps(3);
    assert_eq!(placed(&mut h)[0].integer("Appearance"), Some(15111), "Light, Red");
}

/// With a terrain brush in hand, a right click is the brush's (Raise/
/// Lower lowers) though an object lies under the pointer: its menu does
/// not open.
#[test]
fn a_right_click_with_a_terrain_brush_is_the_brush_s_over_an_object() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("brush-over-object") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("↕ Raise/Lower").click();
    h.run_steps(2);
    // (On the waypoint at 20, 20.)
    let at = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    h.hover_at(at);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    assert!(h.query_by_label("Properties").is_none(), "no object menu");
    assert!(h.query_by_label("Go To").is_none());
    assert!(h.state().palette.tile_brush.is_some(), "the brush stays in hand");
    // A right click where no ground of the area is under the pointer (off
    // its edge; a cliff face with nothing to stand on, the ray going on
    // past the area): Raise/Lower lowers nothing there, and stays in hand.
    // It was dropped without a word, and the next click was the object's.
    let off = h.state().area_views[&area].rect.left_top() + egui::vec2(12.0, 40.0);
    assert!(h.state().area_views[&area].ground_spot(off).is_none(), "off the area");
    h.hover_at(off);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: off,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    assert!(h.state().palette.tile_brush.is_some(), "Raise/Lower stays in hand");
    h.hover_at(at);
    h.run_steps(2);
    // Fast and sloppy, as when working along a slope: presses and
    // releases a few points apart, left and right, single and double,
    // some begun before the last stroke is drawn. The object under them
    // is never selected and its menu never opens.
    let sel = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].selection.clone();
    assert!(sel(&h).is_empty());
    for round in 0..24 {
        let button = if round % 2 == 0 {
            egui::PointerButton::Primary
        } else {
            egui::PointerButton::Secondary
        };
        let slip = egui::vec2((round % 5) as f32 * 2.0, (round % 3) as f32 * 2.0);
        let modifiers = egui::Modifiers::NONE;
        h.event(egui::Event::PointerButton { pos: at, button, pressed: true, modifiers });
        if round % 3 != 0 {
            h.run_steps(1);
        }
        h.event(egui::Event::PointerMoved(at + slip));
        h.event(egui::Event::PointerButton { pos: at + slip, button, pressed: false, modifiers });
        h.run_steps(if round % 4 == 0 { 1 } else { 2 });
        assert!(sel(&h).is_empty(), "round {round}: selected {:?}", sel(&h));
        assert!(h.query_by_label("Go To").is_none(), "round {round}: the object's menu");
        assert!(h.query_by_label("Properties").is_none(), "round {round}: the object's menu");
        assert!(h.state().palette.tile_brush.is_some(), "round {round}: the brush dropped");
    }
}

#[test]
#[ignore = "a look at items lying in an area (MG_ITEMS=resref,resref…)"]
fn look_items_placed() {
    use glam::Vec3;
    let items = std::env::var("MG_ITEMS")
        .unwrap_or_else(|_| "nw_it_mpotion001,nw_ashsw001,nw_wswss001,nw_cloth001".into());
    let Some((mut h, area)) = area_harness("items-placed") else { return };
    h.set_size(egui::vec2(1500.0, 1000.0));
    h.run_steps(10);
    for (i, name) in items.split(',').enumerate() {
        h.state_mut().palette.selected = ResKey::parse(name.trim(), ResType::UTI);
        let at = screen(&h, area, Vec3::new(12.0 + 1.5 * i as f32, 14.0, 0.0));
        h.hover_at(at);
        h.run_steps(2);
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        h.run_steps(4);
    }
    h.state_mut().palette.selected = None;
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    view.selection.clear();
    if let Some(o) = view.orbit.as_mut() {
        o.target = Vec3::new(15.0, 14.0, 0.3);
        (o.distance, o.yaw, o.pitch) = (4.5, -1.57, 0.6);
    }
    h.run_steps(20);
    let dir = mg_testkit::scratch_dir("ui-items-placed");
    h.render().expect("render").save(dir.join("a.png")).unwrap();
}

#[test]
#[ignore = "a count of the particles a blueprint shows in an area (MG_BP=resref.utc)"]
fn look_particles_of() {
    use glam::Vec3;
    let Ok(bp) = std::env::var("MG_BP") else { return };
    let Some((mut h, area)) = area_harness("particles-of") else { return };
    h.run_steps(10);
    let (name, ext) = bp.split_once('.').unwrap();
    let restype = if ext == "utc" { ResType::UTC } else { ResType::UTP };
    h.state_mut().palette.selected = ResKey::parse(name, restype);
    let at = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(60);
    eprintln!("PARTICLES {bp}: {}", h.state().area_views[&area].particles_shown);
}

/// A placeable that is only an effect (flames, magic sparks: models with
/// emitters and no mesh) shows its particles in the area, while the
/// animations play.
#[test]
fn effect_placeables_show_their_particles_in_the_area() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("particles") else { return };
    h.run_steps(40);
    assert_eq!(h.state().area_views[&area].particles_shown, 0);
    h.state_mut().palette.selected = ResKey::parse("plc_flamelarge", ResType::UTP);
    let at = screen(&h, area, Vec3::new(15.0, 30.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(30);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    assert_eq!(git.root.list("Placeable List").map(<[mg_gff::Struct]>::len), Some(1));
    assert!(h.state().area_views[&area].particles_shown > 0, "its particles are drawn");
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    h.hover_at(at + egui::vec2(150.0, 150.0));
    h.run_steps(200);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-area-particles").join("particles.png")).unwrap();
}

/// An area's picture is kept while nothing it shows changes (a frame is
/// drawn for every move of the pointer, and the picture is most of what a
/// frame costs): drawn again as its animations step, and at once when the
/// camera moves, the view is set to show something else, or the area is
/// edited.
#[test]
fn an_area_s_picture_is_kept_until_something_in_it_changes() {
    let Some((mut h, area)) = area_harness("picture-kept") else { return };
    h.run_steps(10);
    let pictures = |h: &Harness<'static, Moonglow>| h.state().area_views[&area].pictures;
    let mut now = h.ctx.input(|i| i.time);
    let mut frame = |h: &mut Harness<'static, Moonglow>, later: f64| {
        now += later;
        h.input_mut().time = Some(now);
        h.run_steps(1);
    };
    frame(&mut h, 0.25);
    let drawn = pictures(&h);
    // Frames a 200th of a second apart: the same picture.
    for _ in 0..5 {
        frame(&mut h, 0.005);
    }
    assert_eq!(pictures(&h), drawn, "kept from frame to frame");
    // The animations step some 25 times a second.
    frame(&mut h, 0.03);
    assert_eq!(pictures(&h), drawn + 1, "drawn as the animations step");
    // The camera turned: at once.
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw += 0.1;
    frame(&mut h, 0.005);
    assert_eq!(pictures(&h), drawn + 2, "drawn as the camera moves");
    // By night.
    h.state_mut().area_views.get_mut(&area).unwrap().night ^= true;
    frame(&mut h, 0.005);
    assert_eq!(pictures(&h), drawn + 3, "drawn as the view changes");
    // The pointer over it, with nothing to place: the same picture.
    let waypoint = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(waypoint);
    frame(&mut h, 0.005);
    h.hover_at(waypoint + egui::vec2(3.0, 0.0));
    frame(&mut h, 0.005);
    assert_eq!(pictures(&h), drawn + 3, "kept under the pointer");
    // The area edited (a waypoint deleted).
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Waypoint, 0)];
    h.key_press(egui::Key::Delete);
    frame(&mut h, 0.005);
    frame(&mut h, 0.005);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    assert_eq!(git.root.list("WaypointList").map(<[mg_gff::Struct]>::len), Some(1));
    assert!(pictures(&h) > drawn + 3, "drawn as the area changes");
    let drawn = pictures(&h);
    for _ in 0..3 {
        frame(&mut h, 0.005);
    }
    assert_eq!(pictures(&h), drawn, "and kept again");
}

/// With Shift held, a selected placeable has a handle that scales its
/// model about its feet: pulled twice as far from the object, twice the
/// size, in one undoable step; Escape drops the drag.
#[test]
fn a_scale_handle_scales_a_placeable_s_model_while_shift_is_held() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("scale-handle") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    let pivot = Vec3::new(12.0, 20.0, 0.0);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [pivot.x, pivot.y, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    h.run_steps(3);
    let scale = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let chest = &git.root.list("Placeable List").unwrap()[0];
        mg_area::VisualTransform::read(chest).map(|v| v.scale)
    };
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(ObjectKind::Placeable, 0)];
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.yaw, o.pitch, o.distance) = (pivot, 0.3, 0.5, 14.0);
    }
    h.run_steps(2);
    // The handle: up and to the camera's right of the chest's feet.
    let camera = h.state().area_views[&area].orbit.unwrap().camera();
    let radius = ((camera.eye - pivot).length() * 0.0765).max(0.5);
    let right = (camera.target - camera.eye).cross(Vec3::Z).normalize();
    let handle = screen(&h, area, pivot + (right + Vec3::Z).normalize() * radius * 1.25);
    let middle = screen(&h, area, pivot);
    let shift = egui::Modifiers::SHIFT;
    let left = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: shift,
    };
    // Pulled to `times` as far from the chest; let go, or dropped.
    let pull = |h: &mut Harness<'_, Moonglow>, times: f32, escape: bool, shot: bool| {
        h.event(egui::Event::ModifiersChanged(shift));
        h.hover_at(handle);
        h.run_steps(1);
        h.event(left(handle, true));
        let to = middle + (handle - middle) * times;
        for k in 1..=6 {
            h.hover_at(handle + (to - handle) * (k as f32 / 6.0));
            h.run_steps(1);
        }
        if shot {
            let img = h.render().expect("render");
            img.save(mg_testkit::scratch_dir("ui-area-scale").join("scale_handle.png")).unwrap();
        }
        if escape {
            h.key_press(egui::Key::Escape);
            h.run_steps(1);
        }
        h.event(left(to, false));
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };
    assert_eq!(scale(&mut h), None, "no visual transform yet");
    pull(&mut h, 2.0, true, false);
    assert_eq!(scale(&mut h), None, "Escape dropped it");
    pull(&mut h, 2.0, false, true);
    let twice = scale(&mut h).expect("a visual transform");
    assert!((twice.x - 2.0).abs() < 0.05, "scaled {twice:?}");
    assert_eq!((twice.y, twice.z), (twice.x, twice.x), "the same every way");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Scale"));
    // Pushed halfway in: half of that, from what it has.
    pull(&mut h, 0.5, false, false);
    let back = scale(&mut h).unwrap();
    assert!((back.x - 1.0).abs() < 0.05, "scaled back {back:?}");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(scale(&mut h), None, "an undo each");
}

/// A look (`MG_SHOT=1`): a dragon selected in an area, its box around it
/// as it stands.
#[test]
fn a_dragon_s_box_in_an_area() {
    use mg_module::instances::{Placement, Placing, instance};
    if std::env::var_os("MG_SHOT").is_none() {
        return;
    }
    let Some((mut h, area)) = area_harness("dragon-box") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("nw_drgred001", ResType::UTC).unwrap();
        let dragon = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [20.0, 20.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTC, &dragon, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Creature List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    h.run_steps(3);
    {
        let view = h.state_mut().area_views.get_mut(&area).unwrap();
        view.selection = vec![(mg_area::ObjectKind::Creature, 0)];
        let o = view.orbit.as_mut().unwrap();
        (o.target, o.yaw, o.pitch, o.distance) =
            (glam::Vec3::new(20.0, 20.0, 2.0), 0.6, 0.45, 28.0);
    }
    h.run_steps(6);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-dragon-box").join("dragon.png")).unwrap();
}

/// Ctrl + wheel over the view scales the selected placeable's model, as
/// Aurora's does: shown as the wheel turns, one command when Ctrl is let
/// go, and Undo takes it back whole (or drops it while it is under way).
/// A static placeable takes no scale, and with nothing selected that
/// does the wheel zooms, as it did.
#[test]
fn ctrl_and_the_wheel_scale_what_is_selected() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("scale-wheel") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    let pivot = Vec3::new(12.0, 20.0, 0.0);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [pivot.x, pivot.y, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    h.run_steps(3);
    let scale = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        let chest = &git.root.list("Placeable List").unwrap()[0];
        mg_area::VisualTransform::read(chest).map(|v| v.scale.x)
    };
    let distance = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().distance;
    let ctrl = egui::Modifiers::COMMAND;
    // The wheel turned with Ctrl held (and still held after).
    let turn = |h: &mut Harness<'_, Moonglow>, by: f32| {
        h.event(egui::Event::ModifiersChanged(ctrl));
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, by),
            phase: egui::TouchPhase::Move,
            modifiers: ctrl,
        });
        h.run_steps(12);
    };
    let let_go = |h: &mut Harness<'_, Moonglow>| {
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(3);
    };
    let over = screen(&h, area, pivot);
    h.hover_at(over);
    h.run_steps(2);
    // Nothing selected: the wheel zooms, slowly.
    let far = distance(&h);
    turn(&mut h, 200.0);
    let_go(&mut h);
    assert!(distance(&h) < far, "zoomed in");
    assert_eq!(scale(&mut h), None);
    // The chest selected: it grows as the wheel turns, shown before it is
    // a command, and the view stays where it is.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    h.run_steps(2);
    let far = distance(&h);
    turn(&mut h, 200.0);
    assert_eq!(scale(&mut h), None, "not a command while Ctrl is held");
    assert!(h.query_by_label_contains("Scale 1.").is_some(), "the status line says how large");
    turn(&mut h, 200.0);
    let_go(&mut h);
    let grown = scale(&mut h).expect("a visual transform");
    assert!(grown > 1.2 && grown < 2.0, "scaled {grown}");
    assert_eq!(distance(&h), far, "the wheel was the chest's, not the camera's");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Scale"));
    // One undo for the wheel's whole turn.
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(scale(&mut h), None, "one undo");
    // Turned the other way: smaller. Left to rest, it is a command too.
    turn(&mut h, -200.0);
    h.run_steps(40);
    let shrunk = scale(&mut h).expect("made a command when the wheel rests");
    assert!(shrunk < 0.9, "scaled {shrunk}");
    let_go(&mut h);
    // Saved while a turn is under way: the turn is part of what is saved.
    let_go(&mut h);
    turn(&mut h, 200.0);
    // (The wheel's turning is smoothed over some frames: all of it in.)
    h.run_steps(8);
    assert!(h.state().area_views[&area].wheel_scaling(), "still under way");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert!(!h.state().area_views[&area].wheel_scaling(), "made a command by the save");
    let saved = scale(&mut h).expect("a command before the save");
    assert!(saved > shrunk, "scaled {saved}");
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "and saved");
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run_steps(3);
    let_go(&mut h);
    assert_eq!(scale(&mut h), Some(shrunk));
    // Undo while a turn is under way drops that, and no more.
    turn(&mut h, 200.0);
    h.key_press_modifiers(ctrl, egui::Key::Z);
    h.run_steps(3);
    let_go(&mut h);
    assert_eq!(scale(&mut h), Some(shrunk), "the turn under way was dropped");
    // A static placeable takes no scale: the wheel zooms, and the status
    // line says why.
    let set = mg_edit::Edit::SetField {
        key: git_key,
        path: mg_edit::GffPath::root().item("Placeable List", 0),
        label: "Static".into(),
        value: Some(mg_gff::Value::Byte(1)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Static", vec![set])));
    h.run_steps(3);
    let far = distance(&h);
    turn(&mut h, 200.0);
    let_go(&mut h);
    assert!(distance(&h) < far, "zoomed");
    assert!(h.query_by_label_contains("static placeable takes no scale").is_some());
}

/// The palette's Gallery shows a category's blueprints as pictures: a
/// click on one chooses it to place, as a click on its name does.
#[test]
fn the_palette_s_gallery_shows_blueprints_as_pictures() {
    let Some((mut h, _area)) = area_harness("gallery") else { return };
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Placeable;
        (p.tiles, p.custom) = (false, false);
        p.filter = "chest".into();
    }
    h.state_mut().settings.palette_gallery = true;
    // (A few are made a frame.)
    h.run_steps(30);
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let name = {
        let game = h.state().game.as_deref().unwrap();
        let utp = Gff::read(&game.resman.get(&chest).unwrap()).unwrap();
        game.locstring(utp.root.locstring("LocName").unwrap()).unwrap()
    };
    let tile = h.get_all_by_label(&name).next().expect("the chest's picture");
    let size = tile.rect().size();
    assert!(size.x > 80.0 && size.y > size.x, "a picture over a name: {size:?}");
    tile.click();
    h.run_steps(3);
    assert!(h.state().palette.selected.is_some(), "chosen, to place");
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-gallery").join("gallery.png")).unwrap();
    // List goes back to the names, and Gallery to the pictures.
    h.get_by_label("☰").click();
    h.run_steps(3);
    assert!(!h.state().settings.palette_gallery);
    let row = h.get_all_by_label(&name).next().expect("the chest's row").rect().size();
    assert!(row.y < 30.0, "a row of the list: {row:?}");
    h.get_by_label("⊞").click();
    h.run_steps(3);
    assert!(h.state().settings.palette_gallery);
}

/// A gallery's pictures are made a few a frame and kept while they are in
/// sight, however many: more of them than are kept out of sight aren't let
/// go and made again frame after frame. An edit that leaves the blueprints
/// looking as they did draws none of them anew; and those gone out of
/// sight go, down to how many are kept.
#[test]
fn a_gallery_s_pictures_are_made_once_and_kept_while_in_sight() {
    let Some((mut h, area)) = area_harness("gallery-kept") else { return };
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Placeable;
        (p.tiles, p.custom) = (false, false);
        p.filter = "a".into();
    }
    h.state_mut().settings.palette_gallery = true;
    h.state_mut().settings.gallery_tile = Some(64);
    // (Few kept out of sight, for the test: fewer than are in sight.)
    h.state_mut().thumbnails.keep = Some(4);
    let settle = |h: &mut Harness<'static, Moonglow>| {
        for _ in 0..400 {
            h.run_steps(1);
            if h.state().thumbnails.waiting() == 0 {
                break;
            }
        }
        h.run_steps(2);
        assert_eq!(h.state().thumbnails.waiting(), 0, "every picture in sight is made");
    };
    settle(&mut h);
    let (kept, drawn) = (h.state().thumbnails.kept(), h.state().thumbnails.drawn);
    assert!(kept > 4, "all those in sight are kept: {kept}");
    assert_eq!(drawn, kept as u64, "each drawn once");
    h.run_steps(10);
    assert_eq!(h.state().thumbnails.drawn, drawn, "none drawn again");
    assert_eq!(h.state().thumbnails.kept(), kept);
    // An edit elsewhere (a waypoint's tag): the pictures are looked at
    // again, found to be of what the blueprints still look like, and kept.
    let set = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::GIT),
        path: mg_edit::GffPath::root().item("WaypointList", 0),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String("moved".into())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![set])));
    h.run_steps(2);
    settle(&mut h);
    assert_eq!(h.state().thumbnails.drawn, drawn, "none drawn anew after an edit");
    // Other pictures in sight: the first ones, out of sight, go.
    h.state_mut().palette.filter = "chest".into();
    settle(&mut h);
    h.run_steps(3);
    let (now, more) = (h.state().thumbnails.kept(), h.state().thumbnails.drawn - drawn);
    assert!(more > 0, "the chests' pictures are drawn");
    assert!(
        (now as u64) < kept as u64 + more && now as u64 <= more + 4,
        "those out of sight are let go, but for four: {now} kept of {kept} and {more} more"
    );
}

/// Replace Selected with This, on a blueprint in the palette: the selected
/// object of its type becomes one of that blueprint, where it stands; the
/// others stay; one undo.
#[test]
fn the_palette_replaces_the_selected_object() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("replace") else { return };
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    {
        let app = h.state_mut();
        app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.palette.filter = "nw_wp_tavern".into();
        app.area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    }
    h.run_steps(5);
    let read = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let w = &git.root.list("WaypointList").unwrap()[i];
        (
            w.resref("TemplateResRef").map(|r| r.to_string()),
            w.float("XPosition"),
            w.float("YPosition"),
        )
    };
    let (first, second) = (read(&mut h, 0), read(&mut h, 1));
    h.get_by_label(&tavern).click_secondary();
    h.run_steps(2);
    h.get_by_label("Replace Selected with This").click();
    h.run_steps(3);
    let now = read(&mut h, 0);
    assert_eq!(now.0.as_deref(), Some("nw_wp_tavern"));
    assert_eq!((now.1, now.2), (first.1, first.2), "where it stood");
    assert_eq!(read(&mut h, 1), second, "the other is untouched");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(read(&mut h, 0), first, "one undo");
}

/// A placeable's Properties open a gallery of every appearance in
/// placeables.2da at its own: a click on a neighbour gives it that one.
#[test]
fn a_placeable_s_appearance_is_chosen_from_pictures() {
    let Some((mut h, _area)) = area_harness("appearances") else { return };
    let key = ResKey::parse("my_chest", ResType::UTP).unwrap();
    {
        let app = h.state_mut();
        let stock = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let data = app.game.as_deref().unwrap().resman.get(&stock).unwrap().into_owned();
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "copy",
            vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
        )));
        app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    }
    h.run_steps(5);
    let was = field(&mut h, &key).integer("Appearance").unwrap();
    h.get_by_label("Gallery…").click();
    h.run_steps(30);
    // The row after its own, by its name in the table.
    let next = {
        let game = h.state().game.as_deref().unwrap();
        let cols = mg_rules::ChoiceColumns { name: Some("StrRef"), label: Some("Label") };
        let all = game.choices("placeables", cols).unwrap();
        let at = all.iter().position(|c| c.row as i64 == was).unwrap();
        all[at + 1].clone()
    };
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-appearances").join("appearances.png")).unwrap();
    let tile = |n: &egui_kittest::Node<'_>| n.rect().height() > 90.0;
    h.get_all_by_label(&next.text).find(tile).expect("the next appearance's picture").click();
    h.run_steps(3);
    assert_eq!(field(&mut h, &key).integer("Appearance"), Some(next.row as i64));
    assert!(h.state().dock.find_tab(&Tab::PlaceableGallery).is_some(), "the gallery stays, a tab");
    assert!(h.query_by_label_contains("A click gives my_chest.utp the appearance").is_some());
}

/// The Placeable Gallery (Tools, and the palette's All Appearances…) is
/// open without a placeable's Properties: a click on a picture gives its
/// appearance to the placeables selected in the area. Models that are
/// only an effect have pictures too.
#[test]
fn the_placeable_gallery_gives_the_selection_an_appearance() {
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("placeable-gallery") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [12.0, 20.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
        // (A small window, so some of the area shows beside it.)
        app.settings.window_sizes = vec![("placeable-gallery".into(), [330, 430])];
        app.actions.push(mg_ui::Action::PlaceableGallery);
    }
    h.run_steps(5);
    assert!(h.query_by_label_contains("Select placeables in an area").is_some());
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    h.state_mut().placeable_gallery.as_mut().unwrap().filter = "Flame".into();
    h.run_steps(40);
    assert!(h.query_by_label_contains("1 placeable selected in the area").is_some());
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-placeable-gallery").join("gallery.png")).unwrap();
    let appearance = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        git.root.list("Placeable List").unwrap()[0].integer("Appearance").unwrap()
    };
    let was = appearance(&mut h);
    let tile = |n: &egui_kittest::Node<'_>| n.rect().height() > 90.0;
    let flame = h.get_all_by_label("Flame").find(tile).expect("a flame's picture");
    flame.click();
    h.run_steps(3);
    let now = appearance(&mut h);
    assert_ne!(now, was, "the chest is a flame");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Appearance"));
    // Dragged into the area's view (beside the gallery's window), a picture
    // places a placeable of that appearance there.
    let count = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap().len()
    };
    let from = h.get_all_by_label("Flame").find(tile).unwrap().rect().center();
    let window = h
        .state()
        .dock
        .iter_leaves()
        .find(|(p, _)| !p.surface.is_main())
        .map(|(_, leaf)| leaf.rect)
        .unwrap();
    // (In the area's pane, clear of its toolbar.)
    let at_area = h.state().dock.find_tab(&Tab::Area(area)).unwrap();
    let pane = h
        .state()
        .dock
        .iter_leaves()
        .find(|(p, _)| p.surface == at_area.surface && p.node == at_area.node)
        .map(|(_, leaf)| leaf.viewport)
        .unwrap();
    let pane = egui::Rect::from_min_max(
        pane.min + egui::vec2(10.0, 200.0),
        pane.max - egui::vec2(10.0, 10.0),
    );
    let spots = (0..=20).flat_map(|x| (0..=20).map(move |y| (x as f32 * 2.0, y as f32 * 2.0)));
    let to = spots
        .map(|(x, y)| screen(&h, area, glam::Vec3::new(x, y, 0.0)))
        .find(|p| !window.expand(8.0).contains(*p) && pane.contains(*p))
        .expect("ground beside the gallery");
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for k in 1..=6 {
        h.hover_at(from + (to - from) * (k as f32 / 6.0));
        h.run_steps(1);
    }
    // Over the area it shows, see-through, where it would go.
    let ghost = h.state().area_views[&area].ghost_shown.clone().expect("its ghost");
    assert_eq!(ghost.kind, ObjectKind::Placeable);
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(count(&mut h), 2, "a second placeable");
    let ws = h.state_mut().ws.as_mut().unwrap();
    let placed = ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[1].clone();
    assert_eq!(placed.integer("Appearance"), Some(now), "of the flame dragged");
    assert_eq!(placed.integer("Static"), Some(1));
}

#[test]
#[ignore = "a look at some appearances' pictures"]
fn look_gallery_pictures() {
    let Some((mut h, _area)) = area_harness("gallery-look") else { return };
    h.state_mut().settings.gallery_tile = Some(220);
    h.state_mut().actions.push(mg_ui::Action::PlaceableGallery);
    h.run_steps(5);
    h.state_mut().placeable_gallery.as_mut().unwrap().filter =
        std::env::var("MG_FILTER").unwrap_or_default();
    h.run_steps(60);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-gallery-look").join("look.png")).unwrap();
}

#[test]
#[ignore = "a look at the Creature Wizard's Appearance page"]
fn look_creature_wizard_appearance() {
    let Some((mut h, _area)) = area_harness("cw-look") else { return };
    let mut w = mg_ui::creature_wizard::CreatureWizard::default();
    (w.page, w.race, w.appearance) = (3, Some(6), 6);
    w.classes = vec![(4, 1)];
    w.appearance_find = std::env::var("MG_FILTER").unwrap_or_default();
    h.state_mut().creature_wizard = Some(w);
    h.run_steps(40);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-cw-look").join("look.png")).unwrap();
}

/// Between objects and tiles without the toolbar: a double click on the
/// ground selects its tile (Select Tiles comes on), and a click on an
/// object there selects it (and Select Tiles goes off).
#[test]
fn a_click_goes_between_objects_and_tiles() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tiles-and-objects") else { return };
    h.run_steps(40);
    let ground = screen(&h, area, Vec3::new(5.0, 5.0, 0.0));
    h.hover_at(ground);
    for _ in 0..2 {
        press(&h, ground, true, egui::Modifiers::NONE);
        press(&h, ground, false, egui::Modifiers::NONE);
    }
    h.run_steps(3);
    let view = &h.state().area_views[&area];
    assert!(view.tile_mode, "over to the tiles");
    assert_eq!(view.tile_selection, [(0, 0)]);
    // (Later: not a third click of the same.)
    h.run_steps(60);
    let flag = screen(&h, area, Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(flag);
    press(&h, flag, true, egui::Modifiers::NONE);
    press(&h, flag, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let view = &h.state().area_views[&area];
    assert!(!view.tile_mode, "back to the objects");
    assert_eq!(view.selection, [(ObjectKind::Waypoint, 0)]);
    assert!(view.tile_selection.is_empty());
}

#[test]
#[ignore = "a look at a game's item, viewed: its parts as pictures, read-only"]
fn look_viewed_item() {
    let Some((mut h, _area)) = area_harness("viewed-item-look") else { return };
    let name = std::env::var("MG_ITEM").unwrap_or_else(|_| "nw_wswls001".into());
    if std::env::var("MG_SCREEN").is_ok() {
        h.set_size(egui::vec2(1920.0, 1080.0));
        h.run_steps(3);
    }
    if let Ok(w) = std::env::var("MG_WIDTH") {
        let w: u32 = w.parse().unwrap();
        h.set_size(egui::vec2(w as f32 + 500.0, 900.0));
        h.state_mut().settings.window_sizes.push(("item".into(), [w, 700]));
        h.run_steps(3);
    }
    let key = ResKey::parse(&name, ResType::UTI).unwrap();
    {
        let app = h.state_mut();
        let data = app.game.as_ref().unwrap().resman.get(&key).unwrap();
        app.ws.as_mut().unwrap().view(key, mg_gff::Gff::read(&data).unwrap());
        app.blueprint_pages.insert((key, mg_edit::GffPath::root()), "Appearance");
        app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    }
    h.run_steps(20);
    if std::env::var("MG_FEMALE").is_ok() {
        h.get_by_label("Female").click();
        h.run_steps(10);
    }
    if let Ok(w) = std::env::var("MG_RESIZE") {
        let path = h.state().dock.find_tab(&Tab::Blueprint(key)).unwrap();
        let state = h.state_mut().dock.get_window_state_mut(path.surface).unwrap();
        state.set_size(egui::vec2(w.parse().unwrap(), 700.0));
        h.run_steps(20);
    }
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-viewed-item-look").join("look.png")).unwrap();
}

#[test]
#[ignore = "a look at the palette's Gallery of waypoints, by their flags"]
fn look_marker_gallery() {
    let Some((mut h, _area)) = area_harness("marker-gallery-look") else { return };
    h.state_mut().settings.palette_gallery = true;
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Waypoint;
        (p.tiles, p.custom) = (false, false);
        p.filter = std::env::var("MG_FILTER").unwrap_or_else(|_| "a".into());
    }
    h.run_steps(60);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-marker-gallery-look").join("waypoints.png")).unwrap();
}

/// An item's Appearance page shows its model in the page, over the fields
/// in a window too narrow to have it beside them.
#[test]
fn an_item_s_appearance_page_shows_its_model() {
    let Some((mut h, _area)) = area_harness("item-model") else { return };
    let key = ResKey::parse("nw_wswls001", ResType::UTI).unwrap();
    {
        let app = h.state_mut();
        let data = app.game.as_ref().unwrap().resman.get(&key).unwrap();
        app.ws.as_mut().unwrap().view(key, mg_gff::Gff::read(&data).unwrap());
        app.blueprint_pages.insert((key, mg_edit::GffPath::root()), "Appearance");
        app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    }
    h.run_steps(10);
    // (The viewer's own buttons: it is there, and has the game's data.)
    assert!(h.query_by_label("Pop Out").is_some());
    assert!(h.query_by_label_contains("No game data").is_none());
    // Each of the sword's parts as pictures.
    assert!(h.query_all_by_label_contains("Top ").count() > 1);
}

/// Properties on an item in a placed object's inventory opens that item's
/// own Properties (the item as the object holds it), as in Aurora; what is
/// changed there is changed in the object's inventory.
#[test]
fn a_held_item_s_properties_open_from_the_inventory() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("held-item") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let chest = {
        let game = h.state().game.clone().unwrap();
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let at = Placement { position: [12.0, 12.0, 0.0], rotation: 0.0 };
        let mut chest =
            instance(&placing, ResType::UTP, &read("plc_chest1", ResType::UTP), at, &[]).unwrap();
        let mut sword = read("nw_wswls001", ResType::UTI);
        sword.set("Repos_PosX", mg_gff::Value::Word(0));
        sword.set("Repos_Posy", mg_gff::Value::Word(0));
        chest.set("HasInventory", mg_gff::Value::Byte(1));
        chest.set("ItemList", mg_gff::Value::List(vec![sword]));
        chest
    };
    let placed = Edit::InsertItem {
        key: git,
        path: GffPath::root(),
        list: "Placeable List".into(),
        index: 0,
        item: chest,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    let path = GffPath::root().item("Placeable List", 0);
    h.state_mut().blueprint_pages.insert((git, path.clone()), "Inventory");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path: path.clone() }));
    h.run_steps(6);
    // (The one held, right of the one in the palette beside it.)
    let row = h
        .query_all_by_label("Longsword")
        .map(|n| n.rect().center())
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    h.hover_at(row);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Properties").click();
    h.run_steps(6);
    let held = path.item("ItemList", 0);
    let tab = Tab::Instance { area, path: held.clone() };
    assert!(h.state().dock.find_tab(&tab).is_some(), "{:?}", h.state().log.entries);
    // An item's pages, for the item the chest holds.
    assert!(h.query_all_by_label("Appearance").count() >= 1);
    // Changed there, it is changed in the chest.
    let set = Edit::SetField {
        key: git,
        path: held.clone(),
        label: "Charges".into(),
        value: Some(mg_gff::Value::Byte(7)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Charges", vec![set])));
    h.run_steps(4);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let item = held.get(&ws.doc(&git).unwrap().root).cloned().unwrap();
    assert_eq!(item.integer("Charges"), Some(7));
    assert!(item.integer("Cost").is_some());
}

/// Shown on a woman, an armor and a cloak are her models, and the armor's
/// inventory icon the one her inventory shows.
#[test]
fn armor_and_cloaks_are_shown_on_a_woman_when_asked() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let game = mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let read = |name: &str| {
        Gff::read(&game.resman.get(&ResKey::parse(name, ResType::UTI).unwrap()).unwrap())
            .unwrap()
            .root
    };
    let cloak = read("nw_maarcl055");
    let model = |female| mg_preview::item_on(&game, &cloak, female).unwrap().base.model;
    assert!(model(false).starts_with("pmh0_cloak_"), "{}", model(false));
    assert!(model(true).starts_with("pfh0_cloak_"), "{}", model(true));
    let armor = read("nw_aarcl001");
    let parts = |female| {
        let p = mg_preview::item_on(&game, &armor, female).unwrap();
        std::iter::once(p.base.model)
            .chain(p.parts.into_iter().map(|p| p.model))
            .collect::<Vec<_>>()
    };
    assert!(parts(false).iter().any(|m| m.starts_with("pmh0_")));
    assert!(parts(true).iter().any(|m| m.starts_with("pfh0_")), "{:?}", parts(true));
    assert!(!parts(true).iter().any(|m| m.starts_with("pmh0_chest")));
}

#[test]
#[ignore = "a look at the grid from the area's edge, low"]
fn look_grid_from_the_edge() {
    let Some((mut h, area)) = area_harness("grid-look") else { return };
    let dir = mg_testkit::scratch_dir("ui-grid-look");
    for (name, pitch) in [("low", 0.08), ("mid", 0.4), ("high", 1.2)] {
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        (o.yaw, o.pitch, o.distance) = (-1.57, pitch, 45.0);
        h.run_steps(5);
        let img = h.render().expect("render");
        img.save(dir.join(format!("{name}.png"))).unwrap();
    }
}

#[test]
#[ignore = "a look at a wide fog placeable in the area view"]
fn look_fog_placeable() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("fog-look") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let row: u32 = std::env::var("MG_ROW").ok().and_then(|r| r.parse().ok()).unwrap_or(15601);
    let fog = {
        let game = h.state().game.clone().unwrap();
        let key = ResKey::parse("x3_plc_mist", ResType::UTP).unwrap();
        let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let at = Placement { position: [20.0, 20.0, 0.0], rotation: 0.0 };
        let mut fog = instance(&placing, ResType::UTP, &bp, at, &[]).unwrap();
        fog.set("Appearance", mg_gff::Value::Dword(row));
        fog.set("Static", mg_gff::Value::Byte(1));
        fog
    };
    let placed = Edit::InsertItem {
        key: git,
        path: GffPath::root(),
        list: "Placeable List".into(),
        index: 0,
        item: fog,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    h.run_steps(240);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-fog-look").join("look.png")).unwrap();
}

/// The arrow keys step through a list of choices once it is clicked, as
/// in Aurora: here a placeable's Appearance.
#[test]
fn arrow_keys_step_through_a_list_of_choices() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "arrow_copy", ResType::UTP) else {
        return;
    };
    h.run();
    let appearance = |h: &mut Harness<'_, Moonglow>| field(h, &key).integer("Appearance");
    let before = appearance(&mut h);
    let combo = egui::accesskit::Role::ComboBox;
    h.get_all_by_role(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Chest"))
        .expect("the Appearance list")
        .click();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    let after = appearance(&mut h);
    assert_ne!(after, before, "the next appearance");
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(appearance(&mut h), before, "and back");
    // The pointer resting on the list's box shows the chosen one's picture.
    h.key_press(egui::Key::Escape);
    h.run();
    let pictures = |h: &Harness<'_, Moonglow>| {
        h.query_all_by_role(egui::accesskit::Role::Image).count()
            + h.query_all_by_label("(no picture)").count()
    };
    let without = pictures(&h);
    let at = h
        .get_all_by_role(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Chest"))
        .unwrap()
        .rect()
        .center();
    h.hover_at(at);
    h.run_steps(90);
    assert_eq!(pictures(&h), without + 1, "the chosen appearance's picture");
}

#[test]
#[ignore = "a look at a builder's tileset group (MG_TEST_USER, MG_TILESET, MG_GROUP)"]
fn look_custom_group() {
    use glam::Vec3;
    let tileset = std::env::var("MG_TILESET").unwrap_or_else(|_| "testing".into());
    let group = std::env::var("MG_GROUP").unwrap_or_else(|_| "Volcano - Path 1".into());
    let Some((mut h, area)) = area_harness_on("custom-group-look", &tileset, None) else { return };
    h.set_size(egui::vec2(1500.0, 1000.0));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    // (The palette's own Expand All: the one to the right.)
    let expand = h
        .query_all_by_label("Expand All")
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .unwrap()
        .rect()
        .center();
    h.hover_at(expand);
    press(&h, expand, true, egui::Modifiers::NONE);
    press(&h, expand, false, egui::Modifiers::NONE);
    h.run_steps(3);
    h.get_by_label(&group).click();
    h.run_steps(2);
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(5);
    h.key_press(egui::Key::Escape);
    {
        let o = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap();
        o.pitch = std::env::var("MG_PITCH").ok().and_then(|p| p.parse().ok()).unwrap_or(0.9);
    }
    let dir = mg_testkit::scratch_dir("ui-custom-group-look");
    for (name, steps) in [("a", 30), ("b", 240)] {
        h.run_steps(steps);
        h.render().expect("render").save(dir.join(format!("{name}.png"))).unwrap();
    }
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    eprintln!("LOG {}", log.join(" | ").chars().take(600).collect::<String>());
}

#[test]
#[ignore = "a look at an area of a module with a user folder's haks (MG_MODULE, MG_AREA, MG_TEST_USER)"]
fn look_module_area() {
    let (Ok(module), Ok(area)) = (std::env::var("MG_MODULE"), std::env::var("MG_AREA")) else {
        return;
    };
    let Some(root) = mg_testkit::nwn_root() else { return };
    mg_testkit::gpu::hold();
    if mg_render::Gpu::headless().is_none() {
        return;
    }
    let user = std::env::var_os("MG_TEST_USER").map(std::path::PathBuf::from);
    let install = mg_resman::GameInstall::new(&root, user, "en");
    let area = ResRef::from_str(&area).unwrap();
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(std::path::Path::new(&module));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1500.0, 1000.0))
        .with_step_dt(1.0 / 60.0)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(40);
    // MG_AT: "x,y,z,distance,yaw,pitch" to look at one place.
    if let Ok(at) = std::env::var("MG_AT") {
        let v: Vec<f32> = at.split(',').filter_map(|t| t.trim().parse().ok()).collect();
        if let (Some(view), [x, y, z, distance, yaw, pitch]) =
            (h.state_mut().area_views.get_mut(&area), v.as_slice())
            && let Some(o) = view.orbit.as_mut()
        {
            o.target = glam::Vec3::new(*x, *y, *z);
            (o.distance, o.yaw, o.pitch) = (*distance, *yaw, *pitch);
        }
        h.run_steps(10);
    }
    let dir = mg_testkit::scratch_dir("ui-module-area-look");
    h.render().expect("render").save(dir.join("a.png")).unwrap();
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    eprintln!("LOG {}", log.join(" | ").chars().take(900).collect::<String>());
}

/// A plugin asks for a folder and for leave to write a hak in its job's
/// window; the hak is written into the user folder and the module lists
/// it.
#[test]
fn a_plugin_reads_a_chosen_folder_and_writes_a_hak() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-plugin-outside");
    let path = sample_module(&dir);
    let (user, export) = (dir.join("user"), dir.join("export"));
    std::fs::create_dir_all(&user).unwrap();
    std::fs::create_dir_all(&export).unwrap();
    std::fs::write(export.join("note.txt"), "from the folder").unwrap();
    let dialogs = NoDialogs { folders: vec![export], ..Default::default() };
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    app.open_module(&path);
    app.open_palette = false;
    app.background_jobs = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().plugins.console = r#"
        local folder = ctx.ui:open_folder({ title = "The folder to take" })
        folder:to_hak("made", "note.txt")
        ctx.hak:attach("made")
    "#
    .into();
    h.state_mut().run_console();
    let wait_for = |h: &mut Harness<'_, Moonglow>, label: &str| {
        let started = std::time::Instant::now();
        while h.query_by_label(label).is_none() {
            h.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(5));
            assert!(started.elapsed() < std::time::Duration::from_secs(20), "no {label:?}");
        }
        h.run_steps(3);
    };
    wait_for(&mut h, "Choose Folder…");
    h.get_by_label("The folder to take");
    h.get_by_label("Choose Folder…").click();
    // Then its leave to write the hak, in the API's own words.
    wait_for(&mut h, "Yes");
    h.get_by_label_contains("made.hak in your hak folder");
    h.get_by_label("Yes").click();
    let started = std::time::Instant::now();
    while h.state().busy() {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "the job never ended");
    }
    h.run_steps(2);
    let hak = mg_module::hak_edit::Hak::open(&user.join("hak/made.hak")).unwrap();
    assert_eq!(hak.data(ResKey::parse("note", ResType::TXT).unwrap()).unwrap(), b"from the folder");
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    assert_eq!(ws.module.haks().unwrap(), ["made"]);
    assert!(
        h.state().log.entries.iter().any(|(_, m)| m.contains("put 1 resources into")),
        "{:?}",
        h.state().log.entries
    );
}

/// The Delete key on a row of the module tree asks as its menu's Delete…
/// does.
#[test]
fn delete_on_a_tree_row_asks_first() {
    let Some((mut h, key)) = plugin_harness("ui-tree-delete-key", false) else { return };
    h.run();
    // The guard's row (its type opened out), under the pointer.
    h.get_by_label("Expand All").click();
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

/// A script's copy takes its compiled script along.
#[test]
fn a_script_is_copied_with_its_compiled_script() {
    let Some((mut h, _)) = plugin_harness("ui-copy-script", false) else { return };
    let (nss, ncs) = (
        ResKey::parse("greet", ResType::NSS).unwrap(),
        ResKey::parse("greet", ResType::NCS).unwrap(),
    );
    let edits = vec![
        mg_edit::Edit::SetResource { key: nss, data: Some(b"void main() { }\n".to_vec()) },
        mg_edit::Edit::SetResource { key: ncs, data: Some(b"NCS V1.0 compiled".to_vec()) },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(nss));
    h.run_steps(3);
    h.state_mut().copy_as.as_mut().expect("the Copy window").resref = "greet_two".into();
    h.run_steps(2);
    h.get_by_label("Create Copy").click();
    h.run_steps(3);
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    let copy = |t| ws.module.get(&ResKey::parse("greet_two", t).unwrap()).map(<[u8]>::to_vec);
    assert_eq!(copy(ResType::NSS).as_deref(), Some(&b"void main() { }\n"[..]));
    assert_eq!(copy(ResType::NCS).as_deref(), Some(&b"NCS V1.0 compiled"[..]));
}

#[test]
#[ignore = "a look at fields that differ among blueprints edited together"]
fn look_edit_together_mixed() {
    use mg_edit::{Command, Edit};
    let Some(root) = mg_testkit::nwn_root() else { return };
    mg_testkit::gpu::hold();
    let dir = mg_testkit::scratch_dir("ui-mixed-look");
    let path = sample_module(&dir);
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(&root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.open_palette = false;
    let game = app.game.as_deref().unwrap();
    let base = Gff::read(&game.resman.get_named("plc_chest1", ResType::UTP).unwrap()).unwrap();
    let keys: Vec<ResKey> =
        ["mg_box_a", "mg_box_b"].iter().map(|n| ResKey::parse(n, ResType::UTP).unwrap()).collect();
    let edits = keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let mut g = base.clone();
            g.root.set("TemplateResRef", mg_gff::Value::resref(k.resref));
            if i == 1 {
                g.root.set("Tag", mg_gff::Value::String(b"OTHER".to_vec()));
                g.root.set("Hardness", mg_gff::Value::Byte(77));
                g.root.set("Plot", mg_gff::Value::Byte(1));
            }
            Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprints(keys)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 800.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(5);
    h.render().expect("render").save(dir.join("a.png")).unwrap();
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

/// Options › Script Editor › Automatically Compile Scripts on Save:
/// saving the module compiles the scripts whose text it stores (it was
/// only a script's own Save that did).
#[test]
fn saving_the_module_compiles_edited_scripts_when_asked() {
    let Some((mut h, _)) = area_harness("save-compiles") else { return };
    let (nss, ncs) = (
        ResKey::parse("hello", ResType::NSS).unwrap(),
        ResKey::parse("hello", ResType::NCS).unwrap(),
    );
    let edit = mg_edit::Edit::SetResource { key: nss, data: Some(b"void main()\n{\n}\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss)));
    h.run_steps(3);
    let has = |h: &mut Harness<'_, Moonglow>, k: &ResKey| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        ws.module.contains(k)
    };
    // Typed in (a line indented), then the module saved: not compiled
    // while the option is off.
    let edit_and_save = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().script_tools.jump = Some((nss, 0));
        h.run_steps(2);
        h.key_press(egui::Key::Tab);
        h.run_steps(2);
        h.state_mut().actions.push(mg_ui::Action::Save);
        h.run_steps(3);
    };
    h.state_mut().settings.auto_compile = false;
    edit_and_save(&mut h);
    assert!(!has(&mut h, &ncs));
    h.state_mut().settings.auto_compile = true;
    edit_and_save(&mut h);
    assert!(has(&mut h, &ncs), "compiled on save");
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("Compiled on save: hello")));
}

/// A script another program saved in the module's folder is compiled
/// when it is read again, with Automatically Compile Scripts on Save: the
/// folder's compiled script was the older one until Compile All.
#[test]
fn a_script_saved_outside_is_compiled_when_asked() {
    use mg_module::ModuleLocation;
    use mg_module::new::new_module;
    let Some(root) = mg_testkit::nwn_root() else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut m = new_module(&game, "Outside", &mut fastrand::Rng::with_seed(7)).unwrap();
    let (nss, ncs) = (
        ResKey::parse("hello", ResType::NSS).unwrap(),
        ResKey::parse("hello", ResType::NCS).unwrap(),
    );
    m.set(nss, b"void main()\n{\n}\n".to_vec());
    let dir = mg_testkit::scratch_dir("ui-folder-compile").join("module");
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.settings.no_mod_beside_folder = true;
    app.settings.auto_compile = false;
    app.open_module(&dir);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let compiled = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        ws.module.get(&ncs).map(<[u8]>::to_vec)
    };
    let save_outside = |h: &mut Harness<'_, Moonglow>, text: &str| {
        std::fs::write(dir.join("hello.nss"), text).unwrap();
        h.state_mut().reload_project_files();
        h.run_steps(3);
    };
    // Not asked: read again, not compiled.
    save_outside(&mut h, "void main()\n{\n    int n = 1;\n}\n");
    let text = h.state().ws.as_ref().unwrap().module.get(&nss).unwrap().to_vec();
    assert!(String::from_utf8_lossy(&text).contains("int n = 1;"));
    assert!(compiled(&mut h).is_none());
    // Asked: compiled, and again when its text changes.
    h.state_mut().settings.auto_compile = true;
    save_outside(&mut h, "void main()\n{\n    int n = 2;\n}\n");
    let first = compiled(&mut h).expect("compiled when read again");
    save_outside(&mut h, "void main()\n{\n    int n = 2;\n    n += GetHitDice(OBJECT_SELF);\n}\n");
    assert_ne!(compiled(&mut h).unwrap(), first);
    // An include saved outside: the script that includes it is compiled.
    std::fs::write(dir.join("inc_n.nss"), "int N() { return 1; }\n").unwrap();
    save_outside(&mut h, "#include \"inc_n\"\nvoid main()\n{\n    int n = N();\n}\n");
    let with_one = compiled(&mut h).unwrap();
    std::fs::write(dir.join("inc_n.nss"), "int N() { return 2 + GetHitDice(OBJECT_SELF); }\n")
        .unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(3);
    assert_ne!(compiled(&mut h).unwrap(), with_one, "its includer is compiled again");
    // One that does not compile keeps its compiled script, and is named.
    save_outside(&mut h, "void main()\n{\n    nothing();\n}\n");
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("Did not compile")));
    // The save writes the compiled script beside its source.
    save_outside(&mut h, "void main()\n{\n    int n = 3;\n}\n");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert_eq!(std::fs::read(dir.join("hello.ncs")).ok(), compiled(&mut h));
    assert_eq!(
        std::fs::read(dir.join("hello.nss")).unwrap(),
        b"void main()\n{\n    int n = 3;\n}\n"
    );
}

/// Compile All Scripts passes over the scripts that are as they were when
/// it last compiled them: after an include changes, only the scripts that
/// include it are compiled again.
#[test]
fn compile_all_compiles_only_what_changed() {
    let Some((mut h, _)) = area_harness("compile-changed") else { return };
    let nss = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let set = |h: &mut Harness<'_, Moonglow>, name: &str, text: &str| {
        let edit = mg_edit::Edit::SetResource { key: nss(name), data: Some(text.into()) };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
        h.run_steps(2);
    };
    set(&mut h, "inc_n", "int N() { return 1; }\n");
    set(&mut h, "uses_n", "#include \"inc_n\"\nvoid main() { int n = N(); }\n");
    set(&mut h, "alone", "void main() { int a = 1; }\n");
    let compile = |h: &mut Harness<'_, Moonglow>| -> String {
        let before = h.state().log.entries.len();
        h.state_mut().actions.push(mg_ui::Action::CompileScripts);
        h.run_steps(3);
        let mut said = h.state().log.entries[before..].iter().map(|(_, m)| m.clone());
        said.rfind(|m| m.starts_with("Compiled ")).expect("Compile All's line")
    };
    let first = compile(&mut h);
    assert!(!first.contains("left alone"), "{first}");
    let again = compile(&mut h);
    assert!(again.starts_with("Compiled 0 scripts") && again.contains("left alone"), "{again}");
    // The include changes: it and its includer are compiled, the third
    // script is not; the includer's compiled script is another.
    set(&mut h, "inc_n", "int N() { return 2 + GetHitDice(OBJECT_SELF); }\n");
    let after = compile(&mut h);
    assert!(after.starts_with("Compiled 2 scripts: 0 failed, 1 changed"), "{after}");
    // A compiled script that went missing is made again.
    let ncs = ResKey::parse("alone", ResType::NCS).unwrap();
    let gone = mg_edit::Edit::SetResource { key: ncs, data: None };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Gone", vec![gone])));
    h.run_steps(2);
    let back = compile(&mut h);
    assert!(back.starts_with("Compiled 1 scripts: 0 failed, 1 changed"), "{back}");
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
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.settings.no_last_area = true;
    app.settings.external_editor = Some("/bin/true".into());
    app.settings.scripts_external = true;
    app.open_module(&dir);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let said = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state().log.entries.iter().filter(|(_, m)| m.contains(what)).count()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenResource(key));
    h.run_steps(4);
    let file = dir.join("src/hello.nss");
    assert!(file.is_file(), "the project's own file");
    assert_eq!(said(&h, &format!("editing {} in /bin/true", file.display())), 1);
    // Saved there by the editor: read again as a file changed outside is.
    std::fs::write(&file, "void main()\n{\n    int n = 1;\n}\n").unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(3);
    assert!(h.state().script_text(key).unwrap().contains("int n = 1;"));
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
    h.state_mut().tree_reveal = Some((area, false));
    h.run_steps(2);
    assert_eq!(h.state().tree_area, Some(area));
    assert!(h.state().tree_reveal.is_none(), "done once");
    assert!(h.query_by_label("Waypoints (2)").is_none(), "not opened out");
    // Show in Module Tree: opened out too.
    h.state_mut().tree_reveal = Some((area, true));
    h.run_steps(3);
    assert!(h.query_by_label("Waypoints (2)").is_some());
}

/// Where a text has none in the language edited, its text in another
/// shows in its field, with the language named: in Module Properties and
/// in a blueprint's fields alike.
#[test]
fn text_in_another_language_shows_where_the_one_edited_has_none() {
    use mg_core::{Gender, Language, LocString};
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_named", ResType::UTC) else {
        return;
    };
    // The module's name and the bandit's first name, in English alone.
    let mut name = LocString::default();
    name.set(Language::ENGLISH, Gender::Male, b"Only English".to_vec());
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let edits = vec![
        mg_edit::Edit::SetField {
            key: ifo,
            path: mg_edit::GffPath::root(),
            label: "Mod_Name".into(),
            value: Some(mg_gff::Value::LocString(name.clone())),
        },
        mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "FirstName".into(),
            value: Some(mg_gff::Value::LocString(name)),
        },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    h.run_steps(4);
    // Edited in English: its own text, unmarked.
    let own = h.get_all_by_value("Only English").count();
    assert!(own >= 2, "both fields have it");
    assert_eq!(h.query_all_by_label("English").count(), 0);
    // Edited in German: the English text still shows, named as English.
    mg_ui::set_edit_language(Language::GERMAN);
    h.run_steps(3);
    let shown = h.get_all_by_value("Only English").count();
    let named = h.query_all_by_label("English").count();
    mg_ui::set_edit_language(Language::ENGLISH);
    assert_eq!((shown, named), (own, 2));
}

/// An object selected in the area's view is shown in the module tree
/// where its area is opened out there: its kind's list opened, its row
/// marked. An area left closed in the tree stays closed.
#[test]
fn the_module_tree_shows_the_object_selected_in_the_area() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tree-follows") else { return };
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
    let folded = h.query_all_by_label_contains("Waypoint").count();
    // Another object selected: its kind's list opens to it.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    h.run_steps(6);
    assert_eq!(h.state().tree_object, Some((area, ObjectKind::Waypoint, 0)));
    assert!(!h.state().tree_object_pending, "gone to");
    let rows = h.query_all_by_label_contains("Waypoint").count();
    assert!(rows >= folded + 2, "both waypoints' rows are there: {folded} → {rows}");
    // Nothing selected: the tree stays as it is.
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    h.run_steps(3);
    assert_eq!(h.state().tree_object, None);
    assert!(h.query_by_label("Waypoints (2)").is_some());
}

/// A script editor's Open… lists the scripts there are to open, the
/// module's alone or its haks' or all of them, found by name; the game's
/// open to be read.
#[test]
fn a_script_editor_opens_other_scripts() {
    let Some((mut h, _)) = area_harness("open-script") else { return };
    let (hello, other) = (
        ResKey::parse("hello", ResType::NSS).unwrap(),
        ResKey::parse("other_one", ResType::NSS).unwrap(),
    );
    let edits = [hello, other]
        .map(|key| mg_edit::Edit::SetResource { key, data: Some(b"void main() { }\n".to_vec()) })
        .to_vec();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    h.run_steps(4);
    h.get_by_label("Open…").click();
    h.run_steps(3);
    // The module's two, as it opens.
    h.get_by_label("2 scripts");
    // All of them: the game's too, found by name.
    h.get_by_label("All Resources").click();
    h.run_steps(2);
    assert!(h.query_by_label("2 scripts").is_none());
    h.state_mut().script_tools.open_script.as_mut().unwrap().filter = "nw_s0_firebal".into();
    h.run_steps(2);
    h.get_by_label("nw_s0_fireball").click();
    h.run_steps(4);
    let fireball = ResKey::parse("nw_s0_fireball", ResType::NSS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Resource(fireball)).is_some(), "read, not edited");
    assert!(h.state().script_tools.open_script.is_none(), "the window closed");
    // The module's own opens in its editor.
    let tab = h.state().dock.find_tab(&Tab::Resource(fireball)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    h.run_steps(3);
    h.get_by_label("Open…").click();
    h.run_steps(3);
    h.get_by_label("Module Resources Only").click();
    h.run_steps(2);
    h.get_by_label("other_one").click();
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Script(other)).is_some());
}

#[test]
#[ignore = "a look at the palettes with the game's data in another language (MG_LANG)"]
fn look_palette_in_language() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    mg_testkit::gpu::hold();
    let lang = std::env::var("MG_LANG").unwrap_or_else(|_| "es".into());
    let dir = mg_testkit::scratch_dir("ui-palette-language");
    let path = sample_module(&dir);
    let user = std::env::var_os("MG_TEST_USER").map(std::path::PathBuf::from);
    let install = mg_resman::GameInstall::new(&root, user, &lang);
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    app.set_render_state(rs.clone());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Creature;
    app.palette.tiles = false;
    app.palette.custom = std::env::var_os("MG_CUSTOM").is_some();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 700.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(6);
    h.render().expect("render").save(dir.join("a.png")).unwrap();
    let game = h.state().game.as_deref().unwrap();
    eprintln!("LANG {:?} sample {:?}", game.language, game.string(mg_core::StrRef(6686)));
}
