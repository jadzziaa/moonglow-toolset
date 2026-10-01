//! The Creature Wizard against Aurora's: a Human Fighter 1 made with the
//! wizard's defaults (male, the Human appearance, portrait `hu_m_01_`
//! (row 93), Hostile, Tutorial category; Aurora's random name "Hent
//! Fynolds"), captured in `creature-wizard/human-fighter1.mod`. Every field
//! (and the order Aurora writes them in) must be Aurora's, but the four it
//! leaves uninitialized.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_module::blueprints::{CreatureSpec, creature};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

/// Aurora writes stray values in these.
const UNINITIALIZED: [&str; 4] = ["Interruptable", "NoPermDeath", "Disarmable", "SoundSetFile"];

fn lines(s: &Struct, indent: &str, out: &mut Vec<String>) {
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        if indent.is_empty() && UNINITIALIZED.contains(&label.as_str()) {
            out.push(format!("{label} (uninitialized)"));
            continue;
        }
        match &f.value {
            Value::List(items) => {
                out.push(format!("{indent}{label} list[{}]", items.len()));
                for (i, item) in items.iter().enumerate() {
                    out.push(format!("{indent}  [{i}] struct {}", item.id));
                    lines(item, &format!("{indent}    "), out);
                }
            }
            v => out.push(format!("{indent}{label} {v:?}")),
        }
    }
}

#[test]
fn creature_wizard_matches_aurora() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let capture = Module::open(&aurora_capture!("creature-wizard/human-fighter1.mod")).unwrap();
    let aurora = capture.gff(&ResKey::parse("hent", ResType::UTC).unwrap()).unwrap().unwrap();
    let item = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let spec = CreatureSpec {
        resref: ResRef::from_str("hent").unwrap(),
        first_name: "Hent".into(),
        last_name: "Fynolds".into(),
        race: 6,
        gender: 0,
        appearance: 6,
        portrait: 93,
        faction: 1,
        classes: vec![(4, 1)],
        category: 46,
    };
    let ours = creature(&game, &spec, &item);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    lines(&ours.root, "", &mut a);
    lines(&aurora.root, "", &mut b);
    let first = a.iter().zip(&b).position(|(x, y)| x != y);
    if let Some(i) = first.or((a.len() != b.len()).then(|| a.len().min(b.len()))) {
        let from = i.saturating_sub(4);
        panic!(
            "differs at line {i}:\nMoonglow {:#?}\nAurora {:#?}",
            &a[from..(i + 4).min(a.len())],
            &b[from..(i + 4).min(b.len())]
        );
    }
}
