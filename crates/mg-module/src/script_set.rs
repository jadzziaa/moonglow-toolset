//! Script sets: the event scripts of a creature, door, placeable, trigger,
//! encounter or area saved to and loaded from an `.ini` file (Aurora's Load
//! Script Set and Save Script Set). The game ships some in `data/scr`
//! (`default_ai.ini`):
//!
//! ```ini
//! [ResRefs]
//! OnBlocked=x2_def_onblocked
//! OnDamaged=x2_def_ondamage
//! ```
//!
//! Keys are Aurora's event names (`OnSpellCast`, `OnCombatRoundEnd`, ...),
//! written in its order ([`KEYS`]).
//!
//! Class spell lists (the creature Spells page's Save and Load Class Spell
//! List) are `.ini` files too, one section per class (`[Spells10]` for the
//! wizard, classes.2da row 10), as the game's `wizard_melee_20.ini`:
//!
//! ```ini
//! [Spells10]
//! Spell000=58,0,4
//! Count=8
//! Spell001=184,1,0
//! ```
//!
//! Each entry is spell (spells.2da row), flags and metamagic; `Count`
//! comes after the first, as Aurora writes it (it updates the count after
//! every spell).

use std::collections::HashMap;

/// The section that holds the scripts.
pub const SECTION: &str = "ResRefs";

/// Every event a script set names, in the order Aurora writes them.
pub const KEYS: [&str; 23] = [
    "OnBlocked",
    "OnClick",
    "OnClose",
    "OnDamaged",
    "OnDeath",
    "OnConversation",
    "OnDisturbed",
    "OnEnter",
    "OnExhausted",
    "OnExit",
    "OnFailToOpen",
    "OnCombatRoundEnd",
    "OnHeartbeat",
    "OnLock",
    "OnOpen",
    "OnPhysicalAttacked",
    "OnPerception",
    "OnRested",
    "OnSpawn",
    "OnSpellCast",
    "OnUnlock",
    "OnUse",
    "OnUserDefined",
];

/// The script set key of an event field (of a creature, door, placeable,
/// trigger, encounter or area); `None` for events script sets leave out
/// (a trap's).
pub fn key(field: &str) -> Option<&'static str> {
    Some(match field {
        "ScriptOnBlocked" => "OnBlocked",
        "OnClick" => "OnClick",
        "OnClosed" => "OnClose",
        "OnDamaged" | "ScriptDamaged" => "OnDamaged",
        "OnDeath" | "ScriptDeath" => "OnDeath",
        "ScriptDialogue" => "OnConversation",
        "OnInvDisturbed" | "ScriptDisturbed" => "OnDisturbed",
        "OnEnter" | "OnEntered" | "ScriptOnEnter" => "OnEnter",
        "OnExhausted" => "OnExhausted",
        "OnExit" | "ScriptOnExit" => "OnExit",
        "OnFailToOpen" => "OnFailToOpen",
        "ScriptEndRound" => "OnCombatRoundEnd",
        "OnHeartbeat" | "ScriptHeartbeat" => "OnHeartbeat",
        "OnLock" => "OnLock",
        "OnOpen" => "OnOpen",
        "OnMeleeAttacked" | "ScriptAttacked" => "OnPhysicalAttacked",
        "ScriptOnNotice" => "OnPerception",
        "ScriptRested" => "OnRested",
        "ScriptSpawn" => "OnSpawn",
        "OnSpellCastAt" | "ScriptSpellAt" => "OnSpellCast",
        "OnUnlock" => "OnUnlock",
        "OnUsed" => "OnUse",
        "OnUserDefined" | "ScriptUserDefine" => "OnUserDefined",
        _ => return None,
    })
}

/// The scripts of a script set file, by key (keys match without regard to
/// case, as Windows reads `.ini` files).
pub fn read(text: &str) -> HashMap<&'static str, String> {
    let mut out = HashMap::new();
    let mut in_section = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_section = name.trim().eq_ignore_ascii_case(SECTION);
            continue;
        }
        if !in_section || line.starts_with(';') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        if let Some(key) = KEYS.iter().find(|key| key.eq_ignore_ascii_case(k.trim())) {
            out.entry(*key).or_insert_with(|| v.trim().to_string());
        }
    }
    out
}

/// A script set file of `scripts` (key, script), in Aurora's order, with
/// CRLF line ends.
pub fn write(scripts: &[(&str, &str)]) -> String {
    let mut out = format!("[{SECTION}]\r\n");
    for key in KEYS {
        if let Some((_, script)) = scripts.iter().find(|(k, _)| *k == key) {
            out.push_str(&format!("{key}={script}\r\n"));
        }
    }
    out
}

/// An entry of a class spell list: what a `MemorizedList` or `KnownList`
/// entry holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListedSpell {
    pub spell: u16,
    pub flags: u8,
    pub metamagic: u8,
}

/// The spell list section of a class.
fn spells_section(class: u32) -> String {
    format!("Spells{class}")
}

/// The spells a class spell list file holds for `class`; `None` if it has
/// no list for that class.
pub fn read_spells(text: &str, class: u32) -> Option<Vec<ListedSpell>> {
    let section = spells_section(class);
    let mut entries: HashMap<String, String> = HashMap::new();
    let mut in_section = false;
    let mut found = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_section = name.trim().eq_ignore_ascii_case(&section);
            found |= in_section;
            continue;
        }
        if in_section && let Some((k, v)) = line.split_once('=') {
            entries.entry(k.trim().to_ascii_lowercase()).or_insert_with(|| v.trim().to_string());
        }
    }
    if !found {
        return None;
    }
    let count = entries.get("count").and_then(|c| c.parse::<usize>().ok());
    let mut out = Vec::new();
    for i in 0..count.unwrap_or(usize::MAX) {
        let Some(v) = entries.get(&format!("spell{i:03}")) else { break };
        let mut parts = v.split(',').map(|p| p.trim().parse::<u32>().ok());
        let mut next = || parts.next().flatten().unwrap_or(0);
        let (spell, flags, metamagic) = (next(), next(), next());
        out.push(ListedSpell {
            spell: spell.min(u32::from(u16::MAX)) as u16,
            flags: flags.min(255) as u8,
            metamagic: metamagic.min(255) as u8,
        });
    }
    Some(out)
}

/// A class spell list file, as Aurora writes it.
pub fn write_spells(class: u32, spells: &[ListedSpell]) -> String {
    let mut out = format!("[{}]\r\n", spells_section(class));
    for (i, s) in spells.iter().enumerate() {
        out.push_str(&format!("Spell{i:03}={},{},{}\r\n", s.spell, s.flags, s.metamagic));
        if i == 0 {
            out.push_str(&format!("Count={}\r\n", spells.len()));
        }
    }
    if spells.is_empty() {
        out.push_str("Count=0\r\n");
    }
    out
}

/// The spell levels metamagic adds (Empower 2, Extend 1, Maximize 3,
/// Quicken 4, Silent 1, Still 1), for the slot a prepared spell takes.
pub fn metamagic_levels(metamagic: u8) -> u32 {
    [(1, 2), (2, 1), (4, 3), (8, 4), (16, 1), (32, 1)]
        .iter()
        .filter(|(bit, _)| metamagic & bit != 0)
        .map(|(_, levels)| levels)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_the_shipped_layout() {
        let text = "[ResRefs]\r\nOnBlocked=x2_def_onblocked\r\nonspawn = x2_def_spawn\r\n\
                    [Other]\r\nOnDeath=elsewhere\r\n";
        let set = read(text);
        assert_eq!(set.get("OnBlocked").map(String::as_str), Some("x2_def_onblocked"));
        assert_eq!(set.get("OnSpawn").map(String::as_str), Some("x2_def_spawn"));
        assert!(!set.contains_key("OnDeath"), "only [ResRefs]");
        // Written in Aurora's order, whatever the order given.
        let out = write(&[("OnSpawn", "b"), ("OnBlocked", "a"), ("OnDamaged", "")]);
        assert_eq!(out, "[ResRefs]\r\nOnBlocked=a\r\nOnDamaged=\r\nOnSpawn=b\r\n");
        assert_eq!(key("ScriptSpellAt"), Some("OnSpellCast"));
        assert_eq!(key("OnTrapTriggered"), None);
    }

    /// The game's own script sets (`data/scr`) name only events Aurora
    /// knows.
    #[test]
    fn reads_the_game_s_script_sets() {
        let root = mg_testkit::corpus!();
        let mut n = 0;
        for entry in std::fs::read_dir(root.join("data/scr")).into_iter().flatten().flatten() {
            let text = String::from_utf8_lossy(&std::fs::read(entry.path()).unwrap()).into_owned();
            if !text.starts_with("[ResRefs]") {
                continue;
            }
            let lines = text.lines().filter(|l| l.contains('=')).count();
            assert_eq!(read(&text).len(), lines, "{}", entry.path().display());
            n += 1;
        }
        assert!(n >= 3, "default_ai.ini and the henchman sets");
    }

    #[test]
    fn spell_lists_round_trip_in_aurora_s_layout() {
        let text = "[Spells10]\r\nSpell000=58,0,4\r\nCount=2\r\nSpell001=184,1,0\r\n";
        let spells = read_spells(text, 10).unwrap();
        assert_eq!(
            spells,
            [
                ListedSpell { spell: 58, flags: 0, metamagic: 4 },
                ListedSpell { spell: 184, flags: 1, metamagic: 0 }
            ]
        );
        assert_eq!(write_spells(10, &spells), text);
        assert_eq!(read_spells(text, 2), None, "no cleric list");
        assert_eq!(metamagic_levels(4), 3);
    }

    #[test]
    fn reads_the_game_s_spell_lists() {
        let root = mg_testkit::corpus!();
        let text = std::fs::read_to_string(root.join("data/scr/cleric_evil_10.ini")).unwrap();
        let spells = read_spells(&text, 2).unwrap();
        assert_eq!(spells.len(), 13);
        assert_eq!(spells[0], ListedSpell { spell: 449, flags: 1, metamagic: 0 });
    }
}
