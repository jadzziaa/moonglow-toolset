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

/// A module with two creatures and two merchants.
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
    Input { module: module.clone(), game: None }
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
            "example.creature-report",
            "example.hello",
            "example.merchant-markup",
            "example.tag-conventions"
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
    let outcome = run_command(&plugin, "stamp", input(&module()), host.clone()).unwrap();
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
    assert_eq!(found[0].resource, key("guard.utc"));
    assert_eq!(found[0].message, "tag \"gate_guard\" is not upper case");

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

#[test]
fn merchant_markup_sets_what_its_form_says() {
    let plugin = example("merchant-markup");
    let host = Rc::new(TestHost::default());
    let values = BTreeMap::from([
        ("sell".to_string(), serde_json::json!(200)),
        ("buy".to_string(), serde_json::json!(40)),
        ("only".to_string(), serde_json::json!("shop_")),
    ]);
    host.answers.borrow_mut().push_back(Some(Answer::Values(values)));
    let outcome = run_command(&plugin, "set-markup", input(&module()), host.clone()).unwrap();
    let Question::Form { title, fields } = &host.asked.borrow()[0] else { panic!("no form") };
    assert_eq!(title, "Markup of 2 merchants");
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
    assert_eq!(
        host.log.borrow().as_slice(),
        ["1 of 2 merchants set to sell at 200% and buy at 40%"]
    );

    // The form closed: nothing changes.
    let host = Rc::new(TestHost::default());
    let outcome = run_command(&plugin, "set-markup", input(&module()), host).unwrap();
    assert!(outcome.edits.is_empty());

    // No merchants: it says so, and asks nothing more.
    let mut bare = Module::new();
    bare.set_info(&Gff::new(*b"IFO ")).unwrap();
    let host = Rc::new(TestHost::default());
    let outcome = run_command(&plugin, "set-markup", input(&bare), host.clone()).unwrap();
    assert!(outcome.edits.is_empty());
    assert_eq!(
        host.asked.borrow().as_slice(),
        [Question::Message("The module has no merchant blueprints.".into())]
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
    let input = Input { module: module(), game: Some(game) };
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
            Some((ctx, member)) => {
                // `ctx.module:gff` is `gff` of the type of `ctx.module`.
                let of = match ctx {
                    "ctx.module" => "Reader",
                    "ctx.game" => "Game",
                    "ctx.edit" => "Edit",
                    "ctx.log" => "Log",
                    "ctx.ui" => "Ui",
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
