//! New blueprints, as Aurora's blueprint wizards make them: the fields,
//! their order and default values of each type, the default name
//! ("<category> 001"), and the resref and tag a name gives. Checked against
//! blueprints Aurora's wizards made (`aurora_blueprints.rs`).
//!
//! The Creature Wizard's creature ([`creature`]) is levelled up from
//! nothing by its classes' packages (`mg_rules::levelup`).

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

/// The Sound Properties' play styles as Aurora writes them: (label,
/// `Looping`, `Continuous`). Seamless looping takes a single sound and
/// plays it in order (`Random` 0).
pub const SOUND_PLAY_STYLES: [(&str, bool, bool); 3] =
    [("Once", false, false), ("Repeating", false, true), ("Seamlessly looping", true, false)];

/// A sound's prioritygroups.2da row, as Aurora sets it whenever its play
/// style or positioning changes: looping area-wide 2, looping positional 3,
/// else (single shots) area-wide 19, positional 20.
pub fn sound_priority(looping: bool, positional: bool) -> u8 {
    match (looping, positional) {
        (true, false) => 2,
        (true, true) => 3,
        (false, false) => 19,
        (false, true) => 20,
    }
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
        sound_priority(self.looping(), self.positional())
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

/// What the Creature Wizard asks for.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatureSpec {
    pub resref: ResRef,
    pub first_name: String,
    pub last_name: String,
    /// racialtypes.2da row.
    pub race: u32,
    /// 0 male, 1 female.
    pub gender: u8,
    /// appearance.2da row.
    pub appearance: u16,
    /// portraits.2da row.
    pub portrait: u16,
    pub faction: u16,
    /// (class, level), up to eight.
    pub classes: Vec<(u32, u32)>,
    pub category: u8,
}

/// The default event scripts of new creatures.
const CREATURE_SCRIPTS: [(&str, &str); 13] = [
    ("ScriptHeartbeat", "x2_def_heartbeat"),
    ("ScriptOnNotice", "x2_def_percept"),
    ("ScriptSpellAt", "x2_def_spellcast"),
    ("ScriptAttacked", "x2_def_attacked"),
    ("ScriptDamaged", "x2_def_ondamage"),
    ("ScriptDisturbed", "x2_def_ondisturb"),
    ("ScriptEndRound", "x2_def_endcombat"),
    ("ScriptDialogue", "x2_def_onconv"),
    ("ScriptSpawn", "x2_def_spawn"),
    ("ScriptRested", "x2_def_rested"),
    ("ScriptDeath", "x2_def_ondeath"),
    ("ScriptUserDefine", "x2_def_userdef"),
    ("ScriptOnBlocked", "x2_def_onblocked"),
];

/// The body parts of a new creature, in Aurora's order, and their value
/// (Aurora writes the right foot as `ArmorPart_RFoot`).
const CREATURE_PARTS: [(&str, u8); 18] = [
    ("ArmorPart_RFoot", 1),
    ("BodyPart_LFoot", 1),
    ("BodyPart_RShin", 1),
    ("BodyPart_LShin", 1),
    ("BodyPart_LThigh", 1),
    ("BodyPart_RThigh", 1),
    ("BodyPart_Pelvis", 1),
    ("BodyPart_Torso", 1),
    ("BodyPart_Belt", 0),
    ("BodyPart_Neck", 1),
    ("BodyPart_RFArm", 1),
    ("BodyPart_LFArm", 1),
    ("BodyPart_RBicep", 1),
    ("BodyPart_LBicep", 1),
    ("BodyPart_RShoul", 0),
    ("BodyPart_LShoul", 0),
    ("BodyPart_RHand", 1),
    ("BodyPart_LHand", 1),
];

/// The alignment (good–evil, lawful–chaotic) Aurora's Creature Wizard
/// gives a racial type (a table of its own: racialtypes.2da has none);
/// other types are neutral.
pub fn race_alignment(race: u32) -> (u8, u8) {
    match race {
        0 => (100, 100),       // dwarf: lawful good
        1 | 4 => (100, 0),     // elf, half-elf: chaotic good
        2 => (100, 50),        // gnome: neutral good
        5 | 17 => (50, 0),     // half-orc, fey: chaotic neutral
        12 => (0, 50),         // goblinoid: neutral evil
        7 | 13 | 14 => (0, 0), // aberration, monstrous, orc: chaotic evil
        _ => (50, 50),         // the others captured: true neutral
    }
}

/// The creature hide (creature armour slot) Aurora's Creature Wizard gives
/// a racial type: the generic "Dragon Properties" and "Elemental
/// Properties" (captured), "Construct Properties" (by analogy, not
/// captured).
pub fn race_hide(race: u32) -> Option<&'static str> {
    match race {
        10 => Some("nw_it_creitemcon"),
        11 => Some("nw_it_creitemdra"),
        16 => Some("nw_it_creitemele"),
        _ => None,
    }
}

/// The sound set Aurora's Creature Wizard writes: no row of soundset.2da
/// (the creature is silent until one is chosen).
const WIZARD_SOUND_SET: u16 = 24448;

/// A creature as Aurora's Creature Wizard makes it: the fields in its
/// order, the first class's recommended abilities (classes.2da `Str` to
/// `Cha`) and package, the race's alignment ([`race_alignment`]) and hide
/// ([`race_hide`]), levelled
/// up from nothing by the classes' packages (hit points, skills, feats,
/// spells; [`GameData::level_up`]), each class's package equipment, and its
/// hit points and challenge rating. `item` reads item blueprints. Aurora
/// writes `Interruptable`, `NoPermDeath` and `Disarmable` as 144, 95 and
/// 16 (read as set): Moonglow writes 1.
pub fn creature(
    game: &GameData,
    spec: &CreatureSpec,
    item: &dyn Fn(ResRef) -> Option<Struct>,
) -> Gff {
    let classes = game.table("classes").ok();
    let first = spec.classes.first().map_or(0, |&(c, _)| c as usize);
    let cell = |c: &str| classes.as_ref().and_then(|t| t.get_int(first, c)).unwrap_or(10);
    let package = classes.as_ref().and_then(|t| t.get_int(first, "Package")).unwrap_or(0);
    let skills = game.table("skills").map_or(28, |t| t.len());
    let skill_list = (0..skills)
        .map(|_| {
            let mut s = Struct::new(0);
            s.set("Rank", Value::Byte(0));
            s
        })
        .collect();
    let mut fields: Vec<(&str, Value)> = vec![
        ("TemplateResRef", resref_value(spec.resref)),
        ("Race", Value::Byte(spec.race.min(255) as u8)),
        ("FirstName", name(&spec.first_name)),
        ("LastName", name(&spec.last_name)),
        ("Appearance_Type", Value::Word(spec.appearance)),
        ("Gender", Value::Byte(spec.gender)),
        ("Phenotype", Value::Int(0)),
        ("PortraitId", Value::Word(spec.portrait)),
        ("Description", empty_name()),
        ("Tag", text(&spec.first_name)),
        ("Conversation", no_script()),
        ("IsPC", Value::Byte(0)),
        ("FactionID", Value::Word(spec.faction)),
        ("Disarmable", Value::Byte(1)),
        ("Subrace", text("")),
        ("Deity", text("")),
        ("Wings_New", Value::Dword(0)),
        ("Tail_New", Value::Dword(0)),
        ("SoundSetFile", Value::Word(WIZARD_SOUND_SET)),
        ("Plot", Value::Byte(0)),
        ("IsImmortal", Value::Byte(0)),
        ("Interruptable", Value::Byte(1)),
        ("Lootable", Value::Byte(0)),
        ("NoPermDeath", Value::Byte(1)),
        ("BodyBag", Value::Byte(0)),
        ("StartingPackage", Value::Byte(package.clamp(0, 255) as u8)),
        ("DecayTime", Value::Dword(5000)),
    ];
    // A body of parts (appearance.2da MODELTYPE P) has its parts and
    // colours; others have none.
    let parts = game
        .table("appearance")
        .ok()
        .and_then(|t| {
            t.get(usize::from(spec.appearance), "MODELTYPE").map(|m| m.eq_ignore_ascii_case("P"))
        })
        .unwrap_or(false);
    let twins: Vec<(String, String, u8)> = CREATURE_PARTS
        .iter()
        .map(|(l, v)| (l.to_string(), format!("x{l}"), *v))
        .chain(std::iter::once(("Appearance_Head".to_string(), "xAppearance_Head".to_string(), 1)))
        .collect();
    let mut g = Gff::new(*b"UTC ");
    for (label, v) in fields.drain(..) {
        g.root.set(label, v);
    }
    for (l, x, v) in twins.iter().filter(|_| parts) {
        g.root.set(l, Value::Byte(*v));
        g.root.set(x, Value::Word(u16::from(*v)));
    }
    let abilities = ["Str", "Dex", "Con", "Int", "Wis", "Cha"];
    let rest: Vec<(&str, Value)> = vec![
        ("Color_Skin", Value::Byte(1)),
        ("Color_Hair", Value::Byte(1)),
        ("Color_Tattoo1", Value::Byte(1)),
        ("Color_Tattoo2", Value::Byte(1)),
    ];
    for (label, v) in rest.into_iter().filter(|_| parts) {
        g.root.set(label, v);
    }
    for a in abilities {
        g.root.set(a, Value::Byte(cell(a).clamp(0, 255) as u8));
    }
    // The walk rate: creaturespeed.2da's row for the appearance's
    // MOVERATE (a human NORM 4, a leopard FAST 5).
    let walk = game
        .table("appearance")
        .ok()
        .and_then(|t| t.get(usize::from(spec.appearance), "MOVERATE").map(str::to_owned))
        .and_then(|m| {
            let speed = game.table("creaturespeed").ok()?;
            (0..speed.len()).find(|&r| speed.get(r, "2DAName") == Some(m.as_str()))
        })
        .unwrap_or(4);
    for (label, v) in [
        ("WalkRate", Value::Int(walk as i32)),
        ("NaturalAC", Value::Byte(0)),
        ("HitPoints", Value::Short(0)),
        ("CurrentHitPoints", Value::Short(0)),
        ("MaxHitPoints", Value::Short(0)),
        ("refbonus", Value::Short(0)),
        ("willbonus", Value::Short(0)),
        ("fortbonus", Value::Short(0)),
        ("GoodEvil", Value::Byte(race_alignment(spec.race).0)),
        ("LawfulChaotic", Value::Byte(race_alignment(spec.race).1)),
        ("ChallengeRating", Value::Float(0.0)),
        ("CRAdjust", Value::Int(0)),
        ("PerceptionRange", Value::Byte(11)),
    ] {
        g.root.set(label, v);
    }
    for (label, script) in CREATURE_SCRIPTS {
        g.root.set(label, Value::resref(ResRef::from_str(script).expect("valid")));
    }
    for (label, v) in [
        ("SkillList", Value::List(skill_list)),
        ("FeatList", Value::List(Vec::new())),
        ("TemplateList", Value::List(Vec::new())),
        ("SpecAbilityList", Value::List(Vec::new())),
        ("ClassList", Value::List(Vec::new())),
        ("ItemList", Value::List(Vec::new())),
        ("Equip_ItemList", Value::List(Vec::new())),
        ("PaletteID", Value::Byte(spec.category)),
        ("Comment", text("")),
    ] {
        g.root.set(label, v);
    }

    // Levelled up from nothing, the race's hide, each class's gear, then
    // the derived numbers.
    let mut c = game.level_up(&g.root, &spec.classes);
    if let Some(hide) = race_hide(spec.race).and_then(|h| ResRef::from_str(h).ok()) {
        let mut e = Struct::new(0x20000);
        e.set("EquippedRes", Value::resref(hide));
        c.set("Equip_ItemList", Value::List(vec![e]));
    }
    for &(class, _) in &spec.classes {
        for (res, at) in game.new_class_gear(&c, class, item) {
            match at {
                mg_rules::levelup::GearPlace::Equip(slot) => {
                    let mut e = Struct::new(slot);
                    e.set("EquippedRes", Value::resref(res));
                    let mut list = c.list("Equip_ItemList").unwrap_or(&[]).to_vec();
                    let at = list.iter().position(|s| s.id > slot).unwrap_or(list.len());
                    list.insert(at, e);
                    c.set("Equip_ItemList", Value::List(list));
                }
                mg_rules::levelup::GearPlace::Carry(x, y) => {
                    let mut list = c.list("ItemList").unwrap_or(&[]).to_vec();
                    let mut e = Struct::new(list.len() as u32);
                    e.set("InventoryRes", Value::resref(res));
                    e.set("Repos_PosX", Value::Word(x));
                    e.set("Repos_Posy", Value::Word(y));
                    list.push(e);
                    c.set("ItemList", Value::List(list));
                }
            }
        }
    }
    // An empty backpack is left out.
    if c.list("ItemList").is_some_and(<[Struct]>::is_empty) {
        c.remove("ItemList");
    }
    let mut sheet = mg_rules::CreatureSheet::from_gff(&c);
    let max = game.creature_stats(&sheet).max_hit_points;
    c.set("MaxHitPoints", Value::Short(max.clamp(0, i32::from(i16::MAX)) as i16));
    sheet.gear_value = game.gear_value(&c, item);
    c.set("ChallengeRating", Value::Float(game.challenge(&sheet).rating));
    g.root = c;
    g
}
