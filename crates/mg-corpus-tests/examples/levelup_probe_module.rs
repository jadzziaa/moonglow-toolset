//! Writes a module for Aurora's Levelup Wizard, to capture how it levels
//! creatures up: six bandits (Fighter 1) in a row across a flat 4 by 4
//! rural area, at (8, 20), (12, 20), ... (28, 20), facing north:
//! `cargo run -p mg-corpus-tests --example levelup_probe_module OUT.mod`.
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

fn main() {
    let out = std::env::args().nth(1).expect("output module");
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(17);
    let mut m = new_module(&game, "Levelup Probe", &mut rng).unwrap();
    let spec = AreaSpec { name: "Field".into(), tileset: r("ttr01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bandit =
        Gff::read(&game.resman.get(&ResKey::new(r("nw_bandit001"), ResType::UTC)).unwrap())
            .unwrap();
    let item = |res: ResRef| {
        let data = game.resman.get(&ResKey::new(res, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let placing = Placing { game: &game, item: &item };
    let creatures = (0..6)
        .map(|i| {
            let at = Placement { position: [8.0 + 4.0 * i as f32, 20.0, 0.0], rotation: 0.0 };
            let mut c = instance(&placing, ResType::UTC, &bandit.root, at, &[]).unwrap();
            c.set("Tag", Value::String(format!("LVL{i}").into_bytes()));
            c
        })
        .collect();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set(git_list(ResType::UTC).unwrap().0, Value::List(creatures));
    m.set_gff(git_key, &git).unwrap();
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
