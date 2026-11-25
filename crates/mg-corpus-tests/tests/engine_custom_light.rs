//! A light Moonglow made (Custom Light, `mg_module::lights`), in the
//! engine: the server reads the hak it was written into, finds its model
//! there and its placeables.2da row naming it, and places a placeable of
//! that row. Whether the light lights creatures and placeables, and how
//! many such lights the game draws at one place, are the client's to say
//! (the deferred list's "Lights placed on the fly, way B"). A flickering
//! one too: its row (`Static` 0) and its model (its `default` keyed) in the
//! same hak, its placeable placed dynamic.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::lights::{
    Flicker, HakChoice, LightSpec, dynamic_if_flickering, new_hak, plan, write,
};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// Logs, for the steady light and the flickering one, the row's model and
/// `Static` as the server's placeables.2da has them, where the server finds
/// that model, and the placed light.
fn probe(lights: &[(&str, usize, &str)]) -> String {
    let mut body = String::new();
    for (tag, row, model) in lights {
        body += &format!(
            r#"
    o = GetObjectByTag("{tag}");
    WriteTimestampedLogEntry("MG_ROW " + Get2DAString("placeables", "ModelName", {row}) + "|"
        + ResManGetAliasFor("{model}", RESTYPE_MDL) + "|" + IntToString(GetIsObjectValid(o))
        + "|" + IntToString(GetObjectType(o)) + "|" + Get2DAString("placeables", "Static", {row}));"#
        );
    }
    format!(
        "void main()\n{{\n    object o;{body}\n    WriteTimestampedLogEntry(\"MG_DONE\");\n}}\n"
    )
}

#[test]
fn the_engine_reads_a_custom_light_from_its_hak() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_custom_light");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    // (The game's own data: the hak is new, as Custom Light makes one.)
    let game = GameData::open(&GameInstall::new(&root, Some(user.clone()), "en")).unwrap();
    let spec = LightSpec { color: [255, 60, 20], radius: 6.0, ..Default::default() };
    let hak = new_hak(&user.join("hak"), "mg_lights").unwrap();
    let p = plan(&game.resman, game.codepage(), spec, hak.clone(), None).unwrap();
    write(&p, game.codepage()).unwrap();
    assert!(user.join("hak/mg_lights.hak").is_file());
    // A flickering light into the same hak, now listed (its copy of the
    // table read, as the module would have it).
    let mut game = game;
    let erf = mg_resman::ErfContainer::open(&hak.path).unwrap();
    let hak_label = "hak:mg_lights";
    game.resman.add(mg_resman::priority::HAK_USER, hak_label, mg_resman::LayerClass::Erf, erf);
    let torch = LightSpec { flicker: Flicker::Torch, ..spec };
    let listed = HakChoice { new: false, ..hak.clone() };
    let f = plan(&game.resman, game.codepage(), torch, listed, None).unwrap();
    assert_eq!((f.row, f.reused), (p.row + 1, false));
    write(&f, game.codepage()).unwrap();

    let mut rng = fastrand::Rng::with_seed(3);
    let mut m = new_module(&game, "Custom Light", &mut rng).unwrap();
    let spec_area = AreaSpec { name: "Lit".into(), tileset: r("ttr01"), width: 2, height: 2 };
    let area = add_area(&mut m, &game, &spec_area, &mut rng).unwrap();
    let mut bp = mg_module::blueprints::placeable(ResRef::EMPTY, "Light", 0);
    bp.root.set("Appearance", Value::Dword(p.row as u32));
    bp.root.set("Tag", Value::String(b"MG_LIGHT".to_vec()));
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [10.0, 10.0, 1.5], rotation: 0.0 };
    let placed = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    // The flickering light: of its row, made dynamic as Custom Light
    // places it.
    bp.root.set("Appearance", Value::Dword(f.row as u32));
    bp.root.set("Tag", Value::String(b"MG_FLICKER".to_vec()));
    assert!(dynamic_if_flickering(&mut bp.root, f.table()), "the row flickers");
    let at = Placement { position: [14.0, 10.0, 1.5], rotation: 0.0 };
    let flickering = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    assert_eq!((placed.integer("Static"), flickering.integer("Static")), (Some(1), Some(0)));
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git: Gff = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTP).unwrap();
    git.root.set(list, Value::List(vec![placed, flickering]));
    m.set_gff(git_key, &git).unwrap();
    let model = p.model.to_string();
    let flicker_model = f.model.to_string();
    let lights = [("MG_LIGHT", p.row, model.as_str()), ("MG_FLICKER", f.row, &flicker_model)];
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), compile(&dir, "mg_probe", &probe(&lights)));
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    info.root.set("Mod_HakList", mg_module::attach::hak_list(&info.root, &["mg_lights".into()]));
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_light.mod"))).unwrap();

    let run = run_server(&root, &user, "mg_light", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let seen = run.values("MG_ROW");
    assert_eq!(seen.len(), 2, "two probe lines:\n{}", run.log);
    for ((_, _, model), (line, is_static)) in lights.iter().zip(seen.iter().zip(["1", "0"])) {
        let parts: Vec<&str> = line.split('|').collect();
        assert!(parts[0].eq_ignore_ascii_case(model), "the row names the model: {line}");
        assert!(parts[1].to_ascii_lowercase().contains("mg_lights"), "found in the hak: {line}");
        assert_eq!(&parts[2..], ["1", "64", is_static], "a placeable of that row, placed: {line}");
    }
}
