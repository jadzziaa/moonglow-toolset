//! A project's palette groupings (`[palette.*]` in moonglow.toml) on the
//! command line: `mg find --palette-group` lists a group's blueprints,
//! `mg verify` notes a group no blueprint falls in and a list naming a
//! blueprint that is nowhere.

use mg_module::{Module, ModuleLocation};

const RULES: &str = r#"
[palette.kind]
types = ["utp", "uti"]
from = "palette_kinds.toml"

[[palette.kind.group]]
name = "Containers"
types = ["utp"]
names = ["chest"]

[[palette.kind.group]]
name = "Dreams"
names = ["zzqqxx"]

[palette.role]
title = "Role"
types = ["uti"]
"#;

/// Chess as a module folder with the groupings beside it.
fn project(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir(name).join("chess");
    Module::open(&chess).unwrap().save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join("moonglow.toml"), RULES).unwrap();
    std::fs::write(
        dir.join("palette_kinds.toml"),
        "\"Containers\" = [\"plc_nowhere_zz\", \"plc_chest1.uti\"]\n",
    )
    .unwrap();
    dir
}

#[test]
fn find_lists_a_palette_group_s_blueprints() {
    let root = mg_testkit::corpus!();
    let dir = project(&root, "mg-find-palette-group");
    let find = |group: &str, more: &[&str]| {
        crate::json_out(
            crate::mg_on(&root, None)
                .args(["--json", "find"])
                .arg(&dir)
                .args(["--palette-group", group])
                .args(more),
        )
    };
    let (ok, json) = find("kind/Containers", &[]);
    assert!(ok, "{json}");
    assert_eq!(
        (json["grouping"].as_str(), json["group"].as_str()),
        (Some("kind"), Some("Containers"))
    );
    let found = json["found"].as_array().unwrap();
    assert!(!found.is_empty(), "the game's chests: {json}");
    for f in found {
        assert_eq!((f["type"].as_str(), f["palette"].as_str()), (Some("utp"), Some("standard")));
        let name = f["name"].as_str().unwrap().to_lowercase();
        assert!(name.split(|c: char| !c.is_alphanumeric()).any(|w| w == "chest"), "{f}");
        assert!(!f["category"].as_str().unwrap().is_empty(), "{f}");
    }
    // Narrowed by name; by title and in any case.
    let (_, some) = find("KIND/containers", &["--name", "zzzzqqq"]);
    assert_eq!(some["found"].as_array().unwrap().len(), 0, "{some}");
    // Other: those no group takes, among the placeables and the items.
    let (ok, other) = find("kind/Other", &["--type", "uti"]);
    assert!(ok);
    let other = other["found"].as_array().unwrap();
    assert!(!other.is_empty() && other.iter().all(|f| f["type"] == "uti"));
    // A grouping offered among the items alone has no placeables.
    let (ok, role) = find("Role/Other", &["--type", "utp"]);
    assert!(ok && role["found"].as_array().unwrap().is_empty(), "{role}");
    // A group or a grouping there is none of: it fails, and says which there are.
    let out = crate::mg_on(&root, None)
        .arg("find")
        .arg(&dir)
        .args(["--palette-group", "kind/Tables"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains("Kind has no group \"Tables\": its groups are Containers, Dreams"),
        "{said}"
    );
    let out =
        crate::mg_on(&root, None).arg("find").arg(&dir).args(["--palette-group", "x/y"]).output();
    let said = String::from_utf8_lossy(&out.unwrap().stderr).into_owned();
    assert!(said.contains("the project's are kind (Kind), role (Role)"), "{said}");
}

#[test]
fn verify_notes_an_empty_group_and_a_list_naming_what_is_nowhere() {
    let root = mg_testkit::corpus!();
    let dir = project(&root, "mg-verify-palette-groups");
    let out = crate::mg_on(&root, None).args(["verify", "--json"]).arg(&dir).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let of = |check: &str| -> Vec<String> {
        (json["findings"].as_array().unwrap().iter())
            .filter(|f| f["check"] == check)
            .map(|f| format!("{} | {}", f["at"].as_str().unwrap(), f["message"].as_str().unwrap()))
            .collect()
    };
    assert_eq!(
        of("palette-group-empty"),
        ["moonglow.toml [palette.kind] | the group \"Dreams\" of Kind is empty: no blueprint \
             of the item, placeable palettes falls in it"]
    );
    assert_eq!(
        of("palette-list-missing"),
        ["palette_kinds.toml \"Containers\" | 2 blueprints the list of \"Containers\" names \
             are nowhere (the module, its haks, the game): plc_nowhere_zz, plc_chest1.uti"]
    );
    assert!(
        of("palette-rules")
            .iter()
            .any(|f| f.contains("the group isn't offered in the item palette"))
    );
    let checks = crate::mg().args(["--json", "checks"]).output().unwrap();
    let checks: serde_json::Value = serde_json::from_slice(&checks.stdout).unwrap();
    for id in ["palette-rules", "palette-group-empty", "palette-list-missing"] {
        let check = checks["checks"].as_array().unwrap().iter().find(|c| c["id"] == id);
        assert!(check.is_some_and(|c| c["group"] == "module"), "{id}: {checks}");
    }
    // Left out by its id.
    let out = crate::mg_on(&root, None)
        .args(["verify", "--json", "--skip", "palette-group-empty"])
        .arg(&dir)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        !json["findings"].as_array().unwrap().iter().any(|f| f["check"] == "palette-group-empty")
    );
}
