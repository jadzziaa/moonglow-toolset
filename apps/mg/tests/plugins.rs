//! `mg plugin` and `mg verify --plugins`, on a module made here and the
//! plugin host's fixture plugins: a command's edits are saved (or printed),
//! a form is answered from the arguments, a plugin's checks count in
//! verify.

use std::path::{Path, PathBuf};

use mg_gff::{Gff, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use serde_json::{Value as Json, json};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/mg-plugin/tests/fixtures")
}

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

/// A module with a guard whose tag is in lower case.
fn module(dir: &Path) -> PathBuf {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("Tag", Value::String(b"gate_guard".to_vec()));
    m.set_gff(key("guard.utc"), &guard).unwrap();
    let path = dir.join("keep.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

fn tag(module: &Path) -> String {
    let g = Module::open(module).unwrap().gff(&key("guard.utc")).unwrap().unwrap();
    String::from_utf8(g.root.string("Tag").unwrap().to_vec()).unwrap()
}

fn mg(args: &[&str]) -> (bool, Json) {
    crate::quiet_json_out(crate::mg().args(["--no-user-dir", "--json"]).args(args))
}

#[test]
fn plugins_are_listed_and_checked() {
    let fixtures = fixtures();
    let (ok, listed) = mg(&["plugin", "list", fixtures.to_str().unwrap()]);
    assert!(ok, "{listed}");
    let plugins = listed["plugins"].as_array().unwrap();
    let tags = plugins.iter().find(|p| p["id"] == "example.tag-conventions").unwrap();
    assert_eq!(
        tags["commands"][0],
        json!({
            "id": "fix-tags", "title": "Fix Creature Tags",
            "hint": "Upper-case every creature's tag", "key": null, "for": [],
        })
    );
    assert_eq!(tags["checks"][0]["severity"], "warning");

    // One plugin's folder, whose code and manifest agree.
    let one = fixtures.join("tag-conventions");
    let (ok, checked) = mg(&["plugin", "check", one.to_str().unwrap()]);
    assert!(ok, "{checked}");
    assert_eq!(checked["plugins"], json!([{ "plugin": "example.tag-conventions", "faults": [] }]));
    // One whose code and manifest do not: it fails, saying how.
    let broken = fixtures.join("broken");
    let (ok, checked) = mg(&["plugin", "check", broken.to_str().unwrap()]);
    assert!(!ok);
    assert_eq!(checked["plugins"][0]["faults"].as_array().unwrap().len(), 4, "{checked}");
}

#[test]
fn a_plugin_s_command_runs_on_a_module() {
    let dir = mg_testkit::scratch_dir("mg-plugin-run");
    let path = module(&dir);
    let m = path.to_str().unwrap();
    let plugin = fixtures().join("tag-conventions");
    let plugin = plugin.to_str().unwrap();

    // A dry run prints the edits (a file `mg apply` reads) and saves nothing.
    let (ok, dry) = mg(&["plugin", "run", m, plugin, "fix-tags", "--dry-run"]);
    assert!(ok, "{dry}");
    assert_eq!((&dry["label"], &dry["edits"]), (&json!("Upper-case creature tags"), &json!(1)));
    assert_eq!(dry["command"]["edits"][0]["value"]["value"], "GATE_GUARD");
    assert_eq!(dry["log"], json!([{ "level": "info", "text": "1 creature tags changed" }]));
    assert_eq!(tag(&path), "gate_guard");

    let (ok, done) = mg(&["plugin", "run", m, plugin, "fix-tags"]);
    assert!(ok, "{done}");
    assert_eq!(done["resources"], json!(["guard.utc"]));
    assert_eq!(tag(&path), "GATE_GUARD");

    // What the plugin does by itself when a module is saved, run from
    // here: with the resources named, or all of the module's.
    let (ok, saved) = mg(&["plugin", "run", m, plugin, "saved", "--event"]);
    assert!(ok, "{saved}");
    assert_eq!(saved["log"], json!([{ "level": "info", "text": "module-saved: guard.utc" }]));
    let event = ["plugin", "run", m, plugin, "saved", "--event", "--changed"];
    let (ok, other) = mg(&[&event[..], &["module.ifo"]].concat());
    assert!(ok && other["log"] == json!([]), "{other}");
    let (ok, wrong) = mg(&[&event[..], &["no name"]].concat());
    assert!(!ok && wrong["error"].as_str().unwrap().contains("no resource's name"), "{wrong}");
    // A command is no event, nor an event a command.
    let (ok, wrong) = mg(&["plugin", "run", m, plugin, "fix-tags", "--event"]);
    assert!(!ok && wrong["error"].as_str().unwrap().contains("event"), "{wrong}");
    let (ok, wrong) = mg(&["plugin", "run", m, plugin, "saved"]);
    assert!(!ok && wrong["error"].as_str().unwrap().contains("command"), "{wrong}");

    // A form is answered from the arguments: its defaults, and what is
    // given; a question, with --yes.
    let forms = fixtures().join("forms");
    let forms = forms.to_str().unwrap();
    let (ok, declined) = mg(&["plugin", "run", m, forms, "rename", "--answer", "prefix=SGT_"]);
    assert!(ok, "{declined}");
    assert_eq!(declined["edits"], 0);
    assert!(declined["log"][0]["text"].as_str().unwrap().ends_with("no (pass --yes to agree)"));
    let (ok, done) = mg(&["plugin", "run", m, forms, "rename", "--answer", "prefix=SGT_", "--yes"]);
    assert!(ok, "{done}");
    assert_eq!(tag(&path), "SGT_GATE_GUARD");
    // An answer nobody asked for, or of the wrong kind, is an error, and
    // the module stays as it was.
    let fails = |args: &[&str]| {
        let (ok, json) = mg(&[&["plugin", "run", m, forms, "rename", "--yes"][..], args].concat());
        assert!(!ok, "{args:?}: {json}");
        json["error"].as_str().unwrap().to_string()
    };
    assert!(fails(&["--answer", "prefx=A_"]).contains("no form asked for it"));
    assert!(fails(&["--answer", "count=many"]).contains("is not a number"));
    assert!(fails(&["--answer", "count=500"]).contains("outside what the field takes"));
    assert!(fails(&["--answer", "kind=dragons"]).contains("is not one of creatures, doors"));
    assert_eq!(tag(&path), "SGT_GATE_GUARD");

    // Code that fails changes nothing, and says where.
    let hostile = fixtures().join("hostile");
    let (ok, failed) = mg(&["plugin", "run", m, hostile.to_str().unwrap(), "fails"]);
    assert!(!ok);
    let error = failed["error"].as_str().unwrap();
    assert!(error.contains("something went wrong") && error.contains("main.luau"), "{error}");
    assert_eq!(tag(&path), "SGT_GATE_GUARD");
}

/// API 0.4: a command is run on what `--select` names, as if selected in
/// the window; `mg plugin list` says what each command is for.
#[test]
fn a_plugin_s_command_runs_on_what_is_selected() {
    let dir = mg_testkit::scratch_dir("mg-plugin-select");
    let path = module(&dir);
    let m = path.to_str().unwrap();
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples");
    let plugin = examples.join("set-local");
    let plugin = plugin.to_str().unwrap();
    let (ok, listed) = mg(&["plugin", "list", plugin]);
    assert!(ok, "{listed}");
    assert_eq!(listed["plugins"][0]["commands"][1]["for"], json!(["area"]));

    let answers = ["--answer", "name=role", "--answer", "value=workbench"];
    let (ok, done) =
        mg(&[&["plugin", "run", m, plugin, "set-local", "--select", "guard.utc"][..], &answers]
            .concat());
    assert!(ok, "{done}");
    assert_eq!(done["log"], json!([{ "level": "info", "text": "gate_guard: role = workbench" }]));
    let g = Module::open(&path).unwrap().gff(&key("guard.utc")).unwrap().unwrap();
    let var = &g.root.list("VarTable").unwrap()[0];
    assert_eq!(var.string("Value"), Some(&b"workbench"[..]));

    // Nothing selected: the plugin says so, and nothing changes.
    let (ok, none) = mg(&["plugin", "run", m, plugin, "set-local"]);
    assert!(ok, "{none}");
    assert_eq!(none["edits"], json!(0), "{none}");
    let said = none["log"][0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("Select objects, or blueprints of the module, first"), "{none}");
    // Named wrong, or not in the module: an error.
    let fails = |select: &str| {
        let args = ["plugin", "run", m, plugin, "set-local", "--select", select];
        let (ok, json) = mg(&[&args[..], &answers].concat());
        assert!(!ok, "{select}: {json}");
        json["error"].as_str().unwrap().to_string()
    };
    assert!(fails("guard").contains("not a resource's name"));
    assert!(fails("start.git/Tile_List[0]").contains("not a list of an area's objects"));
    assert!(fails("captain.utc").contains("captain.utc is not in the module"));
}

/// A plugin's folder is packed into an archive, and the archive installed
/// into a folder of plugins: the plugin there is the one packed.
#[test]
fn a_plugin_is_packed_and_installed() {
    let dir = mg_testkit::scratch_dir("mg-plugin-pack");
    let path = module(&dir);
    let source = fixtures().join("tag-conventions");
    let archive = dir.join("tags.zip");
    let plugins = dir.join("plugins");
    let (zip, to) = (archive.to_str().unwrap(), plugins.to_str().unwrap());

    let (ok, packed) = mg(&["plugin", "pack", source.to_str().unwrap(), "-o", zip]);
    assert!(ok, "{packed}");
    assert_eq!(packed["files"], json!(["main.luau", "plugin.cfg", "rules.luau"]));
    assert_eq!(packed["bytes"], std::fs::metadata(&archive).unwrap().len());

    let (ok, installed) = mg(&["plugin", "install", zip, to]);
    assert!(ok, "{installed}");
    let folder = plugins.join("example.tag-conventions");
    assert_eq!(installed["folder"], folder.to_str().unwrap());
    assert_eq!(installed["replaced"], Json::Null);
    // It is listed there, and runs from there.
    let (ok, listed) = mg(&["plugin", "list", to]);
    assert!(ok, "{listed}");
    assert_eq!(listed["plugins"][0]["id"], "example.tag-conventions");
    let (ok, done) =
        mg(&["plugin", "run", path.to_str().unwrap(), folder.to_str().unwrap(), "fix-tags"]);
    assert!(ok, "{done}");
    assert_eq!(tag(&path), "GATE_GUARD");

    // Again: only over itself, and only when told to.
    let (ok, again) = mg(&["plugin", "install", zip, to]);
    assert!(!ok);
    assert!(again["error"].as_str().unwrap().contains("1.0.0 is installed already (--replace"));
    let (ok, again) = mg(&["plugin", "install", zip, to, "--replace"]);
    assert!(ok, "{again}");
    assert_eq!(again["replaced"], "1.0.0");

    // The archive is checked like a folder: its layout and manifest, and
    // that its code registers what the manifest declares.
    let (ok, checked) = mg(&["plugin", "check", zip]);
    assert!(ok, "{checked}");
    assert_eq!(checked["plugins"], json!([{ "plugin": "example.tag-conventions", "faults": [] }]));

    // A plugin whose code and manifest disagree is not packed; a file
    // that is no archive is not installed, and fails the check.
    let broken = fixtures().join("broken");
    let (ok, failed) = mg(&["plugin", "pack", broken.to_str().unwrap(), "-o", zip]);
    assert!(!ok, "{failed}");
    let junk = dir.join("junk.zip");
    std::fs::write(&junk, "junk").unwrap();
    let (ok, failed) = mg(&["plugin", "install", junk.to_str().unwrap(), to]);
    assert!(!ok);
    assert!(failed["error"].as_str().unwrap().ends_with("junk.zip: it is not a zip archive"));
    let (ok, checked) = mg(&["plugin", "check", junk.to_str().unwrap()]);
    assert!(!ok);
    assert_eq!(checked["plugins"][0]["faults"], json!(["it is not a zip archive"]));

    // Removed by its id: what was installed, not what was copied in.
    let copied = plugins.join("by-hand");
    std::fs::create_dir_all(&copied).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        std::fs::copy(fixtures().join("forms").join(file), copied.join(file)).unwrap();
    }
    let (ok, removed) = mg(&["plugin", "remove", "example.tag-conventions", to]);
    assert!(ok, "{removed}");
    assert_eq!(removed["version"], "1.0.0");
    assert!(!folder.exists());
    let (ok, refused) = mg(&["plugin", "remove", "test.forms", to]);
    assert!(!ok);
    assert!(refused["error"].as_str().unwrap().contains("was not installed from a file"));
    assert!(copied.join("main.luau").is_file());
    let (ok, missing) = mg(&["plugin", "remove", "example.tag-conventions", to]);
    assert!(!ok);
    assert!(
        missing["error"].as_str().unwrap().ends_with("no plugin example.tag-conventions there")
    );
}

/// A plugin's checks run with the doctor's (where there is a game to
/// verify against).
#[test]
fn a_plugin_s_checks_count_in_verify() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("mg-plugin-verify");
    let path = module(&dir);
    let plugins = fixtures().join("tag-conventions");
    let args = ["--root", root.to_str().unwrap(), "verify", path.to_str().unwrap()];
    let (_, plain) = mg(&args);
    assert_eq!(plain["findings"], json!([]));
    let with_plugins = [&args[..], &["--plugins", plugins.to_str().unwrap()]].concat();
    let (ok, with) = mg(&with_plugins);
    assert!(ok, "a warning is no failure: {with}");
    assert_eq!(with["warnings"], plain["warnings"].as_u64().unwrap() + 1);
    // The finding offers its fix, the edits as `mg apply` takes them.
    let set = json!({
        "op": "set_field", "resource": "guard.utc", "field": "/Tag",
        "value": { "type": "cexostring", "value": "GATE_GUARD" },
    });
    assert_eq!(
        with["findings"],
        json!([{
            "severity": "warning",
            "check": "example.tag-conventions/tag-case",
            "source": "module",
            "resource": "guard.utc",
            "at": "Tag",
            "message": "tag \"gate_guard\" is not upper case",
            "fix": {
                "label": "Upper-case this tag",
                "command": { "version": 1, "label": "Upper-case this tag", "edits": [set] },
            },
        }])
    );
    // Nothing is changed unless asked; asked, the check's fixes are made.
    assert_eq!(tag(&path), "gate_guard");
    let fix = ["--fix", "example.tag-conventions/tag-case"];
    let (ok, fixed) = mg(&[&with_plugins[..], &fix].concat());
    assert!(ok, "{fixed}");
    assert_eq!(fixed["fixes"]["made"][0]["label"], "Upper-case this tag");
    assert_eq!(tag(&path), "GATE_GUARD");
}

/// `mg plugin test`: the example's own tests pass, as a build pipeline
/// would run them (an exit, a line a test, a summary); on a fixture where
/// they do not hold, or for a plugin with none, it fails.
#[test]
fn a_plugin_s_own_tests_run_as_in_a_pipeline() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/plugins/examples/tag-conventions")
        .canonicalize()
        .unwrap();
    let run = |more: &[&str]| {
        crate::mg()
            .args(["--no-user-dir", "plugin", "test"])
            .arg(&example)
            .args(more)
            .output()
            .unwrap()
    };
    let out = run(&[]);
    let text = String::from_utf8_lossy(&out.stdout);
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{text}{said}");
    assert!(
        text.lines()
            .all(|l| l.starts_with("ok\ttags.toml › ") || l.starts_with("ok\tbuiltin.toml › ")),
        "{text}"
    );
    assert!(said.contains("example.tag-conventions: 5 tests, 5 passed, 0 failed"), "{said}");
    let (ok, json) = mg(&["plugin", "test", example.to_str().unwrap()]);
    assert!(ok);
    assert_eq!((json["passed"].as_u64(), json["failed"].as_u64()), (Some(5), Some(0)));

    // The module of `module()`, which has no captain: the command's test
    // finds no such resource, and nor do builtin.toml's find the gate's
    // posts; the others pass.
    let dir = mg_testkit::scratch_dir("mg-plugin-test");
    let other = module(&dir);
    let out = run(&["--module", other.to_str().unwrap()]);
    assert!(!out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("FAIL\ttags.toml › the command upper-cases every tag, in one step: "),
        "{text}"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("2 passed, 3 failed"));
    let none = crate::mg()
        .args(["--no-user-dir", "plugin", "test"])
        .arg(fixtures().join("hostile"))
        .output()
        .unwrap();
    assert!(!none.status.success());
    assert!(String::from_utf8_lossy(&none.stderr).contains("has no tests"));
}

/// `mg plugin test` with no game: a test whose game data is its own (a
/// `game/` folder beside the test file) passes; one that names a root
/// that is no game says so, and how to say none; with a game, a test's
/// `root` reads that one, and `root = false` none (a test that passes
/// without what it read is told so, in a note).
#[test]
fn a_plugin_s_tests_run_on_their_own_game_data() {
    let dir = mg_testkit::scratch_dir("mg-plugin-test-game");
    let source = dir.join("skills");
    std::fs::create_dir_all(source.join("tests/game")).unwrap();
    std::fs::write(
        source.join("plugin.cfg"),
        "[plugin]\nid = \"test.skills\"\nname = \"Skills\"\nversion = \"1\"\napi = \"0.4\"\n\
         license = \"GPL-3.0-or-later\"\n[check]\nid = \"skills\"\ntitle = \"Skills\"\n\
         [check]\nid = \"quiet\"\ntitle = \"Quiet\"\n",
    )
    .unwrap();
    std::fs::write(
        source.join("main.luau"),
        "local mg = require(\"@moonglow\")\n\
         mg.check(\"skills\", function(ctx)\n\
             local skills = ctx.game:table(\"skills\")\n\
             return { { resource = \"guard.utc\", message = skills:get(0, \"Label\") } }\n\
         end)\n\
         mg.check(\"quiet\", function(ctx)\n\
             local ok, skills = pcall(function() return ctx.game:table(\"skills\") end)\n\
             if not ok then return {} end\n\
             return { { resource = \"guard.utc\", message = skills:get(0, \"Label\") } }\n\
         end)\n",
    )
    .unwrap();
    std::fs::write(source.join("tests/game/skills.2da"), "2DA V2.0\n\n   Label\n0  Lore\n")
        .unwrap();
    std::fs::create_dir_all(source.join("tests/bare")).unwrap();
    module(&source.join("tests"));
    let mut tests = String::from(
        "module = \"keep.mod\"\n\
         [[test]]\nname = \"its own\"\ncheck = \"skills\"\n\
         [[test.finding]]\nresource = \"guard.utc\"\nmessage = \"Lore\"\n\
         [[test]]\nname = \"a root that is no game\"\ncheck = \"skills\"\nroot = \"nowhere\"\n\
         [[test]]\nname = \"no game\"\ncheck = \"quiet\"\nroot = false\ngame = \"bare\"\n\
         findings = 0\n",
    );
    let game = mg_testkit::nwn_root();
    if let Some(root) = &game {
        // (The game's own first skill, over which the folder is not read.)
        tests.push_str(&format!(
            "[[test]]\nname = \"the game at a root\"\ncheck = \"skills\"\nroot = {:?}\n\
             game = \"bare\"\n[[test.finding]]\nresource = \"guard.utc\"\nmessage = \"AnimalEmpathy\"\n",
            root.display().to_string()
        ));
    }
    std::fs::write(source.join("tests/game.toml"), tests).unwrap();
    let out = crate::mg()
        .args(["--no-user-dir", "--root"])
        .arg(dir.join("no-game"))
        .args(["--json", "plugin", "test"])
        .arg(&source)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(!out.status.success(), "{json}");
    let test = |json: &serde_json::Value, name: &str| {
        json["tests"].as_array().unwrap().iter().find(|t| t["name"] == name).cloned().unwrap()
    };
    assert_eq!(test(&json, "its own")["passed"], true, "{json}");
    assert_eq!(test(&json, "its own")["notes"], serde_json::json!([]), "{json}");
    let nowhere = test(&json, "a root that is no game");
    let said = nowhere["problems"][0].as_str().unwrap();
    assert!(
        said.starts_with("root ")
            && said.ends_with(
                "nowhere: no game install there (no data/nwn_base.key): for a test with no game, \
                 its own game folder and the module alone, root = false"
            ),
        "{said}"
    );
    // Passed, without the table it read: said in a note, and counted.
    let without = "it passed without skills.2da, which it read and the game's data has not (the \
                   test reads no game, root = false, nor the test's game folder, bare): to test it \
                   with it, put it in bare, or name a game with root = \"…\"";
    let none = test(&json, "no game");
    assert_eq!(none["passed"], true, "{json}");
    assert_eq!(none["notes"], serde_json::json!([without]), "{json}");
    assert_eq!(json["noted"], 1, "{json}");
    // With the machine's game too, `root = false` reads none of it.
    let out = crate::mg()
        .args(["--no-user-dir", "plugin", "test"])
        .arg(&source)
        .env("NWN_ROOT", game.clone().unwrap_or_else(|| dir.join("no-game")))
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("ok\tgame.toml › no game\n"), "{text}");
    assert!(text.contains(&format!("note\tgame.toml › no game: {without}\n")), "{text}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("passed (1 with notes)"), "{text}");
    if game.is_some() {
        assert!(text.contains("ok\tgame.toml › the game at a root\n"), "{text}");
    }
}

/// A project (a module folder) with a creature whose tag lacks the team's
/// prefix, the team's prefix in its moonglow.toml, and the builder-tags
/// example in its `plugins/`.
fn project_with_plugins(name: &str) -> PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut captain = Gff::new(*b"UTC ");
    captain.root.set("Tag", Value::String(b"captain".to_vec()));
    m.set_gff(key("captain.utc"), &captain).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(
        dir.join("moonglow.toml"),
        "[plugins.\"example.builder-tags\"]\nprefix = \"KRZ_\"\n",
    )
    .unwrap();
    let example =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples/builder-tags");
    let into = dir.join("plugins/builder-tags");
    std::fs::create_dir_all(&into).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        std::fs::copy(example.join(file), into.join(file)).unwrap();
    }
    dir
}

/// `mg` with a data folder of its own (where Moonglow keeps the answers
/// about projects' plugins), so that tests running at once keep apart.
/// `mg` with Moonglow's data folder in `home` on every platform
/// (`mg_module::build::data_dir`: XDG_DATA_HOME on Linux, APPDATA on
/// Windows, HOME's Library on macOS).
fn mg_in(home: &Path) -> std::process::Command {
    let mut c = crate::mg();
    c.env("XDG_DATA_HOME", home.join("data")).env("APPDATA", home.join("data")).env("HOME", home);
    c
}

/// Where `mg_in(home)` keeps the project plugins' answers.
fn answers_in(home: &Path) -> std::path::PathBuf {
    let data = if cfg!(windows) {
        home.join("data/Moonglow")
    } else if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Moonglow")
    } else {
        home.join("data/moonglow")
    };
    data.join("project-plugins.toml")
}

fn mg_at(home: &Path, args: &[&str]) -> (bool, Json) {
    crate::quiet_json_out(mg_in(home).args(["--no-user-dir", "--json"]).args(args))
}

/// A project's plugins: listed with what the builder allowed and their
/// settings' values in the project; allowed by content (`mg plugin
/// allow`), the answer kept in Moonglow's own folder, not the project.
#[test]
fn a_project_s_plugins_are_listed_as_allowed() {
    let dir = project_with_plugins("mg-project-plugins-list");
    let home = mg_testkit::scratch_dir("mg-project-plugins-list-home");
    let project = dir.to_str().unwrap();
    let (ok, listed) = mg_at(&home, &["plugin", "list", "--project", project]);
    assert!(ok, "{listed}");
    let p = &listed["plugins"][0];
    assert_eq!(p["id"], "example.builder-tags");
    assert_eq!(p["project"]["trust"], "not asked");
    assert_eq!(p["project"]["runs"], false);
    assert!(p["project"]["digest"].as_str().unwrap().starts_with("sha256:"));
    let prefix = &p["settings"][0];
    assert_eq!(
        (&prefix["id"], &prefix["scope"], &prefix["value"], &prefix["default"]),
        (&json!("prefix"), &json!("project"), &json!("KRZ_"), &json!(""))
    );
    assert_eq!(p["settings"][2]["max"], 32);
    let (ok, allowed) = mg_at(&home, &["plugin", "allow", project]);
    assert!(ok, "{allowed}");
    assert_eq!(allowed["plugins"][0]["allow"], true);
    assert!(answers_in(&home).is_file());
    assert!(!dir.join(".moonglow").exists(), "nothing written into the project");
    let (_, listed) = mg_at(&home, &["plugin", "list", "--project", project]);
    assert_eq!(listed["plugins"][0]["project"]["trust"], "allowed");
    // Changed since: asked again.
    let main = dir.join("plugins/builder-tags/main.luau");
    let code = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{code}\n-- changed\n")).unwrap();
    let (_, listed) = mg_at(&home, &["plugin", "list", "--project", project]);
    assert_eq!(listed["plugins"][0]["project"]["trust"], "changed since allowed");
    let (_, _) = mg_at(&home, &["plugin", "allow", project, "example.builder-tags", "--deny"]);
    let (_, listed) = mg_at(&home, &["plugin", "list", "--project", project]);
    assert_eq!(listed["plugins"][0]["project"]["trust"], "denied");
    // As text: a line for it and one for each setting.
    let out = mg_in(&home)
        .args(["--no-user-dir", "plugin", "list", "--project", project])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("\tthe project's: denied"), "{text}");
    assert!(text.contains("\tsetting\tprefix\tstring\tproject\t\"KRZ_\""), "{text}");
    // A plugin the project has not.
    let out = mg_in(&home)
        .args(["--no-user-dir", "plugin", "allow", project, "nobody.else"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("the project has no plugin nobody.else"));
}

/// `mg verify` runs a project's plugins only as the builder allowed them
/// in Moonglow's own folder, or every one with `--allow-project-plugins`
/// (a build pipeline); answers the project carries itself allow nothing.
#[test]
fn verify_runs_a_project_s_plugins_as_allowed() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = project_with_plugins("mg-project-plugins-verify");
    let home = mg_testkit::scratch_dir("mg-project-plugins-verify-home");
    let args = ["--root", root.to_str().unwrap(), "verify", dir.to_str().unwrap()];
    let ours = |v: &Json| -> Vec<String> {
        (v["findings"].as_array().unwrap().iter())
            .filter(|f| f["check"] == "example.builder-tags/tag-prefix")
            .map(|f| f["message"].as_str().unwrap().to_string())
            .collect()
    };
    let (_, plain) = mg_at(&home, &args);
    assert!(ours(&plain).is_empty(), "not allowed: not run");
    let notes = plain["notes"].to_string();
    assert!(
        notes.contains(
            "the project's plugin example.builder-tags 1.0.0 did not run: you have not allowed it"
        ),
        "{notes}"
    );
    // A project that comes with an answer of its own, digest and all:
    // said, and nothing allowed.
    let (_, listed) = mg_at(&home, &["plugin", "list", "--project", dir.to_str().unwrap()]);
    let claim = format!(
        "[project_plugins]\n\"example.builder-tags\" = {{ allow = true, version = \"1.0.0\", \
         digest = {} }}\n",
        listed["plugins"][0]["project"]["digest"]
    );
    mg_module::project_file::ProjectFile::write_user(&dir, &claim).unwrap();
    let (_, claimed) = mg_at(&home, &args);
    assert!(ours(&claimed).is_empty(), "a project's own answer allows nothing");
    let notes = claimed["notes"].to_string();
    assert!(notes.contains("[project_plugins] is not read"), "{notes}");
    assert!(notes.contains("did not run: you have not allowed it"), "{notes}");
    std::fs::remove_dir_all(dir.join(".moonglow")).unwrap();
    // Every one, in a pipeline.
    let (_, all) = mg_at(&home, &[&args[..], &["--allow-project-plugins"]].concat());
    assert_eq!(ours(&all), ["tag \"captain\" does not begin with KRZ_"], "the team's prefix");
    // Allowed by the builder: it runs without the flag.
    mg_at(&home, &["plugin", "allow", dir.to_str().unwrap()]);
    let (_, allowed) = mg_at(&home, &args);
    assert_eq!(ours(&allowed).len(), 1);
    // Changed since: not run, and said.
    let main = dir.join("plugins/builder-tags/main.luau");
    let code = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{code}\n-- changed\n")).unwrap();
    let (_, changed) = mg_at(&home, &args);
    assert!(ours(&changed).is_empty());
    assert!(
        changed["notes"].to_string().contains("it changed since you allowed it (version 1.0.0)")
    );
}

/// `mg plugin run`: the project's settings for the run, `--setting` over
/// them, and what the plugin sets kept in the project's files.
#[test]
fn a_plugin_s_settings_are_read_and_kept_on_the_command_line() {
    let dir = project_with_plugins("mg-plugin-settings");
    let plugin = dir.join("plugins/builder-tags");
    let run = |more: &[&str]| {
        let args = [&["plugin", "run", dir.to_str().unwrap(), plugin.to_str().unwrap()][..], more]
            .concat();
        mg(&args)
    };
    let (ok, out) = run(&["remember-code", "--answer", "code=zz"]);
    assert!(ok, "{out}");
    assert_eq!(
        out["settings"],
        json!([{ "id": "code", "scope": "user-project", "value": "zz", "kept": true }])
    );
    let own = std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap();
    assert_eq!(own, "[plugins.\"example.builder-tags\"]\ncode = \"zz\"\n");
    // The code kept, read by the next run; a --setting over it.
    let (ok, out) = run(&["prefix-tags", "--select", "captain.utc", "--dry-run"]);
    assert!(ok, "{out}");
    assert!(out["log"].to_string().contains("1 tags begin with KRZ_zz_"), "{out}");
    let (_, out) = run(&[
        "prefix-tags",
        "--select",
        "captain.utc",
        "--dry-run",
        "--setting",
        "code=ab",
        "--setting",
        "max_length=9",
    ]);
    assert!(out["log"].to_string().contains("1 tags begin with KRZ_ab_"), "{out}");
    assert!(out.to_string().contains("KRZ_ab_ca"), "cut to 9: {out}");
    // A --setting that is none, or not of its kind.
    let fails = |more: &[&str]| {
        let out = crate::mg()
            .args([
                "--no-user-dir",
                "plugin",
                "run",
                dir.to_str().unwrap(),
                plugin.to_str().unwrap(),
                "prefix-tags",
            ])
            .args(more)
            .output()
            .unwrap();
        assert!(!out.status.success());
        String::from_utf8_lossy(&out.stderr).into_owned()
    };
    assert!(
        fails(&["--setting", "nope=1"])
            .contains("has no such setting (prefix, kinds, max_length, code, strict)")
    );
    assert!(fails(&["--setting", "max_length=40"]).contains("40 is more than 32"));
    // With --dry-run, what it would keep is said, and nothing is.
    let (_, out) = run(&["remember-code", "--answer", "code=yy", "--dry-run"]);
    assert_eq!(out["settings"][0]["kept"], false);
    let said = |n: &Json| {
        n.as_array().unwrap().iter().any(|l| {
            l.as_str().unwrap().starts_with("would set code to \"yy\" (yours, for this project")
        })
    };
    assert!(said(&out["notes"]), "{out}");
    assert!(std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap().contains("\"zz\""));
}

/// The House rules example (a resource's own events, API 0.4).
fn house_rules() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples/house-rules")
}

/// A module folder with an area tagged TAVERN; `moonglow.toml` holds the
/// team's settings of House rules (`toml`).
fn tavern(name: &str, toml: &str) -> PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", Value::String(b"TAVERN".to_vec()));
    m.set_gff(key("tavern.are"), &are).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join("moonglow.toml"), format!("[plugins.\"example.house-rules\"]\n{toml}"))
        .unwrap();
    dir
}

fn area_tag(dir: &Path) -> String {
    let g = Module::open(dir).unwrap().gff(&key("tavern.are")).unwrap().unwrap();
    String::from_utf8(g.root.string("Tag").unwrap().to_vec()).unwrap()
}

/// `mg set` and `mg apply` with `--plugins`: what a plugin's
/// `resource-saving` handler refuses is not written (an error says why),
/// what it warns of is written and said; without the plugin, as before.
#[test]
fn a_plugin_refuses_or_warns_of_what_mg_saves() {
    let rules = house_rules();
    let dir = tavern("mg-plugin-save-refused", "");
    let set = |tag: &str, plugins: bool| {
        let mut cmd = crate::mg();
        cmd.args(["--no-user-dir", "--json", "set", dir.to_str().unwrap(), "tavern.are"]);
        cmd.arg(format!("Tag={tag}"));
        if plugins {
            cmd.args(["--plugins", rules.to_str().unwrap()]);
        }
        cmd.output().unwrap()
    };
    let out = set("INN", true);
    assert!(!out.status.success());
    // (With --json, the error is in the JSON.)
    let stderr = String::from_utf8_lossy(&out.stdout);
    assert!(stderr.contains("not saved: a plugin refuses it"), "{stderr}");
    assert!(
        stderr.contains(
            "House rules: tavern.are: its tag was TAVERN, now INN: players' places are kept by it \
             (fix: Put back TAVERN)"
        ),
        "{stderr}"
    );
    assert_eq!(area_tag(&dir), "TAVERN", "nothing written");
    // --dry-run says the same, and writes nothing either.
    let out = crate::mg()
        .args(["--no-user-dir", "set", dir.to_str().unwrap(), "tavern.are", "Tag=INN", "--dry-run"])
        .args(["--plugins", rules.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
    // Without the plugin, a save is a save.
    assert!(set("INN", false).status.success());
    assert_eq!(area_tag(&dir), "INN");

    // The team keeps no tags: a change is a warning, and is written.
    let dir = tavern("mg-plugin-save-warned", "keep_tags = false\n");
    let edits = dir.join("edits.json");
    std::fs::write(
        &edits,
        r#"{ "label": "Rename", "edits": [ { "op": "set_field", "resource": "tavern.are",
             "field": "/Tag", "value": { "type": "cexostring", "value": "Inn" } } ] }"#,
    )
    .unwrap();
    let (ok, out) = mg(&[
        "apply",
        dir.to_str().unwrap(),
        edits.to_str().unwrap(),
        "--plugins",
        rules.to_str().unwrap(),
    ]);
    assert!(ok, "{out}");
    assert_eq!(area_tag(&dir), "Inn");
    assert_eq!(out["said"][0]["refused"], false, "{out}");
    assert_eq!(out["said"][0]["fix"], "Put back TAVERN", "{out}");
    let notes = out["notes"].to_string();
    assert!(
        notes.contains("warning: House rules: tavern.are: its tag was TAVERN, now Inn"),
        "{out}"
    );
}

/// `mg plugin run --event` runs the new events' handlers by hand: a save's
/// (with the module as it was, `--was`), objects pasted (`--select`), a
/// resource made; a refusal fails it.
#[test]
fn a_plugin_s_new_events_run_by_hand() {
    let rules = house_rules();
    let fixture = rules.join("tests/module");
    let run = |more: &[&str]| -> (bool, Json) {
        let out = crate::mg()
            .args(["--no-user-dir", "--json", "plugin", "run"])
            .args([fixture.to_str().unwrap(), rules.to_str().unwrap()])
            .args(more)
            .output()
            .unwrap();
        let json = serde_json::from_slice(&out.stdout).unwrap_or(Json::Null);
        (out.status.success(), json)
    };
    let was = rules.join("tests/was");
    let (ok, out) = run(&["area-tags", "--event", "--was", was.to_str().unwrap()]);
    assert!(!ok, "refused: {out}");
    assert_eq!(out["said"][0]["resource"], "cellar.are", "{out}");
    assert_eq!(out["said"][0]["refused"], true);
    let (ok, out) = run(&["area-tags", "--event", "--changed", "tavern.are"]);
    assert!(ok, "a warning alone: {out}");
    assert_eq!(out["said"][0]["text"], "tag Tavern is not upper case");
    let (ok, out) = run(&[
        "pasted-waypoints",
        "--event",
        "--select",
        "tavern.git/WaypointList[1]",
        "--by",
        "paste",
        "--dry-run",
    ]);
    assert!(ok, "{out}");
    assert!(out["command"].to_string().contains("WP_DOOR_2"), "{out}");
    let (ok, out) = run(&["new-area", "--event", "--changed", "cellar.are", "--dry-run"]);
    assert!(ok, "{out}");
    assert!(out["command"].to_string().contains("barovia"), "{out}");
    // An object event names its objects.
    let (ok, _) = run(&["pasted-waypoints", "--event", "--dry-run"]);
    assert!(!ok);
    // And mg plugin list says what each does by itself.
    let (ok, listed) = mg(&["plugin", "list", rules.to_str().unwrap()]);
    assert!(ok);
    let events = &listed["plugins"][0]["events"];
    assert_eq!(events[0]["on"], "resource-saving");
    assert_eq!(events[0]["types"], json!(["are"]));
    assert_eq!(events[1]["for"], json!(["waypoint"]));
    assert!(events[2]["says"].as_str().unwrap().starts_with("when a wizard, Add to Palette"));
}
