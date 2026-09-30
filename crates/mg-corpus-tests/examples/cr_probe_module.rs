//! Writes a module of creatures with a wrong stored CR (42), for Aurora to
//! recompute when each one's properties are confirmed:
//! `cargo run -p mg-corpus-tests --example cr_probe_module OUT.mod`.
//! Each creature is the bandit (nw_bandit001) with one thing changed.
use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;

/// (name, classes as (class, level), ability overrides, natural AC, base HP,
/// special abilities, drop feats)
type Probe =
    (&'static str, &'static [(i32, i16)], &'static [(&'static str, u8)], u8, i16, u8, bool);

const PROBES: &[Probe] = &[
    ("cra01", &[(4, 1)], &[], 0, 10, 0, false),
    ("cra02", &[(4, 2)], &[], 0, 10, 0, false),
    ("cra03", &[(4, 3)], &[], 0, 10, 0, false),
    ("cra04", &[(4, 5)], &[], 0, 10, 0, false),
    ("cra05", &[(4, 10)], &[], 0, 10, 0, false),
    ("cra06", &[(4, 20)], &[], 0, 10, 0, false),
    ("cra07", &[(10, 5)], &[], 0, 10, 0, false),
    ("cra08", &[(20, 1)], &[], 0, 10, 0, false),
    ("cra09", &[(20, 5)], &[], 0, 10, 0, false),
    ("cra10", &[(12, 1)], &[], 0, 10, 0, false),
    ("cra11", &[(12, 5)], &[], 0, 10, 0, false),
    ("cra12", &[(18, 10)], &[], 0, 10, 0, false),
    ("cra13", &[(4, 5), (10, 5)], &[], 0, 10, 0, false),
    ("cra14", &[(4, 5)], &[("Str", 20)], 0, 10, 0, false),
    ("cra15", &[(4, 5)], &[("Con", 20)], 0, 10, 0, false),
    ("cra16", &[(4, 5)], &[("Dex", 20)], 0, 10, 0, false),
    ("cra17", &[(4, 5)], &[], 10, 10, 0, false),
    ("cra18", &[(4, 5)], &[], 0, 100, 0, false),
    ("cra19", &[(4, 5)], &[], 0, 10, 3, false),
    ("cra20", &[(4, 5)], &[], 0, 10, 0, true),
    ("cra21", &[(4, 1)], &[("Str", 3), ("Dex", 3), ("Con", 3)], 0, 1, 0, true),
    ("cra22", &[(19, 8)], &[], 0, 10, 0, false),
];

fn main() {
    let out = std::env::args().nth(1).expect("output module");
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "CR Probe", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "A".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 2,
        height: 2,
    };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let base = game
        .resman
        .get(&ResKey::new(ResRef::from_str("nw_bandit001").unwrap(), ResType::UTC))
        .unwrap();
    for (name, classes, abilities, nac, hp, specials, no_feats) in PROBES {
        let mut g = Gff::read(&base).unwrap();
        g.root.set("TemplateResRef", Value::resref(ResRef::from_str(name).unwrap()));
        g.root.set(
            "FirstName",
            Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, *name)),
        );
        g.root.set("LastName", Value::LocString(LocString::default()));
        g.root.set("Tag", Value::String(name.to_uppercase().into_bytes()));
        g.root.set("ChallengeRating", Value::Float(42.0));
        g.root.set("PaletteID", Value::Byte(45));
        let list: Vec<Struct> = classes
            .iter()
            .map(|(c, l)| {
                let mut s = Struct::new(2);
                s.set("Class", Value::Int(*c));
                s.set("ClassLevel", Value::Short(*l));
                s
            })
            .collect();
        g.root.set("ClassList", Value::List(list));
        for (a, v) in *abilities {
            g.root.set(a, Value::Byte(*v));
        }
        g.root.set("NaturalAC", Value::Byte(*nac));
        g.root.set("HitPoints", Value::Short(*hp));
        g.root.set("CurrentHitPoints", Value::Short(*hp));
        let spec: Vec<Struct> = (0..*specials)
            .map(|i| {
                let mut s = Struct::new(4);
                s.set("Spell", Value::Word([58, 25, 88][i as usize])); // fireball, bless?, lightning
                s.set("SpellCasterLevel", Value::Byte(5));
                s.set("SpellFlags", Value::Byte(1));
                s
            })
            .collect();
        g.root.set("SpecAbilityList", Value::List(spec));
        if *no_feats {
            g.root.set("FeatList", Value::List(Vec::new()));
        }
        m.set_gff(ResKey::parse(name, ResType::UTC).unwrap(), &g).unwrap();
    }
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
