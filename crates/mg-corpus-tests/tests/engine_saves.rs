//! A creature's saving throws as the game has them, against the ones
//! Moonglow's Statistics page shows: single classes, several classes, and
//! past level 20 (Aurora computes an epic multiclass creature's wrongly:
//! nwn-issues 816, and so did Moonglow until this measured it). The
//! creatures are the Creature Wizard's, whose epic ones have the Great
//! ability feats, which the game counts into the scores.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_module::blueprints::{CreatureSpec, creature};
use mg_resman::{GameInstall, ResKey};
use mg_rules::{CreatureSheet, GameData};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::probe_module;

/// (fighter 4, wizard 10, rogue 8, cleric 2 of classes.2da.)
const CASES: [(&str, &[(u32, u32)]); 13] = [
    ("f10", &[(4, 10)]),
    ("f20", &[(4, 20)]),
    ("f30", &[(4, 30)]),
    ("w25", &[(10, 25)]),
    ("f10w10", &[(4, 10), (10, 10)]),
    ("f20w20", &[(4, 20), (10, 20)]),
    ("f10w10r10", &[(4, 10), (10, 10), (8, 10)]),
    ("w25c15", &[(10, 25), (2, 15)]),
    ("c15w25", &[(2, 15), (10, 25)]),
    ("w20c20", &[(10, 20), (2, 20)]),
    ("w10c30", &[(10, 10), (2, 30)]),
    ("r5f20", &[(8, 5), (4, 20)]),
    ("f5r5w15", &[(4, 5), (8, 5), (10, 15)]),
];

#[test]
fn saving_throws_match_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_saves");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut extra = Vec::new();
    let mut ours = Vec::new();
    let mut probes = String::new();
    for (name, classes) in CASES {
        let resref = ResRef::from_str(&format!("mg_{name}")).unwrap();
        let spec = CreatureSpec {
            resref,
            first_name: name.to_string(),
            last_name: String::new(),
            race: 6,
            gender: 0,
            appearance: 6,
            portrait: 1,
            faction: 1,
            classes: classes.to_vec(),
            category: 0,
        };
        let utc = creature(&game, &spec, &|_| None);
        let stats = game.creature_stats(&CreatureSheet::from_gff(&utc.root));
        ours.push((name, stats.saves));
        extra.push((ResKey::new(resref, ResType::UTC), utc.to_bytes().unwrap()));
        probes += &format!(
            "    o = CreateObject(OBJECT_TYPE_CREATURE, \"mg_{name}\", l);\n    \
             WriteTimestampedLogEntry(\"MG_SAVE {name}=\" + IntToString(GetIsObjectValid(o)) + \
             \",\" + IntToString(GetFortitudeSavingThrow(o)) + \",\" + \
             IntToString(GetReflexSavingThrow(o)) + \",\" + IntToString(GetWillSavingThrow(o)));\n"
        );
    }
    let script = format!(
        "void main()\n{{\n    location l = GetStartingLocation();\n    object o;\n{probes}    \
         WriteTimestampedLogEntry(\"MG_DONE\");\n}}\n"
    );
    probe_module(&root, &dir, "mg_saves", &script, &[], &extra);
    let run = run_server(&root, &dir, "mg_saves", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "{}", &run.log[run.log.len().saturating_sub(2000)..]);
    let engine: std::collections::HashMap<String, Vec<i32>> = run
        .values("MG_SAVE")
        .into_iter()
        .filter_map(|v| {
            let (name, rest) = v.split_once('=')?;
            Some((name.to_string(), rest.split(',').filter_map(|n| n.parse().ok()).collect()))
        })
        .collect();
    let mut wrong = Vec::new();
    for (name, saves) in ours {
        let theirs = &engine[name];
        assert_eq!(theirs[0], 1, "{name} was made");
        if theirs[1..] != saves {
            wrong.push(format!("{name}: ours {saves:?}, the engine's {:?}", &theirs[1..]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The Great Constitution feats' hit points: the game adds them to the
/// maximum itself (as it adds Epic Toughness's), but works out the hit
/// points a creature starts with from `CurrentHitPoints` and the
/// Constitution without them. A fighter 30 of Constitution 14 with none
/// of the feats, and with four (a modifier 2 higher: 60 hit points), its
/// `CurrentHitPoints` the base as Aurora writes it, and as Moonglow does
/// (the base and the feats' 60): Aurora's starts 60 short.
#[test]
fn great_constitution_s_hit_points_are_the_engine_s_to_add() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_great_constitution");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut extra = Vec::new();
    let mut ours = Vec::new();
    let mut probes = String::new();
    for (name, great) in [("mg_plain", 0u16), ("mg_aurora", 4), ("mg_great", 4)] {
        let resref = ResRef::from_str(name).unwrap();
        let spec = CreatureSpec {
            resref,
            first_name: name.to_string(),
            last_name: String::new(),
            race: 6,
            gender: 0,
            appearance: 6,
            portrait: 1,
            faction: 1,
            classes: vec![(4, 30)],
            category: 0,
        };
        let mut utc = creature(&game, &spec, &|_| None);
        utc.root.set("Con", Value::Byte(14));
        let list = utc.root.list_mut("FeatList").unwrap();
        // (The wizard's own Great feats out, these in.)
        list.retain(|f| !f.integer("Feat").is_some_and(|f| (764..824).contains(&f)));
        for feat in 774..774 + great {
            let mut f = Struct::new(1);
            f.set("Feat", Value::Word(feat));
            list.push(f);
        }
        let stats = game.creature_stats(&CreatureSheet::from_gff(&utc.root));
        let base = utc.root.integer("HitPoints").unwrap() as i32;
        let current = if name == "mg_aurora" { base } else { stats.current_hit_points };
        utc.root.set("MaxHitPoints", Value::Short(stats.max_hit_points as i16));
        utc.root.set("CurrentHitPoints", Value::Short(current as i16));
        ours.push((name, stats));
        extra.push((ResKey::new(resref, ResType::UTC), utc.to_bytes().unwrap()));
        probes += &format!(
            "    o = CreateObject(OBJECT_TYPE_CREATURE, \"{name}\", l);\n    \
             WriteTimestampedLogEntry(\"MG_HP {name}=\" + IntToString(GetMaxHitPoints(o)) + \
             \",\" + IntToString(GetCurrentHitPoints(o)));\n"
        );
    }
    let script = format!(
        "void main()\n{{\n    location l = GetStartingLocation();\n    object o;\n{probes}    \
         WriteTimestampedLogEntry(\"MG_DONE\");\n}}\n"
    );
    probe_module(&root, &dir, "mg_greatcon", &script, &[], &extra);
    let run = run_server(&root, &dir, "mg_greatcon", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "{}", &run.log[run.log.len().saturating_sub(2000)..]);
    let engine: std::collections::HashMap<String, (i32, i32)> = run
        .values("MG_HP")
        .into_iter()
        .filter_map(|v| {
            let (name, rest) = v.split_once('=')?;
            let (max, current) = rest.split_once(',')?;
            Some((name.to_string(), (max.parse().ok()?, current.parse().ok()?)))
        })
        .collect();
    let [(_, plain), (_, aurora), (_, great)] = &ours[..] else { unreachable!() };
    assert_eq!((plain.great_hit_points, great.great_hit_points), (0, 60));
    // The same stored maximum, Aurora's; the engine's own is 60 more.
    assert_eq!(plain.max_hit_points, great.max_hit_points);
    let (plain_max, plain_current) = engine["mg_plain"];
    assert_eq!(plain_current, plain_max, "{engine:?}");
    assert_eq!(engine["mg_aurora"], (plain_max + 60, plain_max), "Aurora's starts short");
    assert_eq!(engine["mg_great"], (plain_max + 60, plain_max + 60), "Moonglow's starts whole");
    assert_eq!(aurora.current_hit_points, great.current_hit_points);
}
