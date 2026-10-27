//! `mg test`: a module's test scripts run in the game's server, in a copy
//! of the module; what they assert comes back as results.

use std::path::Path;

use serde_json::Value;

const MATH: &str = r#"#include "mg_test"
void main()
{
    Describe("math");
    AssertEqualInt(1 + 1, 2, "adds");
    AssertEqualInt(2 * 2, 5, "multiplies");
    AssertEqualFloat(0.5 + 0.25, 0.75, "halves");
    Describe("");
    AssertEqualString(GetStringUpperCase("mg"), "MG", "upper case");
    AssertValid(GetModule(), "the module");
    Assert(GetIsObjectValid(GetFirstArea()), "an area", "the module has none");
}
"#;
const OK: &str = "#include \"mg_test\"\nvoid main() { Pass(\"fine\"); }\n";
/// A test that stops inside its main(), at a division by nothing: what
/// ran it goes on, and it asserted nothing before.
const STOPS: &str = "#include \"mg_test\"\nvoid main()\n{\n    int nNone = \
                     GetLocalInt(GetModule(), \"mg_none\");\n    int nBad = 10 / nNone;\n    \
                     Pass(\"after\");\n}\n";
const ENDLESS: &str = "void main() { int n = 0; while (TRUE) { n++; } }\n";

fn mg(root: &Path, user: &Path, args: &[&str]) -> (bool, Value) {
    crate::json_out(
        crate::mg()
            .args([
                "--root",
                root.to_str().unwrap(),
                "--user-dir",
                user.to_str().unwrap(),
                "--json",
            ])
            .arg("test")
            .args(args),
    )
}

#[test]
fn tests_run_in_the_server_and_report() {
    let root = mg_testkit::corpus!();
    if mg_serve::server_binary(&root).is_none() {
        return;
    }
    let _alone = mg_testkit::engine::timed_turn();
    let dir = mg_testkit::scratch_dir("mg-test");
    let user = dir.join("user");
    let tests = dir.join("tests");
    std::fs::create_dir_all(&user).unwrap();
    std::fs::create_dir_all(&tests).unwrap();
    std::fs::write(tests.join("test_math.nss"), MATH).unwrap();
    std::fs::write(tests.join("test_ok.nss"), OK).unwrap();
    std::fs::write(tests.join("test_endless.nss"), ENDLESS).unwrap();
    std::fs::write(tests.join("test_stops.nss"), STOPS).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let before = std::fs::read(&chess).unwrap();
    let (c, t) = (chess.to_str().unwrap(), tests.to_str().unwrap());
    let (served, junit) = (dir.join("served"), dir.join("results.xml"));
    let common = ["--tests", t, "--dir", served.to_str().unwrap()];

    let all = [&[c, "--junit", junit.to_str().unwrap()][..], &common[..]].concat();
    let (ok, r) = mg(&root, &user, &all);
    assert!(!ok && r["ok"] == false && r["done"] == true, "{r}");
    assert_eq!(
        (&r["passed"], &r["failed"], &r["errors"]),
        (&6.into(), &1.into(), &2.into()),
        "{r}"
    );
    let of = |name: &str| {
        r["tests"].as_array().unwrap().iter().find(|t| t["script"] == name).unwrap().clone()
    };
    let math = of("test_math");
    let failed: Vec<&Value> =
        math["cases"].as_array().unwrap().iter().filter(|c| c["ok"] == false).collect();
    assert_eq!(failed[0]["name"], "math: multiplies");
    assert_eq!(failed[0]["detail"], "got 4, expected 5");
    assert_eq!(math["cases"][3]["name"], "upper case");
    let endless = of("test_endless");
    let why = endless["error"].as_str().unwrap();
    assert!(why.contains("TOO MANY INSTRUCTIONS"), "{why}");
    assert_eq!(of("test_ok")["passed"], 1);
    // One that stopped inside is an error, not a test that passed with
    // nothing asserted.
    let stops = of("test_stops");
    let why = stops["error"].as_str().unwrap_or_default();
    assert!(why.starts_with("did not run to its end"), "{stops}");
    assert_eq!(stops["passed"], 0);
    let xml = std::fs::read_to_string(&junit).unwrap();
    assert!(xml.contains("<failure message=\"got 4, expected 5\"/>"), "{xml}");
    // The module is as it was, and the player's folder got nothing.
    assert!(std::fs::read(&chess).unwrap() == before);
    assert_eq!(std::fs::read_dir(&user).unwrap().count(), 0);

    // Those asked for alone: all pass.
    let (ok, r) = mg(&root, &user, &[&[c, "--filter", "test_ok"][..], &common[..]].concat());
    assert!(ok && r["ok"] == true && r["passed"] == 1 && r["tests"].as_array().unwrap().len() == 1);

    // A test that does not compile: nothing is run.
    std::fs::write(tests.join("test_broken.nss"), "void main() { nothing here }\n").unwrap();
    let (ok, r) = mg(&root, &user, &[&[c][..], &common[..]].concat());
    assert!(!ok && r["compile_errors"].as_array().unwrap().len() == 1, "{r}");
    assert!(r["tests"].as_array().unwrap().is_empty());
}

/// `--driver none`: a module that runs its tests itself (its own script
/// at its load, which says when they are done) is run as it is.
#[test]
fn a_module_s_own_driver_runs_its_tests() {
    use mg_core::ResType;
    use mg_gff::Value as Field;
    use mg_resman::ResKey;
    let root = mg_testkit::corpus!();
    if mg_serve::server_binary(&root).is_none() {
        return;
    }
    let _alone = mg_testkit::engine::timed_turn();
    let dir = mg_testkit::scratch_dir("mg-test-own");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let mut m = mg_module::Module::open(&chess).unwrap();
    let script = |name: &str| ResKey::parse(name, ResType::NSS).unwrap();
    m.set(script("mg_test"), mg_module::testing::INCLUDE.as_bytes().to_vec());
    let own = "#include \"mg_test\"\nvoid main()\n{\n    Pass(\"its own\");\n    \
               Fail(\"its other\", \"not so\");\n    TestsDone();\n}\n";
    m.set(script("own_tests"), own.as_bytes().to_vec());
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let mut gff = m.gff(&ifo).unwrap().unwrap();
    gff.root.set("Mod_OnModLoad", Field::resref("own_tests".parse().unwrap()));
    m.set_gff(ifo, &gff).unwrap();
    m.save().unwrap();

    let served = dir.join("served");
    let args = [chess.to_str().unwrap(), "--driver", "none", "--dir", served.to_str().unwrap()];
    let (ok, r) = mg(&root, &user, &args);
    assert!(!ok && r["done"] == true, "{r}");
    assert_eq!(
        (&r["passed"], &r["failed"], &r["errors"]),
        (&1.into(), &1.into(), &0.into()),
        "{r}"
    );
    let cases: Vec<&Value> = r["tests"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|t| t["cases"].as_array().unwrap())
        .collect();
    assert_eq!(cases.len(), 2, "{r}");
    assert_eq!((&cases[1]["name"], &cases[1]["detail"]), (&"its other".into(), &"not so".into()));
}
