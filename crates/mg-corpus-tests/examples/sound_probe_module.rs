//! Writes a module for Aurora's Sound Properties, to capture what its play
//! styles and positioning write (`Looping`, `Continuous`, `Positional`,
//! `Priority`): six "animalcriesday" sounds in a row across a flat 4 by 4
//! rural area, at (8, 20), (12, 20), ... (28, 20):
//! `cargo run -p mg-corpus-tests --example sound_probe_module OUT.mod`.
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;

fn main() {
    let out = std::env::args().nth(1).expect("output module");
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(17);
    let mut m = new_module(&game, "Sound Probe", &mut rng).unwrap();
    let tileset = ResRef::from_str("ttr01").unwrap();
    let spec = AreaSpec { name: "Field".into(), tileset, width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bp = ResKey::parse("animalcriesday", ResType::UTS).unwrap();
    let bp = Gff::read(&game.resman.get(&bp).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let sounds = (0..6)
        .map(|i| {
            let at = Placement { position: [8.0 + 4.0 * i as f32, 20.0, 0.0], rotation: 0.0 };
            let mut s = instance(&placing, ResType::UTS, &bp.root, at, &[]).unwrap();
            s.set("Tag", Value::String(format!("SND{i}").into_bytes()));
            s
        })
        .collect();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set(git_list(ResType::UTS).unwrap().0, Value::List(sounds));
    m.set_gff(git_key, &git).unwrap();
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
