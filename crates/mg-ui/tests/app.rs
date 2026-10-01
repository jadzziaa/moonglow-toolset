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
    h.get_by_label("Placeables");
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

    // The Area Wizard follows, as in Aurora's Module Wizard.
    h.run();
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

    // Test mode walks greeting → reply → answer, and back.
    click(&mut h, "Test");
    click(&mut h, "NPC: Hello there.");
    // The tree has a row of the same text: look inside the test window.
    let window =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::Window;
    h.get_all_by_label("Conversation Test: talk.dlg")
        .find(window)
        .unwrap()
        .get_by_label("Who are you?")
        .click();
    h.run();
    click(&mut h, "NPC: A traveller.");
    h.get_by_label("[END DIALOGUE]");
    click(&mut h, "<-- Back");
    assert_eq!(h.state().dialog_views[&key].test.as_ref().unwrap().len(), 2);
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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
    assert_eq!(h.state().model_views[&key].animation.as_deref(), Some("open"));
    let img = h.render().expect("render");
    assert!(img.width() > 0);
}

#[test]
fn blueprint_previews_open() {
    let root = mg_testkit::corpus!();
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
    assert_eq!(h.state().model_views[&key].animation.as_deref(), Some("pause1"));
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
    let tavern = app.game.as_ref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
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
        h.state().game.as_ref().unwrap(),
    );
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
    let game = app.game.as_ref().unwrap();
    let data = game.resman.get_named(name, t).unwrap().into_owned();
    let key = ResKey::parse(copy, t).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "copy",
        vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
    )));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
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
        let game = h.state().game.as_ref().unwrap();
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
fn item_editor_adds_properties_and_keeps_the_cost() {
    let Some((mut h, key)) = blueprint_harness("nw_wswls001", "sword_copy", ResType::UTI) else {
        return;
    };
    let cost_of = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let value = mg_rules::ItemValue::from_gff(&s);
        let game = h.state().game.as_ref().unwrap();
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
        let game = h.state().game.as_ref().unwrap();
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
    let game = h.state().game.as_ref().unwrap();
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
    h.get_by_label("Area Properties").click();
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

    // Visual: a lighting scheme sets the lighting and the tiles' lights in
    // one command.
    h.get_by_label("Visual").click();
    h.run_steps(2);
    let game_row = 2; // environment.2da ExteriorDark
    let name = {
        let game = h.state().game.as_ref().unwrap();
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
        let game = app.game.as_ref().unwrap();
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
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
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
        let game = app.game.as_ref().unwrap();
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
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
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
    assert!(h.state().model_views.contains_key(&chest), "the 3D view");
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
        let game = app.game.as_ref().unwrap();
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
fn area_viewer_paints_terrain() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("terrain") else { return };
    let index = {
        let game = h.state().game.as_ref().unwrap();
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
    h.get_by_label("Tiles").click();
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
    h.get_by_label("Raise/Lower").click();
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
    h.get_by_label("Eraser").click();
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
    assert!(h.state().palette.tile_brush.is_none(), "one group a click");
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
        ("Raise/Lower", [40.0, 30.0]),
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
    h.get_by_label("Select Tiles").click();
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
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(2);
    let t = tile(&mut h, 5);
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
    h.get_by_label("Walkmesh").click();
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
    h.get_by_label("Select Tiles").click();
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
    let game = h.state().game.as_ref().unwrap();
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
    let game = h.state().game.as_ref().unwrap();
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
    let mut h = Harness::builder()
        .with_size(egui::vec2(1000.0, 900.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Wizards").click();
    h.run();
    h.get_by_label("Creature Wizard…").click();
    h.run();
    let next = |h: &mut Harness<'_, Moonglow>| {
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
    h.get_by_label("hu_m_01_").click();
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
    h.get_all_by_label("Finish").last().unwrap().click();
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

/// A look at Options › Area's marks: a door's orientation arrow and an
/// encounter's spawn point posts. Writes
/// `target/test-output/ui-area-marks/marks.png`; run by hand.
#[test]
#[ignore]
fn area_marks_screen() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("marks") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let game = h.state().game.as_ref().unwrap();
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
        let t = h.state().game.as_ref().unwrap().table(table).unwrap();
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
        let game = h.state().game.as_ref().unwrap();
        let strref = game.table("soundset").unwrap().get_int(row, "STRREF")?;
        game.string(mg_core::StrRef(strref as u32))
    };
    let rows = |h: &Harness<'_, Moonglow>, gender: i32| -> Vec<String> {
        let t = h.state().game.as_ref().unwrap().table("soundset").unwrap();
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
