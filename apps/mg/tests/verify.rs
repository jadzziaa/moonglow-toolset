//! `mg verify`: a clean module passes; one with a placed object naming a
//! placeables.2da row that doesn't exist fails, the JSON naming it.

use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

#[test]
fn verify_fails_on_errors_and_names_them_in_json() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify");
    let verify = |path: &std::path::Path| {
        crate::mg()
            .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
            .arg(path)
            .output()
            .unwrap()
    };
    let clean = verify(&chess);
    assert!(clean.status.success(), "{}", String::from_utf8_lossy(&clean.stderr));

    let mut m = Module::open(&chess).unwrap();
    let git = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    let mut p = Struct::new(9);
    p.set("Appearance", Value::Dword(99_999));
    p.set("Tag", Value::String(b"BROKEN".to_vec()));
    g.root.set("Placeable List", Value::List(vec![p]));
    m.set_gff(git, &g).unwrap();
    let broken = dir.join("broken.mod");
    m.save_as(&ModuleLocation::Archive(broken.clone())).unwrap();
    let out = verify(&broken);
    assert!(!out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["errors"], 1);
    let f = &json["findings"][0];
    assert_eq!(f["check"], "object-row");
    assert_eq!(f["severity"], "error");
    assert!(
        f["at"].as_str().unwrap().eq_ignore_ascii_case("chess › placeable BROKEN › Appearance")
    );
}

/// Variables in an area's ARE, which the game does not read (it reads an
/// area's from its GIT: `engine_area_variables`), as Moonglow wrote them
/// before 1.34.3: Verify fails, naming them.
#[test]
fn verify_finds_area_variables_the_game_does_not_read() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-area-vars");
    let mut m = Module::open(&chess).unwrap();
    let are = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::ARE);
    let mut g = m.gff(&are).unwrap().unwrap();
    let mut v = Struct::new(0);
    v.set("Name", Value::String(b"iNoMap".to_vec()));
    v.set("Type", Value::Dword(1));
    v.set("Value", Value::Int(1));
    g.root.set("VarTable", Value::List(vec![v]));
    m.set_gff(are, &g).unwrap();
    let path = dir.join("area-vars.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let f = &json["findings"][0];
    assert_eq!(f["check"], "area-variables");
    assert!(f["message"].as_str().unwrap().contains("iNoMap"), "{f}");
}

/// A module naming a talk table the game can't find doesn't load: Verify
/// fails on it. The table in the module's own resources is found, as the
/// game finds it there (`tests/engine_tlk.rs`).
#[test]
fn verify_finds_the_talk_table_where_the_game_does() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-tlk");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("tlk")).unwrap();
    let verify = |path: &std::path::Path| {
        let out = crate::mg()
            .args(["--root", root.to_str().unwrap(), "--user-dir", user.to_str().unwrap()])
            .args(["verify", "--json"])
            .arg(path)
            .output()
            .unwrap();
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()
    };
    let mut m = Module::open(&chess).unwrap();
    let mut ifo = m.info().unwrap();
    ifo.root.set("Mod_CustomTlk", Value::String(b"mg_verify".to_vec()));
    m.set_info(&ifo).unwrap();
    let path = dir.join("named.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let json = verify(&path);
    assert_eq!(json["errors"], 1, "{json}");
    assert_eq!(json["findings"][0]["check"], "custom-tlk");

    let tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH).to_bytes().unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_verify").unwrap(), ResType::TLK), tlk.clone());
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    assert_eq!(verify(&path)["errors"], 0);
    // In the tlk folder instead.
    std::fs::write(user.join("tlk/mg_verify.tlk"), tlk).unwrap();
    m.remove(&ResKey::new(ResRef::from_str("mg_verify").unwrap(), ResType::TLK));
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    assert_eq!(verify(&path)["errors"], 0);
}
