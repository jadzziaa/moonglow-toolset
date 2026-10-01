//! Writes a module of creature blueprints for Aurora's Build (Compile ›
//! Creature CR) to rate, to work out its challenge rating calculation:
//! `cargo run -p mg-corpus-tests --example cr_probe2_module OUT.mod [hp] > probes.tsv`
//! (`hp`: instead, hit points against level for five hit dice; `fine`: in
//! finer steps; `terms`: movement rates, special abilities, class spells,
//! gear and feats without a CRValue, each over hit points 1 to 90, so where
//! the rating steps up shows how much the change adds).
//! Each creature is a plain one (human, abilities 10, no feats, skills,
//! special abilities or equipment, natural AC 0, base HP 25, Fighter 5)
//! with one thing changed; the TSV on stdout says what (name, factor,
//! value).
use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;

/// What a probe changes.
#[derive(Clone)]
enum Change {
    Base,
    Level(i16, i16),
    Hp(i16),
    NaturalAc(u8),
    Feats(Vec<u16>),
    Specials(u8),
    Ability(&'static str, u8),
    Class(i32, i16),
    Race(u8),
    Save(&'static str, i16),
    Skills(u8),
    Appearance(u16),
    Classes(Vec<(i32, i16)>),
    ClassHp(i32, i16, i16),
    /// Changes made by a function, and base hit points.
    Edit(std::rc::Rc<dyn Fn(&mut Struct)>, i16),
}

/// A spell list entry.
fn spell(id: u16) -> Struct {
    let mut s = Struct::new(3);
    s.set("Spell", Value::Word(id));
    s.set("SpellFlags", Value::Byte(1));
    s.set("SpellMetaMagic", Value::Byte(0));
    s
}

fn main() {
    let out = std::env::args().nth(1).expect("output module");
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "CR Probe 2", &mut rng).unwrap();
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
    let feat_cr = |cr: &str| -> Vec<u16> {
        let t = game.table("feat").unwrap();
        (0..t.len())
            .filter(|&r| t.get(r, "CRValue").is_some_and(|v| v.trim() == cr))
            .filter(|&r| t.get(r, "ALLCLASSESCANUSE").is_some_and(|v| v.trim() == "1"))
            .map(|r| r as u16)
            .take(30)
            .collect()
    };
    let (f1, f05, f02) = (feat_cr("1"), feat_cr("0.5"), feat_cr("0.2"));

    let mut probes: Vec<(String, String, Change)> =
        vec![("c000".into(), "base".into(), Change::Base)];
    let sweep = std::env::args().nth(2).is_some_and(|a| a == "hp");
    let fine = std::env::args().nth(2).is_some_and(|a| a == "fine");
    let terms = std::env::args().nth(2).is_some_and(|a| a == "terms");
    let mut add = |factor: &str, value: String, c: Change| {
        let name = format!("c{:03}", probes.len());
        probes.push((name, format!("{factor}\t{value}"), c));
    };
    if terms {
        use std::rc::Rc;
        type E = Rc<dyn Fn(&mut Struct)>;
        let class = |c: i32| -> E {
            Rc::new(move |r: &mut Struct| {
                let mut s = Struct::new(2);
                s.set("Class", Value::Int(c));
                s.set("ClassLevel", Value::Short(5));
                r.set("ClassList", Value::List(vec![s]));
            })
        };
        let with_list = |c: i32, list: &'static str, spells: Vec<u16>| -> E {
            Rc::new(move |r: &mut Struct| {
                let mut s = Struct::new(2);
                s.set("Class", Value::Int(c));
                s.set("ClassLevel", Value::Short(5));
                s.set(list, Value::List(spells.iter().map(|&i| spell(i)).collect()));
                r.set("ClassList", Value::List(vec![s]));
            })
        };
        let specials = |spells: Vec<u16>| -> E {
            Rc::new(move |r: &mut Struct| {
                let list = spells
                    .iter()
                    .map(|&i| {
                        let mut s = Struct::new(4);
                        s.set("Spell", Value::Word(i));
                        s.set("SpellCasterLevel", Value::Byte(5));
                        s.set("SpellFlags", Value::Byte(1));
                        s
                    })
                    .collect();
                r.set("SpecAbilityList", Value::List(list));
            })
        };
        let equip = |items: Vec<(u32, &'static str)>| -> E {
            Rc::new(move |r: &mut Struct| {
                let list = items
                    .iter()
                    .map(|&(slot, res)| {
                        let mut s = Struct::new(slot);
                        s.set("EquippedRes", Value::resref(ResRef::from_str(res).unwrap()));
                        s
                    })
                    .collect();
                r.set("Equip_ItemList", Value::List(list));
            })
        };
        let carry = |items: Vec<&'static str>| -> E {
            Rc::new(move |r: &mut Struct| {
                let list = items
                    .iter()
                    .enumerate()
                    .map(|(i, res)| {
                        let mut s = Struct::new(i as u32);
                        s.set("InventoryRes", Value::resref(ResRef::from_str(res).unwrap()));
                        s.set("Repos_PosX", Value::Word(i as u16));
                        s.set("Repos_Posy", Value::Word(0));
                        s.set("Dropable", Value::Byte(0));
                        s
                    })
                    .collect();
                r.set("ItemList", Value::List(list));
            })
        };
        let mut variants: Vec<(String, E)> = vec![("base".into(), Rc::new(|_: &mut Struct| {}))];
        for mr in 0..=8u8 {
            variants.push((
                format!("mr{mr}"),
                Rc::new(move |r: &mut Struct| r.set("MovementRate", Value::Byte(mr))),
            ));
        }
        for mr in [0u8, 4, 7] {
            variants.push((
                format!("deer_mr{mr}"),
                Rc::new(move |r: &mut Struct| {
                    r.set("Appearance_Type", Value::Word(37));
                    r.set("MovementRate", Value::Byte(mr));
                }),
            ));
        }
        // Fireball (3), Cone of Cold (5), Light (0).
        for (label, list) in [
            ("spec_fireball", vec![58]),
            ("spec_cone", vec![25]),
            ("spec_light", vec![100]),
            ("spec_fireball_cone", vec![58, 25]),
        ] {
            variants.push((label.into(), specials(list)));
        }
        variants.push(("wizard".into(), class(10)));
        variants.push(("wizard_mem3".into(), with_list(10, "MemorizedList3", vec![58, 58])));
        variants.push(("wizard_known3".into(), with_list(10, "KnownList3", vec![58, 58])));
        variants.push(("wizard_known0".into(), with_list(10, "KnownList0", vec![100, 100])));
        variants.push(("sorcerer".into(), class(9)));
        variants.push(("sorcerer_known3".into(), with_list(9, "KnownList3", vec![58, 58])));
        // Boots 20000 gold, bracers 207360 (Moonglow's item value).
        variants.push(("equip_boots".into(), equip(vec![(0x4, "nw_it_mboots005")])));
        variants.push(("equip_bracers".into(), equip(vec![(0x8, "x2_it_mbracer004")])));
        variants.push(("carry_bracers".into(), carry(vec!["x2_it_mbracer004"])));
        variants.push((
            "feats_nocr".into(),
            Rc::new(|r: &mut Struct| {
                let list = [1089u16, 1090, 1091, 1092, 1093]
                    .iter()
                    .map(|&f| {
                        let mut s = Struct::new(1);
                        s.set("Feat", Value::Word(f));
                        s
                    })
                    .collect();
                r.set("FeatList", Value::List(list));
            }),
        ));
        for (label, edit) in variants {
            for hp in 1..=90i16 {
                add(&format!("terms_{label}"), hp.to_string(), Change::Edit(edit.clone(), hp));
            }
        }
    } else if fine {
        // Hit points in fine steps: level 1 (d4, d10), levels 5, 10, 20
        // (d10); levels 1 to 40 at the least hit points.
        for (class, label, l, top, step) in [
            (10, "d4", 1i16, 60i16, 1i16),
            (4, "d10", 1, 80, 1),
            (4, "d10", 5, 300, 2),
            (4, "d10", 10, 300, 2),
            (4, "d10", 20, 400, 3),
        ] {
            for hp in (1..=top).step_by(step as usize) {
                add(&format!("fine_{label}"), format!("{l} {hp}"), Change::ClassHp(class, l, hp));
            }
        }
        for l in 1..=40i16 {
            add("fine_d10", format!("{l} 1"), Change::ClassHp(4, l, 1));
        }
    } else if sweep {
        // Hit points against level and hit die: wizard d4, rogue d6, cleric
        // d8, fighter d10, barbarian d12.
        for (class, label) in [(10, "d4"), (8, "d6"), (2, "d8"), (4, "d10"), (0, "d12")] {
            for l in [1i16, 2, 3, 5, 8, 12, 20] {
                for hp in [
                    1i16, 2, 3, 4, 6, 8, 10, 12, 15, 20, 25, 30, 40, 50, 60, 80, 100, 130, 160,
                    200, 250, 300,
                ] {
                    add(
                        &format!("sweep_{label}"),
                        format!("{l} {hp}"),
                        Change::ClassHp(class, l, hp),
                    );
                }
            }
        }
    } else {
        for l in [1, 2, 3, 4, 6, 7, 8, 9, 10, 12, 15, 20, 25, 30, 35, 40] {
            add("level_hp25", l.to_string(), Change::Level(l, 25));
            add("level_hp10l", l.to_string(), Change::Level(l, 10 * l));
        }
        for hp in [1, 5, 10, 15, 20, 30, 35, 40, 45, 50, 60, 70, 80, 100, 125, 150, 200, 300, 500] {
            add("hp", hp.to_string(), Change::Hp(hp));
        }
        for ac in 1..=30 {
            add("nac", ac.to_string(), Change::NaturalAc(ac));
        }
        for n in 1..=20 {
            add("feats1", n.to_string(), Change::Feats(f1.iter().take(n).copied().collect()));
        }
        for n in [1, 2, 3, 4, 6, 8, 10] {
            add("feats05", n.to_string(), Change::Feats(f05.iter().take(n).copied().collect()));
            add("feats02", n.to_string(), Change::Feats(f02.iter().take(n).copied().collect()));
        }
        for n in 1..=10 {
            add("specials", n.to_string(), Change::Specials(n));
        }
        for a in ["Str", "Dex", "Con", "Int", "Wis", "Cha"] {
            for v in [3, 6, 8, 12, 14, 16, 18, 20, 24, 30, 40] {
                add(&format!("ability_{a}"), v.to_string(), Change::Ability(a, v));
            }
        }
        for c in 0..=38 {
            for l in [1, 5, 10] {
                add(&format!("class_{c}"), l.to_string(), Change::Class(c, l));
            }
        }
        for r in 0..=29u8 {
            add("race", r.to_string(), Change::Race(r));
        }
        for s in ["fortbonus", "refbonus", "willbonus"] {
            for v in [1, 2, 5, 10] {
                add(&format!("save_{s}"), v.to_string(), Change::Save(s, v));
            }
        }
        for r in [1, 5, 10, 20] {
            add("skills", r.to_string(), Change::Skills(r));
        }
        // Sizes: a tiny (rat), small (halfling), large (ogre), huge (giant).
        for a in [3u16, 37, 127, 30, 54, 88] {
            add("appearance", a.to_string(), Change::Appearance(a));
        }
        for (label, list) in [
            ("f5w5", vec![(4, 5), (10, 5)]),
            ("f1w1", vec![(4, 1), (10, 1)]),
            ("f10w10", vec![(4, 10), (10, 10)]),
            ("f2r2b2", vec![(4, 2), (8, 2), (0, 2)]),
        ] {
            add("classes", label.into(), Change::Classes(list));
        }
    }

    for (name, what, change) in &probes {
        println!("{name}\t{what}");
        let mut g = Gff::read(&base).unwrap();
        let r = &mut g.root;
        r.set("TemplateResRef", Value::resref(ResRef::from_str(name).unwrap()));
        r.set(
            "FirstName",
            Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, name.as_str())),
        );
        r.set("LastName", Value::LocString(LocString::default()));
        r.set("Tag", Value::String(name.to_uppercase().into_bytes()));
        r.set("ChallengeRating", Value::Float(42.0));
        r.set("CRAdjust", Value::Int(0));
        r.set("PaletteID", Value::Byte(45));
        for a in ["Str", "Dex", "Con", "Int", "Wis", "Cha"] {
            r.set(a, Value::Byte(10));
        }
        r.set("NaturalAC", Value::Byte(0));
        r.set("FeatList", Value::List(Vec::new()));
        r.set("SpecAbilityList", Value::List(Vec::new()));
        r.set("Equip_ItemList", Value::List(Vec::new()));
        r.set("ItemList", Value::List(Vec::new()));
        if let Some(skills) = r.list_mut("SkillList") {
            for s in skills {
                s.set("Rank", Value::Byte(0));
            }
        }
        let classes = |r: &mut Struct, list: &[(i32, i16)]| {
            let list: Vec<Struct> = list
                .iter()
                .map(|(c, l)| {
                    let mut s = Struct::new(2);
                    s.set("Class", Value::Int(*c));
                    s.set("ClassLevel", Value::Short(*l));
                    s
                })
                .collect();
            r.set("ClassList", Value::List(list));
        };
        let hp = |r: &mut Struct, hp: i16| {
            r.set("HitPoints", Value::Short(hp));
            r.set("CurrentHitPoints", Value::Short(hp));
            r.set("MaxHitPoints", Value::Short(hp));
        };
        classes(r, &[(4, 5)]);
        hp(r, 25);
        match change {
            Change::Base => {}
            Change::Level(l, h) => {
                classes(r, &[(4, *l)]);
                hp(r, *h);
            }
            Change::Hp(h) => hp(r, *h),
            Change::NaturalAc(ac) => r.set("NaturalAC", Value::Byte(*ac)),
            Change::Feats(fs) => {
                let list = fs
                    .iter()
                    .map(|f| {
                        let mut s = Struct::new(1);
                        s.set("Feat", Value::Word(*f));
                        s
                    })
                    .collect();
                r.set("FeatList", Value::List(list));
            }
            Change::Specials(n) => {
                let list = (0..*n)
                    .map(|i| {
                        let mut s = Struct::new(4);
                        s.set(
                            "Spell",
                            Value::Word([58, 25, 88, 100, 22, 31, 59, 130, 150, 11][i as usize]),
                        );
                        s.set("SpellCasterLevel", Value::Byte(5));
                        s.set("SpellFlags", Value::Byte(1));
                        s
                    })
                    .collect();
                r.set("SpecAbilityList", Value::List(list));
            }
            Change::Ability(a, v) => r.set(a, Value::Byte(*v)),
            Change::Class(c, l) => classes(r, &[(*c, *l)]),
            Change::Race(race) => r.set("Race", Value::Byte(*race)),
            Change::Save(s, v) => r.set(s, Value::Short(*v)),
            Change::Skills(rank) => {
                if let Some(skills) = r.list_mut("SkillList") {
                    for s in skills {
                        s.set("Rank", Value::Byte(*rank));
                    }
                }
            }
            Change::Appearance(a) => r.set("Appearance_Type", Value::Word(*a)),
            Change::Classes(list) => classes(r, list),
            Change::ClassHp(c, l, h) => {
                classes(r, &[(*c, *l)]);
                hp(r, *h);
            }
            Change::Edit(edit, h) => {
                hp(r, *h);
                edit(r);
            }
        }
        m.set_gff(ResKey::parse(name, ResType::UTC).unwrap(), &g).unwrap();
    }
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    m.save_as(&ModuleLocation::Archive(out.into())).unwrap();
}
