//! Modules as files: opening, saving, closing, recovering, building,
//! testing, publishing, exporting and importing; the module-wide editors.

use crate::common::*;

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
    let mut h = harness(app, 1200.0, 1000.0);
    h.run();
    h.get_by_label("Advanced Controls").click();
    h.run();
    h.get_by_label("Creature CR");
    let done = h.get_by_label("Done").rect();
    assert!(done.bottom() < 800.0, "the buttons are in the window: {done:?}");
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
    let app =
        game_app_with(&root, None, NoDialogs { save: vec![path.clone()], ..Default::default() });
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
    // (Its name, and the folder it is in.)
    let place = dir.file_name().unwrap().to_string_lossy().into_owned();
    h.get_by_label(&format!("sample.mod  in {place}")).click();
    h.run();
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()));
}

/// Export Resources lists the module kind by kind, and one kind alone
/// when it is chosen: the buttons under the list are for those listed.
#[test]
fn export_lists_a_kind() {
    let dir = mg_testkit::scratch_dir("ui-export-kinds");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    for name in ["mg_one", "mg_two"] {
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().actions.push(mg_ui::Action::ExportDialog(Vec::new()));
    h.run();
    // As Aurora's Export begins: an archive that is there is added to
    // (Aurora asks, Yes to append first).
    let draft = h.state().export.clone().unwrap();
    assert!(draft.add && draft.dependencies);
    // Every kind, each under its heading, the tree's kinds first.
    assert!(h.query_by_label("Areas (.are)").is_some());
    assert!(h.query_by_label("Scripts (.nss)").is_some());
    assert!(h.query_by_label("start.are").is_some());
    h.state_mut().export.as_mut().unwrap().kind = Some(ResType::NSS);
    h.run();
    assert!(h.query_by_label("start.are").is_none(), "scripts alone");
    assert!(h.query_by_label("mg_one.nss").is_some());
    let scripts =
        h.state().ws.as_ref().unwrap().module.keys().filter(|k| k.restype == ResType::NSS);
    let scripts: std::collections::BTreeSet<ResKey> = scripts.copied().collect();
    // (The window settles round its shorter list first.)
    h.run_steps(3);
    h.get_by_label(&format!("Select the {} Listed", scripts.len())).click();
    h.run();
    assert_eq!(h.state().export.as_ref().unwrap().selected, scripts);
    // With the filter, those it leaves.
    h.state_mut().export.as_mut().unwrap().filter = "mg_t".into();
    h.run_steps(3);
    h.get_by_label("Clear Them").click();
    h.run();
    let left = &h.state().export.as_ref().unwrap().selected;
    assert_eq!(left.len(), scripts.len() - 1);
    assert!(!left.contains(&ResKey::parse("mg_two", ResType::NSS).unwrap()));
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

/// An area's `.are` exported without the resources it uses still takes
/// its `.git` and `.gic` (the builders' reports on 1.25.1: the window
/// lists the three apart, and the archive held the `.are` alone, which
/// imports as an area with nothing placed in it), as Export as Files does.
#[test]
fn an_area_exported_alone_takes_its_git_and_gic() {
    let dir = mg_testkit::scratch_dir("ui-export-area-alone");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    for (t, kind) in [(ResType::GIT, *b"GIT "), (ResType::GIC, *b"GIC ")] {
        m.set(ResKey::parse("start", t).unwrap(), Gff::new(kind).to_bytes().unwrap());
    }
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    m.save().unwrap();
    let erf = dir.join("start.erf");
    let dialogs = NoDialogs { save: vec![erf.clone()], ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    let area = ResKey::parse("start", ResType::ARE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::ExportDialog(vec![area]));
    h.run();
    h.state_mut().export.as_mut().unwrap().dependencies = false;
    h.run();
    h.get_by_label("Export 1…").click();
    h.run();
    let bytes = std::fs::read(&erf).unwrap();
    let archive = mg_erf::Erf::read(&bytes).unwrap();
    let mut names: Vec<String> =
        archive.entries.iter().map(|e| ResKey::new(e.resref, e.restype).to_string()).collect();
    names.sort();
    assert_eq!(names, ["start.are", "start.gic", "start.git"]);
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

/// A `.mod` with a folder of its name beside it (a module directory, as
/// Aurora keeps one) asks which to open, as Aurora does: the folder (saved
/// into, and written as the `.mod` too), or the `.mod`; Options › General
/// opens either without asking (GitHub issue 30).
#[test]
fn a_mod_with_its_folder_beside_it_asks_which_to_open() {
    let dir = mg_testkit::scratch_dir("ui-mod-folder");
    let path = sample_module(&dir);
    let folder = dir.join("sample");
    Module::open(&path).unwrap().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let mut h =
        Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(vec![path.clone()]));
    h.run();
    let location = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref()?.module.location.clone();
    // File › Open Module… (the welcome page's button): asked first.
    h.get_by_label("Open Module…").click();
    h.run();
    assert_eq!(h.state().module_folder.as_deref(), Some(path.as_path()));
    assert!(h.state().ws.is_none(), "nothing opened before the answer");
    h.get_by_label_contains("A directory with the same name has been found");
    h.get_by_label("Open Directory").click();
    h.run();
    assert_eq!(location(&h), Some(ModuleLocation::Folder(folder.clone())));
    assert!(h.state().module_folder.is_none());
    assert_eq!(h.state().settings.recent.first(), Some(&folder), "the folder is remembered");
    // An edit saved goes into the folder, and into the .mod beside it.
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = b"void main() { int saved; }\n".to_vec();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "edit",
        vec![mg_edit::Edit::SetResource { key, data: Some(text.clone()) }],
    )));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert_eq!(std::fs::read(folder.join("hello.nss")).unwrap(), text);
    assert_eq!(Module::open(&path).unwrap().get(&key), Some(&text[..]));
    // The .mod again (Recent Modules): asked again; the .mod opened.
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
    h.run();
    assert!(h.state().module_folder.is_some(), "asked again");
    h.get_by_label("Open .mod File").click();
    h.run();
    assert_eq!(location(&h), Some(ModuleLocation::Archive(path.clone())));
    // Cancel opens nothing: the module open stays.
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
    h.run();
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().module_folder.is_none());
    assert_eq!(location(&h), Some(ModuleLocation::Archive(path.clone())));
    // Options › General: always the folder, or always the .mod.
    use mg_ui::settings::ModuleFolders;
    assert_eq!(h.state().settings.module_folders, ModuleFolders::Ask, "asks unless chosen");
    h.state_mut().settings.module_folders = ModuleFolders::Folder;
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
    h.run();
    assert!(h.state().module_folder.is_none(), "not asked");
    assert_eq!(location(&h), Some(ModuleLocation::Folder(folder.clone())));
    h.state_mut().settings.module_folders = ModuleFolders::Module;
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
    h.run();
    assert!(h.state().module_folder.is_none(), "not asked");
    assert_eq!(location(&h), Some(ModuleLocation::Archive(path.clone())));
    // A folder of its name with no module in it (no module.ifo) is no
    // module directory: the .mod opens as it is.
    h.state_mut().settings.module_folders = ModuleFolders::Ask;
    std::fs::remove_dir_all(&folder).unwrap();
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("notes.txt"), "not a module").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
    h.run();
    assert!(h.state().module_folder.is_none());
    assert_eq!(location(&h), Some(ModuleLocation::Archive(path)));
}

/// The welcome page's Recent modules tell a module folder from the `.mod`
/// beside it (GitHub issue 30: both showed as their names alone): a
/// folder's name ends in `/`, and the folder each is in follows it.
#[test]
fn the_welcome_page_tells_a_folder_from_its_mod() {
    let dir = mg_testkit::scratch_dir("ui-welcome-recent");
    let path = sample_module(&dir);
    let folder = dir.join("sample");
    Module::open(&path).unwrap().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.recent = vec![folder.clone(), path.clone()];
    app.settings.module_folders = mg_ui::settings::ModuleFolders::Folder;
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let place = dir.file_name().unwrap().to_string_lossy().into_owned();
    h.get_by_label(&format!("sample.mod  in {place}"));
    h.get_by_label(&format!("sample/  in {place}")).click();
    h.run();
    let location = h.state().ws.as_ref().unwrap().module.location.clone();
    assert_eq!(location, Some(ModuleLocation::Folder(folder)));
}

/// The folder beside a `.mod` is found by its name in any case, as Aurora
/// on Windows finds it (`Sample/` beside `sample.mod`, on Linux): asked
/// about, opened, and saved as that `.mod`, not a second one.
#[test]
fn a_mod_s_folder_in_another_case_is_found() {
    let dir = mg_testkit::scratch_dir("ui-mod-folder-case");
    let path = sample_module(&dir);
    let folder = dir.join("Sample");
    Module::open(&path).unwrap().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let mut h =
        Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(vec![path.clone()]));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OpenModule(path.clone()));
    h.run();
    assert_eq!(h.state().module_folder.as_deref(), Some(path.as_path()), "asked");
    h.get_by_label(&folder.display().to_string());
    h.get_by_label("Open Directory").click();
    h.run();
    let location = h.state().ws.as_ref().unwrap().module.location.clone();
    assert_eq!(location, Some(ModuleLocation::Folder(folder.clone())));
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let mods: Vec<String> = (std::fs::read_dir(&dir).unwrap().flatten())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".mod"))
        .collect();
    assert_eq!(mods, ["sample.mod"], "the .mod beside it, not a second one");
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
    h.get_all_by_label("OK").last().unwrap().click();
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
    h.get_all_by_label("OK").last().unwrap().click();
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
    let mut app = game_app(&root, Some(user));
    app.background_jobs = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.get_by_label("Build").click();
    h.run();
    h.get_by_label("Publish to NWSync…").click();
    h.run();
    let repo = dir.join("repo");
    let _ = std::fs::remove_dir_all(&repo);
    h.state_mut().publish.as_mut().unwrap().folder = repo.display().to_string();
    h.run();
    // It runs as a job (its window, with Cancel), the window behind it
    // showing what it wrote.
    let until_done = |h: &mut Harness<'_, Moonglow>| {
        let started = std::time::Instant::now();
        while h.state().busy() {
            h.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(5));
            assert!(started.elapsed().as_secs() < 30, "the job never ended");
        }
        h.run_steps(2);
    };
    h.get_by_label("Publish").click();
    h.run_steps(1);
    until_done(&mut h);
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

    // Remove Unused Data: a data file no manifest names, older than two
    // weeks, goes; the manifest's own stays.
    let unused = repo.join("data/sha1/ab/cd").join("ab".repeat(20));
    std::fs::create_dir_all(unused.parent().unwrap()).unwrap();
    std::fs::write(&unused, b"left by a manifest taken away").unwrap();
    let month_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 86_400);
    let file = std::fs::File::options().write(true).open(&unused).unwrap();
    file.set_modified(month_ago).unwrap();
    drop(file);
    h.get_by_label("Remove Unused Data").click();
    h.run_steps(1);
    until_done(&mut h);
    assert!(!unused.exists() && !repo.join("data/sha1/ab").exists());
    let h1 = mg_core::sha1::hex(&mg_core::sha1::sha1(&table));
    assert!(repo.join("data/sha1").join(&h1[..2]).join(&h1[2..4]).join(&h1).is_file());
    let pruned = h.state().publish.as_ref().unwrap().pruned.clone().expect("pruned");
    assert_eq!((pruned.manifests, pruned.removed.len()), (1, 1));
    h.get_by_label_contains("Removed 1 unused data file (");
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
    h.get_all_by_label("OK").last().unwrap().click();
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

    let mut app = game_app(&root, Some(user));
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
    let h = harness(app, 1000.0, 800.0);
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

/// Build Module's Unused says a script once, with its files; Mark as Used
/// writes its name into the project file's `[used]`, which Counted as
/// Used lists and takes out again.
#[test]
fn build_module_marks_an_unused_script_as_used() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = project_folder("ui-build-used", &[("moonglow.toml", "# the team's\n")]);
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&dir);
    let mut w = mg_ui::build_view::BuildWindow::default();
    w.unused = true;
    app.build = Some(w);
    let mut h = Harness::builder()
        .with_size([900.0, 900.0])
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(2);
    h.get_all_by_label("Build").last().unwrap().click();
    h.run_steps(3);
    let results = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        h.state().build.as_ref().unwrap().results.iter().map(|f| f.text.clone()).collect()
    };
    let line = "Unused: hello.nss (source and compiled)";
    assert!(results(&h).iter().any(|r| r == line), "{:?}", results(&h));
    assert!(!results(&h).iter().any(|r| r.contains("hello.ncs")), "once: {:?}", results(&h));
    h.get_by_label(line).click();
    h.run();
    h.get_by_label("Mark as Used").click();
    h.run_steps(2);
    let toml = std::fs::read_to_string(dir.join("moonglow.toml")).unwrap();
    assert!(toml.starts_with("# the team's\n"), "{toml}");
    assert!(toml.contains("[used]\nscripts = [\"hello\"]"), "{toml}");
    assert!(!results(&h).iter().any(|r| r == line), "no longer said: {:?}", results(&h));
    let used = &h.state().ws.as_ref().unwrap().module.project_file.used;
    assert_eq!(used.scripts.globs, ["hello"]);
    // Built again: still not said.
    h.get_all_by_label("Build").last().unwrap().click();
    h.run_steps(3);
    assert!(!results(&h).is_empty() && !results(&h).iter().any(|r| r.contains("hello")));
    // Counted as Used lists it, and takes it out.
    h.get_by_label("Counted as Used").click();
    h.run();
    h.get_by_label("Script: hello");
    h.get_by_label("Remove").click();
    h.run_steps(2);
    let toml = std::fs::read_to_string(dir.join("moonglow.toml")).unwrap();
    assert!(!toml.contains("\"hello\""), "{toml}");
    assert!(h.state().ws.as_ref().unwrap().module.project_file.used.is_empty());
    // And one typed: a glob.
    type_into_hint(&mut h, "name or glob (ai_*)", "ai_*");
    h.get_by_label("Add").click();
    h.run_steps(2);
    let toml = std::fs::read_to_string(dir.join("moonglow.toml")).unwrap();
    assert!(toml.contains("scripts = [\"ai_*\"]"), "{toml}");
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
    let mut app =
        game_app_with(&root, None, NoDialogs { open: vec![erf.clone()], ..Default::default() });
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

/// A frame that panics writes the recovery copy before the panic goes
/// on (Sacha's crash lost what was not saved): the work as it was then,
/// not as the copy made minutes before had it. Nothing unsaved, no copy.
#[test]
fn a_panic_keeps_the_unsaved_work() {
    let dir = mg_testkit::scratch_dir("ui-recovery-panic");
    let path = sample_module(&dir);
    let recovery = dir.join("recovery");
    let mut app = app_with(Vec::new());
    app.recovery_dir = Some(recovery.clone());
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // (Its message is printed: the test's own.)
    let panics = |app: &mut Moonglow| {
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            app.recovering(|_| panic!("a frame that fails, as the test means it to"));
        }));
        assert!(caught.is_err(), "the panic goes on");
    };
    panics(h.state_mut());
    assert!(mg_ui::recovery::find(&recovery).is_empty(), "nothing to keep");
    let key = ResKey::parse("module", ResType::IFO).unwrap();
    let edit = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: "Mod_Tag".into(),
        value: Some(mg_gff::Value::String(b"KEPT".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("tag", vec![edit])));
    h.run();
    panics(h.state_mut());
    let copies = mg_ui::recovery::find(&recovery);
    assert_eq!(copies.len(), 1);
    let kept = Module::open(&copies[0].archive).unwrap();
    let ifo = kept.gff(&key).unwrap().unwrap();
    assert_eq!(ifo.root.string("Mod_Tag"), Some(&b"KEPT"[..]));
    // A frame that does not panic is only the frame.
    h.state_mut().recovering(|app| app.log.info("drawn"));
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

/// Build Module without the game's data says that it is the game's data
/// that is missing (there is a module).
#[test]
fn a_build_without_game_data_says_so() {
    let dir = mg_testkit::scratch_dir("ui-build-no-game");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1200.0, 900.0);
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

/// Verify's report: errors first; objects placed from a blueprint that is
/// gone (common, and harmless as the module plays) one line a blueprint,
/// last, not one an object among the errors (a builder's report: some 200
/// lines read as errors).
#[test]
fn verify_says_errors_first_and_gone_blueprints_once_each() {
    use mg_module::refs::{RefKind, Reference};
    use mg_module::verify::{Category, Missing};
    use mg_ui::LogLevel;
    let missing = |from: &str, path: &str, kind: RefKind, target: &str| Missing {
        reference: Reference {
            from: ResKey::from_filename(from).unwrap(),
            path: path.into(),
            kind,
            target: ResRef::from_str(target).unwrap(),
        },
        category: Category::Placeables,
        uncompiled: false,
        only_local: None,
    };
    let utp = RefKind::Blueprint(ResType::UTP);
    let list = [
        missing("forest.git", "/Placeable List[3]/TemplateResRef", utp, "ub_seta004"),
        missing("forest.git", "/Placeable List[9]/TemplateResRef", utp, "ub_seta004"),
        missing("cave.git", "/Placeable List[0]/TemplateResRef", utp, "ub_seta004"),
        missing("cave.git", "/Placeable List[1]/TemplateResRef", utp, "ub_roca"),
        missing("cave.git", "/Placeable List[1]/OnUsed", RefKind::Script, "no_script"),
    ];
    let report = mg_ui::verify_report(&list, &[]);
    assert_eq!(report[0].0, LogLevel::Error);
    assert!(report[0].1.contains("no_script"), "{report:#?}");
    assert!(report.iter().all(|(l, _)| *l != LogLevel::Warn), "no warning an object");
    let lines: Vec<&str> = report[1..].iter().map(|(_, t)| t.as_str()).collect();
    assert!(lines[0].starts_with("4 object(s) are placed from 2 blueprint(s)"), "{lines:#?}");
    assert_eq!(
        lines[1..],
        ["  ub_roca.utp: 1 placed, in cave", "  ub_seta004.utp: 3 placed, in forest, cave"]
    );
}

/// The script editor's To Scratch: the script saved and compiled, then it
/// and its compiled script copied into the scratch folder. One that
/// doesn't compile (an include file, one with an error) goes there too,
/// without a compiled script: not the one the module has from before (El
/// Grillo's request).
#[test]
fn a_script_goes_to_the_scratch_folder_compiled() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-script-scratch");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let open = |module: &std::path::Path, scratch: &std::path::Path| {
        let mut app = game_app(&root, None);
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
    let texts = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let m = &h.state().script_tools.messages;
        m.iter().filter(|m| m.script == key).map(|m| m.text.clone()).collect()
    };
    let said = texts(&h);
    assert!(said.iter().any(|t| t.contains("copied hello.nss, hello.ncs to")), "{said:?}");
    // Compiled without debug information: the debug file of an older
    // compile is removed, and another program's spelling of the compiled
    // script too (where the file system tells cases apart, a file of its
    // own the game could read in its place).
    std::fs::write(scratch.join("hello.ndb"), b"NDB older").unwrap();
    let other_spelling = scratch.join("HELLO.NCS");
    let apart = !other_spelling.exists();
    if apart {
        std::fs::write(&other_spelling, b"NCS V1.0 older").unwrap();
    }
    h.get_by_label("To Scratch").click();
    h.run_steps(3);
    assert!(std::fs::read(scratch.join("hello.ncs")).unwrap().starts_with(b"NCS V1.0"));
    assert!(!scratch.join("hello.ndb").exists(), "no debug file of an older compile");
    let said = texts(&h);
    let removed = match apart {
        true => "removed HELLO.NCS, hello.ndb, left by an earlier copy (the game would still run",
        false => "removed hello.ndb, left by an earlier copy (made by an older compile)",
    };
    assert!(said.iter().any(|t| t.contains(removed)), "{said:?}");
    if apart {
        assert!(!other_spelling.exists());
    }
    // With debug information: the debug file made goes with it.
    h.state_mut().settings.debug_info = true;
    h.get_by_label("To Scratch").click();
    h.run_steps(3);
    assert!(scratch.join("hello.ndb").exists(), "{:?}", texts(&h));
    h.state_mut().settings.debug_info = false;
    // A script with an error: its source copied, and the compiler's error
    // said; the compiled script the module has from before is not.
    let broken_path = sample_module(&dir.join("broken"));
    let mut m = Module::open(&broken_path).unwrap();
    m.set(key, b"void main() { x = 1; }\n".to_vec());
    m.set(ResKey::parse("hello", ResType::NCS).unwrap(), b"NCS V1.0 old".to_vec());
    m.save().unwrap();
    let scratch = dir.join("scratch2");
    let mut h = open(&broken_path, &scratch);
    h.get_by_label("To Scratch").click();
    h.run_steps(3);
    assert_eq!(std::fs::read(scratch.join("hello.nss")).unwrap(), b"void main() { x = 1; }\n");
    assert!(!scratch.join("hello.ncs").exists(), "no compiled script");
    let messages = &h.state().script_tools.messages;
    assert!(messages.iter().any(|m| m.error && m.location.is_some()), "the error: {messages:?}");
    let said = texts(&h);
    let copied = said.iter().find(|t| t.contains("copied hello.nss to")).expect("{said:?}");
    assert!(copied.contains("not compiled"), "{copied}");
    // A compiled script left in the folder by an earlier copy, and its
    // debug file, are removed (the game would run it), and that is said.
    std::fs::write(scratch.join("hello.ncs"), b"NCS V1.0 older").unwrap();
    std::fs::write(scratch.join("hello.ndb"), b"NDB older").unwrap();
    std::fs::write(scratch.join("other.ncs"), b"NCS V1.0 other").unwrap();
    h.get_by_label("To Scratch").click();
    h.run_steps(3);
    assert!(!scratch.join("hello.ncs").exists() && !scratch.join("hello.ndb").exists());
    assert!(scratch.join("other.ncs").exists(), "another script's is left");
    let said = texts(&h);
    let removed = "removed hello.ncs, hello.ndb, left by an earlier copy";
    assert!(said.iter().any(|t| t.contains(removed)), "{said:?}");
    let log = &h.state().log.entries;
    assert!(log.iter().any(|(_, m)| m.contains(removed)), "{log:?}");
    // An include file (no main): copied, not compiled.
    let include_path = sample_module(&dir.join("include"));
    let mut m = Module::open(&include_path).unwrap();
    m.set(key, b"int Twice(int n) { return 2 * n; }\n".to_vec());
    m.save().unwrap();
    let scratch = dir.join("scratch3");
    let mut h = open(&include_path, &scratch);
    h.get_by_label("To Scratch").click();
    h.run_steps(3);
    let source = std::fs::read(scratch.join("hello.nss")).unwrap();
    assert_eq!(source, b"int Twice(int n) { return 2 * n; }\n");
    assert!(!scratch.join("hello.ncs").exists());
    let said = texts(&h);
    assert!(said.iter().any(|t| t.contains("an include file")), "{said:?}");
    assert!(said.iter().any(|t| t.contains("copied hello.nss to")), "{said:?}");
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
    // One that no longer compiles goes out as its source alone, as the
    // script editor's To Scratch sends it, and the compiled script the
    // last copy left is taken out of the folder (the game would run it).
    set(&mut h, key, b"void main() { x = 1; }\n");
    let export = mg_ui::Action::ExportFiles { keys: vec![key], dependencies: false, scratch: true };
    h.state_mut().actions.push(export);
    h.run_steps(3);
    assert_eq!(std::fs::read(scratch.join("hello.nss")).unwrap(), b"void main() { x = 1; }\n");
    assert!(!scratch.join("hello.ncs").exists(), "no compiled script, old or new");
    assert!(said(&h, "No longer compiles (its source copied alone"), "{:?}", h.state().log.entries);
    assert!(said(&h, "Scratch folder: removed hello.ncs, left by an earlier copy"));
    // Export as Files to a folder chosen sends it as before, with the
    // module's compiled script, and says it is older.
    let chosen = scratch.with_file_name("chosen");
    h.state_mut().dialogs =
        Box::new(NoDialogs { folders: vec![chosen.clone()], ..Default::default() });
    let export =
        mg_ui::Action::ExportFiles { keys: vec![key], dependencies: false, scratch: false };
    h.state_mut().actions.push(export);
    h.run_steps(3);
    assert_eq!(std::fs::read(chosen.join("hello.ncs")).unwrap(), written);
    assert!(said(&h, "No longer compiles (the .ncs written is older than the script): hello"));
}

/// Without the game's data nothing can be compiled: a script goes out
/// with the compiled script the module has, and the log says it may be
/// older than its source.
#[test]
fn without_game_data_a_script_goes_out_with_the_module_s_compiled_script() {
    let dir = mg_testkit::scratch_dir("ui-no-game-export");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(ResKey::new(key.resref, ResType::NCS), b"NCS V1.0 old".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let scratch = dir.join("scratch");
    app.settings.scratch_dir = Some(scratch.clone());
    app.actions.push(mg_ui::Action::ExportFiles {
        keys: vec![key],
        dependencies: false,
        scratch: true,
    });
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    assert_eq!(std::fs::read(scratch.join("hello.ncs")).unwrap(), b"NCS V1.0 old");
    let log = &h.state().log.entries;
    let warned = "No game data to compile with: the scripts go out with the compiled scripts";
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Warning && m.starts_with(warned)),
        "{log:?}"
    );
}

/// The area view's To Scratch: the area as it is now (its .are, .git and
/// .gic) copied into the scratch folder, with what of the module's it
/// uses (its OnEnter script, compiled), not only the area.
#[test]
fn an_area_goes_to_the_scratch_folder() {
    let enter = |m: &mut Module, _: &mg_rules::GameData, area: ResRef| {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = Gff::read(m.get(&key).unwrap()).unwrap();
        are.root.set("OnEnter", mg_gff::Value::ResRef(b"mg_enter".to_vec()));
        m.set(key, are.to_bytes().unwrap());
        m.set(ResKey::parse("mg_enter", ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
        // (One the area doesn't use stays behind.)
        m.set(ResKey::parse("mg_other", ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    };
    let Some((mut h, area)) = area_harness_with("area-scratch", "ttr01", None, 4, None, &enter)
    else {
        return;
    };
    // No scratch folder yet, and none chosen when asked: nothing is copied.
    h.get_by_label("📤 To Scratch").click();
    h.run_steps(2);
    assert_eq!(h.state().settings.scratch_dir, None);
    let scratch = mg_testkit::scratch_dir("ui-area-scratch").join("scratch");
    h.state_mut().settings.scratch_dir = Some(scratch.clone());
    h.run_steps(2);
    h.get_by_label("📤 To Scratch").click();
    h.run_steps(3);
    let mut names: Vec<String> = std::fs::read_dir(&scratch)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut wanted: Vec<String> = ["are", "gic", "git"].map(|e| format!("{area}.{e}")).into();
    wanted.extend(["mg_enter.ncs".to_string(), "mg_enter.nss".to_string()]);
    wanted.sort();
    assert_eq!(names, wanted);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().to_bytes().unwrap();
    assert_eq!(std::fs::read(scratch.join(format!("{area}.are"))).unwrap(), are);
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
    let mut h = harness(app, 1100.0, 800.0);
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

/// The window's own close button asks before Moonglow closes where there
/// is nothing unsaved to ask about; with the option off it closes, and
/// with unsaved work (a table's here) the question is about that.
#[test]
fn closing_the_window_asks_first() {
    let dir = mg_testkit::scratch_dir("ui-close-asks");
    let file = dir.join("loose.2da");
    std::fs::write(&file, SMALL_TABLE).unwrap();
    let mut h = harness(app_with(Vec::new()), 1000.0, 700.0);
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::CloseWindow);
    h.run_steps(3);
    assert!(h.state().closing && !h.state().quit_requested);
    h.get_by_label("Cancel").click();
    h.run_steps(2);
    assert!(!h.state().closing && !h.state().quit_requested);
    // An unsaved table: asked about that instead.
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(2);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().edit.add_rows(1);
    h.state_mut().actions.push(mg_ui::Action::CloseWindow);
    h.run_steps(3);
    assert!(!h.state().closing && h.state().confirm_discard.is_some());
    h.state_mut().confirm_discard = None;
    assert!(mg_ui::table_view::save(h.state_mut(), id));
    // Nothing unsaved again: asked, and closed on the answer.
    h.state_mut().actions.push(mg_ui::Action::CloseWindow);
    h.run_steps(3);
    h.get_by_label("Close Moonglow").click();
    h.run_steps(3);
    assert!(h.state().quit_requested);
    // With the option off, the button closes.
    h.state_mut().quit_requested = false;
    h.state_mut().settings.close_without_asking = true;
    h.state_mut().actions.push(mg_ui::Action::CloseWindow);
    h.run_steps(3);
    assert!(h.state().quit_requested && !h.state().closing);
    // File › Exit never asks this.
    h.state_mut().quit_requested = false;
    h.state_mut().settings.close_without_asking = false;
    h.state_mut().actions.push(mg_ui::Action::Quit);
    h.run_steps(3);
    assert!(h.state().quit_requested);
}

/// Test Module and a game running that Moonglow did not start (GitHub
/// issue 6): one started on the module stops the test (the module would be
/// written under it), as one Moonglow started does; another is warned of
/// and the test goes on. Never stopped. (Stand-ins named as the client: a
/// shell for the game running, `true` for the client the test starts.)
#[cfg(target_os = "linux")]
#[test]
fn a_game_moonglow_did_not_start_is_known_before_a_test() {
    let dir = mg_testkit::scratch_dir("ui-other-game");
    let (root, user, elsewhere) = (dir.join("root"), dir.join("user"), dir.join("elsewhere"));
    let client = root.join("bin/linux-x86/nwmain-linux");
    for d in [client.parent().unwrap(), &user.join("modules"), &elsewhere] {
        std::fs::create_dir_all(d).unwrap();
    }
    let (Some(sh), Some(truth)) = (which("sh"), true_program()) else { return };
    std::fs::copy(&truth, &client).unwrap();
    let running = elsewhere.join("nwmain-linux");
    std::fs::copy(&sh, &running).unwrap();
    let path = sample_module(&dir);
    std::fs::copy(&path, user.join("modules/mgtest.mod")).unwrap();
    let game = |module: &str| {
        std::process::Command::new(&running)
            .args(["-c", "sleep 30; true", "nwmain-linux", "+TestNewModule", module])
            .spawn()
            .unwrap()
    };
    let mut app = game_app_with(&root, Some(user.clone()), NoDialogs::default());
    app.settings.no_last_area = true;
    app.open_module(&user.join("modules/mgtest.mod"));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(2);
    let test = |h: &mut Harness<'_, Moonglow>| {
        let from = h.state().log.entries.len();
        h.state_mut().actions.push(mg_ui::Action::TestModule);
        h.run_steps(3);
        h.state().log.entries[from..].iter().map(|(_, m)| m.clone()).collect::<Vec<_>>()
    };
    // On the module: no test.
    let mut on_it = game("mgtest");
    let said = test(&mut h);
    on_it.kill().unwrap();
    on_it.wait().unwrap();
    let refused =
        format!("A game is running on mgtest (process {}), not started by Moonglow", on_it.id());
    assert!(said.iter().any(|m| m.starts_with(&refused)), "{said:?}");
    assert!(!said.iter().any(|m| m == "Testing mgtest"), "{said:?}");
    // F9 looks before Test Module's save: the module's file is not
    // written under that game.
    let file = user.join("modules/mgtest.mod");
    let before = std::fs::read(&file).unwrap();
    let edit = mg_edit::Edit::SetResource {
        key: ResKey::parse("added", ResType::NSS).unwrap(),
        data: Some(b"void main() { }\n".to_vec()),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Add", vec![edit])));
    h.run_steps(2);
    let mut on_it = game("mgtest");
    h.key_press(egui::Key::F9);
    h.run_steps(3);
    on_it.kill().unwrap();
    on_it.wait().unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), before, "not saved under the game");
    assert!(h.state().has_unsaved_work());
    let log = &h.state().log.entries;
    assert!(log.iter().any(|(_, m)| m.contains(&format!("(process {})", on_it.id()))), "{log:?}");
    // On another: warned of, and the test goes on.
    let mut other = game("elsewhere");
    let said = test(&mut h);
    other.kill().unwrap();
    other.wait().unwrap();
    let warned = "A game is running that Moonglow did not start (process ";
    assert!(
        said.iter().any(|m| m.starts_with(warned) && m.contains(&other.id().to_string())),
        "{said:?}"
    );
    assert!(said.iter().any(|m| m == "Testing mgtest"), "{said:?}");
}

/// A program on `PATH`.
#[cfg(target_os = "linux")]
fn which(name: &str) -> Option<std::path::PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(name)).find(|p| p.is_file())
}
