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
    let more: &'static str = Box::leak(header::more_name(NUI_HEADER).into_boxed_str());
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
    found.unwrap_or_else(|| panic!("no row {words} in Layers"))
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
    for variant in [false, true] {
        let what = if variant { "a variant edited" } else { "the window" };
        let (w, s) = with_a_variant();
        let mut h = designer_with(Mode::Tabs, 1920.0, w, s);
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
            let names: Vec<&str> = rec.places.iter().map(|(n, _)| n.as_str()).collect();
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
            .get_all_by_label_contains("A bind is a named value")
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
        let create = h.get_by_label("Create Bind").rect();
        assert!(create.top() >= bindings.top() && pane.contains_rect(create), "{what}: {create:?}");
        // Each used: a component added to the control chosen's row, a bind
        // made.
        let column = h
            .get_all_by_label("Column")
            .find(|n| rect(n).is_some_and(|r| components.contains_rect(r)));
        column.unwrap().click();
        settle(&mut h);
        let row = &document(&h)["root"]["children"][2]["children"];
        assert_eq!(row.as_array().unwrap().len(), 3, "{what}: added beside Send");
        h.get_by_label("Create Bind").click();
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
        for layout in ["Preview only", "Split source", "Editor"] {
            layout_chosen(&mut h, layout);
            check(&h, layout);
            assert_eq!(editor_rect(&h).width(), width, "{mode:?} {layout}: the editor's width");
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
    let in_sight = |n: &egui_kittest::Node<'_>| {
        let r = n.rect();
        r.left() >= dock.left() - 0.5
            && r.top() >= dock.top() - 0.5
            && r.bottom() <= dock.bottom()
            && r.width() > 0.0
    };
    let past: Vec<String> = h
        .get_all_by(|n| n.bounding_box().is_some())
        .filter(in_sight)
        .filter(|n| n.rect().right() > dock.right() + 0.5)
        // (A scroll's bar with nothing to scroll is said to stand past its
        // edge, as wide as egui's room for it: not drawn.)
        .filter(|n| n.accesskit_node().role() != egui::accesskit::Role::ScrollBar)
        .map(|n| {
            let a = n.accesskit_node();
            format!("{:?} {:?} {:?} {:?}", a.role(), a.label(), a.value(), n.rect())
        })
        .collect();
    assert!(past.is_empty(), "{what}: past the edge {past:?}");
    let margins = {
        let m = mg_uikit::controls::FIELD_MARGIN;
        f32::from(m.left) + f32::from(m.right)
    };
    let cut: Vec<String> = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .filter(in_sight)
        .filter_map(|n| {
            let value = n.accesskit_node().value()?;
            let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
            let words = h.ctx.fonts_mut(|f| {
                f.layout_no_wrap(value.clone(), font, egui::Color32::WHITE).size().x
            });
            (n.rect().width() + 0.5 < words + margins)
                .then(|| format!("{value:?}: {} for {}", n.rect().width(), words + margins))
        })
        .collect();
    assert!(cut.is_empty(), "{what}: cut short in their fields {cut:?}");
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
    let mut h = designer_of(Mode::Tabs, 1600.0, w);
    for canvas in ["Canvas Label · A long label", "Canvas Dropdown"] {
        h.state_mut().settings.right_dock_width = None;
        h.set_size(egui::vec2(1600.0, 900.0));
        settle(&mut h);
        let row = h.query_all_by_label_contains(canvas).next().is_some();
        assert!(row, "{canvas}");
        h.get_all_by_label_contains(canvas).next().unwrap().click();
        settle(&mut h);
        let mut was: Option<f32> = None;
        let mut least: Option<(u32, f32)> = None;
        let mut changes = Vec::new();
        for w in 270..=480u32 {
            h.state_mut().settings.right_dock_width = Some(w);
            settle(&mut h);
            let high = properties_height(&h);
            if let Some((at, low)) = least {
                assert!(high <= low + 0.5, "{canvas}: {low} at {at}, {high} at {w}");
            }
            if was.is_some_and(|b| (high - b).abs() > 0.5) {
                changes.push(w);
            }
            was = Some(high);
            if least.is_none_or(|(_, low)| high < low) {
                least = Some((w, high));
            }
        }
        for w in changes {
            for at in w.saturating_sub(2)..=w + 2 {
                h.state_mut().settings.right_dock_width = Some(at);
                for high in (700..=900).step_by(20) {
                    h.set_size(egui::vec2(1600.0, high as f32));
                    settle(&mut h);
                }
            }
            h.set_size(egui::vec2(1600.0, 900.0));
        }
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
    let plus = h.get_by_label("New Bind for Property").rect();
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
