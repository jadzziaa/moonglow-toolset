//! Creature spell warnings against Aurora's: thirty creatures given
//! spells (`examples/spell_warning_probe_module.rs`, five sets of six,
//! captured as `spell-warnings/set1.mod` to `set5.mod`), each opened in
//! Aurora's Creature Properties and closed with OK; the warnings it showed,
//! in order, were these.

use std::path::{Path, PathBuf};

use mg_core::ResType;
use mg_gff::{Struct, Value};
use mg_module::Module;
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

const TOO_HIGH: &str = "This creature has spells assigned to it that are too high for its \
                        current Wizard level. Do you wish to proceed?";
const WIZARD_INT: &str = "This creature's Wizard class has been assigned spells that it cannot \
                          use because its Intelligence is too low. Do you wish to proceed?";

fn too_many(class: &str, max: u32, level: u32, count: u32) -> String {
    format!(
        "This creature's {class} class can have a maximum of {max} level {level} spells, but \
         has been assigned {count} spells. Do you wish to proceed?"
    )
}

/// The creatures of a set's area, by tag.
fn creatures(module: &Path) -> Vec<(String, Struct)> {
    let m = Module::open(module).unwrap();
    let git = m.gff(&ResKey::parse("field", ResType::GIT).unwrap()).unwrap().unwrap();
    let mut out: Vec<(String, Struct)> = git
        .root
        .list("Creature List")
        .unwrap()
        .iter()
        .map(|c| (String::from_utf8_lossy(c.string("Tag").unwrap()).into_owned(), c.clone()))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn spell_warnings_match_aurora() {
    let root = corpus!();
    let mut sets: Vec<PathBuf> = Vec::new();
    for n in 1..=5 {
        sets.push(aurora_capture!(&format!("spell-warnings/set{n}.mod")));
    }
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let sorcerer = |max, level, count| too_many("Sorcerer", max, level, count);
    let wizard = |max, level, count| too_many("Wizard", max, level, count);
    let cleric = |max, level, count| too_many("Cleric", max, level, count);
    let seen: [[Vec<String>; 6]; 5] = [
        [
            vec![TOO_HIGH.into()],
            vec![WIZARD_INT.into()],
            vec![],
            vec![],
            vec![WIZARD_INT.into()],
            vec![],
        ],
        [
            vec![WIZARD_INT.into()],
            vec![wizard(2, 1, 3)],
            vec![TOO_HIGH.into()],
            vec![sorcerer(5, 0, 6)],
            vec![WIZARD_INT.into()],
            vec![],
        ],
        [
            vec![sorcerer(5, 0, 6)],
            vec![wizard(2, 2, 3)],
            vec![wizard(1, 1, 2)],
            vec![TOO_HIGH.into()],
            vec![TOO_HIGH.into(), sorcerer(5, 0, 6)],
            vec![],
        ],
        [
            vec![],
            vec![
                "This creature's Sorcerer class has been assigned spells that it cannot use \
                  because its Charisma is too low. Do you wish to proceed?"
                    .into(),
            ],
            vec![
                "This creature has spells assigned to it that are too high for its current \
                  Bard level. Do you wish to proceed?"
                    .into(),
            ],
            vec![],
            vec![cleric(4, 0, 5)],
            vec![cleric(3, 1, 4)],
        ],
        [
            vec![sorcerer(2, 1, 3)],
            vec![sorcerer(2, 1, 3)],
            vec![],
            vec![sorcerer(5, 0, 6)],
            vec![wizard(5, 0, 6)],
            vec![sorcerer(2, 1, 4)],
        ],
    ];
    let warned = |c: &Struct| -> Vec<String> {
        game.spell_warnings(c).iter().map(|w| game.spell_warning_text(w)).collect()
    };
    let mut wrong = Vec::new();
    for ((set, path), expected) in (1..).zip(&sets).zip(seen) {
        let list = creatures(path);
        assert_eq!(list.len(), 6);
        for ((tag, c), want) in list.iter().zip(expected) {
            let got = warned(c);
            if got != want {
                wrong.push(format!("set {set} {tag}: {got:?}, Aurora {want:?}"));
            }
        }
    }
    // And the first set's Wizard 1 with four cantrips, a fifth prepared
    // on the Spells page.
    let (_, mut c) = creatures(&sets[0]).swap_remove(2);
    let mut class = c.list("ClassList").unwrap()[0].clone();
    let mut list = class.list("MemorizedList0").unwrap().to_vec();
    list.push(list[0].clone());
    class.set("MemorizedList0", Value::List(list));
    c.set("ClassList", Value::List(vec![class]));
    if warned(&c) != [wizard(4, 0, 5)] {
        wrong.push(format!("five cantrips: {:?}", warned(&c)));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
