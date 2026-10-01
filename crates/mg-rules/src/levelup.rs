//! Levelling a creature up by its classes' packages, as Aurora's Levelup
//! Wizard does (captured: `levelup/bandit-fighter5.mod`, `six-after.mod`).
//! Each level, in class slot order:
//!
//! - hit points: a player class's whole hit die at the first level, else
//!   the class's average, (HitDie + 1) / 2, the total rounded down at the end
//!   (Fighter 1 to 10: + 49);
//! - every fourth character level, one point to the package's ability
//!   (packages.2da `Attribute`);
//! - skill points (classes.2da `SkillPointBase` + Intelligence modifier +
//!   the race's `ExtraSkillPointsPerLevel`, times its
//!   `FirstLevelSkillPointsMultiplier` at the first level), spent a rank at
//!   a time on the class skills down the package's skill list
//!   (`SkillPref2DA`), round after round while points last, up to the
//!   character level + 3; cross-class skills get none;
//! - feats: the race's (`FeatsTable`) at the first level, a cleric's
//!   domain powers (the package's domains) at its first level, the class's own (`FeatsTable` list 3 at that level, replacing
//!   the feat they succeed), a normal feat every third character level
//!   (the race's `NormalFeatEveryNthLevel`; and its `ExtraFeatsAtFirstLevel`
//!   at the first) and a bonus feat where the
//!   class's `BonusFeatsTable` has one, each the first of the package's
//!   feats (`FeatPref2DA`) the levelling class lists (its `FeatsTable`:
//!   list 0 or 1 for a normal feat, 1 or 2 for a bonus one) and whose
//!   prerequisites the creature meets, the feats granted that level
//!   counting but not the level's other picks.
//!
//! A class the creature did not have gets its spells: for a class that
//! prepares them, each spell level's slots (the `SpellGainTable`, and
//! bonus slots for the casting ability) filled with a domain spell (the
//! package's first domain's for that level, else the second's) and then
//! the package's spells of that level (`SpellPref2DA`) in order, the first
//! repeated in what is left; for one that knows them, the package's first
//! spells of each level up to the `SpellKnownTable` (not yet checked
//! against Aurora). Its package's equipment is [`GameData::new_class_gear`]'s
//! (the caller makes the items).

use std::collections::BTreeSet;

use mg_gff::{Struct, Value};

use crate::GameData;
use crate::creatures::{ABILITIES, modifier};

/// Struct ids of a creature's lists, as Aurora writes them.
const FEAT_ID: u32 = 1;
const CLASS_ID: u32 = 2;
const SPELL_ID: u32 = 3;

/// What a feat may be taken as.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pick {
    Normal,
    Bonus,
}

/// A creature being levelled up.
struct State {
    classes: Vec<(u32, u32)>,
    feats: Vec<u16>,
    skills: Vec<u8>,
    abilities: [i32; 6],
    racial: [i32; 6],
    race: usize,
}

impl State {
    fn level(&self) -> u32 {
        self.classes.iter().map(|&(_, l)| l).sum()
    }
    fn has(&self, feat: u16) -> bool {
        self.feats.contains(&feat)
    }
    fn total(&self, ability: usize) -> i32 {
        self.abilities[ability] + self.racial[ability]
    }
}

impl GameData {
    /// The creature levelled up to `targets` (class, level): its classes'
    /// levels raised, classes it does not have added (up to eight), as
    /// Aurora's Levelup Wizard does. Fields besides the ones levelling
    /// changes are kept; `MaxHitPoints` and `ChallengeRating` are the
    /// caller's to recompute.
    pub fn level_up(&self, creature: &Struct, targets: &[(u32, u32)]) -> Struct {
        let mut c = creature.clone();
        let int = |s: &Struct, l: &str| s.integer(l).unwrap_or(0);
        let mut state = State {
            classes: c
                .list("ClassList")
                .unwrap_or(&[])
                .iter()
                .map(|e| (int(e, "Class").max(0) as u32, int(e, "ClassLevel").max(0) as u32))
                .collect(),
            feats: c
                .list("FeatList")
                .unwrap_or(&[])
                .iter()
                .filter_map(|f| f.integer("Feat"))
                .map(|f| f as u16)
                .collect(),
            skills: c
                .list("SkillList")
                .unwrap_or(&[])
                .iter()
                .map(|s| int(s, "Rank").clamp(0, 255) as u8)
                .collect(),
            abilities: ABILITIES.map(|a| int(&c, a) as i32),
            racial: self.racial_adjustments(int(&c, "Race").max(0) as u32),
            race: int(&c, "Race").max(0) as usize,
        };
        let mut spare = int(&c, "SkillPoints").max(0) as u32;
        let mut hit_points = 0.0f64;
        for &(class, target) in targets {
            if !state.classes.iter().any(|&(k, _)| k == class) {
                if state.classes.len() >= 8 {
                    continue;
                }
                state.classes.push((class, 0));
            }
            let slot = state.classes.iter().position(|&(k, _)| k == class).expect("added");
            while state.classes[slot].1 < target.min(60) {
                hit_points += self.level_once(&mut state, slot, &mut spare);
            }
        }

        // Back into the creature.
        let added = hit_points.floor() as i64;
        for label in ["HitPoints", "CurrentHitPoints"] {
            let hp = (int(&c, label) + added).clamp(i64::from(i16::MIN), i64::from(i16::MAX));
            c.set(label, Value::Short(hp as i16));
        }
        for (i, a) in ABILITIES.iter().enumerate() {
            c.set(a, Value::Byte(state.abilities[i].clamp(0, 255) as u8));
        }
        let mut list: Vec<Struct> = c.list("ClassList").unwrap_or(&[]).to_vec();
        for (i, &(class, level)) in state.classes.iter().enumerate() {
            if i >= list.len() {
                let mut e = Struct::new(CLASS_ID);
                e.set("Class", Value::Int(class as i32));
                e.set("ClassLevel", Value::Short(level as i16));
                for (label, spells) in self.new_class_spells(&state, class, level) {
                    let entries = spells
                        .into_iter()
                        .map(|spell| {
                            let mut s = Struct::new(SPELL_ID);
                            s.set("Spell", Value::Word(spell));
                            s.set("SpellMetaMagic", Value::Byte(0));
                            s.set("SpellFlags", Value::Byte(1));
                            s
                        })
                        .collect();
                    e.set(&label, Value::List(entries));
                }
                list.push(e);
            }
            list[i].set("ClassLevel", Value::Short(level as i16));
        }
        c.set("ClassList", Value::List(list));
        let old: Vec<Struct> = c.list("FeatList").unwrap_or(&[]).to_vec();
        let feats = state
            .feats
            .iter()
            .map(|&f| {
                old.iter()
                    .find(|s| s.integer("Feat") == Some(i64::from(f)))
                    .cloned()
                    .unwrap_or_else(|| {
                        let mut s = Struct::new(FEAT_ID);
                        s.set("Feat", Value::Word(f));
                        s
                    })
            })
            .collect();
        c.set("FeatList", Value::List(feats));
        if let Some(skills) = c.list_mut("SkillList") {
            for (s, rank) in skills.iter_mut().zip(&state.skills) {
                s.set("Rank", Value::Byte(*rank));
            }
        }
        if c.contains("SkillPoints") {
            c.set("SkillPoints", Value::Word(spare.min(u32::from(u16::MAX)) as u16));
        }
        c
    }

    /// One level of the class in `slot`; returns the hit points it adds.
    fn level_once(&self, s: &mut State, slot: usize, spare: &mut u32) -> f64 {
        s.classes[slot].1 += 1;
        let (class, class_level) = s.classes[slot];
        let level = s.level();
        let row = class as usize;
        let classes = self.table("classes").ok();
        let col = |c: &str| classes.as_ref().and_then(|t| t.get(row, c).map(str::to_owned));
        let num = |c: &str| classes.as_ref().and_then(|t| t.get_int(row, c));
        let package = num("Package").unwrap_or(-1);
        let packages = self.table("packages").ok();
        let pkg = |c: &str| {
            packages.as_ref().and_then(|t| {
                usize::try_from(package).ok().and_then(|p| t.get(p, c)).map(str::to_owned)
            })
        };

        // Hit points and the ability score.
        let die = f64::from(num("HitDie").unwrap_or(0).max(0));
        if level.is_multiple_of(4)
            && let Some(a) = pkg("Attribute")
            && let Some(i) = ABILITIES.iter().position(|x| x.eq_ignore_ascii_case(&a))
        {
            s.abilities[i] += 1;
        }

        // Skills: class skills only, a rank each down the package's list,
        // again and again while points last (a first level's 12 points go
        // two rounds).
        let extra = self.race_value(s, "ExtraSkillPointsPerLevel").unwrap_or(0);
        let mut per_level =
            (num("SkillPointBase").unwrap_or(0) + modifier(s.total(3)) + extra).max(1);
        if level == 1 {
            per_level *= self.race_value(s, "FirstLevelSkillPointsMultiplier").unwrap_or(4).max(1);
        }
        let mut points = *spare as i32 + per_level;
        let skill_table = col("SkillsTable").and_then(|t| self.table(&t.to_lowercase()).ok());
        let class_skill = |skill: u32| -> bool {
            skill_table.as_ref().is_some_and(|t| {
                (0..t.len()).any(|r| {
                    t.get_int(r, "SkillIndex") == Some(skill as i32)
                        && t.get_int(r, "ClassSkill") == Some(1)
                })
            })
        };
        let order: Vec<usize> = pkg("SkillPref2DA")
            .and_then(|n| self.table(&n.to_lowercase()).ok())
            .map(|t| {
                (0..t.len())
                    .filter_map(|r| t.get_int(r, "SkillIndex").and_then(|v| u32::try_from(v).ok()))
                    .filter(|&k| class_skill(k))
                    .map(|k| k as usize)
                    .collect()
            })
            .unwrap_or_default();
        loop {
            let mut spent = false;
            for &k in &order {
                let Some(rank) = s.skills.get_mut(k) else { continue };
                if points >= 1 && u32::from(*rank) < level + 3 {
                    *rank += 1;
                    points -= 1;
                    spent = true;
                }
            }
            if !spent || points < 1 {
                break;
            }
        }
        *spare = points.max(0) as u32;

        // Feats: the race's at the first level, domains, the class's own,
        // then the picks.
        let mut taken: BTreeSet<u16> = BTreeSet::new();
        let feat_2da = self.table("feat").ok();
        let grant = |s: &mut State, f: u16| {
            if s.has(f) {
                return;
            }
            // A feat replaces the one it succeeds (Sneak Attack 2, 1).
            s.feats.retain(|&old| {
                feat_2da
                    .as_ref()
                    .and_then(|t| t.get_int(usize::from(old), "SUCCESSOR"))
                    .is_none_or(|succ| succ != i32::from(f))
            });
            s.feats.push(f);
        };
        if level == 1
            && let Some(t) = self
                .table("racialtypes")
                .ok()
                .and_then(|r| r.get(s.race, "FeatsTable").map(str::to_lowercase))
                .and_then(|n| self.table(&n).ok())
        {
            for r in 0..t.len() {
                if let Some(f) = t.get_int(r, "FeatIndex").and_then(|f| u16::try_from(f).ok()) {
                    grant(s, f);
                }
            }
        }
        if class_level == 1 {
            for d in ["Domain1", "Domain2"] {
                let feat = pkg(d)
                    .and_then(|v| v.parse::<usize>().ok())
                    .and_then(|d| self.table("domains").ok()?.get_int(d, "GrantedFeat"));
                if let Some(f) = feat.and_then(|f| u16::try_from(f).ok()) {
                    grant(s, f);
                }
            }
        }
        let feats_table = col("FeatsTable").and_then(|t| self.table(&t.to_lowercase()).ok());
        if let Some(t) = &feats_table {
            for r in 0..t.len() {
                if t.get_int(r, "List") == Some(3)
                    && t.get_int(r, "GrantedOnLevel") == Some(class_level as i32)
                    && let Some(f) = t.get_int(r, "FeatIndex").and_then(|f| u16::try_from(f).ok())
                {
                    grant(s, f);
                }
            }
        }
        // What the picks' prerequisites see: the feats granted so far, this
        // level's included (a fighter's first-level Martial Weapon
        // Proficiency lets it take Weapon Focus), but not the other picks.
        let before = s.feats.clone();
        let every = self.race_value(s, "NormalFeatEveryNthLevel").unwrap_or(3).max(1) as u32;
        let mut normal = if level == 1 || level.is_multiple_of(every) {
            self.race_value(s, "NumberNormalFeatsEveryNthLevel").unwrap_or(1).max(0) as u32
        } else {
            0
        };
        if level == 1 {
            normal += self.race_value(s, "ExtraFeatsAtFirstLevel").unwrap_or(0).max(0) as u32;
        }
        let bonus = col("BonusFeatsTable")
            .and_then(|t| self.table(&t.to_lowercase()).ok())
            .and_then(|t| t.get_int(class_level as usize - 1, "Bonus"))
            .unwrap_or(0)
            .max(0) as u32;
        let prefs: Vec<u16> = pkg("FeatPref2DA")
            .and_then(|n| self.table(&n.to_lowercase()).ok())
            .map(|t| {
                (0..t.len())
                    .filter_map(|r| t.get_int(r, "FeatIndex").and_then(|f| u16::try_from(f).ok()))
                    .collect()
            })
            .unwrap_or_default();
        for (pick, count) in [(Pick::Normal, normal), (Pick::Bonus, bonus)] {
            for _ in 0..count {
                let choice = prefs.iter().copied().find(|&f| {
                    !s.has(f) && self.may_take(s, &before, slot, f, pick) && !taken.contains(&f)
                });
                if let Some(f) = choice {
                    taken.insert(f);
                    s.feats.push(f);
                }
            }
        }
        // A player class's first level has its whole hit die; other levels,
        // and a monster class's first (a goblin's Humanoid 1: 4 of 8), the
        // average.
        if level == 1 && num("PlayerClass") == Some(1) { die } else { (die + 1.0) / 2.0 }
    }

    /// The spell lists of a class new to the creature at `level`: (label,
    /// spells) for each spell level with any.
    fn new_class_spells(&self, s: &State, class: u32, level: u32) -> Vec<(String, Vec<u16>)> {
        let Ok(classes) = self.table("classes") else { return Vec::new() };
        let row = class as usize;
        if classes.get_int(row, "SpellCaster") != Some(1) || level == 0 {
            return Vec::new();
        }
        let prepares = classes.get_int(row, "MemorizesSpells") == Some(1);
        let Some(column) = classes.get(row, "SpellTableColumn").map(str::to_owned) else {
            return Vec::new();
        };
        let table = |c: &str| classes.get(row, c).and_then(|n| self.table(&n.to_lowercase()).ok());
        let spells_2da = self.table("spells").ok();
        let spell_level = |spell: u16| -> Option<usize> {
            spells_2da.as_ref()?.get_int(usize::from(spell), &column).map(|l| l.max(0) as usize)
        };
        let packages = self.table("packages").ok();
        let package = classes.get_int(row, "Package").and_then(|p| usize::try_from(p).ok());
        let pkg = |c: &str| package.and_then(|p| packages.as_ref()?.get(p, c).map(str::to_owned));
        let prefs: Vec<u16> = pkg("SpellPref2DA")
            .and_then(|n| self.table(&n.to_lowercase()).ok())
            .map(|t| {
                (0..t.len())
                    .filter_map(|r| t.get_int(r, "SpellIndex").and_then(|v| u16::try_from(v).ok()))
                    .collect()
            })
            .unwrap_or_default();
        let ability = classes
            .get(row, "SpellcastingAbil")
            .and_then(|a| ABILITIES.iter().position(|x| x.eq_ignore_ascii_case(a)))
            .map_or(0, |i| modifier(s.total(i)));
        let domains = self.table("domains").ok();
        let domain_spell = |n: usize| -> Option<u16> {
            ["Domain1", "Domain2"].iter().find_map(|d| {
                let d = pkg(d)?.parse::<usize>().ok()?;
                let v = domains.as_ref()?.get_int(d, &format!("Level_{n}"))?;
                u16::try_from(v).ok()
            })
        };
        let mut out = Vec::new();
        for n in 0..10usize {
            let of_level: Vec<u16> =
                prefs.iter().copied().filter(|&sp| spell_level(sp) == Some(n)).collect();
            if prepares {
                let base = table("SpellGainTable")
                    .and_then(|t| t.get_int(level as usize - 1, &format!("SpellLevel{n}")))
                    .unwrap_or(0)
                    .max(0);
                // Bonus slots: one for each four points of modifier from
                // the spell level up.
                let bonus = if n > 0 && base > 0 && ability >= n as i32 {
                    (ability - n as i32) / 4 + 1
                } else {
                    0
                };
                let slots = (base + bonus) as usize;
                let mut list: Vec<u16> = Vec::new();
                if n > 0
                    && let Some(d) = domain_spell(n)
                    && slots > 0
                {
                    list.push(d);
                }
                for &sp in &of_level {
                    if list.len() < slots && !list.contains(&sp) {
                        list.push(sp);
                    }
                }
                if let Some(&first) = of_level.first().or(list.first()) {
                    while list.len() < slots {
                        list.push(first);
                    }
                }
                if !list.is_empty() {
                    out.push((format!("MemorizedList{n}"), list));
                }
            } else {
                let known = table("SpellKnownTable")
                    .and_then(|t| t.get_int(level as usize - 1, &format!("SpellLevel{n}")))
                    .unwrap_or(0)
                    .max(0) as usize;
                let list: Vec<u16> = of_level.into_iter().take(known).collect();
                if !list.is_empty() {
                    out.push((format!("KnownList{n}"), list));
                }
            }
        }
        out
    }

    /// A racialtypes.2da value of the creature's race.
    fn race_value(&self, s: &State, column: &str) -> Option<i32> {
        self.table("racialtypes").ok()?.get_int(s.race, column)
    }

    /// Whether the creature may take `feat` this level (as a normal or the
    /// levelling class's bonus feat), meeting its prerequisites with the
    /// feats it had before the level (`before`).
    fn may_take(&self, s: &State, before: &[u16], slot: usize, feat: u16, pick: Pick) -> bool {
        let Ok(t) = self.table("feat") else { return false };
        let row = usize::from(feat);
        if t.get(row, "FEAT").is_none() {
            return false;
        }
        let int = |c: &str| t.get_int(row, c);
        // The class lists: list 0 general, 1 general or bonus, 2 bonus only.
        let classes = self.table("classes").ok();
        let list_of = |class: u32| -> Option<i32> {
            let name = classes.as_ref()?.get(class as usize, "FeatsTable")?.to_lowercase();
            let ft = self.table(&name).ok()?;
            (0..ft.len())
                .find(|&r| ft.get_int(r, "FeatIndex") == Some(i32::from(feat)))
                .and_then(|r| ft.get_int(r, "List"))
        };
        // Only the levelling class's list counts, whatever ALLCLASSESCANUSE
        // says (a rogue does not take Weapon Finesse, a cleric Combat
        // Casting: neither class lists them).
        let allowed = match pick {
            Pick::Normal => matches!(list_of(s.classes[slot].0), Some(0 | 1)),
            Pick::Bonus => matches!(list_of(s.classes[slot].0), Some(1 | 2)),
        };
        if !allowed {
            return false;
        }
        let had = |f: i32| before.contains(&(f as u16));
        if int("MINATTACKBONUS").is_some_and(|b| self.base_attack(&s.classes) < b) {
            return false;
        }
        for (i, c) in
            ["MINSTR", "MINDEX", "MINCON", "MININT", "MINWIS", "MINCHA"].iter().enumerate()
        {
            // ABILITIES order is Str, Dex, Con, Int, Wis, Cha.
            if int(c).is_some_and(|m| s.total(i) < m) {
                return false;
            }
        }
        if ["PREREQFEAT1", "PREREQFEAT2"].iter().any(|c| int(c).is_some_and(|f| !had(f))) {
            return false;
        }
        let ors: Vec<i32> = (0..5).filter_map(|i| int(&format!("OrReqFeat{i}"))).collect();
        if !ors.is_empty() && !ors.iter().any(|&f| had(f)) {
            return false;
        }
        for (skill, ranks) in [("REQSKILL", "ReqSkillMinRanks"), ("REQSKILL2", "ReqSkillMinRanks2")]
        {
            if let Some(k) = int(skill) {
                let have = s.skills.get(k.max(0) as usize).copied().unwrap_or(0);
                if i32::from(have) < int(ranks).unwrap_or(1) {
                    return false;
                }
            }
        }
        if let Some(min) = int("MinLevel") {
            let level = match int("MinLevelClass") {
                Some(k) => s.classes.iter().filter(|&&(c, _)| c as i32 == k).map(|&(_, l)| l).sum(),
                None => s.level(),
            };
            if (level as i32) < min {
                return false;
            }
        }
        if int("MaxLevel").is_some_and(|m| s.level() as i32 > m) {
            return false;
        }
        if int("PreReqEpic") == Some(1) && s.level() < 21 {
            return false;
        }
        if let Some(min) = int("MinFortSave") {
            let fort: i32 = s.classes.iter().map(|&(c, l)| self.class_saves(c, l)[0]).sum();
            if fort < min {
                return false;
            }
        }
        if int("MINSPELLLVL").is_some_and(|m| self.top_spell_level(&s.classes) < m) {
            return false;
        }
        true
    }

    /// The base attack bonus of a creature's classes.
    pub fn base_attack(&self, classes: &[(u32, u32)]) -> i32 {
        let Ok(t) = self.table("classes") else { return 0 };
        classes
            .iter()
            .filter(|&&(_, l)| l > 0)
            .filter_map(|&(c, l)| {
                let name = t.get(c as usize, "AttackBonusTable")?.to_lowercase();
                self.table(&name).ok()?.get_int(l as usize - 1, "BAB")
            })
            .sum()
    }

    /// The highest spell level any of the classes casts (-1 if none).
    fn top_spell_level(&self, classes: &[(u32, u32)]) -> i32 {
        let Ok(t) = self.table("classes") else { return -1 };
        classes
            .iter()
            .filter(|&&(_, l)| l > 0)
            .filter_map(|&(c, l)| {
                let name = t.get(c as usize, "SpellGainTable")?.to_lowercase();
                let g = self.table(&name).ok()?;
                (0..10).rev().find(|n| {
                    g.get_int(l as usize - 1, &format!("SpellLevel{n}")).is_some_and(|v| v > 0)
                })
            })
            .max()
            .unwrap_or(-1)
    }
}

/// Where a new class's package item goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GearPlace {
    /// Equipped in a slot (an `Equip_ItemList` struct id: its bit).
    Equip(u32),
    /// In the backpack at a place of its 10-wide grid.
    Carry(u16, u16),
}

impl GameData {
    /// The equipment a class new to the creature brings (its package's
    /// `Equip2DA`), as Aurora places it: an item goes to the first slot of
    /// its base item's `EquipableSlots` if that is free and it is a weapon,
    /// shield, armour, helmet or ammunition (baseitems.2da `Category` 1 to
    /// 8: a torch is carried) or a creature item (a dragon's hide), else
    /// into the backpack at the first place it fits, rows first. `item`
    /// reads an item blueprint.
    pub fn new_class_gear(
        &self,
        creature: &Struct,
        class: u32,
        item: &dyn Fn(mg_core::ResRef) -> Option<Struct>,
    ) -> Vec<(mg_core::ResRef, GearPlace)> {
        let Ok(base_items) = self.table("baseitems") else { return Vec::new() };
        let base_of = |s: &Struct| s.integer("BaseItem").and_then(|b| usize::try_from(b).ok());
        let size = |b: usize| {
            let n = |c: &str| base_items.get_int(b, c).unwrap_or(1).max(1) as u16;
            (n("InvSlotWidth"), n("InvSlotHeight"))
        };
        let mut used: u32 =
            creature.list("Equip_ItemList").unwrap_or(&[]).iter().fold(0, |m, e| m | e.id);
        // The backpack's cells taken.
        let mut taken: BTreeSet<(u16, u16)> = BTreeSet::new();
        let occupy = |taken: &mut BTreeSet<(u16, u16)>, x: u16, y: u16, (w, h): (u16, u16)| {
            for i in 0..w {
                for j in 0..h {
                    taken.insert((x + i, y + j));
                }
            }
        };
        for e in creature.list("ItemList").unwrap_or(&[]) {
            let b = base_of(e)
                .or_else(|| e.resref("InventoryRes").and_then(item).and_then(|i| base_of(&i)));
            let at = |l: &str| e.integer(l).unwrap_or(0).clamp(0, 255) as u16;
            if let Some(b) = b {
                occupy(&mut taken, at("Repos_PosX"), at("Repos_Posy"), size(b));
            }
        }
        let classes = self.table("classes").ok();
        let package = classes
            .as_ref()
            .and_then(|t| t.get_int(class as usize, "Package"))
            .and_then(|p| usize::try_from(p).ok());
        let table = package
            .and_then(|p| self.table("packages").ok()?.get(p, "Equip2DA").map(str::to_lowercase))
            .and_then(|n| self.table(&n).ok());
        let Some(table) = table else { return Vec::new() };
        let mut out = Vec::new();
        for r in 0..table.len() {
            let Some(res) = table
                .get(r, "Label")
                .and_then(|l| mg_core::ResRef::from_str(&l.to_lowercase()).ok())
            else {
                continue;
            };
            let Some(bp) = item(res) else { continue };
            let Some(b) = base_of(&bp) else { continue };
            let slots = base_items
                .get(b, "EquipableSlots")
                .and_then(|v| {
                    u32::from_str_radix(v.trim_start_matches("0x").trim_start_matches("0X"), 16)
                        .ok()
                })
                .unwrap_or(0);
            let first = slots.isolate_lowest_one();
            // Weapons, shields, armour, helmets, ammunition, and creature
            // items (claws, bites, hides) for their slots.
            let wearable = base_items.get_int(b, "Category").is_some_and(|c| (1..=8).contains(&c))
                || first >= 0x4000;
            if first != 0 && wearable && used & first == 0 {
                used |= first;
                out.push((res, GearPlace::Equip(first)));
                continue;
            }
            let (w, h) = size(b);
            let fits =
                |x: u16, y: u16| (0..w).all(|i| (0..h).all(|j| !taken.contains(&(x + i, y + j))));
            let place = (0..200u16)
                .flat_map(|y| (0..=10u16.saturating_sub(w)).map(move |x| (x, y)))
                .find(|&(x, y)| fits(x, y));
            if let Some((x, y)) = place {
                occupy(&mut taken, x, y, (w, h));
                out.push((res, GearPlace::Carry(x, y)));
            }
        }
        out
    }
}
