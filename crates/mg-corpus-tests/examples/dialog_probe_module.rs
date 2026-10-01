//! Writes a module for Aurora's Conversation Editor options (the Input
//! Text popup, the Paste As Link and drag link directions): one area and
//! the conversation `zz_conv`, the greetings "Hello" (answered by "Hi") and
//! "Second":
//! `cargo run -p mg-corpus-tests --example dialog_probe_module OUT.mod`.
use mg_core::{ResRef, ResType};
use mg_module::ModuleLocation;
use mg_module::dialog::{Kind, Parent, add_node, new_dialog};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;

fn main() {
    let out = std::env::args().nth(1).expect("output module");
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(17);
    let mut m = new_module(&game, "Dialog Probe", &mut rng).unwrap();
    let tileset = ResRef::from_str("ttr01").unwrap();
    let spec = AreaSpec { name: "Field".into(), tileset, width: 4, height: 4 };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Hi");
    add_node(&mut g, Parent::Root, "Second");
    m.set_gff(ResKey::parse("zz_conv", ResType::DLG).unwrap(), &g).unwrap();
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
