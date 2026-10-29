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
//! - where it stands; all in the order Aurora writes each type, local
//!   variables included (`aurora_update_instances.rs`; fields it does not
//!   know follow, kept).
//!
//! Aurora's two quirks with blueprints lacking `Comment` (a creature then
//! gets an empty `Comment`, and its first skill another) are not copied.

use mg_core::{LocString, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_rules::{GameData, ObjectKind};

pub use crate::placement::Placement;

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

/// The GIT list a blueprint type's instances go in, and their struct id
/// ([`ObjectKind::list`], [`ObjectKind::struct_id`]).
pub fn git_list(restype: ResType) -> Option<(&'static str, u32)> {
    ObjectKind::from_restype(restype).map(|k| (k.list(), k.struct_id()))
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

/// A part-based creature's parts, as a new blueprint has them, and its
/// colours.
fn creature_parts() -> [&'static str; 19] {
    crate::blueprints::CREATURE_PARTS.map(|(label, _)| label)
}
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
    "VarTable",
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
    "VarTable",
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
    "VarTable",
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
    "VisTransformList",
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
    "VarTable",
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
    "VarTable",
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
    "VarTable",
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
    "VarTable",
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
    "VarTable",
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

/// An armour's parts, in Aurora's order (the game's).
const ARMOR_PARTS: [&str; 19] = {
    let mut fields = [""; 19];
    let mut i = 0;
    while i < fields.len() {
        fields[i] = mg_rules::items::ARMOR_PARTS[i].0;
        i += 1;
    }
    fields
};
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
    "VarTable",
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

/// An item's fields in Aurora's order: its parts and then its colors where
/// the order has `#parts`.
fn arrange_item(s: &mut Struct) {
    let parts: Vec<&str> = MODEL_PARTS.iter().chain(&ARMOR_PARTS).copied().collect();
    let mut order: Vec<&str> = ITEM_ORDER.to_vec();
    let after_parts = order.iter().position(|l| *l == "#parts").map_or(order.len(), |i| i + 1);
    order.splice(after_parts..after_parts, ITEM_COLORS);
    arrange(s, &order, &parts);
}

/// An item as a placed object holds it (in its inventory, a store page or
/// equipped), as Aurora writes it: the item blueprint's fields as placed, at
/// −1, −1, −1 facing north; containers hold theirs. `id` is its struct id
/// (its place in the list, or its equipment slot).
pub fn held(p: &Placing<'_>, bp: &Struct, id: u32) -> Struct {
    held_item(p, bp, id, 1)
}

/// The fields that say where an item is held in an inventory, which a
/// move changes: its place in the grid. (Its struct id too: its place in
/// a list, or the slot it is equipped in.)
pub const PLACE_FIELDS: [&str; 2] = ["Repos_PosX", "Repos_Posy"];

/// Renames field `from` of `s` to `to` where it is, keeping the order.
fn rename_field(s: &mut Struct, from: &str, to: &str) {
    if s.contains(to) {
        return;
    }
    if let Some(f) = s.fields.iter_mut().find(|f| f.label.as_bytes() == from.as_bytes()) {
        f.label = mg_gff::Label::new(to).expect("a short label");
    }
}

/// `item` (from a backpack, a slot, a container or a store) as an item of
/// a list of items (`ItemList`): item `id` of it, at `x`, `y` in the grid;
/// all else as it was (a blueprint's `EquippedRes` named `InventoryRes`),
/// as Aurora moves an item into a creature's backpack.
pub fn listed_item(item: &Struct, id: u32, x: u16, y: u16) -> Struct {
    let mut s = item.clone();
    s.id = id;
    rename_field(&mut s, "EquippedRes", "InventoryRes");
    s.set("Repos_PosX", Value::Word(x));
    s.set("Repos_Posy", Value::Word(y));
    s
}

/// `item` as equipped in slot `bit` (its struct id): without a place in
/// the grid, all else as it was (a blueprint's `InventoryRes` named
/// `EquippedRes`), as the game's modules hold them and Aurora equips an
/// item from the backpack.
pub fn equipped_item(item: &Struct, bit: u32) -> Struct {
    let mut s = item.clone();
    s.id = bit;
    rename_field(&mut s, "InventoryRes", "EquippedRes");
    for label in PLACE_FIELDS {
        s.remove(label);
    }
    s
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
    arrange_item(&mut s);
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
        // Aurora writes every property's struct id as 0 (a few game
        // blueprints have others: nw_storgenral003's items).
        property.id = 0;
        arrange(property, PROPERTY_ORDER, &[]);
    }
    s.set("PropertiesList", Value::List(properties));
    // (Its fields are put in Aurora's order by who asked, once the fields
    // of the item's place are in.)
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
    let kind = ObjectKind::from_restype(restype)?;
    let id = kind.struct_id();
    let mut s = blueprint.clone();
    s.id = id;
    for label in ["PaletteID", "Comment"] {
        s.remove(label);
    }
    let [x, y, _] = at.position;
    let f = Value::Float;
    // (Where it stands, as its kind keeps it: `placement`.)
    let orientation = |s: &mut Struct| {
        for (label, v) in at.fields(kind) {
            s.set(label, f(v));
        }
    };
    match restype {
        ResType::UTC => {
            renew(&mut s, "Tail", "Tail_New");
            renew(&mut s, "Wings", "Wings_New");
            fill(&mut s, CREATURE);
            if part_based(p.game, &s) {
                for label in creature_parts() {
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
            // Equipment: each slot's item in full, keeping the entry's own
            // fields (`Dropable`, `Pickpocketable`), as the backpack's do.
            let equipped: Vec<Struct> = s
                .list("Equip_ItemList")
                .unwrap_or(&[])
                .iter()
                .filter_map(|e| match e.resref("EquippedRes") {
                    Some(r) => (p.item)(r).map(|bp| {
                        let mut item = held_item(p, &bp, e.id, 1);
                        for f in &e.fields {
                            let label = f.label.to_string_lossy();
                            if label != "EquippedRes" {
                                item.set(&label, f.value.clone());
                            }
                        }
                        item
                    }),
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
            arrange_item(&mut s);
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
            orientation(&mut s);
        }
        ResType::UTD => {
            renew(&mut s, "GenericType", "GenericType_New");
            s.remove("Portrait");
            fill(&mut s, SITUATED);
            fill(&mut s, DOOR);
            orientation(&mut s);
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
            orientation(&mut s);
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
    arrange_as(restype, &mut s);
    Some(s)
}

/// Puts a placed object's fields in the order Aurora writes its type
/// (items are written in order).
fn arrange_as(restype: ResType, s: &mut Struct) {
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
        _ => return,
    };
    arrange(s, &order, &creature_parts());
}

/// Where a placed object stands, as [`instance`] writes it (the inverse
/// of its placement), and its outline (triggers' and encounters'
/// `Geometry`, as stored).
pub fn placement_of(restype: ResType, s: &Struct) -> (Placement, Vec<[f32; 3]>) {
    let Some(kind) = ObjectKind::from_restype(restype) else { return Default::default() };
    (Placement::read(kind, s), crate::placement::outline(kind, s))
}

/// What Update Instances keeps of a placed object besides where it
/// stands: its visual transform (EE's list, or the older struct) and an
/// encounter's spawn points.
const KEPT: [&str; 3] = ["VisTransformList", "VisualTransform", "SpawnPointList"];

/// Update Instances (Aurora's palette › Update Instances, with every area
/// or one): the placed objects of the blueprints `blueprint` returns,
/// made again from them where they stand, as Aurora does
/// (`aurora_update_instances.rs`): everything else (tag, name, scripts,
/// variables, a door's transition) comes from the blueprint. A store's
/// blueprint is named by `ResRef`, the rest by `TemplateResRef`.
///
/// `which` picks objects by GIT list and index (Aurora lists them to
/// untick). Returns the GIT with the objects replaced, and how many, or
/// `None` when it has none of them.
pub fn update(
    p: &Placing<'_>,
    git: &Struct,
    blueprint: &dyn Fn(ResType, ResRef) -> Option<Struct>,
    which: &dyn Fn(&str, usize) -> bool,
) -> Option<(Struct, usize)> {
    let mut out = git.clone();
    let mut count = 0;
    for kind in ObjectKind::ALL {
        let (list, restype, field) = (kind.list(), kind.restype(), kind.resref_field());
        let Some(items) = out.list_mut(list) else { continue };
        for (i, s) in items.iter_mut().enumerate() {
            if !which(list, i) {
                continue;
            }
            let Some(bp) =
                s.resref(field).filter(|r| !r.is_empty()).and_then(|r| blueprint(restype, r))
            else {
                continue;
            };
            let Some(made) = remade(p, restype, s, &bp) else { continue };
            *s = made;
            count += 1;
        }
    }
    (count > 0).then_some((out, count))
}

/// Placed object `old` (of blueprint type `restype`) made again from
/// blueprint `bp`, as Update Instances makes it: where it stands, facing
/// as it faces, with its outline (a trigger's or encounter's `Geometry`),
/// its visual transform and an encounter's spawn points; everything else
/// (tag, name, scripts, variables, a door's transition) the blueprint's.
pub fn remade(p: &Placing<'_>, restype: ResType, old: &Struct, bp: &Struct) -> Option<Struct> {
    let (at, outline) = placement_of(restype, old);
    let mut made = instance(p, restype, bp, at, &outline)?;
    for label in KEPT {
        if let Some(v) = old.get(label) {
            made.set(label, v.clone());
        }
    }
    arrange_as(restype, &mut made);
    Some(made)
}

/// Replace Selected with This: placed object `old` made anew from another
/// blueprint `bp` of its type, kept as [`remade`] keeps it (Aurora has no
/// such command; its Update Instances is the same remaking,
/// `aurora_update_instances.rs`). A placeable the blueprint makes static
/// takes no visual transform, as Static ticked in its Properties takes it
/// off: the game draws a static placeable without one.
pub fn replaced(p: &Placing<'_>, restype: ResType, old: &Struct, bp: &Struct) -> Option<Struct> {
    let mut made = remade(p, restype, old, bp)?;
    if restype == ResType::UTP && made.integer("Static").unwrap_or(0) != 0 {
        for label in crate::areas::VISUAL_TRANSFORM_FIELDS {
            made.remove(label);
        }
    }
    Some(made)
}

/// The fields of a placed object's name: a creature's first and last
/// names, the others' one name.
const NAME_FIELDS: [&str; 4] = ["LocName", "LocalizedName", "FirstName", "LastName"];

/// Whether a field is one of an object's event scripts (`OnUsed`,
/// `ScriptHeartbeat`, a trap's `OnTrapTriggered`, a store's
/// `OnOpenStore`...): a resref named so.
fn is_script(label: &str, v: &Value) -> bool {
    matches!(v, Value::ResRef(_)) && (label.starts_with("On") || label.starts_with("Script"))
}

/// [`replaced`], keeping the old object's own tag, name, scripts and
/// variables: a placeholder replaced by the thing it stood for keeps what
/// the scripts know it by (Replace Selected with This, Keeping Tags and
/// Scripts). Everything else is the blueprint's; a script the old object
/// had no field for is left empty, and variables it had none of are none.
pub fn replaced_keeping_own(
    p: &Placing<'_>,
    restype: ResType,
    old: &Struct,
    bp: &Struct,
) -> Option<Struct> {
    let mut made = replaced(p, restype, old, bp)?;
    for label in std::iter::once("Tag").chain(NAME_FIELDS) {
        if let Some(v) = old.get(label) {
            made.set(label, v.clone());
        }
    }
    let scripts = |s: &Struct| -> Vec<(String, Value)> {
        (s.fields.iter())
            .map(|f| (f.label.to_string_lossy(), &f.value))
            .filter(|(label, v)| is_script(label, v))
            .map(|(label, v)| (label, v.clone()))
            .collect()
    };
    for (label, _) in scripts(&made) {
        made.set(&label, Value::resref(ResRef::EMPTY));
    }
    for (label, v) in scripts(old) {
        made.set(&label, v);
    }
    match old.get("VarTable") {
        Some(v) => made.set("VarTable", v.clone()),
        None => {
            made.remove("VarTable");
        }
    }
    Some(made)
}

/// Where an item is held in a GIT: each list and index down to it from
/// the GIT's root: the placed object that holds it (`Placeable List` 3),
/// then where in that (`ItemList` 0; a creature's `Equip_ItemList`; a
/// store's `StoreList` page and its `ItemList`; a bag's `ItemList` in
/// any of them, or of a bag lying in the area).
pub type Held = Vec<(&'static str, usize)>;

/// Picks held items by where they are ([`Held`]).
pub type WhichHeld<'a> = &'a dyn Fn(&[(&'static str, usize)]) -> bool;

/// What an item keeps where it is held, when it is made again from its
/// blueprint: its place in the grid, whether it drops or can be stolen,
/// and whether a store has no end of it. (The slot it is equipped in is
/// its struct's id, kept too.) How many there are is the blueprint's, as
/// Aurora's Update Instances makes them (captured 2026-10-29: a chest's
/// three and a creature's two became the blueprint's one).
const HELD_KEPT: [&str; 5] = ["Repos_PosX", "Repos_Posy", "Dropable", "Pickpocketable", "Infinite"];

/// The lists of held items a struct of a GIT list (or an item) has.
fn held_lists(list: &str) -> &'static [&'static str] {
    match list {
        "Creature List" => &["Equip_ItemList", "ItemList"],
        // (A store's `StoreList` is its pages, each with an `ItemList`.)
        "StoreList" => &["StoreList"],
        "Placeable List" | "List" | "ItemList" | "Equip_ItemList" => &["ItemList"],
        _ => &[],
    }
}

/// Whether the structs of `list` under one of `parent`'s are items (a
/// store's pages are not: their `ItemList`s are).
fn holds_items(parent: &str, list: &str) -> bool {
    !(parent == "StoreList" && list == "StoreList")
}

/// Walks the items held under the struct `s` of `list`, which is at `at`.
fn walk_held(
    s: &Struct,
    list: &'static str,
    at: &mut Held,
    depth: u32,
    found: &mut dyn FnMut(&Held, &Struct),
) {
    if depth > 6 {
        return;
    }
    for &inner in held_lists(list) {
        for (i, held) in s.list(inner).unwrap_or(&[]).iter().enumerate() {
            at.push((inner, i));
            if holds_items(list, inner) {
                found(at, held);
                walk_held(held, "ItemList", at, depth + 1, found);
            } else {
                // (A store's page.)
                walk_held(held, "Placeable List", at, depth + 1, found);
            }
            at.pop();
        }
    }
}

/// The items held by a GIT's placed objects (in chests, creatures' packs
/// and hands, stores' pages, and bags among them or lying in the area)
/// that are of the item blueprints `wanted` names: where each is, its
/// blueprint and its tag.
pub fn held_items(git: &Struct, wanted: &dyn Fn(ResRef) -> bool) -> Vec<(Held, ResRef, String)> {
    let mut out = Vec::new();
    for list in ObjectKind::ALL.map(ObjectKind::list) {
        for (i, s) in git.list(list).unwrap_or(&[]).iter().enumerate() {
            let mut at: Held = vec![(list, i)];
            walk_held(s, list, &mut at, 0, &mut |at, item| {
                if let Some(r) = item.resref("TemplateResRef").filter(|r| wanted(*r)) {
                    let tag = String::from_utf8_lossy(item.string("Tag").unwrap_or_default());
                    out.push((at.clone(), r, tag.into_owned()));
                }
            });
        }
    }
    out
}

/// [`update`] for the items a GIT's placed objects hold
/// ([`held_items`]): each that `which` picks (by where it is) made again
/// from its blueprint, keeping where it is held (its place in the grid,
/// its slot) and what is said of it there (`Dropable`, `Pickpocketable`,
/// a store's `Infinite`); how many there are (`StackSize`) is the
/// blueprint's, as Aurora's Update Instances has it. A bag made again holds what its blueprint
/// does. Returns the GIT with them replaced, and how many, or `None`
/// when it has none of them.
pub fn update_held(
    p: &Placing<'_>,
    git: &Struct,
    blueprint: &dyn Fn(ResRef) -> Option<Struct>,
    which: WhichHeld<'_>,
) -> Option<(Struct, usize)> {
    #[allow(clippy::too_many_arguments)]
    fn remake(
        p: &Placing<'_>,
        s: &mut Struct,
        list: &'static str,
        at: &mut Held,
        depth: u32,
        blueprint: &dyn Fn(ResRef) -> Option<Struct>,
        which: WhichHeld<'_>,
        count: &mut usize,
    ) {
        if depth > 6 {
            return;
        }
        for &inner in held_lists(list) {
            let Some(items) = s.list_mut(inner) else { continue };
            for (i, held) in items.iter_mut().enumerate() {
                at.push((inner, i));
                if !holds_items(list, inner) {
                    remake(p, held, "Placeable List", at, depth + 1, blueprint, which, count);
                } else if which(at)
                    && let Some(bp) = held.resref("TemplateResRef").and_then(blueprint)
                {
                    let mut made = held_item(p, &bp, held.id, depth + 1);
                    // (In the order it has them.)
                    for f in &held.fields {
                        let label = f.label.to_string_lossy();
                        if HELD_KEPT.contains(&label.as_str()) {
                            made.set(&label, f.value.clone());
                        }
                    }
                    *held = made;
                    *count += 1;
                } else {
                    remake(p, held, "ItemList", at, depth + 1, blueprint, which, count);
                }
                at.pop();
            }
        }
    }
    let mut out = git.clone();
    let mut count = 0;
    for list in ObjectKind::ALL.map(ObjectKind::list) {
        let Some(objects) = out.list_mut(list) else { continue };
        for (i, s) in objects.iter_mut().enumerate() {
            let mut at: Held = vec![(list, i)];
            remake(p, s, list, &mut at, 0, blueprint, which, &mut count);
        }
    }
    (count > 0).then_some((out, count))
}

/// The waypoint Aurora's Create Waypoint makes for a creature (one of its
/// walk waypoints): from no blueprint, facing north, at `position`; `tag`
/// in the module's codepage.
pub fn walk_waypoint(tag: &[u8], position: [f32; 3]) -> Struct {
    let id = ObjectKind::Waypoint.struct_id();
    let mut s = Struct::new(id);
    let text = |t: &str| Value::String(t.as_bytes().to_vec());
    let none = || Value::LocString(LocString::default());
    for (label, value) in [
        ("Appearance", Value::Byte(1)),
        ("LinkedTo", text("")),
        ("TemplateResRef", Value::resref(ResRef::EMPTY)),
        ("Tag", Value::String(tag.to_vec())),
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

    /// Replace Selected with This: a trigger keeps its outline and where it
    /// stands, an encounter its spawn points, a placeable its place,
    /// facing and visual transform; the tag, scripts and variables are the
    /// new blueprint's. A static placeable takes no transform.
    #[test]
    fn replaced_objects_keep_where_they_stand_and_their_shape() {
        let game = game();
        let none = |_: ResRef| None;
        let p = Placing { game: &game, item: &none };
        let text = |s: &str| Value::String(s.as_bytes().to_vec());
        let blueprint = |tag: &str| {
            let mut bp = Struct::new(0);
            bp.set("Tag", text(tag));
            bp.set("TemplateResRef", Value::resref(ResRef::from_str(&tag.to_lowercase()).unwrap()));
            bp.set("OnUsed", Value::resref(ResRef::from_str("new_script").unwrap()));
            bp
        };
        let mut old_bp = blueprint("OLD");
        let mut var = Struct::new(0);
        var.set("Name", text("nOld"));
        old_bp.set("VarTable", Value::List(vec![var]));
        let square = [[0.0, 0.0, 0.025], [2.0, 0.0, 0.025], [2.0, 2.0, 0.3]];
        let at = Placement { position: [5.0, 6.0, 1.0], rotation: 0.7 };

        let trigger = instance(&p, ResType::UTT, &old_bp, at, &square).unwrap();
        let new = replaced(&p, ResType::UTT, &trigger, &blueprint("NEW")).unwrap();
        assert_eq!(new.string("Tag"), Some(&b"NEW"[..]));
        assert_eq!(new.list("Geometry"), trigger.list("Geometry"), "its outline");
        assert_eq!(placement_of(ResType::UTT, &new), placement_of(ResType::UTT, &trigger));
        assert!(!new.contains("VarTable"), "the old one's variables go");

        let mut encounter = instance(&p, ResType::UTE, &old_bp, at, &square).unwrap();
        let mut point = Struct::new(0);
        point.set("X", Value::Float(6.0));
        encounter.set("SpawnPointList", Value::List(vec![point]));
        let new = replaced(&p, ResType::UTE, &encounter, &blueprint("NEW")).unwrap();
        assert_eq!(new.list("SpawnPointList"), encounter.list("SpawnPointList"));
        assert_eq!(new.list("Geometry"), encounter.list("Geometry"));

        let mut chest = instance(&p, ResType::UTP, &old_bp, at, &[]).unwrap();
        let mut scale = Struct::new(6);
        scale.set("Scope", Value::Int(0));
        chest.set("VisTransformList", Value::List(vec![scale]));
        let mut dynamic = blueprint("NEW");
        dynamic.set("Static", Value::Byte(0));
        let new = replaced(&p, ResType::UTP, &chest, &dynamic).unwrap();
        assert_eq!((new.float("X"), new.float("Bearing")), (Some(5.0), Some(0.7)));
        assert_eq!(new.get("VisTransformList"), chest.get("VisTransformList"));
        assert_eq!(new.resref("OnUsed").map(|r| r.to_string()).as_deref(), Some("new_script"));
        let mut fixed = blueprint("ROCK");
        fixed.set("Static", Value::Byte(1));
        let new = replaced(&p, ResType::UTP, &chest, &fixed).unwrap();
        assert!(!new.contains("VisTransformList"), "a static placeable takes no transform");
    }

    /// Replace Selected, keeping their tags and scripts: a placeholder made
    /// anew from the blueprint keeps its own tag, name, scripts and
    /// variables (a script the blueprint has and it hadn't is left empty);
    /// the rest, its appearance here, is the blueprint's.
    #[test]
    fn replaced_objects_can_keep_their_own_tag_name_scripts_and_variables() {
        let game = game();
        let none = |_: ResRef| None;
        let p = Placing { game: &game, item: &none };
        let text = |s: &str| Value::String(s.as_bytes().to_vec());
        let script = |s: &str| Value::resref(ResRef::from_str(s).unwrap());
        let name = |s: &str| {
            Value::LocString(mg_core::LocString::from_text(
                mg_core::Language::ENGLISH,
                mg_core::Gender::Male,
                s,
            ))
        };
        let mut placeholder = Struct::new(0);
        placeholder.set("Tag", text("QUEST_CHEST"));
        placeholder.set("LocName", name("Placeholder"));
        placeholder.set("Appearance", Value::Dword(1));
        placeholder.set("OnUsed", script("q_chest_used"));
        let mut var = Struct::new(0);
        var.set("Name", text("nQuest"));
        placeholder.set("VarTable", Value::List(vec![var]));
        let at = Placement { position: [5.0, 6.0, 1.0], rotation: 0.7 };
        let old = instance(&p, ResType::UTP, &placeholder, at, &[]).unwrap();
        let mut chest = Struct::new(0);
        chest.set("Tag", text("FINE_CHEST"));
        chest.set("LocName", name("Fine Chest"));
        chest.set("Appearance", Value::Dword(7));
        chest.set("OnUsed", script("chest_used"));
        chest.set("OnOpen", script("chest_open"));
        chest.set("TemplateResRef", script("fine_chest"));
        let new = replaced_keeping_own(&p, ResType::UTP, &old, &chest).unwrap();
        assert_eq!(new.string("Tag"), Some(&b"QUEST_CHEST"[..]));
        assert_eq!(new.get("LocName"), old.get("LocName"));
        assert_eq!(new.resref("OnUsed").unwrap().to_string(), "q_chest_used");
        let open = new.resref("OnOpen").map(|r| r.to_string());
        assert!(open.is_none_or(|s| s.is_empty()), "its own: none");
        assert_eq!(new.list("VarTable"), old.list("VarTable"));
        assert_eq!(new.integer("Appearance"), Some(7), "the rest the blueprint's");
        assert_eq!(new.resref("TemplateResRef").unwrap().to_string(), "fine_chest");
        assert_eq!((new.float("X"), new.float("Bearing")), (Some(5.0), Some(0.7)));
        // A creature's two names.
        let mut bandit = Struct::new(0);
        bandit.set("FirstName", name("Old"));
        bandit.set("LastName", name("Bandit"));
        bandit.set("ScriptSpawn", script("q_spawn"));
        let old = instance(&p, ResType::UTC, &bandit, at, &[]).unwrap();
        let mut guard = Struct::new(0);
        guard.set("FirstName", name("Guard"));
        guard.set("ScriptSpawn", script("nw_c2_default9"));
        let new = replaced_keeping_own(&p, ResType::UTC, &old, &guard).unwrap();
        assert_eq!(new.get("FirstName"), old.get("FirstName"));
        assert_eq!(new.get("LastName"), old.get("LastName"));
        assert_eq!(new.resref("ScriptSpawn").unwrap().to_string(), "q_spawn");
        assert!(!new.contains("VarTable"));
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

    /// Update Instances reaches the items placed objects hold: in a
    /// chest, a creature's pack and hands, a store's page and a bag, each
    /// made again from its blueprint where it is held.
    #[test]
    fn held_items_are_found_and_made_again_from_their_blueprint() {
        let game = game();
        let blueprint = |cost: u32| {
            let mut s = Struct::new(0);
            s.set("TemplateResRef", Value::resref(ResRef::from_str("potion").unwrap()));
            s.set("BaseItem", Value::Int(49));
            s.set("ModelPart1", Value::Byte(21));
            s.set("Cost", Value::Dword(cost));
            s
        };
        let old = |r: ResRef| (r.to_string() == "potion").then(|| blueprint(20));
        let p = Placing { game: &game, item: &old };
        let entry = |x: u16| {
            let mut e = Struct::new(0);
            e.set("InventoryRes", Value::resref(ResRef::from_str("potion").unwrap()));
            e.set("Repos_PosX", Value::Word(x));
            e.set("Repos_Posy", Value::Word(0));
            e
        };
        // A chest of two, a creature with one in hand and one in its
        // pack (which does not drop), a store with one it has no end of.
        let mut chest = Struct::new(0);
        chest.set("ItemList", Value::List(vec![entry(0), entry(2)]));
        let chest = instance(&p, ResType::UTP, &chest, Placement::default(), &[]).unwrap();
        let mut creature = Struct::new(0);
        let mut hand = Struct::new(16);
        hand.set("EquippedRes", Value::resref(ResRef::from_str("potion").unwrap()));
        creature.set("Equip_ItemList", Value::List(vec![hand]));
        let mut pack = entry(1);
        pack.set("Dropable", Value::Byte(0));
        pack.set("StackSize", Value::Word(20));
        creature.set("ItemList", Value::List(vec![pack]));
        let creature = instance(&p, ResType::UTC, &creature, Placement::default(), &[]).unwrap();
        let mut store = Struct::new(0);
        let mut page = Struct::new(0);
        let mut stock = entry(0);
        stock.set("Infinite", Value::Byte(1));
        page.set("ItemList", Value::List(vec![stock]));
        store.set("StoreList", Value::List(vec![page]));
        let store = instance(&p, ResType::UTM, &store, Placement::default(), &[]).unwrap();
        let mut git = Struct::new(u32::MAX);
        git.set("Placeable List", Value::List(vec![chest]));
        git.set("Creature List", Value::List(vec![creature]));
        git.set("StoreList", Value::List(vec![store]));

        let all = |_: ResRef| true;
        let found: Vec<Held> = held_items(&git, &all).into_iter().map(|(at, ..)| at).collect();
        let expected: Vec<Held> = vec![
            vec![("Creature List", 0), ("Equip_ItemList", 0)],
            vec![("Creature List", 0), ("ItemList", 0)],
            vec![("Placeable List", 0), ("ItemList", 0)],
            vec![("Placeable List", 0), ("ItemList", 1)],
            vec![("StoreList", 0), ("StoreList", 0), ("ItemList", 0)],
        ];
        assert_eq!(found, expected);
        assert!(held_items(&git, &|_| false).is_empty());

        // The blueprint changed: all but the chest's second are made again.
        let new = |r: ResRef| (r.to_string() == "potion").then(|| blueprint(99));
        let skipped: Held = vec![("Placeable List", 0), ("ItemList", 1)];
        let which = |at: &[(&'static str, usize)]| at != skipped.as_slice();
        let (updated, n) = update_held(&p, &git, &new, &which).unwrap();
        assert_eq!(n, 4);
        let cost = |s: &Struct| s.get("Cost").cloned();
        let chest = &updated.list("Placeable List").unwrap()[0].list("ItemList").unwrap().to_vec();
        assert_eq!(cost(&chest[0]), Some(Value::Dword(99)));
        assert_eq!(cost(&chest[1]), Some(Value::Dword(20)), "not picked: as it was");
        assert_eq!(chest[0].get("Repos_PosX"), Some(&Value::Word(0)), "its place in the grid");
        let creature = &updated.list("Creature List").unwrap()[0];
        let hand = &creature.list("Equip_ItemList").unwrap()[0];
        assert_eq!((hand.id, cost(hand)), (16, Some(Value::Dword(99))), "in the same slot");
        let pack = &creature.list("ItemList").unwrap()[0];
        assert_eq!(cost(pack), Some(Value::Dword(99)));
        assert_eq!(pack.get("Dropable"), Some(&Value::Byte(0)), "what is said of it there");
        // How many: the blueprint's, as Aurora makes them again.
        assert_eq!(pack.get("StackSize"), Some(&Value::Word(1)), "the blueprint's stack");
        let page = &updated.list("StoreList").unwrap()[0].list("StoreList").unwrap()[0];
        let stock = &page.list("ItemList").unwrap()[0];
        assert_eq!(cost(stock), Some(Value::Dword(99)));
        assert_eq!(stock.get("Infinite"), Some(&Value::Byte(1)));
        // Nothing of the blueprints asked for: nothing to do.
        assert!(update_held(&p, &git, &|_| None, &|_| true).is_none());
    }

    /// A creature placed from its blueprint holds its items whole, and
    /// each keeps what the blueprint says of it there: equipped as in the
    /// backpack, `Dropable` and `Pickpocketable` (the engine reads them
    /// from the placed creature's items: `engine_item_flags`).
    #[test]
    fn a_placed_creature_s_items_keep_their_flags() {
        let game = game();
        let potion = |r: ResRef| {
            (r.to_string() == "potion").then(|| {
                let mut s = Struct::new(0);
                s.set("TemplateResRef", Value::resref(r));
                s.set("BaseItem", Value::Int(49));
                s
            })
        };
        let p = Placing { game: &game, item: &potion };
        let flagged = |mut e: Struct| {
            e.set("Dropable", Value::Byte(1));
            e.set("Pickpocketable", Value::Byte(1));
            e
        };
        let mut hand = Struct::new(16);
        hand.set("EquippedRes", Value::resref(ResRef::from_str("potion").unwrap()));
        let mut pack = Struct::new(0);
        pack.set("InventoryRes", Value::resref(ResRef::from_str("potion").unwrap()));
        let mut creature = Struct::new(0);
        creature.set("Equip_ItemList", Value::List(vec![flagged(hand)]));
        creature.set("ItemList", Value::List(vec![flagged(pack)]));
        let placed = instance(&p, ResType::UTC, &creature, Placement::default(), &[]).unwrap();
        for list in ["Equip_ItemList", "ItemList"] {
            let item = &placed.list(list).unwrap()[0];
            assert_eq!(item.integer("BaseItem"), Some(49), "{list}: the whole item");
            assert!(!item.contains("EquippedRes") && !item.contains("InventoryRes"), "{list}");
            for flag in ["Dropable", "Pickpocketable"] {
                assert_eq!(item.get(flag), Some(&Value::Byte(1)), "{list}: {flag}");
            }
        }
        assert_eq!(placed.list("Equip_ItemList").unwrap()[0].id, 16, "in its slot");
    }

    /// An item moved between the backpack and a slot changes where it is
    /// held and nothing else, as the game's modules and Aurora write them:
    /// a place in the grid in the backpack, none in a slot (the slot its
    /// struct id); a blueprint's entry names its item `InventoryRes` in the
    /// backpack and `EquippedRes` in a slot, in the same place among its
    /// fields.
    #[test]
    fn an_item_moved_changes_only_where_it_is_held() {
        let mut entry = {
            let mut s = Struct::new(0);
            s.set("InventoryRes", Value::resref(ResRef::from_str("nw_it_torch001").unwrap()));
            s.set("Dropable", Value::Byte(1));
            s
        };
        entry.id = 3;
        entry.set("Repos_PosX", Value::Word(4));
        entry.set("Repos_Posy", Value::Word(1));
        entry.set("MG_Probe", Value::Int(7));
        let worn = equipped_item(&entry, 0x100);
        let labels = |s: &Struct| s.fields.iter().map(|f| f.label.to_string()).collect::<Vec<_>>();
        assert_eq!(worn.id, 0x100);
        assert_eq!(labels(&worn), ["EquippedRes", "Dropable", "MG_Probe"]);
        assert_eq!(worn.resref("EquippedRes"), entry.resref("InventoryRes"));
        let back = listed_item(&worn, 3, 4, 1);
        assert_eq!(back.id, 3);
        assert_eq!(
            labels(&back),
            ["InventoryRes", "Dropable", "MG_Probe", "Repos_PosX", "Repos_Posy"]
        );
        assert_eq!(back.get("MG_Probe"), entry.get("MG_Probe"));
        // A whole item (a placed creature's) has neither name: as it is.
        let mut whole = Struct::new(0);
        whole.set("BaseItem", Value::Int(52));
        whole.set("TemplateResRef", Value::resref(ResRef::from_str("nw_it_mring022").unwrap()));
        whole.set("Pickpocketable", Value::Byte(1));
        let worn = equipped_item(&listed_item(&whole, 0, 0, 0), 0x80);
        assert_eq!((worn.id, labels(&worn)), (0x80, labels(&whole)));
    }
}
