//! A project's own plugins (API 0.4): read whole and known by their
//! content, allowed or denied by the builder, and run from what was read;
//! and a plugin's settings, read and set through `ctx.settings`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mg_gff::{Gff, Value};
use mg_module::Module;
use mg_module::project_file::ProjectFile;
use mg_plugin::project::{self, Answers, Trust};
use mg_plugin::{
    Answer, Host, Input, Level, Plugin, PluginError, Question, SettingValue, Settings, run_check,
    run_command, settings,
};
use mg_resman::ResKey;

#[derive(Default)]
struct TestHost {
    log: RefCell<Vec<String>>,
}

impl Host for TestHost {
    fn log(&self, _: Level, text: &str) {
        self.log.borrow_mut().push(text.to_string());
    }

    fn ask(&self, _: &Question) -> Option<Answer> {
        None
    }
}

fn module() -> Module {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut g = Gff::new(*b"UTC ");
    g.root.set("Tag", Value::String(b"guard".to_vec()));
    m.set_gff(ResKey::from_filename("guard.utc").unwrap(), &g).unwrap();
    m
}

const MANIFEST: &str = r#"[plugin]
id = "team.rules"
name = "Team rules"
version = "1.0.0"
api = "0.4"
license = "GPL-3.0-or-later"

[command]
id = "show"
title = "Show"

[command]
id = "remember"
title = "Remember"

[check]
id = "prefix"
title = "Tags begin with the prefix"

[setting]
id = "prefix"
scope = "project"
default = "TDOD_"

[setting]
id = "code"
scope = "user-project"

[setting]
id = "most"
type = "int"
min = 1
max = 32
default = 32

[setting]
id = "loud"
type = "bool"
"#;

const CODE: &str = r#"
local mg = require("@moonglow")
local rules = require("rules")
mg.command("show", function(ctx)
    local seen = {}
    for name, value in ctx.settings do
        table.insert(seen, name .. "=" .. tostring(value))
    end
    ctx.log:info(table.concat(seen, " "))
    ctx.log:info(rules.word)
end)
mg.command("remember", function(ctx)
    ctx.settings:set("code", "ab")
    ctx.settings:set("most", 12)
    ctx.settings:set("loud", nil)
    ctx.log:info(ctx.settings.code .. " " .. ctx.settings.most .. " " .. tostring(ctx.settings.loud))
end)
mg.check("prefix", function(ctx)
    local found = {}
    local tag = ctx.module:gff("guard.utc").Tag
    if tag:sub(1, #ctx.settings.prefix) ~= ctx.settings.prefix then
        table.insert(found, { resource = "guard.utc", message = "no " .. ctx.settings.prefix })
    end
    return found
end)
"#;

/// A project with its plugin in `plugins/team-rules`.
fn project_with_plugin(name: &str) -> (PathBuf, PathBuf) {
    let root = mg_testkit::scratch_dir(name);
    let dir = root.join("plugins/team-rules");
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(dir.join("plugin.cfg"), MANIFEST).unwrap();
    std::fs::write(dir.join("main.luau"), CODE).unwrap();
    std::fs::write(dir.join("rules.luau"), "return { word = \"first\" }").unwrap();
    std::fs::write(dir.join("tests/a.toml"), "[[test]]\nname = \"x\"\n").unwrap();
    (root, dir)
}

fn found(root: &Path) -> Vec<Plugin> {
    let file = ProjectFile::read(root);
    project::discover(root, &file).into_iter().map(|f| f.plugin.unwrap()).collect()
}

fn run(
    plugin: &Plugin,
    command: &str,
    settings: Option<Settings>,
) -> (Vec<String>, mg_plugin::Outcome) {
    let host = Rc::new(TestHost::default());
    let input = Input { settings, ..Input::new(module(), None) };
    let outcome = run_command(plugin, command, input, host.clone()).unwrap();
    let log = host.log.borrow().clone();
    (log, outcome)
}

#[test]
fn a_project_s_plugin_is_known_by_its_content() {
    let (root, dir) = project_with_plugin("project-plugin-digest");
    let p = found(&root);
    assert_eq!(p.len(), 1);
    let digest = p[0].digest().unwrap().to_string();
    assert!(digest.starts_with("sha256:") && digest.len() == 7 + 64, "{digest}");
    assert!(p[0].is_project());
    // Its tests, and hidden files, are not part of it.
    std::fs::write(dir.join("tests/b.toml"), "").unwrap();
    std::fs::write(dir.join(".notes"), "mine").unwrap();
    assert_eq!(found(&root)[0].digest().unwrap(), digest);
    // Its code is.
    std::fs::write(dir.join("rules.luau"), "return { word = \"second\" }").unwrap();
    let changed = found(&root);
    assert_ne!(changed[0].digest().unwrap(), digest);
    assert_eq!(changed[0].manifest.version, "1.0.0", "the same version, other content");
    // And it runs from what was read: what was allowed, whatever the
    // folder holds since.
    let (log, _) = run(&p[0], "show", None);
    assert_eq!(log[1], "first");
    let (log, _) = run(&changed[0], "show", None);
    assert_eq!(log[1], "second");
    // An installed plugin's folder is read as it runs.
    let installed = Plugin::load(&dir).unwrap();
    assert!(!installed.is_project() && installed.digest().is_none());
}

#[test]
fn the_builder_s_answer_holds_for_the_plugin_as_it_was() {
    let (root, dir) = project_with_plugin("project-plugin-trust");
    // (Moonglow's own folder, here a scratch one: never the project.)
    let kept = mg_testkit::scratch_dir("project-plugin-trust-answers").join("answers.toml");
    let trust = |p: &Plugin| Answers::read(&kept).unwrap().trust(&root, p);
    let p = &found(&root)[0];
    assert_eq!(trust(p), Trust::Unasked);
    assert!(Trust::Unasked.to_ask() && !Trust::Unasked.runs());
    project::decide(&kept, &root, p, true).unwrap();
    assert_eq!(trust(p), Trust::Allowed);
    // Kept by the project's canonical path, the plugin and its digest.
    let text = std::fs::read_to_string(&kept).unwrap();
    let canonical = root.canonicalize().unwrap();
    // (TOML quotes a Windows path as a literal string: the value is compared.)
    let table: toml::Table = text.parse().unwrap();
    let answer = &table["answer"].as_array().unwrap()[0];
    assert_eq!(
        answer["project"].as_str(),
        Some(canonical.display().to_string().as_str()),
        "{text}"
    );
    assert!(text.contains("plugin = \"team.rules\"") && text.contains("digest = \"sha256:"));
    assert!(!root.join(".moonglow").exists(), "nothing written into the project");
    // Another project with the same plugin: not allowed by this answer.
    let (other, _) = project_with_plugin("project-plugin-trust-other");
    let theirs = &found(&other)[0];
    assert_eq!(theirs.digest(), p.digest());
    assert_eq!(Answers::read(&kept).unwrap().trust(&other, theirs), Trust::Unasked);
    // Changed: asked again.
    std::fs::write(dir.join("main.luau"), format!("{CODE}\n-- more")).unwrap();
    let changed = &found(&root)[0];
    let t = trust(changed);
    assert_eq!(t, Trust::Changed { was: "1.0.0".into() });
    assert!(t.to_ask() && !t.runs());
    // Denied: off, whatever it becomes, and not asked again.
    project::decide(&kept, &root, changed, false).unwrap();
    assert_eq!(trust(changed), Trust::Denied);
    std::fs::write(dir.join("main.luau"), CODE).unwrap();
    let back = &found(&root)[0];
    assert_eq!(trust(back), Trust::Denied);
    assert!(!Trust::Denied.to_ask());
    assert_eq!(Answers::read(&kept).unwrap().answers.len(), 1, "one answer a plugin");
    // A file of answers that can't be read allows nothing.
    std::fs::write(&kept, "[[answer]\n").unwrap();
    assert!(Answers::read(&kept).is_err());
    // The folder the team's file names, inside the project only.
    std::fs::write(root.join("moonglow.toml"), "[plugins]\nfolder = \"tools\"\n").unwrap();
    assert!(found(&root).is_empty());
    std::fs::rename(root.join("plugins"), root.join("tools")).unwrap();
    assert_eq!(found(&root).len(), 1);
    // A plugin that can't be read is said, and not run.
    std::fs::write(root.join("tools/team-rules/plugin.cfg"), "[plugin]\nid = \"x\"\n").unwrap();
    let file = ProjectFile::read(&root);
    let f = project::discover(&root, &file);
    assert!(f[0].plugin.as_ref().unwrap_err().contains("needs"), "{:?}", f[0].plugin);
}

/// A project that comes with answers of its own (a `.moonglow/user.toml`
/// committed with its plugin "allowed", digest and all) allows nothing:
/// the section is said, and the plugin is still to be asked about.
#[test]
fn answers_a_project_carries_allow_nothing() {
    let (root, _) = project_with_plugin("project-plugin-carried-answers");
    let p = &found(&root)[0];
    let claim = format!(
        "[project_plugins]\n\"team.rules\" = {{ allow = true, version = \"1.0.0\", digest = {:?} }}\n",
        p.digest().unwrap()
    );
    ProjectFile::write_user(&root, &claim).unwrap();
    std::fs::write(root.join("moonglow.toml"), &claim).unwrap();
    let file = ProjectFile::read(&root);
    assert_eq!(file.warnings.len(), 2, "{:?}", file.warnings);
    assert!(file.warnings.iter().all(|w| w.contains("[project_plugins] is not read")));
    let kept = mg_testkit::scratch_dir("project-plugin-carried-answers-kept").join("answers.toml");
    let t = Answers::read(&kept).unwrap().trust(&root, p);
    assert_eq!(t, Trust::Unasked);
    assert!(t.to_ask());
}

#[test]
fn a_plugin_reads_its_settings_and_sets_them_for_the_host_to_keep() {
    let (root, _) = project_with_plugin("plugin-settings-ctx");
    let p = &found(&root)[0];
    // Their defaults, without a host's values.
    let (log, outcome) = run(p, "show", None);
    assert_eq!(log[0], "prefix=TDOD_ code= most=32 loud=false");
    assert!(outcome.settings.is_empty());
    // The host's values, and what the job set read back.
    std::fs::write(root.join("moonglow.toml"), "[plugins.\"team.rules\"]\nprefix = \"KRZ_\"\n")
        .unwrap();
    let file = ProjectFile::read(&root);
    let user = BTreeMap::from([("loud".to_string(), SettingValue::Bool(true))]);
    let (s, said) = settings::resolve(&p.manifest, &user, Some(&file));
    assert_eq!(said, Vec::<String>::new());
    let (log, _) = run(p, "show", Some(s.clone()));
    assert_eq!(log[0], "prefix=KRZ_ code= most=32 loud=true");
    let (log, outcome) = run(p, "remember", Some(s.clone()));
    assert_eq!(log[0], "ab 12 false");
    assert_eq!(
        outcome.settings,
        [
            ("code".to_string(), Some(SettingValue::Text("ab".into()))),
            ("most".to_string(), Some(SettingValue::Int(12))),
            ("loud".to_string(), None),
        ]
    );
    // Without a project, the project's settings can't be set.
    let host = Rc::new(TestHost::default());
    let input = Input::new(module(), None);
    let e = run_command(p, "remember", input, host).unwrap_err();
    assert!(e.to_string().contains("no project is open"), "{e}");
    // A check reads them, and sets none.
    let host = Rc::new(TestHost::default());
    let input = Input { settings: Some(s), ..Input::new(module(), None) };
    let found = run_check(p, "prefix", input, host).unwrap();
    assert_eq!(found[0].finding.message, "no KRZ_");
}

/// What a plugin is told when it gets its settings wrong.
#[test]
fn a_setting_misused_is_an_error_that_says_how() {
    let (root, dir) = project_with_plugin("plugin-settings-faults");
    let fails = |code: &str| -> String {
        std::fs::write(dir.join("main.luau"), format!(
            "local mg = require(\"@moonglow\")\nmg.command(\"show\", function(ctx) {code} end)\n\
             mg.command(\"remember\", function() end)\nmg.check(\"prefix\", function(ctx) {code} end)"
        ))
        .unwrap();
        let p = &found(&root)[0];
        let host = Rc::new(TestHost::default());
        let s = Settings { project: true, ..Default::default() };
        let input = Input { settings: Some(s), ..Input::new(module(), None) };
        match run_command(p, "show", input, host) {
            Err(PluginError::Script { message, .. }) => message,
            other => panic!("{code}: {other:?}"),
        }
    };
    let e = fails("return ctx.settings.claimz");
    assert!(e.contains("declares no setting \"claimz\" (it has prefix, code, most, loud)"), "{e}");
    let e = fails("ctx.settings.code = 'x'");
    assert!(e.contains("is set with ctx.settings:set(\"code\", value)"), "{e}");
    assert!(fails("ctx.settings:set('most', 40)").contains("40 is more than 32"));
    assert!(fails("ctx.settings:set('most', 2.5)").contains("number is no setting's value"));
    assert!(fails("ctx.settings:set('most', 'x')").contains("\"x\" is not a whole number"));
    assert!(fails("ctx.settings:set('loud', {})").contains("table is no setting's value"));
    assert!(fails("ctx.settings:set('gone', 1)").contains("declares no setting \"gone\""));
    assert!(fails("setmetatable(ctx.settings, {})").contains("protected metatable"));
    // A check that sets one fails.
    std::fs::write(
        dir.join("main.luau"),
        "local mg = require(\"@moonglow\")\n\
        mg.command(\"show\", function() end)\nmg.command(\"remember\", function() end)\n\
        mg.check(\"prefix\", function(ctx) ctx.settings:set(\"loud\", true) end)",
    )
    .unwrap();
    let p = &found(&root)[0];
    let e = run_check(p, "prefix", Input::new(module(), None), Rc::new(TestHost::default()))
        .unwrap_err();
    assert!(e.to_string().contains("a check only reads"), "{e}");
}
