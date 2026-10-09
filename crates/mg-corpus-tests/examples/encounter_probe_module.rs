//! Writes a module for Aurora's Build (Compile › Encounters), to capture
//! what it does to encounters' creature lists: an encounter blueprint
//! (`mgp_enc`) and a placed copy, each listing the bandit, a custom
//! creature (`mgp_crit`, its CR 7 and appearance 4 stored) and a creature
//! that does not exist, every entry with a stale CR (99) and appearance
//! (0):
//! `cargo run -p mg-corpus-tests --example encounter_probe_module OUT.mod`.
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
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
    let mut rng = fastrand::Rng::with_seed(19);
    let mut m = new_module(&game, "Encounter Probe", &mut rng).unwrap();
    let spec = AreaSpec { name: "Field".into(), tileset: r("ttr01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let read = |res: &str, t: ResType| {
        Gff::read(&game.resman.get(&ResKey::new(r(res), t)).unwrap()).unwrap()
    };
    let mut crit = read("nw_bandit001", ResType::UTC);
    crit.root.set("TemplateResRef", Value::resref(r("mgp_crit")));
    crit.root.set("ChallengeRating", Value::Float(7.0));
    crit.root.set("Appearance_Type", Value::Word(4));
    m.set_gff(ResKey::new(r("mgp_crit"), ResType::UTC), &crit).unwrap();
    let entry = |res: &str| {
        let mut e = Struct::new(0);
        e.set("ResRef", Value::resref(r(res)));
        e.set("CR", Value::Float(99.0));
        e.set("Appearance", Value::Int(0));
        e.set("SingleSpawn", Value::Byte(0));
        e
    };
    let list = vec![entry("nw_bandit001"), entry("mgp_crit"), entry("mgp_nosuch")];
    let mut enc = read("nw_giantevil", ResType::UTE);
    enc.root.set("TemplateResRef", Value::resref(r("mgp_enc")));
    enc.root.set("CreatureList", Value::List(list));
    m.set_gff(ResKey::new(r("mgp_enc"), ResType::UTE), &enc).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let outline = [[-3.0, -3.0, 0.0], [3.0, -3.0, 0.0], [3.0, 3.0, 0.0], [-3.0, 3.0, 0.0]];
    let at = Placement { position: [20.0, 20.0, 0.0], rotation: 0.0 };
    let placed = instance(&placing, ResType::UTE, &enc.root, at, &outline).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set(git_list(ResType::UTE).unwrap().0, Value::List(vec![placed]));
    m.set_gff(git_key, &git).unwrap();
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
