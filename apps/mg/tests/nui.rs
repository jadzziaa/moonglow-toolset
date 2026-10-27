//! `mg nui`, on a module made here: a window is added, checked and its
//! scripts written and compiled; `validate` changes nothing and reports as
//! `mg verify` does; what fails, and `--dry-run`, leave the module as it
//! was.

use std::path::{Path, PathBuf};

use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use serde_json::{Value, json};

/// An empty module in a scratch folder of its own.
fn module(test: &str) -> PathBuf {
    let path = mg_testkit::scratch_dir(test).join("keep.mod");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

fn command(root: Option<&Path>, args: &[&str]) -> std::process::Command {
    let mut mg = crate::mg();
    if let Some(root) = root {
        mg.arg("--root").arg(root);
    }
    mg.arg("--no-user-dir").args(args);
    mg
}

fn output(root: Option<&Path>, args: &[&str]) -> std::process::Output {
    command(root, args).output().unwrap()
}

/// `mg --json nui …`: whether it succeeded, and what it printed. Nothing
/// goes to standard error.
fn mg(root: Option<&Path>, args: &[&str]) -> (bool, Value) {
    let args: Vec<&str> = ["--json", "nui"].iter().chain(args).copied().collect();
    crate::quiet_json_out(&mut command(root, &args))
}

#[test]
fn a_new_window_is_saved_and_passes_the_checks() {
    let path = module("mg-nui-new");
    let p = path.to_str().unwrap();
    let (ok, made) = mg(None, &["new", p, "shop"]);
    assert!(ok, "{made}");
    assert_eq!(made["resources"], json!(["shop.jui", "shop.txt"]));
    assert_eq!((&made["saved"], &made["dry_run"]), (&json!(true), &json!(false)));
    let m = Module::open(&path).unwrap();
    assert!(m.contains(&key("shop.jui")) && m.contains(&key("shop.txt")));

    let (ok, checked) = mg(None, &["validate", p, "shop"]);
    assert!(ok, "{checked}");
    assert_eq!((&checked["errors"], &checked["warnings"]), (&json!(0), &json!(0)));
    assert_eq!(checked["findings"], json!([]));
}

#[test]
fn new_refuses_a_window_the_module_has_and_a_name_too_long() {
    let path = module("mg-nui-refused");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    let before = std::fs::read(&path).unwrap();
    let (ok, again) = mg(None, &["new", p, "shop"]);
    assert!(!ok && again["error"].as_str().unwrap().contains("shop.jui"), "{again}");
    // (Fourteen characters at most: the scripts' names add two.)
    let (ok, long) = mg(None, &["new", p, "a_name_of_15_ch"]);
    assert!(!ok && long["error"].as_str().unwrap().contains("a_name_of_15_ch"), "{long}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn validate_reports_problems_as_verify_does_and_changes_nothing() {
    let path = module("mg-nui-validate");
    let p = path.to_str().unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(
        key("broken.jui"),
        br#"{"version": 2, "root": {"type": "row", "children": []}}"#.to_vec(),
    );
    m.save().unwrap();
    let before = std::fs::read(&path).unwrap();

    let (ok, checked) = mg(None, &["validate", p, "broken"]);
    assert!(!ok, "{checked}");
    assert!(checked["errors"].as_u64().unwrap() >= 1, "{checked}");
    let findings = checked["findings"].as_array().unwrap();
    assert_eq!(
        findings.len() as u64,
        checked["errors"].as_u64().unwrap() + checked["warnings"].as_u64().unwrap()
    );
    let version = findings.iter().find(|f| f["path"] == "/version").unwrap();
    assert_eq!(version["severity"], "error");
    assert!(version["message"].as_str().unwrap().contains("version 1"));
    assert_eq!(std::fs::read(&path).unwrap(), before);

    // For people: a line each on standard output, and the failure said.
    let out = output(None, &["nui", "validate", p, "broken"]);
    assert!(!out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.lines().any(|l| l.starts_with("error\t/version: ")), "{text}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("broken.jui has errors"));

    let (ok, missing) = mg(None, &["validate", p, "nowhere"]);
    assert!(!ok && missing["error"].as_str().unwrap().contains("nowhere.jui"), "{missing}");
}

#[test]
fn validate_checks_a_window_of_any_name() {
    let path = module("mg-nui-any-name");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    // Sixteen characters: a name `new` would not give one.
    let mut m = Module::open(&path).unwrap();
    for ext in ["jui", "txt"] {
        let data = m.get(&key(&format!("shop.{ext}"))).unwrap().to_vec();
        m.set(key(&format!("a_name_of_16_chr.{ext}")), data);
    }
    m.save().unwrap();
    let (ok, checked) = mg(None, &["validate", p, "a_name_of_16_chr"]);
    assert!(ok && checked["errors"] == 0, "{checked}");
    // A name no resource has is an error said, not a crash.
    let (ok, bad) = mg(None, &["validate", p, "a_name_longer_than_16"]);
    assert!(!ok && bad["error"].is_string(), "{bad}");
}

#[test]
fn generate_writes_the_scripts_and_compiles_the_event_script() {
    let root = mg_testkit::corpus!();
    let path = module("mg-nui-generate");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    let (ok, made) = mg(Some(&root), &["generate", p, "shop"]);
    assert!(ok, "{made}");
    assert_eq!(made["saved"], true);
    let resources = made["resources"].as_array().unwrap();
    assert!(resources.contains(&json!("shop_o.nss")) && resources.contains(&json!("shop_e.ncs")));
    // The opener is an include: it has no compiled script of its own.
    let m = Module::open(&path).unwrap();
    assert!(m.contains(&key("shop_o.nss")) && m.contains(&key("shop_e.nss")));
    assert!(m.contains(&key("shop_e.ncs")) && !m.contains(&key("shop_o.ncs")));

    // For people: the resources and a summary on standard output, and
    // nothing on standard error.
    let out = output(Some(&root), &["nui", "generate", p, "shop", "--dry-run"]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.lines().last().unwrap().starts_with("shop: would change "), "{text}");
}

#[test]
fn a_second_generate_changes_nothing() {
    let root = mg_testkit::corpus!();
    let path = module("mg-nui-again");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    assert!(mg(Some(&root), &["generate", p, "shop"]).0);
    let before = std::fs::read(&path).unwrap();
    let (ok, again) = mg(Some(&root), &["generate", p, "shop"]);
    assert!(ok, "{again}");
    assert_eq!((&again["resources"], &again["saved"]), (&json!([]), &json!(false)));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_generate_that_fails_saves_nothing() {
    let root = mg_testkit::corpus!();
    let path = module("mg-nui-failed");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    assert!(mg(Some(&root), &["generate", p, "shop"]).0);
    // The event script, written over with one that isn't the window's.
    let mut m = Module::open(&path).unwrap();
    m.set(key("shop_e.nss"), b"void main() { InvalidFunction(); }".to_vec());
    m.save().unwrap();
    let before = std::fs::read(&path).unwrap();
    let (ok, failed) = mg(Some(&root), &["generate", p, "shop"]);
    assert!(!ok, "{failed}");
    let error = failed["error"].as_str().unwrap();
    assert!(error.contains("keep.mod: window shop: "), "{error}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_dry_run_lists_the_resources_and_writes_nothing() {
    let root = mg_testkit::corpus!();
    let path = module("mg-nui-dry-run");
    let p = path.to_str().unwrap();
    let empty = std::fs::read(&path).unwrap();
    let (ok, new) = mg(None, &["new", p, "shop", "--dry-run"]);
    assert!(ok, "{new}");
    assert_eq!(new["resources"], json!(["shop.jui", "shop.txt"]));
    assert_eq!((&new["saved"], &new["dry_run"]), (&json!(false), &json!(true)));
    assert_eq!(std::fs::read(&path).unwrap(), empty);

    assert!(mg(None, &["new", p, "shop"]).0);
    let before = std::fs::read(&path).unwrap();
    let (ok, made) = mg(Some(&root), &["generate", p, "shop", "--dry-run"]);
    assert!(ok, "{made}");
    assert!(made["resources"].as_array().unwrap().contains(&json!("shop_e.ncs")), "{made}");
    assert_eq!((&made["saved"], &made["dry_run"]), (&json!(false), &json!(true)));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn code_that_does_not_compile_in_a_handlers_section_saves_nothing() {
    let root = mg_testkit::corpus!();
    let path = module("mg-nui-bad-section");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    assert!(mg(Some(&root), &["generate", p, "shop"]).0);
    // Bad code between the markers of the Close button's handler, a
    // change to the window, and a script that includes the opener: none
    // of what Build would write for them may be saved.
    let mut m = Module::open(&path).unwrap();
    let script = String::from_utf8(m.get(&key("shop_e.nss")).unwrap().to_vec()).unwrap();
    let broken =
        mg_nui::insert_code(&script, "click mg_close", "        NoSuchFunction(oPlayer);\n")
            .unwrap();
    m.set(key("shop_e.nss"), broken.into_bytes());
    let mut window: Value = serde_json::from_slice(m.get(&key("shop.jui")).unwrap()).unwrap();
    window["title"] = json!("Another title");
    m.set(key("shop.jui"), serde_json::to_vec(&window).unwrap());
    m.set(
        key("my_enter.nss"),
        b"#include \"shop_o\"\nvoid main() { Open_shop(GetEnteringObject()); }\n".to_vec(),
    );
    m.save().unwrap();
    let before = std::fs::read(&path).unwrap();
    let (ok, failed) = mg(Some(&root), &["generate", p, "shop"]);
    assert!(!ok, "{failed}");
    let error = failed["error"].as_str().unwrap();
    assert!(error.contains("keep.mod: window shop: ") && error.contains("shop_e.nss("), "{error}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!Module::open(&path).unwrap().contains(&key("my_enter.ncs")));
}

#[test]
fn a_script_that_includes_the_opener_and_fails_is_printed_and_the_window_is_built() {
    let root = mg_testkit::corpus!();
    let path = module("mg-nui-bad-caller");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    let mut m = Module::open(&path).unwrap();
    m.set(
        key("my_broken.nss"),
        b"#include \"shop_o\"\nvoid main() { Open_shop(NoSuchFunction()); }\n".to_vec(),
    );
    m.set(key("my_broken.ncs"), b"NCS as it was".to_vec());
    m.set(
        key("my_enter.nss"),
        b"#include \"shop_o\"\nvoid main() { Open_shop(GetEnteringObject()); }\n".to_vec(),
    );
    m.save().unwrap();
    // As `mg compile` does of a script that fails: said, the rest saved.
    let (ok, made) = mg(Some(&root), &["generate", p, "shop"]);
    assert!(ok, "{made}");
    assert_eq!((&made["saved"], &made["failed"]), (&json!(true), &json!(1)), "{made}");
    assert_eq!(made["errors"][0]["script"], "my_broken", "{made}");
    let resources = made["resources"].as_array().unwrap();
    assert!(resources.contains(&json!("my_enter.ncs")), "{made}");
    assert!(!resources.contains(&json!("my_broken.ncs")), "{made}");
    let m = Module::open(&path).unwrap();
    assert!(m.contains(&key("shop_e.ncs")) && m.contains(&key("my_enter.ncs")));
    assert_eq!(m.get(&key("my_broken.ncs")).unwrap(), b"NCS as it was");
    // For people: the compiler's line on standard output.
    let out = output(Some(&root), &["nui", "generate", p, "shop", "--dry-run"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.lines().any(|l| l.starts_with("my_broken.nss(")), "{text}");
}

/// What `mg nui generate` compiled, `mg compile` passes over: both keep
/// what they compiled in one place.
#[test]
fn compile_passes_over_what_generate_compiled() {
    let root = mg_testkit::corpus!();
    // (A module no earlier run compiled: what is remembered goes by its path.)
    let made = module("mg-nui-remembered");
    let stamp = std::time::SystemTime::UNIX_EPOCH.elapsed().unwrap().as_nanos();
    let path = made.with_file_name(format!("fresh{stamp}.mod"));
    std::fs::rename(&made, &path).unwrap();
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    assert!(mg(Some(&root), &["generate", p, "shop"]).0);
    let out = output(Some(&root), &["--json", "compile", p]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let compiled: Value = serde_json::from_slice(&out.stdout).unwrap();
    // (The opener is looked at, an include: the event script is not.)
    assert_eq!(
        (&compiled["scripts"], &compiled["unchanged"]),
        (&json!(1), &json!(1)),
        "{compiled}"
    );
}

/// A window whose events go to the module's own event script: generate
/// writes its event script as an include (nothing compiled of its own),
/// compiles it through a caller and compiles the module's dispatcher that
/// includes it; a second generate changes nothing.
#[test]
fn generate_makes_an_event_function_the_module_s_script_compiles_with() {
    let root = mg_testkit::corpus!();
    let path = module("mg-nui-event-function");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    let mut m = Module::open(&path).unwrap();
    let mut s = mg_nui::Settings::parse(m.get(&key("shop.txt")).unwrap()).unwrap();
    s.event_function = true;
    m.set(key("shop.txt"), s.bytes());
    m.set(
        key("mod_nui.nss"),
        b"#include \"shop_e\"\nvoid main()\n{\n    if (shop_OnEvent()) return;\n}\n".to_vec(),
    );
    m.save().unwrap();
    let (ok, made) = mg(Some(&root), &["generate", p, "shop"]);
    assert!(ok, "{made}");
    let m = Module::open(&path).unwrap();
    let events = String::from_utf8(m.get(&key("shop_e.nss")).unwrap().to_vec()).unwrap();
    assert!(events.contains("int shop_OnEvent()"), "{events}");
    assert!(!m.contains(&key("shop_e.ncs")) && m.contains(&key("mod_nui.ncs")));
    let opener = String::from_utf8(m.get(&key("shop_o.nss")).unwrap().to_vec()).unwrap();
    assert!(opener.contains("NuiCreate(oPlayer, JsonParse(sLayout), \"shop\");"), "{opener}");
    let before = std::fs::read(&path).unwrap();
    let (ok, again) = mg(Some(&root), &["generate", p, "shop"]);
    assert!(ok && again["resources"] == json!([]), "{again}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

/// A handler that ends early with `return;`, written while the window had
/// a script of its own, compiles once the window's events go to the
/// module's own event script: generate makes it `return TRUE;` (what
/// `NAME_OnEvent()` returns) and says where; switched back, `return;`
/// again. The Domains of Dread's trial failed here on the compiler's
/// "RETURN TYPE AND FUNCTION TYPE MISMATCHED".
#[test]
fn generate_fits_a_handler_s_return_to_the_event_script_s_function() {
    let root = mg_testkit::corpus!();
    let path = module("mg-nui-event-function-return");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    assert!(mg(Some(&root), &["generate", p, "shop"]).0);
    let mut m = Module::open(&path).unwrap();
    let mut s = mg_nui::Settings::parse(m.get(&key("shop.txt")).unwrap()).unwrap();
    let script = String::from_utf8(m.get(&key("shop_e.nss")).unwrap().to_vec()).unwrap();
    let early =
        "        if (!GetIsPC(oPlayer)) return;\n        SendMessageToPC(oPlayer, \"Bought.\");\n";
    let mine = mg_nui::insert_code(&script, "click mg_close", early).unwrap();
    m.set(key("shop_e.nss"), mine.clone().into_bytes());
    s.event_function = true;
    m.set(key("shop.txt"), s.bytes());
    m.save().unwrap();
    let (ok, made) = mg(Some(&root), &["generate", p, "shop"]);
    assert!(ok, "{made}");
    let m = Module::open(&path).unwrap();
    let events = String::from_utf8(m.get(&key("shop_e.nss")).unwrap().to_vec()).unwrap();
    let line = events.lines().position(|l| l.contains("GetIsPC(oPlayer)) return TRUE;")).unwrap();
    assert_eq!(
        made["returns"],
        json!([{"section": "click mg_close", "line": line + 1, "was": "return;", "now": "return TRUE;"}])
    );
    let note = made["notes"][0].as_str().unwrap();
    assert!(note.starts_with(&format!("note: shop_e.nss:{}: ", line + 1)), "{note}");
    assert!(note.contains("\"click mg_close\"") && note.contains("shop_OnEvent()"), "{note}");
    // And back to a script of its own: the code as it was written.
    let mut s = mg_nui::Settings::parse(m.get(&key("shop.txt")).unwrap()).unwrap();
    s.event_function = false;
    let mut m = m;
    m.set(key("shop.txt"), s.bytes());
    m.save().unwrap();
    let out = output(Some(&root), &["nui", "generate", p, "shop"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("is now return;") && said.contains("void main"), "{said}");
    let m = Module::open(&path).unwrap();
    assert_eq!(m.get(&key("shop_e.nss")).unwrap(), mine.as_bytes());
    assert!(m.contains(&key("shop_e.ncs")));
}

/// A nasher project whose sources are scripts and JSON, as `nasher init`
/// writes them (and The Domains of Dread's were): a window made in it is
/// said to be left out, with the line to add, before validate can only
/// say it is not in the module; validate then says why. `mg checks` lists
/// the check Verify makes of it.
#[test]
fn a_nasher_project_that_leaves_windows_out_is_told_the_line() {
    let root = mg_testkit::scratch_dir("mg-nui-nasher");
    let cfg = "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
               [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n";
    std::fs::write(root.join("nasher.cfg"), cfg).unwrap();
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.save_as(&ModuleLocation::Project { root: root.clone(), target: "m".into() }).unwrap();
    let p = root.to_str().unwrap();
    let line = "include = \"src/**/*.{jui,txt}\"";

    let out = output(None, &["--json", "nui", "new", p, "crier"]);
    assert!(out.status.success());
    let made: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(made["include"], line, "{made}");
    assert_eq!(made["left_out"], json!(["src/crier.jui", "src/crier.txt"]));
    assert!(made["notes"][0].as_str().unwrap().contains(line), "{made}");
    assert!(root.join("src/crier.jui").is_file());

    let (ok, checked) = mg(None, &["validate", p, "crier"]);
    let error = checked["error"].as_str().unwrap();
    assert!(!ok && error.contains("crier.jui is not in the module: src/crier.jui"), "{error}");
    assert!(error.contains(line), "{error}");

    // With the line added, the window is read and passes.
    std::fs::write(root.join("nasher.cfg"), cfg.replace("{nss,json}", "{nss,json,jui,txt}"))
        .unwrap();
    let (ok, checked) = mg(None, &["validate", p, "crier"]);
    assert!(ok && checked["warnings"] == 0, "{checked}");

    let out = output(None, &["--json", "checks"]);
    let checks: Value = serde_json::from_slice(&out.stdout).unwrap();
    let nui = checks["checks"].as_array().unwrap().iter().find(|c| c["id"] == "nui-sources");
    assert_eq!(nui.unwrap()["group"], "module");
}

/// `mg nui import` from a window's file, a folder, an ERF and a zip: the
/// layout, the settings and the event script's source come into the
/// module, nothing else; one the module has differently is refused but
/// with --replace; a dry run changes nothing.
#[test]
fn import_brings_windows_from_files_folders_erfs_and_zips() {
    let jui = crier_files("mg-nui-import-files");
    let dir = jui.parent().unwrap().to_path_buf();
    std::fs::write(dir.join("crier_e.nss"), b"// mine\nvoid main() {}\n").unwrap();
    std::fs::write(dir.join("crier_o.nss"), b"// generated elsewhere\n").unwrap();
    let path = module("mg-nui-import");
    let p = path.to_str().unwrap();
    let before = std::fs::read(&path).unwrap();
    let (ok, dry) = mg(None, &["import", p, jui.to_str().unwrap(), "--dry-run"]);
    assert!(ok, "{dry}");
    assert_eq!(dry["resources"], json!(["crier.jui", "crier.txt", "crier_e.nss"]));
    assert_eq!(std::fs::read(&path).unwrap(), before);

    let (ok, made) = mg(None, &["import", p, jui.to_str().unwrap()]);
    assert!(ok, "{made}");
    assert_eq!((&made["saved"], &made["windows"][0]["errors"]), (&json!(true), &json!(0)));
    let m = Module::open(&path).unwrap();
    assert_eq!(m.get(&key("crier.jui")).unwrap(), std::fs::read(&jui).unwrap());
    assert!(m.contains(&key("crier_e.nss")) && !m.contains(&key("crier_o.nss")));
    assert!(mg(None, &["validate", p, "crier"]).0);
    // The same again changes nothing; another copy is refused, then replaces.
    let (ok, again) = mg(None, &["import", p, dir.to_str().unwrap()]);
    assert!(ok && again["resources"] == json!([]), "{again}");
    std::fs::write(&jui, std::fs::read_to_string(&jui).unwrap().replace("Town Crier", "Crier"))
        .unwrap();
    let (ok, refused) = mg(None, &["import", p, jui.to_str().unwrap()]);
    let error = refused["error"].as_str().unwrap();
    assert!(!ok && error.contains("crier.jui") && error.contains("--replace"), "{error}");
    let (ok, replaced) = mg(None, &["import", p, jui.to_str().unwrap(), "--replace"]);
    assert!(ok && replaced["resources"] == json!(["crier.jui"]), "{replaced}");

    // From an ERF (a module archive of windows) and a zip, into others.
    let archive = dir.join("windows.mod");
    let mut windows = Module::new();
    windows.set(key("crier.jui"), std::fs::read(&jui).unwrap());
    windows.set(key("crier.txt"), std::fs::read(dir.join("crier.txt")).unwrap());
    windows.save_as(&ModuleLocation::Archive(archive.clone())).unwrap();
    let into = module("mg-nui-import-erf");
    let (ok, from_erf) = mg(None, &["import", into.to_str().unwrap(), archive.to_str().unwrap()]);
    assert!(ok && from_erf["resources"] == json!(["crier.jui", "crier.txt"]), "{from_erf}");
    let zipped = dir.join("windows.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&zipped).unwrap());
    for (name, file) in [("ui/crier.jui", "crier.jui"), ("ui/crier.txt", "crier.txt")] {
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut zip, &std::fs::read(dir.join(file)).unwrap()).unwrap();
    }
    zip.finish().unwrap();
    let into = module("mg-nui-import-zip");
    let (ok, from_zip) = mg(None, &["import", into.to_str().unwrap(), zipped.to_str().unwrap()]);
    assert!(ok && from_zip["resources"] == json!(["crier.jui", "crier.txt"]), "{from_zip}");
    // Nothing that is a window: said.
    let empty = module("mg-nui-import-none");
    let (ok, none) = mg(None, &["import", p, empty.to_str().unwrap()]);
    assert!(!ok && none["error"].as_str().unwrap().contains("no NUI window"), "{none}");
}

/// `mg nui schema`: the settings' form as JSON Schema, the same as text and
/// with --json; with --layout, the controls as the Creator makes them.
#[test]
fn schema_prints_the_settings_form_and_the_controls() {
    let (ok, schema) = mg(None, &["schema"]);
    assert!(ok, "{schema}");
    assert_eq!(schema["properties"]["format"]["const"], "moonglow.nui/1");
    assert_eq!(schema, mg_nui::settings_schema());
    let out = output(None, &["nui", "schema"]);
    assert!(out.status.success());
    let text: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(text, schema);
    let (ok, layout) = mg(None, &["schema", "--layout"]);
    assert!(ok);
    assert_eq!(layout["controls"]["textedit"]["max"], 255);
    assert_eq!(layout["window"]["version"], 1);
}

/// The town crier's window (`crates/mg-nui/tests/data`), as files in a
/// scratch folder of its own: `crier.jui` and `crier.txt`.
fn crier_files(test: &str) -> PathBuf {
    window_files(test, "crier")
}

/// A window of `crates/mg-nui/tests/data` as files in a scratch folder:
/// NAME.jui, NAME.txt and the binds a script would set, NAME.binds.json.
fn window_files(test: &str, name: &str) -> PathBuf {
    let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/mg-nui/tests/data");
    let dir = mg_testkit::scratch_dir(test);
    for ext in ["jui", "txt", "binds.json"] {
        let file = format!("{name}.{ext}");
        std::fs::copy(from.join(&file), dir.join(&file)).unwrap();
    }
    dir.join(format!("{name}.jui"))
}

/// `layout` and `render` draw a window in a state asked for, with binds
/// from a file and a swap layout variant in its group; the same options
/// give both the same layout. What doesn't fit the window is refused, or
/// said.
#[cfg(feature = "nui-preview")]
#[test]
fn layout_and_render_take_states_binds_and_a_variant() {
    let jui = window_files("mg-nui-layout-options", "cries");
    let file = jui.to_str().unwrap();
    let binds = jui.with_file_name("cries.binds.json");
    let binds = binds.to_str().unwrap();
    let options = ["--binds", binds, "--state", "hover:read", "--state", "disabled"];
    let (ok, layout) = mg(None, &[&["layout", file][..], &options].concat());
    assert!(ok, "{layout}");
    let elements = layout["elements"].as_array().unwrap();
    let read: Vec<_> = elements.iter().filter(|e| e["id"] == "read").collect();
    assert_eq!(read.len(), 4, "{layout}");
    assert!(read.iter().all(|e| e["state"] == "hover"), "{layout}");
    let list = elements.iter().find(|e| e["id"] == "cries").unwrap();
    assert_eq!(list["state"], "disabled");
    let row = elements.iter().find(|e| e["row"] == 3 && e["kind"] == "label").unwrap();
    assert_eq!(row["text"], "A traveler");
    // As it opens, nothing has a state.
    let (_, first) = mg(None, &["layout", file]);
    assert!(first["elements"].as_array().unwrap().iter().all(|e| e.get("state").is_none()));
    // The variant, in the swap layout it was made for.
    let (ok, details) =
        mg(None, &["layout", file, "--variant", "details", "--state", "pressed:back"]);
    assert!(ok, "{details}");
    let back = details["elements"].as_array().unwrap().iter().find(|e| e["id"] == "back");
    assert_eq!(back.unwrap()["state"], "pressed", "{details}");
    // For people: the state on the control's line.
    let out = output(None, &[&["nui", "layout", file][..], &options].concat());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.lines().any(|l| l.contains("#read\t") && l.ends_with("\thover")), "{text}");

    // Refused: an ID no control has, a variant there isn't, a state that
    // isn't one or is given twice, binds that aren't an object of JSON.
    let error = |args: &[&str]| {
        let (ok, said) = mg(None, &[&["layout", file][..], args].concat());
        assert!(!ok, "{args:?}: {said}");
        said["error"].as_str().unwrap().to_owned()
    };
    assert!(error(&["--state", "hover:nope"]).contains("no control has the ID \"nope\""));
    assert!(error(&["--variant", "cover"]).contains("the window's are \"details\""));
    let twice = error(&["--state", "hover:read", "--state", "pressed:read"]);
    assert!(twice.contains("\"read\" is given hover and pressed"), "{twice}");
    let list = jui.with_file_name("list.json");
    std::fs::write(&list, "[1, 2]").unwrap();
    assert!(error(&["--binds", list.to_str().unwrap()]).contains("one JSON object"));
    std::fs::write(&list, "{").unwrap();
    assert!(error(&["--binds", list.to_str().unwrap()]).contains("not JSON"));
    for bad in ["glow", "hover:"] {
        let out = output(None, &["nui", "layout", file, "--state", bad]);
        assert!(!out.status.success());
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(said.contains("normal, hover, pressed or disabled") || said.contains("ID after"));
    }
    // Said: a bind no property reads.
    std::fs::write(&list, r#"{"cry_count": 2, "nope": true}"#).unwrap();
    let (ok, said) = mg(None, &["layout", file, "--binds", list.to_str().unwrap()]);
    assert!(ok, "{said}");
    let issues = said["issues"].as_array().unwrap();
    assert!(issues.contains(&json!("bind \"nope\": no property of the window reads it")));

    // render takes the same options.
    mg_testkit::gpu::hold();
    let png = jui.with_file_name("cries.png");
    let args = [file, "-o", png.to_str().unwrap(), "--binds", binds, "--variant", "details"];
    let Some(drawn) = render(&[&args[..], &["--state", "pressed:back"]].concat()) else {
        return;
    };
    assert_eq!(png_size(&png), (600, 330));
    assert_eq!(drawn["window"], "cries");
}

#[cfg(feature = "nui-preview")]
#[test]
fn layout_gives_each_control_s_place_in_the_window() {
    let path = module("mg-nui-layout");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    let (ok, layout) = mg(None, &["layout", p, "shop"]);
    assert!(ok, "{layout}");
    assert_eq!((&layout["width"], &layout["height"]), (&json!(420.0), &json!(240.0)));
    let elements = layout["elements"].as_array().unwrap();
    let close = elements.iter().find(|e| e["id"] == "mg_close").unwrap();
    assert_eq!(close["kind"], "button");
    assert_eq!(close["text"], "Close");
    // Below the title bar (33) and the 8-point inset, a label's height on.
    assert!(close["y"].as_f64().unwrap() > 41.0 + 30.0, "{close}");
    assert_eq!((&close["w"], &close["h"]), (&json!(150.0), &json!(30.0)));
    for flag in ["zero_sized", "clipped", "out_of_view", "hidden"] {
        assert_eq!(close[flag], false, "{flag}");
    }
    // At 150% the places are the same points.
    let (_, scaled) = mg(None, &["layout", p, "shop", "--scale", "1.5"]);
    assert_eq!(scaled["scale"], 1.5);
    assert_eq!(scaled["elements"], layout["elements"]);
    // For people: a line a control.
    let out = output(None, &["nui", "layout", p, "shop", "--size", "300x200"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("shop: 300 × 200 points at 100%"), "{text}");
    assert!(text.lines().any(|l| l.starts_with("/root/children/1\tbutton #mg_close\t")), "{text}");
}

#[cfg(feature = "nui-preview")]
#[test]
fn a_window_s_file_is_laid_out_with_its_settings() {
    let jui = crier_files("mg-nui-layout-file");
    let file = jui.to_str().unwrap();
    let (ok, layout) = mg(None, &["layout", file]);
    assert!(ok, "{layout}");
    assert_eq!(layout["window"], "crier");
    let elements = layout["elements"].as_array().unwrap();
    // The fee's text is its bind's first value, from crier.txt.
    let fee = elements.iter().find(|e| e["path"] == "/root/children/2").unwrap();
    assert_eq!(fee["text"], "The fee is 5 silver pieces.");
    assert_eq!(fee["binds"], json!(["fee_text"]));
    let spacer = elements.iter().find(|e| e["kind"] == "spacer").unwrap();
    assert_eq!(spacer["zero_sized"], true);
    // A smaller window cuts off its buttons.
    let (_, small) = mg(None, &["layout", file, "--size", "200x150"]);
    let cancel = small["elements"].as_array().unwrap().iter().find(|e| e["id"] == "cancel");
    assert_eq!(cancel.unwrap()["out_of_view"], true, "{small}");

    // A file with a window's name after it, a module without one, a size
    // that isn't one: said, not guessed.
    let (ok, both) = mg(None, &["layout", file, "crier"]);
    assert!(!ok && both["error"].as_str().unwrap().contains("a .jui file is a window"), "{both}");
    let path = module("mg-nui-layout-no-name");
    let (ok, which) = mg(None, &["layout", path.to_str().unwrap()]);
    assert!(!ok && which["error"].as_str().unwrap().contains("which window?"), "{which}");
    let out = output(None, &["nui", "layout", file, "--size", "wide"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("WIDTHxHEIGHT"));
}

/// `mg nui render`'s answer, or `None` where there is no GPU to draw with.
#[cfg(feature = "nui-preview")]
fn render(args: &[&str]) -> Option<Value> {
    let args: Vec<&str> = ["--json", "nui", "render"].iter().chain(args).copied().collect();
    let out = output(None, &args);
    // (A driver may say things of its own on standard error.)
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    if json["error"].as_str().is_some_and(|e| e.contains("no GPU adapter")) {
        eprintln!("skipped: no GPU adapter");
        return None;
    }
    assert!(out.status.success(), "{json}");
    Some(json)
}

/// A PNG's width and height.
#[cfg(feature = "nui-preview")]
fn png_size(path: &Path) -> (u32, u32) {
    let png = std::fs::read(path).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let word = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
    (word(16), word(20))
}

#[cfg(feature = "nui-preview")]
#[test]
fn render_draws_a_window_into_a_png_at_its_scale() {
    mg_testkit::gpu::hold();
    let path = module("mg-nui-render");
    let p = path.to_str().unwrap();
    assert!(mg(None, &["new", p, "shop"]).0);
    let png = path.with_file_name("shop.png");
    let Some(drawn) = render(&[p, "shop", "-o", png.to_str().unwrap(), "--scale", "2"]) else {
        return;
    };
    assert_eq!((&drawn["width"], &drawn["height"]), (&json!(840), &json!(480)));
    assert_eq!(png_size(&png), (840, 480));
    // A window's file, at the size asked for.
    let jui = crier_files("mg-nui-render-file");
    let out = jui.with_file_name("crier.png");
    let drawn = render(&[jui.to_str().unwrap(), "--size", "500x320", "-o", out.to_str().unwrap()]);
    assert_eq!(drawn.unwrap()["window"], "crier");
    assert_eq!(png_size(&out), (500, 320));
}
