//! Challenge rating, as Aurora calculates it (when a creature's properties
//! open, and in Build › Compile › Creature CR): a sum of terms, most of them
//! scaled by the creature's level L, then the race's `CRModifier`, rounding,
//! `CRAdjust`, and fractions (fractionalcr.2da) below 1. The terms, with
//! their weights as Aurora has them:
//!
//! | term | value |
//! |---|---|
//! | level | 0.15 L |
//! | natural AC | 0.1 × NaturalAC |
//! | gear | 0.2 L × value / (20000 L + 100000) |
//! | hit points | 0.2 L × HitPoints / expected × walk rate / the PC's walk rate |
//! | abilities | 0.1 L × (sum of the six) / (L + 50) |
//! | special abilities | 0.15 L × (sum of spell levels) / (L² + 6 L) |
//! | spells | 0.15 L × (sum of spell levels) / (L² + L) |
//! | saves | 0.15 L × (base + bonuses) / base |
//! | feats | 0.1 L × (sum of feat.2da CRValue) / (L/2 + 7) |
//!
//! "Expected" hit points are the average of the classes' hit dice
//! (level × (HitDie + 1) / 2 per class, truncated as it adds up); the walk
//! rate is creaturespeed.2da's for the appearance's `MOVERATE` (whatever the
//! creature's `MovementRate`). Spell levels are spells.2da `Innate` (0
//! counts nothing); the spells are those a class casts from (`MemorizedList`
//! for classes that memorize, else `KnownList`) and the special abilities
//! again; only equipped gear counts; saves are the classes' saving throw tables
//! at their levels (1 if they add up to 0) plus `fortbonus`, `refbonus`
//! and `willbonus`; CRValue is read as an integer (so 0.5 counts 0) and a
//! feat without one counts 1. Aurora keeps the running sum in single
//! precision, as here. The full derivation is in
//! `docs/research/notes_creature_cr.md`.

use crate::{CreatureSheet, GameData};

/// A creature's challenge rating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Challenge {
    /// The rating before rounding and `CRAdjust` (shifted down a little
    /// below 1.5, as Aurora does before it rounds).
    pub calculated: f32,
    /// What Aurora stores as `ChallengeRating`: the rounded rating plus
    /// `CRAdjust`, or below 1 a fraction (1/2, 1/3, 1/4, 1/6, 1/8).
    pub rating: f32,
}

impl Challenge {
    /// The rating as the toolset writes it: a whole number or "1/2" etc.
    pub fn text(&self) -> String {
        if self.rating >= 1.0 || self.rating <= 0.0 {
            format!("{}", self.rating.round() as i64)
        } else {
            format!("1/{}", (1.0 / self.rating).round() as i64)
        }
    }
}

/// Single-precision weights, as Aurora stores them.
const W15: f64 = 0.15f32 as f64;
const W10: f64 = 0.1f32 as f64;
const W20: f64 = 0.2f32 as f64;

impl GameData {
    /// The challenge rating of a creature, as Aurora calculates it.
    pub fn challenge(&self, c: &CreatureSheet) -> Challenge {
        let classes: Vec<(u32, u32)> =
            c.classes.iter().take(8).copied().filter(|&(_, l)| l > 0).collect();
        let level: u32 = classes.iter().map(|&(_, l)| l).sum();
        if level == 0 {
            let rating = self.fraction(0.0);
            return Challenge { calculated: 0.0, rating };
        }
        let l = f64::from(level);
        let table = |name: &str| self.table(name).ok();
        // The running sum, stored as a float after every term.
        let mut cr: f32 = 0.0;
        let mut add = |term: f64| cr = (f64::from(cr) + f64::from(term as f32)) as f32;

        add(l * W15);
        add(f64::from(c.natural_ac.clamp(0, 255)) * W10);
        add(f64::from(c.gear_value) / (l * 20000.0 + 100000.0) * W20 * l);

        // Hit points against the classes' average, times speed.
        let classes_2da = table("classes");
        let mut expected: i64 = 0;
        for &(class, lvl) in &classes {
            let die = classes_2da
                .as_ref()
                .and_then(|t| t.get_int(class as usize, "HitDie"))
                .unwrap_or(0)
                .max(0);
            let sum = expected as f64 + f64::from(lvl * (die as u32 + 1)) / 2.0;
            expected = sum.trunc() as i64;
        }
        if expected > 0 {
            let hp = f64::from(c.hit_points as i16);
            let term = (hp / expected as f64 * W20 * l) as f32;
            let (walk, pc) = self.walk_rates(c);
            add(f64::from(term) * f64::from(walk) / f64::from(pc));
        }

        let abilities: i32 = c.abilities.iter().map(|&a| a.clamp(0, 255)).sum();
        add(f64::from(abilities) / (l + 50.0) * W10 * l);

        let spells_2da = table("spells");
        let innate = |s: u16| {
            spells_2da.as_ref().and_then(|t| t.get_int(usize::from(s), "Innate")).unwrap_or(0)
        };
        let specials: i32 = c.special_abilities.iter().map(|&s| innate(s)).sum();
        if specials > 0 {
            add(f64::from(specials) / (l * (l + 1.0) + l * 5.0) * W15 * l);
        }
        // The spells a class casts from (memorized for the classes that
        // memorize, known for the others), and the special abilities again.
        let memorizes = |class: u32| {
            classes_2da.as_ref().and_then(|t| t.get_int(class as usize, "MemorizesSpells"))
                == Some(1)
        };
        let cast = c
            .spells
            .iter()
            .flat_map(|s| if memorizes(s.class) { s.memorized.iter() } else { s.known.iter() });
        let spells: i32 = c.special_abilities.iter().chain(cast).map(|&s| innate(s).max(0)).sum();
        if spells > 0 {
            add(f64::from(spells) / (l * (l + 1.0)) * W15 * l);
        }

        let mut base: i32 = classes.iter().flat_map(|&(cl, lv)| self.class_saves(cl, lv)).sum();
        if base == 0 {
            base = 1;
        }
        let bonus: i32 = c.save_bonus.iter().map(|&b| i32::from(b as i16)).sum();
        add(f64::from(base + bonus) / f64::from(base) * W15 * l);

        let feat_2da = table("feat");
        let feats: i32 = c
            .feats
            .iter()
            .map(|&f| {
                feat_2da.as_ref().and_then(|t| t.get_int(usize::from(f), "CRValue")).unwrap_or(1)
            })
            .sum();
        add(f64::from(feats) / (l * 0.5 + 7.0) * W10 * l);

        if let Some(m) =
            table("racialtypes").and_then(|t| t.get_float((c.race & 0xff) as usize, "CRModifier"))
        {
            cr = (f64::from(m) * f64::from(cr)) as f32;
        }

        // Rounding: above 0.75 to the nearest whole number (halves down),
        // a quarter less below 1.5; at most 0.75, 0.35 less, and fractions.
        let mut value = cr;
        if value > 0.75 {
            if value < 1.5 {
                value = (f64::from(value) - 0.25) as f32;
            }
        } else {
            value = (f64::from(value) - 0.35) as f32;
        }
        let whole = f64::from(value).trunc();
        let rounded = if f64::from(value) - whole > 0.5 { whole + 1.0 } else { whole } as i64;
        let calculated = value;
        let adjusted = (f64::from(value) + f64::from(c.cr_adjust)) as f32;
        let rating = if adjusted > 0.75 {
            (rounded + i64::from(c.cr_adjust)) as f32
        } else {
            self.fraction(adjusted)
        };
        Challenge { calculated, rating }
    }

    /// The creature's walk rate and the PC's (creaturespeed.2da row 0).
    fn walk_rates(&self, c: &CreatureSheet) -> (f32, f32) {
        let Ok(speed) = self.table("creaturespeed") else { return (0.0, 1.0) };
        let walk = |row: usize| speed.get_float(row, "WALKRATE");
        let pc = walk(0).unwrap_or(0.0);
        // Aurora's calculation reads the creaturespeed.2da row of its
        // movement rate and, for the Default row, the appearance's; but
        // the rate it reads is always Default: every `MovementRate` rates
        // the same (`creature-cr-terms.mod`), only the appearance counts.
        let moverate = self
            .table("appearance")
            .ok()
            .and_then(|t| t.get(c.appearance as usize, "MOVERATE").map(str::to_owned));
        let rate = (1..speed.len())
            .find(|&r| moverate.is_some() && speed.get(r, "2DAName") == moverate.as_deref())
            .and_then(walk);
        (rate.unwrap_or(0.0), pc)
    }

    /// A rating below 1 as a fraction: the first fractionalcr.2da row whose
    /// `Min` it reaches gives 1 / `Denominator`.
    fn fraction(&self, value: f32) -> f32 {
        if value >= 1.0 {
            return value;
        }
        let Ok(t) = self.table("fractionalcr") else { return 0.125 };
        let mut denominator = 8;
        for row in 0..t.len() {
            let min = t.get_float(row, "Min").unwrap_or(f32::MAX);
            denominator = t.get_int(row, "Denominator").unwrap_or(denominator);
            if value >= min {
                break;
            }
        }
        (1.0 / f64::from(denominator.max(1))) as f32
    }
}

impl GameData {
    /// What a creature's equipped items are worth, in gold (the challenge
    /// rating's gear term). A blueprint names its items (`EquippedRes`,
    /// read with `item`); a placed creature carries them.
    pub fn gear_value(
        &self,
        creature: &mg_gff::Struct,
        item: &dyn Fn(mg_core::ResRef) -> Option<mg_gff::Struct>,
    ) -> u32 {
        let mut total: u32 = 0;
        for entry in creature.list("Equip_ItemList").unwrap_or(&[]) {
            let value = if entry.get("BaseItem").is_some() {
                Some(self.item_cost(&crate::ItemValue::from_gff(entry)))
            } else {
                entry
                    .resref("EquippedRes")
                    .and_then(item)
                    .map(|i| self.item_cost(&crate::ItemValue::from_gff(&i)))
            };
            total = total.saturating_add(value.unwrap_or(0));
        }
        total
    }
}
