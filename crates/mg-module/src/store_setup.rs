//! The Store Setup Wizard (Aurora's `TdlgStoreSetupWizard`, Setup Store on
//! a creature in the area viewer): a shopkeeper conversation (a greeting,
//! a reply that opens the store, one that does not), the script that opens
//! it, the store placed where the shopkeeper stands, and the shopkeeper's
//! conversation and, if its faction is hostile, a friendlier faction.
//! Captured from Aurora (`store-setup/defaults.mod`, `second.mod`).
//!
//! Also Add Popup Text (a placeable's context menu): a conversation of one
//! line, which the placeable then speaks when used (`popup-text.mod`).

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;

use crate::Module;
use crate::dialog::{Kind, Parent, add_node, new_dialog, nodes};
use crate::factions::Factions;

/// What the shopkeeper says first (Aurora's default).
pub const GREETING: &str = "Welcome to my shop. Would you like to see my wares?";
/// The reply that opens the store.
pub const YES: &str = "Yes. I'd like to see what you have for sale.";
/// The reply that does not.
pub const NO: &str = "No, thanks.";

/// The first `stem001`, `stem002`, ... the module has no `restype` of.
pub fn next_name(module: &Module, stem: &str, restype: ResType) -> ResRef {
    (1..1000)
        .filter_map(|n| ResRef::from_str(&format!("{stem}{n:03}")).ok())
        .find(|r| !module.contains(&ResKey::new(*r, restype)))
        .unwrap_or(ResRef::EMPTY)
}

/// The script that opens the store with `tag` for the PC speaker (with
/// the appraise skill adjusting prices if `appraise`), or says that there
/// is none (talk table 53090), as Aurora writes it: CRLF line ends, no
/// final one.
pub fn script(tag: &str, appraise: bool) -> String {
    let (include, open) = if appraise {
        ("#include \"nw_i0_plot\"\r\n", "gplotAppraiseOpenStore")
    } else {
        ("", "OpenStore")
    };
    format!(
        "{include}void main()\r\n{{\r\n    object oStore = GetNearestObjectByTag(\"{tag}\");\r\n    \
         if (GetObjectType(oStore) == OBJECT_TYPE_STORE)\r\n    {{\r\n        \
         {open}(oStore, GetPCSpeaker());\r\n    }}\r\n    else\r\n    {{\r\n        \
         ActionSpeakStringByStrRef(53090, TALKVOLUME_TALK);\r\n    }}\r\n}}"
    )
}

/// The shopkeeper's conversation: the greeting, then the replies, the
/// first running `open` (the store script).
pub fn conversation(greeting: &str, yes: &str, no: &str, open: ResRef) -> Gff {
    let mut g = new_dialog();
    // Aurora's wizard leaves the end scripts empty.
    g.root.set("EndConversation", Value::resref(ResRef::EMPTY));
    g.root.set("EndConverAbort", Value::resref(ResRef::EMPTY));
    let entry = add_node(&mut g, Parent::Root, greeting);
    let opens = add_node(&mut g, Parent::Node(Kind::Entry, entry), yes);
    add_node(&mut g, Parent::Node(Kind::Entry, entry), no);
    if let Some(Value::List(replies)) = g.root.get_mut(Kind::Reply.list()) {
        replies[opens as usize].set("Script", Value::resref(open));
    }
    debug_assert_eq!(nodes(&g, Kind::Reply).len(), 2);
    g
}

/// Add Popup Text's conversation: the one line, no replies.
pub fn popup(text: &str) -> Gff {
    let mut g = new_dialog();
    g.root.set("EndConversation", Value::resref(ResRef::EMPTY));
    g.root.set("EndConverAbort", Value::resref(ResRef::EMPTY));
    add_node(&mut g, Parent::Root, text);
    g
}

/// Whether a faction is hostile to players: it likes the PC faction 10 or
/// less (the standard Hostile faction); a module without `repute.fac` has
/// the standard factions.
pub fn hostile(factions: Option<&Factions>, faction: u32) -> bool {
    match factions.and_then(|f| f.reputation(faction, 0)) {
        Some(rep) => rep <= 10,
        None => faction == 1,
    }
}

/// The faction Aurora suggests for a hostile shopkeeper: the module's
/// Merchant faction.
pub fn merchant(factions: Option<&Factions>) -> u32 {
    factions
        .and_then(|f| f.factions.iter().position(|x| x.name.eq_ignore_ascii_case("Merchant")))
        .map_or(3, |i| i as u32)
}

/// The store instance's place: where the shopkeeper stands, facing as it
/// does (its orientation's bearing, less the quarter turn the instance's
/// facing adds).
pub fn store_placement(creature: &Struct) -> crate::instances::Placement {
    let f = |l: &str| creature.float(l).unwrap_or(0.0);
    let rotation = f("YOrientation").atan2(f("XOrientation")) - std::f32::consts::FRAC_PI_2;
    crate::instances::Placement {
        position: [f("XPosition"), f("YPosition"), f("ZPosition")],
        rotation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_aurora_s_script_and_conversation() {
        let s = script("MGP_STORE", true);
        assert!(s.starts_with("#include \"nw_i0_plot\"\r\nvoid main()\r\n{\r\n    object oStore"));
        assert!(s.ends_with("TALKVOLUME_TALK);\r\n    }\r\n}"));
        assert!(script("X", false).starts_with("void main()"));
        assert!(script("X", false).contains("        OpenStore(oStore, GetPCSpeaker());"));
        let g = conversation(GREETING, YES, NO, ResRef::from_str("openstore001").unwrap());
        assert_eq!(g.root.dword("NumWords"), Some(23));
        let replies = nodes(&g, Kind::Reply);
        assert_eq!(replies[0].resref("Script"), Some(ResRef::from_str("openstore001").unwrap()));
        assert_eq!(replies[1].resref("Script"), Some(ResRef::EMPTY));
        assert!(hostile(None, 1) && !hostile(None, 3));
        assert_eq!(merchant(None), 3);
        let mut m = Module::new();
        assert_eq!(next_name(&m, "store", ResType::DLG).to_string(), "store001");
        m.set(ResKey::parse("store001", ResType::DLG).unwrap(), Vec::new());
        assert_eq!(next_name(&m, "store", ResType::DLG).to_string(), "store002");
    }
}
