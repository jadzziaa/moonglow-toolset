//! Creatures: the statistics the creature editor derives from a creature's
//! fields (racialtypes.2da, classes.2da and its saving throw tables,
//! appearance.2da and creaturesize.2da, and the feats the engine hard-codes).

use crate::GameData;

/// Abilities in the order the editor and the 2DAs name them.
pub const ABILITIES: [&str; 6] = ["Str", "Dex", "Con", "Int", "Wis", "Cha"];

/// racialtypes.2da adjustment columns, in [`ABILITIES`] order.
const ADJUST: [&str; 6] =
    ["StrAdjust", "DexAdjust", "ConAdjust", "IntAdjust", "WisAdjust", "ChaAdjust"];

/// Feats the engine gives fixed effects (nwscript `FEAT_*`).
pub mod feat {
    pub const GREAT_FORTITUDE: u16 = 14;
    pub const IRON_WILL: u16 = 22;
    pub const LIGHTNING_REFLEXES: u16 = 24;
    pub const TOUGHNESS: u16 = 40;
    pub const DIVINE_GRACE: u16 = 217;
    pub const LUCKY: u16 = 248;
    pub const BULLHEADED: u16 = 380;
    pub const LUCK_OF_HEROES: u16 = 382;
    pub const SNAKEBLOOD: u16 = 386;
    pub const STRONGSOUL: u16 = 388;
    pub const DARK_BLESSING: u16 = 473;
    pub const EPIC_FORTITUDE: u16 = 583;
    pub const EPIC_REFLEXES: u16 = 585;
    pub const EPIC_WILL: u16 = 695;
}

/// What each save feat the engine hard-codes adds to Fortitude, Reflex
/// and Will (measured in the game, a creature a feat:
/// `engine_saves.rs`'s `save_feats_match_the_engine`). Divine Grace and
/// Dark Blessing add the Charisma modifier instead, each its own.
const SAVE_FEATS: [(u16, [i32; 3]); 11] = [
    (feat::GREAT_FORTITUDE, [2, 0, 0]),
    (feat::LIGHTNING_REFLEXES, [0, 2, 0]),
    (feat::IRON_WILL, [0, 0, 2]),
    (feat::LUCKY, [1, 1, 1]),
    (feat::LUCK_OF_HEROES, [1, 1, 1]),
    (feat::BULLHEADED, [0, 0, 1]),
    (feat::SNAKEBLOOD, [0, 1, 0]),
    (feat::STRONGSOUL, [1, 0, 1]),
    (feat::EPIC_FORTITUDE, [4, 0, 0]),
    (feat::EPIC_REFLEXES, [0, 4, 0]),
    (feat::EPIC_WILL, [0, 0, 4]),
];

/// What the statistics depend on (a creature's fields).
#[derive(Debug, Clone, Default)]
pub struct CreatureSheet {
    pub race: u32,
    pub appearance: u32,
    /// Base scores, in [`ABILITIES`] order.
    pub abilities: [i32; 6],
    /// (class, level) per `ClassList` entry.
    pub classes: Vec<(u32, u32)>,
    pub feats: Vec<u16>,
    pub natural_ac: i32,
    /// `fortbonus`, `refbonus`, `willbonus`.
    pub save_bonus: [i32; 3],
    /// `HitPoints`: the base hit points (the rolled hit dice).
    pub hit_points: i32,
    /// `SpecAbilityList` spells.
    pub special_abilities: Vec<u16>,
    /// Each class's spells.
    pub spells: Vec<ClassSpells>,
    /// `CRAdjust`.
    pub cr_adjust: i32,
    /// What the creature's gear is worth, in gold (the challenge rating's
    /// wealth term; the caller resolves the items).
    pub gear_value: u32,
}

/// The editor's Statistics page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureStats {
    pub racial: [i32; 6],
    pub totals: [i32; 6],
    pub modifiers: [i32; 6],
    pub level: u32,
    /// Armor class: 10 + Dexterity + size + natural.
    pub ac_dex: i32,
    pub ac_size: i32,
    pub ac: i32,
    /// Fortitude, Reflex, Will: from the classes' tables.
    pub saves_base: [i32; 3],
    /// Abilities and feats.
    pub saves_modifier: [i32; 3],
    pub saves: [i32; 3],
    /// Constitution per level and Toughness.
    pub hp_bonus: i32,
    pub max_hit_points: i32,
    /// The hit points the Great Constitution feats add: the game adds them
    /// to the maximum itself, but not to the hit points a creature starts
    /// with, which it works out from `CurrentHitPoints` and the
    /// Constitution without them (measured: `engine_saves.rs`).
    pub great_hit_points: i32,
    /// What `CurrentHitPoints` holds for the creature to start with all
    /// its hit points in the game: the base hit points and
    /// [`great_hit_points`](Self::great_hit_points). (Aurora writes the
    /// base alone, and such a creature starts short of them.)
    pub current_hit_points: i32,
}

/// The levels a creature's saves come from its classes' tables for; past
/// them they are epic levels.
const EPIC_FROM: u32 = 20;

/// The points the epic Great Charisma, Constitution, Dexterity,
/// Intelligence, Wisdom and Strength feats add (feat.2da rows 764 to 823,
/// ten of each, each a point), in [`ABILITIES`] order.
fn great_abilities(feats: &[u16]) -> [i32; 6] {
    // (Strength, Dexterity, Constitution, Intelligence, Wisdom, Charisma.)
    const FIRST: [u16; 6] = [814, 784, 774, 794, 804, 764];
    FIRST.map(|first| feats.iter().filter(|f| (first..first + 10).contains(f)).count() as i32)
}

/// An ability score's modifier.
pub fn modifier(score: i32) -> i32 {
    score.saturating_sub(10).div_euclid(2)
}

/// The most levels a class entry is read as having (`ClassLevel` is a
/// SHORT).
const MOST_LEVELS: i32 = i16::MAX as i32;

impl CreatureSheet {
    /// The fields of a creature (a UTC, or a creature in an area). Each
    /// number is held to what its field holds (an ability a BYTE, a level
    /// and a save bonus a SHORT), whatever type a file gives it: an
    /// ability of 2,147,483,647 in a file some tool wrote is 255, and a
    /// feat or spell past a WORD is none (not the feat its low bits spell).
    pub fn from_gff(s: &mg_gff::Struct) -> CreatureSheet {
        let int = |label: &str| s.integer(label).unwrap_or(0);
        let short = |v: i64| crate::held(v, i32::from(i16::MIN), i32::from(i16::MAX));
        CreatureSheet {
            race: crate::row(int("Race")),
            appearance: crate::row(int("Appearance_Type")),
            abilities: ABILITIES.map(|a| crate::held(int(a), 0, 255)),
            classes: s
                .list("ClassList")
                .unwrap_or(&[])
                .iter()
                .map(|c| {
                    let n = |l: &str| c.integer(l).unwrap_or(0);
                    (crate::row(n("Class")), crate::held(n("ClassLevel"), 0, MOST_LEVELS) as u32)
                })
                .collect(),
            feats: s
                .list("FeatList")
                .unwrap_or(&[])
                .iter()
                .filter_map(|f| u16::try_from(f.integer("Feat")?).ok())
                .collect(),
            natural_ac: crate::held(int("NaturalAC"), 0, 255),
            save_bonus: ["fortbonus", "refbonus", "willbonus"].map(|l| short(int(l))),
            hit_points: short(int("HitPoints")),
            special_abilities: spell_ids(s.list("SpecAbilityList")),
            spells: s
                .list("ClassList")
                .unwrap_or(&[])
                .iter()
                .map(|c| {
                    let all = |list: &str| -> Vec<u16> {
                        (0..10).flat_map(|l| spell_ids(c.list(&format!("{list}{l}")))).collect()
                    };
                    ClassSpells {
                        class: crate::row(c.integer("Class").unwrap_or(0)),
                        known: all("KnownList"),
                        memorized: all("MemorizedList"),
                    }
                })
                .collect(),
            cr_adjust: crate::held(int("CRAdjust"), i32::MIN, i32::MAX),
            gear_value: 0,
        }
    }
}

/// A class's spells (`ClassList` entry).
#[derive(Debug, Clone, Default)]
pub struct ClassSpells {
    pub class: u32,
    /// `KnownList0`-`9`.
    pub known: Vec<u16>,
    /// `MemorizedList0`-`9`.
    pub memorized: Vec<u16>,
}

/// The `Spell` of each entry of a spell list (one past a WORD is none).
fn spell_ids(list: Option<&[mg_gff::Struct]>) -> Vec<u16> {
    let spell = |e: &mg_gff::Struct| u16::try_from(e.integer("Spell")?).ok();
    list.unwrap_or(&[]).iter().filter_map(spell).collect()
}

/// Numbers added up without wrapping: a table's cell may be any number.
fn sum(parts: impl IntoIterator<Item = i32>) -> i32 {
    parts.into_iter().fold(0, i32::saturating_add)
}

impl GameData {
    /// A race's ability adjustments, in [`ABILITIES`] order.
    pub fn racial_adjustments(&self, race: u32) -> [i32; 6] {
        let t = self.table("racialtypes").ok();
        ADJUST.map(|col| t.as_ref().and_then(|t| t.get_int(race as usize, col)).unwrap_or(0))
    }

    /// A class's base saves at a level (its SavingThrowTable), Fortitude,
    /// Reflex, Will.
    pub fn class_saves(&self, class: u32, level: u32) -> [i32; 3] {
        let table = self
            .table("classes")
            .ok()
            .and_then(|t| t.get(class as usize, "SavingThrowTable").map(str::to_lowercase));
        let Some(t) = table.and_then(|n| self.table(&n).ok()) else { return [0; 3] };
        let row = level.saturating_sub(1) as usize;
        ["FortSave", "RefSave", "WillSave"].map(|c| t.get_int(row, c).unwrap_or(0))
    }

    /// The armor class modifier of an appearance's size (creaturesize.2da
    /// `ACATTACKMOD` of appearance.2da `SIZECATEGORY`).
    pub fn size_modifier(&self, appearance: u32) -> i32 {
        let size = self
            .table("appearance")
            .ok()
            .and_then(|t| t.get_int(appearance as usize, "SIZECATEGORY"));
        size.and_then(|s| {
            self.table("creaturesize").ok()?.get_int(s.max(0) as usize, "ACATTACKMOD")
        })
        .unwrap_or(0)
    }

    /// The base saves (Fortitude, Reflex, Will) of a creature of `classes`
    /// (class, level), as the game has them: the classes' tables for the
    /// first 20 levels, in the classes' order; past them every two levels
    /// are a point on each save, whatever the classes (measured in the
    /// game: `engine_saves.rs`; Aurora adds each class's table at its full
    /// level, nwn-issues 816). The one working of them: the Statistics
    /// page's, and what a feat's `MinFortSave` is held against.
    pub fn base_saves(&self, classes: &[(u32, u32)]) -> [i32; 3] {
        let mut saves = [0; 3];
        let mut left = EPIC_FROM;
        for &(class, lvl) in classes {
            let counted = lvl.min(left);
            left -= counted;
            if counted > 0 {
                let s = self.class_saves(class, counted);
                for i in 0..3 {
                    saves[i] = sum([saves[i], s[i]]);
                }
            }
        }
        let level = classes.iter().fold(0u32, |all, &(_, l)| all.saturating_add(l));
        let epic = i32::try_from(level.saturating_sub(EPIC_FROM) / 2).unwrap_or(i32::MAX);
        saves.map(|s| sum([s, epic]))
    }

    /// The Statistics page's numbers. (A sheet's numbers and the tables'
    /// are added up without wrapping: [`sum`].)
    pub fn creature_stats(&self, c: &CreatureSheet) -> CreatureStats {
        let racial = self.racial_adjustments(c.race);
        // (The epic Great Strength… feats, each a point: the game counts
        // them into the score itself, `engine_saves.rs`.)
        let great = great_abilities(&c.feats);
        let totals: [i32; 6] = std::array::from_fn(|i| sum([c.abilities[i], racial[i], great[i]]));
        let modifiers = totals.map(modifier);
        let level = c.classes.iter().fold(0u32, |all, &(_, l)| all.saturating_add(l));
        let has = |f: u16| c.feats.contains(&f);
        let saves_base = self.base_saves(&c.classes);
        let levels = i32::try_from(level).unwrap_or(i32::MAX);
        // Constitution, Dexterity, Wisdom; the save feats ([`SAVE_FEATS`]);
        // and the Charisma modifier for Divine Grace, and again for Dark
        // Blessing (a paladin turned blackguard has both).
        let mut saves_modifier = [modifiers[2], modifiers[1], modifiers[4]];
        for (_, adds) in SAVE_FEATS.iter().filter(|(f, _)| has(*f)) {
            for i in 0..3 {
                saves_modifier[i] = sum([saves_modifier[i], adds[i]]);
            }
        }
        let charisma = modifiers[5].max(0);
        let graces = i32::from(has(feat::DIVINE_GRACE)) + i32::from(has(feat::DARK_BLESSING));
        for m in &mut saves_modifier {
            *m = sum([*m, charisma.saturating_mul(graces)]);
        }
        let saves: [i32; 3] =
            std::array::from_fn(|i| sum([saves_base[i], saves_modifier[i], c.save_bonus[i]]));

        let ac_dex = modifiers[1];
        let ac_size = self.size_modifier(c.appearance);
        let ac = sum([10, ac_dex, ac_size, c.natural_ac]);

        // Epic Toughness is the engine's to add: the toolset's maximum
        // leaves it out.
        let toughness = if has(feat::TOUGHNESS) { levels } else { 0 };
        // (Nor does it count the Great Constitution feats: the stored
        // maximum is Aurora's, by the Constitution the blueprint has.)
        let con = sum([c.abilities[2], racial[2]]);
        let hp_bonus = sum([modifier(con).saturating_mul(levels), toughness]);
        let with_great = modifier(sum([con, great[2]]));
        let great_hit_points = with_great.saturating_sub(modifier(con)).saturating_mul(levels);
        CreatureStats {
            racial,
            totals,
            modifiers,
            level,
            ac_dex,
            ac_size,
            ac,
            saves_base,
            saves_modifier,
            saves,
            hp_bonus,
            max_hit_points: sum([c.hit_points, hp_bonus]).max(1),
            great_hit_points,
            current_hit_points: sum([c.hit_points, great_hit_points]),
        }
    }
}

#[cfg(test)]
mod tests {
    use mg_gff::{Struct, Value};

    use super::*;

    const TABLES: [(&str, &str); 6] = [
        ("racialtypes", "Label StrAdjust ConAdjust CRModifier\n0 Odd 2147483647 -2147483648 1\n"),
        ("classes", "Label HitDie SavingThrowTable\n0 Fighter 2147483647 cls_savthr_a\n"),
        ("cls_savthr_a", "FortSave RefSave WillSave\n0 2147483647 2147483647 -2147483648\n"),
        ("appearance", "LABEL SIZECATEGORY MOVERATE\n0 Odd 0 NORM\n"),
        ("creaturesize", "LABEL ACATTACKMOD\n0 Odd 2147483647\n"),
        // (The PC's row has no walk rate.)
        (
            "creaturespeed",
            "Label 2DAName WALKRATE\n0 PC_Movement PLAYER ****\n1 Normal NORM 1.75\n",
        ),
    ];

    /// A creature whose every number is the largest a file can hold, in
    /// tables whose cells are the largest a table can: a sheet held to
    /// its fields, statistics added up without wrapping (a test build
    /// panicked, a release showed wrapped numbers), and a challenge rating
    /// that is a number, with a PC's walk rate the table does not give.
    #[test]
    fn numbers_at_their_ends_wrap_nothing() {
        let game = crate::test_game(&TABLES);
        let mut c = Struct::new(0);
        for label in ABILITIES.into_iter().chain(["NaturalAC", "HitPoints", "CRAdjust"]) {
            c.set(label, Value::Int64(i64::MAX));
        }
        for label in ["fortbonus", "refbonus", "willbonus"] {
            c.set(label, Value::Int64(i64::MIN));
        }
        let mut class = Struct::new(2);
        class.set("Class", Value::Int(0));
        class.set("ClassLevel", Value::Int64(i64::MAX));
        c.set("ClassList", Value::List(vec![class.clone(), class]));
        let feat = |f: i64| {
            let mut s = Struct::new(1);
            s.set("Feat", Value::Int64(f));
            s
        };
        c.set("FeatList", Value::List(vec![feat(40), feat(70_000 + 40), feat(-1)]));
        let sheet = CreatureSheet::from_gff(&c);
        assert_eq!(sheet.abilities, [255; 6]);
        assert_eq!(sheet.classes, [(0, 32_767), (0, 32_767)]);
        assert_eq!(sheet.feats, [40], "a feat past a WORD is none, not the one its low bits spell");
        assert_eq!((sheet.natural_ac, sheet.hit_points), (255, 32_767));
        assert_eq!(sheet.save_bonus, [-32_768; 3]);
        let stats = game.creature_stats(&sheet);
        assert_eq!(stats.level, 65_534);
        assert_eq!(stats.totals[0], i32::MAX);
        assert_eq!(stats.saves_base[0], 32_757, "the epic levels' points alone");
        assert_eq!(game.base_saves(&[(0, 1), (0, 1)]), [i32::MAX, i32::MAX, i32::MIN]);
        assert_eq!(stats.ac, i32::MAX);
        assert!(stats.max_hit_points >= 1);
        let cr = game.challenge(&sheet);
        assert!(cr.calculated.is_finite() && cr.rating.is_finite(), "{cr:?}");
        // The same creature at a level a creature has: its hit points'
        // term left out for the walk rate the PC's row lacks, not
        // infinite.
        let mut class = Struct::new(2);
        class.set("Class", Value::Int(0));
        class.set("ClassLevel", Value::Short(5));
        c.set("ClassList", Value::List(vec![class]));
        c.set("CRAdjust", Value::Int(1));
        let cr = game.challenge(&CreatureSheet::from_gff(&c));
        assert!(cr.calculated.is_finite() && cr.rating.is_finite(), "{cr:?}");
        assert!(cr.rating < 1.0e6, "{cr:?}");
    }

    /// The base saves are one working, the game's: the classes' tables
    /// for the first 20 levels in the classes' order, then a point every
    /// two levels.
    #[test]
    fn base_saves_are_the_first_twenty_levels_and_a_point_every_two_after() {
        let mut rows = String::from("FortSave RefSave WillSave\n");
        for level in 1..=40 {
            rows += &format!("{} {level} 0 0\n", level - 1);
        }
        let game = crate::test_game(&[
            ("classes", "Label SavingThrowTable\n0 A cls_savthr_a\n1 B cls_savthr_a\n"),
            ("cls_savthr_a", &rows),
        ]);
        assert_eq!(game.base_saves(&[(0, 10)]), [10, 0, 0]);
        assert_eq!(game.base_saves(&[(0, 10), (1, 5)]), [15, 0, 0]);
        // (20 of the first class, then ten epic levels: five points.)
        assert_eq!(game.base_saves(&[(0, 30)]), [25, 5, 5]);
        assert_eq!(game.base_saves(&[(0, 15), (1, 15)]), [15 + 5 + 5, 5, 5]);
        let sheet = CreatureSheet { classes: vec![(0, 15), (1, 15)], ..Default::default() };
        assert_eq!(game.creature_stats(&sheet).saves_base, game.base_saves(&sheet.classes));
    }
}
