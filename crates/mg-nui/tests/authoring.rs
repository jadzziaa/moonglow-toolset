use mg_core::ResType;
use mg_edit::Workspace;
use mg_module::{Module, ModuleLocation};
use mg_nui::{Binding, Settings, Severity, key};
use mg_resman::{GameInstall, ResMan};
use serde_json::{Value, json};

fn project() -> Workspace {
    let mut ws = Workspace::new(Module::new());
    ws.apply(mg_nui::create(&ws.module, "test_nui").unwrap()).unwrap();
    ws
}

#[test]
fn nested_group_swap_routes_compile_in_both_delivery_modes_without_window_fallback() {
    use mg_nui::{Action, Route};
    let rm = ResMan::for_game(&GameInstall::new(mg_testkit::corpus!(), None, "en")).unwrap();
    for from_resref in [false, true] {
        let mut ws = project();
        let mut doc = mg_nui::window();
        doc["root"]["children"] = json!([
            {"type":"label","id":"header","value":"Fixed header"},
            {"type":"row","children":[{"type":"button","id":"next","label":"Next"},
                {"type":"group","id":"panel","width":260.0,"height":160.0,"children":[
                    {"type":"label","value":"Original"}]}]},
            {"type":"label","value":"Fixed footer"}]);
        let mut settings = Settings { from_resref, ..Default::default() };
        settings.views.insert("details".into(), json!({"type":"col","children":[
            {"type":"button","id":"inner_next","label":"Inner next"},
            {"type":"group","id":"nested","children":[{"type":"label","value":"Inner original"}]}]}));
        settings
            .views
            .insert("inner_details".into(), json!({"type":"label","value":"Inner replaced"}));
        for (element, group, view) in
            [("next", "panel", "details"), ("inner_next", "nested", "inner_details")]
        {
            settings.actions.push(Route {
                event: "click".into(),
                element: element.into(),
                action: Action::View { group: group.into(), view: view.into() },
            });
        }
        assert!(mg_nui::validate(&doc, &settings).iter().all(|d| d.severity != Severity::Error));
        ws.module.set(key("test_nui", ResType::JUI), serde_json::to_vec(&doc).unwrap());
        ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
        ws.apply(
            mg_nui::generate(&ws.module, "test_nui", |n, t| {
                rm.get_named(n, t).ok().map(|v| v.into_owned())
            })
            .unwrap(),
        )
        .unwrap();
        let source =
            String::from_utf8_lossy(ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap());
        assert_eq!(source.matches("NuiSetGroupLayout").count(), 2);
        assert!(!source.contains("_window_"));
        assert!(source.contains("panel") && source.contains("nested"));
        assert!(ws.module.get(&key("test_nui_e", ResType::NCS)).unwrap().starts_with(b"NCS "));
        assert_eq!(
            mg_nui::parse(ws.module.get(&key("test_nui", ResType::JUI)).unwrap()).unwrap(),
            doc
        );
        settings.actions[0].action =
            Action::View { group: "header".into(), view: "details".into() };
        assert!(
            mg_nui::validate(&doc, &settings)
                .iter()
                .any(|d| d.severity == Severity::Error && d.message.contains("Group ID"))
        );
    }
}

#[test]
fn focus_blur_and_range_routes_compile_with_installed_engine_api() {
    let rm = ResMan::for_game(&GameInstall::new(mg_testkit::corpus!(), None, "en")).unwrap();
    let mut ws = project();
    let mut doc = mg_nui::window();
    doc["root"]["children"] = json!([
        {"type":"textedit","id":"edit","value":{"bind":"text"},"length":128,"multiline":false,"wordwrap":false},
        {"type":"list","id":"rows","row_count":0,"row_height":30.0,"row_template":[]}
    ]);
    let mut s = Settings::default();
    s.bindings.insert("text".into(), Binding { value: json!(""), ..Default::default() });
    for (event, element) in [("focus", "edit"), ("blur", "edit"), ("range", "rows")] {
        s.actions.push(mg_nui::Route {
            event: event.into(),
            element: element.into(),
            action: mg_nui::Action::Close,
        });
    }
    let errors: Vec<_> =
        mg_nui::validate(&doc, &s).into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    ws.module.set(key("test_nui", ResType::JUI), serde_json::to_vec(&doc).unwrap());
    ws.module.set(key("test_nui", ResType::TXT), s.bytes());
    let generated = mg_nui::generate(&ws.module, "test_nui", |n, t| {
        rm.get_named(n, t).ok().map(|b| b.into_owned())
    })
    .unwrap();
    ws.apply(generated).unwrap();
    let source = String::from_utf8_lossy(ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap());
    for event in ["focus", "blur", "range"] {
        assert!(source.contains(event));
    }
    assert!(!ws.module.get(&key("test_nui_e", ResType::NCS)).unwrap().is_empty());
    s.actions[0].event = "closed".into();
    assert!(mg_nui::validate(&doc, &s).iter().any(|d| d.message == "Unsupported event type"));
}

#[test]
fn rejects_collisions_and_names_without_truncation() {
    for name in ["", "longer_than_fourteen", "Upper", "a/b", "ą", "a\0"] {
        assert!(mg_nui::check_name(name).is_err(), "{name}");
    }
    assert!(mg_nui::check_name("abcdefghijklm1").is_ok());
    for ty in [ResType::JUI, ResType::TXT, ResType::NSS, ResType::NCS] {
        let mut m = Module::new();
        let name =
            if matches!(ty, ResType::NSS | ResType::NCS) { "test_nui_e" } else { "test_nui" };
        m.set(key(name, ty), vec![1, 2, 3]);
        assert!(mg_nui::create(&m, "test_nui").is_err());
        assert_eq!(m.get(&key(name, ty)).unwrap(), [1, 2, 3]);
    }
}

#[test]
fn retains_unknown_json_and_rejects_ambiguous_or_invalid_input() {
    let raw =
        br#"{"version":1,"root":{"type":"future","payload":[2.5,{"a":1}]},"vendor":{"x":true}}"#;
    let parsed = mg_nui::parse(raw).unwrap();
    assert_eq!(parsed["vendor"], json!({"x":true}));
    assert!(
        mg_nui::validate(&parsed, &Settings::default())
            .iter()
            .any(|d| d.severity == Severity::Warning && d.message.contains("Unknown widget"))
    );
    for bad in [
        br#"{"a":1,"a":2}"#.as_slice(),
        br#"{"root":{"x":1,"x":2}}"#,
        b"[] junk",
        &[0xff],
        b"{",
        b"1e999",
    ] {
        assert!(mg_nui::parse(bad).is_err());
    }
}

#[test]
fn validates_stock_shapes_static_geometry_and_list_bind_arrays() {
    let mut w = mg_nui::window();
    for ty in mg_nui::ELEMENTS {
        w["root"]["children"] = json!([mg_nui::template(ty)]);
        let errors: Vec<_> = mg_nui::validate(&w, &Settings::default())
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{ty}: {errors:?}");
    }
    w["root"] = mg_nui::template("list");
    w["root"]["row_template"] = json!([[{"type":"label","value":{"bind":"names"}}, 150.0, true]]);
    let mut s = Settings::default();
    s.bindings.insert(
        "names".into(),
        Binding { value: json!(["A", "B"]), watch: false, ..Default::default() },
    );
    assert!(!mg_nui::validate(&w, &s).iter().any(|d| d.severity == Severity::Error));
    s.bindings.get_mut("names").unwrap().value = json!("not an array");
    assert!(mg_nui::validate(&w, &s).iter().any(|d| d.message.contains("array for list")));
    w["root"]["width"] = json!({"bind":"width"});
    assert!(mg_nui::validate(&w, &s).iter().any(|d| d.message.contains("static numbers")));
}

#[test]
fn missing_defaults_and_bad_settings_are_not_silently_invented() {
    let mut w = mg_nui::window();
    w["title"] = json!({"bind":"caption"});
    assert!(
        mg_nui::validate(&w, &Settings::default())
            .iter()
            .any(|d| d.message.contains("explicit initial"))
    );
    assert!(Settings::parse(br#"{"format":"another-author"}"#).is_err());
    let mut s = Settings::default();
    s.extra.insert("future".into(), json!({"keep":true}));
    assert_eq!(Settings::parse(&s.bytes()).unwrap(), s);
}

#[test]
fn warns_about_native_spacer_rectangle_scissor_without_rewriting_output() {
    let mut w = mg_nui::window();
    w["root"]["children"] = json!([{
        "type":"spacer", "draw_list_scissor":true,
        "draw_list":[{"type":7,"enabled":true,"rect":{"x":10.0,"y":10.0,"w":140.0,"h":70.0}}]
    }]);
    let before = w.clone();
    let warnings = mg_nui::validate(&w, &Settings::default());
    assert!(warnings.iter().any(|d| d.severity == Severity::Warning
        && d.path == "/root/children/0/draw_list_scissor"
        && d.message.contains("blank window")));
    assert!(!warnings.iter().any(|d| d.severity == Severity::Error));
    assert_eq!(w, before, "A runtime advisory must not silently change authored JSON");
    for (host, clip, kind, enabled) in [
        ("spacer", false, 7, true),
        ("spacer", true, 0, true),
        ("button", true, 7, true),
        ("spacer", true, 7, false),
    ] {
        w["root"]["children"][0]["type"] = json!(host);
        w["root"]["children"][0]["draw_list_scissor"] = json!(clip);
        w["root"]["children"][0]["draw_list"][0]["type"] = json!(kind);
        w["root"]["children"][0]["draw_list"][0]["enabled"] = json!(enabled);
        assert!(
            !mg_nui::validate(&w, &Settings::default())
                .iter()
                .any(|d| d.message.contains("blank window")),
            "Do not generalize the observed native case"
        );
    }
}

#[test]
fn api_edge_cases_options_list_count_and_draw_text_font() {
    let mut w = mg_nui::window();
    let mut s = Settings::default();
    w["root"] = mg_nui::template("list");
    w["root"]["row_count"] = json!({"bind":"names"});
    w["root"]["row_template"] = json!([[{"type":"label","value":{"bind":"names"}}, 150.0, true]]);
    s.bindings
        .insert("names".into(), Binding { value: json!(["one", "two"]), ..Default::default() });
    assert!(!mg_nui::validate(&w, &s).iter().any(|d| d.severity == Severity::Error));
    w["root"] = mg_nui::template("options");
    w["root"]["elements"] = json!([["not a combo", 0]]);
    assert!(mg_nui::validate(&w, &s).iter().any(|d| d.message.contains("string labels")));
    w["root"] = mg_nui::template("col");
    w["root"]["draw_list"] = json!([{"type":4,"enabled":true,"color":{"r":0,"g":0,"b":0,"a":255},"fill":null,"line_thickness":null,"rect":{"x":0.0,"y":0.0,"w":100.0,"h":30.0},"text":"żółć","font":"","arrayBinds":false}]);
    assert!(!mg_nui::validate(&w, &s).iter().any(|d| d.severity == Severity::Error));
    let raw = br#"{"format":"moonglow.nui/1","bindings":{"x":{"value":true,"custom":{"keep":3}}}}"#;
    let parsed = Settings::parse(raw).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&parsed.bytes()).unwrap()["bindings"]["x"]["custom"],
        json!({"keep":3})
    );
}

#[test]
fn ascii_script_reconstructs_unicode_json_and_initializes_before_watching() {
    let mut w = mg_nui::window();
    w["title"] = json!("Zażółć 🐉 \"quoted\" \\ newline\n".repeat(60));
    let mut s = Settings::default();
    s.bindings.insert(
        "żółć".into(),
        Binding { value: json!(["🐉", false, 1, 1.25]), watch: true, ..Default::default() },
    );
    let source = mg_nui::opener_source("test_nui", &w, &s).unwrap();
    assert!(source.is_ascii());
    let mut text = String::new();
    for line in source.lines().filter_map(|l| l.trim().strip_prefix("sLayout += ")) {
        text.push_str(&serde_json::from_str::<String>(line.strip_suffix(';').unwrap()).unwrap());
    }
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), w);
    assert!(source.find("NuiSetBind(").unwrap() < source.find("NuiSetBindWatch(").unwrap());
    assert!(source.contains("\"test_nui_e\""));
}

#[test]
fn create_and_json_drafts_use_exact_undo_and_archive_roundtrip() {
    let mut ws = project();
    let raw = b"{\"version\":1,\n\"unknown\":false,\"unfinished\":".to_vec();
    ws.apply(mg_edit::Command::new(
        "draft",
        vec![mg_edit::Edit::SetResource {
            key: key("test_nui", ResType::JUI),
            data: Some(raw.clone()),
        }],
    ))
    .unwrap();
    ws.undo().unwrap();
    assert!(mg_nui::parse(ws.module.get(&key("test_nui", ResType::JUI)).unwrap()).is_ok());
    ws.redo().unwrap();
    assert_eq!(ws.module.get(&key("test_nui", ResType::JUI)).unwrap(), raw);
    let dir = mg_testkit::scratch_dir("nui-roundtrip");
    let path = dir.join("fixture.mod");
    ws.module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let reopened = Module::open(&path).unwrap();
    for k in ws.module.keys() {
        assert_eq!(reopened.get(k), ws.module.get(k));
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn installed_compiler_accepts_generation_and_failures_are_transactional() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let compile = |ws: &Workspace| {
        mg_nui::generate(&ws.module, "test_nui", |n, t| {
            rm.get_named(n, t).ok().map(|b| b.into_owned())
        })
    };
    let mut ws = project();
    let original = ws.module.clone();
    let mut settings = Settings::default();
    settings.bindings.insert(
        "unicode".into(),
        Binding { value: json!("Zażółć 🐉"), watch: true, ..Default::default() },
    );
    ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
    let cmd = compile(&ws).unwrap();
    assert_eq!(ws.module.keys().count(), 2, "prepare must not mutate");
    ws.apply(cmd).unwrap();
    assert!(ws.module.get(&key("test_nui_o", ResType::NCS)).is_none());
    assert!(ws.module.get(&key("test_nui_e", ResType::NCS)).unwrap().starts_with(b"NCS "));
    assert!(compile(&ws).unwrap().edits.is_empty(), "regeneration is idempotent");
    let changed = ws.module.clone();
    ws.undo().unwrap();
    assert!(ws.module.get(&key("test_nui_o", ResType::NCS)).is_none());
    ws.redo().unwrap();
    for k in changed.keys() {
        assert_eq!(ws.module.get(k), changed.get(k));
    }
    let custom = b"void main() { SendMessageToPC(NuiGetEventPlayer(), \"custom\"); }".to_vec();
    ws.module.set(key("test_nui_e", ResType::NSS), custom.clone());
    ws.apply(compile(&ws).unwrap()).unwrap();
    assert_eq!(ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap(), custom);
    ws.module.set(key("test_nui_e", ResType::NSS), b"void main() { MissingFunction(); }".to_vec());
    let before = ws.module.clone();
    assert!(compile(&ws).is_err());
    for k in before.keys() {
        assert_eq!(ws.module.get(k), before.get(k));
    }
    ws.module.set(key("test_nui_e", ResType::NSS), custom);
    ws.module.set(key("test_nui_o", ResType::NSS), b"void main() {} // handwritten".to_vec());
    assert!(compile(&ws).unwrap_err().contains("edited"));
    // Actual game API also accepts the client-JUI variant.
    let mut ws = Workspace::new(original);
    settings.from_resref = true;
    ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
    ws.apply(compile(&ws).unwrap()).unwrap();
}

#[test]
fn opener_is_an_include_with_explicit_manual_hookup_in_both_delivery_modes() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    for from_resref in [false, true] {
        let mut ws = project();
        let settings = Settings { from_resref, ..Default::default() };
        ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
        let cmd = mg_nui::generate(&ws.module, "test_nui", |n, t| {
            rm.get_named(n, t).ok().map(|b| b.into_owned())
        })
        .unwrap();
        ws.apply(cmd).unwrap();
        let source =
            std::str::from_utf8(ws.module.get(&key("test_nui_o", ResType::NSS)).unwrap()).unwrap();
        assert!(source.contains("void Open_test_nui(object oPlayer)"));
        for implicit in
            ["void main(", "GetPCSpeaker", "GetEnteringObject", "GetLastUsedBy", "OBJECT_SELF"]
        {
            assert!(!source.contains(implicit), "unexpected automatic hookup: {implicit}");
        }
        assert!(!ws.module.contains(&key("test_nui_o", ResType::NCS)));
        assert!(!ws.module.contains(&key("__nui_validate", ResType::NSS)));
        // The user chooses OnClientEnter in their own script. The generated
        // include must not introduce a second main or assume that event itself.
        let mut compiler = mg_script::Compiler::new(|name, ty| {
            if name == "manual_hookup" && ty == ResType::NSS {
                return Some(
                    b"#include \"test_nui_o\"\nvoid main() { Open_test_nui(GetEnteringObject()); }"
                        .to_vec(),
                );
            }
            mg_resman::ResKey::parse(name, ty)
                .and_then(|k| ws.module.get(&k))
                .map(<[u8]>::to_vec)
                .or_else(|| rm.get_named(name, ty).ok().map(|b| b.into_owned()))
        });
        assert!(compiler.compile("manual_hookup").unwrap().ncs.starts_with(b"NCS "));
        drop(compiler);
        // Stale bytecode from the previous generator must not stay executable.
        ws.module.set(key("test_nui_o", ResType::NCS), b"old opener bytecode".to_vec());
        let saved =
            Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
        let window = mg_nui::parse(ws.module.get(&key("test_nui", ResType::JUI)).unwrap()).unwrap();
        assert!(!mg_nui::is_current(&ws.module, "test_nui", &window, &saved));
        let cmd = mg_nui::generate(&ws.module, "test_nui", |n, t| {
            rm.get_named(n, t).ok().map(|b| b.into_owned())
        })
        .unwrap();
        ws.apply(cmd).unwrap();
        assert!(!ws.module.contains(&key("test_nui_o", ResType::NCS)));
        ws.undo().unwrap();
        assert_eq!(
            ws.module.get(&key("test_nui_o", ResType::NCS)).unwrap(),
            b"old opener bytecode"
        );
    }
}

#[test]
fn installed_include_has_the_supported_widget_constructors() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let bytes = rm.get_named("nw_inc_nui", ResType::NSS).unwrap();
    let source = String::from_utf8_lossy(&bytes);
    for ty in mg_nui::ELEMENTS {
        assert!(
            source.contains(&format!("NuiElement(\"{ty}\"")),
            "stock constructor disappeared: {ty}"
        );
    }
    assert!(source.contains("\"accepts_input\""));
    assert!(source.contains("\"row_template\""));
}

#[test]
fn installed_compiler_accepts_view_actions_unicode_and_row_bind_toggles() {
    use mg_nui::{Action, Route};
    let rm = ResMan::for_game(&GameInstall::new(mg_testkit::corpus!(), None, "en")).unwrap();
    let mut ws = project();
    let mut w = mg_nui::window();
    w["root"] = json!({"type":"group","id":"pages","children":[{"type":"col","children":[
        {"type":"button","id":"toggle","label":"Toggle"},
        {"type":"button","id":"set","label":"Set"},
        {"type":"button","id":"page","label":"Next"},
        {"type":"button","id":"array","label":"Row"},
        {"type":"button","id":"close","label":"Close"}
    ]}]});
    let mut s = Settings { window_id: Some("Żółw 🐉".into()), ..Default::default() };
    for (name, value) in
        [("flag", json!(false)), ("rows", json!([false, true])), ("caption", json!("first"))]
    {
        s.bindings.insert(name.into(), Binding { value, ..Default::default() });
    }
    s.views.insert(
        "details".into(),
        json!({"type":"col","children":[{"type":"label","value":{"bind":"caption"}}]}),
    );
    for (element, action) in [
        ("toggle", Action::Toggle { bind: "flag".into() }),
        ("set", Action::Set { bind: "caption".into(), value: json!("Zażółć 🐉") }),
        ("array", Action::Toggle { bind: "rows".into() }),
        ("page", Action::View { group: "pages".into(), view: "details".into() }),
        ("close", Action::Close),
    ] {
        s.actions.push(Route { event: "click".into(), element: element.into(), action });
    }
    s.actions.push(Route {
        event: "watch".into(),
        element: "flag".into(),
        action: Action::Set { bind: "caption".into(), value: json!("watched") },
    });
    ws.module.set(key("test_nui", ResType::JUI), serde_json::to_vec(&w).unwrap());
    ws.module.set(key("test_nui", ResType::TXT), s.bytes());
    let compile = |ws: &Workspace| {
        mg_nui::generate(&ws.module, "test_nui", |n, t| {
            rm.get_named(n, t).ok().map(|b| b.into_owned())
        })
    };
    ws.apply(compile(&ws).unwrap()).unwrap();
    assert!(ws.module.get(&key("test_nui_o", ResType::NCS)).is_none());
    assert!(ws.module.get(&key("test_nui_e", ResType::NCS)).unwrap().starts_with(b"NCS "));
    let built = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    assert!(mg_nui::is_current(&ws.module, "test_nui", &w, &built));
    let mut stale = built.clone();
    stale.actions[0].action = Action::Close;
    assert!(!mg_nui::is_current(&ws.module, "test_nui", &w, &stale));
    assert!(compile(&ws).unwrap().edits.is_empty());
    let bytes = ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap();
    assert!(bytes.is_ascii());
    let mut custom = bytes.to_vec();
    custom.extend_from_slice(b"\n// User annotation\n");
    ws.module.set(key("test_nui_e", ResType::NSS), custom.clone());
    ws.apply(compile(&ws).unwrap()).unwrap();
    assert_eq!(ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap(), custom);
    let mut settings =
        Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    settings.actions[0].action = Action::Close;
    ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
    assert!(compile(&ws).unwrap_err().contains("edited"));
    assert_eq!(ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap(), custom);
}

#[test]
fn rejects_watch_cycles_bad_action_types_and_non_native_draw_points() {
    use mg_nui::{Action, Route};
    let mut w = mg_nui::window();
    w["root"]["children"] =
        json!([{"type":"check","id":"flag","label":"Flag","value":{"bind":"a"}}]);
    let mut s = Settings::default();
    for name in ["a", "b"] {
        s.bindings.insert(name.into(), Binding { value: json!(false), ..Default::default() });
    }
    s.actions = vec![
        Route {
            event: "watch".into(),
            element: "a".into(),
            action: Action::Toggle { bind: "b".into() },
        },
        Route {
            event: "watch".into(),
            element: "b".into(),
            action: Action::Toggle { bind: "a".into() },
        },
    ];
    assert!(mg_nui::validate(&w, &s).iter().any(|d| d.message.contains("recursive")));
    s.actions = vec![Route {
        event: "click".into(),
        element: "flag".into(),
        action: Action::Set { bind: "a".into(), value: json!("wrong type") },
    }];
    assert!(mg_nui::validate(&w, &s).iter().any(|d| d.message.contains("Set a")));
    s.actions.clear();
    w["root"]["draw_list"] = json!([{"type":0,"points":[0.0,0.0,20.0,30.0]}]);
    assert!(!mg_nui::validate(&w, &s).iter().any(|d| d.severity == Severity::Error));
    w["root"]["draw_list"][0]["points"] = json!([{"x":0,"y":0}]);
    assert!(mg_nui::validate(&w, &s).iter().any(|d| d.message.contains("number pairs")));
    w["root"]["draw_list"] = json!([]);
    w["geometry"] = json!({"bind":"bounds"});
    s.bindings.insert(
        "bounds".into(),
        Binding { value: json!({"x":-1.0,"y":-1.0,"w":420.0,"h":240.0}), ..Default::default() },
    );
    s.actions = vec![Route {
        event: "click".into(),
        element: "flag".into(),
        action: Action::Set { bind: "bounds".into(), value: json!(true) },
    }];
    assert!(
        mg_nui::validate(&w, &s)
            .iter()
            .any(|d| d.message.contains("Set bounds") && d.message.contains("at /geometry"))
    );
}

#[test]
fn stock_static_constructor_arguments_reject_dynamic_binds() {
    let mut settings = Settings::default();
    settings
        .bindings
        .insert("dynamic".into(), Binding { value: json!(true), ..Default::default() });
    for (widget, field) in [
        ("group", "border"),
        ("text", "scrollbars"),
        ("list", "row_height"),
        ("options", "direction"),
        ("options", "elements"),
        ("tabbar", "elements"),
        ("textedit", "multiline"),
        ("textedit", "wordwrap"),
    ] {
        let mut window = mg_nui::window();
        window["root"]["children"] = json!([mg_nui::template(widget)]);
        window["root"]["children"][0][field] = json!({"bind":"dynamic"});
        assert!(
            mg_nui::validate(&window, &settings).iter().any(|d| d.severity == Severity::Error
                && d.path.ends_with(field)
                && d.message.contains("static")),
            "{widget}.{field}"
        );
    }
    // Window border is a json argument and is explicitly bindable; widget border is not.
    let mut window = mg_nui::window();
    window["border"] = json!({"bind":"dynamic"});
    assert!(!mg_nui::validate(&window, &settings).iter().any(|d| d.severity == Severity::Error));
    window["root"] = mg_nui::template("list");
    window["root"]["row_count"] = json!([1, 2]);
    assert!(mg_nui::validate(&window, &settings).iter().any(|d| d.path == "/root/row_count"));
    window["root"]["row_count"] = json!({"bind":"dynamic"});
    settings.bindings.get_mut("dynamic").unwrap().value = json!(["one", "two"]);
    window["border"] = true.into();
    assert!(!mg_nui::validate(&window, &settings).iter().any(|d| d.severity == Severity::Error));
}

#[test]
fn option_labels_require_literal_arrays_including_empty_and_singleton() {
    let mut settings = Settings::default();
    settings.bindings.insert(
        "labels".into(),
        Binding { value: json!(["First", "Second"]), ..Default::default() },
    );
    for ty in ["options", "tabbar"] {
        let mut window = mg_nui::window();
        window["root"] = mg_nui::template(ty);
        window["root"]["elements"] = json!({"bind":"labels"});
        assert!(
            mg_nui::validate(&window, &settings)
                .iter()
                .any(|d| d.severity == Severity::Error && d.path == "/root/elements")
        );
        for labels in [json!([]), json!(["Only"]), json!(["First", "Second", "Third"])] {
            window["root"]["elements"] = labels;
            assert!(
                !mg_nui::validate(&window, &settings).iter().any(|d| d.severity == Severity::Error),
                "{ty}"
            );
        }
    }
    let mut window = mg_nui::window();
    window["root"] = mg_nui::template("combo");
    window["root"]["elements"] = json!({"bind":"labels"});
    settings.bindings.get_mut("labels").unwrap().value = json!([["First", 0], ["Second", 1]]);
    assert!(!mg_nui::validate(&window, &settings).iter().any(|d| d.severity == Severity::Error));
}

#[test]
fn validates_chart_slot_data_image_regions_and_bind_parameters() {
    let mut window = mg_nui::window();
    let mut settings = Settings::default();
    settings
        .bindings
        .insert("points".into(), Binding { value: json!([1.0, 2.0]), ..Default::default() });
    window["root"]["children"] = json!([mg_nui::template("chart"), mg_nui::template("image")]);
    window["root"]["children"][0]["value"][0]["data"] = json!({"bind":"points"});
    window["root"]["children"][1]["image_region"] = json!({"x":0.0,"y":0.0,"w":32.0,"h":32.0});
    assert!(!mg_nui::validate(&window, &settings).iter().any(|d| d.severity == Severity::Error));
    settings.bindings.get_mut("points").unwrap().value = json!([1.0, "bad"]);
    assert!(
        mg_nui::validate(&window, &settings)
            .iter()
            .any(|d| d.path.ends_with("/data") && d.message.contains("numbers"))
    );
    settings.bindings.get_mut("points").unwrap().value = json!([1.0, 2.0]);
    window["root"]["children"][1]["image_region"] = json!({"x":0.0,"y":0.0});
    assert!(mg_nui::validate(&window, &settings).iter().any(|d| d.path.ends_with("/image_region")));
    window["root"]["children"] =
        json!([{"type":"label","value":{"bind":"caption","number_precision":"not an integer"}}]);
    settings
        .bindings
        .insert("caption".into(), Binding { value: json!(1.25), ..Default::default() });
    assert!(
        mg_nui::validate(&window, &settings).iter().any(|d| d.path.ends_with("/number_precision"))
    );
    window["root"]["children"][0]["value"]["number_precision"] = json!(2);
    assert!(!mg_nui::validate(&window, &settings).iter().any(|d| d.severity == Severity::Error));
}

#[test]
fn window_group_actions_and_lifecycle_route_identity_follow_native_api() {
    use mg_nui::{Action, Route};
    let window = mg_nui::window();
    let mut settings = Settings::default();
    settings.views.insert("other".into(), mg_nui::template("col"));
    settings.actions.push(Route {
        event: "open".into(),
        element: "ignored_a".into(),
        action: Action::View { group: "_window_".into(), view: "other".into() },
    });
    assert!(!mg_nui::validate(&window, &settings).iter().any(|d| d.severity == Severity::Error));
    settings.actions.push(Route {
        event: "open".into(),
        element: "ignored_b".into(),
        action: Action::Close,
    });
    assert!(mg_nui::validate(&window, &settings).iter().any(|d| d.message.contains("Duplicate")));
}

#[test]
fn installed_api_keeps_the_validated_static_dynamic_boundaries() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
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
        assert!(source.contains(contract), "Re-audit changed stock API contract: {contract}");
    }
    let bytes = rm.get_named("nwscript", ResType::NSS).unwrap();
    let source = String::from_utf8_lossy(&bytes);
    assert!(source.contains("special \"_window_\" root group"));
    assert!(source.contains("Returns -1 if the event is not originating from within an array."));
}

#[test]
fn close_requires_explicit_route_and_legacy_template_upgrades_safely() {
    let rm = ResMan::for_game(&GameInstall::new(mg_testkit::corpus!(), None, "en")).unwrap();
    let mut ws = project();
    let initial = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    assert_eq!(
        initial.actions,
        vec![mg_nui::Route {
            event: "click".into(),
            element: "mg_close".into(),
            action: mg_nui::Action::Close,
        }],
        "new windows must store a visible, editable Close event"
    );
    ws.module.set(key("test_nui", ResType::TXT), Settings::default().bytes());
    let build = |ws: &mut Workspace| {
        let command = mg_nui::generate(&ws.module, "test_nui", |n, t| {
            rm.get_named(n, t).ok().map(|b| b.into_owned())
        })
        .unwrap();
        ws.apply(command).unwrap();
    };
    let source = |ws: &Workspace| {
        String::from_utf8(ws.module.get(&key("test_nui_e", ResType::NSS)).unwrap().to_vec())
            .unwrap()
    };
    build(&mut ws);
    assert!(!source(&ws).contains("NuiDestroy"));
    assert!(!source(&ws).contains("mg_close"));
    let mut settings =
        Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    settings.actions.push(mg_nui::Route {
        event: "click".into(),
        element: "mg_close".into(),
        action: mg_nui::Action::Close,
    });
    ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
    build(&mut ws);
    assert_eq!(source(&ws).matches("NuiDestroy").count(), 1);
    assert_eq!(
        source(&ws),
        mg_nui::event_source(&settings).unwrap(),
        "preview and compiled handler use the same emitter"
    );
    settings = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    settings.actions.clear();
    ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
    build(&mut ws);
    assert!(!source(&ws).contains("NuiDestroy"));
    let legacy = r#"// Moonglow NUI event script: preserved on regeneration; edit this file.
void main()
{
    object oPlayer = NuiGetEventPlayer();
    int nToken = NuiGetEventWindow();
    string sType = NuiGetEventType();
    string sElement = NuiGetEventElement();
    if (sType == "click" && sElement == "mg_close")
        NuiDestroy(oPlayer, nToken);
    // Add click/watch handling here. List events expose NuiGetEventArrayIndex().
}
"#;
    settings = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
    settings.event_hash = None;
    ws.module.set(key("test_nui", ResType::TXT), settings.bytes());
    ws.module.set(key("test_nui_e", ResType::NSS), legacy.as_bytes().to_vec());
    build(&mut ws);
    assert!(!source(&ws).contains("NuiDestroy"));
    // Hand-edited scripts remain user-owned even when based on the legacy template.
    let manual = format!("{legacy}\n// Manual customization\n");
    ws.module.set(key("test_nui_e", ResType::NSS), manual.as_bytes().to_vec());
    build(&mut ws);
    assert_eq!(source(&ws), manual);
}
