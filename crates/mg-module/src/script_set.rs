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
}
