//! Writes a module for Aurora to paint terrain in, to capture how its
//! brushes change an area's tiles: one area per `TILESET:WIDTHxHEIGHT`
//! argument, as Moonglow's area wizard makes it, named after its tileset,
//! after a small area for the module's start location (Aurora will not save
//! a module whose start location is painted over):
//! `cargo run -p mg-corpus-tests --example terrain_probe_module OUT.mod ttr01:8x8 tic01:6x6`.
use mg_core::ResRef;
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::GameInstall;
use mg_rules::GameData;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("output module");
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Terrain Probe", &mut rng).unwrap();
    let start = AreaSpec {
        name: "START".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    add_area(&mut m, &game, &start, &mut rng).unwrap();
    for spec in args {
        let (tileset, size) = spec.split_once(':').expect("TILESET:WxH");
        let (w, h) = size.split_once('x').expect("WxH");
        let spec = AreaSpec {
            name: tileset.to_uppercase(),
            tileset: ResRef::from_str(tileset).unwrap(),
            width: w.parse().unwrap(),
            height: h.parse().unwrap(),
        };
        add_area(&mut m, &game, &spec, &mut rng).unwrap();
    }
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
