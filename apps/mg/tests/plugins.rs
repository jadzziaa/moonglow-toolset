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
    guard.root.set("FactionID", Value::Word(1));
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
    crate::quiet_json_out(crate::mg_alone().arg("--json").args(args))
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
            "menu": [],
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

/// `mg plugin check` (and `pack`, which checks first) runs a plugin's
/// top-level code, an archive's from anywhere too: code that never
/// returns is stopped, and said to have run out of time.
#[test]
fn a_plugin_whose_code_never_loads_is_stopped() {
    let dir = mg_testkit::scratch_dir("mg-plugin-endless");
    std::fs::write(
        dir.join("plugin.cfg"),
        "[plugin]\nid = \"test.endless\"\nname = \"Endless\"\nversion = \"1\"\napi = \"0.1\"\n\
         license = \"MIT\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("main.luau"), "while true do end\n").unwrap();
    let started = std::time::Instant::now();
    let (ok, checked) = mg(&["plugin", "check", dir.to_str().unwrap()]);
    assert!(!ok, "{checked}");
    let fault = checked["plugins"][0]["faults"][0].as_str().unwrap_or_default();
    assert!(fault.contains("ran out of time: its code may take 5 seconds to load"), "{checked}");
    let out = dir.join("endless.zip");
    let (ok, packed) = mg(&["plugin", "pack", dir.to_str().unwrap(), "-o", out.to_str().unwrap()]);
    assert!(!ok && !out.exists(), "{packed}");
    assert!(started.elapsed() < std::time::Duration::from_secs(60), "{:?}", started.elapsed());
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
    // (They parse as numbers, and are none.)
    assert!(fails(&["--answer", "count=nan"]).contains("\"nan\" is not a number"));
    assert!(fails(&["--answer", "count=inf"]).contains("is not a number"));
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
    assert!(fails("guard.utc@key:nwn_base").contains("a layer's copy is only --open"));
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

    // An archive of several plugins: each checked, and each installed in
    // its own folder, or none (one installed already, without --replace).
    let bundle = dir.join("bundle.zip");
    {
        use std::io::Write;
        let manifest = std::fs::read_to_string(source.join("plugin.cfg")).unwrap();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for id in ["example.tag-conventions", "example.other"] {
            let m = manifest.replace("example.tag-conventions", id);
            zip.start_file(format!("{id}/plugin.cfg"), options).unwrap();
            zip.write_all(m.as_bytes()).unwrap();
            for file in ["main.luau", "rules.luau"] {
                zip.start_file(format!("{id}/{file}"), options).unwrap();
                zip.write_all(&std::fs::read(source.join(file)).unwrap()).unwrap();
            }
        }
        std::fs::write(&bundle, zip.finish().unwrap().into_inner()).unwrap();
    }
    let both = bundle.to_str().unwrap();
    let (ok, checked) = mg(&["plugin", "check", both]);
    assert!(ok, "{checked}");
    assert_eq!(checked["plugins"].as_array().unwrap().len(), 2);
    let (ok, refused) = mg(&["plugin", "install", both, to]);
    assert!(!ok);
    assert!(refused["error"].as_str().unwrap().contains("is installed already"), "{refused}");
    assert!(!plugins.join("example.other").exists(), "none of them installed");
    let (ok, installed) = mg(&["plugin", "install", both, to, "--replace"]);
    assert!(ok, "{installed}");
    let ids: Vec<&str> = installed["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["example.other", "example.tag-conventions"]);
    assert_eq!(installed["plugins"][1]["replaced"], "1.0.0");
    assert!(plugins.join("example.other/main.luau").is_file());

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
        crate::mg_alone().args(["plugin", "test"]).arg(&example).args(more).output().unwrap()
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
    let none = crate::mg_alone()
        .args(["plugin", "test"])
        .arg(fixtures().join("hostile"))
        .output()
        .unwrap();
    assert!(!none.status.success());
    assert!(String::from_utf8_lossy(&none.stderr).contains("has no tests"));

    // A folder of plugins: each one's tests, its lines under its id.
    let examples = example.parent().unwrap();
    let (ok, json) = mg(&["plugin", "test", examples.to_str().unwrap()]);
    assert!(ok, "{json}");
    let ids: Vec<&str> =
        json["plugins"].as_array().unwrap().iter().map(|p| p["plugin"].as_str().unwrap()).collect();
    assert!(ids.contains(&"example.tag-conventions") && ids.len() >= 2, "{ids:?}");

    // `mg plugin check` reads the test files too (not running them).
    let work = dir.join("checked");
    std::fs::create_dir_all(work.join("tests")).unwrap();
    for f in ["plugin.cfg", "main.luau", "rules.luau"] {
        std::fs::copy(example.join(f), work.join(f)).unwrap();
    }
    std::fs::write(
        work.join("tests/bad.toml"),
        "module = \"nowhere\"\n[[test]]\nname = \"x\"\ncommand = \"no-such\"\n",
    )
    .unwrap();
    let (ok, checked) = mg(&["plugin", "check", work.to_str().unwrap()]);
    assert!(!ok);
    let faults = checked["plugins"][0]["faults"].to_string();
    assert!(faults.contains("the manifest declares no command \\\"no-such\\\""), "{faults}");
    assert!(faults.contains("its fixture nowhere is not there"), "{faults}");
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
    let out = crate::mg_on(dir.join("no-game"), None)
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
    let out = crate::mg_alone()
        .args(["plugin", "test"])
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

/// `mg` with no user folder of the game's and a data folder of its own
/// (where Moonglow keeps the answers about projects' plugins), so that
/// tests running at once keep apart: `MOONGLOW_DATA_DIR`, on every
/// platform (`mg_module::build::data_dir`).
fn mg_in(home: &Path) -> std::process::Command {
    let mut c = crate::mg_alone();
    c.env("MOONGLOW_DATA_DIR", home.join("data/moonglow"));
    c
}

/// Where `mg_in(home)` keeps the project plugins' answers.
fn answers_in(home: &Path) -> std::path::PathBuf {
    home.join("data/moonglow/project-plugins.toml")
}

fn mg_at(home: &Path, args: &[&str]) -> (bool, Json) {
    crate::quiet_json_out(mg_in(home).arg("--json").args(args))
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
    let out = mg_in(&home).args(["plugin", "list", "--project", project]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("\tthe project's: denied"), "{text}");
    assert!(text.contains("\tsetting\tprefix\tstring\tproject\t\"KRZ_\""), "{text}");
    // A plugin the project has not.
    let out = mg_in(&home).args(["plugin", "allow", project, "nobody.else"]).output().unwrap();
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
        let out = crate::mg_alone()
            .args(["plugin", "run", dir.to_str().unwrap(), plugin.to_str().unwrap(), "prefix-tags"])
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

/// A setting of scope `user` is kept where the window keeps it (the file
/// in Moonglow's data folder it shares with `mg`), and read from there by
/// the next run, by `mg plugin list` and by the window alike.
#[test]
fn a_plugin_s_own_settings_are_kept_with_moonglow_s() {
    let dir = mg_testkit::scratch_dir("mg-plugin-user-settings");
    let home = dir.join("home");
    let path = module(&dir);
    let plugin = dir.join("counter");
    std::fs::create_dir_all(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.cfg"),
        "[plugin]\nid = \"test.counter\"\nname = \"Counter\"\nversion = \"1\"\napi = \"0.5\"\nlicense = \"MIT\"\n\
         \n[command]\nid = \"count\"\ntitle = \"Count\"\n\n[setting]\nid = \"runs\"\ntype = \
         \"int\"\n\n[setting]\nid = \"scale\"\ntype = \"float\"\ndefault = 0.5\n",
    )
    .unwrap();
    std::fs::write(
        plugin.join("main.luau"),
        "local mg = require(\"@moonglow\")\nmg.command(\"count\", function(ctx)\n  \
         print(ctx.settings.runs, ctx.settings.scale)\n  ctx.settings:set(\"runs\", \
         ctx.settings.runs + 1)\n  ctx.settings:set(\"scale\", ctx.settings.scale * 3)\nend)\n",
    )
    .unwrap();
    let run = || {
        let (m, p) = (path.to_str().unwrap(), plugin.to_str().unwrap());
        mg_at(&home, &["plugin", "run", m, p, "count"])
    };
    let (ok, out) = run();
    assert!(ok, "{out}");
    assert_eq!(
        out["settings"][0],
        json!({ "id": "runs", "scope": "user", "value": 1, "kept": true })
    );
    let kept = home.join("data/moonglow/plugin-settings.json");
    let values: Json = serde_json::from_str(&std::fs::read_to_string(&kept).unwrap()).unwrap();
    assert_eq!(values, json!({ "test.counter": { "runs": 1, "scale": 1.5 } }));
    let (ok, out) = run();
    assert!(ok, "{out}");
    assert!(out["log"].to_string().contains("1\\t1.5"), "the values kept, read: {out}");
    let (_, listed) = mg_at(&home, &["plugin", "list", plugin.to_str().unwrap()]);
    assert_eq!(listed["plugins"][0]["settings"][0]["value"], 2, "{listed}");
    assert_eq!(listed["plugins"][0]["settings"][1]["type"], "float");
}

/// `mg plugin tidy`: what is kept wrong of the project's plugins'
/// settings taken out or moved, said first with `--dry-run`; another
/// plugin's table left alone, and said.
#[test]
fn a_project_s_plugin_settings_are_tidied() {
    let dir = project_with_plugins("mg-plugin-tidy");
    let home = dir.join("home");
    let team = dir.join("moonglow.toml");
    std::fs::write(
        &team,
        "[plugins.\"example.builder-tags\"]\nprefix = \"KRZ_\"\nmax_length = 40\nlost = 1\ncode = \
         \"ab\"\n\n[plugins.\"other.plugin\"]\nx = 1\n",
    )
    .unwrap();
    let project = dir.to_str().unwrap();
    let (ok, out) = mg_at(&home, &["plugin", "tidy", project, "--dry-run"]);
    assert!(ok, "{out}");
    assert_eq!(out["plugins"][0]["done"].as_array().unwrap().len(), 3, "{out}");
    assert!(std::fs::read_to_string(&team).unwrap().contains("max_length = 40"));
    let (ok, out) = mg_at(&home, &["plugin", "tidy", project]);
    assert!(ok, "{out}");
    assert!(out["notes"].to_string().contains("there is no plugin other.plugin"), "{out}");
    assert_eq!(
        std::fs::read_to_string(&team).unwrap(),
        "[plugins.\"example.builder-tags\"]\nprefix = \"KRZ_\"\n\n[plugins.\"other.plugin\"]\nx = 1\n"
    );
    let own = std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap();
    assert!(own.contains("code = \"ab\""), "moved where its scope keeps it: {own}");
}

/// A plugin that refuses every save, written into `dir`.
pub(crate) fn refuser(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("plugin.cfg"),
        "[plugin]\nid = \"test.refuser\"\nname = \"Refuser\"\nversion = \"1\"\napi = \"0.4\"\n\
         license = \"MIT\"\n\n[event]\nid = \"no\"\non = \"resource-saving\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.luau"),
        "local mg = require(\"@moonglow\")\n\
         mg.on(\"no\", function(ctx) ctx.event:refuse(\"not today\") end)\n",
    )
    .unwrap();
}

/// A value `mg plugin tidy` moves out of the project's file into the
/// user's own is written there before anything else is tidied: a plugin
/// further on whose tidy fails (here, the builder's own project file
/// can't be written) left it in neither place.
#[cfg(unix)]
#[test]
fn a_setting_moved_by_tidy_is_kept_when_a_later_tidy_fails() {
    use std::os::unix::fs::PermissionsExt;
    let dir = project_with_plugins("mg-plugin-tidy-fails");
    let home = dir.join("home");
    let plugin = |id: &str, setting: &str| {
        let at = dir.join("named").join(id);
        std::fs::create_dir_all(&at).unwrap();
        let cfg = format!(
            "[plugin]\nid = \"{id}\"\nname = \"{id}\"\nversion = \"1\"\napi = \"0.4\"\n\
             license = \"MIT\"\n\n{setting}"
        );
        std::fs::write(at.join("plugin.cfg"), cfg).unwrap();
        std::fs::write(at.join("main.luau"), "local mg = require(\"@moonglow\")\n").unwrap();
        at
    };
    let one = plugin("a.one", "[setting]\nid = \"name\"\nlabel = \"Name\"\nscope = \"user\"\n");
    let two = plugin("b.two", "");
    std::fs::write(dir.join("moonglow.toml"), "[plugins.\"a.one\"]\nname = \"mine\"\n").unwrap();
    let own = dir.join(".moonglow");
    std::fs::create_dir_all(&own).unwrap();
    std::fs::write(own.join("user.toml"), "[plugins.\"b.two\"]\nlost = 1\n").unwrap();
    std::fs::set_permissions(&own, std::fs::Permissions::from_mode(0o555)).unwrap();
    // (Who may write anywhere, the administrator, is not held by it.)
    let held = std::fs::write(own.join("probe"), b"").is_err();
    let project = dir.to_str().unwrap();
    let (ok, out) =
        mg_at(&home, &["plugin", "tidy", project, one.to_str().unwrap(), two.to_str().unwrap()]);
    std::fs::set_permissions(&own, std::fs::Permissions::from_mode(0o755)).unwrap();
    if !held {
        return;
    }
    assert!(!ok && out["error"].as_str().unwrap().starts_with("b.two: "), "{out}");
    // Out of the project's file, and in the user's own.
    assert!(!std::fs::read_to_string(dir.join("moonglow.toml")).unwrap().contains("mine"));
    let mut kept = Vec::new();
    let mut folders = vec![home.clone()];
    while let Some(folder) = folders.pop() {
        for e in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            match e.path().is_dir() {
                true => folders.push(e.path()),
                false => kept.push(std::fs::read_to_string(e.path()).unwrap_or_default()),
            }
        }
    }
    assert!(kept.iter().any(|text| text.contains("a.one") && text.contains("mine")), "{kept:?}");
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
        let mut cmd = crate::mg_alone();
        cmd.args(["--json", "set", dir.to_str().unwrap(), "tavern.are"]);
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
    let out = crate::mg_alone()
        .args(["set", dir.to_str().unwrap(), "tavern.are", "Tag=INN", "--dry-run"])
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
    // --strict: a warning stops the save too.
    let (ok, out) = mg(&[
        "set",
        dir.to_str().unwrap(),
        "tavern.are",
        "Tag=Hall",
        "--strict",
        "--plugins",
        rules.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(out["error"].as_str().unwrap().contains("not saved (--strict): a plugin warns"));
    assert_eq!(area_tag(&dir), "Inn", "nothing written");

    // The project's own plugin, allowed, is asked by mg's other saving
    // commands too (mg replace here): its refusal writes nothing.
    let dir = tavern("mg-plugin-save-replace", "");
    let home = dir.join("home");
    let into = dir.join("plugins/house-rules");
    std::fs::create_dir_all(&into).unwrap();
    for f in std::fs::read_dir(&rules).unwrap().flatten() {
        if f.file_type().unwrap().is_file() {
            std::fs::copy(f.path(), into.join(f.file_name())).unwrap();
        }
    }
    let (ok, out) = mg_at(&home, &["plugin", "allow", dir.to_str().unwrap()]);
    assert!(ok, "{out}");
    let (ok, out) = mg_at(
        &home,
        &["replace", dir.to_str().unwrap(), "TAVERN", "INN", "--only", "tags", "--match-case"],
    );
    assert!(!ok, "{out}");
    assert!(out["error"].as_str().unwrap().contains("not saved: a plugin refuses it"), "{out}");
    assert_eq!(area_tag(&dir), "TAVERN", "nothing written");
}

/// Every command of `mg` that changes a module's resources asks the
/// project's plugins before it saves, as `mg set` does: with a plugin of
/// the project's that refuses every save, each fails saying so and the
/// module's files are as they were (`mg compile`, `mg dialog-import`, `mg
/// attach`, `mg nui new`, `generate` and `import` and `mg rename` of a
/// window saved without asking; `mg attach` copies nothing either).
#[test]
fn every_saving_command_asks_the_project_s_plugins() {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog};
    let root = mg_testkit::corpus!();
    let scratch = mg_testkit::scratch_dir("mg-plugin-save-asked");
    let (dir, home, user) = (scratch.join("keep"), scratch.join("home"), scratch.join("user"));
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.set(key("hello.nss"), b"void main() { }".to_vec());
    let mut greet = new_dialog();
    let hello = add_node(&mut greet, Parent::Root, "Hello.");
    add_node(&mut greet, Parent::Node(Kind::Entry, hello), "Bye.");
    m.set_gff(key("greet.dlg"), &greet).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let p = dir.to_str().unwrap();
    let mg = |args: &[&str]| -> (bool, Json) {
        let mut c = crate::mg_on(&root, Some(&user));
        c.env("MOONGLOW_DATA_DIR", home.join("data/moonglow")).arg("--json").args(args);
        crate::json_out(&mut c)
    };
    // What the commands work on, made while nothing refuses: a window, a
    // conversation's lines to read back, a hak to attach, a window's files.
    let (ok, made) = mg(&["nui", "new", p, "shop"]);
    assert!(ok, "{made}");
    let csv = scratch.join("greet.csv");
    let (ok, made) = mg(&["dialog-export", p, "greet.dlg", csv.to_str().unwrap()]);
    assert!(ok, "{made}");
    let lines = std::fs::read_to_string(&csv).unwrap().replace("Hello.", "Hello there.");
    std::fs::write(&csv, lines).unwrap();
    let hak = scratch.join("mg_asked.hak");
    std::fs::write(&hak, mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap()).unwrap();
    let crier = crate::nui::crier_files("mg-plugin-save-asked-crier");

    refuser(&dir.join("plugins/refuser"));
    let (ok, allowed) = mg(&["plugin", "allow", p]);
    assert!(ok, "{allowed}");
    let files = || {
        let mut all: Vec<(PathBuf, Vec<u8>)> = (std::fs::read_dir(&dir).unwrap().flatten())
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| (e.path(), std::fs::read(e.path()).unwrap()))
            .collect();
        all.sort();
        all
    };
    let before = files();
    let (c, h, j) = (csv.to_str().unwrap(), hak.to_str().unwrap(), crier.to_str().unwrap());
    let commands: [&[&str]; 8] = [
        &["compile", p],
        &["dialog-import", p, c],
        &["attach", p, h],
        &["nui", "new", p, "crier2"],
        &["nui", "generate", p, "shop"],
        &["nui", "import", p, j],
        &["rename", p, "shop.jui", "store"],
        &["set", p, "greet.dlg", "EndConversation=mg_other"],
    ];
    for command in commands {
        let (ok, out) = mg(command);
        let why = out["error"].as_str().unwrap_or_default();
        assert!(!ok && why.contains("not saved: a plugin refuses it"), "{command:?}: {out}");
        assert!(why.contains("Refuser: ") && why.contains("not today"), "{command:?}: {out}");
        assert!(files() == before, "{command:?} wrote to the module");
    }
    assert_eq!(std::fs::read_dir(user.join("hak")).unwrap().count(), 0, "mg attach copied");
}

/// `mg plugin run --event` runs the new events' handlers by hand: a save's
/// (with the module as it was, `--was`), objects pasted (`--select`), a
/// resource made; a refusal fails it.
#[test]
fn a_plugin_s_new_events_run_by_hand() {
    let rules = house_rules();
    let fixture = rules.join("tests/module");
    let run = |more: &[&str]| -> (bool, Json) {
        let out = crate::mg_alone()
            .args(["--json", "plugin", "run"])
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

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples").join(name)
}

/// API 0.6 from the command line: `mg plugin export` writes the file an
/// exporter makes and changes nothing (`--dry-run` writes none either);
/// `mg plugin import` reads a file into the module and saves; `mg plugin
/// list` says what each adds.
#[test]
fn exporters_and_importers_run_from_the_command_line() {
    let dir = mg_testkit::scratch_dir("mg-plugin-export");
    let m = module(&dir);
    let sheet = example("creature-sheet");
    let (m, sheet) = (m.to_str().unwrap(), sheet.to_str().unwrap());
    let out = dir.join("creatures.csv");
    let dry = dir.join("dry.csv");
    let (ok, said) =
        mg(&["plugin", "export", m, sheet, "creatures-csv", dry.to_str().unwrap(), "--dry-run"]);
    assert!(ok && !dry.exists(), "{said}");
    let (ok, said) = mg(&["plugin", "export", m, sheet, "creatures-csv", out.to_str().unwrap()]);
    assert!(ok, "{said}");
    assert_eq!(said["files"], json!([out.display().to_string()]));
    assert!(said["log"][0]["text"].as_str().unwrap().contains("1 creatures written"), "{said}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("guard,gate_guard,,0"), "{text}");
    assert_eq!(tag(Path::new(m)), "gate_guard", "an export changes nothing");
    // What it has not.
    let (ok, said) = mg(&["plugin", "export", m, sheet, "pdf", out.to_str().unwrap()]);
    assert!(!ok && said.to_string().contains("no exporter \\\"pdf\\\""), "{said}");

    let input = dir.join("night.csv");
    std::fs::write(&input, "resref,tag,name\nguard,NIGHT_GUARD,Night\n").unwrap();
    let (ok, said) = mg(&["plugin", "import", m, sheet, "creatures-csv", input.to_str().unwrap()]);
    assert!(ok, "{said}");
    assert_eq!(said["label"], "Import 1 creatures", "{said}");
    assert_eq!(tag(Path::new(m)), "NIGHT_GUARD");
    // A file of another kind is not read.
    let (ok, said) = mg(&["plugin", "import", m, sheet, "creatures-csv", m]);
    assert!(!ok && said.to_string().contains("reads .csv"), "{said}");

    let (ok, listed) = mg(&["plugin", "list", sheet]);
    assert!(ok);
    let p = &listed["plugins"][0];
    assert_eq!(p["exporters"][0]["extension"], "csv");
    assert_eq!(p["exporters"][1]["for"], json!(["creature blueprint"]));
    assert_eq!(p["importers"][0]["extensions"], json!(["csv"]));
}

/// `mg plugin wizard`: each page answered from `--answers` (a JSON file)
/// and `--answer`, over the defaults; its Finish's edits saved. A page's
/// check that objects is an error, and nothing is saved.
#[test]
fn a_wizard_is_answered_from_the_command_line() {
    let dir = mg_testkit::scratch_dir("mg-plugin-wizard");
    let m = module(&dir);
    let wizard = example("store-wizard");
    let (m, wizard) = (m.to_str().unwrap(), wizard.to_str().unwrap());
    let answers = dir.join("answers.json");
    std::fs::write(&answers, r#"{ "name": "Iron Hand", "kind": "Smith", "markup": 170 }"#).unwrap();
    let (ok, said) = mg(&[
        "plugin",
        "wizard",
        m,
        wizard,
        "new-store",
        "--answers",
        answers.to_str().unwrap(),
        "--answer",
        "markup=160",
    ]);
    assert!(ok, "{said}");
    assert_eq!(said["label"], "New store Iron Hand");
    let store = Module::open(Path::new(m)).unwrap().gff(&key("ironhand.utm")).unwrap().unwrap();
    assert_eq!(store.root.get("MarkUp"), Some(&Value::Int(160)), "--answer over --answers");
    let log: Vec<&str> =
        said["log"].as_array().unwrap().iter().filter_map(|l| l["text"].as_str()).collect();
    assert!(log.contains(&"page 3 of 3 (What it buys)"), "{log:?}");

    let (ok, said) =
        mg(&["plugin", "wizard", m, wizard, "new-store", "--answer", "name= ", "--dry-run"]);
    assert!(!ok && said.to_string().contains("A store needs a name."), "{said}");
    let (ok, listed) = mg(&["plugin", "list", wizard]);
    assert!(ok);
    assert_eq!(listed["plugins"][0]["wizards"][0]["title"], "Shop Wizard…");
    let (ok, listed) = mg(&["plugin", "list", example("module-overview").to_str().unwrap()]);
    assert!(ok);
    assert_eq!(listed["plugins"][0]["panels"][0]["refresh"], json!(["edits", "selection"]));
}
