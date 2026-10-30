//! A light Moonglow made (Custom Light, `mg_module::lights`), in the
//! engine: the server reads the hak it was written into, finds its model
//! there and its placeables.2da row naming it, and places a placeable of
//! that row. Whether the light lights creatures and placeables, and how
//! many such lights the game draws at one place, are the client's to say
//! (the deferred list's "Lights placed on the fly, way B").

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::lights::{LightSpec, new_hak, plan, write};
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

/// Logs the row's model as the server's placeables.2da has it, where the
/// server finds that model, and the placed light.
fn probe(row: usize, model: &str) -> String {
    format!(
        r#"
void main()
{{
    string sModel = Get2DAString("placeables", "ModelName", {row});
    object o = GetObjectByTag("MG_LIGHT");
    WriteTimestampedLogEntry("MG_ROW " + sModel + "|" + ResManGetAliasFor("{model}", RESTYPE_MDL)
        + "|" + IntToString(GetIsObjectValid(o)) + "|" + IntToString(GetObjectType(o)));
    WriteTimestampedLogEntry("MG_DONE");
}}
"#
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
    let p = plan(&game.resman, game.codepage(), spec, hak, None).unwrap();
    write(&p, game.codepage()).unwrap();
    assert!(user.join("hak/mg_lights.hak").is_file());

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
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git: Gff = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTP).unwrap();
    git.root.set(list, Value::List(vec![placed]));
    m.set_gff(git_key, &git).unwrap();
    let model = p.model.to_string();
    m.set(
        ResKey::new(r("mg_probe"), ResType::NCS),
        compile(&dir, "mg_probe", &probe(p.row, &model)),
    );
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    info.root.set("Mod_HakList", mg_module::attach::hak_list(&info.root, &["mg_lights".into()]));
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_light.mod"))).unwrap();

    let run = run_server(&root, &user, "mg_light", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let seen = run.values("MG_ROW");
    let line = seen.first().unwrap_or_else(|| panic!("no probe line:\n{}", run.log));
    let parts: Vec<&str> = line.split('|').collect();
    assert!(parts[0].eq_ignore_ascii_case(&model), "the row names the model: {line}");
    assert!(parts[1].to_ascii_lowercase().contains("mg_lights"), "found in the hak: {line}");
    assert_eq!(&parts[2..], ["1", "64"], "a placeable of that row, placed: {line}");
}
