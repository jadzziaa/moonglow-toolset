//! Writes a module for Aurora's creature spell warnings (Options › General,
//! "Show invalid creature spell assignment warning"), to capture which
//! assignments it warns about: six bandits made spellcasters, each breaking
//! one rule, in a row across a flat 4 by 4 rural area at (8, 20), (12, 20),
//! ... (28, 20):
//! 0. Wizard 1, Int 18, a level 2 spell (above what the level casts);
//! 1. Wizard 5, Int 12, Fireball (level 3 needs Int 13);
//! 2. Wizard 1, Int 10, four cantrips (three slots);
//! 3. Wizard 1, Int 18, two level 1 spells (one slot and a bonus one);
//! 4. Half-orc Wizard 1, Int 12 (10 with the race's - 2), a level 1 spell;
//! 5. Sorcerer 1, Cha 10, five known cantrips (four known).
//!
//! The second set (`2`) narrows down the maximum count:
//! 0. Wizard 1, Int 10, two level 1 spells;
//! 1. Wizard 1, Int 18, three level 1 spells;
//! 2. Wizard 2, Int 18, a level 2 spell;
//! 3. Sorcerer 1, Cha 10, six known cantrips;
//! 4. Wizard 3, Int 10, two level 2 spells;
//! 5. Wizard 2, Int 18, five cantrips.
//!
//! The third (`3`):
//! 0. Sorcerer 1, Cha 18, six known cantrips;
//! 1. Wizard 3, Int 14, three level 2 spells;
//! 2. Half-orc Wizard 1, Int 13 (11), two level 1 spells;
//! 3. Wizard 1, Int 10, a level 2 spell (too high and too little Int);
//! 4. Wizard 1, Int 18, a level 2 spell, and Sorcerer 1, six cantrips;
//! 5. Bard 1, Cha 10, four known cantrips.
//!
//! The fourth (`4`):
//! 0. Bard 1, Cha 10, five known cantrips;
//! 1. Sorcerer 1, Cha 10, three known level 1 spells;
//! 2. Bard 1, Cha 12, a known level 1 spell;
//! 3. Sorcerer 2, Cha 10, six known cantrips;
//! 4. Cleric 1, Wis 10, five cantrips;
//! 5. Cleric 1, Wis 12, four level 1 spells.
//!
//! The fifth (`5`):
//! 0. Sorcerer 1, Cha 11, three known level 1 spells;
//! 1. Sorcerer 1, Cha 18, three known level 1 spells;
//! 2. Wizard 1, Int 18, five cantrips;
//! 3. Sorcerer 1, Cha 10, Int and Wis 18, six known cantrips;
//! 4. Wizard 1, Int 18, six cantrips;
//! 5. Sorcerer 1, Cha 18, four known level 1 spells.
//!
//! And `inventory`: six Fighter 1 bandits without feats or equipment, for
//! the creature inventory warning (equipping what needs a feat).
//!
//! `cargo run -p mg-corpus-tests --example spell_warning_probe_module OUT.mod [2|3|4|5|inventory]`.
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

/// A class entry: class, level and spell lists (label, spells).
fn class(class: i32, level: i16, lists: &[(&str, &[u16])]) -> Struct {
    let mut c = Struct::new(2);
    c.set("Class", Value::Int(class));
    c.set("ClassLevel", Value::Short(level));
    for (label, spells) in lists {
        let entries = spells
            .iter()
            .map(|&sp| {
                let mut s = Struct::new(3);
                s.set("Spell", Value::Word(sp));
                s.set("SpellMetaMagic", Value::Byte(0));
                s.set("SpellFlags", Value::Byte(1));
                s
            })
            .collect();
        c.set(label, Value::List(entries));
    }
    c
}

fn main() {
    let out = std::env::args().nth(1).expect("output module");
    let set = std::env::args().nth(2).unwrap_or_default();
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(17);
    let mut m = new_module(&game, "Spell Warning Probe", &mut rng).unwrap();
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
    // (race, Int and Wis, Cha, class entries)
    let (bard, cleric, sorcerer, wizard) = (1, 2, 9, 10);
    let cantrips: &[u16] = &[37, 100, 144, 151, 416, 424];
    let probes: Vec<(u8, u8, u8, Vec<Struct>)> = match set.as_str() {
        "inventory" => (0..6).map(|_| (6, 10, 10, vec![class(4, 1, &[])])).collect(),
        "5" => vec![
            (6, 10, 11, vec![class(sorcerer, 1, &[("KnownList1", &[107, 102, 10])])]),
            (6, 10, 18, vec![class(sorcerer, 1, &[("KnownList1", &[107, 102, 10])])]),
            (6, 18, 10, vec![class(wizard, 1, &[("MemorizedList0", &cantrips[..5])])]),
            (6, 18, 10, vec![class(sorcerer, 1, &[("KnownList0", cantrips)])]),
            (6, 18, 10, vec![class(wizard, 1, &[("MemorizedList0", cantrips)])]),
            (6, 10, 18, vec![class(sorcerer, 1, &[("KnownList1", &[107, 102, 10, 24])])]),
        ],
        "4" => vec![
            (6, 10, 10, vec![class(bard, 1, &[("KnownList0", &[33, 37, 100, 151, 416])])]),
            (6, 10, 10, vec![class(sorcerer, 1, &[("KnownList1", &[107, 102, 10])])]),
            (6, 10, 12, vec![class(bard, 1, &[("KnownList1", &[32])])]),
            (6, 10, 10, vec![class(sorcerer, 2, &[("KnownList0", cantrips)])]),
            (6, 10, 10, vec![class(cleric, 1, &[("MemorizedList0", &[33, 33, 33, 33, 33])])]),
            (6, 12, 10, vec![class(cleric, 1, &[("MemorizedList1", &[32, 32, 32, 32])])]),
        ],
        "3" => vec![
            (6, 10, 18, vec![class(sorcerer, 1, &[("KnownList0", cantrips)])]),
            (6, 14, 10, vec![class(wizard, 3, &[("MemorizedList2", &[9, 13, 36])])]),
            (5, 13, 10, vec![class(wizard, 1, &[("MemorizedList1", &[107, 102])])]),
            (6, 10, 10, vec![class(wizard, 1, &[("MemorizedList2", &[9])])]),
            (
                6,
                18,
                10,
                vec![
                    class(wizard, 1, &[("MemorizedList2", &[9])]),
                    class(sorcerer, 1, &[("KnownList0", cantrips)]),
                ],
            ),
            (6, 10, 10, vec![class(bard, 1, &[("KnownList0", &[33, 37, 100, 151])])]),
        ],
        "2" => [
            (6, 10, 10, class(wizard, 1, &[("MemorizedList1", &[107, 102])])),
            (6, 18, 10, class(wizard, 1, &[("MemorizedList1", &[107, 102, 10])])),
            (6, 18, 10, class(wizard, 2, &[("MemorizedList2", &[9])])),
            (6, 10, 10, class(sorcerer, 1, &[("KnownList0", cantrips)])),
            (6, 10, 10, class(wizard, 3, &[("MemorizedList2", &[9, 13])])),
            (6, 18, 10, class(wizard, 2, &[("MemorizedList0", &cantrips[..5])])),
        ]
        .map(|(r, i, c, e)| (r, i, c, vec![e]))
        .into(),
        _ => [
            (6, 18, 10, class(wizard, 1, &[("MemorizedList2", &[9])])),
            (6, 12, 10, class(wizard, 5, &[("MemorizedList3", &[58])])),
            (6, 10, 10, class(wizard, 1, &[("MemorizedList0", &cantrips[..4])])),
            (6, 18, 10, class(wizard, 1, &[("MemorizedList1", &[107, 102])])),
            (5, 12, 10, class(wizard, 1, &[("MemorizedList1", &[107])])),
            (6, 10, 10, class(sorcerer, 1, &[("KnownList0", &cantrips[..5])])),
        ]
        .map(|(r, i, c, e)| (r, i, c, vec![e]))
        .into(),
    };
    let creatures = probes
        .into_iter()
        .enumerate()
        .map(|(i, (race, int, cha, entries))| {
            let at = Placement { position: [8.0 + 4.0 * i as f32, 20.0, 0.0], rotation: 0.0 };
            let mut c = instance(&placing, ResType::UTC, &bandit.root, at, &[]).unwrap();
            c.set("Tag", Value::String(format!("SPW{i}").into_bytes()));
            c.set("Race", Value::Byte(race));
            c.set("Int", Value::Byte(int));
            c.set("Wis", Value::Byte(int));
            c.set("Cha", Value::Byte(cha));
            c.set("ClassList", Value::List(entries));
            if set == "inventory" {
                c.set("FeatList", Value::List(Vec::new()));
                c.set("Equip_ItemList", Value::List(Vec::new()));
                c.set("ItemList", Value::List(Vec::new()));
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
