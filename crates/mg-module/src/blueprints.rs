//! New blueprints, as Aurora's blueprint wizards make them: the fields,
//! their order and default values of each type, the default name
//! ("<category> 001"), and the resref and tag a name gives. Checked against
//! blueprints Aurora's wizards made (`aurora_blueprints.rs`).
//!
//! The creature wizard (which also levels the creature up by its class
//! package) is not here yet.

use mg_core::{Gender, Language, LocString, ResRef};
use mg_gff::{Gff, Struct, Value};
use mg_rules::{GameData, ItemValue};

/// The name Aurora's wizards suggest: the category and a number.
pub fn default_name(category: &str, number: u32) -> String {
    format!("{category} {number:03}")
}

/// A blueprint's resref for a name, as Aurora's wizards derive it: the
/// name's letters and digits in lower case, cut so that a trailing number
/// survives within 16 characters; a name with other characters than
/// letters, digits, spaces and underscores becomes `blueprint` and a
/// number. `taken` names resrefs in use; a number is appended until free.
pub fn resref(name: &str, taken: impl Fn(&ResRef) -> bool) -> ResRef {
    let plain = name.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_');
    let compact: String = if plain {
        name.chars().filter(|c| *c != ' ').map(|c| c.to_ascii_lowercase()).collect()
    } else {
        String::new()
    };
    let make = |s: &str| ResRef::from_str(s).expect("letters and digits");
    if !compact.is_empty() {
        // A trailing number stays whole.
        let digits = compact.len() - compact.trim_end_matches(|c: char| c.is_ascii_digit()).len();
        let (stem, number) = compact.split_at(compact.len() - digits.min(compact.len()));
        let mut stem = stem.to_string();
        stem.truncate(16usize.saturating_sub(number.len()));
        let candidate = make(&format!("{stem}{number}"));
        if !taken(&candidate) {
            return candidate;
        }
    }
    let stem = if compact.is_empty() { "blueprint".to_string() } else { compact };
    (1..)
        .map(|n| {
            let suffix = format!("{n:03}");
            let mut s = stem.trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
            s.truncate(16 - suffix.len());
            make(&format!("{s}{suffix}"))
        })
        .find(|r| !taken(r))
        .expect("a free resref")
}

/// A blueprint's tag for a name: its letters, digits and underscores.
pub fn tag(name: &str) -> String {
    crate::new::tag_for(name)
}

fn text(s: &str) -> Value {
    Value::String(s.as_bytes().to_vec())
}

fn name(s: &str) -> Value {
    Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, s))
}

fn empty_name() -> Value {
    Value::LocString(LocString::default())
}

fn resref_value(r: ResRef) -> Value {
    Value::resref(r)
}

fn no_script() -> Value {
    Value::resref(ResRef::EMPTY)
}

fn gff(file_type: &[u8; 4], fields: Vec<(&str, Value)>) -> Gff {
    let mut g = Gff::new(*file_type);
    for (label, v) in fields {
        g.root.set(label, v);
    }
    g
}

/// A waypoint: its name is its tag.
pub fn waypoint(resref: ResRef, tag_name: &str, appearance: u8, category: u8) -> Gff {
    gff(
        b"UTW ",
        vec![
            ("Appearance", Value::Byte(appearance)),
            ("LinkedTo", text("")),
            ("TemplateResRef", resref_value(resref)),
            ("Tag", text(tag_name)),
            ("LocalizedName", name(tag_name)),
            ("Description", empty_name()),
            ("HasMapNote", Value::Byte(0)),
            ("MapNote", empty_name()),
            ("MapNoteEnabled", Value::Byte(0)),
            ("PaletteID", Value::Byte(category)),
            ("Comment", text("")),
        ],
    )
}

/// How a new sound plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundStyle {
    /// One wave repeating seamlessly, everywhere in the area.
    LoopingAreaWide,
    /// Looping, from where it stands.
    LoopingPositional,
    /// Separate plays with a pause, everywhere in the area.
    SingleShotAreaWide,
    /// Separate plays, each from near where it stands.
    SingleShotRandom,
    /// Separate plays, from where it stands.
    SingleShotPositional,
}

impl SoundStyle {
    /// Its prioritygroups.2da row.
    pub fn priority(self) -> u8 {
        match self {
            SoundStyle::LoopingAreaWide => 2,
            SoundStyle::LoopingPositional => 3,
            SoundStyle::SingleShotAreaWide => 19,
            SoundStyle::SingleShotRandom | SoundStyle::SingleShotPositional => 20,
        }
    }

    fn looping(self) -> bool {
        matches!(self, SoundStyle::LoopingAreaWide | SoundStyle::LoopingPositional)
    }

    fn positional(self) -> bool {
        !matches!(self, SoundStyle::LoopingAreaWide | SoundStyle::SingleShotAreaWide)
    }
}

/// A sound object playing `sounds`.
pub fn sound(
    resref: ResRef,
    display: &str,
    category: u8,
    style: SoundStyle,
    sounds: &[ResRef],
) -> Gff {
    let looping = style.looping();
    let random = style == SoundStyle::SingleShotRandom;
    let f = Value::Float;
    let list = sounds
        .iter()
        .map(|r| {
            let mut s = Struct::new(0);
            s.set("Sound", Value::resref(*r));
            s
        })
        .collect();
    gff(
        b"UTS ",
        vec![
            ("Tag", text(&tag(display))),
            ("LocName", name(display)),
            ("TemplateResRef", resref_value(resref)),
            ("Active", Value::Byte(1)),
            ("Continuous", Value::Byte(1)),
            ("Looping", Value::Byte(u8::from(looping))),
            ("Positional", Value::Byte(u8::from(style.positional()))),
            ("RandomPosition", Value::Byte(u8::from(random))),
            ("Random", Value::Byte(u8::from(!looping))),
            ("Elevation", f(if style.positional() { 1.5 } else { 1.0 })),
            ("MaxDistance", f(10.0)),
            ("MinDistance", f(1.0)),
            ("RandomRangeX", f(if random { 10.0 } else { 0.0 })),
            ("RandomRangeY", f(if random { 10.0 } else { 0.0 })),
            ("Interval", Value::Dword(if looping { 0 } else { 25000 })),
            ("IntervalVrtn", Value::Dword(if looping { 0 } else { 5000 })),
            ("PitchVariation", f(if looping { 0.0 } else { 0.1 })),
            ("Priority", Value::Byte(style.priority())),
            ("Hours", Value::Dword(0)),
            ("Times", Value::Byte(3)),
            ("Volume", Value::Byte(127)),
            ("VolumeVrtn", Value::Byte(0)),
            ("Sounds", Value::List(list)),
            ("PaletteID", Value::Byte(category)),
            ("Comment", text("")),
        ],
    )
}

/// Trigger palette categories with their own kind of trigger: Area
/// Transition, and the traps by strength (their trap type).
const AREA_TRANSITION_CATEGORY: u8 = 5;
const TRAP_CATEGORIES: std::ops::RangeInclusive<u8> = 11..=15;

/// A trigger; its category decides whether it is an area transition or a
/// trap (of the strength the category names).
pub fn trigger(game: &GameData, resref: ResRef, display: &str, category: u8) -> Gff {
    let trap = TRAP_CATEGORIES.contains(&category).then(|| category - TRAP_CATEGORIES.start());
    let kind = if category == AREA_TRANSITION_CATEGORY {
        1
    } else if trap.is_some() {
        2
    } else {
        0
    };
    let traps = game.table("traps").ok();
    let dc = |col: &str, default: u8| {
        trap.and_then(|t| traps.as_ref()?.get_int(t as usize, col))
            .map_or(default, |v| v.clamp(0, 255) as u8)
    };
    gff(
        b"UTT ",
        vec![
            ("Tag", text(&tag(display))),
            ("TemplateResRef", resref_value(resref)),
            ("LocalizedName", name(display)),
            ("AutoRemoveKey", Value::Byte(0)),
            ("Faction", Value::Dword(1)),
            ("Cursor", Value::Byte(0)),
            ("HighlightHeight", Value::Float(3.0)),
            ("KeyName", text("")),
            ("LinkedTo", text("")),
            ("LinkedToFlags", Value::Byte(0)),
            ("LoadScreenID", Value::Word(0)),
            ("PortraitId", Value::Word(0)),
            ("Type", Value::Int(kind)),
            ("TrapDetectable", Value::Byte(1)),
            ("TrapDetectDC", Value::Byte(dc("DetectDCMod", 10))),
            ("TrapDisarmable", Value::Byte(1)),
            ("DisarmDC", Value::Byte(dc("DisarmDCMod", 10))),
            ("TrapFlag", Value::Byte(u8::from(trap.is_some()))),
            ("TrapOneShot", Value::Byte(1)),
            ("TrapType", Value::Byte(trap.unwrap_or(0))),
            ("OnDisarm", no_script()),
            ("OnTrapTriggered", no_script()),
            ("OnClick", no_script()),
            ("ScriptHeartbeat", no_script()),
            ("ScriptOnEnter", no_script()),
            ("ScriptOnExit", no_script()),
            ("ScriptUserDefine", no_script()),
            ("PaletteID", Value::Byte(category)),
            ("Comment", text("")),
        ],
    )
}

/// An encounter spawning `creatures` (encounter creature list entries).
pub fn encounter(resref: ResRef, display: &str, category: u8, creatures: Vec<Struct>) -> Gff {
    gff(
        b"UTE ",
        vec![
            ("Tag", text(&tag(display))),
            ("LocalizedName", name(display)),
            ("TemplateResRef", resref_value(resref)),
            ("Active", Value::Byte(1)),
            ("Difficulty", Value::Int(1)),
            ("DifficultyIndex", Value::Int(1)),
            ("Faction", Value::Dword(1)),
            ("MaxCreatures", Value::Int(4)),
            ("PlayerOnly", Value::Byte(0)),
            ("RecCreatures", Value::Int(2)),
            ("Reset", Value::Byte(0)),
            ("ResetTime", Value::Int(60)),
            ("Respawns", Value::Int(0)),
            ("SpawnOption", Value::Int(1)),
            ("OnEntered", no_script()),
            ("OnExit", no_script()),
            ("OnExhausted", no_script()),
            ("OnHeartbeat", no_script()),
            ("OnUserDefined", no_script()),
            ("CreatureList", Value::List(creatures)),
            ("PaletteID", Value::Byte(category)),
            ("Comment", text("")),
        ],
    )
}

/// A store, with its five (empty) pages.
pub fn store(resref: ResRef, display: &str, category: u8) -> Gff {
    let pages = [0, 4, 2, 3, 1].into_iter().map(Struct::new).collect();
    gff(
        b"UTM ",
        vec![
            ("ResRef", resref_value(resref)),
            ("LocName", name(display)),
            ("Tag", text(&tag(display))),
            ("MarkUp", Value::Int(100)),
            ("MarkDown", Value::Int(65)),
            ("BlackMarket", Value::Byte(0)),
            ("BM_MarkDown", Value::Int(25)),
            ("IdentifyPrice", Value::Int(100)),
            ("MaxBuyPrice", Value::Int(-1)),
            ("StoreGold", Value::Int(-1)),
            ("OnOpenStore", no_script()),
            ("OnStoreClosed", no_script()),
            ("WillNotBuy", Value::List(Vec::new())),
            ("WillOnlyBuy", Value::List(Vec::new())),
            ("StoreList", Value::List(pages)),
            ("ID", Value::Byte(category)),
            ("Comment", text("")),
        ],
    )
}

/// The fields doors and placeables share, up to their events.
fn situated(resref: ResRef, display: &str, portrait: u16) -> Vec<(&'static str, Value)> {
    vec![
        ("Tag", text(&tag(display))),
        ("LocName", name(display)),
        ("Description", empty_name()),
        ("TemplateResRef", resref_value(resref)),
        ("AutoRemoveKey", Value::Byte(0)),
        ("CloseLockDC", Value::Byte(0)),
        ("Conversation", no_script()),
        ("Interruptable", Value::Byte(1)),
        ("Faction", Value::Dword(1)),
        ("Plot", Value::Byte(0)),
        ("KeyRequired", Value::Byte(0)),
        ("Lockable", Value::Byte(1)),
        ("Locked", Value::Byte(0)),
        ("OpenLockDC", Value::Byte(0)),
        ("PortraitId", Value::Word(portrait)),
        ("TrapDetectable", Value::Byte(1)),
        ("TrapDetectDC", Value::Byte(0)),
        ("TrapDisarmable", Value::Byte(1)),
        ("DisarmDC", Value::Byte(0)),
        ("TrapFlag", Value::Byte(0)),
        ("TrapOneShot", Value::Byte(1)),
        ("TrapType", Value::Byte(0)),
        ("KeyName", text("")),
        ("AnimationState", Value::Byte(0)),
        ("Appearance", Value::Dword(0)),
        ("HP", Value::Short(10)),
        ("CurrentHP", Value::Short(10)),
        ("Hardness", Value::Byte(5)),
        ("Fort", Value::Byte(5)),
        ("Ref", Value::Byte(0)),
        ("Will", Value::Byte(0)),
    ]
}

/// The events doors and placeables share (`on_death` for OnDeath).
fn situated_events(on_death: Value) -> Vec<(&'static str, Value)> {
    vec![
        ("OnClosed", no_script()),
        ("OnDamaged", no_script()),
        ("OnDeath", on_death),
        ("OnDisarm", no_script()),
        ("OnHeartbeat", no_script()),
        ("OnLock", no_script()),
        ("OnMeleeAttacked", no_script()),
        ("OnOpen", no_script()),
        ("OnSpellCastAt", no_script()),
        ("OnTrapTriggered", no_script()),
        ("OnUnlock", no_script()),
        ("OnUserDefined", no_script()),
        ("OnClick", no_script()),
    ]
}

/// A placeable: static scenery until made otherwise.
pub fn placeable(resref: ResRef, display: &str, category: u8) -> Gff {
    let mut fields = situated(resref, display, 0);
    fields.extend(situated_events(no_script()));
    fields.extend([
        ("HasInventory", Value::Byte(0)),
        ("BodyBag", Value::Byte(0)),
        ("Static", Value::Byte(1)),
        ("Type", Value::Byte(0)),
        ("Useable", Value::Byte(0)),
        ("OnInvDisturbed", no_script()),
        ("OnUsed", no_script()),
        ("PaletteID", Value::Byte(category)),
        ("Comment", text("")),
    ]);
    gff(b"UTP ", fields)
}

/// The portrait (portraits.2da row) and death script of new doors.
const DOOR_PORTRAIT: u16 = 558;
const DOOR_ON_DEATH: &str = "x2_door_death";

/// A door (the area's tileset door).
pub fn door(resref: ResRef, display: &str, category: u8) -> Gff {
    let mut fields = situated(resref, display, DOOR_PORTRAIT);
    fields.extend(situated_events(Value::resref(ResRef::from_str(DOOR_ON_DEATH).expect("valid"))));
    fields.extend([
        ("LinkedTo", text("")),
        ("LinkedToFlags", Value::Byte(0)),
        ("LoadScreenID", Value::Word(0)),
        ("GenericType_New", Value::Dword(0)),
        ("OnFailToOpen", no_script()),
        ("PaletteID", Value::Byte(category)),
        ("Comment", text("")),
    ]);
    gff(b"UTD ", fields)
}

/// The EE part fields of armor, in the order Aurora writes them.
const ARMOR_PARTS: [&str; 18] = [
    "ArmorPart_RFoot",
    "ArmorPart_LFoot",
    "ArmorPart_RShin",
    "ArmorPart_LShin",
    "ArmorPart_LThigh",
    "ArmorPart_RThigh",
    "ArmorPart_Pelvis",
    "ArmorPart_Torso",
    "ArmorPart_Belt",
    "ArmorPart_Neck",
    "ArmorPart_RFArm",
    "ArmorPart_LFArm",
    "ArmorPart_RBicep",
    "ArmorPart_LBicep",
    "ArmorPart_RShoul",
    "ArmorPart_LShoul",
    "ArmorPart_RHand",
    "ArmorPart_LHand",
];

const COLORS: [&str; 6] =
    ["Leather1Color", "Leather2Color", "Cloth1Color", "Cloth2Color", "Metal1Color", "Metal2Color"];

/// An item of a base item: its model fields by the base item's model type
/// (one model, coloured layers, three weapon parts, or armor parts), and
/// its cost.
pub fn item(game: &GameData, resref: ResRef, display: &str, base_item: u32, category: u8) -> Gff {
    let model_type = game
        .table("baseitems")
        .ok()
        .and_then(|t| t.get_int(base_item as usize, "ModelType"))
        .unwrap_or(0);
    let part = |fields: &mut Vec<(String, Value)>, label: &str, v: u16| {
        fields.push((label.to_string(), Value::Byte(v as u8)));
        fields.push((mg_rules::items::wide_label(label), Value::Word(v)));
    };
    let mut model: Vec<(String, Value)> = Vec::new();
    match model_type {
        2 => {
            for label in ["ModelPart1", "ModelPart2", "ModelPart3"] {
                part(&mut model, label, 11);
            }
        }
        3 => {
            for label in ARMOR_PARTS {
                part(&mut model, label, 4);
            }
            part(&mut model, "ArmorPart_Robe", 1);
        }
        _ => part(&mut model, "ModelPart1", 1),
    }
    if matches!(model_type, 1 | 3) {
        model.extend(COLORS.iter().map(|c| (c.to_string(), Value::Byte(0))));
    }
    let mut g = gff(
        b"UTI ",
        vec![
            ("TemplateResRef", resref_value(resref)),
            ("BaseItem", Value::Int(base_item as i32)),
            ("LocalizedName", name(display)),
            ("Description", empty_name()),
            ("DescIdentified", empty_name()),
            ("Tag", text(&tag(display))),
            ("Charges", Value::Byte(0)),
            ("Cost", Value::Dword(0)),
            ("Stolen", Value::Byte(0)),
            ("StackSize", Value::Word(1)),
            ("Plot", Value::Byte(0)),
            ("AddCost", Value::Dword(0)),
            ("Identified", Value::Byte(1)),
            ("Cursed", Value::Byte(0)),
        ],
    );
    for (label, v) in model {
        g.root.set(&label, v);
    }
    g.root.set("PropertiesList", Value::List(Vec::new()));
    g.root.set("PaletteID", Value::Byte(category));
    g.root.set("Comment", text(""));
    let cost = game.item_cost(&ItemValue::from_gff(&g.root));
    g.root.set("Cost", Value::Dword(cost));
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    #[test]
    fn resrefs_and_tags_as_aurora_derives_them() {
        let free = |_: &ResRef| false;
        assert_eq!(resref("Civilization 001", free), r("civilization001"));
        assert_eq!(resref("Area Transition 001", free), r("areatransitio001"));
        assert_eq!(resref("Tileset Specific 001", free), r("tilesetspecif001"));
        assert_eq!(resref("Wizard Sword", free), r("wizardsword"));
        assert_eq!(resref("Containers & Switches 001", free), r("blueprint001"));
        assert_eq!(resref("1. Average 001", free), r("blueprint001"));
        // Taken: numbered.
        let taken = |x: &ResRef| *x == r("wizardsword") || *x == r("blueprint001");
        assert_eq!(resref("Wizard Sword", taken), r("wizardsword001"));
        assert_eq!(resref("A & B", taken), r("blueprint002"));
        assert_eq!(tag("Containers & Switches 001"), "ContainersSwitches001");
        assert_eq!(tag("1. Average 001"), "1Average001");
        assert_eq!(default_name("Civilization", 1), "Civilization 001");
    }
}
