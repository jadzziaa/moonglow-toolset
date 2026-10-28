//! A lighting scheme chosen for an existing area, compared with Aurora's
//! Area Properties › Visual. The probe is the placement capture
//! (`placement-defaults.mod`) with each tile's four lights set to 31, its
//! `FogClipDist` to 77 and its `SkyBox` to 3 (`lighting-scheme-probe.mod`);
//! Aurora opened it, chose Interior Evil (environment.2da 16) for its
//! area, OK, File › Save: `lighting-scheme.mod`.
//!
//! Aurora sets the scheme's fields, sets the fog's clip distance and the
//! sky box back to a new area's (45, 0), and picks every tile's lights
//! anew from the scheme's colors, both source lights alike. (It also
//! counts the area's `Version` up, as it does whenever it saves an area.)

use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_module::Module;
use mg_module::areas::{Change, apply};
use mg_module::new::{Scheme, TILE_LIGHT_FIELDS};
use mg_resman::ResKey;
use mg_testkit::{aurora_capture, corpus};

const INTERIOR_EVIL: usize = 16;

/// The probe Aurora was given, made again from the placement capture.
fn probe(capture: &std::path::Path) -> Module {
    let mut m = Module::open(capture).unwrap();
    let key = ResKey::new(ResRef::from_str("field").unwrap(), ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    for tile in are.root.list_mut("Tile_List").unwrap() {
        for label in TILE_LIGHT_FIELDS {
            tile.set(label, Value::Byte(31));
        }
    }
    are.root.set("FogClipDist", Value::Float(77.0));
    are.root.set("SkyBox", Value::Byte(3));
    m.set_gff(key, &are).unwrap();
    m
}

#[test]
fn a_lighting_scheme_is_given_as_aurora_gives_it() {
    use mg_resman::GameInstall;
    use mg_rules::GameData;
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let aurora = Module::open(&aurora_capture!("lighting-scheme.mod")).unwrap();
    let mut m = probe(&aurora_capture!("placement-defaults.mod"));
    let area = ResRef::from_str("field").unwrap();
    let key = ResKey::new(area, ResType::ARE);
    let before = m.gff(&key).unwrap().unwrap().root;
    let scheme = Scheme::read(&game, INTERIOR_EVIL).unwrap();
    let env = game.table("environment").unwrap();
    apply(&mut m, &[area], &[Change::LightingScheme(scheme)], mg_core::Codepage::WINDOWS_1252, 1)
        .unwrap();
    let ours = m.gff(&key).unwrap().unwrap().root;
    let theirs = aurora.gff(&key).unwrap().unwrap().root;
    // The fields, in Aurora's order, but the version it counts and the
    // tiles' lights it picks at random.
    let fields = |s: &Struct| -> Vec<(String, Option<Value>)> {
        s.fields
            .iter()
            .map(|f| f.label.to_string_lossy().to_string())
            .filter(|l| l != "Version" && l != "Tile_List")
            .map(|l| (l.clone(), s.get(&l).cloned()))
            .collect()
    };
    assert_eq!(fields(&ours), fields(&theirs));
    assert_ne!(fields(&before), fields(&theirs), "Aurora changed the fields");
    let colors = |prefix: &str| -> Vec<i64> {
        (1..=4)
            .map(|n| i64::from(env.get_int(INTERIOR_EVIL, &format!("{prefix}{n}")).unwrap()))
            .collect()
    };
    let (ours, theirs) = (ours.list("Tile_List").unwrap(), theirs.list("Tile_List").unwrap());
    assert_eq!(ours.len(), theirs.len());
    for (o, t) in ours.iter().zip(theirs) {
        for tile in [o, t] {
            let light = |l: &str| tile.integer(l).unwrap();
            assert!(colors("MAIN1_COLOR").contains(&light("Tile_MainLight1")), "{tile:?}");
            assert!(colors("MAIN2_COLOR").contains(&light("Tile_MainLight2")), "{tile:?}");
            assert!(colors("SECONDARY_COLOR").contains(&light("Tile_SrcLight1")), "{tile:?}");
            assert_eq!(light("Tile_SrcLight2"), light("Tile_SrcLight1"));
        }
        // The rest of each tile as it was.
        let rest = |s: &Struct| -> Vec<String> {
            s.fields
                .iter()
                .filter(|f| !TILE_LIGHT_FIELDS.contains(&f.label.to_string_lossy().as_ref()))
                .map(|f| format!("{} {:?}", f.label.to_string_lossy(), f.value))
                .collect()
        };
        assert_eq!(rest(o), rest(t));
    }
}
