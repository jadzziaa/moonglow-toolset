//! `mg bake-walkmesh`: placed placeables' walkmeshes baked to their visual
//! transforms (the project's `[bake]`), shared where the transform is the
//! same, written into the project's hak and rows file; and unbaked.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::instances::{Placement, Placing, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;

const RULES: &str = "[bake]\nrows = \"50000-50009\"\nhak = \"mybaked\"\nnames = \"mgbk_\"\n";

/// A module folder with an area of four armoires (`plc_a01`, whose
/// walkmesh is the game's): three at Scale 0.5 turned 30, one static;
/// one plain. Its area's ResRef.
fn module(dir: &std::path::Path, game: &GameData, rules: Option<&str>) -> ResRef {
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(game, "Bake", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Yard".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, game, &spec, &mut rng).unwrap();
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    // (`VisTransformList`'s scope 0, as Adjust Location writes it.)
    let axis = |v: f32| {
        let mut s = mg_gff::Struct::new(0);
        s.set("LerpType", Value::Int(0));
        s.set("ValueTo", Value::Float(v));
        Value::Struct(s)
    };
    let mut entry = mg_gff::Struct::new(6);
    entry.set("Scope", Value::Int(0));
    entry.set("AnimationSpeed", axis(1.0));
    for (prefix, v) in [("Scale", [0.5; 3]), ("Rotate", [30.0, 0.0, 0.0]), ("Translate", [0.0; 3])]
    {
        for (i, a) in ["X", "Y", "Z"].iter().enumerate() {
            entry.set(&format!("{prefix}{a}"), axis(v[i]));
        }
    }
    let vt = Value::List(vec![entry]);
    let mut placed = Vec::new();
    for (i, (transformed, is_static)) in
        [(true, false), (true, false), (true, true), (false, false)].into_iter().enumerate()
    {
        let at = Placement { position: [5.0 + 3.0 * i as f32, 5.0, 0.0], rotation: 0.0 };
        let mut p = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
        p.set("Static", Value::Byte(is_static as u8));
        if transformed {
            p.set("VisTransformList", vt.clone());
        }
        placed.push(p);
    }
    git.root.set("Placeable List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.to_path_buf())).unwrap();
    if let Some(rules) = rules {
        std::fs::write(dir.join("moonglow.toml"), rules).unwrap();
    }
    area
}

fn appearances(dir: &std::path::Path, area: ResRef) -> Vec<i64> {
    let m = Module::open(dir).unwrap();
    let git = m.gff(&ResKey::new(area, ResType::GIT)).unwrap().unwrap();
    git.root
        .list("Placeable List")
        .unwrap()
        .iter()
        .map(|p| p.integer("Appearance").unwrap())
        .collect()
}

#[test]
fn placeables_are_baked_shared_and_unbaked() {
    let root = mg_testkit::corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let scratch = mg_testkit::scratch_dir("mg-bake-walkmesh");
    let (dir, user) = (scratch.join("bake"), scratch.join("user"));
    std::fs::create_dir_all(&user).unwrap();
    let area = module(&dir, &game, Some(RULES));
    let original = appearances(&dir, area)[0];
    let mg = || {
        let mut mg = crate::mg_on(&root, Some(&user));
        mg.arg("--json").arg("bake-walkmesh").arg(&dir).args(["--area", &area.to_string()]);
        mg
    };
    // Said, nothing written.
    let (ok, json) = crate::json_out(mg().args(["--object", "0", "--dry-run"]));
    assert!(ok, "{json}");
    assert_eq!(json["baked"][0]["row"], 50000);
    assert_eq!(json["baked"][0]["model"], "mgbk_0001");
    assert!(!user.join("hak/mybaked.hak").exists() && !dir.join("baked_rows.toml").exists());
    assert_eq!(appearances(&dir, area)[0], original);

    // Two of one transform: one bake.
    let (ok, json) = crate::json_out(mg().args(["--object", "0", "--object", "1"]));
    assert!(ok, "{json}");
    let baked = json["baked"].as_array().unwrap();
    assert_eq!((baked[0]["row"].as_u64(), baked[1]["row"].as_u64()), (Some(50000), Some(50000)));
    assert_eq!(
        (baked[0]["reused"].as_bool(), baked[1]["reused"].as_bool()),
        (Some(false), Some(true))
    );
    assert_eq!(baked[0]["copy_of"], original);
    assert_eq!(baked[0]["original"], "plc_a01");
    assert_eq!(json["files"], serde_json::json!(["mgbk_0001.mdl", "mgbk_0001.pwk"]));
    assert_eq!(appearances(&dir, area)[..2], [50000, 50000]);
    let hak = user.join("hak/mybaked.hak");
    let names: Vec<String> = mg_module::hak_edit::Hak::open(&hak)
        .unwrap()
        .items()
        .iter()
        .map(|i| i.key.to_string())
        .collect();
    assert_eq!(names, ["mgbk_0001.mdl", "mgbk_0001.pwk"]);
    let rows = mg_module::bake::RowsFile::read(&dir.join("baked_rows.toml")).unwrap().unwrap();
    assert_eq!(rows.rows.len(), 1);
    assert_eq!((rows.rows[0].row, rows.rows[0].copy_of as i64), (50000, original));
    assert_eq!(rows.rows[0].label, "Baked: Armoire at 0.5, turned 30 0 0");
    let m = Module::open(&dir).unwrap();
    assert_eq!(m.haks().unwrap().first().map(String::as_str), Some("mybaked"));

    // Baked again: the bake there is taken, nothing written anew.
    let (ok, json) = crate::json_out(mg().args(["--object", "1"]));
    assert!(ok, "{json}");
    assert_eq!(json["baked"][0]["reused"], true);
    assert_eq!(json["files"], serde_json::json!([]));
    assert_eq!(json["edits"], 0);

    // The static one and the plain one: not baked, and why.
    for (n, why) in [("2", "static"), ("3", "no visual transform")] {
        let (ok, json) = crate::json_out(mg().args(["--object", n]));
        assert!(!ok, "{json}");
        assert!(json["error"].as_str().unwrap().contains(why), "{json}");
    }
    // Unbaked: back to the row it copies; one not baked is said.
    let (ok, json) = crate::json_out(mg().args(["--object", "0", "--unbake"]));
    assert!(ok, "{json}");
    assert_eq!(json["unbaked"][0]["row"], original);
    assert_eq!(appearances(&dir, area)[..2], [original, 50000]);
    let (ok, json) = crate::json_out(mg().args(["--object", "3", "--unbake"]));
    assert!(!ok && json["error"].as_str().unwrap().contains("not baked"), "{json}");
    let (ok, json) = crate::json_out(mg().args(["--object", "9"]));
    assert!(!ok && json["error"].as_str().unwrap().contains("--object 9"), "{json}");

    // Verify says the row is pending (the build hasn't added it), and
    // nothing is unused (object 1 still has it).
    let (_, json) =
        crate::json_out(crate::mg_on(&root, Some(&user)).arg("--json").arg("verify").arg(&dir));
    let checks: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["check"].as_str().filter(|c| c.starts_with("bake-")))
        .collect();
    assert_eq!(checks, ["bake-row-pending"], "{json}");
}

#[test]
fn without_bake_rules_nothing_is_baked() {
    let root = mg_testkit::corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let scratch = mg_testkit::scratch_dir("mg-bake-walkmesh-none");
    let dir = scratch.join("bake");
    for (rules, said) in [
        (None, "no [bake] section"),
        (Some("[bake]\nrows = \"65000-66000\"\nhak = \"b\"\n"), "past row 65535"),
    ] {
        let area = module(&dir, &game, rules);
        let (ok, json) = crate::json_out(
            crate::mg_on(&root, Some(&scratch))
                .arg("--json")
                .arg("bake-walkmesh")
                .arg(&dir)
                .args(["--area", &area.to_string(), "--object", "0"]),
        );
        assert!(!ok, "{json}");
        assert!(json["error"].as_str().unwrap().contains(said), "{json}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// A save a plugin refuses writes nothing outside the module either: the
/// baked model went into the hak and its row into the rows file before
/// the plugins were asked, and stayed there with the placeables at their
/// old rows.
#[test]
fn a_refused_bake_writes_no_hak_and_no_rows() {
    let root = mg_testkit::corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let scratch = mg_testkit::scratch_dir("mg-bake-refused");
    let (dir, user, plugin) = (scratch.join("bake"), scratch.join("user"), scratch.join("refuser"));
    std::fs::create_dir_all(&user).unwrap();
    crate::plugins::refuser(&plugin);
    let area = module(&dir, &game, Some(RULES));
    let before = appearances(&dir, area);
    let mut mg = crate::mg_on(&root, Some(&user));
    mg.arg("--json").arg("bake-walkmesh").arg(&dir).args(["--area", &area.to_string()]);
    mg.args(["--object", "0"]).arg("--plugins").arg(&plugin);
    let (ok, json) = crate::json_out(&mut mg);
    let why = json["error"].as_str().unwrap_or_default();
    assert!(!ok && why.contains("not saved: a plugin refuses it"), "{json}");
    assert!(why.contains("not today"), "{json}");
    assert!(!user.join("hak/mybaked.hak").exists(), "the hak was written");
    assert!(!dir.join("baked_rows.toml").exists(), "the rows file was written");
    assert_eq!(appearances(&dir, area), before);
}
