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

fn output(root: Option<&Path>, args: &[&str]) -> std::process::Output {
    let mut mg = crate::mg();
    if let Some(root) = root {
        mg.arg("--root").arg(root);
    }
    mg.arg("--no-user-dir").args(args).output().unwrap()
}

/// `mg --json nui …`: whether it succeeded, and what it printed. Nothing
/// goes to standard error.
fn mg(root: Option<&Path>, args: &[&str]) -> (bool, Value) {
    let args: Vec<&str> = ["--json", "nui"].iter().chain(args).copied().collect();
    let out = output(root, &args);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.trim().is_empty(), "{args:?}: stderr {stderr}");
    let json = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{args:?}: {e}: {}", String::from_utf8_lossy(&out.stdout)));
    (out.status.success(), json)
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

/// The town crier's window (`crates/mg-nui/tests/data`), as files in a
/// scratch folder of its own: `crier.jui` and `crier.txt`.
fn crier_files(test: &str) -> PathBuf {
    let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/mg-nui/tests/data");
    let dir = mg_testkit::scratch_dir(test);
    for file in ["crier.jui", "crier.txt"] {
        std::fs::copy(from.join(file), dir.join(file)).unwrap();
    }
    dir.join("crier.jui")
}

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
fn png_size(path: &Path) -> (u32, u32) {
    let png = std::fs::read(path).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    let word = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
    (word(16), word(20))
}

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
