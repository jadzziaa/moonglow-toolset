//! The list pages of several blueprints edited together
//! (`docs/lists-together-proposal.md`): an entry some have is marked ≠
//! with how many, adding gives it to those that lack it, removing takes it
//! from those that have it, a value is set on each, and each change is one
//! step that Undo takes back.

use crate::common::*;
use mg_gff::{Struct, Value};

/// `n` copies of the game's blueprint `name` in the sample module
/// (`mg_a`, `mg_b`...), each made its own by `tweak` (given its place),
/// opened together on page `page`.
pub(crate) fn together(
    name: &str,
    t: ResType,
    n: usize,
    page: &'static str,
    tweak: impl Fn(usize, &mut Struct, &mg_rules::GameData),
) -> Option<(Harness<'static, Moonglow>, Vec<ResKey>)> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    // (A folder of each call's own: two tests of one blueprint's page
    // would empty each other's.)
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let page_name = page.replace(' ', "-");
    let dir = mg_testkit::scratch_dir(&format!("ui-together-{name}-{page_name}-{call}"));
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    close_tab(&mut app, &Tab::ModuleProperties);
    app.open_palette = false;
    let game = app.game.clone().unwrap();
    let base = Gff::read(&game.resman.get_named(name, t).unwrap()).unwrap();
    let keys: Vec<ResKey> = (0..n)
        .map(|i| ResKey::parse(&format!("mg_{}", (b'a' + i as u8) as char), t).unwrap())
        .collect();
    let edits = keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let mut g = base.clone();
            g.root.set("TemplateResRef", Value::resref(k.resref));
            tweak(i, &mut g.root, &game);
            mg_edit::Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(mg_edit::Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprints(keys.clone())));
    let mut h = harness(app, 1100.0, 860.0);
    h.run();
    h.state_mut().blueprint_pages.insert((keys[0], mg_edit::GffPath::root()), page);
    h.run();
    Some((h, keys))
}

/// What `read` makes of each blueprint.
pub(crate) fn each<T>(
    h: &mut Harness<'_, Moonglow>,
    keys: &[ResKey],
    read: impl Fn(&Struct) -> T,
) -> Vec<T> {
    keys.iter().map(|k| read(&field(h, k))).collect()
}

/// The row of `label` in a 2DA of the game's.
fn row_of(h: &Harness<'_, Moonglow>, table: &str, column: &str, label: &str) -> usize {
    let game = h.state().game.clone().unwrap();
    let t = game.table(table).unwrap();
    (0..t.len()).find(|&r| t.get(r, column) == Some(label)).unwrap()
}

fn undo(h: &mut Harness<'_, Moonglow>) {
    h.state_mut().actions.push(mg_ui::Action::Undo);
    // (A grid's widths settle in a few frames as a column comes or goes.)
    h.run_steps(4);
    h.run();
}

/// Skills: a rank set on several creatures is set on each, at the
/// skill's place in each one's list (a list too short for it filled up
/// to it), in one step; where they differ, the rank is marked ≠.
#[test]
fn a_skill_rank_is_set_on_each_creature() {
    let short = |i: usize, c: &mut Struct, _: &mg_rules::GameData| match i {
        // The second has another rank in its first skill, the third a list
        // of two skills only.
        1 => {
            let mut list = c.list("SkillList").unwrap().to_vec();
            list[0].set("Rank", Value::Byte(11));
            c.set("SkillList", Value::List(list));
        }
        2 => {
            let list = c.list("SkillList").unwrap()[..2].to_vec();
            c.set("SkillList", Value::List(list));
        }
        _ => {}
    };
    let Some((mut h, keys)) = together("nw_bandit001", ResType::UTC, 3, "Skills", short) else {
        return;
    };
    assert!(h.query_by_label("Skills").is_some(), "the Skills page is offered");
    assert!(h.query_all_by_value("≠").count() >= 1, "the first skill's ranks differ");
    let hide = row_of(&h, "skills", "Label", "Hide");
    let rank = |s: &Struct| s.list("SkillList").and_then(|l| l.get(hide)?.integer("Rank"));
    let before = each(&mut h, &keys, rank);
    assert_eq!(before[2], None, "the third's list is short");
    let name = {
        let game = h.state().game.clone().unwrap();
        let t = game.table("skills").unwrap();
        game.string(mg_core::StrRef(t.get_int(hide, "Name").unwrap() as u32)).unwrap()
    };
    let field_node = number_beside(&h, &name, 0);
    set_number(&h, &field_node, 9.0);
    h.run();
    assert_eq!(each(&mut h, &keys, rank), [Some(9); 3]);
    let third = field(&mut h, &keys[2]);
    assert_eq!(third.list("SkillList").unwrap().len(), hide + 1, "filled up to Hide");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Skill rank"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, rank), before, "one step");
    assert_eq!(field(&mut h, &keys[2]).list("SkillList").unwrap().len(), 2);
}

/// Special abilities: an ability (Fireball at caster level 5) two of the
/// creatures have, in two and three uses, is marked "≠ 2 of 3" and its
/// uses ≠; uses typed are set on the two, Give to All gives the third the
/// first one's two uses, Remove takes it from all, and a spell clicked
/// adds a use to each. Each one step.
#[test]
fn a_special_ability_is_set_on_those_that_have_it() {
    let fireball = 58u16;
    let ability = || {
        let mut s = Struct::new(4);
        s.set("Spell", Value::Word(fireball));
        s.set("SpellCasterLevel", Value::Byte(5));
        s.set("SpellFlags", Value::Byte(1));
        s
    };
    let tweak = move |i: usize, c: &mut Struct, _: &mg_rules::GameData| {
        let uses = [2, 3, 0][i];
        c.set("SpecAbilityList", Value::List(vec![ability(); uses]));
    };
    let Some((mut h, keys)) = together("nw_bandit001", ResType::UTC, 3, "Special Abilities", tweak)
    else {
        return;
    };
    let uses = |s: &Struct| {
        let list = s.list("SpecAbilityList").unwrap_or(&[]);
        list.iter().filter(|e| e.integer("Spell") == Some(i64::from(fireball))).count()
    };
    assert_eq!(each(&mut h, &keys, uses), [2, 3, 0]);
    assert!(h.query_by_label("≠ 2 of 3").is_some());
    assert!(h.query_all_by_value("≠").count() >= 1, "the uses differ");
    let name = name_of(&h, "spells", "Name", fireball as usize);
    // The ability's row is the right-hand one.
    let right = |h: &Harness<'_, Moonglow>| {
        let mut all: Vec<_> = h.get_all_by_label(&name).map(|n| n.rect()).collect();
        all.sort_by(|a, b| a.min.x.total_cmp(&b.min.x));
        (all[0], *all.last().unwrap())
    };
    let row = right(&h).1;
    let field_node = h
        .get_all_by_role(egui::accesskit::Role::SpinButton)
        .filter(|s| (s.rect().center().y - row.center().y).abs() < 10.0)
        .min_by(|a, b| a.rect().min.x.total_cmp(&b.rect().min.x))
        .expect("its uses");
    set_number(&h, &field_node, 4.0);
    h.run();
    assert_eq!(each(&mut h, &keys, uses), [4, 4, 0]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Uses"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, uses), [2, 3, 0]);
    h.get_by_label("Give to All").click();
    // (The grid's widths settle as its last column goes.)
    h.run_steps(6);
    assert_eq!(each(&mut h, &keys, uses), [2, 3, 2]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add special ability"));
    undo(&mut h);
    h.get_by_label("Remove").click();
    h.run();
    assert_eq!(each(&mut h, &keys, uses), [0, 0, 0]);
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, uses), [2, 3, 0]);
    // A spell clicked on the left: a use more on each, at its own level.
    type_into_hint(&mut h, "Find", &name);
    let left = right(&h).0;
    h.get_all_by_label(&name).find(|n| n.rect() == left).unwrap().click();
    h.run();
    assert_eq!(each(&mut h, &keys, uses), [3, 4, 1]);
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, uses), [2, 3, 0]);
}

/// An encounter's creatures: a bandit only the second spawns is marked
/// "≠ 1 of 3", Give to All adds it to the others, Remove Creature takes it
/// from all, Unique is set on each that spawns it; each one step.
#[test]
fn an_encounter_s_creature_is_added_to_those_that_lack_it() {
    let bandit = ResRef::from_str("nw_bandit001").unwrap();
    let tweak = move |i: usize, e: &mut Struct, _: &mg_rules::GameData| {
        let mut list = e.list("CreatureList").unwrap_or(&[]).to_vec();
        match i {
            1 => {
                let mut c = Struct::new(0);
                c.set("Appearance", Value::Int(4));
                c.set("CR", Value::Float(1.0));
                c.set("ResRef", Value::resref(bandit));
                c.set("SingleSpawn", Value::Byte(0));
                list.push(c);
            }
            2 => list.clear(),
            _ => {}
        }
        e.set("CreatureList", Value::List(list));
    };
    let Some((mut h, keys)) = together("nw_verminbeet", ResType::UTE, 3, "Creature List", tweak)
    else {
        return;
    };
    let spawns = |s: &Struct| -> Vec<String> {
        let list = s.list("CreatureList").unwrap_or(&[]);
        list.iter().filter_map(|c| c.resref("ResRef")).map(|r| r.to_string()).collect()
    };
    let before = each(&mut h, &keys, spawns);
    assert!(!before[0].is_empty());
    assert!(h.query_by_label("≠ 1 of 3").is_some(), "the bandit");
    assert_eq!(h.query_all_by_label("≠ 2 of 3").count(), before[0].len(), "the beetles");
    // Give to All, the bandit chosen.
    let name = {
        let game = h.state().game.clone().unwrap();
        let utc = Gff::read(&game.resman.get_named("nw_bandit001", ResType::UTC).unwrap()).unwrap();
        mg_module::palette::blueprint_name(
            mg_module::palette::BlueprintKind::Creature,
            &utc.root,
            &game,
        )
    };
    let choose = |h: &mut Harness<'_, Moonglow>| {
        let row = h.get_by_label("nw_bandit001").rect();
        h.get_all_by_label(&name)
            .find(|n| (n.rect().center().y - row.center().y).abs() < 6.0)
            .expect("the bandit's row")
            .click();
        h.run();
    };
    choose(&mut h);
    h.get_by_label("Give to All").click();
    h.run_steps(4);
    let given = each(&mut h, &keys, spawns);
    for s in &given {
        assert_eq!(s.iter().filter(|r| *r == "nw_bandit001").count(), 1, "{s:?}");
    }
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add creature"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, spawns), before);
    // Unique, on the bandit's row: set on the one that spawns it.
    let unique = |s: &Struct| -> Vec<i64> {
        let list = s.list("CreatureList").unwrap_or(&[]);
        list.iter().filter_map(|c| c.integer("SingleSpawn")).collect()
    };
    let was = each(&mut h, &keys, unique);
    h.get_all_by_role(egui::accesskit::Role::CheckBox).last().unwrap().click();
    h.run();
    let now = each(&mut h, &keys, unique);
    assert_eq!(now[0], was[0]);
    assert_eq!(now[1].last(), Some(&1));
    undo(&mut h);
    // Remove Creature, the bandit chosen.
    choose(&mut h);
    h.get_by_label("Remove Creature").click();
    h.run_steps(4);
    let taken = each(&mut h, &keys, spawns);
    assert!(taken.iter().all(|s| !s.contains(&"nw_bandit001".to_string())));
    assert_eq!(taken[0], before[0]);
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, spawns), before);
}

/// A store's restrictions: stores in different modes are marked ≠; the
/// first one's list is shown merged; a base item added goes to each in
/// that mode that lacks it (not to one in the other mode, which is said),
/// Remove takes it from each, and a mode clicked moves the others' items
/// to it. Each one step.
#[test]
fn a_store_s_restrictions_are_set_on_each() {
    let (torch, longsword) = (15i32, 1i32);
    let tweak = move |i: usize, s: &mut Struct, _: &mg_rules::GameData| {
        let entry = |row: i32| {
            let mut e = Struct::new(97869);
            e.set("BaseItem", Value::Int(row));
            e
        };
        let (not, only) = match i {
            0 => (vec![entry(torch)], vec![]),
            1 => (vec![entry(torch), entry(longsword)], vec![]),
            _ => (vec![], vec![entry(longsword)]),
        };
        s.set("WillNotBuy", Value::List(not));
        s.set("WillOnlyBuy", Value::List(only));
    };
    let Some((mut h, keys)) = together("nw_storebar01", ResType::UTM, 3, "Restrictions", tweak)
    else {
        return;
    };
    let bases = |s: &Struct| -> (Vec<i64>, Vec<i64>) {
        let of = |l: &str| s.list(l).unwrap_or(&[]).iter().filter_map(|i| i.integer("BaseItem"));
        (of("WillNotBuy").collect(), of("WillOnlyBuy").collect())
    };
    let before = each(&mut h, &keys, bases);
    assert!(h.query_all_by_value("≠").count() >= 1, "the modes differ");
    assert!(h.query_by_label("≠ 2 of 3").is_some(), "the torch");
    assert!(h.query_by_label("≠ 1 of 3").is_some(), "the longsword");
    // The longsword, added: to the first; the third, which will only buy
    // its own, is left.
    type_into_hint(&mut h, "Find", "Longsword");
    h.get_all_by_label("Longsword").next().unwrap().click();
    h.run();
    h.get_by_label("Add").click();
    h.run();
    let added = each(&mut h, &keys, bases);
    assert_eq!(added[0].0, [i64::from(torch), i64::from(longsword)]);
    assert_eq!(added[1], before[1]);
    assert_eq!(added[2], before[2]);
    assert!(h.query_by_label_contains("mg_c: it will only buy its list").is_some());
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add restricted item"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, bases), before);
    // The torch, removed from the two that have it.
    h.get_all_by_label("Torch").last().unwrap().click();
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    let taken = each(&mut h, &keys, bases);
    assert!(taken.iter().all(|(not, _)| !not.contains(&i64::from(torch))), "{taken:?}");
    undo(&mut h);
    // Will Only Buy: the first two's items moved; the third's as it was.
    h.get_by_label("Store will ONLY buy the following items").click();
    h.run();
    let moved = each(&mut h, &keys, bases);
    assert_eq!(moved[0], (vec![], before[0].0.clone()));
    assert_eq!(moved[1], (vec![], before[1].0.clone()));
    assert_eq!(moved[2], before[2]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Restriction mode"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, bases), before);
}

/// Classes: Fighter (levels 3 and 5) and Rogue, each of which two of the
/// three have, are marked ≠ 2 of 3 and the levels ≠. A level is set on
/// each that has the class; Remove takes the class from each but the one
/// it is the only class of; Give to All gives the Rogue to the first at
/// the second's level. Each one step.
#[test]
fn a_class_is_set_on_those_that_have_it_within_the_rules() {
    let (fighter, rogue) = (4i32, 8i32);
    let tweak = move |i: usize, c: &mut Struct, _: &mg_rules::GameData| {
        let class = |row: i32, level: i16| {
            let mut e = Struct::new(2);
            e.set("Class", Value::Int(row));
            e.set("ClassLevel", Value::Short(level));
            e
        };
        let list = match i {
            0 => vec![class(fighter, 3)],
            1 => vec![class(fighter, 5), class(rogue, 2)],
            _ => vec![class(rogue, 1)],
        };
        c.set("ClassList", Value::List(list));
    };
    let Some((mut h, keys)) = together("nw_bandit001", ResType::UTC, 3, "Classes", tweak) else {
        return;
    };
    let classes = |s: &Struct| -> Vec<(i64, i64)> {
        let list = s.list("ClassList").unwrap_or(&[]);
        list.iter()
            .map(|c| (c.integer("Class").unwrap(), c.integer("ClassLevel").unwrap()))
            .collect()
    };
    let before = each(&mut h, &keys, classes);
    assert_eq!(h.query_all_by_label("≠ 2 of 3").count(), 2);
    assert!(h.query_all_by_value("≠").count() >= 1, "the Fighter's levels differ");
    assert!(h.get_by_label("Levelup Wizard").accesskit_node().is_disabled());
    let level = number_beside(&h, "Class 1", 0);
    set_number(&h, &level, 7.0);
    h.run();
    let set = each(&mut h, &keys, classes);
    assert_eq!(set[0], [(4, 7)]);
    assert_eq!(set[1], [(4, 7), (8, 2)]);
    assert_eq!(set[2], before[2]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Class level"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, classes), before);
    // Remove, on the Fighter's row: the first keeps it, its only class.
    let row = h.get_by_label("Class 1").rect();
    h.get_all_by_label("Remove")
        .find(|n| (n.rect().center().y - row.center().y).abs() < 10.0)
        .unwrap()
        .click();
    h.run();
    let taken = each(&mut h, &keys, classes);
    assert_eq!(taken[0], before[0]);
    assert_eq!(taken[1], [(8, 2)]);
    assert!(h.query_by_label_contains("mg_a: its only class").is_some());
    undo(&mut h);
    // Give to All, on the Rogue's row: the first gains it at level 2.
    let row = h.get_by_label("Class 2").rect();
    h.get_all_by_label("Give to All")
        .find(|n| (n.rect().center().y - row.center().y).abs() < 10.0)
        .unwrap()
        .click();
    h.run_steps(4);
    let given = each(&mut h, &keys, classes);
    assert_eq!(given[0], [(4, 3), (8, 2)]);
    assert_eq!(given[2], before[2]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add class"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, classes), before);
}

/// Spells: of three creatures, two are sorcerers (the class ≠ 2 of 3),
/// the second not knowing the first's first cantrip, half-ticked: ticked,
/// it is added to the second only (the third has no Sorcerer class); its
/// Remove takes it from both; Clear empties both lists; each one step.
#[test]
fn a_known_spell_is_added_to_each_creature_of_the_class() {
    let sorcerer = 9i64;
    let tweak = move |i: usize, c: &mut Struct, _: &mg_rules::GameData| {
        let mut list = c.list("ClassList").unwrap().to_vec();
        let at = list.iter().position(|e| e.integer("Class") == Some(sorcerer)).unwrap();
        match i {
            1 => {
                let mut known = list[at].list("KnownList0").unwrap().to_vec();
                known.remove(0);
                list[at].set("KnownList0", Value::List(known));
            }
            2 => list[at].set("Class", Value::Int(4)),
            _ => {}
        }
        c.set("ClassList", Value::List(list));
    };
    let Some((mut h, keys)) = together("db_tanarukk_do", ResType::UTC, 3, "Spells", tweak) else {
        return;
    };
    let known = |s: &Struct| -> Vec<i64> {
        let classes = s.list("ClassList").unwrap();
        let Some(c) = classes.iter().find(|c| c.integer("Class") == Some(sorcerer)) else {
            return Vec::new();
        };
        c.list("KnownList0").unwrap_or(&[]).iter().filter_map(|s| s.integer("Spell")).collect()
    };
    let before = each(&mut h, &keys, known);
    let cantrip = before[0][0];
    assert!(!before[1].contains(&cantrip));
    assert!(h.query_by_label("≠ 2 of 3").is_some(), "the Sorcerer class");
    let name = name_of(&h, "spells", "Name", cantrip as usize);
    type_into_hint(&mut h, "Find", &name);
    let boxes: Vec<_> = h
        .get_all_by_label(&name)
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .collect();
    boxes[0].click();
    h.run();
    let added = each(&mut h, &keys, known);
    assert_eq!(added[0], before[0]);
    assert_eq!(added[1].iter().filter(|s| **s == cantrip).count(), 1);
    assert_eq!(added[2], before[2]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add known spell"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, known), before);
    // Remove beside the second cantrip, which both know: forgotten by both.
    let other = before[0][1];
    let other_name = name_of(&h, "spells", "Name", other as usize);
    let shown = h
        .get_all_by_label(&other_name)
        .find(|n| n.accesskit_node().role() != egui::accesskit::Role::CheckBox)
        .unwrap()
        .rect();
    let named = format!("Remove {other_name}");
    let cross = h.get_by_label(&named);
    assert!((cross.rect().center().y - shown.center().y).abs() < 6.0, "on its row");
    cross.click();
    h.run();
    assert!(each(&mut h, &keys, known).iter().all(|k| !k.contains(&other)));
    undo(&mut h);
    h.get_by_label("Clear Class Spell List").click();
    h.run_steps(4);
    let cleared = each(&mut h, &keys, known);
    assert!(cleared[0].is_empty() && cleared[1].is_empty());
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Clear class spell list"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, known), before);
    assert!(h.get_by_label("Save Class Spell List").accesskit_node().is_disabled());
}

/// Item properties: an Enhancement Bonus +1 on the first and +2 on the
/// second are two entries (matched by type, subtype and value), each ≠ 1
/// of 3. Give to All gives the +1 to the second (beside its +2) and leaves
/// the third, whose base item takes no enhancement; a parameter set is
/// set on each that has the property; Remove takes it from each. Each one
/// step.
#[test]
fn an_item_property_is_matched_by_type_subtype_and_value() {
    let enhancement = |cost: u16| mg_rules::ItemProperty {
        property: 6,
        subtype: 0,
        cost_table: 2,
        cost_value: cost,
        param1: 255,
        param1_value: 0,
        chance: 100,
    };
    let tweak = move |i: usize, s: &mut Struct, game: &mg_rules::GameData| {
        let props = match i {
            0 => vec![enhancement(1).to_gff()],
            1 => vec![enhancement(2).to_gff()],
            _ => {
                // A base item that takes no enhancement.
                let t = game.table("baseitems").unwrap();
                let row = (0..t.len())
                    .find(|&r| {
                        t.get_int(r, "PropColumn").is_some()
                            && !game.available_properties(r as u32).iter().any(|p| p.row == 6)
                            && !game.available_properties(r as u32).is_empty()
                    })
                    .unwrap();
                s.set("BaseItem", Value::Int(row as i32));
                vec![]
            }
        };
        s.set("PropertiesList", Value::List(props));
    };
    let Some((mut h, keys)) = together("nw_wswls001", ResType::UTI, 3, "Properties", tweak) else {
        return;
    };
    let props = |s: &Struct| -> Vec<(i64, i64, i64)> {
        let list = s.list("PropertiesList").unwrap_or(&[]);
        list.iter()
            .map(|p| {
                let n = |l: &str| p.integer(l).unwrap_or(0);
                (n("PropertyName"), n("CostValue"), n("ChanceAppear"))
            })
            .collect()
    };
    let before = each(&mut h, &keys, props);
    assert_eq!(h.query_all_by_label("≠ 1 of 3").count(), 2);
    let game = h.state().game.clone().unwrap();
    let text = |p| game.property_text(&p);
    let plus_one = text(enhancement(1));
    h.get_by_label(&plus_one).click();
    h.run();
    h.get_by_label("Give to All").click();
    h.run();
    let given = each(&mut h, &keys, props);
    assert_eq!(given[0], before[0]);
    assert_eq!(given[1], [(6, 2, 100), (6, 1, 100)]);
    assert_eq!(given[2], before[2]);
    assert!(h.query_by_label_contains("mg_c: its base item doesn't take it").is_some());
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add property"));
    // Its chance of appearing, set on the two that have it.
    let chance = number_beside(&h, "Chance of Appearing (%)", 0);
    set_number(&h, &chance, 50.0);
    h.run();
    let set = each(&mut h, &keys, props);
    assert_eq!(set[0], [(6, 1, 50)]);
    assert_eq!(set[1], [(6, 2, 100), (6, 1, 50)]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Property parameters"));
    undo(&mut h);
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, props), before);
    // The +2 removed: from the second.
    let plus_two = text(enhancement(2));
    h.get_by_label(&plus_two).click();
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    let taken = each(&mut h, &keys, props);
    assert!(taken[1].is_empty());
    assert_eq!(taken[0], before[0]);
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, props), before);
}

/// An inventory's entry of an item blueprint, at a place.
fn held_entry(r: &str, x: u16) -> Struct {
    let mut s = Struct::new(u32::from(x));
    s.set("InventoryRes", Value::resref(ResRef::from_str(r).unwrap()));
    s.set("Repos_PosX", Value::Word(x));
    s.set("Repos_Posy", Value::Word(0));
    s
}

/// The item blueprints a list holds.
fn held_items(s: &Struct, list: &str) -> Vec<String> {
    let items = s.list(list).unwrap_or(&[]);
    items.iter().filter_map(|i| i.resref("InventoryRes")).map(|r| r.to_string()).collect()
}

/// Inventories: three chests, a longsword in two of them (one holding
/// two) and a torch in one, are listed merged, ≠ with how many. Give to
/// All puts a torch in the two without, each at its own free place;
/// Remove takes every longsword from each; Paste adds the item copied to
/// each. Each one step.
#[test]
fn an_inventory_s_items_are_added_to_each_and_taken_from_each() {
    let tweak = |i: usize, p: &mut Struct, _: &mg_rules::GameData| {
        p.set("HasInventory", Value::Byte(1));
        let items = match i {
            0 => vec![held_entry("nw_wswls001", 0)],
            1 => vec![
                held_entry("nw_wswls001", 0),
                held_entry("nw_wswls001", 2),
                held_entry("nw_it_torch001", 4),
            ],
            _ => vec![],
        };
        p.set("ItemList", Value::List(items));
    };
    let Some((mut h, keys)) = together("plc_chest1", ResType::UTP, 3, "Inventory", tweak) else {
        return;
    };
    let items = |s: &Struct| held_items(s, "ItemList");
    let before = each(&mut h, &keys, items);
    assert!(h.query_by_label("≠ 2 of 3").is_some(), "the longsword");
    assert!(h.query_by_label("≠ 1 of 3").is_some(), "the torch");
    assert!(h.query_all_by_value("≠").count() >= 1, "two longswords and one");
    h.get_all_by_label("Give to All").last().unwrap().click();
    h.run_steps(4);
    let given = each(&mut h, &keys, items);
    for g in &given {
        assert_eq!(g.iter().filter(|r| *r == "nw_it_torch001").count(), 1, "{g:?}");
    }
    // At its own first free place: beside the first's longsword.
    let first = field(&mut h, &keys[0]);
    let torch = &first.list("ItemList").unwrap()[1];
    assert_ne!(torch.integer("Repos_PosX"), Some(0), "not on the longsword");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add item"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, items), before);
    // Remove, on the longsword's row (the first).
    h.get_all_by_label("Remove").next().unwrap().click();
    h.run_steps(4);
    let taken = each(&mut h, &keys, items);
    assert!(taken.iter().all(|t| !t.contains(&"nw_wswls001".to_string())), "{taken:?}");
    assert_eq!(taken[1], ["nw_it_torch001"]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Remove Item"));
    undo(&mut h);
    // Paste: the item copied, added to each.
    h.state_mut().item_clip = vec![held_entry("nw_it_gem001", 0)];
    h.run();
    h.get_by_label("Paste").click();
    h.run_steps(4);
    let pasted = each(&mut h, &keys, items);
    for (p, b) in pasted.iter().zip(&before) {
        assert_eq!(p.len(), b.len() + 1);
        assert_eq!(p.last().map(String::as_str), Some("nw_it_gem001"));
    }
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, items), before);
}

/// Placeables edited together of which the first has no inventory: the
/// Inventory page is the others', and an item pasted goes to those with
/// one, the one without left and said.
#[test]
fn a_placeable_without_an_inventory_is_left() {
    let tweak = |i: usize, p: &mut Struct, _: &mg_rules::GameData| {
        p.set("HasInventory", Value::Byte(u8::from(i > 0)));
        p.set("ItemList", Value::List(Vec::new()));
    };
    let Some((mut h, keys)) = together("plc_chest1", ResType::UTP, 2, "Inventory", tweak) else {
        return;
    };
    assert!(h.query_by_label_contains("has no inventory").is_none());
    h.state_mut().item_clip = vec![held_entry("nw_it_gem001", 0)];
    h.run();
    h.get_by_label("Paste").click();
    h.run_steps(4);
    let items = each(&mut h, &keys, |s| held_items(s, "ItemList"));
    assert_eq!(items, [vec![], vec!["nw_it_gem001".to_string()]]);
    assert!(h.query_by_label_contains("mg_a: no inventory").is_some());
}

/// A creature's equipment, edited together: a battleaxe chosen in the
/// palette and equipped in the primary weapon slot is worn by each, what
/// each wore there going to its backpack, in one step.
#[test]
fn an_item_is_equipped_on_each_creature() {
    let tweak = |i: usize, c: &mut Struct, _: &mg_rules::GameData| {
        if i == 2 {
            c.set("Equip_ItemList", Value::List(Vec::new()));
        }
    };
    let Some((mut h, keys)) = together("nw_bandit001", ResType::UTC, 3, "Inventory", tweak) else {
        return;
    };
    let primary = |s: &Struct| -> Option<String> {
        let list = s.list("Equip_ItemList").unwrap_or(&[]);
        list.iter()
            .find(|e| e.id == 0x10)
            .and_then(|e| e.resref("EquippedRes"))
            .map(|r| r.to_string())
    };
    let before = each(&mut h, &keys, primary);
    let backpacks = each(&mut h, &keys, |s| held_items(s, "ItemList"));
    assert!(before[0].is_some() && before[2].is_none());
    assert!(h.query_all_by_value("≠").count() >= 1, "the third wears nothing");
    type_into_hint(&mut h, "Find", "nw_waxbt001");
    h.get_by_label("Battleaxe").click();
    h.run();
    let row = h.get_by_label("Primary Weapon").rect();
    h.get_all_by_label("Equip")
        .find(|n| (n.rect().center().y - row.center().y).abs() < 8.0)
        .expect("the slot's Equip")
        .click();
    h.run_steps(4);
    assert_eq!(each(&mut h, &keys, primary), vec![Some("nw_waxbt001".to_string()); 3]);
    let now = each(&mut h, &keys, |s| held_items(s, "ItemList"));
    for i in 0..2 {
        assert_eq!(now[i].len(), backpacks[i].len() + 1, "the old weapon in the backpack");
    }
    assert_eq!(now[2], backpacks[2]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Equip"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, primary), before);
}

/// A store's inventory, edited together: an item pasted goes to each
/// store, on the page of its kind (made where a store has none).
#[test]
fn a_store_s_item_goes_to_each_on_its_page() {
    let tweak = |i: usize, s: &mut Struct, _: &mg_rules::GameData| {
        if i == 1 {
            s.set("StoreList", Value::List(Vec::new()));
        }
    };
    let Some((mut h, keys)) = together("nw_storebar01", ResType::UTM, 2, "Inventory", tweak) else {
        return;
    };
    let pages = |s: &Struct| -> Vec<(u32, Vec<String>)> {
        let list = s.list("StoreList").unwrap_or(&[]);
        list.iter().map(|p| (p.id, held_items(p, "ItemList"))).collect()
    };
    let before = each(&mut h, &keys, pages);
    h.state_mut().item_clip = vec![held_entry("nw_wswls001", 0)];
    h.run();
    h.get_by_label("Paste").click();
    h.run_steps(4);
    let after = each(&mut h, &keys, pages);
    for (a, b) in after.iter().zip(&before) {
        let count = |p: &[(u32, Vec<String>)]| {
            p.iter().flat_map(|(_, i)| i).filter(|r| *r == "nw_wswls001").count()
        };
        assert_eq!(count(a), count(b) + 1);
    }
    // The second, which had no pages, gains the weapons' page.
    assert_eq!(after[1].len(), 1);
    assert_eq!(after[1][0].1, ["nw_wswls001"]);
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, pages), before);
}

/// The feats of each, as rows of feat.2da.
fn feat_rows(s: &Struct) -> Vec<i64> {
    s.list("FeatList").unwrap_or(&[]).iter().filter_map(|f| f.integer("Feat")).collect()
}

/// The game's name of a row of a table (its `column` StrRef).
fn name_of(h: &Harness<'_, Moonglow>, table: &str, column: &str, row: usize) -> String {
    let game = h.state().game.clone().unwrap();
    let t = game.table(table).unwrap();
    game.string(mg_core::StrRef(t.get_int(row, column).unwrap() as u32)).unwrap()
}

/// Feats: one some of the creatures have is half-ticked and marked "≠ 1
/// of 3"; ticked, it is given to those that lack it (one that has it gets
/// no second); Remove takes a feat from all that have it; each one step.
#[test]
fn a_feat_is_given_to_those_that_lack_it_and_taken_from_all() {
    let alertness = 0u16;
    let tweak = |i: usize, c: &mut Struct, _: &mg_rules::GameData| {
        let mut list = c.list("FeatList").unwrap_or(&[]).to_vec();
        list.retain(|f| f.integer("Feat") != Some(i64::from(alertness)));
        match i {
            // The second has Alertness, the third lacks the first feat.
            1 => {
                let mut f = Struct::new(1);
                f.set("Feat", Value::Word(alertness));
                list.push(f);
            }
            2 => {
                list.remove(0);
            }
            _ => {}
        }
        c.set("FeatList", Value::List(list));
    };
    let Some((mut h, keys)) = together("nw_bandit001", ResType::UTC, 3, "Feats", tweak) else {
        return;
    };
    let before = each(&mut h, &keys, feat_rows);
    let first = before[0][0];
    assert!(!before[2].contains(&first));
    // Marked where some have it: Alertness (1 of 3) and the first feat (2
    // of 3).
    assert!(h.query_by_label("≠ 1 of 3").is_some());
    assert!(h.query_by_label("≠ 2 of 3").is_some());
    let name = name_of(&h, "feat", "FEAT", alertness as usize);
    let boxes: Vec<_> = h
        .get_all_by_label(&name)
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .collect();
    assert_eq!(boxes.len(), 1);
    boxes[0].click();
    h.run();
    let after = each(&mut h, &keys, feat_rows);
    for (i, feats) in after.iter().enumerate() {
        let n = feats.iter().filter(|f| **f == i64::from(alertness)).count();
        assert_eq!(n, 1, "{i} has Alertness once");
    }
    assert_eq!(after[1], before[1], "the second already had it");
    assert!(h.query_by_label_contains("Added to 2; left: mg_b: has it already").is_some());
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add feat"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, feat_rows), before);
    // Remove beside the first feat, which two have: taken from both.
    let first_name = name_of(&h, "feat", "FEAT", first as usize);
    let row = h
        .get_all_by_label(&first_name)
        .find(|n| n.accesskit_node().role() != egui::accesskit::Role::CheckBox)
        .unwrap()
        .rect();
    let named = format!("Remove {first_name}");
    let cross = h.get_by_label(&named);
    assert!((cross.rect().center().y - row.center().y).abs() < 6.0, "on its row");
    cross.click();
    h.run();
    for feats in each(&mut h, &keys, feat_rows) {
        assert!(!feats.contains(&first));
    }
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Remove feat"));
    undo(&mut h);
    assert_eq!(each(&mut h, &keys, feat_rows), before);
    // Give to All, beside the first feat, which the third lacks.
    let row = h
        .get_all_by_label(&first_name)
        .find(|n| n.accesskit_node().role() != egui::accesskit::Role::CheckBox)
        .unwrap()
        .rect();
    h.get_all_by_label("Give to All")
        .find(|n| (n.rect().center().y - row.center().y).abs() < 6.0)
        .expect("its Give to All")
        .click();
    h.run();
    let given = each(&mut h, &keys, feat_rows);
    assert!(given.iter().all(|f| f.contains(&first)), "{given:?}");
    assert_eq!(given[0], before[0]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Add feat"));
    // What a window of several says of a change is its own, and goes
    // with it (the review's B28: kept by the document, the first one's
    // own Properties showed it too, for the rest of the session).
    undo(&mut h);
    let boxes: Vec<_> = h
        .get_all_by_label(&name)
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .collect();
    boxes[0].click();
    h.run();
    let said = "Added to 2; left: mg_b: has it already";
    assert!(h.query_by_label_contains(said).is_some());
    close_tab(h.state_mut(), &Tab::Blueprints(keys.clone()));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(keys[0])));
    h.run();
    h.run();
    assert!(h.query_by_label_contains(said).is_none(), "not the first one's own window's");
    close_tab(h.state_mut(), &Tab::Blueprint(keys[0]));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprints(keys.clone())));
    h.run();
    h.run();
    assert!(h.query_by_label_contains(said).is_none(), "nor the window's opened again");
}

/// Advanced Search's Add an Item and Remove an Item, through the same
/// rules as the Inventory page: a torch added to each chest found, at its
/// own first free place (beside what it holds), and every longsword
/// taken.
#[test]
fn advanced_search_adds_an_item_to_each_inventory_found() {
    use mg_edit::found::{self, Change, Entry};
    use mg_module::search::{Query, Search};
    let tweak = |i: usize, p: &mut Struct, _: &mg_rules::GameData| {
        p.set("HasInventory", Value::Byte(1));
        let items = if i == 0 { vec![held_entry("nw_wswls001", 0)] } else { Vec::new() };
        p.set("ItemList", Value::List(items));
    };
    let Some((mut h, keys)) = together("plc_chest1", ResType::UTP, 2, "Basic", tweak) else {
        return;
    };
    let game = h.state().game.clone().unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    let q = Query::new(Search::Field { label: Some("HasInventory".into()), value: None });
    let hits = mg_module::search::search(&ws.module, &q, Some(&game), game.codepage());
    let hits: Vec<_> = hits.iter().collect();
    assert_eq!(hits.len(), 2);
    let torch = ResRef::from_str("nw_it_torch001").unwrap();
    let add = Change::AddEntry { entry: Entry::Item(torch), name: "torch".into() };
    let out = found::edits(&ws.module, &hits, &add, game.codepage(), Some(&game)).unwrap();
    assert_eq!(out.changed, 2);
    ws.apply(mg_edit::Command::new(add.describe(), out.edits)).unwrap();
    let items = each(&mut h, &keys, |s| held_items(s, "ItemList"));
    assert_eq!(items, [vec!["nw_wswls001", "nw_it_torch001"], vec!["nw_it_torch001"]]);
    let first = field(&mut h, &keys[0]);
    let placed = &first.list("ItemList").unwrap()[1];
    assert_ne!(placed.integer("Repos_PosX"), Some(0), "beside the longsword");
    let sword = ResRef::from_str("nw_wswls001").unwrap();
    let remove = Change::RemoveEntry { entry: Entry::Item(sword), name: "longsword".into() };
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    let hits = mg_module::search::search(&ws.module, &q, Some(&game), game.codepage());
    let hits: Vec<_> = hits.iter().collect();
    let out = found::edits(&ws.module, &hits, &remove, game.codepage(), Some(&game)).unwrap();
    assert_eq!(out.changed, 1);
    ws.apply(mg_edit::Command::new(remove.describe(), out.edits)).unwrap();
    let items = each(&mut h, &keys, |s| held_items(s, "ItemList"));
    assert_eq!(items, [vec!["nw_it_torch001"], vec!["nw_it_torch001"]]);
}
