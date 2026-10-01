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
    pub const LUCK_OF_HEROES: u16 = 382;
}

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
}

/// An ability score's modifier.
pub fn modifier(score: i32) -> i32 {
    (score - 10).div_euclid(2)
}

impl CreatureSheet {
    /// The fields of a creature (a UTC, or a creature in an area).
    pub fn from_gff(s: &mg_gff::Struct) -> CreatureSheet {
        let int = |label: &str| s.integer(label).unwrap_or(0);
        CreatureSheet {
            race: int("Race").max(0) as u32,
            appearance: int("Appearance_Type").max(0) as u32,
            abilities: ABILITIES.map(|a| int(a) as i32),
            classes: s
                .list("ClassList")
                .unwrap_or(&[])
                .iter()
                .map(|c| {
                    let n = |l: &str| c.integer(l).unwrap_or(0).max(0) as u32;
                    (n("Class"), n("ClassLevel"))
                })
                .collect(),
            feats: s
                .list("FeatList")
                .unwrap_or(&[])
                .iter()
                .filter_map(|f| f.integer("Feat"))
                .map(|f| f as u16)
                .collect(),
            natural_ac: int("NaturalAC") as i32,
            save_bonus: ["fortbonus", "refbonus", "willbonus"].map(|l| int(l) as i32),
            hit_points: int("HitPoints") as i32,
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
                        class: c.integer("Class").unwrap_or(0).max(0) as u32,
                        known: all("KnownList"),
                        memorized: all("MemorizedList"),
                    }
                })
                .collect(),
            cr_adjust: int("CRAdjust") as i32,
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

/// The `Spell` of each entry of a spell list.
fn spell_ids(list: Option<&[mg_gff::Struct]>) -> Vec<u16> {
    list.unwrap_or(&[]).iter().filter_map(|e| e.integer("Spell")).map(|s| s as u16).collect()
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

    /// The Statistics page's numbers.
    pub fn creature_stats(&self, c: &CreatureSheet) -> CreatureStats {
        let racial = self.racial_adjustments(c.race);
        let totals: [i32; 6] = std::array::from_fn(|i| c.abilities[i] + racial[i]);
        let modifiers = totals.map(modifier);
        let level: u32 = c.classes.iter().map(|&(_, l)| l).sum();
        let has = |f: u16| c.feats.contains(&f);

        let mut saves_base = [0; 3];
        for &(class, lvl) in &c.classes {
            let s = self.class_saves(class, lvl);
            for i in 0..3 {
                saves_base[i] += s[i];
            }
        }
        // Constitution, Dexterity, Wisdom; the save feats; luck.
        let mut saves_modifier = [modifiers[2], modifiers[1], modifiers[4]];
        for (i, f) in [feat::GREAT_FORTITUDE, feat::LIGHTNING_REFLEXES, feat::IRON_WILL]
            .into_iter()
            .enumerate()
        {
            if has(f) {
                saves_modifier[i] += 2;
            }
        }
        let luck = i32::from(has(feat::LUCKY)) + i32::from(has(feat::LUCK_OF_HEROES));
        let grace = if has(feat::DIVINE_GRACE) { modifiers[5].max(0) } else { 0 };
        for m in &mut saves_modifier {
            *m += luck + grace;
        }
        let saves: [i32; 3] =
            std::array::from_fn(|i| saves_base[i] + saves_modifier[i] + c.save_bonus[i]);

        let ac_dex = modifiers[1];
        let ac_size = self.size_modifier(c.appearance);
        let ac = 10 + ac_dex + ac_size + c.natural_ac;

        let levels = level as i32;
        // Epic Toughness is the engine's to add: the toolset's maximum
        // leaves it out.
        let toughness = if has(feat::TOUGHNESS) { levels } else { 0 };
        let hp_bonus = modifiers[2] * levels + toughness;
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
            max_hit_points: (c.hit_points + hp_bonus).max(1),
        }
    }
}
