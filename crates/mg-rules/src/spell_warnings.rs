//! What Aurora warns about in a creature's assigned spells when its
//! properties close (Options › General, "Show invalid creature spell
//! assignment warning"; `docs/research/notes_creature_spells.md`). Each
//! spellcasting class, in class order, gets at most one warning, the first
//! of:
//!
//! 1. a spell list above the highest spell level the class casts at its
//!    level (its `SpellGainTable`'s `NumSpellLevels`);
//! 2. a spell level its casting ability (`SpellcastingAbil`, with the
//!    race's adjustment) is too low for: 10 + the level;
//! 3. more spells of a level than it may have: a class that prepares its
//!    spells the `SpellGainTable` slots plus a bonus slot per four points of
//!    the ability's modifier from the level up (level 0 too); one that knows
//!    them its `SpellKnownTable` count, with no ability bonus but one more
//!    at level 0 (the same rule with a modifier of 0).
//!
//! The lists checked are the ones the Spells page edits: `MemorizedList0`–`9`
//! for a class that prepares spells, `KnownList0`–`9` otherwise.

use mg_core::StrRef;
use mg_gff::Struct;

use crate::GameData;
use crate::creatures::{ABILITIES, modifier};

/// The talk table's ability names, in [`ABILITIES`] order.
const ABILITY_NAMES: [u32; 6] = [135, 133, 132, 134, 136, 131];

/// Aurora's warnings (dialog.tlk), and their English text.
const TOO_HIGH: (u32, &str) = (
    67093,
    "This creature has spells assigned to it that are too high for its current %s level. \
     Do you wish to proceed?",
);
const ABILITY: (u32, &str) = (
    67094,
    "This creature's %s class has been assigned spells that it cannot use because its %s is \
     too low. Do you wish to proceed?",
);
const TOO_MANY: (u32, &str) = (
    67095,
    "This creature's %s class can have a maximum of %d level %d spells, but has been \
     assigned %d spells. Do you wish to proceed?",
);

/// A spellcasting class's first problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellWarning {
    /// Spells above the highest level the class casts.
    TooHigh { class: u32 },
    /// Spells its casting ability (an [`ABILITIES`] index) is too low for.
    Ability { class: u32, ability: usize },
    /// More spells of a level than the class may have.
    TooMany { class: u32, level: usize, max: usize, count: usize },
}

impl GameData {
    /// The warnings for a creature's assigned spells, as Aurora gives them.
    pub fn spell_warnings(&self, creature: &Struct) -> Vec<SpellWarning> {
        let Ok(classes) = self.table("classes") else { return Vec::new() };
        let race = creature.integer("Race").unwrap_or(0).max(0) as u32;
        let racial = self.racial_adjustments(race);
        let score = |i: usize| creature.integer(ABILITIES[i]).unwrap_or(0) as i32 + racial[i];
        let mut out = Vec::new();
        for entry in creature.list("ClassList").unwrap_or(&[]) {
            let class = entry.integer("Class").unwrap_or(0).max(0) as u32;
            let row = class as usize;
            if classes.get_int(row, "SpellCaster") != Some(1) {
                continue;
            }
            let level = entry.integer("ClassLevel").unwrap_or(1).max(1) as usize;
            let prepares = classes.get_int(row, "MemorizesSpells") == Some(1);
            let prefix = if prepares { "MemorizedList" } else { "KnownList" };
            let counts: Vec<usize> = (0..10)
                .map(|l| entry.list(&format!("{prefix}{l}")).map_or(0, <[_]>::len))
                .collect();
            let table = |column: &str| {
                let t = self.table(&classes.get(row, column)?.to_lowercase()).ok()?;
                let r = (level - 1).min(t.len().checked_sub(1)?);
                Some((t, r))
            };
            let gain = table("SpellGainTable");
            // The highest level the class casts (NumSpellLevels counts
            // level 0).
            let levels = gain.as_ref().map_or(0, |(t, r)| {
                t.get_int(*r, "NumSpellLevels").map_or_else(
                    || {
                        (0..10)
                            .rev()
                            .find(|l| t.get_int(*r, &format!("SpellLevel{l}")).is_some())
                            .map_or(0, |l| l + 1)
                    },
                    |n| n.clamp(0, 10) as usize,
                )
            });
            if counts.iter().skip(levels).any(|&n| n > 0) {
                out.push(SpellWarning::TooHigh { class });
                continue;
            }
            let ability = classes
                .get(row, "SpellcastingAbil")
                .and_then(|a| ABILITIES.iter().position(|x| x.eq_ignore_ascii_case(a)))
                .unwrap_or(3);
            let have = score(ability);
            if (0..10).any(|l| counts[l] > 0 && have < 10 + l as i32) {
                out.push(SpellWarning::Ability { class, ability });
                continue;
            }
            let (base, m) =
                if prepares { (gain, modifier(have)) } else { (table("SpellKnownTable"), 0) };
            let max = |l: usize| -> usize {
                let n = base
                    .as_ref()
                    .and_then(|(t, r)| t.get_int(*r, &format!("SpellLevel{l}")))
                    .unwrap_or(0)
                    .max(0) as usize;
                let bonus = if m >= l as i32 { (m - l as i32) / 4 + 1 } else { 0 };
                n + bonus as usize
            };
            if let Some(l) = (0..10).find(|&l| counts[l] > max(l)) {
                out.push(SpellWarning::TooMany { class, level: l, max: max(l), count: counts[l] });
            }
        }
        out
    }

    /// A warning as Aurora words it (the talk table's text, else English).
    pub fn spell_warning_text(&self, warning: &SpellWarning) -> String {
        let text = |(strref, english): (u32, &str)| {
            self.string(StrRef(strref)).unwrap_or_else(|| english.to_string())
        };
        let class_name = |class: u32| {
            let classes = self.table("classes").ok();
            classes
                .as_ref()
                .and_then(|t| t.get_int(class as usize, "Name"))
                .and_then(|s| self.string(StrRef(s as u32)))
                .or_else(|| classes.as_ref()?.get(class as usize, "Label").map(str::to_string))
                .unwrap_or_else(|| format!("({class})"))
        };
        match *warning {
            SpellWarning::TooHigh { class } => fill(&text(TOO_HIGH), &[class_name(class)]),
            SpellWarning::Ability { class, ability } => {
                let name = self
                    .string(StrRef(ABILITY_NAMES[ability]))
                    .unwrap_or_else(|| ABILITIES[ability].to_string());
                fill(&text(ABILITY), &[class_name(class), name])
            }
            SpellWarning::TooMany { class, level, max, count } => fill(
                &text(TOO_MANY),
                &[class_name(class), max.to_string(), level.to_string(), count.to_string()],
            ),
        }
    }
}

/// A printf-style template (`%s`, `%d`) with its arguments in order.
fn fill(template: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut args = args.iter();
    let mut rest = template;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let spec = rest[i + 1..].chars().next();
        match spec {
            Some('s' | 'd' | 'u' | 'i') => {
                out.push_str(args.next().map_or("", String::as_str));
                rest = &rest[i + 2..];
            }
            Some('%') => {
                out.push('%');
                rest = &rest[i + 2..];
            }
            _ => {
                out.push('%');
                rest = &rest[i + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_take_their_arguments_in_order() {
        let args = ["Wizard".to_string(), "4".into(), "0".into(), "5".into()];
        assert_eq!(
            fill(TOO_MANY.1, &args),
            "This creature's Wizard class can have a maximum of 4 level 0 spells, but has been \
             assigned 5 spells. Do you wish to proceed?"
        );
        assert_eq!(fill("100%% %s, %q", &["x".into()]), "100% x, %q");
        assert_eq!(fill("%s and %s", &["one".into()]), "one and ");
    }
}
