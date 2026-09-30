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
    h.get_by_label("start.are");
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
    assert!(h.state().dock.find_tab(&Tab::Gff(key)).is_some(), "the copy opens in its editor");
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
