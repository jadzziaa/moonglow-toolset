//! The Creature Wizard against Aurora's: creatures made with the wizard's
//! defaults but for the racial type (male, the race's appearance, its
//! first portrait, Hostile, Tutorial category, Aurora's random names): a
//! Human Fighter 1 (`creature-wizard/human-fighter1.mod`), an Elf Wizard 3,
//! and the default class of every racial type the wizard's first page
//! shows (`creature-wizard/races.mod`). Every field (and the order Aurora
//! writes them in) must be Aurora's, but three flags Aurora writes as odd
//! values (read as set).

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct};
use mg_module::Module;
use mg_module::blueprints::{CreatureSpec, creature};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

use crate::aurora_common::{PLAIN, Shown, lines};

/// How the GFFs are compared, as lines.
const SHOWN: Shown<'static> = Shown { uninitialized: &UNINITIALIZED, ..PLAIN };

/// Aurora writes odd values in these (144, 95, 16; all read as set).
const UNINITIALIZED: [&str; 3] = ["Interruptable", "NoPermDeath", "Disarmable"];

/// The wizard's choices, read back from what it made.
fn spec_of(c: &Struct) -> CreatureSpec {
    let int = |l: &str| c.integer(l).unwrap_or(0);
    let text = |l: &str| {
        c.locstring(l)
            .and_then(|t| t.strings.first().map(|(_, b)| String::from_utf8_lossy(b).into_owned()))
            .unwrap_or_default()
    };
    CreatureSpec {
        resref: c.resref("TemplateResRef").unwrap(),
        first_name: text("FirstName"),
        last_name: text("LastName"),
        race: int("Race") as u32,
        gender: int("Gender") as u8,
        appearance: int("Appearance_Type") as u16,
        portrait: int("PortraitId") as u16,
        faction: int("FactionID") as u16,
        classes: c
            .list("ClassList")
            .unwrap()
            .iter()
            .map(|e| (e.integer("Class").unwrap() as u32, e.integer("ClassLevel").unwrap() as u32))
            .collect(),
        category: int("PaletteID") as u8,
    }
}

#[test]
fn creature_wizard_matches_aurora() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let item = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let mut n = 0;
    let mut failures = Vec::new();
    for capture in [
        aurora_capture!("creature-wizard/human-fighter1.mod"),
        aurora_capture!("creature-wizard/races.mod"),
    ] {
        let capture = Module::open(&capture).unwrap();
        for key in capture.keys_of(ResType::UTC) {
            let aurora = capture.gff(key).unwrap().unwrap();
            let ours = creature(&game, &spec_of(&aurora.root), &item);
            let (a, b) = (lines(&ours.root, SHOWN), lines(&aurora.root, SHOWN));
            let first = a.iter().zip(&b).position(|(x, y)| x != y);
            if let Some(i) = first.or((a.len() != b.len()).then(|| a.len().min(b.len()))) {
                let from = i.saturating_sub(3);
                failures.push(format!(
                    "{key}: differs at line {i}:\nMoonglow {:#?}\nAurora {:#?}",
                    &a[from..(i + 3).min(a.len())],
                    &b[from..(i + 3).min(b.len())]
                ));
            }
            n += 1;
        }
    }
    assert!(failures.is_empty(), "{} of {n}:\n{}", failures.len(), failures.join("\n"));
    assert!(n >= 10, "{n} creatures");
}
