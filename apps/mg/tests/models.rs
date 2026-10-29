//! Models through `mg`: `mg pack --compile-models` compiles the models
//! kept as text, each against its supermodel, and leaves as text what it
//! can't; `mg cat --text` prints a compiled model as text.

const BASE: &str = "newmodel base\nsetsupermodel base NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom base\n\
node dummy base\n  parent NULL\nendnode\n\
node dummy arm\n  parent base\nendnode\n\
endmodelgeom base\ndonemodel base\n";

use crate::mg_output as mg;

#[test]
fn pack_compiles_the_models_kept_as_text_when_asked() {
    let dir = mg_testkit::scratch_dir("mg-pack-models");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let child = BASE
        .replace("base", "child")
        .replace("setsupermodel child NULL", "setsupermodel child base");
    let orphan = child
        .replace("child", "orphan")
        .replace("setsupermodel orphan base", "setsupermodel orphan mg_nowhere_zz");
    for (name, text) in [("base.mdl", BASE), ("child.mdl", &child), ("orphan.mdl", &orphan)] {
        std::fs::write(src.join(name), text).unwrap();
    }
    std::fs::write(src.join("notes.txt"), "newmodel not_a_model\n").unwrap();
    let at = |name: &str| dir.join(name).to_string_lossy().into_owned();
    let unpacked = |hak: &str, to: &str, name: &str| {
        let out = mg(&["unpack", &at(hak), &at(to)]);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        std::fs::read(dir.join(to).join(name)).unwrap()
    };

    // Not asked: every file goes in as it is.
    let out = mg(&["pack", &at("src"), &at("plain.hak")]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(unpacked("plain.hak", "plain", "child.mdl"), child.as_bytes());

    // Asked: the two that have their supermodel are compiled; the third
    // goes in as text and is named; what is no model is left alone.
    let out = mg(&["--json", "pack", &at("src"), &at("models.hak"), "--compile-models"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["models_compiled"], serde_json::json!(["base.mdl", "child.mdl"]), "{v}");
    assert_eq!(v["models_left_as_text"][0]["model"], "orphan.mdl", "{v}");
    assert!(v["models_left_as_text"][0]["why"].as_str().unwrap().contains("mg_nowhere_zz"));
    let child_bin = unpacked("models.hak", "models", "child.mdl");
    assert!(mg_mdl::is_binary(&child_bin));
    assert_eq!(mg_mdl::Model::read(&child_bin).unwrap().supermodel.as_deref(), Some("base"));
    assert_eq!(std::fs::read(dir.join("models/orphan.mdl")).unwrap(), orphan.as_bytes());
    assert_eq!(std::fs::read(dir.join("models/notes.txt")).unwrap(), b"newmodel not_a_model\n");
}

#[test]
fn cat_prints_a_compiled_model_as_text() {
    let root = mg_testkit::corpus!();
    let run = |args: &[&str]| {
        let out = crate::mg_on(&root, None).args(args).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        out.stdout
    };
    let compiled = run(&["cat", "plc_a01.mdl"]);
    assert!(mg_mdl::is_binary(&compiled));
    let text = String::from_utf8(run(&["cat", "plc_a01.mdl", "--text"])).unwrap();
    assert!(
        text.to_ascii_lowercase().contains("newmodel plc_a01"),
        "{}",
        &text[..200.min(text.len())]
    );
    // The text is the model: it reads as the compiled one does.
    let (a, b) =
        (mg_mdl::Model::read(&compiled).unwrap(), mg_mdl::Model::read(text.as_bytes()).unwrap());
    assert_eq!(a.nodes.len(), b.nodes.len());
}

/// A hak whose entry names a folder above (`../x`, as a crafted download
/// may) is not unpacked there: `mg unpack` refuses it and writes nothing
/// outside the folder asked for.
#[test]
fn unpack_writes_nothing_outside_its_folder() {
    use mg_core::{ResRef, ResType};
    let dir = mg_testkit::scratch_dir("mg-unpack-escape");
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    w.add(ResRef::from_str("../escaped").unwrap(), ResType::TXT, &b"out"[..]).unwrap();
    let mut bytes = Vec::new();
    w.write_to(&mut bytes).unwrap();
    std::fs::write(dir.join("crafted.hak"), bytes).unwrap();
    let to = dir.join("in").join("here");
    let out = mg(&["unpack", &dir.join("crafted.hak").to_string_lossy(), &to.to_string_lossy()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("names a folder"));
    assert!(!dir.join("in").join("escaped.txt").exists(), "written above the folder");
}

/// `mg model compile` compiles one file against its supermodel beside it,
/// with the compiler's notes; `mg model decompile` writes it back as text.
#[test]
fn model_compiles_and_decompiles_one_file() {
    let dir = mg_testkit::scratch_dir("mg-model-file");
    let child = BASE
        .replace("base", "child")
        .replace("setsupermodel child NULL", "setsupermodel child base")
        .replace("endmodelgeom child", "node dummy loose\nendnode\nendmodelgeom child");
    std::fs::write(dir.join("base.mdl"), BASE).unwrap();
    std::fs::write(dir.join("child.mdl"), &child).unwrap();
    let at = |name: &str| dir.join(name).to_string_lossy().into_owned();
    let (ok, json) =
        crate::mg_json(&["--no-user-dir", "model", "compile", &at("child.mdl"), &at("out.mdl")]);
    assert!(ok, "{json}");
    let compiled = std::fs::read(dir.join("out.mdl")).unwrap();
    assert!(mg_mdl::is_binary(&compiled));
    let model = mg_mdl::Model::read(&compiled).unwrap();
    assert_eq!(model.supermodel.as_deref(), Some("base"));
    // (A node with no parent line is left out, as the game leaves it, and
    // the notes say so.)
    assert_eq!(model.node("loose"), None);
    let notes = json["compiler_notes"].to_string();
    assert!(notes.contains("loose: has no parent line"), "{notes}");
    let out = mg(&["--no-user-dir", "model", "decompile", &at("out.mdl"), &at("back.mdl")]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = std::fs::read_to_string(dir.join("back.mdl")).unwrap();
    assert!(text.contains("newmodel child") && text.contains("node dummy arm"), "{text}");
    // Text is not decompiled; a binary is not compiled.
    let out = mg(&["--no-user-dir", "model", "decompile", &at("child.mdl"), &at("x.mdl")]);
    assert!(!out.status.success());
    let out = mg(&["--no-user-dir", "model", "compile", &at("out.mdl"), &at("x.mdl")]);
    assert!(!out.status.success());
}

/// nwn-mdl-spec's conformance suite (`mg_testkit::mdl_spec`): each probe
/// compiled by `mg model compile` reads as the game's own compile of it
/// (the suite's goldens, `nwmain-linux compilemodel`), and each golden
/// decompiled by `mg model decompile` reads as the golden does; with a
/// game install, the suite's stock models decompiled too. The comparison
/// is the suite's own runner (`runner/mdlconf.py`, standard-library
/// Python). Skipped without a checkout of the suite (`NWN_MDL_SPEC`).
#[test]
fn the_model_spec_suite_passes() {
    let Some(spec) = mg_testkit::mdl_spec() else {
        assert!(!mg_testkit::corpus_required(), "nwn-mdl-spec not found (set NWN_MDL_SPEC)");
        eprintln!("skipped: nwn-mdl-spec not found (set NWN_MDL_SPEC)");
        return;
    };
    let home = mg_testkit::output_dir("mg-home");
    let dir = mg_testkit::scratch_dir("mg-mdl-spec");
    let mg = format!("{} --no-user-dir model", env!("CARGO_BIN_EXE_mg"));
    let run = |verb: &str, extra: &[&str]| -> Vec<serde_json::Value> {
        let report = dir.join(format!("{verb}.json"));
        let out = std::process::Command::new("python3")
            .arg("-I")
            .arg(spec.join("runner/mdlconf.py"))
            .args([verb, "--impl", "moonglow", "--cmd", &mg, "--json"])
            .arg(&report)
            .args(extra)
            .env("MOONGLOW_DATA_DIR", home.join("data/moonglow"))
            .env("MOONGLOW_CACHE_DIR", home.join("cache/moonglow"))
            .output()
            .expect("python3");
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{said}{}", String::from_utf8_lossy(&out.stderr));
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        eprintln!("{}", said.lines().last().unwrap_or_default());
        report["results"].as_array().cloned().unwrap_or_default()
    };
    let failed = |results: &[serde_json::Value]| -> Vec<String> {
        (results.iter())
            .filter(|r| !matches!(r["status"].as_str(), Some("pass" | "pending")))
            .map(|r| {
                let must = r["must"].get(0).map(|m| format!(": {m}")).unwrap_or_default();
                let why = r["reason"].as_str().map(|w| format!(": {w}")).unwrap_or_default();
                format!("{} {} {}{must}{why}", r["probe"], r["test"], r["status"])
            })
            .collect()
    };
    let probes = run("run", &[]);
    let passed = probes.iter().filter(|r| r["status"] == "pass").count();
    // (46 probes with goldens at the suite's e627c9f, each compiled and
    // decompiled.)
    assert!(passed >= 92, "{passed} passed");
    let f = failed(&probes);
    assert!(f.is_empty(), "{f:#?}");
    // What the suite holds a compiler only to SHOULD and Moonglow does as
    // the game's compiler: face records and bounds from the text's faces
    // and vertices (MDL-MESH-021, -022, -050), a node with no parent line
    // left out (MDL-ASCII-031).
    let should: Vec<String> = (probes.iter())
        .filter(|r| r["test"] == "compile")
        .flat_map(|r| r["should"].as_array().cloned().unwrap_or_default())
        .filter_map(|s| s[0].as_str().map(str::to_string))
        .filter(|p| {
            [".faces.normal", ".faces.distance", ".faces.adjacent", ".radius", ".average"]
                .iter()
                .any(|end| p.ends_with(end))
                || p == "nodes.tree"
        })
        .collect();
    assert!(should.is_empty(), "{should:#?}");
    if let Some(root) = mg_testkit::nwn_root() {
        let stock = run("stock", &["--root", &root.to_string_lossy()]);
        assert!(stock.len() >= 14, "{} stock models", stock.len());
        let f = failed(&stock);
        assert!(f.is_empty(), "{f:#?}");
    }
}
