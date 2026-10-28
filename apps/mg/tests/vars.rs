//! A project's described variables (`variables.toml`) from the command
//! line: `mg vars` lists them, `mg verify` holds the module's variables to
//! them, `mg areas --var` takes a described variable's type; and `mg
//! builder-code` (The Domains of Dread's builder proposal, C1 and C2).

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

const DESCRIBED: &str = "undescribed = [\"tdod_*\"]\n\n\
    [tdod_kind]\ntype = \"string\"\non = [\"creature\"]\nconstants = \"tdod_kind_inc\"\n\
    prefix = \"TDOD_KIND_\"\nhelp = \"What the creature is, where its race misleads\"\n\n\
    [tdod_settlement]\ntype = \"string\"\non = [\"area\"]\n\n\
    [tdod_lit]\ntype = \"int\"\non = [\"placeable\"]\nvalues = { 1 = \"lit\" }\n\n\
    [tdod_trodden]\nruntime = true\n";

/// A module folder of the module's own (no game data), with its
/// variables described.
fn described_folder(name: &str) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name).join("project");
    let mut m = Module::new();
    let mut ifo = Gff::new(*b"IFO ");
    ifo.root.set("Mod_Tag", Value::String(b"KREZK".to_vec()));
    m.set_info(&ifo).unwrap();
    m.set(
        ResKey::new(ResRef::from_str("tdod_kind_inc").unwrap(), ResType::NSS),
        b"const string TDOD_KIND_BEAST = \"beast\";\nconst string TDOD_KIND_FLORA = \"flora\";\n"
            .to_vec(),
    );
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join("variables.toml"), DESCRIBED).unwrap();
    dir
}

#[test]
fn vars_lists_the_project_s_described_variables() {
    let dir = described_folder("mg-vars-list");
    let out = crate::mg().args(["--json", "--no-user-dir", "vars"]).arg(&dir).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["undescribed"], serde_json::json!(["tdod_*"]));
    let vars = json["variables"].as_array().unwrap();
    assert_eq!(vars.len(), 4, "{json}");
    assert_eq!((&vars[0]["runtime"], &vars[3]["runtime"]), (&false.into(), &true.into()));
    assert_eq!(vars[0]["name"], "tdod_kind");
    assert_eq!(vars[0]["type"], "String");
    assert_eq!(vars[0]["on"], serde_json::json!(["creature"]));
    assert_eq!(vars[0]["line"], 3);
    assert_eq!(
        vars[0]["values"],
        serde_json::json!([
            { "value": "beast", "note": "TDOD_KIND_BEAST" },
            { "value": "flora", "note": "TDOD_KIND_FLORA" },
        ]),
        "the module's script's constants"
    );
    // For people, and only what goes on an area.
    let out =
        crate::mg().args(["--no-user-dir", "vars", "--on", "area"]).arg(&dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text.trim(), "tdod_settlement\ta String, on an area");
    let out =
        crate::mg().args(["--no-user-dir", "vars", "--on", "house"]).arg(&dir).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("one of module, area, creature"));
    let out =
        crate::mg().args(["--no-user-dir", "vars"]).arg(&dir).arg("tdod_lit").output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text.trim(), "tdod_lit\tan Int, on a placeable\t1 (lit)");
    let out =
        crate::mg().args(["--no-user-dir", "vars"]).arg(&dir).arg("tdod_trodden").output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        text.trim(),
        "tdod_trodden\ta variable of any type, set by the scripts while the server runs, never in \
         a file"
    );
}

/// A String's values a table's rows, of a hak the module lists: the
/// row's own label, without a column named (the Domains of Dread's
/// tables are in their haks).
#[test]
fn vars_reads_a_table_of_the_module_s_haks() {
    let root = mg_testkit::corpus!();
    let dir = described_folder("mg-vars-hak");
    let user = dir.parent().unwrap().join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
    let table = b"2DA V2.0\n\n   Label   Skill\n0  forge   Smithing\n1  ****    ****\n2  loom    Tailoring\n";
    hak.add(ResRef::from_str("tdod_crafts").unwrap(), ResType::TWODA, table.to_vec()).unwrap();
    std::fs::write(user.join("hak/tdod_craft.hak"), hak.to_bytes().unwrap()).unwrap();
    let mut m = Module::open(&dir).unwrap();
    let mut ifo = m.info().unwrap();
    let mut listed = Struct::new(8);
    listed.set("Mod_Hak", Value::String(b"tdod_craft".to_vec()));
    ifo.root.set("Mod_HakList", Value::List(vec![listed]));
    m.set_info(&ifo).unwrap();
    m.save().unwrap();
    std::fs::write(
        dir.join("variables.toml"),
        "[tdod_workbench]\ntype = \"string\"\ntable = \"tdod_crafts\"\n\n\
         [tdod_skill]\ntype = \"string\"\ntable = \"tdod_crafts\"\ncolumn = \"Skill\"\n",
    )
    .unwrap();
    let vars = |user: Option<&std::path::Path>| {
        let mut c = crate::mg();
        c.args(["--json", "--root", root.to_str().unwrap()]);
        match user {
            Some(u) => c.args(["--user-dir", u.to_str().unwrap()]),
            None => c.arg("--no-user-dir"),
        };
        let out = c.arg("vars").arg(&dir).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let said = json["notes"].to_string();
        (json, said)
    };
    let (json, _) = vars(Some(&user));
    let vars_of = |json: &serde_json::Value, i: usize| json["variables"][i]["values"].clone();
    assert_eq!(
        vars_of(&json, 0),
        serde_json::json!([
            { "value": "forge", "note": "row 0" },
            { "value": "loom", "note": "row 2" },
        ]),
        "{json}"
    );
    assert_eq!(vars_of(&json, 1)[1]["value"], "Tailoring");
    // The hak not found: said, and no values.
    let (json, said) = vars(None);
    assert_eq!(vars_of(&json, 0), serde_json::json!([]));
    assert!(said.contains("hak tdod_craft not found"), "{said}");
    assert!(said.contains("the table tdod_crafts.2da was not found"), "{said}");
}

#[test]
fn builder_code_is_set_shown_and_taken_out() {
    let dir = described_folder("mg-builder-code");
    std::fs::write(dir.join("moonglow.toml"), "[resrefs]\npattern = \"{name}_{code}\"\n").unwrap();
    let out = crate::mg().arg("builder-code").arg(&dir).arg("AB").output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "ab: new ResRefs are made by {name}_{code} (a new area named Market: market_ab)"
    );
    let user = std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap();
    assert_eq!(user, "[builder]\ncode = \"ab\"\n");
    assert!(dir.join(".moonglow/.gitignore").is_file());
    let out = crate::mg().args(["--json", "builder-code"]).arg(&dir).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["code"], "ab");
    assert_eq!(json["example"], "market_ab");
    let out = crate::mg().arg("builder-code").arg(&dir).arg("much_too_long").output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("at most 8"));
    let out = crate::mg().arg("builder-code").arg(&dir).arg("--clear").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("no builder code"));
    assert_eq!(std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap(), "[builder]\n");
}

/// A described variable of another type is an error (the game reads it
/// as that type's empty value), and Verify fails; one of a value not
/// listed is a warning.
#[test]
fn verify_holds_variables_to_their_descriptions() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-described").join("chess");
    let mut m = Module::open(&chess).unwrap();
    let git = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    let var = |name: &str, t: u32, value: Value| {
        let mut v = Struct::new(0);
        v.set("Name", Value::String(name.as_bytes().to_vec()));
        v.set("Type", Value::Dword(t));
        v.set("Value", value);
        v
    };
    g.root.set(
        "VarTable",
        Value::List(vec![
            var("tdod_settlement", 1, Value::Int(1)),
            var("tdod_lt", 1, Value::Int(1)),
            var("tdod_trodden", 1, Value::Int(1)),
        ]),
    );
    m.set_gff(git, &g).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join("variables.toml"), DESCRIBED).unwrap();
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(!out.status.success(), "a variable of another type fails Verify");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ours: Vec<(&str, &str, &str)> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"].as_str().is_some_and(|c| c.starts_with("var-")))
        .map(|f| {
            (
                f["check"].as_str().unwrap(),
                f["severity"].as_str().unwrap(),
                f["message"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        ours,
        [
            (
                "var-type",
                "error",
                "tdod_settlement is an Int here, and variables.toml:10 says it is a String: the \
                 scripts read \"\""
            ),
            (
                "var-runtime",
                "error",
                "tdod_trodden is set by the scripts while the server runs (variables.toml:19 says \
                 runtime = true), and is never to be kept in a file: one kept here is there from \
                 the start, on every server"
            ),
            (
                "var-name",
                "warning",
                "tdod_lt is named as the project's variables are (tdod_*), and variables.toml \
                 describes no such variable: tdod_lit?"
            ),
        ]
    );
    // The scripts' own taken out when asked; the rest kept.
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify"])
        .args(["--fix", "var-runtime"])
        .arg(&dir)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("fixed\tvar-runtime › chess.git: Remove the variable"), "{text}");
    let g = Module::open(&dir).unwrap().gff(&git).unwrap().unwrap();
    let names: Vec<&[u8]> =
        g.root.list("VarTable").unwrap().iter().map(|v| v.string("Name").unwrap()).collect();
    assert_eq!(names, [&b"tdod_settlement"[..], b"tdod_lt"]);
    let checks = crate::mg().args(["--json", "checks"]).output().unwrap();
    let checks: serde_json::Value = serde_json::from_slice(&checks.stdout).unwrap();
    for id in ["var-type", "var-value", "var-object", "var-name", "var-runtime"] {
        let check = checks["checks"].as_array().unwrap().iter().find(|c| c["id"] == id);
        assert!(check.is_some_and(|c| c["group"] == "objects"), "{id}: {checks}");
    }
}

/// `mg areas --var NAME=VALUE` with no type: the described variable's
/// (`tdod_settlement=1` a String, not the Int the value looks like: the
/// guide's second pitfall), and a type given against it is said.
#[test]
fn areas_var_takes_a_described_variable_s_type() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-areas-described").join("chess");
    let mut m = Module::open(&chess).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join("variables.toml"), DESCRIBED).unwrap();
    let out = crate::mg()
        .args(["--no-user-dir", "areas"])
        .arg(&dir)
        .args(["--all", "--var", "tdod_settlement=1", "--var", "tdod_lit:int=1"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let notes = String::from_utf8_lossy(&out.stderr);
    assert!(
        notes.contains("tdod_lit: variables.toml:14 says it goes on placeable, not on an area"),
        "{notes}"
    );
    let m = Module::open(&dir).unwrap();
    let git =
        m.gff(&ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT)).unwrap().unwrap();
    let vars = git.root.list("VarTable").unwrap();
    let settlement =
        vars.iter().find(|v| v.string("Name") == Some(&b"tdod_settlement"[..])).unwrap();
    assert_eq!(settlement.get("Type"), Some(&Value::Dword(3)), "a String");
    assert_eq!(settlement.get("Value"), Some(&Value::String(b"1".to_vec())));
}
