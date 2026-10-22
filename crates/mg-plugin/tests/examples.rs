//! The documentation is held to the code: the example plugins of
//! `docs/plugins/examples` run, and the API reference and the editor's type
//! file name everything a plugin can use.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;

use mg_edit::{Command, Workspace};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_plugin::{
    Answer, Host, Input, Level, Plugin, Question, api_names, discover, inspect, run_check,
    run_command,
};
use mg_resman::ResKey;

fn docs() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs")
}

fn example(name: &str) -> Plugin {
    Plugin::load(&docs().join("plugins/examples").join(name)).unwrap()
}

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

/// A module with two creatures and two stores.
fn module() -> Module {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for (name, tag, class) in [("guard", "gate_guard", 4), ("captain", "CAPTAIN", 6)] {
        let mut g = Gff::new(*b"UTC ");
        g.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
        let mut levels = Struct::new(2);
        levels.set("Class", Value::Int(class));
        levels.set("ClassLevel", Value::Short(3));
        g.root.set("ClassList", Value::List(vec![levels]));
        m.set_gff(key(&format!("{name}.utc")), &g).unwrap();
    }
    for (name, tag) in [("smith", "shop_smith"), ("fence", "fence")] {
        let mut g = Gff::new(*b"UTM ");
        g.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
        g.root.set("MarkUp", Value::Int(120));
        g.root.set("MarkDown", Value::Int(60));
        m.set_gff(key(&format!("{name}.utm")), &g).unwrap();
    }
    m
}

/// A host that keeps the log and answers from a list.
#[derive(Default)]
struct TestHost {
    log: RefCell<Vec<String>>,
    answers: RefCell<VecDeque<Option<Answer>>>,
    asked: RefCell<Vec<Question>>,
}

impl Host for TestHost {
    fn log(&self, _: Level, text: &str) {
        self.log.borrow_mut().push(text.to_string());
    }

    fn ask(&self, question: &Question) -> Option<Answer> {
        self.asked.borrow_mut().push(question.clone());
        self.answers.borrow_mut().pop_front().flatten()
    }
}

fn input(module: &Module) -> Input {
    Input::new(module.clone(), None)
}

#[test]
fn every_example_registers_what_it_declares() {
    let found = discover(&docs().join("plugins/examples"));
    let mut ids = Vec::new();
    for plugin in found {
        let plugin = plugin.unwrap();
        let host = Rc::new(TestHost::default());
        assert_eq!(inspect(&plugin, host).unwrap(), Vec::<String>::new(), "{}", plugin.manifest.id);
        assert!(!plugin.manifest.description.is_empty(), "{}", plugin.manifest.id);
        ids.push(plugin.manifest.id);
    }
    assert_eq!(
        ids,
        [
            "example.builder-tags",
            "example.creature-report",
            "example.hello",
            "example.house-rules",
            "example.set-local",
            "example.store-markup",
            "example.tag-conventions",
            "example.tileset-import"
        ]
    );
}

/// The plugin the manual builds: it runs, and the manual has its lines.
#[test]
fn hello_is_the_plugin_of_the_manual() {
    let plugin = example("hello");
    let host = Rc::new(TestHost::default());
    let outcome = run_command(&plugin, "count", input(&module()), host.clone()).unwrap();
    assert!(outcome.edits.is_empty());
    assert_eq!(host.log.borrow().as_slice(), ["The module has 2 creature blueprints"]);

    // Asked, and answered no: nothing. Yes: every creature's comment.
    let outcome = run_command(&plugin, "stamp", input(&module()), host.clone()).unwrap();
    assert!(outcome.edits.is_empty());
    host.answers.borrow_mut().push_back(Some(Answer::Yes));
    let outcome = run_command(&plugin, "stamp", input(&module()), host).unwrap();
    assert_eq!(outcome.label, "Stamp creature comments");
    let mut ws = Workspace::new(module());
    ws.apply(Command::new(outcome.label, outcome.edits)).unwrap();
    ws.flush().unwrap();
    let guard = ws.module.gff(&key("guard.utc")).unwrap().unwrap();
    assert_eq!(
        guard.root.get("Comment"),
        Some(&Value::String(b"gate_guard: seen by Hello".to_vec()))
    );

    let guide = std::fs::read_to_string(docs().join("manual/16-writing-plugins.md")).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        let text = std::fs::read_to_string(plugin.dir.join(file)).unwrap();
        for line in text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with(';')) {
            assert!(guide.contains(line), "the manual lacks {file}'s line {line:?}");
        }
    }
}

#[test]
fn tag_conventions_fixes_what_its_check_finds() {
    let plugin = example("tag-conventions");
    let host = Rc::new(TestHost::default());
    let found = run_check(&plugin, "tag-case", input(&module()), host.clone()).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].finding.resource, key("guard.utc"));
    assert_eq!(found[0].finding.message, "tag \"gate_guard\" is not upper case");
    // Its fix: that tag alone, as one step.
    let fix = found[0].fix.as_ref().unwrap();
    let mut ws = Workspace::new(module());
    ws.apply(fix.command()).unwrap();
    let guard = ws.doc(&key("guard.utc")).unwrap();
    assert_eq!(guard.root.get("Tag"), Some(&Value::String(b"GATE_GUARD".to_vec())));
    assert_eq!(ws.can_undo(), Some("Upper-case this tag"));

    let outcome = run_command(&plugin, "fix-tags", input(&module()), host.clone()).unwrap();
    assert_eq!(outcome.label, "Upper-case creature tags");
    assert_eq!(host.log.borrow().as_slice(), ["1 creature tags changed"]);
    let mut ws = Workspace::new(module());
    ws.apply(Command::new(outcome.label, outcome.edits)).unwrap();
    ws.flush().unwrap();
    let fixed = ws.module.gff(&key("guard.utc")).unwrap().unwrap();
    assert_eq!(fixed.root.get("Tag"), Some(&Value::String(b"GATE_GUARD".to_vec())));
    assert!(run_check(&plugin, "tag-case", input(&ws.module), host).unwrap().is_empty());
}

/// The example's own tests (`mg plugin test`) pass on its fixtures (one
/// of them a project, on which Moonglow's own `tag-unique` runs with no
/// game), and the manual shows their files and the check's fix as they
/// are.
#[test]
fn tag_conventions_passes_its_own_tests() {
    let plugin = example("tag-conventions");
    // (A game that is there is not read: builtin.toml says root = false.)
    let results =
        mg_plugin::run_tests(&plugin, None, &|_, _| Err("no game is to be read".to_string()));
    let builtin: Vec<&mg_plugin::TestResult> =
        results.iter().filter(|r| r.file == "builtin.toml").collect();
    assert_eq!(builtin.len(), 2, "{results:#?}");
    assert!(builtin.iter().all(|r| r.passed() && r.notes.is_empty()), "{results:#?}");
    let results = mg_plugin::run_tests(&plugin, None, &|_, _| Ok(None));
    assert_eq!(results.len(), 5, "{results:#?}");
    assert!(results.iter().all(|r| r.passed()), "{results:#?}");

    let guide = std::fs::read_to_string(docs().join("manual/16-writing-plugins.md")).unwrap();
    for file in ["tags.toml", "builtin.toml"] {
        let tests = std::fs::read_to_string(plugin.dir.join("tests").join(file)).unwrap();
        for line in tests.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
            assert!(guide.contains(line), "the manual lacks tests/{file}'s line {line:?}");
        }
    }
    for line in ["local fix = ctx.fix(\"Upper-case this tag\")", "fix = fix,"] {
        assert!(guide.contains(line), "the manual lacks main.luau's line {line:?}");
    }
}

/// A resource's own events and the after-create hook (API 0.4): the
/// example's own tests pass (a save refused and warned of, a paste and a
/// new area edited), and the manual shows its save's handler as it is.
#[test]
fn house_rules_acts_on_its_events() {
    let plugin = example("house-rules");
    let on: Vec<mg_plugin::EventKind> = plugin.manifest.events.iter().map(|e| e.on).collect();
    use mg_plugin::EventKind as K;
    assert_eq!(on, [K::ResourceSaving, K::ObjectPasted, K::ResourceCreated]);
    let results = mg_plugin::run_tests(&plugin, None, &|_, _| Ok(None));
    assert_eq!(results.len(), 6, "{results:#?}");
    assert!(results.iter().all(|r| r.passed()), "{results:#?}");

    let guide = std::fs::read_to_string(docs().join("manual/16-writing-plugins.md")).unwrap();
    let code = std::fs::read_to_string(plugin.dir.join("main.luau")).unwrap();
    let start = code.find("mg.on(\"area-tags\"").unwrap();
    let end = start + code[start..].find("\nend)").unwrap();
    for line in code[start..end].lines().filter(|l| !l.trim().is_empty()) {
        assert!(guide.contains(line), "the manual lacks main.luau's line {line:?}");
    }
}

/// Settings (API 0.4): the example's own tests pass, its team's prefix
/// read from its fixture's moonglow.toml; and the manual shows its
/// settings as the manifest declares them.
#[test]
fn builder_tags_keeps_its_settings() {
    let plugin = example("builder-tags");
    let results = mg_plugin::run_tests(&plugin, None, &|_, _| Ok(None));
    assert_eq!(results.len(), 6, "{results:#?}");
    assert!(results.iter().all(|r| r.passed()), "{results:#?}");
    let scopes: Vec<(&str, mg_plugin::Scope)> =
        plugin.manifest.settings.iter().map(|s| (s.id.as_str(), s.scope)).collect();
    use mg_plugin::Scope::{Project, User, UserProject};
    assert_eq!(
        scopes,
        [
            ("prefix", Project),
            ("kinds", Project),
            ("max_length", Project),
            ("code", UserProject),
            ("strict", User)
        ]
    );
    // Its settings, and what sets one, are as the manual shows them.
    let guide = std::fs::read_to_string(docs().join("manual/16-writing-plugins.md")).unwrap();
    let cfg = std::fs::read_to_string(plugin.dir.join("plugin.cfg")).unwrap();
    let settings = &cfg[cfg.find("[setting]").unwrap()..];
    for line in settings.lines().filter(|l| !l.trim().is_empty()).take(12) {
        assert!(guide.contains(line), "the manual lacks plugin.cfg's line {line:?}");
    }
    for line in ["ctx.settings:set(\"code\", answer.code)", "local s = ctx.settings"] {
        assert!(guide.contains(line), "the manual lacks main.luau's line {line:?}");
    }
}

#[test]
fn store_markup_sets_what_its_form_says() {
    let plugin = example("store-markup");
    let host = Rc::new(TestHost::default());
    let values = BTreeMap::from([
        ("sell".to_string(), serde_json::json!(200)),
        ("buy".to_string(), serde_json::json!(40)),
        ("only".to_string(), serde_json::json!("shop_")),
    ]);
    host.answers.borrow_mut().push_back(Some(Answer::Values(values)));
    let outcome = run_command(&plugin, "set-markup", input(&module()), host.clone()).unwrap();
    let Question::Form { title, fields } = &host.asked.borrow()[0] else { panic!("no form") };
    assert_eq!(title, "Markup of 2 stores");
    assert_eq!(fields.len(), 3);
    let mut ws = Workspace::new(module());
    ws.apply(Command::new(outcome.label, outcome.edits)).unwrap();
    ws.flush().unwrap();
    let prices = |name: &str| {
        let g = ws.module.gff(&key(name)).unwrap().unwrap();
        (g.root.get("MarkUp").cloned(), g.root.get("MarkDown").cloned())
    };
    assert_eq!(prices("smith.utm"), (Some(Value::Int(200)), Some(Value::Int(40))));
    assert_eq!(prices("fence.utm"), (Some(Value::Int(120)), Some(Value::Int(60))));
    assert_eq!(host.log.borrow().as_slice(), ["1 of 2 stores set to sell at 200% and buy at 40%"]);

    // The form closed: nothing changes.
    let host = Rc::new(TestHost::default());
    let outcome = run_command(&plugin, "set-markup", input(&module()), host).unwrap();
    assert!(outcome.edits.is_empty());

    // No stores: it says so, and asks nothing more.
    let mut bare = Module::new();
    bare.set_info(&Gff::new(*b"IFO ")).unwrap();
    let host = Rc::new(TestHost::default());
    let outcome = run_command(&plugin, "set-markup", input(&bare), host.clone()).unwrap();
    assert!(outcome.edits.is_empty());
    assert_eq!(
        host.asked.borrow().as_slice(),
        [Question::Message("The module has no store blueprints.".into())]
    );
}

/// API 0.4: commands for what was right-clicked. A local variable set on
/// a placed object and a blueprint (one of that name replaced), the
/// game's blueprint left alone; and an area's objects counted.
#[test]
fn set_local_acts_on_the_selection() {
    use mg_core::ResRef;
    use mg_plugin::{Selected, Selection, Target};
    use mg_rules::ObjectKind;
    let plugin = example("set-local");
    let decl = |id: &str| plugin.manifest.commands.iter().find(|c| c.id == id).unwrap().clone();
    assert_eq!(decl("set-local").targets.len(), 9);
    assert_eq!(decl("count-objects").targets, [Target::Area]);
    let mut m = module();
    let mut git = Gff::new(*b"GIT ");
    let mut chest = Struct::new(9);
    chest.set("Tag", Value::String(b"CHEST".to_vec()));
    let mut waypoint = Struct::new(5);
    waypoint.set("Tag", Value::String(b"WP_DOOR".to_vec()));
    git.root.set("Placeable List", Value::List(vec![chest]));
    git.root.set("WaypointList", Value::List(vec![waypoint.clone(), waypoint]));
    m.set_gff(key("tavern.git"), &git).unwrap();
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", Value::String(b"TAVERN".to_vec()));
    m.set_gff(key("tavern.are"), &are).unwrap();
    // The guard has the variable already, as an int.
    let mut guard = m.gff(&key("guard.utc")).unwrap().unwrap();
    let mut var = Struct::new(0);
    var.set("Name", Value::String(b"role".to_vec()));
    var.set("Type", Value::Dword(1));
    var.set("Value", Value::Int(7));
    guard.root.set("VarTable", Value::List(vec![var]));
    m.set_gff(key("guard.utc"), &guard).unwrap();

    let area = ResRef::from_str("tavern").unwrap();
    let selection = Selection {
        selected: vec![
            Selected::Object { area, kind: ObjectKind::Placeable, index: 0 },
            Selected::Blueprint(key("guard.utc")),
            Selected::Blueprint(key("nw_chest.utp")),
        ],
        open: Some(Selected::Area(area)),
    };
    let host = Rc::new(TestHost::default());
    let values = BTreeMap::from([
        ("name".to_string(), serde_json::json!("role")),
        ("type".to_string(), serde_json::json!("string")),
        ("value".to_string(), serde_json::json!("workbench")),
    ]);
    host.answers.borrow_mut().push_back(Some(Answer::Values(values)));
    let run_on = Input { selection, ..input(&m) };
    let outcome = run_command(&plugin, "set-local", run_on, host.clone()).unwrap();
    let Question::Form { title, .. } = &host.asked.borrow()[0] else { panic!("no form") };
    assert_eq!(title, "A local variable on 2 objects", "not the game's blueprint");
    assert_eq!(outcome.label, "Set role");
    assert_eq!(
        host.log.borrow().as_slice(),
        ["CHEST: role = workbench", "gate_guard: role = workbench"]
    );
    let mut ws = Workspace::new(m.clone());
    ws.apply(Command::new(outcome.label, outcome.edits)).unwrap();
    let vars = |ws: &mut Workspace, name: &str, at: Option<(&str, usize)>| {
        let g = ws.doc(&key(name)).unwrap();
        let s = match at {
            Some((list, i)) => &g.root.list(list).unwrap()[i],
            None => &g.root,
        };
        (s.list("VarTable").unwrap().iter())
            .map(|v| {
                let text = |l: &str| String::from_utf8(v.string(l).unwrap().to_vec()).unwrap();
                (text("Name"), v.integer("Type").unwrap(), text("Value"))
            })
            .collect::<Vec<_>>()
    };
    let role = ("role".to_string(), 3, "workbench".to_string());
    assert_eq!(
        vars(&mut ws, "tavern.git", Some(("Placeable List", 0))),
        std::slice::from_ref(&role)
    );
    assert_eq!(vars(&mut ws, "guard.utc", None), [role], "the int replaced");

    // The area right-clicked; else the one open.
    let host = Rc::new(TestHost::default());
    let on = Selection { selected: vec![Selected::Area(area)], open: None };
    run_command(&plugin, "count-objects", Input { selection: on, ..input(&m) }, host.clone())
        .unwrap();
    let opened = Selection { selected: vec![], open: Some(Selected::Area(area)) };
    run_command(&plugin, "count-objects", Input { selection: opened, ..input(&m) }, host.clone())
        .unwrap();
    assert_eq!(
        host.log.borrow().as_slice(),
        ["TAVERN: 1 placeables, 2 waypoints", "TAVERN: 1 placeables, 2 waypoints"]
    );
    // Nothing selected, no area open: it says so.
    let host = Rc::new(TestHost::default());
    run_command(&plugin, "count-objects", input(&m), host.clone()).unwrap();
    assert_eq!(
        host.asked.borrow().as_slice(),
        [Question::Message("Right-click an area, or open one.".into())]
    );
}

/// With the game's data (where there is an install): class names come
/// from `classes.2da` and the talk table.
#[test]
fn creature_report_names_the_classes() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = std::sync::Arc::new(mg_rules::GameData::open(&install).unwrap());
    let host = Rc::new(TestHost::default());
    let input = Input::new(module(), Some(game));
    let outcome =
        run_command(&example("creature-report"), "list-classes", input, host.clone()).unwrap();
    assert!(outcome.edits.is_empty());
    assert_eq!(
        host.log.borrow().as_slice(),
        [
            "captain.utc (CAPTAIN): Paladin 3",
            "guard.utc (gate_guard): Fighter 3",
            "2 creature blueprints"
        ]
    );
}

/// The reference lists every name of the API, as the code builds it.
#[test]
fn the_reference_names_the_whole_api() {
    let names = api_names();
    assert!(names.len() > 40, "{names:?}");
    let reference = std::fs::read_to_string(docs().join("manual/17-plugin-api.md")).unwrap();
    let missing: Vec<&String> =
        names.iter().filter(|n| !reference.contains(&format!("`{n}"))).collect();
    assert!(missing.is_empty(), "not in docs/manual/17-plugin-api.md: {missing:?}");
    // And says which API it describes.
    assert!(reference.contains(&format!("API {}", mg_plugin::API)));
    let changes = std::fs::read_to_string(docs().join("plugins/CHANGES.md")).unwrap();
    assert!(changes.contains(&format!("## {}", mg_plugin::API)));
}

/// The block of a type in the type file: from `export type NAME =` to the
/// brace that closes it.
fn type_block<'a>(types: &'a str, name: &str) -> &'a str {
    let start = types
        .find(&format!("export type {name} ="))
        .unwrap_or_else(|| panic!("the type file has no type {name}"));
    let end = types[start..].find("\n}").map_or(types.len(), |e| start + e);
    &types[start..end]
}

/// The editor's type file is Luau that loads, and has every name too.
#[test]
fn the_type_file_has_the_whole_api() {
    let file = docs().join("plugins/types/moonglow.luau");
    let types = std::fs::read_to_string(&file).unwrap();
    let lua = mlua::Lua::new();
    let mg: mlua::Table = lua.load(types.as_str()).set_name("@moonglow.luau").eval().unwrap();
    assert_eq!(mg.get::<String>("api").unwrap(), mg_plugin::API);

    let mut missing = Vec::new();
    for name in api_names() {
        let has = match name.rsplit_once(['.', ':']) {
            Some(("mg", member)) => mg.contains_key(member).unwrap(),
            Some(("twoda", member)) => type_block(&types, "TwoDa").contains(&format!(" {member}:")),
            Some(("tlk", member)) => type_block(&types, "Tlk").contains(&format!(" {member}:")),
            Some((ctx, member)) => {
                // `ctx.module:gff` is `gff` of the type of `ctx.module`.
                let of = match ctx {
                    "ctx.module" => "Reader",
                    "ctx.game" => "Game",
                    "ctx.edit" => "Edit",
                    "ctx.log" => "Log",
                    "ctx.ui" => "Ui",
                    "ctx.hak" => "Hak",
                    "ctx.terrain" => "Terrain",
                    "area" => "Area",
                    "file" => "File",
                    "folder" => "Folder",
                    "image" => "Image",
                    "selected" => "Selected",
                    "fix" => "Fix",
                    "ctx.settings" => "Settings",
                    "ctx.event" => "Event",
                    "ctx" | "ctx.plugin" => "Context",
                    other => panic!("no type for {other}"),
                };
                let field = format!(" {member}:");
                type_block(&types, of).contains(&field)
                    || (of == "Game" && type_block(&types, "Reader").contains(&field))
            }
            // (Luau's own: require, print.)
            None => true,
        };
        if !has {
            missing.push(name);
        }
    }
    assert!(missing.is_empty(), "not in {}: {missing:?}", file.display());
}
