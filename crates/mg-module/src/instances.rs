//! Blueprints placed in areas: the instance (a GIT list entry) of a
//! blueprint, as Aurora writes it (captured: `aurora_instances.rs`).
//!
//! Aurora reads the blueprint into its object, then writes the object:
//!
//! - every field of the type, the blueprint's where it has one and Aurora's
//!   default where not; the palette's fields (`PaletteID`, a store's `ID`,
//!   `Comment`) left out; old fields in their EE form (`Tail` →
//!   `Tail_New`, a door's `GenericType` → `GenericType_New`; a placeable's
//!   or trigger's `Portrait` resref is dropped, its `PortraitId` 0);
//! - body parts, colours and their EE twins (`xBodyPart_Neck`) for
//!   part-based creatures only (appearance.2da `MODELTYPE` P); items'
//!   twins (`xModelPart1`, `xArmorPart_LBice`) always;
//! - equipment and inventories in full: each item's own fields (as placed,
//!   at −1, −1, −1 facing north), with its place in the grid;
//! - a skill list as long as skills.2da, sounds' struct ids 0, a sound's
//!   priority from how it plays, an item's cost when it has none, a
//!   store's five pages, an encounter's creatures by challenge rating;
//! - where it stands; all in the order Aurora writes each type (fields it
//!   does not know, such as local variables, follow, kept).
//!
//! Aurora's two quirks with blueprints lacking `Comment` (a creature then
//! gets an empty `Comment`, and its first skill another) are not copied.

use std::f32::consts::FRAC_PI_2;

use mg_core::{LocString, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_rules::GameData;

/// Where an instance stands: a position in metres and its model's turn
/// around Z in radians (its facing less 90°: 0 faces north, as Aurora
/// places everything).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Placement {
    pub position: [f32; 3],
    pub rotation: f32,
}

/// What placing a blueprint reads besides it: the rules, and item
/// blueprints (the module's, else the game's) for inventories.
pub struct Placing<'a> {
    pub game: &'a GameData,
    pub item: &'a dyn Fn(ResRef) -> Option<Struct>,
}

impl std::fmt::Debug for Placing<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Placing").finish_non_exhaustive()
    }
}

/// The GIT list a blueprint type's instances go in, and their struct id.
pub fn git_list(restype: ResType) -> Option<(&'static str, u32)> {
    Some(match restype {
        ResType::UTC => ("Creature List", 4),
        ResType::UTD => ("Door List", 8),
        ResType::UTE => ("Encounter List", 7),
        ResType::UTI => ("List", 0),
        ResType::UTP => ("Placeable List", 9),
        ResType::UTS => ("SoundList", 6),
        ResType::UTM => ("StoreList", 11),
        ResType::UTT => ("TriggerList", 1),
        ResType::UTW => ("WaypointList", 5),
        _ => return None,
    })
}

/// How high above the ground Aurora puts a trigger's or encounter's
/// points, metres.
pub const OUTLINE_LIFT: f32 = 0.025;

/// A default value (a field Aurora writes when the blueprint lacks it).
enum D {
    Byte(u8),
    Word(u16),
    Dword(u32),
    Int(i32),
    Short(i16),
    Float(f32),
    ResRef,
    String,
    LocString,
    List,
}

impl D {
    fn value(&self) -> Value {
        match *self {
            D::Byte(v) => Value::Byte(v),
            D::Word(v) => Value::Word(v),
            D::Dword(v) => Value::Dword(v),
            D::Int(v) => Value::Int(v),
            D::Short(v) => Value::Short(v),
            D::Float(v) => Value::Float(v),
            D::ResRef => Value::resref(ResRef::EMPTY),
            D::String => Value::String(Vec::new()),
            D::LocString => Value::LocString(LocString::default()),
            D::List => Value::List(Vec::new()),
        }
    }
}

const CREATURE: &[(&str, D)] = &[
    ("BodyBag", D::Byte(0)),
    ("Cha", D::Byte(0)),
    ("ChallengeRating", D::Float(1.0)),
    ("ClassList", D::List),
    ("Con", D::Byte(0)),
    ("Conversation", D::ResRef),
    ("CRAdjust", D::Int(0)),
    ("CurrentHitPoints", D::Short(0)),
    ("DecayTime", D::Dword(0)),
    ("Deity", D::String),
    ("Description", D::LocString),
    ("Dex", D::Byte(0)),
    ("Disarmable", D::Byte(1)),
    ("Equip_ItemList", D::List),
    ("FactionID", D::Word(1)),
    ("FeatList", D::List),
    ("FirstName", D::LocString),
    ("fortbonus", D::Short(0)),
    ("Gender", D::Byte(0)),
    ("GoodEvil", D::Byte(50)),
    ("HitPoints", D::Short(0)),
    ("Int", D::Byte(0)),
    ("Interruptable", D::Byte(1)),
    ("IsImmortal", D::Byte(0)),
    ("IsPC", D::Byte(0)),
    ("LastName", D::LocString),
    ("LawfulChaotic", D::Byte(50)),
    ("Lootable", D::Byte(0)),
    ("MaxHitPoints", D::Short(0)),
    ("NaturalAC", D::Byte(0)),
    ("NoPermDeath", D::Byte(0)),
    ("PerceptionRange", D::Byte(11)),
    ("Phenotype", D::Int(0)),
    ("Plot", D::Byte(0)),
    ("PortraitId", D::Word(0)),
    ("Race", D::Byte(0)),
    ("refbonus", D::Short(0)),
    ("ScriptAttacked", D::ResRef),
    ("ScriptDamaged", D::ResRef),
    ("ScriptDeath", D::ResRef),
    ("ScriptDialogue", D::ResRef),
    ("ScriptDisturbed", D::ResRef),
    ("ScriptEndRound", D::ResRef),
    ("ScriptHeartbeat", D::ResRef),
    ("ScriptOnBlocked", D::ResRef),
    ("ScriptOnNotice", D::ResRef),
    ("ScriptRested", D::ResRef),
    ("ScriptSpawn", D::ResRef),
    ("ScriptSpellAt", D::ResRef),
    ("ScriptUserDefine", D::ResRef),
    ("SoundSetFile", D::Word(0)),
    ("SpecAbilityList", D::List),
    ("StartingPackage", D::Byte(0)),
    ("Str", D::Byte(0)),
    ("Subrace", D::String),
    ("Tag", D::String),
    ("Tail_New", D::Dword(0)),
    ("TemplateList", D::List),
    ("TemplateResRef", D::ResRef),
    ("WalkRate", D::Int(4)),
    ("willbonus", D::Short(0)),
    ("Wings_New", D::Dword(0)),
    ("Wis", D::Byte(0)),
];

/// A part-based creature's parts (each with its EE twin), in Aurora's
/// order, and its colours.
const CREATURE_PARTS: [&str; 19] = [
    "ArmorPart_RFoot",
    "BodyPart_LFoot",
    "BodyPart_RShin",
    "BodyPart_LShin",
    "BodyPart_LThigh",
    "BodyPart_RThigh",
    "BodyPart_Pelvis",
    "BodyPart_Torso",
    "BodyPart_Belt",
    "BodyPart_Neck",
    "BodyPart_RFArm",
    "BodyPart_LFArm",
    "BodyPart_RBicep",
    "BodyPart_LBicep",
    "BodyPart_RShoul",
    "BodyPart_LShoul",
    "BodyPart_RHand",
    "BodyPart_LHand",
    "Appearance_Head",
];
const CREATURE_COLORS: [&str; 4] = ["Color_Skin", "Color_Hair", "Color_Tattoo1", "Color_Tattoo2"];

/// The order Aurora writes a creature's fields in (parts and their twins
/// go after `DecayTime`).
const CREATURE_ORDER: &[&str] = &[
    "XPosition",
    "YPosition",
    "ZPosition",
    "XOrientation",
    "YOrientation",
    "TemplateResRef",
    "Race",
    "FirstName",
    "LastName",
    "Appearance_Type",
    "Gender",
    "Phenotype",
    "PortraitId",
    "Description",
    "Tag",
    "Conversation",
    "IsPC",
    "FactionID",
    "Disarmable",
    "Subrace",
    "Deity",
    "Wings_New",
    "Tail_New",
    "SoundSetFile",
    "Plot",
    "IsImmortal",
    "Interruptable",
    "Lootable",
    "NoPermDeath",
    "BodyBag",
    "StartingPackage",
    "DecayTime",
    "#parts",
    "Str",
    "Dex",
    "Con",
    "Int",
    "Wis",
    "Cha",
    "WalkRate",
    "NaturalAC",
    "HitPoints",
    "CurrentHitPoints",
    "MaxHitPoints",
    "refbonus",
    "willbonus",
    "fortbonus",
    "GoodEvil",
    "LawfulChaotic",
    "ChallengeRating",
    "CRAdjust",
    "PerceptionRange",
    "ScriptHeartbeat",
    "ScriptOnNotice",
    "ScriptSpellAt",
    "ScriptAttacked",
    "ScriptDamaged",
    "ScriptDisturbed",
    "ScriptEndRound",
    "ScriptDialogue",
    "ScriptSpawn",
    "ScriptRested",
    "ScriptDeath",
    "ScriptUserDefine",
    "ScriptOnBlocked",
    "SkillList",
    "FeatList",
    "TemplateList",
    "SpecAbilityList",
    "ClassList",
    "ItemList",
    "Equip_ItemList",
];

/// An item's fields in Aurora's order (model fields go after `Cursed`).
const ITEM_ORDER: &[&str] = &[
    "XPosition",
    "YPosition",
    "ZPosition",
    "XOrientation",
    "YOrientation",
    "TemplateResRef",
    "BaseItem",
    "LocalizedName",
    "Description",
    "DescIdentified",
    "Tag",
    "Charges",
    "Cost",
    "Stolen",
    "StackSize",
    "Plot",
    "AddCost",
    "Identified",
    "Cursed",
    "#parts",
    "PropertiesList",
];

const PLACEABLE_ORDER: &[&str] = &[
    "Tag",
    "LocName",
    "Description",
    "TemplateResRef",
    "AutoRemoveKey",
    "CloseLockDC",
    "Conversation",
    "Interruptable",
    "Faction",
    "Plot",
    "KeyRequired",
    "Lockable",
    "Locked",
    "OpenLockDC",
    "PortraitId",
    "TrapDetectable",
    "TrapDetectDC",
    "TrapDisarmable",
    "DisarmDC",
    "TrapFlag",
    "TrapOneShot",
    "TrapType",
    "KeyName",
    "AnimationState",
    "Appearance",
    "HP",
    "CurrentHP",
    "Hardness",
    "Fort",
    "Ref",
    "Will",
    "OnClosed",
    "OnDamaged",
    "OnDeath",
    "OnDisarm",
    "OnHeartbeat",
    "OnLock",
    "OnMeleeAttacked",
    "OnOpen",
    "OnSpellCastAt",
    "OnTrapTriggered",
    "OnUnlock",
    "OnUserDefined",
    "OnClick",
    "HasInventory",
    "BodyBag",
    "Static",
    "Type",
    "Useable",
    "OnInvDisturbed",
    "OnUsed",
    "X",
    "Y",
    "Z",
    "Bearing",
    "ItemList",
];

const STORE_ORDER: &[&str] = &[
    "XPosition",
    "YPosition",
    "ZPosition",
    "XOrientation",
    "YOrientation",
    "ResRef",
    "LocName",
    "Tag",
    "MarkUp",
    "MarkDown",
    "BlackMarket",
    "BM_MarkDown",
    "IdentifyPrice",
    "MaxBuyPrice",
    "StoreGold",
    "OnOpenStore",
    "OnStoreClosed",
    "WillNotBuy",
    "WillOnlyBuy",
    "StoreList",
];

const SOUND_ORDER: &[&str] = &[
    "Tag",
    "LocName",
    "TemplateResRef",
    "Active",
    "Continuous",
    "Looping",
    "Positional",
    "RandomPosition",
    "Random",
    "Elevation",
    "MaxDistance",
    "MinDistance",
    "RandomRangeX",
    "RandomRangeY",
    "Interval",
    "IntervalVrtn",
    "PitchVariation",
    "Priority",
    "Hours",
    "Times",
    "Volume",
    "VolumeVrtn",
    "Sounds",
    "GeneratedType",
    "XPosition",
    "YPosition",
    "ZPosition",
];

const WAYPOINT_ORDER: &[&str] = &[
    "Appearance",
    "LinkedTo",
    "TemplateResRef",
    "Tag",
    "LocalizedName",
    "Description",
    "HasMapNote",
    "MapNote",
    "MapNoteEnabled",
    "XPosition",
    "YPosition",
    "ZPosition",
    "XOrientation",
    "YOrientation",
];

const TRIGGER_ORDER: &[&str] = &[
    "Tag",
    "TemplateResRef",
    "LocalizedName",
    "AutoRemoveKey",
    "Faction",
    "Cursor",
    "HighlightHeight",
    "KeyName",
    "LinkedTo",
    "LinkedToFlags",
    "LoadScreenID",
    "PortraitId",
    "Type",
    "TrapDetectable",
    "TrapDetectDC",
    "TrapDisarmable",
    "DisarmDC",
    "TrapFlag",
    "TrapOneShot",
    "TrapType",
    "OnDisarm",
    "OnTrapTriggered",
    "OnClick",
    "ScriptHeartbeat",
    "ScriptOnEnter",
    "ScriptOnExit",
    "ScriptUserDefine",
    "XPosition",
    "YPosition",
    "ZPosition",
    "XOrientation",
    "YOrientation",
    "ZOrientation",
    "Geometry",
];

const ENCOUNTER_ORDER: &[&str] = &[
    "Tag",
    "LocalizedName",
    "TemplateResRef",
    "Active",
    "Difficulty",
    "DifficultyIndex",
    "Faction",
    "MaxCreatures",
    "PlayerOnly",
    "RecCreatures",
    "Reset",
    "ResetTime",
    "Respawns",
    "SpawnOption",
    "OnEntered",
    "OnExit",
    "OnExhausted",
    "OnHeartbeat",
    "OnUserDefined",
    "CreatureList",
    "XPosition",
    "YPosition",
    "ZPosition",
    "Geometry",
    "SpawnPointList",
];

const ITEM: &[(&str, D)] = &[
    ("AddCost", D::Dword(0)),
    ("BaseItem", D::Int(0)),
    ("Charges", D::Byte(50)),
    ("Cursed", D::Byte(0)),
    ("DescIdentified", D::LocString),
    ("Description", D::LocString),
    ("Identified", D::Byte(0)),
    ("LocalizedName", D::LocString),
    ("Plot", D::Byte(0)),
    ("PropertiesList", D::List),
    ("StackSize", D::Word(1)),
    ("Stolen", D::Byte(0)),
    ("Tag", D::String),
    ("TemplateResRef", D::ResRef),
];

/// An armour's parts, in Aurora's order.
const ARMOR_PARTS: [&str; 19] = [
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
    "ArmorPart_Robe",
];
const MODEL_PARTS: [&str; 3] = ["ModelPart1", "ModelPart2", "ModelPart3"];
/// An item property's fields in Aurora's order.
const PROPERTY_ORDER: &[&str] =
    &["PropertyName", "Subtype", "CostTable", "CostValue", "Param1", "Param1Value", "ChanceAppear"];
/// A store's pages (struct ids: armour, weapons, potions and scrolls,
/// rings and amulets, miscellaneous), as a new store has them.
const STORE_PAGES: [u32; 5] = [0, 4, 2, 3, 1];
const ITEM_COLORS: [&str; 6] =
    ["Leather1Color", "Leather2Color", "Cloth1Color", "Cloth2Color", "Metal1Color", "Metal2Color"];

/// A placeable's and a door's common defaults (situated objects).
const SITUATED: &[(&str, D)] = &[
    ("Tag", D::String),
    ("LocName", D::LocString),
    ("Description", D::LocString),
    ("TemplateResRef", D::ResRef),
    ("AutoRemoveKey", D::Byte(0)),
    ("CloseLockDC", D::Byte(0)),
    ("Conversation", D::ResRef),
    ("Interruptable", D::Byte(1)),
    ("Faction", D::Dword(1)),
    ("Plot", D::Byte(0)),
    ("KeyRequired", D::Byte(0)),
    ("Lockable", D::Byte(1)),
    ("Locked", D::Byte(0)),
    ("OpenLockDC", D::Byte(0)),
    ("PortraitId", D::Word(0)),
    ("TrapDetectable", D::Byte(1)),
    ("TrapDetectDC", D::Byte(0)),
    ("TrapDisarmable", D::Byte(1)),
    ("DisarmDC", D::Byte(0)),
    ("TrapFlag", D::Byte(0)),
    ("TrapOneShot", D::Byte(1)),
    ("TrapType", D::Byte(0)),
    ("KeyName", D::String),
    ("AnimationState", D::Byte(0)),
    ("Appearance", D::Dword(0)),
    ("HP", D::Short(10)),
    ("CurrentHP", D::Short(10)),
    ("Hardness", D::Byte(5)),
    ("Fort", D::Byte(5)),
    ("Ref", D::Byte(0)),
    ("Will", D::Byte(0)),
    ("OnClosed", D::ResRef),
    ("OnDamaged", D::ResRef),
    ("OnDeath", D::ResRef),
    ("OnDisarm", D::ResRef),
    ("OnHeartbeat", D::ResRef),
    ("OnLock", D::ResRef),
    ("OnMeleeAttacked", D::ResRef),
    ("OnOpen", D::ResRef),
    ("OnSpellCastAt", D::ResRef),
    ("OnTrapTriggered", D::ResRef),
    ("OnUnlock", D::ResRef),
    ("OnUserDefined", D::ResRef),
    ("OnClick", D::ResRef),
];

/// A placeable's own defaults (`Static` follows `Useable`: see
/// [`instance`]).
const PLACEABLE: &[(&str, D)] = &[
    ("HasInventory", D::Byte(0)),
    ("BodyBag", D::Byte(0)),
    ("Type", D::Byte(0)),
    ("Useable", D::Byte(0)),
    ("OnInvDisturbed", D::ResRef),
    ("OnUsed", D::ResRef),
];

/// A door's own defaults.
const DOOR: &[(&str, D)] = &[
    ("LinkedTo", D::String),
    ("LinkedToFlags", D::Byte(0)),
    ("LoadScreenID", D::Word(0)),
    ("GenericType_New", D::Dword(0)),
    ("OnFailToOpen", D::ResRef),
];

/// The order Aurora writes a door in (a placeable: the situated fields,
/// then its own).
const DOOR_ORDER: &[&str] = &[
    "Tag",
    "LocName",
    "Description",
    "TemplateResRef",
    "AutoRemoveKey",
    "CloseLockDC",
    "Conversation",
    "Interruptable",
    "Faction",
    "Plot",
    "KeyRequired",
    "Lockable",
    "Locked",
    "OpenLockDC",
    "PortraitId",
    "TrapDetectable",
    "TrapDetectDC",
    "TrapDisarmable",
    "DisarmDC",
    "TrapFlag",
    "TrapOneShot",
    "TrapType",
    "KeyName",
    "AnimationState",
    "Appearance",
    "HP",
    "CurrentHP",
    "Hardness",
    "Fort",
    "Ref",
    "Will",
    "OnClosed",
    "OnDamaged",
    "OnDeath",
    "OnDisarm",
    "OnHeartbeat",
    "OnLock",
    "OnMeleeAttacked",
    "OnOpen",
    "OnSpellCastAt",
    "OnTrapTriggered",
    "OnUnlock",
    "OnUserDefined",
    "OnClick",
    "LinkedTo",
    "LinkedToFlags",
    "LoadScreenID",
    "GenericType_New",
    "OnFailToOpen",
    "X",
    "Y",
    "Z",
    "Bearing",
];

const STORE: &[(&str, D)] = &[
    ("BlackMarket", D::Byte(0)),
    ("BM_MarkDown", D::Int(0)),
    ("IdentifyPrice", D::Int(100)),
    ("LocName", D::LocString),
    ("MarkDown", D::Int(0)),
    ("MarkUp", D::Int(0)),
    ("MaxBuyPrice", D::Int(-1)),
    ("OnOpenStore", D::ResRef),
    ("OnStoreClosed", D::ResRef),
    ("ResRef", D::ResRef),
    ("StoreGold", D::Int(-1)),
    ("Tag", D::String),
    ("WillNotBuy", D::List),
    ("WillOnlyBuy", D::List),
];

const SOUND: &[(&str, D)] = &[
    ("Active", D::Byte(0)),
    ("Continuous", D::Byte(0)),
    ("Elevation", D::Float(0.0)),
    ("GeneratedType", D::Dword(0)),
    ("Hours", D::Dword(0)),
    ("Interval", D::Dword(0)),
    ("IntervalVrtn", D::Dword(0)),
    ("LocName", D::LocString),
    ("Looping", D::Byte(0)),
    ("MaxDistance", D::Float(0.0)),
    ("MinDistance", D::Float(0.0)),
    ("PitchVariation", D::Float(0.0)),
    ("Positional", D::Byte(0)),
    ("Random", D::Byte(0)),
    ("RandomPosition", D::Byte(0)),
    ("RandomRangeX", D::Float(0.0)),
    ("RandomRangeY", D::Float(0.0)),
    ("Sounds", D::List),
    ("Tag", D::String),
    ("TemplateResRef", D::ResRef),
    ("Times", D::Byte(0)),
    ("Volume", D::Byte(0)),
    ("VolumeVrtn", D::Byte(0)),
];

const WAYPOINT: &[(&str, D)] = &[
    ("Appearance", D::Byte(1)),
    ("Description", D::LocString),
    ("HasMapNote", D::Byte(0)),
    ("LinkedTo", D::String),
    ("LocalizedName", D::LocString),
    ("MapNote", D::LocString),
    ("MapNoteEnabled", D::Byte(1)),
    ("Tag", D::String),
    ("TemplateResRef", D::ResRef),
];

const TRIGGER: &[(&str, D)] = &[
    ("AutoRemoveKey", D::Byte(0)),
    ("Cursor", D::Byte(0)),
    ("DisarmDC", D::Byte(0)),
    ("Faction", D::Dword(0)),
    ("HighlightHeight", D::Float(0.0)),
    ("KeyName", D::String),
    ("LinkedTo", D::String),
    ("LinkedToFlags", D::Byte(0)),
    ("LoadScreenID", D::Word(0)),
    ("LocalizedName", D::LocString),
    ("OnClick", D::ResRef),
    ("OnDisarm", D::ResRef),
    ("OnTrapTriggered", D::ResRef),
    ("PortraitId", D::Word(0)),
    ("ScriptHeartbeat", D::ResRef),
    ("ScriptOnEnter", D::ResRef),
    ("ScriptOnExit", D::ResRef),
    ("ScriptUserDefine", D::ResRef),
    ("Tag", D::String),
    ("TemplateResRef", D::ResRef),
    ("TrapDetectable", D::Byte(1)),
    ("TrapDetectDC", D::Byte(0)),
    ("TrapDisarmable", D::Byte(1)),
    ("TrapFlag", D::Byte(0)),
    ("TrapOneShot", D::Byte(0)),
    ("TrapType", D::Byte(0)),
    ("Type", D::Int(0)),
];

const ENCOUNTER: &[(&str, D)] = &[
    ("Active", D::Byte(1)),
    ("CreatureList", D::List),
    ("Difficulty", D::Int(1)),
    ("DifficultyIndex", D::Int(1)),
    ("Faction", D::Dword(1)),
    ("LocalizedName", D::LocString),
    ("MaxCreatures", D::Int(4)),
    ("OnEntered", D::ResRef),
    ("OnExhausted", D::ResRef),
    ("OnExit", D::ResRef),
    ("OnHeartbeat", D::ResRef),
    ("OnUserDefined", D::ResRef),
    ("PlayerOnly", D::Byte(0)),
    ("RecCreatures", D::Int(2)),
    ("Reset", D::Byte(0)),
    ("ResetTime", D::Int(60)),
    ("Respawns", D::Int(0)),
    ("SpawnOption", D::Int(1)),
    ("SpawnPointList", D::List),
    ("Tag", D::String),
    ("TemplateResRef", D::ResRef),
];

/// Adds each default the struct lacks.
fn fill(s: &mut Struct, defaults: &[(&str, D)]) {
    for (label, d) in defaults {
        if !s.contains(label) {
            s.set(label, d.value());
        }
    }
}

/// Adds a part field's EE twin (a WORD of the same number) when missing.
fn twin(s: &mut Struct, label: &str) {
    let wide = mg_rules::items::wide_label(label);
    if !s.contains(&wide) {
        let v = s.integer(label).unwrap_or(0).clamp(0, i64::from(u16::MAX)) as u16;
        s.set(&wide, Value::Word(v));
    }
}

/// Renames an old field to its EE form (keeping its number) unless the EE
/// field is there.
fn renew(s: &mut Struct, old: &str, new: &str) {
    let v = s.integer(old);
    s.remove(old);
    if !s.contains(new)
        && let Some(v) = v
    {
        s.set(new, Value::Dword(v.clamp(0, i64::from(u32::MAX)) as u32));
    }
}

/// Puts a struct's fields in `order` (`#parts` stands for `parts`, each
/// followed by its EE twin); fields not named follow in their own order.
fn arrange(s: &mut Struct, order: &[&str], parts: &[&str]) {
    let mut labels: Vec<String> = Vec::new();
    for label in order {
        if *label == "#parts" {
            for part in parts {
                labels.push(part.to_string());
                labels.push(mg_rules::items::wide_label(part));
            }
        } else {
            labels.push(label.to_string());
        }
    }
    let rank = |f: &mg_gff::Field| {
        let l = f.label.to_string_lossy();
        labels.iter().position(|x| *x == l).unwrap_or(labels.len())
    };
    s.fields.sort_by_key(rank);
}

/// An item as a placed object holds it (in its inventory, a store page or
/// equipped), as Aurora writes it: the item blueprint's fields as placed, at
/// −1, −1, −1 facing north; containers hold theirs. `id` is its struct id
/// (its place in the list, or its equipment slot).
pub fn held(p: &Placing<'_>, bp: &Struct, id: u32) -> Struct {
    held_item(p, bp, id, 1)
}

/// An item as it is held (in an inventory, a store or equipped): its
/// fields as placed, at −1, −1, −1 facing north; containers hold theirs.
fn held_item(p: &Placing<'_>, bp: &Struct, id: u32, depth: u32) -> Struct {
    let mut s = item_fields(p, bp, depth);
    s.id = id;
    for (label, v) in [
        ("XOrientation", 0.0),
        ("YOrientation", 1.0),
        ("XPosition", -1.0),
        ("YPosition", -1.0),
        ("ZPosition", -1.0),
    ] {
        s.set(label, Value::Float(v));
    }
    let parts: Vec<&str> = MODEL_PARTS.iter().chain(&ARMOR_PARTS).copied().collect();
    let mut order: Vec<&str> = ITEM_ORDER.to_vec();
    let after_parts = order.iter().position(|l| *l == "#parts").map_or(order.len(), |i| i + 1);
    order.splice(after_parts..after_parts, ITEM_COLORS);
    arrange(&mut s, &order, &parts);
    s
}

/// An item blueprint's fields as Aurora writes an item.
fn item_fields(p: &Placing<'_>, bp: &Struct, depth: u32) -> Struct {
    let mut s = bp.clone();
    for label in ["PaletteID", "Comment"] {
        s.remove(label);
    }
    fill(&mut s, ITEM);
    // The model's fields for the base item's model type, when missing:
    // simple 0, layered 1 (with colours), composite 2, armour 3.
    let model_type = p
        .game
        .table("baseitems")
        .ok()
        .zip(usize::try_from(s.integer("BaseItem").unwrap_or(-1)).ok())
        .and_then(|(t, row)| t.get(row, "ModelType").and_then(|v| v.trim().parse::<i64>().ok()));
    let (models, colors): (&[&str], bool) = match model_type {
        Some(1) => (&MODEL_PARTS[..1], true),
        Some(2) => (&MODEL_PARTS, false),
        Some(3) => (&ARMOR_PARTS, true),
        _ => (&MODEL_PARTS[..1], false),
    };
    let present = |s: &Struct, labels: &[&str]| labels.iter().any(|l| s.contains(l));
    if !present(&s, &MODEL_PARTS) && !present(&s, &ARMOR_PARTS) {
        for label in models {
            s.set(label, Value::Byte(0));
        }
    }
    if colors && !present(&s, &ITEM_COLORS) {
        for label in ITEM_COLORS {
            s.set(label, Value::Byte(0));
        }
    }
    // An armour has every part.
    if present(&s, &ARMOR_PARTS) {
        for label in ARMOR_PARTS {
            if !s.contains(label) {
                s.set(label, Value::Byte(0));
            }
        }
    }
    for label in MODEL_PARTS.iter().chain(&ARMOR_PARTS) {
        if s.contains(label) {
            twin(&mut s, label);
        }
    }
    if !s.contains("Cost") {
        let cost = p.game.item_cost(&mg_rules::ItemValue::from_gff(&s));
        s.set("Cost", Value::Dword(cost));
    }
    held_list(p, &mut s, "ItemList", depth);
    let mut properties = s.list("PropertiesList").unwrap_or(&[]).to_vec();
    for property in &mut properties {
        arrange(property, PROPERTY_ORDER, &[]);
    }
    s.set("PropertiesList", Value::List(properties));
    let parts: Vec<&str> = MODEL_PARTS.iter().chain(&ARMOR_PARTS).copied().collect();
    let mut order: Vec<&str> = ITEM_ORDER.to_vec();
    let after_parts = order.iter().position(|l| *l == "#parts").map_or(order.len(), |i| i + 1);
    order.splice(after_parts..after_parts, ITEM_COLORS);
    arrange(&mut s, &order, &parts);
    s
}

/// Expands a list of held items (`InventoryRes` entries) in place: each
/// entry becomes the item, keeping the entry's own fields (its place in
/// the grid, `Dropable`, `Infinite`); struct ids number them. Items whose
/// blueprint is not found are left out.
fn held_list(p: &Placing<'_>, s: &mut Struct, list: &str, depth: u32) {
    let Some(entries) = s.list(list) else { return };
    if depth > 4 {
        return;
    }
    let mut out = Vec::new();
    for e in entries {
        let Some(r) = e.resref("InventoryRes") else {
            // Already an item (placed by Aurora): keep it.
            out.push(e.clone());
            continue;
        };
        let Some(bp) = (p.item)(r) else { continue };
        let mut item = held_item(p, &bp, out.len() as u32, depth + 1);
        for f in &e.fields {
            let label = f.label.to_string_lossy();
            if label != "InventoryRes" {
                item.set(&label, f.value.clone());
            }
        }
        out.push(item);
    }
    s.set(list, Value::List(out));
}

/// The instance of a blueprint of `restype` at `at`, as Aurora places it.
/// Triggers and encounters take their outline from `outline`: points
/// relative to the position in X and Y, and at their own height (the
/// ground under them plus [`OUTLINE_LIFT`]); Aurora stands them at their
/// first point, at height 0.
pub fn instance(
    p: &Placing<'_>,
    restype: ResType,
    blueprint: &Struct,
    at: Placement,
    outline: &[[f32; 3]],
) -> Option<Struct> {
    let (_, id) = git_list(restype)?;
    let mut s = blueprint.clone();
    s.id = id;
    for label in ["PaletteID", "Comment"] {
        s.remove(label);
    }
    let [x, y, z] = at.position;
    let facing = at.rotation + FRAC_PI_2;
    let f = Value::Float;
    let orientation = |s: &mut Struct| {
        for (label, v) in [
            ("XPosition", x),
            ("YPosition", y),
            ("ZPosition", z),
            ("XOrientation", facing.cos()),
            ("YOrientation", facing.sin()),
        ] {
            s.set(label, f(v));
        }
    };
    match restype {
        ResType::UTC => {
            renew(&mut s, "Tail", "Tail_New");
            renew(&mut s, "Wings", "Wings_New");
            fill(&mut s, CREATURE);
            if part_based(p.game, &s) {
                for label in CREATURE_PARTS {
                    if !s.contains(label) {
                        s.set(label, Value::Byte(0));
                    }
                    twin(&mut s, label);
                }
                for label in CREATURE_COLORS {
                    if !s.contains(label) {
                        s.set(label, Value::Byte(0));
                    }
                }
            }
            // Every skill, in skills.2da order.
            let skills = p.game.table("skills").map_or(0, |t| t.len());
            let mut list = s.list("SkillList").unwrap_or(&[]).to_vec();
            while list.len() < skills {
                let mut rank = Struct::new(0);
                rank.set("Rank", Value::Byte(0));
                list.push(rank);
            }
            s.set("SkillList", Value::List(list));
            // Equipment: each slot's item in full.
            let equipped: Vec<Struct> = s
                .list("Equip_ItemList")
                .unwrap_or(&[])
                .iter()
                .filter_map(|e| match e.resref("EquippedRes") {
                    Some(r) => (p.item)(r).map(|bp| held_item(p, &bp, e.id, 1)),
                    None => Some(e.clone()),
                })
                .collect();
            s.set("Equip_ItemList", Value::List(equipped));
            held_list(p, &mut s, "ItemList", 0);
            orientation(&mut s);
        }
        ResType::UTI => {
            s = item_fields(p, &s, 0);
            s.id = id;
            orientation(&mut s);
            let parts: Vec<&str> = MODEL_PARTS.iter().chain(&ARMOR_PARTS).copied().collect();
            let mut order: Vec<&str> = ITEM_ORDER.to_vec();
            let after = order.iter().position(|l| *l == "#parts").map_or(order.len(), |i| i + 1);
            order.splice(after..after, ITEM_COLORS);
            arrange(&mut s, &order, &parts);
            return Some(s);
        }
        ResType::UTM => {
            s.remove("ID");
            fill(&mut s, STORE);
            if !s.contains("StoreList") {
                let pages = STORE_PAGES.iter().map(|&id| Struct::new(id)).collect();
                s.set("StoreList", Value::List(pages));
            }
            let mut pages = s.list("StoreList").unwrap_or(&[]).to_vec();
            for page in &mut pages {
                held_list(p, page, "ItemList", 0);
            }
            s.set("StoreList", Value::List(pages));
            orientation(&mut s);
        }
        ResType::UTP => {
            s.remove("Portrait");
            fill(&mut s, SITUATED);
            fill(&mut s, PLACEABLE);
            // A placeable without `Static` is static unless it is usable.
            if !s.contains("Static") {
                let usable = s.integer("Useable").unwrap_or(0) != 0;
                s.set("Static", Value::Byte(u8::from(!usable)));
            }
            held_list(p, &mut s, "ItemList", 0);
            for (label, v) in [("X", x), ("Y", y), ("Z", z), ("Bearing", at.rotation)] {
                s.set(label, f(v));
            }
        }
        ResType::UTD => {
            renew(&mut s, "GenericType", "GenericType_New");
            s.remove("Portrait");
            fill(&mut s, SITUATED);
            fill(&mut s, DOOR);
            for (label, v) in [("X", x), ("Y", y), ("Z", z), ("Bearing", at.rotation)] {
                s.set(label, f(v));
            }
        }
        ResType::UTS => {
            fill(&mut s, SOUND);
            let looping = s.integer("Looping").unwrap_or(0) != 0;
            let positional = s.integer("Positional").unwrap_or(0) != 0;
            let priority = match (looping, positional) {
                (true, false) => 2,
                (true, true) => 3,
                (false, false) => 19,
                (false, true) => 20,
            };
            s.set("Priority", Value::Byte(priority));
            let mut sounds = s.list("Sounds").unwrap_or(&[]).to_vec();
            sounds.iter_mut().for_each(|w| w.id = 0);
            s.set("Sounds", Value::List(sounds));
            for (label, v) in [("XPosition", x), ("YPosition", y), ("ZPosition", z)] {
                s.set(label, f(v));
            }
        }
        ResType::UTW => {
            fill(&mut s, WAYPOINT);
            orientation(&mut s);
        }
        ResType::UTT => {
            for label in ["PartyRequired", "Portrait"] {
                s.remove(label);
            }
            fill(&mut s, TRIGGER);
            for (label, v) in [
                ("XPosition", x),
                ("YPosition", y),
                ("ZPosition", 0.0),
                ("XOrientation", 0.0),
                ("YOrientation", 0.0),
                ("ZOrientation", 0.0),
            ] {
                s.set(label, f(v));
            }
            s.set("Geometry", Value::List(points(outline, 3, ["PointX", "PointY", "PointZ"])));
        }
        ResType::UTE => {
            fill(&mut s, ENCOUNTER);
            // Creatures by challenge rating (then resref).
            let mut creatures = s.list("CreatureList").unwrap_or(&[]).to_vec();
            creatures.sort_by(|a, b| {
                let cr = |c: &Struct| c.float("CR").unwrap_or(0.0);
                let r = |c: &Struct| c.resref("ResRef").map(|r| r.to_string()).unwrap_or_default();
                cr(a).total_cmp(&cr(b)).then_with(|| r(a).cmp(&r(b)))
            });
            s.set("CreatureList", Value::List(creatures));
            for (label, v) in [("XPosition", x), ("YPosition", y), ("ZPosition", 0.0)] {
                s.set(label, f(v));
            }
            s.set("Geometry", Value::List(points(outline, 1, ["X", "Y", "Z"])));
        }
        _ => return None,
    }
    let order: Vec<&str> = match restype {
        ResType::UTC => {
            let mut order = CREATURE_ORDER.to_vec();
            let after = order.iter().position(|l| *l == "#parts").map_or(order.len(), |i| i + 1);
            order.splice(after..after, CREATURE_COLORS);
            order
        }
        ResType::UTP => PLACEABLE_ORDER.to_vec(),
        ResType::UTM => STORE_ORDER.to_vec(),
        ResType::UTS => SOUND_ORDER.to_vec(),
        ResType::UTW => WAYPOINT_ORDER.to_vec(),
        ResType::UTT => TRIGGER_ORDER.to_vec(),
        ResType::UTE => ENCOUNTER_ORDER.to_vec(),
        ResType::UTD => DOOR_ORDER.to_vec(),
        // Items are in order already.
        _ => return Some(s),
    };
    arrange(&mut s, &order, &CREATURE_PARTS);
    Some(s)
}

/// The waypoint Aurora's Create Waypoint makes for a creature (one of its
/// walk waypoints): from no blueprint, facing north, at `position`.
pub fn walk_waypoint(tag: &str, position: [f32; 3]) -> Struct {
    let (_, id) = git_list(ResType::UTW).expect("waypoints have a list");
    let mut s = Struct::new(id);
    let text = |t: &str| Value::String(t.as_bytes().to_vec());
    let none = || Value::LocString(LocString::default());
    for (label, value) in [
        ("Appearance", Value::Byte(1)),
        ("LinkedTo", text("")),
        ("TemplateResRef", Value::resref(ResRef::EMPTY)),
        ("Tag", text(tag)),
        ("LocalizedName", none()),
        ("Description", none()),
        ("HasMapNote", Value::Byte(0)),
        ("MapNote", none()),
        ("MapNoteEnabled", Value::Byte(0)),
        ("XPosition", Value::Float(position[0])),
        ("YPosition", Value::Float(position[1])),
        ("ZPosition", Value::Float(position[2])),
        ("XOrientation", Value::Float(0.0)),
        ("YOrientation", Value::Float(1.0)),
    ] {
        s.set(label, value);
    }
    s
}

/// The tag of the next waypoint in a set named `name` (`<name>_01`, the
/// first number not among `tags`): Create Waypoint's `WP_<creature tag>`
/// and Create Set's names.
pub fn set_tag(name: &str, tags: &[String]) -> String {
    (1..1000)
        .map(|n| format!("{name}_{n:02}"))
        .find(|t| !tags.iter().any(|x| x.eq_ignore_ascii_case(t)))
        .unwrap_or_else(|| format!("{name}_01"))
}

/// An outline's points as GIT structs.
fn points(outline: &[[f32; 3]], id: u32, labels: [&str; 3]) -> Vec<Struct> {
    outline
        .iter()
        .map(|c| {
            let mut s = Struct::new(id);
            for (label, v) in labels.iter().zip(c) {
                s.set(label, Value::Float(*v));
            }
            s
        })
        .collect()
}

/// Whether a creature's appearance is part-based (appearance.2da
/// `MODELTYPE` P).
fn part_based(game: &GameData, s: &Struct) -> bool {
    let row = mg_rules::items::part_number(s, "Appearance_Type").unwrap_or(-1);
    let Ok(t) = game.table("appearance") else { return false };
    usize::try_from(row)
        .ok()
        .and_then(|r| t.get(r, "MODELTYPE"))
        .is_some_and(|m| m.trim().eq_ignore_ascii_case("P"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::Language;
    use mg_resman::ResMan;
    use mg_tlk::Tlk;

    fn game() -> GameData {
        GameData::new(ResMan::new(), Tlk::new(Language::ENGLISH))
    }

    fn labels(s: &Struct) -> Vec<String> {
        s.fields.iter().map(|f| f.label.to_string_lossy()).collect()
    }

    #[test]
    fn instances_stand_where_placed_sorted() {
        let game = game();
        let none = |_: ResRef| None;
        let p = Placing { game: &game, item: &none };
        let mut bp = Struct::new(0);
        bp.set("Tag", Value::String(b"DOOR".to_vec()));
        bp.set("PaletteID", Value::Byte(3));
        bp.set("GenericType", Value::Byte(4));
        let at = Placement { position: [1.0, 2.0, 0.5], rotation: 1.5 };
        let d = instance(&p, ResType::UTD, &bp, at, &[]).unwrap();
        assert_eq!(d.id, 8);
        assert_eq!((d.float("X"), d.float("Bearing")), (Some(1.0), Some(1.5)));
        assert_eq!(d.get("GenericType_New"), Some(&Value::Dword(4)));
        assert!(!d.contains("PaletteID") && !d.contains("GenericType"));
        let l = labels(&d);
        assert_eq!(l[0], "Tag");
        assert_eq!(l[l.len() - 4..], ["X", "Y", "Z", "Bearing"]);
        // A waypoint turned by nothing faces north.
        let w = instance(&p, ResType::UTW, &bp, Placement::default(), &[]).unwrap();
        assert!(w.float("XOrientation").unwrap().abs() < 1e-6);
        assert_eq!(w.float("YOrientation"), Some(1.0));
        assert_eq!(w.get("MapNoteEnabled"), Some(&Value::Byte(1)));
    }

    #[test]
    fn outlines_and_sounds() {
        let game = game();
        let none = |_: ResRef| None;
        let p = Placing { game: &game, item: &none };
        let square = [[0.0, 0.0, 0.025], [2.0, 0.0, 0.025], [2.0, 2.0, 0.3]];
        let at = Placement { position: [5.0, 6.0, 1.0], rotation: 0.0 };
        let t = instance(&p, ResType::UTT, &Struct::new(0), at, &square).unwrap();
        let g = t.list("Geometry").unwrap();
        assert_eq!((g.len(), g[1].id, g[2].float("PointZ")), (3, 3, Some(0.3)));
        assert_eq!(t.float("ZPosition"), Some(0.0), "triggers stand at height 0");
        let e = instance(&p, ResType::UTE, &Struct::new(0), at, &square).unwrap();
        assert!(e.list("SpawnPointList").unwrap().is_empty(), "Aurora adds no spawn point");
        let mut s = Struct::new(0);
        s.set("Positional", Value::Byte(1));
        s.set("Priority", Value::Byte(18));
        let mut wave = Struct::new(1_243_572);
        wave.set("Sound", Value::resref(ResRef::from_str("as_an_cryday1").unwrap()));
        s.set("Sounds", Value::List(vec![wave]));
        let placed = instance(&p, ResType::UTS, &s, at, &[]).unwrap();
        assert_eq!(placed.get("Priority"), Some(&Value::Byte(20)));
        assert_eq!(placed.list("Sounds").unwrap()[0].id, 0);
    }

    #[test]
    fn inventories_hold_whole_items() {
        let game = game();
        let potion = |r: ResRef| {
            (r.to_string() == "potion").then(|| {
                let mut s = Struct::new(0);
                s.set("BaseItem", Value::Int(49));
                s.set("ModelPart1", Value::Byte(21));
                s.set("Cost", Value::Dword(20));
                s.set("Comment", Value::String(b"1".to_vec()));
                s
            })
        };
        let p = Placing { game: &game, item: &potion };
        let entry = |r: &str, x: u16| {
            let mut e = Struct::new(0);
            e.set("InventoryRes", Value::resref(ResRef::from_str(r).unwrap()));
            e.set("Repos_PosX", Value::Word(x));
            e.set("Repos_Posy", Value::Word(0));
            e
        };
        let mut chest = Struct::new(0);
        chest.set(
            "ItemList",
            Value::List(vec![entry("potion", 0), entry("missing", 1), entry("potion", 2)]),
        );
        let placed = instance(&p, ResType::UTP, &chest, Placement::default(), &[]).unwrap();
        let items = placed.list("ItemList").unwrap();
        assert_eq!(items.len(), 2, "an item without a blueprint is left out");
        assert_eq!((items[0].id, items[1].id), (0, 1));
        assert_eq!(items[1].get("Repos_PosX"), Some(&Value::Word(2)));
        assert_eq!(items[0].get("xModelPart1"), Some(&Value::Word(21)));
        assert_eq!(items[0].float("ZPosition"), Some(-1.0));
        assert!(!items[0].contains("Comment") && !items[0].contains("InventoryRes"));
        // Aurora's order: where it is first, where it sits in the grid last.
        let names = labels(&items[0]);
        assert_eq!(names[..2], ["XPosition", "YPosition"]);
        assert_eq!(names[names.len() - 2..], ["Repos_PosX", "Repos_Posy"]);
    }
}
