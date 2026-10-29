//! What a creature may not wear, settled in the engine: an item equipped
//! in a creature's file that the game takes off (into the backpack) when
//! it loads the creature, for a placed creature (whole items in the GIT)
//! and for one made from its blueprint by script (`EquippedRes` entries),
//! and that `ActionEquipItem` does not put on. A commoner (`nw_commale`:
//! class Commoner, level 1, human, true neutral, no proficiency feats)
//! wears, each in its own copy, a cloak with use limitations, or armor, a
//! shield or a weapon. `mg_rules::items::wear_problem` is held to it:
//! Moonglow's preview of a creature leaves such an item off, as the game
//! does, and the inventory says why.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// An item property (`PropertyName`, `Subtype`, `CostTable`, `CostValue`).
fn property(name: u16, subtype: u16, table: u8, value: u16) -> Struct {
    let mut p = Struct::new(0);
    p.set("PropertyName", Value::Word(name));
    p.set("Subtype", Value::Word(subtype));
    p.set("CostTable", Value::Byte(table));
    p.set("CostValue", Value::Word(value));
    p.set("Param1", Value::Byte(255));
    p.set("Param1Value", Value::Byte(0));
    p.set("ChanceAppear", Value::Byte(100));
    p
}

/// A copy of the commoner and what it wears.
struct Case {
    name: &'static str,
    /// The game's item it is made of.
    item: &'static str,
    /// The properties the item gets (`None`: its own).
    properties: Option<Vec<Struct>>,
    /// The slot's bit.
    slot: u32,
    /// The commoner's level, its `GoodEvil` and `LawfulChaotic`, and the
    /// feats it gets beyond its own.
    level: u8,
    alignment: (u8, u8),
    feats: &'static [u16],
    /// Whether the game lets it wear the item.
    worn: bool,
}

fn cases() -> Vec<Case> {
    const CLOAK: u32 = 0x40;
    let p = property;
    let cloak = |name, properties: Vec<Struct>, worn| Case {
        name,
        item: "nw_maarcl055",
        properties: Some(properties),
        slot: CLOAK,
        level: 1,
        alignment: (50, 50),
        feats: &[],
        worn,
    };
    let aligned = |name, properties: Vec<Struct>, alignment, worn| Case {
        alignment,
        ..cloak(name, properties, worn)
    };
    let held = |name, item, slot, feats, worn| Case {
        item,
        slot,
        feats,
        properties: Some(Vec::new()),
        ..cloak(name, Vec::new(), worn)
    };
    vec![
        cloak("plain", vec![], true),
        // The cloak seen taken off in the client: limited to bards,
        // sorcerers and wizards, with other properties.
        Case { item: "nw_aarcl013", properties: None, ..cloak("seen", vec![], false) },
        // Its worth is no matter, whatever the level.
        cloak("ac1", vec![p(1, 0, 2, 1)], true),
        cloak("ac5", vec![p(1, 0, 2, 5)], true),
        Case { level: 15, ..cloak("ac5lvl15", vec![p(1, 0, 2, 5)], true) },
        // Use Limitation: Class, any of those listed.
        cloak("wizard", vec![p(63, 10, 0, 0)], false),
        cloak("commoner", vec![p(63, 20, 0, 0)], true),
        cloak("wiz_comm", vec![p(63, 10, 0, 0), p(63, 20, 0, 0)], true),
        // Racial Type, any of those listed.
        cloak("elf", vec![p(64, 1, 0, 0)], false),
        cloak("human", vec![p(64, 6, 0, 0)], true),
        cloak("elf_human", vec![p(64, 1, 0, 0), p(64, 6, 0, 0)], true),
        // Each kind of limit held to apart.
        cloak("comm_elf", vec![p(63, 20, 0, 0), p(64, 1, 0, 0)], false),
        cloak("wiz_human", vec![p(63, 10, 0, 0), p(64, 6, 0, 0)], false),
        cloak("comm_human", vec![p(63, 20, 0, 0), p(64, 6, 0, 0)], true),
        // Alignment Group (1 Neutral, 2 Lawful, 3 Chaotic, 4 Good, 5 Evil)
        // and Specific Alignment (0 LG … 4 TN … 8 CE).
        cloak("lawful", vec![p(62, 2, 0, 0)], false),
        cloak("neutral", vec![p(62, 1, 0, 0)], true),
        aligned("neutral_lg", vec![p(62, 1, 0, 0)], (100, 100), false),
        aligned("neutral_ln", vec![p(62, 1, 0, 0)], (50, 100), true),
        aligned("neutral_ng", vec![p(62, 1, 0, 0)], (100, 50), true),
        aligned("lawful_ln", vec![p(62, 2, 0, 0)], (50, 100), true),
        aligned("lawful_70", vec![p(62, 2, 0, 0)], (50, 70), true),
        aligned("lawful_69", vec![p(62, 2, 0, 0)], (50, 69), false),
        aligned("good_ng", vec![p(62, 4, 0, 0)], (100, 50), true),
        aligned("evil_30", vec![p(62, 5, 0, 0)], (30, 50), true),
        aligned("evil_31", vec![p(62, 5, 0, 0)], (31, 50), false),
        aligned("chaotic_ce", vec![p(62, 3, 0, 0)], (0, 0), true),
        cloak("lg", vec![p(65, 0, 0, 0)], false),
        cloak("tn", vec![p(65, 4, 0, 0)], true),
        aligned("lg_lg", vec![p(65, 0, 0, 0)], (100, 100), true),
        aligned("ce_ce", vec![p(65, 8, 0, 0)], (0, 0), true),
        aligned("ng_lg", vec![p(65, 3, 0, 0)], (100, 100), false),
        // A group and a specific alignment are one kind: either fits.
        cloak("neutral_or_lg", vec![p(62, 1, 0, 0), p(65, 0, 0, 0)], true),
        cloak("lawful_or_tn", vec![p(62, 2, 0, 0), p(65, 4, 0, 0)], true),
        cloak("lawful_or_lg", vec![p(62, 2, 0, 0), p(65, 0, 0, 0)], false),
        // Feats (the creature's own list; the commoner has no
        // proficiency): heavy armor, a tower shield, a martial and a
        // simple weapon.
        held("plate", "nw_aarcl007", 0x2, &[], false),
        held("plate_feat", "nw_aarcl007", 0x2, &[2], true),
        held("tower", "nw_ashto001", 0x20, &[], false),
        held("tower_feat", "nw_ashto001", 0x20, &[32], true),
        held("longsword", "nw_wswls001", 0x10, &[], false),
        held("club", "nw_wblcl001", 0x10, &[], false),
        held("club_feat", "nw_wblcl001", 0x10, &[46], true),
    ]
}

/// Where case `i`'s copies stand: placed, carrying it and made by script.
fn at(i: usize, row: f32) -> [f32; 3] {
    [4.0 + (i % 10) as f32 * 3.0, row + (i / 10) as f32 * 3.0, 0.0]
}

/// The probe: what each commoner wears and carries, at load and later;
/// the copies carrying their item are given it by `ActionEquipItem`.
fn probe(cases: &[Case]) -> String {
    let mut s = String::from(
        r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

void Worn(string sWhen, object o)
{
    string s = "";
    int i;
    for (i = 0; i < NUM_INVENTORY_SLOTS; i++)
    {
        object e = GetItemInSlot(i, o);
        if (GetIsObjectValid(e)) s += " " + IntToString(i) + "=" + GetTag(e);
    }
    string p = "";
    object b = GetFirstItemInInventory(o);
    while (GetIsObjectValid(b)) { p += " " + GetTag(b); b = GetNextItemInInventory(o); }
    Log("MG_WORN " + sWhen + "|" + GetTag(o) + "|" + s + "|" + p);
}

void Report(string sWhen)
{
    object oArea = GetArea(GetObjectByTag("PLACED_plain"));
    object o = GetFirstObjectInArea(oArea);
    while (GetIsObjectValid(o))
    {
        if (GetObjectType(o) == OBJECT_TYPE_CREATURE) Worn(sWhen, o);
        o = GetNextObjectInArea(oArea);
    }
}

void Equip(string sName, int nSlot)
{
    object o = GetObjectByTag("ACT_" + sName);
    AssignCommand(o, ActionEquipItem(GetFirstItemInInventory(o), nSlot));
}

void main()
{
    object oArea = GetArea(GetObjectByTag("PLACED_plain"));
"#,
    );
    for (i, c) in cases.iter().enumerate() {
        let [x, y, _] = at(i, 44.0);
        let name = c.name;
        s += &format!(
            "    CreateObject(OBJECT_TYPE_CREATURE, \"mg_c_{i}\", \
             Location(oArea, Vector({x:.1}, {y:.1}, 0.0), 0.0), FALSE, \"MADE_{name}\");\n"
        );
        let slot = c.slot.trailing_zeros();
        s += &format!("    DelayCommand(1.0, Equip(\"{name}\", {slot}));\n");
    }
    s += "    Report(\"load\");\n    DelayCommand(6.0, Report(\"later\"));\n    \
          DelayCommand(6.5, Log(\"MG_DONE\"));\n}\n";
    s
}

#[test]
fn what_a_creature_may_not_wear_comes_off() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_equipped_items");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(&game, "Equipped Items", &mut rng).unwrap();
    let spec = AreaSpec { name: "Worn".into(), tileset: r("tic01"), width: 8, height: 8 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let read = |name: ResRef, t: ResType| -> Option<Struct> {
        Some(Gff::read(&game.resman.get(&ResKey::new(name, t)).ok()?).ok()?.root)
    };
    let cases = cases();
    // (Each its own tags: a script finds the copies by them.)
    let names: std::collections::BTreeSet<_> = cases.iter().map(|c| c.name).collect();
    assert_eq!(names.len(), cases.len(), "a case's name twice");
    // The items, each a blueprint of the module's own, tagged by its case.
    let mut items = std::collections::HashMap::new();
    for (i, c) in cases.iter().enumerate() {
        let mut uti = read(r(c.item), ResType::UTI).unwrap_or_else(|| panic!("{}", c.item));
        if let Some(p) = &c.properties {
            uti.set("PropertiesList", Value::List(p.clone()));
        }
        let resref = r(&format!("mg_i_{i}"));
        uti.set("TemplateResRef", Value::resref(resref));
        uti.set("Tag", Value::String(format!("I_{}", c.name).into_bytes()));
        uti.set("Identified", Value::Byte(1));
        let mut g = Gff::new(*b"UTI ");
        g.root = uti.clone();
        m.set_gff(ResKey::new(resref, ResType::UTI), &g).unwrap();
        items.insert(resref, uti);
    }
    let held = |name: ResRef| items.get(&name).cloned().or_else(|| read(name, ResType::UTI));
    let placing = Placing { game: &game, item: &held };
    let commoner = read(r("nw_commale"), ResType::UTC).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTC).unwrap();
    let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
    // Each case's creature, as the rule reads it.
    let mut creatures = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let item = r(&format!("mg_i_{i}"));
        let mut utc = commoner.clone();
        for f in &mut utc.fields {
            if f.label.as_bytes().starts_with(b"Script") {
                f.value = Value::resref(ResRef::EMPTY);
            }
        }
        if let Some(Value::List(classes)) = utc.get_mut("ClassList") {
            classes[0].set("ClassLevel", Value::Short(i16::from(c.level)));
        }
        utc.set("GoodEvil", Value::Byte(c.alignment.0));
        utc.set("LawfulChaotic", Value::Byte(c.alignment.1));
        if let Some(Value::List(feats)) = utc.get_mut("FeatList") {
            for &feat in c.feats {
                let mut f = Struct::new(1);
                f.set("Feat", Value::Word(feat));
                feats.push(f);
            }
        }
        let mut equipped = Struct::new(c.slot);
        equipped.set("EquippedRes", Value::resref(item));
        utc.set("Equip_ItemList", Value::List(vec![equipped]));
        utc.set("ItemList", Value::List(Vec::new()));
        let resref = r(&format!("mg_c_{i}"));
        utc.set("TemplateResRef", Value::resref(resref));
        let mut g = Gff::new(*b"UTC ");
        g.root = utc.clone();
        m.set_gff(ResKey::new(resref, ResType::UTC), &g).unwrap();
        creatures.push(utc.clone());

        let place = |row| Placement { position: at(i, row), rotation: 0.0 };
        let mut placed = utc.clone();
        placed.set("Tag", Value::String(format!("PLACED_{}", c.name).into_bytes()));
        objects.push(instance(&placing, ResType::UTC, &placed, place(4.0), &[]).unwrap());
        // The same, carrying it in its backpack, to be equipped by script.
        let mut act = utc;
        act.set("Tag", Value::String(format!("ACT_{}", c.name).into_bytes()));
        act.set("Equip_ItemList", Value::List(Vec::new()));
        let mut carried = Struct::new(0);
        carried.set("InventoryRes", Value::resref(item));
        carried.set("Repos_PosX", Value::Word(0));
        carried.set("Repos_Posy", Value::Word(0));
        act.set("ItemList", Value::List(vec![carried]));
        objects.push(instance(&placing, ResType::UTC, &act, place(24.0), &[]).unwrap());
    }
    git.root.set(list, Value::List(objects));
    m.set_gff(git_key, &git).unwrap();

    let probe = compile(&dir, "mg_probe", &probe(&cases));
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_worn.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_worn", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let got = run.values("MG_WORN");

    // Each copy wears its item, or carries it, as the case says: placed
    // and made at load (the backpack taking what came off), and the one
    // given it by `ActionEquipItem` later.
    let mut wrong = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let slot = c.slot.trailing_zeros();
        let (on, off) = (format!(" {slot}=I_{}|", c.name), format!("|| I_{}", c.name));
        let want = if c.worn { &on } else { &off };
        for (when, who) in [("load", "PLACED"), ("load", "MADE"), ("later", "ACT")] {
            let line = format!("{when}|{who}_{}|", c.name);
            let seen = got.iter().find(|l| l.starts_with(&line));
            if !seen.is_some_and(|l| l.ends_with(want.as_str()) || l.contains(want.as_str())) {
                wrong.push(format!("{line}: {seen:?}, wanted {want:?}"));
            }
        }
        // And Moonglow's rule says the same.
        let uti = &items[&r(&format!("mg_i_{i}"))];
        let problem = game.wear_problem(&creatures[i], uti);
        if problem.is_none() != c.worn {
            wrong.push(format!("{}: wear_problem says {problem:?}", c.name));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
