//! Writes a module for Aurora's Levelup Wizard, to capture how it levels
//! creatures up: six bandits (Fighter 1) in a row across a flat 4 by 4
//! rural area, at (8, 20), (12, 20), ... (28, 20), facing north:
//! `cargo run -p mg-corpus-tests --example levelup_probe_module OUT.mod`.
//! With `spells` after it, three for a new class's spells (`spells-before.mod`):
//! the first lawful good with Wisdom 14 (Paladin 4 added: a level whose
//! slots are bonus slots alone), the second and third as they are
//! (Sorcerer 1 and Bard 1 added: classes that know their spells). (Each
//! levelled from its Properties window's Classes page, 2026-11-10. The
//! wizard refused every level of the first, even Fighter 2, saying
//! "Maximum total character level is 180", while its Wisdom was the
//! file's 14; set to 12 and back to 14 on the Statistics page, it took
//! Paladin 4. The other two were warned their Charisma is too low for
//! their spells: Yes.)
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
    let spells = std::env::args().nth(2).as_deref() == Some("spells");
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
    let creatures = (0..if spells { 3 } else { 6 })
        .map(|i| {
            let at = Placement { position: [8.0 + 4.0 * i as f32, 20.0, 0.0], rotation: 0.0 };
            let mut c = instance(&placing, ResType::UTC, &bandit.root, at, &[]).unwrap();
            c.set("Tag", Value::String(format!("LVL{i}").into_bytes()));
            if spells && i == 0 {
                c.set("Wis", Value::Byte(14));
                c.set("GoodEvil", Value::Byte(100));
                c.set("LawfulChaotic", Value::Byte(100));
            }
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
