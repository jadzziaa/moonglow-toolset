//! Writes a module for Aurora's area context menu (Setup Store on a
//! creature, Add Popup Text on a placeable), to capture what those write:
//! one flat 4 by 4 rural area with a bandit placed at its centre (20, 20)
//! and a store blueprint (`mgp_store`) in the custom palette; with
//! `placeable`, a barrel (`x3_plc_barrel1`) at the centre and the bandit at
//! (30, 30):
//! `cargo run -p mg-corpus-tests --example popup_probe_module OUT.mod [placeable]`.
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
    let mut rng = fastrand::Rng::with_seed(13);
    let mut m = new_module(&game, "Popup Probe", &mut rng).unwrap();
    let spec = AreaSpec { name: "Field".into(), tileset: r("ttr01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let read = |res: &str, t: ResType| {
        Gff::read(&game.resman.get(&ResKey::new(r(res), t)).unwrap()).unwrap()
    };
    let item = |res: ResRef| {
        let data = game.resman.get(&ResKey::new(res, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let placing = Placing { game: &game, item: &item };
    let with_placeable = std::env::args().nth(2).is_some_and(|a| a == "placeable");
    let bandit = read("nw_bandit001", ResType::UTC);
    let at = if with_placeable { [30.0, 30.0, 0.0] } else { [20.0, 20.0, 0.0] };
    let at = Placement { position: at, rotation: 0.0 };
    let creature = instance(&placing, ResType::UTC, &bandit.root, at, &[]).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTC).unwrap();
    git.root.set(list, Value::List(vec![creature]));
    if with_placeable {
        let barrel = read("x3_plc_barrel1", ResType::UTP);
        let at = Placement { position: [20.0, 20.0, 0.0], rotation: 0.0 };
        let placed = instance(&placing, ResType::UTP, &barrel.root, at, &[]).unwrap();
        let (list, _) = git_list(ResType::UTP).unwrap();
        git.root.set(list, Value::List(vec![placed]));
    }
    m.set_gff(git_key, &git).unwrap();
    let mut store = read("nw_storgenral003", ResType::UTM);
    store.root.set("ResRef", Value::resref(r("mgp_store")));
    store.root.set("Tag", Value::String(b"MGP_STORE".to_vec()));
    m.set_gff(ResKey::new(r("mgp_store"), ResType::UTM), &store).unwrap();
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
