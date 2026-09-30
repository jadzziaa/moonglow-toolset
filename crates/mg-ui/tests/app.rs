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
    Moonglow::new(None, Box::new(NoDialogs { open, save: Vec::new() }))
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
        Box::new(NoDialogs { open: Vec::new(), save: vec![path.clone()] }),
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
