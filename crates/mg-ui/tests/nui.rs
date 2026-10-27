//! The NUI Creator, driven as the other editors are (`app/`): a window's
//! tab open in the whole application, read through the accessibility tree
//! and the module. No game install is needed but for Build and Images.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use mg_core::ResType;
use mg_nui::{Binding, Settings, Severity};
use mg_resman::ResKey;
use mg_ui::{Action, Moonglow, NoDialogs, Tab};
use serde_json::{Value, json};

/// A module with the window `name` in it, new, and its tab opening.
fn app_with(name: &str, dialogs: NoDialogs) -> Moonglow {
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
    app.run(Action::NewNui(name.into()));
    app
}

fn app() -> Moonglow {
    app_with("nui_test", NoDialogs::default())
}

fn key() -> ResKey {
    ResKey::parse("nui_test", ResType::JUI).unwrap()
}

/// A resource of the module as text.
fn text(app: &Moonglow, key: ResKey) -> String {
    String::from_utf8(app.ws.as_ref().unwrap().module.get(&key).unwrap().to_vec()).unwrap()
}

/// The window of `harness`, as the module has it.
fn document(h: &Harness<'_, Moonglow>) -> Value {
    mg_nui::parse(h.state().ws.as_ref().unwrap().module.get(&key()).unwrap()).unwrap()
}

/// Its settings (binds, handlers, variants).
fn settings(h: &Harness<'_, Moonglow>) -> Settings {
    let key = ResKey::new(key().resref, ResType::TXT);
    Settings::parse(h.state().ws.as_ref().unwrap().module.get(&key).unwrap()).unwrap()
}

fn revision(h: &Harness<'_, Moonglow>) -> u64 {
    h.state().ws.as_ref().unwrap().revision()
}

fn undo(h: &mut Harness<'_, Moonglow>) {
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run();
}

/// The application drawing `app`, its NUI Creator's window of `size`: the
/// room a test gives the editor, whatever is drawn around it.
fn creator(mut app: Moonglow, size: [u32; 2]) -> Harness<'static, Moonglow> {
    app.settings.window_sizes.retain(|(kind, _)| kind != "nui");
    app.settings.window_sizes.push(("nui".into(), size));
    let mut h = Harness::builder()
        .with_size(egui::vec2(size[0] as f32 + 100.0, size[1] as f32 + 150.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h
}

/// Closes the window Edit code opened the script in, over the Creator's.
fn close_script(h: &mut Harness<'_, Moonglow>, script: ResKey) {
    let dock = &mut h.state_mut().dock;
    let tab = dock.find_tab(&Tab::Script(script)).expect("the script's tab");
    dock.remove_tab(tab);
    h.run();
}

/// The window `nui_test`, new, in an editor with room for its three panes.
fn harness() -> Harness<'static, Moonglow> {
    creator(app(), [1200, 850])
}

#[test]
fn a_control_duplicated_from_its_menu_is_one_undo_step() {
    let key = key();
    let mut h = creator(app(), [1200, 850]);
    h.run();
    let before = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.get_by_label("Element actions").scroll_to_me();
    h.run();
    h.get_by_label("Element actions").click();
    h.run();
    h.get_by_label_contains("Duplicate element").click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let doc = mg_nui::parse(ws.module.get(&key).unwrap()).unwrap();
    assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 3);
    assert_ne!(doc["root"]["children"][1]["id"], doc["root"]["children"][2]["id"]);
    ws.undo().unwrap();
    assert_eq!(ws.module.get(&key).unwrap(), before);
}

/// The pages' buttons stay in one row, of one height, in a narrow window.
#[test]
fn the_pages_stay_in_one_row_in_a_narrow_window() {
    for width in [1200, 820, 640] {
        let h = creator(app(), [width, 850]);
        let first = h.get_by_label("Design").rect();
        for label in ["Bindings", "Events & variants", "Event script…", "Advanced"] {
            let rect = h.get_by_label(label).rect();
            assert!((rect.center().y - first.center().y).abs() < 0.1, "{width}: {label}: {rect:?}");
            assert!((rect.height() - first.height()).abs() < 0.1, "{width}: {label}: {rect:?}");
        }
    }
}

#[test]
fn a_bind_made_for_a_property_and_its_handler_are_undone_on_the_design_page() {
    let mut h = harness();
    h.run();
    h.get_by_label("Canvas Label · Label").click();
    h.run();
    let before = document(&h);
    h.get_by_label("+ New bind for property").click();
    h.run();
    // (The dialog's Cancel, drawn after the one under the editor.)
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert_eq!(document(&h), before);
    h.get_by_label("+ New bind for property").click();
    h.run();
    h.get_by_label("Create").click();
    h.run();
    assert_eq!(document(&h)["root"]["children"][0]["value"], json!({"bind":"label_value"}));
    // A bound value offers its change first.
    h.get_all_by_label("+ Add handler").last().unwrap().click();
    h.run();
    assert_eq!(settings(&h).bindings["label_value"].value, json!("Label"));
    assert!(settings(&h).actions.iter().any(|r| r.event == "watch"
        && r.element == "label_value"
        && r.action == mg_nui::Action::Code));
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run();
    assert!(!settings(&h).actions.iter().any(|r| r.event == "watch"));
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run();
    assert_eq!(document(&h), before);
    assert!(!settings(&h).bindings.contains_key("label_value"));
    h.get_by_label("Property binding");
    h.get_by_label("Canvas Label · Label");
}

#[test]
fn handlers_follow_a_renamed_control_and_go_with_a_deleted_one() {
    let mut h = harness();
    h.run();
    let script_key = ResKey::parse("nui_test_e", ResType::NSS).unwrap();
    let script = |h: &Harness<'_, Moonglow>| text(h.state(), script_key);
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.get_by_label("Edit code").click();
    h.run();
    close_script(&mut h, script_key);
    h.get_by_label("Element ID").click();
    h.run();
    h.get_by_label("Element ID").type_text("_x");
    h.run();
    // (Set when the field is left.)
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(settings(&h).actions.iter().any(|r| r.event == "click" && r.element == "mg_close_x"));
    assert!(script(&h).contains("// mg:begin click mg_close_x\n"));
    assert!(!script(&h).contains("// mg:begin click mg_close\n"));
    // The guard follows too, so the script still builds.
    assert!(script(&h).contains(r#"sElement == "mg_close_x""#));
    assert!(!script(&h).contains(r#""mg_close""#));
    assert!(!mg_nui::edited_outside("nui_test", &script(&h)));
    // Deleting the control takes its handler away; its code stays.
    h.get_all_by_label("Button · Close").next().unwrap().click();
    h.run();
    h.hover_at(h.get_by_label("Layers").rect().center());
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    assert!(!settings(&h).actions.iter().any(|r| r.event == "click"));
    assert!(
        mg_nui::validate(
            &document(&h),
            &Settings { actions: settings(&h).actions, ..Default::default() }
        )
        .iter()
        .all(|d| !d.message.contains("Unknown element ID"))
    );
}

/// A picture field's From disk… puts the file into the module and names
/// it in the field: an Image control's, and a draw layer's.
#[test]
fn picture_fields_take_a_file_from_disk() {
    let dir = mg_testkit::scratch_dir("nui-from-disk");
    let mut tga = vec![0u8; 18];
    (tga[2], tga[12], tga[14], tga[16]) = (2, 1, 1, 24);
    tga.extend([255u8; 3]);
    // A name the game can't read (36 characters) is shortened.
    let (photo, layer) =
        (dir.join("c40e2b2-1610-41b0-8e43-bd4c4cf56464.tga"), dir.join("my_layer.tga"));
    std::fs::write(&photo, &tga).unwrap();
    std::fs::write(&layer, &tga).unwrap();
    let dialogs = NoDialogs { open: vec![layer, photo], ..Default::default() };
    let mut app = app_with("pictures", dialogs);
    let jui = ResKey::parse("pictures", ResType::JUI).unwrap();
    let doc = |app: &Moonglow| -> Value {
        serde_json::from_slice(app.ws.as_ref().unwrap().module.get(&jui).unwrap()).unwrap()
    };
    let mut window = doc(&app);
    window["root"]["children"] = json!([{"type":"image","id":"photo","value":"","height":40.0,
        "draw_list_scissor":false,
        "draw_list":[{"type":5,"enabled":true,"order":1,"render":0,"arrayBinds":false,
            "rect":{"x":0.0,"y":0.0,"w":10.0,"h":10.0},"image":"",
            "image_aspect":0,"image_halign":0,"image_valign":0}]}]);
    app.ws.as_mut().unwrap().module.set(jui, serde_json::to_vec(&window).unwrap());
    // (The module has a picture of the layer's name already, another one.)
    let had = ResKey::parse("my_layer", ResType::TGA).unwrap();
    app.ws.as_mut().unwrap().module.set(had, b"another picture".to_vec());
    let mut h = creator(app, [1400, 2400]);
    h.run();
    h.get_by_label_contains("Canvas Image").click();
    h.run();
    h.get_all_by_label("From disk…").next().unwrap().click();
    h.run();
    h.run();
    let module = &h.state().ws.as_ref().unwrap().module;
    assert!(module.contains(&ResKey::parse("c40e2b2-1610-41b", ResType::TGA).unwrap()));
    assert_eq!(doc(h.state())["root"]["children"][0]["value"], "c40e2b2-1610-41b");
    // Chosen in Layers, it opens in the inspector.
    h.get_by_label("Draw layer 1 · Image").click();
    h.run();
    h.run();
    h.get_all_by_label("From disk…").last().unwrap().click();
    h.run();
    h.run();
    assert!(
        h.state()
            .ws
            .as_ref()
            .unwrap()
            .module
            .contains(&ResKey::parse("my_layer", ResType::TGA).unwrap())
    );
    assert_eq!(doc(h.state())["root"]["children"][0]["draw_list"][0]["image"], "my_layer");
    // It took the place of the one the module had, and that is said.
    assert_eq!(h.state().ws.as_ref().unwrap().module.get(&had), Some(tga.as_slice()));
    let said = |line: &str| line.contains("my_layer.tga takes the place");
    assert!(h.state().log.entries.iter().any(|(_, line)| said(line)));
    assert!(!h.state().log.entries.iter().any(|(_, line)| line.contains("c40e2b2-1610-41b takes")));
}

/// Advanced › Images: a picture from disk into the module, used as the
/// window's background, then moved into a new hak the module lists. The
/// user folder is a scratch one.
#[test]
fn images_from_disk_go_into_the_module_and_a_hak() {
    let dir = mg_testkit::scratch_dir("nui-images");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut tga = vec![0u8; 18];
    (tga[2], tga[12], tga[14], tga[16]) = (2, 1, 1, 24);
    tga.extend([255u8; 3]);
    let picture = dir.join("my_bg.tga");
    std::fs::write(&picture, &tga).unwrap();
    let second = dir.join("my_icon.tga");
    std::fs::write(&second, &tga).unwrap();
    let hak = user.join("hak").join("nui_images.hak");
    let dialogs = NoDialogs {
        open_many: vec![vec![second.clone()], vec![picture]],
        save: vec![hak.clone()],
        ..Default::default()
    };
    let install = mg_resman::GameInstall::new(mg_testkit::corpus!(), Some(user.clone()), "en");
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
    let mut module = mg_module::Module::new();
    module.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    app.ws = Some(mg_edit::Workspace::new(module));
    app.run(Action::NewNui("pictures".into()));
    let mut h = creator(app, [1100, 900]);
    h.run();
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("Images").click();
    h.run();
    let key = ResKey::parse("my_bg", ResType::TGA).unwrap();
    h.get_by_label("Add images from disk…").click();
    h.run();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key));
    // Name it in the window: it is listed, and can be the background.
    let jui = ResKey::parse("pictures", ResType::JUI).unwrap();
    let mut doc: Value =
        serde_json::from_slice(h.state().ws.as_ref().unwrap().module.get(&jui).unwrap()).unwrap();
    doc["root"]["children"] = json!([{"type":"image","value":"my_bg","height":40.0}]);
    h.state_mut().ws.as_mut().unwrap().module.set(jui, serde_json::to_vec(&doc).unwrap());
    h.run();
    h.get_by_label("Use as background").click();
    h.run();
    let doc: Value =
        serde_json::from_slice(h.state().ws.as_ref().unwrap().module.get(&jui).unwrap()).unwrap();
    let back = &doc["root"]["draw_list"][0];
    assert_eq!(
        (back["image"].clone(), back["image_aspect"].clone(), back["order"].clone()),
        (json!("my_bg"), json!(5), json!(-1))
    );
    h.get_by_label("Move module images into a new hak").click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    assert!(!ws.module.contains(&key), "the picture left the module");
    ws.flush().unwrap();
    assert_eq!(ws.module.haks().unwrap(), ["nui_images"]);
    let written = mg_module::hak_edit::Hak::open(&hak).unwrap();
    assert!(written.items().iter().any(|i| i.key == key));
    let logged = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state()
            .log
            .entries
            .iter()
            .any(|(_, line)| line.contains(what) && line.contains(&hak.display().to_string()))
    };
    assert!(logged(&h, "Added 1 picture to"), "the log names the hak written");
    // Into the hak the module now lists, in place: the log names it and
    // the copy kept of what it was.
    h.get_by_label("Into").click();
    h.run();
    h.get_by_label("nui_images.hak").click();
    h.run();
    h.get_by_label("Add images to nui_images.hak…").click();
    h.run();
    let written = mg_module::hak_edit::Hak::open(&hak).unwrap();
    let icon = ResKey::parse("my_icon", ResType::TGA).unwrap();
    assert!(written.items().iter().any(|i| i.key == icon));
    assert!(written.items().iter().any(|i| i.key == key), "what it held stays");
    let bak = hak.with_extension("hak.bak");
    assert!(mg_module::hak_edit::Hak::open(&bak).unwrap().items().iter().all(|i| i.key == key));
    assert!(logged(&h, "nui_images.hak.bak"), "the log names the copy kept");
    // One open in the hak editor is left to its Save there.
    mg_ui::hak_view::open_at(h.state_mut(), &hak).expect("the hak opens");
    h.state_mut().actions.clear();
    let before = std::fs::read(&hak).unwrap();
    h.state_mut().dialogs =
        Box::new(NoDialogs { open_many: vec![vec![second]], ..Default::default() });
    h.get_by_label("Add images to nui_images.hak…").click();
    h.run();
    assert_eq!(std::fs::read(&hak).unwrap(), before);
    assert!(h.state().log.entries.iter().any(|(_, line)| line.contains("hak editor")));
}

#[test]
fn edit_code_opens_the_event_script_at_the_handler_s_section() {
    let mut h = harness();
    h.run();
    let script_key = ResKey::parse("nui_test_e", ResType::NSS).unwrap();
    let script = |h: &Harness<'_, Moonglow>| text(h.state(), script_key);
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    // Edit code writes the handlers' sections and opens the script at this one.
    h.get_by_label("Edit code").click();
    h.run();
    let text = script(&h);
    assert!(text.contains("// mg:begin click mg_close"));
    assert!(h.state().dock.find_tab(&Tab::Script(script_key)).is_some());
    let at = mg_nui::handler_offset(&text, "click mg_close").unwrap();
    assert!(h.state().script_tools.jump.is_none_or(|(k, i)| k == script_key && i == at));
    // Insert adds ready-made code to the handler, after what is there.
    close_script(&mut h, script_key);
    h.get_by_label("Insert…").click();
    h.run();
    h.get_by_label("Close window").click();
    h.run();
    assert_eq!(script(&h).matches("NuiDestroy(oPlayer, nToken);").count(), 2);
    // Event script… opens the whole script; Undo takes the insert back.
    h.get_by_label("Event script…").click();
    h.run();
    assert_eq!(script(&h).matches("NuiDestroy(oPlayer, nToken);").count(), 2);
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run();
    assert_eq!(script(&h).matches("NuiDestroy(oPlayer, nToken);").count(), 1);
}

/// A script written by hand opens as it is: Edit code writes no sections
/// into it.
#[test]
fn edit_code_leaves_a_script_written_by_hand_as_it_is() {
    let mut app = app();
    let script = ResKey::parse("nui_test_e", ResType::NSS).unwrap();
    let manual = "// Manually authored handler\nvoid main() {}";
    app.ws.as_mut().unwrap().module.set(script, manual.as_bytes().to_vec());
    let mut h = creator(app, [1200, 850]);
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.get_by_label("Edit code").click();
    h.run();
    assert_eq!(text(h.state(), script), manual);
    assert!(h.state().dock.find_tab(&Tab::Script(script)).is_some());
}

#[test]
fn add_property_adds_a_color_and_a_window_s_size_limits() {
    let mut h = harness();
    h.run();
    h.get_by_label("Canvas Label · Label").click();
    h.run();
    let before = document(&h);
    h.get_by_label("+ Add property").click();
    h.run();
    h.get_by_label("Foreground color").click();
    h.run();
    assert!(h.query_by_label("Foreground color").is_some(), "new color field must be expanded");
    let colored = document(&h);
    assert_eq!(
        colored["root"]["children"][0]["foreground_color"],
        json!({"r":255,"g":255,"b":255,"a":255})
    );
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    assert_eq!(document(&h), before);
    h.run();
    h.get_by_label("Window").click();
    h.run();
    h.get_by_label("+ Add property").scroll_to_me();
    h.run();
    h.get_by_label("+ Add property").click();
    h.run();
    h.query_all_by_label("Window size limits")
        .find(|n| n.accesskit_node().role() != egui::accesskit::Role::Label)
        .expect("Add window limits menu entry")
        .click();
    h.run();
    assert!(h.query_by_label("Minimum width").is_some(), "new limits must be expanded");
    assert!(h.query_by_label("Advanced properties").is_none());
    let limited = document(&h);
    assert_eq!(limited["size_constraint"], json!({"x":0.0,"y":0.0,"w":0.0,"h":0.0}));
    assert!(
        !mg_nui::validate(&limited, &Settings::default())
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
}

#[test]
fn draw_layers_start_unclipped_and_clipping_written_by_hand_is_turned_off() {
    let mut h = harness();
    h.run();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.get_by_label("Draw layers").scroll_to_me();
    h.run();
    h.get_by_label("Draw layers").click();
    h.run();
    h.get_by_label("+ Draw primitive").scroll_to_me();
    h.run();
    h.get_by_label("+ Draw primitive").click();
    h.run();
    h.get_by_label("Rectangle").click();
    h.run();
    let node = &document(&h)["root"]["children"][1];
    assert_eq!(node["draw_list_scissor"], false);
    assert_eq!(node["draw_list"][0]["type"], 7);
    // It does nothing in the game but blank a window: not offered.
    assert!(h.query_by_label("Clip to control").is_none());
    // Written on by hand, it is flagged and one click turns it off.
    let key = key();
    let mut doc = document(&h);
    doc["root"]["children"][1]["draw_list_scissor"] = json!(true);
    h.state_mut().ws.as_mut().unwrap().module.set(key, serde_json::to_vec(&doc).unwrap());
    h.run();
    h.get_all_by_label("Turn it off").next().unwrap().click();
    h.run();
    let doc = document(&h);
    assert_eq!(doc["root"]["children"][1]["draw_list_scissor"], false);
    assert_eq!(doc["root"]["children"][1]["draw_list"].as_array().unwrap().len(), 1);
}

#[test]
fn opening_draw_layers_changes_nothing() {
    let mut h = harness();
    h.run();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    let before = document(&h);
    h.get_by_label("Draw layers").click();
    h.run();
    assert_eq!(document(&h), before, "Opening Draw layers must not add null draw_list");
    h.get_by_label("Canvas Label · Label").click();
    h.run();
    assert_eq!(document(&h), before, "Selection with Draw layers open must not alter the document");
}

#[test]
fn add_property_offers_hover_properties_on_controls_only() {
    for (label, key, expected) in [
        ("Tooltip when disabled", "disabled_tooltip", json!("")),
        ("Encouraged", "encouraged", json!(false)),
    ] {
        let mut h = harness();
        h.run();
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        let before = document(&h);
        h.get_by_label("+ Add property").click();
        h.run();
        h.query_by_label(label)
            .expect("Supported element property missing from Add property")
            .click();
        h.run();
        let authored = document(&h);
        assert_eq!(authored["root"]["children"][1][key], expected);
        if key == "disabled_tooltip" {
            assert!(h.query_by_label("Tooltips").is_some());
            assert!(h.query_by_label(label).is_some(), "Added tooltip must be visible immediately");
            assert!(h.query_by_label("Advanced properties").is_none());
        }
        assert!(
            !mg_nui::validate(&authored, &Settings::default())
                .iter()
                .any(|d| d.severity == Severity::Error)
        );
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        assert_eq!(document(&h), before);
        h.run();
        h.get_by_label("Window").click();
        h.run();
        h.get_by_label("+ Add property").click();
        h.run();
        assert!(h.query_by_label(label).is_none(), "Element-only property on Window");
    }
}

#[test]
fn interact_still_adds_duplicates_and_deletes_controls() {
    let mut h = harness();
    h.run();
    h.get_by_label("Interact").click();
    h.run();
    let original = document(&h);
    h.get_by_label("Button").click();
    h.run();
    let added = document(&h);
    assert_eq!(added["root"]["children"].as_array().unwrap().len(), 3);
    h.get_by_label("Element actions").scroll_to_me();
    h.run();
    h.get_by_label("Element actions").click();
    h.run();
    h.get_by_label_contains("Duplicate element").click();
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 4);
    // Delete over the simulated controls must never delete a design element.
    h.hover_at(h.get_by_label("Canvas Button · Close").rect().center());
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 4);
    h.hover_at(h.get_by_label("Layers").rect().center());
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(document(&h), added);
    let ws = h.state_mut().ws.as_mut().unwrap();
    for _ in 0..3 {
        ws.undo().unwrap();
    }
    assert_eq!(document(&h), original);
}

#[test]
fn a_window_resized_on_the_canvas_is_one_undo_step() {
    for live in [false, true] {
        let mut h = harness();
        h.run();
        h.get_by_label("UI scale 100%").click();
        h.run();
        h.get_by_label("150%").click();
        h.run();
        if live {
            h.get_by_label("Interact").click();
            h.run();
        }
        h.get_by_label("Canvas window").click();
        h.run();
        let doc = document(&h);
        let revision = h.state().ws.as_ref().unwrap().revision();
        let header = h.get_by_label("Canvas window").rect();
        let from = h.get_by_label("Resize window width and height").rect().center();
        // Measure the full window, independently of its title-bar buttons.
        let scale = (from.x - header.left()) / doc["geometry"]["w"].as_f64().unwrap() as f32;
        h.hover_at(from);
        h.run();
        h.drag_at(from);
        h.run();
        h.hover_at(from + egui::vec2(12.0, 12.0));
        h.run();
        let to = from + egui::vec2(50.0, 30.0);
        h.hover_at(to);
        h.run();
        assert_eq!(document(&h), doc);
        assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision);
        let during = h.get_by_label("Canvas window").rect();
        assert!((during.min - header.min).length() < 1.0, "window origin must stay fixed");
        assert!((during.width() - header.width() - 50.0).abs() < 1.5);
        h.drop_at(to);
        h.run();
        let resized = document(&h);
        assert_eq!(resized["geometry"]["w"], (420.0 + 50.0 / scale).round());
        assert_eq!(resized["geometry"]["h"], (240.0 + 30.0 / scale).round());
        assert_eq!(resized["geometry"]["x"], doc["geometry"]["x"]);
        assert_eq!(resized["geometry"]["y"], doc["geometry"]["y"]);
        assert_eq!(resized["resizable"], doc["resizable"]);
        assert!(resized.get("width").is_none());
        assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision + 1);
        let from = h.get_by_label("Resize window height").rect().center();
        h.hover_at(from);
        h.run();
        h.drag_at(from);
        h.run();
        h.hover_at(from + egui::vec2(0.0, 30.0));
        h.run();
        h.key_press(egui::Key::Escape);
        h.run();
        h.drop_at(from + egui::vec2(0.0, 30.0));
        h.run();
        assert_eq!(document(&h), resized);
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        assert_eq!(document(&h), doc);
    }
}

#[test]
fn keys_duplicate_move_delete_undo_and_redo() {
    let mut h = harness();
    h.run();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::D);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 3);
    let id = document(&h)["root"]["children"][2]["id"].clone();
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowUp);
    h.run();
    assert_eq!(document(&h)["root"]["children"][1]["id"], id);
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 2);
}

#[test]
fn a_control_selected_on_the_canvas_is_revealed_in_collapsed_layers() {
    let mut h = harness();
    h.run();
    let before = document(&h);
    h.get_by_label("Collapse Column").click();
    h.run();
    assert!(h.query_by_label("Button · Close").is_none());
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    assert!(
        h.get_all_by_label("Button · Close")
            .any(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
    );
    h.get_by_label("Collapse all").click();
    h.run();
    assert!(h.query_by_label("Collapse Column").is_none());
    assert!(h.query_by_label("Expand Column").is_none());
    h.get_by_label("Expand all").click();
    h.run();
    assert!(
        h.get_all_by_label("Button · Close")
            .any(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
    );
    assert_eq!(document(&h), before);
}

#[test]
fn a_list_resized_on_the_canvas_is_one_undo_step_and_escape_calls_it_off() {
    let mut h = harness();
    let key = key();
    let mut doc = mg_nui::window();
    let mut list = mg_nui::template("list");
    list["width"] = 200.0.into();
    list["height"] = 100.0.into();
    doc["root"]["children"] = json!([list]);
    h.state_mut().ws.as_mut().unwrap().module.set(key, serde_json::to_vec(&doc).unwrap());
    h.run();
    h.get_by_label("Canvas List").click();
    h.run();
    let revision = h.state().ws.as_ref().unwrap().revision();
    let from = h.get_by_label("Resize width and height").rect().center();
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(12.0, 12.0));
    h.run();
    let to = from + egui::vec2(60.0, 50.0);
    h.hover_at(to);
    h.run();
    assert_eq!(document(&h), doc, "drag must not write intermediate module revisions");
    assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision);
    h.drop_at(to);
    h.run();
    let resized = document(&h);
    assert_eq!(resized["root"]["children"][0]["width"], 260.0);
    assert_eq!(resized["root"]["children"][0]["height"], 150.0);
    assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision + 1);
    let from = h.get_by_label("Resize height").rect().center();
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(0.0, 30.0));
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    h.drop_at(from + egui::vec2(0.0, 30.0));
    h.run();
    assert_eq!(document(&h), resized);
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    assert_eq!(document(&h), doc);
}

/// New NUI Window asks for the name as New Script does; a name with no
/// room for the scripts' suffixes is refused in the log and can be mended.
#[test]
fn new_nui_window_asks_for_its_name() {
    let mut h = harness();
    // (From the command palette, as Tools › New NUI Window… gives it.)
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
    h.run();
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.placeholder() == Some("Type a command's name")
    };
    h.get(egui_kittest::kittest::by().predicate(hint)).type_text("New NUI Window");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let name = |h: &Harness<'_, Moonglow>| {
        h.query_by_label("Name (up to 14 characters)").map(|n| n.rect())
    };
    assert!(name(&h).is_some(), "the window that asks for the name");
    // (The field has the keyboard when its window opens.)
    h.event(egui::Event::Text("fifteen_letters".into()));
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let long = ResKey::parse("fifteen_letters", ResType::JUI).unwrap();
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&long));
    assert!(h.state().log.entries.iter().any(|(_, line)| line.contains("fifteen_letters")));
    assert!(name(&h).is_some(), "the window stays, for the name to be mended");
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().value().as_deref() == Some("fifteen_letters"))
        .expect("the name, as it was typed")
        .click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run();
    h.event(egui::Event::Text("shop".into()));
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let shop = ResKey::parse("shop", ResType::JUI).unwrap();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&shop));
    assert!(name(&h).is_none(), "the window is gone");
    assert!(h.state().dock.find_tab(&Tab::Nui(shop)).is_some());
}

/// A second NUI window opens as a tab of the first one's window.
#[test]
fn two_nui_windows_are_tabs_of_one_window() {
    let mut h = harness();
    h.state_mut().run(Action::NewNui("second".into()));
    h.run();
    let first = Tab::Nui(key());
    let second = Tab::Nui(ResKey::parse("second", ResType::JUI).unwrap());
    let dock = &h.state().dock;
    let (a, b) = (dock.find_tab(&first).unwrap(), dock.find_tab(&second).unwrap());
    assert_eq!(a.node_path(), b.node_path());
}

/// Deleting a window takes its settings and its scripts with it, as one
/// step; a text of the same name that is not its settings stays.
#[test]
fn deleting_a_window_takes_its_settings_and_scripts() {
    let mut app = app();
    app.run(Action::NewNui("gone".into()));
    let key = ResKey::parse("gone", ResType::JUI).unwrap();
    let script = ResKey::parse("gone_e", ResType::NSS).unwrap();
    let module = |app: &Moonglow| app.ws.as_ref().unwrap().module.keys().count();
    let before = module(&app);
    app.ws.as_mut().unwrap().module.set(script, b"void main() {}".to_vec());
    app.run(Action::DeleteResource(key));
    let left = &app.ws.as_ref().unwrap().module;
    assert!(!left.contains(&key) && !left.contains(&script));
    assert!(!left.contains(&ResKey::parse("gone", ResType::TXT).unwrap()));
    assert_eq!(module(&app), before - 2);
    app.ws.as_mut().unwrap().undo().unwrap();
    assert!(app.ws.as_ref().unwrap().module.contains(&script));

    let notes = ResKey::parse("notes", ResType::TXT).unwrap();
    let module = &mut app.ws.as_mut().unwrap().module;
    module.set(ResKey::parse("notes", ResType::JUI).unwrap(), b"{}".to_vec());
    module.set(notes, b"not settings".to_vec());
    app.run(Action::DeleteResource(ResKey::parse("notes", ResType::JUI).unwrap()));
    assert!(app.ws.as_ref().unwrap().module.contains(&notes));
}

/// Cancel after a Build sets back what Build made with the settings that
/// say what it made: set back alone, the settings took the opener Build
/// left for one written by hand, and no Build went through after.
#[test]
fn cancel_after_a_build_leaves_a_window_that_builds() {
    let root = mg_testkit::corpus!();
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    app.ws = Some(mg_edit::Workspace::new(m));
    app.run(Action::NewNui("nui_test".into()));
    let build = Action::GenerateNui { name: "nui_test".into(), export: false };
    app.run(build.clone());
    let built = |app: &Moonglow| {
        let module = &app.ws.as_ref().unwrap().module;
        let name = |t| ResKey::new(key().resref, t);
        let window = mg_nui::parse(module.get(&name(ResType::JUI)).unwrap()).unwrap();
        let settings = Settings::parse(module.get(&name(ResType::TXT)).unwrap()).unwrap();
        mg_nui::is_current(module, "nui_test", &window, &settings)
    };
    assert!(built(&app));
    let mut h = creator(app, [1200, 850]);
    h.state_mut().run(Action::OpenTab(Tab::Nui(key())));
    h.run();
    let opened = document(&h);
    // A control deleted, and the window built as it is then.
    h.get_all_by_label("Button · Close").next().unwrap().click();
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    h.state_mut().run(build.clone());
    h.run();
    assert!(built(h.state()));
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!(document(&h), opened);
    assert!(built(h.state()), "the scripts are those of the window as it was");
    h.state_mut().run(build);
    assert!(built(h.state()), "and Build goes through");
}

/// Cancel leaves the event script's code: what was written in its
/// sections from the script editor since the tab was opened is the
/// builder's, and the script is rebuilt around the handlers Cancel
/// brings back. (It was set back whole, to what it was when the Creator
/// first changed it: gone, where there had been none.) The window is set
/// back to what it was when the tab was opened, as one step that Undo
/// takes back.
#[test]
fn cancel_sets_back_the_window_and_keeps_the_code_written_in_the_event_script() {
    let mut h = harness();
    h.state_mut().run(Action::OpenTab(Tab::Nui(key())));
    h.run();
    let opened = (document(&h), settings(&h));
    let script = ResKey::parse("nui_test_e", ResType::NSS).unwrap();
    // The Close button deleted: its handler goes, and the Creator writes
    // the event script without it.
    h.get_all_by_label("Button · Close").next().unwrap().click();
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    assert_ne!(document(&h), opened.0);
    let written = text(h.state(), script);
    // Code of the builder's own, in the script editor, saved.
    let mine = "void Mine() { SendMessageToPC(OBJECT_SELF, \"mine\"); }\n";
    let with = mg_nui::insert_code(&written, "top", mine).expect("the top section");
    let edit = mg_edit::Edit::SetResource { key: script, data: Some(with.into_bytes()) };
    h.state_mut().run(Action::Apply(mg_edit::Command::new("Edit nui_test_e.nss", vec![edit])));
    h.run();
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!((document(&h), settings(&h)), opened, "the window is as it was");
    let after = text(h.state(), script);
    assert!(after.contains(mine.trim_end()), "the code is kept: {after}");
    // Around the handlers that are back: as Build would write it.
    assert!(!mg_nui::edited_outside("nui_test", &after));
    for handler in &opened.1.actions {
        let begin = format!("{}{}", mg_nui::BEGIN, mg_nui::handler_key(handler));
        assert!(after.contains(&begin), "{begin}: {after}");
    }
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    assert_ne!(document(&h), opened.0, "Cancel is one step");
}

/// A window whose text reads as something other than an object (a list,
/// a number) is said to be no window: the pages, which set its fields,
/// took it for one.
#[test]
fn text_that_is_no_object_is_no_window() {
    for text in ["[]", "7", "\"window\""] {
        let mut app = app();
        let edit = mg_edit::Edit::SetResource { key: key(), data: Some(text.as_bytes().to_vec()) };
        app.run(Action::Apply(mg_edit::Command::new("By hand", vec![edit])));
        let mut h = creator(app, [1200, 850]);
        h.state_mut().run(Action::OpenTab(Tab::Nui(key())));
        h.run_steps(3);
        assert!(h.state().dock.find_tab(&Tab::Nui(key())).is_some(), "{text}");
    }
}

/// An open window's editor stays with its window when something else of
/// its name (a store, a script) is renamed.
#[test]
fn another_resource_of_the_window_s_name_renamed_leaves_its_editor() {
    let mut app = app();
    let store = ResKey::new(key().resref, ResType::UTM);
    let script = ResKey::new(key().resref, ResType::NSS);
    for (k, data) in [
        (store, mg_gff::Gff::new(*b"UTM ").to_bytes().unwrap()),
        (script, b"void main() {}\n".to_vec()),
    ] {
        let edit = mg_edit::Edit::SetResource { key: k, data: Some(data) };
        app.run(Action::Apply(mg_edit::Command::new("Add", vec![edit])));
    }
    app.run(Action::OpenTab(Tab::Nui(key())));
    app.run(Action::OpenTab(Tab::Script(script)));
    app.rename_resource(store, mg_core::ResRef::from_str("market").unwrap(), false);
    assert!(app.dock.find_tab(&Tab::Nui(key())).is_some(), "the window's editor");
    assert!(app.dock.find_tab(&Tab::Script(script)).is_some(), "the script's");
    assert!(
        app.ws.as_ref().unwrap().module.contains(&ResKey::parse("market", ResType::UTM).unwrap())
    );
}

#[test]
fn a_key_chosen_in_options_deletes_the_selected_control() {
    let mut h = harness();
    h.state_mut().settings.key_bindings.insert("nui-delete".into(), vec!["F8".into()]);
    h.run();
    h.get_all_by_label("Button · Close").next().unwrap().click();
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 2);
    h.key_press(egui::Key::ArrowUp);
    h.run();
    h.key_press(egui::Key::F8);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 1);
    assert_eq!(document(&h)["root"]["children"][0]["id"], "mg_close");
}

#[test]
fn keys_are_left_to_a_text_field_a_menu_and_what_is_outside_the_editor() {
    let mut h = harness();
    h.run();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    let before = document(&h);
    // Search text gets Delete and clipboard keys even with an element selected.
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().click();
    h.run();
    h.key_press(egui::Key::Delete);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::D);
    h.event(egui::Event::Cut);
    h.run();
    assert_eq!(document(&h), before);
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.get_by_label("Element actions").click();
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(document(&h), before);
    h.key_press(egui::Key::Escape);
    h.run();
    // (The pointer beside the Creator's window, over the area's place.)
    h.hover_at(egui::pos2(10.0, h.ctx.content_rect().bottom() - 10.0));
    h.key_press(egui::Key::Delete);
    h.run();
    assert_eq!(document(&h), before);
}

#[test]
fn copy_paste_and_cut_work_on_the_selected_control() {
    let mut h = harness();
    h.run();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.event(egui::Event::Copy);
    h.run();
    h.event(egui::Event::Paste(String::new()));
    h.run();
    let doc = document(&h);
    assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 3);
    assert_ne!(doc["root"]["children"][1]["id"], doc["root"]["children"][2]["id"]);
    h.event(egui::Event::Cut);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 3);
}

#[test]
fn a_control_dragged_from_the_palette_lands_where_it_is_dropped_in_layers() {
    for live in [false, true] {
        for (target, offset, index) in
            [("Column", 0.0, 2), ("Button · Close", -7.0, 1), ("Button · Close", 7.0, 2)]
        {
            let mut h = harness();
            h.run();
            if live {
                h.get_by_label("Interact").click();
                h.run();
            }
            let before = document(&h);
            let from = h.get_by_label("Row").rect().center();
            let onto = h.get_all_by_label(target).last().unwrap().rect().center()
                + egui::vec2(0.0, offset);
            h.hover_at(from);
            h.run();
            h.drag_at(from);
            h.run();
            h.hover_at(from + egui::vec2(14.0, 0.0));
            h.run();
            h.hover_at(onto);
            h.run();
            assert_eq!(document(&h), before, "hover must not modify the module");
            h.drop_at(onto);
            h.run();
            let doc = document(&h);
            assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 3);
            assert_eq!(doc["root"]["children"][index]["type"], "row");
            h.state_mut().ws.as_mut().unwrap().undo().unwrap();
            h.run();
            assert_eq!(document(&h), before);
        }
    }
}

#[test]
fn a_control_dragged_from_the_palette_onto_the_canvas_joins_that_container() {
    for live in [false, true] {
        let key = key();
        let mut h = creator(app(), [1200, 850]);
        h.run();
        if live {
            h.get_by_label("Interact").click();
            h.run();
        }
        let from = h.get_by_label("Row").rect().center();
        let onto = h.get_by_label("Canvas Column").rect().right_bottom() - egui::vec2(12.0, 12.0);
        h.hover_at(from);
        h.run();
        h.drag_at(from);
        h.run();
        h.hover_at(from + egui::vec2(14.0, 0.0));
        h.run();
        h.hover_at(onto);
        h.run();
        h.drop_at(onto);
        h.run();
        let doc = mg_nui::parse(h.state().ws.as_ref().unwrap().module.get(&key).unwrap()).unwrap();
        assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 3);
        assert_eq!(doc["root"]["children"][2]["type"], "row");
    }
}

#[test]
fn source_typed_is_the_module_s_when_its_field_is_left_even_if_it_does_not_read() {
    let key = key();
    let mut h = creator(app(), [1200, 850]);
    h.run();
    let before = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("JUI source").click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).next().unwrap().click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run();
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .next()
        .unwrap()
        .type_text("invalid draft");
    h.run();
    // A draft is the tab's until its field is left: then it is the
    // module's, also one that doesn't read.
    assert_eq!(h.state().ws.as_ref().unwrap().module.get(&key).unwrap(), before);
    h.key_press(egui::Key::Escape);
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let draft = ws.module.get(&key).unwrap().to_vec();
    assert_ne!(draft, before);
    assert!(String::from_utf8_lossy(&draft).contains("invalid draft"));
    ws.undo().unwrap();
    assert_eq!(ws.module.get(&key).unwrap(), before);
    ws.redo().unwrap();
    assert_eq!(ws.module.get(&key).unwrap(), draft);
    h.get_by_label("Design").click();
    h.run();
    h.get_by_label_contains("The window's JSON doesn't read");
}

/// A window whose name leaves no room for its scripts' suffixes, and whose
/// settings are some other text, opens and shows its pages.
#[test]
fn a_sixteen_letter_name_and_settings_that_do_not_read_open() {
    let mut app = app();
    let key = ResKey::parse("sixteen_charname", ResType::JUI).unwrap();
    let module = &mut app.ws.as_mut().unwrap().module;
    module.set(key, serde_json::to_vec(&mg_nui::window()).unwrap());
    module.set(ResKey::new(key.resref, ResType::TXT), b"unrelated text".to_vec());
    app.actions.push(Action::OpenTab(Tab::Nui(key)));
    let mut h = creator(app, [1200, 850]);
    h.get_by_label("Bindings").click();
    h.run();
}

/// Code typed into the window's event script is built without saving it
/// first; another unsaved script still stops the build.
#[test]
fn build_takes_the_window_s_own_scripts_as_typed() {
    let root = mg_testkit::corpus!();
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let other = ResKey::parse("other", ResType::NSS).unwrap();
    m.set(other, b"void main() {}\n".to_vec());
    app.ws = Some(mg_edit::Workspace::new(m));
    app.run(Action::NewNui("typed".into()));
    // (Built or not: whether the scripts are those of the window as it is.)
    let built = |app: &Moonglow| {
        let module = &app.ws.as_ref().unwrap().module;
        let name = |t| ResKey::parse("typed", t).unwrap();
        let window = mg_nui::parse(module.get(&name(ResType::JUI)).unwrap()).unwrap();
        let settings = Settings::parse(module.get(&name(ResType::TXT)).unwrap()).unwrap();
        mg_nui::is_current(module, "typed", &window, &settings)
    };
    let build = Action::GenerateNui { name: "typed".into(), export: false };
    assert!(!built(&app));
    app.run(build.clone());
    assert!(built(&app));

    // Typed in the script's editor, at the Close button's handler.
    let script = ResKey::parse("typed_e", ResType::NSS).unwrap();
    let saved = text(&app, script);
    let mut h = creator(app, [1200, 850]);
    let type_at = |h: &mut Harness<'_, Moonglow>, key: ResKey, at: usize, typed: &str| {
        h.state_mut().actions.push(Action::OpenTab(Tab::Script(key)));
        h.run();
        h.state_mut().script_tools.jump = Some((key, at));
        h.run();
        h.event(egui::Event::Text(typed.into()));
        h.run();
    };
    let at = mg_nui::handler_offset(&saved, "click mg_close").unwrap();
    type_at(&mut h, script, at, "int nTyped = 1; ");
    let typed = h.state().script_text(script).unwrap();
    assert!(typed.contains("int nTyped = 1;") && typed != saved, "{typed}");
    assert_eq!(text(h.state(), script), saved, "typed, not saved");
    h.state_mut().run(build.clone());
    assert_eq!(text(h.state(), script), typed);

    // Another script with unsaved text stops the build, and is named: the
    // window's own is not, being saved by the build before.
    type_at(&mut h, other, 0, "// ");
    h.state_mut().log.entries.clear();
    h.state_mut().run(build);
    let log = &h.state().log.entries;
    assert!(log.iter().any(|(_, m)| m.contains("Save other.nss")), "{log:?}");
    assert!(!log.iter().any(|(_, m)| m.contains("typed_e.nss")), "{log:?}");
}

#[test]
fn export_compiles_the_window_s_files_and_leaves_the_module_s_events() {
    let root = mg_testkit::corpus!();
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(root, None, "en")),
        Box::new(NoDialogs::default()),
    );
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_OnNuiEvent", mg_gff::Value::ResRef(b"my_event".to_vec()));
    m.set_info(&info).unwrap();
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let before = m.get(&ifo).unwrap().to_vec();
    app.ws = Some(mg_edit::Workspace::new(m));
    app.run(Action::NewNui("export_ui".into()));
    app.run(Action::GenerateNui { name: "export_ui".into(), export: true });
    for action in std::mem::take(&mut app.actions) {
        app.run(action);
    }
    let exported = &app.export.as_ref().expect("export opened after compilation").selected;
    assert_eq!(exported.len(), 5);
    for key in exported {
        assert!(app.ws.as_ref().unwrap().module.contains(key));
    }
    assert_eq!(app.ws.as_ref().unwrap().module.get(&ifo).unwrap(), before);
    let revision = app.ws.as_ref().unwrap().revision();
    app.run(Action::GenerateNui { name: "export_ui".into(), export: false });
    assert_eq!(
        app.ws.as_ref().unwrap().revision(),
        revision,
        "unchanged generation must not dirty the module or add an undo step"
    );
    app.ws.as_mut().unwrap().undo().unwrap();
    assert!(
        !app.ws
            .as_ref()
            .unwrap()
            .module
            .contains(&ResKey::parse("export_ui_o", ResType::NCS).unwrap())
    );
}

/// Hand-written or foreign JUI reaches every page and Interact: wrong
/// types, impossible sizes and missing binds are diagnostics, never a panic.
#[test]
fn windows_written_by_hand_reach_every_page_and_interact() {
    let mut deep = json!({"type":"label","value":"bottom"});
    for _ in 0..100 {
        deep = json!({"type":"col","children":[deep]});
    }
    let roots = [
        json!({"type":"col","children":5}),
        json!({"type":"group","children":[]}),
        json!({"type":"group","children":[{"type":"label"},{"type":"label"}],"scrollbars":99}),
        json!({"type":"list","row_template":[[1],[{"type":"label"}],"x",[null,-5.0,true]],
            "row_count":-5,"row_height":0.0,"scrollbars":4}),
        json!({"type":"list","row_template":[[{"type":"check","value":{"bind":"rows"}},0.0,true]],
            "row_count":{"bind":"rows"},"row_height":-25.0}),
        json!({"type":"list","row_template":[[{"type":"label","value":"x"},1e30,false]],
            "row_count":1_000_000_000_000_i64}),
        json!({"type":"row","children":[
            {"type":"combo","elements":"abc","value":"x"},
            {"type":"combo","elements":[[1,"a"],["b"]],"value":99},
            {"type":"options","elements":[1,2],"value":-7,"direction":5},
            {"type":"tabbar","elements":{"bind":"missing"},"value":{"bind":"missing"}},
            {"type":"slider","value":5,"min":10,"max":0,"step":0},
            {"type":"sliderf","value":0.5,"min":0.0,"max":1.0,"step":-1.0},
            {"type":"progress","value":"half"},
            {"type":"textedit","max":0,"value":7,"label":{"strref":-1}},
            {"type":"color_picker","value":"red"},
            {"type":"chart","value":"x"},
            {"type":"chart","value":[{"type":9,"data":["a",null]},5]},
            {"type":"image","value":"","image_region":{"x":-5,"y":1e9,"w":-1,"h":0}},
            {"type":"button_image","label":{"strref":999999999}},
            {"type":"text","value":{"strref":-1},"scrollbars":-3},
            {"type":"label","value":{"bind":7},"text_halign":"left"},
            {"type":"canvas","children":"none"},
            {"type":7},
            {"children":[]},
            {"type":"spacer","width":"wide","margin":1e30,"padding":-5.0,"aspect":0.0},
            {"type":"spacer","width":-100.0,"height":-100.0,"aspect":-1.0}
        ]}),
        json!({"type":"col","draw_list":"x","children":[
            {"type":"spacer","draw_list_scissor":true,"draw_list":[
                {"type":0,"points":[1.0,2.0,3.0]},
                {"type":0,"points":{"bind":"missing"},"arrayBinds":true},
                {"type":1},{"type":2,"rect":"x"},{"type":3,"radius":-1.0,"amin":1e30},
                {"type":4,"text":{"strref":-1},"rect":{"x":0,"y":0,"w":-1,"h":-1}},
                {"type":5,"image":"","rect":{"x":0,"y":0,"w":1e9,"h":1e9}},
                {"type":6},{"type":7,"fill":"yes"},{"type":99},{"type":"x"},5,null
            ]}
        ]}),
        deep,
        json!(5),
    ];
    let windows = roots
        .into_iter()
        .map(|root| {
            let mut window = mg_nui::window();
            window["root"] = root;
            window
        })
        .chain([
            json!({"version":"one","root":{"type":"col","children":[]},"geometry":
                {"x":"a","y":null,"w":-100.0,"h":0.0},"title":{"bind":"missing"},
                "size_constraint":{"x":900.0,"y":900.0,"w":10.0,"h":10.0}}),
            json!({"geometry":{"bind":"geometry"},"root":{"type":"col","children":[]}}),
            json!([]),
            json!(null),
        ]);
    let mut settings = Settings::default();
    settings.bindings.insert("rows".into(), Binding { value: json!(7), ..Default::default() });
    settings
        .bindings
        .insert("geometry".into(), Binding { value: json!("x"), ..Default::default() });
    settings.views.insert("broken".into(), json!({"type":"group","children":7}));
    for window in windows {
        let mut app = app();
        let ws = app.ws.as_mut().unwrap();
        ws.module.set(key(), serde_json::to_vec_pretty(&window).unwrap());
        ws.module.set(ResKey::parse("nui_test", ResType::TXT).unwrap(), settings.bytes());
        let mut h = creator(app, [1200, 850]);
        h.run();
        if let Some(interact) = h.query_by_label("Interact") {
            interact.click();
            h.run();
        }
        for page in ["Bindings", "Events & variants", "Design"] {
            h.get_by_label(page).click();
            h.run();
        }
    }
}

#[test]
fn a_problem_clicked_selects_its_control() {
    let mut h = harness();
    h.run();
    let mut window = document(&h);
    window["root"]["children"][0] = json!({"type":"textedit","label":"Name","max":0});
    h.state_mut().ws.as_mut().unwrap().module.set(key(), serde_json::to_vec(&window).unwrap());
    h.run();
    h.get_by_label("Text input · Name › Max").click();
    h.run();
    assert!(h.query_by_label("Placeholder").is_some(), "the text input's properties show");
}

/// Twenty letters typed are one change, made when the field is left, and
/// one Undo takes them back. The canvas shows them as they are typed.
#[test]
fn text_typed_in_a_field_is_one_undo_step() {
    let mut h = harness();
    h.get_by_label("Canvas Label · Label").click();
    h.run();
    let (before, at) = (document(&h), revision(&h));
    h.get_by_label("Text").click();
    h.run();
    h.get_by_label("Text").type_text(" of twenty letters");
    h.run();
    assert_eq!((document(&h), revision(&h)), (before.clone(), at), "nothing is set at a key");
    h.get_by_label("Canvas Label · Label of twenty letters");
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(revision(&h), at + 1, "one change");
    assert_eq!(document(&h)["root"]["children"][0]["value"], "Label of twenty letters");
    undo(&mut h);
    assert_eq!(document(&h), before);
    h.get_by_label("Canvas Label · Label");
}

/// What is typed in one control's field stays that control's when another
/// control is selected.
#[test]
fn text_typed_for_one_control_is_not_set_on_the_next() {
    let mut h = harness();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    let before = document(&h);
    h.get_by_label("Element ID").click();
    h.run();
    h.get_by_label("Element ID").type_text("_typed");
    h.run();
    h.get_by_label("Canvas Label · Label").click();
    h.run();
    let after = document(&h);
    assert_eq!(after["root"]["children"][0], before["root"]["children"][0], "the label's own");
    assert!(!after.to_string().contains("mg_close_typed_typed"));
}

/// A click in Layers on another control sets what was typed for the control
/// selected before it: the press leaves the field (`widgets::text_begin`),
/// which is still drawn then.
#[test]
fn text_typed_is_set_when_layers_selects_another_control() {
    let mut h = harness();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    h.get_by_label("Element ID").click();
    h.run();
    h.get_by_label("Element ID").type_text("_typed");
    h.run();
    let row = h.get_all_by_label("Label · Label").next().expect("its row in Layers").rect();
    h.hover_at(row.center());
    h.run();
    h.drag_at(row.center());
    h.drop_at(row.center());
    h.run();
    let after = document(&h);
    assert_eq!(after["root"]["children"][1]["id"], "mg_close_typed");
    // (The label is selected now: its text is in Properties.)
    h.get_by_label("Text");
    assert!(after["root"]["children"][0]["id"].is_null(), "the label has no ID of its own");
}

/// A drag across a number is one change, made when it is let go; the number
/// it has reached shows meanwhile.
#[test]
fn a_number_dragged_is_one_undo_step() {
    let mut h = harness();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    let (before, at) = (document(&h), revision(&h));
    let height = before["root"]["children"][1]["height"].as_f64().expect("a button's height");
    let field = h
        .get_all_by_label("Height")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::SpinButton)
        .expect("the height's number field")
        .rect()
        .center();
    h.hover_at(field);
    h.run();
    h.drag_at(field);
    h.run();
    for step in 1..=4 {
        h.hover_at(field + egui::vec2(10.0 * step as f32, 0.0));
        h.run();
        assert_eq!((document(&h), revision(&h)), (before.clone(), at), "nothing is set in a drag");
    }
    h.drop_at(field + egui::vec2(40.0, 0.0));
    h.run();
    assert_eq!(revision(&h), at + 1, "one change");
    let dragged = document(&h)["root"]["children"][1]["height"].as_f64().unwrap();
    assert!(dragged > height, "{dragged} from {height}");
    undo(&mut h);
    assert_eq!(document(&h), before);
}

/// Undo takes back the control that is selected: the next frame draws
/// without it, and nothing is selected.
#[test]
fn a_selection_undo_takes_away_is_cleared() {
    let mut h = harness();
    h.get_by_label("Toggle button").click();
    h.run();
    // (Added last, and selected: Properties shows its ID.)
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 3);
    h.get_by_label("Element ID");
    h.get_by_label("Draw layers").click();
    h.run();
    h.get_by_label("+ Draw primitive").scroll_to_me();
    h.run();
    h.get_by_label("+ Draw primitive").click();
    h.run();
    h.get_by_label("Rectangle").click();
    h.run();
    h.get_by_label("Draw layer 1 · Rectangle").click();
    h.run();
    // The draw layer, then the control.
    for _ in 0..2 {
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    }
    h.run();
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 2);
    assert!(h.query_by_label("Element ID").is_none(), "no control's properties");
    assert!(h.query_by_label("Draw layer 1 · Rectangle").is_none());
    // The pages that don't show the selection draw too.
    for page in ["Bindings", "Events & variants", "Design"] {
        h.get_by_label(page).click();
        h.run();
    }
}

/// The Events & variants page of a window whose Close button does `action`.
fn events_harness(action: mg_nui::Action) -> Harness<'static, Moonglow> {
    let mut app = app();
    let settings = Settings {
        actions: vec![mg_nui::Route::new("click", "mg_close", action)],
        ..Default::default()
    };
    app.ws.as_mut().unwrap().module.set(ResKey::new(key().resref, ResType::TXT), settings.bytes());
    let mut h = creator(app, [1100, 1100]);
    h.get_by_label("Events & variants").click();
    h.run();
    h
}

#[test]
fn the_bindings_page_adds_a_bind_s_change_handler() {
    let mut h = events_harness(mg_nui::Action::Close);
    h.get_by_label("Bindings").click();
    h.run();
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("volume");
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let before = settings(&h);
    h.get_by_label("+ Add handler").click();
    h.run();
    let after = settings(&h);
    assert!(
        after.actions.iter().any(|r| r.event == "watch"
            && r.element == "volume"
            && r.action == mg_nui::Action::Code)
    );
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run();
    assert_eq!(settings(&h), before);
}

#[test]
fn binds_are_renamed_and_deleted_with_their_handlers() {
    let mut h = events_harness(mg_nui::Action::Close);
    h.get_by_label("Bindings").click();
    h.run();
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("volume");
    h.run();
    h.get_by_label("Create").click();
    h.run();
    h.get_by_label("+ Add handler").click();
    h.run();
    h.get_by_label("Name").click();
    h.run();
    h.get_by_label("Name").type_text("_2");
    h.run();
    h.get_by_label("Rename").click();
    h.run();
    let renamed = settings(&h);
    assert!(renamed.bindings.contains_key("volume_2") && !renamed.bindings.contains_key("volume"));
    assert!(renamed.actions.iter().any(|r| r.event == "watch" && r.element == "volume_2"));
    h.get_by_label("Delete").click();
    h.run();
    let deleted = settings(&h);
    assert!(deleted.bindings.is_empty());
    assert!(!deleted.actions.iter().any(|r| r.event == "watch"));
}

/// A drag that begins on the editor's page moves nothing: the window the
/// editor is in stays where it is.
#[test]
fn a_drag_in_the_editor_does_not_move_its_window() {
    let mut h = events_harness(mg_nui::Action::Close);
    let place = |h: &Harness<'_, Moonglow>| {
        (h.get_by_label("Swap layout variants").rect(), h.get_by_label("Bindings").rect())
    };
    let before = place(&h);
    let from = before.0.center();
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(60.0, 30.0));
    h.run();
    h.drop_at(from + egui::vec2(60.0, 30.0));
    h.run();
    assert_eq!(place(&h), before);
    h.get_by_label("Bindings").click();
    h.run();
    h.get_by_label("+ Create bind");
}

/// Where each control of each window of a module is, relative to its
/// window, as the preview lays it out: where to click in the game client.
/// A tool run by hand, on the module NUI_PREVIEW_MODULE (read, never
/// written), to the file NUI_LAYOUT_JSON.
#[test]
#[ignore]
fn a_module_s_windows_are_laid_out_as_json() {
    let module = std::env::var("NUI_PREVIEW_MODULE").expect("NUI_PREVIEW_MODULE");
    let output = std::env::var("NUI_LAYOUT_JSON").expect("NUI_LAYOUT_JSON");
    let path = std::path::Path::new(&module);
    let keys: Vec<_> = mg_module::Module::open(path)
        .unwrap()
        .keys()
        .filter(|k| k.restype == ResType::JUI)
        .copied()
        .collect();
    let mut all = serde_json::Map::new();
    for key in keys {
        let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
        app.ws = Some(mg_edit::Workspace::new(mg_module::Module::open(path).unwrap()));
        app.actions.push(Action::OpenTab(Tab::Nui(key)));
        let mut h = creator(app, [1300, 1150]);
        h.run_steps(3);
        h.get_by_value("Editor").click();
        h.run();
        h.get_by_label("Preview only").click();
        h.run();
        h.get_by_label("100%").click();
        h.run_steps(3);
        // A window without a title bar has nothing to measure from.
        let Some(window) = h.query_by_label("Canvas window").map(|n| n.rect()) else { continue };
        let controls: Vec<_> = h
            .query_all_by_label_contains("Canvas ")
            .filter_map(|n| {
                let label = n.accesskit_node().label()?;
                let r = n.rect();
                Some(json!({
                    "label": label,
                    "rect": [r.min.x - window.min.x, r.min.y - window.min.y, r.width(), r.height()],
                }))
            })
            .collect();
        all.insert(
            key.resref.to_string(),
            json!({"window": [window.width(), window.height()], "controls": controls}),
        );
    }
    std::fs::write(output, serde_json::to_string_pretty(&all).unwrap()).unwrap();
}

/// Bindings › Client delivery turns the event function on: the settings
/// say so, as one Undo step.
#[test]
fn the_event_function_is_chosen_under_client_delivery() {
    let mut h = harness();
    h.get_by_label("Bindings").click();
    h.run();
    h.get_by_label("Client delivery").click();
    h.run();
    assert!(!settings(&h).event_function);
    h.get_by_label("Events go to the module's event script (a function to call)").click();
    h.run();
    assert!(settings(&h).event_function);
    undo(&mut h);
    assert!(!settings(&h).event_function);
}

/// New NUI Window in a nasher project whose include takes in scripts and
/// JSON only: the log says the window's files would be left out, with the
/// line to add; in one that takes them in, nothing is said.
#[test]
fn a_new_window_in_a_project_that_leaves_it_out_says_the_line() {
    let cfg = "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
               [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n";
    for (include, said) in [("{nss,json}", true), ("{nss,json,jui,txt}", false)] {
        let root = mg_testkit::scratch_dir(&format!("nui-nasher-new-{said}"));
        std::fs::write(root.join("nasher.cfg"), cfg.replace("{nss,json}", include)).unwrap();
        let mut m = mg_module::Module::new();
        m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        let location =
            mg_module::ModuleLocation::Project { root: root.clone(), target: "m".into() };
        m.save_as(&location).unwrap();
        let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
        app.ws = Some(mg_edit::Workspace::new(mg_module::Module::open(&root).unwrap()));
        app.run(Action::NewNui("crier".into()));
        let line = "include = \"src/**/*.{jui,txt}\"";
        let warned = app.log.entries.iter().any(|(_, s)| s.contains(line));
        assert_eq!(warned, said, "{include}: {:?}", app.log.entries);
        assert!(
            app.ws
                .as_ref()
                .unwrap()
                .module
                .contains(&ResKey::parse("crier", ResType::JUI).unwrap())
        );
    }
}

/// The town crier's window (`crates/mg-nui/tests/data`): a label, a text
/// box, a label from a bind and a row of two buttons, one off by a bind.
fn crier() -> mg_module::Module {
    fixture("crier")
}

/// A window of `crates/mg-nui/tests/data` in a module of its own.
fn fixture(name: &str) -> mg_module::Module {
    let mut m = mg_module::Module::new();
    for ty in [ResType::JUI, ResType::TXT] {
        let key = ResKey::parse(name, ty).unwrap();
        m.set(key, std::fs::read(fixture_file(&key.to_string())).unwrap());
    }
    m
}

fn fixture_file(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mg-nui/tests/data").join(file)
}

/// A window's binds as a script would set them: `NAME.binds.json`.
fn fixture_binds(name: &str) -> std::collections::BTreeMap<String, Value> {
    serde_json::from_slice(&std::fs::read(fixture_file(&format!("{name}.binds.json"))).unwrap())
        .unwrap()
}

/// The town crier's recent cries (`crates/mg-nui/tests/data/cries.*`): a
/// list whose rows come of binds, in a swap layout with a variant.
fn cries_window(m: &mg_module::Module) -> mg_ui::nui_render::Window<'_> {
    mg_ui::nui_render::Window { name: "cries", ..crier_window(m) }
}

/// Wrapped text that runs past the bottom of a box that doesn't scroll is
/// said, with the height that shows it all (the Domains of Dread Design
/// session's survey: four windows' text cut after a line or two, which the
/// layout didn't say). At that height it fits; with a scrollbar it isn't
/// cut.
#[test]
fn text_cut_at_the_bottom_of_its_box_is_said_with_the_height_it_needs() {
    use mg_ui::nui_render::{Options, layout};
    let long = "Performance points: 5 of 5. Performances known: 0 of 3.";
    let text = |id: &str, height: f32, scrollbars: i64| {
        json!({"type": "text", "id": id, "value": long, "width": 150.0, "height": height,
               "scrollbars": scrollbars})
    };
    let laid = |children: Value| {
        let mut doc = mg_nui::window();
        doc["root"]["children"] = children;
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse("cut", ResType::JUI).unwrap(), serde_json::to_vec(&doc).unwrap());
        let w = mg_ui::nui_render::Window { name: "cut", ..crier_window(&m) };
        layout(&w, &Options::default()).unwrap()
    };
    let of = |l: &mg_ui::nui_render::Layout, id: &str| {
        l.elements.iter().find(|e| e.id.as_deref() == Some(id)).unwrap().clone()
    };
    let l = laid(json!([text("short", 40.0, 0), text("scrolls", 40.0, 4)]));
    let short = of(&l, "short");
    assert_eq!(short.cut_text, [long]);
    let needs = short.needs_height.expect("the height it needs");
    assert!(needs > 40.0, "{needs}");
    let scrolls = of(&l, "scrolls");
    assert!(scrolls.cut_text.is_empty() && scrolls.needs_height.is_none(), "{scrolls:?}");
    // At the height said, all of it shows.
    let l = laid(json!([text("short", needs, 0)]));
    let fits = of(&l, "short");
    assert!(fits.cut_text.is_empty() && fits.needs_height.is_none(), "{fits:?}");
    assert_eq!(fits.area.h, needs);
}

/// A text field's text in the layout is what it shows: its value, or its
/// placeholder (its label) while that is empty (the Design session's
/// question: the placeholder was given though the binds filled it).
#[test]
fn a_text_field_s_text_is_its_value_or_else_its_placeholder() {
    use mg_ui::nui_render::{Options, layout};
    let m = crier();
    let w = crier_window(&m);
    let message = |o: &Options| {
        let l = layout(&w, o).unwrap();
        l.elements.iter().find(|e| e.id.as_deref() == Some("message")).unwrap().text.clone()
    };
    let empty = Options { binds: [("message".to_owned(), json!(""))].into(), ..Default::default() };
    assert_eq!(message(&empty).as_deref(), Some("Write your message..."));
    let o = Options { binds: fixture_binds("crier"), ..Default::default() };
    assert_eq!(message(&o).as_deref(), Some("Hear ye: the market opens at dawn on the morrow."));
}

/// Off the screen a window can be drawn otherwise than as it opens: a
/// control in a state of its own, or every one; binds set as a script sets
/// them, a list's rows from their arrays; a swap layout variant in place
/// of what its group holds. Where they don't fit the window, said.
#[test]
fn a_window_is_laid_out_in_the_states_binds_and_variant_asked() {
    use mg_ui::nui_render::{Look, Options, layout};
    let m = fixture("cries");
    let w = cries_window(&m);
    // As it opens: one row, its first values.
    let first = layout(&w, &Options::default()).unwrap();
    let rows = |l: &mg_ui::nui_render::Layout| {
        l.elements.iter().filter(|e| e.id.as_deref() == Some("read")).count()
    };
    assert_eq!(rows(&first), 1);
    assert!(first.elements.iter().all(|e| e.state.is_none()), "{first:#?}");
    // With the binds a character would see, and one control hovered: each
    // row's Read button, which has that ID.
    let binds = fixture_binds("cries");
    let o = Options {
        binds: binds.clone(),
        states: [(Some("read".to_owned()), Look::Hover)].into(),
        ..Default::default()
    };
    let l = layout(&w, &o).unwrap();
    assert_eq!(rows(&l), 4);
    let texts: Vec<_> = l
        .elements
        .iter()
        .filter(|e| e.path.ends_with("row_template/1/0"))
        .map(|e| (e.row, e.text.clone().unwrap()))
        .collect();
    assert_eq!(texts[3], (Some(3), "News of my brother, lost in the fog?".into()));
    for e in &l.elements {
        let hovered = e.id.as_deref() == Some("read");
        assert_eq!(e.state, hovered.then_some("hover"), "{e:?}");
    }
    // Disabled: a list, and what is in it with it; every control hovered
    // but one given its own.
    let o = Options {
        states: [(Some("cries".to_owned()), Look::Disabled), (None, Look::Hover)].into(),
        ..o
    };
    let l = layout(&w, &o).unwrap();
    for e in &l.elements {
        let inside = e.path.starts_with("/root/children/1/children/0");
        assert_eq!(e.state, Some(if inside { "disabled" } else { "hover" }), "{e:?}");
    }
    // The variant in its group: its controls where it puts them.
    let o = Options {
        variant: Some("details".into()),
        binds,
        states: [(Some("back".to_owned()), Look::Pressed)].into(),
        ..Default::default()
    };
    let l = layout(&w, &o).unwrap();
    let back = l.elements.iter().find(|e| e.id.as_deref() == Some("back")).unwrap();
    assert_eq!(
        (back.path.as_str(), back.state),
        ("/root/children/1/children/0/children/1", Some("pressed"))
    );
    assert!(l.elements.iter().all(|e| e.id.as_deref() != Some("cries")));
    let text = l.elements.iter().find(|e| e.kind == "text").unwrap();
    assert!(text.text.as_deref().unwrap().starts_with("Wolves seen"), "{text:?}");
    // (Without the game the skin is said to be missing: nothing of the binds.)
    assert!(!l.issues.iter().any(|i| i.starts_with("bind")), "{:?}", l.issues);
    // A control its bind disables stays so, whatever is asked.
    let crier = crier();
    let o = Options { states: [(None, Look::Hover)].into(), ..Default::default() };
    let l = layout(&crier_window(&crier), &o).unwrap();
    let state = |id: &str| l.elements.iter().find(|e| e.id.as_deref() == Some(id)).unwrap().state;
    assert_eq!((state("send"), state("cancel")), (Some("disabled"), Some("hover")));

    // What can't be: said, not drawn.
    let said = |o: Options| layout(&w, &o).unwrap_err().to_string();
    let o = Options { variant: Some("cover".into()), ..Default::default() };
    assert_eq!(said(o), "no swap layout variant \"cover\": the window's are \"details\"");
    let o =
        Options { states: [(Some("nope".to_owned()), Look::Hover)].into(), ..Default::default() };
    assert_eq!(
        said(o),
        "no control has the ID \"nope\": the IDs are \"cries\", \"pages\", \"read\""
    );
    // (Not in the window as shown: the variant's own, without it.)
    let o =
        Options { states: [(Some("back".to_owned()), Look::Hover)].into(), ..Default::default() };
    assert!(said(o).starts_with("no control has the ID \"back\""));
    let o = Options {
        binds: [("nope".to_owned(), json!(1)), ("cry_count".to_owned(), json!("four"))].into(),
        ..Default::default()
    };
    assert_eq!(
        (layout(&w, &o).unwrap().issues.into_iter())
            .filter(|i| i.starts_with("bind"))
            .collect::<Vec<_>>(),
        [
            "bind \"cry_count\": given a string where its first value is a number",
            "bind \"nope\": no property of the window reads it"
        ]
    );
    // A variant no handler shows and made for no group has nowhere to go.
    let mut lost = fixture("cries");
    let txt = ResKey::parse("cries", ResType::TXT).unwrap();
    let mut s = Settings::parse(lost.get(&txt).unwrap()).unwrap();
    s.actions.clear();
    s.view_targets.clear();
    lost.set(txt, s.bytes());
    let o = Options { variant: Some("details".into()), ..Default::default() };
    let e = layout(&cries_window(&lost), &o).unwrap_err().to_string();
    assert!(e.starts_with("the variant \"details\" has no group to be shown in"), "{e}");
}

fn crier_window(m: &mg_module::Module) -> mg_ui::nui_render::Window<'_> {
    mg_ui::nui_render::Window { module: m, resman: None, game: None, name: "crier" }
}

/// `mg nui layout` is the editor's canvas: each control it reports is where
/// the canvas, at 100%, puts it, measured from the window's corner.
#[test]
fn the_layout_off_the_screen_is_the_canvas_s() {
    let m = crier();
    let layout =
        mg_ui::nui_render::layout(&crier_window(&m), &mg_ui::nui_render::Options::default())
            .unwrap();
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.ws = Some(mg_edit::Workspace::new(m.clone()));
    app.actions.push(Action::OpenTab(Tab::Nui(ResKey::parse("crier", ResType::JUI).unwrap())));
    let mut h = creator(app, [1300, 900]);
    h.run_steps(3);
    h.get_by_value("Editor").click();
    h.run();
    h.get_by_label("Preview only").click();
    h.run();
    h.get_by_label("100%").click();
    h.run_steps(3);
    let window = h.get_by_label("Canvas window").rect().min;
    let round = |v: [f32; 4]| v.map(|n| (n * 2.0).round() / 2.0);
    let mut canvas: Vec<[f32; 4]> = h
        .query_all_by_label_contains("Canvas ")
        .filter(|n| n.accesskit_node().label().is_some_and(|l| l != "Canvas window"))
        .map(|n| {
            let r = n.rect();
            round([r.min.x - window.x, r.min.y - window.y, r.width(), r.height()])
        })
        .collect();
    let mut reported: Vec<[f32; 4]> = layout
        .elements
        .iter()
        .filter(|e| !e.zero_sized && !e.out_of_view)
        .map(|e| round([e.area.x, e.area.y, e.area.w, e.area.h]))
        .collect();
    let order = |a: &[f32; 4], b: &[f32; 4]| a.partial_cmp(b).unwrap();
    canvas.sort_by(order);
    reported.sort_by(order);
    assert_eq!(reported.len(), 7, "{layout:#?}");
    assert_eq!(canvas, reported);
    assert_eq!((layout.width, layout.height), (420.0, 300.0));
}

/// What `mg nui layout` says of a window: a spacer with no room, the binds
/// a control reads and the text it shows, and controls a smaller window
/// cuts off.
#[test]
fn the_layout_says_what_does_not_fit() {
    let mut m = crier();
    let options = mg_ui::nui_render::Options::default();
    let layout = mg_ui::nui_render::layout(&crier_window(&m), &options).unwrap();
    let at = |path: &str| layout.elements.iter().find(|e| e.path == path).unwrap();
    let spacer = at("/root/children/3/children/0");
    assert!(spacer.zero_sized && spacer.out_of_view, "{spacer:?}");
    let message = at("/root/children/1");
    assert_eq!(
        (message.binds.as_slice(), message.id.as_deref()),
        (["message".to_owned()].as_slice(), Some("message"))
    );
    let fee = at("/root/children/2");
    assert_eq!(fee.text.as_deref(), Some("The fee is 5 silver pieces."));
    let send = at("/root/children/3/children/1");
    assert_eq!((send.area.w, send.area.h), (100.0, 34.0));
    assert_eq!(send.binds, ["can_send"]);
    assert!(send.overlaps.is_empty() && !send.clipped && !send.hidden);
    // A window made smaller: Cancel runs past it and is cut off.
    let small = mg_ui::nui_render::Options { size: Some([200.0, 150.0]), ..Default::default() };
    let small = mg_ui::nui_render::layout(&crier_window(&m), &small).unwrap();
    assert_eq!((small.width, small.height), (200.0, 150.0));
    let cancel = small.elements.iter().find(|e| e.id.as_deref() == Some("cancel")).unwrap();
    assert!(cancel.clipped || cancel.out_of_view, "{cancel:?}");
    // A window that isn't there, or isn't JSON, is an error, not a panic.
    let missing = mg_ui::nui_render::Window { name: "nowhere", ..crier_window(&m) };
    let said = mg_ui::nui_render::layout(&missing, &options).unwrap_err();
    assert_eq!(said.to_string(), "nowhere.jui is not in the module");
    m.set(ResKey::parse("crier", ResType::JUI).unwrap(), b"{\"version\": 1, \"root\":".to_vec());
    assert!(mg_ui::nui_render::layout(&crier_window(&m), &options).is_err());
}

/// The picture is the window at its scale, drawn in the frame the layout
/// comes from: a button's place has its frame, the body beside the column
/// is the window's background, and twice the scale is twice the pixels.
/// Drawn in a state, with binds or a variant, the picture is the layout
/// with the same options: a hovered Cancel changes Cancel's pixels and no
/// others, and a list drawn from the binds' arrays has a row for each,
/// each row's button drawn.
#[test]
fn a_window_drawn_off_the_screen_is_where_its_layout_says_in_any_state() {
    use mg_ui::nui_render::{Look, Options, layout, render};
    mg_testkit::gpu::hold();
    let Some(gpu) = mg_render::Gpu::headless() else {
        eprintln!("skipped: no GPU adapter");
        return;
    };
    // With the game's skin and fonts when it is installed.
    let game = mg_testkit::nwn_root().and_then(|root| {
        mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).ok()
    });
    let m = crier();
    let window = mg_ui::nui_render::Window { resman: game.as_ref(), ..crier_window(&m) };
    for scale in [1.0, 2.0] {
        let options = Options { scale, ..Default::default() };
        let picture = render(&gpu, &window, &options).unwrap();
        let laid = layout(&window, &options).unwrap();
        assert_eq!(picture.layout, laid);
        let size = ((420.0 * scale) as u32, (300.0 * scale) as u32);
        assert_eq!((picture.width, picture.height), size);
        assert_eq!(picture.rgba.len(), (picture.width * picture.height * 4) as usize);
        let pixel = |x: f32, y: f32| {
            let (x, y) = ((x * scale) as u32, (y * scale) as u32);
            let i = ((y * picture.width + x) * 4) as usize;
            [picture.rgba[i], picture.rgba[i + 1], picture.rgba[i + 2], picture.rgba[i + 3]]
        };
        // The body right of the column has nothing in it: one color.
        let empty = pixel(330.0, 150.0);
        for (x, y) in [(300.0, 100.0), (380.0, 200.0), (260.0, 260.0)] {
            assert_eq!(pixel(x, y), empty, "{x}, {y} at {scale}");
        }
        // Cancel's button is drawn where the layout puts it.
        let cancel = laid.elements.iter().find(|e| e.id.as_deref() == Some("cancel")).unwrap();
        let a = cancel.area;
        let drawn = (0..a.w as u32)
            .flat_map(|x| (0..a.h as u32).map(move |y| (x, y)))
            .filter(|&(x, y)| pixel(a.x + x as f32, a.y + y as f32) != empty)
            .count();
        assert!(drawn as f32 > a.w * a.h * 0.1, "{drawn} pixels of Cancel drawn at {scale}");
        let png = picture.png();
        assert_eq!(&png[1..4], b"PNG");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), picture.width);
    }

    let plain = render(&gpu, &window, &Options::default()).unwrap();
    let o =
        Options { states: [(Some("cancel".to_owned()), Look::Hover)].into(), ..Default::default() };
    let hovered = render(&gpu, &window, &o).unwrap();
    assert_eq!(hovered.layout, layout(&window, &o).unwrap());
    let cancel =
        hovered.layout.elements.iter().find(|e| e.id.as_deref() == Some("cancel")).unwrap();
    let a = cancel.area;
    let mut changed = 0;
    for (i, (p, h)) in plain.rgba.chunks(4).zip(hovered.rgba.chunks(4)).enumerate() {
        if p != h {
            let (x, y) = ((i as u32 % plain.width) as f32, (i as u32 / plain.width) as f32);
            assert!(
                x >= a.x && x < a.x + a.w && y >= a.y && y < a.y + a.h,
                "{x}, {y} changed, outside Cancel ({a:?})"
            );
            changed += 1;
        }
    }
    assert!(changed > 0, "hovering Cancel changed nothing");

    // The cries' list from the binds: four rows, with their buttons drawn.
    let m = fixture("cries");
    let window = mg_ui::nui_render::Window { resman: game.as_ref(), ..cries_window(&m) };
    let o = Options {
        binds: fixture_binds("cries"),
        states: [(Some("read".to_owned()), Look::Hover)].into(),
        ..Default::default()
    };
    let picture = render(&gpu, &window, &o).unwrap();
    assert_eq!(picture.layout, layout(&window, &o).unwrap());
    let pixel = |x: f32, y: f32| {
        let i = ((y as u32 * picture.width + x as u32) * 4) as usize;
        picture.rgba[i..i + 4].to_vec()
    };
    let buttons: Vec<_> =
        picture.layout.elements.iter().filter(|e| e.id.as_deref() == Some("read")).collect();
    assert_eq!(buttons.len(), 4);
    let empty = pixel(400.0, 300.0);
    for b in buttons {
        let a = b.area;
        let drawn = (0..a.w as u32)
            .flat_map(|x| (0..a.h as u32).map(move |y| (x, y)))
            .filter(|&(x, y)| pixel(a.x + x as f32, a.y + y as f32) != empty)
            .count();
        assert!(drawn as f32 > a.w * a.h * 0.1, "row {:?}: {drawn} pixels", b.row);
    }
    // The variant: drawn as laid out too.
    let o = Options { variant: Some("details".into()), states: Default::default(), ..o };
    let picture = render(&gpu, &window, &o).unwrap();
    assert_eq!(picture.layout, layout(&window, &o).unwrap());
}

/// What `mg nui schema --layout` says of widths is how the preview lays
/// controls out: one without a width keeps 150 points or takes its
/// column's, a column is as wide as its widest control however wide the
/// window, and the window's width less 20 fills it.
#[test]
fn controls_without_a_width_are_as_the_layout_form_says() {
    use mg_ui::nui_render::{Options, layout};
    let about = mg_nui::controls()["description"].as_str().unwrap().to_owned();
    let mut column = Vec::new();
    for ty in mg_nui::ELEMENTS {
        if !matches!(*ty, "col" | "row" | "group" | "spacer" | "options" | "tabbar") {
            column.push(mg_nui::template(ty));
        }
    }
    let mut doc = mg_nui::window();
    doc["geometry"]["w"] = json!(600.0);
    doc["root"]["children"] = json!(column);
    let mut m = mg_module::Module::new();
    let key = |ty| ResKey::parse("widths", ty).unwrap();
    m.set(key(ResType::JUI), serde_json::to_vec(&doc).unwrap());
    let w = mg_ui::nui_render::Window { name: "widths", ..crier_window(&m) };
    let l = layout(&w, &Options::default()).unwrap();
    let width = |kind: &str| l.elements.iter().find(|e| e.kind == kind).unwrap().area.w;
    // The widest: the color picker's 300, and its column 304.
    assert_eq!(width("col"), 304.0);
    for kind in ["button", "button_image", "button_select", "check", "combo", "slider", "sliderf"] {
        assert_eq!(width(kind), 150.0, "{kind}");
        assert!(about.contains(kind), "{kind}");
    }
    // Text and a text input as a label (NWN EE 8193.37, the Domains of
    // Dread's windows: a text across a 460 column, a description field
    // filling its 468 one).
    for kind in ["label", "text", "textedit", "image", "progress", "list", "chart", "color_picker"]
    {
        assert_eq!(width(kind), 300.0, "{kind}");
    }
    assert!(about.contains("A label, text, textedit, image"));
    // And they count nothing toward their column: beside a 120 button, a
    // text is 120 wide, and wraps there (the game's "You do not shout.").
    let mut doc = mg_nui::window();
    doc["root"]["children"] = json!([
        {"type": "text", "value": "You do not shout.", "height": 40.0, "scrollbars": 0},
        {"type": "button", "label": "Close", "width": 120.0, "height": 30.0}
    ]);
    let mut m = mg_module::Module::new();
    m.set(key(ResType::JUI), serde_json::to_vec(&doc).unwrap());
    let w = mg_ui::nui_render::Window { name: "widths", ..crier_window(&m) };
    let l = layout(&w, &Options::default()).unwrap();
    assert_eq!((l.elements[0].area.w, l.elements[1].area.w), (124.0, 120.0));
    assert!(about.contains("color_picker 300"));
    // A new window's: 154 wide, its Close button 150, at any width.
    let mut m = mg_module::Module::new();
    m.set(key(ResType::JUI), serde_json::to_vec(&mg_nui::window()).unwrap());
    let w = mg_ui::nui_render::Window { name: "widths", ..crier_window(&m) };
    for size in [None, Some([900.0, 300.0])] {
        let l = layout(&w, &Options { size, ..Default::default() }).unwrap();
        assert_eq!((l.elements[0].area.w, l.elements[2].area.w), (154.0, 150.0), "{size:?}");
    }
    assert!(about.contains("a new window's column is 154 wide, its Close button 150"));
    // A label the window's width less 20 fills the window's column.
    let mut doc = mg_nui::window();
    doc["root"]["children"][0]["width"] = json!(400.0);
    m.set(key(ResType::JUI), serde_json::to_vec(&doc).unwrap());
    let w = mg_ui::nui_render::Window { name: "widths", ..crier_window(&m) };
    let l = layout(&w, &Options::default()).unwrap();
    assert_eq!((l.width, l.elements[0].area.w, l.body.w), (420.0, l.body.w, 404.0));
    assert!(about.contains("the window's width less 20 fills its root column"));
}
