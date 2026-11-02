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
    let dir = mg_testkit::scratch_dir(&format!("ui-together-{name}-{}", page.replace(' ', "-")));
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
/// no second); × takes a feat from all that have it; each one step.
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
    // × beside the first feat, which two have: taken from both.
    let first_name = name_of(&h, "feat", "FEAT", first as usize);
    let row = h
        .get_all_by_label(&first_name)
        .find(|n| n.accesskit_node().role() != egui::accesskit::Role::CheckBox)
        .unwrap()
        .rect();
    let cross = h
        .get_all_by_label("×")
        .find(|n| (n.rect().center().y - row.center().y).abs() < 6.0)
        .expect("its ×");
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
}
