//! An area's grass settings in the engine: where it reads what
//! `SetAreaGrassOverride` and `SetAreaDefaultGrassDisabled` set. No script
//! reads them back, so the server saves the game (`save` on its console)
//! and the saved areas are read: the engine keeps them in a GIT's
//! `AreaProperties` and reads them from a module's GIT when the module
//! loads; the same fields in the ARE are not read.

use std::time::Duration;

use mg_area::grass::{self, AreaGrass, GrassLook, GrassOverride};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Workspace};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{save_game, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::compile;

const READY: &str = r#"
void main()
{
    WriteTimestampedLogEntry("MG_READY");
}
"#;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

fn an_override(material: u32, texture: &str) -> GrassOverride {
    GrassOverride {
        material,
        look: GrassLook {
            texture: texture.into(),
            density: 2.5,
            height: 0.75,
            ambient: [0.25, 0.5, 0.75],
            diffuse: [0.5, 0.75, 1.0],
        },
    }
}

#[test]
fn an_area_s_grass_is_read_from_its_git() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_grass");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Grass", &mut rng).unwrap();
    let mut area = |name: &str| {
        let spec = AreaSpec { name: name.into(), tileset: r("ttr01"), width: 2, height: 2 };
        add_area(&mut m, &game, &spec, &mut rng).unwrap()
    };
    let (field, meadow) = (area("Field"), area("Meadow"));
    // Meadow: the fields in its ARE.
    let key = ResKey::new(meadow, ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    are.root.set(grass::DISABLED, Value::Byte(1));
    let mut list = mg_gff::Struct::new(grass::OVERRIDES_ID);
    list.set(grass::LIST, Value::List(vec![an_override(9, "mgare").item()]));
    are.root.set(grass::OVERRIDES, Value::Struct(list));
    m.set_gff(key, &are).unwrap();
    let ready = compile(&dir, "mg_ready", READY);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_ready"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_ready"), ResType::NCS), ready);
    // Field: in its GIT, as Moonglow's edits write them: two overrides
    // and the tileset's grass off.
    let wanted = AreaGrass {
        default_disabled: true,
        overrides: vec![an_override(7, "mggit"), an_override(1, "mgdirt")],
    };
    let key = ResKey::new(field, ResType::GIT);
    let mut ws = Workspace::new(m);
    let git = ws.doc(&key).unwrap().root.clone();
    ws.apply(Command::new("Grass off", grass::set_disabled(key, &git, true))).unwrap();
    for o in &wanted.overrides {
        let git = ws.doc(&key).unwrap().root.clone();
        ws.apply(Command::new("Grass", grass::set_override(key, &git, o))).unwrap();
    }
    let user = dir.join("user");
    ws.save_as(&ModuleLocation::Archive(user.join("modules/mg_grass.mod"))).unwrap();

    let sav = save_game(&root, &user, "mg_grass", "MG_READY", Duration::from_secs(120))
        .unwrap_or_else(|e| panic!("the server did not save the game: {e}"));
    let data = std::fs::read(&sav).unwrap();
    let erf = mg_erf::Erf::read(&data).unwrap();
    let saved = |area: ResRef| {
        let entry = erf.find(&area, ResType::GIT).expect("the save has the area's GIT");
        let mut grass = AreaGrass::read(&Gff::read(&erf.data(entry).unwrap()).unwrap().root);
        // (The engine keeps them by material, in no order.)
        grass.overrides.sort_by_key(|o| o.material);
        grass
    };
    let mut wanted = wanted;
    wanted.overrides.sort_by_key(|o| o.material);
    assert_eq!(saved(field), wanted, "the engine kept the GIT's grass");
    assert_eq!(saved(meadow), AreaGrass::default(), "the engine did not read the ARE's");
}
