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
    run_command(plugin, command, Input::new(module(), None), host.clone())
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
    assert_eq!(
        ids,
        ["test.asks", "test.broken", "test.forms", "test.hostile", "example.tag-conventions"]
    );
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
    let input = || Input::new(module(), None);
    let found = run_check(&p, "tag-case", input(), host.clone()).unwrap();
    assert_eq!(found.len(), 1);
    let f = &found[0].finding;
    assert_eq!(f.check, "example.tag-conventions/tag-case");
    assert_eq!(
        (f.severity, f.resource, f.at.as_str()),
        (Severity::Warning, key("guard.utc"), "Tag")
    );
    assert_eq!(f.message, "tag \"gate_guard\" is not upper case");
    // Its fix: the edit recorded, none made.
    let fix = found[0].fix.as_ref().unwrap();
    assert_eq!(fix.label, "Upper-case this tag");
    let mut ws = Workspace::new(module());
    ws.apply(fix.command()).unwrap();
    let guard = ws.doc(&key("guard.utc")).unwrap();
    assert_eq!(guard.root.get("Tag"), Some(&Value::String(b"GATE_GUARD".to_vec())));
    // A check that edits is refused.
    let e = run_check(&plugin("hostile"), "edits", input(), host).unwrap_err().to_string();
    assert!(e.contains("a check only reads"), "{e}");
}

/// A fix records edits as `ctx.edit` makes them, each fix from the module
/// as the check reads it, which its fixes leave as it is; a fix that edits
/// nothing is none, and a fix is one made with `ctx.fix`.
#[test]
fn a_check_s_fixes_are_recorded_apart() {
    let dir = mg_testkit::scratch_dir("plugin-fixes");
    let manifest = "[plugin]\nid = \"test.fixes\"\nname = \"Fixes\"\nversion = \"1.0.0\"\napi = \"0.4\"\nlicense = \"MIT\"\ndescription = \"d\"\n\n[check]\nid = \"many\"\ntitle = \"Many\"\n\n[check]\nid = \"bad\"\ntitle = \"Bad\"\n";
    std::fs::write(dir.join("plugin.cfg"), manifest).unwrap();
    let code = r#"
        local mg = require("@moonglow")
        mg.check("many", function(ctx)
            local one = ctx.fix("Rename")
            one:set("guard.utc", "Comment", "Bob")
            local at = one:insert("guard.utc", "/ClassList", { __struct_id = 2, Class = mg.int(1) })
            one:set("guard.utc", `/ClassList[{at}]/ClassLevel`, mg.short(2))
            -- The check reads the module as it is.
            assert(ctx.module:gff("guard.utc").Comment == "draft")
            local two = ctx.fix("Another")
            two:set("guard.utc", "Tag", "TWO")
            assert(two.label == "Another")
            local nothing = ctx.fix("Nothing")
            return {
                { resource = "guard.utc", message = "one", fix = one },
                { resource = "guard.utc", message = "two", fix = two },
                { resource = "guard.utc", message = "none", fix = nothing },
            }
        end)
        mg.check("bad", function(ctx)
            return { { resource = "guard.utc", message = "x", fix = { label = "Mine" } } }
        end)
    "#;
    std::fs::write(dir.join("main.luau"), code).unwrap();
    let p = Plugin::load(&dir).unwrap();
    let host = Rc::new(TestHost::default());
    let found = run_check(&p, "many", Input::new(module(), None), host.clone()).unwrap();
    let one = found[0].fix.as_ref().unwrap();
    assert_eq!((one.label.as_str(), one.edits.len()), ("Rename", 3));
    let two = found[1].fix.as_ref().unwrap();
    assert_eq!(two.edits.len(), 1, "each fix from the module as it is: {two:?}");
    assert_eq!(found[2].fix, None);
    let mut ws = Workspace::new(module());
    ws.apply(one.command()).unwrap();
    assert_eq!(ws.can_undo(), Some("Rename"));
    let e = run_check(&p, "bad", Input::new(module(), None), host).unwrap_err().to_string();
    assert!(e.contains("made with ctx.fix(label)"), "{e}");
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
            "the event \"never\" is declared, but the code registers none",
        ]
    );
    let e = run(&plugin("broken"), "declared", &host).unwrap_err().to_string();
    assert!(e.contains("the code registers none"), "{e}");
}

#[test]
fn the_console_runs_what_is_typed() {
    use mg_plugin::run_console;
    let host = Rc::new(TestHost::default());
    let input = || Input::new(module(), None);
    // What it returns is shown; its edits are a command's.
    let code = r#"
        ctx.edit:set("guard.utc", "Tag", "FROM_CONSOLE")
        return ctx.module:resources("utc"), ctx.module:gff("guard.utc").Tag, 2 + 2
    "#;
    let outcome = run_console(code, input(), host.clone()).unwrap();
    assert_eq!(outcome.edits.len(), 1);
    let log: Vec<String> = host.log.borrow().iter().map(|(_, m)| m.clone()).collect();
    assert_eq!(log, ["{\"captain.utc\", \"guard.utc\"}", "\"FROM_CONSOLE\"", "4"]);
    // A fault says where; no file can be required.
    let e = run_console("local x = nil\nreturn x.y", input(), host.clone()).unwrap_err();
    assert!(e.to_string().contains("console:4"), "{e}");
    let e = run_console("return require('rules')", input(), host).unwrap_err().to_string();
    assert!(e.contains("there are no files to require here"), "{e}");
}

/// Values written by hand are read as meant: an empty table is a list
/// where one is said to be (set as a value, in an item added, in a whole
/// GFF), a localized string is made from either shape of one, and the
/// root's path is empty.
#[test]
fn values_written_by_hand_are_read_as_meant() {
    let dir = mg_testkit::scratch_dir("plugin-empty-list");
    std::fs::write(
        dir.join("plugin.cfg"),
        "[plugin]\nid = \"test.lists\"\nname = \"Lists\"\nversion = \"1\"\napi = \"0.1\"\n\
         license = \"GPL-3.0-or-later\"\n[command]\nid = \"empty\"\ntitle = \"Empty\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.luau"),
        r#"
        local mg = require("@moonglow")
        mg.command("empty", function(ctx)
            ctx.edit:set("guard.utc", "FeatList", { type = "list", value = {} })
            ctx.edit:insert("guard.utc", "/ClassList", {
                __struct_id = 2,
                Class = mg.int(1),
                KnownList0 = { type = "list", value = {} },
            })
            local copy = ctx.module:raw("guard.utc")
            copy.SkillList = { type = "list", value = {} }
            -- A localized string with its type, from either shape.
            copy.LastName = mg.locstring({ [0] = "Stone", [4] = "Stein", strref = 9 })
            copy.Description = mg.locstring(ctx.module:raw("guard.utc").FirstName.value)
            ctx.edit:write_gff("copy.utc", copy)
            -- The root's path is empty: a field's is that and its label.
            local guard = ctx.module:gff("guard.utc")
            assert(mg.path(guard) == "" and mg.path(guard.ClassList[1]) == "/ClassList[0]")
            ctx.edit:set("guard.utc", mg.path(guard) .. "/Tag", "ROOT")
            assert(ctx.module:raw("guard.utc", mg.path(guard)).Tag.value == "ROOT")
            assert(ctx.module:raw("guard.utc", "/ClassList[0]").Class.value == 4)
            -- A fault says which field.
            local ok, err = pcall(function()
                ctx.edit:set("guard.utc", "FeatList", { type = "list", value = 3 })
            end)
            return tostring(err)
        end)
        "#,
    )
    .unwrap();
    let host = Rc::new(TestHost::default());
    let outcome = run(&Plugin::load(&dir).unwrap(), "empty", &host).unwrap();
    assert!(outcome.label.contains("guard.utc FeatList: expected an array"), "{}", outcome.label);
    let mut ws = applied(outcome.edits);
    ws.flush().unwrap();
    let guard = ws.module.gff(&key("guard.utc")).unwrap().unwrap();
    assert_eq!(guard.root.get("FeatList"), Some(&Value::List(Vec::new())));
    let classes = guard.root.list("ClassList").unwrap();
    assert_eq!(classes[1].get("KnownList0"), Some(&Value::List(Vec::new())));
    let copy = ws.module.gff(&key("copy.utc")).unwrap().unwrap();
    assert_eq!(copy.root.get("SkillList"), Some(&Value::List(Vec::new())));
    assert_eq!(copy.root.list("ClassList").unwrap().len(), 2);
    let mut name = LocString::from_strref(StrRef(9));
    name.strings = vec![(LocStringKey(0), b"Stone".to_vec()), (LocStringKey(4), b"Stein".to_vec())];
    assert_eq!(copy.root.get("LastName"), Some(&Value::LocString(name)));
    assert_eq!(
        copy.root.get("Description"),
        Some(&Value::LocString(LocString::from_strref(StrRef(77))))
    );
    assert_eq!(guard.root.get("Tag"), Some(&Value::String(b"ROOT".to_vec())));
}

/// A struct read with `gff` has its id beside it (`mg.struct_id`), not
/// among its fields: a field may have any label, `__struct_id` too, and
/// a table built from one goes into an edit with the fields it has.
#[test]
fn a_struct_s_id_is_read_beside_its_fields() {
    let dir = mg_testkit::scratch_dir("plugin-struct-id");
    std::fs::write(
        dir.join("plugin.cfg"),
        "[plugin]\nid = \"test.ids\"\nname = \"Ids\"\nversion = \"1\"\napi = \"0.4\"\n\
         license = \"GPL-3.0-or-later\"\n[command]\nid = \"ids\"\ntitle = \"Ids\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.luau"),
        r#"
        local mg = require("@moonglow")
        mg.command("ids", function(ctx)
            local guard = ctx.module:gff("guard.utc")
            local class = guard.ClassList[1]
            assert(mg.struct_id(class) == 2, "a list's item")
            assert(mg.struct_id(guard) == 4294967295, "the root")
            local labels = {}
            for label in class do
                table.insert(labels, label)
            end
            table.sort(labels)
            assert(table.concat(labels, ",") == "Class,ClassLevel", table.concat(labels, ","))
            assert(mg.struct_id(ctx.module:raw("guard.utc", "/ClassList[0]")) == 2, "raw")
            assert(mg.struct_id({}) == nil and mg.struct_id({ Class = 4 }) == nil)
            -- A field so labeled is a field, and the id still the struct's.
            ctx.edit:set("guard.utc", mg.path(class) .. "/__struct_id", mg.int(7))
            local again = ctx.module:gff("guard.utc").ClassList[1]
            assert(again.__struct_id == 7 and mg.struct_id(again) == 2)
            -- A copy of it, as an item of its id with its fields.
            ctx.edit:insert("guard.utc", "/ClassList", {
                __struct_id = mg.struct_id(class),
                Class = mg.int(class.Class + 1),
                ClassLevel = mg.short(class.ClassLevel),
            })
            ctx.edit:set("guard.utc", mg.path(class) .. "/ClassLevel", class.ClassLevel + 1)
        end)
        "#,
    )
    .unwrap();
    let host = Rc::new(TestHost::default());
    let outcome = run(&Plugin::load(&dir).unwrap(), "ids", &host).unwrap();
    let mut ws = applied(outcome.edits);
    ws.flush().unwrap();
    let guard = ws.module.gff(&key("guard.utc")).unwrap().unwrap();
    let classes = guard.root.list("ClassList").unwrap();
    assert_eq!(classes.len(), 2);
    assert_eq!(classes[0].get("ClassLevel"), Some(&Value::Short(4)));
    let copy = &classes[1];
    assert_eq!(copy.id, 2);
    let labels: Vec<String> = copy.fields.iter().map(|f| f.label.to_string_lossy()).collect();
    assert_eq!(labels, ["Class", "ClassLevel"], "no field for the id");
    assert_eq!(copy.get("Class"), Some(&Value::Int(5)));
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
    let input = Input::new(module(), Some(game));
    let outcome = run_command(&Plugin::load(&dir).unwrap(), "look", input, host).unwrap();
    assert_eq!(outcome.label, "Paladin|true|Paladin|true|false|NW_IT_TORCH001|true");
}

/// An overridden copy open in the window (the resource browser's) is
/// `ctx.open` with its layer, and `ctx.game` reads that layer's copy, not
/// the one the game loads.
#[test]
fn a_layer_s_copy_is_open_and_read() {
    use mg_plugin::{Selected, Selection, run_console};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = std::sync::Arc::new(mg_rules::GameData::open(&install).unwrap());
    let rm = &game.resman;
    // A blueprint of which a lower layer has another copy.
    let (copy, layers) = rm
        .every_copy()
        .into_iter()
        .find(|(k, layers)| {
            k.restype.is_gff()
                && layers.len() > 1
                && rm.get_in(layers[0], k).ok() != rm.get_in(layers[1], k).ok()
        })
        .expect("a resource the game has two copies of");
    let label = rm.layers()[layers[1]].label.clone();
    let lower = rm.get_in(layers[1], &copy).unwrap().into_owned();
    let open = Selected::Copy { key: copy, layer: label.clone() };
    assert_eq!(Selected::parse(&format!("{copy}@{label}")).unwrap(), open);
    assert_eq!(open.target(), None);
    let code = r#"
        local o = ctx.open
        local mine = ctx.game:bytes(o.resource, o.layer)
        local loaded = ctx.game:bytes(o.resource)
        local gff = ctx.game:gff(o.resource, o.layer)
        return o.kind, o.layer, tostring(o.module), #mine, mine ~= loaded, type(gff),
            tostring(select(2, pcall(function() return ctx.game:bytes(o.resource, "no such layer") end))),
            tostring(select(2, pcall(function() return ctx.module:bytes(o.resource, o.layer) end)))
    "#;
    let host = Rc::new(TestHost::default());
    let selection = Selection { selected: Vec::new(), open: Some(open) };
    let input = Input { selection, ..Input::new(module(), Some(game.clone())) };
    run_console(code, input, host.clone()).unwrap();
    let said: Vec<String> = host.log.borrow().iter().map(|(_, t)| t.clone()).collect();
    // (Strings said as written, quoted.)
    let ext = format!("{:?}", copy.restype.extension().unwrap());
    let expected = [ext, format!("{label:?}"), "\"false\"".into(), lower.len().to_string()];
    assert_eq!(said[..4], expected);
    assert_eq!(said[4..6], ["true", "\"table\""], "the layer's copy, not the one loaded");
    assert!(said[6].contains("is not a layer of the game's load order"), "{}", said[6]);
    assert!(said[7].contains("ctx.module has no layers"), "{}", said[7]);
}

/// 2DAs and talk tables are made, changed and written as values
/// (`mg.twoda`, `mg.tlk`), not as text put together by hand.
#[test]
fn tables_are_made_and_written() {
    use mg_plugin::run_console;
    let host = Rc::new(TestHost::default());
    let input = || Input::new(module(), None);
    let code = r#"
        local mg = require("@moonglow")
        local t = mg.twoda.parse("2DA V2.0\r\n\r\n   Label  Name\r\n0  a      10\r\n1  \"b c\"  ****\r\n")
        t:set(1, "name", 11)
        t:add_column("Cost")
        local row = t:add_row({ Label = "d", Cost = 2.5 })
        t:set(0, "Cost", nil)
        local again = mg.twoda.parse(t:write())
        local new = mg.twoda.new({ "Label" })
        new:add_row({ Label = "only" })
        return row, t.rows, #t.columns, again:get(1, "Name"), again:get(1, "Label"),
            again:get(2, "Cost"), again:get(2, "Name"), new:write(),
            mg.twoda.kinds("feat").FEAT, mg.twoda.kinds("placeables").ModelName,
            mg.twoda.kinds("feat").MASTERFEAT, next(mg.twoda.kinds("nothing")),
            mg.twoda.kinds("spells").ImpactScript, mg.twoda.kinds("vfx_persistent").ONENTER
    "#;
    run_console(code, input(), host.clone()).unwrap();
    let log: Vec<String> = host.log.borrow().iter().map(|(_, m)| m.clone()).collect();
    assert_eq!(
        log,
        [
            "2",
            "3",
            "3",
            "\"11\"",
            "\"b c\"",
            "\"2.5\"",
            "nil",
            "\"2DA V2.0\\r\\n\\r\\n   Label\\r\\n0  only\\r\\n\"",
            "\"strref\"",
            "\"model\"",
            "\"row:masterfeats\"",
            "nil",
            "\"script\"",
            "\"script\""
        ]
    );
    let e = run_console(
        "local mg = require('@moonglow') mg.twoda.new({'A'}):set(0, 'A', 1)",
        input(),
        host.clone(),
    );
    assert!(e.unwrap_err().to_string().contains("no row 0"));

    host.log.borrow_mut().clear();
    let code = r#"
        local mg = require("@moonglow")
        local t = mg.tlk.new()
        local first = t:add("Cleave")
        t:set(3, { text = "Hide", sound = "VS_Hide", sound_length = 1.5 })
        local again = mg.tlk.parse(t:write())
        return first, again.lines, again:get(0).text, again:get(1), again:get(3).sound,
            again:get(3).sound_length
    "#;
    run_console(code, input(), host.clone()).unwrap();
    let log: Vec<String> = host.log.borrow().iter().map(|(_, m)| m.clone()).collect();
    assert_eq!(log, ["0", "4", "\"Cleave\"", "nil", "\"vs_hide\"", "1.5"]);
    // A sound is named by the rule the talk-table editor and `mg tlk`
    // hold one to (`ResRef::new_name`): one they refuse, a plugin can't
    // write either.
    let e = run_console(
        "local mg = require('@moonglow') mg.tlk.new():set(0, { text = 'x', sound = 'a b.wav' })",
        input(),
        host.clone(),
    );
    let said = e.unwrap_err().to_string();
    assert!(said.contains("a b.wav: a sound's name: ' '"), "{said}");
}

/// What the tables for plugins did not do (the deferred list's "Tables
/// for plugins"): `kinds` knows a class's requirements, the tables a
/// column names and the item properties' (and a table by the column that
/// names it, with the game's data); a table is written as the file it
/// was read from; a talk table has its language.
#[test]
fn tables_know_more_kinds_keep_their_layout_and_have_a_language() {
    use mg_plugin::run_console;
    let host = Rc::new(TestHost::default());
    let mut rm = mg_resman::ResMan::new();
    let mut base = mg_resman::MemContainer::new();
    base.insert(
        key("itempropdef.2da"),
        b"2DA V2.0\n\n   Name Label SubTypeResRef\n0 649 Ability my_abilities\n".to_vec(),
    );
    rm.add(mg_resman::priority::KEY, "base", mg_resman::LayerClass::Key, base);
    let game = std::sync::Arc::new(mg_rules::GameData::new(
        rm,
        mg_tlk::Tlk::new(mg_core::Language::ENGLISH),
    ));
    let code = r#"
        local mg = require("@moonglow")
        local pres = mg.twoda.kinds("cls_pres_wm").ReqParam1
        local wide = "2DA V2.0\n\n      Label        Cost\n0     Sword        10\n1     Axe          5\n"
        local t = mg.twoda.parse(wide)
        local same = t:write() == wide
        t:set(1, "Cost", 7)
        return mg.twoda.kinds("classes").FeatsTable, pres.by, pres.rows.FEAT, pres.rows.SKILL,
            mg.twoda.kinds("itempropdef").CostTableResRef, mg.twoda.kinds("my_abilities").Name,
            same, t:write()
    "#;
    run_console(code, Input::new(module(), Some(game.clone())), host.clone()).unwrap();
    let log: Vec<String> = host.log.borrow().iter().map(|(_, m)| m.clone()).collect();
    assert_eq!(
        log,
        [
            "\"table\"",
            "\"ReqType\"",
            "\"feat\"",
            "\"skills\"",
            "\"row:iprp_costtable\"",
            "\"strref\"",
            "true",
            "\"2DA V2.0\\n\\n      Label        Cost\\n0     Sword        10\\n1     Axe          7\\n\"",
        ]
    );
    // Without the game's data, a table is known by its name alone.
    host.log.borrow_mut().clear();
    let code = r#"
        local mg = require("@moonglow")
        return mg.twoda.kinds("my_abilities").Name
    "#;
    run_console(code, Input::new(module(), None), host.clone()).unwrap();
    assert_eq!(host.log.borrow()[0].1, "nil");

    // A talk table in Polish, its text in the language's codepage.
    host.log.borrow_mut().clear();
    let code = r#"
        local mg = require("@moonglow")
        local t = mg.tlk.new("pl")
        t:add("Łódź")
        local again = mg.tlk.parse(t:write())
        local english = mg.tlk.new()
        local ok = pcall(function() english:add("Łódź") end)
        english:set_language("fr")
        local bad = pcall(function() mg.tlk.new("xx") end)
        return again.language, again:get(0).text, english.language, ok, bad
    "#;
    run_console(code, Input::new(module(), None), host.clone()).unwrap();
    let log: Vec<String> = host.log.borrow().iter().map(|(_, m)| m.clone()).collect();
    assert_eq!(log, ["\"pl\"", "\"Łódź\"", "\"fr\"", "false", "false"]);
}

/// `mg.twoda.describe`: a table's description read, a value named, and a
/// table held to it.
#[test]
fn a_plugin_reads_a_table_s_description() {
    use mg_plugin::run_console;
    let host = Rc::new(TestHost::default());
    let input = || Input::new(module(), None);
    let code = r#"
        local mg = require("@moonglow")
        local d = mg.twoda.describe([[
rows = "places"
[columns.Chance]
kind = "int"
max = 100
[columns.Kinds]
kind = "flags"
constants = "loot_inc"
prefix = "KIND_"
[columns.Place]
kind = "row"
table = "places"
]], { loot_inc = "const int KIND_GOLD = 0x01;\nconst int KIND_GEMS = 0x02;" })
        local t = mg.twoda.parse("2DA V2.0\r\n\r\n  Label Chance Kinds Place\r\n0 a 50 0x03 1\r\n1 b 500 0x04 9\r\n")
        local faults = d:faults(t, { places = 3 })
        local said = {}
        for _, f in faults do table.insert(said, f.row .. " " .. f.column .. " " .. tostring(f.value)) end
        local ok, err = pcall(mg.twoda.describe, "[columns.A]\nkind = \"odd\"")
        return d.rows, d.columns.Chance, d.columns.Kinds, d.columns.Place,
            d:name("Kinds", "0x03"), d:name("Kinds", 2), tostring(d:name("Chance", 5)),
            table.concat(said, "; "), #d:faults(t), ok, tostring(err):find("no kind") ~= nil
    "#;
    run_console(code, input(), host.clone()).unwrap();
    let log: Vec<String> = host.log.borrow().iter().map(|(_, m)| m.clone()).collect();
    assert_eq!(
        log,
        [
            "\"places\"",
            "\"int\"",
            "\"flags\"",
            "\"row:places\"",
            "\"KIND_GOLD | KIND_GEMS\"",
            "\"KIND_GEMS\"",
            "\"nil\"",
            "\"1 Chance 500; 1 Kinds 0x04; 1 Place 9\"",
            // (Without the other table's length, its rows are not held to.)
            "2",
            "false",
            "true",
        ]
    );
}

/// API 0.4: what is selected and open, as a plugin reads it. Each entry
/// says what it is and where (the resource and path `ctx.edit` takes),
/// with its tag and resref from the job's copy of the module.
#[test]
fn a_plugin_reads_what_is_selected_and_open() {
    use mg_core::ResRef;
    use mg_plugin::{Selected, Selection};
    use mg_rules::ObjectKind;
    let mut m = module();
    let mut git = Gff::new(*b"GIT ");
    let mut chest = Struct::new(9);
    chest.set("Tag", Value::String(b"OLD_CHEST".to_vec()));
    chest.set("TemplateResRef", Value::resref(ResRef::from_str("plc_chest1").unwrap()));
    let mut barrel = chest.clone();
    barrel.set("Tag", Value::String(b"BARREL".to_vec()));
    git.root.set("Placeable List", Value::List(vec![barrel, chest]));
    m.set_gff(key("tavern.git"), &git).unwrap();
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", Value::String(b"TAVERN".to_vec()));
    m.set_gff(key("tavern.are"), &are).unwrap();
    let area = ResRef::from_str("tavern").unwrap();
    let selection = Selection {
        selected: vec![
            Selected::Object { area, kind: ObjectKind::Placeable, index: 1 },
            Selected::Blueprint(key("guard.utc")),
            // (One of the game's, which the module lacks; no game here.)
            Selected::Blueprint(key("nw_chest.utp")),
            Selected::Area(area),
        ],
        open: Some(Selected::Resource(key("old.nss"))),
    };
    let code = r#"
        local function line(s)
            return table.concat({ s.kind, s.resource, s.path, tostring(s.blueprint),
                tostring(s.module), tostring(s.area), tostring(s.index), tostring(s.tag),
                tostring(s.resref) }, " | ")
        end
        local out = {}
        for _, s in ipairs(ctx.selection) do
            table.insert(out, line(s))
        end
        table.insert(out, "open: " .. line(ctx.open))
        -- The first is renamed where it stands: its resource and path.
        local first = ctx.selection[1]
        ctx.edit:set(first.resource, first.path .. "/Tag", "NEW_CHEST")
        return table.concat(out, "\n")
    "#;
    let host = Rc::new(TestHost::default());
    let input = Input { selection, ..Input::new(m.clone(), None) };
    let outcome = mg_plugin::run_console(code, input, host.clone()).unwrap();
    let lines = [
        "placeable | tavern.git | /Placeable List[1] | false | true | tavern | 1 | OLD_CHEST | plc_chest1",
        "creature | guard.utc |  | true | true | nil | nil | gate_guard | guard",
        "placeable | nw_chest.utp |  | true | false | nil | nil | nil | nw_chest",
        "area | tavern.are |  | false | true | tavern | nil | TAVERN | tavern",
        "open: nss | old.nss |  | false | true | nil | nil | nil | old",
    ];
    let said: Vec<String> = host.log.borrow().iter().map(|(_, t)| t.clone()).collect();
    assert_eq!(said, [format!("{:?}", lines.join("\n"))]);
    let mut ws = Workspace::new(m);
    ws.apply(Command::new("plugin", outcome.edits)).unwrap();
    let git = ws.doc(&key("tavern.git")).unwrap();
    let tags: Vec<&[u8]> =
        git.root.list("Placeable List").unwrap().iter().map(|s| s.string("Tag").unwrap()).collect();
    assert_eq!(tags, [&b"BARREL"[..], b"NEW_CHEST"]);

    // Nothing selected (a check, or `mg` without --select): an empty
    // list, and nothing open.
    let code = "return #ctx.selection, ctx.open == nil";
    let host = Rc::new(TestHost::default());
    mg_plugin::run_console(code, Input::new(module(), None), host.clone()).unwrap();
    let said: Vec<String> = host.log.borrow().iter().map(|(_, t)| t.clone()).collect();
    assert_eq!(said, ["0", "true"]);
}

/// What `mg plugin run --select` takes: a resource's name, and for an
/// object placed in an area its place in the GIT's list.
#[test]
fn a_selection_is_named_as_a_resource_and_a_path() {
    use mg_core::ResRef;
    use mg_plugin::{Selected, Target};
    use mg_rules::ObjectKind;
    let tavern = ResRef::from_str("tavern").unwrap();
    let chest = Selected::parse("tavern.git/Placeable List[3]").unwrap();
    assert_eq!(chest, Selected::Object { area: tavern, kind: ObjectKind::Placeable, index: 3 });
    assert_eq!(chest.target(), Some(Target::Object(ObjectKind::Placeable)));
    assert_eq!(chest.place().1.to_string(), "/Placeable List[3]");
    assert_eq!(Selected::parse("tavern.are").unwrap(), Selected::Area(tavern));
    assert_eq!(Selected::parse("guard.utc").unwrap(), Selected::Blueprint(key("guard.utc")));
    assert_eq!(Selected::parse("old.nss").unwrap(), Selected::Resource(key("old.nss")));
    assert_eq!(Selected::parse("old.nss").unwrap().target(), None);
    for (bad, says) in [
        ("tavern", "is not a resource's name"),
        ("tavern.git", "its place in a list"),
        ("tavern.git/Tile_List[0]", "not a list of an area's objects"),
        ("guard.utc/ClassList[0]", "only an object placed in an area"),
    ] {
        let e = Selected::parse(bad).unwrap_err();
        assert!(e.contains(says), "{bad}: {e}");
    }
}

/// A plugin's own tests (`mg plugin test`): each says what was not as its
/// file wants, and a file that does not read, or names no fixture, is a
/// test that failed.
#[test]
fn a_plugin_s_own_tests_say_what_is_not_as_wanted() {
    use mg_module::ModuleLocation;
    let dir = mg_testkit::scratch_dir("plugin-own-tests");
    let source = dir.join("tags");
    std::fs::create_dir_all(source.join("tests")).unwrap();
    for file in ["plugin.cfg", "main.luau", "rules.luau"] {
        std::fs::copy(fixtures().join("tag-conventions").join(file), source.join(file)).unwrap();
    }
    let mut m = module();
    m.save_as(&ModuleLocation::Folder(source.join("tests/fixture"))).unwrap();
    let tests = r#"
        module = "fixture"

        [[test]]
        name = "passes"
        check = "tag-case"
        fix = "Upper-case this tag"
        left = 0
        [[test.finding]]
        resource = "guard.utc"
        message = 'tag "gate_guard" is not upper case'
        fix = "Upper-case this tag"
        [[test.field]]
        resource = "guard.utc"
        field = "Tag"
        value = "GATE_GUARD"
        [[test.field]]
        resource = "guard.utc"
        field = "ClassList[0]"
        [[test.field]]
        resource = "guard.utc"
        field = "/ClassList[1]"
        absent = true
        [[test.field]]
        resource = "guard.utc"
        field = "/Equip_ItemList[0]"
        absent = true

        [[test]]
        name = "list items there and gone"
        command = "fix-tags"
        [[test.field]]
        resource = "guard.utc"
        field = "/ClassList[0]"
        absent = true
        [[test.field]]
        resource = "guard.utc"
        field = "ClassList[3]"
        [[test.field]]
        resource = "guard.utc"
        field = "ClassList[0]"
        value = 1

        [[test]]
        name = "wrong finding"
        check = "tag-case"
        [[test.finding]]
        resource = "captain.utc"
        message = "no"
        severity = "error"
        fix = ""

        [[test]]
        name = "wrong field and log"
        command = "promote"
        edits = 3
        log = ["promoted"]
        [[test.field]]
        resource = "guard.utc"
        field = "/ClassList[0]/ClassLevel"
        value = 9
        [[test.field]]
        resource = "guard.utc"
        field = "Comment"
        absent = true
        [[test.field]]
        resource = "guard.utc"
        field = "Wis"
        value = 14

        [[test]]
        name = "it was to fail"
        command = "fix-tags"
        error = "nope"

        [[test]]
        name = "both"
        check = "tag-case"
        command = "fix-tags"
    "#;
    std::fs::write(source.join("tests/a.toml"), tests).unwrap();
    std::fs::write(source.join("tests/b.toml"), "[[test]]\nname = \"x\"\ncheck = \"tag-case\"\n")
        .unwrap();
    std::fs::write(source.join("tests/c.toml"), "[[test]]\nname = \"x\"\nwhat = 1\n").unwrap();
    let plugin = Plugin::load(&source).unwrap();
    let results = mg_plugin::run_tests(&plugin, None, &|_, _| Ok(None));
    let of = |name: &str| -> Vec<String> {
        results.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{name}")).problems.clone()
    };
    assert_eq!(of("passes"), Vec::<String>::new());
    let wrong = of("wrong finding").join("\n");
    for says in [
        "is of guard.utc, not captain.utc",
        "says \"tag \\\"gate_guard\\\"",
        "is a warning, not a error",
        "has the fix \"Upper-case this tag\", not none",
    ] {
        assert!(wrong.contains(says), "{says}: {wrong}");
    }
    let field = of("wrong field and log").join("\n");
    for says in ["ClassLevel is 1, not 9", "no line with \"promoted\"", "18 edits, not 3"] {
        assert!(field.contains(says), "{says}: {field}");
    }
    assert!(!field.contains("Comment") && !field.contains("Wis"), "{field}");
    // A list's item, as a field of the test: there or gone, never a value.
    assert_eq!(
        of("list items there and gone"),
        [
            "guard.utc /ClassList[0] is there, and was to be gone",
            "guard.utc ClassList[3] is not there",
            "guard.utc ClassList[0]: a list's item has no value: name a field of it",
        ]
    );
    assert_eq!(of("it was to fail"), ["it was to fail, saying \"nope\", and did not"]);
    assert!(of("both")[0].contains("one thing"));
    let b = results.iter().find(|r| r.file == "b.toml").unwrap();
    assert!(b.problems[0].contains("no fixture"), "{b:?}");
    let c = results.iter().find(|r| r.file == "c.toml").unwrap();
    assert!(c.problems[0].contains("does not read") && c.problems[0].contains("what"), "{c:?}");
    // Another fixture for every test (`--module`): there the guard's tag
    // is upper case, so the finding is not there.
    let mut fixed = module();
    let mut guard = fixed.gff(&key("guard.utc")).unwrap().unwrap();
    guard.root.set("Tag", Value::String(b"GATE_GUARD".to_vec()));
    fixed.set_gff(key("guard.utc"), &guard).unwrap();
    let other = dir.join("other.mod");
    fixed.save_as(&ModuleLocation::Archive(other.clone())).unwrap();
    let results = mg_plugin::run_tests(&plugin, Some(&other), &|_, _| Ok(None));
    let passes = results.iter().find(|r| r.name == "passes").unwrap();
    assert!(passes.problems[0].starts_with("0 findings, not 1"), "{passes:?}");
    assert!(results.iter().any(|r| r.file == "b.toml" && r.passed()), "{results:?}");
}

/// The Domains of Dread's notes on their setup, 3: a test brings its own
/// game data (a folder of 2DAs beside it, over the game's), so that it
/// runs the same with any game or none; a test may name a game's root;
/// and one that read what its game data lacks says so.
#[test]
fn a_plugin_s_tests_bring_their_own_game_data() {
    use mg_module::ModuleLocation;
    let dir = mg_testkit::scratch_dir("plugin-tests-game");
    let source = dir.join("armor");
    for folder in ["tests/game", "tests/empty"] {
        std::fs::create_dir_all(source.join(folder)).unwrap();
    }
    std::fs::write(
        source.join("plugin.cfg"),
        "[plugin]\nid = \"test.armor\"\nname = \"Armor\"\nversion = \"1\"\napi = \"0.4\"\n\
         license = \"GPL-3.0-or-later\"\n[check]\nid = \"chest\"\ntitle = \"Chest\"\n\
         [check]\nid = \"quiet\"\ntitle = \"Quiet\"\n\
         [check]\nid = \"listed\"\ntitle = \"Listed\"\n",
    )
    .unwrap();
    std::fs::write(
        source.join("main.luau"),
        r#"
        local mg = require("@moonglow")
        -- A table the game has: read whole, or the job fails.
        mg.check("chest", function(ctx)
            local chest = ctx.game:table("parts_chest")
            return { { resource = "guard.utc", message = `AC {chest:get(1, "ACBONUS")}` } }
        end)
        -- The same, quiet without it (as many a plugin is).
        mg.check("quiet", function(ctx)
            local ok, chest = pcall(function()
                return ctx.game:table("parts_chest")
            end)
            if not ok then
                return {}
            end
            return { { resource = "guard.utc", message = `AC {chest:get(1, "ACBONUS")}` } }
        end)
        -- The game's tables, by name.
        mg.check("listed", function(ctx)
            local found = {}
            for _, name in ipairs(ctx.game:resources("2da")) do
                table.insert(found, { resource = "guard.utc", message = name })
            end
            return found
        end)
        "#,
    )
    .unwrap();
    std::fs::write(
        source.join("tests/game/parts_chest.2da"),
        "2DA V2.0\n\n   ACBONUS\n0  0.00\n1  3.00\n",
    )
    .unwrap();
    module().save_as(&ModuleLocation::Folder(source.join("tests/fixture"))).unwrap();
    let tests = r#"
        module = "fixture"

        [[test]]
        name = "the folder's table"
        check = "chest"
        [[test.finding]]
        resource = "guard.utc"
        message = "AC 3.00"

        [[test]]
        name = "another folder, without it"
        check = "chest"
        game = "empty"
        findings = 1

        [[test]]
        name = "quiet without it"
        check = "quiet"
        game = "empty"
        findings = 1

        [[test]]
        name = "a folder not there"
        check = "chest"
        game = "nowhere"

        [[test]]
        name = "a root of its own"
        check = "chest"
        root = "elsewhere"

        [[test]]
        name = "no game, the folder"
        check = "chest"
        root = false
        [[test.finding]]
        resource = "guard.utc"
        message = "AC 3.00"

        [[test]]
        name = "no game, quiet"
        check = "quiet"
        root = false
        game = "empty"
        findings = 0

        [[test]]
        name = "no game, listed"
        check = "listed"
        root = false
        [[test.finding]]
        resource = "guard.utc"
        message = "parts_chest.2da"
    "#;
    std::fs::write(source.join("tests/a.toml"), tests).unwrap();
    // A file with no game, but for a test that reads the machine's.
    let tests = r#"
        module = "fixture"
        root = false

        [[test]]
        name = "the machine's game after all"
        check = "chest"
        root = true
        game = "empty"
        findings = 1
    "#;
    std::fs::write(source.join("tests/b.toml"), tests).unwrap();
    let plugin = Plugin::load(&source).unwrap();
    // No game: the folder's table, and only it.
    let roots = RefCell::new(Vec::new());
    let no_game = |_: &Module, root: Option<&std::path::Path>| {
        roots.borrow_mut().push(root.map(std::path::Path::to_path_buf));
        match root {
            Some(_) => Err("no game install there".to_string()),
            None => Ok(None),
        }
    };
    let results = mg_plugin::run_tests(&plugin, None, &no_game);
    let of = |results: &[mg_plugin::TestResult], name: &str| -> Vec<String> {
        results.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{name}")).problems.clone()
    };
    assert_eq!(of(&results, "the folder's table"), Vec::<String>::new());
    let lacked = "the game's data has no parts_chest.2da, which it read (there is no game \
                  install, nor the test's game folder, empty): put it in empty, or name a game \
                  with root = \"…\"";
    let without = of(&results, "another folder, without it");
    assert_eq!(without.len(), 2, "{without:?}");
    assert!(without[0].contains("parts_chest.2da not found"), "{without:?}");
    assert_eq!(without[1], lacked);
    assert_eq!(of(&results, "quiet without it"), ["0 findings, not 1: []", lacked]);
    // Passed without it: told so, in a note (The Domains of Dread's notes
    // on 2.2.0, 3: their armor test passed silently without
    // parts_chest.2da).
    let notes = |results: &[mg_plugin::TestResult], name: &str| -> Vec<String> {
        results.iter().find(|r| r.name == name).unwrap().notes.clone()
    };
    assert_eq!(of(&results, "no game, quiet"), Vec::<String>::new());
    assert_eq!(
        notes(&results, "no game, quiet"),
        ["it passed without parts_chest.2da, which it read and the game's data has not (the \
             test reads no game, root = false, nor the test's game folder, empty): to test it \
             with it, put it in empty, or name a game with root = \"…\""]
    );
    assert_eq!(notes(&results, "the folder's table"), Vec::<String>::new(), "it had it");
    assert_eq!(notes(&results, "quiet without it"), Vec::<String>::new(), "a problem, not a note");
    // A list of the game's resources with no game: of the folder alone.
    assert_eq!(of(&results, "no game, listed"), Vec::<String>::new());
    assert_eq!(
        notes(&results, "no game, listed"),
        ["it listed the game's resources (ctx.game:resources()), and the test reads no game, \
             root = false: the list is only the module's and those of the test's game folder, game"]
    );
    assert_eq!(of(&results, "no game, the folder"), Vec::<String>::new());
    // (`root = true`: the machine's, and here there is none.)
    let after_all = of(&results, "the machine's game after all");
    assert!(after_all.last().unwrap().contains("(there is no game install, nor"), "{after_all:?}");
    let nowhere = of(&results, "a folder not there");
    assert!(nowhere[0].starts_with("game: ") && nowhere[0].ends_with("nowhere is not a folder"));
    let root = source.join("tests").join("elsewhere");
    assert_eq!(
        of(&results, "a root of its own"),
        [format!("root {}: no game install there", root.display())]
    );
    assert!(roots.borrow().contains(&Some(root)), "{:?}", roots.borrow());
    // A game whose parts_chest says otherwise: the folder's is over it,
    // and another folder reads the game's.
    let game = |m: &Module, _: Option<&std::path::Path>| {
        let mut rm = mg_resman::ResMan::new();
        let mut base = mg_resman::MemContainer::new();
        base.insert(key("parts_chest.2da"), b"2DA V2.0\n\n   ACBONUS\n0  0.00\n1  9.00\n".to_vec());
        rm.add(mg_resman::priority::KEY, "base", mg_resman::LayerClass::Key, base);
        rm.add(mg_resman::priority::MODULE, "module", mg_resman::LayerClass::Erf, m.container());
        Ok(Some(mg_rules::GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH))))
    };
    let results = mg_plugin::run_tests(&plugin, None, &game);
    assert_eq!(of(&results, "the folder's table"), Vec::<String>::new(), "3, not the game's 9");
    assert_eq!(of(&results, "another folder, without it"), Vec::<String>::new());
    // With a game, root = false reads none of it: the folder's 3, and
    // nothing without the folder; root = true reads it again.
    assert_eq!(of(&results, "no game, the folder"), Vec::<String>::new());
    assert_eq!(of(&results, "no game, quiet"), Vec::<String>::new());
    assert_eq!(of(&results, "no game, listed"), Vec::<String>::new(), "not the game's");
    assert_eq!(of(&results, "the machine's game after all"), Vec::<String>::new());
}

/// The Domains of Dread's notes on 2.2.0, 4: a plugin's test runs one of
/// Moonglow's own checks (`builtin`) on its fixture, by the fixture's
/// project file (moonglow.toml, variables.toml), its findings and fixes
/// held as a plugin's are; what of the project file is not understood is
/// a note. Several of them in one test, or all (the notes on 2.2.0: one a
/// test). And, 3: listing the game's resources with no game and no game
/// folder lists the module's, and says so (the notes on 2.2.0: `ctx.game`
/// was empty, the fixture not in it).
#[test]
fn a_plugin_s_tests_run_moonglow_s_own_checks() {
    use mg_module::ModuleLocation;
    let dir = mg_testkit::scratch_dir("plugin-tests-builtin");
    let source = dir.join("rules");
    std::fs::create_dir_all(source.join("tests")).unwrap();
    std::fs::write(
        source.join("plugin.cfg"),
        "[plugin]\nid = \"test.rules\"\nname = \"Rules\"\nversion = \"1\"\napi = \"0.4\"\n\
         license = \"GPL-3.0-or-later\"\n[check]\nid = \"listed\"\ntitle = \"Listed\"\n\
         [check]\nid = \"vars\"\ntitle = \"Vars\"\n\
         [command]\nid = \"names\"\ntitle = \"Names\"\n",
    )
    .unwrap();
    std::fs::write(
        source.join("main.luau"),
        r#"
        local mg = require("@moonglow")
        mg.check("listed", function(ctx)
            return { { resource = "module.ifo", message = `{#ctx.game:resources()}` } }
        end)
        -- Names for new resources, as the window proposes them (the
        -- builder proposal's C1: plugins' new resources had no code).
        mg.command("names", function(ctx)
            local guard = ctx.module:new_name("Gate Guard", "utc")
            ctx.edit:write_bytes(guard, "")
            ctx.log:info(`{guard} {ctx.module:new_name("Gate Guard", "utc")}`)
            ctx.log:info(`{ctx.module:new_name("market", "are")} {ctx.module:new_name("kz_hello", ".nss")}`)
            local ok, why = pcall(function() return ctx.module:new_name("x", "zzz") end)
            ctx.log:info(`{tostring(ok)} {why}`)
        end)
        -- The project's descriptions (the notes on 2.2.0: no API read them).
        mg.check("vars", function(ctx)
            local text = mg.vars.describe("tdod_lore_text")
            local lore = mg.vars.describe("tdod_lore_old")
            local all = mg.vars.described()
            local said = `{text.name} {text.type} {text.file}:{text.line} {tostring(text.runtime)} `
                .. `{text.default} {text.requires[1]} {#text.requires}, `
                .. `{lore.name} {tostring(lore.pattern)} {tostring(lore.runtime)}, `
                .. `{tostring(mg.vars.describe("nope"))}, {#all} {all[2].name} {all[2].type}`
            return { { resource = "module.ifo", message = said } }
        end)
        "#,
    )
    .unwrap();
    // The fixture: a module folder that is a project, whose module keeps a
    // test's hook and a typo of a builder's variable, and has a start
    // waypoint twice.
    let fixture = source.join("tests").join("project");
    let mut m = Module::new();
    let var = |name: &str, value: Value| {
        let mut v = Struct::new(0);
        v.set("Name", Value::String(name.as_bytes().to_vec()));
        v.set("Type", Value::Dword(if matches!(value, Value::Int(_)) { 1 } else { 3 }));
        v.set("Value", value);
        v
    };
    let mut ifo = Gff::new(*b"IFO ");
    ifo.root.set(
        "VarTable",
        Value::List(vec![
            var("tdod_skill_d20", Value::Int(20)),
            var("tdod_lore_txt", Value::String(b"A plaque".to_vec())),
        ]),
    );
    m.set_info(&ifo).unwrap();
    let waypoint = |tag: &str| {
        let mut w = Struct::new(5);
        w.set("Tag", Value::String(tag.as_bytes().to_vec()));
        w
    };
    let mut git = Gff::new(*b"GIT ");
    git.root.set(
        "WaypointList",
        Value::List(vec![waypoint("TDOD_START_KREZK"), waypoint("TDOD_START_KREZK")]),
    );
    m.set_gff(key("krezk.git"), &git).unwrap();
    m.save_as(&ModuleLocation::Folder(fixture.clone())).unwrap();
    std::fs::write(
        fixture.join("moonglow.toml"),
        "[tags]\nunique = [\"TDOD_START_*\"]\n[resrefs]\npattern = \"kz_{name}\"\n",
    )
    .unwrap();
    std::fs::write(
        fixture.join("variables.toml"),
        "colour = 3\n\
         [tdod_lore_text]\ntype = \"string\"\n\
         default = \"a plaque\"\nrequires = [\"tdod_lore_seen\"]\n\
         [\"tdod_*_d20\"]\ntype = \"int\"\nruntime = true\n\
         [\"tdod_lore_*\"]\nruntime = true\n",
    )
    .unwrap();
    let tests = r#"
        module = "project"

        [[test]]
        name = "the scripts' own, kept in the module"
        builtin = "var-runtime"
        [[test.finding]]
        resource = "module.ifo"
        at = "Module › VarTable › tdod_skill_d20"
        severity = "error"
        fix = "Remove the variable"
        [[test.finding]]
        resource = "module.ifo"
        at = "Module › VarTable › tdod_lore_txt"
        message = "tdod_lore_txt (tdod_lore_*) is set by the scripts while the server runs (variables.toml:9 says runtime = true), and is never to be kept in a file: one kept here is there from the start, on every server: tdod_lore_text? (variables.toml:2)"

        [[test]]
        name = "a start waypoint twice, retagged"
        builtin = "tag-unique"
        fix = "Retag the other as TDOD_START_KREZK_2"
        left = 0
        [[test.field]]
        resource = "krezk.git"
        field = "/WaypointList[1]/Tag"
        value = "TDOD_START_KREZK_2"

        [[test]]
        name = "a check Moonglow has not"
        builtin = "tag-uniq"

        [[test]]
        name = "a builtin check runs on the module alone"
        builtin = "tag-unique"
        select = ["krezk.git"]
        settings = { x = 1 }

        [[test]]
        name = "two things"
        builtin = "tag-unique"
        check = "listed"

        [[test]]
        name = "the game's resources, with nothing"
        check = "listed"
        root = false
        [[test.finding]]
        resource = "module.ifo"
        message = "2"

        [[test]]
        name = "two of Moonglow's checks"
        builtin = ["var-runtime", "tag-unique", "var-runtime"]
        findings = 3

        [[test]]
        name = "all of Moonglow's checks"
        builtin = "all"
        findings = 3

        [[test]]
        name = "all, fixed"
        builtin = "all"
        fix = "Remove the variable"
        left = 1

        [[test]]
        name = "a list with one Moonglow has not"
        builtin = ["tag-unique", "tag-uniq"]

        [[test]]
        name = "an empty list"
        builtin = []

        [[test]]
        name = "names for new resources"
        command = "names"
        log = [
            "kz_gateguard.utc kz_gateguard001.utc",
            "kz_market.are kz_hello.nss",
            "false ",
        ]

        [[test]]
        name = "the project's descriptions"
        check = "vars"
        [[test.finding]]
        resource = "module.ifo"
        message = "tdod_lore_text string variables.toml:2 false a plaque tdod_lore_seen 1, tdod_lore_* true true, nil, 3 tdod_*_d20 int"
    "#;
    std::fs::write(source.join("tests").join("own.toml"), tests).unwrap();
    let plugin = Plugin::load(&source).unwrap();
    let results = mg_plugin::run_tests(&plugin, None, &|_, _| Ok(None));
    let result = |name: &str| results.iter().find(|r| r.name == name).unwrap().clone();
    let runtime = result("the scripts' own, kept in the module");
    assert_eq!(runtime.problems, Vec::<String>::new());
    // The project file's line it did not understand: a note.
    assert_eq!(
        runtime.notes,
        ["the fixture's project file: variables.toml:1: \"colour\" is neither a variable \
             ([colour]) nor undescribed = [...] or [[rules]]"]
    );
    assert_eq!(result("a start waypoint twice, retagged").problems, Vec::<String>::new());
    let unknown = result("a check Moonglow has not").problems;
    assert!(unknown[0].starts_with("builtin: Moonglow has no check \"tag-uniq\" (its checks: "));
    assert!(unknown[0].contains("tag-unique, ") && unknown[0].contains("var-runtime"));
    assert_eq!(
        result("a builtin check runs on the module alone").problems,
        ["select, settings are for a plugin's own check, command or event, not a builtin check"]
    );
    assert!(result("two things").problems[0].contains("one thing"));
    // With no game and no game folder, the game's data is the fixture's.
    let nothing = result("the game's resources, with nothing");
    assert_eq!(nothing.problems, Vec::<String>::new());
    assert_eq!(
        nothing.notes,
        ["it listed the game's resources (ctx.game:resources()), and the test reads no game, \
             root = false, nor a game folder: the list is only the module's (put those it looks \
             for in a folder game/ beside the test file, or name a game with root = \"…\")"]
    );
    // Several of Moonglow's checks, each once, their findings as Verify
    // lists them (errors first), and all of them: the fixture has nothing
    // else wrong.
    for name in [
        "two of Moonglow's checks",
        "all of Moonglow's checks",
        "all, fixed",
        "the project's descriptions",
        "names for new resources",
    ] {
        assert_eq!(result(name).problems, Vec::<String>::new(), "{name}");
    }
    let unknown = result("a list with one Moonglow has not").problems;
    assert!(unknown[0].starts_with("builtin: Moonglow has no check \"tag-uniq\""), "{unknown:?}");
    assert!(unknown[0].ends_with("\"all\" for every one)"), "{unknown:?}");
    assert_eq!(
        result("an empty list").problems,
        ["builtin: name a check, several in a list, or \"all\""]
    );
}

/// The Domains of Dread's setup on 2.1.0 and notes on 2.2.0: a test's
/// game folder brings a `dialog.tlk`, the module's custom talk table and
/// the module's haks, which `ctx.game` and Moonglow's checks read; with
/// no talk table to count, the checks that count its strings say so
/// rather than find what the game would not; and what is `left` after a
/// fix reads the module as fixed through the game's data too.
#[test]
fn a_plugin_s_test_game_folder_brings_talk_tables_and_haks() {
    use mg_module::ModuleLocation;
    let dir = mg_testkit::scratch_dir("plugin-tests-talk");
    let source = dir.join("talk");
    std::fs::create_dir_all(source.join("tests/talk/hak")).unwrap();
    std::fs::write(
        source.join("plugin.cfg"),
        "[plugin]\nid = \"test.talk\"\nname = \"Talk\"\nversion = \"1\"\napi = \"0.4\"\n\
         license = \"GPL-3.0-or-later\"\n[check]\nid = \"said\"\ntitle = \"Said\"\n\
         [check]\nid = \"rows\"\ntitle = \"Rows\"\n",
    )
    .unwrap();
    std::fs::write(
        source.join("main.luau"),
        r#"
        local mg = require("@moonglow")
        -- What the talk tables say, and whether the hak's table is there.
        mg.check("said", function(ctx)
            local base = ctx.game:string(1) or "none"
            local custom = ctx.game:string(16777216) or "none"
            local hak = if ctx.game:has("extra.2da") then "hak" else "no hak"
            return { { resource = "module.ifo", message = `{base}, {custom}, {hak}` } }
        end)
        -- A module's table read through the game's data: a row too many,
        -- with a fix that takes it out.
        mg.check("rows", function(ctx)
            local t = ctx.game:table("skills")
            if t.rows < 3 then
                return {}
            end
            local fix = ctx.fix("Two rows")
            fix:write("skills.2da", "2DA V2.0\n\n   Label Name\n0  Hide  16777216\n1  Look  1\n")
            return { { resource = "skills.2da", message = `{t.rows} rows`, fix = fix } }
        end)
        "#,
    )
    .unwrap();
    // The module: a custom talk table it names, a hak, and a skills table
    // with a string of each table, and one past dialog.tlk's.
    let mut m = Module::new();
    let mut ifo = Gff::new(*b"IFO ");
    ifo.root.set("Mod_CustomTlk", Value::String(b"mine".to_vec()));
    let mut hak = Struct::new(8);
    hak.set("Mod_Hak", Value::String(b"extra".to_vec()));
    ifo.root.set("Mod_HakList", Value::List(vec![hak]));
    m.set_info(&ifo).unwrap();
    m.set(
        key("skills.2da"),
        b"2DA V2.0\n\n   Label Name\n0  Hide  16777216\n1  Look  1\n2  Lore  9\n".to_vec(),
    );
    m.save_as(&ModuleLocation::Folder(source.join("tests/fixture"))).unwrap();
    // The test's game folder: a dialog.tlk of three strings, the module's
    // of one, and its hak.
    let tlk = |texts: &[&str]| {
        let mut t = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
        for text in texts {
            t.entries.push(mg_tlk::TlkEntry::text(text.as_bytes().to_vec()));
        }
        t.to_bytes().unwrap()
    };
    std::fs::write(source.join("tests/talk/dialog.tlk"), tlk(&["Bad Strref", "Look", "Lore"]))
        .unwrap();
    std::fs::write(source.join("tests/talk/mine.tlk"), tlk(&["Hide"])).unwrap();
    let mut extra = Module::new();
    extra.set(key("extra.2da"), b"2DA V2.0\n\n   A\n0  1\n".to_vec());
    extra.save_as(&ModuleLocation::Archive(source.join("tests/talk/hak/extra.hak"))).unwrap();
    std::fs::create_dir_all(source.join("tests/empty")).unwrap();
    let tests = r#"
        module = "fixture"
        root = false

        [[test]]
        name = "the folder's talk tables and hak"
        check = "said"
        game = "talk"
        [[test.finding]]
        resource = "module.ifo"
        message = "Look, Hide, hak"

        [[test]]
        name = "none of them"
        check = "said"
        game = "empty"
        [[test.finding]]
        resource = "module.ifo"
        message = "none, none, no hak"

        [[test]]
        name = "strings counted by the folder's tables"
        builtin = ["2da-strref", "custom-tlk"]
        game = "talk"
        [[test.finding]]
        resource = "skills.2da"
        at = "row 2, Name"
        message = "string 9 doesn't exist: dialog.tlk has 3 strings (\"Bad Strref\")"

        [[test]]
        name = "no talk tables, a check named"
        builtin = "2da-strref"
        game = "empty"

        [[test]]
        name = "no talk tables, all the checks"
        builtin = "all"
        game = "empty"

        [[test]]
        name = "a fix read through the game's data"
        check = "rows"
        game = "talk"
        fix = "Two rows"
        left = 0
    "#;
    std::fs::write(source.join("tests/a.toml"), tests).unwrap();
    let plugin = Plugin::load(&source).unwrap();
    let results = mg_plugin::run_tests(&plugin, None, &|_, _| Ok(None));
    let result = |name: &str| {
        results.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{results:?}")).clone()
    };
    assert_eq!(result("the folder's talk tables and hak").problems, Vec::<String>::new());
    assert_eq!(result("the folder's talk tables and hak").notes, Vec::<String>::new());
    let none = result("none of them");
    assert_eq!(none.problems, Vec::<String>::new());
    assert_eq!(none.notes, Vec::<String>::new(), "a plugin's reads say what they lacked");
    // A test of Moonglow's checks is told of the haks its game data lacks.
    assert_eq!(
        result("no talk tables, a check named").notes,
        ["the fixture's haks extra are not in the test's game data, which has no game: put it in \
             its game folder's hak/ (game/hak/ beside the test file), or name a game with root = \
             \"…\""]
    );
    assert_eq!(result("strings counted by the folder's tables").problems, Vec::<String>::new());
    let uncounted = "counts no talk-table strings: the test reads no game, root = false, and its \
                     game folder, empty, has neither dialog.tlk nor the module's mine.tlk: put them \
                     in empty, or name a game with root = \"…\"";
    assert_eq!(
        result("no talk tables, a check named").problems,
        [format!("2da-strref {uncounted}")]
    );
    // All of them: told so, and none of the strings said to be missing,
    // nor the module's table (which the game, with its tlk folder, may
    // well find).
    let all = result("no talk tables, all the checks");
    assert_eq!(all.problems, Vec::<String>::new());
    assert!(
        all.notes.iter().any(|n| n
            == "custom-tlk, 2da-strref, 2da-described count no talk-table strings: the test \
                reads no game, root = false, and its game folder, empty, has neither dialog.tlk \
                nor the module's mine.tlk: put them in empty, or name a game with root = \"…\""),
        "{:?}",
        all.notes
    );
    assert_eq!(result("a fix read through the game's data").problems, Vec::<String>::new());
}
