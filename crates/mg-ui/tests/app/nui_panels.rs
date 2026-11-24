//! The NUI designer's header and docks (M4's U18; the NUI board): the
//! header on the kit's row (the pages' switch, Build & Compile, Export…,
//! More with Import… and How to Open It…, Interact, Reset, View, OK and
//! Cancel), its View popover; Components over Properties over Bindings in
//! the Right Dock (a floating designer's window's right pane) and Layers in
//! the Outliner; and what holds them: Properties a panel in the narrow
//! layout, its combo boxes opening in it; the panels following the designer
//! in front; one Undo step an edit and Cancel setting the panels' edits
//! back; typed text keeping to its own control; no copy of the window in a
//! frame that changes nothing; every panel fitting the dock from its least
//! width; the window coming to rest; records from before reading without a
//! fault.

use crate::common::*;
use mg_nui::Settings;
use mg_ui::Panel;
use mg_ui::nui_names::{self, NUI_HEADER};
use serde_json::{Value, json};

/// The window of ours: a column with a label, a text input and a row of
/// two buttons (Send and Cancel), its settings with a bind.
fn our_window() -> Value {
    let mut w = mg_nui::window();
    w["title"] = json!("The Town Crier");
    w["root"]["children"] = json!([
        {"type":"label","id":"title","value":"Hear ye","height":30.0,"text_halign":0,"text_valign":0},
        {"type":"textedit","id":"message","label":"Write your message","value":{"bind":"message"},"height":30.0},
        {"type":"row","height":40.0,"children":[
            {"type":"button","id":"send","label":"Send","height":34.0,"width":100.0},
            {"type":"button","id":"cancel","label":"Cancel","height":34.0,"width":100.0}
        ]}
    ]);
    w
}

fn our_settings() -> Settings {
    let mut s = Settings::default();
    s.bindings.insert("message".into(), mg_nui::Binding { value: json!(""), ..Default::default() });
    s
}

fn key() -> ResKey {
    ResKey::parse("crier", ResType::JUI).unwrap()
}

/// The module's window, read.
fn document(h: &Harness<'_, Moonglow>) -> Value {
    mg_nui::parse(h.state().ws.as_ref().unwrap().module.get(&key()).unwrap()).unwrap()
}

/// A module of ours in `mode`, in a window `width` by 900, with
/// [`our_window`] open in its designer.
fn designer(mode: Mode, width: f32) -> Harness<'static, Moonglow> {
    designer_of(mode, width, our_window())
}

/// [`designer`] of `window`.
fn designer_of(mode: Mode, width: f32, window: Value) -> Harness<'static, Moonglow> {
    designer_with(mode, width, window, our_settings())
}

/// [`designer`] of `window` and its `settings`.
fn designer_with(
    mode: Mode,
    width: f32,
    window: Value,
    settings: Settings,
) -> Harness<'static, Moonglow> {
    let mut app = app_in(mode, Vec::new());
    let mut module = Module::new();
    module.set(key(), serde_json::to_vec_pretty(&window).unwrap());
    module.set(ResKey::new(key().resref, ResType::TXT), settings.bytes());
    app.ws = Some(mg_edit::Workspace::new(module));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
    let mut h = harness(app, width, 900.0);
    settle(&mut h);
    h
}

/// The control `canvas` ("Button · Send") chosen on the canvas.
#[track_caller]
fn choose(h: &mut Harness<'_, Moonglow>, canvas: &str) {
    h.get_by_label(&format!("Canvas {canvas}")).click();
    settle(h);
}

/// The designer's header's control `name`, through More where it went.
#[track_caller]
fn nui_tool<'t>(h: &'t mut Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    use mg_uikit::header::{self, Place};
    let rec = header::records(&h.ctx, NUI_HEADER).into_iter().next().expect("the header");
    let place = rec.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p);
    static MORE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let more = MORE.get_or_init(|| header::more_name(NUI_HEADER)).as_str();
    if place == Some(Place::More) && h.query_by_label(more).is_none() {
        h.get_by_label(NUI_HEADER).get_by_label(header::MORE).click();
        settle(h);
    }
    let h: &'t Harness<'_, Moonglow> = h;
    match place {
        Some(Place::More) => h.get_by_label(more).get_by_label(name),
        Some(Place::Row | Place::Icon) => h.get_by_label(NUI_HEADER).get_by_label(name),
        other => panic!("{name} in the header: {other:?} ({:?})", rec.places),
    }
}

/// The page `name` chosen in the header's switch.
#[track_caller]
fn page(h: &mut Harness<'_, Moonglow>, name: &str) {
    nui_tool(h, nui_names::PAGE).get_by_label(name).click();
    settle(h);
}

/// Whether `n` is a combo box named `name`.
fn combo<'a>(name: &'a str) -> impl Fn(&egui_kittest::kittest::AccessKitNode<'_>) -> bool + 'a {
    move |a| a.role() == egui::accesskit::Role::ComboBox && a.label().as_deref() == Some(name)
}

/// The first text field named `name`, brought into its scroll's sight.
#[track_caller]
fn field_in_sight(h: &mut Harness<'_, Moonglow>, name: &str) -> egui::Rect {
    let field = |a: &egui_kittest::kittest::AccessKitNode<'_>| {
        a.role() == egui::accesskit::Role::TextInput && a.label().as_deref() == Some(name)
    };
    h.get_all_by(field).next().unwrap_or_else(|| panic!("no field {name}")).scroll_to_me();
    settle(h);
    h.get_all_by(field).next().unwrap().rect()
}

/// Types `text` at the end of the field named `name` (clicked, End).
#[track_caller]
fn type_in(h: &mut Harness<'_, Moonglow>, name: &str, text: &str) {
    let at = field_in_sight(h, name);
    h.get_all_by_role(egui::accesskit::Role::TextInput).find(|n| n.rect() == at).unwrap().click();
    settle(h);
    h.key_press(egui::Key::End);
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.rect() == at)
        .unwrap()
        .type_text(text);
    h.run();
}

/// The Right Dock shows Properties (unfolded on it under 1400, where its
/// sections are one bar).
fn properties_in_front(h: &mut Harness<'_, Moonglow>) {
    let tab = |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::Tab;
    if h.query_all_by_label("Properties").any(|n| tab(&n)) {
        h.get_all_by_label("Properties").find(tab).unwrap().click();
        settle(h);
    }
}

/// The designer's editor area as its header spans it (the kit's
/// `EditorHeader`, across the area).
#[track_caller]
fn editor_rect(h: &Harness<'_, Moonglow>) -> egui::Rect {
    let records = mg_uikit::header::records(&h.ctx, NUI_HEADER);
    records.first().expect("the designer's header").row
}

/// Under 760 points of editor the Properties are a panel, not a menu (the
/// plan's Settles: a combo box opened in Properties chose without closing
/// anything): docked, the Right Dock's (one bar beside an editor under 760
/// points); floating, its window's right pane. A field in it edits the
/// control; a combo box in it (Text Halign) opens, its choice is set, and
/// Properties is still there.
#[test]
fn the_narrow_layout_s_properties_are_a_panel() {
    for (mode, width) in [(Mode::Tabs, 1000.0), (Mode::Floating, 1000.0)] {
        let mut h = designer(mode, width);
        choose(&mut h, "Label · Hear ye");
        properties_in_front(&mut h);
        let what = format!("{mode:?}");
        // (The editor measured by its header, across its area: a tab's
        // rect is its title's.)
        let editor = editor_rect(&h);
        assert!(editor.width() < 760.0, "{what}: the editor {editor:?}");
        if mode == Mode::Tabs {
            assert_eq!(h.state().right_dock_shows(), Some(Panel::Properties), "{what}");
        }
        type_in(&mut h, "Text", "!");
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(document(&h)["root"]["children"][0]["value"], "Hear ye!", "{what}");
        // A combo box in it opens, and chooses, closing nothing.
        h.get_all_by_label("Appearance").next().unwrap().scroll_to_me();
        settle(&mut h);
        h.get_all_by_label("Appearance").next().unwrap().click();
        settle(&mut h);
        h.get_all_by(combo("Text halign")).next().expect("its combo box").scroll_to_me();
        settle(&mut h);
        h.get_all_by(combo("Text halign")).next().unwrap().click();
        settle(&mut h);
        h.get_by_label("Right").click();
        settle(&mut h);
        assert_eq!(document(&h)["root"]["children"][0]["text_halign"], 2, "{what}: chosen");
        assert!(h.query_by_label("Element ID").is_some(), "{what}: Properties still there");
        if mode == Mode::Tabs {
            assert_eq!(h.state().right_dock_shows(), Some(Panel::Properties), "{what}");
        }
    }
}

/// The control chosen in the designer of [`key`].
fn chosen(h: &Harness<'_, Moonglow>) -> String {
    h.state().nui_chosen(&key()).expect("the designer drawn")
}

/// Whether `n` is in the Outliner (the group named so: Layers' rows, not a
/// component's button or the canvas's control of the same words).
fn in_outliner(n: &egui_kittest::Node<'_>) -> bool {
    let mut up = n.accesskit_node().parent();
    while let Some(p) = up {
        if p.label().as_deref() == Some(mg_ui::OUTLINER) {
            return true;
        }
        up = p.parent();
    }
    false
}

/// Layers' row `words` ("Button · Send"), in the Outliner.
#[track_caller]
fn layer<'t>(h: &'t Harness<'_, Moonglow>, words: &'t str) -> egui_kittest::Node<'t> {
    let found = h.get_all_by_label(words).find(in_outliner);
    found.unwrap_or_else(|| panic!("no row {words} in the Outliner"))
}

/// A component's tip says where a drag adds it by the panel's name, the
/// Outliner (it said Layers, the panel's name before M4's U18).
#[test]
fn a_component_s_tip_names_the_outliner() {
    let mut h = designer(Mode::Tabs, 1920.0);
    h.get_by_label("Button").scroll_to_me();
    settle(&mut h);
    let rect = h.get_by_label("Button").rect();
    let tip = tip_at(&mut h, rect).expect("the Button component's tip");
    assert!(tip.contains("or drag into the Outliner or onto the canvas"), "{tip}");
    assert!(!tip.contains("Layers"), "{tip}");
}

/// Choosing goes both ways: a click on a Layers row chooses the control on
/// the canvas (its handles, Properties' detail), Ctrl+click several; a
/// click on the canvas chooses its row in Layers (opened out to it where
/// folded). Layers' keys: Down and Up from control to control, Left folds
/// and goes to the parent, Right unfolds, Home and End, letters to a
/// control by its words, Delete deletes the controls chosen; Escape hands
/// them back, and the canvas's Delete is then the canvas's again.
#[test]
fn layers_choose_on_the_canvas_and_take_the_keys() {
    let mut h = designer(Mode::Tabs, 1600.0);
    layer(&h, "Button · Send").click();
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/2/children/0", "a row's click chooses");
    let detail = |h: &Harness<'_, Moonglow>| {
        h.query_all_by_label_contains("Properties, ")
            .next()
            .and_then(|n| n.accesskit_node().label())
    };
    assert_eq!(detail(&h).as_deref(), Some("Properties, Button · Send"), "its detail");
    // The canvas chooses: its row is the one marked.
    choose(&mut h, "Label · Hear ye");
    assert_eq!(chosen(&h), "/root/children/0");
    assert_eq!(detail(&h).as_deref(), Some("Properties, Label · Hear ye"));
    // Folded, the canvas's choice opens its row out.
    h.get_by_label("Collapse Row").click();
    settle(&mut h);
    assert!(h.query_by_label("Button · Cancel").is_none(), "folded");
    choose(&mut h, "Button · Cancel");
    assert!(h.query_by_label("Button · Cancel").is_some(), "opened out to the canvas's choice");
    // The keys: a row's click gives them to Layers.
    layer(&h, "Label · Hear ye").click();
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/1", "Down");
    h.key_press(egui::Key::End);
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/2/children/1", "End");
    h.key_press(egui::Key::ArrowLeft);
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/2", "Left: the row holding it");
    h.key_press(egui::Key::ArrowLeft);
    settle(&mut h);
    assert!(h.query_by_label("Expand Row").is_some(), "Left again folds it");
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert!(h.query_by_label("Collapse Row").is_some(), "Right unfolds it");
    h.key_press(egui::Key::Home);
    settle(&mut h);
    assert_eq!(chosen(&h), "", "Home: the window");
    h.event(egui::Event::Text("t".into()));
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/1", "letters: Text input · …");
    // Ctrl+click chooses several; Delete deletes them.
    let ctrl = egui::Modifiers::COMMAND;
    layer(&h, "Button · Send").click_modifiers(ctrl);
    settle(&mut h);
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    let doc = document(&h);
    assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 2, "{doc}");
    assert_eq!(doc["root"]["children"][1]["children"].as_array().unwrap().len(), 1, "{doc}");
    // Escape hands them back: Layers' Delete no more, the canvas's again
    // with the pointer over it.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    choose(&mut h, "Label · Hear ye");
    h.hover_at(h.get_by_label("Canvas Label · Hear ye").rect().center());
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 1);
}

/// Layers' rows: only those in sight laid out (a window of a thousand
/// controls lists its first rows alone), and Find a layer or ID lists the
/// controls that hold what is typed, the last of the thousand among them.
#[test]
fn layers_lay_out_the_rows_in_sight_alone() {
    let mut w = our_window();
    let many: Vec<Value> = (0..1000)
        .map(|i| json!({"type":"label","id":format!("l{i}"),"value":format!("Line {i}"),"height":20.0}))
        .collect();
    w["root"]["children"] = Value::Array(many);
    let mut h = designer_of(Mode::Tabs, 1600.0, w);
    let listed = h
        .query_all_by_label_contains("Label · Line ")
        .filter(|n| !n.accesskit_node().label().is_some_and(|l| l.starts_with("Canvas")));
    let count = listed.count();
    assert!((5..80).contains(&count), "{count} rows laid out");
    let mut w = document(&h);
    w["root"]["children"][999]["value"] = json!("The last");
    let data = serde_json::to_vec_pretty(&w).unwrap();
    let edit = mg_edit::Edit::SetResource { key: key(), data: Some(data) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
    settle(&mut h);
    h.get_by_label("Find a layer or ID").click();
    settle(&mut h);
    h.get_by_label("Find a layer or ID").type_text("The last");
    settle(&mut h);
    assert!(h.query_by_label("Label · The last").is_some(), "found by Find");
    assert!(h.query_by_label("Label · Line 3").is_none(), "the others not listed");
}

/// The designer's panels follow the designer in front: two windows in two
/// groups, each its own Properties as its group is chosen; beside a script,
/// the script's Reference, then the designer's again; floating, its
/// window's right pane its own and the main window's Right Dock not its.
#[test]
fn the_nui_panels_follow_the_designer_in_front() {
    let mut h = designer(Mode::Tabs, 1920.0);
    let other = ResKey::parse("other", ResType::JUI).unwrap();
    let mut w = our_window();
    w["root"]["children"][0]["value"] = json!("Another window");
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.module.set(other, serde_json::to_vec_pretty(&w).unwrap());
    choose(&mut h, "Label · Hear ye");
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(other)));
    settle(&mut h);
    choose(&mut h, "Label · Another window");
    let detail = |h: &Harness<'_, Moonglow>| {
        h.query_all_by_label_contains("Properties, ")
            .next()
            .and_then(|n| n.accesskit_node().label())
    };
    assert_eq!(detail(&h).as_deref(), Some("Properties, Label · Another window"), "the other's");
    // (Its Layers: its own rows, not the first window's.)
    assert!(h.query_by_label("Label · Another window").is_some(), "the other's Layers");
    assert!(h.query_by_label("Label · Hear ye").is_none(), "not the first's");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
    settle(&mut h);
    assert_eq!(detail(&h).as_deref(), Some("Properties, Label · Hear ye"), "the first's again");
    // Beside a script: the script's panels, then the designer's again.
    let script = ResKey::parse("crier_s", ResType::NSS).unwrap();
    h.state_mut().ws.as_mut().unwrap().module.set(script, b"void main() {}\n".to_vec());
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(script)));
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Reference), "the script's");
    assert!(h.query_by_label("Find a control").is_none(), "no Components");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
    settle(&mut h);
    assert_eq!(detail(&h).as_deref(), Some("Properties, Label · Hear ye"));
    // Floating: in its window, the docks' panels not its.
    let mut h = designer(Mode::Floating, 1920.0);
    choose(&mut h, "Label · Hear ye");
    assert_eq!(detail(&h).as_deref(), Some("Properties, Label · Hear ye"), "its window's");
    assert!(
        !matches!(h.state().right_dock_shows(), Some(Panel::Properties | Panel::Components)),
        "not the main window's dock"
    );
}

/// The window's revision, the module's history's.
fn revision(h: &Harness<'_, Moonglow>) -> u64 {
    h.state().ws.as_ref().unwrap().revision()
}

/// Each edit in the panels is one step of the Top Bar's Undo, taken back
/// with the pointer over the docked panel; Cancel sets back what the
/// panels changed (a component added, a property typed, a bind's value).
#[test]
fn a_panel_s_edit_is_one_undo_step_and_cancel_sets_it_back() {
    for mode in MODES {
        let mut h = designer(mode, 1920.0);
        let what = format!("{mode:?}");
        let before = document(&h);
        let start = revision(&h);
        choose(&mut h, "Label · Hear ye");
        type_in(&mut h, "Text", "!");
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(revision(&h), start + 1, "{what}: one step");
        assert_eq!(document(&h)["root"]["children"][0]["value"], "Hear ye!");
        // (The Top Bar's Undo, the pointer over Properties.)
        let over = field_in_sight(&mut h, "Text").center();
        h.hover_at(over);
        settle(&mut h);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        settle(&mut h);
        assert_eq!(document(&h), before, "{what}: Ctrl+Z over the panel");
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
        settle(&mut h);
        assert_eq!(document(&h)["root"]["children"][0]["value"], "Hear ye!", "{what}: Redo");
        tool(&h, "Undo").click();
        settle(&mut h);
        assert_eq!(document(&h), before, "{what}: the Top Bar's Undo");
        // A component added: one step.
        choose(&mut h, "Label · Hear ye");
        let at = revision(&h);
        h.get_by_label("Button").scroll_to_me();
        settle(&mut h);
        h.get_by_label("Button").click();
        settle(&mut h);
        assert_eq!(revision(&h), at + 1, "{what}: a component, one step");
        assert_eq!(document(&h)["root"]["children"].as_array().unwrap().len(), 4, "{what}");
        assert_eq!(chosen(&h), "/root/children/3", "{what}: the new one chosen");
        // Cancel sets every one of them back.
        h.get_by_label(NUI_HEADER).get_by_label("Cancel").click();
        settle(&mut h);
        assert_eq!(document(&h), before, "{what}: Cancel");
        assert!(h.state().dock.find_tab(&Tab::Nui(key())).is_none(), "{what}: closed");
    }
}

/// What is typed in a Properties field is its own control's: the control
/// chosen in Layers, then while its field is typed in another row clicked,
/// a component added, the control moved, duplicated, deleted, Undo pressed:
/// what was typed is set on the control it was typed for (or taken back
/// with Undo), and what is typed after the move goes to the control chosen
/// then, never to the one now at its old place.
#[test]
fn typed_text_keeps_to_its_own_control() {
    let mut h = designer(Mode::Tabs, 1920.0);
    let value = |h: &Harness<'_, Moonglow>, path: &str| document(h).pointer(path).cloned();
    // Another row clicked while Send's Label is typed in.
    layer(&h, "Button · Send").click();
    settle(&mut h);
    type_in(&mut h, "Label", "1");
    layer(&h, "Button · Cancel").click();
    settle(&mut h);
    assert_eq!(value(&h, "/root/children/2/children/0/label"), Some(json!("Send1")));
    assert_eq!(value(&h, "/root/children/2/children/1/label"), Some(json!("Cancel")));
    // A component added while Cancel's is typed in: set on Cancel.
    type_in(&mut h, "Label", "2");
    h.get_by_label("Button").scroll_to_me();
    settle(&mut h);
    h.get_by_label("Button").click();
    settle(&mut h);
    assert_eq!(value(&h, "/root/children/2/children/1/label"), Some(json!("Cancel2")));
    assert_eq!(value(&h, "/root/children/2/children/2/label"), Some(json!("Button")), "added");
    // Moved up while typed in: set on the control moved; typed after the
    // move, on it at its new place, not on the one now at its old.
    layer(&h, "Button · Cancel2").click();
    settle(&mut h);
    type_in(&mut h, "Label", "3");
    h.get_by_label(nui_names::ELEMENT_ACTIONS).click();
    settle(&mut h);
    h.get_by_label_contains("Move Up").click();
    settle(&mut h);
    assert_eq!(value(&h, "/root/children/2/children/0/label"), Some(json!("Cancel23")));
    assert_eq!(chosen(&h), "/root/children/2/children/0", "it is still chosen");
    type_in(&mut h, "Label", "4");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(value(&h, "/root/children/2/children/0/label"), Some(json!("Cancel234")));
    assert_eq!(value(&h, "/root/children/2/children/1/label"), Some(json!("Send1")));
    // Duplicated while typed in: the original's, the copy chosen; typed
    // after, the copy's.
    type_in(&mut h, "Label", "5");
    h.get_by_label(nui_names::ELEMENT_ACTIONS).click();
    settle(&mut h);
    h.get_by_label_contains("Duplicate Element").click();
    settle(&mut h);
    assert_eq!(value(&h, "/root/children/2/children/0/label"), Some(json!("Cancel2345")));
    assert_eq!(chosen(&h), "/root/children/2/children/1", "the copy chosen");
    type_in(&mut h, "Label", "6");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(value(&h, "/root/children/2/children/1/label"), Some(json!("Cancel23456")));
    assert_eq!(value(&h, "/root/children/2/children/0/label"), Some(json!("Cancel2345")));
    // Deleted while typed in: the typing goes with it; typed after, the
    // control chosen then's.
    type_in(&mut h, "Label", "7");
    h.get_by_label(nui_names::ELEMENT_ACTIONS).click();
    settle(&mut h);
    h.get_by_label_contains("Delete Element").click();
    settle(&mut h);
    let labels = |h: &Harness<'_, Moonglow>| -> Vec<Value> {
        let doc = document(h);
        doc["root"]["children"][2]["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["label"].clone())
            .collect()
    };
    assert_eq!(labels(&h), [json!("Cancel2345"), json!("Send1"), json!("Button")]);
    let now = chosen(&h);
    type_in(&mut h, "Label", "8");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let label = value(&h, &format!("{now}/label")).unwrap();
    assert!(label.as_str().unwrap().ends_with('8'), "the control chosen after the delete: {label}");
    // Undo while typed in: the Top Bar's Undo takes back what was typed.
    let before = document(&h);
    type_in(&mut h, "Label", "9");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(document(&h), before, "typed and taken back");
    // (Taken back as the step it was: Redo brings it again.)
    tool(&h, "Redo").click();
    settle(&mut h);
    let label = value(&h, &format!("{now}/label")).unwrap();
    assert!(label.as_str().unwrap().ends_with("89"), "Redo: what was typed {label}");
}

/// A Properties field typed in and not left is set as its panel goes:
/// another tab chosen on the Right Dock's one bar (under 1400), the dock
/// folded, the page switched.
#[test]
fn a_field_typed_in_is_set_when_its_panel_goes() {
    for case in ["tab", "folded", "page"] {
        let width = if case == "tab" { 1300.0 } else { 1920.0 };
        let mut h = designer(Mode::Tabs, width);
        choose(&mut h, "Label · Hear ye");
        properties_in_front(&mut h);
        type_in(&mut h, "Text", "?");
        assert_eq!(document(&h)["root"]["children"][0]["value"], "Hear ye", "{case}: not set yet");
        match case {
            "tab" => {
                let tab = |n: &egui_kittest::Node<'_>| {
                    n.accesskit_node().role() == egui::accesskit::Role::Tab
                };
                h.get_all_by_label("Components").find(tab).unwrap().click();
            }
            "folded" => h.state_mut().settings.hide_palettes = true,
            _ => {
                nui_tool(&mut h, nui_names::PAGE).get_by_label("Bindings").click();
            }
        }
        settle(&mut h);
        assert_eq!(document(&h)["root"]["children"][0]["value"], "Hear ye?", "{case}: set");
    }
}

/// [`our_window`] with a swap layout ("content", its initial contents a
/// label, "Initial") and its settings with a variant of it ("details",
/// two labels, "Left" and "Right").
fn with_a_variant() -> (Value, Settings) {
    let mut w = our_window();
    w["root"]["children"].as_array_mut().unwrap().push(json!(
        {"type":"group","id":"content","width":360.0,"height":100.0,
            "children":[{"type":"col","children":[{"type":"label","value":"Initial"}]}]}
    ));
    let mut s = our_settings();
    s.variants.insert(
        "details".into(),
        json!({"type":"row","children":[
            {"type":"label","value":"Left","width":150.0,"height":30.0},
            {"type":"label","value":"Right","width":150.0,"height":30.0}]}),
    );
    s.variant_targets.insert("details".into(), "content".into());
    (w, s)
}

/// No copy of the window is made in a frame that changes nothing: the
/// designer and its four panels drawn over many frames, typed in, a combo
/// box opened; one copy made for each edit. Nor of its root or a variant,
/// the one edited in the root's place too: where their words are kept is
/// the same from frame to frame (a copy put in their place would move
/// them).
#[test]
fn a_frame_that_changes_nothing_copies_no_window() {
    // (And with the source group open beside it, M4's U19: its text is the
    // module's, laid out once for the text it is.)
    for (variant, source) in [(false, false), (true, false), (false, true)] {
        let what = match (variant, source) {
            (true, _) => "a variant edited",
            (_, true) => "the source group open",
            _ => "the window",
        };
        let (w, s) = with_a_variant();
        let mut h = designer_with(Mode::Tabs, 1920.0, w, s);
        if source {
            layout_chosen(&mut h, "Split source");
        }
        if variant {
            layer(&h, "Variant · details").click();
            settle(&mut h);
            choose(&mut h, "Label · Left");
        } else {
            choose(&mut h, "Label · Hear ye");
        }
        let copies = h.state().nui_copies(&key());
        assert!(copies >= 1, "{what}: a copy to edit");
        let at = h.state().nui_identity(&key()).expect("its working copy");
        assert_eq!(at.len(), 3, "{what}: the window, its root, the variant");
        for _ in 0..5 {
            h.hover_at(egui::pos2(800.0, 500.0));
            h.run();
            h.hover_at(egui::pos2(810.0, 510.0));
            h.run();
            assert_eq!(h.state().nui_identity(&key()).unwrap(), at, "{what}: frame by frame");
        }
        type_in(&mut h, "Text", "x");
        settle(&mut h);
        assert_eq!(h.state().nui_copies(&key()), copies, "{what}: none while drawn and typed in");
        assert_eq!(h.state().nui_identity(&key()).unwrap(), at, "{what}: typed in");
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(h.state().nui_copies(&key()), copies + 1, "{what}: one for the edit");
    }
}

/// The View popover does what the canvas's row did, its state the
/// designer's as before (kept per window for the session): Layout (Preview
/// only and Editor, the panels kept in both),
/// UI scale, State, Resources, Clean view and Screen; its combo boxes open
/// in it without closing it; Escape closes it alone (the designer stays
/// open); reached by the keys (Tab to its button, Enter).
#[test]
fn the_view_popover_does_what_the_canvas_row_did() {
    let mut h = designer(Mode::Tabs, 1920.0);
    let canvas_size = |h: &Harness<'_, Moonglow>| h.get_by_label("Canvas window").rect().size();
    let at_100 = canvas_size(&h);
    nui_tool(&mut h, nui_names::VIEW).click();
    settle(&mut h);
    h.get_all_by(combo("UI scale")).next().expect("UI scale").click();
    settle(&mut h);
    h.get_all_by_label("150%").last().unwrap().click();
    settle(&mut h);
    assert!(h.query_by_label("Clean view").is_some(), "open after a combo box's choice");
    let at_150 = canvas_size(&h);
    assert!(
        at_150.x > at_100.x * 1.3 || h.query_by_label_contains("UI scale 150%").is_some(),
        "scaled"
    );
    h.get_by_label("Clean view").click();
    settle(&mut h);
    assert!(h.query_by_label("Clean view").is_some(), "still open after a check");
    // (The popover's, drawn after the Left Dock's tab of the name.)
    h.get_all_by_label("Resources").last().unwrap().click();
    settle(&mut h);
    assert!(h.query_by_label_contains("Stock bevel").is_some(), "Resources shown");
    // Escape closes it alone.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label("Clean view").is_none(), "closed");
    assert!(h.state().dock.find_tab(&Tab::Nui(key())).is_some(), "the designer stays");
    assert!(h.query_by_label_contains("UI scale 150%").is_some(), "its state kept");
    // Layout: Preview only and Editor, the panels kept in both.
    for layout in ["Preview only", "Editor"] {
        layout_chosen(&mut h, layout);
        assert!(h.query_by_label("Find a control").is_some(), "{layout}: Components kept");
    }
    // Screen: a size chosen puts the window on it.
    nui_tool(&mut h, nui_names::VIEW).click();
    settle(&mut h);
    h.get_all_by(combo("Screen")).next().unwrap().click();
    settle(&mut h);
    h.get_by_label("1280 × 720").click();
    settle(&mut h);
    let on_screen = |h: &Harness<'_, Moonglow>| {
        h.get_by_label("Preview on screen").accesskit_node().toggled()
            == Some(egui::accesskit::Toggled::True)
    };
    assert!(on_screen(&h), "on the screen chosen");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // State: off in Interact, as its row's was.
    nui_tool(&mut h, nui_names::INTERACT).click();
    settle(&mut h);
    nui_tool(&mut h, nui_names::VIEW).click();
    settle(&mut h);
    assert!(h.get_all_by(combo("State")).next().unwrap().accesskit_node().is_disabled());
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label("Clean view").is_none(), "closed");
    // Reached by the keys: its button given the keyboard, Enter opens it.
    nui_tool(&mut h, nui_names::VIEW).focus();
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.query_by_label("Clean view").is_some(), "Enter on View opens it");
}

/// The header's controls, in its order.
const NUI_CONTROLS: [&str; 8] = [
    nui_names::PAGE,
    nui_names::BUILD,
    nui_names::EXPORT,
    nui_names::IMPORT,
    nui_names::HOW_TO_OPEN,
    nui_names::INTERACT,
    nui_names::RESET,
    nui_names::VIEW,
];

/// The designer's header holds every control at every width from 420 to
/// 1600 (on the row, as an icon, or in More), both ways: the pages' switch
/// and Build & Compile on the row at the widest, Import… and How to Open
/// It… in More, OK and Cancel in the end slot.
#[test]
fn the_nui_header_holds_every_control_from_420_to_1600() {
    use mg_uikit::header::{self, Place};
    for mode in MODES {
        let mut h = designer(mode, 1600.0);
        let mut widths: Vec<f32> = (21..=80).rev().map(|n| n as f32 * 20.0).collect();
        widths.extend([1399.0, 1400.0, 961.0, 962.0, 759.0, 760.0]);
        let mut top = true;
        for width in widths {
            h.set_size(egui::vec2(width, 800.0));
            settle(&mut h);
            let what = format!("{mode:?} {width}");
            let rec = header::records(&h.ctx, NUI_HEADER);
            let rec = rec.into_iter().next().unwrap_or_else(|| panic!("{what}: no header"));
            // (What the last Build left, a floating designer's header's
            // first at its right, M4's U19: not built here.)
            let names: Vec<&str> = rec.places.iter().map(|(n, _)| n.as_str()).collect();
            let built = names.contains(&nui_names::BUILD_STATE);
            assert_eq!(built, mode == Mode::Floating, "{what}: the build's state");
            let names: Vec<&str> =
                names.into_iter().filter(|n| *n != nui_names::BUILD_STATE).collect();
            assert_eq!(names, NUI_CONTROLS, "{what}");
            let place = |n: &str| rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
            if std::mem::take(&mut top) {
                for n in [nui_names::PAGE, nui_names::BUILD, nui_names::VIEW] {
                    assert!(matches!(place(n), Some(Place::Row | Place::Icon)), "{what}: {n}");
                }
            }
            for n in [nui_names::IMPORT, nui_names::HOW_TO_OPEN] {
                assert_eq!(place(n), Some(Place::More), "{what}: {n} in More");
            }
            let end = rec.end.expect("the end slot");
            for n in ["OK", "Cancel"] {
                let r = h.get_by_label(NUI_HEADER).get_by_label(n).rect();
                assert!(end.expand(0.5).contains_rect(r), "{what}: {n} in {end:?}");
            }
            header_holds(&mut h, NUI_HEADER, &NUI_CONTROLS);
        }
    }
}

/// A floating designer opens at its first size within the main window (a
/// screen of 1280 by 720 and of 1600 by 900): as wide as it asks (its
/// canvas's 760, its right pane and its Layers') where the screen has it;
/// its header's commands on the row as the tip's were (Import… and How to
/// Open It… in More), clear of its right pane; Properties' Element ID and
/// Label whole in its pane as it opens, not scrolled to.
#[test]
fn a_floating_designer_opens_at_its_first_size() {
    use mg_uikit::header::{self, Place};
    for (width, height) in [(1280.0, 720.0), (1600.0, 900.0)] {
        let what = format!("{width} by {height}");
        let mut app = app_in(Mode::Floating, Vec::new());
        let mut module = Module::new();
        module.set(key(), serde_json::to_vec_pretty(&our_window()).unwrap());
        module.set(ResKey::new(key().resref, ResType::TXT), our_settings().bytes());
        app.ws = Some(mg_edit::Workspace::new(module));
        let mut h = harness(app, width, height);
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
        settle(&mut h);
        let at = h.state().dock.find_tab(&Tab::Nui(key())).expect("its tab");
        let path = egui_dock::NodePath { surface: at.surface, node: at.node };
        let pane = h.state().dock.leaf(path).unwrap().rect;
        let want = (760.0_f32 + 281.0 + 261.0).min(width - 48.0);
        assert!(pane.width() >= want - 12.0, "{what}: {} wide, wants {want}", pane.width());
        let rec = header::records(&h.ctx, NUI_HEADER).into_iter().next().expect("its header");
        for n in [
            nui_names::PAGE,
            nui_names::BUILD,
            nui_names::EXPORT,
            nui_names::INTERACT,
            nui_names::RESET,
            nui_names::VIEW,
        ] {
            let place = rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
            assert!(matches!(place, Some(Place::Row | Place::Icon)), "{what}: {n} {place:?}");
        }
        header_holds(&mut h, NUI_HEADER, &NUI_CONTROLS);
        choose(&mut h, "Button · Send");
        fn field(name: &str) -> impl Fn(&egui_kittest::kittest::AccessKitNode<'_>) -> bool + '_ {
            move |a| {
                a.role() == egui::accesskit::Role::TextInput && a.label().as_deref() == Some(name)
            }
        }
        // (The Properties section's own room: over the Bindings section's
        // header, which stands under it.)
        // (Bounded by the Bindings section's first words, under its header.)
        let bindings = h
            .get_all_by_label(nui_names::BINDS_TABLE)
            .map(|n| n.rect())
            .find(|r| r.left() >= rec.row.right())
            .unwrap_or_else(|| panic!("{what}: the Bindings section in the right pane"));
        let properties =
            egui::Rect::from_min_max(pane.min, egui::pos2(pane.right(), bindings.top()));
        for name in ["Element ID", "Label"] {
            let r = h.get_all_by(field(name)).next().unwrap_or_else(|| panic!("{name}")).rect();
            assert!(properties.contains_rect(r), "{what}: {name} whole: {r:?} in {properties:?}");
            assert!(r.left() >= rec.row.right(), "{what}: {name} in the right pane");
        }
        // Components usable: its search and a row of its components whole
        // above Properties' header; Bindings: Create Bind whole in the pane.
        let rect = |n: &egui_kittest::Node<'_>| {
            let b = n.accesskit_node().bounding_box()?;
            Some(egui::Rect::from_min_max(
                egui::pos2(b.x0 as f32, b.y0 as f32),
                egui::pos2(b.x1 as f32, b.y1 as f32),
            ))
        };
        // (Properties' header, a group of no rect of its own: over its
        // first section's heading, its height and the room under it.)
        let first = h.get_by_label("Property Binding").rect();
        let header = first.translate(egui::vec2(0.0, -(mg_uikit::metrics::DOCK_TABS + 16.0)));
        let components = egui::Rect::from_min_max(
            egui::pos2(rec.row.right(), pane.top()),
            egui::pos2(pane.right(), header.top()),
        );
        let search = h.get_all_by(field("Find a control")).next().expect("its search").rect();
        assert!(components.contains_rect(search), "{what}: the search {search:?} {components:?}");
        for name in ["Column", "Row", "Group"] {
            let whole = h.get_all_by_label(name).filter_map(|n| rect(&n));
            assert!(
                whole.into_iter().any(|r| components.contains_rect(r)),
                "{what}: {name} whole in {components:?}"
            );
        }
        // (Create Bind: the Bindings header's plus, M4's U19.)
        let create = h.get_by_label("New Bind").rect();
        assert!(create.top() >= first.top() && pane.contains_rect(create), "{what}: {create:?}");
        // Each used: a component added to the control chosen's row, a bind
        // made.
        let column = h
            .get_all_by_label("Column")
            .find(|n| rect(n).is_some_and(|r| components.contains_rect(r)));
        column.unwrap().click();
        settle(&mut h);
        let row = &document(&h)["root"]["children"][2]["children"];
        assert_eq!(row.as_array().unwrap().len(), 3, "{what}: added beside Send");
        h.get_by_label("New Bind").click();
        settle(&mut h);
        assert!(h.query_by_label("Bind name").is_some(), "{what}: Create Bind opened");
    }
}

/// Records from before the docks read without a fault, as the tip before
/// them (27ce7852) wrote them: the settings keep the designer window's
/// size (`window_sizes`' "nui", 1140 by 792, the three panes' window, as
/// a builder who sized it left it); the open-tabs record keeps its tab,
/// in its own window at that size (Floating) or a tab of the middle
/// (Tabs), and no view of it (the tip wrote none: its panes' widths were
/// the window's). Opened again: the designer at that size, Layers a pane
/// or the strip, every header control held, Components, and the
/// selection's Properties fields whole in their pane.
#[test]
fn records_naming_the_old_panes_read_without_error() {
    use mg_uikit::header::{self, Place};
    let text = "(window_sizes:[(\"nui\",(1140,792))],window_sizes_scaled:true,name_resrefs:true)";
    let (settings, _unread) = mg_ui::Settings::read_fields(text).expect("read");
    assert!(settings.name_resrefs, "the rest read");
    for mode in MODES {
        let dir = mg_testkit::scratch_dir(&mode.dir("ui-nui-old-records"));
        let path = dir.join("crier.mod");
        let mut module = Module::new();
        module.set_info(&Gff::new(*b"IFO ")).unwrap();
        module.set(key(), serde_json::to_vec_pretty(&our_window()).unwrap());
        module.set(ResKey::new(key().resref, ResType::TXT), our_settings().bytes());
        module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        let tabs = dir.join("open-tabs");
        std::fs::create_dir_all(&tabs).unwrap();
        // (The tip's own record, its window's rect the size kept.)
        let record = match mode {
            Mode::Floating => {
                r#"{"panes":[{"place":"middle","tabs":["module-properties"],"active":"module-properties"},
                {"place":"window","tabs":["nui:crier.jui"],"active":"nui:crier.jui",
                 "rect":[367.3,39.0,1140.0,792.0]}],
                "front":"nui:crier.jui","views":{"module-properties":{"page":"Basic"}},
                "rects_scaled":true}"#
            }
            Mode::Tabs => {
                r#"{"panes":[{"place":"middle","tabs":["module-properties","nui:crier.jui"],
                "active":"nui:crier.jui"}],"front":"nui:crier.jui",
                "views":{"module-properties":{"page":"Basic"}},"rects_scaled":true}"#
            }
        };
        std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
        let mut app = app_in(mode, Vec::new());
        app.settings.window_sizes = settings.window_sizes.clone();
        app.settings.window_sizes_scaled = settings.window_sizes_scaled;
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(tabs.clone());
        app.open_module(&path);
        let mut h = harness(app, 1600.0, 900.0);
        settle(&mut h);
        let what = format!("{mode:?}");
        let at = h.state().dock.find_tab(&Tab::Nui(key())).expect("the tab kept");
        let pane = h.state().dock.leaf(egui_dock::NodePath { surface: at.surface, node: at.node });
        let pane = pane.unwrap().rect;
        if mode == Mode::Floating {
            assert!(!at.surface.is_main(), "{what}: its own window");
            // (Its panes inside the window's frame: a little less.)
            assert!(pane.width() > 1100.0 && pane.height() > 730.0, "{what}: {pane:?}");
        }
        let strip = h.query_by_label(mg_ui::dialog_view::OUTLINER_STRIP).is_some();
        let layers = h.query_by_label("Find a layer or ID").is_some();
        assert!(layers || strip, "{what}: Layers, a pane or the strip");
        assert!(h.query_by_label("Find a control").is_some(), "{what}: Components");
        let rec = header::records(&h.ctx, NUI_HEADER).into_iter().next().expect("its header");
        for n in [nui_names::PAGE, nui_names::BUILD, nui_names::VIEW] {
            let place = rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
            assert!(matches!(place, Some(Place::Row | Place::Icon)), "{what}: {n} {place:?}");
        }
        header_holds(&mut h, NUI_HEADER, &NUI_CONTROLS);
        choose(&mut h, "Button · Send");
        let rec = header::records(&h.ctx, NUI_HEADER).into_iter().next().unwrap();
        for name in ["Element ID", "Label"] {
            let field = |a: &egui_kittest::kittest::AccessKitNode<'_>| {
                a.role() == egui::accesskit::Role::TextInput && a.label().as_deref() == Some(name)
            };
            let r = h.get_all_by(field).next().unwrap_or_else(|| panic!("{what}: {name}")).rect();
            assert!(r.left() >= rec.row.right(), "{what}: {name} right of the editor");
            let room = r.width() >= 100.0 && r.right() <= 1600.0 && r.bottom() <= 900.0;
            assert!(room, "{what}: {name} whole: {r:?}");
            if mode == Mode::Floating {
                assert!(pane.contains_rect(r), "{what}: {name} in its window: {r:?} {pane:?}");
            }
        }
    }
}

/// F6 goes through the designer's panes: the Outliner's Layers (their
/// rows the keys'), the editor, the Right Dock's Components (Find a control
/// given the keyboard); Shift+F6 back.
#[test]
fn f6_reaches_layers_and_the_components() {
    // (Floating too: the window's own panes, its Layers a pane at 1920.)
    for mode in MODES {
        let mut h = designer(mode, 1920.0);
        choose(&mut h, "Label · Hear ye");
        let mut seen = Vec::new();
        for _ in 0..6 {
            h.key_press(egui::Key::F6);
            settle(&mut h);
            let keys = mg_ui::palette_view::arrow_keys_are(&h.ctx);
            if keys == "NuiComponents" {
                let find = h.get_by_label("Find a control");
                assert!(find.accesskit_node().is_focused(), "{mode:?}: Find a control focused");
            }
            seen.push(keys);
        }
        assert!(seen.iter().any(|k| k == "Marks"), "{mode:?}: Layers: {seen:?}");
        assert!(seen.iter().any(|k| k == "NuiComponents"), "{mode:?}: Components: {seen:?}");
        // Layers' keys work there.
        while mg_ui::palette_view::arrow_keys_are(&h.ctx) != "Marks" {
            h.key_press(egui::Key::F6);
            settle(&mut h);
        }
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
        assert_eq!(chosen(&h), "/root/children/1", "{mode:?}: Layers' Down after F6");
    }
}

/// The Layout `name` chosen in the View popover (closed after).
#[track_caller]
fn layout_chosen(h: &mut Harness<'_, Moonglow>, name: &str) {
    nui_tool(h, nui_names::VIEW).click();
    settle(h);
    h.get_all_by(combo("Layout")).next().unwrap().click();
    settle(h);
    h.get_by_label(name).click();
    settle(h);
    h.key_press(egui::Key::Escape);
    settle(h);
}

/// The docks stay on every page and in every Layout (the coordinator's
/// ruling): Components, Layers and the selection's Properties on
/// Bindings, Events & Variants and Images, in Preview only and Split
/// source; and the editor's width never jumps as they change, not for a
/// frame (a panel brought or taken away on the way would).
#[test]
fn the_docks_stay_on_every_page_and_layout_and_the_editor_keeps_its_width() {
    for mode in MODES {
        let mut h = designer(mode, 1920.0);
        choose(&mut h, "Button · Send");
        let width = editor_rect(&h).width();
        let check = |h: &Harness<'_, Moonglow>, what: &str| {
            assert!(h.query_by_label("Find a control").is_some(), "{mode:?} {what}: Components");
            assert!(h.query_by_label("Find a layer or ID").is_some(), "{mode:?} {what}: Layers");
            assert_eq!(chosen(h), "/root/children/2/children/0", "{mode:?} {what}: kept");
            let detail = h.query_all_by_label_contains("Properties, ").next().unwrap();
            assert_eq!(
                detail.accesskit_node().label().as_deref(),
                Some("Properties, Button · Send"),
                "{mode:?} {what}"
            );
        };
        for name in ["Bindings", "Events & Variants", "Images", "Design"] {
            nui_tool(&mut h, nui_names::PAGE).get_by_label(name).click();
            // (Frame by frame: the width the same at each.)
            for _ in 0..6 {
                h.run_steps(1);
                let now = editor_rect(&h).width();
                assert_eq!(now, width, "{mode:?} {name}: the editor's width");
            }
            settle(&mut h);
            check(&h, name);
        }
        // (Split source is the source group beside the design, M4's U19: the
        // design narrows to its group, the docks stay; Editor gives it its
        // width back.)
        for layout in ["Preview only", "Split source", "Editor"] {
            layout_chosen(&mut h, layout);
            check(&h, layout);
            let source = h.query_by_label(nui_names::SOURCE_TEXT).is_some();
            assert_eq!(source, layout == "Split source", "{mode:?} {layout}: the source group");
            if layout != "Split source" {
                let now = editor_rect(&h).width();
                assert_eq!(now, width, "{mode:?} {layout}: the editor's width");
            }
        }
        // Components adds to the control chosen from another page too.
        nui_tool(&mut h, nui_names::PAGE).get_by_label("Images").click();
        settle(&mut h);
        let column = h.get_all_by_label("Column").find(|n| {
            n.accesskit_node().role() == egui::accesskit::Role::Button && !in_outliner(n)
        });
        column.expect("Components' Column").click();
        settle(&mut h);
        let row = &document(&h)["root"]["children"][2]["children"];
        assert_eq!(row.as_array().unwrap().len(), 3, "{mode:?}: added beside Send, from Images");
    }
}

/// A window of every component the palette adds, in a column.
fn every_component() -> Value {
    let mut w = mg_nui::window();
    let kinds = [
        "col",
        "row",
        "group",
        "spacer",
        "label",
        "text",
        "button",
        "button_select",
        "textedit",
        "check",
        "combo",
        "slider",
        "sliderf",
        "color_picker",
        "image",
        "button_image",
        "progress",
        "list",
        "options",
        "tabbar",
        "chart",
    ];
    let children: Vec<Value> = kinds
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let mut c = mg_nui::template(k);
            c["id"] = json!(format!("c{i}"));
            c
        })
        .collect();
    w["root"]["children"] = Value::Array(children);
    w
}

/// What the docked Properties shows in `dock` fits it: nothing in sight
/// past its edge, every one-line field in sight as wide as its text.
#[track_caller]
fn fits(h: &Harness<'_, Moonglow>, dock: egui::Rect, what: &str) {
    fits_dock(h, dock, what, egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE), true);
}

/// Every panel fits the Right Dock from its least width: for every
/// component kind (and the window), each with its own properties' kinds
/// (text, numbers, colors, choices, options, a chart's series, a list's
/// rows, bounds and limits), Components, Properties scrolled to its top
/// and its foot, and Bindings, at 280, 340, 440 and 600.
#[test]
fn every_nui_panel_fits_the_right_dock() {
    let mut h = designer_of(Mode::Tabs, 1920.0, every_component());
    for width in [280.0, 340.0, 440.0, 600.0] {
        right_dock_width(&mut h, width);
        settle(&mut h);
        let dock = right_dock_rect(&h);
        assert!((dock.width() - width - 1.0).abs() < 2.0, "{width}: {dock:?}");
        // (From the window, down every control, by Layers' keys: Home
        // where they have them, the window's row clicked first.)
        match h.query_by_label("Window") {
            Some(_) if chosen(&h).is_empty() => layer(&h, "Window").click(),
            _ => h.key_press(egui::Key::Home),
        }
        settle(&mut h);
        assert_eq!(chosen(&h), "", "{width}: the window chosen");
        for step in 0..23 {
            let at = chosen(&h);
            let what = format!("{width} {at}");
            fits(&h, dock, &what);
            // (Scrolled to its foot: its last section's heading.)
            let foot = h.query_all_by_label("Draw Layers").next().is_some();
            if foot {
                h.get_all_by_label("Draw Layers").next().unwrap().scroll_to_me();
                settle(&mut h);
                fits(&h, dock, &format!("{what} at its foot"));
            }
            // (Layers keep the keys: a scroll takes none.)
            if step < 22 {
                h.key_press(egui::Key::ArrowDown);
                settle(&mut h);
            }
        }
    }
}

/// Properties' content's height, as drawn: from its first section's
/// heading to the foot of its last.
fn properties_height(h: &Harness<'_, Moonglow>) -> f32 {
    let top = h.get_all_by_label("Property Binding").next().unwrap().rect().top();
    let foot = h.get_all_by_label("Draw Layers").next().unwrap().rect().bottom();
    foot - top
}

/// Properties is never taller where wider, and comes to rest: the Right
/// Dock swept from 270 to 480 a point at a time beside a label of a long
/// text, a dropdown of long options and the window, its height held to the
/// least at any narrower width; around each width where its layout changes,
/// settled at window heights a few points apart.
#[test]
fn properties_are_never_taller_where_wider_and_come_to_rest() {
    let mut w = our_window();
    w["root"]["children"][0]["value"] =
        json!("A long label that wraps in the panel as a sentence of a book does, and more");
    w["root"]["children"][1] = json!({"type":"combo","id":"pick","value":0,"height":30.0,
        "elements":[["A first option of some length",0],["Another option that is longer still than it",1]]});
    // (A property bound: its row a chip, M4's U19, of a long bind's name.)
    let bound = json!({"type":"button","id":"bound","label":"Bound","height":30.0,
        "enabled":{"bind":"a_bind_of_a_long_name_that_is_cut_short"}});
    w["root"]["children"][2]["children"].as_array_mut().unwrap().push(bound);
    let mut h = designer_of(Mode::Tabs, 1600.0, w);
    for canvas in ["Canvas Label · A long label", "Canvas Dropdown", "Canvas Button · Bound"] {
        if canvas.ends_with("Bound") {
            // (Its Behavior opened: the chip in sight of the sweep.)
            h.get_all_by_label_contains(canvas).next().unwrap().click();
            settle(&mut h);
            h.get_by_label("Behavior").click();
            settle(&mut h);
            assert!(h.query_by_label("Bound to a_bind_of_a_long_name_that_is_cut_short").is_some());
        }
        h.state_mut().settings.right_dock_width = None;
        h.set_size(egui::vec2(1600.0, 900.0));
        settle(&mut h);
        let row = h.query_all_by_label_contains(canvas).next().is_some();
        assert!(row, "{canvas}");
        h.get_all_by_label_contains(canvas).next().unwrap().click();
        settle(&mut h);
        sweep_dock_heights(&mut h, canvas, properties_height, true);
    }
}

/// The window comes to rest (`settle`: no frame asked for once its work is
/// done) with each panel in front, behind and folded, the docks closed,
/// docked and floating, on each page, with the View popover open, while
/// typing in Properties, dragging and resizing on the canvas, choosing in
/// Layers, Interact on, after Undo, and after a delete that leaves no
/// control chosen (which changed the window).
#[test]
fn the_window_comes_to_rest_with_each_nui_panel() {
    let cases: [(Mode, &str, f32); 8] = [
        (Mode::Tabs, "stacked", 1920.0),
        (Mode::Tabs, "Components", 1300.0),
        (Mode::Tabs, "Properties", 1300.0),
        (Mode::Tabs, "Bindings", 1300.0),
        (Mode::Tabs, "folded", 1920.0),
        (Mode::Tabs, "closed", 1920.0),
        (Mode::Floating, "pane", 1920.0),
        (Mode::Floating, "small", 1280.0),
    ];
    for (mode, case, width) in cases {
        let mut h = designer(mode, width);
        let what = format!("{mode:?} {case}");
        match case {
            "Components" | "Properties" | "Bindings" => {
                let tab = h
                    .get_all_by_label(case)
                    .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab);
                tab.unwrap_or_else(|| panic!("{what}: its tab")).click();
                settle(&mut h);
            }
            "folded" => {
                h.state_mut().settings.hide_palettes = true;
                settle(&mut h);
            }
            "closed" => {
                h.state_mut().settings.hide_palettes = true;
                h.state_mut().settings.hide_tree = true;
                settle(&mut h);
            }
            _ => {}
        }
        choose(&mut h, "Label · Hear ye");
        // Typing in Properties, where it is drawn.
        if h.query_all_by_label("Text")
            .any(|n| n.accesskit_node().role() == egui::accesskit::Role::TextInput)
        {
            type_in(&mut h, "Text", "!");
            settle(&mut h);
            h.key_press(egui::Key::Enter);
            settle(&mut h);
        }
        // Choosing in Layers, where they are drawn.
        if h.query_all_by_label("Button · Send").next().is_some() {
            h.get_all_by_label("Button · Send").next().unwrap().click();
            settle(&mut h);
        }
        // Each page.
        for name in ["Bindings", "Events & Variants", "Images", "Design"] {
            page(&mut h, name);
        }
        // The View popover open.
        nui_tool(&mut h, nui_names::VIEW).click();
        settle(&mut h);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        // Resizing on the canvas: the window's corner dragged.
        choose(&mut h, "Button · Send");
        h.get_by_label("Canvas window").click();
        settle(&mut h);
        let from = h.get_by_label("Resize window width and height").rect().center();
        h.hover_at(from);
        h.drag_at(from);
        settle(&mut h);
        h.hover_at(from + egui::vec2(20.0, 10.0));
        settle(&mut h);
        h.drop_at(from + egui::vec2(20.0, 10.0));
        settle(&mut h);
        // Interact on and off.
        nui_tool(&mut h, nui_names::INTERACT).click();
        settle(&mut h);
        nui_tool(&mut h, nui_names::INTERACT).click();
        settle(&mut h);
        // Undo.
        tool(&h, "Undo").click();
        settle(&mut h);
        // A delete: the label, by the canvas's Delete.
        let before = document(&h);
        h.get_all_by_label_contains("Canvas Label · Hear ye").next().unwrap().click();
        settle(&mut h);
        let label = h.get_all_by_label_contains("Canvas Label · Hear ye").next().unwrap().rect();
        h.hover_at(label.center());
        h.key_press(egui::Key::Delete);
        settle(&mut h);
        let after = document(&h);
        assert_ne!(after, before, "{what}: the delete changed the window");
        // A control added (chosen), then Undo taking it away: nothing chosen
        // (its path is no control's), the window as before.
        if h.query_all_by_label("Button").next().is_some() {
            let added = document(&h);
            h.get_all_by_label("Button").next().unwrap().scroll_to_me();
            settle(&mut h);
            h.get_all_by_label("Button").next().unwrap().click();
            settle(&mut h);
            assert_ne!(document(&h), added, "{what}: added");
            tool(&h, "Undo").click();
            settle(&mut h);
            assert_eq!(document(&h), added, "{what}: taken away");
            assert_eq!(chosen(&h), "", "{what}: nothing chosen");
        }
        settle(&mut h);
    }
}

/// A Layers row dragged: before another (reorder), into a row (reparent),
/// one step of Undo each; docked, and in a floating designer's window.
#[test]
fn a_layers_row_dragged_reorders_and_reparents() {
    for mode in MODES {
        let mut h = designer(mode, 1920.0);
        let what = format!("{mode:?}");
        let before = document(&h);
        let drag = |h: &mut Harness<'_, Moonglow>, from: &str, onto: &str, by: f32| {
            let from = layer(h, from).rect().center();
            let onto = layer(h, onto).rect();
            let to = egui::pos2(onto.center().x, onto.top() + onto.height() * by);
            h.hover_at(from);
            settle(h);
            h.drag_at(from);
            settle(h);
            h.hover_at(from + egui::vec2(0.0, 8.0));
            settle(h);
            h.hover_at(to);
            settle(h);
            h.drop_at(to);
            settle(h);
        };
        // The label before the text input, after it: reordered.
        drag(&mut h, "Label · Hear ye", "Text input · Write your message", 0.85);
        let doc = document(&h);
        assert_eq!(doc["root"]["children"][1]["id"], "title", "{what}: {doc}");
        tool(&h, "Undo").click();
        settle(&mut h);
        assert_eq!(document(&h), before, "{what}: one step");
        // The label into the row: reparented, its last.
        drag(&mut h, "Label · Hear ye", "Row", 0.5);
        let doc = document(&h);
        let row = doc["root"]["children"][1]["children"].as_array().unwrap();
        assert!(row.iter().any(|c| c["id"] == "title"), "{what}: into the row {doc}");
        tool(&h, "Undo").click();
        settle(&mut h);
        assert_eq!(document(&h), before, "{what}: one step");
    }
}

/// The header's Reset sets the simulation of Interact back to its start:
/// a value typed on the canvas is the window's again; off outside Interact.
#[test]
fn reset_sets_the_simulation_back() {
    let mut h = designer(Mode::Tabs, 1920.0);
    let reset = |h: &mut Harness<'_, Moonglow>| {
        !nui_tool(h, nui_names::RESET).accesskit_node().is_disabled()
    };
    assert!(!reset(&mut h), "off outside Interact");
    nui_tool(&mut h, nui_names::INTERACT).click();
    settle(&mut h);
    assert!(reset(&mut h), "on in Interact");
    let before = document(&h);
    // (A control's state in the simulation: the window collapsed.)
    h.get_by_label("Collapse preview window").click();
    settle(&mut h);
    assert!(h.query_by_label("Expand preview window").is_some(), "collapsed");
    nui_tool(&mut h, nui_names::RESET).click();
    settle(&mut h);
    assert!(h.query_by_label("Collapse preview window").is_some(), "Reset: as it began");
    assert_eq!(document(&h), before, "the window itself untouched");
}

/// Layers' keys do nothing while a popover or a dialog is open over them:
/// Delete and Duplicate with the View popover open, and with How to Open
/// It… open, leave the window as it was; Escape closes each alone.
#[test]
fn layers_keys_wait_while_a_popover_or_a_dialog_is_open() {
    let mut h = designer(Mode::Tabs, 1920.0);
    layer(&h, "Button · Send").click();
    settle(&mut h);
    let before = document(&h);
    nui_tool(&mut h, nui_names::VIEW).click();
    settle(&mut h);
    h.key_press(egui::Key::Delete);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::D);
    settle(&mut h);
    assert_eq!(document(&h), before, "the popover open");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    nui_tool(&mut h, nui_names::HOW_TO_OPEN).click();
    settle(&mut h);
    let said = "Open it from a script of yours";
    assert!(h.query_by_label_contains(said).is_some(), "How to Open It… open");
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(document(&h), before, "the dialog open");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label_contains(said).is_none(), "closed");
    assert!(h.state().dock.find_tab(&Tab::Nui(key())).is_some(), "Escape closed it alone");
    // (Layers keep the keys: Delete now deletes.)
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_ne!(document(&h), before, "the keys Layers' again");
}

/// A floating designer whose window has not the room for its Layers' pane
/// folds them to a strip at the canvas's left; its icon unfolds them over
/// the canvas, where a row's click chooses the control and the keys move
/// from it, done as the frame ends.
#[test]
fn a_floating_designer_s_layers_fold_to_a_strip() {
    let mut h = designer(Mode::Floating, 1100.0);
    assert!(h.query_by_label("Find a layer or ID").is_none(), "folded");
    h.get_by_label(mg_ui::dialog_view::OUTLINER_STRIP).get_by_label(mg_ui::OUTLINER).click();
    settle(&mut h);
    layer(&h, "Button · Send").click();
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/2/children/0", "chosen from the strip's Layers");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/2/children/1", "their keys");
}

/// The canvas's keys are the active designer's: two designers in two
/// groups, a control chosen in each, the second's chosen last by a click on
/// the first's canvas; Delete with the pointer over the second's canvas
/// deletes nothing in either.
#[test]
fn the_canvas_keys_are_the_active_designer_s() {
    let mut h = designer(Mode::Tabs, 1920.0);
    let other = ResKey::parse("other", ResType::JUI).unwrap();
    let mut w = our_window();
    w["root"]["children"][0]["value"] = json!("Another window");
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.module.set(other, serde_json::to_vec_pretty(&w).unwrap());
    // (The other opened beside it, then split into a group of its own: the
    // two side by side.)
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(other)));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    assert!(h.query_by_label("Canvas Label · Hear ye").is_some(), "both drawn");
    choose(&mut h, "Label · Another window");
    choose(&mut h, "Label · Hear ye");
    let other_doc = |h: &Harness<'_, Moonglow>| {
        mg_nui::parse(h.state().ws.as_ref().unwrap().module.get(&other).unwrap()).unwrap()
    };
    let (mine, theirs) = (document(&h), other_doc(&h));
    let over = h.get_by_label("Canvas Label · Another window").rect().center();
    h.hover_at(over);
    settle(&mut h);
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!(other_doc(&h), theirs, "not the other designer's, under the pointer");
    assert_eq!(document(&h), mine, "nor the active one's, the pointer elsewhere");
    // (Over its own canvas, the active one's Delete deletes.)
    let own = h.get_by_label("Canvas Label · Hear ye").rect().center();
    h.hover_at(own);
    settle(&mut h);
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_ne!(document(&h), mine, "its own canvas's");
}

/// The window changed from elsewhere under a field still typed in (an edit
/// that numbers the controls anew: two labels, "Hear ye" and "Oyez",
/// swapped): its typing ends, what was typed lands nowhere (not on
/// "Oyez", now where "Hear ye" was); typed after, it goes to the control
/// chosen then.
#[test]
fn a_field_s_typing_ends_as_the_window_changes_under_it() {
    let mut w = our_window();
    w["root"]["children"][1] = json!({"type":"label","id":"other","value":"Oyez","height":30.0});
    let mut h = designer_of(Mode::Tabs, 1920.0, w);
    choose(&mut h, "Label · Hear ye");
    type_in(&mut h, "Text", "?");
    // (The two labels swapped, from elsewhere.)
    let mut w = document(&h);
    w["root"]["children"].as_array_mut().unwrap().swap(0, 1);
    let data = serde_json::to_vec_pretty(&w).unwrap();
    let edit = mg_edit::Edit::SetResource { key: key(), data: Some(data) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Swap", vec![edit])));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let doc = document(&h);
    assert_eq!(doc["root"]["children"][0]["value"], "Oyez", "not on the other label");
    assert_eq!(doc, w, "what was typed lands nowhere: {doc}");
    // Typed after, to the control chosen now (the first, Oyez).
    assert_eq!(chosen(&h), "/root/children/0");
    type_in(&mut h, "Text", "!");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let doc = document(&h);
    assert_eq!(doc["root"]["children"][0]["value"], "Oyez!", "typed after: {doc}");
    assert_eq!(doc["root"]["children"][1]["value"], "Hear ye", "the other kept: {doc}");
}

/// Properties stand as the board draws them (the coordinator's ruling):
/// each label beside its field in its column, the fields at one left edge;
/// Width and Height on one row; Source's dropdown and its plus (New Bind
/// for Property) on one row; the sections' headings fold (Content, a
/// click, folds away Element ID and brings it back); Add Property a
/// button named by its words alone (its plus drawn). The View popover's
/// rows are form rows too: their combo boxes at one left edge.
#[test]
fn properties_stand_as_the_board_draws_them() {
    let mut h = designer(Mode::Tabs, 1920.0);
    choose(&mut h, "Button · Send");
    let role = |h: &Harness<'_, Moonglow>, name: &str, role: egui::accesskit::Role| {
        let found = h.get_all_by_label(name).find(|n| n.accesskit_node().role() == role);
        found.unwrap_or_else(|| panic!("{name} as {role:?}")).rect()
    };
    use egui::accesskit::Role::{ComboBox, SpinButton, TextInput};
    let id = role(&h, "Element ID", TextInput);
    let label = role(&h, "Label", TextInput);
    let source = role(&h, "Source", ComboBox);
    let width = role(&h, "Width", SpinButton);
    let height = role(&h, "Height", SpinButton);
    // (The label column, 104, beside: the fields at its right.)
    for (name, r) in [("Label", label), ("Source", source), ("Width", width)] {
        assert_eq!(r.left(), id.left(), "{name} at the fields' edge");
    }
    // (Beside its label: the field past the label column, from the
    // panel's left, where Content's heading stands.)
    let edge = h.get_by_label("Content").rect().left();
    assert!(id.left() - edge >= 104.0 && id.left() - edge < 130.0, "{id:?} from {edge}");
    assert!((height.center().y - width.center().y).abs() < 1.0, "Width and Height on one row");
    assert!(height.left() > width.right());
    let plus = h.get_by_label("New Bind for a Property").rect();
    assert!((plus.center().y - source.center().y).abs() < 1.0 && plus.left() >= source.right());
    let add = h.get_by_label("Add Property");
    assert_eq!(add.accesskit_node().role(), egui::accesskit::Role::Button);
    // Content folds and unfolds.
    h.get_by_label("Content").click();
    settle(&mut h);
    assert!(h.query_by_label("Element ID").is_none(), "folded");
    h.get_by_label("Content").click();
    settle(&mut h);
    assert!(h.query_by_label("Element ID").is_some(), "unfolded");
    // The View popover's combo boxes at one edge.
    nui_tool(&mut h, nui_names::VIEW).click();
    settle(&mut h);
    let lefts: Vec<f32> = ["Layout", "UI scale", "State", "Screen"]
        .iter()
        .map(|name| h.get_all_by(combo(name)).next().unwrap().rect().left())
        .collect();
    assert!(lefts.iter().all(|l| *l == lefts[0]), "View's rows: {lefts:?}");
    // Their labels flush left in their column, as the checks under them.
    let check = h.get_all_by_label("Clean view").next().unwrap().rect().left();
    for name in ["Layout", "UI scale", "State", "Screen"] {
        let combo = h.get_all_by(combo(name)).next().unwrap().rect();
        let words = |n: &egui_kittest::Node<'_>| {
            let a = n.accesskit_node();
            a.role() == egui::accesskit::Role::Label
                && (a.label().as_deref() == Some(name) || a.value().as_deref() == Some(name))
        };
        let all = h.get_all_by_role(egui::accesskit::Role::Label);
        let label = all
            .filter(words)
            .map(|n| n.rect())
            .find(|r| r.right() <= combo.left() && (r.center().y - combo.center().y).abs() < 2.0);
        let label = label.unwrap_or_else(|| panic!("{name}'s label"));
        // (A check's row begins at its padding, 10 before its box.)
        assert!(
            (label.left() - check - 10.0).abs() <= 2.0,
            "{name} at {label:?}, checks at {check}"
        );
    }
}

/// The module's window and its settings, read.
fn read_both(h: &Harness<'_, Moonglow>) -> (Value, Settings) {
    let m = &h.state().ws.as_ref().unwrap().module;
    let s = Settings::parse(m.get(&ResKey::new(key().resref, ResType::TXT)).unwrap()).unwrap();
    (mg_nui::parse(m.get(&key()).unwrap()).unwrap(), s)
}

/// A variant edited in its slot keeps to the variant: a label typed in
/// sets the variant's, the window untouched; Split source shows it; Back
/// to Main Window (from the Editor and from Split source) brings the
/// window's own contents back, the window and the variant as they were.
#[test]
fn a_variant_edited_keeps_to_it_through_split_source_and_back() {
    let (w, st) = with_a_variant();
    let mut h = designer_with(Mode::Tabs, 1920.0, w.clone(), st);
    layer(&h, "Variant · details").click();
    settle(&mut h);
    choose(&mut h, "Label · Left");
    type_in(&mut h, "Text", "!");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let (doc, s) = read_both(&h);
    assert_eq!(doc, w, "the window untouched");
    assert_eq!(s.variants["details"]["children"][0]["value"], "Left!", "the variant's");
    for layout in ["Split source", "Editor"] {
        layer(&h, "Variant · details").click();
        settle(&mut h);
        layout_chosen(&mut h, layout);
        if layout == "Split source" {
            let shows = h.query_all_by_role(egui::accesskit::Role::MultilineTextInput);
            let mut shows = shows.filter_map(|n| n.accesskit_node().value());
            assert!(shows.any(|t| t.contains("Left!")), "Split source shows the variant");
        }
        h.get_all_by_label("Back to Main Window").next().expect("Back").click();
        settle(&mut h);
        assert!(h.query_by_label("Canvas Label · Initial").is_some(), "{layout}: back");
        assert_eq!(read_both(&h), (doc.clone(), s.clone()), "{layout}: as they were");
    }
}

/// A designer closed while a field is typed in sets what was typed, as a
/// field whose panel goes does (the tip lost it); its OK sets it too, its
/// Cancel sets nothing.
#[test]
fn a_designer_closed_while_typed_in_sets_what_was_typed() {
    for (how, kept) in [("closed", true), ("OK", true), ("Cancel", false)] {
        let mut h = designer(Mode::Tabs, 1920.0);
        choose(&mut h, "Label · Hear ye");
        type_in(&mut h, "Text", "!");
        match how {
            "closed" => {
                h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Nui(key())]));
            }
            _ => h.get_by_label(NUI_HEADER).get_by_label(how).click(),
        }
        settle(&mut h);
        assert!(h.state().dock.find_tab(&Tab::Nui(key())).is_none(), "{how}: closed");
        let want = if kept { "Hear ye!" } else { "Hear ye" };
        assert_eq!(document(&h)["root"]["children"][0]["value"], want, "{how}");
    }
}

/// One designer to a window: opened again, the same tab is brought to
/// the front, never a second (two designers of one window cannot be).
#[test]
fn a_window_has_one_designer() {
    for mode in MODES {
        let mut h = designer(mode, 1920.0);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
        settle(&mut h);
        let tabs = h.state().dock.iter_all_tabs().filter(|(_, t)| **t == Tab::Nui(key())).count();
        assert_eq!(tabs, 1, "{mode:?}");
    }
}

/// The NUI keys remapped are the keys: Move Element Up on Shift+U moves
/// the control from Layers and on the canvas; its old Alt+Up no longer
/// does. Escape during a canvas resize, Layers having had the keys,
/// drops the resize.
#[test]
fn remapped_nui_keys_and_escape_during_a_resize() {
    let shift_u = egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::U);
    let cancel_first = |h: &Harness<'_, Moonglow>| {
        document(h)["root"]["children"][2]["children"][0]["id"] == "cancel"
    };
    for on_canvas in [false, true] {
        let mut h = designer(Mode::Tabs, 1920.0);
        h.state_mut().keymap.set(mg_ui::keys::Cmd::NuiMoveUp, vec![shift_u]);
        if on_canvas {
            choose(&mut h, "Button · Cancel");
            h.hover_at(h.get_by_label("Canvas Button · Cancel").rect().center());
        } else {
            layer(&h, "Button · Cancel").click();
        }
        settle(&mut h);
        h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowUp);
        settle(&mut h);
        assert!(!cancel_first(&h), "canvas {on_canvas}: the old key does nothing");
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::U);
        settle(&mut h);
        assert!(cancel_first(&h), "canvas {on_canvas}: moved up");
    }
    let mut h = designer(Mode::Tabs, 1920.0);
    layer(&h, "Label · Hear ye").click();
    settle(&mut h);
    let before = document(&h);
    let r = h.get_by_label("Canvas Label · Hear ye").rect();
    let from = r.right_bottom() - egui::vec2(1.0, 1.0);
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(40.0, 30.0));
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    h.drop_at(from + egui::vec2(40.0, 30.0));
    settle(&mut h);
    assert_eq!(document(&h), before, "the resize dropped");
}

/// In Interact the window comes to rest: a control's hover and a click
/// played, the frames end.
#[test]
fn interact_comes_to_rest() {
    let mut h = designer(Mode::Tabs, 1920.0);
    nui_tool(&mut h, nui_names::INTERACT).click();
    settle(&mut h);
    let send = h.get_by_label("Canvas Button · Send").rect().center();
    h.hover_at(send);
    settle(&mut h);
    h.get_by_label("Canvas Button · Send").click();
    settle(&mut h);
    h.hover_at(egui::pos2(900.0, 700.0));
    settle(&mut h);
}

/// Save with a field still typed in saves what it shows, the field keeping
/// the keyboard; the window gone from the module, or changed from
/// elsewhere, in the frame the designer closes with a field typed in:
/// what was typed lands nowhere (the outside bytes kept), and closing the
/// module with one typed in writes nothing and does not fault.
#[test]
fn typed_text_is_saved_and_never_written_over_other_bytes() {
    let dir = mg_testkit::scratch_dir("ui-nui-save-typed");
    let path = dir.join("crier.mod");
    // (Each time from the module as it was.)
    let open = || {
        let mut module = Module::new();
        module.set_info(&Gff::new(*b"IFO ")).unwrap();
        module.set(key(), serde_json::to_vec_pretty(&our_window()).unwrap());
        module.set(ResKey::new(key().resref, ResType::TXT), our_settings().bytes());
        module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        let mut app = app_in(Mode::Tabs, Vec::new());
        app.settings.no_last_area = true;
        app.open_module(&path);
        let mut h = harness(app, 1920.0, 900.0);
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
        settle(&mut h);
        choose(&mut h, "Label · Hear ye");
        type_in(&mut h, "Text", "!");
        h
    };
    let mut h = open();
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    let saved = Module::open(&path).unwrap();
    let doc = mg_nui::parse(saved.get(&key()).unwrap()).unwrap();
    assert_eq!(doc["root"]["children"][0]["value"], "Hear ye!", "saved as shown");
    let field = h.get_all_by_role(egui::accesskit::Role::TextInput).find(|n| {
        n.accesskit_node().label().as_deref() == Some("Text") && n.accesskit_node().is_focused()
    });
    assert!(field.is_some(), "the field keeps the keyboard");
    // Changed from elsewhere, or gone, as the designer closes.
    for gone in [false, true] {
        let mut h = open();
        let mut w = our_window();
        w["title"] = json!("Elsewhere");
        let data = (!gone).then(|| serde_json::to_vec_pretty(&w).unwrap());
        let edit = mg_edit::Edit::SetResource { key: key(), data };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Out", vec![edit])));
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Nui(key())]));
        settle(&mut h);
        let now = h.state().ws.as_ref().unwrap().module.get(&key()).map(|b| b.to_vec());
        match gone {
            true => assert!(now.is_none(), "still gone"),
            false => assert_eq!(mg_nui::parse(&now.unwrap()).unwrap(), w, "the outside bytes kept"),
        }
    }
    // The module closed with a field typed in.
    let mut h = open();
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    let saved = Module::open(&path).unwrap();
    let doc = mg_nui::parse(saved.get(&key()).unwrap()).unwrap();
    assert_eq!(doc["root"]["children"][0]["value"], "Hear ye", "nothing written");
}

/// With a variant edited in its slot, the Bindings panel works on the
/// window as it is kept: a bind renamed there follows into the window's
/// own controls (held aside while the variant stands in the root) and
/// into the variant's.
#[test]
fn a_bind_renamed_with_a_variant_edited_reaches_the_window_and_the_variant() {
    let (w, mut st) = with_a_variant();
    st.variants.get_mut("details").unwrap()["children"][0]["value"] = json!({"bind":"message"});
    let mut h = designer_with(Mode::Tabs, 1920.0, w, st);
    layer(&h, "Variant · details").click();
    settle(&mut h);
    // (The bind chosen in the Bindings table shows its form: M4's U19.)
    h.get_by_label("message").click();
    settle(&mut h);
    let name = |a: &egui_kittest::kittest::AccessKitNode<'_>| {
        a.role() == egui::accesskit::Role::TextInput
            && a.label().as_deref() == Some("Name")
            && a.value().as_deref() == Some("message")
    };
    h.get_all_by(name).next().expect("message's Name").scroll_to_me();
    settle(&mut h);
    let at = h.get_all_by(name).next().unwrap().rect();
    h.get_all_by(name).next().unwrap().click();
    settle(&mut h);
    h.key_press(egui::Key::End);
    let field = h.get_all_by_role(egui::accesskit::Role::TextInput).find(|n| n.rect() == at);
    field.unwrap().type_text("_2");
    h.run();
    let rename = h.get_all_by_label("Rename").min_by(|a, b| {
        let (a, b) =
            (a.rect().center().distance(at.center()), b.rect().center().distance(at.center()));
        a.total_cmp(&b)
    });
    rename.unwrap().click();
    settle(&mut h);
    let (doc, s) = read_both(&h);
    assert_eq!(doc["root"]["children"][1]["value"], json!({"bind":"message_2"}), "the window's");
    assert_eq!(
        s.variants["details"]["children"][0]["value"],
        json!({"bind":"message_2"}),
        "the variant's"
    );
}

// ------------------------------------------------- M4's U19: the source
// group, Checks, the canvas's corners and the Status Bar, the Outliner's
// marks, the bind chip and the Bindings table.

/// The module's window set to `window`, as an edit from elsewhere.
fn set_window(h: &mut Harness<'_, Moonglow>, window: &Value) {
    let edit = mg_edit::Edit::SetResource {
        key: key(),
        data: Some(serde_json::to_vec_pretty(window).unwrap()),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
    settle(h);
}

/// [`our_window`] with a text input whose value no bind keeps (the NUI
/// board's "Text input · Enter text", the checks' one warning).
fn with_a_warning() -> Value {
    let mut w = our_window();
    let enter = json!({"type":"textedit","label":"Enter text","value":"","height":30.0});
    w["root"]["children"].as_array_mut().unwrap().insert(2, enter);
    w
}

/// The Checks switch's count follows the checks (the plan's): none where
/// they find nothing, the warnings and errors where they find some, in
/// `warn` or `error`; a warning opens nothing, an error opens the Bottom
/// Dock on Checks (U13's rule), Undo leaves it open; a row's control
/// chosen by its link, in Layers too (its warning mark), and Bind It puts
/// it right, one Undo step. Floating, the window's bottom pane takes it.
#[test]
fn the_checks_switch_s_count_follows_the_checks() {
    use mg_uikit::BadgeKind;
    let mut h = designer(Mode::Tabs, 1920.0);
    let badge = |h: &Harness<'_, Moonglow>| h.state().switch_badge(Panel::Checks);
    assert_eq!(badge(&h), None, "nothing found, no count");
    // (Said in the panel, in sight in the Bottom Dock.)
    h.get_all_by_label("Checks").next().unwrap().click();
    settle(&mut h);
    let said = h.get_by_label("The checks found nothing.").rect();
    let dock = h.state().bottom_dock_rect().expect("the Bottom Dock open");
    assert!(dock.contains_rect(said), "nothing found, said: {said:?} in {dock:?}");
    h.get_all_by_label("Checks").next().unwrap().click();
    settle(&mut h);
    // (Frames counted, not settled: a request made every frame would keep
    // them coming, and the assertion is the test's, not a timeout.)
    let edit = mg_edit::Edit::SetResource {
        key: key(),
        data: Some(serde_json::to_vec_pretty(&with_a_warning()).unwrap()),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
    h.run_steps(8);
    assert_eq!(badge(&h), Some(("1".into(), BadgeKind::Warn)), "a warning");
    assert_ne!(h.state().bottom_dock_shows(), Some(Panel::Checks), "a warning opens nothing");
    settle(&mut h);
    // An error: the window no longer reads.
    let edit = mg_edit::Edit::SetResource { key: key(), data: Some(b"{".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Out", vec![edit])));
    settle(&mut h);
    assert_eq!(badge(&h), Some(("1".into(), BadgeKind::Error)), "an error");
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Checks), "a failure opens it");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(badge(&h), Some(("1".into(), BadgeKind::Warn)), "the warning again");
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Checks), "left open");
    // Fixed (the Undo) and made again, the dock closed meanwhile: opened again.
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    tool(&h, "Redo").click();
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Checks), "the error made again");
    tool(&h, "Undo").click();
    settle(&mut h);
    // The row: its control's link chooses it; its mark in Layers.
    let rows = h.get_by_label(nui_names::CHECKS_ROWS);
    rows.get_by_label("Text input · Enter text").click();
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/2", "chosen by the row's link");
    assert!(h.query_by_label_contains("Warning: What the player enters here is lost").is_some());
    let before = document(&h);
    h.get_by_label(nui_names::CHECKS_ROWS).get_by_label("Bind It").click();
    settle(&mut h);
    assert_eq!(badge(&h), None, "put right");
    assert!(document(&h)["root"]["children"][2]["value"]["bind"].is_string());
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(document(&h), before, "one Undo step");
    // Floating: an error brings Checks to its window's bottom pane.
    let mut h = designer_of(Mode::Floating, 1920.0, with_a_warning());
    assert_eq!(h.state().window_bottom_for_test(&Tab::Nui(key())), None);
    let edit = mg_edit::Edit::SetResource { key: key(), data: Some(b"[".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Out", vec![edit])));
    settle(&mut h);
    assert_eq!(h.state().window_bottom_for_test(&Tab::Nui(key())), Some(Panel::Checks));
    assert!(h.query_by_label(nui_names::CHECKS_ROWS).is_some(), "drawn in its window");
}

/// The source group opens beside the design and keeps its switch (the
/// plan's): View › Layout › Split source opens it as a group of its own at
/// the design's right, in its window where it floats; Binding JSON chosen
/// on its switch is kept as it closes (Layout's Editor, its own ×) and
/// opens again; the design's tab closed takes it with it.
#[test]
fn the_source_group_opens_beside_the_design_and_keeps_its_switch() {
    let source = Tab::NuiSource(key());
    for mode in MODES {
        let mut h = designer(mode, 1920.0);
        let what = format!("{mode:?}");
        layout_chosen(&mut h, "Split source");
        let dock = &h.state().dock;
        let design = dock.find_tab(&Tab::Nui(key())).unwrap();
        let at = dock.find_tab(&source).unwrap_or_else(|| panic!("{what}: opened"));
        assert_eq!(at.surface, design.surface, "{what}: in the design's window");
        assert_ne!(at.node, design.node, "{what}: a group of its own");
        let (d, s) = (h.state().tab_rect(&Tab::Nui(key())).unwrap(), h.state().tab_rect(&source));
        assert!(s.unwrap().left() >= d.right() - 1.0, "{what}: at the design's right");
        assert!(h.query_by_label(nui_names::SOURCE_TEXT).is_some(), "{what}: JUI Source");
        h.get_by_label(nui_names::SOURCE_HEADER).get_by_label(nui_names::BINDING_JSON).click();
        settle(&mut h);
        assert!(h.query_by_label(nui_names::SETTINGS_TEXT).is_some(), "{what}: Binding JSON");
        layout_chosen(&mut h, "Editor");
        assert!(h.state().dock.find_tab(&source).is_none(), "{what}: closed by the Layout");
        layout_chosen(&mut h, "Split source");
        assert!(h.query_by_label(nui_names::SETTINGS_TEXT).is_some(), "{what}: its switch kept");
        h.state_mut().actions.push(mg_ui::Action::CloseTab(source.clone()));
        settle(&mut h);
        assert!(h.state().dock.find_tab(&source).is_none(), "{what}: its ×");
        layout_chosen(&mut h, "Split source");
        assert!(h.query_by_label(nui_names::SETTINGS_TEXT).is_some(), "{what}: kept again");
        h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Nui(key())));
        settle(&mut h);
        assert!(h.state().dock.find_tab(&source).is_none(), "{what}: gone with the design");
    }
}

/// What is typed in the source is the module's when its field is left:
/// by the switch moved, by a press on the design, by the group closed with
/// the keyboard still in it, each one Undo step; the canvas shows it as it
/// is typed. The keys are the active editor's: Delete in the source deletes
/// text, not the control chosen; Tab puts a tab in it; Delete over the
/// canvas once it is pressed deletes the control.
#[test]
fn typed_source_is_set_as_it_is_left_and_the_keys_are_the_active_group_s() {
    let source = Tab::NuiSource(key());
    let typed_in = |h: &mut Harness<'_, Moonglow>, field: &str, at_end: &str| {
        h.get_by_label(field).click();
        settle(h);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
        h.get_by_label(field).type_text(at_end);
        h.run();
    };
    for mode in MODES {
        let what = format!("{mode:?}");
        let mut h = designer(mode, 1920.0);
        choose(&mut h, "Button · Send");
        layout_chosen(&mut h, "Split source");
        let before = document(&h);
        // (The window's text: a label's words changed, live on the canvas.)
        let text =
            String::from_utf8(h.state().ws.as_ref().unwrap().module.get(&key()).unwrap().to_vec())
                .unwrap();
        let at = text.find("Hear ye").unwrap() + "Hear ye".len();
        h.get_by_label(nui_names::SOURCE_TEXT).click();
        settle(&mut h);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.key_press(egui::Key::ArrowLeft);
        h.run();
        for _ in 0..text[..at].chars().count() {
            h.key_press(egui::Key::ArrowRight);
        }
        h.run();
        h.get_by_label(nui_names::SOURCE_TEXT).type_text("!");
        h.run();
        assert!(h.query_by_label("Canvas Label · Hear ye!").is_some(), "{what}: shown as typed");
        assert_eq!(document(&h), before, "{what}: a draft until left");
        // Delete is the text's: the control chosen (the label the cursor is
        // in, as the design follows it) stays.
        let was = chosen(&h);
        assert_eq!(was, "/root/children/0", "{what}: the cursor's control chosen");
        h.key_press(egui::Key::Delete);
        h.run();
        assert_eq!(chosen(&h), was, "{what}: still chosen");
        assert_eq!(document(&h), before, "{what}: nothing deleted");
        // (The quote Delete took, typed back; Tab at the line's end puts a
        // tab there, not the keys anywhere else.)
        h.get_by_label(nui_names::SOURCE_TEXT).type_text("\"");
        h.run();
        h.key_press(egui::Key::End);
        h.run();
        h.key_press(egui::Key::Tab);
        h.run();
        assert!(
            h.query_by_label(nui_names::SOURCE_TEXT)
                .is_some_and(|n| n.accesskit_node().value().is_some_and(|v| v.contains(",\t\n"))),
            "{what}: a tab typed"
        );
        // The switch moved: set, one Undo step.
        h.get_by_label(nui_names::SOURCE_HEADER).get_by_label(nui_names::BINDING_JSON).click();
        settle(&mut h);
        let set = document(&h);
        assert_eq!(set["root"]["children"][0]["value"], "Hear ye!", "{what}: set by the switch");
        tool(&h, "Undo").click();
        settle(&mut h);
        assert_eq!(document(&h), before, "{what}: one Undo step");
        // In Binding JSON, typed and the group closed with the keys in it.
        typed_in(&mut h, nui_names::SETTINGS_TEXT, " ");
        let settings_before = h
            .state()
            .ws
            .as_ref()
            .unwrap()
            .module
            .get(&ResKey::new(key().resref, ResType::TXT))
            .unwrap()
            .to_vec();
        h.state_mut().actions.push(mg_ui::Action::CloseTab(source.clone()));
        settle(&mut h);
        let now = h
            .state()
            .ws
            .as_ref()
            .unwrap()
            .module
            .get(&ResKey::new(key().resref, ResType::TXT))
            .unwrap()
            .to_vec();
        assert_ne!(now, settings_before, "{what}: set as the group closed");
        // The source group in front, nothing typed in: Delete over the
        // canvas is not the canvas's (its keys are the active editor's).
        layout_chosen(&mut h, "Split source");
        choose(&mut h, "Button · Send");
        h.get_by_label(nui_names::SOURCE_HEADER).get_by_label(nui_names::JUI_SOURCE).click();
        settle(&mut h);
        let kept = document(&h);
        h.hover_at(h.get_by_label("Canvas Button · Send").rect().center());
        h.key_press(egui::Key::Delete);
        settle(&mut h);
        assert_eq!(document(&h), kept, "{what}: the source in front, the canvas's Delete waits");
        // The canvas's keys again where it is pressed: Delete deletes.
        choose(&mut h, "Button · Send");
        h.hover_at(h.get_by_label("Canvas Button · Send").rect().center());
        h.key_press(egui::Key::Delete);
        settle(&mut h);
        let row = &document(&h)["root"]["children"][2]["children"];
        assert_eq!(row.as_array().unwrap().len(), 1, "{what}: the canvas's Delete");
    }
}

/// The design and the source follow each other (the coordinator's ruling
/// at U19's review): a control chosen on the canvas brings the text's cursor
/// to its first line, that line alone marked (the board's); a click in the
/// text chooses the control it is in, and the cursor moved by the keys goes
/// on choosing while it was the text that chose; a control chosen on the
/// canvas is not replaced by reading with the keys (the text given the
/// keyboard without a click), and several chosen in Layers are never let go
/// by the cursor, by keys or by a click.
#[test]
fn the_design_and_the_source_follow_each_other() {
    let mut h = designer(Mode::Tabs, 1920.0);
    layout_chosen(&mut h, "Split source");
    choose(&mut h, "Button · Cancel");
    h.run();
    let text =
        String::from_utf8(h.state().ws.as_ref().unwrap().module.get(&key()).unwrap().to_vec())
            .unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let (a, b) = h.state().nui_source_lines(&key()).expect("the cursor's line marked");
    assert_eq!(a, b, "one line marked");
    let cancel = text.find("\"cancel\"").unwrap();
    let first = text[..text[..cancel].rfind('{').unwrap()].matches('\n').count();
    assert_eq!(a, first, "Cancel's first line: {:?}", lines[a]);
    // Its band on that line's row, at the script editor's pitch (18).
    let field = h.get_by_label(nui_names::SOURCE_TEXT).rect();
    let run = egui_kittest::kittest::by().role(egui::accesskit::Role::TextRun);
    let row0 = h
        .query_all(run)
        .map(|n| n.rect())
        .filter(|r| field.x_range().contains(r.left()) && field.y_range().contains(r.top()))
        .map(|r| r.top())
        .fold(f32::INFINITY, f32::min);
    let band = Moonglow::nui_source_band_for_test(&h.ctx).expect("a band");
    let row = row0 + a as f32 * 18.0;
    assert_eq!((band.min, band.max), (row, row + 18.0), "line {a}'s band");
    // The gutter's numbers painted on their rows, before the text (those
    // around the chosen line, which is scrolled into sight).
    let texts = painted_texts(&h);
    for n in a - 2..a + 3 {
        let y = egui::Rangef::new(row0 + n as f32 * 18.0, row0 + (n + 1) as f32 * 18.0);
        let number = (n + 1).to_string();
        let on = texts
            .iter()
            .any(|(t, r)| *t == number && r.right() < field.left() && y.contains(r.center().y));
        assert!(on, "line {number}'s number on its row");
    }
    // For a screen reader: the gutter by what it shows, the field by the
    // cursor's line.
    let lines = text.lines().count();
    assert!(h.query_by_label(&format!("Line numbers 1 to {lines}")).is_some(), "the gutter named");
    let field = h.get_by_label(nui_names::SOURCE_TEXT);
    let line = format!("Line {} of {lines}", a + 1);
    assert_eq!(field.accesskit_node().description().as_deref(), Some(line.as_str()));
    // A click in the text: the control it is in, then the keys go on.
    let at = text.find("\"Hear ye\"").unwrap();
    let to_start = |h: &mut Harness<'_, Moonglow>| {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.key_press(egui::Key::ArrowLeft);
        h.run();
    };
    let right = |h: &mut Harness<'_, Moonglow>, n: usize| {
        for _ in 0..n {
            h.key_press(egui::Key::ArrowRight);
        }
        settle(h);
    };
    h.get_by_label(nui_names::SOURCE_TEXT).click();
    settle(&mut h);
    let clicked = chosen(&h);
    assert_ne!(clicked, "/root/children/2/children/1", "the click chose its control");
    to_start(&mut h);
    right(&mut h, text[..at].chars().count());
    assert_eq!(chosen(&h), "/root/children/0", "the keys go on choosing");
    // Chosen on the canvas, the text given the keyboard without a click:
    // reading with the keys does not replace it.
    choose(&mut h, "Button · Cancel");
    h.get_by_label(nui_names::SOURCE_TEXT).focus();
    settle(&mut h);
    to_start(&mut h);
    right(&mut h, text[..at].chars().count());
    assert_eq!(chosen(&h), "/root/children/2/children/1", "the canvas's choice kept");
    // Several chosen in Layers: neither the keys nor a click let them go.
    layer(&h, "Button · Send").click();
    settle(&mut h);
    let ctrl = egui::Modifiers::COMMAND;
    layer(&h, "Button · Cancel").click_modifiers(ctrl);
    settle(&mut h);
    let several = h.state().nui_chosen_many(&key());
    assert_eq!(several.len(), 2, "two chosen");
    h.get_by_label(nui_names::SOURCE_TEXT).focus();
    settle(&mut h);
    right(&mut h, 3);
    assert_eq!(h.state().nui_chosen_many(&key()), several, "the keys keep them");
    h.get_by_label(nui_names::SOURCE_TEXT).click();
    settle(&mut h);
    assert_eq!(h.state().nui_chosen_many(&key()), several, "a click keeps them");
    // Binding JSON shows no control's line: no band, not the last one drawn.
    h.get_by_label(nui_names::SOURCE_HEADER).get_by_label(nui_names::BINDING_JSON).click();
    settle(&mut h);
    assert!(h.query_by_label(nui_names::SETTINGS_TEXT).is_some(), "Binding JSON shown");
    assert_eq!(Moonglow::nui_source_band_for_test(&h.ctx), None, "a band left from JUI Source");
}

/// A bound property's chip (the board's): beside its label, a click shows
/// it in Property Binding, whose Source takes the bind off (one Undo step);
/// its formatting and initial value there.
#[test]
fn a_bound_property_s_chip_shows_it_in_property_binding() {
    // (Send's Enabled bound, as the board's: Property Binding begins on its
    // Label, the chip brings it to Enabled.)
    let mut w = our_window();
    w["root"]["children"][2]["children"][0]["enabled"] = json!({"bind": "can_send"});
    let mut s = our_settings();
    s.bindings
        .insert("can_send".into(), mg_nui::Binding { value: json!(false), ..Default::default() });
    let mut h = designer_with(Mode::Tabs, 1920.0, w, s);
    choose(&mut h, "Button · Send");
    let property = |h: &Harness<'_, Moonglow>| {
        let combo = h.get_all_by(combo("Property")).next().unwrap();
        combo.accesskit_node().value()
    };
    assert_eq!(property(&h).as_deref(), Some("Label"), "Property Binding's first");
    h.get_by_label("Behavior").click();
    settle(&mut h);
    let chip = h.get_by_label("Bound to can_send");
    assert!(chip.rect().width() > 20.0, "the chip");
    // (By the keyboard: the chip given the focus, Enter.)
    chip.focus();
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(property(&h).as_deref(), Some("Enabled"), "its property");
    // The table marks the bind the property takes its value from.
    let row = h.get_by_label("can_send");
    let said = row.accesskit_node().description();
    assert_eq!(said.as_deref(), Some("Bound to the property chosen"), "can_send's row marked");
    assert!(h.query_by_label("Bind Formatting").is_some(), "its formatting there");
    let before = document(&h);
    h.get_all_by(combo("Source")).next().unwrap().click();
    settle(&mut h);
    h.get_by_label("Constant (no bind)").click();
    settle(&mut h);
    let enabled = &document(&h)["root"]["children"][2]["children"][0]["enabled"];
    assert!(enabled.is_boolean(), "taken off: {enabled}");
    assert!(h.query_by_label("Bound to can_send").is_none(), "no chip");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(document(&h), before, "one Undo step");
}

/// The Bindings panel's table (the board's): a row a bind with its value;
/// Watch adds and takes off its handler, one Undo step each; a row chosen
/// shows its bind's form, where it is renamed; the plus on its header opens
/// New Bind; the Bindings page shows every bind's form, the panel the table
/// alone (one of each field).
#[test]
fn the_bindings_table_does_what_the_page_did() {
    for mode in MODES {
        let what = format!("{mode:?}");
        let mut h = designer(mode, 1920.0);
        assert!(h.query_by_label(nui_names::BINDS_TABLE).is_some(), "{what}: the table");
        let read = |h: &Harness<'_, Moonglow>| {
            let module = &h.state().ws.as_ref().unwrap().module;
            Settings::parse(module.get(&ResKey::new(key().resref, ResType::TXT)).unwrap()).unwrap()
        };
        h.get_by_label("Watch message").click();
        settle(&mut h);
        assert!(read(&h).handlers.iter().any(|r| r.event == "watch" && r.element == "message"));
        tool(&h, "Undo").click();
        settle(&mut h);
        assert!(read(&h).handlers.is_empty(), "{what}: one Undo step");
        h.get_by_label("message").click();
        settle(&mut h);
        type_in(&mut h, "Name", "_2");
        settle(&mut h);
        h.get_by_label("Rename").click();
        settle(&mut h);
        assert!(read(&h).bindings.contains_key("message_2"), "{what}: renamed");
        assert_eq!(document(&h)["root"]["children"][1]["value"]["bind"], "message_2");
        h.get_by_label("New Bind").click();
        settle(&mut h);
        assert!(h.query_by_label("Bind name").is_some(), "{what}: New Bind's dialog");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        page(&mut h, "Bindings");
        let names = h
            .query_all_by_label("Name")
            .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::TextInput)
            .count();
        assert_eq!(names, 1, "{what}: the bind's form once, on the page");
        assert!(h.query_by_label(nui_names::BINDS_TABLE).is_some(), "{what}: the panel's table");
    }
}

/// The canvas's hints are the Status Bar's (the board's): Click Select,
/// Drag Resize by handle, Esc Cancel resize, and with a control chosen its
/// keys as bound (remapped: the new key); none while a field is typed in,
/// a popover is open, another page is in front or the source group is the
/// active editor; Interact's own. Its counts: Not built in `warn`, the
/// binds and handlers.
#[test]
fn the_canvas_hints_and_counts_are_the_status_bar_s() {
    let mut h = designer(Mode::Tabs, 1920.0);
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said().join(" | ");
    let hinted = |h: &Harness<'_, Moonglow>, words: &str| said(h).contains(words);
    assert!(hinted(&h, "Select") && hinted(&h, "Resize by handle") && hinted(&h, "Cancel resize"));
    assert!(!hinted(&h, "Duplicate"), "nothing chosen: {}", said(&h));
    choose(&mut h, "Button · Send");
    for words in ["Duplicate", "Move up", "Delete"] {
        assert!(hinted(&h, words), "{words}: {}", said(&h));
    }
    assert!(hinted(&h, "Not built") && hinted(&h, "1 bind, 0 handlers"), "{}", said(&h));
    // Remapped: the new key.
    let shift_u = egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::U);
    h.state_mut().keymap.set(mg_ui::keys::Cmd::NuiMoveUp, vec![shift_u]);
    settle(&mut h);
    assert!(said(&h).contains("Shift, U: Move up"), "{}", said(&h));
    // A field typed in, a popover open, another page: none.
    let at = field_in_sight(&mut h, "Label");
    h.get_all_by_role(egui::accesskit::Role::TextInput).find(|n| n.rect() == at).unwrap().click();
    settle(&mut h);
    assert!(!hinted(&h, "Select"), "typing: {}", said(&h));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    nui_tool(&mut h, nui_names::VIEW).click();
    settle(&mut h);
    assert!(!hinted(&h, "Select"), "a popover: {}", said(&h));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    page(&mut h, "Bindings");
    assert!(!hinted(&h, "Select"), "Bindings: {}", said(&h));
    page(&mut h, "Design");
    assert!(hinted(&h, "Select"), "back: {}", said(&h));
    // The source group in front: its counts, no canvas hints.
    layout_chosen(&mut h, "Split source");
    assert!(!hinted(&h, "Select") && hinted(&h, "Not built"), "the source: {}", said(&h));
    layout_chosen(&mut h, "Editor");
    nui_tool(&mut h, nui_names::INTERACT).click();
    settle(&mut h);
    assert!(hinted(&h, "Try a control") && !hinted(&h, "Duplicate"), "{}", said(&h));
}

/// The canvas's corners (the board's): "Game preview" at its top left,
/// the zoom at its bottom right over the window's room (no row of their
/// own), Fit and 100% each doing what the View popover's Zoom does.
#[test]
fn the_canvas_s_corners_hold_the_zoom_and_the_label() {
    // (A window larger than the canvas: Fit draws it smaller than 100%.)
    let mut big = our_window();
    big["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 1500.0, "h": 1200.0});
    for mode in MODES {
        let what = format!("{mode:?}");
        let mut h = designer_of(mode, 1920.0, big.clone());
        // (No game here: "Preview · game skin unavailable · UI scale 100%".)
        let label = h.get_by_label_contains("UI scale 100%").rect();
        let zoom = h.get_by_label("Zoom").rect();
        let canvas = h.get_by_label("Canvas window").rect();
        assert!(zoom.top() > canvas.bottom(), "{what}: the zoom at the bottom");
        assert!(label.bottom() < canvas.top(), "{what}: the label at the top");
        // (Over the canvas's room, under the header: no row between.)
        let header = editor_rect(&h);
        let gap = label.top() - header.bottom();
        assert!((0.0..16.0).contains(&gap), "{what}: the label {gap} under the header");
        let fit = h.get_by_label("Canvas window").rect().width();
        h.get_by_label("Zoom").get_by_label("100%").click();
        settle(&mut h);
        let full = h.get_by_label("Canvas window").rect().width();
        assert!(full != fit, "{what}: 100% {full} against Fit {fit}");
        h.get_by_label("Zoom").get_by_label("Fit").click();
        settle(&mut h);
        assert_eq!(h.get_by_label("Canvas window").rect().width(), fit, "{what}: Fit again");
    }
}

/// No endless redraw with U19's parts (as `the_window_comes_to_rest_with_
/// each_nui_panel`): the window comes to rest, docked and floating, with
/// the source group opened, on each switch position, typed in and left,
/// closed; Checks in front, the design's tab brought forward, folded (the
/// Bottom Dock closed) and opened again, with a warning; the corners' zoom; the Bindings table's row
/// chosen and its Name typed in; a Build under way and done (no game: it
/// fails); and each step's `settle` would fail where frames kept coming.
#[test]
fn the_window_comes_to_rest_with_the_source_group_and_checks() {
    for mode in MODES {
        let what = format!("{mode:?}");
        let mut h = designer_of(mode, 1920.0, with_a_warning());
        layout_chosen(&mut h, "Split source");
        for switch in [nui_names::BINDING_JSON, nui_names::JUI_SOURCE] {
            h.get_by_label(nui_names::SOURCE_HEADER).get_by_label(switch).click();
            settle(&mut h);
        }
        let bytes = |h: &Harness<'_, Moonglow>| {
            h.state().ws.as_ref().unwrap().module.get(&key()).unwrap().to_vec()
        };
        let before = bytes(&h);
        h.get_by_label(nui_names::SOURCE_TEXT).click();
        settle(&mut h);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
        h.get_by_label(nui_names::SOURCE_TEXT).type_text(" ");
        settle(&mut h);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        // Checks: in front, behind, folded (docked: the Bottom Dock).
        if mode == Mode::Tabs {
            h.get_all_by_label("Checks").next().unwrap().click();
            settle(&mut h);
            assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Checks), "{what}");
            h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
            settle(&mut h);
            h.state_mut().settings.hide_log = true;
            settle(&mut h);
            h.state_mut().settings.hide_log = false;
            settle(&mut h);
        }
        for zoom in ["100%", "Fit"] {
            h.get_by_label("Zoom").get_by_label(zoom).click();
            settle(&mut h);
        }
        h.get_by_label("message").click();
        settle(&mut h);
        type_in(&mut h, "Name", "x");
        settle(&mut h);
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        h.state_mut()
            .actions
            .push(mg_ui::Action::GenerateNui { name: "crier".into(), export: false });
        settle(&mut h);
        layout_chosen(&mut h, "Editor");
        settle(&mut h);
        assert_ne!(bytes(&h), before, "{what}: the source's typing set");
    }
}

/// A floating designer's source group (M4's U19) in a screen of 1280 by 720
/// and of 1600 by 900: its window widened by the source's room (moved left
/// where the screen's edge is in the way), on the screen, the design
/// keeping its panes; its Checks in the window's bottom pane; closed, the
/// window as wide as before.
#[test]
fn a_floating_designer_s_source_group_stays_on_the_screen() {
    for (width, height) in [(1280.0, 720.0), (1600.0, 900.0)] {
        let what = format!("{width} by {height}");
        let mut h = designer_of(Mode::Floating, width, with_a_warning());
        h.set_size(egui::vec2(width, height));
        settle(&mut h);
        let leaf = |h: &Harness<'_, Moonglow>, tab: &Tab| {
            let at = h.state().dock.find_tab(tab)?;
            let path = egui_dock::NodePath { surface: at.surface, node: at.node };
            Some(h.state().dock.leaf(path).ok()?.rect)
        };
        let alone = leaf(&h, &Tab::Nui(key())).unwrap();
        layout_chosen(&mut h, "Split source");
        let design = leaf(&h, &Tab::Nui(key())).unwrap();
        let source = leaf(&h, &Tab::NuiSource(key())).expect("the source group");
        assert!(source.left() >= design.right() - 2.0, "{what}: beside the design");
        assert!(source.right() <= width + 0.5 && design.left() >= -0.5, "{what}: on the screen");
        assert!(source.width() >= 300.0, "{what}: the source {} wide", source.width());
        assert!(h.query_by_label("Find a control").is_some(), "{what}: the design's panes");
        h.get_by_label(nui_names::SOURCE_HEADER).get_by_label(nui_names::BINDING_JSON).click();
        settle(&mut h);
        assert!(h.query_by_label(nui_names::SETTINGS_TEXT).is_some(), "{what}: its switch whole");
        layout_chosen(&mut h, "Editor");
        let back = leaf(&h, &Tab::Nui(key())).unwrap();
        assert!((back.width() - alone.width()).abs() < 1.0, "{what}: {back:?} against {alone:?}");
        // (Closed by its own ×: the same.)
        layout_chosen(&mut h, "Split source");
        h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::NuiSource(key())));
        settle(&mut h);
        let back = leaf(&h, &Tab::Nui(key())).unwrap();
        assert!((back.width() - alone.width()).abs() < 1.0, "{what}: by its ×, {back:?}");
    }
}

/// The source group across a restart (M4's U19): the module's open-tabs
/// record keeps it beside the design with its switch (`nui-source:` and
/// its view's page, as U19 writes them), and opened again it is there on
/// Binding JSON; a record of the source group alone (its design not kept)
/// opens its design first, the source beside it.
#[test]
fn the_source_group_comes_back_with_its_switch() {
    for mode in MODES {
        let what = format!("{mode:?}");
        let dir = mg_testkit::scratch_dir(&mode.dir("ui-nui-source-record"));
        let path = dir.join("crier.mod");
        let mut module = Module::new();
        module.set_info(&Gff::new(*b"IFO ")).unwrap();
        module.set(key(), serde_json::to_vec_pretty(&our_window()).unwrap());
        module.set(ResKey::new(key().resref, ResType::TXT), our_settings().bytes());
        module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        let kept = dir.join("open-tabs");
        std::fs::create_dir_all(&kept).unwrap();
        let open = |kept: &std::path::Path| {
            let mut app = app_in(mode, Vec::new());
            app.settings.no_last_area = true;
            app.open_tabs_dir = Some(kept.to_path_buf());
            app.open_module(&path);
            let mut h = harness(app, 1920.0, 900.0);
            settle(&mut h);
            h
        };
        let mut h = open(&kept);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
        settle(&mut h);
        layout_chosen(&mut h, "Split source");
        h.get_by_label(nui_names::SOURCE_HEADER).get_by_label(nui_names::BINDING_JSON).click();
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::Close);
        settle(&mut h);
        let file = mg_ui::open_tabs::file(&kept, &path);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("nui-source:crier.jui"), "{what}: {text}");
        let h = open(&kept);
        assert!(h.state().dock.find_tab(&Tab::NuiSource(key())).is_some(), "{what}: back");
        assert!(h.query_by_label(nui_names::SETTINGS_TEXT).is_some(), "{what}: on Binding JSON");
        drop(h);
        // (The source alone: its design not kept.)
        let mut record: serde_json::Value = serde_json::from_str(&text).unwrap();
        for pane in record["panes"].as_array_mut().unwrap() {
            if let Some(tabs) = pane["tabs"].as_array_mut() {
                tabs.retain(|t| t != "nui:crier.jui");
            }
            if pane["active"] == "nui:crier.jui" {
                pane.as_object_mut().unwrap().remove("active");
            }
            pane.as_object_mut().unwrap().remove("split");
        }
        std::fs::write(&file, serde_json::to_string(&record).unwrap()).unwrap();
        // (Never without its design: the design opens first, the source
        // beside it, on its switch. The coordinator's ruling.)
        let h = open(&kept);
        let design = h.state().dock.find_tab(&Tab::Nui(key())).expect("the design opened");
        let source = h.state().dock.find_tab(&Tab::NuiSource(key())).expect("its source");
        assert_eq!(design.surface, source.surface, "{what}: alone, beside its design");
        assert_ne!(design.node, source.node, "{what}: a group of its own");
    }
}

/// The Bindings panel (its table and the window's settings under it, M4's
/// U19) is never taller where wider: the Right Dock swept from 270 to 480,
/// its height held to the least at any narrower width.
#[test]
fn the_bindings_table_is_never_taller_where_wider() {
    let mut s = our_settings();
    s.bindings.insert(
        "a_bind_of_a_long_name_that_is_cut_short".into(),
        mg_nui::Binding {
            value: json!("A value of some length, longer than its column"),
            ..Default::default()
        },
    );
    s.window_id = Some("a_custom_window_id_of_length".into());
    let mut h = designer_with(Mode::Tabs, 1600.0, our_window(), s);
    let height = |h: &Harness<'_, Moonglow>| {
        let top = h.get_by_label(nui_names::BINDS_TABLE).rect().top();
        let foot = h.get_by_label("Events go to the module's event script").rect().bottom();
        foot - top
    };
    sweep_dock_heights(&mut h, "bindings", height, false);
}

/// The Bindings panel's Custom window ID (M4's U19): typed with none set, it
/// is given as the field is left (one Undo step); one set, typed in and its
/// panel folded away meanwhile, what was typed is set (the form's rule).
#[test]
fn the_custom_window_id_is_set_as_it_is_left() {
    let mut h = designer(Mode::Tabs, 1920.0);
    let read = |h: &Harness<'_, Moonglow>| {
        let module = &h.state().ws.as_ref().unwrap().module;
        Settings::parse(module.get(&ResKey::new(key().resref, ResType::TXT)).unwrap()).unwrap()
    };
    // (None set, typed and its panel folded before the field is left: set
    // as it goes, the coordinator's ruling.)
    type_in(&mut h, "Custom window ID", "folded_id");
    h.state_mut().settings.hide_palettes = true;
    settle(&mut h);
    assert_eq!(read(&h).window_id.as_deref(), Some("folded_id"), "set as its panel went");
    tool(&h, "Undo").click();
    settle(&mut h);
    h.state_mut().settings.hide_palettes = false;
    settle(&mut h);
    assert_eq!(read(&h).window_id, None, "none again");
    type_in(&mut h, "Custom window ID", "crier_id");
    assert_eq!(read(&h).window_id, None, "not while typed in");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(read(&h).window_id.as_deref(), Some("crier_id"), "set as it is left");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(read(&h).window_id, None, "one Undo step");
    tool(&h, "Redo").click();
    settle(&mut h);
    type_in(&mut h, "Custom window ID", "_2");
    h.state_mut().settings.hide_palettes = true;
    settle(&mut h);
    assert_eq!(read(&h).window_id.as_deref(), Some("crier_id_2"), "set as its panel went");
}

/// A second designer gathered into the first's window (M4's U19): its
/// source group opens beside it there, the window widened for it as for the
/// first; a window renamed with its source group open keeps the group, on
/// the new name; the repair's Open JUI Source brings the group forward
/// where it is open behind another tab.
#[test]
fn the_source_group_of_a_second_designer_a_renamed_window_and_the_repair() {
    let other = ResKey::parse("crier_two", ResType::JUI).unwrap();
    let mut app = app_in(Mode::Floating, Vec::new());
    let mut module = Module::new();
    module.set_info(&Gff::new(*b"IFO ")).unwrap();
    for k in [key(), other] {
        module.set(k, serde_json::to_vec_pretty(&our_window()).unwrap());
        module.set(ResKey::new(k.resref, ResType::TXT), our_settings().bytes());
    }
    app.ws = Some(mg_edit::Workspace::new(module));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
    let mut h = harness(app, 1920.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(other)));
    settle(&mut h);
    let (a, b) =
        (h.state().dock.find_tab(&Tab::Nui(key())), h.state().dock.find_tab(&Tab::Nui(other)));
    assert_eq!(a.unwrap().surface, b.unwrap().surface, "one window, two designers");
    h.state_mut().actions.push(mg_ui::Action::NuiSource(other, true));
    settle(&mut h);
    let at = h.state().dock.find_tab(&Tab::NuiSource(other)).expect("its source group");
    let leaf = h.state().dock.leaf(egui_dock::NodePath { surface: at.surface, node: at.node });
    assert!(leaf.unwrap().rect.width() >= 300.0, "the window widened for it");
    // Renamed: the group follows the window's new name.
    let to = ResRef::from_str("crier_new").unwrap();
    h.state_mut().actions.push(mg_ui::Action::RenameResource { from: other, to, literals: false });
    settle(&mut h);
    let renamed = ResKey::new(to, ResType::JUI);
    assert!(h.state().dock.find_tab(&Tab::NuiSource(renamed)).is_some(), "on the new name");
    assert!(h.state().dock.find_tab(&Tab::NuiSource(other)).is_none());
    // The repair: the window not reading, its source behind another tab.
    let edit = mg_edit::Edit::SetResource { key: renamed, data: Some(b"{".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Out", vec![edit])));
    settle(&mut h);
    let script = ResKey::parse("crier_new_e", ResType::NSS).unwrap();
    h.state_mut().ws.as_mut().unwrap().module.set(script, b"void main() {}".to_vec());
    // (Its event script a tab of the source's group, in front of it.)
    let at = h.state().dock.find_tab(&Tab::NuiSource(renamed)).unwrap();
    let path = egui_dock::NodePath { surface: at.surface, node: at.node };
    h.state_mut().dock.set_focused_node_and_surface(path);
    h.state_mut().dock.push_to_focused_leaf(Tab::Script(script));
    settle(&mut h);
    assert!(h.query_by_label(nui_names::SOURCE_TEXT).is_none(), "the source behind");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(renamed)));
    settle(&mut h);
    h.get_all_by_label("Open JUI Source").next().expect("the repair's button").click();
    settle(&mut h);
    assert!(h.query_by_label(nui_names::SOURCE_TEXT).is_some(), "brought forward");
}

// ------------------------------------------- M4's U19 review: fix round.

/// Where `tab`'s pane is now.
fn pane_of(h: &Harness<'_, Moonglow>, tab: &Tab) -> Option<egui::Rect> {
    let at = h.state().dock.find_tab(tab)?;
    let path = egui_dock::NodePath { surface: at.surface, node: at.node };
    Some(h.state().dock.leaf(path).ok()?.rect)
}

/// The source is bound to its design (the coordinator's ruling): Reopen
/// Closed Tab after its × brings it back beside its design, a group of its
/// own (in a floating designer's window, which widens for it), not a tab
/// hiding the design; an `OpenTab` of it with its design closed opens the
/// design first; moved by Split Editor or out to a window of its own, it
/// goes with its design as the design closes.
#[test]
fn the_source_is_bound_to_its_design() {
    let source = Tab::NuiSource(key());
    for mode in MODES {
        let what = format!("{mode:?}");
        let mut h = designer(mode, 1920.0);
        layout_chosen(&mut h, "Split source");
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![source.clone()]));
        settle(&mut h);
        assert!(h.state().dock.find_tab(&source).is_none(), "{what}: closed by its ×");
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::T);
        settle(&mut h);
        let design = h.state().dock.find_tab(&Tab::Nui(key())).unwrap();
        let at = h.state().dock.find_tab(&source).unwrap_or_else(|| panic!("{what}: reopened"));
        assert_eq!((at.surface, at.node == design.node), (design.surface, false), "{what}: beside");
        assert!(pane_of(&h, &source).unwrap().width() >= 300.0, "{what}: its room");
        // Its design closed, an OpenTab of it: the design first.
        h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Nui(key())));
        settle(&mut h);
        assert!(h.state().dock.find_tab(&source).is_none(), "{what}: gone with its design");
        h.state_mut().actions.push(mg_ui::Action::OpenTab(source.clone()));
        settle(&mut h);
        let design = h.state().dock.find_tab(&Tab::Nui(key())).expect("the design opened first");
        let at = h.state().dock.find_tab(&source).unwrap();
        assert!(at.surface == design.surface && at.node != design.node, "{what}: beside it");
        // Out to a window of its own: it still goes with its design.
        let at = h.state().dock.find_tab(&source).unwrap();
        h.state_mut().dock.remove_tab(at);
        h.state_mut().dock.add_window(vec![source.clone()]);
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Nui(key())));
        settle(&mut h);
        assert!(h.state().dock.find_tab(&source).is_none(), "{what}: its window goes too");
    }
    // Split Editor on it (As Tabs): moved, it goes with the design.
    let mut h = designer(Mode::Tabs, 1920.0);
    layout_chosen(&mut h, "Split source");
    // (A second tab in the source's group, the source in front: Split
    // Editor moves it to a group of its own.)
    let at = h.state().dock.find_tab(&source).unwrap();
    let path = egui_dock::NodePath { surface: at.surface, node: at.node };
    h.state_mut().dock.set_focused_node_and_surface(path);
    h.state_mut().dock.push_to_focused_leaf(Tab::ModuleProperties);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(source.clone()));
    settle(&mut h);
    let before = h.state().dock.find_tab(&source).unwrap().node;
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    assert_ne!(h.state().dock.find_tab(&source).unwrap().node, before, "split off");
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Nui(key())));
    settle(&mut h);
    assert!(h.state().dock.find_tab(&source).is_none(), "split off, gone with its design");
}

/// New errors open Checks while the source is the tab drawn and its design
/// is hidden behind another tab of its group.
#[test]
fn new_errors_open_checks_with_the_design_hidden() {
    let mut h = designer(Mode::Tabs, 1920.0);
    layout_chosen(&mut h, "Split source");
    let at = h.state().dock.find_tab(&Tab::Nui(key())).unwrap();
    let path = egui_dock::NodePath { surface: at.surface, node: at.node };
    h.state_mut().dock.set_focused_node_and_surface(path);
    h.state_mut().dock.push_to_focused_leaf(Tab::ModuleProperties);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::NuiSource(key())));
    settle(&mut h);
    let at = h.state().dock.find_tab(&Tab::Nui(key())).unwrap();
    let leaf =
        h.state().dock.leaf(egui_dock::NodePath { surface: at.surface, node: at.node }).unwrap();
    assert_ne!(leaf.tabs[leaf.active.0], Tab::Nui(key()), "design tab covered by another tab");
    h.state_mut().settings.hide_log = true;
    let edit = mg_edit::Edit::SetResource { key: key(), data: Some(b"{".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Out", vec![edit])));
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Checks), "the source's new error");
}

/// A floating designer's place comes back as its source group closes (the
/// coordinator's ruling): Split source on and off three times, the window
/// where it was and as wide; moved meanwhile, it keeps the builder's place,
/// narrowed back by the room it was given.
#[test]
fn a_floating_designer_s_place_comes_back() {
    for width in [1280.0, 1600.0] {
        let mut h = designer(Mode::Floating, width);
        h.set_size(egui::vec2(width, 900.0));
        settle(&mut h);
        let alone = pane_of(&h, &Tab::Nui(key())).unwrap();
        for round in 0..3 {
            layout_chosen(&mut h, "Split source");
            layout_chosen(&mut h, "Editor");
            let back = pane_of(&h, &Tab::Nui(key())).unwrap();
            let near = (back.min - alone.min).length() < 1.5
                && (back.size() - alone.size()).length() < 1.5;
            assert!(near, "{width}, round {round}: {back:?} against {alone:?}");
        }
        // Moved by its bar while the source is open: its new place kept.
        layout_chosen(&mut h, "Split source");
        // (The bar beside its tab: the window moves by it.)
        let tab = h.state().tab_rect(&Tab::Nui(key())).unwrap();
        let bar = tab.right_center() + egui::vec2(60.0, 0.0);
        h.hover_at(bar);
        h.run_steps(2);
        h.drag_at(bar);
        h.run_steps(2);
        h.hover_at(bar + egui::vec2(0.0, 40.0));
        h.run_steps(2);
        h.drop_at(bar + egui::vec2(0.0, 40.0));
        settle(&mut h);
        let moved = pane_of(&h, &Tab::Nui(key())).unwrap();
        assert!(moved.top() > alone.top() + 20.0, "{width}: moved by its bar");
        layout_chosen(&mut h, "Editor");
        let back = pane_of(&h, &Tab::Nui(key())).unwrap();
        assert!((back.top() - moved.top()).abs() < 1.5, "{width}: the builder's place kept");
        assert!((back.width() - alone.width()).abs() < 1.5, "{width}: narrowed back");
    }
}

/// The Bindings table walks the window for its binds' uses once for the
/// bytes it reads, a bind chosen or not: no frame that changes nothing
/// walks it (the tip's walked it twice a frame).
#[test]
fn the_bindings_table_walks_the_window_once_for_its_bytes() {
    let mut h = designer(Mode::Tabs, 1920.0);
    h.get_by_label("message").click();
    settle(&mut h);
    let walks = h.state().nui_binds_walks(&key());
    assert!(walks >= 1, "walked once");
    for _ in 0..5 {
        h.hover_at(egui::pos2(800.0, 500.0));
        h.run();
        h.hover_at(egui::pos2(810.0, 510.0));
        h.run();
    }
    assert_eq!(h.state().nui_binds_walks(&key()), walks, "none in frames that change nothing");
    let mut w = our_window();
    w["title"] = json!("Changed");
    set_window(&mut h, &w);
    assert_eq!(h.state().nui_binds_walks(&key()), walks + 1, "once for the new bytes");
}

/// What is typed in the source is saved (U18's rule for a field): Save sets
/// it first, the field keeping the keyboard; the design closed with the
/// bytes changed from elsewhere lets it go (the outside bytes kept); the
/// module closed with it typed writes nothing and does not fault.
#[test]
fn the_source_s_typing_is_saved_and_never_written_over_other_bytes() {
    let dir = mg_testkit::scratch_dir("ui-nui-save-source");
    let path = dir.join("crier.mod");
    let open = || {
        let mut module = Module::new();
        module.set_info(&Gff::new(*b"IFO ")).unwrap();
        module.set(key(), serde_json::to_vec_pretty(&our_window()).unwrap());
        module.set(ResKey::new(key().resref, ResType::TXT), our_settings().bytes());
        module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        let mut app = app_in(Mode::Tabs, Vec::new());
        app.settings.no_last_area = true;
        app.open_module(&path);
        let mut h = harness(app, 1920.0, 900.0);
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(key())));
        settle(&mut h);
        layout_chosen(&mut h, "Split source");
        h.get_by_label(nui_names::SOURCE_TEXT).click();
        settle(&mut h);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
        h.get_by_label(nui_names::SOURCE_TEXT).type_text(" ");
        h.run();
        h
    };
    let saved = |path: &std::path::Path| Module::open(path).unwrap().get(&key()).unwrap().to_vec();
    let mut h = open();
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    let typed = saved(&path);
    assert!(
        typed.ends_with(b"} "),
        "saved as typed: {:?}",
        String::from_utf8_lossy(&typed[typed.len().saturating_sub(8)..])
    );
    let before = serde_json::to_vec_pretty(&our_window()).unwrap();
    assert_ne!(typed, before, "Save set what was typed");
    assert!(h.get_by_label(nui_names::SOURCE_TEXT).accesskit_node().is_focused(), "keyboard kept");
    // Changed from elsewhere as the design closes: let go.
    let mut h = open();
    let mut w = our_window();
    w["title"] = json!("Elsewhere");
    let data = Some(serde_json::to_vec_pretty(&w).unwrap());
    let edit = mg_edit::Edit::SetResource { key: key(), data };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Out", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Nui(key())]));
    settle(&mut h);
    let now = h.state().ws.as_ref().unwrap().module.get(&key()).unwrap().to_vec();
    assert_eq!(now, serde_json::to_vec_pretty(&w).unwrap(), "the outside bytes kept");
    // The module closed with it typed: nothing written.
    let mut h = open();
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    assert_eq!(saved(&path), before, "nothing written");
}

/// A second NUI window opened As Tabs, the first's source group the
/// focused group, opens among the designs, never in a source group.
#[test]
fn a_second_docked_designer_opens_among_the_designs() {
    let other = ResKey::parse("crier_two", ResType::JUI).unwrap();
    let mut h = designer(Mode::Tabs, 1920.0);
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.module.set(other, serde_json::to_vec_pretty(&our_window()).unwrap());
    layout_chosen(&mut h, "Split source");
    h.get_by_label(nui_names::SOURCE_TEXT).click();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Nui(other)));
    settle(&mut h);
    let (a, b) =
        (h.state().dock.find_tab(&Tab::Nui(key())), h.state().dock.find_tab(&Tab::Nui(other)));
    assert_eq!(a.unwrap().node, b.unwrap().node, "in the first design's group");
}

/// Checks' fixes are whole ("Add Close Handler"), in the Bottom Dock of a
/// narrow window and in a floating designer's bottom pane.
#[test]
fn the_checks_fixes_are_whole() {
    let mut w = our_window();
    w["root"]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"button","id":"mg_close","label":"Close","height":30.0}));
    for (mode, width) in [(Mode::Tabs, 1000.0), (Mode::Floating, 1280.0)] {
        let what = format!("{mode:?} {width}");
        let mut h = designer_of(mode, width, w.clone());
        if mode == Mode::Tabs {
            h.get_all_by_label("Checks").next().unwrap().click();
            settle(&mut h);
        }
        let fix = h.get_by_label("Add Close Handler").rect();
        let room = match mode {
            Mode::Tabs => h.state().bottom_dock_rect().unwrap(),
            Mode::Floating => pane_of(&h, &Tab::Nui(key())).unwrap(),
        };
        assert!(room.contains_rect(fix), "{what}: {fix:?} in {room:?}");
        let words = h.get_by_label("Add Close Handler");
        assert!(words.accesskit_node().bounding_box().is_some(), "{what}: drawn");
        assert!(fix.width() > 90.0, "{what}: whole, {} wide", fix.width());
        // (The message cut short before it, never under it.)
        let message = h
            .get_all_by_label_contains("The mg_close button has no Clicked handler")
            .find(|n| n.rect().top() < fix.bottom() && n.rect().bottom() > fix.top())
            .expect("its message")
            .rect();
        assert!(message.right() <= fix.left() + 0.5, "{what}: {message:?} before {fix:?}");
    }
}

/// The canvas's corners take no click beside them, nor let one through: at
/// 100% over a window whose control fills the canvas, a click on the zoom
/// is the zoom's (the control not chosen), one beside it the control's; in
/// Clean view they stay.
#[test]
fn the_canvas_s_corners_take_no_click_beside_them() {
    let mut w = mg_nui::window();
    w["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 1500.0, "h": 1200.0});
    w["root"]["children"] =
        json!([{"type":"button","id":"big","label":"Big","width":1480.0,"height":1150.0}]);
    let mut h = designer_with(Mode::Tabs, 1920.0, w, our_settings());
    h.get_by_label("Zoom").get_by_label("100%").click();
    settle(&mut h);
    assert_eq!(chosen(&h), "", "the zoom's click chose nothing under it");
    let beside = h.get_by_label("Zoom").rect().left_center() - egui::vec2(60.0, 0.0);
    h.hover_at(beside);
    h.run();
    h.drag_at(beside);
    h.run();
    h.drop_at(beside);
    settle(&mut h);
    assert_eq!(chosen(&h), "/root/children/0", "beside it, the control under the pointer");
    nui_tool(&mut h, nui_names::VIEW).click();
    settle(&mut h);
    h.get_all_by_label("Clean view").last().unwrap().click();
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label("Zoom").is_some(), "in Clean view too");
    assert!(h.query_by_label_contains("UI scale 100%").is_some(), "the label too");
}

/// The source group in front: a plugin sees the window's JUI as open, the
/// Outliner still lists its Layers.
#[test]
fn with_the_source_in_front_plugins_and_the_outliner_see_its_design() {
    let mut h = designer(Mode::Tabs, 1920.0);
    layout_chosen(&mut h, "Split source");
    assert!(matches!(h.state().active_editor(), Some(Tab::NuiSource(_))), "the source in front");
    let open = h.state().plugin_open();
    assert_eq!(open, Some(mg_plugin::Selected::Resource(key())), "the JUI as with the design");
    assert!(h.query_by_label("Find a layer or ID").is_some(), "its Layers");
}

/// The Bindings table of many binds: scrolled to its last, chosen and
/// renamed while a property's field was typed in, both set.
#[test]
fn a_table_of_many_binds_and_a_rename_while_a_field_is_typed_in() {
    let mut s = our_settings();
    for i in 0..40 {
        s.bindings.insert(
            format!("bind_{i:02}"),
            mg_nui::Binding { value: json!(i), ..Default::default() },
        );
    }
    let mut h = designer_with(Mode::Tabs, 1920.0, our_window(), s);
    choose(&mut h, "Label · Hear ye");
    type_in(&mut h, "Text", "!");
    let row = h.get_by_label("bind_39");
    row.scroll_to_me();
    settle(&mut h);
    h.get_by_label("bind_39").click();
    settle(&mut h);
    assert_eq!(document(&h)["root"]["children"][0]["value"], "Hear ye!", "the field set first");
    type_in(&mut h, "Name", "_x");
    h.get_by_label("Rename").click();
    settle(&mut h);
    let module = &h.state().ws.as_ref().unwrap().module;
    let s = Settings::parse(module.get(&ResKey::new(key().resref, ResType::TXT)).unwrap()).unwrap();
    assert!(s.bindings.contains_key("bind_39_x"), "renamed");
}

/// The NUI board's Properties (U30's #6): Send chosen, its Behavior opened
/// and its Enabled's chip clicked, Properties stand at their top, Property
/// Binding and Content in sight under the panel's header.
#[test]
fn properties_open_at_their_top_on_a_chip_s_click() {
    let mut w = our_window();
    w["root"]["children"][2]["children"][0]["enabled"] = json!({"bind": "can_send"});
    let mut s = our_settings();
    s.bindings
        .insert("can_send".into(), mg_nui::Binding { value: json!(false), ..Default::default() });
    let mut h = designer_with(Mode::Tabs, 1600.0, w, s);
    choose(&mut h, "Button · Send");
    h.get_by_label("Behavior").click();
    settle(&mut h);
    h.get_by_label("Bound to can_send").click();
    settle(&mut h);
    // (The panel's header, its Element Actions at its end; its first
    // section right under it, the next in sight.)
    let header = h.get_by_label(nui_names::ELEMENT_ACTIONS).rect();
    let binding = h.get_by_label("Property Binding").rect();
    assert!(binding.top() > header.bottom(), "{binding:?} under the header {header:?}");
    assert!(binding.top() < header.bottom() + 24.0, "{binding:?} scrolled from {header:?}");
    let content = h.get_by_label("Content").rect();
    assert!(right_dock_rect(&h).contains_rect(content), "Content in sight: {content:?}");
}

/// The NUI designer's Checks (5.2's W4): a row's menu (a right click, on
/// its words too) has Copy (its kind, control and whole message) and Copy
/// All (every row, in its order); the message cut short is whole in its
/// tip.
#[test]
fn a_check_s_row_copies_from_its_menu_and_its_tip_says_it_whole() {
    use mg_ui::row_copy::{COPY, COPY_ALL};
    let mut h = designer_of(Mode::Tabs, 1100.0, with_a_warning());
    h.get_all_by_label("Checks").next().unwrap().click();
    settle(&mut h);
    let lost = "What the player enters here is lost";
    let rows = h.get_by_label(nui_names::CHECKS_ROWS);
    let words = rows.get_by_label_contains(lost);
    let node = words.accesskit_node();
    let message = node.label().filter(|l| !l.is_empty()).or(node.value()).unwrap_or_default();
    let at = words.rect().left_center() + egui::vec2(40.0, 0.0);
    let right = |h: &mut Harness<'_, Moonglow>| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
    };
    let copied = |h: &mut Harness<'_, Moonglow>, item: &str| -> Vec<String> {
        h.get_all_by_label(item).last().unwrap_or_else(|| panic!("{item} in the menu")).click();
        h.step();
        let out = (h.output().platform_output.commands.iter())
            .filter_map(|c| match c {
                egui::OutputCommand::CopyText(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        h.run();
        out
    };
    let line = format!("Warning: Text input · Enter text, {message}");
    right(&mut h);
    assert_eq!(copied(&mut h, COPY), [line.as_str()], "Copy: the whole message");
    right(&mut h);
    assert_eq!(copied(&mut h, COPY_ALL), [line.as_str()], "Copy All: the one row");
    let tip = tip_at(&mut h, egui::Rect::from_center_size(at, egui::vec2(2.0, 2.0)));
    assert!(tip.is_some_and(|t| t.lines().any(|l| l == message)), "the message in its tip");
    // The keyboard on the row's place (its Tab stop, a label here, its
    // control found by no link): Ctrl+C copies the row, Shift+F10 opens
    // its menu, Copy taking the keys, Escape giving them back.
    let place = "Text input · Enter text";
    let link = |h: &Harness<'_, Moonglow>| {
        let rows = h.get_by_label(nui_names::CHECKS_ROWS);
        rows.get_by_label(place).accesskit_node().is_focused()
    };
    h.get_by_label(nui_names::CHECKS_ROWS).get_by_label(place).focus();
    h.run();
    assert!(link(&h), "its place has the keyboard");
    h.event(egui::Event::Copy);
    h.step();
    let out: Vec<String> = (h.output().platform_output.commands.iter())
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    h.run();
    assert_eq!(out, [line.as_str()], "Ctrl+C: the row the keyboard is on");
    press_now(&mut h, egui::Modifiers::SHIFT, egui::Key::F10);
    h.run();
    let copy = h.get_all_by_label(COPY).last().expect("Shift+F10 opens its menu");
    assert!(copy.accesskit_node().is_focused(), "Copy has the keyboard");
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.query_by_label(COPY_ALL).is_none(), "Escape closes it");
    assert!(link(&h), "Escape gives the keyboard back to its place");
}
