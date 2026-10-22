//! `mg haks --usage`: a module of our own, with two haks of our own in a
//! scratch user directory, over the game's install: which hak resources
//! its area's tile, its placeable and its area's script reach, which none
//! does, which a higher hak shadows, and which aren't judged.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_schema::{ExoString, StructExt, ifo};
use serde_json::Value as Json;

const MODEL: &str = "newmodel NAME\nsetsupermodel NAME NULL\nclassification Tile\n\
    setanimationscale 1\nbeginmodelgeom NAME\nnode dummy NAME\n  parent NULL\nendnode\n\
    node trimesh box\n  parent NAME\n  bitmap TEX\n  verts 3\n    0 0 0\n    1 0 0\n    \
    0 1 0\n  faces 1\n    0 1 2 1 0 0 0 0\nendnode\nendmodelgeom NAME\ndonemodel NAME\n";

fn model(name: &str, texture: &str) -> Vec<u8> {
    MODEL.replace("NAME", name).replace("TEX", texture).into_bytes()
}

fn hak(path: &std::path::Path, files: &[(&str, Vec<u8>)]) {
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    for (name, data) in files {
        let k = ResKey::from_filename(name).unwrap();
        w.add(k.resref, k.restype, data.clone()).unwrap();
    }
    std::fs::write(path, w.to_bytes().unwrap()).unwrap();
}

#[test]
fn usage_says_what_the_module_reaches_in_each_hak() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-haks-usage");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    // The first tile of the city tileset, which the area places.
    let game = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let set = game.get(&ResKey::parse("tcn01", ResType::SET).unwrap()).unwrap();
    let set = mg_set::Tileset::parse(&set, mg_core::Codepage::WINDOWS_1252).unwrap();
    let tile = set.tiles[0].model.to_ascii_lowercase();

    hak(
        &user.join("hak/mg_usage.hak"),
        &[
            // The game's armoire and the tile, replaced, and what they use.
            ("plc_a01.mdl", model("plc_a01", "mg_usage_wood")),
            ("mg_usage_wood.tga", b"tga".to_vec()),
            (&format!("{tile}.mdl"), model(&tile, "mg_usage_tile")),
            ("mg_usage_tile.dds", b"dds".to_vec()),
            ("mg_usage_s.ncs", b"NCS V1.0".to_vec()),
            ("mg_usage_unused.mdl", model("mg_usage_unused", "mg_usage_wood")),
            ("mg_usage.2da", b"2DA V2.0\n".to_vec()),
        ],
    );
    hak(&user.join("hak/mg_usage_top.hak"), &[("mg_usage_wood.tga", b"tga2".to_vec())]);

    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    for h in ["mg_usage_top", "mg_usage"] {
        let mut s = ifo::MOD_HAK_LIST.new_item();
        s.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from(h));
        info.root.items_mut(&ifo::MOD_HAK_LIST).push(s);
    }
    let mut a = ifo::MOD_AREA_LIST.new_item();
    a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str("a1").unwrap());
    info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
    m.set_info(&info).unwrap();
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tileset", Value::resref(ResRef::from_str("tcn01").unwrap()));
    are.root.set("OnEnter", Value::resref(ResRef::from_str("mg_usage_s").unwrap()));
    let mut t = Struct::new(1);
    t.set("Tile_ID", Value::Int(0));
    are.root.set("Tile_List", Value::List(vec![t]));
    m.set_gff(ResKey::parse("a1", ResType::ARE).unwrap(), &are).unwrap();
    let mut git = Gff::new(*b"GIT ");
    let mut p = Struct::new(9);
    p.set("Appearance", Value::Dword(0));
    git.root.set("Placeable List", Value::List(vec![p]));
    m.set_gff(ResKey::parse("a1", ResType::GIT).unwrap(), &git).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.join("module"))).unwrap();

    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--user-dir", user.to_str().unwrap(), "--json"])
        .args(["haks", "--usage", dir.join("module").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let v: Json = serde_json::from_slice(&out.stdout).unwrap();
    let usage = v["usage"].as_array().unwrap();
    let hak = |name: &str| usage.iter().find(|u| u["name"] == name).unwrap();
    let names = |u: &Json, list: &str| -> Vec<String> {
        u[list]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().map_or_else(|| r["resource"].to_string(), str::to_string))
            .map(|s| s.trim_matches('"').to_string())
            .collect()
    };
    let content = hak("mg_usage");
    let mut used = names(content, "used_resources");
    used.sort();
    let mut expected = vec![
        "mg_usage_s.ncs".to_string(),
        "mg_usage_tile.dds".into(),
        "plc_a01.mdl".into(),
        format!("{tile}.mdl"),
    ];
    expected.sort();
    assert_eq!(used, expected, "{v}");
    let from = |r: &str| {
        content["used_resources"].as_array().unwrap().iter().find(|u| u["resource"] == r).unwrap()
            ["from"]
            .clone()
    };
    assert_eq!(from("plc_a01.mdl"), "a1.git");
    assert_eq!(from("mg_usage_s.ncs"), "a1.are");
    assert_eq!(names(content, "unused_resources"), ["mg_usage_unused.mdl"]);
    assert_eq!(names(content, "shadowed_resources"), ["mg_usage_wood.tga"]);
    assert_eq!(names(content, "not_judged_resources"), ["mg_usage.2da"]);
    assert_eq!(content["resources"], 7);
    assert_eq!(names(hak("mg_usage_top"), "used_resources"), ["mg_usage_wood.tga"]);

    // As text: a line a hak, and the whole.
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--user-dir", user.to_str().unwrap()])
        .args(["haks", "--usage", dir.join("module").to_str().unwrap()])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("mg_usage\t4 of 7 used"), "{text}");
    assert!(text.contains("1 unused, 1 shadowed, 1 not judged"), "{text}");
    assert!(text.contains("All\t5 of 8 used"), "{text}");
}
