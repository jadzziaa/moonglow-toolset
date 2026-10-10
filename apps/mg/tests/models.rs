//! Models through `mg`: `mg pack --compile-models` compiles the models
//! kept as text, each against its supermodel, and leaves as text what it
//! can't; `mg cat --text` prints a compiled model as text.

use std::process::Command;

const BASE: &str = "newmodel base\nsetsupermodel base NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom base\n\
node dummy base\n  parent NULL\nendnode\n\
node dummy arm\n  parent base\nendnode\n\
endmodelgeom base\ndonemodel base\n";

fn mg(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mg")).args(args).output().unwrap()
}

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
        let out = Command::new(env!("CARGO_BIN_EXE_mg"))
            .args(["--root", root.to_str().unwrap(), "--no-user-dir"])
            .args(args)
            .output()
            .unwrap();
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
