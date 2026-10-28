//! The NUI Creator's tables and scripts against the game's own: the widgets
//! and the fixed and bound arguments `mg-nui` knows are those of the
//! install's `nw_inc_nui.nss`, and what Build writes compiles against its
//! `nwscript.nss`, by the module's compile path.

use mg_core::ResType;
use mg_edit::Workspace;
use mg_module::Module;
use mg_module::build::Remembered;
use mg_module::external_compiler::ExternalCompiler;
use mg_nui::{Action, Binding, Build, Generated, Handler, NuiError, Settings, Severity};
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_testkit::corpus;
use serde_json::{Value, json};

fn key(name: &str, ty: ResType) -> ResKey {
    ResKey::parse(name, ty).unwrap()
}

fn game(root: &std::path::Path) -> ResMan {
    ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap()
}

/// A module with one new window, `test_nui`.
fn project() -> Workspace {
    let mut ws = Workspace::new(Module::new());
    ws.apply(mg_nui::create(&ws.module, "test_nui").unwrap()).unwrap();
    ws
}

/// A module with the window `test_nui` as given.
fn project_of(window: &Value, settings: &Settings) -> Workspace {
    let mut ws = project();
    ws.module.set(key("test_nui", ResType::JUI), serde_json::to_vec(window).unwrap());
    ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
    ws
}

fn generate(ws: &Workspace, rm: &ResMan) -> Result<Generated, NuiError> {
    mg_nui::generate(&ws.module, "test_nui", &Build::new(rm))
}

/// Build, applied: what it could not compile of the module's own scripts.
fn build(ws: &mut Workspace, rm: &ResMan) -> Generated {
    let made = generate(ws, rm).unwrap();
    ws.apply(made.command.clone()).unwrap();
    made
}

fn text(ws: &Workspace, name: &str) -> String {
    String::from_utf8(ws.module.get(&key(name, ResType::NSS)).unwrap().to_vec()).unwrap()
}

fn compiled(ws: &Workspace, name: &str) -> bool {
    ws.module.get(&key(name, ResType::NCS)).is_some_and(|b| b.starts_with(b"NCS "))
}

fn click(element: &str, action: Action) -> Handler {
    Handler::new("click", element, action)
}

#[test]
fn the_games_include_builds_every_widget_the_creator_offers() {
    let rm = game(&corpus!());
    let bytes = rm.get_named("nw_inc_nui", ResType::NSS).unwrap();
    let source = String::from_utf8_lossy(&bytes);
    for ty in mg_nui::ELEMENTS {
        assert!(source.contains(&format!("NuiElement(\"{ty}\"")), "no constructor for {ty}");
    }
    for field in ["accepts_input", "row_template"] {
        assert!(source.contains(&format!("\"{field}\"")), "{field}");
    }
}

#[test]
fn the_games_include_has_the_fixed_and_bound_arguments_the_checks_hold() {
    let rm = game(&corpus!());
    let bytes = rm.get_named("nw_inc_nui", ResType::NSS).unwrap();
    let source = String::from_utf8_lossy(&bytes);
    for contract in [
        "NuiGroup(json jChild, int bBorder",
        "NuiText(json jValue, int bBorder",
        "NuiOptions(int nDirection, json jElements",
        "NuiTextEdit(json jPlaceholder, json jValue, int nMaxLength, int bMultiline, int bWordWrap",
        "float fRowHeight = NUI_STYLE_ROW_HEIGHT",
        "\"number_precision\", JsonInt(nNumberPrecision)",
        "\"arrayBinds\", JsonBool(nBindArrays)",
        "\"c\", jCenter",
        "\"data\", jData",
    ] {
        assert!(source.contains(contract), "the include changed, check again: {contract}");
    }
}

#[test]
fn the_games_functions_are_documented_as_the_generated_scripts_use_them() {
    let rm = game(&corpus!());
    let bytes = rm.get_named("nwscript", ResType::NSS).unwrap();
    let source = String::from_utf8_lossy(&bytes);
    // A variant shown in place of the whole window, and a list's row.
    assert!(source.contains("special \"_window_\" root group"));
    assert!(source.contains("Returns -1 if the event is not originating from within an array."));
}

#[test]
fn a_new_window_builds_and_changes_nothing_until_applied() {
    let rm = game(&corpus!());
    let mut ws = project();
    let made = generate(&ws, &rm).unwrap();
    assert_eq!(ws.module.keys().count(), 2, "nothing is changed by preparing");
    assert!(made.failed.is_empty());
    ws.apply(made.command).unwrap();
    assert!(compiled(&ws, "test_nui_e"));
    assert!(!ws.module.contains(&key("test_nui_o", ResType::NCS)), "the opener is an include");
    assert!(!ws.module.keys().any(|k| k.resref.to_string().starts_with("__nui")));
    let built = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    let window = mg_nui::parse(ws.module.get(&key("test_nui", ResType::JUI)).unwrap()).unwrap();
    assert!(mg_nui::is_current(&ws.module, "test_nui", &window, &built));
}

#[test]
fn a_second_build_changes_nothing() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    assert!(generate(&ws, &rm).unwrap().command.edits.is_empty());
}

#[test]
fn one_undo_takes_back_a_whole_build() {
    let rm = game(&corpus!());
    let mut ws = project();
    let before = ws.module.clone();
    build(&mut ws, &rm);
    let built = ws.module.clone();
    ws.undo().unwrap();
    assert_eq!(ws.module.keys().count(), 2);
    for k in before.keys() {
        assert_eq!(ws.module.get(k), before.get(k));
    }
    ws.redo().unwrap();
    for k in built.keys() {
        assert_eq!(ws.module.get(k), built.get(k));
    }
}

#[test]
fn the_built_event_script_is_the_one_the_creator_shows() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    let settings = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    let script = text(&ws, "test_nui_e");
    assert_eq!(script, mg_nui::event_source("test_nui", &settings).unwrap());
    assert_eq!(script.matches("NuiDestroy").count(), 1, "the Close button's handler");
}

#[test]
fn a_window_without_handlers_builds_a_script_that_does_nothing() {
    let rm = game(&corpus!());
    let mut ws = project_of(&mg_nui::window(), &Settings::default());
    build(&mut ws, &rm);
    assert!(!text(&ws, "test_nui_e").contains("NuiDestroy"));
    assert!(compiled(&ws, "test_nui_e"));
}

#[test]
fn a_removed_handlers_code_stays_in_the_built_script_commented_out() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    let mut settings =
        Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    settings.handlers.clear();
    ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
    build(&mut ws, &rm);
    let script = text(&ws, "test_nui_e");
    assert!(!script.lines().any(|l| l.trim_start().starts_with("NuiDestroy")));
    assert!(script.contains("// mg:removed click mg_close"));
}

#[test]
fn the_opener_compiles_in_a_script_that_calls_it_in_both_deliveries() {
    let rm = game(&corpus!());
    for from_resref in [false, true] {
        let settings = Settings { from_resref, ..Default::default() };
        let mut ws = project_of(&mg_nui::window(), &settings);
        ws.module.set(
            key("my_enter", ResType::NSS),
            b"#include \"test_nui_o\"\nvoid main() { Open_test_nui(GetEnteringObject()); }\n"
                .to_vec(),
        );
        let made = build(&mut ws, &rm);
        assert!(made.failed.is_empty(), "{:?}", made.failed);
        assert!(compiled(&ws, "my_enter"));
        assert!(!text(&ws, "test_nui_o").contains("void main("));
    }
}

#[test]
fn an_openers_compiled_script_left_by_an_earlier_build_goes_and_undo_brings_it_back() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    ws.module.set(key("test_nui_o", ResType::NCS), b"old opener bytecode".to_vec());
    ws.module.set(key("test_nui_e", ResType::NDB), b"old debug file".to_vec());
    let saved = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    let window = mg_nui::parse(ws.module.get(&key("test_nui", ResType::JUI)).unwrap()).unwrap();
    assert!(!mg_nui::is_current(&ws.module, "test_nui", &window, &saved));
    build(&mut ws, &rm);
    assert!(!ws.module.contains(&key("test_nui_o", ResType::NCS)));
    assert!(!ws.module.contains(&key("test_nui_e", ResType::NDB)), "no debug file was asked for");
    ws.undo().unwrap();
    assert_eq!(ws.module.get(&key("test_nui_o", ResType::NCS)).unwrap(), b"old opener bytecode");
    assert_eq!(ws.module.get(&key("test_nui_e", ResType::NDB)).unwrap(), b"old debug file");
}

#[test]
fn handlers_that_show_variants_in_nested_groups_compile_in_both_deliveries() {
    let rm = game(&corpus!());
    for from_resref in [false, true] {
        let mut doc = mg_nui::window();
        doc["root"]["children"] = json!([
            {"type":"label","id":"header","value":"Fixed header"},
            {"type":"row","children":[{"type":"button","id":"next","label":"Next"},
                {"type":"group","id":"panel","width":260.0,"height":160.0,"children":[
                    {"type":"label","value":"Original"}]}]},
            {"type":"label","value":"Fixed footer"}]);
        let mut settings = Settings { from_resref, ..Default::default() };
        settings.variants.insert("details".into(), json!({"type":"col","children":[
            {"type":"button","id":"inner_next","label":"Inner next"},
            {"type":"group","id":"nested","children":[{"type":"label","value":"Inner original"}]}]}));
        settings
            .variants
            .insert("inner_details".into(), json!({"type":"label","value":"Inner replaced"}));
        for (element, group, view) in
            [("next", "panel", "details"), ("inner_next", "nested", "inner_details")]
        {
            let show = Action::Variant { group: group.into(), variant: view.into() };
            settings.handlers.push(click(element, show));
        }
        let mut ws = project_of(&doc, &settings);
        build(&mut ws, &rm);
        let source = text(&ws, "test_nui_e");
        assert_eq!(source.matches("NuiSetGroupLayout").count(), 2);
        assert!(!source.contains("_window_"));
        assert!(source.contains("\"panel\"") && source.contains("\"nested\""));
        assert!(compiled(&ws, "test_nui_e"));
        // The layout is the builder's, untouched.
        assert_eq!(
            mg_nui::parse(ws.module.get(&key("test_nui", ResType::JUI)).unwrap()).unwrap(),
            doc
        );
    }
}

#[test]
fn handlers_of_focus_and_of_a_lists_range_compile() {
    let rm = game(&corpus!());
    let mut doc = mg_nui::window();
    doc["root"]["children"] = json!([
        {"type":"textedit","id":"edit","value":{"bind":"text"},"length":128,"multiline":false,"wordwrap":false},
        {"type":"list","id":"rows","row_count":0,"row_height":30.0,"row_template":[]}
    ]);
    let mut s = Settings::default();
    s.bindings.insert("text".into(), Binding { value: json!(""), ..Default::default() });
    for (event, element) in [("focus", "edit"), ("blur", "edit"), ("range", "rows")] {
        s.handlers.push(Handler::new(event, element, Action::Close));
    }
    let mut ws = project_of(&doc, &s);
    build(&mut ws, &rm);
    let source = text(&ws, "test_nui_e");
    for event in ["focus", "blur", "range"] {
        assert!(source.contains(&format!("sType == \"{event}\"")), "{event}");
    }
    assert!(compiled(&ws, "test_nui_e"));
}

#[test]
fn every_kind_of_handler_and_names_no_literal_holds_compile() {
    let rm = game(&corpus!());
    let mut w = mg_nui::window();
    w["root"] = json!({"type":"group","id":"pages","children":[{"type":"col","children":[
        {"type":"button","id":"toggle","label":"Toggle"},
        {"type":"button","id":"set","label":"Set"},
        {"type":"button","id":"page","label":"Next"},
        {"type":"button","id":"array","label":"Row"},
        {"type":"button","id":"Żółw \"x\"","label":"Odd"},
        {"type":"button","id":"close","label":"Close"}
    ]}]});
    let mut s = Settings { window_id: Some("Żółw 🐉".into()), ..Default::default() };
    for (name, value) in [
        ("flag", json!(false)),
        ("rows", json!([false, true])),
        ("caption", json!("first")),
        ("zażółć", json!("Zażółć 🐉")),
    ] {
        s.bindings.insert(name.into(), Binding { value, watch: true, ..Default::default() });
    }
    s.variants.insert(
        "details 🐉".into(),
        json!({"type":"col","children":[{"type":"label","value":{"bind":"caption"}}]}),
    );
    for (element, action) in [
        ("toggle", Action::Toggle { bind: "flag".into() }),
        ("set", Action::Set { bind: "caption".into(), value: json!("Zażółć 🐉") }),
        ("array", Action::Toggle { bind: "rows".into() }),
        ("page", Action::Variant { group: "pages".into(), variant: "details 🐉".into() }),
        ("Żółw \"x\"", Action::Set { bind: "zażółć".into(), value: json!({"a": [1, 2.5]}) }),
        ("close", Action::Close),
    ] {
        s.handlers.push(click(element, action));
    }
    s.handlers.push(Handler::new(
        "watch",
        "flag",
        Action::Set { bind: "caption".into(), value: json!("watched") },
    ));
    let errors: Vec<_> =
        mg_nui::validate(&w, &s).into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let mut ws = project_of(&w, &s);
    build(&mut ws, &rm);
    assert!(compiled(&ws, "test_nui_e"));
    assert!(ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap().is_ascii());
    assert!(ws.module.get(&key("test_nui_o", ResType::NSS)).unwrap().is_ascii());
    // Code in the section of the control whose name no literal holds is
    // kept by the next Build.
    let odd = mg_nui::handler_key(&s.handlers[4]);
    let written =
        mg_nui::insert_code(&text(&ws, "test_nui_e"), &odd, "        int nKept = 1;\n").unwrap();
    ws.module.set(key("test_nui_e", ResType::NSS), written.clone().into_bytes());
    build(&mut ws, &rm);
    assert_eq!(text(&ws, "test_nui_e"), written);
}

#[test]
fn a_new_handler_makes_the_built_scripts_out_of_date() {
    let rm = game(&corpus!());
    let w = mg_nui::window();
    let mut ws = project();
    build(&mut ws, &rm);
    let built = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    assert!(mg_nui::is_current(&ws.module, "test_nui", &w, &built));
    let mut more = built.clone();
    more.handlers.push(Handler::new("open", "", Action::Code));
    assert!(!mg_nui::is_current(&ws.module, "test_nui", &w, &more));
    // A handler's action is only the code it starts with: changing it is
    // no change to what was built.
    let mut other = built.clone();
    other.handlers[0].action = Action::Code;
    assert!(mg_nui::is_current(&ws.module, "test_nui", &w, &other));
}

#[test]
fn code_written_in_a_section_is_built_and_kept() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    let first = ws.module.get(&key("test_nui_e", ResType::NCS)).unwrap().to_vec();
    let custom = mg_nui::insert_code(
        &text(&ws, "test_nui_e"),
        "top",
        "void Say(object oPC) { SendMessageToPC(oPC, \"custom\"); }\n",
    )
    .unwrap();
    let custom = mg_nui::insert_code(&custom, "click mg_close", "        Say(oPlayer);\n").unwrap();
    ws.module.set(key("test_nui_e", ResType::NSS), custom.clone().into_bytes());
    build(&mut ws, &rm);
    assert_eq!(text(&ws, "test_nui_e"), custom);
    assert_ne!(ws.module.get(&key("test_nui_e", ResType::NCS)).unwrap(), first.as_slice());
    assert!(generate(&ws, &rm).unwrap().command.edits.is_empty());
}

#[test]
fn code_that_does_not_compile_in_a_section_fails_the_build_and_leaves_nothing() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    // The window changes too: none of it may come through.
    let mut w = mg_nui::window();
    w["title"] = json!("Another title");
    ws.module.set(key("test_nui", ResType::JUI), serde_json::to_vec(&w).unwrap());
    ws.module.set(
        key("my_enter", ResType::NSS),
        b"#include \"test_nui_o\"\nvoid main() { Open_test_nui(GetEnteringObject()); }\n".to_vec(),
    );
    let broken = mg_nui::insert_code(
        &text(&ws, "test_nui_e"),
        "click mg_close",
        "        NoSuchFunction(oPlayer);\n",
    )
    .unwrap();
    ws.module.set(key("test_nui_e", ResType::NSS), broken.into_bytes());
    let before = ws.module.clone();
    let failed = generate(&ws, &rm).unwrap_err();
    let NuiError::Compile(e) = &failed else { panic!("{failed}") };
    assert_eq!(e.location().map(|(script, _)| script).as_deref(), Some("test_nui_e"), "{e}");
    assert!(e.message.contains("NoSuchFunction") || e.message.contains("UNDEFINED"), "{e}");
    // No command came of it: the module is as it was, to the byte.
    assert_eq!(ws.module.keys().count(), before.keys().count());
    for k in before.keys() {
        assert_eq!(ws.module.get(k), before.get(k), "{k}");
    }
    assert!(!ws.module.contains(&key("my_enter", ResType::NCS)));
}

#[test]
fn an_event_script_written_by_hand_is_compiled_as_it_is_while_there_are_no_handlers() {
    let rm = game(&corpus!());
    let mut ws = project_of(&mg_nui::window(), &Settings::default());
    build(&mut ws, &rm);
    let custom = b"void main() { SendMessageToPC(NuiGetEventPlayer(), \"custom\"); }".to_vec();
    ws.module.set(key("test_nui_e", ResType::NSS), custom.clone());
    build(&mut ws, &rm);
    assert_eq!(ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap(), custom);
    assert!(compiled(&ws, "test_nui_e"));
    // One that does not compile changes nothing.
    ws.module.set(key("test_nui_e", ResType::NSS), b"void main() { MissingFunction(); }".to_vec());
    assert!(matches!(generate(&ws, &rm), Err(NuiError::Compile(_))));
    // One without a main makes no compiled script: not a window's script.
    ws.module.set(key("test_nui_e", ResType::NSS), b"void Helper() { }".to_vec());
    let silent = generate(&ws, &rm).unwrap_err();
    assert!(silent.to_string().contains("did not produce bytecode"), "{silent}");
}

#[test]
fn a_script_changed_outside_its_sections_is_not_built() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    let outside = format!("{}\n// After main\n", text(&ws, "test_nui_e"));
    ws.module.set(key("test_nui_e", ResType::NSS), outside.into_bytes());
    let refused = generate(&ws, &rm).unwrap_err();
    assert!(matches!(refused, NuiError::EditedByHand(_)), "{refused}");
    assert!(refused.to_string().contains("outside its mg:begin and mg:end lines"));
}

#[test]
fn an_opener_changed_by_hand_after_a_build_is_not_written_over() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    ws.module.set(key("test_nui_o", ResType::NSS), b"void main() {} // handwritten".to_vec());
    let refused = generate(&ws, &rm).unwrap_err();
    assert!(matches!(refused, NuiError::EditedByHand(_)), "{refused}");
    assert!(refused.to_string().contains("test_nui_o.nss was edited"));
}

/// Edit code writes the event script before the first Build: Build takes it
/// as the window's.
#[test]
fn an_event_script_written_before_the_first_build_is_the_windows() {
    let rm = game(&corpus!());
    let mut s = Settings::default();
    s.handlers.push(Handler::new("open", "", Action::Code));
    let mut ws = project_of(&mg_nui::window(), &s);
    let early = mg_nui::insert_code(
        &mg_nui::event_source("test_nui", &s).unwrap(),
        "open",
        "        SendMessageToPC(oPlayer, \"early\");\n",
    )
    .unwrap();
    ws.module.set(key("test_nui_e", ResType::NSS), early.clone().into_bytes());
    build(&mut ws, &rm);
    assert_eq!(text(&ws, "test_nui_e"), early);
    assert!(compiled(&ws, "test_nui_e"));
}

/// A script that includes the opener carries its layout in its compiled
/// code: every Build compiles it again, or the game would open the old one.
#[test]
fn scripts_that_include_the_opener_are_compiled_again_by_every_build() {
    let rm = game(&corpus!());
    let mut w = mg_nui::window();
    w["root"]["children"] = json!([{"type":"label","value":"First"}]);
    let mut ws = project_of(&w, &Settings::default());
    // Directly, through an include of the builder's own, and written
    // as the compiler takes it but a text search would not.
    ws.module.set(
        key("my_enter", ResType::NSS),
        b"#include \"test_nui_o\"\nvoid main() { Open_test_nui(GetEnteringObject()); }\n".to_vec(),
    );
    ws.module.set(
        key("my_inc", ResType::NSS),
        b"#include   \"TEST_NUI_O\"\nvoid Show(object o) { Open_test_nui(o); }\n".to_vec(),
    );
    ws.module.set(
        key("my_used", ResType::NSS),
        b"#include \"my_inc\"\nvoid main() { Show(GetLastUsedBy()); }\n".to_vec(),
    );
    // One that only speaks of it is left alone.
    ws.module.set(
        key("my_other", ResType::NSS),
        b"// #include \"test_nui_o\" was here\nvoid main() { }\n".to_vec(),
    );
    let made = build(&mut ws, &rm);
    assert!(made.failed.is_empty(), "{:?}", made.failed);
    let first: Vec<Vec<u8>> = ["my_enter", "my_used"]
        .map(|n| ws.module.get(&key(n, ResType::NCS)).unwrap().to_vec())
        .into();
    assert!(!ws.module.contains(&key("my_inc", ResType::NCS)), "an include makes none");
    assert!(!ws.module.contains(&key("my_other", ResType::NCS)), "not the window's to compile");
    w["root"]["children"] = json!([{"type":"label","value":"Second, longer text"}]);
    ws.module.set(key("test_nui", ResType::JUI), serde_json::to_vec(&w).unwrap());
    build(&mut ws, &rm);
    for (name, was) in ["my_enter", "my_used"].iter().zip(&first) {
        assert_ne!(ws.module.get(&key(name, ResType::NCS)).unwrap(), was.as_slice(), "{name}");
    }
}

#[test]
fn a_script_that_includes_the_opener_and_fails_is_said_and_keeps_what_it_had() {
    let rm = game(&corpus!());
    let mut ws = project();
    ws.module.set(
        key("my_broken", ResType::NSS),
        b"#include \"test_nui_o\"\nvoid main() { Open_test_nui(NoSuchFunction()); }\n".to_vec(),
    );
    ws.module.set(key("my_broken", ResType::NCS), b"NCS as it was".to_vec());
    ws.module.set(
        key("my_enter", ResType::NSS),
        b"#include \"test_nui_o\"\nvoid main() { Open_test_nui(GetEnteringObject()); }\n".to_vec(),
    );
    let made = build(&mut ws, &rm);
    // The window's own scripts and the other caller are built all the same.
    assert!(compiled(&ws, "test_nui_e") && compiled(&ws, "my_enter"));
    assert_eq!(made.failed.len(), 1, "{:?}", made.failed);
    assert_eq!(made.failed[0].script, key("my_broken", ResType::NSS));
    let said = made.failed[0].result.as_ref().unwrap_err();
    assert_eq!(said.location().map(|(script, _)| script).as_deref(), Some("my_broken"), "{said}");
    assert_eq!(ws.module.get(&key("my_broken", ResType::NCS)).unwrap(), b"NCS as it was");
    // It is not remembered as compiled: Compile All tries it again.
    assert!(!made.remembered.scripts.contains_key(&key("my_broken", ResType::NSS)));
    assert!(made.remembered.scripts.contains_key(&key("my_enter", ResType::NSS)));
}

#[test]
fn a_build_with_debug_information_writes_the_debug_files() {
    let rm = game(&corpus!());
    let mut ws = project();
    ws.module.set(
        key("my_enter", ResType::NSS),
        b"#include \"test_nui_o\"\nvoid main() { Open_test_nui(GetEnteringObject()); }\n".to_vec(),
    );
    let how = Build { debug: true, ..Build::new(&rm) };
    let made = mg_nui::generate(&ws.module, "test_nui", &how).unwrap();
    ws.apply(made.command).unwrap();
    for name in ["test_nui_e", "my_enter"] {
        assert!(ws.module.get(&key(name, ResType::NDB)).is_some_and(|b| !b.is_empty()), "{name}");
    }
    assert!(!ws.module.contains(&key("test_nui_o", ResType::NDB)));
    assert!(!ws.module.keys().any(|k| k.resref.to_string().starts_with("__nui")));
}

/// What Build compiled is remembered as Compile All remembers it, so that
/// Compile All passes over it.
#[test]
fn compile_all_passes_over_what_a_build_compiled() {
    let rm = game(&corpus!());
    let mut ws = project();
    ws.module.set(
        key("my_enter", ResType::NSS),
        b"#include \"test_nui_o\"\nvoid main() { Open_test_nui(GetEnteringObject()); }\n".to_vec(),
    );
    ws.module.set(key("my_other", ResType::NSS), b"void main() { }\n".to_vec());
    let known = Remembered { made_with: 7, ..Default::default() };
    let how = Build { known: Some(&known), ..Build::new(&rm) };
    let made = mg_nui::generate(&ws.module, "test_nui", &how).unwrap();
    ws.apply(made.command).unwrap();
    assert_eq!(made.remembered.made_with, 7);
    let noted: Vec<String> = {
        let mut names: Vec<String> =
            made.remembered.scripts.keys().map(|k| k.resref.to_string()).collect();
        names.sort();
        names
    };
    assert_eq!(noted, ["my_enter", "test_nui_e"]);
    let pass =
        mg_module::build::compile_changed(&mut ws.module, &rm, None, &made.remembered, None, false);
    let mut again: Vec<String> = pass.results.iter().map(|r| r.script.resref.to_string()).collect();
    again.sort();
    // (The opener is an include: Compile All looks at it and makes nothing.)
    assert_eq!(again, ["my_other", "test_nui_o"], "only what Build did not compile");
}

#[test]
fn a_build_compiles_with_the_compiler_of_the_options() {
    let rm = game(&corpus!());
    let ws = project();
    // One that cannot be run: the built-in one would have built the window.
    let external = ExternalCompiler {
        program: "/nonexistent/moonglow-test-compiler".into(),
        ..Default::default()
    };
    let how = Build { external: Some(&external), ..Build::new(&rm) };
    let failed = mg_nui::generate(&ws.module, "test_nui", &how).unwrap_err();
    let NuiError::Compile(e) = &failed else { panic!("{failed}") };
    assert!(e.message.starts_with("External compiler: "), "{e}");
}

/// Build with a real external compiler (neverwinter.nim's
/// `nwn_script_comp`, the official compiler as a program, in its own
/// arguments): the window's event script compiles to the bytes the built-in
/// compiler makes (the same compiler), and with debug files asked for, its
/// `.ndb` is made too.
#[test]
fn a_build_with_an_external_compiler_makes_what_the_built_in_makes() {
    let root = corpus!();
    let tool = mg_testkit::oracle_tool!("nwn_script_comp");
    let rm = game(&root);
    let ws = project();
    let built_in = mg_nui::generate(&ws.module, "test_nui", &Build::new(&rm)).unwrap();
    let external = ExternalCompiler {
        program: tool,
        game: Some(root.clone()),
        user: Some(mg_testkit::scratch_dir("nui-external-compiler-user")),
        ..Default::default()
    };
    // (Debug files asked for as Options asks: of the compiler, and of the build.)
    let made = |debug: bool| {
        let external = ExternalCompiler { debug, ..external.clone() };
        let how = Build { external: Some(&external), debug, ..Build::new(&rm) };
        let made = mg_nui::generate(&ws.module, "test_nui", &how).unwrap();
        let mut ws = project();
        ws.apply(made.command).unwrap();
        ws
    };
    let with = made(false);
    let mut theirs = project();
    theirs.apply(built_in.command).unwrap();
    let ncs = key("test_nui_e", ResType::NCS);
    assert!(compiled(&with, "test_nui_e"));
    assert_eq!(with.module.get(&ncs), theirs.module.get(&ncs), "the same compiler, the same bytes");
    assert_eq!(text(&with, "test_nui_o"), text(&theirs, "test_nui_o"));
    let debug = made(true);
    assert!(debug.module.contains(&key("test_nui_e", ResType::NDB)), "a debug file");
}

/// A handler shows a swap layout variant by calling the opener include's
/// function for it, so a variant edited after the code was written still
/// reaches the game on the next Build.
#[test]
fn a_variant_edited_after_its_handler_was_written_reaches_the_next_build() {
    let rm = game(&corpus!());
    let mut w = mg_nui::window();
    w["root"] = json!({"type":"col","children":[
        {"type":"button","id":"next","label":"Next"},
        {"type":"group","id":"pages","children":[{"type":"col","children":[{"type":"label","value":"First"}]}]}
    ]});
    let mut s = Settings::default();
    s.variants.insert(
        "details page".into(),
        json!({"type":"col","children":[{"type":"label","value":"Old text"}]}),
    );
    s.variant_targets.insert("details page".into(), "pages".into());
    s.handlers.push(click("next", Action::Code));
    let mut ws = project_of(&w, &s);
    build(&mut ws, &rm);
    let function = mg_nui::variant_function("test_nui", "details page");
    let initial = mg_nui::initial_function("test_nui", "pages");
    let call = format!(
        "        NuiSetGroupLayout(oPlayer, nToken, \"pages\", {function}());\n        NuiSetGroupLayout(oPlayer, nToken, \"pages\", {initial}());\n"
    );
    let written = mg_nui::insert_code(&text(&ws, "test_nui_e"), "click next", &call).unwrap();
    ws.module.set(key("test_nui_e", ResType::NSS), written.clone().into_bytes());
    build(&mut ws, &rm);
    assert!(compiled(&ws, "test_nui_e"));
    let opener = text(&ws, "test_nui_o");
    assert!(opener.contains(&format!("json {function}()")));
    assert!(opener.contains(&format!("json {initial}()")));
    assert!(opener.contains("Old text"));
    // Edit the variant and build: the opener has it, the handler is unchanged.
    s = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    s.variants.insert(
        "details page".into(),
        json!({"type":"col","children":[{"type":"label","value":"New text"}]}),
    );
    ws.module.set(key("test_nui", ResType::TXT), s.bytes());
    let before = ws.module.get(&key("test_nui_e", ResType::NCS)).unwrap().to_vec();
    build(&mut ws, &rm);
    let opener = text(&ws, "test_nui_o");
    assert!(opener.contains("New text") && !opener.contains("Old text"));
    assert_eq!(text(&ws, "test_nui_e"), written);
    assert_ne!(ws.module.get(&key("test_nui_e", ResType::NCS)).unwrap(), before.as_slice());
}

#[test]
fn settings_built_before_the_two_hashes_were_one_read_as_built() {
    let rm = game(&corpus!());
    let mut ws = project();
    build(&mut ws, &rm);
    // As the settings were written then: the same number under two names.
    let txt = key("test_nui", ResType::TXT);
    let mut raw: Value = serde_json::from_slice(ws.module.get(&txt).unwrap()).unwrap();
    raw["compiled_event_hash"] = raw["event_hash"].clone();
    ws.module.set(txt, serde_json::to_vec_pretty(&raw).unwrap());
    let read = Settings::parse(ws.module.get(&txt).unwrap()).unwrap();
    assert!(mg_nui::is_current(&ws.module, "test_nui", &mg_nui::window(), &read));
    // The next Build that writes the settings leaves the one.
    let mut w = mg_nui::window();
    w["title"] = json!("Changed");
    ws.module.set(key("test_nui", ResType::JUI), serde_json::to_vec(&w).unwrap());
    build(&mut ws, &rm);
    let raw: Value = serde_json::from_slice(ws.module.get(&txt).unwrap()).unwrap();
    assert!(raw.get("compiled_event_hash").is_none() && raw["event_hash"].is_string());
}
