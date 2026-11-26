//! `mg compile` passes over the scripts that are as they were when last
//! compiled, and keeps what it remembers in Moonglow's data folder (where
//! the window keeps the same), never beside the module.

use std::path::Path;

use mg_core::ResType;
use mg_resman::ResKey;
use serde_json::Value;

fn compile(root: &Path, dir: &Path, module: &Path, more: &[&str]) -> Value {
    let out = crate::mg_on(root, Some(&dir.join("user")))
        .args(["--json", "compile"])
        .arg(module)
        .args(more)
        // (A data folder that starts empty: what was compiled before is
        // remembered there.)
        .env("MOONGLOW_DATA_DIR", dir.join("data"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn a_second_compile_passes_over_what_is_unchanged() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-compile");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let at = dir.join("module");
    std::fs::create_dir_all(&at).unwrap();
    let chess = at.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();

    let first = compile(&root, &dir, &chess, &[]);
    let scripts = first["scripts"].as_u64().unwrap();
    assert!(scripts > 10 && first["failed"] == 0 && first["unchanged"] == 0, "{first}");
    // Remembered in the data folder; nothing new beside the module.
    let kept: Vec<_> = walk(&dir.join("data")).into_iter().filter(|p| is_json(p)).collect();
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert!(kept[0].to_string_lossy().contains("compiled"), "{kept:?}");
    assert!(!walk(&at).iter().any(|p| is_json(p)));

    // As they were: none compiled, and the module is not saved or backed up anew.
    let bak = at.join("chess.mod.bak");
    let _ = std::fs::remove_file(&bak);
    let saved_bytes = std::fs::read(&chess).unwrap();
    let again = compile(&root, &dir, &at.join("../module/chess.mod"), &[]);
    assert!(again["scripts"] == 0 && again["unchanged"] == scripts, "{again}");
    assert!(!bak.exists(), "no-op compile should not create .mod.bak");
    assert_eq!(std::fs::read(&chess).unwrap(), saved_bytes);

    // One script changed: it alone, unless others include it.
    let mut m = mg_module::Module::open(&chess).unwrap();
    let key = *(m.keys_of(ResType::NSS))
        .find(|k| m.contains(&ResKey::new(k.resref, ResType::NCS)))
        .expect("a script with a compiled one");
    let mut source = m.get(&key).unwrap().to_vec();
    source.extend_from_slice(b"\n// changed\n");
    m.set(key, source);
    m.save().unwrap();
    let includers = mg_module::refs::includers(&m, &[key]).len() as u64;
    let third = compile(&root, &dir, &chess, &[]);
    assert_eq!(third["scripts"], 1 + includers, "{third}");

    // --all compiles them all, whatever is remembered.
    let all = compile(&root, &dir, &chess, &["--all"]);
    assert!(all["scripts"] == scripts && all["unchanged"] == 0, "{all}");

    // --ndb: those compiled without a debug file are compiled again for
    // one, once.
    // (Of the scripts with a source: one compiled script has none.)
    let debug_files = |m: &mg_module::Module| {
        let made = |t| {
            (m.keys_of(ResType::NSS))
                .filter(|k| m.get(&ResKey::new(k.resref, t)).is_some_and(|d| !d.is_empty()))
                .count()
        };
        (made(ResType::NCS), made(ResType::NDB))
    };
    assert_eq!(debug_files(&mg_module::Module::open(&chess).unwrap()).1, 0);
    let with = compile(&root, &dir, &chess, &["--ndb"]);
    let (ncs, ndb) = debug_files(&mg_module::Module::open(&chess).unwrap());
    assert!(ncs > 10 && ndb == ncs, "{ncs} compiled scripts, {ndb} debug files");
    assert_eq!(with["scripts"], ncs as u64, "{with}");
    let again = compile(&root, &dir, &chess, &["--ndb"]);
    assert_eq!(again["scripts"], 0, "{again}");
}

/// `mg compile --uncompiled` and `mg build` remember what they compiled
/// where `mg compile` (and the window) do: the next compile passes over
/// it. A project is one to them whether named by its folder or by its
/// `nasher.cfg`.
/// `--json` says, for each script compiled, why it was and whether its
/// compiled script changed.
#[test]
fn json_says_why_each_script_was_compiled() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-compile-why");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let why = |v: &Value, script: &str| -> (String, bool) {
        let e = v["compiled_scripts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["script"] == script)
            .unwrap_or_else(|| panic!("{script} in {v}"));
        (e["why"].as_str().unwrap().to_string(), e["changed"].as_bool().unwrap())
    };
    // Nothing remembered: each has its compiled script from the module, which is
    // not what Moonglow made.
    let first = compile(&root, &dir, &chess, &[]);
    let list = first["compiled_scripts"].as_array().unwrap();
    assert_eq!(list.len() as u64, first["scripts"].as_u64().unwrap(), "{first}");
    assert!(list.iter().all(|e| e["why"].is_string() && e["ok"] == true), "{first}");
    assert!(
        list.iter().all(|e| e["why"] == "not-compiled-here" || e["why"] == "no-compiled-script")
    );
    // An edit that leaves the compiled script as it was: the source changed,
    // the bytes did not.
    let mut m = mg_module::Module::open(&chess).unwrap();
    let key = *(m.keys_of(ResType::NSS))
        .find(|k| m.contains(&ResKey::new(k.resref, ResType::NCS)))
        .expect("a script with a compiled one");
    let mut source = m.get(&key).unwrap().to_vec();
    source.extend_from_slice(b"\n// changed\n");
    m.set(key, source);
    m.save().unwrap();
    let third = compile(&root, &dir, &chess, &[]);
    assert_eq!(
        why(&third, &key.resref.to_string()),
        ("source-changed".to_string(), false),
        "{third}"
    );
}

#[test]
fn uncompiled_and_build_remember_what_they_compiled() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-compile-remembered");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let mut m = mg_module::Module::open(&chess).unwrap();
    m.set(ResKey::parse("fresh", ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    m.save().unwrap();
    // (Those without a compiled script: the new one, and include files.)
    let none = (m.keys_of(ResType::NSS))
        .filter(|k| !m.contains(&ResKey::new(k.resref, ResType::NCS)))
        .count() as u64;

    let first = compile(&root, &dir, &chess, &["--uncompiled"]);
    assert_eq!(first["scripts"], none, "{first}");
    // The others were never compiled here: all but those just compiled.
    let next = compile(&root, &dir, &chess, &[]);
    assert_eq!(next["unchanged"], none, "{next}");

    let kept = || walk(&dir.join("data")).into_iter().filter(|p| is_json(p)).count();
    assert_eq!(kept(), 1);
    let project = dir.join("project");
    let mg = |args: &[&str]| {
        let out = crate::mg_on(&root, Some(&dir.join("user")))
            .args(args)
            .env("MOONGLOW_DATA_DIR", dir.join("data"))
            .output()
            .unwrap();
        assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    };
    mg(&["init", chess.to_str().unwrap(), project.to_str().unwrap()]);
    let built = dir.join("built.mod");
    mg(&["build", project.to_str().unwrap(), "-o", built.to_str().unwrap()]);
    assert_eq!(kept(), 2, "mg build remembers what it compiled");
    // (A project's tree keeps no compiled scripts: they are made again.)
    let by_cfg = compile(&root, &dir, &project.join("nasher.cfg"), &[]);
    assert!(by_cfg["scripts"].as_u64().unwrap() > 10, "{by_cfg}");
    assert_eq!(kept(), 2, "named by its nasher.cfg, the same project");
}

/// Scripts that include the base game's `x2_inc_cutscene` (its
/// `CutCreateObjectCopy` is declared `object` and defined `void`) compile,
/// as in Aurora (El Grillo's report: they failed): the warning is said
/// once, with how many scripts it was met in, on standard error and in
/// the JSON.
#[test]
fn x2_inc_cutscene_s_includers_compile_with_one_warning() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-compile-return-types");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let mut m = mg_module::Module::open(&chess).unwrap();
    for (name, main) in [
        ("cut_a", "{}"),
        (
            "cut_b",
            "{ object o = CutCreateObjectCopy(0.0, OBJECT_SELF, GetLocation(OBJECT_SELF)); }",
        ),
    ] {
        let text = format!("#include \"x2_inc_cutscene\"\nvoid main() {main}\n");
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), text.into_bytes());
    }
    m.save().unwrap();
    let json = compile(&root, &dir, &chess, &["--all"]);
    assert_eq!(json["failed"], 0, "{json}");
    let warnings = json["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{json}");
    let w = &warnings[0];
    assert_eq!(
        (&w["script"], &w["line"], &w["scripts"]),
        (&"x2_inc_cutscene".into(), &628.into(), &2.into())
    );
    assert!(w["message"].as_str().unwrap().contains("declared object at x2_inc_cutscene.nss(303)"));
    let m = mg_module::Module::open(&chess).unwrap();
    assert!(m.contains(&ResKey::parse("cut_b", ResType::NCS).unwrap()));

    // As text: on standard error, once.
    let out = crate::mg_on(&root, Some(&dir.join("user")))
        .args(["compile", "--all"])
        .arg(&chess)
        .env("MOONGLOW_DATA_DIR", dir.join("data"))
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{said}");
    assert_eq!(said.matches("WARNING: CutCreateObjectCopy").count(), 1, "{said}");
    assert!(said.contains("(in 2 scripts)"), "{said}");
}

/// An include's `main` that Aurora's Build compiles in place of the
/// script's own (the owner's ruling of 2026-11-26: Moonglow keeps the
/// script's, and says so): a warning for each such script, on standard
/// error and in the JSON.
#[test]
fn an_includes_main_is_warned_of() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-compile-include-main");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let mut m = mg_module::Module::open(&chess).unwrap();
    for (name, text) in [
        ("mg_inc_lib", "void Say() { PrintString(\"x\"); }\nvoid main() { Say(); }\n"),
        ("mg_cond", "#include \"mg_inc_lib\"\nint StartingConditional() { return TRUE; }\n"),
        ("mg_only", "#include \"mg_inc_lib\"\n"),
    ] {
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), text.as_bytes().to_vec());
    }
    m.save().unwrap();
    let json = compile(&root, &dir, &chess, &["--all"]);
    assert_eq!(json["failed"], 0, "{json}");
    let warnings: Vec<&str> = (json["warnings"].as_array().unwrap().iter())
        .map(|w| w["message"].as_str().unwrap())
        .filter(|w| w.starts_with("mg_inc_lib.nss(2): WARNING"))
        .collect();
    assert_eq!(warnings.len(), 2, "{json}");
    assert!(warnings.iter().any(|w| w.contains("compiles as mg_cond in place of its")));
    assert!(warnings.iter().any(|w| w.contains("Aurora compiles mg_inc_lib's main as mg_only")));

    let out = crate::mg_on(&root, Some(&dir.join("user")))
        .args(["compile", "--all"])
        .arg(&chess)
        .env("MOONGLOW_DATA_DIR", dir.join("data"))
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{said}");
    assert_eq!(said.matches("WARNING: mg_inc_lib has a main").count(), 1, "{said}");
    assert_eq!(said.matches("WARNING: mg_only has no main").count(), 1, "{said}");
}

fn is_json(p: &Path) -> bool {
    p.extension().is_some_and(|e| e == "json")
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        match e.path() {
            p if p.is_dir() => out.extend(walk(&p)),
            p => out.push(p),
        }
    }
    out
}

/// A script that does not compile fails the command: what compiled is
/// saved and the JSON says which did not and why, but the exit is a
/// failure, so that a pipeline does not go on with the module.
#[test]
fn a_script_that_does_not_compile_fails_the_command() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-compile-fails");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let path = dir.join("broken.mod");
    let mut m = mg_module::Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let script = |name: &str| ResKey::parse(name, ResType::NSS).unwrap();
    m.set(script("good"), b"void main() { }".to_vec());
    m.set(script("bad"), b"void main() { int n = ; }".to_vec());
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    for more in [&[][..], &["--uncompiled"][..]] {
        let out = crate::mg_on(&root, Some(&dir.join("user")))
            .args(["--json", "compile"])
            .arg(&path)
            .args(more)
            .env("MOONGLOW_DATA_DIR", dir.join("data"))
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(!out.status.success(), "{more:?}: {text}");
        let json: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(json["failed"], 1, "{json}");
        assert!(json["errors"][0].to_string().contains("bad"), "{json}");
        // What compiled is in the module all the same.
        let m = mg_module::Module::open(&path).unwrap();
        assert!(m.contains(&ResKey::parse("good", ResType::NCS).unwrap()));
        assert!(!m.contains(&ResKey::parse("bad", ResType::NCS).unwrap()));
    }
    // Without the one that fails: a success.
    let mut m = mg_module::Module::open(&path).unwrap();
    m.remove(&script("bad"));
    m.save().unwrap();
    assert_eq!(compile(&root, &dir, &path, &["--all"])["failed"], 0);
}
