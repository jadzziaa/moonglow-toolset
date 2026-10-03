//! Plugins run on a module made here: what a command hands back, what a
//! check finds, and what the sandbox keeps a plugin from.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;

use mg_core::{LocString, LocStringKey, StrRef};
use mg_edit::{Command, Edit, GffPath, Workspace};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_module::doctor::Severity;
use mg_plugin::{
    Answer, FieldKind, Host, Input, Level, Plugin, PluginError, Question, discover, inspect,
    run_check, run_command,
};
use mg_resman::ResKey;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn plugin(name: &str) -> Plugin {
    Plugin::load(&fixtures().join(name)).unwrap()
}

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

/// A module with two creatures and two scripts.
fn module() -> Module {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for (name, tag) in [("guard", "gate_guard"), ("captain", "CAPTAIN")] {
        let mut g = Gff::new(*b"UTC ");
        g.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
        g.root.set("FirstName", Value::LocString(LocString::from_strref(StrRef(77))));
        g.root.set("Comment", Value::String(b"draft".to_vec()));
        let mut class = Struct::new(2);
        class.set("Class", Value::Int(4));
        class.set("ClassLevel", Value::Short(3));
        g.root.set("ClassList", Value::List(vec![class]));
        m.set_gff(key(&format!("{name}.utc")), &g).unwrap();
    }
    m.set(key("old.nss"), b"void main() { }".to_vec());
    m
}

/// A host that keeps the log, answers from a list, and may call the job
/// off after so many looks.
#[derive(Default)]
struct TestHost {
    log: RefCell<Vec<(Level, String)>>,
    answers: RefCell<VecDeque<Option<Answer>>>,
    asked: RefCell<Vec<Question>>,
    /// Looks at `cancelled` left before it says yes (`None`: never).
    cancel_after: Cell<Option<u32>>,
}

impl Host for TestHost {
    fn log(&self, level: Level, text: &str) {
        self.log.borrow_mut().push((level, text.to_string()));
    }

    fn cancelled(&self) -> bool {
        match self.cancel_after.get() {
            None => false,
            Some(0) => true,
            Some(n) => {
                self.cancel_after.set(Some(n - 1));
                false
            }
        }
    }

    fn ask(&self, question: &Question) -> Option<Answer> {
        self.asked.borrow_mut().push(question.clone());
        self.answers.borrow_mut().pop_front().flatten()
    }
}

fn run(
    plugin: &Plugin,
    command: &str,
    host: &Rc<TestHost>,
) -> Result<mg_plugin::Outcome, PluginError> {
    run_command(plugin, command, Input { module: module(), game: None }, host.clone())
}

/// The module with a command's edits applied, as the host applies them.
fn applied(edits: Vec<Edit>) -> Workspace {
    let mut ws = Workspace::new(module());
    ws.apply(Command::new("plugin", edits)).unwrap();
    ws
}

#[test]
fn plugins_are_found_by_their_manifests() {
    let found = discover(&fixtures());
    let ids: Vec<String> = found
        .iter()
        .map(|p| p.as_ref().map(|p| p.manifest.id.clone()).unwrap_or_else(|e| e.to_string()))
        .collect();
    assert_eq!(ids, ["test.broken", "test.forms", "test.hostile", "example.tag-conventions"]);
    let p = plugin("tag-conventions");
    assert_eq!(p.manifest.commands.len(), 2);
    assert_eq!(p.check_id("tag-case"), "example.tag-conventions/tag-case");
    // A folder without its entry script is no plugin.
    let dir = mg_testkit::scratch_dir("plugin-no-entry");
    std::fs::write(
        dir.join("plugin.cfg"),
        std::fs::read_to_string(fixtures().join("forms/plugin.cfg")).unwrap(),
    )
    .unwrap();
    let e = Plugin::load(&dir).unwrap_err().to_string();
    assert!(e.contains("its entry main.luau is not in the folder"), "{e}");
}

#[test]
fn a_command_hands_back_its_edits() {
    let host = Rc::new(TestHost::default());
    let p = plugin("tag-conventions");
    let outcome = run(&p, "fix-tags", &host).unwrap();
    // One creature's tag was not upper case: one edit, the field's type kept.
    assert_eq!(outcome.label, "Upper-case creature tags");
    assert_eq!(
        outcome.edits,
        [Edit::SetField {
            key: key("guard.utc"),
            path: GffPath::root(),
            label: "Tag".into(),
            value: Some(Value::String(b"GATE_GUARD".to_vec())),
        }]
    );
    assert_eq!(*host.log.borrow(), [(Level::Info, "1 creature tags changed".to_string())]);
    // Nothing was changed here: the edits are the host's to apply.
    let e = run(&p, "no-such", &host).unwrap_err();
    assert_eq!(e.to_string(), "example.tag-conventions has no command \"no-such\"");
}

#[test]
fn every_kind_of_edit_is_one_the_editors_make() {
    let host = Rc::new(TestHost::default());
    let outcome = run(&plugin("tag-conventions"), "promote", &host).unwrap();
    assert_eq!(outcome.label, "Promote the Guards", "its title, when it names nothing");
    let mut ws = applied(outcome.edits);
    for name in ["guard.utc", "captain.utc"] {
        let g = ws.doc(&key(name)).unwrap().root.clone();
        let classes = g.list("ClassList").unwrap();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].get("Class"), Some(&Value::Int(7)));
        assert_eq!(classes[0].get("ClassLevel"), Some(&Value::Short(1)));
        assert_eq!(classes[0].id, 2);
        // The name keeps its talk-table reference and takes the text as
        // its English; fields the creature lacked take the game's type,
        // the one given, or the typed value's.
        let mut name_ = LocString::from_strref(StrRef(77));
        name_.strings.push((LocStringKey(0), b"Captain".to_vec()));
        assert_eq!(g.get("FirstName"), Some(&Value::LocString(name_)));
        assert_eq!(g.get("Wis"), Some(&Value::Byte(14)));
        assert_eq!(g.get("Rank"), Some(&Value::Byte(3)));
        let mut motto = LocString::default();
        motto.strings.push((LocStringKey(0), b"Hold the gate".to_vec()));
        assert_eq!(g.get("Motto"), Some(&Value::LocString(motto)));
        assert_eq!(g.get("Comment"), None);
    }
    ws.flush().unwrap();
    assert_eq!(ws.module.get(&key("promoted.nss")), Some(&b"// promoted\nvoid main() { }\n"[..]));
    assert!(!ws.module.contains(&key("old.nss")));
}

#[test]
fn a_check_finds_and_does_not_edit() {
    let host = Rc::new(TestHost::default());
    let p = plugin("tag-conventions");
    let input = || Input { module: module(), game: None };
    let found = run_check(&p, "tag-case", input(), host.clone()).unwrap();
    assert_eq!(found.len(), 1);
    let f = &found[0];
    assert_eq!(f.check, "example.tag-conventions/tag-case");
    assert_eq!(
        (f.severity, f.resource, f.at.as_str()),
        (Severity::Warning, key("guard.utc"), "Tag")
    );
    assert_eq!(f.message, "tag \"gate_guard\" is not upper case");
    // A check that edits is refused.
    let e = run_check(&plugin("hostile"), "edits", input(), host).unwrap_err().to_string();
    assert!(e.contains("a check only reads"), "{e}");
}

#[test]
fn the_sandbox_has_no_way_out() {
    let host = Rc::new(TestHost::default());
    let p = plugin("hostile");
    // No file, process or loader functions; no file outside the plugin's
    // folder; the built-in functions cannot be replaced.
    let outcome = run(&p, "reach", &host).unwrap();
    assert_eq!(outcome.label, "", "it reached: {}", outcome.label);
    assert!(outcome.edits.is_empty());
}

#[test]
fn a_job_called_off_stops_whatever_it_catches() {
    let host = Rc::new(TestHost::default());
    host.cancel_after.set(Some(10_000));
    let started = std::time::Instant::now();
    let e = run(&plugin("hostile"), "spin", &host).unwrap_err();
    assert_eq!(e, PluginError::Canceled("Hostile".into()));
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
    // Also from inside a long pattern match, where none of its code runs.
    let host = Rc::new(TestHost::default());
    host.cancel_after.set(Some(200));
    let started = std::time::Instant::now();
    let e = run(&plugin("hostile"), "match", &host).unwrap_err();
    assert_eq!(e, PluginError::Canceled("Hostile".into()));
    assert!(started.elapsed() < std::time::Duration::from_secs(20), "{:?}", started.elapsed());
}

#[test]
fn memory_has_a_limit() {
    let host = Rc::new(TestHost::default());
    let e = run(&plugin("hostile"), "hoard", &host).unwrap_err().to_string();
    assert!(e.contains("ran out of the 256 MB"), "{e}");
}

#[test]
fn a_bad_edit_fails_where_it_is_made() {
    let host = Rc::new(TestHost::default());
    let p = plugin("hostile");
    let outcome = run(&p, "bad-edits", &host).unwrap();
    assert!(outcome.edits.is_empty(), "{:?}", outcome.edits);
    let log: Vec<String> = host.log.borrow().iter().map(|(_, m)| m.clone()).collect();
    let said = |what: &str, why: &str| {
        log.iter().any(|l| l.starts_with(&format!("{what}: ")) && l.contains(why))
    };
    assert!(said("no resource", "nobody.utc is not in the module"), "{log:#?}");
    assert!(said("no name", "is not a resource's name"), "{log:#?}");
    assert!(said("no path", "no struct at /ClassList[9]"), "{log:#?}");
    assert!(said("too big", "70000 does not fit a short"), "{log:#?}");
    assert!(said("not a number", "a short takes a whole number"), "{log:#?}");
    assert!(said("no type", "give its type"), "{log:#?}");
    assert!(said("a list", "is not set as a value"), "{log:#?}");
    assert!(said("no item", "no item 4"), "{log:#?}");
    assert!(said("no field", "has no field Nothing"), "{log:#?}");
    assert!(said("not there", "nobody.nss is not in the module"), "{log:#?}");
    assert_eq!(log.len(), 10);
    // Code that fails hands back nothing, and says where it failed.
    let e = run(&p, "fails", &host).unwrap_err().to_string();
    assert!(e.contains("something went wrong") && e.contains("main.luau"), "{e}");
}

#[test]
fn questions_go_to_the_host() {
    let p = plugin("forms");
    // Nobody to ask: the form is canceled, nothing changes.
    let host = Rc::new(TestHost::default());
    let outcome = run(&p, "rename", &host).unwrap();
    assert!(outcome.edits.is_empty());
    assert_eq!(host.log.borrow()[0].1, "canceled");
    // The form as the plugin described it.
    let asked = host.asked.borrow();
    let Question::Form { title, fields } = &asked[0] else { panic!("{asked:?}") };
    assert_eq!(title, "Prefix Tags");
    let ids: Vec<&str> = fields.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["prefix", "count", "upper", "kind"]);
    assert_eq!(fields[0].kind, FieldKind::Text { default: "NPC_".into() });
    assert_eq!(
        fields[1].kind,
        FieldKind::Number { default: 10.0, min: Some(1.0), max: Some(100.0) }
    );
    assert_eq!(fields[2].kind, FieldKind::Check { default: true });
    assert_eq!(
        fields[3].kind,
        FieldKind::Choice { choices: vec!["creatures".into(), "doors".into()], default: 0 }
    );

    // Answered, then declined: still nothing.
    let values = BTreeMap::from([
        ("prefix".to_string(), serde_json::json!("NPC_")),
        ("count".to_string(), serde_json::json!(5)),
        ("upper".to_string(), serde_json::json!(true)),
        ("kind".to_string(), serde_json::json!("creatures")),
    ]);
    let host = Rc::new(TestHost::default());
    host.answers.borrow_mut().extend([Some(Answer::Values(values.clone())), Some(Answer::No)]);
    assert!(run(&p, "rename", &host).unwrap().edits.is_empty());
    assert_eq!(host.asked.borrow()[1], Question::Confirm("Prefix up to 5 creatures?".into()));

    // Answered and confirmed: the edits, a message, and print in the log.
    let host = Rc::new(TestHost::default());
    host.answers.borrow_mut().extend([Some(Answer::Values(values)), Some(Answer::Yes)]);
    let outcome = run(&p, "rename", &host).unwrap();
    let mut ws = applied(outcome.edits);
    let tag = |ws: &mut Workspace, name: &str| {
        String::from_utf8(ws.doc(&key(name)).unwrap().root.string("Tag").unwrap().to_vec()).unwrap()
    };
    assert_eq!(tag(&mut ws, "guard.utc"), "NPC_GATE_GUARD");
    assert_eq!(tag(&mut ws, "captain.utc"), "NPC_CAPTAIN");
    assert_eq!(host.asked.borrow()[2], Question::Message("Done.".into()));
    assert_eq!(host.log.borrow()[0], (Level::Info, "prefixed\tNPC_\t5".to_string()));
}

#[test]
fn code_and_manifest_must_agree() {
    let host = Rc::new(TestHost::default());
    assert_eq!(inspect(&plugin("tag-conventions"), host.clone()).unwrap(), Vec::<String>::new());
    let faults = inspect(&plugin("broken"), host.clone()).unwrap();
    assert_eq!(
        faults,
        [
            "the command \"declared\" is declared, but the code registers none",
            "the code registers the command \"undeclared\", which the manifest lacks",
            "the check \"also-declared\" is declared, but the code registers none",
        ]
    );
    let e = run(&plugin("broken"), "declared", &host).unwrap_err().to_string();
    assert!(e.contains("the code registers none"), "{e}");
}

/// The game's own data, where there is an install: a 2DA's cells by row
/// and column, a talk-table string, a resource the game would load.
#[test]
fn the_game_s_data_is_read() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = std::sync::Arc::new(mg_rules::GameData::open(&install).unwrap());
    let dir = mg_testkit::scratch_dir("plugin-game");
    std::fs::write(
        dir.join("plugin.cfg"),
        "[plugin]\nid = \"test.game\"\nname = \"Game\"\nversion = \"1\"\napi = \"0.1\"\n\
         license = \"GPL-3.0-or-later\"\n[command]\nid = \"look\"\ntitle = \"Look\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.luau"),
        r#"
        local mg = require("@moonglow")
        mg.command("look", function(ctx)
            local classes = ctx.game:table("classes")
            local torch = ctx.game:gff("nw_it_torch001.uti")
            return table.concat({
                classes:get(6, "Label"),
                tostring(classes.rows > 10),
                ctx.game:string(12),
                tostring(ctx.game:has("nwscript.nss")),
                tostring(ctx.game:has("nosuchthing.nss")),
                torch.Tag,
                tostring(#ctx.game:resources("set") > 30),
            }, "|")
        end)
        "#,
    )
    .unwrap();
    let host = Rc::new(TestHost::default());
    let input = Input { module: module(), game: Some(game) };
    let outcome = run_command(&Plugin::load(&dir).unwrap(), "look", input, host).unwrap();
    assert_eq!(outcome.label, "Paladin|true|Paladin|true|false|NW_IT_TORCH001|true");
}
