//! `mg attach`: haks and a talk table downloaded somewhere are copied into
//! the user folder and attached to a module, which then verifies clean (its
//! haks and talk table found where the game looks).

use mg_core::{ResRef, ResType};
use mg_erf::ErfWriter;
use mg_module::Module;

#[test]
fn haks_and_a_talk_table_attach_in_one_step() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-attach");
    let (download, user) = (dir.join("download"), dir.join("user"));
    std::fs::create_dir_all(&download).unwrap();
    std::fs::create_dir_all(&user).unwrap();
    for name in ["mg_top", "mg_low"] {
        let mut w = ErfWriter::new(*b"HAK ");
        let table = format!("2DA V2.0\n\n   Label\n0  {name}\n");
        w.add(ResRef::from_str(name).unwrap(), ResType::TWODA, table.into_bytes()).unwrap();
        std::fs::write(download.join(format!("{name}.hak")), w.to_bytes().unwrap()).unwrap();
    }
    let tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH).to_bytes().unwrap();
    std::fs::write(download.join("mg_text.tlk"), tlk).unwrap();
    let path = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &path).unwrap();

    let mg = |args: &[&str]| crate::mg_on(&root, Some(user.as_ref())).args(args).output().unwrap();
    let file = |n: &str| download.join(n).to_str().unwrap().to_string();
    let (top, low, text) = (file("mg_top.hak"), file("mg_low.hak"), file("mg_text.tlk"));
    let out = mg(&["attach", path.to_str().unwrap(), &top, &low, &text]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(user.join("hak/mg_top.hak").is_file() && user.join("tlk/mg_text.tlk").is_file());
    let m = Module::open(&path).unwrap();
    assert_eq!(m.haks().unwrap(), ["mg_top", "mg_low"]);
    assert_eq!(m.custom_tlk().unwrap().as_deref(), Some("mg_text"));
    let out = mg(&["verify", "--json", path.to_str().unwrap()]);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["errors"], 0, "{json}");

    // Another file of the same name is replaced only when asked.
    std::fs::write(download.join("mg_low.hak"), b"changed").unwrap();
    let out = mg(&["attach", path.to_str().unwrap(), &low]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--replace"));
    let out = mg(&["attach", path.to_str().unwrap(), &low, "--replace"]);
    assert!(out.status.success());
    assert_eq!(std::fs::read(user.join("hak/mg_low.hak")).unwrap(), b"changed");
    // Attached again, it moves to the top.
    assert_eq!(Module::open(&path).unwrap().haks().unwrap(), ["mg_low", "mg_top"]);
}

/// `mg attach` takes content as it was downloaded, a RAR or 7z archive
/// (mg-module's fixtures): its haks and talk table are taken from it;
/// one with a password is refused, said why.
#[test]
fn haks_and_a_talk_table_attach_from_a_rar_or_a_7z() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-attach-archives");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let path = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &path).unwrap();
    let fixture = |name: &str| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let file = dir.join("../../crates/mg-module/tests/fixtures/archives").join(name);
        file.to_str().unwrap().to_string()
    };
    let mg = |args: &[&str]| crate::mg_on(&root, Some(user.as_ref())).args(args).output().unwrap();

    let out = mg(&["--json", "attach", path.to_str().unwrap(), &fixture("content-rar5.rar")]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["copied"], 3, "{json}");
    assert_eq!(json["custom_tlk"], "z_text", "{json}");
    let m = Module::open(&path).unwrap();
    assert_eq!(m.haks().unwrap(), ["z_one", "z_two"]);
    let one = std::fs::read_to_string(user.join("hak/z_one.hak")).unwrap();
    assert!(one.starts_with("one line 0: Moonglow's archive fixture"), "{one}");

    // The same from the 7z: already there.
    let out = mg(&["--json", "attach", path.to_str().unwrap(), &fixture("content.7z")]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["copied"], 0, "{json}");

    for locked in ["locked.7z", "hidden.rar"] {
        let out = mg(&["attach", path.to_str().unwrap(), &fixture(locked)]);
        assert!(!out.status.success());
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(said.contains(locked) && said.contains("password"), "{said}");
    }
}
