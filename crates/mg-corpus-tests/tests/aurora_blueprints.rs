//! New blueprints compared with Aurora's: blueprints made by Aurora's
//! blueprint wizards (the oracle capture `blueprint-wizards.mod`: a
//! waypoint, sounds of three styles, a generic, an area transition and a
//! trap trigger, an encounter, a store, a placeable, a door, and a weapon,
//! armor, an amulet and a cloak). Moonglow's must have the same fields,
//! types, order and values, and the same resrefs.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct};
use mg_module::Module;
use mg_module::blueprints::{self, SoundStyle};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// Field labels, types and values, one line each, for a readable diff.
fn lines(s: &Struct, indent: &str, out: &mut Vec<String>) {
    for f in &s.fields {
        match &f.value {
            mg_gff::Value::List(items) => {
                out.push(format!("{indent}{} list[{}]", f.label.to_string_lossy(), items.len()));
                for (i, item) in items.iter().enumerate() {
                    out.push(format!("{indent}  [{i}] struct {}", item.id));
                    lines(item, &format!("{indent}    "), out);
                }
            }
            v => out.push(format!("{indent}{} {:?}", f.label.to_string_lossy(), v)),
        }
    }
}

fn same(name: &str, ours: &Gff, aurora: &Gff) -> Option<String> {
    if ours == aurora {
        return None;
    }
    let (mut a, mut b) = (Vec::new(), Vec::new());
    lines(&ours.root, "", &mut a);
    lines(&aurora.root, "", &mut b);
    let diff: Vec<String> = a
        .iter()
        .zip(&b)
        .filter(|(x, y)| x != y)
        .map(|(x, y)| format!("  ours   {x}\n  aurora {y}"))
        .take(8)
        .collect();
    Some(format!(
        "{name}: {} vs {} lines; first differences:\n{}",
        a.len(),
        b.len(),
        diff.join("\n")
    ))
}

#[test]
fn new_blueprints_match_aurora() {
    let root = corpus!();
    let capture = Module::open(&aurora_capture!("blueprint-wizards.mod")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let get = |name: &str, t: ResType| -> Gff {
        capture
            .gff(&ResKey::new(r(name), t))
            .unwrap_or_else(|| panic!("{name}.{t:?} missing"))
            .unwrap()
    };
    let free = |_: &ResRef| false;
    let res = |name: &str| blueprints::resref(name, free);
    let mut failures = Vec::new();
    let mut check = |name: &str, t: ResType, ours: Gff| {
        if let Some(e) = same(name, &ours, &get(name, t)) {
            failures.push(e);
        }
    };

    check(
        "wizwaypoint",
        ResType::UTW,
        blueprints::waypoint(res("WizWaypoint"), "WizWaypoint", 1, 5),
    );
    for (name, display, category, style, sound) in [
        (
            "civilization001",
            "Civilization 001",
            13,
            SoundStyle::LoopingPositional,
            "al_cv_firecamp1",
        ),
        ("weather001", "Weather 001", 8, SoundStyle::SingleShotRandom, "as_wt_thundercl1"),
        ("nature001", "Nature 001", 7, SoundStyle::LoopingAreaWide, "al_na_stream3"),
    ] {
        assert_eq!(res(display), r(name));
        check(
            name,
            ResType::UTS,
            blueprints::sound(r(name), display, category, style, &[r(sound)]),
        );
    }
    for (name, display, category) in [
        ("generictrigge001", "Generic Trigger 001", 6),
        ("areatransitio001", "Area Transition 001", 5),
        ("blueprint001", "1. Average 001", 12),
    ] {
        assert_eq!(res(display), r(name));
        check(name, ResType::UTT, blueprints::trigger(&game, r(name), display, category));
    }
    let wolves = get("normal001", ResType::UTE).root.list("CreatureList").unwrap().to_vec();
    check(
        "normal001",
        ResType::UTE,
        blueprints::encounter(res("Normal 001"), "Normal 001", 7, wolves),
    );
    check(
        "merchants001",
        ResType::UTM,
        blueprints::store(res("Merchants 001"), "Merchants 001", 5),
    );
    let chest = "Containers & Switches 001";
    check("blueprint001", ResType::UTP, blueprints::placeable(res(chest), chest, 6));
    let door = "Tileset Specific 001";
    check("tilesetspecif001", ResType::UTD, blueprints::door(res(door), door, 5));
    for (name, display, base, category) in [
        ("wizardsword", "Wizard Sword", 3, 33),
        ("wizardarmor", "Wizard Armor", 16, 7),
        ("wizardamulet", "Wizard Amulet", 19, 23),
        ("wizardcloak", "Wizard Cloak", 80, 5),
    ] {
        assert_eq!(res(display), r(name));
        check(name, ResType::UTI, blueprints::item(&game, r(name), display, base, category));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    // The comparison sees a difference: another category.
    let other = blueprints::store(res("Merchants 001"), "Merchants 001", 6);
    assert!(same("merchants001", &other, &get("merchants001", ResType::UTM)).is_some());
}
