//! Levelling a creature up by its classes' packages, as Aurora's Levelup
//! Wizard does (captured: `levelup/bandit-fighter5.mod`, `six-after.mod`).
//! Each level, in class slot order:
//!
//! - hit points: the class's average, (HitDie + 1) / 2, the total rounded
//!   down at the end (Fighter 1 to 10: + 49);
//! - every fourth character level, one point to the package's ability
//!   (packages.2da `Attribute`);
//! - skill points (classes.2da `SkillPointBase` + Intelligence modifier +
//!   the race's `ExtraSkillPointsPerLevel`), spent one rank at a time down
//!   the package's skill list (`SkillPref2DA`): a class skill costs 1 and
//!   goes up to the character level + 3, a cross-class skill 2 and half
//!   that; one rank per skill per level, and what is left is kept;
//! - feats: a cleric's domain powers (the package's domains) at its first
//!   level, the class's own (`FeatsTable` list 3 at that level, replacing
//!   the feat they succeed), a normal feat every third character level
//!   (the race's `NormalFeatEveryNthLevel`) and a bonus feat where the
//!   class's `BonusFeatsTable` has one, each the first of the package's
//!   feats (`FeatPref2DA`) the levelling class lists (its `FeatsTable`:
//!   list 0 or 1 for a normal feat, 1 or 2 for a bonus one) and whose
//!   prerequisites the creature meets, feats taken the same level not
//!   counting as prerequisites.
//!
//! Spells and the package's equipment for a new class are not given yet.

use std::collections::BTreeSet;

use mg_gff::{Struct, Value};

use crate::GameData;
use crate::creatures::{ABILITIES, modifier};

/// Struct ids of a creature's lists, as Aurora writes them.
const FEAT_ID: u32 = 1;
const CLASS_ID: u32 = 2;

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
        if level % 4 == 0
            && let Some(a) = pkg("Attribute")
            && let Some(i) = ABILITIES.iter().position(|x| x.eq_ignore_ascii_case(&a))
        {
            s.abilities[i] += 1;
        }

        // Skills, one rank each down the package's list.
        let extra = self.race_value(s, "ExtraSkillPointsPerLevel").unwrap_or(0);
        let mut points = *spare as i32
            + (num("SkillPointBase").unwrap_or(0) + modifier(s.total(3)) + extra).max(1);
        let skill_table = col("SkillsTable").and_then(|t| self.table(&t.to_lowercase()).ok());
        let class_skill = |skill: u32| -> Option<bool> {
            let t = skill_table.as_ref()?;
            (0..t.len())
                .find(|&r| t.get_int(r, "SkillIndex") == Some(skill as i32))
                .map(|r| t.get_int(r, "ClassSkill") == Some(1))
        };
        let skills_2da = self.table("skills").ok();
        if let Some(t) = pkg("SkillPref2DA").and_then(|n| self.table(&n.to_lowercase()).ok()) {
            for r in 0..t.len() {
                let Some(skill) = t.get_int(r, "SkillIndex").and_then(|v| u32::try_from(v).ok())
                else {
                    continue;
                };
                let usable = skills_2da
                    .as_ref()
                    .and_then(|k| k.get_int(skill as usize, "AllClassesCanUse"))
                    .is_some_and(|v| v == 1);
                let own = class_skill(skill).unwrap_or(false);
                if !own && !usable {
                    continue;
                }
                let (cost, max) = if own { (1, level + 3) } else { (2, (level + 3) / 2) };
                let Some(rank) = s.skills.get_mut(skill as usize) else { continue };
                if u32::from(*rank) < max && points >= cost {
                    *rank += 1;
                    points -= cost;
                }
            }
        }
        *spare = points.max(0) as u32;

        // Feats: domains, the class's own, then the picks.
        let before = s.feats.clone();
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
        let every = self.race_value(s, "NormalFeatEveryNthLevel").unwrap_or(3).max(1) as u32;
        let normal = if level == 1 || level % every == 0 {
            self.race_value(s, "NumberNormalFeatsEveryNthLevel").unwrap_or(1).max(0) as u32
        } else {
            0
        };
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
        (die + 1.0) / 2.0
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
                (0..10)
                    .rev()
                    .find(|n| {
                        g.get_int(l as usize - 1, &format!("SpellLevel{n}")).is_some_and(|v| v > 0)
                    })
                    .map(|n| n as i32)
            })
            .max()
            .unwrap_or(-1)
    }
}
